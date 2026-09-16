use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::BridgeTable;
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::{
    CredentialType, SnmpV3AuthProtocol, SnmpV3PrivProtocol,
};
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "secure-switch-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#557",
            defect: "SNMPv3 AuthPriv — the USM handshake and an encrypted session",
        },
        credential: CredentialType::SnmpV3 {
            security_name: "scanopyv3".into(),
            auth_protocol: SnmpV3AuthProtocol::Sha256,
            auth_password: inline("authpass12345"),
            priv_protocol: SnmpV3PrivProtocol::Aes128,
            priv_password: inline("privpass12345"),
            context_name: None,
        },
        system: SystemInfo {
            sys_descr: Some("Huawei S5000 Series, VRP V200R019C10".into()),
            sys_object_id: Some("1.3.6.1.4.1.2011.2.23.999".into()),
            sys_name: Some("secure-switch-01".into()),
            sys_location: Some("Server Room A, Rack 4".into()),
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
        IfRow::port(
            1,
            "GigabitEthernet0/0/1",
            Some("00:1a:2b:00:17:01".parse().unwrap()),
        )
        .name("GE0/0/1")
        .high_speed()
        .alias("Uplink to switch-core-01"),
        IfRow::port(
            2,
            "GigabitEthernet0/0/2",
            Some("00:1a:2b:00:17:02".parse().unwrap()),
        )
        .name("GE0/0/2")
        .high_speed()
        .alias("Server port"),
        IfRow::port(
            3,
            "GigabitEthernet0/0/3",
            Some("00:1a:2b:00:17:03".parse().unwrap()),
        )
        .name("GE0/0/3")
        .high_speed()
        .alias("Server port"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:17:00".into()),
            MacEncoding::AsciiLower,
        ),
        "secure-switch-01",
    )
    .sys_desc("Huawei S5000 Series, VRP V200R019C10")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/1".into())),
        )
        .port_desc("GigabitEthernet0/1")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// GH #557's v3 half.
    ///
    /// The USM handshake is a session concern and cannot be reached through `SnmpWalkTransport`,
    /// so what a unit test can hold is that this device's *data* is ordinary — the encrypted
    /// session is covered by `make snmp-verify`, which authenticates against it for real. Keeping
    /// this device's tables unremarkable is deliberate: if a v3 scan comes back short, the cause
    /// is the session, not the fixture.
    #[tokio::test]
    async fn its_data_is_ordinary_so_a_short_v3_scan_means_the_session() {
        let scan = harness::scan("secure-switch-01").await;

        assert_eq!(scan.if_table.entries.len(), 3);
        assert!(scan.if_table.set_complete && scan.if_table.attributes_complete);
        assert_eq!(scan.neighbours.records.len(), 1);
        assert!(scan.neighbours.complete);
        assert_eq!(scan.bridge_ports.len(), 3);
    }
}
