use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::bindings::service::BindingService;
use crate::server::daemons::r#impl::api::{DiscoveryUpdatePayload, ScannedEntityIds};
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::discovery::service::DiscoveryService;
use crate::server::services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt};
use crate::server::shared::storage::snapshot::DiscoveryTracked;
use crate::server::{
    digest::payload::{
        AffectedHostCard, DigestRecipient, DiscoveryDigestFlags, DiscoveryDigestOperation,
        DiscoveryDigestPayload, DiscoveryDigestScope, EntityFreshness, HostSummary,
        InterfaceSummary, IpAddressSummary, PortSummary, ServiceSummary, SubnetSummary,
        VlanSummary,
    },
    hosts::{r#impl::base::Host, service::HostService},
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    ip_addresses::{r#impl::base::IPAddress, service::IPAddressService},
    networks::{r#impl::Network, service::NetworkService},
    ports::{r#impl::base::Port, service::PortService},
    services::{r#impl::base::Service as NetworkServiceEntity, service::ServiceService},
    shared::{
        events::{bus::EventBus, traits::Event},
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, traits::Storage},
    },
    subnets::{r#impl::base::Subnet, service::SubnetService},
    users::service::UserService,
    vlans::{r#impl::base::Vlan, service::VlanService},
};

/// Read-only aggregator that answers "what changed in this network during
/// session [T_start, T_end]" by composing SCD2 timestamp filters across the
/// per-entity-tracked tables. Mirrors `TopologyService`'s shape: holds Arcs
/// to the entity services it queries, no storage-layer deps.
pub struct DiscoveryDigestService {
    pub host_service: Arc<HostService>,
    pub service_service: Arc<ServiceService>,
    pub port_service: Arc<PortService>,
    pub ip_address_service: Arc<IPAddressService>,
    pub interface_service: Arc<InterfaceService>,
    pub binding_service: Arc<BindingService>,
    pub subnet_service: Arc<SubnetService>,
    pub vlan_service: Arc<VlanService>,
    pub user_service: Arc<UserService>,
    pub network_service: Arc<NetworkService>,
    pub discovery_service: Arc<DiscoveryService>,
    pub event_bus: Arc<EventBus>,
}

impl DiscoveryDigestService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host_service: Arc<HostService>,
        service_service: Arc<ServiceService>,
        port_service: Arc<PortService>,
        ip_address_service: Arc<IPAddressService>,
        interface_service: Arc<InterfaceService>,
        binding_service: Arc<BindingService>,
        subnet_service: Arc<SubnetService>,
        vlan_service: Arc<VlanService>,
        user_service: Arc<UserService>,
        network_service: Arc<NetworkService>,
        discovery_service: Arc<DiscoveryService>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            host_service,
            service_service,
            port_service,
            ip_address_service,
            interface_service,
            binding_service,
            subnet_service,
            vlan_service,
            user_service,
            network_service,
            discovery_service,
            event_bus,
        }
    }

    /// Compute the digest for `payload` and publish a
    /// `DiscoveryDigestOperation::Computed` event. Skips publishing entirely
    /// when timestamps are missing — those would yield meaningless filter
    /// windows.
    pub async fn compute_and_publish(
        &self,
        payload: &DiscoveryUpdatePayload,
        scanned: &ScannedEntityIds,
    ) -> Result<()> {
        let (Some(t_start), Some(t_end)) = (payload.started_at, payload.finished_at) else {
            tracing::warn!(
                session_id = %payload.session_id,
                "Discovery session terminal payload missing started_at or finished_at; skipping digest",
            );
            return Ok(());
        };

        let network = match self.network_service.get_by_id(&payload.network_id).await? {
            Some(n) => n,
            None => {
                tracing::warn!(
                    network_id = %payload.network_id,
                    "Network missing for digest computation; skipping",
                );
                return Ok(());
            }
        };

        let digest = self
            .compute(payload, scanned, t_start, t_end, &network)
            .await?;

        let scope = DiscoveryDigestScope {
            organization_id: network.base.organization_id,
            network_id: payload.network_id,
        };
        let event = Event::new(
            scope,
            DiscoveryDigestOperation::Computed {
                payload: Box::new(digest),
            },
            AuthenticatedEntity::System,
        )
        .with_flags(DiscoveryDigestFlags::default());
        self.event_bus.publish(event).await?;
        Ok(())
    }

    async fn compute(
        &self,
        payload: &DiscoveryUpdatePayload,
        scanned: &ScannedEntityIds,
        t_start: DateTime<Utc>,
        t_end: DateTime<Utc>,
        network: &Network,
    ) -> Result<DiscoveryDigestPayload> {
        let network_id = payload.network_id;
        let network_name = network.base.name.as_str();
        let organization_id = network.base.organization_id;

        // Subnets scanned: prefer the discovery config's explicit subnet
        // list when set (the user targeted a specific subset). Fall back
        // to the daemon's reported set. Either way, drop loopback subnets
        // (127.0.0.0/8 + ::1) — they're not user-meaningful.
        let targeted: Option<&[Uuid]> = match &payload.discovery_type {
            DiscoveryType::Network {
                subnet_ids: Some(ids),
                ..
            }
            | DiscoveryType::Unified {
                subnet_ids: Some(ids),
                ..
            } if !ids.is_empty() => Some(ids.as_slice()),
            _ => None,
        };
        let subnet_ids: &[Uuid] = targeted.unwrap_or(scanned.subnet_ids.as_slice());

        let subnets_scanned: Vec<SubnetSummary> = if subnet_ids.is_empty() {
            Vec::new()
        } else {
            self.subnet_service
                .get_all(StorableFilter::<Subnet>::new_from_entity_ids(subnet_ids))
                .await?
                .iter()
                .filter(|s| !s.base.cidr.first_address().is_loopback())
                .map(subnet_summary)
                .collect()
        };

        // Staleness is time-based and anchored on this network's configured
        // window, identical to what the inventory and topology render. The
        // previous session's finish gives the "was it stale then?" anchor used
        // to spot the transition.
        let prev_finished_at = self
            .discovery_service
            .previous_historical_finished_at(network_id, t_end)
            .await?;
        let window = DigestWindow {
            t_start,
            t_end,
            cutoff: network.stale_cutoff(t_end),
            prev_cutoff: prev_finished_at.map(|t| network.stale_cutoff(t)),
        };

        // What this session could actually see. Entities outside it are dropped
        // from the digest entirely — their absence carries no information.
        let coverage = ScanCoverage::for_session(&payload.discovery_type, scanned);

        // One query for all live hosts on the network — the generic helper
        // buckets them by status. Per-entity-type queries for children are
        // batched the same way inside fetch_current_children.
        let all_hosts: Vec<Host> = self
            .host_service
            .get_all(StorableFilter::<Host>::new_from_network_ids(&[network_id]).live())
            .await?;
        let scanned_host_ids: HashSet<Uuid> = scanned.host_ids.iter().copied().collect();

        // Live IPs for the whole network, once: they place each host in its
        // subnets (for the coverage gate) and are re-filtered for the affected
        // hosts' child rows below, so this replaces the old per-affected-host
        // IP query rather than adding to it.
        let all_ips: Vec<IPAddress> = self
            .ip_address_service
            .get_all(StorableFilter::<IPAddress>::new_from_network_ids(&[network_id]).live())
            .await?;
        let mut host_subnets: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
        for ip in &all_ips {
            host_subnets
                .entry(ip.base.host_id)
                .or_default()
                .insert(ip.base.subnet_id);
        }

        // First pass on hosts: bucket each one by status + fresh.
        struct HostBucket {
            host: Host,
            status: EntityFreshness,
            is_fresh: bool,
        }
        let host_buckets: Vec<HostBucket> = all_hosts
            .into_iter()
            .filter(|h| {
                coverage.covers_host(
                    h.id,
                    host_subnets.get(&h.id),
                    scanned_host_ids.contains(&h.id),
                )
            })
            .map(|h| {
                let (status, is_fresh) = compute_digest_status(&h, &scanned_host_ids, &window);
                HostBucket {
                    host: h,
                    status,
                    is_fresh,
                }
            })
            .collect();

        // Affected = added + every host that's seen-this-scan (their
        // children might have fresh deltas) + every host that just turned
        // stale (we want to render their last-known children too).
        let affected: Vec<(Uuid, ChildPolicy)> = host_buckets
            .iter()
            .filter(|b| match b.status {
                EntityFreshness::New | EntityFreshness::Current => true,
                EntityFreshness::Stale => b.is_fresh,
            })
            .map(|b| {
                // Only a host we actually reached can tell us anything about
                // which of its children are still there.
                let policy = if scanned_host_ids.contains(&b.host.id) {
                    ChildPolicy::Classify
                } else {
                    ChildPolicy::Inherit(b.status, b.is_fresh)
                };
                (b.host.id, policy)
            })
            .collect();

        let current_children = self
            .fetch_current_children(&affected, &all_ips, scanned, &window)
            .await?;

        let mut hosts_added: Vec<AffectedHostCard> = Vec::new();
        let mut hosts_stale: Vec<AffectedHostCard> = Vec::new();
        let mut hosts_changed: Vec<AffectedHostCard> = Vec::new();

        for HostBucket {
            host,
            status,
            is_fresh,
        } in host_buckets
        {
            match status {
                EntityFreshness::New => {
                    hosts_added.push(build_card(&host, status, &current_children));
                }
                EntityFreshness::Stale => {
                    if is_fresh {
                        hosts_stale.push(build_card(&host, status, &current_children));
                    }
                }
                EntityFreshness::Current => {
                    let card = build_card(&host, status, &current_children);
                    if card_has_fresh_children(&card) {
                        hosts_changed.push(card);
                    }
                }
            }
        }
        hosts_changed.sort_by(|a, b| a.host.label.cmp(&b.host.label));

        // VLANs added / removed mirror the host added/vanished logic but on
        // the network scope.
        let vlans_added_records: Vec<Vlan> = self
            .vlan_service
            .get_all(
                StorableFilter::<Vlan>::new_from_network_ids(&[network_id])
                    .live()
                    .created_between(t_start, t_end),
            )
            .await?;
        let vlans_added: Vec<VlanSummary> = vlans_added_records.iter().map(vlan_summary).collect();
        let scanned_vlan_ids: HashSet<Uuid> = scanned.vlan_ids.iter().copied().collect();

        // VLANs are network-scoped, so only a run that actually swept subnets
        // can say anything about them — a Docker or self-report run observes no
        // VLANs and must not conclude they all went stale.
        let vlans_stale: Vec<VlanSummary> = if coverage.swept_subnets() {
            let live_vlans: Vec<Vlan> = self
                .vlan_service
                .get_all(
                    StorableFilter::<Vlan>::new_from_network_ids(&[network_id])
                        .live()
                        .created_before(t_start),
                )
                .await?;
            live_vlans
                .iter()
                .filter(|v| {
                    let (status, is_fresh) = compute_digest_status(*v, &scanned_vlan_ids, &window);
                    status == EntityFreshness::Stale && is_fresh
                })
                .map(vlan_summary)
                .collect()
        } else {
            Vec::new()
        };

        let recipients = self.resolve_recipients(network_id, organization_id).await?;

        Ok(DiscoveryDigestPayload {
            session_id: payload.session_id,
            network_id,
            network_name: network_name.to_string(),
            started_at: t_start,
            finished_at: t_end,
            stale_after_hours: network.stale_after().num_hours(),
            subnets_scanned,
            hosts_added,
            hosts_stale,
            hosts_changed,
            vlans_added,
            vlans_stale,
            recipients,
        })
    }

    /// Fetch live children for the affected-host set and bucket them by
    /// host_id. Each child summary carries its `EntityDigestStatus` and
    /// an `is_fresh` flag (computed via the generic
    /// [`compute_digest_status`] helper). Filters Unclaimed Open Ports and
    /// loopback IPs.
    async fn fetch_current_children(
        &self,
        affected: &[(Uuid, ChildPolicy)],
        all_ips: &[IPAddress],
        scanned: &ScannedEntityIds,
        window: &DigestWindow,
    ) -> Result<HashMap<Uuid, HostChildren>> {
        if affected.is_empty() {
            return Ok(HashMap::new());
        }
        let host_ids: Vec<Uuid> = affected.iter().map(|(id, _)| *id).collect();
        let policies: HashMap<Uuid, &ChildPolicy> =
            affected.iter().map(|(id, p)| (*id, p)).collect();
        let host_ids = host_ids.as_slice();

        // Resolve a child's status under its host's policy: classify it on its
        // own merits only when the host was actually reached this session.
        let resolve = |host_id: Uuid, own: (EntityFreshness, bool)| {
            policies
                .get(&host_id)
                .map_or(own, |policy| policy.resolve(own))
        };

        let services: Vec<NetworkServiceEntity> = self
            .service_service
            .storage()
            .get_all(
                StorableFilter::<NetworkServiceEntity>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;
        // Reuse the network-wide live IP set already loaded for the coverage
        // gate rather than re-querying by host.
        let ips: Vec<IPAddress> = all_ips
            .iter()
            .filter(|ip| policies.contains_key(&ip.base.host_id))
            .cloned()
            .collect();
        let interfaces: Vec<Interface> = self
            .interface_service
            .storage()
            .get_all(
                StorableFilter::<Interface>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;
        let ports: Vec<Port> = self
            .port_service
            .storage()
            .get_all(
                StorableFilter::<Port>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;

        let scanned_service_ids: HashSet<Uuid> = scanned.service_ids.iter().copied().collect();
        let scanned_port_ids: HashSet<Uuid> = scanned.port_ids.iter().copied().collect();
        let scanned_ip_ids: HashSet<Uuid> = scanned.ip_address_ids.iter().copied().collect();
        let scanned_interface_ids: HashSet<Uuid> = scanned.interface_ids.iter().copied().collect();

        let mut out: HashMap<Uuid, HostChildren> = HashMap::new();
        for id in host_ids {
            out.entry(*id).or_default();
        }
        for s in &services {
            // Skip the synthetic "Unclaimed Open Ports" service — it's
            // useful in the UI's open-ports panel but noise in the digest.
            if s.base.service_definition.is_open_ports() {
                continue;
            }
            let own = compute_digest_status(s, &scanned_service_ids, window);
            let (status, is_fresh) = resolve(s.base.host_id, own);
            out.entry(s.base.host_id)
                .or_default()
                .services
                .push(service_summary(s, status, is_fresh));
        }
        for ip in &ips {
            // Skip loopback (127.0.0.0/8, ::1) — typically the daemon's own
            // local address, set once at daemon registration and never
            // re-included in subsequent scan sets. Would go stale on its own
            // and falsely mark the daemon host as Changed.
            if ip.base.ip_address.is_loopback() {
                continue;
            }
            let own = compute_digest_status(ip, &scanned_ip_ids, window);
            let (status, is_fresh) = resolve(ip.base.host_id, own);
            out.entry(ip.base.host_id)
                .or_default()
                .ip_addresses
                .push(ip_summary(ip, status, is_fresh));
        }
        for i in &interfaces {
            let own = compute_digest_status(i, &scanned_interface_ids, window);
            let (status, is_fresh) = resolve(i.base.host_id, own);
            out.entry(i.base.host_id)
                .or_default()
                .interfaces
                .push(interface_summary(i, status, is_fresh));
        }
        for p in &ports {
            let own = compute_digest_status(p, &scanned_port_ids, window);
            let (status, is_fresh) = resolve(p.base.host_id, own);
            out.entry(p.base.host_id)
                .or_default()
                .ports
                .push(port_summary(p, status, is_fresh));
        }
        Ok(out)
    }

    async fn resolve_recipients(
        &self,
        network_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Vec<DigestRecipient>> {
        let users = self
            .user_service
            .get_users_with_network_access(&network_id, &organization_id)
            .await?;
        Ok(users
            .into_iter()
            .map(|u| DigestRecipient {
                user_id: u.id,
                email: u.base.email,
                discovery_digest_enabled: u.base.email_settings.discovery_digest,
            })
            .collect())
    }
}

impl EventBusService<Host> for DiscoveryDigestService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &Host) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, _entity: &Host) -> Option<Uuid> {
        None
    }
}

/// Per-scan context for [`compute_digest_status`]. Built once per digest.
struct DigestWindow {
    t_start: DateTime<Utc>,
    t_end: DateTime<Utc>,
    /// Instant before which a `last_seen_at` counts as stale, from this
    /// network's configured window (`Network::stale_cutoff`) anchored at
    /// `t_end`. The same rule the inventory and topology apply, so a host
    /// reported stale here is the host badged stale in the app.
    cutoff: DateTime<Utc>,
    /// The same cutoff anchored at the *previous* session's finish. An entity
    /// that is stale now but was not stale then crossed the line during this
    /// session — that transition is what makes a card worth emailing.
    /// `None` on a network's first-ever scan, where no transition is claimable.
    prev_cutoff: Option<DateTime<Utc>>,
}

/// Compute the per-entity digest status. Generic over any
/// `DiscoveryTracked` type — hosts and their children all flow through
/// this single helper.
///
/// Returns `(status, is_fresh)` where `is_fresh` means "this status was
/// acquired in THIS scan" (a transition just happened). Stably-stale
/// entities have `is_fresh == false` and don't trigger card inclusion.
///
/// Staleness itself is delegated to [`DiscoveryTracked::freshness`], the single
/// definition shared with the read path — this function only adds the
/// digest-specific `New` bucket and the transition detection.
///
/// `scanned_ids` is the daemon-reported set for whichever entity kind
/// `T` is — `scanned.host_ids` for hosts, `scanned.port_ids` for ports,
/// etc.
fn compute_digest_status<T: DiscoveryTracked>(
    entity: &T,
    scanned_ids: &HashSet<Uuid>,
    window: &DigestWindow,
) -> (EntityFreshness, bool) {
    let created_at = entity.created_at();
    let is_new = scanned_ids.contains(&entity.id())
        && created_at >= window.t_start
        && created_at <= window.t_end;
    if is_new {
        return (EntityFreshness::New, true);
    }
    match entity.freshness(window.cutoff) {
        EntityFreshness::Stale => {
            // Crossed the line during this session iff it was still inside the
            // window as of the previous session's finish.
            let just_crossed = window
                .prev_cutoff
                .is_some_and(|prev| entity.last_seen_at() >= prev);
            (EntityFreshness::Stale, just_crossed)
        }
        other => (other, false),
    }
}

/// What a discovery session structurally covered, and therefore which entities
/// its silence says anything about.
///
/// Absence of detection is only meaningful for an entity the session could have
/// reached. Without this gate a targeted-subnet scan, a second daemon covering
/// disjoint subnets, or a Docker/self-report run reports the rest of the network
/// as stale.
enum ScanCoverage {
    /// Swept these subnets. A host is covered iff it holds a live IP in one.
    Subnets(HashSet<Uuid>),
    /// Touched a single host — no subnet sweep at all. A rescan, or the frozen
    /// `Docker` / `SelfReport` types.
    SingleHost(Uuid),
}

impl ScanCoverage {
    fn for_session(discovery_type: &DiscoveryType, scanned: &ScannedEntityIds) -> Self {
        match discovery_type {
            DiscoveryType::Docker { host_id, .. } | DiscoveryType::SelfReport { host_id } => {
                Self::SingleHost(*host_id)
            }
            // Unreachable while the digest skips rescans outright, but it stops
            // the fallback below from claiming subnet-wide coverage for a run
            // that touched one host, should that suppression ever be relaxed.
            DiscoveryType::Rescan { target_host_id, .. } => Self::SingleHost(*target_host_id),
            // An explicit subnet list is the user's stated target. Otherwise
            // fall back to the subnets the daemon actually confirmed this run.
            DiscoveryType::Network {
                subnet_ids: Some(ids),
                ..
            }
            | DiscoveryType::Unified {
                subnet_ids: Some(ids),
                ..
            } if !ids.is_empty() => Self::Subnets(ids.iter().copied().collect()),
            _ => Self::Subnets(scanned.subnet_ids.iter().copied().collect()),
        }
    }

    /// Whether this session's silence about `host_id` is meaningful.
    ///
    /// `observed` is whether the session actually reported this host. It only matters to the
    /// subnet arm, and it is there because a host's subnets are derived from its addresses: a host
    /// with none belongs to no swept subnet, so the arithmetic dropped it before it was ever
    /// bucketed — and a device the scan had just discovered produced no digest line at all, not
    /// even as an addition. A session that saw a host covers that host, whatever placing it in a
    /// subnet would have concluded.
    ///
    /// It deliberately does not widen the `SingleHost` arm. Those sessions name the one host they
    /// speak for, and a Docker or self-report run touches several while claiming authority over
    /// exactly one.
    ///
    /// Silence about an address-less host stays un-asserted, which is the honest reading: an
    /// address sweep cannot conclude anything about a device that has no address to sweep.
    fn covers_host(
        &self,
        host_id: Uuid,
        host_subnets: Option<&HashSet<Uuid>>,
        observed: bool,
    ) -> bool {
        match self {
            Self::SingleHost(id) => *id == host_id,
            Self::Subnets(swept) => {
                observed
                    || host_subnets.is_some_and(|subnets| subnets.iter().any(|s| swept.contains(s)))
            }
        }
    }

    /// Whether this session swept subnets at all. Network-scoped conclusions
    /// (VLANs) are only drawn from a run that did.
    fn swept_subnets(&self) -> bool {
        matches!(self, Self::Subnets(s) if !s.is_empty())
    }
}

/// How a host's children should be classified this session.
enum ChildPolicy {
    /// The host was reached, so a child's absence is the child's own — a
    /// genuinely closed port or removed service.
    Classify,
    /// The host was not reached. Its children inherit the host's verdict rather
    /// than each decaying independently — otherwise a host offline for a week
    /// reads as "host stale AND every service stale", and the children appear
    /// to have been removed one by one when nothing was observed at all.
    Inherit(EntityFreshness, bool),
}

impl ChildPolicy {
    /// Apply this policy to a child's own classification.
    fn resolve(&self, own: (EntityFreshness, bool)) -> (EntityFreshness, bool) {
        match self {
            Self::Classify => own,
            Self::Inherit(status, is_fresh) => (*status, *is_fresh),
        }
    }
}

/// True when at least one child has `is_fresh == true` — i.e. a real
/// transition happened on this host this scan. Used to drop noisy host
/// cards whose only "non-Unchanged" children have been in that state for
/// multiple scans already.
fn card_has_fresh_children(card: &AffectedHostCard) -> bool {
    card.services.iter().any(|s| s.is_fresh)
        || card.ip_addresses.iter().any(|x| x.is_fresh)
        || card.interfaces.iter().any(|i| i.is_fresh)
        || card.ports.iter().any(|p| p.is_fresh)
}

#[derive(Default)]
struct HostChildren {
    services: Vec<ServiceSummary>,
    ip_addresses: Vec<IpAddressSummary>,
    interfaces: Vec<InterfaceSummary>,
    ports: Vec<PortSummary>,
}

fn build_card(
    host: &Host,
    status: EntityFreshness,
    children: &HashMap<Uuid, HostChildren>,
) -> AffectedHostCard {
    let kids = children.get(&host.id);
    AffectedHostCard {
        host: host_summary(host),
        status,
        services: kids.map(|c| c.services.clone()).unwrap_or_default(),
        ip_addresses: kids.map(|c| c.ip_addresses.clone()).unwrap_or_default(),
        interfaces: kids.map(|c| c.interfaces.clone()).unwrap_or_default(),
        ports: kids.map(|c| c.ports.clone()).unwrap_or_default(),
    }
}

fn host_summary(h: &Host) -> HostSummary {
    let label = crate::server::shared::attribution::text_of(&h.base.hostname)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| h.base.name.to_string());
    HostSummary { id: h.id, label }
}

fn port_summary(p: &Port, status: EntityFreshness, is_fresh: bool) -> PortSummary {
    PortSummary {
        id: p.id,
        host_id: p.base.host_id,
        // Port's Display impl is `"{port_type} (ID: {id})"`. Drop the ID
        // suffix — recipients only want the human-readable port type.
        label: p.base.port_type.to_string(),
        status,
        is_fresh,
    }
}

fn service_summary(
    s: &NetworkServiceEntity,
    status: EntityFreshness,
    is_fresh: bool,
) -> ServiceSummary {
    let logo_url = {
        let url = s.base.service_definition.logo_url();
        if url.is_empty() {
            None
        } else {
            Some(url.to_string())
        }
    };
    // Any container runtime, not just Docker: matching one variant left Podman containers
    // uncounted in the digest.
    let is_container = s.base.virtualization_metadata.is_some();
    ServiceSummary {
        id: s.id,
        host_id: s.base.host_id,
        name: s.base.name.clone(),
        is_container,
        logo_url,
        status,
        is_fresh,
    }
}

fn ip_summary(ip: &IPAddress, status: EntityFreshness, is_fresh: bool) -> IpAddressSummary {
    IpAddressSummary {
        id: ip.id,
        host_id: ip.base.host_id,
        address: ip.base.ip_address.to_string(),
        status,
        is_fresh,
    }
}

fn interface_summary(i: &Interface, status: EntityFreshness, is_fresh: bool) -> InterfaceSummary {
    // Interface's Display includes its UUID. For the digest we want only the
    // human-readable bits: the description if discovery provided one, else
    // the ifIndex.
    let label = match (
        i.base.if_descr.as_deref().filter(|d| !d.is_empty()),
        i.base.if_index,
    ) {
        (Some(descr), _) => descr.to_string(),
        (None, Some(if_index)) => format!("ifIndex {if_index}"),
        // Neither a description nor an index: a port known only as a name a neighbour published.
        (None, None) => i.base.if_name.clone().unwrap_or_default(),
    };
    InterfaceSummary {
        id: i.id,
        host_id: i.base.host_id,
        label,
        status,
        is_fresh,
    }
}

fn subnet_summary(s: &Subnet) -> SubnetSummary {
    SubnetSummary {
        id: s.id,
        label: s.base.name.clone(),
    }
}

fn vlan_summary(v: &Vlan) -> VlanSummary {
    VlanSummary {
        id: v.id,
        vlan_number: v.base.vlan_number,
        name: v.base.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::HostBase;
    use crate::server::networks::r#impl::{DEFAULT_STALE_AFTER_HOURS, Network, NetworkBase};
    use crate::server::services::r#impl::base::{Service as Svc, ServiceBase};
    use crate::server::shared::types::entities::EntitySource;

    const HOUR: i64 = 3600;

    fn t(secs_ago: i64) -> DateTime<Utc> {
        // Fixed epoch-based clock: these tests compare instants, never "now".
        DateTime::from_timestamp(1_800_000_000 - secs_ago, 0).unwrap()
    }

    fn network(stale_after_hours: Option<i64>) -> Network {
        Network {
            id: Uuid::new_v4(),
            base: NetworkBase {
                stale_after_hours,
                ..NetworkBase::new(Uuid::new_v4())
            },
            ..Default::default()
        }
    }

    /// A discovery-created host last observed `secs_ago` before the reference.
    fn host(seen_secs_ago: i64) -> Host {
        let mut h = Host::new(HostBase {
            source: EntitySource::Discovery,
            ..Default::default()
        });
        h.last_seen_at = t(seen_secs_ago);
        h.created_at = t(seen_secs_ago + 100 * 24 * HOUR);
        h
    }

    fn service(host_id: Uuid, seen_secs_ago: i64) -> Svc {
        Svc {
            id: Uuid::new_v4(),
            last_seen_at: t(seen_secs_ago),
            created_at: t(seen_secs_ago + 100 * 24 * HOUR),
            base: ServiceBase {
                host_id,
                source: EntitySource::Discovery,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Window anchored at the reference instant, for a network with the given
    /// threshold, with the previous session `prev_secs_ago` before it.
    fn window(stale_after_hours: i64, prev_secs_ago: Option<i64>) -> DigestWindow {
        let net = network(Some(stale_after_hours));
        let t_end = t(0);
        DigestWindow {
            t_start: t(600),
            t_end,
            cutoff: net.stale_cutoff(t_end),
            prev_cutoff: prev_secs_ago.map(|p| net.stale_cutoff(t(p))),
        }
    }

    fn none_scanned() -> HashSet<Uuid> {
        HashSet::new()
    }

    // ---- A far end minted by this session's resolution pass ---------------

    /// Minting happens server-side after the daemon's own work, so the two gates that decide
    /// whether a host reaches the digest are the ones this exercises together: `covers_host`
    /// admits it only because the session observed it (it has no addresses, so no subnet can
    /// place it), and `compute_digest_status` calls it new only because it is in the scanned set
    /// *and* its `created_at` falls inside the window. Either gate alone still drops it, which is
    /// why they are asserted as a pair rather than separately.
    #[test]
    fn a_far_end_this_session_minted_is_reported_as_added() {
        let w = window(24 * 7, Some(HOUR));

        // Stamped with the session's clock, the way the resolution pass now stamps it.
        let mut minted = Host::new(HostBase {
            source: EntitySource::Inferred,
            ..Default::default()
        });
        minted.last_seen_at = w.t_end;
        minted.created_at = w.t_end;

        let scanned: HashSet<Uuid> = HashSet::from([minted.id]);
        let coverage = ScanCoverage::Subnets(HashSet::from([Uuid::new_v4()]));

        assert!(
            coverage.covers_host(minted.id, None, true),
            "a minted far end has no addresses, so only having been observed can cover it"
        );
        assert_eq!(
            compute_digest_status(&minted, &scanned, &w),
            (EntityFreshness::New, true),
            "a host this session created and touched is an addition, not a silent arrival"
        );
    }

    /// The trap the scan-time stamp exists to avoid. `Utc::now()` at mint time is *after* the
    /// daemon's `finished_at`, which is where the window closes — so a host stamped with the wall
    /// clock falls outside the window that exists to report it, and `is_new` rejects it even
    /// though the session both created and scanned it. Nothing about this is visible at the call
    /// site, which is why it is pinned here.
    #[test]
    fn a_minted_host_stamped_after_the_window_closes_is_not_reported_as_new() {
        let w = window(24 * 7, Some(HOUR));

        let mut too_late = Host::new(HostBase {
            source: EntitySource::Inferred,
            ..Default::default()
        });
        // One second past `finished_at` — the gap between the daemon finishing and the server
        // minting.
        too_late.last_seen_at = w.t_end + chrono::Duration::seconds(1);
        too_late.created_at = w.t_end + chrono::Duration::seconds(1);

        let scanned: HashSet<Uuid> = HashSet::from([too_late.id]);
        assert_ne!(
            compute_digest_status(&too_late, &scanned, &w).0,
            EntityFreshness::New,
            "stamping mint time instead of scan time puts the host outside its own scan's window"
        );
    }

    // ---- The decay scenario from the task -------------------------------

    // A host reported with 3 services loses one per scan, then goes dark
    // itself. Each entity's verdict must follow its OWN last observation, and
    // once the host stops answering its children must stop decaying separately.
    #[test]
    fn services_decay_individually_while_the_host_is_still_answering() {
        let w = window(24 * 7, Some(HOUR));
        let h = host(0); // host answered this scan
        let host_id = h.id;
        let mut scanned = HashSet::new();
        scanned.insert(host_id);

        // Still reported this scan.
        let live = service(host_id, 0);
        // Dropped out 2 days ago — inside the 7-day window, so not yet stale.
        let recently_gone = service(host_id, 2 * 24 * HOUR);
        // Dropped out 8 days ago — past the window.
        let long_gone = service(host_id, 8 * 24 * HOUR);

        let mut svc_scanned = HashSet::new();
        svc_scanned.insert(live.id);

        assert_eq!(
            compute_digest_status(&h, &scanned, &w).0,
            EntityFreshness::Current,
            "a host answering this scan is current"
        );
        assert_eq!(
            compute_digest_status(&live, &svc_scanned, &w).0,
            EntityFreshness::Current
        );
        assert_eq!(
            compute_digest_status(&recently_gone, &svc_scanned, &w).0,
            EntityFreshness::Current,
            "absent for 2 days is well inside a 7-day window — not stale yet"
        );
        assert_eq!(
            compute_digest_status(&long_gone, &svc_scanned, &w).0,
            EntityFreshness::Stale
        );
    }

    #[test]
    fn once_the_host_goes_dark_its_children_inherit_instead_of_decaying_alone() {
        let w = window(24 * 7, Some(HOUR));
        let h = host(8 * 24 * HOUR); // host itself unreachable for 8 days
        let host_id = h.id;

        let (host_status, host_fresh) = compute_digest_status(&h, &none_scanned(), &w);
        assert_eq!(host_status, EntityFreshness::Stale);

        // A service last seen at the same time as the host. On its own merits
        // it is stale, but the host was never reached, so nothing was observed
        // about the service — it must inherit rather than assert its own decay.
        let svc = service(host_id, 8 * 24 * HOUR);
        let own = compute_digest_status(&svc, &none_scanned(), &w);
        let policy = ChildPolicy::Inherit(host_status, host_fresh);

        assert_eq!(
            policy.resolve(own),
            (host_status, host_fresh),
            "an unreached host's children report the host's verdict"
        );
        assert_eq!(
            ChildPolicy::Classify.resolve(own),
            own,
            "a reached host's children keep their own verdict"
        );
    }

    // ---- Staleness tracks elapsed time, not scan count -------------------

    // The mismatch this model exists to remove: with a scan-count measure, an
    // entity missing N scans was "missing" whether that was 45 minutes or 3
    // months. The verdict must depend only on elapsed time vs the window.
    #[test]
    fn verdict_depends_on_elapsed_time_not_on_how_many_scans_were_missed() {
        let w = window(24 * 7, Some(HOUR));

        // Fast-cadence network: missing many scans, but only 45 minutes.
        let missed_three_quarter_hourly_scans = host(45 * 60);
        assert_eq!(
            compute_digest_status(&missed_three_quarter_hourly_scans, &none_scanned(), &w).0,
            EntityFreshness::Current,
            "45 minutes is not stale under a 7-day window, however many scans it spans"
        );

        // Slow-cadence network: missed a single scan, but a month has passed.
        let missed_one_monthly_scan = host(30 * 24 * HOUR);
        assert_eq!(
            compute_digest_status(&missed_one_monthly_scan, &none_scanned(), &w).0,
            EntityFreshness::Stale,
            "a month unobserved is stale even though only one scan was missed"
        );
    }

    #[test]
    fn a_network_with_no_configured_window_falls_back_to_the_default() {
        let default_net = network(None);
        let explicit_net = network(Some(DEFAULT_STALE_AFTER_HOURS));
        assert_eq!(
            default_net.stale_cutoff(t(0)),
            explicit_net.stale_cutoff(t(0))
        );

        // And a tighter window genuinely bites sooner.
        let strict = network(Some(1));
        assert!(strict.stale_cutoff(t(0)) > default_net.stale_cutoff(t(0)));
    }

    #[test]
    fn entities_discovery_never_refreshes_are_never_stale() {
        let w = window(1, None); // 1-hour window; everything below is older
        let mut manual = host(30 * 24 * HOUR);
        manual.base.source = EntitySource::Manual;
        let mut system = host(30 * 24 * HOUR);
        system.base.source = EntitySource::System;
        let discovered = host(30 * 24 * HOUR);

        assert_eq!(
            compute_digest_status(&manual, &none_scanned(), &w).0,
            EntityFreshness::Current,
            "a hand-created host is never refreshed by discovery, so it cannot go stale"
        );
        assert_eq!(
            compute_digest_status(&system, &none_scanned(), &w).0,
            EntityFreshness::Current
        );
        assert_eq!(
            compute_digest_status(&discovered, &none_scanned(), &w).0,
            EntityFreshness::Stale,
            "the same age on a discovered host does go stale"
        );
    }

    // ---- Transition reporting -------------------------------------------

    #[test]
    fn a_host_is_reported_only_on_the_scan_where_it_crosses_into_stale() {
        // 7-day window, previous session an hour before this one.
        let w = window(24 * 7, Some(HOUR));

        // Last seen 7 days + 30 minutes ago: inside the window as of the
        // previous session, outside it now.
        let just_crossed = host(7 * 24 * HOUR + 30 * 60);
        let (status, is_fresh) = compute_digest_status(&just_crossed, &none_scanned(), &w);
        assert_eq!(status, EntityFreshness::Stale);
        assert!(
            is_fresh,
            "crossing the line this session is worth reporting"
        );

        // Stale for weeks: still stale, but nothing happened this session.
        let long_stale = host(30 * 24 * HOUR);
        let (status, is_fresh) = compute_digest_status(&long_stale, &none_scanned(), &w);
        assert_eq!(status, EntityFreshness::Stale);
        assert!(
            !is_fresh,
            "an entity stale for weeks must not be re-reported every scan"
        );
    }

    #[test]
    fn no_transition_is_claimed_on_a_networks_first_scan() {
        let w = window(24 * 7, None);
        let (status, is_fresh) = compute_digest_status(&host(30 * 24 * HOUR), &none_scanned(), &w);
        assert_eq!(status, EntityFreshness::Stale);
        assert!(
            !is_fresh,
            "with no previous session there is no crossing to report"
        );
    }

    // ---- Coverage: whose absence means anything --------------------------

    fn subnets(ids: &[Uuid]) -> HashSet<Uuid> {
        ids.iter().copied().collect()
    }

    #[test]
    fn a_targeted_scan_says_nothing_about_hosts_in_subnets_it_did_not_sweep() {
        let swept = Uuid::new_v4();
        let untouched = Uuid::new_v4();
        let coverage = ScanCoverage::for_session(
            &DiscoveryType::Unified {
                host_id: Uuid::new_v4(),
                subnet_ids: Some(vec![swept]),
                host_naming_fallback: Default::default(),
                scan_settings: Default::default(),
            },
            &ScannedEntityIds::default(),
        );

        let in_scope = Uuid::new_v4();
        let out_of_scope = Uuid::new_v4();
        assert!(coverage.covers_host(in_scope, Some(&subnets(&[swept])), false));
        assert!(
            !coverage.covers_host(out_of_scope, Some(&subnets(&[untouched])), false),
            "a host in an unswept subnet must be left out of the digest entirely"
        );
        assert!(
            !coverage.covers_host(Uuid::new_v4(), None, false),
            "a host with no IPs cannot be placed in a swept subnet"
        );
    }

    /// A host's subnets come from its addresses, so a host with none is placed in nothing and the
    /// subnet arm concluded it was out of scope. That dropped it before bucketing — so a device
    /// the scan had just discovered produced no digest line at all, not even as an addition.
    /// Seeing it is the coverage; the subnet arithmetic is only how coverage is inferred for the
    /// hosts it can speak for.
    #[test]
    fn a_scan_that_saw_an_address_less_host_still_reports_it() {
        let swept = Uuid::new_v4();
        let coverage = ScanCoverage::for_session(
            &DiscoveryType::Unified {
                host_id: Uuid::new_v4(),
                subnet_ids: Some(vec![swept]),
                host_naming_fallback: Default::default(),
                scan_settings: Default::default(),
            },
            &ScannedEntityIds::default(),
        );

        let mac_only_host = Uuid::new_v4();
        assert!(
            coverage.covers_host(mac_only_host, None, true),
            "a session that observed a host covers it, whatever placing it in a subnet concludes"
        );
        assert!(
            !coverage.covers_host(Uuid::new_v4(), None, false),
            "an address-less host this session never saw stays un-asserted, not stale"
        );
    }

    #[test]
    fn two_daemons_on_disjoint_subnets_do_not_report_each_others_hosts() {
        let daemon_a_subnet = Uuid::new_v4();
        let daemon_b_subnet = Uuid::new_v4();
        // Daemon A scans without an explicit target: coverage is what it
        // actually confirmed, which is only its own subnet.
        let coverage = ScanCoverage::for_session(
            &DiscoveryType::Unified {
                host_id: Uuid::new_v4(),
                subnet_ids: None,
                host_naming_fallback: Default::default(),
                scan_settings: Default::default(),
            },
            &ScannedEntityIds {
                subnet_ids: vec![daemon_a_subnet],
                ..Default::default()
            },
        );

        assert!(coverage.covers_host(Uuid::new_v4(), Some(&subnets(&[daemon_a_subnet])), false));
        assert!(
            !coverage.covers_host(Uuid::new_v4(), Some(&subnets(&[daemon_b_subnet])), false),
            "daemon A's silence about daemon B's subnet carries no information"
        );
    }

    #[test]
    fn runs_that_sweep_no_subnets_speak_only_for_the_daemons_own_host() {
        let daemon_host = Uuid::new_v4();
        let other_host = Uuid::new_v4();
        let any_subnet = subnets(&[Uuid::new_v4()]);

        for discovery_type in [
            DiscoveryType::SelfReport {
                host_id: daemon_host,
            },
            DiscoveryType::Docker {
                host_id: daemon_host,
                host_naming_fallback: Default::default(),
            },
        ] {
            let coverage = ScanCoverage::for_session(&discovery_type, &ScannedEntityIds::default());
            assert!(coverage.covers_host(daemon_host, Some(&any_subnet), false));
            assert!(
                !coverage.covers_host(other_host, Some(&any_subnet), false),
                "a container/self-report run never swept the network"
            );
            assert!(
                !coverage.swept_subnets(),
                "network-wide conclusions (VLANs) must not be drawn from it"
            );
        }
    }

    #[test]
    fn a_scan_that_confirmed_no_subnets_covers_nothing() {
        let coverage = ScanCoverage::for_session(
            &DiscoveryType::Unified {
                host_id: Uuid::new_v4(),
                subnet_ids: None,
                host_naming_fallback: Default::default(),
                scan_settings: Default::default(),
            },
            &ScannedEntityIds::default(),
        );
        assert!(!coverage.covers_host(Uuid::new_v4(), Some(&subnets(&[Uuid::new_v4()])), false));
        assert!(!coverage.swept_subnets());
    }
}
