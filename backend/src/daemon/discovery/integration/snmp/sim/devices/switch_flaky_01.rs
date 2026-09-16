use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, ChassisDefect, LldpTable, RemoteNeighbour,
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
        name: "switch-flaky-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#668",
            defect: "malformed neighbour records — each variant drives a different discard counter and a different piece of operator advice",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Scanopy SNMP simulator, flaky-LLDP profile".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.1".into()),
            sys_name: Some("switch-flaky-01".into()),
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
        lldp: Some(lldp_complete()),
        bridge: bridge_table(),
        lldp_variants: vec![
            ("badsubtype", lldp_badsubtype()),
            ("complete", lldp_complete()),
            ("ghost", lldp_ghost()),
            ("nochassis", lldp_nochassis()),
            ("nosubtype", lldp_nosubtype()),
        ],
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(1, "uplink0", Some("00:1a:2b:00:1f:01".parse().unwrap())).name("uplink0"),
        IfRow::port(2, "uplink1", Some("00:1a:2b:00:1f:02".parse().unwrap())).name("uplink1"),
    ])
}

pub fn lldp_complete() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
    ])
}

pub fn lldp_nochassis() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::octets(LldpChassisId::MacAddress("00:00:00:00:00:00".into())),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960")
        .defect(ChassisDefect::NoChassisColumns),
    ])
}

pub fn lldp_nosubtype() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960")
        .defect(ChassisDefect::NoSubtype),
    ])
}

pub fn lldp_badsubtype() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960")
        .defect(ChassisDefect::SubtypeWrongType("macAddress")),
    ])
}

pub fn lldp_ghost() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            2,
            Advertised::octets(LldpChassisId::MacAddress("00:00:00:00:00:00".into())),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/4".into())),
        )
        .port_desc("GigabitEthernet0/4")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960")
        .defect(ChassisDefect::NoChassisColumns),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {

    use crate::daemon::discovery::integration::snmp::sim::device;
    use crate::daemon::discovery::integration::snmp::sim::lldp::LldpTable;
    use crate::daemon::discovery::integration::snmp::sim::transport::SimAgent;
    use crate::daemon::discovery::integration::snmp::sim::wire::{DataFile, Ordering};
    use crate::daemon::discovery::integration::snmp::types::LldpNeighbor;
    use crate::daemon::discovery::integration::snmp::{SnmpCollection, query_lldp_neighbors};
    use crate::daemon::discovery::service::warnings::MalformedNeighbourReason;

    /// Serve one of the swappable variants and read its neighbours, the way the deployed device
    /// does when a variant is copied over `-lldp-active.txt`.
    async fn serve(variant: &str) -> SnmpCollection<Vec<LldpNeighbor>> {
        let flaky = device("switch-flaky-01");
        let table: &LldpTable = &flaky
            .tables
            .lldp_variants
            .iter()
            .find(|(name, _)| *name == variant)
            .unwrap_or_else(|| panic!("no {variant} variant"))
            .1;
        let file = DataFile::new("active", Ordering::Ascending, table.wire_rows());
        let mut agent = SimAgent::new(&[file], flaky.registrations_for_lldp_only());
        query_lldp_neighbors(&mut agent, flaky.ip.into())
            .await
            .expect("the walk runs")
    }

    /// The well-formed variant, and the baseline the other four are read against.
    #[tokio::test]
    async fn the_complete_variant_discards_nothing() {
        let neighbours = serve("complete").await;
        assert_eq!(neighbours.records.len(), 1);
        assert_eq!(neighbours.discarded, 0);
        assert_eq!(neighbours.discard_reason, None);
    }

    /// GH #668: a chassis column listing none of a row's positions is indistinguishable from one
    /// that never had them, so both read as ghost rows. This variant keeps *nothing* — the device
    /// contributes no physical links at all, which is a different place on the map from
    /// contributing some.
    #[tokio::test]
    async fn the_chassis_less_variant_keeps_nothing_and_says_why() {
        let neighbours = serve("nochassis").await;
        assert!(neighbours.records.is_empty(), "kept=0");
        assert!(neighbours.discarded > 0);
        assert_eq!(
            neighbours.discard_reason,
            Some(MalformedNeighbourReason::GhostRows)
        );
    }

    /// The sparse-chassis-column shape: one row complete, one listed only in the later columns.
    /// It reads as the same cause as above and differs in `kept`, which is what decides whether
    /// the warning says the device contributes nothing or only that some links are missing.
    #[tokio::test]
    async fn the_ghost_variant_keeps_the_row_that_is_whole() {
        let neighbours = serve("ghost").await;
        assert_eq!(neighbours.records.len(), 1, "kept=1");
        assert!(neighbours.discarded > 0);
        assert_eq!(
            neighbours.discard_reason,
            Some(MalformedNeighbourReason::GhostRows)
        );
    }

    /// A chassis id with no subtype: the record is incomplete rather than absent, and a rescan
    /// will not help.
    #[tokio::test]
    async fn the_subtype_less_variant_reports_an_incomplete_record() {
        let neighbours = serve("nosubtype").await;
        assert!(neighbours.discarded > 0);
        assert_eq!(
            neighbours.discard_reason,
            Some(MalformedNeighbourReason::IncompleteRecords)
        );
    }

    /// The one that matters most. The subtype arrives as an OCTET STRING where an INTEGER belongs,
    /// so the walk reads as *complete* — no truncation signal anywhere — and before the per-cause
    /// counters the only evidence was the record silently going missing.
    #[tokio::test]
    async fn the_wrong_typed_subtype_is_reported_rather_than_vanishing() {
        let neighbours = serve("badsubtype").await;

        assert!(neighbours.discarded > 0);
        assert_eq!(
            neighbours.discard_reason,
            Some(MalformedNeighbourReason::UnexpectedType),
            "the agent answered every column, so nothing about the read was short — the cause is \
             what it put in one of them"
        );
        assert_ne!(
            neighbours.discard_reason,
            Some(MalformedNeighbourReason::WalkCutShort),
            "reporting truncation here tells an operator to rescan a firmware defect that will \
             answer identically forever"
        );
    }
}
