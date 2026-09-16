use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::BridgeTable;
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-omada-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#614",
            defect: "16 ports at ifIndex 49153+ with no ifName and one shared chassis MAC; all 17 interfaces must persist rather than collapsing via the MAC tier",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("public"),
        },
        system: SystemInfo {
            sys_descr: Some(
                "TP-Link Omada TL-SG3216 JetStream 16-Port Gigabit L2 Managed Switch".into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.11863.6.96".into()),
            sys_name: Some("switch".into()),
            sys_location: Some("Floor 2, Comms Cupboard".into()),
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
            "Vlan-interface1",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        )
        .name("Vlan-interface1"),
        IfRow::port(
            49153,
            "gigabitEthernet 1/0/1",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49154,
            "gigabitEthernet 1/0/2",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49155,
            "gigabitEthernet 1/0/3",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49156,
            "gigabitEthernet 1/0/4",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49157,
            "gigabitEthernet 1/0/5",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49158,
            "gigabitEthernet 1/0/6",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49159,
            "gigabitEthernet 1/0/7",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49160,
            "gigabitEthernet 1/0/8",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49161,
            "gigabitEthernet 1/0/9",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49162,
            "gigabitEthernet 1/0/10",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49163,
            "gigabitEthernet 1/0/11",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49164,
            "gigabitEthernet 1/0/12",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49165,
            "gigabitEthernet 1/0/13",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49166,
            "gigabitEthernet 1/0/14",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49167,
            "gigabitEthernet 1/0/15",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
        IfRow::port(
            49168,
            "gigabitEthernet 1/0/16",
            Some("30:de:4b:30:f0:ac".parse().unwrap()),
        ),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress("30:de:4b:30:f0:ac".into())),
        "switch",
    )
    .sys_desc("TP-Link Omada TL-SG3216")
    .local_ports(vec![
        LocalPort::new(
            1,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/1".into())),
        ),
        LocalPort::new(
            2,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/2".into())),
        ),
        LocalPort::new(
            3,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/3".into())),
        ),
        LocalPort::new(
            4,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/4".into())),
        ),
        LocalPort::new(
            5,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/5".into())),
        ),
        LocalPort::new(
            6,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/6".into())),
        ),
        LocalPort::new(
            7,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/7".into())),
        ),
        LocalPort::new(
            8,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/8".into())),
        ),
        LocalPort::new(
            9,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/9".into())),
        ),
        LocalPort::new(
            10,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/10".into())),
        ),
        LocalPort::new(
            11,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/11".into())),
        ),
        LocalPort::new(
            12,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/12".into())),
        ),
        LocalPort::new(
            13,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/13".into())),
        ),
        LocalPort::new(
            14,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/14".into())),
        ),
        LocalPort::new(
            15,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/15".into())),
        ),
        LocalPort::new(
            16,
            Advertised::octets(LldpPortId::InterfaceName("gigabitEthernet 1/0/16".into())),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            5,
            Advertised::octets(LldpChassisId::MacAddress("00:ad:24:af:4e:00".into())),
            Advertised::octets(LldpPortId::MacAddress("00:ad:24:af:4e:00".into())),
        )
        .port_desc("Uplink")
        .sys_name("switch-dlink-01")
        .sys_desc("D-Link DGS-1210-48 Rev.GX/7.20.003"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    use crate::daemon::discovery::integration::snmp::unique_interface_macs;

    /// GH #614: 16 ports at ifIndex 49153+ with no `ifName`, all sharing the chassis address.
    ///
    /// All 17 interfaces must persist as distinct rows. Before the fix the 16 nameless ports
    /// collapsed onto the management interface through the MAC tier — and note what that means
    /// here: the tier only runs at all because these addresses are readable. Served as text they
    /// would be dropped, and this device would pass while never reaching the code it documents.
    #[tokio::test]
    async fn every_nameless_port_survives_a_shared_chassis_address() {
        let scan = harness::scan("switch-omada-01").await;

        assert_eq!(scan.if_table.entries.len(), 17);
        assert!(scan.if_table.set_complete);

        // The addresses are read, not dropped — which is what puts the MAC tier under test.
        assert!(
            scan.if_table
                .entries
                .iter()
                .all(|e| e.if_phys_address.is_some()),
            "a shared address that is not stored cannot make anything ambiguous"
        );

        // ...and being shared, none of them identifies a port.
        assert!(
            unique_interface_macs(&scan.if_table.entries).is_empty(),
            "one address on every port must name no port at all"
        );

        // Only the management interface has a name; the physical ports have none.
        assert_eq!(
            scan.interface(1).if_name.as_deref(),
            Some("Vlan-interface1")
        );
        assert!(scan.interface(49153).if_name.is_none());
    }
}
