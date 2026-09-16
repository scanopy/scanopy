use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

/// GH #668, the last of the reported symptoms: the far end names one MAC that is genuinely worn
/// by more than one physical interface on the resolved host (`pc_windows_nic_filters`) — a real
/// NIC plus its NDIS filter/LWF pseudo-interfaces, all reporting the identical `ifPhysAddress` and
/// an ordinary ethernet `if_type`. A separate switch from `switch_dlink_01` (whose four ports
/// already cover the earlier symptoms from the same issue) so this one scenario's fixture and test
/// can change without touching that one's.
pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-dlink-02",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#668",
            defect: "a far end's MAC is carried by several of its own physical interfaces (a real NIC and its NDIS filter/LWF pseudo-interfaces), so the port cannot be told apart by if_type alone",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("D-Link DGS-1210-48 Rev.GX/7.20.003".into()),
            sys_object_id: Some("1.3.6.1.4.1.171.10.76.28".into()),
            sys_name: Some("switch-dlink-02".into()),
            sys_location: Some("Lab".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
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
        IfRow::port(
            7,
            "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 7",
            Some("00:ad:24:cc:00:07".parse().unwrap()),
        )
        .name("Slot0/7"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:ad:24:cc:00:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-dlink-02",
    )
    .sys_desc("D-Link DGS-1210-48 Rev.GX/7.20.003")
    .local_ports(vec![LocalPort::new(
        7,
        Advertised::octets(LldpPortId::InterfaceName("Slot0/7".into())),
    )])
    .neighbours(vec![
        // The decisive record: this switch learned one MAC on port 7, and that MAC is carried by
        // several of the far end's own physical interfaces — not several ports of *this* switch,
        // which is `switch_dlink_01`'s scenario.
        RemoteNeighbour::new(
            7,
            Advertised::octets(LldpChassisId::MacAddress("50:eb:f6:26:54:79".into())),
            Advertised::octets(LldpPortId::MacAddress("50:eb:f6:26:54:79".into())),
        )
        .sys_name("pc-windows-nic-filters"),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// The far end's chassis id and port id are both its shared NIC MAC — exactly the shape the
    /// real report's `LldpPortAmbiguous` warning carried (`port_id: MacAddress("50:eb:f6:26:54:
    /// 79")`, `if_descr: "...Port 7"`).
    #[tokio::test]
    async fn port_seven_hears_the_shared_nic_mac() {
        let scan = harness::scan("switch-dlink-02").await;

        let neighbour = scan
            .neighbours_on(7)
            .into_iter()
            .next()
            .expect("no neighbour on local port 7");
        assert_eq!(
            neighbour
                .remote_chassis_id_bytes
                .as_deref()
                .map(|b| b.to_vec()),
            Some(vec![0x50, 0xeb, 0xf6, 0x26, 0x54, 0x79])
        );
        assert_eq!(
            neighbour
                .remote_port_id_bytes
                .as_deref()
                .map(|b| b.to_vec()),
            Some(vec![0x50, 0xeb, 0xf6, 0x26, 0x54, 0x79])
        );
    }
}
