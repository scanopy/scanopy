use super::parse::{Leaf, normalised_names};
use super::*;
use crate::server::interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, if_type};
use crate::server::lldp::LldpPortId;
use crate::server::snmp::generated::get_if_type_number;
use proto::gnmi::{Notification, TypedValue, Update, typed_value};

/// A device answering Subscribe ONCE per subtree from scripts of `path = value` lines, the
/// way ArcOS does: one typed leaf per update, no prefix. A subtree with no script is
/// refused with the `InvalidArgument` a real device sends. Paths are gnmic-style
/// (`lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=1]/state/port-id`).
#[derive(Default)]
struct ScriptedDevice {
    served: BTreeMap<String, &'static str>,
    /// Subtrees whose Subscribe fails with this error rather than a refusal.
    failures: BTreeMap<String, &'static str>,
    models: Vec<String>,
}

impl ScriptedDevice {
    fn serve(mut self, subtree: Subtree, script: &'static str) -> Self {
        self.served.insert(subtree_key(subtree), script);
        self
    }

    fn fail(mut self, subtree: Subtree, error: &'static str) -> Self {
        self.failures.insert(subtree_key(subtree), error);
        self
    }

    /// The YANG modules this device names in its `Capabilities` reply.
    fn advertising(mut self, models: &[&str]) -> Self {
        self.models = models.iter().map(|m| m.to_string()).collect();
        self
    }
}

fn subtree_key(subtree: Subtree) -> String {
    format!("{}:{}", subtree.origin, subtree.elems.join("/"))
}

fn render_path(path: &Path) -> String {
    let elems = path
        .elem
        .iter()
        .map(|e| {
            let keys: String = e.key.iter().map(|(k, v)| format!("[{k}={v}]")).collect();
            format!("{}{keys}", e.name)
        })
        .collect::<Vec<_>>()
        .join("/");
    format!("{}:{}", path.origin, elems)
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
    async fn capabilities(&mut self) -> anyhow::Result<Vec<String>> {
        Ok(self.models.clone())
    }
    async fn subscribe_once(&mut self, paths: Vec<Path>) -> anyhow::Result<Vec<Notification>> {
        let [path] = paths.as_slice() else {
            panic!("the collector subscribes one subtree at a time");
        };
        let key = render_path(path);
        if let Some(error) = self.failures.get(key.as_str()) {
            anyhow::bail!("{error}");
        }
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

/// No `chassis-id` leaf on any neighbour, so the remote identity comes from the
/// management address or a MAC-shaped port-id.
///
/// `swp53` hears one peer twice: both entries carry port-id `98:03:9b:7f:6f:58`, which is
/// netlab-server's `ens1f0np0`, and only one of them also carries the management address and
/// system name. That is a double listing — the neighbor id's `6-`/`7-` prefix is the LLDP
/// port-id *subtype* (agent circuit id vs. locally assigned), so the device is advertising the
/// same value under two different subtypes rather than reporting two distinct peers.
///
/// `swp1` is NOT that, though an earlier version of this comment said it was. Its two entries
/// carry DIFFERENT port-ids: `34:80:0d:44:44:f5` is netlab-server's `eno2` (confirmed from the
/// far end -- `lldpcli` on eno2 reports netlab-leaf1:swp1), while `34:80:0d:44:45:05` is not
/// netlab-server at all -- not one of its NICs, not its chassis id (`34:80:0d:44:44:f4`, eno1),
/// and absent from the management network's ARP and FDB when checked on 2026-09-15. So swp1
/// heard two different devices: a shared segment, which is GH #701's own case, captured here
/// by accident on 2026-08-30 and mislabelled until now.
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
        .serve(Subtree::INTERFACE_STATE, ARCOS_INTERFACE_STATE)
        .serve(Subtree::ETHERNET_STATE, ARCOS_ETHERNET_STATE)
        .serve(OPENCONFIG_LLDP.subtrees[1], ARCOS_LLDP_NEIGHBORS)
}

async fn rows(device: &mut ScriptedDevice) -> (Collection, Vec<Interface>) {
    let models = device.capabilities().await.expect("capabilities");
    let coll = collect(device, &models).await.expect("collection succeeds");
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

/// The one LLDP neighbour a row carries.
fn lldp(base: &InterfaceBase) -> &InterfaceNeighborEvidence {
    base.neighbor_candidates
        .first()
        .unwrap_or_else(|| panic!("no neighbour on {:?}", base.if_name))
}

/// The whole ArcOS shape: rows from `/interfaces` with real ifIndexes and types, the LLDP
/// neighbour joined on name, `/lldp/state` refused without consequence.
#[tokio::test]
async fn arcos_rows_join_interfaces_and_lldp() {
    let (coll, rows) = rows(&mut arcos()).await;
    assert!(
        coll.data_complete().lldp,
        "/lldp answered: its neighbour set is authoritative"
    );
    assert_eq!(
        rows.len(),
        7,
        "one row per /interfaces entry, LLDP adds none"
    );

    let swp1 = row(&rows, "swp1");
    assert_eq!(swp1.if_index, Some(1001));
    assert_eq!(swp1.if_descr.as_deref(), Some("swp1"));
    assert_eq!(swp1.if_alias, None, "a blank description leaf is no alias");
    assert_eq!(swp1.if_type, Some(6), "ethernetCsmacd");
    assert_eq!(swp1.admin_status, Some(IfAdminStatus::Up));
    assert_eq!(swp1.oper_status, Some(IfOperStatus::Up));
    assert_eq!(swp1.mac_address, None, "ArcOS serves no mac-address leaf");
    assert_eq!(
        swp1.speed_bps, None,
        "effective-speed is not the model's port-speed"
    );
    // Two entries for the same peer, both kept. Neither carries a chassis-id leaf: the thin
    // one is identified by its MAC-shaped port-id, the other by its management address.
    let [thin, rich] = swp1.neighbor_candidates.as_slice() else {
        panic!(
            "every neighbour on the port, got {:?}",
            swp1.neighbor_candidates
        );
    };
    assert_eq!(
        thin.lldp_chassis_id,
        Some(LldpChassisId::MacAddress("34:80:0d:44:45:05".into())),
        "no chassis-id leaf and no address: the MAC-shaped port-id is the identity"
    );
    assert_eq!(thin.lldp_sys_name, None);
    assert_eq!(thin.lldp_mgmt_addr, None);
    assert_eq!(rich.lldp_sys_name.as_deref(), Some("netlab-server"));
    assert_eq!(
        rich.lldp_chassis_id,
        Some(LldpChassisId::NetworkAddress(
            "10.22.64.101".parse().unwrap()
        )),
        "no chassis-id leaf: the management address is the identity"
    );
    assert_eq!(
        rich.lldp_port_id,
        Some(LldpPortId::MacAddress("34:80:0d:44:44:f5".into()))
    );
    assert_eq!(
        row(&rows, "swp53").neighbor_candidates.len(),
        2,
        "the same double listing on swp53"
    );

    assert_eq!(
        row(&rows, "swp46").if_alias.as_deref(),
        Some("netlab-mgmt0 : Ethernet48")
    );
    assert_eq!(
        row(&rows, "loopback0").if_type,
        Some(24),
        "softwareLoopback"
    );
    // IANA 136, the number SNMP's ifType reports for the same SVI. Not `if_type::L3_IPVLAN`,
    // which is 137 (IANA's l3ipxvlan).
    assert_eq!(row(&rows, "vlan1000").if_type, Some(136), "l3ipvlan");
    assert_eq!(row(&rows, "ma1").if_index, Some(4));

    let swp55 = lldp(row(&rows, "swp55"));
    assert_eq!(swp55.lldp_sys_name.as_deref(), Some("netlab-spine2"));
    assert_eq!(swp55.lldp_mgmt_addr, Some("10.22.64.103".parse().unwrap()));
    assert_eq!(
        swp55.lldp_sys_desc.as_deref(),
        Some("Arrcus Operating System (ArcOS)")
    );
    // A link-local management address still parses; whether it resolves is the server's
    // business.
    assert_eq!(
        lldp(row(&rows, "swp46")).lldp_mgmt_addr,
        Some("fe80::deda:4dff:fe86:f4ea".parse().unwrap())
    );
}

/// A device whose LLDP read fails, refused or timed out, still yields its rows but not an
/// authoritative neighbour set, so the server keeps the neighbours it holds instead of
/// clearing them on one bad read. Both LLDP models get this: openconfig's dedicated neighbours
/// subtree refusing or timing out, and DriveNets' single combined subtree doing the same.
#[tokio::test]
async fn failed_lldp_read_keeps_rows_and_is_not_authoritative() {
    let refused = ScriptedDevice::default()
        .serve(Subtree::INTERFACE_STATE, ARCOS_INTERFACE_STATE)
        .serve(Subtree::ETHERNET_STATE, ARCOS_ETHERNET_STATE);
    let timed_out = ScriptedDevice::default()
        .serve(Subtree::INTERFACE_STATE, ARCOS_INTERFACE_STATE)
        .serve(Subtree::ETHERNET_STATE, ARCOS_ETHERNET_STATE)
        .fail(
            OPENCONFIG_LLDP.subtrees[1],
            "gNMI Subscribe stream timed out",
        );
    let native_failed = ScriptedDevice::default()
        .serve(Subtree::INTERFACE_STATE, CDNOS_INTERFACE_STATE)
        .fail(DN_LLDP.subtrees[0], "gNMI Subscribe stream timed out")
        .advertising(&["openconfig-interfaces", "dn-lldp"]);
    for (case, mut device, expected_rows) in [
        ("refused", refused, 7),
        ("timed out", timed_out, 7),
        ("native tree failed", native_failed, 2),
    ] {
        let (coll, rows) = rows(&mut device).await;
        assert!(
            !coll.data_complete().lldp,
            "{case}: lldp must not be authoritative"
        );
        assert_eq!(
            rows.len(),
            expected_rows,
            "{case}: interface rows come through without LLDP"
        );
        assert!(
            rows.iter().all(|r| r.base.neighbor_candidates.is_empty()),
            "{case}"
        );
    }
}

/// An update whose JSON blob does not parse is reported, so a garbled LLDP subtree counts as
/// a failed read rather than an empty one.
#[test]
fn unparseable_json_update_is_reported() {
    let n = Notification {
        update: vec![Update {
            path: Some(parse_path("lldp")),
            val: Some(TypedValue {
                value: Some(typed_value::Value::JsonIetfVal(b"{not json".to_vec())),
            }),
            ..Default::default()
        }],
        ..Default::default()
    };
    assert!(!absorb_notification(
        &mut Collection::default(),
        &OPENCONFIG_LLDP,
        &n
    ));
}

/// A device serving LLDP but not `openconfig-interfaces` is an error naming the refused
/// path, not a set of rows invented from neighbour names: such rows (no ifIndex, no
/// statuses) would shadow a real ifTable when SNMP runs against the same device.
#[tokio::test]
async fn interfaces_refused_is_an_error_even_with_lldp_present() {
    let mut device =
        ScriptedDevice::default().serve(OPENCONFIG_LLDP.subtrees[1], ARCOS_LLDP_NEIGHBORS);
    let err = collect(&mut device, &[]).await.expect_err("no /interfaces");
    let msg = format!("{err:#}");
    assert!(msg.contains("openconfig-interfaces is required"), "{msg}");
    assert!(
        msg.contains("':interfaces/interface[name=*]/state' is not supported"),
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
    absorb_notification(&mut coll, &OPENCONFIG_LLDP, &n);
    let rows = collection_to_interfaces(&coll, uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let eth1 = row(&rows, "Ethernet1");
    assert_eq!(
        eth1.if_index,
        Some(1),
        "the interface's ifindex, not the subinterface's"
    );
    assert_eq!(eth1.if_alias.as_deref(), Some("uplink"));
    assert_eq!(eth1.oper_status, Some(IfOperStatus::LowerLayerDown));
    assert_eq!(
        crate::server::ip_addresses::r#impl::base::mac_of(&eth1.mac_address)
            .map(|m| m.to_string().to_lowercase()),
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
            (
                "interfaces/interface[name=eth0]/neighbors/neighbor[id=1]/state/chassis-id",
                "C0:C9:89:EF:20:D2",
            ),
            (
                "interfaces/interface[name=eth0]/neighbors/neighbor[id=1]/state/chassis-id-type",
                "openconfig-lldp-types:MAC_ADDRESS",
            ),
            (
                "interfaces/interface[name=eth0]/neighbors/neighbor[id=1]/state/port-id",
                "Gi1/0/1",
            ),
            (
                "interfaces/interface[name=eth0]/neighbors/neighbor[id=1]/state/port-id-type",
                "openconfig-lldp-types:INTERFACE_NAME",
            ),
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
    absorb_notification(&mut coll, &OPENCONFIG_LLDP, &n);
    // The row itself comes from `/interfaces`; the neighbour only decorates it.
    absorb_notification(
        &mut coll,
        &OPENCONFIG_LLDP,
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
    assert_eq!(eth0.if_index, Some(3));
    assert_eq!(
        lldp(eth0).lldp_chassis_id,
        Some(LldpChassisId::MacAddress("c0:c9:89:ef:20:d2".into()))
    );
    assert_eq!(
        lldp(eth0).lldp_port_id,
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
/// authoritative interface set, neighbourless; vendor-private types land as `other`. Its
/// neighbour set is not authoritative: `/lldp` refused as unsupported is SNMP's
/// `unsupported`, which never clears.
#[tokio::test]
async fn dnos_interfaces_without_any_lldp_model() {
    let mut device =
        ScriptedDevice::default().serve(Subtree::INTERFACE_STATE, DNOS_INTERFACE_STATE);
    let (coll, rows) = rows(&mut device).await;
    assert!(!coll.data_complete().lldp);
    assert_eq!(rows.len(), 6);
    assert!(rows.iter().all(|r| r.base.neighbor_candidates.is_empty()));
    let ge = row(&rows, "ge10-0/0/0");
    assert_eq!(ge.if_index, Some(1));
    assert_eq!(ge.oper_status, Some(IfOperStatus::Down));
    assert_eq!(row(&rows, "bundle-10").if_type, Some(161), "ieee8023adLag");
    assert_eq!(row(&rows, "bundle-10.4090").if_type, Some(135), "l2vlan");
    assert_eq!(row(&rows, "irb100").if_type, Some(if_type::OTHER));
    assert_eq!(row(&rows, "mgmt-ncc-0/0").if_type, Some(if_type::OTHER));
    assert_eq!(row(&rows, "lo0").if_alias.as_deref(), Some("loopback"));
}

// Captured 2026-08-30 from clab-ml-20-edge-dnos (DriveNets cDNOS 26.2), Subscribe ONCE,
// PROTO encoding, via gnmic. DNOS serves NO openconfig-lldp -- it is absent from Capabilities
// and every openconfig `/lldp` path is refused with InvalidArgument -- and puts LLDP under its
// own model instead. Verbatim except for trimming to the two ports that have neighbours.
//
// The refusal message recorded at capture time was "Path does not exist: /lldp"; re-checked
// against the same NOS version on 2026-09-15 it was "No valid requests in the session". The
// collector keys off the advertised model list, not this string, which is why that drift is
// harmless -- but do not turn either message into an assertion.
const CDNOS_INTERFACE_STATE: &str = "
    interfaces/interface[name=ge100-0/0/1]/state/ifindex = 2
    interfaces/interface[name=ge100-0/0/1]/state/type = ethernetCsmacd
    interfaces/interface[name=ge100-0/0/1]/state/admin-status = UP
    interfaces/interface[name=ge100-0/0/1]/state/oper-status = UP
    interfaces/interface[name=ge100-0/0/1]/state/description = edge-dnos -> core2
    interfaces/interface[name=ge100-0/0/2]/state/ifindex = 3
    interfaces/interface[name=ge100-0/0/2]/state/type = ethernetCsmacd
    interfaces/interface[name=ge100-0/0/2]/state/admin-status = UP
    interfaces/interface[name=ge100-0/0/2]/state/oper-status = UP
    interfaces/interface[name=ge100-0/0/2]/state/description = edge-dnos -> [mcast-src,mcast-rcv]
";

// The same device's LLDP, under `drivenets-top`. Note `oper-items` where openconfig writes
// `state`, and that the list keys are IDENTICAL to openconfig's -- which is what lets one
// routing table read both.
const DNOS_LLDP_NATIVE: &str = "
    drivenets-top/protocols/lldp/oper-items/chassis-id = 84:40:76:56:95:25
    drivenets-top/protocols/lldp/oper-items/chassis-id-type = MAC_ADDRESS
    drivenets-top/protocols/lldp/oper-items/system-name = edge-dnos
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/1]/name = ge100-0/0/1
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/1]/neighbors/neighbor[id=0]/oper-items/chassis-id = aa:c1:ab:1a:bf:7a
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/1]/neighbors/neighbor[id=0]/oper-items/chassis-id-type = MAC_ADDRESS
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/1]/neighbors/neighbor[id=0]/oper-items/port-id = eth3
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/1]/neighbors/neighbor[id=0]/oper-items/port-id-type = INTERFACE_NAME
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/1]/neighbors/neighbor[id=0]/oper-items/system-name = core2
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/1]/oper-items/counters/lldp-in-pkts = 1239
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/2]/neighbors/neighbor[id=0]/oper-items/chassis-id = aa:c1:ab:1f:3b:e8
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/2]/neighbors/neighbor[id=0]/oper-items/port-id = eth1
    drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/2]/neighbors/neighbor[id=0]/oper-items/system-name = mcast-src
";

fn cdnos() -> ScriptedDevice {
    ScriptedDevice::default()
        .serve(Subtree::INTERFACE_STATE, CDNOS_INTERFACE_STATE)
        .serve(DN_LLDP.subtrees[0], DNOS_LLDP_NATIVE)
}

/// The design decision this module settles: `lldp_complete` is true only when the profile's
/// `neighbors` subtree answered and parsed. DriveNets serves local identity and neighbours in
/// ONE subtree, so a DriveNets router that answers its native tree perfectly must be
/// authoritative -- keying this on a FIXED variant (as the openconfig-only version did, always
/// checking the openconfig neighbours path) would leave it reporting an incomplete read
/// forever, and its neighbours would never age out.
#[tokio::test]
async fn a_drivenets_device_reading_its_native_tree_is_authoritative() {
    let mut device = cdnos().advertising(&["openconfig-interfaces", "dn-lldp", "dn-interfaces"]);
    let models = device.capabilities().await.expect("capabilities");
    let coll = collect(&mut device, &models).await.expect("collection");

    assert_eq!(coll.lldp_model, LldpModel::Advertised("dn-lldp"));
    assert!(
        coll.data_complete().lldp,
        "the native tree answered and parsed, so its neighbour set is authoritative"
    );
    assert!(
        !coll.neighbors.is_empty(),
        "the native tree yields neighbours"
    );
}

#[tokio::test]
async fn a_device_advertising_no_known_model_reads_openconfig_and_says_so() {
    let mut device = arcos().advertising(&["openconfig-interfaces"]);
    let models = device.capabilities().await.expect("capabilities");
    let coll = collect(&mut device, &models).await.expect("collection");

    assert_eq!(coll.lldp_model, LldpModel::NoneAdvertised);
    assert!(
        !coll.neighbors.is_empty(),
        "openconfig is read anyway: a device may serve a model it does not advertise"
    );
}

/// ArcOS refuses `/lldp/state` and serves a complete neighbour list anyway. The device's own
/// chassis identity is not its neighbour set, so a refusal there must not cost it authority --
/// if it did, every ArcOS neighbour would be kept forever on the theory it might still be there.
#[tokio::test]
async fn refusing_the_local_identity_path_does_not_cost_a_device_its_authority() {
    let mut device = arcos().advertising(&["openconfig-interfaces"]);
    let models = device.capabilities().await.expect("capabilities");
    let coll = collect(&mut device, &models).await.expect("collection");

    assert!(!coll.neighbors.is_empty(), "the neighbour list was served");
    assert!(
        coll.data_complete().lldp,
        "/lldp/state is the device's own identity, not its neighbours"
    );
}

#[tokio::test]
async fn capabilities_reports_the_models_the_device_advertises() {
    let mut device = ScriptedDevice::default().advertising(&["openconfig-interfaces", "dn-lldp"]);
    let models = device.capabilities().await.expect("capabilities");
    assert_eq!(models, vec!["openconfig-interfaces", "dn-lldp"]);
}

#[test]
fn a_device_advertising_dn_lldp_selects_the_drivenets_profile() {
    let models = vec!["openconfig-interfaces".to_string(), "dn-lldp".to_string()];
    let profile = LldpModelProfile::select(&models).expect("a known profile");
    assert_eq!(profile.module, "dn-lldp");
    assert_eq!(profile.root, &["drivenets-top", "protocols"]);
    assert_eq!(profile.state_container, "oper-items");
}

#[test]
fn advertising_both_reads_openconfig() {
    let models = vec!["dn-lldp".to_string(), "openconfig-lldp".to_string()];
    assert_eq!(
        LldpModelProfile::select(&models)
            .expect("a known profile")
            .module,
        "openconfig-lldp",
        "openconfig is the model this collector was built against"
    );
}

#[test]
fn advertising_no_known_lldp_model_selects_nothing() {
    let models = vec!["openconfig-interfaces".to_string()];
    assert!(LldpModelProfile::select(&models).is_none());
}

/// `neighbors` names a path that must also be in `subtrees` -- two literals that have to agree,
/// with nothing but this test making them. A profile whose `neighbors` matches none of the
/// subtrees it reads never sets `is_lldp`, so `lldp_complete` stays false and every device on
/// that profile reports non-authoritative, its neighbours never ageing out (see
/// `a_profile_that_never_reads_its_neighbours_subtree_is_not_authoritative` for that behaviour).
#[test]
fn every_profile_names_a_neighbours_subtree_it_actually_reads() {
    for p in LldpModelProfile::KNOWN {
        assert!(
            p.subtrees.contains(&p.neighbors),
            "{} names a neighbors subtree it does not read",
            p.module
        );
    }
}

/// Which way the invariant above fails when it does fail: a profile that reads the openconfig
/// subtrees but names a `neighbors` path none of them is. The device answers every subtree and
/// its neighbours parse, and the collection is still non-authoritative, because nothing read
/// the subtree this profile says decides that. Costing a device its pruning is recoverable;
/// claiming authority over a set that was never read deletes stored neighbours.
#[tokio::test]
async fn a_profile_that_never_reads_its_neighbours_subtree_is_not_authoritative() {
    let drifted = LldpModelProfile {
        module: "openconfig-lldp",
        subtrees: OPENCONFIG_LLDP.subtrees,
        root: &[],
        state_container: "state",
        // One element longer than the subtree that is actually read.
        neighbors: Subtree::default_origin(&["lldp", "interfaces", "interface[name=*]", "state"]),
    };
    let mut device = arcos();
    let coll = collect_profile(
        &mut device,
        &drifted,
        LldpModel::Advertised("openconfig-lldp"),
    )
    .await
    .expect("collection succeeds");

    assert!(!coll.neighbors.is_empty(), "the neighbours were read");
    assert!(
        !coll.lldp_complete,
        "no subtree the profile calls its neighbours was read, so it holds no authority"
    );
}

/// `Subtree::path()` is the only place `origin` is actually put on the wire — nothing else
/// reads the field. Every profile above sets it empty (openconfig and DriveNets both answer
/// their tree under the device's default schema tree), so nothing exercises the non-empty
/// case without a synthetic subtree here. Guards against `path()` silently dropping the field,
/// which `subtree_key`/`render_path` (both origin-aware, see above) would not by itself catch.
#[test]
fn a_non_empty_origin_reaches_the_rendered_path() {
    let subtree = Subtree {
        origin: "srl_nokia",
        elems: &["lldp", "interfaces", "interface[name=*]"],
    };
    let path = subtree.path();
    assert_eq!(path.origin, "srl_nokia");
    assert_eq!(
        render_path(&path),
        "srl_nokia:lldp/interfaces/interface[name=*]"
    );
}

/// Parse a gnmic-style path into a `Leaf` with no value, reusing the same path-parsing the
/// fixtures above rely on.
fn leaf_from(path: &str) -> Leaf {
    Leaf {
        elems: parse_path(path).elem,
        value: String::new(),
    }
}

/// The scoping bug this fold has to avoid: a device whose profile renames the state
/// container must not have that rename applied to its `/interfaces` tree, where `state`
/// is already `state` and an `oper-items` container means something else entirely.
#[test]
fn the_state_rewrite_is_scoped_to_the_models_own_lldp_tree() {
    let leaf = leaf_from("interfaces/interface[name=ge100-0/0/1]/oper-items/ifindex");
    let names = normalised_names(&DN_LLDP, &leaf);
    assert_eq!(
        names,
        vec!["interfaces", "interface", "oper-items", "ifindex"],
        "not this model's LLDP tree: nothing is stripped and nothing is renamed"
    );
}

#[test]
fn the_drivenets_root_is_stripped_and_oper_items_reads_as_state() {
    let leaf = leaf_from(
        "drivenets-top/protocols/lldp/interfaces/interface[name=ge100-0/0/2]\
         /neighbors/neighbor[id=0]/oper-items/system-name",
    );
    let names = normalised_names(&DN_LLDP, &leaf);
    assert_eq!(
        names,
        vec![
            "lldp",
            "interfaces",
            "interface",
            "neighbors",
            "neighbor",
            "state",
            "system-name"
        ],
    );
}

#[test]
fn openconfig_normalisation_is_the_identity() {
    let leaf =
        leaf_from("lldp/interfaces/interface[name=swp1]/neighbors/neighbor[id=1]/state/port-id");
    let before: Vec<&str> = leaf.elems.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(normalised_names(&OPENCONFIG_LLDP, &leaf), before);
}
