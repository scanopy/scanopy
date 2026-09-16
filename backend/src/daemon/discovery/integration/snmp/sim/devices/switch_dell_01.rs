use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour, TimeMark,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{BridgeTable, FdbEntry};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfNumber, IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-dell-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#685",
            defect: "OS10 breakout port names carry both anchor characters, and lldpLocPortNum is a separate namespace numbering the management port 4 and the front panel from 555",
        },
        credential: CredentialType::SnmpV2c { community: inline("netdefault") },
        system: SystemInfo {
            sys_descr: Some("Dell EMC Networking OS10 Enterprise. Dell EMC Networking S4112T-ON. OS Version 10.4.3.4".into()),
            sys_object_id: Some("1.3.6.1.4.1.674.11000.5000.100.2.1".into()),
            sys_name: Some("switch-dell-01".into()),
            sys_location: Some("Rack 4, breakout panel".into()),
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
            .mtu(65535)
            .name("lo")
            .high_speed(),
        IfRow::port(
            17301505,
            "ethernet1/1/1",
            Some("14:18:77:aa:bb:11".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/1")
        .high_speed(),
        IfRow::port(
            17301508,
            "ethernet1/1/4",
            Some("14:18:77:aa:bb:14".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/4")
        .high_speed(),
        IfRow::port(
            17301518,
            "ethernet1/1/14:1",
            Some("14:18:77:aa:bb:21".parse().unwrap()),
        )
        .mtu(1532)
        .speed(25000000000)
        .name("ethernet1/1/14:1")
        .high_speed()
        .alias("breakout lane 1"),
        IfRow::port(
            17301519,
            "ethernet1/1/14:2",
            Some("14:18:77:aa:bb:22".parse().unwrap()),
        )
        .mtu(1532)
        .speed(25000000000)
        .name("ethernet1/1/14:2")
        .high_speed()
        .alias("breakout lane 2"),
        IfRow::port(
            17301520,
            "ethernet1/1/14:3",
            Some("14:18:77:aa:bb:23".parse().unwrap()),
        )
        .mtu(1532)
        .speed(25000000000)
        .name("ethernet1/1/14:3")
        .high_speed()
        .alias("breakout lane 3"),
        IfRow::port(
            35127296,
            "mgmt1/1/1",
            Some("14:18:77:aa:bb:01".parse().unwrap()),
        )
        .mtu(1532)
        .name("mgmt1/1/1")
        .high_speed()
        .alias("out of band"),
    ])
    .declaring(IfNumber::Declares(52))
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress("14:18:77:aa:bb:00".into())),
        "switch-dell-01",
    )
    .sys_desc(
        "Dell EMC Networking OS10 Enterprise. Dell EMC Networking S4112T-ON. OS Version 10.4.3.4",
    )
    .local_ports(vec![
        LocalPort::new(
            4,
            Advertised::octets(LldpPortId::InterfaceName("mgmt1/1/1".into())),
        )
        .desc("mgmt1/1/1"),
        LocalPort::new(
            555,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/1".into())),
        )
        .desc("ethernet1/1/1"),
        LocalPort::new(
            558,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/4".into())),
        )
        .desc("ethernet1/1/4"),
        LocalPort::new(
            568,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/14:1".into())),
        )
        .desc("ethernet1/1/14:1"),
        LocalPort::new(
            569,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/14:2".into())),
        )
        .desc("ethernet1/1/14:2"),
        LocalPort::new(
            570,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/14:3".into())),
        )
        .desc("ethernet1/1/14:3"),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            570,
            Advertised::octets(LldpChassisId::LocallyAssigned("TAMMIERENEW".into())),
            Advertised::octets(LldpPortId::MacAddress("9c:6b:00:41:8d:21".into())),
        )
        .time_mark(TimeMark::At(31577700))
        .index(55)
        .port_desc("Realtek PCIe GbE Family Controller")
        .sys_name("TAMMIERENEW")
        .sys_desc("Windows 11 Pro 10.0.26100 x64"),
        RemoteNeighbour::new(
            4,
            Advertised::octets(LldpChassisId::MacAddress("f6:6b:d4:b4:b9:df".into())),
            Advertised::octets(LldpPortId::MacAddress("f6:6b:d4:b4:b9:df".into())),
        )
        .time_mark(TimeMark::At(93300700))
        .index(78),
        RemoteNeighbour::new(
            568,
            Advertised::octets(LldpChassisId::LocallyAssigned("EVILCORP".into())),
            Advertised::octets(LldpPortId::MacAddress("3c:ec:ef:40:12:aa".into())),
        )
        .time_mark(TimeMark::At(123380800))
        .index(85)
        .port_desc("Intel(R) Ethernet Controller X550")
        .sys_name("EVILCORP")
        .sys_desc("Ubuntu 24.04.1 LTS Linux 6.8.0-51-generic x86_64"),
        RemoteNeighbour::new(
            569,
            Advertised::octets(LldpChassisId::LocallyAssigned("VIRTUALPC".into())),
            Advertised::octets(LldpPortId::MacAddress("00:15:5d:01:64:0c".into())),
        )
        .time_mark(TimeMark::At(127153800))
        .index(87)
        .port_desc("Hyper-V Virtual Ethernet Adapter")
        .sys_name("VIRTUALPC")
        .sys_desc("Windows Server 2022 Datacenter 10.0.20348 x64"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::with_ports(vec![
        (1, 17301505),
        (2, 17301506),
        (3, 17301507),
        (4, 17301508),
        (10, 17301514),
        (11, 17301515),
        (12, 17301516),
    ])
    .fdb(vec![
        FdbEntry::learned("00:1a:2b:00:10:01".parse().unwrap(), 1),
        FdbEntry::learned("00:1a:2b:00:11:01".parse().unwrap(), 1),
        FdbEntry::learned("00:1a:2b:00:12:01".parse().unwrap(), 2),
        FdbEntry::learned("00:1a:2b:00:13:01".parse().unwrap(), 3),
        FdbEntry::learned("00:1a:2b:00:14:01".parse().unwrap(), 4),
        FdbEntry::learned("00:1a:2b:00:15:01".parse().unwrap(), 4),
        FdbEntry::learned("14:18:77:aa:bb:11".parse().unwrap(), 10),
        FdbEntry::learned("14:18:77:aa:bb:12".parse().unwrap(), 11),
        FdbEntry::learned("14:18:77:aa:bb:13".parse().unwrap(), 12),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// GH #685: a Dell PowerSwitch S4112T-ON running OS10, which discovered cleanly in every other
    /// respect and showed no physical connections at all.
    ///
    /// `lldpLocPortNum` is a separate namespace from `ifIndex`, and not a small one: the management
    /// port is 4 and the front panel runs 555-570, against ifIndex values in the millions. This is
    /// the mapping the reporter published, and a neighbour on any other port is the bug, not a
    /// near miss.
    #[tokio::test]
    async fn its_breakout_neighbours_land_on_the_ports_the_switch_names() {
        let scan = harness::scan("switch-dell-01").await;

        assert_eq!(scan.neighbours.records.len(), 4);

        // The names that make this device the case it is: a bare `1` ends at a `/` or `:`
        // boundary in three places at once, so the suffix tier has to require the boundary to
        // name exactly one interface rather than taking the first match.
        let names: Vec<&str> = scan
            .local_ports
            .values()
            .filter_map(|port| port.port_id.as_deref())
            .collect();
        for required in ["mgmt1/1/1", "ethernet1/1/1", "ethernet1/1/14:1"] {
            assert!(
                names.contains(&required),
                "{required} is what makes the boundary ambiguous; without it this device is an \
                 ordinary switch: {names:?}"
            );
        }

        // Each neighbour reached a real interface rather than an index no interface holds.
        for neighbour in &scan.neighbours.records {
            let port = neighbour.local_port_index;
            assert!(
                scan.if_table.entries.iter().any(|e| e.if_index == port),
                "a neighbour landed on {port}, which names no interface"
            );
        }

        // The three breakout lanes are distinct ports, so the three end hosts must not collapse
        // onto one: taking the first match bound neighbours to a plausible-looking wrong port.
        let mut ports: Vec<i32> = scan
            .neighbours
            .records
            .iter()
            .map(|n| n.local_port_index)
            .collect();
        ports.sort_unstable();
        ports.dedup();
        assert_eq!(ports.len(), 4, "four neighbours, four distinct local ports");
    }

    /// It declares 52 interfaces and serves 23, deliberately.
    ///
    /// Every other device agrees with itself, which demonstrates the count check staying quiet but
    /// cannot demonstrate it firing — and a guard nobody has watched fire is a guard nobody knows
    /// works. A scan must still record every row it serves: a device that misreports itself is
    /// still a device to scan.
    #[tokio::test]
    async fn it_misreports_its_own_interface_count_on_purpose() {
        let scan = harness::scan("switch-dell-01").await;

        let claimed = scan.system.if_number.expect("it publishes a count") as usize;
        let served = scan.if_table.entries.len();

        // The contradiction, not the two numbers. Trimming this fixture to the ports it exists to
        // prove is a legitimate edit and must not fail a test about the count check.
        assert!(
            claimed > served,
            "this device must claim more than it serves, or the count check cannot be watched \
             firing: claims {claimed}, serves {served}"
        );
        assert!(
            scan.if_table.set_complete,
            "the walk itself is complete — the contradiction is the device's, not the read's"
        );
        assert!(
            served > 0,
            "a device that misreports itself is still a device to scan"
        );
    }
}
