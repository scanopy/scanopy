use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour, V2,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::BridgeTable;
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

/// Modelled on a UfiSpace S9600-32X running IP Infusion OcNOS 7.0.1, from an `snmpwalk` of the
/// real switch (GH #688). Identifiers are rewritten for the lab; the structure is the device's
/// own: a management port at ifIndex 3, thirty-two 100G ports at 10001, 10005, … 10125, and an
/// LLDP-V2-MIB with three neighbours whose rows are keyed by those ifIndex values.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-ocnos-01",
        ip: Ipv4Addr::new(192, 168, 7, 252),
        purpose: Purpose::Regression {
            issue: "#688",
            defect: "serves LLDP-V2-MIB only: classic lldpRemTable is absent, so the walk finds no neighbours and the device contributes no L2 edges",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some(
                "Hardware Model:UFI_S9600-32X, Software version: OcNOS,7.0.1.60".into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.36673.100.1.2.1.1.2.52257.2.19".into()),
            sys_name: Some("switch-ocnos-01".into()),
            sys_location: Some("Lab".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(14),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
    }
}

fn tables() -> Tables {
    Tables {
        if_table: Some(if_table()),
        lldp: Some(lldp_table()),
        bridge: bridge_table(),
        ..Default::default()
    }
}

/// The management port's address, which is also the chassis id the switch advertises.
const CHASSIS_MAC: &str = "00:1a:2b:6e:ee:70";

/// The ifIndex of front-panel port `ceN`. OcNOS numbers them 10001 + 4N: the real walk lists
/// 10001, 10005, … 10125, with nothing in between, so the neighbour indices below (10009 is
/// `ce2`, 10073 is `ce18`) are only meaningful against a table with the same gaps.
fn ce_if_index(n: i32) -> i32 {
    10001 + 4 * n
}

/// The two fabric uplinks, `ce2` and `ce18` — the only front-panel ports that are up.
const UPLINK_CE2: i32 = 10009;
const UPLINK_CE18: i32 = 10073;

/// The hardware address of front-panel port `ceN`, consecutive from the chassis address.
fn ce_mac(n: i32) -> String {
    format!("00:1a:2b:6e:ee:{:02x}", 0x71 + n)
}

pub fn if_table() -> IfTable {
    let mut rows = vec![
        IfRow::virtual_if(1, "lo", if_type::SOFTWARE_LOOPBACK)
            .mtu(16436)
            .name("lo")
            .high_speed(),
        IfRow::port(3, "eth0", Some(CHASSIS_MAC.parse().unwrap()))
            .name("eth0")
            .high_speed(),
    ];
    for n in 0..32 {
        let mut port = IfRow::port(
            ce_if_index(n),
            &format!("ce{n}"),
            Some(ce_mac(n).parse().unwrap()),
        )
        .speed(100_000_000_000)
        .name(&format!("ce{n}"))
        .high_speed();
        // Only the two uplinks are up; the other thirty ports have nothing plugged in.
        if ce_if_index(n) != UPLINK_CE2 && ce_if_index(n) != UPLINK_CE18 {
            port = port.oper_down();
        }
        rows.push(port);
    }
    // The real walk also lists loopbacks and a sub-interface (`lo.management`,
    // `lo.INTERNET-VRF`, `loopback10`, `ce10.20`). They carry no LLDP and no MAC of their own,
    // so they are left out rather than modelled.
    IfTable::new(rows)
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress(CHASSIS_MAC.into())),
        "switch-ocnos-01",
    )
    .in_mib(&V2)
    .sys_desc("Hardware Model:UFI_S9600-32X, Software version: OcNOS,7.0.1.60")
    // `lldpV2LocPortTable` is keyed by ifIndex and lists every port, each identifying itself
    // by its own MAC — the same thirty-three rows as the walk. The daemon never reads it on
    // the V2 path; it is here so the served subtree is the device's, not a subset of it.
    .local_ports(local_ports())
    .neighbours(vec![
        // The management port, cabled to the lab's core switch. The remote index is 4, not 1:
        // OcNOS numbers `lldpV2RemIndex` across the whole device rather than per port, and a
        // fixture with every index at 1 would not catch a splitter that read the wrong sub-id.
        RemoteNeighbour::new(
            3,
            Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:00:10:00".into())),
            Advertised::octets(LldpPortId::LocallyAssigned("Ethernet5".into())),
        )
        .index(4)
        .port_desc("S9600-32X : eth0")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        // Two fabric uplinks to switches outside this lab. Their port descriptions are a single
        // space, which is what the real far ends send.
        RemoteNeighbour::new(
            UPLINK_CE2 as u32,
            Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:40:e9:ca".into())),
            Advertised::octets(LldpPortId::InterfaceName("swp5".into())),
        )
        .index(6)
        .port_desc(" ")
        .sys_name("switch-arcos-01")
        .sys_desc("Arrcus Operating System (ArcOS)"),
        RemoteNeighbour::new(
            UPLINK_CE18 as u32,
            Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:40:d4:ca".into())),
            Advertised::octets(LldpPortId::InterfaceName("swp5".into())),
        )
        .index(2)
        .port_desc(" ")
        .sys_name("switch-arcos-02")
        .sys_desc("Arrcus Operating System (ArcOS)"),
    ])
}

fn local_ports() -> Vec<LocalPort> {
    let mut ports = vec![
        LocalPort::new(
            3,
            Advertised::octets(LldpPortId::MacAddress(CHASSIS_MAC.into())),
        )
        .desc("eth0"),
    ];
    ports.extend((0..32).map(|n| {
        LocalPort::new(
            ce_if_index(n) as u32,
            Advertised::octets(LldpPortId::MacAddress(ce_mac(n))),
        )
        .desc(&format!("ce{n}"))
    }));
    ports
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// GH #688: a device that serves only the LLDP-V2-MIB.
    ///
    /// Its classic `lldpRemTable` is absent, so before the fallback the walk found no neighbours
    /// and the device contributed no L2 edges at all. The V2 rows are keyed
    /// `timeMark.localIfIndex.destMacIndex.remIndex`, and the local identifier is a real ifIndex
    /// — so the neighbours have to land on 10009 and 10073 directly, not be translated through a
    /// `lldpLocPortTable` the device does not serve, and not be collapsed onto the
    /// destination-address index by a splitter that reads the classic layout off the end.
    #[tokio::test]
    async fn v2_only_neighbours_are_built_on_their_real_if_index() {
        let scan = harness::scan("switch-ocnos-01").await;

        assert_eq!(
            scan.neighbours.records.len(),
            3,
            "the V2 table must be read when the classic one is absent"
        );
        assert!(scan.neighbours.complete);
        assert!(!scan.neighbours.unsupported);
        assert_eq!(scan.neighbours.discarded, 0);
        assert!(
            scan.neighbours.local_port_is_if_index,
            "a V2 result must tell the caller its ports are already ifIndex values"
        );

        // Each neighbour sits on the ifIndex the device keyed it by, and each is an interface the
        // device actually has.
        let mut on = scan
            .neighbours
            .records
            .iter()
            .map(|n| n.local_port_index)
            .collect::<Vec<_>>();
        on.sort_unstable();
        assert_eq!(on, vec![3, 10009, 10073]);
        assert_eq!(scan.interface(10009).if_name.as_deref(), Some("ce2"));
        assert_eq!(scan.interface(10073).if_name.as_deref(), Some("ce18"));
        assert_eq!(scan.local_port_outcome.unmatched, 0);
        assert_eq!(scan.local_port_outcome.dropped, 0);
        assert_eq!(scan.dropped_neighbours, 0);

        assert_eq!(
            scan.neighbour_named("switch-arcos-01").local_port_index,
            10009
        );
        assert_eq!(scan.neighbour_named("switch-core-01").local_port_index, 3);
    }

    /// The classic `lldpLocPortTable` is not served, and nothing on this path may depend on it.
    #[tokio::test]
    async fn it_serves_no_classic_lldp_tables_at_all() {
        let scan = harness::scan("switch-ocnos-01").await;

        assert!(
            scan.local_ports.is_empty(),
            "a classic local-port row would mean the device is not V2-only"
        );
        // lo, eth0 and thirty-two front-panel ports; only the two uplinks are up.
        assert_eq!(scan.if_table.entries.len(), 34);
        let up: Vec<i32> = scan
            .if_table
            .entries
            .iter()
            .filter(|e| e.if_index >= 10001 && e.if_oper_status == Some(1))
            .map(|e| e.if_index)
            .collect();
        assert_eq!(up, vec![10009, 10073]);
    }
}
