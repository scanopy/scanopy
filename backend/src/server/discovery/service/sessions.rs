//! Live session access, snapshot-coordination acquire/release, state transitions, and cancellation pull.
use super::state;
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
    pub async fn get_all_sessions(&self, site_ids: &[Uuid]) -> Vec<DiscoveryUpdatePayload> {
        let all_sessions = self.sessions.read().await;
        all_sessions
            .values()
            .filter(|v| {
                site_ids.contains(&v.site_id)
                    && !v.phase.is_terminal()
                    && v.discovery_type.records_history()
            })
            .cloned()
            .collect()
    }

    /// Send a session's state to the live stream, unless its type is kept out of view.
    pub(super) fn send_session_update(&self, session: &DiscoveryUpdatePayload) {
        if session.discovery_type.records_history() {
            let _ = self.update_tx.send(session.clone());
        }
    }

    /// Publish a session's phase event, unless its type is kept out of view. Metrics, analytics
    /// and failure emails all read these, and an interface refresh is none of their business.
    pub(super) async fn publish_session_event(
        &self,
        event: crate::server::shared::events::traits::Event<DiscoveryPhase>,
    ) {
        if event.scope.discovery_type.records_history() {
            self.event_bus().publish(event).await;
        }
    }

    pub async fn get_sessions_for_daemon(&self, daemon_id: &Uuid) -> Vec<DiscoveryUpdatePayload> {
        // `sessions` before `daemon_sessions`: see the lock order on `DiscoveryService`.
        let all_sessions = self.sessions.read().await;
        let daemon_sessions = self.daemon_sessions.read().await;
        // Only Pending: once dispatched, a session moves to Starting.
        state::pending_sessions_for_daemon(&all_sessions, &daemon_sessions, *daemon_id)
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
        }

        daemon_pull_cancellations.remove(daemon_id);
    }

    /// Check if daemon has an active (dispatched, non-terminal) discovery session.
    /// Both Queued and Pending are excluded — neither has been dispatched yet.
    pub async fn has_active_session_for_daemon(&self, daemon_id: &Uuid) -> bool {
        // `sessions` before `daemon_sessions`: see the lock order on `DiscoveryService`.
        let all_sessions = self.sessions.read().await;
        let daemon_sessions = self.daemon_sessions.read().await;
        state::daemon_has_dispatched_session(&all_sessions, &daemon_sessions, *daemon_id)
    }

    /// Check if a discovery has an active session (any non-terminal phase).
    pub async fn has_active_session_for_discovery(&self, discovery_id: &Uuid) -> bool {
        let discovery_sessions = self.discovery_sessions.read().await;
        discovery_sessions.contains_key(discovery_id)
    }

    /// Transition a session from Pending to Starting phase.
    /// Called when the session is dispatched to the daemon.
    pub async fn transition_session_to_starting(&self, session_id: Uuid) {
        let starting = {
            let mut sessions = self.sessions.write().await;
            match sessions.get_mut(&session_id) {
                Some(session) if session.phase == DiscoveryPhase::Pending => {
                    session.phase = DiscoveryPhase::Starting;
                    self.session_last_updated
                        .write()
                        .await
                        .insert(session_id, Utc::now());
                    Some(session.clone())
                }
                _ => None,
            }
        };

        // Published once the lock is released, as `record_update` does.
        if let Some(session) = starting {
            let mut event = session.into_discovery_event();
            event.scope.previous_phase = Some(DiscoveryPhase::Pending);
            self.publish_session_event(event).await;
        }
    }

    /// Reserve a site for snapshotting, returning a reservation that releases it.
    ///
    /// `None` when [`Self::try_acquire_site_for_snapshot`] would return `false`. Prefer this to
    /// pairing acquire and release by hand: the reservation also releases the site when the
    /// holder is dropped without releasing it, which is what a request handler's future does when
    /// its client disconnects.
    pub async fn try_reserve_site_for_snapshot(
        self: &Arc<Self>,
        site_id: Uuid,
    ) -> Option<SnapshotReservation> {
        self.try_acquire_site_for_snapshot(site_id)
            .await
            .then(|| SnapshotReservation {
                service: self.clone(),
                site_id,
                released: false,
            })
    }

    /// Atomically reserve a site for snapshotting.
    ///
    /// Returns `true` iff the site has zero non-terminal sessions AND is
    /// not already reserved. On success, the site is added to
    /// `running_snapshots`; the caller MUST pair this with
    /// `release_site_for_snapshot` (typically via the manual-snapshot
    /// API handler's acquire → run → release sequence).
    ///
    /// Returns `false` if any non-terminal session exists on the site or
    /// if another snapshot is already in progress for it.
    pub async fn try_acquire_site_for_snapshot(&self, site_id: Uuid) -> bool {
        // Lock order: running_snapshots → sessions. This matches start_session,
        // which takes running_snapshots.read before sessions.write to decide
        // AwaitingSnapshot vs Queued/Pending. With this consistent order, the
        // "no non-terminal session" check and the insert into running_snapshots
        // are atomic against any in-flight start_session for the same site:
        // start_session is either fully visible (try_acquire returns false) or
        // not started yet (start_session sees running_snapshots and goes
        // AwaitingSnapshot).
        let mut running = self.running_snapshots.write().await;
        let sessions = self.sessions.read().await;

        if running.contains(&site_id) {
            return false;
        }

        let has_non_terminal_session = sessions
            .values()
            .any(|s| s.site_id == site_id && !s.phase.is_terminal());
        if has_non_terminal_session {
            return false;
        }

        running.insert(site_id);
        true
    }

    /// Release a site from snapshotting and unblock any AwaitingSnapshot
    /// sessions on it.
    ///
    /// For each session on this site whose phase is `AwaitingSnapshot`,
    /// runs the same Queued/Pending decision that `start_session` uses: if
    /// the daemon's queue would otherwise be empty after promotion, the
    /// session is promoted to `Pending` and a discovery event published;
    /// otherwise it stays `Queued`.
    pub async fn release_site_for_snapshot(&self, site_id: Uuid) {
        // Drop the running_snapshots entry up front so any subsequent
        // start_session for this site goes through the normal path.
        {
            let mut running = self.running_snapshots.write().await;
            running.remove(&site_id);
        }

        // Identify AwaitingSnapshot sessions on this site and decide
        // their next phase. Walk daemons in turn so the Queued/Pending
        // decision matches start_session's "promote only if daemon has no
        // other dispatched sessions" rule.
        // The lock order documented on `DiscoveryService`.
        let mut sessions = self.sessions.write().await;
        let mut last_updated = self.session_last_updated.write().await;
        let daemon_sessions = self.daemon_sessions.read().await;

        let mut to_publish: Vec<DiscoveryUpdatePayload> = Vec::new();
        let awaiting_session_ids: Vec<Uuid> = sessions
            .values()
            .filter(|s| s.site_id == site_id && s.phase == DiscoveryPhase::AwaitingSnapshot)
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
                    last_updated.insert(session_id, Utc::now());
                    to_publish.push(session.clone());
                }
                self.send_session_update(session);
            }
        }

        drop(daemon_sessions);
        drop(last_updated);
        drop(sessions);

        for payload in to_publish {
            self.publish_session_event(payload.into_discovery_event())
                .await;
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
        site_id: Uuid,
        session_id: Uuid,
        daemon_id: Uuid,
        warnings: &[DiscoveryWarning],
    ) {
        for warning in warnings {
            let scope = DiscoveryWarningScope::new(site_id, session_id, daemon_id, warning.clone());
            let code = warning.code();
            self.event_bus
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

    /// Record the credentials a dispatch to this daemon withheld for its version, replacing what
    /// an earlier dispatch recorded: only the scan now starting reports them.
    pub async fn note_withheld_credentials(
        &self,
        daemon_id: Uuid,
        warnings: Vec<DiscoveryWarning>,
    ) {
        let mut withheld = self.withheld_credentials.write().await;
        if warnings.is_empty() {
            withheld.remove(&daemon_id);
        } else {
            withheld.insert(daemon_id, warnings);
        }
    }

    /// Take this daemon's withheld-credential warnings, clearing them.
    pub async fn take_withheld_credentials(&self, daemon_id: &Uuid) -> Vec<DiscoveryWarning> {
        self.withheld_credentials
            .write()
            .await
            .remove(daemon_id)
            .unwrap_or_default()
    }

    /// Record subnets a host request from this daemon stored, for its scan's terminal update.
    ///
    /// Latched per daemon for the same reason as `note_superseded_wire_shape`: the submission
    /// path sees the stored ids but not the session. Idempotent across hosts sharing a subnet.
    pub async fn note_touched_subnets(&self, daemon_id: Uuid, subnet_ids: Vec<Uuid>) {
        if subnet_ids.is_empty() {
            return;
        }
        self.touched_subnets
            .write()
            .await
            .entry(daemon_id)
            .or_default()
            .extend(subnet_ids);
    }

    /// Take this daemon's latched subnets, clearing them, so the next scan starts empty.
    pub async fn take_touched_subnets(&self, daemon_id: &Uuid) -> Vec<Uuid> {
        self.touched_subnets
            .write()
            .await
            .remove(daemon_id)
            .map(|ids| ids.into_iter().collect())
            .unwrap_or_default()
    }

    /// The session a polling daemon is told to cancel; see [`state::cancellation_to_deliver`].
    pub async fn pull_cancellation_for_daemon(&self, daemon_id: &Uuid) -> Option<Uuid> {
        // `sessions` before `daemon_pull_cancellations`: see the lock order on `DiscoveryService`.
        let sessions = self.sessions.read().await;
        let mut pull_cancellations = self.daemon_pull_cancellations.write().await;
        state::cancellation_to_deliver(&sessions, &mut pull_cancellations, *daemon_id)
    }
}

/// A site reserved for a snapshot.
///
/// Release it with [`Self::release`] when the snapshot is done. If it is dropped unreleased (the
/// request serving the snapshot was abandoned mid-way), it releases the site itself. Without
/// that, the reservation outlived the request and every later scan on the site waited in
/// `AwaitingSnapshot`, which the stall sweep never touches, until the server restarted.
pub struct SnapshotReservation {
    service: Arc<DiscoveryService>,
    site_id: Uuid,
    released: bool,
}

impl SnapshotReservation {
    pub async fn release(mut self) {
        self.service.release_site_for_snapshot(self.site_id).await;
        // Set only once the release has run: a release interrupted part-way is finished by `Drop`.
        // Releasing twice is harmless.
        self.released = true;
    }
}

impl Drop for SnapshotReservation {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        // `Drop` cannot await the release, so it runs on its own task. With no runtime left (the
        // server is shutting down) there is nothing to release: the reservation is in memory only.
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        tracing::warn!(
            site_id = %self.site_id,
            "Snapshot request ended without releasing its site; releasing it now"
        );
        let service = self.service.clone();
        let site_id = self.site_id;
        runtime.spawn(async move {
            service.release_site_for_snapshot(site_id).await;
        });
    }
}
