use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{BridgeTable, FdbEntry};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::{BulkRejection, Handler};
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

/// GH #710: a Hikvision DS-3T1512HP-SI-8P4F that answers a GETBULK of more than ten repetitions
/// with `genErr` and the request's own varbind echoed back.
///
/// The walk asks for twenty. snmp2 hands back any response whose request id and community match,
/// whatever its error status, so the echo arrived as a one-row page whose only OID was the column
/// base. That OID sits outside the subtree and does not advance, which is exactly an answer to some
/// other question: the walk re-asked twice and truncated every table with nothing, as
/// `stop=StaleResponse detail="responded with [1, 3, 6, 1, 2, 1, 2, 2, 1, 1]" entries=0`.
/// `snmpbulkwalk`, which asks for ten, read the same switch in full.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-hikvision-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#710",
            defect: "answers a getbulk above ten repetitions with genErr and the request echoed; the walk read the echo as a stale row and truncated every table with nothing",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Hikvision DS-3T1512HP-SI-8P4F, V3.3.3 build 260119".into()),
            sys_object_id: Some("1.3.6.1.4.1.39165.1.1".into()),
            sys_name: Some("switch-hikvision-01".into()),
            sys_location: Some("Camera closet, East wing".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
        rejects_getbulk: Some(BulkRejection {
            above_repetitions: 10,
            error_status: 5,
        }),
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

/// Four ports: an uplink and three PoE ports for cameras.
pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "GigabitEthernet1/0/1",
            Some("00:1a:2b:00:2e:01".parse().unwrap()),
        )
        .name("Gi1/0/1")
        .high_speed()
        .alias("Uplink to switch-core-01"),
        IfRow::port(
            2,
            "GigabitEthernet1/0/2",
            Some("00:1a:2b:00:2e:02".parse().unwrap()),
        )
        .name("Gi1/0/2")
        .high_speed()
        .alias("Camera lobby"),
        IfRow::port(
            3,
            "GigabitEthernet1/0/3",
            Some("00:1a:2b:00:2e:03".parse().unwrap()),
        )
        .name("Gi1/0/3")
        .high_speed()
        .alias("Camera loading dock"),
        IfRow::port(
            4,
            "GigabitEthernet1/0/4",
            Some("00:1a:2b:00:2e:04".parse().unwrap()),
        )
        .name("Gi1/0/4")
        .high_speed()
        .alias("Camera car park"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:2e:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-hikvision-01",
    )
    .sys_desc("Hikvision DS-3T1512HP-SI-8P4F, V3.3.3 build 260119")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/14".into())),
        )
        .port_desc("GigabitEthernet0/14")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
    ])
}

/// The core switch's Gi0/1, learned on the uplink.
pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived().fdb(vec![FdbEntry::learned(
        "00:1a:2b:00:10:01".parse().unwrap(),
        1,
    )])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::queries::SnmpWalkTransport;
    use crate::daemon::discovery::integration::snmp::sim::{device, harness};
    use crate::daemon::discovery::integration::snmp::walk_if_table;

    /// The reporter's log, as an assertion: every table the switch serves is read in full.
    ///
    /// Before the fix each of these came back empty and incomplete, because the refused page was
    /// read as a stale row from the column base.
    #[tokio::test]
    async fn every_table_is_read_from_a_device_that_refuses_a_twenty_row_page() {
        let scan = harness::scan("switch-hikvision-01").await;

        assert_eq!(scan.if_table.entries.len(), 4);
        assert!(
            scan.if_table.set_complete && scan.if_table.attributes_complete,
            "the device answered every page of ten, so its ifTable is authoritative"
        );
        assert_eq!(scan.neighbours.records.len(), 1);
        assert!(scan.neighbours.complete);
        assert_eq!(scan.neighbours.discarded, 0);
        assert_eq!(scan.fdb.records.len(), 1);
        assert!(scan.fdb.complete);
    }

    /// The device refused a page size, not getbulk. The walk settles on a size it accepts and
    /// keeps reading in bulk, rather than dropping to one varbind per round trip for the rest of
    /// the host.
    #[tokio::test]
    async fn a_refused_page_size_does_not_cost_the_session_getbulk() {
        let device = device("switch-hikvision-01");
        let mut agent = device.agent();

        let walk = walk_if_table(&mut agent, device.ip.into()).await.unwrap();

        assert_eq!(walk.entries.len(), 4);
        assert!(
            !agent.getbulk_unusable(),
            "a device that serves pages of ten still serves getbulk"
        );
    }
}
