use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour, TimeMark,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::BridgeTable;
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-tplink-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#668",
            defect: "lldpRemTable indexed without lldpRemTimeMark, so every row arrives one sub-id short and the device vanishes raising no warning at all",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("TL-SX3016F 1.0 - TP-Link 16-Port 10G SFP+ Managed Switch".into()),
            sys_object_id: Some("1.3.6.1.4.1.11863.5.1.1".into()),
            sys_name: Some("switch-tplink-01".into()),
            sys_location: Some("Lab".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
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
        lldp: Some(lldp_table()),
        bridge: bridge_table(),
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "ten-gigabitEthernet 1/0/1",
            Some("18:66:da:5d:aa:01".parse().unwrap()),
        )
        .speed(10000000000),
        IfRow::port(
            2,
            "ten-gigabitEthernet 1/0/2",
            Some("18:66:da:5d:aa:02".parse().unwrap()),
        )
        .speed(10000000000)
        .oper_down(),
        IfRow::port(
            3,
            "ten-gigabitEthernet 1/0/3",
            Some("18:66:da:5d:aa:03".parse().unwrap()),
        )
        .speed(10000000000),
        IfRow::port(
            4,
            "ten-gigabitEthernet 1/0/4",
            Some("18:66:da:5d:aa:04".parse().unwrap()),
        )
        .speed(10000000000)
        .oper_down(),
        IfRow::port(
            5,
            "ten-gigabitEthernet 1/0/5",
            Some("18:66:da:5d:aa:05".parse().unwrap()),
        )
        .speed(10000000000),
        IfRow::virtual_if(17, "Vlan-interface1", if_type::PROP_VIRTUAL)
            .mac("18:66:da:5d:aa:8e".parse().unwrap()),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("18:66:da:5d:aa:8e".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-tplink-01",
    )
    .sys_desc("TL-SX3016F 1.0 - TP-Link Switch")
    .local_ports(vec![
        LocalPort::new(
            1,
            Advertised::octets(LldpPortId::InterfaceName(
                "ten-gigabitEthernet 1/0/1".into(),
            )),
        ),
        LocalPort::new(
            2,
            Advertised::octets(LldpPortId::InterfaceName(
                "ten-gigabitEthernet 1/0/2".into(),
            )),
        ),
        LocalPort::new(
            3,
            Advertised::octets(LldpPortId::InterfaceName(
                "ten-gigabitEthernet 1/0/3".into(),
            )),
        ),
        LocalPort::new(
            4,
            Advertised::octets(LldpPortId::InterfaceName(
                "ten-gigabitEthernet 1/0/4".into(),
            )),
        ),
        LocalPort::new(
            5,
            Advertised::octets(LldpPortId::InterfaceName(
                "ten-gigabitEthernet 1/0/5".into(),
            )),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiUpper,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .time_mark(TimeMark::Omitted)
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            2,
            Advertised::text(
                LldpChassisId::MacAddress("9c:ad:97:1f:22:40".into()),
                MacEncoding::AsciiUpper,
            ),
            Advertised::octets(LldpPortId::InterfaceName("1".into())),
        )
        .time_mark(TimeMark::Omitted)
        .port_desc("Port 1")
        .sys_name("desk-phone-4021")
        .sys_desc("Polycom VVX 411"),
        RemoteNeighbour::new(
            3,
            Advertised::text(
                LldpChassisId::MacAddress("00:ad:24:af:4e:00".into()),
                MacEncoding::AsciiUpper,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Slot0/3".into())),
        )
        .time_mark(TimeMark::Omitted)
        .port_desc("D-Link DGS-1210-48 Rev.GX/7.20.003 Port 3")
        .sys_name("switch-dlink-01")
        .sys_desc("D-Link DGS-1210-48 Rev.GX/7.20.003"),
        RemoteNeighbour::new(
            4,
            Advertised::text(
                LldpChassisId::MacAddress("00:ad:24:af:4e:00".into()),
                MacEncoding::AsciiUpper,
            ),
            Advertised::text(
                LldpPortId::MacAddress("00:ad:24:af:4e:00".into()),
                MacEncoding::AsciiUpper,
            ),
        )
        .time_mark(TimeMark::Omitted)
        .port_desc("Uplink to core")
        .sys_name("switch-dlink-01")
        .sys_desc("D-Link DGS-1210-48 Rev.GX/7.20.003"),
        RemoteNeighbour::new(
            5,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".into()),
                MacEncoding::AsciiUpper,
            ),
            Advertised::octets(LldpPortId::InterfaceName("3".into())),
        )
        .time_mark(TimeMark::Omitted)
        .port_desc("Slot: 0 Port: 3 Gigabit - Level")
        .sys_name("switch-netgear-01")
        .sys_desc("GS724Tv3 ProSafe 24-port Gigabit Smart Switch"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// GH #668: a neighbour table indexed without `lldpRemTimeMark`.
    ///
    /// The MIB indexes `lldpRemEntry` as `timeMark.localPort.remIndex`; this firmware omits the
    /// time mark and indexes on the remaining two, so every row arrives one sub-id shorter than on
    /// every other device here.
    ///
    /// This is the shape that made the device vanish *without evidence*. A parser requiring three
    /// sub-ids built no record, so nothing reached the discard counters, the walk still reported
    /// itself complete, and an empty result from a sixteen-port switch was then treated as the
    /// device authoritatively reporting no neighbours — clearing links the server already held. It
    /// was the only failure in this query that raised no warning of any kind.
    #[tokio::test]
    async fn a_two_element_neighbour_index_still_builds_records() {
        let scan = harness::scan("switch-tplink-01").await;

        assert_eq!(
            scan.neighbours.records.len(),
            5,
            "a short index must build records, not silently build none"
        );
        assert!(scan.neighbours.complete);
        assert_eq!(scan.neighbours.discarded, 0);

        // Each sits on a local port the device actually has.
        for neighbour in &scan.neighbours.records {
            assert!(
                (1..=5).contains(&neighbour.local_port_index),
                "local port {} is outside 1/0/1-1/0/5",
                neighbour.local_port_index
            );
        }
    }

    /// Its ports are known only by `ifDescr`: there is no ifXTable at all, which is what the
    /// neighbour port ids over on `switch-dlink-01` have to be matched against.
    #[tokio::test]
    async fn it_serves_no_if_x_table_so_its_ports_are_known_only_by_description() {
        let scan = harness::scan("switch-tplink-01").await;

        assert_eq!(scan.if_table.entries.len(), 6);
        assert!(
            scan.if_table.entries.iter().all(|e| e.if_name.is_none()),
            "an ifName here would hide what this device is for"
        );
        assert_eq!(
            scan.interface(1).if_descr.as_deref(),
            Some("ten-gigabitEthernet 1/0/1")
        );
    }
}
