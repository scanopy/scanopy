use std::net::Ipv4Addr;

use mac_address::MacAddress;

use crate::daemon::discovery::integration::snmp::sim::mibs::{
    BridgeTable, FdbEntry, TplinkPortRow, TplinkVlanRow, TplinkVlanTable,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

use crate::daemon::discovery::integration::snmp::sim::CommunityVersions;

/// The GH #847 reporter's SG3210X-M2 (firmware 1.0.26), from their `-On` walks of the system
/// group, ifTable, ifXTable, the bridge MIB and TPLINK-DOT1Q-VLAN-MIB.
///
/// Interface names, ifIndexes, speeds, VLAN names and port strings are the switch's own. Hardware
/// addresses and sysName are replaced. The walks held no LLDP tables, so the device serves none.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-omada-02",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#847",
            defect: "VLANs kept in TPLINK-DOT1Q-VLAN-MIB with no Q-BRIDGE VLAN group (dot1qBase and the dot1q FDB answer, dot1qVlanStaticName and dot1qPvid do not); the VLAN table and every port's membership came back empty",
        },
        credential: CredentialType::SnmpCommunity {
            community: inline("public"),
        },
        community_versions: CommunityVersions::Both,
        system: SystemInfo {
            sys_descr: Some(
                "Omada 8-Port 2.5GBASE-T L2+ Managed Switch with 2 10GE SFP+ Slots".into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.11863.5.214".into()),
            sys_name: Some("switch-omada-02".into()),
            sys_location: None,
            sys_contact: None,
            sys_services: Some(3),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
        rejects_getbulk: None,
    }
}

fn tables() -> Tables {
    Tables {
        if_table: Some(if_table()),
        bridge: bridge_table(),
        tplink_vlan: vlan_table(),
        ..Default::default()
    }
}

/// One address on every interface, as the switch reports it.
fn chassis() -> MacAddress {
    "30:de:4b:84:70:01".parse().unwrap()
}

/// Port `1/0/<number>`: ifIndex `49152 + number` on this model, which is why the daemon reads the
/// mapping from the port VLAN table and never computes it.
fn port(number: i32, media: &str, bits_per_second: u64) -> IfRow {
    let descr = format!("{media} 1/0/{number}");
    IfRow::port(49152 + number, &descr, Some(chassis()))
        .name(&descr)
        .alias(&format!("Port{number}"))
        .mtu(1518)
        .speed(bits_per_second)
        .high_speed()
}

pub fn if_table() -> IfTable {
    const MBPS_100: u64 = 100_000_000;
    const GBPS_1: u64 = 1_000_000_000;
    IfTable::new(vec![
        IfRow::port(1, "Vlan-interface1", Some(chassis()))
            .name("Vlan-interface1")
            .mtu(1500)
            .high_speed_mbps(0),
        port(1, "two-gigabitEthernet", MBPS_100),
        port(2, "two-gigabitEthernet", 0).oper_down(),
        port(3, "two-gigabitEthernet", MBPS_100),
        port(4, "two-gigabitEthernet", MBPS_100),
        port(5, "two-gigabitEthernet", GBPS_1),
        port(6, "two-gigabitEthernet", GBPS_1),
        port(7, "two-gigabitEthernet", MBPS_100),
        port(8, "two-gigabitEthernet", GBPS_1),
        port(9, "ten-gigabitEthernet", GBPS_1),
        port(10, "ten-gigabitEthernet", GBPS_1),
    ])
}

/// Bridge ports 1 to 10 on ifIndex 49153 to 49162, and a dot1q forwarding table: the part of
/// Q-BRIDGE this firmware does serve. One address on port 10 is learned in all four VLANs, as
/// the uplink's was.
pub fn bridge_table() -> BridgeTable {
    let mac = |last: u8| -> MacAddress { MacAddress::new([0x02, 0x84, 0x70, 0x00, 0x00, last]) };
    // Written out, as the switch serves it. `Vlan-interface1` reports ifType 6 here, so a map
    // derived from the ethernet rows would count it as an eleventh bridge port.
    let ports = (1..=10).map(|port| (port, 49152 + port as i32)).collect();
    BridgeTable::with_ports(ports).fdb(vec![
        FdbEntry::learned(mac(0x01), 10).in_vlan(1),
        FdbEntry::learned(mac(0x02), 9).in_vlan(1),
        FdbEntry::learned(mac(0x01), 10).in_vlan(2),
        FdbEntry::learned(mac(0x03), 4).in_vlan(2),
        FdbEntry::learned(mac(0x01), 10).in_vlan(3),
        FdbEntry::learned(mac(0x04), 9).in_vlan(3),
        FdbEntry::learned(mac(0x05), 3).in_vlan(3),
        FdbEntry::learned(mac(0x06), 1).in_vlan(3),
        FdbEntry::learned(mac(0x07), 7).in_vlan(3),
        FdbEntry::learned(mac(0x01), 10).in_vlan(8),
        FdbEntry::learned(mac(0x08), 6).in_vlan(8),
        FdbEntry::learned(mac(0x09), 5).in_vlan(8),
        FdbEntry::learned(mac(0x0a), 8).in_vlan(8),
    ])
}

/// Both TPLINK-DOT1Q-VLAN-MIB tables, as the issue and its attachment give them.
pub fn vlan_table() -> TplinkVlanTable {
    let port = |number: i32, name: &'static str, pvid: u16| TplinkPortRow {
        if_index: 49152 + number,
        name,
        pvid,
    };
    TplinkVlanTable {
        ports: vec![
            port(1, "1/0/1", 3),
            port(2, "1/0/2", 3),
            port(3, "1/0/3", 3),
            port(4, "1/0/4", 2),
            port(5, "1/0/5", 8),
            port(6, "1/0/6", 8),
            port(7, "1/0/7", 3),
            port(8, "1/0/8", 8),
            port(9, "1/0/9", 1),
            port(10, "1/0/10", 1),
        ],
        vlans: vec![
            TplinkVlanRow::active(1, "VLAN Managment", "1/0/4-6,1/0/8", "1/0/9-10"),
            TplinkVlanRow::active(2, "VLAN Home", "1/0/5-6,1/0/8-10", "1/0/4"),
            TplinkVlanRow::active(3, "VLAN IOT", "1/0/4-6,1/0/8-10", "1/0/1-3,1/0/7"),
            TplinkVlanRow::active(8, "VLAN Servers", "1/0/4,1/0/9-10", "1/0/5-6,1/0/8"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::oids::vlan;
    use crate::daemon::discovery::integration::snmp::sim::{device, harness};

    /// GH #847: the four VLANs the VLANs page never showed.
    #[tokio::test]
    async fn vlan_names_come_from_the_tplink_table() {
        let scan = harness::scan("switch-omada-02").await;

        let mut vlans: Vec<(u16, &str)> = scan
            .vlans
            .iter()
            .map(|v| (v.vlan_id, v.name.as_str()))
            .collect();
        vlans.sort();
        assert_eq!(
            vlans,
            vec![
                (1, "VLAN Managment"),
                (2, "VLAN Home"),
                (3, "VLAN IOT"),
                (8, "VLAN Servers"),
            ]
        );
        assert!(scan.outcome.vlan_names.complete);
    }

    /// Each port's native VLAN and tagged VLANs, as the reporter's walk gives them. The native
    /// VLAN of every port is the VLAN whose untagged list names it.
    #[tokio::test]
    async fn every_port_carries_its_native_and_tagged_vlans() {
        let scan = harness::scan("switch-omada-02").await;

        let mut membership: Vec<(i32, Option<u16>, Vec<u16>)> = scan
            .vlan_membership
            .iter()
            .map(|m| {
                let mut tagged = m.tagged_vlans.clone();
                tagged.sort();
                (m.if_index - 49152, m.native_vlan, tagged)
            })
            .collect();
        membership.sort();
        assert_eq!(
            membership,
            vec![
                (1, Some(3), vec![]),
                (2, Some(3), vec![]),
                (3, Some(3), vec![]),
                (4, Some(2), vec![1, 3, 8]),
                (5, Some(8), vec![1, 2, 3]),
                (6, Some(8), vec![1, 2, 3]),
                (7, Some(3), vec![]),
                (8, Some(8), vec![1, 2, 3]),
                (9, Some(1), vec![2, 3, 8]),
                (10, Some(1), vec![2, 3, 8]),
            ]
        );
        assert!(scan.outcome.vlan_membership.complete);
    }

    /// A LAG has no row in the port table, and neither does a port name the table does not list.
    /// Both are left out and counted, and the ports beside them keep their membership.
    #[tokio::test]
    async fn a_lag_and_an_unlisted_port_are_counted_and_cost_no_other_port_its_vlans() {
        let mut device = device("switch-omada-02");
        // VLAN 1, tagged on ports 4 to 6 and 8 as before, plus a LAG and a twelfth port.
        device.tables.tplink_vlan.vlans[0].tagged = "1/0/4-6,LAG1,1/0/8,1/0/12";
        // The same LAG again in VLAN 2: one member, named twice.
        device.tables.tplink_vlan.vlans[1].tagged = "1/0/5-6,1/0/8-10,LAG1";

        let scan = harness::collect(&device).await;

        assert_eq!(scan.vlan_ports_unresolved, 2, "LAG1 and 1/0/12");
        assert_eq!(scan.vlan_membership.len(), 10);
        let port_8 = scan
            .vlan_membership
            .iter()
            .find(|m| m.if_index == 49160)
            .expect("port 8, listed after the LAG");
        assert!(port_8.tagged_vlans.contains(&1));

        let healthy = harness::scan("switch-omada-02").await;
        assert_eq!(healthy.vlan_ports_unresolved, 0);
    }

    /// A VLAN whose row is not active is no VLAN on the switch.
    #[tokio::test]
    async fn a_vlan_that_is_not_active_is_left_out() {
        let mut device = device("switch-omada-02");
        // notInService(2).
        device.tables.tplink_vlan.vlans[2].status = 2;

        let scan = harness::collect(&device).await;

        let ids: Vec<u16> = scan.vlans.iter().map(|v| v.vlan_id).collect();
        assert_eq!(ids, vec![1, 2, 8]);
    }

    /// The Q-BRIDGE forwarding table this firmware does serve is still read.
    #[tokio::test]
    async fn the_dot1q_forwarding_table_is_still_read() {
        let scan = harness::scan("switch-omada-02").await;
        assert!(!scan.fdb.records.is_empty());
        assert_eq!(scan.bridge_ports.len(), 10);
    }

    /// GH #848: a page that runs past these tables reaches values this firmware takes seconds to
    /// compute, so every GETBULK into TP-Link's tree asks for a short page.
    #[tokio::test]
    async fn every_page_asked_of_the_tplink_tree_is_short() {
        use crate::daemon::discovery::integration::snmp::queries::BULK_MAX_REPETITIONS;

        let (_, requests) = harness::recorded_scan("switch-omada-02").await;
        let pages: Vec<u32> = requests
            .iter()
            .filter(|r| r.is_under(vlan::tplink::TPLINK_DOT1Q_VLAN_MIB))
            .filter_map(|r| r.max_repetitions)
            .collect();

        assert!(!pages.is_empty(), "the TP-Link tables were never asked for");
        assert!(
            pages.iter().all(|reps| *reps < BULK_MAX_REPETITIONS),
            "a full-size page was asked of the TP-Link tree: {pages:?}"
        );
    }
}
