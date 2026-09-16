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
        name: "switch-dlink-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#668",
            defect: "four neighbour records each needing a different route to their far end, and one chassis MAC repeated across every port",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("D-Link DGS-1210-48 Rev.GX/7.20.003".into()),
            sys_object_id: Some("1.3.6.1.4.1.171.10.76.28".into()),
            sys_name: Some("switch-dlink-01".into()),
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
            "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 1",
            Some("00:ad:24:af:4e:00".parse().unwrap()),
        )
        .name("Slot0/1"),
        IfRow::port(
            2,
            "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 2",
            Some("00:ad:24:af:4e:00".parse().unwrap()),
        )
        .name("Slot0/2"),
        IfRow::port(
            3,
            "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 3",
            Some("00:ad:24:af:4e:00".parse().unwrap()),
        )
        .name("Slot0/3"),
        IfRow::port(
            4,
            "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 4",
            Some("00:ad:24:af:4e:00".parse().unwrap()),
        )
        .name("Slot0/4"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:ad:24:af:4e:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-dlink-01",
    )
    .sys_desc("D-Link DGS-1210-48 Rev.GX/7.20.003")
    .local_ports(vec![
        LocalPort::new(
            1,
            Advertised::octets(LldpPortId::InterfaceName("Slot0/1".into())),
        ),
        LocalPort::new(
            2,
            Advertised::octets(LldpPortId::InterfaceName("Slot0/2".into())),
        ),
        LocalPort::new(
            3,
            Advertised::octets(LldpPortId::InterfaceName("Slot0/3".into())),
        ),
        LocalPort::new(
            4,
            Advertised::octets(LldpPortId::InterfaceName("Slot0/4".into())),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("2".into())),
        )
        .port_desc("GigabitEthernet0/2")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            2,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/0/44".into())),
        )
        .port_desc("GigabitEthernet0/1")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            3,
            Advertised::octets(LldpChassisId::MacAddress("00:07:7c:20:01:e0".into())),
            Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e3".into())),
        )
        .port_desc("Ring port to peer")
        .sys_name("switch-macport-01")
        .sys_desc("Westermo WeOS"),
        RemoteNeighbour::new(
            4,
            Advertised::octets(LldpChassisId::MacAddress("30:de:4b:30:f0:ac".into())),
            Advertised::octets(LldpPortId::MacAddress("30:de:4b:30:f0:ac".into())),
        )
        .port_desc("Uplink")
        .sys_name("switch")
        .sys_desc("TP-Link Omada TL-SG3216"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    use crate::daemon::discovery::integration::snmp::unique_interface_macs;
    use crate::server::lldp::LldpPortId;

    /// GH #668: four neighbour records, each needing a different route to its far end.
    ///
    /// The port ids are what this device is for, and each is a distinct shape the resolver has to
    /// handle: a subtype-5 `interfaceName` carrying a bare port *number*, an id matching nothing
    /// whose *description* matches instead, and two MAC port ids — one that identifies exactly one
    /// far-end port and one that cannot.
    #[tokio::test]
    async fn its_four_port_ids_are_four_different_shapes() {
        let scan = harness::scan("switch-dlink-01").await;

        assert_eq!(scan.neighbours.records.len(), 4);
        assert_eq!(scan.neighbours.discarded, 0);

        let id_on = |port: i32| {
            let neighbour = scan
                .neighbours_on(port)
                .into_iter()
                .next()
                .unwrap_or_else(|| panic!("no neighbour on local port {port}"));
            LldpPortId::from_snmp(
                neighbour.remote_port_id_subtype.unwrap(),
                neighbour.remote_port_id_bytes.as_ref().unwrap(),
            )
            .expect("a port id")
        };

        // Subtype 5 carrying a bare number. It used to get a name lookup and nothing else, so it
        // resolved to the host and stopped — and a host-only neighbour draws no edge.
        assert_eq!(id_on(1), LldpPortId::InterfaceName("2".into()));
        // An id that matches nothing on the far end; only its description does.
        assert_eq!(id_on(2), LldpPortId::InterfaceName("ethernet1/0/44".into()));
        // Two MAC port ids, sent as six raw octets — the lab's only end-to-end coverage of
        // `parse_mac_id`'s raw-octet branch, since every other LLDP fixture uses the ASCII form.
        assert!(matches!(id_on(3), LldpPortId::MacAddress(_)));
        assert!(matches!(id_on(4), LldpPortId::MacAddress(_)));
    }

    /// The third report from the same issue: every port answers with the chassis base address.
    ///
    /// The ifTable walk keys each `ifPhysAddress` off its own row, so it cannot copy one row's
    /// value onto another — but a MAC that names three ports names none of them, and the lookups
    /// that treated one as a port identifier picked whichever row came back first.
    #[tokio::test]
    async fn one_address_on_every_port_identifies_no_port() {
        let scan = harness::scan("switch-dlink-01").await;

        let addresses: Vec<String> = scan
            .if_table
            .entries
            .iter()
            .filter_map(|e| e.if_phys_address.map(|m| m.to_string().to_lowercase()))
            .collect();
        assert_eq!(addresses.len(), 4);
        assert!(
            addresses.iter().all(|mac| mac == "00:ad:24:af:4e:00"),
            "the shared base address is the point: {addresses:?}"
        );
        assert!(
            unique_interface_macs(&scan.if_table.entries).is_empty(),
            "an address on every port must identify none of them"
        );
    }
}
