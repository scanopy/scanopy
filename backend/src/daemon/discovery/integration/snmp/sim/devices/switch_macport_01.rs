use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour, TimeMark,
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

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-macport-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "the Westermo WeOS report, August 2026",
            defect: "ifDescr carries the media type in front of the name, so only ifName and ifAlias hold the bare port name a neighbour advertises",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("WeOS 5.21.0 industrial ethernet switch".into()),
            sys_object_id: Some("1.3.6.1.4.1.16177.1.1".into()),
            sys_name: Some("switch-macport-01".into()),
            sys_location: Some("Substation B, DIN rail".into()),
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
        IfRow::virtual_if(1, "lo", if_type::SOFTWARE_LOOPBACK)
            .mac("00:00:00:00:00:00".parse().unwrap())
            .name("lo")
            .alias("lo"),
        IfRow::port(
            10,
            "100-T eth10",
            Some("00:07:7c:20:01:ea".parse().unwrap()),
        )
        .speed(100000000)
        .name("eth10")
        .alias("eth10"),
        IfRow::port(11, "100-T eth9", Some("00:07:7c:20:01:e9".parse().unwrap()))
            .speed(100000000)
            .name("eth9")
            .alias("eth9"),
        IfRow::port(12, "100-T eth8", Some("00:07:7c:20:01:e8".parse().unwrap()))
            .speed(0)
            .name("eth8")
            .alias("eth8")
            .oper_down(),
        IfRow::port(13, "100-T eth7", Some("00:07:7c:20:01:e7".parse().unwrap()))
            .speed(100000000)
            .name("eth7")
            .alias("eth7"),
        IfRow::port(14, "100-T eth6", Some("00:07:7c:20:01:e6".parse().unwrap()))
            .speed(100000000)
            .name("eth6")
            .alias("eth6"),
        IfRow::port(15, "100-T eth5", Some("00:07:7c:20:01:e5".parse().unwrap()))
            .speed(100000000)
            .name("eth5")
            .alias("eth5"),
        IfRow::port(16, "100-T eth4", Some("00:07:7c:20:01:e4".parse().unwrap()))
            .speed(100000000)
            .name("eth4")
            .alias("eth4"),
        IfRow::port(17, "100-T eth3", Some("00:07:7c:20:01:e3".parse().unwrap()))
            .speed(100000000)
            .name("eth3")
            .alias("eth3"),
        IfRow::port(
            18,
            "1000-T eth2",
            Some("00:07:7c:20:01:e2".parse().unwrap()),
        )
        .speed(0)
        .name("eth2")
        .alias("eth2")
        .oper_down(),
        IfRow::port(
            19,
            "1000-LX eth1",
            Some("00:07:7c:20:01:e1".parse().unwrap()),
        )
        .name("eth1")
        .alias("eth1"),
        IfRow::virtual_if(22, "vlan1", if_type::PROP_VIRTUAL)
            .mac("00:07:7c:20:01:e0".parse().unwrap())
            .name("vlan1")
            .alias("vlan1")
            .oper_down(),
        IfRow::virtual_if(23, "vlan6", if_type::PROP_VIRTUAL)
            .mac("00:07:7c:20:01:e0".parse().unwrap())
            .name("vlan6")
            .alias("vlan6"),
        IfRow::virtual_if(26, "vlan832", if_type::PROP_VIRTUAL)
            .mac("00:07:7c:20:01:e0".parse().unwrap())
            .name("vlan832")
            .alias("vlan832"),
        IfRow::virtual_if(28, "vlan1302", if_type::PROP_VIRTUAL)
            .mac("00:07:7c:20:01:e0".parse().unwrap())
            .name("vlan1302")
            .alias("vlan1302"),
        IfRow::virtual_if(29, "vlan1305", if_type::PROP_VIRTUAL)
            .mac("00:07:7c:20:01:e0".parse().unwrap())
            .name("vlan1305")
            .alias("vlan1305"),
        IfRow::virtual_if(30, "vlan1251", if_type::PROP_VIRTUAL)
            .mac("00:07:7c:20:01:e0".parse().unwrap())
            .name("vlan1251")
            .alias("vlan1251"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(Advertised::octets(LldpChassisId::MacAddress("00:07:7c:20:01:e0".into())), "switch-macport-01")
        .sys_desc("WeOS 5.21.0 industrial ethernet switch")
        .local_ports(vec![
            LocalPort::new(10, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:ea".into())))
                .desc("100-T eth10"),
            LocalPort::new(11, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e9".into())))
                .desc("100-T eth9"),
            LocalPort::new(12, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e8".into())))
                .desc("100-T eth8"),
            LocalPort::new(13, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e7".into())))
                .desc("100-T eth7"),
            LocalPort::new(14, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e6".into())))
                .desc("100-T eth6"),
            LocalPort::new(15, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e5".into())))
                .desc("100-T eth5"),
            LocalPort::new(16, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e4".into())))
                .desc("100-T eth4"),
            LocalPort::new(17, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e3".into())))
                .desc("100-T eth3"),
            LocalPort::new(18, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e2".into())))
                .desc("1000-T eth2"),
            LocalPort::new(19, Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e1".into())))
                .desc("1000-LX eth1"),
        ])
        .neighbours(vec![
            RemoteNeighbour::new(11, Advertised::octets(LldpChassisId::LocallyAssigned("C230408".into())), Advertised::octets(LldpPortId::MacAddress("e8:80:88:be:30:e7".into())))
                .time_mark(TimeMark::At(100)),
            RemoteNeighbour::new(19, Advertised::octets(LldpChassisId::MacAddress("f0:64:26:b3:84:00".into())), Advertised::octets(LldpPortId::InterfaceName("1/19".into())))
                .time_mark(TimeMark::At(500))
                .index(2)
                .port_desc("Extreme Networks 5520-24X-FabricEngine - GbicLx Port 1/19")
                .sys_name("VSAFC11")
                .sys_desc("5520-24X-FabricEngine (9.3.1.0)"),
            RemoteNeighbour::new(16, Advertised::octets(LldpChassisId::MacAddress("78:8c:77:e5:92:7d".into())), Advertised::octets(LldpPortId::MacAddress("78:8c:77:e5:92:7d".into())))
                .time_mark(TimeMark::At(1400))
                .index(3)
                .port_desc("eth0")
                .sys_name("M300.printers.motala.se")
                .sys_desc("Lexmark Poky (Yocto Project Reference Distro) 4.0.14 (kirkstone) Linux 5.15.58-yocto-standard aarch64"),
        ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    use crate::daemon::discovery::integration::snmp::unique_interface_macs;

    /// The Westermo WeOS report, August 2026, reconciled against the customer's own walk.
    ///
    /// `ifDescr` carries the media type in front of the name — `100-T eth9` — so a neighbour
    /// advertising the bare port name matches `ifDescr` nowhere on this family. `ifName` *and*
    /// `ifAlias` both hold the bare name, and this is the only fixture serving `ifAlias`: it is
    /// what makes `eth9` resolvable at all.
    #[tokio::test]
    async fn only_if_name_and_if_alias_hold_the_bare_port_name() {
        let scan = harness::scan("switch-macport-01").await;

        let eth9 = scan.interface(11);
        assert_eq!(eth9.if_descr.as_deref(), Some("100-T eth9"));
        assert_eq!(eth9.if_name.as_deref(), Some("eth9"));
        assert_eq!(eth9.if_alias.as_deref(), Some("eth9"));
        assert!(
            !eth9.if_descr.as_deref().unwrap().eq("eth9"),
            "if ifDescr were the bare name this device would prove nothing"
        );
    }

    /// Its physical ports each have a unique address while its six VLAN interfaces repeat the
    /// chassis one. A MAC lookup that counted the virtual rows would find six matches and decline,
    /// costing a port no physical interface ever contested.
    #[tokio::test]
    async fn its_physical_ports_are_individually_addressed() {
        let scan = harness::scan("switch-macport-01").await;

        assert_eq!(scan.if_table.entries.len(), 17);
        let unique = unique_interface_macs(&scan.if_table.entries);
        assert!(
            unique.len() >= 10,
            "each physical port must name itself: {} unique",
            unique.len()
        );
    }

    /// `lldpLocPortTable` is keyed 10-19, which are this device's own `ifIndex` values: the local
    /// port table is the identity mapping. That is exactly why the remap fix changed nothing for
    /// this customer, and the reason its original fixture — which modelled a separate namespace —
    /// was disproved by the walk.
    #[tokio::test]
    async fn its_local_port_table_is_the_identity_mapping() {
        let scan = harness::scan("switch-macport-01").await;

        assert_eq!(scan.local_ports.len(), 10);
        for port in scan.local_ports.keys() {
            assert!(
                (10..=19).contains(port),
                "local port {port} is outside this device's ifIndex range"
            );
            assert!(scan.if_table.entries.iter().any(|e| e.if_index == *port));
        }
        assert_eq!(scan.neighbours.records.len(), 3);
    }
}
