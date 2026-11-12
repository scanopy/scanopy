//! ServerPoll polling loop and per-daemon poll state machine, and ending the sessions a daemon of
//! either mode can no longer finish.
use super::*;
use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};
use crate::server::shared::types::error_codes::ErrorCode;

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
                async move {
                    let _permit = sem.acquire().await.expect("Semaphore closed");
                    // poll_daemon handles retries internally via backon, and every failure it
                    // returns is already logged or on the daemon's unreachable event.
                    let _ = self.poll_daemon(&daemon, email_service).await;
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

    /// Mark a daemon as unreachable in the database and send notification. `error` is why the
    /// poll gave up; `rejected_by_daemon` when the daemon answered with a client error.
    async fn mark_daemon_unreachable(
        &self,
        daemon_id: Uuid,
        email_service: Option<&EmailService>,
        error: &anyhow::Error,
        rejected_by_daemon: bool,
    ) -> Result<()> {
        let mut daemon = self
            .get_by_id(&daemon_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Daemon {} not found", daemon_id))?;

        // Only notify if daemon was previously connected (skip if never seen = still setting up)
        let should_notify = daemon.base.last_seen.is_some();

        daemon.base.is_unreachable = true;

        self.update(
            &mut daemon,
            AuthenticatedEntity::System,
            UpdateReason::DaemonPollFailed {
                error: error.to_string(),
                rejected_by_daemon,
            },
        )
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

    /// End the sessions a DaemonPoll daemon was holding when it announced its startup: a process
    /// that just booted holds none, so every session it was handed is lost.
    ///
    /// `Starting` counts here. A DaemonPoll daemon is handed work only in a work request's
    /// response, and it makes no work request until its startup announcement returns, so a
    /// `Starting` session went to the process before this one.
    pub(crate) async fn fail_sessions_lost_to_restart(&self, daemon_id: Uuid) {
        let sessions = self.discovery_service.sessions_on_daemon(&daemon_id).await;
        for session_id in handed_out_sessions(&sessions) {
            self.discovery_service
                .fail_session(
                    session_id,
                    DiscoveryTerminalReason::DaemonRestarted,
                    "Daemon restarted during discovery; session state was lost".to_string(),
                )
                .await;
        }
    }

    /// End the sessions a DaemonPoll daemon no longer holds, judged from a work request.
    ///
    /// The daemon sends a session's final report, with retries, before it frees its slot, and it
    /// makes one work request at a time, starting the session a response hands it before the next.
    /// So a daemon that reports itself ready for work while the server has it holding a session
    /// either never got that session or gave up reporting how it ended. Either way nothing more is
    /// coming for it. Runs before the request hands out new work, so the session promoted in its
    /// place goes out in the same response.
    pub(crate) async fn fail_sessions_dropped_by_daemon(
        &self,
        daemon: &Daemon,
        ready_for_work: bool,
    ) {
        let sessions = self.discovery_service.sessions_on_daemon(&daemon.id).await;
        for session_id in
            dropped_sessions(ready_for_work, daemon.reports_ready_for_work(), &sessions)
        {
            self.discovery_service
                .fail_session(
                    session_id,
                    DiscoveryTerminalReason::DaemonDroppedSession,
                    "The daemon was idle while the server had it running this scan, and never reported how the scan ended"
                        .to_string(),
                )
                .await;
        }
    }

    /// Settle a session a ServerPoll daemon said it is not running when asked to cancel it.
    ///
    /// The daemon may have finished the session a moment before: it keeps the final report until
    /// its next session starts, so one drain records that outcome. A session still running once
    /// the drain succeeds was lost to a restart, the only way a ServerPoll daemon loses its report.
    /// `Starting` is left alone, since a dispatch may still be on its way to the daemon.
    /// Daemons too old for the poll endpoints are left to the stall sweep.
    pub(crate) async fn reconcile_not_running(&self, daemon_id: Uuid, session_id: Uuid) {
        let daemon = match self.get_by_id(&daemon_id).await {
            Ok(Some(daemon)) if daemon.supports_full_server_poll() => daemon,
            _ => return,
        };
        let api_key = match self.get_daemon_api_key(&daemon).await {
            Ok(key) => key,
            Err(e) => {
                tracing::debug!(
                    daemon_id = %daemon_id,
                    error = %e,
                    "Cannot poll the daemon to settle a session it is not running"
                );
                return;
            }
        };
        if !self.drain_discovery(&daemon, &api_key).await {
            return;
        }

        let sessions = self.discovery_service.sessions_on_daemon(&daemon_id).await;
        if dispatched_sessions(&sessions).contains(&session_id) {
            self.discovery_service
                .fail_session(
                    session_id,
                    DiscoveryTerminalReason::DaemonRestarted,
                    "The daemon no longer had this scan when asked to cancel it; it restarted and lost it"
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
            // Send first contact to assign daemon its server-side ID
            // This must succeed before we can proceed - without the correct ID,
            // discovery updates from the daemon won't be recognized by the server
            match self.send_first_contact(daemon, &api_key).await {
                Ok(status) => status,
                Err(e) => {
                    // First contact failed - abort poll entirely; the unreachable event's WARN
                    // line carries the error. Retried next cycle.
                    if let Err(mark_err) = self
                        .mark_daemon_unreachable(
                            daemon.id,
                            email_service,
                            &e,
                            e.downcast_ref::<super::http::DaemonHttpError>()
                                .is_some_and(|h| h.status.is_client_error()),
                        )
                        .await
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
                    // transient network failure; `DaemonPollFailed` says so on the unreachable
                    // WARN line, with the fix. Either way we mark the daemon unreachable to stop
                    // the poll loop from hammering a daemon that can't currently be polled.
                    let is_auth_failure = e
                        .downcast_ref::<super::http::DaemonHttpError>()
                        .is_some_and(|h| h.status.is_client_error());
                    if let Err(mark_err) = self
                        .mark_daemon_unreachable(daemon.id, email_service, &e, is_auth_failure)
                        .await
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

        // Process status data
        if let Err(e) = self
            .process_status(daemon.id, status.clone(), auth.clone())
            .await
        {
            // A daemon on another OS from the one it was created for gets no work: stop here,
            // before first-contact setup or dispatch. `process_status` has logged why.
            if matches!(e.error_code, Some(ErrorCode::DaemonOsMismatch { .. })) {
                return Err(anyhow::anyhow!(e.message));
            }
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
                .process_startup(daemon.id, version.clone(), status.os, auth.clone())
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
            // Create default discovery jobs. ServerPoll first-contact carries no init-command
            // targeting (admin-provisioned via status, not the register endpoint), so targeting
            // is empty here — a future server-held daemon config would populate it.
            if let Err(e) = self
                .create_default_discovery_jobs(
                    daemon.id,
                    daemon.base.site_id,
                    daemon.base.host_id,
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
                .emit_first_daemon_telemetry(daemon.id, daemon.base.site_id)
                .await
            {
                tracing::warn!(
                    daemon_id = %daemon.id,
                    error = ?e,
                    "Failed to emit first daemon telemetry"
                );
            }
        }

        let poll_ok = self.drain_discovery(daemon, &api_key).await;

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
            && let Some(work) = self.get_pending_work(daemon).await
        {
            let integration_targets = self
                .discovery_service
                .get_integration_targets_for_session(&work.session_id)
                .await;
            let request = self
                .discovery_service
                .build_daemon_request(
                    &work,
                    daemon.base.site_id,
                    &integration_targets,
                    daemon.base.version.as_ref(),
                    self.site_subnets(daemon.base.site_id).await,
                    self.subnets_scanned_elsewhere(&work).await,
                )
                .await;
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
                // Any other failure leaves nothing to start it either: a Starting session counts
                // as active, so `get_pending_work` hands this daemon nothing until it ends. End it
                // now, with the error, rather than at the stall sweep.
                Err(e) => {
                    self.discovery_service
                        .fail_session(
                            session_id,
                            DiscoveryTerminalReason::DaemonUnreachable,
                            e.to_string(),
                        )
                        .await;
                }
            }
        }

        Ok(())
    }

    /// Fetch a ServerPoll daemon's buffered discovery data and process it, entities before
    /// progress. Returns whether the poll succeeded.
    pub(crate) async fn drain_discovery(&self, daemon: &Daemon, api_key: &str) -> bool {
        match self.poll_discovery(daemon, api_key).await {
            Ok(mut poll_response) => {
                let auth = AuthenticatedEntity::System;

                // Processing runs as `System`, so the daemon's site scope is enforced here: the
                // DaemonPoll endpoints refuse a submission for a site the daemon is not assigned
                // to, and the same submission arriving by poll must not get further.
                let site_id = daemon.base.site_id;
                let refused = |site: Uuid, what: &str| {
                    if site != site_id {
                        tracing::warn!(
                            daemon_id = %daemon.id,
                            site_id = %site,
                            "Dropping {what} for a site the daemon is not assigned to"
                        );
                    }
                    site != site_id
                };
                let entities = &mut poll_response.entities;
                entities
                    .vlans
                    .retain(|batch| !refused(batch.request.site_id, "VLAN batch"));
                entities
                    .hosts
                    .retain(|request| !refused(request.host.base.site_id, "host"));
                entities
                    .subnets
                    .retain(|subnet| !refused(subnet.base.site_id, "subnet"));

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
                                .send_created_entities(daemon, api_key, created_entities)
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
                    && let Err(e) = self
                        .process_discovery_progress(daemon.id, site_id, progress)
                        .await
                {
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        error = %e.message,
                        "Refused a polled discovery report"
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
        }
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

    /// The subnets another discovery on `session`'s site scans and `session`'s does not, for
    /// [`DaemonPlacement::subnets_scanned_elsewhere`]. Empty for a rescan, which keeps every
    /// assignment it was sent before, and on a read failure, which costs only the warnings this
    /// set removes.
    ///
    /// [`DaemonPlacement::subnets_scanned_elsewhere`]: crate::server::credentials::service::DaemonPlacement::subnets_scanned_elsewhere
    pub(crate) async fn subnets_scanned_elsewhere(
        &self,
        session: &DiscoveryUpdatePayload,
    ) -> std::collections::HashSet<Uuid> {
        if !matches!(session.discovery_type, DiscoveryType::Unified { .. }) {
            return Default::default();
        }
        let filter =
            StorableFilter::<Discovery>::new_from_site_ids(&[session.site_id]).live_configs();
        let discoveries = match self.discovery_service.get_all(filter).await {
            Ok(discoveries) => discoveries,
            Err(e) => {
                tracing::warn!(
                    site_id = %session.site_id,
                    error = ?e,
                    "Could not read the site's discoveries for a discovery dispatch"
                );
                return Default::default();
            }
        };
        let mut daemon_ids: Vec<Uuid> = discoveries.iter().map(|d| d.base.daemon_id).collect();
        daemon_ids.push(session.daemon_id);
        let interfaced = self.get_interfaced_subnet_ids_batch(&daemon_ids).await;
        subnets_scanned_elsewhere(session, &discoveries, &interfaced)
    }
}

/// The subnets a credentialed scan sweeps: the ones it names, or its daemon's interfaced subnets
/// when it names none. `None` for a type that is sent no credentials, or a rescan, which visits
/// addresses rather than subnets.
fn unified_scan_subnets(
    discovery_type: &DiscoveryType,
    daemon_id: Uuid,
    interfaced: &std::collections::HashMap<Uuid, Vec<Uuid>>,
) -> Option<std::collections::HashSet<Uuid>> {
    let DiscoveryType::Unified { subnet_ids, .. } = discovery_type else {
        return None;
    };
    let ids = subnet_ids
        .as_ref()
        .or_else(|| interfaced.get(&daemon_id))
        .cloned()
        .unwrap_or_default();
    Some(ids.into_iter().collect())
}

/// See [`DaemonService::subnets_scanned_elsewhere`].
fn subnets_scanned_elsewhere(
    session: &DiscoveryUpdatePayload,
    discoveries: &[Discovery],
    interfaced: &std::collections::HashMap<Uuid, Vec<Uuid>>,
) -> std::collections::HashSet<Uuid> {
    let Some(own) = unified_scan_subnets(&session.discovery_type, session.daemon_id, interfaced)
    else {
        return Default::default();
    };
    discoveries
        .iter()
        .filter(|d| Some(d.id) != session.discovery_id)
        .filter_map(|d| unified_scan_subnets(&d.base.discovery_type, d.base.daemon_id, interfaced))
        .flatten()
        .filter(|subnet| !own.contains(subnet))
        .collect()
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

/// The sessions a DaemonPoll daemon was handed and has not finished: the dispatched ones, and
/// `Starting`, which for a DaemonPoll daemon means a work response already carried it (see
/// [`DaemonService::fail_sessions_lost_to_restart`]).
fn handed_out_sessions(sessions: &[DiscoveryUpdatePayload]) -> Vec<Uuid> {
    sessions
        .iter()
        .filter(|s| {
            matches!(
                s.phase,
                DiscoveryPhase::Starting | DiscoveryPhase::Started | DiscoveryPhase::Scanning
            )
        })
        .map(|s| s.session_id)
        .collect()
}

/// Sessions a DaemonPoll daemon has dropped: the ones it was handed, when it reports itself ready
/// for work. Readiness is only evidence when the daemon's version really reports it.
fn dropped_sessions(
    ready_for_work: bool,
    reports_ready: bool,
    sessions: &[DiscoveryUpdatePayload],
) -> Vec<Uuid> {
    if !(ready_for_work && reports_ready) {
        return Vec::new();
    }
    handed_out_sessions(sessions)
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

    fn unified(subnet_ids: Option<Vec<Uuid>>) -> DiscoveryType {
        DiscoveryType::Unified {
            host_id: Uuid::new_v4(),
            subnet_ids,
            host_naming_fallback: Default::default(),
            scan_settings: Default::default(),
        }
    }

    fn discovery(daemon_id: Uuid, discovery_type: DiscoveryType) -> Discovery {
        Discovery {
            id: Uuid::new_v4(),
            base: DiscoveryBase {
                daemon_id,
                discovery_type,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Two daemons on one site, each sweeping its own interfaced subnets, and sharing one. Only
    /// the other daemon's subnet is left to it; the shared one, and a subnet a scan names, are
    /// both this scan's own when it sweeps them.
    #[test]
    fn a_subnet_only_another_discovery_scans_is_scanned_elsewhere() {
        let (here, there) = (Uuid::new_v4(), Uuid::new_v4());
        let (own_lan, shared, other_lan, named) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let interfaced = std::collections::HashMap::from([
            (here, vec![own_lan, shared]),
            (there, vec![other_lan, shared]),
        ]);
        let own = discovery(here, unified(None));
        let discoveries = vec![
            own.clone(),
            discovery(there, unified(None)),
            discovery(there, unified(Some(vec![named]))),
        ];
        let session = DiscoveryUpdatePayload::new(
            Uuid::new_v4(),
            here,
            Uuid::new_v4(),
            own.base.discovery_type.clone(),
            Some(own.id),
        );

        assert_eq!(
            subnets_scanned_elsewhere(&session, &discoveries, &interfaced),
            std::collections::HashSet::from([other_lan, named])
        );

        // Naming the other daemon's subnet makes it this scan's own too.
        let mut naming = session.clone();
        naming.discovery_type = unified(Some(vec![own_lan, other_lan]));
        assert_eq!(
            subnets_scanned_elsewhere(&naming, &discoveries, &interfaced),
            std::collections::HashSet::from([shared, named])
        );

        // A rescan keeps every assignment.
        let mut rescan = session.clone();
        rescan.discovery_type = DiscoveryType::Rescan {
            host_id: Uuid::new_v4(),
            target_host_ids: vec![],
            ips: vec![],
            ports: vec![],
            settings: Default::default(),
        };
        assert!(subnets_scanned_elsewhere(&rescan, &discoveries, &interfaced).is_empty());
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

    /// A DaemonPoll daemon that comes back ready has dropped every session it was handed,
    /// including one a work response carried that it never started. Sessions still waiting on the
    /// server were never its to lose.
    #[test]
    fn a_ready_polling_daemon_has_dropped_what_it_was_handed() {
        let starting = session(DiscoveryPhase::Starting);
        let scanning = session(DiscoveryPhase::Scanning);
        let sessions = [
            starting.clone(),
            scanning.clone(),
            session(DiscoveryPhase::Queued),
            session(DiscoveryPhase::Pending),
            session(DiscoveryPhase::AwaitingSnapshot),
        ];

        assert_eq!(
            dropped_sessions(true, true, &sessions),
            vec![starting.session_id, scanning.session_id]
        );
    }

    /// Busy, or too old to report readiness truthfully: nothing is judged dropped.
    #[test]
    fn readiness_is_only_evidence_when_the_daemon_is_ready_and_reports_it() {
        let sessions = [session(DiscoveryPhase::Scanning)];

        assert!(dropped_sessions(false, true, &sessions).is_empty());
        assert!(dropped_sessions(true, false, &sessions).is_empty());
    }
}
