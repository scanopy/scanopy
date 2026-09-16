//! The remaining MIBs a device can serve: bridge, ARP, IP addresses, entity and CDP.
//!
//! Each is built from the daemon's own collection type wherever one exists, so a fixture row and
//! the row the collection produces cannot describe the same thing differently.

use std::net::{IpAddr, Ipv4Addr};

use mac_address::MacAddress;

use super::wire::{PassValue, Row};
use crate::daemon::discovery::integration::snmp::oids::{arp, bridge, cdp, entity, ip_mib, vlan};
use crate::daemon::discovery::integration::snmp::types::{
    ArpEntry, CdpNeighbor, DeviceInventory, VlanInfo,
};

/// The six decimal sub-ids a MAC contributes to a forwarding-table index.
fn mac_suffix(mac: &MacAddress) -> Vec<u64> {
    mac.bytes().iter().map(|b| *b as u64).collect()
}

fn ipv4_suffix(ip: &Ipv4Addr) -> Vec<u64> {
    ip.octets().iter().map(|o| *o as u64).collect()
}

/// A forwarding-database entry.
///
/// The MAC is both the index — six decimal sub-ids, one per octet — and the value of the address
/// column, as six raw bytes. One field emits both, which is the only way those two can never
/// disagree, and the repetition is the lab's only end-to-end coverage of a binary MAC on a table
/// the daemon joins across three columns.
#[derive(Debug, Clone)]
pub struct FdbEntry {
    pub mac: MacAddress,
    pub bridge_port: u32,
    pub status: FdbStatus,
    /// Present for a `dot1qTpFdb` row, absent for the legacy `dot1dTpFdb` one. The Q-BRIDGE table
    /// keys on `{ vlan, mac }`, so the VLAN is part of the index rather than a column.
    pub vlan: Option<u16>,
}

/// `dot1dTpFdbStatus`. The daemon keeps learned and mgmt and drops self, so a lab table with a
/// mix of statuses yields fewer entries than rows — and a filter that stopped working shows up as
/// a count that is too *high*, not as an empty table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdbStatus {
    Other = 1,
    Invalid = 2,
    Learned = 3,
    Self_ = 4,
    Mgmt = 5,
}

impl FdbEntry {
    pub fn learned(mac: MacAddress, bridge_port: u32) -> Self {
        Self {
            mac,
            bridge_port,
            status: FdbStatus::Learned,
            vlan: None,
        }
    }

    pub fn status(mut self, status: FdbStatus) -> Self {
        self.status = status;
        self
    }

    pub fn in_vlan(mut self, vlan: u16) -> Self {
        self.vlan = Some(vlan);
        self
    }
}

/// How a device's bridge port numbering is arrived at.
#[derive(Debug, Clone)]
pub enum BridgePorts {
    /// Numbered from the device's own ethernet interfaces, in ifIndex order — what an
    /// unconfigured managed switch reports, and what section 5b of the old deploy script
    /// generated for every switch that had no bridge file of its own.
    DerivedFromEthernetPorts,
    /// `(bridge port, ifIndex)` written out, for the devices whose mapping encodes a shape: the
    /// Dell's OS10 breakout lanes, the Cisco's per-context tables.
    Explicit(Vec<(u32, i32)>),
}

/// BRIDGE-MIB and Q-BRIDGE-MIB.
#[derive(Debug, Clone, Default)]
pub struct BridgeTable {
    pub ports: Option<BridgePorts>,
    pub fdb: Vec<FdbEntry>,
    pub vlans: Vec<VlanInfo>,
    /// `dot1qPvid` — the untagged VLAN per bridge port.
    pub port_vlans: Vec<(u32, u16)>,
}

impl BridgeTable {
    /// A switch reporting the bridge ports its own interface list implies.
    pub fn derived() -> Self {
        Self {
            ports: Some(BridgePorts::DerivedFromEthernetPorts),
            ..Default::default()
        }
    }

    pub fn with_ports(ports: Vec<(u32, i32)>) -> Self {
        Self {
            ports: Some(BridgePorts::Explicit(ports)),
            ..Default::default()
        }
    }

    pub fn fdb(mut self, fdb: Vec<FdbEntry>) -> Self {
        self.fdb = fdb;
        self
    }

    pub fn vlans(mut self, vlans: Vec<VlanInfo>) -> Self {
        self.vlans = vlans;
        self
    }

    pub fn port_vlans(mut self, port_vlans: Vec<(u32, u16)>) -> Self {
        self.port_vlans = port_vlans;
        self
    }

    /// The `(bridge port, ifIndex)` pairs this device serves.
    pub fn port_map(&self, ethernet_if_indexes: &[i32]) -> Vec<(u32, i32)> {
        match &self.ports {
            None => Vec::new(),
            Some(BridgePorts::Explicit(pairs)) => pairs.clone(),
            Some(BridgePorts::DerivedFromEthernetPorts) => ethernet_if_indexes
                .iter()
                .enumerate()
                .map(|(i, if_index)| (i as u32 + 1, *if_index))
                .collect(),
        }
    }

    /// `dot1dBaseNumPorts.0` — derived from the mapping, never written. A count written by hand
    /// goes stale the moment someone adds a port, and a stale count turns every scan of that
    /// device into a warning about a device that is fine.
    pub fn declared_port_count(&self, ethernet_if_indexes: &[i32]) -> usize {
        self.port_map(ethernet_if_indexes).len()
    }

    pub fn is_empty(&self) -> bool {
        self.ports.is_none()
            && self.fdb.is_empty()
            && self.vlans.is_empty()
            && self.port_vlans.is_empty()
    }

    pub fn wire_rows(&self, ethernet_if_indexes: &[i32]) -> Vec<Row> {
        let port_map = self.port_map(ethernet_if_indexes);
        let mut rows = Vec::new();
        if !port_map.is_empty() {
            rows.push(Row::scalar(
                bridge::DOT1D_BASE_NUM_PORTS,
                PassValue::Integer(port_map.len() as i64),
            ));
        }
        for (port, if_index) in &port_map {
            rows.push(Row::at(
                bridge::DOT1D_BASE_PORT_IF_INDEX,
                &[*port as u64],
                PassValue::Integer(*if_index as i64),
            ));
        }

        for entry in &self.fdb {
            let mac = mac_suffix(&entry.mac);
            match entry.vlan {
                None => {
                    rows.push(Row::at(
                        bridge::fdb_entry::DOT1D_TP_FDB_ADDRESS,
                        &mac,
                        PassValue::Octets(entry.mac.bytes().to_vec()),
                    ));
                    rows.push(Row::at(
                        bridge::fdb_entry::DOT1D_TP_FDB_PORT,
                        &mac,
                        PassValue::Integer(entry.bridge_port as i64),
                    ));
                    rows.push(Row::at(
                        bridge::fdb_entry::DOT1D_TP_FDB_STATUS,
                        &mac,
                        PassValue::Integer(entry.status as i64),
                    ));
                }
                Some(vlan_id) => {
                    let mut suffix = vec![vlan_id as u64];
                    suffix.extend_from_slice(&mac);
                    rows.push(Row::at(
                        bridge::q_fdb_entry::DOT1Q_TP_FDB_PORT,
                        &suffix,
                        PassValue::Integer(entry.bridge_port as i64),
                    ));
                    rows.push(Row::at(
                        bridge::q_fdb_entry::DOT1Q_TP_FDB_STATUS,
                        &suffix,
                        PassValue::Integer(entry.status as i64),
                    ));
                }
            }
        }

        for info in &self.vlans {
            rows.push(Row::at(
                vlan::q_bridge::DOT1Q_VLAN_STATIC_NAME,
                &[info.vlan_id as u64],
                PassValue::Str(info.name.clone()),
            ));
        }
        for (port, vlan_id) in &self.port_vlans {
            rows.push(Row::at(
                vlan::q_bridge::DOT1Q_PVID,
                &[*port as u64],
                PassValue::Integer(*vlan_id as i64),
            ));
        }
        rows
    }
}

/// `ipNetToMediaTable` — the ARP cache.
#[derive(Debug, Clone, Default)]
pub struct ArpTable {
    pub entries: Vec<ArpEntry>,
    /// Serve only `ipNetToMediaIfIndex`, not the other three columns.
    ///
    /// The non-advancing agent needs exactly one row in one column: its handler answers every
    /// request with the first line of the file, so a file carrying the other columns would let
    /// them stand in for each other and muddy what the device is demonstrating.
    pub index_column_only: bool,
}

impl ArpTable {
    pub fn new(entries: Vec<ArpEntry>) -> Self {
        Self {
            entries,
            index_column_only: false,
        }
    }

    /// A table serving its index column alone — see [`Self::index_column_only`].
    pub fn index_column(entries: Vec<ArpEntry>) -> Self {
        Self {
            entries,
            index_column_only: true,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The four columns, each keyed by `ifIndex` then the IPv4 address.
    ///
    /// Emitted column by column rather than row by row. That is not cosmetic: the entry is a join
    /// across all four, so a column that comes up short discards every row the others read — which
    /// is how the GH #674 reporter's switch logged `count=0` while answering hundreds of rows.
    pub fn wire_rows(&self) -> Vec<Row> {
        let suffix = |entry: &ArpEntry| {
            let IpAddr::V4(ip) = entry.ip_address else {
                return None;
            };
            let mut suffix = vec![entry.if_index as u64];
            suffix.extend(ipv4_suffix(&ip));
            Some((suffix, ip))
        };

        // Column by column, not row by row. On the device that serves this table out of order the
        // emission order *is* the defect: each column is shuffled independently, and a walk fails
        // on the second page of a single column. Interleaving the columns would hide that.
        let mut rows = Vec::new();
        for entry in &self.entries {
            if let Some((s, _)) = suffix(entry) {
                rows.push(Row::at(
                    arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
                    &s,
                    PassValue::Integer(entry.if_index as i64),
                ));
            }
        }
        if self.index_column_only {
            return rows;
        }
        for entry in &self.entries {
            if let Some((s, _)) = suffix(entry) {
                rows.push(Row::at(
                    arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
                    &s,
                    PassValue::Octets(entry.mac_address.bytes().to_vec()),
                ));
            }
        }
        for entry in &self.entries {
            if let Some((s, ip)) = suffix(entry) {
                rows.push(Row::at(
                    arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS,
                    &s,
                    PassValue::IpAddress(ip),
                ));
            }
        }
        for entry in &self.entries {
            if let Some((s, _)) = suffix(entry) {
                rows.push(Row::at(
                    arp::entry::IP_NET_TO_MEDIA_TYPE,
                    &s,
                    // dynamic(3) — a learned entry, which is what every row in the lab is.
                    PassValue::Integer(3),
                ));
            }
        }
        rows
    }
}

/// One `ipAddrTable` row.
#[derive(Debug, Clone)]
pub struct IpAddrRow {
    pub address: Ipv4Addr,
    pub if_index: i32,
    pub netmask: Ipv4Addr,
}

/// How a device's own address rides in `ipAddrTable`, alongside whatever [`IpAddrTable`] rows it
/// declares as *extra*.
///
/// Every device serves this row automatically ([`super::SimDevice::data_files`]). The default
/// `ifIndex` is deliberately **not** the device's first port: several fixtures exist to prove a
/// MAC shared across every physical port is declined as ambiguous
/// (`switch-dlink-01`/`switch-dlink-02`/`switch-tplink-01`), with no separate management
/// interface to bind an address to honestly. Defaulting to a real port would hand the resolver a
/// false tiebreak — "this port has an IP configured" — that those fixtures exist to show does not
/// happen. `0` is never a real device's ifIndex (SNMP's `ifIndex` starts at 1), so it can never
/// coincidentally disambiguate one. A device whose own address really does bind to a specific
/// interface — `pc-windows-nic-filters`, at the real NIC's `ifIndex` 7, not a filter
/// pseudo-interface's — says so explicitly.
#[derive(Debug, Clone, Copy)]
pub struct OwnAddress {
    pub if_index: i32,
    pub netmask: Ipv4Addr,
}

impl Default for OwnAddress {
    fn default() -> Self {
        Self {
            if_index: 0,
            netmask: super::allocation::SHARED_NETMASK,
        }
    }
}

/// Addresses a device serves *beyond* its own — the `#663` guest-subnet shape. A device's own
/// address is never listed here: it is served automatically, from [`OwnAddress`].
#[derive(Debug, Clone, Default)]
pub struct IpAddrTable {
    pub rows: Vec<IpAddrRow>,
}

impl IpAddrTable {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn wire_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for row in &self.rows {
            let suffix = ipv4_suffix(&row.address);
            rows.push(Row::at(
                ip_mib::ip_addr_entry::IP_AD_ENT_ADDR,
                &suffix,
                PassValue::IpAddress(row.address),
            ));
            rows.push(Row::at(
                ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
                &suffix,
                PassValue::Integer(row.if_index as i64),
            ));
            rows.push(Row::at(
                ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
                &suffix,
                PassValue::IpAddress(row.netmask),
            ));
        }
        rows
    }
}

/// One `entPhysicalTable` row.
///
/// Separate from [`DeviceInventory`] because the table's index and class are what the collapse
/// selects *on*, and the inventory type carries neither — it is the collapse's output, one row
/// wide, and cannot express the several rows a real chassis serves.
#[derive(Debug, Clone)]
pub struct EntityRow {
    /// `entPhysicalIndex`. The collapse prefers the lowest among the rows of the strongest class,
    /// so a fixture serving more than one is what proves that tiebreak exists.
    pub index: u64,
    /// `entPhysicalClass`: chassis(3), module(9), stack(11).
    pub class: i64,
    /// `entPhysicalName`, which [`DeviceInventory`] has no field for because the collection reads
    /// it only as a fallback description.
    pub name: Option<String>,
    pub inventory: DeviceInventory,
}

impl EntityRow {
    pub fn new(index: u64, class: i64, name: &str, inventory: DeviceInventory) -> Self {
        Self {
            index,
            class,
            name: Some(name.to_string()),
            inventory,
        }
    }
}

/// `entPhysicalTable`.
///
/// Multi-row: a stacked switch serves one chassis row per member, and until it could express that
/// no fixture could exercise which row the collapse picks.
#[derive(Debug, Clone, Default)]
pub struct EntityTable {
    pub rows: Vec<EntityRow>,
}

impl EntityTable {
    /// The single-chassis case, at `entPhysicalIndex` 1 — what most devices serve.
    pub fn chassis(inventory: DeviceInventory, name: &str) -> Self {
        Self {
            rows: vec![EntityRow::new(1, 3, name, inventory)],
        }
    }

    pub fn new(rows: Vec<EntityRow>) -> Self {
        Self { rows }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn wire_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for entry in &self.rows {
            let idx = [entry.index];
            let inventory = &entry.inventory;
            if let Some(descr) = &inventory.description {
                rows.push(Row::at(
                    entity::entry::ENT_PHYSICAL_DESCR,
                    &idx,
                    PassValue::Str(descr.clone()),
                ));
            }
            rows.push(Row::at(
                entity::entry::ENT_PHYSICAL_CLASS,
                &idx,
                PassValue::Integer(entry.class),
            ));
            if let Some(name) = &entry.name {
                rows.push(Row::at(
                    entity::entry::ENT_PHYSICAL_NAME,
                    &idx,
                    PassValue::Str(name.clone()),
                ));
            }
            if let Some(serial) = &inventory.serial_number {
                rows.push(Row::at(
                    entity::entry::ENT_PHYSICAL_SERIAL_NUM,
                    &idx,
                    PassValue::Str(serial.clone()),
                ));
            }
            if let Some(mfg) = &inventory.manufacturer {
                rows.push(Row::at(
                    entity::entry::ENT_PHYSICAL_MFG_NAME,
                    &idx,
                    PassValue::Str(mfg.clone()),
                ));
            }
            if let Some(model) = &inventory.model {
                rows.push(Row::at(
                    entity::entry::ENT_PHYSICAL_MODEL_NAME,
                    &idx,
                    PassValue::Str(model.clone()),
                ));
            }
            if let Some(firmware) = &inventory.firmware_revision {
                rows.push(Row::at(
                    entity::entry::ENT_PHYSICAL_FIRMWARE_REV,
                    &idx,
                    PassValue::Str(firmware.clone()),
                ));
            }
            if let Some(software) = &inventory.software_revision {
                rows.push(Row::at(
                    entity::entry::ENT_PHYSICAL_SOFTWARE_REV,
                    &idx,
                    PassValue::Str(software.clone()),
                ));
            }
        }
        rows
    }
}

/// `cdpCacheTable`, keyed by `(ifIndex, cache index)`.
#[derive(Debug, Clone, Default)]
pub struct CdpTable {
    pub neighbours: Vec<CdpNeighbor>,
    /// `cdpCachePlatform`, which [`CdpNeighbor`] calls `remote_platform`.
    pub cache_index: u32,
}

impl CdpTable {
    pub fn new(neighbours: Vec<CdpNeighbor>) -> Self {
        Self {
            neighbours,
            cache_index: 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.neighbours.is_empty()
    }

    pub fn wire_rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for neighbour in &self.neighbours {
            let suffix = [neighbour.local_port_index as u64, self.cache_index as u64];
            for (base, text) in [
                (cdp::entry::CDP_CACHE_DEVICE_ID, &neighbour.remote_device_id),
                (cdp::entry::CDP_CACHE_DEVICE_PORT, &neighbour.remote_port_id),
                (cdp::entry::CDP_CACHE_PLATFORM, &neighbour.remote_platform),
            ] {
                if let Some(text) = text {
                    rows.push(Row::at(base, &suffix, PassValue::Str(text.clone())));
                }
            }

            // `cdpCacheAddress` is raw address octets, not text — four of them for IPv4, which is
            // the only width the collector decodes (`query_cdp_neighbors`). The field existed on
            // this struct and was never served at all, so the address tier that resolves a CDP
            // neighbour by where it says it lives had no end-to-end coverage.
            if let Some(IpAddr::V4(v4)) = neighbour.remote_address {
                rows.push(Row::at(
                    cdp::entry::CDP_CACHE_ADDRESS,
                    &suffix,
                    PassValue::Octets(v4.octets().to_vec()),
                ));
            }
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac(last: u8) -> MacAddress {
        MacAddress::new([0x00, 0x1a, 0x2b, 0x00, 0x10, last])
    }

    /// The MAC is the index and the value, from one field. A `string` encoding here is what would
    /// silently stop testing the binary path, and there is no way to ask for one.
    #[test]
    fn a_forwarding_entry_keys_itself_on_the_address_it_serves() {
        let table = BridgeTable::default().fdb(vec![FdbEntry::learned(mac(1), 1)]);
        let rows = table.wire_rows(&[]);

        let address = rows
            .iter()
            .find(|row| row.value.type_token() == "octet")
            .expect("the address column is binary");
        assert_eq!(address.value.render(), "00 1a 2b 00 10 01");
        // ...and the last six sub-ids of its OID are the same address.
        let index: Vec<u64> = address.oid[address.oid.len() - 6..].to_vec();
        assert_eq!(index, vec![0, 26, 43, 0, 16, 1]);
    }

    /// Bridge ports follow the interface list, and the count follows the ports.
    #[test]
    fn a_derived_bridge_numbers_its_own_ethernet_ports() {
        let table = BridgeTable::derived();
        assert_eq!(table.port_map(&[1, 2, 3]), vec![(1, 1), (2, 2), (3, 3)]);
        assert_eq!(table.declared_port_count(&[1, 2, 3]), 3);
    }

    /// A device whose mapping encodes a shape keeps it — OS10 breakout lanes are not 1..N.
    #[test]
    fn an_explicit_bridge_keeps_the_mapping_it_was_given() {
        let table = BridgeTable::with_ports(vec![(1, 17301505), (10, 17301514)]);
        assert_eq!(
            table.port_map(&[1, 2, 3]),
            vec![(1, 17301505), (10, 17301514)]
        );
        assert_eq!(table.declared_port_count(&[1, 2, 3]), 2);
    }

    /// A Q-BRIDGE row is keyed by VLAN *and* MAC, one sub-id longer than the legacy table's.
    #[test]
    fn a_vlan_aware_entry_carries_the_vlan_in_its_index() {
        let table = BridgeTable::default().fdb(vec![FdbEntry::learned(mac(1), 1).in_vlan(20)]);
        let rows = table.wire_rows(&[]);
        let port = rows.first().expect("a port column");
        assert_eq!(port.oid[port.oid.len() - 7..], [20, 0, 26, 43, 0, 16, 1]);
    }
}
