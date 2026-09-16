use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

/// GH #668's switch4: neighbours that cannot be placed because the table naming the ports was
/// only half read, reported to the operator as a switch that numbers its ports oddly.
///
/// The panel told the reporter their neighbours "sat on a local port that matches no interface on
/// the device … This usually means the switch numbers its LLDP ports separately from its
/// interfaces, or did not answer for its LLDP port table." Both halves were offered and the
/// operator had no way to tell which applied — on a device whose `lldpLocPortDesc` walk truncated
/// at `entries=280` in the same log, the first half is simply a wrong diagnosis.
///
/// The D-Link shape is what makes the truncation cost anything. It reports the chassis address on
/// every interface, so `unique_interface_macs` drops it as ambiguous and the MAC tier declines;
/// its LLDP port ids are a numbering of their own that matches no ifIndex; and what is left is
/// `lldpLocPortDesc`, matched against `ifDescr`. That is the column the read loses, so losing it
/// loses the placement.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-shortports-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#668",
            defect: "the lldpLocPortTable read stops part way, so neighbours cannot be placed — and the operator is told the device numbers its LLDP ports separately, which is a different fault with a different fix",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("D-Link DGS-1510-52X Gigabit Ethernet SmartPro Switch".into()),
            sys_object_id: Some("1.3.6.1.4.1.171.10.137.8.1".into()),
            sys_name: Some("switch-shortports-01".into()),
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
        ..Default::default()
    }
}

/// One MAC across every port, as the reporter's DGS-1510-52X reports.
///
/// This is not decoration: it is what disqualifies the MAC tier. `unique_interface_macs` drops an
/// address held by more than one interface rather than arbitrating between them, so a device like
/// this has only its port names and descriptions left to be placed by — and one of those is the
/// column that goes unread.
pub fn if_table() -> IfTable {
    let chassis = "00:ad:24:89:cc:f0".parse().unwrap();
    IfTable::new(vec![
        IfRow::port(1, "eth1/0/1", Some(chassis)).name("eth1/0/1"),
        IfRow::port(2, "eth1/0/2", Some(chassis)).name("eth1/0/2"),
        IfRow::port(3, "eth1/0/3", Some(chassis)).name("eth1/0/3"),
        IfRow::port(4, "eth1/0/4", Some(chassis)).name("eth1/0/4"),
    ])
}

/// Local ports numbered in a space of their own, served by a handler that never advances.
///
/// `lldpLocPortNum` runs from 1001, matching no ifIndex, so the identity path places nothing and
/// the port id — `local(7)`, free text with no relation to the interface — places nothing either.
/// `desc` is the only column that names the interface, and the stuck handler means the walk reads
/// the first row and then goes in circles until the non-advancing guard cuts it short.
pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:ad:24:89:cc:f0".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-shortports-01",
    )
    .sys_desc("D-Link DGS-1510-52X Gigabit Ethernet SmartPro Switch")
    .local_ports_served_by(Handler::Stuck)
    .local_ports(vec![
        LocalPort::new(
            1001,
            Advertised::octets(LldpPortId::LocallyAssigned("1001".into())),
        )
        .desc("eth1/0/1"),
        LocalPort::new(
            1002,
            Advertised::octets(LldpPortId::LocallyAssigned("1002".into())),
        )
        .desc("eth1/0/2"),
        LocalPort::new(
            1003,
            Advertised::octets(LldpPortId::LocallyAssigned("1003".into())),
        )
        .desc("eth1/0/3"),
        LocalPort::new(
            1004,
            Advertised::octets(LldpPortId::LocallyAssigned("1004".into())),
        )
        .desc("eth1/0/4"),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            1002,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/21".into())),
        )
        .port_desc("GigabitEthernet0/21")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            1003,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/22".into())),
        )
        .port_desc("GigabitEthernet0/22")
        .sys_name("switch-access-01")
        .sys_desc("Cisco IOS Software, C2960"),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness::scan;

    /// The device this fixture exists for: the port table did not finish, and the neighbours it
    /// would have placed are lost.
    ///
    /// Asserted before the reporting change so the fixture is known to reproduce the situation
    /// rather than only the reporting of it. Both halves have to be true for the warning to be
    /// worth anything — a device that dropped nothing needs no warning, and one whose read
    /// finished needs the *other* warning.
    #[tokio::test]
    async fn a_port_table_that_stops_part_way_loses_the_neighbours_it_would_have_placed() {
        let collected = scan("switch-shortports-01").await;

        assert!(
            !collected.local_ports_complete,
            "the stuck handler cuts the port table short, and a read that finished would make \
             this a numbering fault instead"
        );
        assert!(
            collected.dropped_neighbours > 0,
            "a truncated port table has to actually cost a placement, or the warning it drives \
             describes nothing"
        );
    }

    /// The reporting half, and the reason this fixture is a D-Link: the operator must be told the
    /// read stopped, not that the switch numbers its LLDP ports separately from its interfaces.
    ///
    /// Both sentences are plausible and only one is true here, and the panel offered both at once
    /// — `panel-0827.png`, the bullet covering switch4. A device whose port table demonstrably did
    /// not finish is not evidence of a numbering scheme; it is evidence of a read that stopped,
    /// and the two point an operator at different things.
    #[tokio::test]
    async fn a_truncated_port_table_is_reported_as_a_read_that_stopped_not_as_port_numbering() {
        use crate::daemon::discovery::integration::snmp::LocalPortPlacementReason;

        let collected = scan("switch-shortports-01").await;
        let reason = LocalPortPlacementReason::from_reads(
            collected.local_ports_complete,
            collected.if_table.set_complete,
        );

        assert_eq!(
            reason,
            LocalPortPlacementReason::ReadCutShort,
            "the port table did not finish, so the placement failure is attributable to the read"
        );
    }

    /// The interface table is ordinary, so the shortfall is attributable to the port table alone.
    /// A device short in both would not tell the two branches of the fix apart.
    #[tokio::test]
    async fn the_interface_table_is_whole() {
        let collected = scan("switch-shortports-01").await;

        assert_eq!(collected.if_table.entries.len(), 4);
        assert!(collected.if_table.set_complete);
    }
}
