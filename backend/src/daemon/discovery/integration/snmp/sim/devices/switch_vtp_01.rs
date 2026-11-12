use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::{BridgeTable, VtpTable};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::{SystemInfo, VlanInfo};
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

use crate::daemon::discovery::integration::snmp::sim::CommunityVersions;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-vtp-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Control {
            role: "a Cisco switch whose VLAN names come from CISCO-VTP-MIB with none in Q-BRIDGE; the only device that answers the VTP fallback, and one the TP-Link VLAN walk must never reach",
        },
        credential: CredentialType::SnmpCommunity {
            community: inline("netdefault"),
        },
        community_versions: CommunityVersions::Both,
        system: SystemInfo {
            sys_descr: Some(
                "Cisco IOS Software, C3560 Software (C3560-IPSERVICESK9-M), Version 12.2(55)SE12"
                    .into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.9.1.563".into()),
            sys_name: Some("switch-vtp-01".into()),
            sys_location: Some("Server Room B, Rack 2".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(6),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: Tables {
            if_table: Some(if_table()),
            bridge: BridgeTable::derived(),
            vtp: vtp_table(),
            ..Default::default()
        },
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
        rejects_getbulk: None,
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            10001,
            "FastEthernet0/1",
            Some("00:1a:2b:00:56:01".parse().unwrap()),
        )
        .name("Fa0/1"),
        IfRow::port(
            10002,
            "FastEthernet0/2",
            Some("00:1a:2b:00:56:02".parse().unwrap()),
        )
        .name("Fa0/2"),
        IfRow::port(
            10101,
            "GigabitEthernet0/1",
            Some("00:1a:2b:00:56:65".parse().unwrap()),
        )
        .name("Gi0/1"),
    ])
}

pub fn vtp_table() -> VtpTable {
    let vlan = |vlan_id: u16, name: &str| VlanInfo {
        vlan_id,
        name: name.into(),
    };
    VtpTable {
        vlans: vec![vlan(1, "default"), vlan(10, "DATA"), vlan(20, "VOICE")],
    }
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::oids::vlan;
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// VLAN names read through the VTP fallback, with nothing asked of TP-Link's tree.
    #[tokio::test]
    async fn vlan_names_come_from_vtp_and_no_tplink_walk_is_sent() {
        let (scan, requests) = harness::recorded_scan("switch-vtp-01").await;

        let vlans: Vec<(u16, &str)> = scan
            .vlans
            .iter()
            .map(|v| (v.vlan_id, v.name.as_str()))
            .collect();
        assert_eq!(vlans, vec![(1, "default"), (10, "DATA"), (20, "VOICE")]);
        assert!(scan.vlan_membership.is_empty());

        assert!(
            !requests
                .iter()
                .any(|r| r.is_under(vlan::tplink::TPLINK_DOT1Q_VLAN_MIB)),
            "a Cisco agent was asked for TP-Link's VLAN tables"
        );
    }
}
