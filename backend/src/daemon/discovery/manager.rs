use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::daemon::discovery::service::base::{
    DaemonDiscoveryService, DiscoversNetworkedEntities, DiscoveryRunner, RunsDiscovery,
};
use crate::daemon::discovery::service::self_report::SelfReportDiscovery;
use crate::daemon::discovery::service::unified::UnifiedDiscovery;
use crate::daemon::runtime::service::LOG_TARGET;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;
use crate::server::daemons::r#impl::api::DaemonDiscoveryRequest;
use crate::server::discovery::r#impl::types::DiscoveryType;

pub struct DaemonDiscoverySessionManager {
    current_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    cancellation_token: Arc<RwLock<CancellationToken>>,
    discovery_service: Arc<DaemonDiscoveryService>,
}

impl DaemonDiscoverySessionManager {
    pub fn new(discovery_service: Arc<DaemonDiscoveryService>) -> Self {
        Self {
            current_task: Arc::new(RwLock::new(None)),
            cancellation_token: Arc::new(RwLock::new(CancellationToken::new())),
            discovery_service,
        }
    }

    /// Try to initiate a discovery session. Returns false if already busy.
    pub async fn try_initiate_session(self: &Arc<Self>, request: DaemonDiscoveryRequest) -> bool {
        if self.is_discovery_running().await {
            tracing::warn!(
                session_id = %request.session_id,
                discovery_type = %request.discovery_type,
                "Rejecting discovery request - another session is already running"
            );
            return false;
        }

        self.initiate_session(request).await;
        true
    }

    pub async fn initiate_session(self: &Arc<Self>, request: DaemonDiscoveryRequest) {
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

        if let DiscoveryType::Unified {
            scan_settings,
            scan_local_docker_socket,
            ..
        } = &request.discovery_type
        {
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

            // Docker socket
            tracing::info!(
                "  {:<20}{}",
                "Docker socket:",
                if *scan_local_docker_socket {
                    "enabled"
                } else {
                    "disabled"
                }
            );

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

        let cancel_token = self.start_new_session().await;

        let handle = match &request.discovery_type {
            // Legacy types: log warning and complete immediately
            DiscoveryType::SelfReport { .. }
            | DiscoveryType::Docker { .. }
            | DiscoveryType::Network { .. } => {
                let legacy_type = request.discovery_type.to_string();
                tracing::warn!(
                    "Received legacy discovery type '{}', completing session immediately. \
                     This daemon only supports unified discovery.",
                    legacy_type
                );

                // Report completion via a lightweight SelfReport runner
                // (start_discovery + finish_discovery handles session lifecycle)
                let host_id = match &request.discovery_type {
                    DiscoveryType::SelfReport { host_id } => *host_id,
                    DiscoveryType::Docker { host_id, .. } => *host_id,
                    _ => uuid::Uuid::nil(),
                };
                self.clone().spawn_legacy_stub(
                    DiscoveryRunner::new(
                        self.discovery_service.clone(),
                        self.clone(),
                        SelfReportDiscovery::new(host_id),
                    ),
                    request.clone(),
                    cancel_token,
                )
            }
            DiscoveryType::Unified {
                host_id,
                subnet_ids,
                scan_local_docker_socket,
                host_naming_fallback,
                scan_settings,
            } => {
                let runner = DiscoveryRunner::new(
                    self.discovery_service.clone(),
                    self.clone(),
                    UnifiedDiscovery {
                        host_id: *host_id,
                        subnet_ids: subnet_ids.clone(),
                        scan_local_docker_socket: *scan_local_docker_socket,
                        host_naming_fallback: *host_naming_fallback,
                        scan_settings: scan_settings.clone(),
                        credential_mappings: request.credential_mappings.clone(),
                    },
                );
                self.clone()
                    .spawn_discovery(runner, request.clone(), cancel_token)
            }
        };

        self.set_current_task(handle).await;
    }

    /// Spawn a lightweight stub for legacy discovery types that just reports completion
    fn spawn_legacy_stub<T>(
        self: Arc<Self>,
        discovery: DiscoveryRunner<T>,
        request: DaemonDiscoveryRequest,
        cancel_token: CancellationToken,
    ) -> tokio::task::JoinHandle<()>
    where
        DiscoveryRunner<T>: RunsDiscovery
            + crate::daemon::discovery::service::base::DiscoversNetworkedEntities
            + 'static,
        T: 'static + Send + Sync,
    {
        tokio::spawn(async move {
            // Initialize session and immediately complete it
            if let Err(e) = discovery.start_discovery(request).await {
                tracing::error!("Failed to start legacy stub session: {}", e);
            } else if let Err(e) = discovery
                .finish_discovery(Ok(()), cancel_token.clone())
                .await
            {
                tracing::error!("Failed to finish legacy stub session: {}", e);
            }
            if !cancel_token.is_cancelled() {
                self.clear_completed_task().await;
            }
        })
    }

    fn spawn_discovery<T>(
        self: Arc<Self>,
        discovery: DiscoveryRunner<T>,
        request: DaemonDiscoveryRequest,
        cancel_token: CancellationToken,
    ) -> tokio::task::JoinHandle<()>
    where
        DiscoveryRunner<T>: RunsDiscovery + 'static,
        T: 'static + Send + Sync,
    {
        tokio::spawn(async move {
            match discovery.discover(request, cancel_token.clone()).await {
                Ok(()) => {
                    tracing::info!("Discovery completed successfully");
                }
                Err(e) => {
                    tracing::error!("Discovery failed: {}", e);
                }
            }
            // Only clear if NOT cancelled - the cancel handler will clear it
            if !cancel_token.is_cancelled() {
                self.clear_completed_task().await;
            }
        })
    }

    /// Check if discovery is currently running
    pub async fn is_discovery_running(&self) -> bool {
        tracing::debug!(target: LOG_TARGET, "Checking discovery running on manager instance: {:p}", self);
        let task_guard = self.current_task.read().await;
        let has_task = task_guard.is_some();
        let is_finished = if let Some(handle) = task_guard.as_ref() {
            handle.is_finished()
        } else {
            true
        };
        tracing::debug!(target: LOG_TARGET, "Has task: {}, Is finished: {}", has_task, is_finished);

        if let Some(handle) = task_guard.as_ref() {
            !handle.is_finished()
        } else {
            false
        }
    }

    /// Set the current discovery task for cancellation
    pub async fn start_new_session(&self) -> CancellationToken {
        *self.cancellation_token.write().await = CancellationToken::new();
        *self.current_task.write().await = None;

        self.cancellation_token.read().await.clone()
    }

    pub async fn set_current_task(&self, handle: JoinHandle<()>) {
        *self.current_task.write().await = Some(handle);
    }

    /// Cancel current discovery task
    pub async fn cancel_current_session(&self) -> bool {
        if !self.is_discovery_running().await {
            return false;
        }

        tracing::info!("Cancelling discovery session...");

        // Signal cooperative cancellation
        self.cancellation_token.write().await.cancel();

        // Don't wait - just return success
        // The spawned task will handle cleanup
        true
    }

    pub async fn token(&self) -> CancellationToken {
        self.cancellation_token.read().await.clone()
    }

    /// Clear completed task
    pub async fn clear_completed_task(&self) {
        let mut task_guard = self.current_task.write().await;
        if let Some(handle) = task_guard.as_ref()
            && handle.is_finished()
        {
            *self.cancellation_token.write().await = CancellationToken::new();
            *task_guard = None;
        }
    }
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
