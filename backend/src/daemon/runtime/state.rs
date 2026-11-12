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
    },
    server::{
        daemons::r#impl::{
            api::{DiscoveryUpdatePayload, LegacyCapabilities},
            base::{DaemonMode, DaemonOs},
        },
        hosts::r#impl::{api::DiscoveryHostRequest, api::HostResponse},
        subnets::r#impl::base::Subnet,
        vlans::handlers::{VlanDiscoveryRequest, VlanDiscoveryResponse},
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
    /// The subnets the daemon's host is attached to, as its last session classified them. The
    /// server resolves these via SubnetService::create (create-or-match by CIDR) and replaces
    /// the daemon's interfaced subnets with them. Absent: leave them as they are. A daemon sends
    /// them once after each session; older daemons send them on every report.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interfaced_subnets: Option<Vec<Subnet>>,
    /// A digest of the daemon's interfaces. With `interfaced_subnets`, the interfaces those were
    /// classified from; without, the interfaces as they are now. The server starts an interface
    /// refresh when it differs from the last classified one. Absent from older daemons.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interfaces_fingerprint: Option<String>,
    /// Whether the daemon can accept a new discovery session.
    /// Both DaemonPoll and ServerPoll use this to avoid dispatching work to a busy daemon.
    #[serde(default = "default_true")]
    pub ready_for_work: bool,
    /// The OS this daemon binary runs on. Absent from daemons up to 0.17.19.
    #[serde(
        default,
        deserialize_with = "crate::server::daemons::r#impl::base::lenient_daemon_os",
        skip_serializing_if = "Option::is_none"
    )]
    pub os: Option<DaemonOs>,
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
    /// VLAN submissions waiting for the server to upsert them and answer with their ids. Absent
    /// from daemons that predate ServerPoll VLAN recording.
    #[serde(default)]
    pub vlans: Vec<PendingVlanBatch>,
}

/// One `upsert_vlans` call, buffered for a ServerPoll server to collect. `pending_id` pairs the
/// server's answer with the call waiting on it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingVlanBatch {
    pub pending_id: Uuid,
    pub request: VlanDiscoveryRequest,
}

impl BufferedEntities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.hosts.is_empty() && self.subnets.is_empty() && self.vlans.is_empty()
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
    /// VLAN batches: (pending_id, the ids the server gave each VLAN number). Ignored by daemons
    /// that predate ServerPoll VLAN recording.
    #[serde(default)]
    pub vlans: Vec<(Uuid, VlanDiscoveryResponse)>,
    /// Set when a host was skipped due to billing host limit (limit value, org_id)
    #[serde(skip)]
    pub billing_limit_hit: Option<(u64, Uuid)>,
    /// First per-host processing error (non-billing), captured so the synchronous
    /// single-host `/hosts/discovery` handler can answer with the real cause and its
    /// status instead of a generic "no host returned". Not sent over the wire.
    #[serde(skip)]
    pub first_host_error: Option<crate::server::shared::types::api::ApiError>,
}

/// Daemon state for handlers.
/// Delegates to ConfigStore for metadata, DaemonDiscoveryService for progress,
/// and EntityBuffer for buffered entities.
pub struct DaemonState {
    config: Arc<ConfigStore>,
    discovery_service: Arc<DaemonDiscoveryService>,
    entity_buffer: Arc<EntityBuffer>,
    discovery_manager: Arc<DaemonDiscoverySessionManager>,
}

impl DaemonState {
    pub fn new(
        config: Arc<ConfigStore>,
        discovery_service: Arc<DaemonDiscoveryService>,
        entity_buffer: Arc<EntityBuffer>,
        discovery_manager: Arc<DaemonDiscoverySessionManager>,
    ) -> Self {
        Self {
            config,
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
    pub async fn get_status(&self) -> DaemonStatus {
        let name = self.config.get_name().await.unwrap_or_default();
        let mode = self.config.get_mode().await.unwrap_or_default();
        let version = Version::parse(env!("CARGO_PKG_VERSION")).ok();
        let ready_for_work = !self.discovery_manager.is_discovery_running().await;
        let (interfaced_subnets, interfaces_fingerprint) =
            self.discovery_service.status_interfaces().await;

        DaemonStatus {
            // Don't send URL - server manages this via provisioning for ServerPoll,
            // and doesn't need it for DaemonPoll
            url: None,
            name,
            mode,
            version,
            capabilities: LegacyCapabilities::default(),
            interfaced_subnets,
            interfaces_fingerprint,
            ready_for_work,
            os: Some(DaemonOs::current()),
        }
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
                site_id: s.info.site_id,
                phase: crate::daemon::discovery::types::base::DiscoveryPhase::Scanning,
                discovery_type: s.info.discovery_type.clone(),
                progress,
                error: None,
                warnings: Vec::new(),
                credential_results: Vec::new(),
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
                last_activity_at: Some(s.last_activity_at()),
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
