use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

/// GH #685: a neighbour walk that stops part way, having already read every neighbour whole.
///
/// `lldpRemSysDesc` answers nothing by either PDU type, so the walk reads chassis, port and name
/// for all three neighbours and then spends its retries on a column that never replies. What it
/// read is well-formed; what it did not read is the free-text description.
///
/// Before the server stopped vetoing candidates on a partial walk, all three were thrown away and
/// the switch had no physical links. After it, all three were kept and the operator was still told
/// they had not been recorded.
///
/// Not a model of the reporter's Dell OS10 switch, whose walk stops for a reason its scan logs do
/// not yet show. `switch-dell-01` is that device.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-quietcol-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#685",
            defect: "a neighbour walk that stops part way lost every row it had read, and once it kept them the warning still said they were not recorded",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Scanopy SNMP simulator, silent-column profile".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.4".into()),
            sys_name: Some("switch-quietcol-01".into()),
            sys_location: Some("Lab".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Default::default(),
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
        IfRow::port(1, "Gi0/1", Some("00:1a:2b:00:2d:01".parse().unwrap())).name("Gi0/1"),
        IfRow::port(2, "Gi0/2", Some("00:1a:2b:00:2d:02".parse().unwrap())).name("Gi0/2"),
        IfRow::port(3, "Gi0/3", Some("00:1a:2b:00:2d:03".parse().unwrap())).name("Gi0/3"),
    ])
}

/// Three whole neighbours, each with a description the device will never serve.
pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:00:2d:00".into())),
        "switch-quietcol-01",
    )
    .sys_desc("Scanopy SNMP simulator, silent-column profile")
    .column_goes_silent(|columns| columns.sys_desc)
    .local_ports(vec![
        LocalPort::new(
            1,
            Advertised::octets(LldpPortId::InterfaceName("Gi0/1".into())),
        ),
        LocalPort::new(
            2,
            Advertised::octets(LldpPortId::InterfaceName("Gi0/2".into())),
        ),
        LocalPort::new(
            3,
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        ),
    ])
    // Each far end's own chassis id and a port nothing else in the lab is cabled to, so every
    // neighbour lands on a port and no other device's link shares it.
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::octets(LldpChassisId::MacAddress("00:04:38:02:e0:00".into())),
            Advertised::octets(LldpPortId::InterfaceName("1/2".into())),
        )
        .port_desc("1/2")
        .sys_name("switch-voss-01")
        .sys_desc("Extreme Networks VSP-7400, VOSS 8.10"),
        RemoteNeighbour::new(
            2,
            Advertised::octets(LldpChassisId::MacAddress("14:18:77:aa:bb:00".into())),
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/1".into())),
        )
        .port_desc("ethernet1/1/1")
        .sys_name("switch-dell-01")
        .sys_desc("Dell EMC Networking OS10 Enterprise. Dell EMC Networking S4112T-ON. OS Version 10.4.3.4"),
        RemoteNeighbour::new(
            3,
            Advertised::octets(LldpChassisId::MacAddress("00:04:96:01:e0:00".into())),
            Advertised::octets(LldpPortId::InterfaceName("1:2".into())),
        )
        .port_desc("1:2")
        .sys_name("switch-exos-01")
        .sys_desc("ExtremeXOS version 31.7 X435-24P"),
    ])
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use crate::daemon::discovery::integration::snmp::sim::harness::scan;
    use crate::daemon::discovery::service::warnings::{
        ShortfallReason, SnmpCollectionOutcome, SnmpGroupOutcome, SnmpWalkGroup,
        snmp_walk_shortfalls, warn_incomplete_snmp_walks,
    };
    use crate::daemon::discovery::types::warnings::DiscoveryWarning;

    /// The walk stops part way, and stops on a silence rather than on anything the rows did.
    #[tokio::test]
    async fn the_walk_reads_every_neighbour_and_then_stops_on_the_silent_column() {
        let collected = scan("switch-quietcol-01").await;
        let neighbours = &collected.neighbours;

        assert_eq!(neighbours.records.len(), 3);
        assert!(
            !neighbours.complete,
            "the description column never answered, so the walk did not finish"
        );
        assert!(matches!(neighbours.reason, Some(ShortfallReason::NoAnswer)));
        assert_eq!(
            neighbours.discarded, 0,
            "every row carries its chassis id; nothing here is malformed"
        );
        assert!(
            neighbours
                .records
                .iter()
                .all(|n| n.remote_sys_name.is_some() && n.remote_sys_desc.is_none()),
            "every column but the silent one was read"
        );
        assert_eq!(collected.dropped_neighbours, 0);
    }

    /// The operator is told the rows were recorded, because the server records them.
    ///
    /// Fed through the same classifier `execute` uses, with every other group whole so the
    /// neighbour group is the only one with anything to say.
    #[tokio::test]
    async fn the_warning_says_what_was_read_was_recorded() {
        let collected = scan("switch-quietcol-01").await;
        let whole = SnmpGroupOutcome {
            complete: true,
            ..Default::default()
        };
        let outcome = SnmpCollectionOutcome {
            lldp: SnmpGroupOutcome {
                complete: collected.neighbours.complete,
                observed: collected.neighbours.records.len(),
                reason: collected.neighbours.reason,
                claim: None,
            },
            cdp: whole,
            interfaces: whole,
            bridge_port_numbering: whole,
            bridge_forwarding: whole,
            vlan_membership: whole,
            arp_table: whole,
            device_inventory: whole,
            ip_addresses: whole,
            lldp_local_ports: whole,
            vlan_names: whole,
        };

        let address: IpAddr = "192.0.2.1".parse().unwrap();
        let warnings = warn_incomplete_snmp_walks(&snmp_walk_shortfalls(address, outcome));

        assert!(
            matches!(
                warnings.as_slice(),
                [DiscoveryWarning::SnmpWalkPartialRecorded {
                    group: SnmpWalkGroup::Lldp,
                    ..
                }]
            ),
            "a partial neighbour read is recorded as far as it got: {warnings:?}"
        );
    }
}
