//! Drive the real collection against a simulated device.
//!
//! [`collect`] runs the daemon's own query functions, in the order and with the dependencies
//! `SnmpIntegration::execute` uses, against a device's [`super::transport::SimAgent`]. A device
//! test then asserts on what the daemon actually read — not on the fixture, which would only
//! prove the fixture equals itself.
//!
//! What it deliberately does not do is persist anything. Defects that were resolver or SQL
//! semantics failures need a database to fail the way they originally did, and those live in
//! `crate::tests::snmp_sim_resolution` against a real Postgres.

use std::collections::HashMap;
use std::net::IpAddr;

use super::SimDevice;
use crate::daemon::discovery::integration::snmp::queries::{IfTableWalk, SnmpCollection};
use crate::daemon::discovery::integration::snmp::types::{
    ArpEntry, BridgeFdbEntry, IfTableEntry, LldpLocalPort, LldpNeighbor, SystemInfo,
};
use crate::daemon::discovery::integration::snmp::{
    LocalPortOutcome, count_dropped_neighbours, query_arp_table, query_bridge_fdb,
    query_bridge_port_mapping, query_ip_addr_table, query_lldp_local_ports, query_lldp_neighbors,
    query_system_info, remap_lldp_local_ports, walk_if_table,
};

/// Everything one scan of a device reads.
pub struct Collected {
    pub system: SystemInfo,
    pub if_table: IfTableWalk,
    pub local_ports: HashMap<i32, LldpLocalPort>,
    /// Neighbours *after* the local-port remap, which is the form the rest of the daemon sees.
    pub neighbours: SnmpCollection<Vec<LldpNeighbor>>,
    /// What the remap could and could not place.
    pub local_port_outcome: LocalPortOutcome,
    /// Neighbours whose final index names no interface, or names one another neighbour already
    /// claimed. Every one is discarded whole when interfaces are built.
    pub dropped_neighbours: usize,
    pub arp: SnmpCollection<Vec<ArpEntry>>,
    pub bridge_ports: HashMap<i32, i32>,
    pub fdb: SnmpCollection<Vec<BridgeFdbEntry>>,
    pub ip_addresses: usize,
}

impl Collected {
    /// One interface by ifIndex.
    pub fn interface(&self, if_index: i32) -> &IfTableEntry {
        self.if_table
            .entries
            .iter()
            .find(|e| e.if_index == if_index)
            .unwrap_or_else(|| panic!("no interface at ifIndex {if_index}"))
    }

    /// The neighbours sitting on one local port, after the remap.
    pub fn neighbours_on(&self, if_index: i32) -> Vec<&LldpNeighbor> {
        self.neighbours
            .records
            .iter()
            .filter(|n| n.local_port_index == if_index)
            .collect()
    }

    /// Every neighbour naming a far end, by its advertised system name.
    ///
    /// Plural because a pair of devices can legitimately be cabled twice — `switch-aruba-01` and
    /// `switch-netgear-01` are, on purpose, and that pair is half of what GH #664 and #649 are
    /// about.
    pub fn neighbours_named(&self, sys_name: &str) -> Vec<&LldpNeighbor> {
        self.neighbours
            .records
            .iter()
            .filter(|n| n.remote_sys_name.as_deref() == Some(sys_name))
            .collect()
    }

    /// The single neighbour naming a far end. Panics if the far end is named more than once, which
    /// is a real distinction: two links to one device is a different topology from one.
    pub fn neighbour_named(&self, sys_name: &str) -> &LldpNeighbor {
        let mut found = self
            .neighbours
            .records
            .iter()
            .filter(|n| n.remote_sys_name.as_deref() == Some(sys_name));
        let first = found
            .next()
            .unwrap_or_else(|| panic!("no neighbour advertising sysName {sys_name}"));
        assert!(
            found.next().is_none(),
            "more than one neighbour advertises {sys_name}"
        );
        first
    }
}

/// Run a full collection against a device.
pub async fn collect(device: &SimDevice) -> Collected {
    let mut agent = device.agent();
    let ip: IpAddr = device.ip.into();

    let system = query_system_info(&mut agent, ip).await.unwrap_or_default();
    let if_table = walk_if_table(&mut agent, ip).await.unwrap_or_default();

    // The local-port table is read before the neighbours because the remap needs both, which is
    // the order `execute` uses.
    let local_ports = query_lldp_local_ports(&mut agent, ip)
        .await
        .map(|c| c.records)
        .unwrap_or_default();
    let mut neighbours = query_lldp_neighbors(&mut agent, ip)
        .await
        .unwrap_or_default();
    // As in `execute`: neighbours read from the LLDP-V2-MIB are keyed by ifIndex already and are
    // not remapped (GH #688).
    let local_port_outcome = if neighbours.local_port_is_if_index {
        LocalPortOutcome {
            unmatched: 0,
            dropped: count_dropped_neighbours(&neighbours.records, &local_ports, &if_table.entries),
        }
    } else {
        remap_lldp_local_ports(&mut neighbours.records, &local_ports, &if_table.entries)
    };
    let dropped_neighbours =
        count_dropped_neighbours(&neighbours.records, &local_ports, &if_table.entries);

    let arp = query_arp_table(&mut agent, ip).await.unwrap_or_default();
    let bridge_mapping = query_bridge_port_mapping(&mut agent, ip)
        .await
        .unwrap_or_default();
    let fdb = query_bridge_fdb(&mut agent, ip, &bridge_mapping)
        .await
        .unwrap_or_default();
    let bridge_ports = bridge_mapping.records;
    let ip_addresses = query_ip_addr_table(&mut agent, ip)
        .await
        .map(|c| c.records.len())
        .unwrap_or_default();

    Collected {
        system,
        if_table,
        local_ports,
        neighbours,
        local_port_outcome,
        dropped_neighbours,
        arp,
        bridge_ports,
        fdb,
        ip_addresses,
    }
}

/// Collect one device by name.
pub async fn scan(name: &str) -> Collected {
    collect(&super::device(name)).await
}
