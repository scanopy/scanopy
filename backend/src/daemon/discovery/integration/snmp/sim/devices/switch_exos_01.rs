use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::BridgeTable;
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-exos-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "Issue 2, July 2026",
            defect: "ExtremeXOS numbers lldpRemTable local ports in a namespace distinct from ifIndex, so without the lldpLocPortTable remap this switch yields zero neighbours",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("ExtremeXOS version 31.7 X435-24P".into()),
            sys_object_id: Some("1.3.6.1.4.1.1916.2.219".into()),
            sys_name: Some("switch-exos-01".into()),
            sys_location: Some("Floor 3, IDF C".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(6),
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
        IfRow::port(1001, "1:1", Some("00:04:96:01:e0:01".parse().unwrap())).name("1:1"),
        IfRow::port(1002, "1:2", Some("00:04:96:01:e0:02".parse().unwrap())).name("1:2"),
        IfRow::port(1003, "1:3", Some("00:04:96:01:e0:03".parse().unwrap())).name("1:3"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("0:4:96:1:e0:0".into()),
            MacEncoding::AsciiAbbreviated,
        ),
        "switch-exos-01",
    )
    .sys_desc("ExtremeXOS version 31.7 X435-24P")
    .local_ports(vec![
        LocalPort::new(1, Advertised::octets(LldpPortId::InterfaceName("1".into()))),
        LocalPort::new(2, Advertised::octets(LldpPortId::InterfaceName("2".into()))),
        LocalPort::new(3, Advertised::octets(LldpPortId::InterfaceName("3".into()))),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("1".into())),
        )
        .port_desc("1:1")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            3,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:12:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("3".into())),
        )
        .port_desc("1:3")
        .sys_name("router-gw-01")
        .sys_desc("Juniper Networks JunOS MX204"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// Issue 2, July 2026: ExtremeXOS numbers its `lldpRemTable` local ports 1..N, in a namespace
    /// distinct from `ifIndex` — this switch's interfaces are at 1001+.
    ///
    /// The neighbours only reach an interface if the daemon walks `lldpLocPortTable` and matches
    /// `lldpLocPortId` against `ifName`. Before that remap this device yields **zero** usable
    /// neighbours: every one lands on an index no interface holds and is discarded whole.
    #[tokio::test]
    async fn its_neighbours_are_remapped_out_of_a_separate_port_namespace() {
        let scan = harness::scan("switch-exos-01").await;

        assert_eq!(scan.local_ports.len(), 3, "the remap needs this table");
        assert_eq!(scan.neighbours.records.len(), 2);

        // Every neighbour now sits on a real ifIndex rather than on its advertised port number.
        let if_indexes: Vec<i32> = scan.if_table.entries.iter().map(|e| e.if_index).collect();
        for neighbour in &scan.neighbours.records {
            assert!(
                if_indexes.contains(&neighbour.local_port_index),
                "neighbour landed on {} which is no interface: {if_indexes:?}",
                neighbour.local_port_index
            );
            assert!(
                neighbour.local_port_index >= 1001,
                "the advertised 1..N number survived the remap"
            );
        }
    }
}
