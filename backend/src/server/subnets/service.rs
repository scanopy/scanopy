use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    ip_addresses::service::IPAddressService,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        events::{bus::EventBus, types::EntityOperation},
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Storable, Storage},
        },
        types::entities::EntitySource,
    },
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        correction_events::{SubnetCorrection, SubnetCorrectionScope},
        inference::{infer_range_for, overlaps, placeable_subnet},
        types::SubnetType,
    },
    tags::entity_tags::EntityTagService,
};
use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::Utc;
use cidr::IpCidr;
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

pub struct SubnetService {
    storage: Arc<GenericPostgresStorage<Subnet>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    /// For re-filing addresses a narrowed range no longer covers. A *service*, not the storage
    /// behind it, and no cycle: `IPAddressService` depends on nothing and is built before this one.
    /// It has to live here rather than in a caller because a daemon's own interfaced subnets arrive
    /// through `DaemonService`, which cannot reach addresses at all.
    ip_address_service: Arc<IPAddressService>,
}

impl EventBusService<Subnet> for SubnetService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Subnet) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Subnet) -> Option<Uuid> {
        None
    }
}

impl SubnetService {
    /// Re-file every address a corrected range no longer covers, and say what happened.
    ///
    /// Only narrowing displaces anything: widening adds space, and a promotion moves the rung
    /// rather than the range. Displaced addresses go back through [`Self::place_address`] — the
    /// same rule that filed them originally — so nothing is asserted that was not already being
    /// asserted. That call creates `Inferred` subnets, and `corrects_inferred_range` requires a
    /// *strictly* more authoritative reading, so the `create` inside it can correct nothing and
    /// this cannot recurse.
    ///
    /// An address that comes back `Unplaceable` keeps the subnet it has. It is then a row pointing
    /// at a range that does not contain it, which `needs_placement` now notices, so the next scan
    /// re-files it rather than it being stranded silently.
    async fn report_correction(
        &self,
        corrected: &Subnet,
        before: (IpCidr, AttributeSource),
        authentication: AuthenticatedEntity,
    ) {
        let (from_cidr, from_source) = before;

        let kind = if from_cidr == *corrected.base.cidr {
            SubnetCorrection::Promoted
        } else if corrected.base.cidr.contains(&from_cidr.first_address())
            && corrected.base.cidr.network_length() < from_cidr.network_length()
        {
            SubnetCorrection::Widened
        } else {
            SubnetCorrection::Narrowed {
                addresses_replaced: self.replace_displaced_addresses(corrected).await,
            }
        };

        self.event_bus()
            .publish(Event::new(
                SubnetCorrectionScope {
                    network_id: corrected.base.network_id,
                    subnet_id: corrected.id,
                    from_cidr: from_cidr.to_string(),
                    to_cidr: corrected.base.cidr.to_string(),
                    from_source,
                    to_source: corrected.base.cidr.source(),
                },
                kind,
                authentication,
            ))
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    subnet_id = %corrected.id,
                    error = %e,
                    "Could not report a corrected subnet range"
                );
            });
    }

    /// Fold `source` into `target`, moving its addresses and removing the row.
    ///
    /// The resolution for a reading that covers several inferred ranges. Discovery declines that
    /// case because the only way to act on it is to delete rows, so it is offered to a person
    /// instead — and this is what they invoke.
    ///
    /// Containment is required, not merely checked for tidiness: it is what makes the move total.
    /// Every address in a contained range is inside the covering one, so nothing has to be re-placed
    /// and nothing can be orphaned. Refusing the non-contained case is refusing to guess where the
    /// stragglers should go.
    pub async fn merge_into(
        &self,
        source_id: Uuid,
        target_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<Subnet> {
        let source = self
            .get_by_id(&source_id)
            .await?
            .ok_or_else(|| Error::msg("the subnet to merge no longer exists"))?;
        let target = self
            .get_by_id(&target_id)
            .await?
            .ok_or_else(|| Error::msg("the subnet to merge into no longer exists"))?;

        if source.id == target.id {
            return Err(Error::msg("a subnet cannot be merged into itself"));
        }
        if source.base.network_id != target.base.network_id {
            return Err(Error::msg("subnets on different networks cannot be merged"));
        }
        if target.is_organizational_subnet() {
            return Err(Error::msg(
                "the Internet and Remote Network rows hold every address and are not merge targets",
            ));
        }
        if !overlaps(&target.base.cidr, &source.base.cidr)
            || target.base.cidr.network_length() > source.base.cidr.network_length()
        {
            return Err(Error::msg(
                "a subnet can only be merged into a range that contains it",
            ));
        }

        for mut address in self.ip_address_service.get_for_subnet(&source.id).await? {
            address.base.subnet_id = target.id;
            self.ip_address_service
                .update(&mut address, authentication.clone())
                .await?;
        }

        self.delete(&source.id, authentication).await?;

        tracing::info!(
            merged = %source.base.cidr,
            into = %target.base.cidr,
            network_id = %target.base.network_id,
            "Merged an assumed range into the range that contains it"
        );

        Ok(target)
    }

    /// Whether narrowing `existing` to `new_cidr` would leave one of its addresses with nowhere to
    /// go.
    ///
    /// Asked against the subnet list *as it would be after* the correction, which is the whole
    /// point: it is the corrected range itself that blocks the conventional bucket its own castoffs
    /// would otherwise fall into. Narrowing `10.20.30.0/24` to `/28` strands `10.20.30.24` for
    /// exactly that reason — the `/24` it would be filed under now overlaps the `/28`.
    ///
    /// The two arms are the two halves of [`Self::place_address`], asked without performing it, so
    /// this cannot answer differently from the re-placement that follows.
    async fn narrowing_would_strand(
        &self,
        existing: &Subnet,
        new_cidr: IpCidr,
        live: &[Subnet],
    ) -> bool {
        // Only a narrowing displaces anything. Widening adds space and a promotion moves the rung.
        if new_cidr.contains(&existing.base.cidr.first_address())
            && new_cidr.network_length() <= existing.base.cidr.network_length()
        {
            return false;
        }

        let addresses = match self.ip_address_service.get_for_subnet(&existing.id).await {
            Ok(addresses) => addresses,
            // Unreadable is not evidence of a stranding, and refusing the correction on a transient
            // storage error would leave the guess in place for no reason.
            Err(e) => {
                tracing::warn!(
                    subnet_id = %existing.id,
                    error = %e,
                    "Could not check what a narrowed range would displace"
                );
                return false;
            }
        };

        let after: Vec<Subnet> = live
            .iter()
            .map(|s| {
                let mut s = s.clone();
                if s.id == existing.id {
                    s.base.cidr = SubnetCidr::new(SubnetCidrValue(new_cidr), s.base.cidr.source());
                }
                s
            })
            .collect();

        addresses
            .iter()
            .map(|a| a.base.ip_address)
            .filter(|ip| !new_cidr.contains(ip))
            .any(|ip| {
                placeable_subnet(&after, ip).is_none() && infer_range_for(ip, &after).is_none()
            })
    }

    /// Move every address `corrected` no longer covers to wherever it now belongs. Returns how
    /// many moved.
    async fn replace_displaced_addresses(&self, corrected: &Subnet) -> usize {
        let addresses = match self.ip_address_service.get_for_subnet(&corrected.id).await {
            Ok(addresses) => addresses,
            Err(e) => {
                tracing::warn!(
                    subnet_id = %corrected.id,
                    error = %e,
                    "Could not read the addresses a narrowed range displaced"
                );
                return 0;
            }
        };

        let mut moved = 0;
        for mut address in addresses {
            if corrected.base.cidr.contains(&address.base.ip_address) {
                continue;
            }
            let placement = self
                .place_address(corrected.base.network_id, address.base.ip_address)
                .await;
            let subnet_id = match placement {
                Ok(Placement::Existing(id) | Placement::Inferred(id)) => id,
                Ok(Placement::Unplaceable) => {
                    tracing::warn!(
                        address = %address.base.ip_address,
                        subnet_id = %corrected.id,
                        "A narrowed range displaced an address nothing can hold; leaving it in place"
                    );
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        address = %address.base.ip_address,
                        error = %e,
                        "Could not re-place an address a narrowed range displaced"
                    );
                    continue;
                }
            };

            address.base.subnet_id = subnet_id;
            match self
                .ip_address_service
                .update(&mut address, AuthenticatedEntity::System)
                .await
            {
                Ok(_) => moved += 1,
                Err(e) => tracing::warn!(
                    address = %address.base.ip_address,
                    error = %e,
                    "Could not write back an address a narrowed range displaced"
                ),
            }
        }
        moved
    }
}

/// Whether `incoming` is the same subnet as `existing`, and so should refresh that row rather than
/// insert beside it.
///
/// A free function rather than a closure so it can be tested without a database — the same shape
/// `match_existing_interface` uses for the interface identity tiers, and for the same reason: this
/// is the rule that decides whether a scan creates a duplicate, and it was previously reachable
/// only through a container-bridge integration test.
pub(crate) fn matches_existing_subnet(incoming: &Subnet, existing: &Subnet) -> bool {
    // CIDR must match first
    if !incoming.eq(existing) {
        return false;
    }

    // Docker will default to the same subnet range for bridge networks, so we need a way
    // to distinguish docker bridge subnets with the same CIDR but which originate from
    // different hosts. The dedup uses subnet virtualization (which carries service_id
    // for Docker bridges); discovery metadata used to live on EntitySource but moved to
    // FK columns post-terminal.
    match (&existing.base.source, &incoming.base.source) {
        (EntitySource::Discovery, EntitySource::Discovery) => {
            // Container-runtime bridge networks (Docker/Podman) are
            // host-scoped: the same CIDR on different daemons is a
            // distinct subnet, so they only dedupe when the owning
            // runtime service matches. Distinct runtimes carry
            // distinct service ids, so a Docker and a Podman bridge
            // with the same CIDR never collide.
            if incoming.base.subnet_type.is_container_bridge()
                && existing.base.subnet_type.is_container_bridge()
            {
                match (
                    incoming.base.virtualization_service_id,
                    existing.base.virtualization_service_id,
                ) {
                    (Some(a), Some(b)) => a == b,
                    // An owner-less bridge row predates this scoping, or had its owner
                    // quarantined as dangling by the migration. Either way it merges on
                    // CIDR alone, which is what collapses the duplicates it accumulated.
                    _ => true,
                }
            } else {
                true
            }
        }
        (EntitySource::System, _) | (_, EntitySource::System) => false,
        _ => true,
    }
}

#[async_trait]
impl CrudService<Subnet> for SubnetService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Subnet>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    /// Counts only the subnets the user curates, so the dashboard's per-network
    /// subnet count agrees with the rows the management lists show.
    ///
    /// The default counts every live row (`shared/services/traits.rs`), which is
    /// how the reporter's dashboard total came to include a subnet no page would
    /// display (GH #677).
    async fn count_for_networks(&self, network_ids: &[Uuid]) -> Result<u64, anyhow::Error> {
        let filter = StorableFilter::<Subnet>::new_from_network_ids(network_ids)
            .live()
            .user_managed();
        self.storage().count(filter).await
    }

    async fn create(
        &self,
        subnet: Subnet,
        authentication: AuthenticatedEntity,
    ) -> Result<Subnet, anyhow::Error> {
        // SCD2: natural-key match (CIDR + virtualization) runs against live
        // subnets only; closed historical copies must not match.
        let filter =
            StorableFilter::<Subnet>::new_from_network_ids(&[subnet.base.network_id]).live();
        let all_subnets = self.storage.get_all(filter).await?;

        let subnet = if subnet.id == Uuid::nil() {
            Subnet::new(subnet.base)
        } else {
            subnet
        };

        // A reading that nests with a range Scanopy only inferred corrects that row instead of
        // inserting beside it — but only where the answer is unambiguous. Inferred ranges are
        // pairwise disjoint (`infer_ranges` drops any candidate overlapping a range already held),
        // so at most one can equal or contain the incoming range, while several can sit inside it.
        // That last case says the inferred rows are one segment and should be merged, which means
        // deleting rows; discovery does not do that on its own. See `corrects_inferred_range`.
        let nested: Vec<&Subnet> = all_subnets
            .iter()
            .filter(|existing| subnet.corrects_inferred_range(existing))
            .collect();
        let correctable = match nested.as_slice() {
            [one] => Some(*one),
            [] => None,
            several => {
                tracing::info!(
                    network_id = %subnet.base.network_id,
                    observed_cidr = %subnet.base.cidr,
                    inferred_count = several.len(),
                    "A read range covers several inferred ranges; leaving them for a person to merge"
                );
                None
            }
        };

        // A narrowing has to have somewhere to put what it displaces. Where it does not, the
        // correction is declined and the reading is recorded beside the guess instead.
        let correctable = match correctable {
            Some(existing)
                if self
                    .narrowing_would_strand(existing, *subnet.base.cidr, &all_subnets)
                    .await =>
            {
                tracing::info!(
                    network_id = %subnet.base.network_id,
                    observed_cidr = %subnet.base.cidr,
                    assumed_cidr = %existing.base.cidr,
                    "A read range would leave an address of the range it narrows with nowhere to \
                     go; recording it beside that range rather than stranding them"
                );
                None
            }
            other => other,
        };

        let subnet_from_storage = match all_subnets
            .iter()
            .find(|existing_subnet| matches_existing_subnet(&subnet, existing_subnet))
            .or(correctable)
        {
            Some(existing_subnet) => {
                tracing::info!(
                    existing_subnet_id = %existing_subnet.id,
                    existing_subnet_name = %existing_subnet.base.name,
                    new_subnet_id = %subnet.id,
                    new_subnet_name = %subnet.base.name,
                    subnet_cidr = %subnet.base.cidr,
                    "Duplicate subnet found, refreshing last_seen_at and returning existing"
                );
                // SCD2 semantics: every successful natural-key match advances
                // last_seen_at, even when no field changes. Otherwise
                // unchanged subnets falsely look stale to (future) staleness
                // consumers. The incoming `subnet` was pre-stamped by
                // `HostService::discover_host` when called via discovery (see
                // `ScanContext`) so all entities in one submission share one
                // timestamp; for non-discovery callers the value is whatever
                // they put on the entity.
                let mut refreshed = existing_subnet.clone();
                refreshed.last_seen_at = subnet.last_seen_at;

                // Repair rows left behind by the interface-name heuristic that
                // used to be able to type a subnet as a container bridge (#663).
                if subnet.corrects_container_bridge_guess(existing_subnet) {
                    tracing::info!(
                        subnet_id = %existing_subnet.id,
                        subnet_cidr = %existing_subnet.base.cidr,
                        from = ?existing_subnet.base.subnet_type,
                        to = ?subnet.base.subnet_type,
                        "Reclassifying subnet mistyped as a container bridge"
                    );
                    refreshed.base.subnet_type = subnet.base.subnet_type;
                }

                // The confidence ladder, finally applied. `apply_cidr` refuses anything less
                // authoritative than what is stored, so an inference never displaces a reading and
                // nothing displaces a range a person confirmed. Without this the incoming rung was
                // dropped on the floor here and `Inferred` could only ever be cleared by hand.
                let before = (
                    *existing_subnet.base.cidr,
                    existing_subnet.base.cidr.source(),
                );
                let corrected = refreshed.apply_cidr(*subnet.base.cidr, subnet.base.cidr.source());

                self.storage.update(&mut refreshed).await?;

                if corrected {
                    self.report_correction(&refreshed, before, authentication.clone())
                        .await;
                }
                refreshed
            }
            // If there's no existing subnet, create a new one
            None => {
                // SCD2 origin: this row is being inserted for the first
                // time. Stamp created_at + valid_from to the entity's
                // already-refreshed `last_seen_at`. See
                // `DiscoveryTracked::originate_scan_timestamps`.
                use crate::server::shared::storage::snapshot::DiscoveryTracked;
                let mut subnet = subnet;
                subnet.originate_scan_timestamps(subnet.last_seen_at);
                let mut created = self.storage.create(&subnet).await?;

                // Save tags to junction table
                if let Some(tag_service) = self.entity_tag_service()
                    && let Some(org_id) = authentication.organization_id()
                {
                    tag_service
                        .set_tags(
                            created.id,
                            EntityDiscriminants::Subnet,
                            created.base.tags.clone(),
                            org_id,
                        )
                        .await?;
                    created.base.tags = subnet.base.tags.clone();
                }

                let trigger_stale = created.triggers_staleness(None);

                if let Some(scope) = EntityScope::from_ids(
                    created.id,
                    created.clone().into(),
                    self.get_network_id(&created),
                    self.get_organization_id(&created),
                ) {
                    self.event_bus()
                        .publish(
                            Event::new(scope, EntityOperation::Created, authentication).with_flags(
                                EntityEventFlags {
                                    trigger_stale,
                                    ..Default::default()
                                },
                            ),
                        )
                        .await?;
                }

                subnet
            }
        };
        Ok(subnet_from_storage)
    }
}

/// Where an address belongs, and how sure we are of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// A subnet this network already holds contains it.
    Existing(Uuid),
    /// Nothing held it, so a range was inferred and created. The row carries
    /// [`AttributeSource::LldpNeighbourAddress`], so it badges and asks an operator to confirm it.
    Inferred(Uuid),
    /// Nothing holds it and nothing may be invented for it — a public address, or IPv6 global
    /// unicast, neither of which is a segment of this network to create. The caller decides.
    Unplaceable,
}

impl SubnetService {
    /// Place an address, inferring a range for it when nothing this network holds contains it.
    ///
    /// The single automatic-placement entry point. Every source of a discovered address goes
    /// through it — an LLDP far end, a controller-reported device, a repaired daemon payload — so
    /// they cannot disagree about which subnet an address belongs to, and so that "nothing holds
    /// this" is answered the same way once rather than per integration.
    ///
    /// Deliberately not the placement rule for a *container* endpoint: the runtime API already says
    /// which network the endpoint is on, and re-deriving that by address would throw away an
    /// identity for a guess — see `get_container_interfaces`.
    pub async fn place_address(&self, network_id: Uuid, ip: IpAddr) -> Result<Placement, Error> {
        let live = self
            .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
            .await?;

        if let Some(subnet) = placeable_subnet(&live, ip) {
            return Ok(Placement::Existing(subnet.id));
        }

        let Some(cidr) = infer_range_for(ip, &live) else {
            return Ok(Placement::Unplaceable);
        };

        let mut subnet = Subnet::new(SubnetBase {
            // A range nothing read, only inferred — the whole reason this rung exists.
            cidr: SubnetCidr::new(SubnetCidrValue(cidr), AttributeSource::LldpNeighbourAddress),
            network_id,
            name: cidr.to_string(),
            description: None,
            // Not `Management` even where a management address produced it: on a flat network that
            // address is just the device's LAN address, and typing the subnet would be a second
            // guess stacked on the first.
            subnet_type: SubnetType::Unknown,
            virtualization_service_id: None,
            source: EntitySource::Discovery,
            tags: Vec::new(),
        });
        subnet.last_seen_at = Utc::now();

        let created = self.create(subnet, AuthenticatedEntity::System).await?;
        tracing::info!(
            network_id = %network_id,
            ip = %ip,
            cidr = %cidr,
            subnet_id = %created.id,
            "Inferred a subnet for an address nothing on this network holds"
        );
        Ok(Placement::Inferred(created.id))
    }

    pub fn new(
        storage: Arc<GenericPostgresStorage<Subnet>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        ip_address_service: Arc<IPAddressService>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            entity_tag_service,
            ip_address_service,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::subnets::r#impl::base::SubnetBase;

    fn subnet(cidr: &str, source: EntitySource, cidr_source: AttributeSource) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr.parse().expect("valid test CIDR")),
                cidr_source,
            ),
            source,
            network_id: Uuid::nil(),
            ..Default::default()
        })
    }

    fn discovered(cidr: &str) -> Subnet {
        subnet(
            cidr,
            EntitySource::Discovery,
            AttributeSource::DaemonSelfReport,
        )
    }

    fn inferred(cidr: &str) -> Subnet {
        subnet(
            cidr,
            EntitySource::Discovery,
            AttributeSource::LldpNeighbourAddress,
        )
    }

    fn bridge(cidr: &str, owner: Option<Uuid>) -> Subnet {
        let mut s = discovered(cidr);
        s.base.subnet_type = SubnetType::DockerBridge;
        s.base.virtualization_service_id = owner;
        s
    }

    /// The same range stays the same subnet however each side happened to learn it.
    ///
    /// The test above pairs two `discovered()` subnets, so both carry
    /// `AttributeSource::DaemonSelfReport` and the provenance halves agree by construction. That
    /// is the case the dedup was never asked about: `SubnetBase::cidr` is
    /// `Attributed<SubnetCidrValue>`, `Attributed` derives `PartialEq` over BOTH value and source,
    /// and `Subnet::eq` compares that field with a bare `==` — so it does not ask "same CIDR", it
    /// asks "same CIDR, learned the same way".
    ///
    /// Measured consequence on a populated estate upgraded to v0.17.15: every stored subnet
    /// carries `Unspecified`, the daemon reports its interfaced subnets as `DaemonSelfReport`, and
    /// the first registration therefore inserted a second live row for all 20 of them. Duplicate
    /// subnet ids then break host matching, which compares IP *and* `subnet_id`, so every device
    /// duplicated too and the L2 view lost the edges that crossed the split.
    ///
    /// `ip_addresses_match` in `hosts/service/mod.rs` already draws this distinction for MAC and
    /// says why: the branch is asking whether it is the same hardware, not the same paperwork.
    #[test]
    fn the_same_range_is_the_same_subnet_however_it_was_learned() {
        assert!(
            matches_existing_subnet(&discovered("192.168.4.0/22"), &inferred("192.168.4.0/22")),
            "a range read from a daemon is the same range once inferred from a neighbour"
        );

        // The exact pairing a v0.17.15 upgrade produces: everything already stored was stamped
        // `Unspecified` by the attribution backfill, everything the daemon reports is
        // `DaemonSelfReport`.
        let stored = subnet(
            "192.168.20.0/24",
            EntitySource::Discovery,
            AttributeSource::Unspecified,
        );
        assert!(
            matches_existing_subnet(&discovered("192.168.20.0/24"), &stored),
            "an upgraded row must not duplicate the moment a daemon reports the same range"
        );
    }

    /// The rule the whole dedup rests on: same CIDR is the same subnet.
    #[test]
    fn the_same_range_is_the_same_subnet() {
        assert!(matches_existing_subnet(
            &discovered("192.168.4.0/22"),
            &discovered("192.168.4.0/22")
        ));
        assert!(!matches_existing_subnet(
            &discovered("192.168.4.0/22"),
            &discovered("10.0.0.0/24")
        ));
    }

    /// Two ranges where one contains the other are different subnets to the plain matcher. This is
    /// what makes an observed reading insert beside an inferred guess instead of correcting it.
    #[test]
    fn a_nested_range_is_not_the_same_subnet() {
        assert!(!matches_existing_subnet(
            &discovered("10.20.30.0/23"),
            &inferred("10.20.30.0/24")
        ));
        assert!(!matches_existing_subnet(
            &discovered("10.20.30.0/24"),
            &inferred("10.20.28.0/22")
        ));
    }

    /// Container bridges are host-scoped: the same CIDR on two runtimes is two subnets.
    #[test]
    fn bridges_with_the_same_range_and_different_owners_are_distinct() {
        let mine = bridge("172.17.0.0/16", Some(Uuid::new_v4()));
        let theirs = bridge("172.17.0.0/16", Some(Uuid::new_v4()));
        assert!(!matches_existing_subnet(&mine, &theirs));

        let same_owner = Uuid::new_v4();
        assert!(matches_existing_subnet(
            &bridge("172.17.0.0/16", Some(same_owner)),
            &bridge("172.17.0.0/16", Some(same_owner))
        ));
    }

    /// An owner-less bridge row predates the scoping and merges on CIDR alone, which is what
    /// collapses the duplicates it accumulated.
    #[test]
    fn an_ownerless_bridge_still_merges_on_range() {
        assert!(matches_existing_subnet(
            &bridge("172.17.0.0/16", Some(Uuid::new_v4())),
            &bridge("172.17.0.0/16", None)
        ));
    }

    /// The seeded organizational rows are never merged into by anything, and never merge into
    /// anything — both `0.0.0.0/0`, so without this every subnet would match them.
    #[test]
    fn the_organizational_rows_never_match() {
        let internet = subnet(
            "0.0.0.0/0",
            EntitySource::System,
            AttributeSource::DaemonSelfReport,
        );
        assert!(!matches_existing_subnet(
            &discovered("0.0.0.0/0"),
            &internet
        ));
        assert!(!matches_existing_subnet(
            &internet,
            &discovered("0.0.0.0/0")
        ));
    }
}
