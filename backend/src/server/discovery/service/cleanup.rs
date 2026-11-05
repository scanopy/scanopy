//! Old/stalled session cleanup and scheduled-job removal.
use super::state::{self, SessionMaps};
use super::*;
use crate::server::daemons::subscriber::CancelDelivery;

impl DiscoveryService {
    pub async fn cleanup_old_sessions(&self, max_age_hours: i64) {
        let now = crate::server::shared::clock::now();
        let cutoff = now - chrono::Duration::hours(max_age_hours);

        let promotions = {
            // The lock order documented on `DiscoveryService`.
            let mut sessions = self.sessions.write().await;
            let mut last_updated = self.session_last_updated.write().await;
            let mut daemon_sessions = self.daemon_sessions.write().await;
            let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;
            let mut discovery_sessions = self.discovery_sessions.write().await;

            state::sweep_old(
                &mut SessionMaps {
                    sessions: &mut sessions,
                    last_updated: &mut last_updated,
                    daemon_sessions: &mut daemon_sessions,
                    discovery_sessions: &mut discovery_sessions,
                    pull_cancellations: &mut daemon_pull_cancellations,
                },
                cutoff,
                now,
            )
        };

        for promotion in promotions {
            self.publish_promoted(promotion.daemon_id, promotion.site_id, promotion.promoted)
                .await;
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
        let cutoff =
            crate::server::shared::clock::now() - chrono::Duration::hours(older_than_hours);
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
        let now = crate::server::shared::clock::now();
        let stall_threshold = chrono::Duration::minutes(5);

        // First pass: identify stalled sessions (read locks only)
        let stalled_sessions: Vec<DiscoveryUpdatePayload> = {
            let sessions = self.sessions.read().await;
            record_active_sessions(&sessions);
            let last_updated = self.session_last_updated.read().await;
            state::select_stalled(&sessions, &last_updated, now, stall_threshold)
        };

        if stalled_sessions.is_empty() {
            let sessions = self.sessions.read().await;
            let mut last_updated = self.session_last_updated.write().await;
            state::evict_tombstones(&sessions, &mut last_updated, now, stall_threshold);
            return;
        }

        // Why each went quiet, read before anything below contacts the daemons.
        let stalled_with_reasons = self
            .classify_stalls(&stalled_sessions, now, stall_threshold)
            .await;

        // Second pass (no locks held): tell each daemon to stop, in case it is still running the
        // session. A ServerPoll daemon is told directly. A DaemonPoll daemon reads a flag when it
        // next polls for work. Nothing is published here: the one event a stall produces is the
        // `Failed` one published once the session is reaped.
        let pulled_by_daemon = self.deliver_stall_cancellations(&stalled_sessions).await;
        {
            let mut pull_cancellations = self.daemon_pull_cancellations.write().await;
            for session in &stalled_sessions {
                if pulled_by_daemon.contains(&session.session_id) {
                    pull_cancellations.insert(session.daemon_id, (true, session.session_id));
                }
            }
        }

        // Third pass: cleanup session state (write locks, no await while they are held)
        let reaped = {
            let mut sessions = self.sessions.write().await;
            let mut last_updated = self.session_last_updated.write().await;
            let mut daemon_sessions = self.daemon_sessions.write().await;
            let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;
            let mut discovery_sessions = self.discovery_sessions.write().await;

            let reaped = state::reap(
                &mut SessionMaps {
                    sessions: &mut sessions,
                    last_updated: &mut last_updated,
                    daemon_sessions: &mut daemon_sessions,
                    discovery_sessions: &mut discovery_sessions,
                    pull_cancellations: &mut daemon_pull_cancellations,
                },
                &stalled_with_reasons,
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

        // Each reaped session publishes its own discovery_failed. Storage writes run on their own
        // task, for the same reason as `update_session`'s: the sessions have already left the
        // maps, so an interrupted write would lose their record.
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
    }

    /// Pair each stalled session with the reason it is recorded with, from when its daemon last
    /// checked in (see [`state::stall_reason`]). A daemon that cannot be read, or no daemon service
    /// to read it from (never the case outside tests), leaves the cause untold.
    async fn classify_stalls(
        &self,
        stalled_sessions: &[DiscoveryUpdatePayload],
        now: chrono::DateTime<Utc>,
        threshold: chrono::Duration,
    ) -> Vec<(Uuid, DiscoveryTerminalReason)> {
        let mut last_seen: HashMap<Uuid, Option<chrono::DateTime<Utc>>> = HashMap::new();
        for session in stalled_sessions {
            if last_seen.contains_key(&session.daemon_id) {
                continue;
            }
            let seen = match self.daemon_service.get() {
                Some(daemon_service) => match daemon_service.get_by_id(&session.daemon_id).await {
                    Ok(daemon) => daemon.and_then(|d| d.base.last_seen),
                    Err(e) => {
                        tracing::warn!(
                            daemon_id = %session.daemon_id,
                            error = ?e,
                            "Could not read a stalled session's daemon to name the stall"
                        );
                        None
                    }
                },
                None => None,
            };
            last_seen.insert(session.daemon_id, seen);
        }

        stalled_sessions
            .iter()
            .map(|session| {
                let seen = last_seen.get(&session.daemon_id).copied().flatten();
                (
                    session.session_id,
                    state::stall_reason(seen, now, threshold),
                )
            })
            .collect()
    }

    /// Ask each stalled session's daemon to stop, concurrently. Returns the sessions whose daemon
    /// polls for its cancellations, which the caller flags.
    ///
    /// Without a daemon service to ask (never the case outside tests), every session is treated
    /// as pulled, so the flag is set and nothing is lost.
    async fn deliver_stall_cancellations(
        &self,
        stalled_sessions: &[DiscoveryUpdatePayload],
    ) -> HashSet<Uuid> {
        let Some(daemon_service) = self.daemon_service.get() else {
            return stalled_sessions.iter().map(|s| s.session_id).collect();
        };

        let deliveries = stalled_sessions.iter().map(|session| async move {
            let delivery = daemon_service
                .deliver_cancellation(session.daemon_id, session.session_id)
                .await;
            (session, delivery)
        });

        let mut pulled_by_daemon = HashSet::new();
        for (session, delivery) in futures::future::join_all(deliveries).await {
            tracing::info!(
                session_id = %session.session_id,
                daemon_id = %session.daemon_id,
                delivery = delivery.outcome(),
                detail = delivery.detail(),
                "Asked the daemon to stop a stalled session"
            );
            if matches!(delivery, CancelDelivery::PulledByDaemon) {
                pulled_by_daemon.insert(session.session_id);
            }
        }
        pulled_by_daemon
    }

    /// The storage and event work for reaped sessions. Runs with no session lock held.
    async fn finish_reaped(&self, reaped: Vec<state::ReapedSession>, now: chrono::DateTime<Utc>) {
        for state::ReapedSession {
            session,
            promoted,
            previous_phase,
        } in reaped
        {
            let daemon_id = session.daemon_id;
            let site_id = session.site_id;
            super::dispatch::record_session_duration(&session);

            // The one event a stall produces: `Failed`, carrying why, the phase it stalled in and
            // when the daemon was last heard from. Metrics and analytics used to see a stall as a
            // user's cancel. Published after the historical row is written, as a finished session's
            // is, so a subscriber reading the discovery's run history finds this run in it.
            let mut event = session.into_discovery_event();
            event.scope.previous_phase = Some(previous_phase);
            self.record_stalled_session(session, now).await;
            self.event_bus().publish(event).await;
            if let Some(promoted) = promoted {
                self.publish_promoted(daemon_id, site_id, promoted).await;
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

        let name = self
            .historical_run_name(&session, session.discovery_id)
            .await;

        let historical_discovery = Discovery {
            id: Uuid::new_v4(),
            created_at: session.started_at.unwrap_or(now),
            updated_at: now,
            base: DiscoveryBase {
                daemon_id: session.daemon_id,
                site_id: session.site_id,
                tags: Vec::new(),
                name,
                discovery_type: session.discovery_type.clone(),
                run_type: RunType::Historical {
                    results: Box::new(session),
                },
            },
            scan_count: 0,
            force_full_scan: false,
            next_run_is_full_scan: false,
            integration_targets: vec![],
        };

        if let Err(e) = self.discovery_storage.create(&historical_discovery).await {
            super::dispatch::record_finish_error(
                super::dispatch::DiscoveryFinishStep::HistoryRecord,
            );
            tracing::error!(
                session_id = %session_id,
                error = %e,
                "Failed to create historical discovery record for stalled session"
            );
        } else if let Err(e) = self.record_warnings(&historical_discovery).await {
            super::dispatch::record_finish_error(
                super::dispatch::DiscoveryFinishStep::WarningRecords,
            );
            tracing::error!(
                session_id = %session_id,
                discovery_id = %historical_discovery.id,
                error = %e,
                "Failed to record warnings for stalled session"
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

/// Live sessions by phase, set on every stall sweep. Every non-terminal phase is set, zeros
/// included, so a phase that empties reads 0 rather than holding its last count. Terminal sessions
/// leave the map as they end, so there is nothing to count for them.
fn record_active_sessions(sessions: &HashMap<Uuid, DiscoveryUpdatePayload>) {
    use crate::server::shared::types::metadata::HasId;
    use strum::IntoEnumIterator;
    for phase in DiscoveryPhase::iter().filter(|p| !p.is_terminal()) {
        let count = sessions.values().filter(|s| s.phase == phase).count();
        metrics::gauge!("scanopy_discovery_sessions_active", "phase" => phase.id())
            .set(count as f64);
    }
}
