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
        name: "switch-voss-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "Issue 2, July 2026",
            defect: "Extreme VOSS reports local-port == ifIndex, so it must stay correct on both old and new code — the regression guard for the remap",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Extreme Networks VSP-7400, VOSS 8.10".into()),
            sys_object_id: Some("1.3.6.1.4.1.2272.30".into()),
            sys_name: Some("switch-voss-01".into()),
            sys_location: Some("Server Room A, Rack 5".into()),
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
        IfRow::port(192, "1/1", Some("00:04:38:02:e0:01".parse().unwrap()))
            .speed(10000000000)
            .name("1/1"),
        IfRow::port(193, "1/2", Some("00:04:38:02:e0:02".parse().unwrap()))
            .speed(10000000000)
            .name("1/2"),
        IfRow::port(194, "1/3", Some("00:04:38:02:e0:03".parse().unwrap()))
            .speed(10000000000)
            .name("1/3"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:04:38:02:e0:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-voss-01",
    )
    .sys_desc("Extreme Networks VSP-7400, VOSS 8.10")
    .local_ports(vec![
        LocalPort::new(
            192,
            Advertised::octets(LldpPortId::InterfaceName("1/1".into())),
        ),
        LocalPort::new(
            193,
            Advertised::octets(LldpPortId::InterfaceName("1/2".into())),
        ),
        LocalPort::new(
            194,
            Advertised::octets(LldpPortId::InterfaceName("1/3".into())),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            192,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("1/1".into())),
        )
        .port_desc("1/1")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            194,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("1/3".into())),
        )
        .port_desc("1/3")
        .sys_name("switch-access-01")
        .sys_desc("Cisco IOS Software, C3750"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// The regression guard for the same fix.
    ///
    /// Extreme VOSS reports `lldpLocPortNum == ifIndex` and `lldpLocPortId` matching `ifName`
    /// exactly, so it is correct on both old and new code. A remap that started rewriting indexes
    /// it should have left alone would break here and nowhere else.
    #[tokio::test]
    async fn a_local_port_table_that_is_the_identity_moves_nothing() {
        let scan = harness::scan("switch-voss-01").await;

        assert_eq!(scan.local_ports.len(), 3);
        assert_eq!(scan.neighbours.records.len(), 2);

        for neighbour in &scan.neighbours.records {
            let port = neighbour.local_port_index;
            assert!(
                scan.if_table.entries.iter().any(|e| e.if_index == port),
                "the identity mapping moved a neighbour to {port}"
            );
        }
        assert_eq!(scan.local_port_outcome.unmatched, 0);
    }
}
