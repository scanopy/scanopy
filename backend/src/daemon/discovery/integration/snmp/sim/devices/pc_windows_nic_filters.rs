use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::OwnAddress;
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

const SHARED_NIC_MAC: &str = "50:eb:f6:26:54:79";

/// GH #668: a Windows host's SNMP agent (the built-in Windows SNMP service) reports the identical
/// `ifPhysAddress` on the real NIC and on every NDIS filter/LWF pseudo-interface layered on top of
/// it — WFP Native MAC Layer, QoS Packet Scheduler, WFP 802.3. All four report an ordinary
/// ethernet `if_type`, so `physical_if_types()`'s exclusion (built for VLAN/loopback/bridge rows)
/// does not separate them, and a neighbour naming this MAC as its LLDP port id resolves to the
/// host but not to a specific port.
///
/// The one thing that does separate them, confirmed against the reporting customer's own debug
/// log (not assumed): `ipAddrTable` binds the host's IP to the real NIC's ifIndex only — Windows
/// never lets a filter driver appear as a distinct IP-configurable adapter. So the real NIC (7) is
/// `ip_addr`-bound here and the three filter ifIndexes (18-20, matching the customer's own
/// `if_index` values) are not.
pub fn device() -> SimDevice {
    SimDevice {
        name: "pc-windows-nic-filters",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#668",
            defect: "one MAC reported on a real NIC and its NDIS filter/LWF pseudo-interfaces makes LLDP port resolution ambiguous",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("public"),
        },
        system: SystemInfo {
            sys_descr: Some("Hardware: x64 Family 25 Model 33 Stepping 0 AT/AT COMPATIBLE - Software: Windows Version 10.0 (Build 19045 Multiprocessor Free)".into()),
            sys_object_id: Some("1.3.6.1.4.1.311.1.1.3.1.1".into()),
            sys_name: Some("pc-windows-nic-filters".into()),
            sys_location: None,
            sys_contact: None,
            sys_services: Some(76),
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
        // Only the real NIC's ifIndex — never a filter driver's, on real Windows. The address
        // itself is synthesised automatically; this only overrides which ifIndex it binds to.
        own_address: OwnAddress {
            if_index: 7,
            netmask: "255.255.255.0".parse().unwrap(),
        },
        // No LLDP local table: a stock Windows SNMP service does not answer lldpLocalSystemData
        // about itself. The neighbour evidence in this scenario comes entirely from what
        // `switch-dlink-02` reports having heard on its own port — this device only needs to be
        // scannable directly, the same way the real PC-069 was.
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    let mac = SHARED_NIC_MAC.parse().unwrap();
    IfTable::new(vec![
        IfRow::port(7, "Realtek Gaming 2.5GbE Family Controller", Some(mac))
            .name("ethernet_32768")
            .speed(2_500_000_000)
            .high_speed(),
        IfRow::port(
            18,
            "Realtek Gaming 2.5GbE Family Controller-WFP Native MAC Layer LightWeight Filter-0000",
            Some(mac),
        )
        .name("ethernet_0")
        .speed(2_500_000_000)
        .high_speed(),
        IfRow::port(
            19,
            "Realtek Gaming 2.5GbE Family Controller-QoS Packet Scheduler-0000",
            Some(mac),
        )
        .name("ethernet_1")
        .speed(2_500_000_000)
        .high_speed(),
        IfRow::port(
            20,
            "Realtek Gaming 2.5GbE Family Controller-WFP 802.3 MAC Layer LightWeight Filter-0000",
            Some(mac),
        )
        .name("ethernet_2")
        .speed(2_500_000_000)
        .high_speed(),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;
    use crate::daemon::discovery::integration::snmp::unique_interface_macs;

    /// The shape the bug needs: four interfaces, one MAC, none of them individually unique.
    #[tokio::test]
    async fn four_interfaces_share_one_mac() {
        let scan = harness::scan("pc-windows-nic-filters").await;

        let addresses: Vec<String> = scan
            .if_table
            .entries
            .iter()
            .filter_map(|e| e.if_phys_address.map(|m| m.to_string().to_lowercase()))
            .collect();
        assert_eq!(addresses.len(), 4);
        assert!(
            addresses.iter().all(|mac| mac == "50:eb:f6:26:54:79"),
            "all four rows must share the real NIC's MAC: {addresses:?}"
        );
        assert!(
            unique_interface_macs(&scan.if_table.entries).is_empty(),
            "a MAC on every candidate interface must identify none of them by uniqueness alone"
        );
    }

    /// The signal the fix depends on: `ipAddrTable` names ifIndex 7 (the real NIC) only. The other
    /// three ifIndexes never appear in it, matching the customer's own `ipAddrTable MAC enrichment`
    /// log line firing for `if_index=7` alone.
    #[tokio::test]
    async fn only_the_real_nic_is_ip_bound() {
        use std::net::IpAddr;

        let device =
            crate::daemon::discovery::integration::snmp::sim::device("pc-windows-nic-filters");
        let scan = harness::collect(&device).await;
        let addr = IpAddr::V4(device.ip);
        assert_eq!(scan.ip_addr_table.len(), 1);
        assert_eq!(scan.ip_addr_table.get(&addr).map(|e| e.if_index), Some(7));
    }
}
