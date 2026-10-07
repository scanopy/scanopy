//! Service upsert from discovery/API.
use super::*;

impl ServiceService {
    pub async fn upsert_service(
        &self,
        mut existing_service: Service,
        new_service_data: Service,
        authentication: AuthenticatedEntity,
    ) -> Result<Service> {
        // NOTE: This function assumes the caller already holds the service lock.
        // It's called from create() which acquires the lock before calling this.
        let mut binding_updates = 0;

        let service_before_updates = existing_service.clone();

        tracing::trace!(
            "Upserting new service data {:?} into {:?}",
            new_service_data,
            existing_service
        );

        for new_service_binding in &new_service_data.base.bindings {
            // Check if this binding is already covered by existing bindings
            // (e.g., a specific interface binding is covered by an "all ip_addresses" binding for the same port)
            let is_covered = Self::is_binding_covered_by_existing(
                new_service_binding,
                &existing_service.base.bindings,
            );

            if is_covered {
                tracing::trace!(
                    "Skipping binding {:?} - already covered by existing all-ip_addresses binding",
                    new_service_binding.base.binding_type
                );
                continue;
            }

            // Check for binding type conflicts (Interface vs Port on same ip_address)
            if let Some(conflict_msg) = Self::validate_binding_no_conflict(
                &new_service_binding.base.binding_type,
                &existing_service.base.bindings,
            ) {
                tracing::warn!(
                    "Skipping binding {:?} - conflicts with existing binding: {}",
                    new_service_binding.base.binding_type,
                    conflict_msg
                );
                continue;
            }

            // If new binding is "all ip_addresses" port binding, remove specific interface bindings for same port
            // (the all-interfaces binding supersedes them)
            if let BindingType::Port {
                port_id,
                ip_address_id: None,
            } = &new_service_binding.base.binding_type
            {
                let before_count = existing_service.base.bindings.len();
                existing_service.base.bindings.retain(|existing| {
                    // Log each comparison for debugging
                    if let BindingType::Port {
                        port_id: existing_port_id,
                        ip_address_id: existing_ip_address_id,
                    } = &existing.base.binding_type
                    {
                        let dominated =
                            existing_ip_address_id.is_some() && existing_port_id == port_id;

                        return !dominated;
                    }
                    true // Keep non-port bindings
                });
                let removed = before_count - existing_service.base.bindings.len();

                if removed > 0 {
                    binding_updates += removed;
                }
            }

            if !existing_service.base.bindings.contains(new_service_binding) {
                binding_updates += 1;
                existing_service.base.bindings.push(*new_service_binding);
            }
        }

        // A service never runs inside itself. The server's container safety net once copied an API
        // proxy's container identity onto the runtime the proxy fronts, leaving the runtime owned
        // by itself. Such a stored row is cleared here, and incoming container details naming the
        // service as its own owner are not taken, since that identity is another container's.
        if existing_service.base.virtualization_service_id == Some(existing_service.id) {
            tracing::warn!(
                service_id = %existing_service.id,
                service_name = %existing_service.base.name,
                "Clearing container details that name the service as its own owner"
            );
            existing_service.base.virtualization_service_id = None;
            existing_service.base.virtualization_metadata = None;
        }
        if new_service_data.base.virtualization_service_id == Some(existing_service.id) {
            tracing::warn!(
                service_id = %existing_service.id,
                service_name = %existing_service.base.name,
                "Ignoring container details whose owner is the service itself"
            );
        } else {
            if let Some(virtualization_metadata) = &new_service_data.base.virtualization_metadata {
                existing_service.base.virtualization_metadata =
                    Some(virtualization_metadata.clone())
            }
            if let Some(owner) = new_service_data.base.virtualization_service_id {
                existing_service.base.virtualization_service_id = Some(owner);
            }
        }

        existing_service.base.source = match (
            existing_service.base.source,
            new_service_data.base.source.clone(),
        ) {
            // Both DiscoveryWithMatch: keep highest confidence and the better
            // match reason. Discovery metadata (date/daemon/discovery_type) used
            // to be appended here; that's now tracked via FK on the entity row
            // (last_discovery_id / first_discovery_id) post-terminal.
            (
                EntitySource::DiscoveryWithMatch {
                    details: existing_service_details,
                },
                EntitySource::DiscoveryWithMatch {
                    details: new_service_details,
                },
            ) => {
                let confidence = existing_service_details
                    .confidence
                    .max(new_service_details.confidence);
                let reason = if new_service_details.confidence > existing_service_details.confidence
                {
                    new_service_details.reason
                } else {
                    existing_service_details.reason
                };
                EntitySource::DiscoveryWithMatch {
                    details: MatchDetails { confidence, reason },
                }
            }

            // New service data upserted to a manually or system-created record
            (_, EntitySource::DiscoveryWithMatch { details }) => {
                EntitySource::DiscoveryWithMatch { details }
            }

            // Shouldn't happen during normal discovery; keep existing source.
            (existing_source, _) => existing_source,
        };

        // SCD2 freshness: advance last_seen_at to this scan's time on every match, even
        // when no binding/field changed — otherwise a re-discovered service looks stale
        // while the FK subscriber still bumps last_discovery_id. Mirrors upsert_host.
        // The incoming new_service_data was pre-stamped with scan_time by the discovery
        // handler. The unconditional update below persists it; the Updated event stays
        // gated on real binding changes, so a pure freshness bump wakes no consumers.
        existing_service.last_seen_at = new_service_data.last_seen_at;

        self.storage.update(&mut existing_service).await?;

        // Save bindings to separate table with correct service_id and site_id
        let bindings_with_ids: Vec<Binding> = existing_service
            .base
            .bindings
            .iter()
            .cloned()
            .map(|b| b.with_service(existing_service.id, existing_service.base.site_id))
            .collect();

        let saved_bindings = self
            .binding_service
            .save_for_parent(
                &existing_service.id,
                &bindings_with_ids,
                authentication.clone(),
            )
            .await?;

        // Update service with the saved bindings (which have actual IDs and preserved created_at)
        existing_service.base.bindings = saved_bindings;

        let mut data = Vec::new();

        if binding_updates > 0 {
            data.push(format!("{} bindings", binding_updates))
        };

        if !data.is_empty() {
            let trigger_stale = existing_service.triggers_staleness(Some(service_before_updates));

            if let Some(scope) = EntityScope::from_ids(
                existing_service.id,
                existing_service.clone().into(),
                self.get_site_id(&existing_service),
                self.get_organization_id(&existing_service),
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
        } else {
            tracing::debug!(
                service_id = %existing_service.id,
                "Service upsert - no binding changes needed"
            );
        }

        Ok(existing_service)
    }
}
