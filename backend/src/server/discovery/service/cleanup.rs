//! Old/stalled session cleanup and scheduled-job removal.
use super::state::{self, SessionMaps};
use super::*;

impl DiscoveryService {
    pub async fn cleanup_old_sessions(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut sessions = self.sessions.write().await;
        let mut daemon_sessions = self.daemon_sessions.write().await;
        let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;
        let mut discovery_sessions = self.discovery_sessions.write().await;

        let mut to_remove = Vec::new();
        for (session_id, session) in sessions.iter() {
            if let Some(finished_at) = session.finished_at
                && finished_at < cutoff
            {
                to_remove.push(*session_id);
            }
        }

        for session_id in to_remove {
            if let Some(session) = sessions.remove(&session_id) {
                daemon_pull_cancellations.remove(&session.daemon_id);

                if let Some(daemon_sessions) = daemon_sessions.get_mut(&session.daemon_id) {
                    daemon_sessions.retain(|s| *s != session.session_id);
                }

                discovery_sessions.retain(|_, sid| *sid != session_id);

                tracing::debug!("Cleaned up old discovery session {}", session_id);
            }
        }
    }

    /// Delete transient rescan discoveries whose session never finished — a
    /// server restart mid-rescan, or a failed delete at terminal.
    ///
    /// Left in place, these read as live discovery configurations to
    /// `exclude_ephemeral`'s call sites and clutter the daemon's row set.
    /// `older_than_hours` must exceed the longest a session can legitimately
    /// live (a queued rescan waits on `max_discovery_duration`, 6h by default).
    pub async fn sweep_orphaned_rescans(&self, older_than_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(older_than_hours);
        let filter = StorableFilter::<Discovery>::new()
            .rescan_discovery()
            .updated_before(cutoff);

        let orphaned = match self.discovery_storage.get_all(filter).await {
            Ok(discoveries) => discoveries,
            Err(e) => {
                tracing::warn!(error = ?e, "Failed to scan for orphaned rescan discoveries");
                return;
            }
        };

        let active = self.discovery_sessions.read().await;
        let ids: Vec<Uuid> = orphaned
            .iter()
            .filter(|d| !active.contains_key(&d.id))
            .map(|d| d.id)
            .collect();
        drop(active);

        if ids.is_empty() {
            return;
        }

        tracing::info!(count = ids.len(), "Sweeping orphaned rescan discoveries");
        for id in ids {
            if let Err(e) = self.discovery_storage.delete(&id).await {
                tracing::warn!(discovery_id = %id, error = ?e, "Failed to delete orphaned rescan discovery");
            }
        }
    }

    /// Cleanup stalled sessions (called periodically from background task)
    pub async fn cleanup_stalled_sessions(&self) {
        let now = Utc::now();
        let stall_threshold = chrono::Duration::minutes(5);

        // First pass: identify stalled sessions (read locks only)
        let stalled_sessions: Vec<DiscoveryUpdatePayload> = {
            let sessions = self.sessions.read().await;
            let last_updated = self.session_last_updated.read().await;
            state::select_stalled(&sessions, &last_updated, now, stall_threshold)
        };

        if stalled_sessions.is_empty() {
            let sessions = self.sessions.read().await;
            let mut last_updated = self.session_last_updated.write().await;
            state::evict_tombstones(&sessions, &mut last_updated, now, stall_threshold);
            return;
        }

        // Second pass: request cancellation for stalled sessions (no locks held)
        // We do BOTH actions to support both daemon modes:
        // 1. Publish DiscoveryCancelled event - DaemonService subscriber handles ServerPoll mode
        // 2. Set cancellation flag - DaemonPoll mode checks on next poll via request_work
        for session in &stalled_sessions {
            let daemon_id = session.daemon_id;
            let session_id = session.session_id;

            tracing::warn!(
                session_id = %session_id,
                daemon_id = %daemon_id,
                "Requesting cancellation for stalled session"
            );

            let discovery_id = self.lookup_discovery_id(&session_id).await;
            let cancelled_update = DiscoveryUpdatePayload {
                session_id,
                network_id: session.network_id,
                daemon_id,
                phase: DiscoveryPhase::Cancelled,
                progress: session.progress,
                error: None,
                warnings: Vec::new(),
                started_at: session.started_at,
                finished_at: Some(Utc::now()),
                discovery_type: session.discovery_type.clone(),
                hosts_discovered: None,
                estimated_remaining_secs: None,
                discovery_id,
                scanned: None,
                reason: Some(DiscoveryTerminalReason::StalledNoUpdates),
                last_update_at: None,
                daemon_version: None,
            };

            if let Err(e) = self
                .event_bus()
                .publish(cancelled_update.into_discovery_event())
                .await
            {
                tracing::warn!(
                    daemon_id = %session.daemon_id,
                    session_id = %session.session_id,
                    error = %e,
                    "Failed to publish cancellation event for stalled session"
                );
            }

            // Set cancellation flag for DaemonPoll mode (checked on next poll)
            self.daemon_pull_cancellations
                .write()
                .await
                .insert(daemon_id, (true, session_id));

            tracing::info!(
                daemon_id = %daemon_id,
                session_id = %session_id,
                "Cancellation requested for stalled session"
            );
        }

        // Third pass: cleanup session state (write locks, no await while they are held)
        let reaped = {
            let mut sessions = self.sessions.write().await;
            let mut last_updated = self.session_last_updated.write().await;
            let mut daemon_sessions = self.daemon_sessions.write().await;
            let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;
            let mut discovery_sessions = self.discovery_sessions.write().await;

            let stalled_ids: Vec<Uuid> = stalled_sessions.iter().map(|s| s.session_id).collect();
            let reaped = state::reap(
                &mut SessionMaps {
                    sessions: &mut sessions,
                    last_updated: &mut last_updated,
                    daemon_sessions: &mut daemon_sessions,
                    discovery_sessions: &mut discovery_sessions,
                    pull_cancellations: &mut daemon_pull_cancellations,
                },
                &stalled_ids,
                now,
            );
            for reaped_session in &reaped {
                let _ = self.update_tx.send(reaped_session.session.clone());
            }
            state::evict_tombstones(&sessions, &mut last_updated, now, stall_threshold);
            reaped
        };

        if reaped.is_empty() {
            return;
        }
        let reaped_count = reaped.len();

        // Storage writes run on their own task, for the same reason as `update_session`'s: the
        // sessions have already left the maps, so an interrupted write would lose their record.
        match self.self_ref.upgrade() {
            Some(this) => {
                if let Err(e) =
                    tokio::spawn(async move { this.finish_reaped(reaped, now).await }).await
                {
                    tracing::error!(error = %e, "Recording reaped discovery sessions panicked");
                }
            }
            None => self.finish_reaped(reaped, now).await,
        }

        tracing::info!(count = reaped_count, "Reaped stalled discovery sessions");
    }

    /// The storage and event work for reaped sessions. Runs with no session lock held.
    async fn finish_reaped(&self, reaped: Vec<state::ReapedSession>, now: chrono::DateTime<Utc>) {
        for state::ReapedSession { session, promoted } in reaped {
            let daemon_id = session.daemon_id;
            let network_id = session.network_id;
            self.record_stalled_session(session, now).await;
            if let Some(promoted) = promoted {
                self.publish_promoted(daemon_id, network_id, promoted).await;
            }
        }
    }

    /// Write the historical record for a reaped session, if its daemon still exists (a deleted
    /// daemon's id would violate the discovery table's foreign key).
    async fn record_stalled_session(
        &self,
        mut session: DiscoveryUpdatePayload,
        now: chrono::DateTime<Utc>,
    ) {
        let session_id = session.session_id;
        let daemon_id = session.daemon_id;

        let daemon = match self.daemon_service.get() {
            Some(ds) => ds.get_by_id(&daemon_id).await.ok().flatten(),
            None => None,
        };

        let Some(daemon) = daemon else {
            tracing::debug!(
                session_id = %session_id,
                daemon_id = %daemon_id,
                "Skipping historical record for stalled session — daemon no longer exists"
            );
            return;
        };
        session.daemon_version = daemon.base.version.map(|v| v.to_string());

        let network_name = match self.network_service.get_by_id(&session.network_id).await {
            Ok(Some(network)) => network.base.name,
            _ => "Unknown Network".to_string(),
        };

        let historical_discovery = Discovery {
            id: Uuid::new_v4(),
            created_at: session.started_at.unwrap_or(now),
            updated_at: now,
            base: DiscoveryBase {
                daemon_id: session.daemon_id,
                network_id: session.network_id,
                tags: Vec::new(),
                name: if matches!(session.discovery_type, DiscoveryType::Unified { .. }) {
                    "Discovery".to_string()
                } else {
                    format!("{} \u{2014} {}", session.discovery_type, network_name)
                },
                discovery_type: session.discovery_type.clone(),
                run_type: RunType::Historical {
                    results: Box::new(session),
                },
            },
            scan_count: 0,
            force_full_scan: false,
            integration_targets: vec![],
        };

        if let Err(e) = self.discovery_storage.create(&historical_discovery).await {
            tracing::error!(
                session_id = %session_id,
                error = %e,
                "Failed to create historical discovery record for stalled session"
            );
        }
    }

    /// Remove a scheduled job using fire-and-forget to prevent deadlocks.
    /// The scheduler's `remove()` can hang indefinitely if the background task is blocked.
    /// We clean up the job_id mapping immediately and spawn the actual removal as a
    /// background task so it never blocks the critical path.
    pub(crate) async fn remove_scheduled_job(&self, discovery_id: &Uuid) {
        // Read the job_id first, then drop the read lock before acquiring write lock.
        // Holding a RwLock read guard while awaiting .write() deadlocks.
        let job_id = self.job_ids.read().await.get(discovery_id).copied();
        if let Some(scheduler) = &self.scheduler
            && let Some(job_id) = job_id
        {
            // Always clean up the mapping immediately
            self.job_ids.write().await.remove(discovery_id);

            // Fire-and-forget the actual scheduler removal — it may hang
            // but won't block the current task
            let scheduler = Arc::clone(scheduler);
            tokio::spawn(async move {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    scheduler.remove(&job_id),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => tracing::warn!(
                        job_id = %job_id,
                        error = ?e,
                        "Failed to remove scheduled job"
                    ),
                    Err(_) => tracing::warn!(
                        job_id = %job_id,
                        "Timed out removing scheduled job"
                    ),
                }
            });
        }
    }
}
