use std::collections::HashMap;
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
    /// Whatever is running. For DaemonPoll, whose cancellation carries no session id.
    Current,
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
            DiscoveryType::Unified { .. } | DiscoveryType::Rescan { .. } => {
                // `new` returns None only for the legacy types handled above.
                let Some(runner) = DiscoveryRunner::new(
                    self.discovery_service.clone(),
                    self.clone(),
                    request.discovery_type.clone(),
                    request.credential_mappings.clone(),
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

    /// Stop the session if it is still running once it has outlived `cap`.
    fn watch(self: &Arc<Self>, session_id: Uuid, cap: Duration) {
        let manager = Arc::downgrade(self);
        let installed_at = Instant::now();
        tokio::spawn(async move {
            let mut deadline = installed_at + cap;
            loop {
                tokio::time::sleep_until(deadline.into()).await;
                let Some(manager) = manager.upgrade() else {
                    return;
                };
                // The network phase's own time limit counts from its start, not the session's,
                // so a long first phase would otherwise put a scan that ends at its limit past
                // the cap. A session stuck before the network phase keeps the install clock.
                let latest = match manager.network_phase_started(session_id).await {
                    Some(started) => deadline.max(started + cap),
                    None => deadline,
                };
                if Instant::now() < latest {
                    deadline = latest;
                    continue;
                }
                manager.expire(session_id, cap).await;
                return;
            }
        });
    }

    /// When the running session's network phase began, if it is `session_id` and has begun.
    async fn network_phase_started(&self, session_id: Uuid) -> Option<Instant> {
        let current = self.discovery_service.current_session.read().await;
        current
            .as_ref()
            .filter(|session| session.info.session_id == session_id)
            .and_then(|session| session.network_phase_started.get().copied())
    }

    /// End a session that outlived its cap. Aborting the task skips everything it would have
    /// done to finish, including reporting the outcome (ServerPoll's only delivery path is the
    /// stored terminal payload), so the watchdog finishes the session itself. The token passed is
    /// a fresh one: a cancelled token would report the session as cancelled.
    async fn expire(&self, session_id: Uuid, cap: Duration) {
        let Some(session) = self.slot.take(session_id).await else {
            return;
        };
        let elapsed = session.started_at.elapsed();
        // Cancel first: workers the session spawned (sender threads, drains, SNMP walks) watch the
        // token and outlive an abort of the task that spawned them.
        session.token.cancel();
        session.handle.abort();

        tracing::error!(
            session_id = %session_id,
            running_for_secs = elapsed.as_secs(),
            cap_secs = cap.as_secs(),
            "Discovery session outlived its maximum duration and was stopped"
        );

        let ops = DiscoveryOps::new(&self.discovery_service, session.discovery_type);
        let abort = DiscoveryAbort::Watchdog {
            elapsed_secs: elapsed.as_secs(),
            cap_secs: cap.as_secs(),
        };
        if let Err(e) = ops
            .finish_session(Err(abort.into()), CancellationToken::new())
            .await
        {
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
        });
        Ok(())
    }

    async fn cancel(&self, target: CancelTarget) -> bool {
        let active = self.active.read().await;
        let Some(session) = active.as_ref().filter(|s| s.is_live()) else {
            return false;
        };
        if let CancelTarget::Session(id) = target
            && id != session.session_id
        {
            return false;
        }
        session.token.cancel();
        true
    }

    /// Empty the slot, but only if it still holds `session_id`: by the time a finished task gets
    /// here, the slot may already hold the next session.
    async fn clear(&self, session_id: Uuid) {
        let mut active = self.active.write().await;
        if active.as_ref().is_some_and(|s| s.session_id == session_id) {
            *active = None;
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

/// The scan settings a scanning session runs with. A rescan's narrower settings widen back.
fn scan_settings_of(discovery_type: &DiscoveryType) -> Option<ScanSettings> {
    match discovery_type {
        DiscoveryType::Unified { scan_settings, .. } => Some(scan_settings.clone()),
        DiscoveryType::Rescan { settings, .. } => Some(ScanSettings::from(settings)),
        DiscoveryType::SelfReport { .. }
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
}
