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

use anyhow::Result;
use snmp2::Value;

use super::SimDevice;
use super::transport::SimAgent;
use crate::daemon::discovery::integration::snmp::catalog::SnmpRequest;
use crate::daemon::discovery::integration::snmp::oids::oid_parts;
use crate::daemon::discovery::integration::snmp::queries::{
    IfTableWalk, SnmpCollection, SnmpWalkTransport, Varbinds, WalkPage,
};
use crate::daemon::discovery::integration::snmp::types::{
    ArpEntry, BridgeFdbEntry, CdpNeighbor, DeviceInventory, IfTableEntry, IpAddrEntry,
    LldpLocalPort, LldpNeighbor, PortVlanMembership, SystemInfo, VlanInfo,
};
use crate::daemon::discovery::integration::snmp::{
    LocalPortOutcome, count_dropped_neighbours, query_arp_table, query_bridge_fdb,
    query_bridge_port_mapping, query_cdp_neighbors, query_entity_physical, query_ip_addr_table,
    query_lldp_local_ports, query_lldp_neighbors, query_port_vlan_membership, query_system_info,
    query_vlan_table, remap_lldp_local_ports, walk_if_table,
};
use crate::daemon::discovery::service::warnings::{
    ClaimSource, DeviceClaim, SnmpCollectionOutcome, SnmpGroupOutcome, snmp_device_findings,
    warn_incomplete_snmp_walks, warn_snmp_collected_nothing,
};
use crate::daemon::discovery::types::warnings::DiscoveryWarning;

/// Everything one scan of a device reads.
pub struct Collected {
    pub system: SystemInfo,
    pub if_table: IfTableWalk,
    pub local_ports: HashMap<i32, LldpLocalPort>,
    /// Whether the `lldpLocPortTable` read finished. Kept alongside the rows because a placement
    /// that failed reads completely differently depending on it: with a whole table the device
    /// numbers its ports in a space of its own, and with a partial one we simply did not read the
    /// numbering (GH #668).
    pub local_ports_complete: bool,
    /// Neighbours *after* the local-port remap, which is the form the rest of the daemon sees.
    pub neighbours: SnmpCollection<Vec<LldpNeighbor>>,
    /// What the remap could and could not place.
    pub local_port_outcome: LocalPortOutcome,
    /// Neighbours whose final index names no interface, or names one another neighbour already
    /// claimed. Every one is discarded whole when interfaces are built.
    pub dropped_neighbours: usize,
    pub arp: SnmpCollection<Vec<ArpEntry>>,
    /// CDP neighbours, read the way `SnmpIntegration::execute` reads them. Collected here because
    /// a column the simulator emits and nothing reads back proves only that it was written.
    pub cdp: SnmpCollection<Vec<CdpNeighbor>>,
    pub bridge_ports: HashMap<i32, i32>,
    pub fdb: SnmpCollection<Vec<BridgeFdbEntry>>,
    /// The `entPhysicalTable` collapsed to one device, the way `SnmpIntegration::execute` reads it.
    /// Collected here because until it was, `query_entity_physical` was referenced by no test at
    /// all — every column the fixtures emitted for it proved only that it had been written.
    pub entity: SnmpCollection<Option<DeviceInventory>>,
    pub ip_addresses: usize,
    /// The raw `ipAddrTable` walk, keyed by address — what `convert_snmp_if_entry` reads to decide
    /// `InterfaceBase::ip_configured` (GH #668). `ip_addresses` above only counts rows; this is for
    /// tests that need to know *which* ifIndex a row names.
    pub ip_addr_table: HashMap<IpAddr, IpAddrEntry>,
    /// The VLAN table and each port's native and tagged VLANs, by ifIndex.
    pub vlans: Vec<VlanInfo>,
    pub vlan_membership: Vec<PortVlanMembership>,
    /// VLAN member ports the device named that matched no interface, which `execute` reports as
    /// a warning.
    pub vlan_ports_unresolved: usize,
    /// Every group's outcome, built from the reads above as `SnmpIntegration::execute` builds it,
    /// so a device test can run the same warning derivation the daemon runs.
    pub outcome: SnmpCollectionOutcome,
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

/// One request a collection made of a device.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub oid: Vec<u64>,
    pub request: SnmpRequest,
    /// The page size asked for, on a GETBULK.
    pub max_repetitions: Option<u32>,
}

impl Recorded {
    pub fn is_under(&self, subtree: &str) -> bool {
        self.oid.starts_with(&oid_parts(subtree))
    }
}

/// A sim agent that remembers every request made of it.
pub struct Recording {
    inner: SimAgent,
    pub requests: Vec<Recorded>,
}

impl Recording {
    pub fn new(inner: SimAgent) -> Self {
        Self {
            inner,
            requests: Vec::new(),
        }
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for Recording {
    fn getbulk_unusable(&self) -> bool {
        self.inner.getbulk_unusable()
    }

    fn note_getbulk_unusable(&mut self) {
        self.inner.note_getbulk_unusable()
    }

    async fn walk_getbulk<'a>(
        &'a mut self,
        from: &[u64],
        max_repetitions: u32,
    ) -> Result<WalkPage<'a>> {
        self.requests.push(Recorded {
            oid: from.to_vec(),
            request: SnmpRequest::Walk,
            max_repetitions: Some(max_repetitions),
        });
        self.inner.walk_getbulk(from, max_repetitions).await
    }

    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
        self.requests.push(Recorded {
            oid: from.to_vec(),
            request: SnmpRequest::Walk,
            max_repetitions: None,
        });
        self.inner.walk_getnext(from).await
    }

    async fn get_scalar<'a>(&'a mut self, oid: &[u64]) -> Result<Option<Value<'a>>> {
        self.requests.push(Recorded {
            oid: oid.to_vec(),
            request: SnmpRequest::Get,
            max_repetitions: None,
        });
        self.inner.get_scalar(oid).await
    }
}

/// Run a full collection against a device.
pub async fn collect(device: &SimDevice) -> Collected {
    collect_over(device, &mut device.agent()).await
}

/// Collect one device by name, returning every request the collection sent with what it read.
pub async fn recorded_scan(name: &str) -> (Collected, Vec<Recorded>) {
    let device = super::device(name);
    let mut agent = Recording::new(device.agent());
    let collected = collect_over(&device, &mut agent).await;
    (collected, agent.requests)
}

/// Run a full collection against a device through `agent`.
async fn collect_over<T: SnmpWalkTransport>(device: &SimDevice, agent: &mut T) -> Collected {
    let ip: IpAddr = device.ip.into();

    let system = query_system_info(agent, ip).await.unwrap_or_default();
    let if_table = walk_if_table(agent, ip).await.unwrap_or_default();

    // The local-port table is read before the neighbours because the remap needs both, which is
    // the order `execute` uses.
    let local_port_walk = query_lldp_local_ports(agent, ip).await.unwrap_or_default();
    let local_ports_complete = local_port_walk.complete;
    let local_port_walk_reason = local_port_walk.reason;
    let local_ports = local_port_walk.records;
    let mut neighbours = query_lldp_neighbors(agent, ip).await.unwrap_or_default();
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

    let arp = query_arp_table(agent, ip).await.unwrap_or_default();
    let cdp = query_cdp_neighbors(agent, ip).await.unwrap_or_default();
    let bridge_mapping = query_bridge_port_mapping(agent, ip)
        .await
        .unwrap_or_default();
    let fdb = query_bridge_fdb(agent, ip, &bridge_mapping)
        .await
        .unwrap_or_default();
    let entity = query_entity_physical(agent, ip).await.unwrap_or_default();
    let ip_addr_walk = query_ip_addr_table(agent, ip).await.unwrap_or_default();
    // From the definition, not from `system`: snmpd answers the system group itself and the
    // in-process agent models only the `pass` tables, so the scalars above come back empty here.
    let sys_object_id = device.system.sys_object_id.as_deref();
    let vlan_names = query_vlan_table(agent, ip, sys_object_id)
        .await
        .unwrap_or_default();
    let vlan_membership = query_port_vlan_membership(agent, ip, &bridge_mapping, sys_object_id)
        .await
        .unwrap_or_default();
    let vlan_ports_unresolved = vlan_membership.discarded;

    // `execute` reads the local port table only when there are classic neighbours to place, and
    // a table it never asked for is not a shortfall.
    let lldp_local_ports = if neighbours.records.is_empty() || neighbours.local_port_is_if_index {
        SnmpGroupOutcome {
            complete: true,
            ..Default::default()
        }
    } else {
        SnmpGroupOutcome {
            complete: local_ports_complete,
            observed: local_ports.len(),
            reason: local_port_walk_reason,
            claim: None,
        }
    };
    let bridge_bit_claim = system
        .sys_services
        .is_some_and(|services| services & 0x02 != 0)
        .then_some(DeviceClaim::Implements {
            source: ClaimSource::SysServicesBridgeBit,
        });
    let mut bridge_port_numbering = bridge_mapping.group_outcome(bridge_mapping.records.len());
    bridge_port_numbering.claim = bridge_port_numbering.claim.or(bridge_bit_claim);
    let outcome = SnmpCollectionOutcome {
        lldp: neighbours.group_outcome(neighbours.records.len()),
        cdp: cdp.group_outcome(cdp.records.len()),
        interfaces: SnmpGroupOutcome {
            complete: if_table.set_complete,
            observed: if_table.entries.len(),
            reason: if_table.reason,
            claim: system
                .if_number
                .filter(|count| *count > 0)
                .map(|count| DeviceClaim::Count {
                    source: ClaimSource::IfNumber,
                    expected: count as usize,
                }),
        },
        bridge_port_numbering,
        bridge_forwarding: fdb.group_outcome(fdb.records.len()),
        vlan_membership: vlan_membership.group_outcome(vlan_membership.records.len()),
        arp_table: arp.group_outcome(arp.records.len()),
        device_inventory: entity.group_outcome(usize::from(entity.records.is_some())),
        ip_addresses: ip_addr_walk.group_outcome(ip_addr_walk.records.len()),
        lldp_local_ports,
        vlan_names: vlan_names.group_outcome(vlan_names.records.len()),
    };

    let bridge_ports = bridge_mapping.records;
    let ip_addr_table = ip_addr_walk.records;
    let ip_addresses = ip_addr_table.len();

    Collected {
        system,
        if_table,
        local_ports,
        local_ports_complete,
        neighbours,
        local_port_outcome,
        dropped_neighbours,
        arp,
        cdp,
        bridge_ports,
        fdb,
        entity,
        ip_addresses,
        ip_addr_table,
        vlans: vlan_names.records,
        vlan_membership: vlan_membership.records,
        vlan_ports_unresolved,
        outcome,
    }
}

/// Collect one device by name.
pub async fn scan(name: &str) -> Collected {
    collect(&super::device(name)).await
}

/// The walk warnings one scan of a device raises, coded as the daemon codes them: the per-group
/// shortfalls, then the whole-device line when it returned nothing.
pub async fn walk_warnings(name: &str) -> Vec<DiscoveryWarning> {
    let device = super::device(name);
    let collected = collect(&device).await;
    let (shortfalls, collected_nothing) = snmp_device_findings(
        device.ip.into(),
        Default::default(),
        Default::default(),
        collected.outcome,
    );
    let mut warnings = warn_incomplete_snmp_walks(&shortfalls);
    warnings.extend(warn_snmp_collected_nothing(collected_nothing.as_slice()));
    warnings
}
