use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::oids::{arp, ip_mib};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

/// This device's identity, for a peer that names it rather than typing its address — see
/// `switch_offsite_01::lldp`.
pub const NAME: &str = "switch-mute-01";

pub fn device() -> SimDevice {
    SimDevice {
        name: NAME,
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "the partial-failure reporting",
            defect: "answers the credential and serves nothing, which used to read to an operator as a clean scan",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Mute agent, system MIB only".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.2.1".into()),
            sys_name: Some("switch-mute-01".into()),
            sys_location: Some("Rack 9, top".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Normal,
        suppresses: vec![
            ip_mib::ip_addr_entry::IP_AD_ENT_ADDR,
            ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
            ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
            arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS,
            arp::entry::IP_NET_TO_MEDIA_TYPE,
        ],
        rejects_getbulk: None,
    }
}

fn tables() -> Tables {
    Tables::default()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// Answers the credential and then serves nothing.
    ///
    /// That is the shape a host takes when SNMP "succeeds" and yields nothing, which used to read
    /// to an operator as a clean scan. Before this device existed, no scan of the environment had
    /// ever produced an incomplete-walk warning for any group, so that whole path went unexercised
    /// while looking healthy.
    ///
    /// The suppressions matter as much as the emptiness: `ipAddrTable` and `ipNetToMediaTable`
    /// cannot be turned off with snmpd's `-I` flag, so without registering them against an empty
    /// file this device would report the host's own addresses and ARP cache and would not be mute.
    #[tokio::test]
    async fn it_answers_and_yields_absolutely_nothing() {
        let scan = harness::scan("switch-mute-01").await;

        assert!(scan.if_table.entries.is_empty());
        assert!(scan.neighbours.records.is_empty());
        assert!(scan.arp.records.is_empty(), "the ARP override must hold");
        assert_eq!(scan.ip_addresses, 0, "the ipAddrTable override must hold");
        assert!(scan.bridge_ports.is_empty());
        assert!(scan.fdb.records.is_empty());
    }

    /// It still sets the datalink bit, so it claims to bridge while serving no bridge MIB. That
    /// contradiction is deliberate and is the lab's only standing one.
    #[tokio::test]
    async fn it_claims_to_bridge_while_serving_no_bridge_table() {
        let device = crate::daemon::discovery::integration::snmp::sim::device("switch-mute-01");
        assert_eq!(device.system.sys_services, Some(2), "the datalink bit");
        assert!(device.tables.bridge.is_empty());
    }
}
