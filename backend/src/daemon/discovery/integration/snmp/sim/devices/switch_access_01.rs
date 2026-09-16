use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{BridgeTable, EntityRow, EntityTable};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::{DeviceInventory, SystemInfo};
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-access-01",
        ip: Ipv4Addr::UNSPECIFIED,
        purpose: Purpose::Control {
            role: "a far end named by switch-core-01's Gi0/1, and the lab's only stacked chassis — the one entPhysicalTable serving several chassis rows, which is what the collapse's index tiebreak is selecting between",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some(
                "Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11"
                    .into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.9.1.516".into()),
            sys_name: Some("switch-access-01".into()),
            sys_location: Some("Floor 2, IDF B".into()),
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
        ..Default::default()
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "GigabitEthernet0/1",
            Some("00:1a:2b:00:11:01".parse().unwrap()),
        )
        .name("Gi0/1")
        .high_speed()
        .alias("Uplink to switch-core-01"),
        IfRow::port(
            2,
            "GigabitEthernet0/2",
            Some("00:1a:2b:00:11:02".parse().unwrap()),
        )
        .name("Gi0/2")
        .high_speed()
        .alias("Access port - Floor 2"),
        IfRow::port(
            3,
            "GigabitEthernet0/3",
            Some("00:1a:2b:00:11:03".parse().unwrap()),
        )
        .name("Gi0/3")
        .high_speed()
        .alias("Downlink to ap-wireless-01"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-access-01",
    )
    .sys_desc("Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/1".into())),
        )
        .port_desc("GigabitEthernet0/1")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3"),
        RemoteNeighbour::new(
            3,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:15:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("eth0".into())),
        )
        .port_desc("eth0")
        .sys_name("ap-wireless-01")
        .sys_desc("Ubiquiti UniFi AP AC Pro, firmware 6.5.28"),
        // A no-IP PROFINET device found only via DCP Identify (see daemon/discovery/service/
        // network/dcp/), attached to the access port on Gi0/2. This is the fixture proving the
        // MAC-tier of `LldpChassisId::resolve_host_id` doesn't care about attribution — the same
        // MAC a DCP sweep discovers (AttributeSource::ProfinetDcp) is also what tools/dcp/
        // dcp-sim.py's --device-mac defaults to, so a real scan against both the SNMP lab and the
        // local DCP sim resolves this into one host with a real neighbor edge, not two.
        RemoteNeighbour::new(
            2,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:dc:90:01".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::LocallyAssigned("sim-port-1".into())),
        )
        .sys_name("scanopy-dcp-sim"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

/// A two-member C3750 stack: the shape that makes the collapse's row selection observable.
///
/// The rows are declared out of index order on purpose, but that ordering is documentation rather
/// than mechanism — the entity data file renders `Ordering::Ascending`, so the wire is sorted by
/// OID whatever this vector says. What actually makes the selection observable is that there is
/// more than one chassis row at all: the collection accumulates rows into a `HashMap`, whose
/// iteration order is randomised per instance, so before the index tiebreak existed the collapse
/// returned member 1 or member 2 depending on nothing.
///
/// Each row is discriminating in a different way:
///
/// - the two chassis rows carry **different serials and different bootloaders**, so an assertion
///   on either proves which row won. Member 2 is an RMA'd unit that never had its ROMMON brought
///   forward, which is both realistic and why `.9` differs while `.10` does not — stack members
///   run one IOS image.
/// - the stack row (class 11) sits at the lowest index of all, so a collapse that sorted by index
///   *before* ranking by class would pick it. It must lose to both chassis rows.
/// - the module row (class 9) carries revisions of its own that must never be read, since a
///   linecard's firmware is not the device's.
pub fn entity_table() -> EntityTable {
    EntityTable::new(vec![
        EntityRow::new(
            1,
            11,
            "Switch Stack",
            DeviceInventory {
                description: Some("Cisco Catalyst 3750 Stack".into()),
                manufacturer: Some("Cisco".into()),
                model: Some("WS-C3750G-24TS-S".into()),
                serial_number: None,
                firmware_revision: None,
                software_revision: None,
            },
        ),
        EntityRow::new(
            1002,
            3,
            "Switch 2",
            DeviceInventory {
                description: Some("Cisco Catalyst 3750 - member 2".into()),
                manufacturer: Some("Cisco".into()),
                model: Some("WS-C3750G-24TS-S".into()),
                serial_number: Some("FDO1441P0CD".into()),
                firmware_revision: Some("12.2(53r)SEY3".into()),
                software_revision: Some("15.0(2)SE11".into()),
            },
        ),
        EntityRow::new(
            1001,
            3,
            "Switch 1",
            DeviceInventory {
                description: Some("Cisco Catalyst 3750 - member 1".into()),
                manufacturer: Some("Cisco".into()),
                model: Some("WS-C3750G-24TS-S".into()),
                serial_number: Some("FDO1441P0AB".into()),
                firmware_revision: Some("15.0(4)".into()),
                software_revision: Some("15.0(2)SE11".into()),
            },
        ),
        EntityRow::new(
            2001,
            9,
            "Uplink module",
            DeviceInventory {
                description: Some("Cisco 2-port SFP uplink".into()),
                manufacturer: Some("Cisco".into()),
                model: Some("C3750-SFP-2".into()),
                serial_number: Some("FDO1441M0EF".into()),
                firmware_revision: Some("1.4.2".into()),
                software_revision: Some("1.4.2".into()),
            },
        ),
    ])
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::query_entity_physical;
    use crate::daemon::discovery::integration::snmp::sim::{self, harness};

    /// The `entPhysicalTable` collapse has to pick the same row every time, and this device is the
    /// only one that can tell whether it does.
    ///
    /// Run in a loop rather than once. The collection accumulates rows into a `HashMap`, and
    /// `RandomState` seeds each instance separately, so the iteration order differs per call —
    /// which means a single pass against the unfixed collapse picks the right row roughly half the
    /// time and proves nothing. Twenty consecutive passes do not pass by luck.
    ///
    /// Driving `query_entity_physical` directly rather than `harness::scan` keeps this to the one
    /// walk under test; twenty full device collections would be twenty times the work for no extra
    /// coverage.
    #[tokio::test]
    async fn its_stacked_chassis_rows_collapse_to_the_lowest_index_every_time() {
        let device = sim::device("switch-access-01");
        let mut agent = device.agent();
        let ip = device.ip.into();

        for pass in 0..20 {
            let collected = query_entity_physical(&mut agent, ip)
                .await
                .expect("the walk answers");
            assert!(collected.complete, "pass {pass}: the walk must finish");
            let inventory = collected
                .records
                .unwrap_or_else(|| panic!("pass {pass}: a stacked chassis must collapse to a row"));

            // Member 1 at entPhysicalIndex 1001, never member 2 at 1002.
            assert_eq!(
                inventory.serial_number.as_deref(),
                Some("FDO1441P0AB"),
                "pass {pass}: the lowest-index chassis row must win"
            );
            // From that same row, not harvested across rows: member 2's bootloader differs, and
            // the module row's revisions are lower still.
            assert_eq!(
                inventory.firmware_revision.as_deref(),
                Some("15.0(4)"),
                "pass {pass}: entPhysicalFirmwareRev comes from the selected row"
            );
            assert_eq!(
                inventory.software_revision.as_deref(),
                Some("15.0(2)SE11"),
                "pass {pass}: entPhysicalSoftwareRev is read, and separately from the firmware"
            );
        }
    }

    /// A control device: it exists to be a far end. `switch-core-01` advertises it on `Gi0/1` by
    /// the chassis id below, so this is the assertion that keeps that test honest — if this
    /// device stops reporting these values, the neighbour over there resolves through a fallback
    /// tier and looks identical while proving nothing.
    #[tokio::test]
    async fn it_reports_the_identity_switch_core_01_names_it_by() {
        let scan = harness::scan("switch-access-01").await;

        assert_eq!(scan.if_table.entries.len(), 3);
        assert!(scan.if_table.set_complete);
        assert_eq!(scan.interface(1).if_name.as_deref(), Some("Gi0/1"));
        assert_eq!(
            scan.interface(1)
                .if_phys_address
                .map(|m| m.to_string().to_lowercase()),
            Some("00:1a:2b:00:11:01".to_string())
        );
    }
}
