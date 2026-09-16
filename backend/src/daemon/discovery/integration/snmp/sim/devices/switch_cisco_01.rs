use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::{BridgeTable, FdbEntry};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::{
    CredentialType, SnmpV3AuthProtocol, SnmpV3PrivProtocol,
};
use crate::server::interfaces::r#impl::base::if_type;

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-cisco-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#686",
            defect: "IOS-XE partitions its forwarding database per VLAN, so a scan that cannot name a context reads the wrong table and is told nothing is wrong",
        },
        credential: CredentialType::SnmpV3 {
            security_name: "scanopyctx".into(),
            auth_protocol: SnmpV3AuthProtocol::Sha256,
            auth_password: inline("ctxauthpass12345"),
            priv_protocol: SnmpV3PrivProtocol::Aes128,
            priv_password: inline("ctxprivpass12345"), context_name: Some("vlan-20".into()),
        },
        system: SystemInfo {
            sys_descr: Some("Cisco IOS Software [Fuji], Catalyst L3 Switch Software (CAT3K_CAA-UNIVERSALK9-M), Version 16.9.5".into()),
            sys_object_id: Some("1.3.6.1.4.1.9.1.1745".into()),
            sys_name: Some("switch-cisco-01".into()),
            sys_location: Some("Server Room B, Rack 2".into()),
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
        bridge: bridge_table(),
        context_bridge: Some(vlan20_bridge_table()),
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "GigabitEthernet1/0/1",
            Some("00:1e:4a:7c:3b:01".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/1")
        .high_speed(),
        IfRow::port(
            2,
            "GigabitEthernet1/0/2",
            Some("00:1e:4a:7c:3b:02".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/2")
        .high_speed(),
        IfRow::port(
            3,
            "GigabitEthernet1/0/3",
            Some("00:1e:4a:7c:3b:03".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/3")
        .high_speed(),
        IfRow::port(
            4,
            "GigabitEthernet1/0/4",
            Some("00:1e:4a:7c:3b:04".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/4")
        .high_speed(),
        IfRow::port(
            5,
            "GigabitEthernet1/0/5",
            Some("00:1e:4a:7c:3b:05".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/5")
        .high_speed(),
        IfRow::port(
            6,
            "GigabitEthernet1/0/6",
            Some("00:1e:4a:7c:3b:06".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/6")
        .high_speed(),
        IfRow::port(
            7,
            "GigabitEthernet1/0/7",
            Some("00:1e:4a:7c:3b:07".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/7")
        .high_speed(),
        IfRow::port(
            8,
            "GigabitEthernet1/0/8",
            Some("00:1e:4a:7c:3b:08".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/8")
        .high_speed(),
        IfRow::virtual_if(101, "Vlan1", if_type::PROP_VIRTUAL)
            .mtu(1500)
            .name("Vlan1")
            .high_speed(),
        IfRow::virtual_if(120, "Vlan20", if_type::PROP_VIRTUAL)
            .mtu(1500)
            .name("Vlan20")
            .high_speed(),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived().fdb(vec![FdbEntry::learned(
        "00:50:56:9a:14:01".parse().unwrap(),
        1,
    )])
}

pub fn vlan20_bridge_table() -> BridgeTable {
    BridgeTable::derived().fdb(vec![
        FdbEntry::learned("00:50:56:9a:20:01".parse().unwrap(), 1),
        FdbEntry::learned("00:50:56:9a:20:02".parse().unwrap(), 2),
        FdbEntry::learned("00:50:56:9a:20:03".parse().unwrap(), 2),
        FdbEntry::learned("00:50:56:9a:20:04".parse().unwrap(), 3),
        FdbEntry::learned("00:50:56:9a:20:05".parse().unwrap(), 4),
        FdbEntry::learned("00:50:56:9a:20:06".parse().unwrap(), 5),
        FdbEntry::learned("00:50:56:9a:20:07".parse().unwrap(), 6),
        FdbEntry::learned("00:50:56:9a:20:08".parse().unwrap(), 7),
        FdbEntry::learned("00:50:56:9a:20:09".parse().unwrap(), 8),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    use crate::daemon::discovery::integration::snmp::sim::device;
    use crate::daemon::discovery::integration::snmp::{
        query_bridge_fdb, query_bridge_port_mapping,
    };

    /// GH #686: a Catalyst with a full MAC-address table reported exactly one entry however it was
    /// queried.
    ///
    /// IOS-XE partitions its forwarding database per VLAN and keeps almost nothing in the default
    /// context, so a scan that cannot name a context reads the wrong table — and is told nothing
    /// is wrong, because a walk that ends cleanly on a one-row table is a complete walk.
    ///
    /// **The comparison is the check, not either count.** An agent that ignores the context answers
    /// both from the same table, which is the failure being guarded against; asserting only "the
    /// context walk returns nine" would pass on one that ignores contexts entirely and happens to
    /// hold nine rows.
    #[tokio::test]
    async fn the_two_contexts_answer_from_different_tables() {
        let cisco = device("switch-cisco-01");
        let ip = cisco.ip.into();

        let default = harness::collect(&cisco).await.fdb;

        let mut context = cisco.context_agent().expect("the vlan-20 back end");
        let mapping = query_bridge_port_mapping(&mut context, ip).await.unwrap();
        let scoped = query_bridge_fdb(&mut context, ip, &mapping).await.unwrap();

        assert_eq!(default.records.len(), 1, "the reporter's symptom");
        assert_eq!(
            scoped.records.len(),
            9,
            "the table that was there all along"
        );
        assert_ne!(
            default.records.len(),
            scoped.records.len(),
            "an agent answering both from the same table is the failure under test"
        );
        assert!(
            default.complete,
            "the one-row read is a *complete* walk, which is why it raised nothing"
        );
    }

    /// Its `ifTable` and system MIB stay in the default context, as they do on the real switch.
    /// That is why the daemon scopes only its bridge and VLAN walks to the credential's context: a
    /// context-wide session would find no interfaces at all here.
    #[tokio::test]
    async fn its_interfaces_stay_in_the_default_context() {
        let scan = harness::scan("switch-cisco-01").await;

        assert_eq!(scan.if_table.entries.len(), 10);
        assert!(scan.if_table.set_complete);
    }
}
