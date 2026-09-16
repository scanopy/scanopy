use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

/// GH #668's switch3: a device whose neighbour columns cannot be read in bulk and can be read one
/// varbind at a time.
///
/// The reporter's log is three getbulk timeouts five seconds apart on `lldpRemChassisId`, then
/// `entries=0`, then all thirteen neighbours discarded for want of the mandatory chassis id — and
/// `snmpwalk`, which is GETNEXT, reading the same column from the same switch without trouble.
/// Nothing was wrong with the device that a different question would not have answered.
///
/// The walk could not ask it. `shrink_page` halves the page and only gives up on getbulk once the
/// page is down to 1, which from `BULK_MAX_REPETITIONS` of 20 takes five halvings, while
/// `MAX_TRANSPORT_RETRIES` allows two — so the fallback the function exists to provide sat three
/// shrinks out of reach and the column was abandoned having never tried getnext.
///
/// The local port table answers normally here, on purpose: this device isolates the neighbour
/// read. `switch-shortports-01` is the other half, where the port table is what falls short.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-slowbulk-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#668",
            defect: "getbulk on the LLDP remote tables never answers while getnext does; before the fallback the chassis-id column returned nothing and every neighbour was discarded, so the switch reported no LLDP at all",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Scanopy SNMP simulator, slow-getbulk profile".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.3".into()),
            sys_name: Some("switch-slowbulk-01".into()),
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

/// Three ports, and no more. On the deployed unit every varbind of the LLDP subtree costs the
/// handler's `sleep 3`, so the whole neighbour read is `3s * varbinds` and has to finish inside
/// `SNMP_WALK_TIMEOUT` (60s). Three neighbours across seven columns is comfortably under it; a
/// switch-sized table here would make the fixture fail for being slow rather than for being wrong.
pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(1, "Gi0/1", Some("00:1a:2b:00:2c:01".parse().unwrap())).name("Gi0/1"),
        IfRow::port(2, "Gi0/2", Some("00:1a:2b:00:2c:02".parse().unwrap())).name("Gi0/2"),
        IfRow::port(3, "Gi0/3", Some("00:1a:2b:00:2c:03".parse().unwrap())).name("Gi0/3"),
    ])
}

/// Neighbours reachable only by getnext, on ports the local table names normally.
///
/// Every neighbour carries a complete chassis id. That is what makes the assertion sharp: nothing
/// about this table is malformed, so a neighbour going missing can only be the read.
pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:2c:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-slowbulk-01",
    )
    .sys_desc("Scanopy SNMP simulator, slow-getbulk profile")
    .neighbours_refuse_getbulk()
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
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/11".into())),
        )
        .port_desc("GigabitEthernet0/11")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            2,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/12".into())),
        )
        .port_desc("GigabitEthernet0/12")
        .sys_name("switch-access-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            3,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:12:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("1/1".into())),
        )
        .port_desc("1/1")
        .sys_name("switch-voss-01")
        .sys_desc("Extreme Networks VSP-7400, VOSS 8.10"),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness::scan;

    /// The reporter's line, as an assertion: three neighbours advertised, three read.
    ///
    /// Before the getnext fallback this fails with `neighbors=0 complete=false`, because the
    /// chassis-id column returned nothing and a neighbour without one is discarded — which is
    /// `switch-3-4-7_log.txt:47-50` exactly.
    #[tokio::test]
    async fn a_device_that_cannot_be_read_in_bulk_still_yields_its_neighbours() {
        let collected = scan("switch-slowbulk-01").await;

        assert_eq!(
            collected.neighbours.records.len(),
            3,
            "every neighbour the device advertises has to survive a read it could only answer one \
             varbind at a time"
        );
        assert!(
            collected.neighbours.complete,
            "the walk reached the end of every column, so the neighbour set is authoritative — \
             reporting it partial is what stops the server recording it"
        );
        assert_eq!(
            collected.neighbours.discarded, 0,
            "nothing about this table is malformed; a discard here means the chassis column was \
             never read"
        );
    }

    /// The neighbours land on real interfaces, so the device contributes links rather than merely
    /// being read. A chassis id read but placed nowhere is the same blank on the map.
    #[tokio::test]
    async fn the_neighbours_reach_the_interfaces_they_sit_on() {
        let collected = scan("switch-slowbulk-01").await;

        assert_eq!(collected.dropped_neighbours, 0);
        assert_eq!(
            collected.neighbour_named("switch-core-01").local_port_index,
            1
        );
        assert_eq!(
            collected
                .neighbour_named("switch-access-01")
                .local_port_index,
            2
        );
        assert_eq!(
            collected.neighbour_named("switch-voss-01").local_port_index,
            3
        );
    }

    /// The rest of the device is not slow, and must not be read as though it were. The reporter's
    /// switch served its ifTable in full on the same scan that lost its neighbours.
    #[tokio::test]
    async fn the_tables_served_normally_are_unaffected() {
        let collected = scan("switch-slowbulk-01").await;

        assert_eq!(collected.if_table.entries.len(), 3);
        assert!(collected.if_table.set_complete);
        assert_eq!(collected.local_ports.len(), 3);
    }
}
