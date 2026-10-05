//! Host deletion.
use super::*;

impl HostService {
    /// Delete a host (children cascade via FK)
    pub async fn delete_host(&self, id: &Uuid, authentication: AuthenticatedEntity) -> Result<()> {
        let lock_guard = self
            .storage()
            .session_lock(LockKey::Host(*id), DEFAULT_LOCK_TIMEOUT)
            .await?;
        self.delete_host_inner(id, authentication).await?;
        lock_guard.release().await?;
        Ok(())
    }

    /// Deletion body without the `Host(id)` lock. Callers that already hold
    /// the host's lock (consolidation) use this directly — re-acquiring on a
    /// second connection would self-deadlock.
    pub(crate) async fn delete_host_inner(
        &self,
        id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        // Can't delete host with daemon
        if self
            .daemon_service
            .exists(StorableFilter::<Daemon>::new_from_host_ids(&[*id]))
            .await?
        {
            return Err(ValidationError::new(
                "Can't delete a host with an associated daemon. Delete the daemon first.",
            )
            .into());
        }

        let host = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Host {} not found", id))?;

        // Remove tags from junction table
        if let Some(tag_service) = self.entity_tag_service() {
            tag_service
                .remove_all_for_entity(*id, EntityDiscriminants::Host)
                .await?;
        }

        // Delete host - children cascade via ON DELETE CASCADE
        self.storage().delete(id).await?;

        let trigger_stale = host.triggers_staleness(None);

        if let Some(scope) = EntityScope::from_ids(
            host.id(),
            host.clone().into(),
            self.get_site_id(&host),
            self.get_organization_id(&host),
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
