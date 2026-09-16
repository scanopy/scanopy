use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::{ArpTable, BridgeTable};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::{ArpEntry, SystemInfo};
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-stuck-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "the walk's retry-then-stop guard",
            defect: "answers every request for its ARP table with the same row, whatever was asked",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Non-advancing agent, ARP table loops".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.3.1".into()),
            sys_name: Some("switch-stuck-01".into()),
            sys_location: Some("Rack 9, middle".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Stuck,
        suppresses: Vec::new(),
        rejects_getbulk: None,
    }
}

fn tables() -> Tables {
    Tables {
        if_table: Some(if_table()),
        bridge: bridge_table(),
        arp: arp_table(),
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(1, "ether1", Some("00:0c:42:7a:00:01".parse().unwrap())).name("ether1"),
        IfRow::port(2, "ether2", Some("00:0c:42:7a:00:02".parse().unwrap())).name("ether2"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

pub fn arp_table() -> ArpTable {
    ArpTable::index_column(vec![ArpEntry {
        if_index: 1,
        mac_address: "00:00:00:00:00:00".parse().unwrap(),
        ip_address: "10.40.50.1".parse().unwrap(),
    }])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// The non-advancing agent the walk's retry-then-stop guard was written for.
    ///
    /// It answers every request for its ARP table with the same row, whatever was asked
    /// (originally a Ubiquiti bridge FDB). Left unguarded the daemon re-requests the same page
    /// until the entry cap or the integration timeout.
    ///
    /// It must report as a *shortfall* — a walk that did not finish — rather than as a device that
    /// quietly has no ARP entries, which is the reading that costs an operator the evidence.
    #[tokio::test]
    async fn a_never_advancing_table_is_reported_short_rather_than_empty() {
        let scan = harness::scan("switch-stuck-01").await;

        assert!(
            !scan.arp.complete,
            "a table that never advances did not finish, and must not read as one that did"
        );
        assert!(
            scan.arp.reason.is_some(),
            "the shortfall must name a cause the operator can act on"
        );
    }

    /// It has an ordinary `ifTable` so that it is a shortfall case rather than a mute one — the
    /// two produce different warnings and must not be confusable.
    #[tokio::test]
    async fn the_rest_of_the_device_reads_normally() {
        let scan = harness::scan("switch-stuck-01").await;

        assert_eq!(scan.if_table.entries.len(), 2);
        assert!(scan.if_table.set_complete && scan.if_table.attributes_complete);
    }
}
