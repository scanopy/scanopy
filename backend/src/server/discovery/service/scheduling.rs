//! Cron scheduler initialization and per-discovery scheduling.
use super::*;

impl DiscoveryService {
    /// Initialize scheduler with all scheduled discoveries
    pub async fn start_scheduler(self: &Arc<Self>) -> Result<()> {
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))?;

        // Clear any stale job_id mappings from previous runs
        self.job_ids.write().await.clear();

        let filter = StorableFilter::<Discovery>::new_for_scheduled_discoveries();

        let discoveries = self.discovery_storage.get_all(filter).await?;
        let count = discoveries.len();

        let mut failed_count = 0;
        for mut discovery in discoveries {
            if let Err(e) = Self::schedule_discovery(self, &discovery).await {
                tracing::error!(
                    "Failed to schedule discovery {}: {}. Disabling.",
                    discovery.id,
                    e
                );

                // Disable and save
                discovery.disable();
                let _ = self.discovery_storage.update(&mut discovery).await;
                failed_count += 1;
            }
        }

        scheduler.start().await?;

        if failed_count == 0 {
            tracing::info!(target: LOG_TARGET, "Discovery scheduler started with {} jobs", count);
        } else {
            tracing::warn!(
                target: LOG_TARGET,
                "Discovery scheduler started with {}/{} jobs. {} failed and were disabled.",
                count - failed_count,
                count,
                failed_count
            );
        }

        Ok(())
    }

    /// Schedule a single discovery
    pub(crate) async fn schedule_discovery(
        service: &Arc<DiscoveryService>,
        discovery: &Discovery,
    ) -> Result<Uuid> {
        let _ = service
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))?;

        let Some(schedule) = discovery.base.run_type.schedule() else {
            return Err(anyhow::anyhow!("Discovery is not scheduled"));
        };
        let (cron_schedule, timezone) = (schedule.cron_schedule, schedule.timezone);

        if !schedule.enabled {
            return Err(anyhow::anyhow!("Discovery is not enabled"));
        }

        let scheduler = service
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))?;

        let tz: chrono_tz::Tz = timezone.unwrap_or("UTC").parse().unwrap_or(chrono_tz::UTC);

        let discovery = discovery.clone();
        let discovery_id = discovery.id;
        let storage = service.discovery_storage.clone();

        // Clone self to use start_session
        let service_clone = Arc::clone(service);

        let job = JobBuilder::new()
            .with_timezone(tz)
            .with_cron_job_type()
            .with_schedule(cron_schedule)?
            .with_run_async(Box::new(move |_uuid, _lock| {
                let discovery = discovery.clone();
                let storage = storage.clone();
                let service = service_clone.clone();

                Box::pin(async move {
                    // Check if discovery is still enabled before running — cron jobs
                    // may linger after failed removals
                    let fresh = match storage.get_by_id(&discovery_id).await {
                        Ok(Some(fresh)) if fresh.base.run_type.is_scheduled_enabled() => fresh,
                        _ => {
                            tracing::debug!("Skipping disabled/deleted discovery {}", discovery_id);
                            return;
                        }
                    };

                    // Skip scheduled runs for Free plan orgs and for lapsed orgs
                    // (subscription ended, no paid plan chosen since) — preserves
                    // schedule config so choosing a paid plan resumes runs
                    // automatically
                    if let Ok(Some(network)) = service
                        .network_service
                        .get_by_id(&fresh.base.network_id)
                        .await
                        && service
                            .organization_service
                            .get_by_id(&network.base.organization_id)
                            .await
                            .ok()
                            .flatten()
                            .map(|o| o.is_lapsed() || o.base.plan.is_none_or(|p| p.is_free()))
                            .unwrap_or(true)
                    {
                        tracing::debug!(
                            discovery_id = %discovery_id,
                            "Skipping scheduled discovery — org is on Free plan or lapsed"
                        );
                        return;
                    }

                    // Check if daemon is reachable before starting session
                    if let Some(daemon_service) = service.daemon_service.get() {
                        match daemon_service
                            .storage()
                            .get_by_id(&fresh.base.daemon_id)
                            .await
                        {
                            Ok(Some(daemon))
                                if daemon.base.is_unreachable || daemon.base.standby =>
                            {
                                tracing::debug!(
                                    discovery_id = %discovery_id,
                                    daemon_id = %fresh.base.daemon_id,
                                    is_unreachable = daemon.base.is_unreachable,
                                    standby = daemon.base.standby,
                                    "Skipping scheduled discovery — daemon is not available"
                                );
                                return;
                            }
                            Ok(None) => {
                                tracing::debug!(
                                    discovery_id = %discovery_id,
                                    daemon_id = %fresh.base.daemon_id,
                                    "Skipping scheduled discovery — daemon not found"
                                );
                                return;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    discovery_id = %discovery_id,
                                    error = %e,
                                    "Failed to check daemon status, proceeding with discovery"
                                );
                            }
                            _ => {} // daemon is reachable, proceed
                        }
                    }

                    tracing::info!("Running scheduled discovery {}", &discovery.id);

                    match service
                        .start_session(discovery.clone(), AuthenticatedEntity::System)
                        .await
                    {
                        Ok(_) => {
                            // Reload fresh discovery from DB to avoid overwriting fields
                            match storage.get_by_id(&discovery_id).await {
                                Ok(Some(mut fresh)) => {
                                    fresh.base.run_type.set_last_run(Utc::now());
                                    if let Err(e) = storage.update(&mut fresh).await {
                                        tracing::error!("Failed to update schedule times: {}", e);
                                    }
                                }
                                _ => {
                                    tracing::warn!(
                                        "Failed to reload discovery {} for last_run update",
                                        discovery_id
                                    );
                                }
                            };
                        }
                        Err(e) => {
                            tracing::error!("Scheduled discovery {} failed: {:?}", discovery_id, e);
                        }
                    }
                })
            }))
            .build()?;

        let job_id = tokio::time::timeout(std::time::Duration::from_secs(5), scheduler.add(job))
            .await
            .map_err(|_| {
                anyhow!(
                    "Timed out adding scheduled job for discovery {}",
                    discovery_id
                )
            })?
            .map_err(|e| {
                anyhow!(
                    "Failed to add scheduled job for discovery {}: {}",
                    discovery_id,
                    e
                )
            })?;

        // Store the mapping so we can remove the job later when the schedule is updated
        service.job_ids.write().await.insert(discovery_id, job_id);

        tracing::debug!(
            "Scheduled discovery {} with job_id {} and cron: {}",
            discovery_id,
            job_id,
            cron_schedule
        );
        Ok(job_id)
    }
}
