//! Inactivity standby detection, notifications, and loopback credential scoping.
use super::*;

impl DaemonService {
    // ========================================================================
    // Inactivity standby
    // ========================================================================

    /// Check all active daemons for inactivity (no completed discovery in 30 days)
    /// and put them on standby, sending notification emails if email service is available.
    pub async fn check_daemon_inactivity(
        &self,
        email_service: Option<&EmailService>,
    ) -> Result<()> {
        let cutoff = Utc::now() - chrono::Duration::days(30);

        // Get all non-standby daemons created more than 30 days ago
        let active_daemons = self
            .get_all(StorableFilter::<Daemon>::new_for_active_daemons().created_before(cutoff))
            .await?;

        for mut daemon in active_daemons {
            // Skip daemons still within their post-reactivation grace window
            // so a freshly-restarted daemon has time for a scheduled
            // discovery to complete before inactivity is re-evaluated.
            if is_within_standby_grace(daemon.base.standby_cleared_at, Utc::now()) {
                tracing::debug!(
                    daemon_id = %daemon.id,
                    cleared_at = ?daemon.base.standby_cleared_at,
                    "Skipping inactivity check (within standby-clear grace)"
                );
                continue;
            }

            // Check for historical discoveries completed by this daemon
            let filter = StorableFilter::<Discovery>::new_from_uuid_column("daemon_id", &daemon.id)
                .historical_discovery();
            let discoveries = self.discovery_service.get_all(filter).await?;

            // Find the most recent finished_at from Historical discoveries
            let last_finished = discoveries
                .iter()
                .filter_map(|d| d.base.run_type.historical_results()?.finished_at)
                .max();

            let should_standby = match last_finished {
                Some(finished) => finished < cutoff,
                None => true, // No historical records at all
            };

            if should_standby {
                daemon.base.standby = true;
                self.update(&mut daemon, AuthenticatedEntity::System)
                    .await?;
                tracing::info!(
                    daemon_id = %daemon.id,
                    "Set daemon to standby (inactive for 30+ days)"
                );

                // Send notification emails if email service is available
                if let Some(email_service) = email_service
                    && let Err(e) = self.send_standby_notification(&daemon, email_service).await
                {
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        error = %e,
                        "Failed to send daemon standby notification email"
                    );
                }
            }
        }

        Ok(())
    }

    /// Send standby notification email to org owner and daemon installer
    async fn send_standby_notification(
        &self,
        daemon: &Daemon,
        email_service: &EmailService,
    ) -> Result<()> {
        let network = self
            .network_service
            .get_by_id(&daemon.base.network_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network not found"))?;

        let org_id = network.base.organization_id;
        let network_name = &network.base.name;
        let daemon_name = &daemon.base.name;

        // Daemons left on a cloud org that moved to a self-hosted plan are
        // expected to go quiet, and the email's link leads into an app the org
        // is locked out of. The daemon is still put on standby.
        if email_service
            .organization_self_hosted_plan_locked(&org_id)
            .await?
        {
            return Ok(());
        }

        // Send to org owner
        let owners = email_service
            .user_service
            .get_organization_owners(&org_id)
            .await?;
        let owner = owners
            .first()
            .ok_or_else(|| anyhow::anyhow!("No owner found for organization {}", org_id))?;
        email_service
            .send_daemon_standby_email(owner.base.email.clone(), daemon_name, network_name)
            .await?;

        // Also send to daemon installer if different from owner
        if daemon.base.user_id != owner.id
            && let Some(user) = email_service
                .user_service
                .get_by_id(&daemon.base.user_id)
                .await?
        {
            email_service
                .send_daemon_standby_email(user.base.email, daemon_name, network_name)
                .await?;
        }

        Ok(())
    }

    /// Send unreachable notification email to org owner and daemon installer
    pub(crate) async fn send_unreachable_notification(
        &self,
        daemon: &Daemon,
        email_service: &EmailService,
    ) -> Result<()> {
        let network = self
            .network_service
            .get_by_id(&daemon.base.network_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network not found"))?;

        let org_id = network.base.organization_id;
        let network_name = &network.base.name;
        let daemon_name = &daemon.base.name;

        // Same skip as the standby notification. The daemon is still marked
        // unreachable.
        if email_service
            .organization_self_hosted_plan_locked(&org_id)
            .await?
        {
            return Ok(());
        }

        // Send to org owner
        let owners = email_service
            .user_service
            .get_organization_owners(&org_id)
            .await?;
        let owner = owners
            .first()
            .ok_or_else(|| anyhow::anyhow!("No owner found for organization {}", org_id))?;
        email_service
            .send_daemon_unreachable_email(owner.base.email.clone(), daemon_name, network_name)
            .await?;

        // Also send to daemon installer if different from owner
        if daemon.base.user_id != owner.id
            && let Some(user) = email_service
                .user_service
                .get_by_id(&daemon.base.user_id)
                .await?
        {
            email_service
                .send_daemon_unreachable_email(user.base.email, daemon_name, network_name)
                .await?;
        }

        Ok(())
    }
}
