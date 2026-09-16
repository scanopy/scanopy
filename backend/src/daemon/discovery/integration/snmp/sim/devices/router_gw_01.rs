use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
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

/// This device's identity, for a peer that names it rather than typing its address — see
/// `switch_core_01::cdp_table`.
pub const NAME: &str = "router-gw-01";

pub fn device() -> SimDevice {
    SimDevice {
        name: NAME,
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Control {
            role: "a far end named by switch-core-01's Gi0/2 and by CDP",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("secret42"),
        },
        system: SystemInfo {
            sys_descr: Some("Juniper Networks, Inc. JunOS 21.4R3-S5, MX204".into()),
            sys_object_id: Some("1.3.6.1.4.1.2636.1.1.1.2.29".into()),
            sys_name: Some("router-gw-01".into()),
            sys_location: Some("Server Room A, Rack 3".into()),
            sys_contact: Some("netops@example.com".into()),
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
        lldp: Some(lldp_table()),
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(1, "ge-0/0/0", Some("00:1a:2b:00:12:01".parse().unwrap()))
            .name("ge-0/0/0")
            .high_speed()
            .alias("Uplink to switch-core-01"),
        IfRow::port(2, "ge-0/0/1", Some("00:1a:2b:00:12:02".parse().unwrap()))
            .name("ge-0/0/1")
            .high_speed()
            .alias("Link to firewall-01"),
        IfRow::virtual_if(3, "lo0.0", if_type::SOFTWARE_LOOPBACK)
            .no_hardware_address()
            .name("lo0.0")
            .high_speed()
            .alias("Loopback"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:12:00".into()),
            MacEncoding::AsciiLower,
        ),
        "router-gw-01",
    )
    .sys_desc("Juniper Networks, Inc. JunOS 21.4R3-S5, MX204")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/2".into())),
        )
        .port_desc("GigabitEthernet0/2")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3"),
        RemoteNeighbour::new(
            2,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:13:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("port1".into())),
        )
        .port_desc("port1")
        .sys_name("firewall-01")
        .sys_desc("Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)"),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    use crate::server::interfaces::r#impl::base::if_type;

    /// A control device, and the one that carries a loopback.
    ///
    /// `lo0.0` answers `ifPhysAddress` with an *empty* value rather than omitting the column —
    /// what an interface with no hardware address reports. It must read back as no address, not
    /// as six zero bytes, or a lookup keyed on `00:00:00:00:00:00` would match every loopback in
    /// the estate.
    #[tokio::test]
    async fn its_loopback_reports_no_hardware_address() {
        let scan = harness::scan("router-gw-01").await;

        let loopback = scan.interface(3);
        assert_eq!(loopback.if_type, Some(if_type::SOFTWARE_LOOPBACK));
        assert_eq!(
            loopback.if_phys_address, None,
            "an empty ifPhysAddress is not an address"
        );
        // ...while the real ports do carry one.
        assert!(scan.interface(1).if_phys_address.is_some());
    }
}
