use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
};
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
        name: "firewall-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Control {
            role: "a non-switch baseline that still answers LLDP",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("secret42"),
        },
        system: SystemInfo {
            sys_descr: Some("Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)".into()),
            sys_object_id: Some("1.3.6.1.4.1.12356.101.1.1".into()),
            sys_name: Some("firewall-01".into()),
            sys_location: Some("Server Room A, Rack 2".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(76),
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
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(1, "port1", Some("00:1a:2b:00:13:01".parse().unwrap()))
            .name("port1")
            .high_speed()
            .alias("WAN - to router-gw-01"),
        IfRow::port(2, "port2", Some("00:1a:2b:00:13:02".parse().unwrap()))
            .name("port2")
            .high_speed()
            .alias("LAN - internal"),
        IfRow::port(3, "port3", Some("00:1a:2b:00:13:03".parse().unwrap()))
            .name("port3")
            .high_speed()
            .alias("DMZ"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:13:00".into()),
            MacEncoding::AsciiLower,
        ),
        "firewall-01",
    )
    .sys_desc("Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:12:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("ge-0/0/1".into())),
        )
        .port_desc("ge-0/0/1")
        .sys_name("router-gw-01")
        .sys_desc("Juniper Networks, Inc. JunOS 21.4R3-S5, MX204"),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// A control device: a non-switch that still answers LLDP, so the neighbour path is exercised
    /// on something that does not bridge.
    #[tokio::test]
    async fn it_advertises_a_neighbour_without_serving_a_bridge_table() {
        let scan = harness::scan("firewall-01").await;

        assert_eq!(scan.neighbours.records.len(), 1);
        assert!(scan.neighbours.complete);
        assert!(
            scan.bridge_ports.is_empty(),
            "a firewall reports no bridge ports"
        );
    }
}
