//! Recording a subnet: dedup against what the site holds, correcting an assumed range, or a new row.
use super::*;
use crate::server::subnets::r#impl::create_purpose::SubnetCreatePurpose;

/// `existing` as a reading of `incoming` leaves it, and whether that corrected its range.
///
/// The name and type go by the same ladder as the range. A placeholder name (the CIDR,
/// `Unspecified`) takes the name a controller gives the network, and a type guessed from an
/// interface name takes the controller's own; a value a person set is never displaced. This also
/// repairs the rows the interface-name heuristic once typed as container bridges (#663): those are
/// stored `Unspecified`.
///
/// `apply_cidr` refuses anything less authoritative than what is stored, so an inference never
/// displaces a reading and nothing displaces a range a person confirmed.
fn refresh_matched(existing: &Subnet, incoming: &Subnet) -> (Subnet, bool) {
    let mut refreshed = existing.clone();
    refreshed.last_seen_at = incoming.last_seen_at;
    refreshed
        .base
        .name
        .apply_in_place(incoming.base.name.clone());
    refreshed
        .base
        .subnet_type
        .apply_in_place(incoming.base.subnet_type.clone());
    if let Some(owner) = incoming.owner_for_ownerless_bridge(existing) {
        refreshed.base.virtualization_service_id = Some(owner);
    }
    let corrected = refreshed.apply_cidr(*incoming.base.cidr, incoming.base.cidr.source());
    (refreshed, corrected)
}

impl SubnetService {
    /// Resolve the subnets a daemon on `site_id` reports it is attached to, returning their ids.
    ///
    /// Reads the site's subnets once. A reported subnet that matches a stored one and would change
    /// nothing on it but `last_seen_at` resolves to that row with no write. Anything else (a new
    /// range, a correction of an assumed one, a name or type to take) goes through
    /// [`Self::create_for`]. A daemon may name only its own site, so the site comes from the
    /// caller, never the report. A subnet that fails to resolve is logged and left out.
    pub async fn resolve_reported(
        &self,
        site_id: Uuid,
        reported: Vec<Subnet>,
        authentication: AuthenticatedEntity,
    ) -> Result<Vec<Uuid>, anyhow::Error> {
        let stored = self
            .storage
            .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[site_id]).live())
            .await?;
        let mut ids = Vec::with_capacity(reported.len());
        for mut subnet in reported {
            subnet.base.site_id = site_id;
            let unchanged = stored
                .iter()
                .find(|existing| matches_existing_subnet(&subnet, existing))
                .filter(|existing| {
                    let (refreshed, corrected) = refresh_matched(existing, &subnet);
                    !corrected
                        && changes(*existing, &refreshed, Subnet::LOG_IGNORED_PATHS).is_empty()
                });
            if let Some(existing) = unchanged {
                ids.push(existing.id);
                continue;
            }
            match self.create_for(subnet, authentication.clone(), None).await {
                Ok(resolved) => ids.push(resolved.id),
                Err(e) => tracing::warn!(error = ?e, "Failed to resolve interfaced subnet"),
            }
        }
        Ok(ids)
    }

    /// [`CrudService::create`], with why a person created the subnet when the create flow says so.
    /// The purpose rides on the `Created` event; a row that matched an existing one sends none.
    pub async fn create_for(
        &self,
        subnet: Subnet,
        authentication: AuthenticatedEntity,
        purpose: Option<SubnetCreatePurpose>,
    ) -> Result<Subnet, anyhow::Error> {
        // SCD2: natural-key match (CIDR + virtualization) runs against live
        // subnets only; closed historical copies must not match.
        let filter = StorableFilter::<Subnet>::new_from_site_ids(&[subnet.base.site_id]).live();
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
                    site_id = %subnet.base.site_id,
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
                    .stranded_by_narrowing(existing, *subnet.base.cidr, &all_subnets)
                    .await
                    .is_some() =>
            {
                tracing::info!(
                    site_id = %subnet.base.site_id,
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
                // SCD2 semantics: every successful natural-key match advances
                // last_seen_at, even when no field changes. Otherwise
                // unchanged subnets falsely look stale to (future) staleness
                // consumers. The incoming `subnet` was pre-stamped by
                // `HostService::discover_host` when called via discovery (see
                // `ScanContext`) so all entities in one submission share one
                // timestamp; for non-discovery callers the value is whatever
                // they put on the entity.
                let before = (
                    *existing_subnet.base.cidr,
                    existing_subnet.base.cidr.source(),
                );
                let (mut refreshed, corrected) = refresh_matched(existing_subnet, &subnet);

                self.storage.update(&mut refreshed).await?;

                // Every rescan moves last_seen_at, which subscribers don't need to hear about;
                // anything else this path changes (the type, the owner, the range) is published,
                // as host and service upserts do.
                if !changes(existing_subnet, &refreshed, Subnet::LOG_IGNORED_PATHS).is_empty() {
                    self.publish_updated(
                        existing_subnet,
                        &refreshed,
                        authentication.clone(),
                        UpdateReason::SubnetRediscovered,
                    )
                    .await;
                }

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
                    self.get_site_id(&created),
                    self.get_organization_id(&created),
                ) {
                    self.event_bus()
                        .publish(
                            Event::new(scope, EntityOperation::Created, authentication.clone())
                                .with_flags(EntityEventFlags {
                                    trigger_stale,
                                    subnet_create_purpose: purpose,
                                    ..Default::default()
                                }),
                        )
                        .await;
                }

                // A range a person records swallows the guesses inside it. Discovery leaves several
                // for a person to merge (above); this is that person.
                if created.base.source == EntitySource::Manual {
                    self.vlan_service
                        .set_manual_subnet_vlans(&created.id, &subnet.base.vlan_ids)
                        .await?;
                    self.absorb_assumed_ranges(&created, authentication).await?;
                }

                subnet
            }
        };
        Ok(subnet_from_storage)
    }
}
