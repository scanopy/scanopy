//! Live session access, snapshot-coordination acquire/release, state transitions, and cancellation pull.
use super::*;

use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::discovery::r#impl::warning_events::DiscoveryWarningScope;

impl DiscoveryService {
    /// Expose stream to handler
    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveryUpdatePayload> {
        self.update_tx.subscribe()
    }

    /// Get session state
    pub async fn get_session(&self, session_id: &Uuid) -> Option<DiscoveryUpdatePayload> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Get session state
    pub async fn get_all_sessions(&self, network_ids: &[Uuid]) -> Vec<DiscoveryUpdatePayload> {
        let all_sessions = self.sessions.read().await;
        all_sessions
            .values()
            .filter(|v| network_ids.contains(&v.network_id) && !v.phase.is_terminal())
            .cloned()
            .collect()
    }

    pub async fn get_sessions_for_daemon(&self, daemon_id: &Uuid) -> Vec<DiscoveryUpdatePayload> {
        // `sessions` before `daemon_sessions`: see the lock order on `DiscoveryService`.
        let all_sessions = self.sessions.read().await;
        let session_ids = self
            .daemon_sessions
            .read()
            .await
            .get(daemon_id)
            .cloned()
            .unwrap_or_default();

        // Preserve order from daemon_sessions Vec (not HashMap iteration order)
        // Only return Pending sessions - once dispatched, they transition to Starting
        session_ids
            .iter()
            .filter_map(|session_id| all_sessions.get(session_id).cloned())
            .filter(|session| session.phase == DiscoveryPhase::Pending)
            .collect()
    }

    /// Every session the server is tracking on a daemon, in any phase.
    pub async fn sessions_on_daemon(&self, daemon_id: &Uuid) -> Vec<DiscoveryUpdatePayload> {
        self.sessions
            .read()
            .await
            .values()
            .filter(|session| session.daemon_id == *daemon_id)
            .cloned()
            .collect()
    }

    /// Clear all sessions for a daemon from in-memory state.
    /// Used by tests to ensure clean state between phases.
    pub async fn clear_sessions_for_daemon(&self, daemon_id: &Uuid) {
        let mut sessions = self.sessions.write().await;
        let mut session_last_updated = self.session_last_updated.write().await;
        let mut daemon_sessions = self.daemon_sessions.write().await;
        let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;
        let mut discovery_sessions = self.discovery_sessions.write().await;

        if let Some(session_ids) = daemon_sessions.remove(daemon_id) {
            for session_id in &session_ids {
                sessions.remove(session_id);
                session_last_updated.remove(session_id);
                discovery_sessions.retain(|_, sid| sid != session_id);
            }
            tracing::debug!(
                daemon_id = %daemon_id,
                session_count = session_ids.len(),
                "Cleared all sessions for daemon"
            );
        }

        daemon_pull_cancellations.remove(daemon_id);
    }

    /// Check if daemon has an active (dispatched, non-terminal) discovery session.
    /// Both Queued and Pending are excluded — neither has been dispatched yet.
    pub async fn has_active_session_for_daemon(&self, daemon_id: &Uuid) -> bool {
        // `sessions` before `daemon_sessions`: see the lock order on `DiscoveryService`.
        let all_sessions = self.sessions.read().await;
        let session_ids = self
            .daemon_sessions
            .read()
            .await
            .get(daemon_id)
            .cloned()
            .unwrap_or_default();

        session_ids.iter().any(|session_id| {
            all_sessions
                .get(session_id)
                .map(|s| {
                    !s.phase.is_terminal()
                        && s.phase != DiscoveryPhase::Queued
                        && s.phase != DiscoveryPhase::Pending
                })
                .unwrap_or(false)
        })
    }

    /// Check if a discovery has an active session (any non-terminal phase).
    pub async fn has_active_session_for_discovery(&self, discovery_id: &Uuid) -> bool {
        let discovery_sessions = self.discovery_sessions.read().await;
        discovery_sessions.contains_key(discovery_id)
    }

    /// Transition a session from Pending to Starting phase.
    /// Called when the session is dispatched to the daemon.
    pub async fn transition_session_to_starting(&self, session_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id)
            && session.phase == DiscoveryPhase::Pending
        {
            session.phase = DiscoveryPhase::Starting;
            self.session_last_updated
                .write()
                .await
                .insert(session_id, Utc::now());
            tracing::debug!(
                session_id = %session_id,
                "Transitioned session to Starting phase"
            );
        }
    }

    /// Atomically reserve a network for snapshotting.
    ///
    /// Returns `true` iff the network has zero non-terminal sessions AND is
    /// not already reserved. On success, the network is added to
    /// `running_snapshots`; the caller MUST pair this with
    /// `release_network_for_snapshot` (typically via the manual-snapshot
    /// API handler's acquire → run → release sequence).
    ///
    /// Returns `false` if any non-terminal session exists on the network or
    /// if another snapshot is already in progress for it.
    pub async fn try_acquire_network_for_snapshot(&self, network_id: Uuid) -> bool {
        // Lock order: running_snapshots → sessions. This matches start_session,
        // which takes running_snapshots.read before sessions.write to decide
        // AwaitingSnapshot vs Queued/Pending. With this consistent order, the
        // "no non-terminal session" check and the insert into running_snapshots
        // are atomic against any in-flight start_session for the same network:
        // start_session is either fully visible (try_acquire returns false) or
        // not started yet (start_session sees running_snapshots and goes
        // AwaitingSnapshot).
        let mut running = self.running_snapshots.write().await;
        let sessions = self.sessions.read().await;

        if running.contains(&network_id) {
            return false;
        }

        let has_non_terminal_session = sessions
            .values()
            .any(|s| s.network_id == network_id && !s.phase.is_terminal());
        if has_non_terminal_session {
            return false;
        }

        running.insert(network_id);
        true
    }

    /// Release a network from snapshotting and unblock any AwaitingSnapshot
    /// sessions on it.
    ///
    /// For each session on this network whose phase is `AwaitingSnapshot`,
    /// runs the same Queued/Pending decision that `start_session` uses: if
    /// the daemon's queue would otherwise be empty after promotion, the
    /// session is promoted to `Pending` and a discovery event published;
    /// otherwise it stays `Queued`.
    pub async fn release_network_for_snapshot(&self, network_id: Uuid) {
        // Drop the running_snapshots entry up front so any subsequent
        // start_session for this network goes through the normal path.
        {
            let mut running = self.running_snapshots.write().await;
            running.remove(&network_id);
        }

        // Identify AwaitingSnapshot sessions on this network and decide
        // their next phase. Walk daemons in turn so the Queued/Pending
        // decision matches start_session's "promote only if daemon has no
        // other dispatched sessions" rule.
        let mut sessions = self.sessions.write().await;
        let daemon_sessions = self.daemon_sessions.read().await;

        let mut to_publish: Vec<DiscoveryUpdatePayload> = Vec::new();
        let awaiting_session_ids: Vec<Uuid> = sessions
            .values()
            .filter(|s| s.network_id == network_id && s.phase == DiscoveryPhase::AwaitingSnapshot)
            .map(|s| s.session_id)
            .collect();

        for session_id in awaiting_session_ids {
            let daemon_id = match sessions.get(&session_id) {
                Some(s) => s.daemon_id,
                None => continue,
            };

            // Mirror start_session's check: is anything already at
            // Pending/Started/Scanning on this daemon? Queued and
            // AwaitingSnapshot don't count — they haven't been dispatched.
            let daemon_has_active = if let Some(queue) = daemon_sessions.get(&daemon_id) {
                queue.iter().any(|sid| {
                    if *sid == session_id {
                        return false;
                    }
                    sessions
                        .get(sid)
                        .map(|s| {
                            !s.phase.is_terminal()
                                && s.phase != DiscoveryPhase::Queued
                                && s.phase != DiscoveryPhase::AwaitingSnapshot
                        })
                        .unwrap_or(false)
                })
            } else {
                false
            };

            if let Some(session) = sessions.get_mut(&session_id) {
                if daemon_has_active {
                    session.phase = DiscoveryPhase::Queued;
                } else {
                    session.phase = DiscoveryPhase::Pending;
                    self.session_last_updated
                        .write()
                        .await
                        .insert(session_id, Utc::now());
                    to_publish.push(session.clone());
                }
                let _ = self.update_tx.send(session.clone());
            }
        }

        drop(daemon_sessions);
        drop(sessions);

        for payload in to_publish {
            if let Err(e) = self
                .event_bus()
                .publish(payload.into_discovery_event())
                .await
            {
                tracing::warn!(
                    network_id = %network_id,
                    error = %e,
                    "Failed to publish discovery event after release_network_for_snapshot",
                );
            }
        }
    }

    /// Publish one event per coded warning, so the metrics, analytics and logging subscribers all
    /// see it once.
    ///
    /// Called from both producers — the daemon's terminal payload and LLDP/CDP resolution — which
    /// is the whole reason warnings have an operation of their own. Failures to publish are
    /// swallowed by the bus itself; a warning that does not reach a counter must never take the
    /// scan record down with it.
    pub async fn publish_warning_events(
        &self,
        network_id: Uuid,
        session_id: Uuid,
        daemon_id: Uuid,
        warnings: &[DiscoveryWarning],
    ) {
        for warning in warnings {
            let scope =
                DiscoveryWarningScope::new(network_id, session_id, daemon_id, warning.clone());
            let code = warning.code();
            let _ = self
                .event_bus
                .publish(Event::new(scope, code, AuthenticatedEntity::System))
                .await;
        }
    }

    /// Record that this daemon submitted a wire shape a current daemon no longer produces.
    ///
    /// Called from the entity-submission path, where the raw body is visible but the session is
    /// not. Idempotent: many interfaces, or many hosts, in one scan latch the same single fact.
    pub async fn note_superseded_wire_shape(&self, daemon_id: Uuid) {
        self.superseded_wire_daemons.write().await.insert(daemon_id);
    }

    /// Take the latched observation for this daemon, clearing it.
    ///
    /// Draining rather than reading is what scopes the fact to one scan: the next session starts
    /// clean and warns only if that scan also submits the old shape, so an upgraded daemon stops
    /// being reported without anyone clearing anything.
    pub async fn take_superseded_wire_shape(&self, daemon_id: &Uuid) -> bool {
        self.superseded_wire_daemons.write().await.remove(daemon_id)
    }

    pub async fn pull_cancellation_for_daemon(&self, daemon_id: &Uuid) -> (bool, Uuid) {
        let mut daemon_cancellation_ids = self.daemon_pull_cancellations.write().await;
        daemon_cancellation_ids
            .remove(daemon_id)
            .unwrap_or((false, Uuid::nil()))
    }
}
