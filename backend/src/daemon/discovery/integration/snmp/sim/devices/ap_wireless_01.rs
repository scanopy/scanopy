use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::allocation::SHARED_NETMASK;
use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{
    BridgeTable, IpAddrRow, IpAddrTable, OwnAddress,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "ap-wireless-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#663",
            defect: "an access point's NAT guest network read as a Docker bridge",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Ubiquiti UniFi AP AC Pro, firmware 6.5.28".into()),
            sys_object_id: Some("1.3.6.1.4.1.41112.1.6.1".into()),
            sys_name: Some("ap-wireless-01".into()),
            sys_location: Some("Floor 3, Ceiling".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(6),
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
        ip_addr: ip_addr_table(),
        // Unlike the shared-MAC switches, eth0 is unambiguous here — every interface has its own
        // distinct MAC — so this device's own address can honestly bind to it.
        own_address: OwnAddress {
            if_index: 1,
            netmask: SHARED_NETMASK,
        },
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(1, "eth0", Some("00:1a:2b:00:15:01".parse().unwrap()))
            .name("eth0")
            .high_speed()
            .alias("Uplink to switch-access-01"),
        IfRow::virtual_if(2, "ath0", if_type::IEEE80211)
            .mac("00:1a:2b:00:15:02".parse().unwrap())
            .name("ath0")
            .high_speed_mbps(867)
            .alias("5GHz radio"),
        IfRow::virtual_if(3, "ath1", if_type::IEEE80211)
            .mac("00:1a:2b:00:15:03".parse().unwrap())
            .name("ath1")
            .high_speed_mbps(400)
            .alias("2.4GHz radio"),
        IfRow::virtual_if(4, "br-guest", if_type::BRIDGE)
            .mac("00:1a:2b:00:15:04".parse().unwrap())
            .name("br-guest")
            .high_speed()
            .alias("NAT guest network"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:15:00".into()),
            MacEncoding::AsciiLower,
        ),
        "ap-wireless-01",
    )
    .sys_desc("Ubiquiti UniFi AP AC Pro, firmware 6.5.28")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-access-01")
        .sys_desc("Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

/// The guest-subnet address alone — the `#663` fixture. This device's own address is served
/// automatically, alongside this one, by [`super::super::SimDevice::data_files`].
pub fn ip_addr_table() -> IpAddrTable {
    IpAddrTable {
        rows: vec![IpAddrRow {
            address: "172.30.10.1".parse().unwrap(),
            if_index: 4,
            netmask: "255.255.255.0".parse().unwrap(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// GH #663: an access point's NAT guest network was read as a Docker bridge.
    ///
    /// This is the only device declaring an *extra* `ipAddrTable` row beyond its own, which is
    /// the only way it can advertise a subnet the VM's kernel does not have. If the per-column
    /// override ever lost its duplicate registration against net-snmp's built-in IP module, the
    /// agent would quietly answer from the host's own addresses instead and this device's guest
    /// row would silently vanish while every other device kept working.
    #[tokio::test]
    async fn it_advertises_a_guest_subnet_the_host_does_not_have() {
        let scan = harness::scan("ap-wireless-01").await;

        assert_eq!(scan.ip_addresses, 2, "the guest address and its own");
        let guest = scan.interface(4);
        assert_eq!(guest.if_name.as_deref(), Some("br-guest"));
    }

    /// A radio reports `0` in the 32-bit speed column and its real rate in `ifHighSpeed`, so the
    /// two genuinely disagree here. Every other port in the lab derives one from the other; this
    /// is the exception that proves the derivation is not being applied blindly.
    #[tokio::test]
    async fn a_radios_rate_comes_from_the_high_speed_column() {
        let scan = harness::scan("ap-wireless-01").await;
        assert_eq!(scan.interface(2).if_speed, Some(867_000_000));
    }
}
