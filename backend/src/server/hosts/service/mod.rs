use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::r#impl::base::{Binding, BindingType},
    credentials::service::CredentialService,
    daemons::{r#impl::base::Daemon, service::DaemonService},
    hosts::r#impl::{
        api::{
            BindingInput, ConflictBehavior, CreateHostRequest, HostResponse, IPAddressInput,
            InterfaceInput, PortInput, ServiceInput, UpdateHostRequest,
        },
        attributes::{
            HostChassisIdValue, HostHostnameValue, HostManagementUrlValue, HostSysContactValue,
            HostSysDescrValue, HostSysLocationValue, HostSysNameValue, HostSysObjectIdValue,
        },
        base::{Host, HostBase},
        name::{HostName, HostNameSources},
    },
    interface_neighbors::{r#impl::base::Neighbor, service::InterfaceNeighborService},
    interfaces::{
        r#impl::base::{Interface, InterfaceDataComplete},
        service::InterfaceService,
    },
    ip_addresses::{
        r#impl::base::{IPAddress, mac_of},
        service::IPAddressService,
    },
    lldp::{IdentityResolution, LldpInventorySnapshot, LldpResolver, resolver::LldpResolverImpl},
    networks::service::NetworkService,
    organizations::service::OrganizationService,
    ports::{r#impl::base::Port, service::PortService},
    services::{
        r#impl::{base::Service, definitions::ServiceDefinitionExt},
        service::ServiceService,
    },
    shared::attribution::{self, AttributeSource, Attributed},
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        events::{bus::EventBus, types::EntityOperation},
        position::resolve_and_validate_input_positions,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            lock::{CONSOLIDATE_LOCK_TIMEOUT, DEFAULT_LOCK_TIMEOUT, LockKey},
            traits::{Entity, PaginatedResult, Storable, Storage},
        },
        types::{
            api::ValidationError,
            entities::{EntitySource, EntitySourceDiscriminants},
        },
    },
    subnets::{r#impl::base::Subnet, service::SubnetService},
    tags::entity_tags::EntityTagService,
    vlans::service::VlanService,
};
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    sync::Arc,
};
use strum::IntoDiscriminant;
use uuid::Uuid;

pub struct HostLimitContext {
    pub limit: u64,
    pub org_id: Uuid,
    pub org_network_ids: Vec<Uuid>,
    pub plan: crate::server::billing::types::base::BillingPlan,
}

pub struct HostService {
    storage: Arc<GenericPostgresStorage<Host>>,
    ip_address_service: Arc<IPAddressService>,
    port_service: Arc<PortService>,
    service_service: Arc<ServiceService>,
    interface_service: Arc<InterfaceService>,
    interface_neighbor_service: Arc<InterfaceNeighborService>,
    pub daemon_service: Arc<DaemonService>,
    credential_service: Arc<CredentialService>,
    subnet_service: Arc<SubnetService>,
    vlan_service: Arc<VlanService>,
    /// Reads the network's staleness window, which is what decides whether a link's neighbour
    /// evidence still counts. Via the service, never the storage layer.
    network_service: Arc<NetworkService>,
    /// Reads the plan a network's organization is on, so minting a host for an unplaceable far end
    /// is gated by the same limit every other creation path is. Via the service, never storage.
    organization_service: Arc<OrganizationService>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<Host> for HostService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Host) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Host) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Host> for HostService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Host>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    /// Create a new host, or upsert if a matching host exists.
    ///
    /// This method uses `Host::eq` (ID comparison) to find existing hosts.
    /// For discovery workflows, `create_with_children` sets the incoming host's ID
    /// to match an existing host found via IP-address comparison, so this method
    /// will find the match and trigger `upsert_host()`.
    ///
    /// Upsert conditions:
    /// - Both hosts are from discovery (merges discovery metadata)
    /// - OR the IDs already match (handles re-discovery of known hosts)
    async fn create(&self, host: Host, authentication: AuthenticatedEntity) -> Result<Host> {
        // DB-level lock, scoped to the network: serializes the dedup in
        // `create_unlocked` across all backend instances. Keyed by network
        // (not host id) because two concurrent submissions of the same NEW
        // device carry distinct fresh UUIDs. Error paths release via Drop.
        //
        // NOTE: this ID-based dedup alone cannot catch two fresh-UUID
        // submissions of the same physical device — the IP/MAC natural-key
        // match lives in `create_with_children`, which holds this same lock
        // across match + create + IP insertion and therefore calls
        // `create_unlocked` directly.
        let dedup_guard = self
            .storage()
            .session_lock(
                LockKey::HostDedup {
                    network_id: host.base.network_id,
                },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?;
        let created = self.create_unlocked(host, authentication).await?;
        dedup_guard.release().await?;
        Ok(created)
    }

    async fn update(
        &self,
        updates: &mut Host,
        authentication: AuthenticatedEntity,
    ) -> Result<Host, Error> {
        let lock_guard = self
            .storage()
            .session_lock(LockKey::Host(updates.id), DEFAULT_LOCK_TIMEOUT)
            .await?;

        let current_host = self
            .get_by_id(&updates.id)
            .await?
            .ok_or_else(|| anyhow!("Host '{}' not found", updates.id))?;

        let updated = self.storage().update(updates).await?;
        let trigger_stale = updated.triggers_staleness(Some(current_host));

        if let Some(scope) = EntityScope::from_ids(
            updated.id(),
            updated.clone().into(),
            self.get_network_id(&updated),
            self.get_organization_id(&updated),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Updated, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        lock_guard.release().await?;
        Ok(updated)
    }
}

mod consolidate;
mod create;
mod delete;
mod discovery;
mod lifecycle;
pub(crate) mod mac_identity;
mod topology;
mod update;

/// Statistics from LLDP link resolution.
///
/// What survives here are the counters no warning covers: the successes, and the one failure mode
/// that is not worth a warning. The five per-reason failure counters this used to carry are now
/// exactly the count of their `DiscoveryWarning`s, and keeping both would leave two sources for
/// one number that can silently disagree — with the warnings being the ones an operator can
/// actually read, since they reach the scan record rather than only the container log.
#[derive(Default, Debug)]
pub struct LldpResolutionStats {
    /// Total number of interfaces with unresolved LLDP data
    pub total: usize,
    /// Number of interfaces where remote host was resolved
    pub hosts_resolved: usize,
    /// Number of interfaces where remote port (interface) was resolved
    pub ports_resolved: usize,
    /// How many of `ports_resolved` were placed by the reciprocal-LLDP tier rather than by an
    /// identifier the far end advertised.
    ///
    /// Separated because it is the tier that carries the switch families reporting one chassis MAC
    /// across every port: a figure of zero on a network full of such devices says the pairing is
    /// not firing, which no other counter distinguishes from "nothing needed it".
    pub ports_resolved_reciprocal: usize,
    /// Neighbor advertised no identifier any strategy can look up.
    ///
    /// The only failure counter left, because it is the only one with no warning behind it: it
    /// counts the `cdp_address`-only rows there was never anything to resolve in, and a warning
    /// per one of those would bury the ones that mean something.
    pub host_no_strategy: usize,
}

/// What one LLDP/CDP resolution pass produced.
///
/// The stats go to the summary log line; the warnings go onto the scan record the operator reads,
/// because a self-hosted operator has no other way to see why the physical view is sparse.
pub struct LldpResolutionOutcome {
    pub stats: LldpResolutionStats,
    /// One coded warning per far end that could not be placed, carrying the evidence needed to
    /// triage it. Coded like the daemon's, so both producers reach the same metric.
    pub warnings: Vec<DiscoveryWarning>,
    /// Hosts this pass minted for far ends nothing had scanned.
    ///
    /// The caller folds these into the session's scanned set before the scan record is written, so
    /// they pick up the discovery FKs and the digest entry every other entity of that scan gets.
    pub minted_host_ids: Vec<Uuid>,
}

impl LldpResolutionStats {
    /// Record a remote-host resolution outcome, returning the host id when there is one.
    pub fn record_host(&mut self, host: IdentityResolution) -> Option<Uuid> {
        match host {
            IdentityResolution::Resolved(host_id) => {
                self.hosts_resolved += 1;
                Some(host_id)
            }
            IdentityResolution::NoStrategy => {
                self.host_no_strategy += 1;
                None
            }
            // Both reach the operator as a warning apiece rather than as a counter — see
            // `LldpNeighbourNotFound` and `LldpNeighbourAmbiguous`, which call for opposite
            // things: an identifier matching nothing is a device that was never discovered,
            // while one matching two is a device we have, twice over.
            IdentityResolution::NotFound | IdentityResolution::Ambiguous => None,
        }
    }

    /// Record a remote-port resolution outcome against an already-known host, returning the
    /// neighbor to persist. Falling back to `Neighbor::Host` keeps the identification we do have.
    pub fn record_port(&mut self, port: IdentityResolution, host_id: Uuid) -> Neighbor {
        match port {
            IdentityResolution::Resolved(interface_id) => {
                self.ports_resolved += 1;
                Neighbor::Interface(interface_id)
            }
            // Each of the three is its own `LldpPort*` warning, which is where the distinction
            // between them now lives. Falling back to `Neighbor::Host` keeps the identification
            // we do have.
            IdentityResolution::NoStrategy
            | IdentityResolution::NotFound
            | IdentityResolution::Ambiguous => Neighbor::Host(host_id),
        }
    }
}

/// Check whether a claimer's `(port_id, ip_address_id)` overlaps with an
/// Open Ports binding's `(port_id, ip_address_id)`.
/// Uses the same semantics as `partition_conflicting_bindings`:
/// None (all ip_addresses) overlaps with anything, Some(a) overlaps Some(a).
fn bindings_overlap(claim_iface: &Option<Uuid>, op_iface: &Option<Uuid>) -> bool {
    match (claim_iface, op_iface) {
        (None, _) | (_, None) => true,
        (Some(a), Some(b)) => a == b,
    }
}

/// Detect VRRP/CARP/HSRP virtual router MAC addresses by their well-known prefixes.
///
/// Virtual router protocols assign deterministic MACs shared across physical router peers.
/// These must never anchor *MAC-based* host identity, or two physical routers in the same
/// redundancy group would dedup into a single host. They remain usable for address-based
/// identity — see `select_matching_host`.
///
/// The VRRP/HSRP group ID is encoded in the last byte(s) of the MAC itself, so detection
/// requires only the MAC prefix — no SNMP MIB query needed.
///
/// Widening this predicate also widens the MAC exclusion in pass 1 of `select_matching_host`,
/// so a newly covered range must always ship together with the pass-2 address fallback —
/// otherwise hosts wearing that range lose MAC matching without gaining anything back.
pub(crate) fn is_virtual_router_mac(mac: &MacAddress) -> bool {
    let bytes = mac.bytes();
    // VRRP IPv4 (RFC 5798 §7.3): 00:00:5e:00:01:XX where XX = VRID (0-255).
    // FreeBSD/OPNsense CARP reuses this range, keyed by vhid.
    (bytes[0..5] == [0x00, 0x00, 0x5e, 0x00, 0x01])
    // VRRP IPv6 (RFC 5798 §7.3): 00:00:5e:00:02:XX where XX = VRID (0-255)
    || (bytes[0..5] == [0x00, 0x00, 0x5e, 0x00, 0x02])
    // HSRP v1 (Cisco): 00:00:0c:07:ac:XX where XX = HSRP group ID (0-255)
    || (bytes[0..5] == [0x00, 0x00, 0x0c, 0x07, 0xac])
    // HSRP v2 (Cisco): 00:00:0c:9f:fX:XX where X:XX = HSRP group ID (0-4095)
    || (bytes[0..4] == [0x00, 0x00, 0x0c, 0x9f] && (bytes[4] & 0xf0) == 0xf0)
}

/// True when this row's MAC is a shared virtual-router MAC (VRRP/CARP/HSRP).
fn has_virtual_router_mac(ip: &IPAddress) -> bool {
    mac_of(&ip.base.mac_address)
        .map(|m| is_virtual_router_mac(&m))
        .unwrap_or(false)
}

/// True when this row pins identity to physical hardware: a MAC that is present and is
/// *not* a shared virtual-router MAC. A row with no MAC is not physical evidence.
fn has_real_mac(ip: &IPAddress) -> bool {
    mac_of(&ip.base.mac_address)
        .map(|m| !is_virtual_router_mac(&m))
        .unwrap_or(false)
}

/// Rows excluded from pass-1 (physical device) matching, on both the incoming and the
/// existing side.
///
/// - Loopbacks: every host has 127.0.0.1, so they would falsely match all hosts.
/// - Virtual router MACs: shared across physical routers, would falsely merge peers.
fn should_skip_for_matching(ip: &IPAddress) -> bool {
    ip.base.ip_address.is_loopback() || has_virtual_router_mac(ip)
}

/// Count IP addresses per MAC within a *single incoming payload*, to detect VLAN
/// sub-interfaces. Shared MAC (count > 1) means VLAN/bridge/bond sub-interfaces that must
/// not trigger MAC-based host matching. Unique MAC (count == 1) means a standalone IP
/// address safe for MAC matching (e.g., a Docker container whose IP changed via DHCP).
fn mac_counts_for_payload<'a>(
    payload: impl IntoIterator<Item = &'a IPAddress>,
) -> HashMap<MacAddress, usize> {
    payload
        .into_iter()
        .filter_map(|i| mac_of(&i.base.mac_address))
        .fold(HashMap::new(), |mut acc, mac| {
            *acc.entry(mac).or_insert(0) += 1;
            acc
        })
}

/// Decide which existing host — if any — an incoming payload's IP addresses identify.
///
/// `candidates` is `(host_id, all live IP rows for that host)` ordered **oldest-first**;
/// the caller owns that ordering. Pure so the HA topologies below are unit-testable.
///
/// Two passes, only ever one of which runs:
///
/// **Pass 1 — the payload has a physical identity** (at least one non-loopback row whose MAC
/// is absent or real). Matches those rows against existing rows via `ip_addresses_match`,
/// skipping virtual-router and loopback rows on both sides. This is the long-standing
/// behavior and is deliberately untouched: it is what keeps the two physical peers of an
/// HA pair apart, since a peer that reports the shared VIP can never match on that row.
///
/// **Pass 2 — the payload is *only* a floating virtual IP** (issue #661). An ARP-discovered
/// CARP/VRRP VIP is a single row carrying the shared virtual MAC, so pass 1 has nothing to
/// work with and the VIP was recreated as a new host on every scan. Here the address itself
/// is the identity: match on IP + subnet only, never MAC.
///
/// A candidate qualifies for pass 2 only if **no** non-loopback row of that host carries a
/// real MAC. The qualification is host-level, not row-level, because `create_with_children`
/// attaches every incoming row to the matched host — so a peer host can itself accumulate a
/// virtual-MAC VIP row, and a row-level test would let the VIP submission be absorbed into
/// that peer. Requiring the whole host to be MAC-less-or-virtual excludes every peer (they
/// always carry a real-MAC management row) while still matching a VIP host whose row was
/// first seen off-L2 with no MAC at all (MACs are immutable once set).
///
/// Pass 2 walks candidates newest-first: in steady state exactly one host qualifies, but
/// where duplicates already accumulated this makes the freshest one canonical.
///
/// **Known limitation.** Two devices sharing an IP+subnet where the daemon reports no real
/// MAC for either are indistinguishable and will match. That is the same address-collision
/// exposure pass 1 has always had.
///
/// **Pass 1b — the chassis id**, consulted only once addresses have failed. `hosts.chassis_id` is
/// the LLDP identity a device asserts about itself, stored in one canonical form precisely so a
/// neighbour's advertisement and a device's own `lldpLocChassisId` can be compared — yet dedup
/// never looked at it, so a device that changed address between scans, or one minted from a
/// neighbour's report into a range later scanned under a different subnet id, duplicated instead
/// of merging. Addresses still win: an IP on a subnet is a direct observation of the same logical
/// interface, while a chassis id identifies the *device* and is the right answer only when no
/// address agrees.
fn select_matching_host(
    incoming: &[IPAddress],
    incoming_chassis_id: Option<&str>,
    candidates: &[HostCandidate],
) -> Option<Uuid> {
    if candidates.is_empty() {
        return None;
    }
    // A payload with no addresses at all can still be identified by what the device calls itself.
    if incoming.is_empty() {
        return match_by_chassis_id(incoming_chassis_id, candidates);
    }

    // Pass 1: physical identity.
    let physical_incoming: Vec<&IPAddress> = incoming
        .iter()
        .filter(|i| !should_skip_for_matching(i))
        .collect();

    if !physical_incoming.is_empty() {
        let incoming_mac_counts = mac_counts_for_payload(physical_incoming.iter().copied());

        for HostCandidate {
            id: host_id,
            ip_addresses: host_ip_addresses,
            ..
        } in candidates
        {
            for incoming_ip in &physical_incoming {
                for existing_ip in host_ip_addresses {
                    if should_skip_for_matching(existing_ip) {
                        continue;
                    }
                    if ip_addresses_match(incoming_ip, existing_ip, &incoming_mac_counts) {
                        tracing::debug!(
                            incoming_ip = %incoming_ip.base.ip_address,
                            existing_ip = %existing_ip.base.ip_address,
                            existing_host_id = %host_id,
                            "Found matching host via IP-address comparison"
                        );
                        return Some(*host_id);
                    }
                }
            }
        }

        return match_by_chassis_id(incoming_chassis_id, candidates);
    }

    // Pass 2: floating virtual IP.
    let virtual_incoming: Vec<&IPAddress> = incoming
        .iter()
        .filter(|i| !i.base.ip_address.is_loopback() && has_virtual_router_mac(i))
        .collect();

    if virtual_incoming.is_empty() {
        return None;
    }

    for HostCandidate {
        id: host_id,
        ip_addresses: host_ip_addresses,
        ..
    } in candidates.iter().rev()
    {
        let host_is_physical = host_ip_addresses
            .iter()
            .any(|ip| !ip.base.ip_address.is_loopback() && has_real_mac(ip));
        if host_is_physical {
            continue;
        }

        for incoming_ip in &virtual_incoming {
            for existing_ip in host_ip_addresses {
                if existing_ip.base.ip_address.is_loopback() {
                    continue;
                }
                if ip_addresses_share_address(incoming_ip, existing_ip) {
                    tracing::debug!(
                        virtual_ip = %incoming_ip.base.ip_address,
                        existing_host_id = %host_id,
                        "Found matching host for virtual router IP via address comparison"
                    );
                    return Some(*host_id);
                }
            }
        }
    }

    match_by_chassis_id(incoming_chassis_id, candidates)
}

/// The one candidate asserting this chassis id, or `None`.
///
/// Resolves on a single match only, for the same reason every other chassis lookup does: two hosts
/// carrying one identifier are a duplicate this cannot choose between, and picking either would
/// merge a scan into an arbitrary one of them.
fn match_by_chassis_id(chassis_id: Option<&str>, candidates: &[HostCandidate]) -> Option<Uuid> {
    let chassis_id = chassis_id.map(str::trim).filter(|id| !id.is_empty())?;
    let mut matched = candidates
        .iter()
        .filter(|c| c.chassis_id.as_deref() == Some(chassis_id));
    let first = matched.next()?;
    if matched.next().is_some() {
        tracing::debug!(
            chassis_id = %chassis_id,
            "Chassis id names more than one host; leaving dedup to the address tiers"
        );
        return None;
    }
    tracing::debug!(
        chassis_id = %chassis_id,
        existing_host_id = %first.id,
        "Found matching host via chassis id"
    );
    Some(first.id)
}

/// A host dedup may match against, and the identities it can be matched on.
pub(crate) struct HostCandidate {
    pub id: Uuid,
    /// The device's own LLDP chassis identity, where a scan or a neighbour recorded one.
    pub chassis_id: Option<String>,
    /// Its **full, unfiltered** live IP rows — loopback and virtual-router rows included, because
    /// the passes above decide for themselves which to skip.
    pub ip_addresses: Vec<IPAddress>,
}

/// Same IP on the same subnet — the same logical interface.
fn ip_addresses_share_address(a: &IPAddress, b: &IPAddress) -> bool {
    a.base.ip_address == b.base.ip_address && a.base.subnet_id == b.base.subnet_id
}

/// Compare two ip_addresses for host dedup matching.
///
/// Three match branches, checked in order:
/// 1. **IP+subnet** (primary): same IP on the same subnet = same logical interface
/// 2. **ID** (secondary): same non-nil database UUID = known same record
/// 3. **MAC** (tertiary, conditional): same MAC address, but only when the MAC is unique
///    among incoming ip_addresses (count == 1). Shared MACs (count > 1) indicate VLAN
///    sub-interfaces, bridge members, or bond members — distinct ip_addresses that must
///    not be collapsed. Unique MACs indicate a standalone interface (e.g., a Docker
///    container whose IP changed via DHCP) where MAC is a valid identity anchor.
fn ip_addresses_match(
    incoming: &IPAddress,
    existing: &IPAddress,
    incoming_mac_counts: &HashMap<MacAddress, usize>,
) -> bool {
    // Primary: same IP on same subnet
    ip_addresses_share_address(incoming, existing)
    // Secondary: same non-nil ID
    || (incoming.id == existing.id
        && incoming.id != Uuid::nil()
        && existing.id != Uuid::nil())
    // Tertiary: MAC match, gated on incoming MAC uniqueness.
    //
    // On the *value*, never on the evidence. `mac_address` carries its provenance since §7, and
    // `Attributed` compares both halves — so a bare `==` on the field silently demanded that the
    // two scans have learned the address the same way before it would call them the same NIC. A
    // MAC first read from a router's forwarding table and later confirmed by our own ARP reply is
    // one NIC, and the branch is asking whether it is the same hardware, not the same paperwork.
    || (match (
        mac_of(&incoming.base.mac_address),
        mac_of(&existing.base.mac_address),
    ) {
        (Some(incoming_mac), Some(existing_mac)) => {
            incoming_mac == existing_mac
                && incoming_mac_counts.get(&incoming_mac).copied().unwrap_or(0) == 1
        }
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;
    use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};

    fn make_interface(ip: IpAddr, subnet_id: Uuid, mac: Option<MacAddress>) -> IPAddress {
        IPAddress {
            id: Uuid::new_v4(),
            base: IPAddressBase {
                ip_address: ip,
                subnet_id,
                mac_address: mac
                    .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::ArpReply)),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    // --- is_virtual_router_mac tests ---

    #[test]
    fn vrrp_mac_detected() {
        // VRRP (RFC 5798): 00:00:5e:00:01:XX
        let mac = MacAddress::new([0x00, 0x00, 0x5e, 0x00, 0x01, 0x01]);
        assert!(is_virtual_router_mac(&mac), "VRRP MAC should be detected");
    }

    #[test]
    fn hsrp_v1_mac_detected() {
        // HSRP v1: 00:00:0c:07:ac:XX
        let mac = MacAddress::new([0x00, 0x00, 0x0c, 0x07, 0xac, 0x0a]);
        assert!(
            is_virtual_router_mac(&mac),
            "HSRP v1 MAC should be detected"
        );
    }

    #[test]
    fn hsrp_v2_mac_detected() {
        // HSRP v2: 00:00:0c:9f:fX:XX
        let mac = MacAddress::new([0x00, 0x00, 0x0c, 0x9f, 0xf0, 0x0a]);
        assert!(
            is_virtual_router_mac(&mac),
            "HSRP v2 MAC should be detected"
        );
    }

    #[test]
    fn normal_mac_not_virtual_router() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        assert!(
            !is_virtual_router_mac(&mac),
            "Regular MAC should not be detected as virtual router"
        );
    }

    // --- ip_addresses_match tests ---

    #[test]
    fn match_by_ip_subnet() {
        let subnet = Uuid::new_v4();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let a = make_interface(ip, subnet, None);
        let b = make_interface(ip, subnet, None);
        let counts = HashMap::new();
        assert!(ip_addresses_match(&a, &b, &counts));
    }

    #[test]
    fn no_match_different_ip_subnet() {
        let a = make_interface("10.0.0.1".parse().unwrap(), Uuid::new_v4(), None);
        let b = make_interface("20.0.0.1".parse().unwrap(), Uuid::new_v4(), None);
        let counts = HashMap::new();
        assert!(!ip_addresses_match(&a, &b, &counts));
    }

    #[test]
    fn mac_match_when_unique_in_batch() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let a = make_interface("10.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let b = make_interface("20.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        // MAC appears only once in the incoming batch — standalone ip_address, safe to match
        let counts = HashMap::from([(mac, 1)]);
        assert!(
            ip_addresses_match(&a, &b, &counts),
            "Unique MAC in batch should allow MAC matching (Docker/DHCP case)"
        );
    }

    #[test]
    fn mac_no_match_when_shared_in_batch() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let a = make_interface("10.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let b = make_interface("20.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        // MAC appears 3 times in the incoming batch — VLAN sub-interfaces, must not match
        let counts = HashMap::from([(mac, 3)]);
        assert!(
            !ip_addresses_match(&a, &b, &counts),
            "Shared MAC in batch (VLANs) must not match"
        );
    }

    // --- #600 characterization: multi-IP host on a single MAC ---
    //
    // These document the *actual* dedup decision for a host that responds on multiple
    // IP addresses behind one MAC (multi-homed server, IP aliases). They are evidence,
    // not a fix: they pin down what the current heuristic does so we can tell whether
    // the reported "tied to lowest IP" behavior is a real defect or expected dedup.

    #[test]
    fn mac_match_same_subnet_when_unique_in_batch() {
        // Multi-homed / IP-alias case: two IPs on the SAME subnet sharing one MAC.
        // Primary (ip+subnet) branch fails (different IP); MAC branch (count==1) must
        // carry the match so both IPs collapse onto one host.
        let subnet = Uuid::new_v4();
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let a = make_interface("192.168.1.10".parse().unwrap(), subnet, Some(mac));
        let b = make_interface("192.168.1.11".parse().unwrap(), subnet, Some(mac));
        let counts = HashMap::from([(mac, 1)]);
        assert!(
            ip_addresses_match(&a, &b, &counts),
            "Same MAC + same subnet, unique in batch, should match (IP-alias multi-homing)"
        );
    }

    #[test]
    fn multi_homed_host_separate_payloads_merge() {
        // The real discovery flow: each scanned IP arrives as its own single-IP payload,
        // so `incoming_mac_counts` for that payload is always {MAC: 1}. Therefore the
        // VLAN guard never trips across payloads, and the second IP merges into the host
        // created by the first — one Host, multiple IPAddress children. This is the
        // model-(a) outcome, contradicting "a separate host per IP".
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02]);

        // Payload 1: first IP (different subnet to exercise the MAC branch, not ip+subnet)
        let first = make_interface("192.168.1.50".parse().unwrap(), Uuid::new_v4(), Some(mac));
        // Payload 2: second IP, scanned independently
        let second = make_interface("10.0.0.50".parse().unwrap(), Uuid::new_v4(), Some(mac));

        // Counts are per-payload; each single-IP payload yields {mac: 1}.
        let counts_for_second = mac_counts_for_payload(std::slice::from_ref(&second));
        assert_eq!(counts_for_second.get(&mac), Some(&1));

        assert!(
            ip_addresses_match(&second, &first, &counts_for_second),
            "Independently scanned same-MAC IPs must merge into one host (model a)"
        );
    }

    #[test]
    fn multi_ip_single_payload_does_not_mac_merge() {
        // Contrast: when one payload carries BOTH same-MAC IPs (count==2), the MAC branch
        // is intentionally disabled (treated as VLAN/bridge sub-interfaces). They are not
        // merged *via MAC*; they remain distinct IPAddress rows under whatever host owns
        // them. Guards the existing behavior so a future fix can't regress it silently.
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x03]);
        let a = make_interface("172.16.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let b = make_interface("172.16.5.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let counts = mac_counts_for_payload(&[a.clone(), b.clone()]);
        assert_eq!(counts.get(&mac), Some(&2));
        assert!(
            !ip_addresses_match(&a, &b, &counts),
            "Two same-MAC IPs in one payload must not MAC-merge (VLAN sub-interface guard)"
        );
    }

    // --- #661: CARP/VRRP virtual IPs in an OPNsense HA pair ---
    //
    // Topology under test: two physical firewalls, each with its own management IP on a
    // real NIC MAC, plus a floating CARP VIP that the scanner also sees standalone via ARP
    // (a single row carrying the shared virtual MAC). Both peers may additionally report
    // the VIP themselves. The VIP must re-match its own host across scans; the two peers
    // must never collapse into one.

    /// The shared virtual MAC an HA pair advertises for VRID/vhid 10.
    fn vrrp_mac() -> MacAddress {
        MacAddress::new([0x00, 0x00, 0x5e, 0x00, 0x01, 0x0a])
    }

    fn candidate(host_id: Uuid, ip_addresses: Vec<IPAddress>) -> HostCandidate {
        HostCandidate {
            id: host_id,
            chassis_id: None,
            ip_addresses,
        }
    }

    /// A candidate that asserts an LLDP chassis identity as well as its addresses.
    fn candidate_with_chassis(
        host_id: Uuid,
        chassis_id: &str,
        ip_addresses: Vec<IPAddress>,
    ) -> HostCandidate {
        HostCandidate {
            id: host_id,
            chassis_id: Some(chassis_id.to_string()),
            ip_addresses,
        }
    }

    /// A device that changed address between scans, or one minted from a neighbour's report into
    /// a range later scanned under a different subnet id. Every address tier fails — the IP is new
    /// and the subnet id differs — and without the chassis tier the scan creates a second host for
    /// a device this network already holds.
    #[test]
    fn a_device_that_changed_address_still_matches_on_its_chassis_id() {
        let known = Uuid::new_v4();
        let candidates = vec![candidate_with_chassis(
            known,
            "00:ad:24:89:cc:f0",
            vec![make_interface(
                "10.20.30.11".parse().unwrap(),
                Uuid::new_v4(),
                None,
            )],
        )];
        // Different address, different subnet — nothing an address tier can join.
        let incoming = vec![make_interface(
            "192.168.7.11".parse().unwrap(),
            Uuid::new_v4(),
            None,
        )];

        assert_eq!(
            select_matching_host(&incoming, Some("00:ad:24:89:cc:f0"), &candidates),
            Some(known)
        );
    }

    /// Addresses outrank the chassis id: an IP on a subnet is a direct observation of the same
    /// logical interface, while a chassis id only identifies the device. Where they disagree the
    /// address has to win, or a payload would be absorbed by whichever host happens to assert a
    /// matching identity.
    #[test]
    fn an_address_match_outranks_a_chassis_id_match() {
        let by_address = Uuid::new_v4();
        let subnet = Uuid::new_v4();
        let ip = "10.20.30.11".parse().unwrap();

        let candidates = vec![
            candidate_with_chassis(
                Uuid::new_v4(),
                "00:ad:24:89:cc:f0",
                vec![make_interface("10.20.30.99".parse().unwrap(), subnet, None)],
            ),
            candidate(by_address, vec![make_interface(ip, subnet, None)]),
        ];
        let incoming = vec![make_interface(ip, subnet, None)];

        assert_eq!(
            select_matching_host(&incoming, Some("00:ad:24:89:cc:f0"), &candidates),
            Some(by_address)
        );
    }

    /// Two hosts carrying one chassis id are a duplicate this cannot choose between, and merging a
    /// scan into an arbitrary one of them is worse than creating a row somebody can consolidate.
    /// The same single-match rule every other chassis lookup applies.
    #[test]
    fn a_chassis_id_held_by_two_hosts_matches_neither() {
        let candidates = vec![
            candidate_with_chassis(Uuid::new_v4(), "00:ad:24:89:cc:f0", vec![]),
            candidate_with_chassis(Uuid::new_v4(), "00:ad:24:89:cc:f0", vec![]),
        ];
        let incoming = vec![make_interface(
            "192.168.7.11".parse().unwrap(),
            Uuid::new_v4(),
            None,
        )];

        assert_eq!(
            select_matching_host(&incoming, Some("00:ad:24:89:cc:f0"), &candidates),
            None
        );
    }

    #[test]
    fn vip_only_payload_rematches_existing_vip_host() {
        // #661: the standalone ARP view of the VIP is nothing but a virtual-MAC row, so it
        // had no matchable identity at all and was recreated on every scan.
        let subnet = Uuid::new_v4();
        let vip: IpAddr = "192.168.1.1".parse().unwrap();
        let vip_host = Uuid::new_v4();

        let candidates = vec![candidate(
            vip_host,
            vec![make_interface(vip, subnet, Some(vrrp_mac()))],
        )];
        let incoming = vec![make_interface(vip, subnet, Some(vrrp_mac()))];

        assert_eq!(
            select_matching_host(&incoming, None, &candidates),
            Some(vip_host),
            "A rediscovered CARP/VRRP VIP must update its existing host, not create a new one"
        );
    }

    #[test]
    fn ha_peers_reporting_the_shared_vip_stay_separate() {
        // Peer B's first-ever payload: its own management IP on its own NIC, plus the VIP
        // it advertises. Peer A already exists and reports the same VIP. Matching on the
        // shared virtual MAC or on the shared VIP address would merge the two firewalls.
        let subnet = Uuid::new_v4();
        let vip: IpAddr = "192.168.1.1".parse().unwrap();
        let peer_a = Uuid::new_v4();

        let candidates = vec![candidate(
            peer_a,
            vec![
                make_interface(
                    "192.168.1.2".parse().unwrap(),
                    subnet,
                    Some(MacAddress::new([0xAA, 0xBB, 0xCC, 0x00, 0x00, 0x01])),
                ),
                make_interface(vip, subnet, Some(vrrp_mac())),
            ],
        )];
        let incoming = vec![
            make_interface(
                "192.168.1.3".parse().unwrap(),
                subnet,
                Some(MacAddress::new([0xAA, 0xBB, 0xCC, 0x00, 0x00, 0x02])),
            ),
            make_interface(vip, subnet, Some(vrrp_mac())),
        ];

        assert_eq!(
            select_matching_host(&incoming, None, &candidates),
            None,
            "Two physical HA peers advertising the same VIP must remain separate hosts"
        );
    }

    #[test]
    fn vip_payload_does_not_absorb_a_physical_peer() {
        // The VIP arrives before its own host exists and both peers are already known,
        // each carrying a VIP row. Matching either would hand the peer's identity and
        // services to the floating address.
        let subnet = Uuid::new_v4();
        let vip: IpAddr = "192.168.1.1".parse().unwrap();

        let peer = |mgmt: &str, mac: u8| {
            candidate(
                Uuid::new_v4(),
                vec![
                    make_interface(
                        mgmt.parse().unwrap(),
                        subnet,
                        Some(MacAddress::new([0xAA, 0xBB, 0xCC, 0x00, 0x00, mac])),
                    ),
                    make_interface(vip, subnet, Some(vrrp_mac())),
                ],
            )
        };
        let candidates = vec![peer("192.168.1.2", 0x01), peer("192.168.1.3", 0x02)];
        let incoming = vec![make_interface(vip, subnet, Some(vrrp_mac()))];

        assert_eq!(
            select_matching_host(&incoming, None, &candidates),
            None,
            "A VIP payload must not match a host that has real-MAC hardware identity"
        );
    }

    #[test]
    fn vip_prefers_its_own_host_over_a_peer_advertising_it() {
        // Both a peer and the standalone VIP host hold a row at the VIP address. The peer
        // is older, so an order-driven match would pick it; hardware identity must win out.
        let subnet = Uuid::new_v4();
        let vip: IpAddr = "192.168.1.1".parse().unwrap();
        let peer_a = Uuid::new_v4();
        let vip_host = Uuid::new_v4();

        let candidates = vec![
            candidate(
                peer_a,
                vec![
                    make_interface(
                        "192.168.1.2".parse().unwrap(),
                        subnet,
                        Some(MacAddress::new([0xAA, 0xBB, 0xCC, 0x00, 0x00, 0x01])),
                    ),
                    make_interface(vip, subnet, Some(vrrp_mac())),
                ],
            ),
            candidate(
                vip_host,
                vec![make_interface(vip, subnet, Some(vrrp_mac()))],
            ),
        ];
        let incoming = vec![make_interface(vip, subnet, Some(vrrp_mac()))];

        assert_eq!(
            select_matching_host(&incoming, None, &candidates),
            Some(vip_host),
            "The VIP must resolve to its own host, not to a peer that advertises it"
        );
    }

    #[test]
    fn vip_host_first_seen_without_a_mac_still_rematches() {
        // First sighting was off-L2 (SNMP/port scan, no ARP), so the stored row has no MAC
        // at all — and MACs are immutable once set, so it never gains one. A later on-L2
        // sighting carrying the virtual MAC must still land on that host.
        let subnet = Uuid::new_v4();
        let vip: IpAddr = "192.168.1.1".parse().unwrap();
        let vip_host = Uuid::new_v4();

        let candidates = vec![candidate(vip_host, vec![make_interface(vip, subnet, None)])];
        let incoming = vec![make_interface(vip, subnet, Some(vrrp_mac()))];

        assert_eq!(
            select_matching_host(&incoming, None, &candidates),
            Some(vip_host),
            "A VIP host first recorded without a MAC must still be re-found"
        );
    }

    #[test]
    fn two_vips_in_one_group_stay_distinct() {
        // Several VIPs can share one VRID, hence one virtual MAC. Only the address
        // distinguishes them, so the shared MAC must never carry a match.
        let subnet = Uuid::new_v4();
        let first_vip = Uuid::new_v4();

        let candidates = vec![candidate(
            first_vip,
            vec![make_interface(
                "192.168.1.1".parse().unwrap(),
                subnet,
                Some(vrrp_mac()),
            )],
        )];
        let incoming = vec![make_interface(
            "192.168.1.4".parse().unwrap(),
            subnet,
            Some(vrrp_mac()),
        )];

        assert_eq!(
            select_matching_host(&incoming, None, &candidates),
            None,
            "Distinct VIPs sharing a VRID must not collapse via their shared MAC"
        );
    }

    #[test]
    fn ipv6_vrrp_vip_rematches_existing_host() {
        // RFC 5798 §7.3 gives IPv6 VRRP its own MAC range (00:00:5e:00:02:XX); it gets the
        // same treatment as the IPv4 range rather than being read as real hardware.
        let subnet = Uuid::new_v4();
        let vip: IpAddr = "2001:db8::1".parse().unwrap();
        let ipv6_mac = MacAddress::new([0x00, 0x00, 0x5e, 0x00, 0x02, 0x0a]);
        let vip_host = Uuid::new_v4();

        let candidates = vec![candidate(
            vip_host,
            vec![make_interface(vip, subnet, Some(ipv6_mac))],
        )];
        let incoming = vec![make_interface(vip, subnet, Some(ipv6_mac))];

        assert_eq!(
            select_matching_host(&incoming, None, &candidates),
            Some(vip_host),
            "An IPv6 VRRP VIP must re-match like its IPv4 counterpart"
        );
    }

    #[test]
    fn loopback_only_payload_matches_nothing() {
        // Every host has 127.0.0.1; a payload with nothing else must not fall through into
        // the virtual-IP pass and match on it.
        let subnet = Uuid::new_v4();
        let loopback: IpAddr = "127.0.0.1".parse().unwrap();

        let candidates = vec![candidate(
            Uuid::new_v4(),
            vec![make_interface(loopback, subnet, None)],
        )];
        let incoming = vec![make_interface(loopback, subnet, None)];

        assert_eq!(select_matching_host(&incoming, None, &candidates), None);
    }

    #[test]
    fn newest_duplicate_vip_host_wins() {
        // Hosts arrive oldest-first. Where #661 already minted duplicates, the freshest one
        // carries the most recent services and last_seen_at, so it becomes canonical.
        let subnet = Uuid::new_v4();
        let vip: IpAddr = "192.168.1.1".parse().unwrap();
        let oldest = Uuid::new_v4();
        let newest = Uuid::new_v4();

        let candidates = vec![
            candidate(oldest, vec![make_interface(vip, subnet, Some(vrrp_mac()))]),
            candidate(newest, vec![make_interface(vip, subnet, Some(vrrp_mac()))]),
        ];
        let incoming = vec![make_interface(vip, subnet, Some(vrrp_mac()))];

        assert_eq!(
            select_matching_host(&incoming, None, &candidates),
            Some(newest),
            "Pre-existing VIP duplicates must collapse onto the most recently created host"
        );
    }

    #[test]
    fn physical_host_rematches_by_unique_mac_after_ip_change() {
        // Pass-1 parity: an ordinary host whose IP moved (DHCP) is still found by its MAC,
        // and a virtual-MAC row sitting on the candidate host doesn't disturb that.
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x11]);
        let host = Uuid::new_v4();

        let candidates = vec![candidate(
            host,
            vec![make_interface(
                "10.0.0.5".parse().unwrap(),
                Uuid::new_v4(),
                Some(mac),
            )],
        )];
        let incoming = vec![make_interface(
            "10.0.0.9".parse().unwrap(),
            Uuid::new_v4(),
            Some(mac),
        )];

        assert_eq!(
            select_matching_host(&incoming, None, &candidates),
            Some(host)
        );
    }
    /// The same rematch, when the two scans learned the MAC from different sources.
    ///
    /// `mac_address` is `Option<Attributed<MacEvidenceValue>>` and `Attributed` compares value
    /// *and* source, so a bare `==` on the field makes the tertiary branch demand that a router's
    /// ARP cache and our own ARP reply agree about provenance before it will call them the same
    /// NIC. They are the same NIC. Every other test builds both sides with `ArpReply`, which is
    /// why none of them can see this.
    #[test]
    fn a_host_rematches_by_mac_even_when_the_two_scans_learned_it_differently() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x22]);
        let host = Uuid::new_v4();

        // First scan: a router's ipNetToMediaTable told us about this MAC.
        let mut existing = make_interface("10.0.0.5".parse().unwrap(), Uuid::new_v4(), Some(mac));
        existing.base.mac_address = Some(MacEvidence::new(
            MacEvidenceValue(mac),
            AttributeSource::ForwardingTable,
        ));

        // Second scan, after DHCP moved it: we ARPed for it ourselves.
        let incoming = vec![make_interface(
            "10.0.0.9".parse().unwrap(),
            Uuid::new_v4(),
            Some(mac),
        )];

        assert_eq!(
            select_matching_host(&incoming, None, &[candidate(host, vec![existing])]),
            Some(host),
            "a NIC is the same NIC however each scan came to hear about it"
        );
    }
}
