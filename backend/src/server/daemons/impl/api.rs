use std::fmt::Display;

use crate::{
    daemon::discovery::types::base::{
        DiscoveryPhase, DiscoverySessionInfo, DiscoverySessionUpdate,
    },
    server::{
        credentials::r#impl::mapping::{CredentialMapping, CredentialQueryPayload},
        daemons::r#impl::{
            base::{Daemon, DaemonBase, DaemonMode},
            version::{DaemonVersionStatus, DeprecationSeverity, DeprecationWarning},
        },
        discovery::r#impl::types::DiscoveryType,
    },
};
use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Daemon capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash, ToSchema)]
pub struct DaemonCapabilities {
    #[serde(default)]
    pub has_docker_socket: bool,
    #[serde(default)]
    #[schema(required)]
    pub interfaced_subnet_ids: Vec<Uuid>,
}

impl Display for DaemonCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DaemonCapabilities {{ has_docker_socket: {}, interfaced_subnet_ids: {:?} }}",
            self.has_docker_socket, self.interfaced_subnet_ids
        )
    }
}

/// Daemon registration request from daemon to server
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonRegistrationRequest {
    pub daemon_id: Uuid,
    pub network_id: Uuid,
    pub name: String,
    /// URL is ignored by server - kept for backwards compat with old daemons.
    /// URL is only set via admin provisioning for ServerPoll daemons.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub mode: DaemonMode,
    pub capabilities: DaemonCapabilities,
    /// User responsible for maintaining this daemon (from frontend install command)
    /// Optional for backwards compat with old daemons - defaults to nil UUID
    #[serde(default)]
    pub user_id: Uuid,
    /// Daemon software version (optional for backwards compat with old daemons)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Credential IDs to assign to daemon's host during registration.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub credential_ids: Vec<Uuid>,
}

/// Daemon registration response from server to daemon
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonRegistrationResponse {
    pub daemon: Daemon,
    pub host_id: Uuid,
    /// Server capabilities (returned if daemon sends version info)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_capabilities: Option<ServerCapabilities>,
}

/// Daemon discovery request from server to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonDiscoveryRequest {
    pub session_id: Uuid,
    pub discovery_type: DiscoveryType,
    /// Generic credential mappings for unified discovery. Old daemons ignore this field.
    #[serde(default)]
    pub credential_mappings: Vec<CredentialMapping<CredentialQueryPayload>>,
}

impl DaemonDiscoveryRequest {
    /// Serialize with SNMP credentials exposed as plaintext for daemon transmission.
    pub fn with_exposed_snmp(&self) -> serde_json::Value {
        serde_json::json!({
            "session_id": self.session_id,
            "discovery_type": self.discovery_type.with_exposed_snmp(),
        })
    }

    /// Serialize with all credentials exposed for daemon transmission.
    /// Used for unified discovery — credentials are in credential_mappings.
    pub fn with_exposed_credentials(&self) -> serde_json::Value {
        serde_json::json!({
            "session_id": self.session_id,
            "discovery_type": self.discovery_type,
            "credential_mappings": self.credential_mappings,
        })
    }
}

impl From<DiscoveryUpdatePayload> for DaemonDiscoveryRequest {
    fn from(payload: DiscoveryUpdatePayload) -> Self {
        Self {
            session_id: payload.session_id,
            discovery_type: payload.discovery_type,
            credential_mappings: vec![],
        }
    }
}

/// Daemon discovery response (for immediate acknowledgment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonDiscoveryResponse {
    pub session_id: Uuid,
}

/// Progress update from daemon to server during discovery
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct DiscoveryUpdatePayload {
    pub session_id: Uuid,
    pub daemon_id: Uuid,
    pub network_id: Uuid,
    pub phase: DiscoveryPhase,
    pub discovery_type: DiscoveryType,
    pub progress: u8,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub hosts_discovered: Option<u32>,
    #[serde(default)]
    pub estimated_remaining_secs: Option<u32>,
}

impl DiscoveryUpdatePayload {
    pub fn new(
        session_id: Uuid,
        daemon_id: Uuid,
        network_id: Uuid,
        discovery_type: DiscoveryType,
    ) -> Self {
        Self {
            session_id,
            daemon_id,
            network_id,
            phase: DiscoveryPhase::Queued,
            progress: 0,
            discovery_type,
            error: None,
            started_at: None,
            finished_at: None,
            hosts_discovered: None,
            estimated_remaining_secs: None,
        }
    }

    pub fn from_state_and_update(
        discovery_type: DiscoveryType,
        info: DiscoverySessionInfo,
        update: DiscoverySessionUpdate,
    ) -> Self {
        Self {
            session_id: info.session_id,
            discovery_type,
            network_id: info.network_id,
            daemon_id: info.daemon_id,
            phase: update.phase,
            progress: update.progress,
            error: update.error,
            started_at: info.started_at,
            finished_at: update.finished_at,
            hosts_discovered: None,
            estimated_remaining_secs: None,
        }
    }

    /// Serialize with SNMP credentials exposed as plaintext for daemon transmission.
    /// Patches the `discovery_type` field to use plaintext community strings while
    /// preserving all other fields the daemon expects.
    pub fn with_exposed_snmp(&self) -> serde_json::Value {
        let mut value = serde_json::to_value(self).unwrap_or_default();
        if let serde_json::Value::Object(ref mut map) = value {
            map.insert(
                "discovery_type".to_string(),
                self.discovery_type.with_exposed_snmp(),
            );
        }
        value
    }
}

/// Legacy heartbeat payload for backwards compatibility with pre-v0.14.0 daemons.
/// Old daemons call POST /api/daemons/{id}/heartbeat with this payload.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonHeartbeatPayload {
    pub url: String,
    pub name: String,
    pub mode: DaemonMode,
}

/// Sent by daemon on startup to report version
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonStartupRequest {
    /// Daemon software version (semver format)
    #[schema(value_type = String)]
    pub daemon_version: Version,
}

/// Server capabilities returned on startup/registration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerCapabilities {
    /// Server software version
    #[schema(value_type = String)]
    pub server_version: Version,
    /// Minimum daemon version supported by this server
    #[schema(value_type = String)]
    pub minimum_daemon_version: Version,
    /// Deprecation warnings for the daemon
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deprecation_warnings: Vec<DeprecationWarning>,
}

impl ServerCapabilities {
    /// Log deprecation warnings from the server.
    /// Logs each warning at the appropriate severity level.
    pub fn log_warnings(&self) {
        for warning in &self.deprecation_warnings {
            let msg = format!(
                "{}{}",
                warning.message,
                warning
                    .sunset_date
                    .as_ref()
                    .map(|d| format!(" (sunset: {})", d))
                    .unwrap_or_default()
            );
            match warning.severity {
                DeprecationSeverity::Critical => {
                    tracing::error!(target: "daemon", "{}", msg);
                }
                DeprecationSeverity::Warning => {
                    tracing::warn!(target: "daemon", "{}", msg);
                }
                DeprecationSeverity::Info => {
                    tracing::info!(target: "daemon", "{}", msg);
                }
            }
        }
    }
}

/// First contact request from server to ServerPoll daemon.
/// Sent on first poll to assign the daemon its server-side ID.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FirstContactRequest {
    /// The daemon's server-assigned ID
    pub daemon_id: Uuid,
    /// Server capabilities (version, deprecation warnings)
    pub server_capabilities: ServerCapabilities,
}

/// Daemon response for UI including computed version status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonResponse {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    pub base: DaemonBase,
    /// Computed version status including health and warnings
    pub version_status: DaemonVersionStatus,
}

/// Request to pre-provision a ServerPoll mode daemon.
/// This creates the daemon record on the server before the daemon is installed.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProvisionDaemonRequest {
    /// Human-readable name for the daemon.
    pub name: String,
    /// Network this daemon will be associated with.
    pub network_id: Uuid,
    /// URL where the server can reach the daemon (required for ServerPoll mode).
    pub url: String,
}

/// Response from provisioning a daemon.
/// Contains the daemon record and the API key (shown only once).
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProvisionDaemonResponse {
    /// The created daemon record (with version status).
    pub daemon: DaemonResponse,
    /// The API key (plaintext) for daemon authentication.
    /// This is shown only once - store it securely.
    pub daemon_api_key: String,
}

/// Request to test reachability of a daemon URL.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TestReachabilityRequest {
    /// Full URL of the daemon (e.g. "https://daemon.example.com:60073")
    pub url: String,
    /// If true, also perform an HTTP GET to {url}/health after the TCP check
    #[serde(default)]
    pub check_health: bool,
}

/// Response from a reachability test.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TestReachabilityResponse {
    /// Whether the TCP connection succeeded
    pub reachable: bool,
    /// Error message if not reachable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Health check result (only present when check_health was true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<bool>,
}
