//! gNMI (OpenConfig) discovery integration: interfaces and LLDP neighbours.
//!
//! For devices whose management plane is gNMI rather than SNMP — gNMI-first NOSes, and
//! fleets where enabling an SNMP agent is its own project (scanopy#690 has the survey). One
//! credential yields the interface rows an ifTable walk would, with the same LLDP neighbour
//! evidence the SNMP collector attaches, so the server-side L2 resolution runs unchanged.
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

mod parse;
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
use crate::server::interface_neighbors::r#impl::base::InterfaceNeighborEvidence;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase, InterfaceDataComplete};
use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::lldp::{LldpChassisId, canonical_mac};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use parse::{
    absorb_notification, admin_status, if_type_from_identity, map_chassis, map_port, oper_status,
    speed_from_identity, unqualified,
};
use proto::gnmi::{Path, PathElem};
use transport::{ConnectError, GnmiTransport, TonicTransport};

pub struct GnmiIntegration;

/// Working credential handed from probe to execute, together with the YANG models the device
/// advertised.
///
/// The model list is carried rather than fetched again: `probe` already completed a
/// `Capabilities` round trip — that is what it probes with — and which LLDP model to read is
/// decided from its answer. Asking a second time would be an extra RPC per scan on every gNMI
/// device.
///
/// What `probe` decides is [`handle_from`], covered by `advertised_models_reach_the_handle`;
/// the dial it wraps is not, which is the boundary the other collectors draw too.
struct GnmiProbeHandle {
    credential: GnmiQueryCredential,
    models: Vec<String>,
}

/// The handle a successful probe hands to `execute`: the credential that dialled, and the
/// models the device named in its `Capabilities` reply.
fn handle_from(credential: GnmiQueryCredential, models: Vec<String>) -> GnmiProbeHandle {
    GnmiProbeHandle { credential, models }
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

/// One neighbour's leaves as the evidence the server resolves. Remote identity, best evidence
/// first: an explicit chassis-id leaf; else the management address (resolves against
/// ip_addresses server-side); else a MAC-shaped port-id. ArcOS serves no chassis-id leaf at all,
/// so the fallbacks are what carry its neighbours.
impl From<&NeighborLeaves> for InterfaceNeighborEvidence {
    fn from(n: &NeighborLeaves) -> Self {
        let lldp_chassis_id = match &n.chassis_id {
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
        };
        Self {
            lldp_chassis_id,
            lldp_port_id: n
                .port_id
                .as_deref()
                .and_then(|id| map_port(id, n.port_id_type.as_deref())),
            lldp_sys_name: n.system_name.clone(),
            lldp_port_desc: n.port_description.clone(),
            lldp_mgmt_addr: n.management_address.as_deref().and_then(|a| a.parse().ok()),
            lldp_sys_desc: n.system_description.clone(),
            ..Default::default()
        }
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
    /// Whether `/lldp/interfaces` answered and every update in it parsed. Only then is a missing
    /// neighbour a vanished one. A refused, failed or garbled read leaves the stored neighbours
    /// in place, and a device refusing the path as unsupported is non-authoritative too: SNMP's
    /// rule, `complete && !unsupported`.
    pub lldp_complete: bool,
    /// Which LLDP model these neighbours actually came from. Reported at info: when an edge is
    /// missing or wrong, "which model produced this" is the first thing worth knowing, and a
    /// line that says openconfig while a vendor model was substituted is worse than no line.
    pub lldp_model: LldpModel,
}

impl Collection {
    /// Which per-interface groups this collection read in full. The interface set itself is
    /// always authoritative (`collect` fails without `/interfaces`); LLDP only when its read was.
    pub(crate) fn data_complete(&self) -> InterfaceDataComplete {
        InterfaceDataComplete {
            lldp: self.lldp_complete,
            cdp: false,
            fdb: false,
            vlan_membership: false,
        }
    }
}

/// Which LLDP model a collection read, as far as the device's own `Capabilities` says.
///
/// Not a bare model name: "this device says it serves openconfig-lldp" and "this device named
/// no LLDP model and openconfig was read regardless" are different facts, and an operator
/// chasing a missing edge needs to tell them apart. The unreadable-`Capabilities` case is not
/// here because it cannot reach a collection — `probe` fails the host on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LldpModel {
    /// The device advertised this model, and it is the one that was read.
    Advertised(&'static str),
    /// The device advertised no LLDP model this collector has a profile for — including a
    /// device that advertises none at all. [`OPENCONFIG_LLDP`] is read anyway: that is what
    /// this collector asked every device before it asked at all, and a device may serve a model
    /// it does not advertise.
    #[default]
    NoneAdvertised,
}

impl std::fmt::Display for LldpModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Advertised(module) => f.write_str(module),
            Self::NoneAdvertised => f.write_str("none advertised"),
        }
    }
}

/// One Subscribe: a path, and the origin whose schema tree it is rooted in.
///
/// Wildcard keys rather than bare list elements: the spec treats both as "every entry", but
/// `[name=*]` is the form every implementation has been exercised with (it is what gnmic sends).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Subtree {
    /// gNMI `origin` (`Path.origin`): which schema tree the path is rooted in. Empty means the
    /// device's default tree, where openconfig is served on every device this collector has
    /// been run against.
    ///
    /// A vendor's own tree conventionally needs its own origin — SR Linux `srl_nokia`, IOS-XR
    /// `Cisco-IOS-XR-*` — so it belongs with the path rather than being assumed away. DriveNets
    /// answers its native tree under the empty origin, which is why both profiles below set
    /// none; the field exists so the next vendor is a profile and not a rewrite.
    pub origin: &'static str,
    /// Path elements, each `name` or `name[key=value]`.
    pub elems: &'static [&'static str],
}

impl Subtree {
    /// `/interfaces/interface[name=*]/state`: the rows. Required — its failure aborts the
    /// collection.
    const INTERFACE_STATE: Self =
        Self::default_origin(&["interfaces", "interface[name=*]", "state"]);
    /// `/interfaces/interface[name=*]/ethernet/state`: MAC and port speed, where served.
    /// Optional: its failure is a debug-level skip, because ArcOS serves no `mac-address` at
    /// all (see the module header).
    const ETHERNET_STATE: Self =
        Self::default_origin(&["interfaces", "interface[name=*]", "ethernet", "state"]);

    /// Always asked for, whichever LLDP model the device turns out to have.
    const BASE: [Subtree; 2] = [Self::INTERFACE_STATE, Self::ETHERNET_STATE];

    /// A path in the device's default schema tree — openconfig, and anything a vendor serves
    /// without demanding an origin of its own.
    const fn default_origin(elems: &'static [&'static str]) -> Self {
        Self { origin: "", elems }
    }

    pub(crate) fn path(self) -> Path {
        Path {
            origin: self.origin.to_string(),
            elem: self
                .elems
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

/// Which LLDP YANG model a device serves, and what it takes to read it.
///
/// `openconfig-lldp` is not the only LLDP model in the field. DriveNets NOSes advertise
/// `dn-lldp`, do not list `openconfig-lldp` in `Capabilities` at all, and refuse every
/// openconfig `/lldp` path with `InvalidArgument` — so a DriveNets router contributed
/// interfaces and never a neighbour. (The refusal's *message* varies: a 2026-08-30 capture
/// recorded "Path does not exist: /lldp", while the same NOS version on 2026-09-15 answered
/// "No valid requests in the session". Match on the model list, never on that text.) Below its own root that tree is
/// openconfig-SHAPED — the same list keys (`interface[name=*]`, `neighbor[id=*]`) and the same
/// leaf names (`system-name`, `chassis-id`, `port-id`, …) — so what differs between models is
/// named here rather than parsed twice. One routing table in [`absorb_leaf`] means a vendor
/// tree cannot drift away from the openconfig one it mirrors, and a further vendor is a static
/// below rather than a branch in three places.
///
/// Everything from the state container down is identical across profiles, so nothing after the
/// walk — `LldpChassisId`, `LldpPortId`, `collection_to_interfaces` — varies by profile.
pub(crate) struct LldpModelProfile {
    /// WHAT IS NOT IN HERE, so the next person knows which vendors the table absorbs: the list
    /// KEY names (`interface[name=…]`, `neighbor[id=…]`) and the literal `lldp` element are
    /// still fixed in `absorb_leaf` and `normalised_names`. A vendor openconfig-SHAPED except
    /// for those needs code, not a static. `LldpMibProfile` draws the same line where it notes
    /// the subtype enums are identical across MIB revisions.
    ///
    /// The YANG module name the device advertises in `Capabilities`, which is what selects this
    /// profile. Compared unqualified, as every model name here is.
    pub module: &'static str,
    /// The subtrees to read for this model, each its own Subscribe: a device refuses a whole
    /// request over one path it does not serve, and which paths those are varies by device.
    pub subtrees: &'static [Subtree],
    /// Path elements above the model's `lldp` root, stripped so what remains matches
    /// openconfig's shape. Empty for a model already rooted at `/lldp`.
    pub root: &'static [&'static str],
    /// What this model calls the container openconfig calls `state`, rewritten to `state` on
    /// the way in. Scoped to this model's own LLDP tree — see [`normalised_names`] for what
    /// happens when it is not.
    pub state_container: &'static str,
    /// The subtree within [`Self::subtrees`] whose read decides whether the neighbour set is
    /// authoritative. Not all of them: `/lldp/state` carries the device's own chassis identity
    /// rather than its neighbours, and ArcOS refuses that path outright while serving a complete
    /// neighbour list — folding it in would leave a real device non-authoritative on every scan,
    /// its neighbours never ageing out. DriveNets serves identity and neighbours together in one
    /// subtree, so for that profile this IS that subtree; a field rather than an index because
    /// which one it is, is the model's business.
    ///
    /// The separation this buys is clean for openconfig, where identity and neighbours are two
    /// Subscribes — not for DriveNets, whose one subtree carries both. A DriveNets device whose
    /// `oper-items/chassis-id` or `system-name` leaves fail to parse is marked non-authoritative
    /// too, even though only its own identity was at fault, not its neighbour list. That errs
    /// conservative — a false "not authoritative" costs nothing but a scan's worth of pruning,
    /// where the reverse silently drops real neighbours — so it is left as is.
    ///
    /// INVARIANT, unenforced by the type: `subtrees.contains(&neighbors)` must hold. Nothing
    /// checks the two literals agree; see `every_profile_names_a_neighbours_subtree_it_actually_reads`
    /// in `tests.rs`. If they drift apart, `is_lldp` is never true and `lldp_complete` stays
    /// `false` in [`collect`]: every device on this profile reports non-authoritative, and its
    /// neighbours stop ageing out until the two literals are put back in step.
    pub neighbors: Subtree,
}

/// `openconfig-lldp`, rooted at `/lldp`: no root to strip and `state` already named `state`, so
/// the normalisation below is the identity for it.
pub(crate) static OPENCONFIG_LLDP: LldpModelProfile = LldpModelProfile {
    module: "openconfig-lldp",
    subtrees: &[
        // `/lldp/state`: the device's own chassis id, where served — ArcOS refuses this path.
        Subtree::default_origin(&["lldp", "state"]),
        // `/lldp/interfaces/interface[name=*]`: the neighbours.
        Subtree::default_origin(&["lldp", "interfaces", "interface[name=*]"]),
    ],
    root: &[],
    state_container: "state",
    neighbors: Subtree::default_origin(&["lldp", "interfaces", "interface[name=*]"]),
};

/// `dn-lldp`, DriveNets' native LLDP under `/drivenets-top/protocols/lldp`.
///
/// One subtree rather than two because the native tree is small and cDNOS 26.2 answers the
/// parent in a single Subscribe, local identity and neighbours together.
pub(crate) static DN_LLDP: LldpModelProfile = LldpModelProfile {
    module: "dn-lldp",
    subtrees: &[Subtree::default_origin(&[
        "drivenets-top",
        "protocols",
        "lldp",
    ])],
    root: &["drivenets-top", "protocols"],
    state_container: "oper-items",
    // Identity and neighbours travel together in this one subtree, so it is both.
    neighbors: Subtree::default_origin(&["drivenets-top", "protocols", "lldp"]),
};

impl LldpModelProfile {
    /// Every model this collector can read, in preference order: a device advertising both is
    /// read over openconfig, the model this collector was built against.
    const KNOWN: [&'static LldpModelProfile; 2] = [&OPENCONFIG_LLDP, &DN_LLDP];

    /// The profile for a device, chosen from what it ADVERTISES rather than by trying one model
    /// and catching the failure — which would cost a wasted round trip per scan and still could
    /// not tell "not served" from "served and empty", the silent kind of wrong.
    ///
    /// `None` is the device that advertises no LLDP model this collector knows, including one
    /// that advertises none at all. The caller falls back to [`OPENCONFIG_LLDP`] there, so a
    /// device that advertises nothing behaves exactly as it did before this existed.
    fn select(models: &[String]) -> Option<&'static LldpModelProfile> {
        Self::KNOWN
            .into_iter()
            .find(|p| models.iter().any(|m| unqualified(m) == p.module))
    }
}

/// Build the interface rows a collection amounts to: one per `/interfaces` entry, with every
/// LLDP neighbour heard on the same name folded in. A neighbour on a name `/interfaces` did not
/// list has no row to live on and is dropped.
pub(crate) fn collection_to_interfaces(
    coll: &Collection,
    host_id: uuid::Uuid,
    network_id: uuid::Uuid,
) -> Vec<Interface> {
    // Every neighbour a port hears is its own evidence entry, as the SNMP remote table and the
    // lldpd reader produce them (GH #701); the server groups entries by the host they resolve
    // to. ArcOS lists a Linux lldpd peer twice on a port, one entry carrying no management
    // address or system name, and both are sent.
    let mut neighbors_by_port: BTreeMap<&str, Vec<InterfaceNeighborEvidence>> = BTreeMap::new();
    for ((ifname, _), leaves) in &coll.neighbors {
        neighbors_by_port
            .entry(ifname)
            .or_default()
            .push(leaves.into());
    }
    coll.interfaces
        .iter()
        .map(|(name, i)| {
            let name = name.as_str();
            let neighbor_candidates = neighbors_by_port.remove(name).unwrap_or_default();
            Interface::new(InterfaceBase {
                host_id,
                network_id,
                if_index: i.ifindex.and_then(|x| i32::try_from(x).ok()),
                // ifDescr is the interface name on every NOS that matters; the operator's
                // text is ifAlias, which is what `description` is in openconfig.
                if_descr: Some(name.to_string()),
                if_name: Some(name.to_string()),
                if_alias: i.description.clone().filter(|d| !d.is_empty()),
                // No `type` leaf is unknown, not "Ethernet".
                if_type: i.if_type.as_deref().map(if_type_from_identity),
                speed_bps: i.port_speed.as_deref().and_then(speed_from_identity),
                admin_status: admin_status(i.admin_status.as_deref()),
                oper_status: oper_status(i.oper_status.as_deref()),
                mac_address: i
                    .mac_address
                    .as_deref()
                    .and_then(|m| m.parse::<MacAddress>().ok())
                    .map(|m| {
                        MacEvidence::new(
                            MacEvidenceValue(m),
                            AttributeSource::Probe(ClientProbe::Gnmi),
                        )
                    }),
                neighbor_candidates,
                ..Default::default()
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
///
/// `models` is the YANG module list `probe` already obtained (see [`GnmiProbeHandle`]); it
/// selects which LLDP profile to read.
pub(crate) async fn collect(
    transport: &mut dyn GnmiTransport,
    models: &[String],
) -> anyhow::Result<Collection> {
    let selected = LldpModelProfile::select(models);
    if selected.is_none() {
        tracing::debug!(
            advertised_models = models.len(),
            "gNMI device advertises no LLDP model this collector has a profile for; reading \
             openconfig-lldp regardless"
        );
    }
    let lldp_model = match selected {
        Some(profile) => LldpModel::Advertised(profile.module),
        None => LldpModel::NoneAdvertised,
    };
    collect_profile(transport, selected.unwrap_or(&OPENCONFIG_LLDP), lldp_model).await
}

/// [`collect`] once the profile is settled. Split out so a profile the selector cannot return —
/// one whose `neighbors` names a subtree its own `subtrees` omit — can be driven from a test.
async fn collect_profile(
    transport: &mut dyn GnmiTransport,
    profile: &LldpModelProfile,
    lldp_model: LldpModel,
) -> anyhow::Result<Collection> {
    let mut coll = Collection {
        lldp_model,
        ..Default::default()
    };
    // Only the profile's `neighbors` subtree gates authority, not every subtree it names:
    // `/lldp/state` is the device's own identity, and ArcOS refuses that path outright while
    // serving a complete neighbour list, so folding it in would leave a real device
    // non-authoritative on every scan. DriveNets serves identity and neighbours in ONE subtree,
    // so for that profile `neighbors` names that same subtree, and keying on a single FIXED
    // variant (as the openconfig-only version did) would leave a device that answered perfectly
    // reporting an incomplete read forever.
    //
    // False until that subtree is read and parsed, so authority follows a read that happened.
    // A profile whose `neighbors` matches none of its `subtrees` never sets it and the device
    // reports non-authoritative, which costs a scan's worth of pruning; the reverse would delete
    // stored neighbours on the strength of a read that never ran.
    let mut lldp_complete = false;
    for subtree in Subtree::BASE.iter().chain(profile.subtrees).copied() {
        let is_lldp = subtree == profile.neighbors;
        match transport.subscribe_once(vec![subtree.path()]).await {
            Ok(notifications) => {
                let mut parsed = true;
                for n in &notifications {
                    parsed &= absorb_notification(&mut coll, profile, n);
                }
                if is_lldp {
                    lldp_complete = parsed;
                }
                if !parsed {
                    tracing::debug!(?subtree, "gNMI subtree had updates that did not parse");
                }
            }
            Err(e) if subtree == Subtree::INTERFACE_STATE => {
                return Err(e.context("openconfig-interfaces is required and was not served"));
            }
            Err(e) => {
                tracing::debug!(?subtree, error = %e, "gNMI subtree not served; continuing");
            }
        }
    }
    coll.lldp_complete = lldp_complete;
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
        let models = transport
            .capabilities()
            .await
            .map_err(|e| ProbeFailure::rejected(e.to_string()))?;
        Ok(ProbeSuccess {
            client_probe: ClientProbe::Gnmi,
            ports: vec![PortType::new_tcp(cred.port)],
            handle: Some(Box::new(handle_from(cred.clone(), models))),
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
        let coll = collect(&mut transport, &handle.models).await?;

        let interfaces =
            collection_to_interfaces(&coll, ctx.host_id, host_data.host.base.network_id);
        tracing::info!(
            ip = %ctx.ip,
            interfaces = coll.interfaces.len(),
            neighbors = coll.neighbors.len(),
            lldp_model = %coll.lldp_model,
            "gNMI openconfig-interfaces/lldp collection complete"
        );
        if let Some(chassis) = coll
            .local_chassis_id
            .as_deref()
            .and_then(|id| map_chassis(id, coll.local_chassis_id_type.as_deref()))
        {
            host_data.with_chassis_id(
                chassis.identifier(),
                AttributeSource::Probe(ClientProbe::Gnmi),
            );
        }
        host_data.contribute_interfaces(
            ctx.interface_source,
            interfaces,
            // `/interfaces` answered or `collect` would have failed: the set is the device's
            // own full account, so the server may prune what is no longer in it.
            true,
            coll.data_complete(),
        );
        Ok(Completeness::Complete)
    }
}

#[cfg(test)]
mod tests;
