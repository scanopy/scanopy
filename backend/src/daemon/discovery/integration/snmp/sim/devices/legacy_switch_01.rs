use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{BridgeTable, FdbEntry, FdbStatus};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::{SystemInfo, VlanInfo};
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "legacy-switch-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Regression {
            issue: "#557",
            defect: "SNMPv1 has no getbulk, so every table must come back over getnext",
        },
        credential: CredentialType::SnmpV1 {
            community: inline("legacyv1"),
        },
        system: SystemInfo {
            sys_descr: Some("Cisco IOS Software, C2950 Software, Version 12.1(22)EA14".into()),
            sys_object_id: Some("1.3.6.1.4.1.9.1.359".into()),
            sys_name: Some("legacy-switch-01".into()),
            sys_location: Some("Closet 1, Legacy Rack".into()),
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
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "FastEthernet0/1",
            Some("00:1a:2b:00:16:01".parse().unwrap()),
        )
        .speed(100000000)
        .name("Fa0/1")
        .high_speed()
        .alias("Uplink to switch-access-01"),
        IfRow::port(
            2,
            "FastEthernet0/2",
            Some("00:1a:2b:00:16:02".parse().unwrap()),
        )
        .speed(100000000)
        .name("Fa0/2")
        .high_speed()
        .alias("Access port"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:16:00".into()),
            MacEncoding::AsciiLower,
        ),
        "legacy-switch-01",
    )
    .sys_desc("Cisco IOS Software, C2950 Software, Version 12.1(22)EA14")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/2".into())),
        )
        .port_desc("GigabitEthernet0/2")
        .sys_name("switch-access-01")
        .sys_desc("Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
        .fdb(vec![
            FdbEntry::learned("00:1a:2b:00:10:01".parse().unwrap(), 1),
            FdbEntry::learned("00:1a:2b:00:16:01".parse().unwrap(), 2).status(FdbStatus::Self_),
        ])
        .vlans(vec![VlanInfo {
            vlan_id: 1,
            name: "default".into(),
        }])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// GH #557: SNMPv1 has no getbulk.
    ///
    /// Its agent refuses the bulk request, so every column of every table has to come back through
    /// getnext — one varbind per round trip. The join across three forwarding-table columns is the
    /// sharpest test of that fallback: it only assembles if each column walked to its end
    /// independently.
    #[tokio::test]
    async fn its_tables_assemble_over_getnext_alone() {
        let scan = harness::scan("legacy-switch-01").await;

        assert_eq!(scan.if_table.entries.len(), 2);
        assert!(scan.if_table.set_complete && scan.if_table.attributes_complete);
        assert_eq!(scan.fdb.records.len(), 1, "the join held over getnext");
        assert!(scan.fdb.complete);
        assert_eq!(scan.neighbours.records.len(), 1);
        assert!(scan.neighbours.complete);
    }

    /// SNMPv1 has no `endOfMibView`. An agent answers a v1 GETNEXT past the end of its MIB view
    /// with `noSuchName` and the request echoed back (RFC 3584 §4.2.2.2.2), and read as a row that
    /// echo is an answer to some other question: the column base, which the walk re-asked and
    /// then reported desynchronised. The ENTITY and CDP walks both start past this device's last
    /// table, so both get that answer.
    #[tokio::test]
    async fn walking_off_the_end_of_its_mib_view_is_a_natural_end() {
        let scan = harness::scan("legacy-switch-01").await;

        assert!(
            scan.entity.complete,
            "the device has no ENTITY-MIB, which noSuchName says in full; got {:?}",
            scan.entity.reason
        );
        assert!(
            scan.cdp.complete,
            "the device has no CDP-MIB, which noSuchName says in full; got {:?}",
            scan.cdp.reason
        );
    }
}
