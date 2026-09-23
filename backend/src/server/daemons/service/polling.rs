//! ServerPoll polling loop and per-daemon poll state machine.
use super::*;
use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};

impl DaemonService {
    // ========================================================================
    // Polling loop methods (moved from poller.rs)
    // ========================================================================

    /// Start the ServerPoll polling loop. Should be called once from main.
    pub async fn start_polling_loop(self: Arc<Self>, email_service: Option<Arc<EmailService>>) {
        let poll_interval = Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS);
        let mut interval = tokio::time::interval(poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            if let Err(e) = self.poll_all_daemons(email_service.as_deref()).await {
                tracing::warn!("Daemon poller cycle failed: {}", e);
            }
        }
    }

    /// Poll all ServerPoll-mode daemons in parallel with semaphore-limited concurrency.
    /// Uses backon for per-daemon retries - daemon is marked unreachable after exhausting retries.
    async fn poll_all_daemons(&self, email_service: Option<&EmailService>) -> Result<()> {
        let daemons = self.get_server_poll_daemons().await?;

        if daemons.is_empty() {
            tracing::trace!("No ServerPoll daemons to poll");
            return Ok(());
        }

        tracing::debug!(
            "Polling {} ServerPoll daemons in parallel (max concurrent: {})",
            daemons.len(),
            MAX_CONCURRENT_POLLS
        );

        // Create parallel poll futures with semaphore-limited concurrency
        // Each daemon poll uses backon internally for retries
        let poll_futures: Vec<_> = daemons
            .into_iter()
            .map(|daemon| {
                let sem = self.poll_semaphore.clone();
                let daemon_id = daemon.id;
                let daemon_name = daemon.base.name.clone();
                async move {
                    let _permit = sem.acquire().await.expect("Semaphore closed");
                    // poll_daemon handles retries internally via backon
                    // Errors are logged inside poll_daemon, but log unexpected ones here too
                    if let Err(e) = self.poll_daemon(&daemon, email_service).await {
                        tracing::debug!(
                            daemon_id = %daemon_id,
                            daemon_name = %daemon_name,
                            error = %e,
                            "Poll cycle failed for daemon"
                        );
                    }
                }
            })
            .collect();

        // Execute all polls in parallel
        join_all(poll_futures).await;

        Ok(())
    }

    /// Get all daemons in ServerPoll mode that are reachable
    async fn get_server_poll_daemons(&self) -> Result<Vec<Daemon>> {
        let filter = StorableFilter::<Daemon>::new_for_daemon_poller_system_job();

        let reachable_server_poll_daemons = self.get_all(filter).await?;

        Ok(reachable_server_poll_daemons)
    }

    /// Mark a daemon as unreachable in the database and send notification
    async fn mark_daemon_unreachable(
        &self,
        daemon_id: Uuid,
        email_service: Option<&EmailService>,
    ) -> Result<()> {
        let mut daemon = self
            .get_by_id(&daemon_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Daemon {} not found", daemon_id))?;

        // Only notify if daemon was previously connected (skip if never seen = still setting up)
        let should_notify = daemon.base.last_seen.is_some();

        daemon.base.is_unreachable = true;

        self.update(&mut daemon, AuthenticatedEntity::System)
            .await?;

        if should_notify
            && let Some(email_service) = email_service
            && let Err(e) = self
                .send_unreachable_notification(&daemon, email_service)
                .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = %e,
                "Failed to send daemon unreachable notification email"
            );
        }

        self.fail_sessions_on_unreachable(daemon_id).await;

        Ok(())
    }

    /// End the sessions a daemon was running when the server gave up on it.
    ///
    /// The poll loop is how a ServerPoll session reaches its outcome, so a daemon it can no longer
    /// reach has none coming. Left alone the session sits until the stall sweep, and says it
    /// stalled when what happened is that its daemon went away.
    async fn fail_sessions_on_unreachable(&self, daemon_id: Uuid) {
        let sessions = self
            .discovery_service
            .get_sessions_for_daemon(&daemon_id)
            .await;

        for session_id in dispatched_sessions(&sessions) {
            self.discovery_service
                .fail_session(
                    session_id,
                    DiscoveryTerminalReason::DaemonUnreachable,
                    "The server stopped being able to reach the daemon running this scan"
                        .to_string(),
                )
                .await;
        }
    }

    /// Poll a single daemon for status and discovery data.
    /// Uses backon for retry with exponential backoff.
    /// Marks daemon unreachable after UNREACHABLE_THRESHOLD failures.
    ///
    /// Legacy daemons (< v0.14.0) are skipped entirely - they don't support
    /// the polling endpoints (/api/status, /api/poll, /api/first-contact,
    /// /api/discovery/entities-created). Legacy daemons stay alive via their
    /// own heartbeat calls to the server's backward-compat endpoint.
    async fn poll_daemon(
        &self,
        daemon: &Daemon,
        email_service: Option<&EmailService>,
    ) -> Result<()> {
        Self::warn_if_insecure_daemon_url(&daemon.base.url);

        // Skip polling only for daemons KNOWN to predate the poll endpoints. A
        // daemon with no recorded version has never contacted the server yet —
        // it must be polled (first contact) precisely so it can report its
        // version, so it is NOT treated as legacy here. Otherwise a provisioned
        // daemon (whose version is None until first contact) would be skipped
        // forever, never poll, and never get a version — a deadlock. A genuinely
        // old daemon that can't be polled simply fails and is marked unreachable.
        if daemon.base.version.is_some() && !daemon.supports_full_server_poll() {
            tracing::debug!(
                daemon_id = %daemon.id,
                daemon_name = %daemon.base.name,
                version = ?daemon.base.version,
                "Skipping poll for legacy daemon (< v0.14.0) - polling endpoints not supported"
            );
            return Ok(());
        }

        tracing::debug!(
            daemon_id = %daemon.id,
            daemon_name = %daemon.base.name,
            daemon_url = %daemon.base.url,
            api_key_id = ?daemon.base.api_key_id,
            "Starting poll for daemon"
        );

        // Get the API key for this daemon
        let api_key = match self.get_daemon_api_key(daemon).await {
            Ok(key) => key,
            Err(e) => {
                // API key lookup failure is a configuration error, not a network error.
                // Log it clearly so the user can fix it.
                tracing::error!(
                    daemon_id = %daemon.id,
                    daemon_name = %daemon.base.name,
                    error = %e,
                    "Failed to get API key for daemon - check that daemon has api_key_id set and the key has plaintext stored"
                );
                return Err(e);
            }
        };

        // Check if this is first contact (last_seen was None)
        let is_first_contact = daemon.base.last_seen.is_none();

        // Get status - either via first contact (which assigns daemon ID) or regular poll
        let status = if is_first_contact {
            tracing::info!(
                daemon_id = %daemon.id,
                daemon_name = %daemon.base.name,
                "First contact with ServerPoll daemon - assigning ID"
            );

            // Send first contact to assign daemon its server-side ID
            // This must succeed before we can proceed - without the correct ID,
            // discovery updates from the daemon won't be recognized by the server
            match self.send_first_contact(daemon, &api_key).await {
                Ok(status) => status,
                Err(e) => {
                    // First contact failed - abort poll entirely
                    // No point continuing since discovery updates won't work without correct ID
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        daemon_name = %daemon.base.name,
                        error = %e,
                        "First contact failed - aborting poll (will retry next cycle)"
                    );
                    // Mark unreachable after threshold failures
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        daemon_name = %daemon.base.name,
                        "Marking daemon unreachable after {} failures",
                        UNREACHABLE_THRESHOLD
                    );
                    if let Err(mark_err) =
                        self.mark_daemon_unreachable(daemon.id, email_service).await
                    {
                        tracing::error!(
                            daemon_id = %daemon.id,
                            "Failed to mark daemon as unreachable: {}",
                            mark_err
                        );
                    }
                    return Err(e);
                }
            }
        } else {
            // Regular status poll (retry is built into the helper)
            match self.poll_status(daemon, &api_key).await {
                Ok(status) => status,
                Err(e) => {
                    // A 4xx (e.g. 401 key mismatch) is a definitive misconfiguration, not a
                    // transient network failure — say so, instead of the generic "unreachable
                    // after N failures". Either way we mark the daemon unreachable to stop the
                    // poll loop from hammering a daemon that can't currently be polled.
                    let is_auth_failure = e
                        .downcast_ref::<super::http::DaemonHttpError>()
                        .is_some_and(|h| h.status.is_client_error());
                    if is_auth_failure {
                        tracing::warn!(
                            daemon_id = %daemon.id,
                            daemon_name = %daemon.base.name,
                            "Daemon rejected the poll ({e}) — likely an API key mismatch. \
                             Re-provision/re-install the daemon so its key matches the server. \
                             Pausing polling of this daemon."
                        );
                    } else {
                        tracing::warn!(
                            daemon_id = %daemon.id,
                            daemon_name = %daemon.base.name,
                            "Marking daemon unreachable after {} failures",
                            UNREACHABLE_THRESHOLD
                        );
                    }
                    if let Err(mark_err) =
                        self.mark_daemon_unreachable(daemon.id, email_service).await
                    {
                        tracing::error!(
                            daemon_id = %daemon.id,
                            "Failed to mark daemon as unreachable: {}",
                            mark_err
                        );
                    }
                    return Err(e);
                }
            }
        };

        let auth = AuthenticatedEntity::System;

        tracing::debug!(
            daemon_id = %daemon.id,
            daemon_name = %daemon.base.name,
            ready_for_work = status.ready_for_work,
            interfaced_subnet_count = status.interfaced_subnets.len(),
            "ServerPoll status received"
        );

        // Process status data
        if let Err(e) = self
            .process_status(daemon.id, status.clone(), auth.clone())
            .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = ?e,
                "Failed to process daemon status"
            );
        }

        // If daemon has a version and it's different from what we have, process startup
        // (process_startup handles migration from legacy to unified discovery internally)
        if let Some(version) = status.version.clone()
            && daemon.base.version.as_ref() != Some(&version)
            && let Err(e) = self
                .process_startup(daemon.id, version.clone(), auth.clone())
                .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = ?e,
                "Failed to process daemon startup"
            );
        }

        // Capabilities are now resolved inside process_status() — no separate call needed.

        // First contact - create default discovery jobs and emit telemetry
        if is_first_contact {
            tracing::info!(
                daemon_id = %daemon.id,
                daemon_name = %daemon.base.name,
                "First contact with ServerPoll daemon"
            );

            // Determine if org is on Free plan for discovery defaults
            let is_free_plan = if let Ok(Some(network)) = self
                .network_service
                .get_by_id(&daemon.base.network_id)
                .await
            {
                self.organization_service
                    .get_by_id(&network.base.organization_id)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|o| o.base.plan)
                    .map(|p| p.is_free())
                    .unwrap_or(true)
            } else {
                false
            };

            // Create default discovery jobs. ServerPoll first-contact carries no init-command
            // targeting (admin-provisioned via status, not the register endpoint), so targeting
            // is empty here — a future server-held daemon config would populate it.
            if let Err(e) = self
                .create_default_discovery_jobs(
                    daemon.id,
                    daemon.base.network_id,
                    daemon.base.host_id,
                    is_free_plan,
                    &[],
                )
                .await
            {
                tracing::warn!(
                    daemon_id = %daemon.id,
                    error = ?e,
                    "Failed to create default discovery jobs"
                );
            }

            // Emit telemetry
            if let Err(e) = self
                .emit_first_daemon_telemetry(daemon.id, daemon.base.network_id)
                .await
            {
                tracing::warn!(
                    daemon_id = %daemon.id,
                    error = ?e,
                    "Failed to emit first daemon telemetry"
                );
            }
        }

        // Poll discovery data
        let poll_ok = match self.poll_discovery(daemon, &api_key).await {
            Ok(poll_response) => {
                let auth = AuthenticatedEntity::System;

                // Entities before progress. A poll response can carry both the
                // session's terminal update and the last hosts of that session,
                // and finalizing a session consumes the discovery's one-shot
                // integration targets — which are only safe to consume once the
                // credentials that probed successfully have been promoted to
                // host assignments, which is what persisting these entities does.
                if !poll_response.entities.is_empty() {
                    match self
                        .process_discovery_entities(poll_response.entities, auth.clone())
                        .await
                    {
                        Ok(created_entities) => {
                            // Send created entities back to daemon
                            if let Err(e) = self
                                .send_created_entities(daemon, &api_key, created_entities)
                                .await
                            {
                                tracing::warn!(
                                    daemon_id = %daemon.id,
                                    "Failed to send created entities to daemon: {}",
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                daemon_id = %daemon.id,
                                error = ?e,
                                "Failed to process discovery entities"
                            );
                        }
                    }
                }

                // Process progress update if available
                if let Some(progress) = poll_response.progress
                    && let Err(e) = self.process_discovery_progress(progress).await
                {
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        error = ?e,
                        "Failed to process discovery progress"
                    );
                }
                true
            }
            Err(e) => {
                tracing::debug!(
                    daemon_id = %daemon.id,
                    "Failed to poll daemon discovery: {}",
                    e
                );
                false
            }
        };

        // A daemon that says it is ready for work while the server has it running a session has
        // restarted and lost that session: it will never report on it again. After the progress
        // drain above, so a session that finished moments ago has already been recorded as such;
        // before dispatch below, so a session dispatched this cycle is not mistaken for a lost one.
        let running = self.discovery_service.sessions_on_daemon(&daemon.id).await;
        for session_id in restarted_sessions(
            status.ready_for_work,
            daemon.reports_ready_for_work(),
            poll_ok,
            &running,
        ) {
            self.discovery_service
                .fail_session(
                    session_id,
                    DiscoveryTerminalReason::DaemonRestarted,
                    "Daemon restarted during discovery; session state was lost".to_string(),
                )
                .await;
        }

        // Check for pending work and initiate if daemon reports ready
        if status.ready_for_work
            && let Some(work) = self.get_pending_work(daemon.id).await
        {
            let integration_targets = self
                .discovery_service
                .get_integration_targets_for_session(&work.session_id)
                .await;
            let request = self
                .discovery_service
                .build_daemon_request(
                    &work,
                    work.network_id,
                    &integration_targets,
                    daemon.base.version.as_ref(),
                    self.network_subnets(work.network_id).await,
                )
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to build daemon request: {}", e);
                    DaemonDiscoveryRequest {
                        session_id: work.session_id,
                        discovery_id: work.discovery_id.unwrap_or_default(),
                        discovery_type: work.discovery_type,
                        credential_mappings: vec![],
                        subnets: vec![],
                    }
                });
            let session_id = request.session_id;
            match self
                .send_discovery_request_to_daemon(daemon, Some(&api_key), request)
                .await
            {
                Ok(()) => {}
                // The daemon refused because it is running another session. Nothing will ever
                // start this one, so end it now rather than leave it in `Starting` until the
                // stall sweep, refusing every retry of the same discovery meanwhile.
                Err(e)
                    if e.downcast_ref::<DaemonHttpError>()
                        .is_some_and(|h| h.status == reqwest::StatusCode::CONFLICT) =>
                {
                    self.discovery_service
                        .fail_session(
                            session_id,
                            DiscoveryTerminalReason::DaemonBusy,
                            "The daemon was already running another session and refused this one"
                                .to_string(),
                        )
                        .await;
                }
                Err(e) => {
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        session_id = %session_id,
                        error = %e,
                        "Failed to initiate discovery"
                    );
                }
            }
        }

        Ok(())
    }

    /// Get the API key for a daemon (from the linked api_key_id)
    pub async fn get_daemon_api_key(&self, daemon: &Daemon) -> Result<String> {
        let api_key_id = daemon
            .base
            .api_key_id
            .ok_or_else(|| anyhow::anyhow!("Daemon {} has no linked API key", daemon.id))?;

        let api_key = self
            .daemon_api_key_service
            .get_by_id(&api_key_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("API key {} not found", api_key_id))?;

        // Get the plaintext key (stored for ServerPoll daemons)
        api_key
            .base
            .plaintext
            .as_ref()
            .map(|s| s.expose_secret().to_string())
            .ok_or_else(|| anyhow::anyhow!("API key {} has no stored plaintext", api_key_id))
    }
}

/// Sessions a daemon has lost to a restart: the ones it is running as far as the server knows,
/// when it reports itself ready for work.
///
/// Readiness is only evidence when the daemon's version really reports it (older daemons always
/// read ready), and only once this cycle's poll has drained the daemon's progress, so that a
/// session it finished just before the status check is recorded as finished rather than lost. Only
/// `Started` and `Scanning` count: a `Starting` session is mid-dispatch, and `Queued`, `Pending`
/// and `AwaitingSnapshot` were never sent to the daemon.
fn restarted_sessions(
    ready_for_work: bool,
    reports_ready: bool,
    poll_ok: bool,
    sessions: &[DiscoveryUpdatePayload],
) -> Vec<Uuid> {
    if !(ready_for_work && reports_ready && poll_ok) {
        return Vec::new();
    }
    dispatched_sessions(sessions)
}

/// The sessions a daemon is holding: the ones it was told to run and has not finished.
///
/// `Started` and `Scanning` count: a `Starting` session is mid-dispatch, and `Queued`, `Pending`
/// and `AwaitingSnapshot` were never sent to the daemon. What happened to the daemon decides what
/// these become; which sessions are its own does not change with it.
fn dispatched_sessions(sessions: &[DiscoveryUpdatePayload]) -> Vec<Uuid> {
    sessions
        .iter()
        .filter(|s| matches!(s.phase, DiscoveryPhase::Started | DiscoveryPhase::Scanning))
        .map(|s| s.session_id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::discovery::r#impl::types::DiscoveryType;

    fn session(phase: DiscoveryPhase) -> DiscoveryUpdatePayload {
        let mut s = DiscoveryUpdatePayload::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            DiscoveryType::default(),
            None,
        );
        s.phase = phase;
        s
    }

    #[test]
    fn a_ready_daemon_with_a_running_session_has_lost_it() {
        let scanning = session(DiscoveryPhase::Scanning);

        assert_eq!(
            restarted_sessions(true, true, true, std::slice::from_ref(&scanning)),
            vec![scanning.session_id]
        );
    }

    #[test]
    fn readiness_from_a_daemon_too_old_to_report_it_proves_nothing() {
        let scanning = session(DiscoveryPhase::Scanning);

        assert!(restarted_sessions(true, false, true, &[scanning]).is_empty());
    }

    #[test]
    fn nothing_is_judged_lost_until_the_daemon_s_progress_has_been_drained() {
        // The session may have finished just before the status check, with its outcome still
        // waiting in the poll that failed.
        let scanning = session(DiscoveryPhase::Scanning);

        assert!(restarted_sessions(true, true, false, &[scanning]).is_empty());
    }

    #[test]
    fn sessions_not_yet_running_on_the_daemon_are_never_judged_lost() {
        let sessions = [
            session(DiscoveryPhase::Starting),
            session(DiscoveryPhase::Queued),
            session(DiscoveryPhase::Pending),
            session(DiscoveryPhase::AwaitingSnapshot),
        ];

        assert!(restarted_sessions(true, true, true, &sessions).is_empty());
    }

    /// The same rule decides what a restart lost and what an unreachable daemon is holding, so a
    /// phase admitted to one is admitted to the other.
    #[test]
    fn a_daemon_holds_exactly_the_sessions_it_was_told_to_run() {
        let scanning = session(DiscoveryPhase::Scanning);
        let started = session(DiscoveryPhase::Started);
        let sessions = [
            scanning.clone(),
            started.clone(),
            session(DiscoveryPhase::Starting),
            session(DiscoveryPhase::Queued),
            session(DiscoveryPhase::Pending),
            session(DiscoveryPhase::AwaitingSnapshot),
            session(DiscoveryPhase::Complete),
        ];

        assert_eq!(
            dispatched_sessions(&sessions),
            vec![scanning.session_id, started.session_id]
        );
    }
}
