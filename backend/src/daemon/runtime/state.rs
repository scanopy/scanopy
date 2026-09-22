use std::sync::Arc;

use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uuid::Uuid;

use crate::{
    daemon::{
        discovery::{
            buffer::EntityBuffer, manager::DaemonDiscoverySessionManager,
            service::base::DaemonDiscoveryService,
        },
        shared::config::ConfigStore,
        utils::base::{DaemonUtils, PlatformDaemonUtils},
    },
    server::{
        daemons::r#impl::{
            api::{DiscoveryUpdatePayload, LegacyCapabilities},
            base::DaemonMode,
        },
        hosts::r#impl::{api::DiscoveryHostRequest, api::HostResponse},
        subnets::r#impl::base::Subnet,
    },
};

/// Lightweight daemon status for polling responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonStatus {
    /// URL is not used by server - kept for backwards compat.
    /// Server never updates daemon URL from status (URL is set during provisioning).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Name the daemon reports for itself.
    pub name: String,
    /// How the daemon connects: it polls the server, or the server polls it.
    pub mode: DaemonMode,
    /// Daemon software version (semver format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub version: Option<Version>,
    /// Backwards compat: pre-v0.15.0 daemons send capabilities instead of interfaced_subnets.
    #[serde(default)]
    pub capabilities: LegacyCapabilities,
    /// Subnets detected from daemon's network ip_addresses. Server resolves these
    /// via SubnetService::create (create-or-match by CIDR) to get real IDs.
    /// v0.15.0+ daemons populate this; pre-v0.15.0 daemons leave it empty.
    #[serde(default)]
    pub interfaced_subnets: Vec<Subnet>,
    /// Whether the daemon can accept a new discovery session.
    /// Both DaemonPoll and ServerPoll use this to avoid dispatching work to a busy daemon.
    #[serde(default = "default_true")]
    pub ready_for_work: bool,
}

fn default_true() -> bool {
    true
}

/// Buffered entities discovered during a discovery session.
/// Used to batch entity creation when server polls daemon (ServerPoll mode).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct BufferedEntities {
    /// Hosts with their ip_addresses, ports, and services
    pub hosts: Vec<DiscoveryHostRequest>,
    /// Discovered subnets
    pub subnets: Vec<Subnet>,
}

impl BufferedEntities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.hosts.is_empty() && self.subnets.is_empty()
    }
}

/// Response type for GET /api/discovery endpoint.
/// Returns current progress and any buffered entities since last poll.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiscoveryPollResponse {
    /// Current discovery session progress (if any active session)
    pub progress: Option<DiscoveryUpdatePayload>,
    /// Entities discovered since last poll
    pub entities: BufferedEntities,
}

/// Payload sent by server to daemon with created entity confirmations.
/// Maps pending (daemon-generated) IDs to actual server entities (after deduplication).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatedEntitiesPayload {
    /// Subnets: (pending_id, actual_subnet) pairs
    pub subnets: Vec<(Uuid, Subnet)>,
    /// Hosts: (pending_id, actual_host_response) pairs - includes children (ip_addresses, ports, services)
    pub hosts: Vec<(Uuid, HostResponse)>,
    /// Set when a host was skipped due to billing host limit (limit value, org_id)
    #[serde(skip)]
    pub billing_limit_hit: Option<(u64, Uuid)>,
    /// First per-host processing error (non-billing), captured so the synchronous
    /// single-host `/hosts/discovery` handler can surface the real cause instead
    /// of a generic "no host returned". Not sent over the wire.
    #[serde(skip)]
    pub first_host_error: Option<String>,
}

/// Daemon state for handlers.
/// Delegates to ConfigStore for metadata, DaemonDiscoveryService for progress,
/// and EntityBuffer for buffered entities.
pub struct DaemonState {
    config: Arc<ConfigStore>,
    utils: PlatformDaemonUtils,
    discovery_service: Arc<DaemonDiscoveryService>,
    entity_buffer: Arc<EntityBuffer>,
    discovery_manager: Arc<DaemonDiscoverySessionManager>,
}

impl DaemonState {
    pub fn new(
        config: Arc<ConfigStore>,
        utils: PlatformDaemonUtils,
        discovery_service: Arc<DaemonDiscoveryService>,
        entity_buffer: Arc<EntityBuffer>,
        discovery_manager: Arc<DaemonDiscoverySessionManager>,
    ) -> Self {
        Self {
            config,
            utils,
            discovery_service,
            entity_buffer,
            discovery_manager,
        }
    }

    /// Get the entity buffer for pushing discovered entities.
    pub fn entity_buffer(&self) -> &Arc<EntityBuffer> {
        &self.entity_buffer
    }
}

impl DaemonState {
    /// Get lightweight daemon status (name, mode, version, capabilities).
    /// Note: URL is intentionally not included - server manages URL via provisioning.
    /// Detects ip_addresses freshly on every call.
    pub async fn get_status(&self) -> DaemonStatus {
        let name = self.config.get_name().await.unwrap_or_default();
        let mode = self.config.get_mode().await.unwrap_or_default();
        let version = Version::parse(env!("CARGO_PKG_VERSION")).ok();
        let ready_for_work = !self.discovery_manager.is_discovery_running().await;

        // Detect ip_addresses fresh — cheap NIC enumeration
        let interfaced_subnets = self.detect_interfaced_subnets().await.unwrap_or_default();

        DaemonStatus {
            // Don't send URL - server manages this via provisioning for ServerPoll,
            // and doesn't need it for DaemonPoll
            url: None,
            name,
            mode,
            version,
            capabilities: LegacyCapabilities::default(),
            interfaced_subnets,
            ready_for_work,
        }
    }

    /// Detect subnets from daemon's network ip_addresses.
    async fn detect_interfaced_subnets(&self) -> anyhow::Result<Vec<Subnet>> {
        let network_id = match self.config.get_network_id().await? {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };
        let interface_filter = self.config.get_interfaces().await?;

        let (_, subnets, _) = self
            .utils
            .get_own_interfaces(network_id, &interface_filter)
            .await?;

        Ok(subnets)
    }

    /// Get current discovery session progress, if any.
    ///
    /// Returns progress in this priority:
    /// 1. If there's an active session, return current progress (Scanning phase)
    /// 2. If session ended, return terminal payload (Complete/Failed/Cancelled phase)
    /// 3. If neither, return None
    ///
    /// The terminal payload is critical for ServerPoll mode: the server polls periodically
    /// and needs to receive the terminal state to update session_last_updated and avoid
    /// marking the session as stalled. The terminal payload persists until a new session starts.
    pub async fn get_progress(&self) -> Option<DiscoveryUpdatePayload> {
        // First check for active session
        let session = self.discovery_service.current_session.read().await;

        if let Some(s) = session.as_ref() {
            let progress = s.last_progress.load(std::sync::atomic::Ordering::Relaxed);

            tracing::trace!(
                session_id = %s.info.session_id,
                progress = progress,
                "get_progress: returning active session progress"
            );

            return Some(DiscoveryUpdatePayload {
                session_id: s.info.session_id,
                daemon_id: s.info.daemon_id,
                network_id: s.info.network_id,
                phase: crate::daemon::discovery::types::base::DiscoveryPhase::Scanning,
                discovery_type: s.info.discovery_type.clone(),
                progress,
                error: None,
                warnings: Vec::new(),
                started_at: s.info.started_at,
                finished_at: None,
                hosts_discovered: {
                    let v = s
                        .hosts_discovered
                        .load(std::sync::atomic::Ordering::Relaxed);
                    if v > 0 { Some(v) } else { None }
                },
                estimated_remaining_secs: {
                    let v = s
                        .estimated_remaining_secs
                        .load(std::sync::atomic::Ordering::Relaxed);
                    if v != u32::MAX { Some(v) } else { None }
                },
                discovery_id: Some(s.info.discovery_id),
                scanned: None,
                reason: None,
                last_update_at: None,
                daemon_version: None,
            });
        }
        drop(session);

        // No active session - check for terminal payload from finished session
        // This allows the server to poll and receive the terminal state
        let terminal = self.discovery_service.terminal_payload.read().await;
        if let Some(ref tp) = *terminal {
            tracing::debug!(
                session_id = %tp.session_id,
                phase = %tp.phase,
                progress = tp.progress,
                "get_progress: returning terminal payload"
            );
        } else {
            tracing::trace!("get_progress: no active session and no terminal payload");
        }
        terminal.clone()
    }

    /// Clear the terminal payload after the server has acknowledged it.
    /// This prevents the daemon from resending the same terminal state on every poll.
    pub async fn clear_terminal_payload(&self) {
        let mut terminal = self.discovery_service.terminal_payload.write().await;
        *terminal = None;
    }

    /// Get pending buffered entities for sending to server.
    /// Returns pending hosts/subnets without clearing them from the buffer.
    ///
    /// In ServerPoll mode, the lifecycle is:
    /// 1. Server polls → get_pending_entities() returns pending entities
    /// 2. Server processes entities → sends confirmation back
    /// 3. Daemon receives confirmation → buffer.mark_*_created() updates state
    /// 4. Session ends → buffer.clear_all() removes all entities
    pub async fn get_pending_entities(&self) -> BufferedEntities {
        self.entity_buffer.get_pending().await
    }
}
