use anyhow::anyhow;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::daemon::discovery::service::base::{DaemonDiscoveryService, DiscoveryRunner};
use crate::daemon::discovery::service::ops::{DiscoveryAbort, DiscoveryOps};
use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;
use crate::server::daemons::r#impl::api::DaemonDiscoveryRequest;
use crate::server::discovery::r#impl::scan_settings::{ScanSettings, defaults};
use crate::server::discovery::r#impl::types::DiscoveryType;

/// How far past a session's own maximum duration the watchdog waits before stopping it. The scan
/// enforces that maximum itself inside the network phase; the margin lets that check fire first
/// whenever it can, so the watchdog only acts on a session stuck somewhere the check never runs.
const WATCHDOG_MARGIN: Duration = Duration::from_secs(10 * 60);

/// How long a session may run without advancing before the watchdog stops it. Above the longest
/// stretch a healthy scan goes without advancing: one SNMP execute, capped at 15 minutes.
const SESSION_ACTIVITY_LIMIT: Duration = Duration::from_secs(20 * 60);

/// How long a cancelled session has to stop on its own before the watchdog aborts it.
const CANCEL_GRACE: Duration = Duration::from_secs(30);

/// How often the watchdog checks the running session.
const WATCH_INTERVAL: Duration = Duration::from_secs(5);

/// What happened when the daemon was asked to start a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitiateOutcome {
    Started,
    /// Another session is running, so this one was refused.
    Busy {
        running: Uuid,
        age: Duration,
    },
}

/// Which session a cancellation is for.
#[derive(Debug, Clone, Copy)]
pub enum CancelTarget {
    /// This session, and only while it is the one running. A late cancel for a session that has
    /// since finished must not land on whatever started next.
    Session(Uuid),
    /// Whatever is running. For a server too old to name the session it cancels.
    Current,
}

impl CancelTarget {
    /// What a DaemonPoll work response's cancellation reaches, given the session running now. A
    /// response that names a session cancels only that one; one from an older server, which
    /// names none, cancels whatever is running.
    pub fn from_work_response(
        running: Option<Uuid>,
        cancel: bool,
        cancel_session_id: Option<Uuid>,
    ) -> Option<Self> {
        if !cancel {
            return None;
        }
        let running = running?;
        match cancel_session_id {
            Some(id) if id == running => Some(CancelTarget::Session(id)),
            Some(_) => None,
            None => Some(CancelTarget::Current),
        }
    }
}

pub struct DaemonDiscoverySessionManager {
    slot: SessionSlot,
    discovery_service: Arc<DaemonDiscoveryService>,
}

impl DaemonDiscoverySessionManager {
    pub fn new(discovery_service: Arc<DaemonDiscoveryService>) -> Self {
        Self {
            slot: SessionSlot::default(),
            discovery_service,
        }
    }

    pub fn discovery_service(&self) -> &Arc<DaemonDiscoveryService> {
        &self.discovery_service
    }

    /// Start a session unless another is already running.
    pub async fn try_initiate_session(
        self: &Arc<Self>,
        request: DaemonDiscoveryRequest,
    ) -> InitiateOutcome {
        // Checked before the banner so a refused request does not print one. `install` checks
        // again under its lock, which is what makes the decision atomic.
        if let Some((running, age)) = self.slot.running().await {
            return self.refuse(&request, running, age);
        }

        log_session_banner(&request);

        let session_id = request.session_id;
        let cap = watchdog_cap(&request.discovery_type);
        let installed = self
            .slot
            .install(session_id, request.discovery_type.clone(), |token| {
                self.spawn_session(request.clone(), token)
            })
            .await;

        match installed {
            Ok(()) => {
                self.watch(session_id, cap);
                InitiateOutcome::Started
            }
            Err(Busy { running, age }) => self.refuse(&request, running, age),
        }
    }

    fn refuse(
        &self,
        request: &DaemonDiscoveryRequest,
        running: Uuid,
        age: Duration,
    ) -> InitiateOutcome {
        tracing::warn!(
            session_id = %request.session_id,
            discovery_type = %request.discovery_type,
            running_session_id = %running,
            running_for_secs = age.as_secs(),
            "Refusing discovery session: another session is already running on this daemon"
        );
        InitiateOutcome::Busy { running, age }
    }

    fn spawn_session(
        self: &Arc<Self>,
        request: DaemonDiscoveryRequest,
        cancel_token: CancellationToken,
    ) -> JoinHandle<()> {
        match &request.discovery_type {
            // Legacy types: log warning and complete immediately
            DiscoveryType::SelfReport { .. }
            | DiscoveryType::Docker { .. }
            | DiscoveryType::Network { .. } => {
                tracing::warn!(
                    "Received legacy discovery type '{}', completing session immediately. \
                     This daemon only supports unified discovery.",
                    request.discovery_type
                );
                self.clone().spawn_legacy_stub(request, cancel_token)
            }
            DiscoveryType::Unified { .. }
            | DiscoveryType::Rescan { .. }
            | DiscoveryType::InterfaceRefresh { .. } => {
                // `new` returns None only for the legacy types handled above.
                let Some(runner) = DiscoveryRunner::new(
                    self.discovery_service.clone(),
                    self.clone(),
                    request.discovery_type.clone(),
                    request.credential_mappings.clone(),
                    request.subnets.clone(),
                ) else {
                    unreachable!("legacy discovery types are stubbed in the arm above")
                };
                self.clone().spawn_discovery(runner, request, cancel_token)
            }
        }
    }

    /// Spawn a lightweight stub for legacy discovery types that just reports completion
    fn spawn_legacy_stub(
        self: Arc<Self>,
        request: DaemonDiscoveryRequest,
        cancel_token: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let ops = DiscoveryOps::new(&self.discovery_service, request.discovery_type.clone());
            // Initialize session and immediately complete it
            if let Err(e) = ops.start_session(&request, Vec::new()).await {
                tracing::error!("Failed to start legacy stub session: {}", e);
            } else if let Err(e) = ops.finish_session(Ok(()), cancel_token).await {
                tracing::error!("Failed to finish legacy stub session: {}", e);
            }
            self.slot.clear(request.session_id).await;
        })
    }

    fn spawn_discovery(
        self: Arc<Self>,
        mut discovery: DiscoveryRunner,
        request: DaemonDiscoveryRequest,
        cancel_token: CancellationToken,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let session_id = request.session_id;
            match discovery.discover(request, cancel_token).await {
                Ok(()) => {
                    tracing::info!(session_id = %session_id, "Discovery completed successfully");
                }
                Err(e) => {
                    tracing::error!(session_id = %session_id, error = %e, "Discovery failed");
                }
            }
            self.slot.clear(session_id).await;
        })
    }

    /// Watch the session until it leaves the slot, and end it when [`expiry_due`] says so. Runs
    /// outside the session's task, so a wedge inside the task cannot stop it.
    fn watch(self: &Arc<Self>, session_id: Uuid, cap: Duration) {
        let manager = Arc::downgrade(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(WATCH_INTERVAL);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                let Some(manager) = manager.upgrade() else {
                    return;
                };
                if !manager
                    .sweep(
                        session_id,
                        cap,
                        Instant::now(),
                        crate::server::shared::clock::now(),
                    )
                    .await
                {
                    return;
                }
            }
        });
    }

    /// One watchdog check of `session_id` at `now`. Returns whether it is still worth watching:
    /// false once the session has left the slot or this check ended it.
    async fn sweep(
        &self,
        session_id: Uuid,
        cap: Duration,
        now: Instant,
        now_utc: DateTime<Utc>,
    ) -> bool {
        let Some((started_at, cancel_requested_at)) = self.slot.watch_state(session_id).await
        else {
            return false;
        };
        let progress = self.session_progress(session_id).await;
        let watched = Watched {
            started_at,
            network_phase_started: progress.as_ref().and_then(|p| p.network_phase_started),
            cancel_requested_at,
            quiet: progress.as_ref().map(|p| {
                now_utc
                    .signed_duration_since(p.last_activity_at)
                    .to_std()
                    .unwrap_or_default()
            }),
        };
        let Some(expiry) = expiry_due(now, cap, &watched) else {
            return true;
        };
        let in_flight = progress.map(|p| p.in_flight).unwrap_or_default();
        self.expire(session_id, expiry, in_flight).await;
        false
    }

    /// What the running session, if it is `session_id`, says about how it is advancing.
    async fn session_progress(&self, session_id: Uuid) -> Option<SessionProgress> {
        let current = self.discovery_service.current_session.read().await;
        current
            .as_ref()
            .filter(|session| session.info.session_id == session_id)
            .map(|session| SessionProgress {
                network_phase_started: session.network_phase_started.get().copied(),
                last_activity_at: session.last_activity_at(),
                in_flight: session.in_flight_deep_scans(),
            })
    }

    /// End a session the watchdog gave up on. Aborting the task skips everything it would have
    /// done to finish, including reporting the outcome (ServerPoll's only delivery path is the
    /// stored terminal payload), so the watchdog finishes the session itself.
    async fn expire(&self, session_id: Uuid, expiry: Expiry, in_flight: Vec<IpAddr>) {
        let Some(session) = self.slot.take(session_id).await else {
            return;
        };
        let elapsed = session.started_at.elapsed();
        // Cancel first: workers the session spawned (sender threads, drains, SNMP walks) watch the
        // token and outlive an abort of the task that spawned them.
        session.token.cancel();
        session.handle.abort();

        let (result, token) = match expiry {
            Expiry::CancelIgnored => {
                tracing::warn!(
                    session_id = %session_id,
                    grace_secs = CANCEL_GRACE.as_secs(),
                    "Discovery session did not stop after its cancellation and was aborted"
                );
                // The cancelled token reports the session as cancelled, which it was.
                (Err(anyhow!("Discovery cancelled")), session.token)
            }
            Expiry::Cap { cap } => {
                tracing::error!(
                    session_id = %session_id,
                    running_for_secs = elapsed.as_secs(),
                    cap_secs = cap.as_secs(),
                    "Discovery session outlived its maximum duration and was stopped"
                );
                let abort = DiscoveryAbort::Watchdog {
                    elapsed_secs: elapsed.as_secs(),
                    cap_secs: cap.as_secs(),
                };
                // A fresh token: a cancelled one would report the session as cancelled.
                (Err(abort.into()), CancellationToken::new())
            }
            Expiry::StoppedAdvancing { quiet } => {
                tracing::error!(
                    session_id = %session_id,
                    quiet_secs = quiet.as_secs(),
                    in_flight = ?in_flight,
                    "Discovery session stopped advancing and was stopped"
                );
                let abort = DiscoveryAbort::StoppedAdvancing {
                    quiet_secs: quiet.as_secs(),
                    in_flight,
                };
                (Err(abort.into()), CancellationToken::new())
            }
        };

        let ops = DiscoveryOps::new(&self.discovery_service, session.discovery_type);
        if let Err(e) = ops.finish_session(result, token).await {
            tracing::warn!(
                session_id = %session_id,
                error = %e,
                "Could not report the stopped session; it may have finished as it was stopped"
            );
        }
    }

    /// Check if discovery is currently running
    pub async fn is_discovery_running(&self) -> bool {
        self.slot.running().await.is_some()
    }

    /// The session running now, if any.
    pub async fn running_session_id(&self) -> Option<Uuid> {
        self.slot.running().await.map(|(id, _)| id)
    }

    /// Signal cancellation to the targeted session. Returns false when it is not running.
    /// Cooperative: the session's collectors observe the token and wind down.
    pub async fn cancel(&self, target: CancelTarget) -> bool {
        let cancelled = self.slot.cancel(target).await;
        if cancelled {
            tracing::info!(target = ?target, "Cancelling discovery session");
        }
        cancelled
    }
}

/// The session running on this daemon, if any.
struct ActiveSession {
    session_id: Uuid,
    discovery_type: DiscoveryType,
    token: CancellationToken,
    handle: JoinHandle<()>,
    started_at: Instant,
    /// When the first cancellation for it arrived. The watchdog aborts a session still running
    /// [`CANCEL_GRACE`] after it.
    cancel_requested_at: Option<Instant>,
}

impl ActiveSession {
    fn is_live(&self) -> bool {
        // A task that panicked or was aborted never clears its own slot; `is_finished` keeps it
        // from reading as running forever.
        !self.handle.is_finished()
    }
}

struct Busy {
    running: Uuid,
    age: Duration,
}

/// The one-session slot. Every check-then-act on it happens under a single acquisition of its
/// lock, which is what stops a cancel or a second start from interleaving with a start.
#[derive(Default)]
struct SessionSlot {
    active: RwLock<Option<ActiveSession>>,
}

impl SessionSlot {
    /// Install a session and start its task, unless one is live. `spawn` runs while the lock is
    /// held; spawning does not await, and the task's own `clear` simply waits its turn.
    async fn install(
        &self,
        session_id: Uuid,
        discovery_type: DiscoveryType,
        spawn: impl FnOnce(CancellationToken) -> JoinHandle<()>,
    ) -> Result<(), Busy> {
        let mut active = self.active.write().await;
        if let Some(current) = active.as_ref().filter(|s| s.is_live()) {
            return Err(Busy {
                running: current.session_id,
                age: current.started_at.elapsed(),
            });
        }

        let token = CancellationToken::new();
        let handle = spawn(token.clone());
        *active = Some(ActiveSession {
            session_id,
            discovery_type,
            token,
            handle,
            started_at: Instant::now(),
            cancel_requested_at: None,
        });
        Ok(())
    }

    async fn cancel(&self, target: CancelTarget) -> bool {
        let mut active = self.active.write().await;
        let Some(session) = active.as_mut().filter(|s| s.is_live()) else {
            return false;
        };
        if let CancelTarget::Session(id) = target
            && id != session.session_id
        {
            return false;
        }
        session.token.cancel();
        session.cancel_requested_at.get_or_insert_with(Instant::now);
        true
    }

    /// Empty the slot, but only if it still holds `session_id`: by the time a finished task gets
    /// here, the slot may already hold the next session.
    ///
    /// Cancels the session's token on the way out, whatever the outcome. The session task has
    /// already reported it, and threads the session spawned (ARP and ICMP senders, drains, SNMP
    /// walks) watch the token: a successful run otherwise left an ARP sender broadcasting to a
    /// large subnet for hours after the run had ended.
    async fn clear(&self, session_id: Uuid) {
        let mut active = self.active.write().await;
        if active.as_ref().is_some_and(|s| s.session_id == session_id)
            && let Some(session) = active.take()
        {
            session.token.cancel();
        }
    }

    /// Remove and return the session if the slot still holds `session_id`.
    async fn take(&self, session_id: Uuid) -> Option<ActiveSession> {
        let mut active = self.active.write().await;
        if active.as_ref().is_some_and(|s| s.session_id == session_id) {
            active.take()
        } else {
            None
        }
    }

    /// When the session was installed and when it was first cancelled, while the slot holds
    /// `session_id`. A task that panicked stays in the slot, so it is still watched and finished.
    async fn watch_state(&self, session_id: Uuid) -> Option<(Instant, Option<Instant>)> {
        self.active
            .read()
            .await
            .as_ref()
            .filter(|s| s.session_id == session_id)
            .map(|s| (s.started_at, s.cancel_requested_at))
    }

    /// The live session's id and how long it has been running.
    async fn running(&self) -> Option<(Uuid, Duration)> {
        self.active
            .read()
            .await
            .as_ref()
            .filter(|s| s.is_live())
            .map(|s| (s.session_id, s.started_at.elapsed()))
    }
}

/// What the running session says about how it is advancing.
struct SessionProgress {
    network_phase_started: Option<Instant>,
    last_activity_at: DateTime<Utc>,
    in_flight: Vec<IpAddr>,
}

/// What the watchdog knows about a session at one check.
struct Watched {
    started_at: Instant,
    network_phase_started: Option<Instant>,
    cancel_requested_at: Option<Instant>,
    /// How long since the session last advanced. `None` before the session exists on the daemon
    /// or after it has finished there.
    quiet: Option<Duration>,
}

/// Why the watchdog ends a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expiry {
    /// It was cancelled and kept running past [`CANCEL_GRACE`].
    CancelIgnored,
    /// It outlived its maximum duration.
    Cap { cap: Duration },
    /// It went [`SESSION_ACTIVITY_LIMIT`] without advancing.
    StoppedAdvancing { quiet: Duration },
}

/// Whether the watchdog ends the session at `now`, and why. A cancellation the session ignored
/// comes first, because the user asked for it to end.
///
/// The network phase's own time limit counts from its start, not the session's, so a long first
/// phase would otherwise put a scan that ends at its limit past the cap. A session stuck before
/// the network phase keeps the install clock.
fn expiry_due(now: Instant, cap: Duration, watched: &Watched) -> Option<Expiry> {
    if watched
        .cancel_requested_at
        .is_some_and(|at| now.saturating_duration_since(at) >= CANCEL_GRACE)
    {
        return Some(Expiry::CancelIgnored);
    }
    let cap_deadline = match watched.network_phase_started {
        Some(started) => (watched.started_at + cap).max(started + cap),
        None => watched.started_at + cap,
    };
    if now >= cap_deadline {
        return Some(Expiry::Cap { cap });
    }
    match watched.quiet {
        Some(quiet) if quiet >= SESSION_ACTIVITY_LIMIT => Some(Expiry::StoppedAdvancing { quiet }),
        _ => None,
    }
}

/// The scan settings a scanning session runs with. A rescan's narrower settings widen back.
fn scan_settings_of(discovery_type: &DiscoveryType) -> Option<ScanSettings> {
    match discovery_type {
        DiscoveryType::Unified { scan_settings, .. } => Some(scan_settings.clone()),
        DiscoveryType::Rescan { settings, .. } => Some(ScanSettings::from(settings)),
        DiscoveryType::InterfaceRefresh { .. }
        | DiscoveryType::SelfReport { .. }
        | DiscoveryType::Network { .. }
        | DiscoveryType::Docker { .. } => None,
    }
}

/// How long a session may run before the watchdog stops it.
fn watchdog_cap(discovery_type: &DiscoveryType) -> Duration {
    let max_secs = scan_settings_of(discovery_type)
        .and_then(|settings| settings.max_discovery_duration)
        .unwrap_or_else(defaults::max_discovery_duration);
    Duration::from_secs(u64::from(max_secs)) + WATCHDOG_MARGIN
}

fn log_session_banner(request: &DaemonDiscoveryRequest) {
    tracing::info!(
        discovery_type = %request.discovery_type,
        session_id = %request.session_id,
        "Initiating discovery"
    );

    // Log session banner — all lines use the manager's tracing target for visual alignment
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("  New Discovery Session");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("  {:<20}{}", "Session ID:", request.session_id);

    // Both scanning types report their settings and credentials the same way.
    if let Some(scan_settings) = scan_settings_of(&request.discovery_type) {
        // Scan settings
        tracing::info!("  ───────────────────────────────────────────────────────────");
        tracing::info!("  Scan Settings:");
        for (label, value, is_override) in scan_settings.formatted_lines() {
            let source = if is_override {
                "(override)"
            } else {
                "(default)"
            };
            tracing::info!("    {:<20}{} {}", label, value, source);
        }

        // Credentials
        if !request.credential_mappings.is_empty() {
            tracing::info!("  ───────────────────────────────────────────────────────────");
            tracing::info!("  Credentials:");
            for mapping in &request.credential_mappings {
                if let Some(ref default) = mapping.default_credential {
                    log_credential_banner(
                        default,
                        &format!("{} on all scanned hosts", default.discovery_label()),
                    );
                }
                // Group IP overrides by credential to avoid duplicate banner output
                let mut grouped: HashMap<&CredentialQueryPayload, Vec<&std::net::IpAddr>> =
                    HashMap::new();
                for ip_override in &mapping.ip_overrides {
                    grouped
                        .entry(&ip_override.credential)
                        .or_default()
                        .push(&ip_override.ip);
                }
                for (credential, ips) in &grouped {
                    let ip_list: Vec<_> = ips.iter().map(|ip| ip.to_string()).collect();
                    let ip_list = ip_list.join(", ");
                    log_credential_banner(
                        credential,
                        &format!("{} on {}", credential.discovery_label(), ip_list),
                    );
                }
            }
        }
    }

    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Log a credential's banner fields with appropriate log levels.
/// FileFailed fields are logged at error level; the header uses warn for visibility.
fn log_credential_banner(credential: &CredentialQueryPayload, context: &str) {
    let lines = credential.banner_lines();
    let has_failures = lines.iter().any(|f| f.value.is_failed());

    if has_failures {
        tracing::warn!("    For {}", context);
    } else {
        tracing::info!("    For {}", context);
    }

    for field in &lines {
        if field.value.is_failed() {
            tracing::error!("      {:<16}{}", field.label, field.value);
        } else {
            tracing::info!("      {:<16}{}", field.label, field.value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};

    /// A task that runs until its token is cancelled, standing in for a discovery.
    fn until_cancelled(token: CancellationToken) -> JoinHandle<()> {
        tokio::spawn(async move { token.cancelled().await })
    }

    #[tokio::test]
    async fn a_cancel_for_a_session_that_is_not_running_leaves_the_running_one_alone() {
        let slot = SessionSlot::default();
        let running = Uuid::new_v4();
        let mut running_token = None;
        slot.install(running, DiscoveryType::default(), |token| {
            running_token = Some(token.clone());
            until_cancelled(token)
        })
        .await
        .ok()
        .unwrap();

        let cancelled = slot.cancel(CancelTarget::Session(Uuid::new_v4())).await;

        assert!(!cancelled);
        assert!(!running_token.unwrap().is_cancelled());
    }

    /// Threads a session spawned watch its token, and a run that ends on its own never cancels
    /// it. Clearing the finished session is what stops them. A late clear for an older session
    /// must not reach the one now running.
    #[tokio::test]
    async fn clearing_a_finished_session_stops_the_workers_watching_its_token() {
        let slot = SessionSlot::default();
        let finished = Uuid::new_v4();
        let mut worker_token = None;
        slot.install(finished, DiscoveryType::default(), |token| {
            worker_token = Some(token.clone());
            tokio::spawn(async {})
        })
        .await
        .ok()
        .unwrap();
        let worker_token = worker_token.unwrap();

        slot.clear(Uuid::new_v4()).await;
        assert!(!worker_token.is_cancelled());

        slot.clear(finished).await;
        assert!(worker_token.is_cancelled());
        assert!(slot.running().await.is_none());
    }

    #[tokio::test]
    async fn a_second_session_is_refused_while_one_is_running() {
        let slot = SessionSlot::default();
        let running = Uuid::new_v4();
        slot.install(running, DiscoveryType::default(), until_cancelled)
            .await
            .ok()
            .unwrap();

        let second = slot
            .install(Uuid::new_v4(), DiscoveryType::default(), until_cancelled)
            .await;

        assert!(matches!(second, Err(Busy { running: id, .. }) if id == running));
    }

    #[tokio::test]
    async fn a_finished_session_clearing_late_does_not_evict_the_next_one() {
        let slot = SessionSlot::default();
        let first = Uuid::new_v4();
        slot.install(first, DiscoveryType::default(), |_| tokio::spawn(async {}))
            .await
            .ok()
            .unwrap();
        // Let the first task finish so the slot will take a new session.
        while slot.running().await.is_some() {
            tokio::task::yield_now().await;
        }
        let next = Uuid::new_v4();
        slot.install(next, DiscoveryType::default(), until_cancelled)
            .await
            .ok()
            .unwrap();

        slot.clear(first).await;

        assert_eq!(slot.running().await.map(|(id, _)| id), Some(next));
    }

    #[tokio::test]
    async fn cancelling_the_running_session_by_id_signals_it() {
        let slot = SessionSlot::default();
        let running = Uuid::new_v4();
        slot.install(running, DiscoveryType::default(), until_cancelled)
            .await
            .ok()
            .unwrap();

        assert!(slot.cancel(CancelTarget::Session(running)).await);
        // The task ends once its token is cancelled, and a finished task no longer reads as
        // running even though nothing has cleared its slot.
        while slot.running().await.is_some() {
            tokio::task::yield_now().await;
        }
    }

    /// A work future that never looks at its token: what a wedged await looks like to the slot.
    fn ignores_its_token(_token: CancellationToken) -> JoinHandle<()> {
        tokio::spawn(std::future::pending::<()>())
    }

    /// A manager whose slot runs `session_id` as a future that ignores cancellation, with the
    /// session live on the daemon.
    async fn wedged_manager(session_id: Uuid) -> Arc<DaemonDiscoverySessionManager> {
        let service = DaemonDiscoveryService::running_for_test(session_id).await;
        let manager = Arc::new(DaemonDiscoverySessionManager::new(service));
        manager
            .slot
            .install(session_id, DiscoveryType::default(), ignores_its_token)
            .await
            .ok()
            .unwrap();
        manager
    }

    async fn terminal(
        manager: &DaemonDiscoverySessionManager,
    ) -> Option<crate::server::daemons::r#impl::api::DiscoveryUpdatePayload> {
        manager
            .discovery_service
            .terminal_payload
            .read()
            .await
            .clone()
    }

    /// The heartbeat proves the task is alive, not that the scan moves, so a scan wedged on one
    /// host used to run until the 6-hour cap. It ends once it has gone the activity limit without
    /// advancing, and says which hosts it was still deep-scanning.
    #[tokio::test]
    async fn a_session_that_stops_advancing_is_ended_naming_the_hosts_it_was_on() {
        let session_id = Uuid::new_v4();
        let manager = wedged_manager(session_id).await;
        let session = manager.discovery_service.get_session().await.unwrap();
        let _in_flight = session.track_deep_scan(IpAddr::from([10, 0, 0, 7]));
        let quiet_since = crate::server::shared::clock::now() - chrono::Duration::minutes(21);
        session.last_activity_ms.store(
            quiet_since.timestamp_millis(),
            std::sync::atomic::Ordering::Relaxed,
        );

        let still_watched = manager
            .sweep(
                session_id,
                watchdog_cap(&DiscoveryType::default()),
                Instant::now(),
                crate::server::shared::clock::now(),
            )
            .await;

        assert!(!still_watched);
        assert!(manager.slot.running().await.is_none());
        let terminal = terminal(&manager).await.expect("the stop was reported");
        assert_eq!(terminal.phase, DiscoveryPhase::Failed);
        assert_eq!(
            terminal.reason,
            Some(DiscoveryTerminalReason::SessionStoppedAdvancing)
        );
        assert!(terminal.error.unwrap_or_default().contains("10.0.0.7"));
    }

    /// Cancel only signals the token, so a session stuck in an await that ignores it kept running
    /// and kept the server's queue blocked. Past the grace period the watchdog aborts it and
    /// reports it cancelled.
    #[tokio::test]
    async fn a_cancelled_session_that_keeps_running_is_ended_as_cancelled() {
        let session_id = Uuid::new_v4();
        let manager = wedged_manager(session_id).await;
        let cap = watchdog_cap(&DiscoveryType::default());
        assert!(manager.cancel(CancelTarget::Session(session_id)).await);

        assert!(
            manager
                .sweep(
                    session_id,
                    cap,
                    Instant::now(),
                    crate::server::shared::clock::now()
                )
                .await
        );
        assert!(manager.slot.running().await.is_some());

        let past_grace = Instant::now() + CANCEL_GRACE + Duration::from_secs(1);
        assert!(
            !manager
                .sweep(
                    session_id,
                    cap,
                    past_grace,
                    crate::server::shared::clock::now()
                )
                .await
        );

        assert!(manager.slot.running().await.is_none());
        let terminal = terminal(&manager).await.expect("the cancel was reported");
        assert_eq!(terminal.phase, DiscoveryPhase::Cancelled);
    }

    #[test]
    fn a_quiet_stretch_is_allowed_up_to_the_activity_limit() {
        let now = Instant::now();
        let watched = |quiet| Watched {
            started_at: now,
            network_phase_started: None,
            cancel_requested_at: None,
            quiet: Some(quiet),
        };
        let cap = Duration::from_secs(6 * 3600);
        // One SNMP execute, the longest a healthy scan goes without advancing.
        assert_eq!(
            expiry_due(now, cap, &watched(Duration::from_secs(900))),
            None
        );
        assert_eq!(
            expiry_due(now, cap, &watched(SESSION_ACTIVITY_LIMIT)),
            Some(Expiry::StoppedAdvancing {
                quiet: SESSION_ACTIVITY_LIMIT
            })
        );
    }

    /// The server's cancellation used to land on whatever was running, so a cancel for a session
    /// that had already ended stopped the next one.
    #[test]
    fn a_cancel_that_names_a_session_reaches_only_that_session() {
        let running = Uuid::new_v4();
        let other = Uuid::new_v4();

        assert!(matches!(
            CancelTarget::from_work_response(Some(running), true, Some(running)),
            Some(CancelTarget::Session(id)) if id == running
        ));
        assert!(CancelTarget::from_work_response(Some(running), true, Some(other)).is_none());
        assert!(CancelTarget::from_work_response(Some(running), false, None).is_none());
        assert!(CancelTarget::from_work_response(None, true, Some(running)).is_none());
        // An older server names no session.
        assert!(matches!(
            CancelTarget::from_work_response(Some(running), true, None),
            Some(CancelTarget::Current)
        ));
    }
}
