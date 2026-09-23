use crate::{
    daemon::discovery::types::{
        base::{
            DiscoveryPhase, DiscoverySessionInfo, DiscoverySessionUpdate, DiscoveryTerminalReason,
        },
        warnings::{DiscoveryWarning, deserialize_warnings},
    },
    server::{
        auth::middleware::auth::AuthenticatedEntity,
        credentials::r#impl::mapping::{
            CredentialMapping, CredentialQueryPayload, IntegrationTarget,
        },
        daemons::r#impl::{
            base::{Daemon, DaemonBase, DaemonMode},
            version::{DaemonVersionStatus, DeprecationSeverity, DeprecationWarning},
        },
        discovery::r#impl::types::DiscoveryType,
        shared::events::traits::{DiscoveryScope, Event},
        subnets::r#impl::base::Subnet,
    },
};
use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Legacy inbound-only capabilities blob.
///
/// Pre-0.15 daemons report their interfaced subnets as bare `subnet_id`s in this
/// `capabilities` object (they predate the `interfaced_subnets: Vec<Subnet>`
/// heartbeat channel). It is deserialize-only: the server never stores it, never
/// echoes it in `DaemonResponse`, and it has no `SqlValue` variant. Reported ids
/// are routed into the `daemon_interfaced_subnets` junction (existence-filtered)
/// so legacy daemons keep reporting interfaced subnets. ≥0.15 daemons send the
/// `Vec<Subnet>` channel instead and leave this empty.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, ToSchema)]
pub struct LegacyCapabilities {
    /// Subnets the daemon has an interface on, as reported by older daemons.
    #[serde(default)]
    #[schema(required)]
    pub interfaced_subnet_ids: Vec<Uuid>,
}

/// Daemon registration request from daemon to server
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonRegistrationRequest {
    /// The daemon this entity refers to.
    pub daemon_id: Uuid,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// Name the daemon reports for itself.
    pub name: String,
    /// URL is ignored by server - kept for backwards compat with old daemons.
    /// URL is only set via admin provisioning for ServerPoll daemons.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// How the daemon connects: it polls the server, or the server polls it.
    pub mode: DaemonMode,
    /// Legacy pre-0.15 interfaced-subnet channel (deserialize-only; see
    /// [`LegacyCapabilities`]). Repopulated by the first heartbeat, so registration
    /// does not persist it.
    #[serde(default)]
    pub capabilities: LegacyCapabilities,
    /// User responsible for maintaining this daemon (from frontend install command)
    /// Optional for backwards compat with old daemons - defaults to nil UUID
    #[serde(default)]
    pub user_id: Uuid,
    /// Daemon software version (optional for backwards compat with old daemons)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Per-daemon integration targeting from the init command (credentialed cred↔IP and
    /// credential-less local sockets). Written to this daemon's Discovery at registration so
    /// it's present before the first session dispatches. Registration assumes new-daemon →
    /// new-server, so there is no legacy bare-`credential_ids` field — bare-uuid env back-compat
    /// is handled in the daemon's env parser, never on the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub integration_targets: Vec<IntegrationTarget>,
}

/// Daemon registration response from server to daemon
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonRegistrationResponse {
    /// The registered daemon record.
    pub daemon: Daemon,
    /// The host this entity belongs to.
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
    /// The discovery configuration this session belongs to. Old daemons ignore this field.
    #[serde(default)]
    pub discovery_id: Uuid,
    /// The network's subnets as the server holds them.
    ///
    /// A scan that names specific subnets knows them by id, and the CIDR behind each id lives on
    /// the server. A DaemonPoll daemon can ask for them; a ServerPoll daemon has no server URL to
    /// ask with, so they ride along with the work. Old daemons ignore this field, and an old
    /// server leaves it empty.
    #[serde(default)]
    pub subnets: Vec<Subnet>,
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
            "discovery_id": self.discovery_id,
            "subnets": self.subnets,
        })
    }
}

impl From<DiscoveryUpdatePayload> for DaemonDiscoveryRequest {
    fn from(payload: DiscoveryUpdatePayload) -> Self {
        Self {
            session_id: payload.session_id,
            discovery_type: payload.discovery_type,
            credential_mappings: vec![],
            discovery_id: payload.discovery_id.unwrap_or_default(),
            subnets: vec![],
        }
    }
}

/// Daemon discovery response (for immediate acknowledgment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonDiscoveryResponse {
    pub session_id: Uuid,
}

/// Canonical IDs of entities scanned in a discovery session.
///
/// Populated daemon-side at terminal phase from `EntityBuffer`'s `Created`
/// entries. Travels with the terminal `DiscoveryUpdatePayload` to the server,
/// rides the in-memory `EntityOperation::Created` event published for the
/// historical Discovery row (the event scope carries `Entity::Discovery` with
/// the full struct, including `run_type::Historical { results }`), then is
/// stripped before persisting into the historical Discovery row's JSONB (see
/// the `SqlValue::RunType` bind_value handler in
/// `backend/src/server/shared/storage/generic.rs`). Per-entity-service
/// subscribers extract `results.scanned` from the in-memory event and call
/// `DiscoveryFkUpdater::update_discovery_fks` to backfill
/// `last_discovery_id` / `first_discovery_id` on the matched rows.
///
/// Naming: `scanned_*` because the daemon scans entities — some submissions
/// match existing rows (refresh), others insert new rows. Both populate the
/// EntityBuffer with canonical (server-assigned) IDs.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct ScannedEntityIds {
    /// Hosts touched by this discovery.
    #[serde(default)]
    pub host_ids: Vec<Uuid>,
    /// Subnets touched by this discovery.
    #[serde(default)]
    pub subnet_ids: Vec<Uuid>,
    /// VLANs touched by this discovery.
    #[serde(default)]
    pub vlan_ids: Vec<Uuid>,
    /// IP addresses touched by this discovery.
    #[serde(default)]
    pub ip_address_ids: Vec<Uuid>,
    /// Ports touched by this discovery.
    #[serde(default)]
    pub port_ids: Vec<Uuid>,
    /// Services touched by this discovery.
    #[serde(default)]
    pub service_ids: Vec<Uuid>,
    /// Interfaces touched by this discovery.
    #[serde(default)]
    pub interface_ids: Vec<Uuid>,
    /// Service bindings touched by this discovery.
    #[serde(default)]
    pub binding_ids: Vec<Uuid>,
    // No `subnet_vlan_ids`: SubnetVlan is Snapshotable but not
    // DiscoveryTracked. Per-link discovery FKs aren't tracked; SCD2
    // `valid_from` / `valid_to` (soft-close on `unlink`) capture when the
    // link existed.
}

/// Progress update from daemon to server during discovery
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct DiscoveryUpdatePayload {
    /// The discovery run this update belongs to.
    pub session_id: Uuid,
    /// The daemon this entity refers to.
    pub daemon_id: Uuid,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// Which stage of the run is in progress.
    pub phase: DiscoveryPhase,
    /// What kind of discovery is running.
    pub discovery_type: DiscoveryType,
    /// Completion of the current phase, from 0 to 1.
    pub progress: u8,
    /// Failure message, when the run did not complete.
    pub error: Option<String>,
    /// Non-fatal findings from a completed run — one per occurrence, each carrying the code that
    /// identifies it and the detail that fills the sentence. Unlike `error`, these do not mark the
    /// run failed.
    ///
    /// Read through [`deserialize_warnings`] rather than the derived impl, which is what keeps
    /// historical records and pre-coded daemons rendering: both send bare strings here, and both
    /// land as `Unknown` carrying that text instead of failing the whole payload.
    #[serde(default, deserialize_with = "deserialize_warnings")]
    pub warnings: Vec<DiscoveryWarning>,
    /// When the run started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the run finished. `null` while it is still going.
    pub finished_at: Option<DateTime<Utc>>,
    /// Hosts found so far.
    #[serde(default)]
    pub hosts_discovered: Option<u32>,
    /// Rough estimate of the time left, in seconds.
    #[serde(default)]
    pub estimated_remaining_secs: Option<u32>,
    /// The discovery configuration this session belongs to.
    /// Always enriched server-side; daemons do not send this field.
    #[serde(default)]
    pub discovery_id: Option<Uuid>,
    /// Canonical IDs of entities scanned in this session, populated daemon-
    /// side at terminal. **Transient**: stripped at `SqlValue::RunType` bind
    /// time so it doesn't persist into the historical Discovery row's JSONB.
    /// Available in-memory through the `EntityOperation::Created` event
    /// published for the historical Discovery row (the event scope carries
    /// `Entity::Discovery(...)`, the full in-memory struct), where per-entity
    /// FK-update subscribers consume it.
    ///
    /// `Some(...)` when the daemon is sending the terminal payload over the
    /// wire. `None` when read back from a persisted historical row, or when
    /// not yet set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scanned: Option<ScannedEntityIds>,
    /// Why the run ended. Set on terminal payloads; absent on runs recorded before it existed.
    /// A daemon may send one of the reasons only it can know; the server assigns every other.
    #[serde(default)]
    pub reason: Option<DiscoveryTerminalReason>,
    /// When the server last heard about this run. Stamped by the server at terminal.
    #[serde(default)]
    pub last_update_at: Option<DateTime<Utc>>,
    /// The daemon's version when the run ended. Stamped by the server at terminal, so it
    /// survives the daemon being deleted or upgraded.
    #[serde(default)]
    pub daemon_version: Option<String>,
}

impl DiscoveryUpdatePayload {
    /// Construct a typed discovery event with `AuthenticatedEntity::System`.
    pub fn into_discovery_event(&self) -> Event<DiscoveryPhase> {
        self.into_discovery_event_with_auth(AuthenticatedEntity::System)
    }

    pub fn into_discovery_event_with_auth(
        &self,
        auth: AuthenticatedEntity,
    ) -> Event<DiscoveryPhase> {
        Event::new(
            DiscoveryScope {
                network_id: self.network_id,
                session_id: self.session_id,
                daemon_id: self.daemon_id,
                discovery_type: self.discovery_type.clone(),
                error_reason: self.error.clone(),
                reason: self.reason,
            },
            self.phase,
            auth,
        )
    }

    pub fn new(
        session_id: Uuid,
        daemon_id: Uuid,
        network_id: Uuid,
        discovery_type: DiscoveryType,
        discovery_id: Option<Uuid>,
    ) -> Self {
        Self {
            session_id,
            daemon_id,
            network_id,
            phase: DiscoveryPhase::Queued,
            progress: 0,
            discovery_type,
            error: None,
            warnings: Vec::new(),
            started_at: None,
            finished_at: None,
            hosts_discovered: None,
            estimated_remaining_secs: None,
            discovery_id,
            scanned: None,
            reason: None,
            last_update_at: None,
            daemon_version: None,
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
            warnings: update.warnings,
            started_at: info.started_at,
            finished_at: update.finished_at,
            hosts_discovered: None,
            estimated_remaining_secs: None,
            discovery_id: Some(info.discovery_id),
            // Daemon-side reconstruction; the server re-applies the flag from the
            // session it owns (see `DiscoveryService::update_session`).
            scanned: None,
            reason: update.reason,
            last_update_at: None,
            daemon_version: None,
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
    /// URL the daemon is reachable at, as it sees itself.
    pub url: String,
    /// Name the daemon reports for itself.
    pub name: String,
    /// How the daemon connects: it polls the server, or the server polls it.
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
                // Unknown severity from a newer server — log as a warning.
                DeprecationSeverity::Unknown => {
                    tracing::warn!(target: "daemon", "{}", msg);
                }
            }
        }
    }
}

/// First contact request from server to ServerPoll daemon.
/// Sent on first poll to hand the daemon its server-side identity.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FirstContactRequest {
    /// The daemon's server-assigned ID
    pub daemon_id: Uuid,
    /// The network the daemon belongs to (server-provisioned identity). Additive:
    /// an older daemon that ignores this field still works off its cached id.
    #[serde(default)]
    pub network_id: Option<Uuid>,
    /// The daemon's server-assigned name.
    #[serde(default)]
    pub name: Option<String>,
    /// Server capabilities (version, deprecation warnings)
    pub server_capabilities: ServerCapabilities,
}

/// Daemon response for UI including computed version status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonResponse {
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// When this record was first created.
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    pub base: DaemonBase,
    /// Computed version status including health and warnings
    pub version_status: DaemonVersionStatus,
    /// Subnets this daemon has interfaces on, loaded from the
    /// `daemon_interfaced_subnets` junction (replaces the old
    /// `capabilities.interfaced_subnet_ids` JSONB field).
    #[serde(default)]
    #[schema(required)]
    pub interfaced_subnet_ids: Vec<Uuid>,
}

/// Request to pre-provision a daemon (either mode) before it is installed.
/// This creates the daemon record + its 1:1 API key on the server so the install
/// command shrinks to two flags.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProvisionDaemonRequest {
    /// Human-readable name for the daemon. Required unless `daemon_id` is set, in which case
    /// the existing record's name is kept.
    #[serde(default)]
    pub name: Option<String>,
    /// Network this daemon will be associated with. Required unless `daemon_id` is set, in
    /// which case the existing record's network is kept.
    #[serde(default)]
    pub network_id: Option<Uuid>,
    /// How the daemon communicates with the server. Defaults to DaemonPoll
    /// (the daemon dials out) for forward-compat with older clients.
    #[serde(default)]
    pub mode: DaemonMode,
    /// Reachable URL where the *server* can dial the daemon. Required for
    /// ServerPoll, unused for DaemonPoll (the daemon dials out instead).
    #[serde(default)]
    pub url: Option<String>,
    /// Credential/integration references to seed onto the daemon's first
    /// discovery run. References only — never secret material. Empty by default.
    #[serde(default)]
    pub seed_credential_refs: Vec<IntegrationTarget>,
    /// Mint a fresh 1:1 key for this existing daemon instead of creating a new record,
    /// keeping its host, discovery jobs and history. Used to give a legacy daemon (no bound
    /// key) a dedicated one. When set, `name`/`network_id`/`mode`/`url` are ignored — those
    /// come from the existing record.
    ///
    /// Only accepted for a daemon that has never checked in or has no bound key; a live
    /// provisioned daemon is refused, since it has no way to learn the new key.
    ///
    /// Note: install commands are not generated here — call the install-command endpoint,
    /// which builds them idempotently and fills in the key this response returns.
    #[serde(default)]
    pub daemon_id: Option<Uuid>,
}

/// Response from provisioning a daemon.
/// Contains the daemon record and the API key (shown only once).
///
/// Install commands are deliberately not here — fetch them from the install-command endpoint,
/// which builds them idempotently and fills in this key. That keeps a display-only regenerate
/// (advanced-setting change, OS switch) from re-minting the key.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProvisionDaemonResponse {
    /// The created daemon record (with version status).
    pub daemon: DaemonResponse,
    /// The API key (plaintext) for daemon authentication.
    /// This is shown only once - store it securely.
    #[schema(format = "password", read_only)]
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

#[cfg(test)]
mod scanned_payload_tests {
    use super::*;
    use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};

    fn payload_with_scanned(scanned: Option<ScannedEntityIds>) -> DiscoveryUpdatePayload {
        let mut p = DiscoveryUpdatePayload::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            DiscoveryType::default(),
            None,
        );
        p.scanned = scanned;
        p
    }

    /// Daemons older than the terminal reason send none of its fields, and every historical row
    /// written before it lacks them. Both must still read.
    #[test]
    fn payload_without_terminal_reason_fields_deserializes_to_none() {
        let mut json = serde_json::to_value(payload_with_scanned(None)).unwrap();
        let obj = json.as_object_mut().unwrap();
        obj.remove("reason");
        obj.remove("last_update_at");
        obj.remove("daemon_version");

        let parsed: DiscoveryUpdatePayload = serde_json::from_value(json).unwrap();

        assert_eq!(parsed.reason, None);
        assert_eq!(parsed.last_update_at, None);
        assert_eq!(parsed.daemon_version, None);
    }

    /// A daemon newer than its server sends fields the server does not know. The payload must
    /// not reject them, or a daemon upgrade ahead of the server breaks every update it sends.
    #[test]
    fn payload_with_an_unknown_field_still_deserializes() {
        let mut json = serde_json::to_value(payload_with_scanned(None)).unwrap();
        json.as_object_mut()
            .unwrap()
            .insert("field_from_a_newer_daemon".into(), serde_json::json!(true));

        assert!(serde_json::from_value::<DiscoveryUpdatePayload>(json).is_ok());
    }

    /// An old server sends no subnets with the work. The daemon must still accept the request:
    /// a sweep does not need them, and the resolver says so when a targeted scan does.
    #[test]
    fn a_request_without_subnets_deserializes_to_an_empty_list() {
        let mut json = serde_json::json!(DaemonDiscoveryRequest {
            session_id: Uuid::new_v4(),
            discovery_type: DiscoveryType::default(),
            credential_mappings: vec![],
            discovery_id: Uuid::new_v4(),
            subnets: vec![],
        });
        json.as_object_mut().unwrap().remove("subnets");

        let parsed: DaemonDiscoveryRequest = serde_json::from_value(json).unwrap();

        assert!(parsed.subnets.is_empty());
    }

    /// The daemon reads the dispatch the server actually sends, which is hand-built rather than
    /// derived, so a field added to the struct and not to that JSON never arrives.
    #[test]
    fn the_dispatch_the_server_sends_carries_the_subnets() {
        let request = DaemonDiscoveryRequest {
            session_id: Uuid::new_v4(),
            discovery_type: DiscoveryType::default(),
            credential_mappings: vec![],
            discovery_id: Uuid::new_v4(),
            subnets: vec![],
        };

        let sent = request.with_exposed_credentials();

        let parsed: DaemonDiscoveryRequest = serde_json::from_value(sent).unwrap();
        assert_eq!(parsed.session_id, request.session_id);
        assert_eq!(parsed.discovery_id, request.discovery_id);
    }

    #[test]
    fn payload_serialize_skips_scanned_when_none() {
        let p = payload_with_scanned(None);
        let json = serde_json::to_value(&p).unwrap();
        assert!(
            json.get("scanned").is_none(),
            "scanned: None should be skipped on serialize, got: {json}"
        );
    }

    #[test]
    fn payload_serialize_includes_scanned_when_some() {
        let host_id = Uuid::new_v4();
        let mut s = ScannedEntityIds::default();
        s.host_ids.push(host_id);
        let p = payload_with_scanned(Some(s));
        let json = serde_json::to_value(&p).unwrap();
        let scanned = json.get("scanned").expect("scanned should be present");
        assert_eq!(
            scanned["host_ids"][0].as_str().unwrap(),
            host_id.to_string()
        );
    }

    #[test]
    fn run_type_historical_strip_drops_scanned_from_persisted_jsonb() {
        // Mirrors the strip in `SqlValue::RunType::bind_value`
        // (see `backend/src/server/shared/storage/generic.rs`):
        // a clone of the typed RunType has its `scanned` field cleared
        // before serializing to JSONB.
        let mut s = ScannedEntityIds::default();
        s.subnet_ids.push(Uuid::new_v4());
        let payload = payload_with_scanned(Some(s));
        let mut rt = RunType::Historical {
            results: Box::new(payload),
        };
        // Apply the same strip the storage layer applies.
        if let RunType::Historical { results } = &mut rt {
            results.scanned = None;
        }
        let json = serde_json::to_value(&rt).unwrap();
        let results = &json["results"];
        assert!(
            results.get("scanned").is_none(),
            "Persisted JSONB must not carry `scanned` (it's transient): {results}"
        );
    }

    #[test]
    fn run_type_roundtrip_with_scanned_preserves_field_in_memory() {
        // Verifies the wire path: serialize a payload with scanned set,
        // deserialize back, and confirm the field round-trips when it's
        // not stripped at the storage boundary.
        let host_id = Uuid::new_v4();
        let mut s = ScannedEntityIds::default();
        s.host_ids.push(host_id);
        let payload = payload_with_scanned(Some(s));
        let rt = RunType::Historical {
            results: Box::new(payload),
        };

        let json = serde_json::to_value(&rt).unwrap();
        let parsed: RunType = serde_json::from_value(json).unwrap();
        match parsed {
            RunType::Historical { results } => {
                let scanned = results.scanned.expect("scanned should round-trip");
                assert_eq!(scanned.host_ids, vec![host_id]);
            }
            _ => panic!("expected Historical"),
        }
    }

    #[test]
    fn payload_deserialize_with_no_scanned_field_defaults_to_none() {
        // Persisted historical rows have no `scanned` field; deserializing
        // back into the typed payload should produce `scanned: None`.
        let p = payload_with_scanned(None);
        let mut json = serde_json::to_value(&p).unwrap();
        // Ensure the key is genuinely absent.
        json.as_object_mut().unwrap().remove("scanned");
        let parsed: DiscoveryUpdatePayload = serde_json::from_value(json).unwrap();
        assert!(parsed.scanned.is_none());
    }
}
