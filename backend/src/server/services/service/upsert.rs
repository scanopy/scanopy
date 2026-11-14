//! Service upsert from discovery/API.
use super::*;
use crate::server::shared::relations as R;

impl ServiceService {
    pub async fn upsert_service(
        &self,
        mut existing_service: Service,
        new_service_data: Service,
        authentication: AuthenticatedEntity,
    ) -> Result<Service> {
        // NOTE: This function assumes the caller already holds the service lock.
        // It's called from create() which acquires the lock before calling this.
        let service_before_updates = existing_service.clone();
        let mut reason = UpdateReason::EntityRediscovered;

        let binding_updates = Self::merge_rediscovered_bindings(
            &mut existing_service.base.bindings,
            &new_service_data.base.bindings,
        );

        // A service never runs inside itself. The server's container safety net once copied an API
        // proxy's container identity onto the runtime the proxy fronts, leaving the runtime owned
        // by itself. Such a stored row is cleared here, and incoming container details naming the
        // service as its own owner are not taken, since that identity is another container's.
        // Either names the update's reason, which Service's log writes at WARN.
        if existing_service.base.virtualization_service_id == Some(existing_service.id) {
            existing_service.base.virtualization_service_id = None;
            existing_service.base.virtualization_metadata = None;
            reason = UpdateReason::ServiceContainerDetailsRejected;
        }
        if new_service_data.base.virtualization_service_id == Some(existing_service.id) {
            reason = UpdateReason::ServiceContainerDetailsRejected;
        } else {
            if let Some(virtualization_metadata) = &new_service_data.base.virtualization_metadata {
                existing_service.base.virtualization_metadata =
                    Some(virtualization_metadata.clone())
            }
            if let Some(owner) = new_service_data.base.virtualization_service_id {
                existing_service.base.virtualization_service_id = Some(owner);
            }
        }
        // A container that stopped, or moved runtime, is a change even with its bindings as they
        // were: the open page and the topology need to hear it.
        let virtualization_changed = existing_service.base.virtualization_metadata
            != service_before_updates.base.virtualization_metadata
            || existing_service.base.virtualization_service_id
                != service_before_updates.base.virtualization_service_id;

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

            // A person or the system created this record, and discovery matching it does not make
            // it discovery's: a manual or system service stays exempt from staleness and is never
            // released when its port goes quiet or changes hands.
            (existing_source @ (EntitySource::Manual | EntitySource::System), _) => existing_source,

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
            .sync_for(
                &R::BINDING_SERVICE.at(existing_service.id),
                &bindings_with_ids,
                authentication.clone(),
                // The only bindings an upsert removes are specific ones an all-addresses binding
                // now covers.
                DeletionReason::EntityNoLongerReported,
                UpdateReason::EntityRediscovered,
            )
            .await?;

        // Update service with the saved bindings (which have actual IDs and preserved created_at)
        existing_service.base.bindings = saved_bindings;

        // A rejection publishes even when nothing else changed, so its WARN line is written.
        if binding_updates > 0
            || virtualization_changed
            || reason == UpdateReason::ServiceContainerDetailsRejected
        {
            self.publish_updated(
                &service_before_updates,
                &existing_service,
                authentication,
                reason,
            )
            .await;
        }

        Ok(existing_service)
    }

    /// Keep only the scanned services and bindings on `site_id`.
    /// See `HostService::retain_scanned_on_site`.
    pub async fn retain_scanned_on_site(
        &self,
        scanned: &mut crate::server::daemons::r#impl::api::ScannedEntityIds,
        site_id: Uuid,
    ) -> Result<()> {
        self.retain_on_site(&mut scanned.service_ids, site_id)
            .await?;
        self.binding_service
            .retain_on_site(&mut scanned.binding_ids, site_id)
            .await?;
        Ok(())
    }

    /// Point the discovery FKs of the services and bindings a finished session scanned at
    /// `discovery_id`. Called from `HostService::stamp_discovery`. Both types are written even
    /// when the first fails.
    pub async fn stamp_discovery(
        &self,
        scanned: &crate::server::daemons::r#impl::api::ScannedEntityIds,
        discovery_id: Uuid,
    ) -> Result<()> {
        use crate::server::shared::services::traits::DiscoveryFkUpdater;

        let services = <Self as DiscoveryFkUpdater<Service>>::update_discovery_fks(
            self,
            scanned,
            discovery_id,
        )
        .await;
        let bindings = <BindingService as DiscoveryFkUpdater<Binding>>::update_discovery_fks(
            &self.binding_service,
            scanned,
            discovery_id,
        )
        .await;
        match (services, bindings) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), Ok(())) => Err(e.context("services")),
            (Ok(()), Err(e)) => Err(e.context("bindings")),
            (Err(s), Err(b)) => Err(anyhow!("services: {s:#}; bindings: {b:#}")),
        }
    }
}
