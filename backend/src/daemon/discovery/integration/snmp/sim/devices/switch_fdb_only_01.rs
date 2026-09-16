use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::{BridgeTable, FdbEntry};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

/// GH #709: a switch that serves the bridge forwarding database but no LLDP remote table at
/// all — not a malformed or cut-short walk, `1.0.8802.1.1.2.1.4` simply isn't there. The reporter's
/// TP-Link T1700G-28TQ returns "No such object" for it. `resolve_lldp_links`'s candidate-based
/// filter has nothing to see on this device by design; `resolve_fdb_links` is the only pass that
/// can produce a link for it.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-fdb-only-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#709",
            defect: "serves dot1dTpFdbTable but no LLDP remote table at all, so a port with an unambiguous single-MAC FDB entry never became a physical link before resolve_fdb_links was wired in",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("TP-Link T1700G-28TQ Switch".into()),
            sys_object_id: Some("1.3.6.1.4.1.11863.1.1.10".into()),
            sys_name: Some("switch-fdb-only-01".into()),
            sys_location: Some("Server Room A, Rack 1".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
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
        bridge: bridge_table(),
        // No `lldp` — the whole point of this fixture. `Tables::default()` leaves it `None`, so
        // `1.0.8802.1.1.2.1.4` isn't registered and the daemon reads back "No such object", the
        // same as the reporter's switch.
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "GigabitEthernet0/1",
            Some("00:1a:2b:00:20:01".parse().unwrap()),
        )
        .name("Gi0/1")
        .high_speed()
        .alias("Server port"),
    ])
}

/// One learned MAC on the one port — `switch-core-01`'s Gi0/3, already a real device in the lab,
/// so `find_host_by_mac`/`find_if_entry_by_mac` resolve against genuine fixture data rather than
/// an address nothing else in the network recognises.
pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived().fdb(vec![FdbEntry::learned(
        "00:1a:2b:00:10:03".parse().unwrap(),
        1,
    )])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// The negative space the fix depends on: this device answers its credential and serves a
    /// real if_table and FDB, but has no LLDP remote table to walk at all.
    #[tokio::test]
    async fn it_serves_fdb_but_no_lldp_remote_table() {
        let scan = harness::scan("switch-fdb-only-01").await;

        assert!(scan.if_table.set_complete && scan.if_table.attributes_complete);
        assert_eq!(scan.fdb.records.len(), 1);
        assert_eq!(
            scan.fdb.records[0].mac_address.to_string().to_lowercase(),
            "00:1a:2b:00:10:03"
        );

        assert_eq!(
            scan.neighbours.records.len(),
            0,
            "no LLDP remote table at all, not an empty or cut-short one"
        );
    }
}
