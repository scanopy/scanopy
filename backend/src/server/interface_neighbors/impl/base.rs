//! GH #701: an interface's neighbours, split across three tables of very different weight.
//!
//! `InterfaceNeighborCandidate` is resolution's *input* — a disposable staging row, one per
//! distinct raw LLDP/CDP identity a scan heard on a port, replaced wholesale every complete scan.
//! No SCD2, no identity across scans: a candidate that keeps getting reported keeps getting
//! replaced-in and keeps getting retried by the next resolution pass, which produces the same
//! practical durability as identity-tracking would, for free.
//!
//! `InterfaceNeighborInterface` and `InterfaceNeighborHost` are the actual adjacency state, and
//! the only tables topology reads. Real `NOT NULL` FKs — an adjacency lives in exactly one of the
//! two by construction, not by convention. Upserted by natural key (`interface_id,
//! neighbor_interface_id` / `interface_id, neighbor_host_id`), mirroring `Interface`'s own
//! discovery-write pattern (`create_or_update_from_discovery`).
//!
//! All three are deliberately `Storable` (+ `Snapshotable`/`ChildStorableEntity` for the resolved
//! two) rather than the full `Entity` trait: `Entity` pulls in `EntityDiscriminants` registration,
//! CSV export, tag support and generic REST/event-bus exposure none of these tables need, and
//! `DiscoveryTracked` (`shared/storage/snapshot.rs`) requires `Entity` as a hard supertrait bound
//! for exactly that machinery — so the two resolved tables carry their own
//! last_seen_at/last_discovery_id/first_discovery_id fields and inherent methods that mirror
//! `DiscoveryTracked`'s audit-column behaviour without the trait. See `service.rs` for the bespoke
//! read/write paths that replace `CrudService`/`ChildCrudService` (both `Entity`-bound) here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::net::IpAddr;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::lldp::{
    AdvertisedFarEndPort, AdvertisedIdentity, LldpChassisId, LldpPortId, is_usable_identity_address,
};

// ============================================================================
// Shared evidence shape
// ============================================================================

/// The raw identity one LLDP or CDP record advertised about a neighbour on one port.
///
/// One candidate is one record — an LLDP entry and a CDP entry for the same physical neighbour are
/// never merged into a single row, even when they name the same device. That is what lets the
/// reciprocal resolution tier dedup by remote host rather than by candidate count (see
/// `hosts/service/topology/reciprocal.rs`), and what lets `edge_builder.rs` read each adjacency's
/// own evidence-derived protocol instead of an interface-level predicate.
///
/// This is both the wire shape a daemon submits (nested under
/// `InterfaceBase::neighbor_candidates`) and `InterfaceNeighborCandidate`'s stored field set — see
/// `InterfaceNeighborCandidateBase`, which pairs this with the `(network_id, interface_id)` the
/// wire submission does not need to repeat per candidate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct InterfaceNeighborEvidence {
    /// Remote chassis identifier from LLDP neighbor (globally/locally unique)
    #[serde(default)]
    pub lldp_chassis_id: Option<LldpChassisId>,
    /// Remote port identifier from LLDP neighbor
    #[serde(default)]
    pub lldp_port_id: Option<LldpPortId>,
    /// Remote system name from LLDP neighbor (lldpRemSysName)
    #[serde(default)]
    pub lldp_sys_name: Option<String>,
    /// Remote port description from LLDP neighbor (lldpRemPortDesc)
    #[serde(default)]
    pub lldp_port_desc: Option<String>,
    /// Remote management IP from LLDP neighbor (lldpRemManAddr). IPv4 or IPv6.
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "192.168.1.1")]
    pub lldp_mgmt_addr: Option<IpAddr>,
    /// Remote system description from LLDP neighbor (lldpRemSysDesc) - platform info
    #[serde(default)]
    pub lldp_sys_desc: Option<String>,
    /// Remote device ID from CDP (typically hostname, locally unique)
    #[serde(default)]
    pub cdp_device_id: Option<String>,
    /// Remote port ID from CDP
    #[serde(default)]
    pub cdp_port_id: Option<String>,
    /// Remote platform from CDP (e.g., "Cisco IOS")
    #[serde(default)]
    pub cdp_platform: Option<String>,
    /// Remote management IP from CDP (cdpCacheAddress). IPv4 or IPv6.
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "192.168.1.1")]
    pub cdp_address: Option<IpAddr>,
}

impl InterfaceNeighborEvidence {
    /// Whether this row carries raw LLDP data (may or may not be resolved).
    pub fn has_lldp_data(&self) -> bool {
        self.lldp_chassis_id.is_some() || self.lldp_port_id.is_some()
    }

    /// Whether this row carries raw CDP data (may or may not be resolved).
    pub fn has_cdp_data(&self) -> bool {
        self.cdp_device_id.is_some() || self.cdp_port_id.is_some()
    }

    /// Whether this row carries evidence that *something* is adjacent to the port at all — the two
    /// sources L2 resolution's chassis/device-id tiers consume. Mirrors `Interface::
    /// has_neighbor_evidence`'s LLDP/CDP half; the FDB half is evaluated separately, against
    /// `Interface::fdb_macs`, which stays on the port rather than becoming a candidate row.
    pub fn has_neighbor_evidence(&self) -> bool {
        self.lldp_chassis_id.is_some() || self.cdp_device_id.is_some()
    }

    /// Everything the far end published about *itself* besides its chassis id, for the
    /// subtype-independent tiers of `LldpChassisId::resolve_host_id`. The successor to
    /// `Interface::advertised_identity`, now read per candidate rather than per interface.
    ///
    /// One method rather than one per protocol, because the tiers do not care which protocol
    /// carried the value: a CDP device id is a `sysName` and `cdpCacheAddress` is a management
    /// address. The address preference is deliberate: `lldpRemManAddr` is the far end's own
    /// statement of where it is managed, so it comes first; a `NetworkAddress` port id is second
    /// (it names the port rather than the device); `cdpCacheAddress` is last.
    pub fn advertised_identity(&self) -> AdvertisedIdentity<'_> {
        let port_id_address = match self.lldp_port_id {
            Some(LldpPortId::NetworkAddress(addr)) => Some(addr),
            _ => None,
        };

        AdvertisedIdentity {
            sys_name: self
                .lldp_sys_name
                .as_deref()
                .or(self.cdp_device_id.as_deref()),
            address: self
                .lldp_mgmt_addr
                .or(port_id_address)
                .or(self.cdp_address)
                .filter(is_usable_identity_address),
        }
    }

    /// What the far end published about its *own* port on this cable. The successor to
    /// `Interface::advertised_far_end_port`, now read per candidate.
    ///
    /// The name is what an interface minted for the far end is keyed on, so the order is by how
    /// closely each source matches the `ifName` a real walk of that device would return.
    /// `lldpRemPortId` with a naming subtype is that column outright; a CDP port id is the same
    /// thing under another protocol; `lldpRemPortDesc` is `ifDescr`, which is a description before
    /// it is a name and so goes last.
    pub fn advertised_far_end_port(&self) -> AdvertisedFarEndPort<'_> {
        let port_id = self.lldp_port_id.as_ref();
        AdvertisedFarEndPort {
            name: port_id
                .and_then(LldpPortId::port_name)
                .or(self.cdp_port_id.as_deref())
                .or(self.lldp_port_desc.as_deref())
                .map(str::trim)
                .filter(|name| !name.is_empty()),
            mac: port_id.and_then(LldpPortId::port_mac),
        }
    }

    /// Whether this evidence row's remote *port*, if resolved from it, could only have been
    /// matched via a MAC — the LLDP half of the old `Interface::port_bound_by_mac` (GH #668), now
    /// scoped to one candidate rather than a whole interface. The FDB half (a bridge-FDB port that
    /// learned exactly one address) has no candidate row at all — `fdb_macs` stayed on `Interface`
    /// — so a caller re-examining a binding placed via FDB checks `Interface::fdb_macs` directly
    /// instead of going through evidence.
    pub fn port_bound_by_mac(&self) -> bool {
        matches!(self.lldp_port_id, Some(LldpPortId::MacAddress(_)))
    }
}

#[cfg(test)]
mod evidence_tests {
    use super::*;

    /// A port id that names a port becomes a name; one that encodes a MAC becomes a MAC. Ported
    /// from `interfaces/impl/base.rs` — the same assertion, now against the candidate-level
    /// evidence rather than the interface's own (removed) `lldp_port_id`.
    #[test]
    fn a_port_id_that_encodes_a_mac_is_not_read_as_a_name() {
        let named = InterfaceNeighborEvidence {
            lldp_port_id: Some(LldpPortId::InterfaceName("Ten-GigabitEthernet1/0/1".into())),
            ..Default::default()
        };
        let port = named.advertised_far_end_port();
        assert_eq!(port.name, Some("Ten-GigabitEthernet1/0/1"));
        assert_eq!(port.mac, None);

        let by_mac = InterfaceNeighborEvidence {
            lldp_port_id: Some(LldpPortId::MacAddress("e8:80:88:be:30:e7".into())),
            ..Default::default()
        };
        let port = by_mac.advertised_far_end_port();
        assert_eq!(
            port.name, None,
            "a MAC identifies the port without naming it"
        );
        assert_eq!(port.mac, Some("e8:80:88:be:30:e7"));
    }

    /// The sources are tried by how closely each matches the `ifName` a real walk would return, and
    /// a port id that carries no name at all falls through to the ones that do.
    #[test]
    fn a_far_end_port_falls_back_past_an_id_that_does_not_name_it() {
        let cdp = InterfaceNeighborEvidence {
            lldp_port_id: Some(LldpPortId::MacAddress("e8:80:88:be:30:e7".into())),
            cdp_port_id: Some("GigabitEthernet1/0/8".into()),
            ..Default::default()
        };
        assert_eq!(
            cdp.advertised_far_end_port().name,
            Some("GigabitEthernet1/0/8"),
            "a CDP port id is the same statement under another protocol"
        );

        let desc_only = InterfaceNeighborEvidence {
            lldp_port_desc: Some("  eth0  ".into()),
            ..Default::default()
        };
        assert_eq!(
            desc_only.advertised_far_end_port().name,
            Some("eth0"),
            "the description is last, and trimmed"
        );

        let blank = InterfaceNeighborEvidence {
            lldp_port_desc: Some("   ".into()),
            ..Default::default()
        };
        assert_eq!(
            blank.advertised_far_end_port().name,
            None,
            "whitespace is not a port name, and a row keyed on it would match nothing forever"
        );
    }

    /// A port id names a port on a device this row cannot identify — on its own it is not
    /// evidence that anything is there.
    #[test]
    fn a_port_id_alone_is_not_neighbor_evidence() {
        let port_id_only = InterfaceNeighborEvidence {
            lldp_port_id: Some(LldpPortId::InterfaceName("Slot0/3".into())),
            lldp_port_desc: Some("uplink".into()),
            ..Default::default()
        };
        assert!(!port_id_only.has_neighbor_evidence());
    }

    /// Only an LLDP port id of subtype 3 (`macAddress`) rests on MAC uniqueness; a named port id
    /// was never resting on it, and re-opening it would tear down a healthy link on every scan.
    /// Ported from `interfaces/impl/base.rs`'s `only_a_mac_matched_port_is_worth_re_examining`.
    #[test]
    fn only_a_mac_port_id_is_bound_by_mac() {
        let by_mac = InterfaceNeighborEvidence {
            lldp_chassis_id: Some(LldpChassisId::MacAddress("00:ad:24:af:4e:00".into())),
            lldp_port_id: Some(LldpPortId::MacAddress("00:ad:24:af:4e:00".into())),
            ..Default::default()
        };
        assert!(by_mac.port_bound_by_mac());

        let by_name = InterfaceNeighborEvidence {
            lldp_chassis_id: Some(LldpChassisId::MacAddress("00:ad:24:af:4e:00".into())),
            lldp_port_id: Some(LldpPortId::InterfaceName("Slot0/3".into())),
            ..Default::default()
        };
        assert!(!by_name.port_bound_by_mac());
    }
}

// ============================================================================
// InterfaceNeighborCandidate — disposable staging row
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct InterfaceNeighborCandidateBase {
    /// The network the reporting interface belongs to.
    pub network_id: Uuid,
    /// The local interface that heard this neighbour advertisement.
    pub interface_id: Uuid,
    pub evidence: InterfaceNeighborEvidence,
}

impl InterfaceNeighborCandidateBase {
    pub fn new(network_id: Uuid, interface_id: Uuid, evidence: InterfaceNeighborEvidence) -> Self {
        Self {
            network_id,
            interface_id,
            evidence,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct InterfaceNeighborCandidate {
    /// This candidate row's own id. Candidates are replaced wholesale on every scan, so the id
    /// does not survive from one scan to the next.
    pub id: Uuid,
    /// When a scan last reported this evidence. Resolution reads it as the evidence's freshness,
    /// so a group a scan could not finish reading keeps its previous value.
    pub created_at: DateTime<Utc>,
    pub base: InterfaceNeighborCandidateBase,
}

impl InterfaceNeighborCandidate {
    pub fn new(base: InterfaceNeighborCandidateBase) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            base,
        }
    }
}

impl Display for InterfaceNeighborCandidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InterfaceNeighborCandidate(interface={}, lldp={}, cdp={})",
            self.base.interface_id,
            self.base.evidence.has_lldp_data(),
            self.base.evidence.has_cdp_data()
        )
    }
}

// ============================================================================
// InterfaceNeighborInterface — full resolution (remote port known)
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InterfaceNeighborInterfaceBase {
    pub network_id: Uuid,
    pub interface_id: Uuid,
    pub neighbor_interface_id: Uuid,
    pub neighbor_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InterfaceNeighborInterface {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
    pub lineage_id: Option<Uuid>,
    pub last_seen_at: DateTime<Utc>,
    pub last_discovery_id: Option<Uuid>,
    pub first_discovery_id: Option<Uuid>,
    pub base: InterfaceNeighborInterfaceBase,
}

impl InterfaceNeighborInterface {
    pub fn new(base: InterfaceNeighborInterfaceBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base,
        }
    }

    /// Refresh-style timestamp that advances on every observation, matching
    /// `DiscoveryTracked::refresh_scan_timestamps` for the tables that cannot implement that trait
    /// (see module docs). Sets `last_seen_at` and `updated_at`.
    pub fn refresh_scan_timestamps(&mut self, scan_time: DateTime<Utc>) {
        self.last_seen_at = scan_time;
        self.updated_at = scan_time;
    }

    /// Origin-style timestamp for a first insert, matching
    /// `DiscoveryTracked::originate_scan_timestamps`. Sets `created_at` and `valid_from`.
    pub fn originate_scan_timestamps(&mut self, scan_time: DateTime<Utc>) {
        self.created_at = scan_time;
        self.valid_from = scan_time;
    }
}

impl Display for InterfaceNeighborInterface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InterfaceNeighborInterface({} -> {})",
            self.base.interface_id, self.base.neighbor_interface_id
        )
    }
}

// ============================================================================
// InterfaceNeighborHost — partial resolution (remote device known, port not)
// ============================================================================

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InterfaceNeighborHostBase {
    pub network_id: Uuid,
    pub interface_id: Uuid,
    pub neighbor_host_id: Uuid,
    pub neighbor_seen_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InterfaceNeighborHost {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
    pub lineage_id: Option<Uuid>,
    pub last_seen_at: DateTime<Utc>,
    pub last_discovery_id: Option<Uuid>,
    pub first_discovery_id: Option<Uuid>,
    pub base: InterfaceNeighborHostBase,
}

impl InterfaceNeighborHost {
    pub fn new(base: InterfaceNeighborHostBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base,
        }
    }

    /// See `InterfaceNeighborInterface::refresh_scan_timestamps`.
    pub fn refresh_scan_timestamps(&mut self, scan_time: DateTime<Utc>) {
        self.last_seen_at = scan_time;
        self.updated_at = scan_time;
    }

    /// See `InterfaceNeighborInterface::originate_scan_timestamps`.
    pub fn originate_scan_timestamps(&mut self, scan_time: DateTime<Utc>) {
        self.created_at = scan_time;
        self.valid_from = scan_time;
    }
}

impl Display for InterfaceNeighborHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "InterfaceNeighborHost({} -> {})",
            self.base.interface_id, self.base.neighbor_host_id
        )
    }
}

// ============================================================================
// Merged read model — what topology and the API actually consume
// ============================================================================

/// One resolved adjacency, tagged by which of the two tables it came from — the same shape
/// `Neighbor` gave a single interface before GH #701, now one per row rather than at most one per
/// interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "id")]
pub enum Neighbor {
    /// Full resolution - the specific remote port was identified
    #[schema(title = "Interface")]
    Interface(Uuid),
    /// Partial resolution - the remote device was identified but not the specific port
    #[schema(title = "Host")]
    Host(Uuid),
}

impl Neighbor {
    pub fn interface_id(&self) -> Option<Uuid> {
        match self {
            Neighbor::Interface(id) => Some(*id),
            Neighbor::Host(_) => None,
        }
    }

    pub fn is_full_resolution(&self) -> bool {
        matches!(self, Neighbor::Interface(_))
    }

    pub fn is_partial_resolution(&self) -> bool {
        matches!(self, Neighbor::Host(_))
    }
}

/// One row of the merged read model: an interface's adjacency to one neighbour, whichever of the
/// two resolved tables it lives in. `TopologyContext` loads a `Vec` of these per network
/// (`live_or_as_of`-pinned the same way `interfaces` is) instead of reading a scalar field —
/// `InterfaceNeighborRow` is what every topology file downstream of resolution actually iterates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct InterfaceNeighborRow {
    /// The resolved row's own id (from whichever of the two tables it came from) — carried so a
    /// row can be told apart from another row naming the same pair (not possible today, since the
    /// natural key is unique per table, but keeps the type honest about which row backs it).
    pub id: Uuid,
    /// The local interface this adjacency belongs to.
    pub interface_id: Uuid,
    pub neighbor: Neighbor,
    /// When a scan last saw the evidence behind this adjacency. Absent when no scan has recorded
    /// a time for it.
    pub neighbor_seen_at: Option<DateTime<Utc>>,
}

impl From<&InterfaceNeighborInterface> for InterfaceNeighborRow {
    fn from(row: &InterfaceNeighborInterface) -> Self {
        Self {
            id: row.id,
            interface_id: row.base.interface_id,
            neighbor: Neighbor::Interface(row.base.neighbor_interface_id),
            neighbor_seen_at: row.base.neighbor_seen_at,
        }
    }
}

impl From<&InterfaceNeighborHost> for InterfaceNeighborRow {
    fn from(row: &InterfaceNeighborHost) -> Self {
        Self {
            id: row.id,
            interface_id: row.base.interface_id,
            neighbor: Neighbor::Host(row.base.neighbor_host_id),
            neighbor_seen_at: row.base.neighbor_seen_at,
        }
    }
}
