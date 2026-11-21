//! Discovery creation and deletion (with scheduler wiring).
use super::*;
use crate::server::shared::events::types::DeletionReason;

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

    /// Refuse a schedule the scheduler can't parse, read the way `schedule_discovery` reads it.
    /// Saved, it would be disabled at the next server start with only a log line to say so.
    pub(crate) fn validate_schedule(run_type: &RunType) -> Result<()> {
        if let Some(schedule) = run_type.schedule()
            && JobBuilder::new()
                .with_cron_job_type()
                .with_schedule(schedule.cron_schedule)
                .is_err()
        {
            bail_validation!(
                "Invalid schedule '{}'. Use a cron expression with seconds first, like '0 0 2 * * *'.",
                schedule.cron_schedule
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
        Self::validate_schedule(&discovery.base.run_type)?;
        let mut created_discovery = if discovery.id == Uuid::nil() {
            self.discovery_storage
                .create(&Discovery::new(discovery.base))
                .await?
        } else {
            self.discovery_storage.create(&discovery).await?
        };
        // Storage returns the entity as given, so the computed field is set here.
        created_discovery.next_run_is_full_scan = created_discovery.full_scan_is_due();
        self.save_subnet_targets(&created_discovery).await?;

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

        let trigger_stale = created_discovery.triggers_staleness(None);

        if let Some(scope) = EntityScope::from_ids(
            created_discovery.id(),
            created_discovery.clone().into(),
            self.get_site_id(&created_discovery),
            self.get_organization_id(&created_discovery),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Created, authentication.clone()).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await;
        }

        // If it's a scheduled discovery, add it to the scheduler. One that can't be scheduled is
        // disabled, and its update names the failure, so the log says why it won't run.
        if created_discovery.base.run_type.schedule().is_some()
            && let Err(e) = Self::schedule_discovery(self, &created_discovery).await
        {
            created_discovery.disable();
            return self
                .update_base(
                    &mut created_discovery,
                    authentication,
                    UpdateReason::DiscoveryScheduleFailed {
                        error: e.to_string(),
                    },
                )
                .await;
        }

        Ok(created_discovery)
    }

    /// Delete group
    pub async fn delete_discovery(
        self: &Arc<Self>,
        id: &Uuid,
        authentication: AuthenticatedEntity,
        reason: DeletionReason,
    ) -> Result<(), Error> {
        let discovery = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Discovery not found"))?;

        // If it's scheduled, remove from scheduler first (with timeout to prevent deadlock)
        if discovery.base.run_type.schedule().is_some() {
            self.remove_scheduled_job(id).await;
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
                    Event::new(scope, EntityOperation::Deleted { reason }, authentication)
                        .with_flags(EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        }),
                )
                .await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduled(cron: &str) -> RunType {
        RunType::Scheduled {
            cron_schedule: cron.to_string(),
            last_run: None,
            enabled: true,
            timezone: Some("Europe/London".to_string()),
        }
    }

    /// The API takes what the scheduler can run, and refuses what it would disable at boot.
    #[test]
    fn a_schedule_the_scheduler_cannot_parse_is_refused() {
        assert!(DiscoveryService::validate_schedule(&scheduled("0 0 2 * * *")).is_ok());
        // The scheduler reads six fields, seconds first.
        assert!(DiscoveryService::validate_schedule(&scheduled("0 2 * * *")).is_err());
        assert!(DiscoveryService::validate_schedule(&scheduled("every night")).is_err());
        assert!(DiscoveryService::validate_schedule(&RunType::default()).is_ok());
    }
}
