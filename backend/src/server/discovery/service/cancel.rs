//! Cancelling a session: removing it from the queue, or asking its daemon to stop.
use super::state::{self, SessionMaps};
use super::*;

impl DiscoveryService {
    pub async fn cancel_session(
        &self,
        session_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        // Get the session
        let session = match self.get_session(&session_id).await {
            Some(session) => session,
            None => {
                return Err(anyhow!("Session '{}' not found", session_id));
            }
        };

        let site_id = session.site_id;
        let daemon_id = session.daemon_id;
        let phase = session.phase;
        let discovery_id = self.lookup_discovery_id(&session_id).await;
        // Capture before `session.discovery_type` is moved into the update below.
        let is_rescan = session.discovery_type.rescan_target_host_ids().is_some();

        let cancelled_update = DiscoveryUpdatePayload {
            session_id,
            site_id,
            daemon_id,
            phase: DiscoveryPhase::Cancelled,
            progress: 0,
            error: None,
            warnings: Vec::new(),
            credential_results: Vec::new(),
            started_at: session.started_at,
            finished_at: Some(Utc::now()),
            discovery_type: session.discovery_type,
            hosts_discovered: None,
            estimated_remaining_secs: None,
            discovery_id,
            // Preserve it: a cancelled rescan still needs its transient subnet
            // reaped and its digest suppressed.
            scanned: None,
            reason: Some(DiscoveryTerminalReason::UserCancelled),
            last_update_at: None,
            daemon_version: None,
            last_activity_at: None,
        };

        // Handle based on current phase
        match phase {
            // Queued/Pending/AwaitingSnapshot sessions: just remove from queue.
            // AwaitingSnapshot is treated like Queued for cancellation — the
            // session has not yet been dispatched to the daemon, so cleanup
            // is just a queue removal.
            DiscoveryPhase::Queued | DiscoveryPhase::Pending | DiscoveryPhase::AwaitingSnapshot => {
                let promoted = {
                    // The lock order documented on `DiscoveryService`.
                    let mut sessions = self.sessions.write().await;
                    let mut last_updated = self.session_last_updated.write().await;
                    let mut daemon_sessions = self.daemon_sessions.write().await;
                    let mut pull_cancellations = self.daemon_pull_cancellations.write().await;
                    let mut discovery_sessions = self.discovery_sessions.write().await;
                    state::cancel_undispatched(
                        &mut SessionMaps {
                            sessions: &mut sessions,
                            last_updated: &mut last_updated,
                            daemon_sessions: &mut daemon_sessions,
                            discovery_sessions: &mut discovery_sessions,
                            pull_cancellations: &mut pull_cancellations,
                        },
                        session_id,
                        Utc::now(),
                    )
                };
                if let Some(promoted) = promoted {
                    self.publish_promoted(daemon_id, site_id, promoted).await;
                }

                // Broadcast cancellation update so frontend knows, and publish the terminal
                // event: it carries `UserCancelled`, which the metrics subscriber counts and the
                // daemon subscriber leaves alone, since the daemon never had this session.
                self.send_session_update(&cancelled_update);
                self.publish_session_event(
                    cancelled_update.into_discovery_event_with_auth(authentication),
                )
                .await;

                // A rescan cancelled before dispatch never reaches the
                // daemon-reported terminal finalization that deletes its transient
                // parent, so the AdHoc Rescan row would linger in the UI with only a
                // delete action. Reap it here, matching what completion does.
                if is_rescan
                    && let Some(discovery_id) = discovery_id
                    && let Err(e) = self.discovery_storage.delete(&discovery_id).await
                {
                    tracing::warn!(
                        discovery_id = %discovery_id,
                        error = ?e,
                        "Failed to delete cancelled transient rescan; the startup sweeper will retry"
                    );
                }
                Ok(())
            }

            // Starting phase: wait briefly then retry
            DiscoveryPhase::Starting => Err(anyhow!(
                "Session is starting on daemon. Please try again in a moment."
            )),

            // Active phases: send cancellation to daemon
            // We do BOTH actions to support both daemon modes:
            // 1. Publish DiscoveryCancelled event - DaemonService subscriber handles ServerPoll mode
            // 2. Set cancellation flag - DaemonPoll mode checks on next poll via request_work
            DiscoveryPhase::Started | DiscoveryPhase::Scanning => {
                // A request, not the ending: the session ends when the daemon reports it, and
                // that event carries the reason. Without one here, the terminal metric counts the
                // cancel once.
                let request = DiscoveryUpdatePayload {
                    reason: None,
                    ..cancelled_update
                };
                self.event_bus()
                    .publish(request.into_discovery_event_with_auth(authentication))
                    .await;

                // Sent on every DaemonPoll work request until the session ends, and timed in both
                // modes against `state::CANCEL_DEADLINE` by the stall sweep.
                state::request_cancel(
                    &mut *self.daemon_pull_cancellations.write().await,
                    daemon_id,
                    session_id,
                    Utc::now(),
                );

                Ok(())
            }

            // Terminal phases: already done
            DiscoveryPhase::Complete | DiscoveryPhase::Failed | DiscoveryPhase::Cancelled => {
                tracing::info!(
                    "Session {} is already in terminal state: {}, nothing to cancel",
                    session_id,
                    phase
                );
                Ok(())
            }
        }
    }
}
