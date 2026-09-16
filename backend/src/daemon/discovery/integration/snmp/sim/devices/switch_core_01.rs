use std::net::Ipv4Addr;

use super::router_gw_01;
use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{
    BridgeTable, CdpTable, EntityTable, FdbEntry, FdbStatus,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::{
    CdpNeighbor, DeviceInventory, SystemInfo, VlanInfo,
};
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-core-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Control {
            role: "the lab's baseline switch and the far end most other devices resolve against; also the first forwarding database and the only CDP cache",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some(
                "Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3".into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.9.1.1208".into()),
            sys_name: Some("switch-core-01".into()),
            sys_location: Some("Server Room A, Rack 1".into()),
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
        entity: entity_table(),
        cdp: cdp_table(),
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "GigabitEthernet0/1",
            Some("00:1a:2b:00:10:01".parse().unwrap()),
        )
        .name("Gi0/1")
        .high_speed()
        .alias("Uplink to switch-access-01"),
        IfRow::port(
            2,
            "GigabitEthernet0/2",
            Some("00:1a:2b:00:10:02".parse().unwrap()),
        )
        .name("Gi0/2")
        .high_speed()
        .alias("Uplink to router-gw-01"),
        IfRow::port(
            3,
            "GigabitEthernet0/3",
            Some("00:1a:2b:00:10:03".parse().unwrap()),
        )
        .name("Gi0/3")
        .high_speed()
        .alias("Server port"),
        IfRow::virtual_if(4, "Vlan10", if_type::PROP_VIRTUAL)
            .mac("00:1a:2b:00:10:00".parse().unwrap())
            .name("Vl10")
            .high_speed()
            .alias("Management VLAN"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-core-01",
    )
    .sys_desc("Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/1".into())),
        )
        .port_desc("GigabitEthernet0/1")
        .sys_name("switch-access-01")
        .sys_desc("Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11"),
        RemoteNeighbour::new(
            2,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:12:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("ge-0/0/0".into())),
        )
        .port_desc("ge-0/0/0")
        .sys_name("router-gw-01")
        .sys_desc("Juniper Networks, Inc. JunOS 21.4R3-S5, MX204"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
        .fdb(vec![
            FdbEntry::learned("00:1a:2b:00:10:00".parse().unwrap(), 0).status(FdbStatus::Self_),
            FdbEntry::learned("00:1a:2b:00:10:01".parse().unwrap(), 1).status(FdbStatus::Mgmt),
            FdbEntry::learned("00:1a:2b:00:11:00".parse().unwrap(), 1),
            FdbEntry::learned("00:1a:2b:00:11:01".parse().unwrap(), 1),
            FdbEntry::learned("00:1a:2b:00:12:01".parse().unwrap(), 2),
            FdbEntry::learned("00:1a:2b:00:13:01".parse().unwrap(), 3),
            FdbEntry::learned("00:1a:2b:00:14:01".parse().unwrap(), 3),
            FdbEntry::learned("00:1a:2b:00:15:01".parse().unwrap(), 3),
            FdbEntry::learned("00:1a:2b:00:11:01".parse().unwrap(), 1).in_vlan(10),
            FdbEntry::learned("00:1a:2b:00:12:01".parse().unwrap(), 2).in_vlan(10),
            FdbEntry::learned("00:1a:2b:00:14:01".parse().unwrap(), 3).in_vlan(20),
        ])
        .vlans(vec![
            VlanInfo {
                vlan_id: 10,
                name: "DATA".into(),
            },
            VlanInfo {
                vlan_id: 20,
                name: "VOICE".into(),
            },
        ])
        .port_vlans(vec![(1, 10), (2, 10), (3, 20)])
}

/// The ordinary single-chassis shape, and the one device that proves the revision columns are read
/// on it. The two values are deliberately different from each other: `.9` is the ROMMON image and
/// `.10` is the IOS version this device's `sysDescr` also names, so a collection that folded the
/// pair into one field would be visible here rather than plausible.
pub fn entity_table() -> EntityTable {
    EntityTable::chassis(
        DeviceInventory {
            description: Some("Cisco Catalyst 2960-24TC-L".into()),
            manufacturer: Some("Cisco".into()),
            model: Some("WS-C2960-24TC-L".into()),
            serial_number: Some("FOC1234X5YZ".into()),
            firmware_revision: Some("12.2(44r)SE".into()),
            software_revision: Some("15.2(7)E3".into()),
        },
        "Chassis",
    )
}

pub fn cdp_table() -> CdpTable {
    CdpTable::new(vec![CdpNeighbor {
        local_port_index: 2,
        remote_device_id: Some(router_gw_01::NAME.into()),
        remote_port_id: Some("ge-0/0/0".into()),
        remote_platform: Some("Juniper MX204".into()),
        // The address `router-gw-01` publishes for itself. CDP carries no chassis id, so this and
        // the device id are the only two identities a CDP neighbour has — and the address is the
        // one that still works when the far end's own tables came back empty. Resolved from the
        // device id above once every device has its final address
        // (`allocation::resolve_peer_addresses`) — a literal here could name a different device
        // the moment the lab is renumbered.
        remote_address: None,
    }])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    use crate::server::lldp::{LldpChassisId, LldpPortId};

    /// The baseline every other device's test leans on. `Gi0/1`-`Gi0/3` are named by
    /// `switch-flaky-01`, `switch-dlink-01` and `switch-tplink-01`, and the far ends they resolve
    /// to are these addresses — so if this device stops reporting them, three other tests are
    /// proving something else.
    ///
    /// The port MACs are also read end to end here. They are six raw octets, and `value_to_mac`
    /// accepts nothing else: sent as text they are silently dropped and every interface stores no
    /// address while the walk still reports itself complete.
    #[tokio::test]
    async fn its_ports_carry_the_addresses_the_rest_of_the_lab_resolves_against() {
        let scan = harness::scan("switch-core-01").await;

        assert!(scan.if_table.set_complete && scan.if_table.attributes_complete);
        assert_eq!(scan.if_table.entries.len(), 4);

        for (if_index, name, mac) in [
            (1, "Gi0/1", "00:1a:2b:00:10:01"),
            (2, "Gi0/2", "00:1a:2b:00:10:02"),
            (3, "Gi0/3", "00:1a:2b:00:10:03"),
        ] {
            let port = scan.interface(if_index);
            assert_eq!(port.if_name.as_deref(), Some(name));
            assert_eq!(
                port.if_phys_address.map(|m| m.to_string().to_lowercase()),
                Some(mac.to_string()),
                "ifIndex {if_index} must store an address, not drop it as text"
            );
        }
    }

    /// The ordinary single-chassis ENTITY-MIB read, on the device that serves it plainly.
    ///
    /// The two revisions are what this is really for. They are distinct MIB objects and are
    /// deliberately distinct values here — the ROMMON image and the IOS version — so a collection
    /// that read only one column, or folded the pair into a single field, fails rather than
    /// looking right by coincidence.
    #[tokio::test]
    async fn it_reports_a_chassis_inventory_carrying_both_revisions() {
        let scan = harness::scan("switch-core-01").await;

        assert!(scan.entity.complete);
        let inventory = scan
            .entity
            .records
            .expect("the chassis row collapses to an inventory");

        assert_eq!(inventory.manufacturer.as_deref(), Some("Cisco"));
        assert_eq!(inventory.model.as_deref(), Some("WS-C2960-24TC-L"));
        assert_eq!(inventory.serial_number.as_deref(), Some("FOC1234X5YZ"));
        assert_eq!(inventory.firmware_revision.as_deref(), Some("12.2(44r)SE"));
        assert_eq!(inventory.software_revision.as_deref(), Some("15.2(7)E3"));
    }

    /// GH #686's read half: the forwarding database is a join across three columns, and the daemon
    /// keeps learned(3) and mgmt(5) while dropping self(4). Eight rows must yield seven entries —
    /// `cdpCacheAddress` is raw address octets in a column the simulator served for no device at
    /// all, so nothing exercised the tier that resolves a CDP neighbour by the address it
    /// publishes — the only identity such a neighbour has besides its device id, and the one that
    /// still works when the far end's own tables came back empty.
    #[tokio::test]
    async fn its_cdp_neighbour_publishes_the_address_it_is_reachable_at() {
        use std::net::IpAddr;

        use crate::daemon::discovery::integration::snmp::sim::device;

        use super::router_gw_01;

        let scan = harness::scan("switch-core-01").await;

        let neighbour = scan
            .cdp
            .records
            .iter()
            .find(|n| n.remote_device_id.as_deref() == Some(router_gw_01::NAME))
            .expect("the CDP neighbour on Gi0/2");

        assert_eq!(
            neighbour.remote_address,
            Some(IpAddr::V4(device(router_gw_01::NAME).ip)),
            "the address must survive the octet-string round trip"
        );
    }

    /// a filter that stopped working shows up as a count too *high*, not as an empty table.
    #[tokio::test]
    async fn its_forwarding_table_drops_only_the_self_rows() {
        let scan = harness::scan("switch-core-01").await;

        assert_eq!(scan.fdb.records.len(), 7, "eight rows, one of them self(4)");
        assert!(scan.fdb.complete);
        assert!(
            scan.fdb.records.iter().all(|entry| entry.status != 4),
            "a self(4) row reached the collection"
        );
        // Every entry resolved its bridge port to a real ifIndex through dot1dBasePortIfIndex.
        assert!(
            scan.fdb
                .records
                .iter()
                .all(|entry| entry.if_index.is_some())
        );
    }

    /// Its neighbours are advertised with padded-ASCII chassis ids, which is the lab's most common
    /// encoding and one `parse_mac_id` accepts by design.
    #[tokio::test]
    async fn its_neighbours_resolve_from_ascii_chassis_ids() {
        let scan = harness::scan("switch-core-01").await;

        assert_eq!(scan.neighbours.records.len(), 2);
        assert!(scan.neighbours.complete);
        assert_eq!(scan.neighbours.discarded, 0);

        let access = scan.neighbour_named("switch-access-01");
        assert_eq!(access.local_port_index, 1);
        assert_eq!(
            LldpChassisId::from_snmp(
                access.remote_chassis_id_subtype.unwrap(),
                access.remote_chassis_id_bytes.as_ref().unwrap()
            ),
            Some(LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()))
        );
        assert_eq!(
            LldpPortId::from_snmp(
                access.remote_port_id_subtype.unwrap(),
                access.remote_port_id_bytes.as_ref().unwrap()
            ),
            Some(LldpPortId::InterfaceName("Gi0/1".into()))
        );
    }
}
