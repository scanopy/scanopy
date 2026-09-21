//! The session state machine, as synchronous functions over the in-memory maps.
//!
//! `DiscoveryService` keeps session state in five maps behind separate `RwLock`s. Every
//! transition here runs with the guards already held, so the steps a transition depends on (a
//! promotion that is only correct because the finished session was just removed from the same
//! queue, a stamp the stall sweep reads next tick) happen inside one synchronous call that no
//! `.await` can split. The async work a transition triggers (storage writes, event publishes) comes
//! back as data for the caller to run after the guards drop, which is what keeps a slow subscriber
//! from holding the locks every reader waits on.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::discovery::r#impl::types::DiscoveryType;

/// The five session maps, borrowed from guards the caller holds.
pub(crate) struct SessionMaps<'a> {
    pub sessions: &'a mut HashMap<Uuid, DiscoveryUpdatePayload>,
    pub last_updated: &'a mut HashMap<Uuid, DateTime<Utc>>,
    pub daemon_sessions: &'a mut HashMap<Uuid, Vec<Uuid>>,
    pub discovery_sessions: &'a mut HashMap<Uuid, Uuid>,
    pub pull_cancellations: &'a mut HashMap<Uuid, (bool, Uuid)>,
}

/// What applying one daemon update did to the maps.
pub(crate) enum UpdateOutcome {
    /// A repeat of a terminal update already processed: nothing changed but the stamp.
    Ignored,
    /// The session is recorded and still running.
    Progress,
    /// The session reached a terminal phase and has left the maps.
    Terminal(Box<TerminalEffects>),
}

/// Everything the post-lock work of a finished session needs, captured while the maps still held it.
pub(crate) struct TerminalEffects {
    /// The session as recorded once the update was applied.
    pub session: DiscoveryUpdatePayload,
    /// The discovery configuration the session ran for. Read here because the mapping it comes from
    /// is cleared once the session's post-lock work is done.
    pub parent_discovery_id: Option<Uuid>,
    /// The queued session promoted to `Pending` in its place, if any.
    pub promoted: Option<PromotedSession>,
}

/// A queued session moved to the front of its daemon's queue.
pub(crate) struct PromotedSession {
    pub session_id: Uuid,
    pub discovery_type: DiscoveryType,
    pub discovery_id: Option<Uuid>,
}

/// Apply a daemon's update to the maps.
///
/// A session the server has never seen is created from the update, which is how a session
/// survives a server restart. A terminal update for a session that has already finished and left
/// the maps is a redundant re-delivery (old daemons repeat their terminal payload) and is ignored.
/// Either way the stamp in `last_updated` is refreshed first, so a daemon that keeps talking keeps
/// its session's tombstone alive.
///
/// On a terminal update the session leaves `sessions` and its daemon's queue, the daemon's pending
/// pull cancellation is dropped (a cancel for a session that already ended must not land on the
/// next one), and the next session in the queue is promoted. The session's `discovery_sessions`
/// entry is left for the caller to remove.
pub(crate) fn apply_update(
    maps: &mut SessionMaps,
    update: &DiscoveryUpdatePayload,
    now: DateTime<Utc>,
) -> UpdateOutcome {
    let session_id = update.session_id;
    let already_seen = maps.last_updated.contains_key(&session_id);
    maps.last_updated.insert(session_id, now);

    if !maps.sessions.contains_key(&session_id) {
        if update.phase.is_terminal() && already_seen {
            tracing::debug!(
                session_id = %session_id,
                phase = %update.phase,
                "Ignoring redundant terminal update (already processed)"
            );
            return UpdateOutcome::Ignored;
        }

        tracing::info!(
            session_id = %session_id,
            daemon_id = %update.daemon_id,
            network_id = %update.network_id,
            "Auto-creating session from daemon update"
        );
        maps.daemon_sessions
            .entry(update.daemon_id)
            .or_default()
            .push(session_id);
        if let Some(discovery_id) = update.discovery_id {
            maps.discovery_sessions.insert(discovery_id, session_id);
        }
        maps.sessions.insert(session_id, update.clone());
    }

    let Some(session) = maps.sessions.get_mut(&session_id) else {
        unreachable!("the session was present or inserted above");
    };
    *session = update.clone();

    if !session.phase.is_terminal() {
        return UpdateOutcome::Progress;
    }

    let session = session.clone();
    let parent_discovery_id = maps
        .discovery_sessions
        .iter()
        .find(|(_, sid)| **sid == session_id)
        .map(|(did, _)| *did);
    maps.pull_cancellations.remove(&session.daemon_id);
    let promoted = remove_from_queue_and_promote_head(maps, session.daemon_id, session_id, now);
    maps.sessions.remove(&session_id);

    UpdateOutcome::Terminal(Box::new(TerminalEffects {
        session,
        parent_discovery_id,
        promoted,
    }))
}

/// Remove a finished session from its daemon's queue and promote whichever session is now at the
/// head, stamping it so the stall sweep times it from its promotion rather than from when it was
/// queued.
fn remove_from_queue_and_promote_head(
    maps: &mut SessionMaps,
    daemon_id: Uuid,
    finished_session_id: Uuid,
    now: DateTime<Utc>,
) -> Option<PromotedSession> {
    let queue = maps.daemon_sessions.get_mut(&daemon_id)?;
    queue.retain(|id| *id != finished_session_id);

    let next = queue
        .first()
        .and_then(|next_id| maps.sessions.get_mut(next_id))?;
    next.phase = DiscoveryPhase::Pending;
    maps.last_updated.insert(next.session_id, now);

    Some(PromotedSession {
        session_id: next.session_id,
        discovery_type: next.discovery_type.clone(),
        discovery_id: next.discovery_id,
    })
}

/// Sessions the daemon has gone quiet on for longer than `threshold`.
///
/// Only phases where a daemon is expected to be making progress qualify (see
/// [`DiscoveryPhase::can_be_cleaned_up`]). `Queued` and `AwaitingSnapshot` are never stamped while
/// they wait, so admitting them would reap every one on the first sweep.
pub(crate) fn select_stalled(
    sessions: &HashMap<Uuid, DiscoveryUpdatePayload>,
    last_updated: &HashMap<Uuid, DateTime<Utc>>,
    now: DateTime<Utc>,
    threshold: Duration,
) -> Vec<DiscoveryUpdatePayload> {
    sessions
        .iter()
        .filter(|(_, session)| session.phase.can_be_cleaned_up())
        .filter(|(session_id, session)| {
            if let Some(last_update) = last_updated.get(*session_id) {
                now.signed_duration_since(*last_update) > threshold
            } else if let Some(started_at) = session.started_at {
                now.signed_duration_since(started_at) > threshold
            } else {
                // Dispatched but never reported back.
                tracing::warn!(
                    session_id = %session_id,
                    phase = ?session.phase,
                    "Session has no tracking timestamps, treating as stalled"
                );
                true
            }
        })
        .map(|(_, session)| session.clone())
        .collect()
}

/// Fail and remove stalled sessions, returning them as recorded.
///
/// Each reaped session leaves every map, and the head of its daemon's queue is promoted if it was
/// waiting in `Queued`.
pub(crate) fn reap(
    maps: &mut SessionMaps,
    stalled: &[Uuid],
    now: DateTime<Utc>,
) -> Vec<DiscoveryUpdatePayload> {
    let mut reaped = Vec::new();

    for session_id in stalled {
        let Some(mut session) = maps.sessions.remove(session_id) else {
            continue;
        };
        let daemon_id = session.daemon_id;

        tracing::warn!(
            session_id = %session_id,
            daemon_id = %daemon_id,
            phase = ?session.phase,
            "Cleaning up stalled discovery session (no updates for 5+ minutes)"
        );

        session.phase = DiscoveryPhase::Failed;
        session.error = Some(
            "Session stalled - no updates received from daemon for more than 5 minutes".to_string(),
        );
        session.finished_at = Some(now);

        if let Some(queue) = maps.daemon_sessions.get_mut(&daemon_id) {
            queue.retain(|id| id != session_id);
            if let Some(next) = queue
                .first()
                .and_then(|next_id| maps.sessions.get_mut(next_id))
                && next.phase == DiscoveryPhase::Queued
            {
                next.phase = DiscoveryPhase::Pending;
                maps.last_updated.insert(next.session_id, now);
            }
        }

        maps.discovery_sessions.retain(|_, sid| sid != session_id);
        maps.last_updated.remove(session_id);

        if let Some((_, cancel_session_id)) = maps.pull_cancellations.get(&daemon_id)
            && cancel_session_id == session_id
        {
            maps.pull_cancellations.remove(&daemon_id);
            tracing::debug!(
                daemon_id = %daemon_id,
                session_id = %session_id,
                "Removed stale cancellation flag"
            );
        }

        reaped.push(session);
    }

    reaped
}

/// Drop tombstones: stamps for sessions that have left `sessions` and gone quiet for longer than
/// `threshold`. A tombstone is what lets a redundant terminal re-delivery be recognised and ignored.
pub(crate) fn evict_tombstones(
    sessions: &HashMap<Uuid, DiscoveryUpdatePayload>,
    last_updated: &mut HashMap<Uuid, DateTime<Utc>>,
    now: DateTime<Utc>,
    threshold: Duration,
) {
    last_updated
        .retain(|id, ts| sessions.contains_key(id) || now.signed_duration_since(*ts) < threshold);
}

#[cfg(test)]
mod tests {
    use super::*;

    const THRESHOLD_MINUTES: i64 = 5;

    fn threshold() -> Duration {
        Duration::minutes(THRESHOLD_MINUTES)
    }

    #[derive(Default)]
    struct Maps {
        sessions: HashMap<Uuid, DiscoveryUpdatePayload>,
        last_updated: HashMap<Uuid, DateTime<Utc>>,
        daemon_sessions: HashMap<Uuid, Vec<Uuid>>,
        discovery_sessions: HashMap<Uuid, Uuid>,
        pull_cancellations: HashMap<Uuid, (bool, Uuid)>,
    }

    impl Maps {
        fn borrow(&mut self) -> SessionMaps<'_> {
            SessionMaps {
                sessions: &mut self.sessions,
                last_updated: &mut self.last_updated,
                daemon_sessions: &mut self.daemon_sessions,
                discovery_sessions: &mut self.discovery_sessions,
                pull_cancellations: &mut self.pull_cancellations,
            }
        }

        /// Record a session the way `start_session` does: in the maps, at the back of its daemon's
        /// queue, mapped from its discovery.
        fn start(&mut self, daemon_id: Uuid, phase: DiscoveryPhase) -> DiscoveryUpdatePayload {
            let mut session = DiscoveryUpdatePayload::new(
                Uuid::new_v4(),
                daemon_id,
                Uuid::new_v4(),
                DiscoveryType::default(),
                Some(Uuid::new_v4()),
            );
            session.phase = phase;
            self.sessions.insert(session.session_id, session.clone());
            self.daemon_sessions
                .entry(daemon_id)
                .or_default()
                .push(session.session_id);
            self.discovery_sessions
                .insert(session.discovery_id.unwrap(), session.session_id);
            session
        }
    }

    fn with_phase(
        session: &DiscoveryUpdatePayload,
        phase: DiscoveryPhase,
    ) -> DiscoveryUpdatePayload {
        let mut update = session.clone();
        update.phase = phase;
        update
    }

    #[test]
    fn a_terminal_update_removes_the_session_and_returns_it_as_updated() {
        let mut maps = Maps::default();
        let daemon = Uuid::new_v4();
        let running = maps.start(daemon, DiscoveryPhase::Scanning);
        let mut update = with_phase(&running, DiscoveryPhase::Complete);
        update.progress = 100;
        update.hosts_discovered = Some(12);

        let UpdateOutcome::Terminal(effects) =
            apply_update(&mut maps.borrow(), &update, Utc::now())
        else {
            panic!("a Complete update is terminal");
        };

        assert!(!maps.sessions.contains_key(&running.session_id));
        assert_eq!(effects.session.progress, 100);
        assert_eq!(effects.session.hosts_discovered, Some(12));
        assert_eq!(effects.parent_discovery_id, running.discovery_id);
    }

    #[test]
    fn a_terminal_update_promotes_the_next_queued_session_and_starts_its_clock() {
        let mut maps = Maps::default();
        let daemon = Uuid::new_v4();
        let running = maps.start(daemon, DiscoveryPhase::Scanning);
        let queued = maps.start(daemon, DiscoveryPhase::Queued);
        let now = Utc::now();

        let UpdateOutcome::Terminal(effects) = apply_update(
            &mut maps.borrow(),
            &with_phase(&running, DiscoveryPhase::Complete),
            now,
        ) else {
            panic!("a Complete update is terminal");
        };

        assert_eq!(
            maps.sessions[&queued.session_id].phase,
            DiscoveryPhase::Pending
        );
        assert_eq!(maps.last_updated.get(&queued.session_id), Some(&now));
        assert_eq!(maps.daemon_sessions[&daemon], vec![queued.session_id]);
        assert_eq!(
            effects.promoted.map(|p| p.session_id),
            Some(queued.session_id)
        );
    }

    #[test]
    fn a_repeated_terminal_update_for_a_finished_session_is_ignored() {
        let mut maps = Maps::default();
        let running = maps.start(Uuid::new_v4(), DiscoveryPhase::Scanning);
        let complete = with_phase(&running, DiscoveryPhase::Complete);
        apply_update(&mut maps.borrow(), &complete, Utc::now());

        let outcome = apply_update(&mut maps.borrow(), &complete, Utc::now());

        assert!(matches!(outcome, UpdateOutcome::Ignored));
        assert!(!maps.sessions.contains_key(&running.session_id));
    }

    #[test]
    fn an_update_for_a_session_the_server_never_saw_recreates_it() {
        // A server restart empties every map while the daemon keeps scanning.
        let mut maps = Maps::default();
        let mut update = DiscoveryUpdatePayload::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            DiscoveryType::default(),
            Some(Uuid::new_v4()),
        );
        update.phase = DiscoveryPhase::Scanning;

        let outcome = apply_update(&mut maps.borrow(), &update, Utc::now());

        assert!(matches!(outcome, UpdateOutcome::Progress));
        assert!(maps.sessions.contains_key(&update.session_id));
        assert_eq!(
            maps.daemon_sessions[&update.daemon_id],
            vec![update.session_id]
        );
        assert_eq!(
            maps.discovery_sessions.get(&update.discovery_id.unwrap()),
            Some(&update.session_id)
        );
    }

    #[test]
    fn the_stall_sweep_never_selects_sessions_that_are_waiting_their_turn() {
        let mut maps = Maps::default();
        let daemon = Uuid::new_v4();
        let queued = maps.start(daemon, DiscoveryPhase::Queued);
        let awaiting = maps.start(Uuid::new_v4(), DiscoveryPhase::AwaitingSnapshot);
        let silent = maps.start(Uuid::new_v4(), DiscoveryPhase::Scanning);
        let long_ago = Utc::now() - Duration::hours(3);
        for id in [queued.session_id, awaiting.session_id, silent.session_id] {
            maps.last_updated.insert(id, long_ago);
        }

        let stalled: Vec<Uuid> =
            select_stalled(&maps.sessions, &maps.last_updated, Utc::now(), threshold())
                .into_iter()
                .map(|s| s.session_id)
                .collect();

        assert_eq!(stalled, vec![silent.session_id]);
    }

    #[test]
    fn reaping_a_stalled_session_promotes_the_next_queued_one_and_starts_its_clock() {
        let mut maps = Maps::default();
        let daemon = Uuid::new_v4();
        let stalled = maps.start(daemon, DiscoveryPhase::Scanning);
        let queued = maps.start(daemon, DiscoveryPhase::Queued);
        let now = Utc::now();

        let reaped = reap(&mut maps.borrow(), &[stalled.session_id], now);

        assert_eq!(reaped.len(), 1);
        assert_eq!(reaped[0].phase, DiscoveryPhase::Failed);
        assert!(!maps.sessions.contains_key(&stalled.session_id));
        assert_eq!(
            maps.sessions[&queued.session_id].phase,
            DiscoveryPhase::Pending
        );
        assert_eq!(maps.last_updated.get(&queued.session_id), Some(&now));
    }
}
