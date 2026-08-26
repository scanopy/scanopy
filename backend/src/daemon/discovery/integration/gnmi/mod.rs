//! gNMI (OpenConfig) discovery integration: interfaces and LLDP neighbours.
//!
//! For devices whose management plane is gNMI rather than SNMP — gNMI-first NOSes, and
//! fleets where enabling an SNMP agent is its own project (scanopy#690 has the survey). One
//! credential yields the interface rows an ifTable walk would, with the same `lldp_*` columns
//! the SNMP collector fills, so the server-side L2 resolution runs unchanged.
//!
//! Two models, one Subscribe per subtree (a device refuses a whole request over one path it
//! does not serve, and which paths those are varies: ArcOS has no `/lldp/state`, no
//! `mac-address`):
//! - `openconfig-interfaces` `/interfaces/interface/state` (plus `ethernet/state` for the MAC
//!   and port speed) supplies `ifindex`, `type`, `description`, `admin-status`, `oper-status`
//!   — the row itself.
//! - `openconfig-lldp` `/lldp/interfaces/interface` supplies the neighbour on each row, joined
//!   on the interface `name` both models key by; `/lldp/state` the device's own chassis id.
//!
//! Transport notes, validated against Arrcus ArcOS 8.2/8.5 (virtual and physical):
//! - Authentication is `username`/`password` request metadata — the OpenConfig convention.
//! - The read is Subscribe `mode: ONCE` with PROTO encoding (see [`transport`] for why not
//!   `Get` and why not JSON). Values arrive one leaf per update; devices that send JSON blobs
//!   instead are flattened to the same leaves.
//! - ArcOS serves **no chassis-id leaf** under `neighbors/neighbor/state`, so the remote
//!   identity falls back: management-address (resolved against `ip_addresses` server-side)
//!   first, then a MAC-shaped port-id. Devices that do serve `chassis-id`/`chassis-id-type`
//!   get the faithful mapping.
//!
//! `openconfig-interfaces` is required — without it there are no rows to hang neighbours on,
//! and rows invented from LLDP alone (no ifIndex, no statuses) would shadow an SNMP walk's
//! real ones when both run against one device. `/lldp/*` and `ethernet/state` are optional.
//! Running gNMI and SNMP against the same device is fine: both contribute at `FullIfTable`
//! scope and `HostData::contribute_interfaces` merges per row, first writer wins.

pub mod proto;
pub mod transport;

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use mac_address::MacAddress;

use super::{
    Checkpoint, Completeness, DiscoveryIntegration, IntegrationContext, IntegrationFailure,
    InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayload, CredentialQueryPayloadDiscriminants, GnmiQueryCredential,
};
use crate::server::interfaces::r#impl::base::{
    IfAdminStatus, IfOperStatus, Interface, InterfaceBase, InterfaceDataComplete, if_type,
};
use crate::server::lldp::{LldpChassisId, LldpPortId, canonical_mac};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::snmp::generated::get_if_type_number;
use proto::gnmi::{Notification, Path, PathElem, TypedValue, typed_value};
use transport::{ConnectError, GnmiTransport, TonicTransport};

pub struct GnmiIntegration;

/// Working credential handed from probe to execute.
struct GnmiProbeHandle {
    credential: GnmiQueryCredential,
}

/// One interface's `openconfig-interfaces` state leaves.
#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct InterfaceLeaves {
    ifindex: Option<u64>,
    /// The `iana-if-type` identity, module prefix and all: `iana-if-type:ethernetCsmacd`.
    if_type: Option<String>,
    description: Option<String>,
    admin_status: Option<String>,
    oper_status: Option<String>,
    mac_address: Option<String>,
    /// `openconfig-if-ethernet` `port-speed` (or the negotiated one), an identity: `SPEED_10GB`.
    port_speed: Option<String>,
}

/// One neighbour's `openconfig-lldp` state leaves.
#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct NeighborLeaves {
    chassis_id: Option<String>,
    chassis_id_type: Option<String>,
    port_id: Option<String>,
    port_id_type: Option<String>,
    port_description: Option<String>,
    system_name: Option<String>,
    system_description: Option<String>,
    management_address: Option<String>,
}

impl NeighborLeaves {
    /// How many of the leaves that identify the far end this entry carries.
    fn evidence(&self) -> usize {
        [
            &self.chassis_id,
            &self.management_address,
            &self.system_name,
            &self.port_id,
        ]
        .into_iter()
        .filter(|v| v.is_some())
        .count()
    }
}

/// Everything the Subscribes yielded, keyed the way the models key it.
#[derive(Default, Debug)]
pub(crate) struct Collection {
    /// By interface name, from `/interfaces`.
    pub interfaces: BTreeMap<String, InterfaceLeaves>,
    /// By (local interface name, neighbor id), from `/lldp/interfaces`.
    pub neighbors: BTreeMap<(String, String), NeighborLeaves>,
    /// Local chassis identity, when the device serves `/lldp/state` (ArcOS does not).
    pub local_chassis_id: Option<String>,
    pub local_chassis_id_type: Option<String>,
}

/// The subtrees one collection subscribes to, each its own Subscribe. Wildcard keys rather
/// than bare list elements: the spec treats both as "every entry", but `[name=*]` is the form
/// every implementation has been exercised with (it is what gnmic sends).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Subtree {
    /// `/interfaces/interface[name=*]/state`: the rows.
    InterfaceState,
    /// `/interfaces/interface[name=*]/ethernet/state`: MAC and port speed, where served.
    EthernetState,
    /// `/lldp/state`: the device's own chassis id, where served.
    LldpLocal,
    /// `/lldp/interfaces/interface[name=*]`: the neighbours.
    LldpNeighbors,
}

impl Subtree {
    const ALL: [Subtree; 4] = [
        Subtree::InterfaceState,
        Subtree::EthernetState,
        Subtree::LldpLocal,
        Subtree::LldpNeighbors,
    ];

    pub(crate) fn path(self) -> Path {
        let elems: &[&str] = match self {
            Self::InterfaceState => &["interfaces", "interface[name=*]", "state"],
            Self::EthernetState => &["interfaces", "interface[name=*]", "ethernet", "state"],
            Self::LldpLocal => &["lldp", "state"],
            Self::LldpNeighbors => &["lldp", "interfaces", "interface[name=*]"],
        };
        Path {
            elem: elems
                .iter()
                .map(|e| match e.split_once('[') {
                    Some((name, key)) => {
                        let (k, v) = key
                            .trim_end_matches(']')
                            .split_once('=')
                            .expect("static path keys are well-formed");
                        PathElem {
                            name: name.to_string(),
                            key: [(k.to_string(), v.to_string())].into_iter().collect(),
                        }
                    }
                    None => PathElem {
                        name: e.to_string(),
                        ..Default::default()
                    },
                })
                .collect(),
            ..Default::default()
        }
    }
}

/// A flattened update: the full path (prefix + update path, JSON keys appended) and the
/// leaf's value as text.
struct Leaf {
    elems: Vec<PathElem>,
    value: String,
}

/// Strip the YANG module prefix json_ietf puts on names: `openconfig-interfaces:ifindex`.
fn unqualified(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn scalar_to_string(v: &typed_value::Value) -> Option<String> {
    Some(match v {
        typed_value::Value::StringVal(s) | typed_value::Value::AsciiVal(s) => s.clone(),
        typed_value::Value::IntVal(i) => i.to_string(),
        typed_value::Value::UintVal(u) => u.to_string(),
        typed_value::Value::BoolVal(b) => b.to_string(),
        _ => return None,
    })
}

/// Flatten one update to leaves. PROTO encoding gives one typed leaf per update; JSON
/// encodings give a blob rooted at the update path, whose objects become path elements
/// (list entries keyed by their `name`/`id` member, the way both models key their lists).
fn flatten_update(prefix: &[PathElem], path: &[PathElem], val: &TypedValue) -> Vec<Leaf> {
    let mut elems: Vec<PathElem> = prefix.iter().chain(path.iter()).cloned().collect();
    let Some(value) = val.value.as_ref() else {
        return vec![];
    };
    match value {
        typed_value::Value::JsonIetfVal(bytes) | typed_value::Value::JsonVal(bytes) => {
            let Ok(json) = serde_json::from_slice::<serde_json::Value>(bytes) else {
                return vec![];
            };
            let mut out = Vec::new();
            flatten_json(&mut elems, &json, &mut out);
            out
        }
        other => scalar_to_string(other)
            .map(|value| vec![Leaf { elems, value }])
            .unwrap_or_default(),
    }
}

fn flatten_json(elems: &mut Vec<PathElem>, json: &serde_json::Value, out: &mut Vec<Leaf>) {
    match json {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                elems.push(PathElem {
                    name: unqualified(k).to_string(),
                    ..Default::default()
                });
                flatten_json(elems, v, out);
                elems.pop();
            }
        }
        serde_json::Value::Array(items) => {
            // A list: each entry re-uses the enclosing element's name and takes its key from
            // the entry itself.
            let Some(list) = elems.pop() else { return };
            for item in items {
                let mut entry = list.clone();
                if let Some(obj) = item.as_object() {
                    for key in ["name", "id"] {
                        if let Some(v) = obj.get(key).and_then(json_scalar) {
                            entry.key.insert(key.to_string(), v);
                            break;
                        }
                    }
                }
                elems.push(entry);
                flatten_json(elems, item, out);
                elems.pop();
            }
            elems.push(list);
        }
        scalar => {
            if let Some(value) = json_scalar(scalar) {
                out.push(Leaf {
                    elems: elems.clone(),
                    value,
                });
            }
        }
    }
}

fn json_scalar(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Fold one notification's updates into the collection.
pub(crate) fn absorb_notification(coll: &mut Collection, notification: &Notification) {
    let prefix = notification.prefix.as_ref().map(|p| p.elem.as_slice());
    for update in &notification.update {
        let Some(val) = update.val.as_ref() else {
            continue;
        };
        let path = update.path.as_ref().map(|p| p.elem.as_slice());
        for leaf in flatten_update(prefix.unwrap_or(&[]), path.unwrap_or(&[]), val) {
            absorb_leaf(coll, &leaf);
        }
    }
}

fn absorb_leaf(coll: &mut Collection, leaf: &Leaf) {
    let names: Vec<&str> = leaf.elems.iter().map(|e| unqualified(&e.name)).collect();
    let Some((&leaf_name, containers)) = names.split_last() else {
        return;
    };
    let key_of = |elem: &str, key: &str| {
        leaf.elems
            .iter()
            .find(|e| unqualified(&e.name) == elem)
            .and_then(|e| e.key.get(key))
            .cloned()
    };
    let value = leaf.value.clone();
    match containers.first().copied() {
        Some("interfaces") => {
            // Subinterfaces carry their own `ifindex`; they are not rows here (an ifTable lists
            // them on some devices and not others, and the parent's identity is what LLDP and
            // the L2 view need).
            if containers.contains(&"subinterface") {
                return;
            }
            let Some(name) = key_of("interface", "name") else {
                return;
            };
            let entry = coll.interfaces.entry(name).or_default();
            match (containers, leaf_name) {
                ([.., "state"], "ifindex") => entry.ifindex = value.parse().ok(),
                ([.., "state"], "type") => entry.if_type = Some(value),
                ([.., "state"], "description") => entry.description = Some(value),
                ([.., "state"], "admin-status") => entry.admin_status = Some(value),
                ([.., "state"], "oper-status") => entry.oper_status = Some(value),
                ([.., "ethernet", "state"], "mac-address") => entry.mac_address = Some(value),
                // The configured speed when there is one, else what autoneg settled on.
                ([.., "ethernet", "state"], "port-speed") => entry.port_speed = Some(value),
                ([.., "ethernet", "state"], "negotiated-port-speed") => {
                    entry.port_speed.get_or_insert(value);
                }
                _ => {}
            }
        }
        Some("lldp") => match (key_of("interface", "name"), key_of("neighbor", "id")) {
            (Some(ifname), Some(nbr)) => {
                let entry = coll.neighbors.entry((ifname, nbr)).or_default();
                match leaf_name {
                    "chassis-id" => entry.chassis_id = Some(value),
                    "chassis-id-type" => entry.chassis_id_type = Some(value),
                    "port-id" => entry.port_id = Some(value),
                    "port-id-type" => entry.port_id_type = Some(value),
                    "port-description" => entry.port_description = Some(value),
                    "system-name" => entry.system_name = Some(value),
                    "system-description" => entry.system_description = Some(value),
                    "management-address" => entry.management_address = Some(value),
                    _ => {}
                }
            }
            // `/lldp/state` leaves carry the device's own identity.
            (None, None) => match leaf_name {
                "chassis-id" => coll.local_chassis_id = Some(value),
                "chassis-id-type" => coll.local_chassis_id_type = Some(value),
                _ => {}
            },
            _ => {}
        },
        _ => {}
    }
}

/// `iana-if-type` identity → IF-MIB ifType number: strip the YANG module prefix
/// (`iana-if-type:`, `ianaift:`), look the enumerator name up in the generated IANA registry
/// table (the identity names are literally the IANAifType labels). Vendor-private identities
/// (DNOS `irb`, `mgmt-ncx-member`) and anything else the registry lacks fall to `other(1)` —
/// the row is kept, never dropped. Not the `if_type` constants: those name the VLAN entries
/// one off from IANA (`L2_VLAN` = 136 where IANA's l2vlan is 135).
pub(crate) fn if_type_from_identity(identity: &str) -> i32 {
    get_if_type_number(unqualified(identity)).unwrap_or(if_type::OTHER)
}

/// `openconfig-if-ethernet` `ETHERNET_SPEED` identity (`SPEED_10GB`, `SPEED_2500MB`) → bits
/// per second. `SPEED_UNKNOWN` and anything unrecognised are "no speed", never a guess.
pub(crate) fn speed_from_identity(identity: &str) -> Option<i64> {
    let s = unqualified(identity).strip_prefix("SPEED_")?;
    let digits = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let n: i64 = digits.parse().ok()?;
    let per: i64 = match &s[digits.len()..] {
        "MB" => 1_000_000,
        "GB" => 1_000_000_000,
        "TB" => 1_000_000_000_000,
        _ => return None,
    };
    Some(n * per)
}

/// `openconfig-interfaces` `admin-status` enumeration → ifAdminStatus.
fn admin_status(v: Option<&str>) -> IfAdminStatus {
    IfAdminStatus::from(match v {
        Some("DOWN") => 2,
        Some("TESTING") => 3,
        // `UP`, and absent: an interface the device lists without an admin state is not one
        // it has shut.
        _ => 1,
    })
}

/// `openconfig-interfaces` `oper-status` enumeration → ifOperStatus. Same enumerators as
/// IF-MIB, same order.
fn oper_status(v: Option<&str>) -> IfOperStatus {
    IfOperStatus::from(match v {
        Some("UP") => 1,
        Some("DOWN") => 2,
        Some("TESTING") => 3,
        Some("DORMANT") => 5,
        Some("NOT_PRESENT") => 6,
        Some("LOWER_LAYER_DOWN") => 7,
        _ => 4,
    })
}

/// Map an `openconfig-lldp-types` identity (`openconfig-lldp-types:MAC_ADDRESS`) plus value
/// onto the 802.1AB chassis subtype. Unknown or absent types fall back to
/// [`LldpChassisId::from_identifier_str`].
fn map_chassis(id: &str, id_type: Option<&str>) -> Option<LldpChassisId> {
    match id_type.map(unqualified) {
        Some("MAC_ADDRESS") => canonical_mac(id).map(LldpChassisId::MacAddress),
        Some("INTERFACE_NAME") => Some(LldpChassisId::InterfaceName(id.to_string())),
        Some("INTERFACE_ALIAS") => Some(LldpChassisId::InterfaceAlias(id.to_string())),
        Some("NETWORK_ADDRESS") => id.parse().ok().map(LldpChassisId::NetworkAddress),
        Some("LOCAL") => Some(LldpChassisId::LocallyAssigned(id.to_string())),
        Some("CHASSIS_COMPONENT") => Some(LldpChassisId::ChassisComponent(id.to_string())),
        Some("PORT_COMPONENT") => Some(LldpChassisId::PortComponent(id.to_string())),
        _ => Some(LldpChassisId::from_identifier_str(id)),
    }
}

fn map_port(id: &str, id_type: Option<&str>) -> Option<LldpPortId> {
    match id_type.map(unqualified) {
        Some("MAC_ADDRESS") => canonical_mac(id).map(LldpPortId::MacAddress),
        Some("INTERFACE_NAME") => Some(LldpPortId::InterfaceName(id.to_string())),
        Some("INTERFACE_ALIAS") => Some(LldpPortId::InterfaceAlias(id.to_string())),
        Some("NETWORK_ADDRESS") => id.parse().ok().map(LldpPortId::NetworkAddress),
        Some("LOCAL") => Some(LldpPortId::LocallyAssigned(id.to_string())),
        Some("PORT_COMPONENT") => Some(LldpPortId::PortComponent(id.to_string())),
        Some("AGENT_CIRCUIT_ID") => Some(LldpPortId::AgentCircuitId(id.to_string())),
        _ => Some(LldpPortId::from_identifier_str(id)),
    }
}

/// Build the interface rows a collection amounts to: one per `/interfaces` entry, with the
/// LLDP neighbour on the same name folded in. A neighbour on a name `/interfaces` did not list
/// has no row to live on and is dropped.
pub(crate) fn collection_to_interfaces(
    coll: &Collection,
    host_id: uuid::Uuid,
    network_id: uuid::Uuid,
) -> Vec<Interface> {
    // Several neighbours on one port keep the one with the most identity (multi-neighbour
    // ports are uplink-shaped and better resolved by the far end anyway). Not the first: ArcOS
    // lists a Linux host twice, one entry per chassis-id subtype it advertises, and the
    // lexically earlier entry is the one without a management address or a system name.
    let mut neighbor_by_port: BTreeMap<&str, &NeighborLeaves> = BTreeMap::new();
    for ((ifname, _), leaves) in &coll.neighbors {
        let slot = neighbor_by_port.entry(ifname).or_insert(leaves);
        if leaves.evidence() > slot.evidence() {
            *slot = leaves;
        }
    }
    coll.interfaces
        .iter()
        .map(|(name, i)| {
            let name = name.as_str();
            let n = neighbor_by_port.get(name).copied();
            // Remote identity, best evidence first: an explicit chassis-id leaf; else the
            // management address (resolves against ip_addresses server-side); else a
            // MAC-shaped port-id. ArcOS serves no chassis-id leaf at all, so the fallbacks are
            // what carries its neighbours.
            let chassis = n.and_then(|n| match &n.chassis_id {
                Some(id) => map_chassis(id, n.chassis_id_type.as_deref()),
                None => n
                    .management_address
                    .as_deref()
                    .and_then(|a| a.parse().ok().map(LldpChassisId::NetworkAddress))
                    .or_else(|| {
                        n.port_id
                            .as_deref()
                            .and_then(canonical_mac)
                            .map(LldpChassisId::MacAddress)
                    }),
            });
            let port = n.and_then(|n| {
                n.port_id
                    .as_deref()
                    .and_then(|id| map_port(id, n.port_id_type.as_deref()))
            });
            Interface::new(InterfaceBase {
                host_id,
                network_id,
                if_index: i.ifindex.and_then(|x| i32::try_from(x).ok()).unwrap_or(0),
                // ifDescr is the interface name on every NOS that matters; the operator's
                // text is ifAlias, which is what `description` is in openconfig.
                if_descr: name.to_string(),
                if_name: Some(name.to_string()),
                if_alias: i.description.clone().filter(|d| !d.is_empty()),
                // No `type` leaf is not "Ethernet"; `other` keeps the row and claims nothing.
                if_type: i
                    .if_type
                    .as_deref()
                    .map(if_type_from_identity)
                    .unwrap_or(if_type::OTHER),
                speed_bps: i.port_speed.as_deref().and_then(speed_from_identity),
                admin_status: admin_status(i.admin_status.as_deref()),
                oper_status: oper_status(i.oper_status.as_deref()),
                mac_address: i
                    .mac_address
                    .as_deref()
                    .and_then(|m| m.parse::<MacAddress>().ok()),
                ip_address_id: None,
                neighbor: None,
                // Stamped server-side from the evidence carried in this payload, on ingest.
                neighbor_seen_at: None,
                lldp_chassis_id: chassis,
                lldp_port_id: port,
                lldp_sys_name: n.and_then(|n| n.system_name.clone()),
                lldp_port_desc: n.and_then(|n| n.port_description.clone()),
                lldp_mgmt_addr: n
                    .and_then(|n| n.management_address.as_deref())
                    .and_then(|a| a.parse().ok()),
                lldp_sys_desc: n.and_then(|n| n.system_description.clone()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
                native_vlan_id: None,
                vlan_ids: None,
            })
        })
        .collect()
}

/// The collection proper, transport-agnostic: one Subscribe per subtree, folded together.
///
/// `/interfaces/interface/state` is required: refusing it is the error, verbatim from the
/// device so the operator sees which path it objected to. The other subtrees are optional
/// extras — `/lldp/state` and `ethernet/state` are not universally served, and a device
/// without the LLDP model at all still has an interface table worth having.
pub(crate) async fn collect(transport: &mut dyn GnmiTransport) -> anyhow::Result<Collection> {
    let mut coll = Collection::default();
    for subtree in Subtree::ALL {
        match transport.subscribe_once(vec![subtree.path()]).await {
            Ok(notifications) => {
                for n in &notifications {
                    absorb_notification(&mut coll, n);
                }
            }
            Err(e) if subtree == Subtree::InterfaceState => {
                return Err(e.context("openconfig-interfaces is required and was not served"));
            }
            Err(e) => {
                tracing::debug!(?subtree, error = %e, "gNMI subtree not served; continuing");
            }
        }
    }
    Ok(coll)
}

#[async_trait]
impl DiscoveryIntegration for GnmiIntegration {
    /// `/interfaces/interface` is the device's own account of every interface it has, the
    /// same standing as an ifTable walk.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::FullIfTable
    }

    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Gnmi
    }

    fn estimated_seconds(&self) -> u32 {
        10
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(120)
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let cred = match ctx.credential {
            CredentialQueryPayload::Gnmi(c) => c,
            _ => return Err(ProbeFailure::malformed("Expected gNMI credential")),
        };
        let mut transport = TonicTransport::connect(ctx.ip, cred, ctx.cancel.clone())
            .await
            .map_err(|e| match e {
                ConnectError::Unsupported(m) => ProbeFailure::malformed(m),
                ConnectError::Tls(m) => ProbeFailure::tls_failed(m),
                ConnectError::Dial(m) => ProbeFailure::unreachable(m),
            })?;
        transport
            .capabilities()
            .await
            .map_err(|e| ProbeFailure::rejected(e.to_string()))?;
        Ok(ProbeSuccess {
            client_probe: ClientProbe::Gnmi,
            ports: vec![PortType::new_tcp(cred.port)],
            handle: Some(Box::new(GnmiProbeHandle {
                credential: cred.clone(),
            })),
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        _checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        let handle = ctx
            .probe_handle
            .and_then(|h| h.downcast_ref::<GnmiProbeHandle>())
            .ok_or_else(|| anyhow::anyhow!("gNMI execute called without GnmiProbeHandle"))?;

        let mut transport = TonicTransport::connect(ctx.ip, &handle.credential, ctx.cancel.clone())
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let coll = collect(&mut transport).await?;

        let interfaces =
            collection_to_interfaces(&coll, ctx.host_id, host_data.host.base.network_id);
        tracing::info!(
            ip = %ctx.ip,
            interfaces = coll.interfaces.len(),
            neighbors = coll.neighbors.len(),
            "gNMI openconfig-interfaces/lldp collection complete"
        );
        if let Some(chassis) = coll
            .local_chassis_id
            .as_deref()
            .and_then(|id| map_chassis(id, coll.local_chassis_id_type.as_deref()))
        {
            host_data.with_chassis_id(chassis.identifier());
        }
        host_data.contribute_interfaces(
            ctx.interface_source,
            interfaces,
            // `/interfaces` answered or `collect` would have failed: the set is the device's
            // own full account, so the server may prune what is no longer in it.
            true,
            InterfaceDataComplete {
                lldp: true,
                cdp: false,
                fdb: false,
                vlan_membership: false,
            },
        );
        Ok(Completeness::Complete)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::gnmi::Update;

    /// A device answering Subscribe ONCE per subtree from scripts of `path = value` lines, the
    /// way ArcOS does: one typed leaf per update, no prefix. A subtree with no script is
    /// refused with the `InvalidArgument` a real device sends. Paths are gnmic-style
    /// (`lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=1]/state/port-id`).
    #[derive(Default)]
    struct ScriptedDevice {
        served: BTreeMap<&'static str, &'static str>,
    }

    impl ScriptedDevice {
        fn serve(mut self, subtree: Subtree, script: &'static str) -> Self {
            self.served.insert(subtree_key(subtree), script);
            self
        }
    }

    fn subtree_key(subtree: Subtree) -> &'static str {
        match subtree {
            Subtree::InterfaceState => "interfaces/interface[name=*]/state",
            Subtree::EthernetState => "interfaces/interface[name=*]/ethernet/state",
            Subtree::LldpLocal => "lldp/state",
            Subtree::LldpNeighbors => "lldp/interfaces/interface[name=*]",
        }
    }

    fn render_path(path: &Path) -> String {
        path.elem
            .iter()
            .map(|e| {
                let keys: String = e.key.iter().map(|(k, v)| format!("[{k}={v}]")).collect();
                format!("{}{keys}", e.name)
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Split on `/` outside brackets only: key values carry slashes (`[name=ge10-0/0/0]`).
    fn split_elems(path: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let (mut start, mut depth) = (0, 0);
        for (i, c) in path.char_indices() {
            match c {
                '[' => depth += 1,
                ']' => depth -= 1,
                '/' if depth == 0 => {
                    out.push(&path[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.push(&path[start..]);
        out.into_iter().filter(|e| !e.is_empty()).collect()
    }

    fn parse_path(path: &str) -> Path {
        Path {
            elem: split_elems(path)
                .into_iter()
                .map(|e| match e.split_once('[') {
                    Some((name, key)) => {
                        let (k, v) = key.trim_end_matches(']').split_once('=').unwrap();
                        PathElem {
                            name: name.into(),
                            key: [(k.to_string(), v.to_string())].into_iter().collect(),
                        }
                    }
                    None => PathElem {
                        name: e.into(),
                        ..Default::default()
                    },
                })
                .collect(),
            ..Default::default()
        }
    }

    fn typed(value: &str) -> TypedValue {
        // Numbers travel as uint leaves on the wire (ifindex, mtu); everything else is text.
        let v = match value.parse::<u64>() {
            Ok(u) => typed_value::Value::UintVal(u),
            Err(_) => typed_value::Value::StringVal(value.to_string()),
        };
        TypedValue { value: Some(v) }
    }

    /// One notification per non-blank script line, `path = value`.
    fn script_to_notifications(script: &str) -> Vec<Notification> {
        script
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|line| {
                let (path, value) = line.split_once(" = ").unwrap_or((line, ""));
                Notification {
                    update: vec![Update {
                        path: Some(parse_path(path.trim())),
                        val: Some(typed(value.trim())),
                        ..Default::default()
                    }],
                    ..Default::default()
                }
            })
            .collect()
    }

    #[async_trait]
    impl GnmiTransport for ScriptedDevice {
        async fn capabilities(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
        async fn subscribe_once(&mut self, paths: Vec<Path>) -> anyhow::Result<Vec<Notification>> {
            let [path] = paths.as_slice() else {
                panic!("the collector subscribes one subtree at a time");
            };
            let key = render_path(path);
            match self.served.get(key.as_str()) {
                Some(script) => Ok(script_to_notifications(script)),
                None => anyhow::bail!(
                    "gNMI Subscribe failed: code: 'Client specified an invalid argument', \
                     message: \"Requested Path '{key}' is not supported\""
                ),
            }
        }
    }

    // Captured 2026-08-25 from netlab-leaf1 (Arrcus ArcOS 8.5, Edgecore AS7326-56X), Subscribe
    // ONCE, PROTO encoding, via this crate's own transport (gnmic was not to hand). Trimmed to
    // a handful of the 60 rows; the leaves kept are verbatim, counters included where they
    // show what is ignored. ArcOS sends `type` without the `iana-if-type:` prefix, blank
    // `description` leaves for undescribed ports, and no `mac-address` anywhere.
    const ARCOS_INTERFACE_STATE: &str = "
        interfaces/interface[name=swp1]/state/counters/out-octets = 2487600
        interfaces/interface[name=swp1]/state/type = ethernetCsmacd
        interfaces/interface[name=swp1]/state/ifindex = 1001
        interfaces/interface[name=swp1]/state/oper-status = UP
        interfaces/interface[name=swp1]/state/admin-status = UP
        interfaces/interface[name=swp1]/state/description =
        interfaces/interface[name=swp1]/state/mtu = 1526
        interfaces/interface[name=swp1]/state/name = swp1
        interfaces/interface[name=swp46]/state/admin-status = UP
        interfaces/interface[name=swp46]/state/description = netlab-mgmt0 : Ethernet48
        interfaces/interface[name=swp46]/state/ifindex = 1046
        interfaces/interface[name=swp46]/state/oper-status = UP
        interfaces/interface[name=swp46]/state/type = ethernetCsmacd
        interfaces/interface[name=swp53]/state/admin-status = UP
        interfaces/interface[name=swp53]/state/description =
        interfaces/interface[name=swp53]/state/ifindex = 1053
        interfaces/interface[name=swp53]/state/oper-status = UP
        interfaces/interface[name=swp53]/state/type = ethernetCsmacd
        interfaces/interface[name=swp55]/state/admin-status = UP
        interfaces/interface[name=swp55]/state/description = PROTECT: netlab-spine2 : swp32 : FOR UNDERLAY
        interfaces/interface[name=swp55]/state/ifindex = 1055
        interfaces/interface[name=swp55]/state/oper-status = UP
        interfaces/interface[name=swp55]/state/type = ethernetCsmacd
        interfaces/interface[name=loopback0]/state/admin-status = UP
        interfaces/interface[name=loopback0]/state/ifindex = 20005
        interfaces/interface[name=loopback0]/state/oper-status = UP
        interfaces/interface[name=loopback0]/state/type = softwareLoopback
        interfaces/interface[name=vlan1000]/state/admin-status = UP
        interfaces/interface[name=vlan1000]/state/type = l3ipvlan
        interfaces/interface[name=vlan1000]/state/ifindex = 20031
        interfaces/interface[name=vlan1000]/state/oper-status = UP
        interfaces/interface[name=ma1]/state/ifindex = 4
        interfaces/interface[name=ma1]/state/type = ethernetCsmacd
        interfaces/interface[name=ma1]/state/admin-status = UP
        interfaces/interface[name=ma1]/state/oper-status = UP
    ";

    /// ArcOS's `ethernet/state` carries only its own `effective-speed` (Mb/s), not the model's
    /// `port-speed` identity, so it contributes nothing to the row.
    const ARCOS_ETHERNET_STATE: &str = "
        interfaces/interface[name=swp1]/ethernet/state/effective-speed = 25000
        interfaces/interface[name=swp53]/ethernet/state/effective-speed = 100000
        interfaces/interface[name=ma1]/ethernet/state/effective-speed = 1000
    ";

    /// No `chassis-id` leaf on any neighbour. A Linux lldpd peer (netlab-server) shows up
    /// twice on swp1 and swp53, one entry per chassis-id subtype it advertises; only one
    /// carries the management address and system name.
    const ARCOS_LLDP_NEIGHBORS: &str = "
        lldp/interfaces/interface[name=swp1]/name = swp1
        lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=5-34:80:0d:44:45:05]/id = 5-34:80:0d:44:45:05
        lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=5-34:80:0d:44:45:05]/state/id = 5-34:80:0d:44:45:05
        lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=5-34:80:0d:44:45:05]/state/port-id = 34:80:0d:44:45:05
        lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=7-34:80:0d:44:44:f5]/id = 7-34:80:0d:44:44:f5
        lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=7-34:80:0d:44:44:f5]/state/id = 7-34:80:0d:44:44:f5
        lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=7-34:80:0d:44:44:f5]/state/management-address = 10.22.64.101
        lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=7-34:80:0d:44:44:f5]/state/port-id = 34:80:0d:44:44:f5
        lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=7-34:80:0d:44:44:f5]/state/system-description = Ubuntu 26.04 LTS Linux 7.0.0-30-generic #30-Ubuntu SMP PREEMPT_DYNAMIC Fri Jul 31 18:22:54 UTC 2026 x86_64
        lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=7-34:80:0d:44:44:f5]/state/system-name = netlab-server
        lldp/interfaces/interface[name=swp46]/name = swp46
        lldp/interfaces/interface[name=swp46]/neighbors/neighbor[id=1-Ethernet48]/state/management-address = fe80::deda:4dff:fe86:f4ea
        lldp/interfaces/interface[name=swp46]/neighbors/neighbor[id=1-Ethernet48]/state/port-id = Ethernet48
        lldp/interfaces/interface[name=swp46]/neighbors/neighbor[id=1-Ethernet48]/state/system-description = SONiC Software Version: SONiC-OS-cls_sonic_plus_4.0.0-de0fd7e72 - HwSku: Celestica ES1010-48CP - Distribution: Debian 11.11 - Kernel: 5.10.0-32-2-amd64
        lldp/interfaces/interface[name=swp46]/neighbors/neighbor[id=1-Ethernet48]/state/system-name = netlab-mgmt0
        lldp/interfaces/interface[name=swp53]/neighbors/neighbor[id=6-98:03:9b:7f:6f:58]/state/port-id = 98:03:9b:7f:6f:58
        lldp/interfaces/interface[name=swp53]/neighbors/neighbor[id=7-98:03:9b:7f:6f:58]/state/management-address = 10.22.64.101
        lldp/interfaces/interface[name=swp53]/neighbors/neighbor[id=7-98:03:9b:7f:6f:58]/state/port-id = 98:03:9b:7f:6f:58
        lldp/interfaces/interface[name=swp53]/neighbors/neighbor[id=7-98:03:9b:7f:6f:58]/state/system-name = netlab-server
        lldp/interfaces/interface[name=swp55]/neighbors/neighbor[id=3-swp32]/state/management-address = 10.22.64.103
        lldp/interfaces/interface[name=swp55]/neighbors/neighbor[id=3-swp32]/state/port-id = swp32
        lldp/interfaces/interface[name=swp55]/neighbors/neighbor[id=3-swp32]/state/system-description = Arrcus Operating System (ArcOS)
        lldp/interfaces/interface[name=swp55]/neighbors/neighbor[id=3-swp32]/state/system-name = netlab-spine2
    ";

    fn arcos() -> ScriptedDevice {
        // `/lldp/state` is what leaf1 refuses: "Requested Path 'lldp/state' is not supported".
        ScriptedDevice::default()
            .serve(Subtree::InterfaceState, ARCOS_INTERFACE_STATE)
            .serve(Subtree::EthernetState, ARCOS_ETHERNET_STATE)
            .serve(Subtree::LldpNeighbors, ARCOS_LLDP_NEIGHBORS)
    }

    async fn rows(device: &mut ScriptedDevice) -> (Collection, Vec<Interface>) {
        let coll = collect(device).await.expect("collection succeeds");
        let rows = collection_to_interfaces(&coll, uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        (coll, rows)
    }

    fn row<'a>(rows: &'a [Interface], name: &str) -> &'a InterfaceBase {
        &rows
            .iter()
            .find(|i| i.base.if_name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("no row for {name}"))
            .base
    }

    /// The whole ArcOS shape: rows from `/interfaces` with real ifIndexes and types, the LLDP
    /// neighbour joined on name, `/lldp/state` refused without consequence.
    #[tokio::test]
    async fn arcos_rows_join_interfaces_and_lldp() {
        let (_coll, rows) = rows(&mut arcos()).await;
        assert_eq!(
            rows.len(),
            7,
            "one row per /interfaces entry, LLDP adds none"
        );

        let swp1 = row(&rows, "swp1");
        assert_eq!(swp1.if_index, 1001);
        assert_eq!(swp1.if_descr, "swp1");
        assert_eq!(swp1.if_alias, None, "a blank description leaf is no alias");
        assert_eq!(swp1.if_type, 6, "ethernetCsmacd");
        assert_eq!(swp1.admin_status, IfAdminStatus::Up);
        assert_eq!(swp1.oper_status, IfOperStatus::Up);
        assert_eq!(swp1.mac_address, None, "ArcOS serves no mac-address leaf");
        assert_eq!(
            swp1.speed_bps, None,
            "effective-speed is not the model's port-speed"
        );
        // Two entries for the same peer: the one with an address and a name is the one kept.
        assert_eq!(swp1.lldp_sys_name.as_deref(), Some("netlab-server"));
        assert_eq!(
            swp1.lldp_chassis_id,
            Some(LldpChassisId::NetworkAddress(
                "10.22.64.101".parse().unwrap()
            )),
            "no chassis-id leaf: the management address is the identity"
        );
        assert_eq!(
            swp1.lldp_port_id,
            Some(LldpPortId::MacAddress("34:80:0d:44:44:f5".into()))
        );

        assert_eq!(
            row(&rows, "swp46").if_alias.as_deref(),
            Some("netlab-mgmt0 : Ethernet48")
        );
        assert_eq!(row(&rows, "loopback0").if_type, 24, "softwareLoopback");
        // IANA 136, the number SNMP's ifType reports for the same SVI. Not `if_type::L3_IPVLAN`,
        // which is 137 (IANA's l3ipxvlan).
        assert_eq!(row(&rows, "vlan1000").if_type, 136, "l3ipvlan");
        assert_eq!(row(&rows, "ma1").if_index, 4);

        let swp55 = row(&rows, "swp55");
        assert_eq!(swp55.lldp_sys_name.as_deref(), Some("netlab-spine2"));
        assert_eq!(swp55.lldp_mgmt_addr, Some("10.22.64.103".parse().unwrap()));
        assert_eq!(
            swp55.lldp_sys_desc.as_deref(),
            Some("Arrcus Operating System (ArcOS)")
        );
        // A link-local management address still parses; whether it resolves is the server's
        // business.
        assert_eq!(
            row(&rows, "swp46").lldp_mgmt_addr,
            Some("fe80::deda:4dff:fe86:f4ea".parse().unwrap())
        );
    }

    /// A device serving LLDP but not `openconfig-interfaces` is an error naming the refused
    /// path, not a set of rows invented from neighbour names: such rows (no ifIndex, no
    /// statuses) would shadow a real ifTable when SNMP runs against the same device.
    #[tokio::test]
    async fn interfaces_refused_is_an_error_even_with_lldp_present() {
        let mut device =
            ScriptedDevice::default().serve(Subtree::LldpNeighbors, ARCOS_LLDP_NEIGHBORS);
        let err = collect(&mut device).await.expect_err("no /interfaces");
        let msg = format!("{err:#}");
        assert!(msg.contains("openconfig-interfaces is required"), "{msg}");
        assert!(
            msg.contains("'interfaces/interface[name=*]/state' is not supported"),
            "{msg}"
        );
    }

    /// A JSON_IETF blob — hand-built to the openconfig-interfaces model, since no device in
    /// the lab answers with one — flattens to the same leaves, module prefixes and all, list
    /// entries keyed by their `name`. Subinterfaces carry their own `ifindex` and are skipped.
    #[test]
    fn json_ietf_blob_flattens_to_the_same_leaves() {
        let blob = serde_json::json!({
            "openconfig-interfaces:interface": [{
                "name": "Ethernet1",
                "state": {
                    "ifindex": 1,
                    "type": "iana-if-type:ethernetCsmacd",
                    "admin-status": "UP",
                    "oper-status": "LOWER_LAYER_DOWN",
                    "description": "uplink"
                },
                "openconfig-if-ethernet:ethernet": {
                    "state": {
                        "mac-address": "c0:c9:89:ef:20:d2",
                        "port-speed": "openconfig-if-ethernet:SPEED_10GB"
                    }
                },
                "subinterfaces": {
                    "subinterface": [{ "index": 0, "state": { "ifindex": 5 } }]
                }
            }]
        });
        let n = Notification {
            update: vec![Update {
                path: Some(parse_path("interfaces")),
                val: Some(TypedValue {
                    value: Some(typed_value::Value::JsonIetfVal(
                        blob.to_string().into_bytes(),
                    )),
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut coll = Collection::default();
        absorb_notification(&mut coll, &n);
        let rows = collection_to_interfaces(&coll, uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        let eth1 = row(&rows, "Ethernet1");
        assert_eq!(
            eth1.if_index, 1,
            "the interface's ifindex, not the subinterface's"
        );
        assert_eq!(eth1.if_alias.as_deref(), Some("uplink"));
        assert_eq!(eth1.oper_status, IfOperStatus::LowerLayerDown);
        assert_eq!(
            eth1.mac_address.map(|m| m.to_string().to_lowercase()),
            Some("c0:c9:89:ef:20:d2".into())
        );
        assert_eq!(eth1.speed_bps, Some(10_000_000_000));
    }

    /// A device that DOES serve chassis-id/-type maps faithfully, notification prefix and
    /// `/lldp/state` included.
    #[test]
    fn explicit_chassis_type_maps_and_prefix_is_honoured() {
        let n = Notification {
            prefix: Some(parse_path("lldp")),
            update: [
                ("interfaces/interface[name=eth0]/neighbors/neighbor[id=1]/state/chassis-id", "C0:C9:89:EF:20:D2"),
                ("interfaces/interface[name=eth0]/neighbors/neighbor[id=1]/state/chassis-id-type", "openconfig-lldp-types:MAC_ADDRESS"),
                ("interfaces/interface[name=eth0]/neighbors/neighbor[id=1]/state/port-id", "Gi1/0/1"),
                ("interfaces/interface[name=eth0]/neighbors/neighbor[id=1]/state/port-id-type", "openconfig-lldp-types:INTERFACE_NAME"),
                ("state/chassis-id", "00:11:22:33:44:55"),
                ("state/chassis-id-type", "openconfig-lldp-types:MAC_ADDRESS"),
            ]
            .into_iter()
            .map(|(p, v)| Update {
                path: Some(parse_path(p)),
                val: Some(typed(v)),
                ..Default::default()
            })
            .collect(),
            ..Default::default()
        };
        let mut coll = Collection::default();
        absorb_notification(&mut coll, &n);
        // The row itself comes from `/interfaces`; the neighbour only decorates it.
        absorb_notification(
            &mut coll,
            &Notification {
                update: vec![Update {
                    path: Some(parse_path("interfaces/interface[name=eth0]/state/ifindex")),
                    val: Some(typed("3")),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        assert_eq!(coll.local_chassis_id.as_deref(), Some("00:11:22:33:44:55"));
        let rows = collection_to_interfaces(&coll, uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        let eth0 = row(&rows, "eth0");
        assert_eq!(eth0.if_index, 3);
        assert_eq!(
            eth0.lldp_chassis_id,
            Some(LldpChassisId::MacAddress("c0:c9:89:ef:20:d2".into()))
        );
        assert_eq!(
            eth0.lldp_port_id,
            Some(LldpPortId::InterfaceName("Gi1/0/1".into()))
        );
    }

    /// The generated reverse map itself, independent of identity handling.
    #[test]
    fn generated_reverse_map_matches_the_registry() {
        assert_eq!(get_if_type_number("ethernetCsmacd"), Some(6));
        assert_eq!(get_if_type_number("l2vlan"), Some(135));
        assert_eq!(get_if_type_number("notAnIfType"), None);
    }

    #[test]
    fn identities_map_by_iana_name_whatever_the_prefix() {
        assert_eq!(if_type_from_identity("ethernetCsmacd"), 6);
        assert_eq!(if_type_from_identity("iana-if-type:ieee8023adLag"), 161);
        assert_eq!(if_type_from_identity("ianaift:propVirtual"), 53);
        assert_eq!(if_type_from_identity("iana-if-type:l2vlan"), 135);
        assert_eq!(if_type_from_identity("iana-if-type:l3ipvlan"), 136);
        // Registered, just not one the constants name: the table covers it.
        assert_eq!(if_type_from_identity("iana-if-type:atm"), 37);
        assert_eq!(
            if_type_from_identity("iana-if-type:noSuchType"),
            if_type::OTHER
        );
        assert_eq!(speed_from_identity("SPEED_100MB"), Some(100_000_000));
        assert_eq!(
            speed_from_identity("openconfig-if-ethernet:SPEED_2500MB"),
            Some(2_500_000_000)
        );
        assert_eq!(speed_from_identity("SPEED_400GB"), Some(400_000_000_000));
        assert_eq!(speed_from_identity("SPEED_UNKNOWN"), None);
    }

    // Captured 2026-08-25 from a DriveNets DNOS 72XC with gnmic 0.47 (`subscribe --mode once
    // --encoding proto`, port 50051). DNOS serves `/interfaces` the same per-leaf way and
    // refuses every `/lldp` path ("No valid requests in the session"); its `type` leaves mix
    // IANA names with vendor ones (`irb`, `mgmt-ncx-member`).
    const DNOS_INTERFACE_STATE: &str = "
        interfaces/interface[name=ge10-0/0/0]/state/admin-status = UP
        interfaces/interface[name=ge10-0/0/0]/state/ifindex = 1
        interfaces/interface[name=ge10-0/0/0]/state/mtu = 1514
        interfaces/interface[name=ge10-0/0/0]/state/name = ge10-0/0/0
        interfaces/interface[name=ge10-0/0/0]/state/oper-status = DOWN
        interfaces/interface[name=ge10-0/0/0]/state/type = ethernetCsmacd
        interfaces/interface[name=bundle-10]/state/admin-status = UP
        interfaces/interface[name=bundle-10]/state/ifindex = 12289
        interfaces/interface[name=bundle-10]/state/oper-status = DOWN
        interfaces/interface[name=bundle-10]/state/type = ieee8023adLag
        interfaces/interface[name=bundle-10.4090]/state/admin-status = UP
        interfaces/interface[name=bundle-10.4090]/state/ifindex = 13313
        interfaces/interface[name=bundle-10.4090]/state/oper-status = DOWN
        interfaces/interface[name=bundle-10.4090]/state/type = l2vlan
        interfaces/interface[name=irb100]/state/admin-status = UP
        interfaces/interface[name=irb100]/state/ifindex = 41985
        interfaces/interface[name=irb100]/state/oper-status = DOWN
        interfaces/interface[name=irb100]/state/type = irb
        interfaces/interface[name=lo0]/state/admin-status = UP
        interfaces/interface[name=lo0]/state/description = loopback
        interfaces/interface[name=lo0]/state/ifindex = 8193
        interfaces/interface[name=lo0]/state/oper-status = UP
        interfaces/interface[name=lo0]/state/type = softwareLoopback
        interfaces/interface[name=mgmt-ncc-0/0]/state/admin-status = UP
        interfaces/interface[name=mgmt-ncc-0/0]/state/ifindex = 46333
        interfaces/interface[name=mgmt-ncc-0/0]/state/oper-status = UP
        interfaces/interface[name=mgmt-ncc-0/0]/state/type = mgmt-ncx-member
    ";

    /// A device with `openconfig-interfaces` and no LLDP model at all still yields an
    /// authoritative interface set, neighbourless; vendor-private types land as `other`.
    #[tokio::test]
    async fn dnos_interfaces_without_any_lldp_model() {
        let mut device =
            ScriptedDevice::default().serve(Subtree::InterfaceState, DNOS_INTERFACE_STATE);
        let (_coll, rows) = rows(&mut device).await;
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|r| r.base.lldp_chassis_id.is_none()));
        let ge = row(&rows, "ge10-0/0/0");
        assert_eq!(ge.if_index, 1);
        assert_eq!(ge.oper_status, IfOperStatus::Down);
        assert_eq!(row(&rows, "bundle-10").if_type, 161, "ieee8023adLag");
        assert_eq!(row(&rows, "bundle-10.4090").if_type, 135, "l2vlan");
        assert_eq!(row(&rows, "irb100").if_type, if_type::OTHER);
        assert_eq!(row(&rows, "mgmt-ncc-0/0").if_type, if_type::OTHER);
        assert_eq!(row(&rows, "lo0").if_alias.as_deref(), Some("loopback"));
    }
}
