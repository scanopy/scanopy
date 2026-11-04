//! Inactivity standby detection, daemon alert notifications, and loopback credential scoping.
use super::*;
use crate::server::daemons::r#impl::version::tags_log_lines_with_session;
use crate::server::email::messages::DaemonScansFailing;
use crate::server::shared::types::metadata::TypeMetadataProvider;

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
        let cutoff = crate::server::shared::clock::now() - chrono::Duration::days(30);

        // Get all non-standby daemons created more than 30 days ago
        let active_daemons = self
            .get_all(StorableFilter::<Daemon>::new_for_active_daemons().created_before(cutoff))
            .await?;

        for mut daemon in active_daemons {
            // Skip daemons still within their post-reactivation grace window
            // so a freshly-restarted daemon has time for a scheduled
            // discovery to complete before inactivity is re-evaluated.
            if is_within_standby_grace(
                daemon.base.standby_cleared_at,
                crate::server::shared::clock::now(),
            ) {
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
                self.update(
                    &mut daemon,
                    AuthenticatedEntity::System,
                    UpdateReason::DaemonStandbyChanged,
                )
                .await?;

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
        // The daemon is still put on standby when nobody is emailed.
        let Some((site_name, recipients)) =
            self.daemon_alert_recipients(daemon, email_service).await?
        else {
            return Ok(());
        };
        for to in recipients {
            email_service
                .send_daemon_standby_email(to, &daemon.base.name, &site_name)
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
        // The daemon is still marked unreachable when nobody is emailed.
        let Some((site_name, recipients)) =
            self.daemon_alert_recipients(daemon, email_service).await?
        else {
            return Ok(());
        };
        for to in recipients {
            email_service
                .send_daemon_unreachable_email(to, &daemon.base.name, &site_name)
                .await?;
        }
        Ok(())
    }

    /// Email the daemon's alert recipients when the failed run recorded for `session_id` makes
    /// [`FAILURE_STREAK_EMAIL_RUNS`] failed runs in a row on the daemon, of any discoveries. Sent
    /// once per streak: a later failure in the same streak sends nothing, and only a run that does
    /// not fail starts a new one.
    pub(crate) async fn send_failure_streak_notification(
        &self,
        session_id: Uuid,
        daemon_id: Uuid,
        email_service: &EmailService,
    ) -> Result<()> {
        let Some(runs) = self
            .discovery_service
            .failure_streak(daemon_id, session_id, FAILURE_STREAK_EMAIL_RUNS)
            .await?
        else {
            return Ok(());
        };
        let Some(newest) = runs.first() else {
            return Ok(());
        };
        let Some(reason) = newest
            .base
            .run_type
            .historical_results()
            .and_then(|results| results.reason)
        else {
            return Ok(());
        };
        let Some(daemon) = self.get_by_id(&daemon_id).await? else {
            return Ok(());
        };
        let Some((site_name, recipients)) =
            self.daemon_alert_recipients(&daemon, email_service).await?
        else {
            return Ok(());
        };

        let mut discovery_names: Vec<&str> = Vec::new();
        for run in &runs {
            if !discovery_names.contains(&run.base.name.as_str()) {
                discovery_names.push(&run.base.name);
            }
        }

        // As the run view decides it: the reason must be one the daemon's logs explain, the run's
        // own daemon version must have tagged its lines with the session, and the daemon must have
        // said which OS it runs on, which picks the command.
        let log_command = newest
            .base
            .run_type
            .historical_results()
            .filter(|results| {
                reason.explained_by_daemon_logs()
                    && tags_log_lines_with_session(
                        results
                            .daemon_version
                            .as_deref()
                            .and_then(|v| Version::parse(v).ok())
                            .as_ref(),
                    )
            })
            .zip(daemon.base.os)
            .map(|(results, os)| os.log_command(&results.session_id.to_string()));

        let email = DaemonScansFailing {
            daemon_name: &daemon.base.name,
            site_name: &site_name,
            failed_runs: runs.len(),
            discovery_names: &discovery_names,
            reason_name: reason.name(),
            reason_description: reason.description(),
            log_command: log_command.as_deref(),
        };
        for to in recipients {
            email_service
                .send_daemon_scans_failing_email(to, &email)
                .await?;
        }
        Ok(())
    }

    /// Who is emailed a daemon's alerts, with the name of its site: the organization's owner, and
    /// the user who installed the daemon when that is someone else.
    ///
    /// `None` for an organization that moved from cloud to a self-hosted plan: the daemons it left
    /// on the cloud org are expected to go quiet, and every alert's link leads into an app it is
    /// locked out of.
    pub(crate) async fn daemon_alert_recipients(
        &self,
        daemon: &Daemon,
        email_service: &EmailService,
    ) -> Result<Option<(String, Vec<email_address::EmailAddress>)>> {
        let site = self
            .site_service
            .get_by_id(&daemon.base.site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Site not found"))?;
        let org_id = site.base.organization_id;

        if email_service
            .organization_self_hosted_plan_locked(&org_id)
            .await?
        {
            return Ok(None);
        }

        let owners = email_service
            .user_service
            .get_organization_owners(&org_id)
            .await?;
        let owner = owners
            .first()
            .ok_or_else(|| anyhow::anyhow!("No owner found for organization {}", org_id))?;
        let mut recipients = vec![owner.base.email.clone()];

        if daemon.base.user_id != owner.id
            && let Some(user) = email_service
                .user_service
                .get_by_id(&daemon.base.user_id)
                .await?
        {
            recipients.push(user.base.email);
        }

        Ok(Some((site.base.name, recipients)))
    }
}

/// How many failed runs in a row on one daemon send its recipients an email.
const FAILURE_STREAK_EMAIL_RUNS: usize = 3;

/// How recently a daemon must have been seen to count in `scanopy_daemons`.
const VERSION_GAUGE_WINDOW: chrono::Duration = chrono::Duration::hours(24);

impl DaemonService {
    /// Sets `scanopy_daemons{daemon_version}` from every daemon seen in the last day. `previous`
    /// holds the labels the last pass set; a label this pass no longer has is set to 0, so a
    /// version that leaves the fleet stops reading its last count.
    pub async fn record_daemon_versions(
        &self,
        previous: &mut std::collections::BTreeSet<String>,
    ) -> Result<()> {
        let daemons = self.get_all(StorableFilter::<Daemon>::new()).await?;
        let counts = daemon_version_counts(&daemons, crate::server::shared::clock::now());
        for (label, count) in daemon_version_gauge_values(previous, &counts) {
            metrics::gauge!("scanopy_daemons", "daemon_version" => label).set(count as f64);
        }
        *previous = counts.into_keys().collect();
        Ok(())
    }
}

/// Daemons seen within `VERSION_GAUGE_WINDOW`, counted by the exact version each last reported:
/// `none` when it never reported one, `unknown` when it is beyond this server's release line.
/// Patch-level, since sunset ceilings are patch-level.
pub(crate) fn daemon_version_counts(
    daemons: &[Daemon],
    now: DateTime<Utc>,
) -> std::collections::BTreeMap<String, u64> {
    use crate::server::daemons::r#impl::version::is_beyond_server_release_line;
    let mut counts = std::collections::BTreeMap::new();
    for daemon in daemons.iter().filter(|d| {
        d.base
            .last_seen
            .is_some_and(|seen| now - seen <= VERSION_GAUGE_WINDOW)
    }) {
        let label = match &daemon.base.version {
            None => "none".to_string(),
            Some(v) if is_beyond_server_release_line(v) => "unknown".to_string(),
            Some(v) => v.to_string(),
        };
        *counts.entry(label).or_insert(0) += 1;
    }
    counts
}

/// The values to set this pass: every current count, plus 0 for each label set last pass that
/// has none now.
fn daemon_version_gauge_values(
    previous: &std::collections::BTreeSet<String>,
    counts: &std::collections::BTreeMap<String, u64>,
) -> Vec<(String, u64)> {
    previous
        .iter()
        .filter(|label| !counts.contains_key(*label))
        .map(|label| (label.clone(), 0))
        .chain(counts.iter().map(|(label, count)| (label.clone(), *count)))
        .collect()
}

#[cfg(test)]
mod version_gauge_tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    fn seen(version: Option<&str>, hours_ago: i64, now: DateTime<Utc>) -> Daemon {
        let mut d = crate::tests::daemon(&Uuid::new_v4(), &Uuid::new_v4());
        d.base.version = version.map(|v| Version::parse(v).unwrap());
        d.base.last_seen = Some(now - chrono::Duration::hours(hours_ago));
        d
    }

    #[test]
    fn counts_recent_daemons_by_exact_version() {
        let now = crate::server::shared::clock::now();
        let daemons = vec![
            seen(Some("0.17.4"), 1, now),
            seen(Some("0.17.4"), 2, now),
            seen(Some("0.17.5"), 1, now),
            seen(None, 1, now),
            seen(Some("999.0.0"), 1, now),
            seen(Some("0.14.19"), 25, now),
        ];
        let counts = daemon_version_counts(&daemons, now);
        assert_eq!(
            counts,
            BTreeMap::from([
                ("0.17.4".to_string(), 2),
                ("0.17.5".to_string(), 1),
                ("none".to_string(), 1),
                ("unknown".to_string(), 1),
            ])
        );
    }

    #[test]
    fn a_version_gone_from_the_fleet_is_set_to_zero() {
        let previous = BTreeSet::from(["0.16.2".to_string(), "0.17.4".to_string()]);
        let counts = BTreeMap::from([("0.17.4".to_string(), 3)]);
        let mut values = daemon_version_gauge_values(&previous, &counts);
        values.sort();
        assert_eq!(
            values,
            vec![("0.16.2".to_string(), 0), ("0.17.4".to_string(), 3)]
        );
    }
}
