//! Discovery creation and deletion (with scheduler wiring).
use super::*;

impl DiscoveryService {
    /// Validate timezone string if present on a scheduled discovery
    pub(crate) fn validate_timezone(run_type: &RunType) -> Result<()> {
        if let Some(tz) = run_type.schedule().and_then(|s| s.timezone)
            && tz.parse::<chrono_tz::Tz>().is_err()
        {
            bail_validation!(
                "Invalid timezone '{}'. Use an IANA timezone like 'America/New_York'.",
                tz
            );
        }
        Ok(())
    }

    /// Create a new scheduled discovery
    pub async fn create_discovery(
        self: &Arc<Self>,
        discovery: Discovery,
        authentication: AuthenticatedEntity,
    ) -> Result<Discovery> {
        Self::validate_timezone(&discovery.base.run_type)?;
        let mut created_discovery = if discovery.id == Uuid::nil() {
            self.discovery_storage
                .create(&Discovery::new(discovery.base))
                .await?
        } else {
            self.discovery_storage.create(&discovery).await?
        };

        // Save tags to junction table
        if let Some(entity_tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
        {
            entity_tag_service
                .set_tags(
                    created_discovery.id,
                    EntityDiscriminants::Discovery,
                    created_discovery.base.tags.clone(),
                    org_id,
                )
                .await?;
        }

        // If it's a scheduled discovery, add it to the scheduler
        if created_discovery.base.run_type.schedule().is_some()
            && let Err(e) = Self::schedule_discovery(self, &created_discovery).await
        {
            // Disable and save to DB
            created_discovery.disable();
            let disabled_discovery = self
                .discovery_storage
                .update(&mut created_discovery)
                .await?;

            tracing::error!(
                "Failed to schedule discovery {}. Discovery created but disabled. Error: {}",
                disabled_discovery.id,
                e
            );

            return Ok(disabled_discovery);
        }

        let trigger_stale = created_discovery.triggers_staleness(None);

        if let Some(scope) = EntityScope::from_ids(
            created_discovery.id(),
            created_discovery.clone().into(),
            self.get_site_id(&created_discovery),
            self.get_organization_id(&created_discovery),
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

        Ok(created_discovery)
    }

    /// Delete group
    pub async fn delete_discovery(
        self: &Arc<Self>,
        id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let discovery = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Discovery not found"))?;

        // If it's scheduled, remove from scheduler first (with timeout to prevent deadlock)
        if discovery.base.run_type.schedule().is_some() {
            self.remove_scheduled_job(id).await;
            tracing::debug!("Removed scheduled job for discovery {}", id);
        }

        // Remove tags from junction table
        if let Some(tag_service) = self.entity_tag_service() {
            tag_service
                .remove_all_for_entity(*id, EntityDiscriminants::Discovery)
                .await?;
        }

        self.discovery_storage.delete(id).await?;

        let trigger_stale = discovery.triggers_staleness(None);

        if let Some(scope) = EntityScope::from_ids(
            discovery.id(),
            discovery.clone().into(),
            self.get_site_id(&discovery),
            self.get_organization_id(&discovery),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Deleted, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }
        Ok(())
    }
}
