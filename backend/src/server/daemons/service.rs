//! Daemon service for managing daemon lifecycle, processing, and polling.
//!
//! This service consolidates:
//! - CRUD operations for daemons
//! - Processing logic for daemon data (formerly in processor.rs)
//! - Polling loop for ServerPoll mode (formerly in poller.rs)
//! - HTTP client for daemon communication

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Error, Result};
use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use chrono::Utc;
use futures::future::join_all;
use secrecy::ExposeSecret;
use semver::Version;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::daemon::runtime::state::{
    BufferedEntities, CreatedEntitiesPayload, DaemonStatus, DiscoveryPollResponse,
};
use crate::daemon::runtime::types::InitializeDaemonRequest;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::types::base::BillingPlan;
use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::credentials::service::CredentialService;
use crate::server::daemon_api_keys::service::DaemonApiKeyService;
use crate::server::daemons::r#impl::api::{
    DaemonCapabilities, DaemonDiscoveryRequest, DaemonRegistrationRequest,
    DaemonRegistrationResponse, DiscoveryUpdatePayload, FirstContactRequest, ServerCapabilities,
};
use crate::server::daemons::r#impl::base::{Daemon, DaemonBase};
use crate::server::daemons::r#impl::version::{DaemonVersionPolicy, supports_unified_discovery};
use crate::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback, RunType};
use crate::server::discovery::service::DiscoveryService;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::service::{HostLimitContext, HostService};
use crate::server::networks::r#impl::Network;
use crate::server::networks::service::NetworkService;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::types::{
    BillingEvent, BillingOperation, EntityEvent, EntityOperation, OnboardingEvent,
    OnboardingOperation,
};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::traits::{Entity, Storable, Storage};
use crate::server::shared::types::api::{ApiError, ApiResponse};
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::service::SubnetService;
use crate::server::tags::entity_tags::EntityTagService;
use crate::server::users::service::UserService;

/// Weekly Sunday midnight cron schedule for default discovery jobs
const WEEKLY_SUNDAY_MIDNIGHT_CRON: &str = "0 0 0 * * 0";

/// Default polling interval in seconds
const DEFAULT_POLL_INTERVAL_SECS: u64 = 30;

/// Number of consecutive failures before marking daemon as unreachable
const UNREACHABLE_THRESHOLD: usize = 5;

/// Maximum number of concurrent daemon polls
const MAX_CONCURRENT_POLLS: usize = 10;

pub struct DaemonService {
    // Storage and core dependencies
    daemon_storage: Arc<GenericPostgresStorage<Daemon>>,
    client: reqwest::Client,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,

    // Direct dependencies (passed to constructor)
    discovery_service: Arc<DiscoveryService>,
    credential_service: Arc<CredentialService>,
    subnet_service: Arc<SubnetService>,
    network_service: Arc<NetworkService>,
    organization_service: Arc<OrganizationService>,
    user_service: Arc<UserService>,
    daemon_api_key_service: Arc<DaemonApiKeyService>,

    // Lazy dependency (set after construction to break circular dependency)
    // HostService uses DaemonService, and DaemonService uses HostService
    host_service: std::sync::OnceLock<Arc<HostService>>,

    // Polling state
    poll_semaphore: Arc<Semaphore>,
}

impl EventBusService<Daemon> for DaemonService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Daemon) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &Daemon) -> Option<Uuid> {
        None
    }

    fn suppress_logs(&self, current: Option<&Daemon>, updated: Option<&Daemon>) -> bool {
        match (current, updated) {
            (Some(current), Some(updated)) => updated.suppress_logs(current),
            _ => false,
        }
    }
}

#[async_trait]
impl CrudService<Daemon> for DaemonService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Daemon>> {
        &self.daemon_storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn delete(
        &self,
        id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), anyhow::Error> {
        // Clean up in-memory discovery session state (queued/pending/terminal)
        // to prevent stale references after deletion.
        self.discovery_service.clear_sessions_for_daemon(id).await;

        // Delegate to the default CrudService delete implementation
        let entity = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("{} with id {} not found", Daemon::table_name(), id))?;

        self.storage().delete(id).await?;

        let trigger_stale = entity.triggers_staleness(None);
        let suppress_logs = self.suppress_logs(Some(&entity), None);

        if let Some(entity_tag_service) = self.entity_tag_service() {
            entity_tag_service
                .remove_all_for_entity(entity.id(), <Daemon as Entity>::entity_type())
                .await?;
        }

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: *id,
                network_id: self.get_network_id(&entity),
                organization_id: self.get_organization_id(&entity),
                entity_type: entity.into(),
                operation: EntityOperation::Deleted,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale,
                    "suppress_logs": suppress_logs
                }),
                authentication,
            })
            .await?;

        Ok(())
    }
}

impl DaemonService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        daemon_storage: Arc<GenericPostgresStorage<Daemon>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        discovery_service: Arc<DiscoveryService>,
        credential_service: Arc<CredentialService>,
        subnet_service: Arc<SubnetService>,
        network_service: Arc<NetworkService>,
        organization_service: Arc<OrganizationService>,
        user_service: Arc<UserService>,
        daemon_api_key_service: Arc<DaemonApiKeyService>,
    ) -> Self {
        Self {
            daemon_storage,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            event_bus,
            entity_tag_service,
            discovery_service,
            credential_service,
            subnet_service,
            network_service,
            organization_service,
            user_service,
            daemon_api_key_service,
            host_service: std::sync::OnceLock::new(),
            poll_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_POLLS)),
        }
    }

    /// Check if an unverified org has reached its daemon limit (1 daemon).
    /// Allows first daemon so users can experience core product value.
    pub async fn check_unverified_daemon_limit(&self, org_id: Uuid) -> Result<(), ApiError> {
        let owners = self
            .user_service
            .get_organization_owners(&org_id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        let any_verified = owners.iter().any(|u| u.base.email_verified);
        if any_verified {
            return Ok(());
        }

        // Get networks for this org, then count daemons on those networks
        let networks = self
            .network_service
            .get_all(StorableFilter::<Network>::new_from_org_id(&org_id))
            .await
            .unwrap_or_default();

        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();
        if network_ids.is_empty() {
            return Ok(());
        }

        let all_daemons = self
            .get_all(StorableFilter::<Daemon>::new_from_network_ids(&network_ids))
            .await
            .unwrap_or_default();

        if !all_daemons.is_empty() {
            return Err(ApiError::coded(
                axum::http::StatusCode::FORBIDDEN,
                crate::server::shared::types::error_codes::ErrorCode::AuthEmailVerificationRequired,
            ));
        }

        Ok(())
    }

    /// Logs a warning if a daemon URL uses HTTP (credentials sent in plaintext).
    /// Does not block — users may have legitimate reasons (VPN, private network).
    fn warn_if_insecure_daemon_url(url: &str) {
        if let Ok(parsed) = url::Url::parse(url)
            && parsed.scheme() != "https"
            && let Some(host) = parsed.host_str()
            && host != "localhost"
            && host != "127.0.0.1"
            && host != "::1"
        {
            tracing::warn!(
                daemon_url = url,
                "Daemon URL uses HTTP — credentials will be sent unencrypted. \
                 Ensure the connection is secured through other means (e.g., VPN, private network)"
            );
        }
    }

    // ========================================================================
    // Dependency injection (for breaking circular dependency with HostService)
    // ========================================================================

    /// Set the host service dependency after construction.
    /// This breaks the circular dependency: HostService needs DaemonService,
    /// and DaemonService needs HostService.
    pub fn set_host_service(&self, service: Arc<HostService>) -> Result<(), Arc<HostService>> {
        self.host_service.set(service)
    }

    // ========================================================================
    // Daemon HTTP helpers with built-in retry
    // ========================================================================

    /// Send GET request to daemon with auth and retry.
    /// Uses exponential backoff: 5 retries, 5-30s delays.
    async fn get_from_daemon<T: serde::de::DeserializeOwned>(
        &self,
        daemon: &Daemon,
        api_key: &str,
        path: &str,
    ) -> Result<T> {
        let url = format!("{}{}", daemon.base.url, path);
        let daemon_id = daemon.id;

        (|| async {
            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?;

            if !response.status().is_success() {
                anyhow::bail!("GET {} failed: HTTP {}", path, response.status());
            }

            let api_response: ApiResponse<T> = response.json().await?;

            if !api_response.success {
                anyhow::bail!(
                    "GET {} failed: {}",
                    path,
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                );
            }

            api_response
                .data
                .ok_or_else(|| anyhow::anyhow!("GET {} response missing data", path))
        })
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(Duration::from_secs(5))
                .with_max_delay(Duration::from_secs(30))
                .with_max_times(UNREACHABLE_THRESHOLD),
        )
        .notify(|e, dur| {
            tracing::warn!(
                daemon_id = %daemon_id,
                path = %path,
                "Request failed, retrying in {:?}: {}",
                dur,
                e
            );
        })
        .await
    }

    /// Send POST request to daemon with optional auth and retry.
    /// Uses exponential backoff: 5 retries, 5-30s delays.
    /// Returns `Option<T>` - `Some(data)` if response contains data, `None` otherwise.
    /// For endpoints that don't return data, use `::<serde_json::Value>` and ignore result.
    ///
    /// If `api_key` is `None`, the request is sent without an Authorization header.
    /// This is used for legacy daemons (< v0.14.0) that don't require authentication.
    async fn post_to_daemon<T: serde::de::DeserializeOwned>(
        &self,
        daemon: &Daemon,
        api_key: Option<&str>,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<Option<T>> {
        let url = format!("{}{}", daemon.base.url, path);
        let daemon_id = daemon.id;
        let body_json = serde_json::to_value(body)?;
        let api_key_owned = api_key.map(|s| s.to_owned());

        (|| async {
            let mut request = self.client.post(&url).json(&body_json);

            // Only add auth header if API key provided (v0.14.0+ daemons)
            if let Some(ref key) = api_key_owned {
                request = request.header("Authorization", format!("Bearer {}", key));
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                anyhow::bail!("POST {} failed: HTTP {}", path, response.status());
            }

            let api_response: ApiResponse<T> = response.json().await?;

            if !api_response.success {
                anyhow::bail!(
                    "POST {} failed: {}",
                    path,
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                );
            }

            Ok(api_response.data)
        })
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(Duration::from_secs(5))
                .with_max_delay(Duration::from_secs(30))
                .with_max_times(UNREACHABLE_THRESHOLD),
        )
        .notify(|e, dur| {
            tracing::warn!(
                daemon_id = %daemon_id,
                path = %path,
                "Request failed, retrying in {:?}: {}",
                dur,
                e
            );
        })
        .await
    }

    // ========================================================================
    // Daemon HTTP methods (using helpers)
    // ========================================================================

    /// Poll daemon status via GET /api/status
    async fn poll_status(&self, daemon: &Daemon, api_key: &str) -> Result<DaemonStatus> {
        self.get_from_daemon(daemon, api_key, "/api/status").await
    }

    /// Poll daemon discovery via GET /api/poll
    async fn poll_discovery(
        &self,
        daemon: &Daemon,
        api_key: &str,
    ) -> Result<DiscoveryPollResponse> {
        self.get_from_daemon(daemon, api_key, "/api/poll").await
    }

    /// Send created entities back to daemon via POST /api/discovery/entities-created
    async fn send_created_entities(
        &self,
        daemon: &Daemon,
        api_key: &str,
        created_entities: CreatedEntitiesPayload,
    ) -> Result<()> {
        // Skip if there's nothing to send
        if created_entities.hosts.is_empty() && created_entities.subnets.is_empty() {
            return Ok(());
        }

        let _: Option<serde_json::Value> = self
            .post_to_daemon(
                daemon,
                Some(api_key),
                "/api/discovery/entities-created",
                &created_entities,
            )
            .await?;

        tracing::debug!(
            daemon_id = %daemon.id,
            hosts_count = created_entities.hosts.len(),
            subnets_count = created_entities.subnets.len(),
            "Sent created entities to ServerPoll daemon"
        );

        Ok(())
    }

    /// Send discovery request to daemon (HTTP only, no event publishing).
    ///
    /// If `api_key` is `None`, the request is sent without authentication.
    /// This is used for legacy daemons (< v0.14.0) that don't require auth.
    pub async fn send_discovery_request_to_daemon(
        &self,
        daemon: &Daemon,
        api_key: Option<&str>,
        request: DaemonDiscoveryRequest,
    ) -> Result<(), Error> {
        Self::warn_if_insecure_daemon_url(&daemon.base.url);

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %request.session_id,
            "Sending discovery request to daemon"
        );

        // Unified: serialize with credential_mappings via with_exposed_credentials().
        // Legacy: serialize with SNMP inline via with_exposed_snmp().
        let payload = if matches!(request.discovery_type, DiscoveryType::Unified { .. }) {
            request.with_exposed_credentials()
        } else {
            request.with_exposed_snmp()
        };
        let _: Option<serde_json::Value> = self
            .post_to_daemon(daemon, api_key, "/api/discovery/initiate", &payload)
            .await?;

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %request.session_id,
            "Discovery request sent successfully"
        );

        Ok(())
    }

    /// Send discovery cancellation to daemon (HTTP only, no event publishing).
    ///
    /// If `api_key` is `None`, the request is sent without authentication.
    /// This is used for legacy daemons (< v0.14.0) that don't require auth.
    pub async fn send_discovery_cancellation_to_daemon(
        &self,
        daemon: &Daemon,
        api_key: Option<&str>,
        session_id: Uuid,
    ) -> Result<(), Error> {
        let _: Option<serde_json::Value> = self
            .post_to_daemon(daemon, api_key, "/api/discovery/cancel", &session_id)
            .await?;

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %session_id,
            "Discovery cancellation sent successfully"
        );

        Ok(())
    }

    /// Send first contact request to ServerPoll daemon.
    /// This assigns the daemon its server-side ID and returns the daemon's status.
    async fn send_first_contact(&self, daemon: &Daemon, api_key: &str) -> Result<DaemonStatus> {
        let policy = DaemonVersionPolicy::default();
        let server_capabilities = ServerCapabilities {
            server_version: policy.latest.clone(),
            minimum_daemon_version: policy.minimum_supported.clone(),
            deprecation_warnings: vec![],
        };

        let request = FirstContactRequest {
            daemon_id: daemon.id,
            server_capabilities,
        };

        self.post_to_daemon(daemon, Some(api_key), "/api/first-contact", &request)
            .await?
            .ok_or_else(|| anyhow::anyhow!("First contact response missing daemon status"))
    }

    /// Initialize a local daemon (for integrated daemon setup)
    pub async fn initialize_local_daemon(
        &self,
        daemon_url: String,
        network_id: Uuid,
        api_key: String,
    ) -> Result<(), Error> {
        match self
            .client
            .post(format!("{}/api/initialize", daemon_url))
            .json(&InitializeDaemonRequest {
                network_id,
                api_key,
            })
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    tracing::info!("Successfully initialized daemon");
                } else {
                    let body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Could not read body".to_string());
                    tracing::warn!(status = %status, body = %body, "Daemon returned error");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to reach daemon");
            }
        }

        Ok(())
    }

    // ========================================================================
    // Processing methods
    // ========================================================================

    /// Process a heartbeat from a daemon.
    /// Also resolves interfaced subnets and updates capabilities.
    pub async fn process_status(
        &self,
        daemon_id: Uuid,
        status: DaemonStatus,
        auth: AuthenticatedEntity,
    ) -> Result<(), ApiError> {
        let mut daemon = self
            .get_by_id(&daemon_id)
            .await?
            .ok_or_else(|| ApiError::entity_not_found::<Daemon>(daemon_id))?;

        daemon.base.last_seen = Some(Utc::now());
        // NOTE: We intentionally do NOT update URL from status.
        // URL is only set:
        // - ServerPoll: Admin provides URL during provisioning
        // - DaemonPoll: URL not needed (server never connects to daemon)
        // This prevents daemons from overwriting admin-configured URLs.
        daemon.base.name = status.name;
        daemon.base.mode = status.mode;

        // Update version if provided (for ServerPoll mode status responses)
        if let Some(version) = status.version {
            daemon.base.version = Some(version);
        }

        // Resolve interfaced subnets based on daemon version
        if supports_unified_discovery(daemon.base.version.as_ref()) {
            // v0.15.0+ daemon: resolve full Subnet objects (empty = no interfaces)
            let mut resolved_ids = Vec::new();
            for subnet in status.interfaced_subnets {
                match self.subnet_service.create(subnet, auth.clone()).await {
                    Ok(resolved) => resolved_ids.push(resolved.id),
                    Err(e) => {
                        tracing::warn!(error = ?e, "Failed to resolve interfaced subnet");
                    }
                }
            }
            daemon.base.capabilities.interfaced_subnet_ids = resolved_ids;
            daemon.base.capabilities.has_docker_socket = status.has_docker_socket;
        } else if !status.capabilities.interfaced_subnet_ids.is_empty() {
            // Pre-v0.15.0: use capabilities as-is
            daemon.base.capabilities = status.capabilities;
        }

        self.update(&mut daemon, auth).await?;
        Ok(())
    }

    /// Process a daemon startup announcement
    pub async fn process_startup(
        &self,
        daemon_id: Uuid,
        version: Version,
        auth: AuthenticatedEntity,
    ) -> Result<ServerCapabilities, ApiError> {
        // Reject daemons with version older than the server
        let server_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        if version < server_version {
            return Err(ApiError::daemon_version_too_old(
                &version.to_string(),
                &server_version.to_string(),
            ));
        }

        let mut daemon = self
            .get_by_id(&daemon_id)
            .await?
            .ok_or_else(|| ApiError::entity_not_found::<Daemon>(daemon_id))?;

        let was_pre_unified = !supports_unified_discovery(daemon.base.version.as_ref());

        daemon.base.version = Some(version.clone());
        daemon.base.last_seen = Some(Utc::now());

        self.update(&mut daemon, auth).await?;

        tracing::info!(daemon_id = %daemon_id, version = %version, "Daemon startup");

        // Migrate legacy discoveries if daemon just upgraded to unified-capable version
        if was_pre_unified
            && supports_unified_discovery(Some(&version))
            && let Err(e) = self
                .migrate_discoveries_to_unified(
                    daemon_id,
                    daemon.base.host_id,
                    daemon.base.network_id,
                )
                .await
        {
            tracing::warn!(
                daemon_id = %daemon_id,
                error = ?e,
                "Failed to migrate legacy discoveries to unified"
            );
        }

        let policy = DaemonVersionPolicy::default();
        let status = policy.evaluate(Some(&version));

        Ok(ServerCapabilities {
            server_version: policy.latest.clone(),
            minimum_daemon_version: policy.minimum_supported.clone(),
            deprecation_warnings: status.warnings,
        })
    }

    /// Process a daemon registration request
    pub async fn process_registration(
        &self,
        request: DaemonRegistrationRequest,
        auth: AuthenticatedEntity,
    ) -> Result<DaemonRegistrationResponse, ApiError> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| ApiError::internal_error("HostService not initialized"))?;

        // Check if this is a demo organization - block daemon registration
        let network = self
            .network_service
            .get_by_id(&request.network_id)
            .await?
            .ok_or_else(|| ApiError::entity_not_found::<Network>(request.network_id))?;

        let org = self
            .organization_service
            .get_by_id(&network.base.organization_id)
            .await?
            .ok_or_else(|| ApiError::not_found("Organization not found".to_string()))?;

        if matches!(org.base.plan, Some(BillingPlan::Demo(_))) {
            return Err(ApiError::demo_mode_blocked());
        }

        tracing::info!("{:?}", request);

        // Parse version early for use in server_capabilities
        let daemon_version = request
            .version
            .as_ref()
            .and_then(|v| semver::Version::parse(v).ok());

        // Reject daemons with version older than the server
        let server_version = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        if daemon_version.as_ref().is_none_or(|v| v < &server_version) {
            let dv = daemon_version
                .as_ref()
                .map_or("unknown".to_string(), |v| v.to_string());
            return Err(ApiError::daemon_version_too_old(
                &dv,
                &server_version.to_string(),
            ));
        }

        // Compute server_capabilities if version was provided
        let policy = DaemonVersionPolicy::default();
        let server_capabilities = daemon_version.as_ref().map(|v| {
            let status = policy.evaluate(Some(v));
            ServerCapabilities {
                server_version: policy.latest.clone(),
                minimum_daemon_version: policy.minimum_supported.clone(),
                deprecation_warnings: status.warnings,
            }
        });

        // Check if daemon already exists (re-registration scenario)
        if let Some(mut existing_daemon) = self.get_by_id(&request.daemon_id).await? {
            tracing::info!(
                daemon_id = %request.daemon_id,
                host_id = %existing_daemon.base.host_id,
                "Daemon already registered, updating registration"
            );

            // Update daemon with current info
            // NOTE: We do NOT update URL from registration request.
            // URL is only set via admin provisioning for ServerPoll daemons.
            existing_daemon.base.capabilities = request.capabilities;
            existing_daemon.base.last_seen = Some(Utc::now());
            existing_daemon.base.mode = request.mode;
            existing_daemon.base.name = request.name;
            let was_pre_unified =
                !supports_unified_discovery(existing_daemon.base.version.as_ref());
            if let Some(v) = daemon_version.clone() {
                existing_daemon.base.version = Some(v);
            }

            let updated_daemon = self.update(&mut existing_daemon, auth).await?;

            // Migrate legacy discoveries if daemon just upgraded to unified-capable version
            if was_pre_unified
                && supports_unified_discovery(existing_daemon.base.version.as_ref())
                && let Err(e) = self
                    .migrate_discoveries_to_unified(
                        existing_daemon.id,
                        existing_daemon.base.host_id,
                        existing_daemon.base.network_id,
                    )
                    .await
            {
                tracing::warn!(
                    daemon_id = %existing_daemon.id,
                    error = ?e,
                    "Failed to migrate legacy discoveries to unified"
                );
            }

            return Ok(DaemonRegistrationResponse {
                daemon: updated_daemon,
                host_id: existing_daemon.base.host_id,
                server_capabilities,
            });
        }

        // Check daemon limit for unverified orgs (allows 1st daemon)
        self.check_unverified_daemon_limit(org.id).await?;

        // New registration - create host and daemon
        let dummy_host = Host::new(HostBase {
            network_id: request.network_id,
            name: request.name.clone(),
            hostname: None,
            description: None,
            source: EntitySource::Discovery { metadata: vec![] },
            virtualization: None,
            hidden: false,
            tags: Vec::new(),
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            credential_assignments: vec![],
        });

        let host_response = host_service
            .discover_host(
                dummy_host,
                vec![],
                vec![],
                vec![],
                vec![],
                auth.clone(),
                None,
            )
            .await?;

        // Assign only localhost-targeted credentials to daemon host during registration.
        // Remote credentials will be auto-assigned to their target hosts during discovery.
        if !request.credential_ids.is_empty() {
            let mut assignments = Vec::new();
            for id in &request.credential_ids {
                let is_local = match self.credential_service.get_by_id(id).await {
                    Ok(Some(cred)) => match &cred.base.target_ips {
                        Some(ips) => ips.iter().any(|ip| ip.is_loopback()),
                        None => false,
                    },
                    Ok(None) => {
                        tracing::warn!(credential_id = %id, "Credential not found during registration");
                        false
                    }
                    Err(e) => {
                        tracing::warn!(credential_id = %id, error = ?e, "Failed to look up credential during registration");
                        false
                    }
                };
                if is_local {
                    assignments.push(CredentialAssignment {
                        credential_id: *id,
                        interface_ids: None,
                    });
                } else {
                    tracing::debug!(
                        credential_id = %id,
                        "Skipping credential with remote target_ips for daemon host assignment"
                    );
                }
            }
            if !assignments.is_empty()
                && let Err(e) = self
                    .credential_service
                    .set_host_credentials(&host_response.id, &assignments)
                    .await
            {
                tracing::warn!(
                    host_id = %host_response.id,
                    error = ?e,
                    "Failed to assign credentials to host during registration"
                );
            }
        }

        // If user_id is nil (old daemon), fall back to org owner
        let user_id = if request.user_id.is_nil() {
            self.user_service
                .get_organization_owners(&org.id)
                .await?
                .first()
                .map(|u| u.id)
                .unwrap_or(request.user_id)
        } else {
            request.user_id
        };

        let mut daemon = Daemon::new(DaemonBase {
            host_id: host_response.id,
            network_id: request.network_id,
            // DaemonPoll mode: URL not needed (server never connects to daemon)
            // ServerPoll mode: URL is set during provisioning, not during registration
            url: String::new(),
            capabilities: request.capabilities.clone(),
            last_seen: Some(Utc::now()),
            mode: request.mode,
            name: request.name,
            tags: Vec::new(),
            version: daemon_version,
            user_id,
            api_key_id: None,
            is_unreachable: false,
            standby: false,
        });

        daemon.id = request.daemon_id;

        let registered_daemon = self.create(daemon, auth.clone()).await?;

        // Send telemetry event if this is the organization's first daemon
        self.emit_first_daemon_telemetry(registered_daemon.id, registered_daemon.base.network_id)
            .await?;

        // Create default discovery jobs
        let is_free_plan = matches!(org.base.plan, Some(BillingPlan::Free(_)));
        self.create_default_discovery_jobs(
            request.daemon_id,
            request.network_id,
            host_response.id,
            request.capabilities.has_docker_socket,
            is_free_plan,
        )
        .await?;

        Ok(DaemonRegistrationResponse {
            daemon: registered_daemon,
            host_id: host_response.id,
            server_capabilities,
        })
    }

    /// Process a capabilities update from a daemon
    pub async fn process_capabilities(
        &self,
        daemon_id: Uuid,
        capabilities: DaemonCapabilities,
        auth: AuthenticatedEntity,
    ) -> Result<(), ApiError> {
        tracing::debug!(
            daemon_id = %daemon_id,
            capabilities = %capabilities,
            "Updating daemon capabilities",
        );

        let mut daemon = self
            .get_by_id(&daemon_id)
            .await?
            .ok_or_else(|| ApiError::entity_not_found::<Daemon>(daemon_id))?;

        daemon.base.capabilities = capabilities;

        self.update(&mut daemon, auth).await?;
        Ok(())
    }

    /// Process a discovery progress update
    pub async fn process_discovery_progress(
        &self,
        update: DiscoveryUpdatePayload,
    ) -> Result<(), ApiError> {
        self.discovery_service.update_session(update).await?;
        Ok(())
    }

    /// Process discovered entities from a daemon.
    ///
    /// This function processes entities with best-effort semantics: if one entity fails,
    /// we continue processing the rest and return confirmations for successfully processed
    /// entities. This is critical for ServerPoll mode where the daemon is waiting for
    /// confirmations - failing the entire batch due to one bad entity would cause the
    /// daemon to timeout and stall.
    pub async fn process_discovery_entities(
        &self,
        entities: BufferedEntities,
        auth: AuthenticatedEntity,
    ) -> Result<CreatedEntitiesPayload, ApiError> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| ApiError::internal_error("HostService not initialized"))?;

        // Compute host limit context from the first host's network → org → plan
        let limit_ctx = if let Some(first_host) = entities.hosts.first() {
            let network_id = first_host.host.base.network_id;
            if let Ok(Some(network)) = self.network_service.get_by_id(&network_id).await
                && let Ok(Some(org)) = self
                    .organization_service
                    .get_by_id(&network.base.organization_id)
                    .await
                && let Some(plan) = &org.base.plan
                && let Some(limit) = plan.host_limit()
            {
                let org_networks = self
                    .network_service
                    .get_all(StorableFilter::<Network>::new_from_org_id(&org.id))
                    .await
                    .unwrap_or_default();
                let org_network_ids: Vec<Uuid> = org_networks.iter().map(|n| n.id).collect();
                Some(HostLimitContext {
                    limit,
                    org_id: org.id,
                    org_network_ids,
                })
            } else {
                None
            }
        } else {
            None
        };

        let mut created_hosts = Vec::new();
        let mut created_subnets = Vec::new();
        let mut host_failures = 0;
        let mut subnet_failures = 0;
        let mut limit_event_emitted = false;
        let mut billing_limit_reached: Option<(u64, Uuid)> = None;

        // Process each discovered host - continue on failure to avoid blocking entire batch
        for host_request in entities.hosts {
            let pending_id = host_request.host.id;
            let host_name = host_request.host.base.name.clone();
            match host_service
                .discover_host(
                    host_request.host,
                    host_request.interfaces,
                    host_request.ports,
                    host_request.services,
                    host_request.if_entries,
                    auth.clone(),
                    limit_ctx.as_ref(),
                )
                .await
            {
                Ok(host_response) => {
                    // Credential assignments are persisted inside discover_host()
                    // after remapping daemon interface UUIDs to server-assigned UUIDs.

                    // Scope loopback credentials to the loopback interface
                    if let Err(e) = self
                        .scope_loopback_credentials(&host_response.id, host_service)
                        .await
                    {
                        tracing::warn!(
                            host_id = %host_response.id,
                            error = ?e,
                            "Failed to scope loopback credentials"
                        );
                    }

                    created_hosts.push((pending_id, host_response));
                }
                Err(e) => {
                    host_failures += 1;

                    // Emit billing event once when host limit is hit
                    if !limit_event_emitted
                        && e.to_string().contains("Host limit reached")
                        && let Some(ctx) = &limit_ctx
                    {
                        limit_event_emitted = true;
                        billing_limit_reached = Some((ctx.limit, ctx.org_id));
                        let _ = self
                            .event_bus
                            .publish_billing(BillingEvent::new(
                                Uuid::new_v4(),
                                ctx.org_id,
                                BillingOperation::FeatureLimitHit,
                                Utc::now(),
                                auth.clone(),
                                serde_json::json!({
                                    "limit_type": "hosts",
                                    "current_count": ctx.limit,
                                    "limit": ctx.limit,
                                    "source": "discovery",
                                }),
                            ))
                            .await;
                    }

                    tracing::warn!(
                        pending_id = %pending_id,
                        host_name = %host_name,
                        error = %e,
                        "Failed to process discovered host - skipping (daemon will retry or timeout)"
                    );
                }
            }
        }

        // Emit FirstHostDiscovered telemetry if this is the org's first discovered host
        if !created_hosts.is_empty()
            && let Some((_, first_host)) = created_hosts.first()
            && let Ok(Some(network)) = self.network_service.get_by_id(&first_host.network_id).await
            && let Ok(Some(org)) = self
                .organization_service
                .get_by_id(&network.base.organization_id)
                .await
            && org.not_onboarded(&OnboardingOperation::FirstHostDiscovered)
        {
            let _ = self
                .event_bus
                .publish_onboarding(OnboardingEvent::new(
                    Uuid::new_v4(),
                    org.id,
                    OnboardingOperation::FirstHostDiscovered,
                    Utc::now(),
                    AuthenticatedEntity::System,
                    serde_json::json!({}),
                ))
                .await;
        }

        // Process discovered subnets - continue on failure to avoid blocking entire batch
        for subnet in entities.subnets {
            let pending_id = subnet.id;
            let cidr = subnet.base.cidr;
            match self.subnet_service.create(subnet, auth.clone()).await {
                Ok(actual_subnet) => {
                    created_subnets.push((pending_id, actual_subnet));
                }
                Err(e) => {
                    subnet_failures += 1;
                    tracing::warn!(
                        pending_id = %pending_id,
                        cidr = %cidr,
                        error = %e,
                        "Failed to process discovered subnet - skipping (daemon will retry or timeout)"
                    );
                }
            }
        }

        if host_failures > 0 || subnet_failures > 0 {
            tracing::info!(
                hosts_created = created_hosts.len(),
                hosts_failed = host_failures,
                subnets_created = created_subnets.len(),
                subnets_failed = subnet_failures,
                "Entity processing completed with some failures"
            );
        }

        Ok(CreatedEntitiesPayload {
            subnets: created_subnets,
            hosts: created_hosts,
            billing_limit_hit: billing_limit_reached,
        })
    }

    /// Get pending discovery work for a daemon.
    /// When work is returned, the session is immediately transitioned to Starting phase
    /// to prevent it from being dispatched again on subsequent poll cycles.
    /// Returns None if there's already an active session running on the daemon.
    pub async fn get_pending_work(&self, daemon_id: Uuid) -> Option<DiscoveryUpdatePayload> {
        // Don't dispatch new work if there's already an active session
        if self
            .discovery_service
            .has_active_session_for_daemon(&daemon_id)
            .await
        {
            return None;
        }

        let sessions = self
            .discovery_service
            .get_sessions_for_daemon(&daemon_id)
            .await;

        if let Some(work) = sessions.first().cloned() {
            // Transition to Starting so this won't be returned again
            self.discovery_service
                .transition_session_to_starting(work.session_id)
                .await;
            Some(work)
        } else {
            None
        }
    }

    /// Get pending cancellation request for a daemon
    pub async fn get_pending_cancellation(&self, daemon_id: Uuid) -> Option<Uuid> {
        let (has_cancellation, session_id) = self
            .discovery_service
            .pull_cancellation_for_daemon(&daemon_id)
            .await;
        if has_cancellation {
            Some(session_id)
        } else {
            None
        }
    }

    /// Create default discovery jobs for a newly contacted daemon
    pub async fn create_default_discovery_jobs(
        &self,
        daemon_id: Uuid,
        network_id: Uuid,
        host_id: Uuid,
        has_docker_socket: bool,
        is_free_plan: bool,
    ) -> Result<(), ApiError> {
        tracing::info!(
            daemon_id = %daemon_id,
            network_id = %network_id,
            host_id = %host_id,
            has_docker_socket,
            is_free_plan,
            "Creating default discovery jobs for daemon"
        );

        // Free plans use AdHoc (run once immediately), paid plans use Scheduled
        let default_run_type = if is_free_plan {
            RunType::AdHoc { last_run: None }
        } else {
            RunType::Scheduled {
                cron_schedule: WEEKLY_SUNDAY_MIDNIGHT_CRON.to_string(),
                last_run: None,
                enabled: true,
                timezone: None,
            }
        };

        // Create a single Unified discovery combining self-report, network, and docker
        let unified_discovery_type = DiscoveryType::Unified {
            host_id,
            subnet_ids: None,
            scan_local_docker_socket: has_docker_socket,
            host_naming_fallback: HostNamingFallback::BestService,
            scan_settings: ScanSettings::default(),
        };

        let unified_discovery = self
            .discovery_service
            .create_discovery(
                Discovery::new(DiscoveryBase {
                    run_type: default_run_type,
                    discovery_type: unified_discovery_type.clone(),
                    name: "Discovery".to_string(),
                    daemon_id,
                    network_id,
                    tags: Vec::new(),
                }),
                AuthenticatedEntity::System,
            )
            .await?;

        self.discovery_service
            .start_session(unified_discovery, AuthenticatedEntity::System)
            .await?;

        Ok(())
    }

    /// Migrate legacy discoveries (SelfReport/Network/Docker) to a single Unified discovery.
    /// Archives old discoveries as Historical and creates a new Unified discovery inheriting
    /// settings from the primary Network discovery (if any).
    pub async fn migrate_discoveries_to_unified(
        &self,
        daemon_id: Uuid,
        host_id: Uuid,
        network_id: Uuid,
    ) -> Result<(), ApiError> {
        let filter =
            StorableFilter::new_from_uuid_column("daemon_id", &daemon_id).exclude_historical();
        let discoveries = self.discovery_service.get_all(filter).await?;

        // Skip if any are already Unified
        if discoveries
            .iter()
            .any(|d| matches!(d.base.discovery_type, DiscoveryType::Unified { .. }))
        {
            return Ok(());
        }

        // Skip if there are no discoveries to migrate
        if discoveries.is_empty() {
            return Ok(());
        }

        // Select primary: prefer Network discovery that is enabled + scheduled + most recently updated
        let primary = discoveries
            .iter()
            .filter(|d| {
                matches!(d.base.discovery_type, DiscoveryType::Network { .. })
                    && matches!(d.base.run_type, RunType::Scheduled { enabled: true, .. })
            })
            .max_by_key(|d| d.updated_at)
            .or_else(|| discoveries.iter().max_by_key(|d| d.updated_at));

        let primary = match primary {
            Some(p) => p,
            None => return Ok(()),
        };

        // Extract settings from primary
        let (subnet_ids, host_naming_fallback) = match &primary.base.discovery_type {
            DiscoveryType::Network {
                subnet_ids,
                host_naming_fallback,
                ..
            } => (subnet_ids.clone(), *host_naming_fallback),
            _ => (None, HostNamingFallback::BestService),
        };

        let has_docker = discoveries
            .iter()
            .any(|d| matches!(d.base.discovery_type, DiscoveryType::Docker { .. }));

        // Create unified discovery inheriting run_type and tags from primary
        let unified_type = DiscoveryType::Unified {
            host_id,
            subnet_ids,
            scan_local_docker_socket: has_docker,
            host_naming_fallback,
            scan_settings: ScanSettings::default(),
        };

        self.discovery_service
            .create_discovery(
                Discovery::new(DiscoveryBase {
                    run_type: primary.base.run_type.clone(),
                    discovery_type: unified_type.clone(),
                    name: "Discovery".to_string(),
                    daemon_id,
                    network_id,
                    tags: primary.base.tags.clone(),
                }),
                AuthenticatedEntity::System,
            )
            .await?;

        // Delete old discoveries
        let count = discoveries.len();
        for old in &discoveries {
            if let Err(e) = self
                .discovery_service
                .delete(&old.id, AuthenticatedEntity::System)
                .await
            {
                tracing::warn!(
                    discovery_id = %old.id,
                    error = ?e,
                    "Failed to delete legacy discovery during migration"
                );
            }
        }

        tracing::info!(
            daemon_id = %daemon_id,
            count = count,
            "Migrated {} legacy discoveries to unified for daemon {}",
            count,
            daemon_id
        );

        Ok(())
    }

    /// Emit FirstDaemonRegistered telemetry event if this is the org's first daemon
    pub async fn emit_first_daemon_telemetry(
        &self,
        daemon_id: Uuid,
        network_id: Uuid,
    ) -> Result<(), ApiError> {
        let network = self
            .network_service
            .get_by_id(&network_id)
            .await?
            .ok_or_else(|| ApiError::entity_not_found::<Network>(network_id))?;

        let org = self
            .organization_service
            .get_by_id(&network.base.organization_id)
            .await?
            .ok_or_else(|| ApiError::not_found("Organization not found".to_string()))?;

        if org.not_onboarded(&OnboardingOperation::FirstDaemonRegistered) {
            let daemon_name = self
                .get_by_id(&daemon_id)
                .await?
                .map(|d| d.base.name.clone())
                .unwrap_or_else(|| "your daemon".to_string());

            tracing::info!(
                daemon_id = %daemon_id,
                organization_id = %org.id,
                "Emitting FirstDaemonRegistered telemetry on first contact"
            );

            self.event_bus
                .publish_onboarding(OnboardingEvent {
                    id: Uuid::new_v4(),
                    organization_id: org.id,
                    operation: OnboardingOperation::FirstDaemonRegistered,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({
                        "mode": "server_poll",
                        "daemon_name": daemon_name,
                        "network_name": network.base.name,
                    }),
                    authentication: AuthenticatedEntity::System,
                })
                .await?;
        }

        Ok(())
    }

    // ========================================================================
    // Polling loop methods (moved from poller.rs)
    // ========================================================================

    /// Start the ServerPoll polling loop. Should be called once from main.
    pub async fn start_polling_loop(
        self: Arc<Self>,
        email_service: Option<Arc<crate::server::email::traits::EmailService>>,
    ) {
        let poll_interval = Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS);
        let mut interval = tokio::time::interval(poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            if let Err(e) = self.poll_all_daemons(email_service.as_deref()).await {
                tracing::warn!("Daemon poller cycle failed: {}", e);
            }
        }
    }

    /// Poll all ServerPoll-mode daemons in parallel with semaphore-limited concurrency.
    /// Uses backon for per-daemon retries - daemon is marked unreachable after exhausting retries.
    async fn poll_all_daemons(
        &self,
        email_service: Option<&crate::server::email::traits::EmailService>,
    ) -> Result<()> {
        let daemons = self.get_server_poll_daemons().await?;

        if daemons.is_empty() {
            tracing::trace!("No ServerPoll daemons to poll");
            return Ok(());
        }

        tracing::debug!(
            "Polling {} ServerPoll daemons in parallel (max concurrent: {})",
            daemons.len(),
            MAX_CONCURRENT_POLLS
        );

        // Create parallel poll futures with semaphore-limited concurrency
        // Each daemon poll uses backon internally for retries
        let poll_futures: Vec<_> = daemons
            .into_iter()
            .map(|daemon| {
                let sem = self.poll_semaphore.clone();
                let daemon_id = daemon.id;
                let daemon_name = daemon.base.name.clone();
                async move {
                    let _permit = sem.acquire().await.expect("Semaphore closed");
                    // poll_daemon handles retries internally via backon
                    // Errors are logged inside poll_daemon, but log unexpected ones here too
                    if let Err(e) = self.poll_daemon(&daemon, email_service).await {
                        tracing::debug!(
                            daemon_id = %daemon_id,
                            daemon_name = %daemon_name,
                            error = %e,
                            "Poll cycle failed for daemon"
                        );
                    }
                }
            })
            .collect();

        // Execute all polls in parallel
        join_all(poll_futures).await;

        Ok(())
    }

    /// Get all daemons in ServerPoll mode that are reachable
    async fn get_server_poll_daemons(&self) -> Result<Vec<Daemon>> {
        let filter = StorableFilter::<Daemon>::new_for_daemon_poller_system_job();

        let reachable_server_poll_daemons = self.get_all(filter).await?;

        Ok(reachable_server_poll_daemons)
    }

    /// Mark a daemon as unreachable in the database and send notification
    async fn mark_daemon_unreachable(
        &self,
        daemon_id: Uuid,
        email_service: Option<&crate::server::email::traits::EmailService>,
    ) -> Result<()> {
        let mut daemon = self
            .get_by_id(&daemon_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Daemon {} not found", daemon_id))?;

        // Only notify if daemon was previously connected (skip if never seen = still setting up)
        let should_notify = daemon.base.last_seen.is_some();

        daemon.base.is_unreachable = true;

        self.update(&mut daemon, AuthenticatedEntity::System)
            .await?;

        if should_notify
            && let Some(email_service) = email_service
            && let Err(e) = self
                .send_unreachable_notification(&daemon, email_service)
                .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = %e,
                "Failed to send daemon unreachable notification email"
            );
        }

        Ok(())
    }

    /// Poll a single daemon for status and discovery data.
    /// Uses backon for retry with exponential backoff.
    /// Marks daemon unreachable after UNREACHABLE_THRESHOLD failures.
    ///
    /// Legacy daemons (< v0.14.0) are skipped entirely - they don't support
    /// the polling endpoints (/api/status, /api/poll, /api/first-contact,
    /// /api/discovery/entities-created). Legacy daemons stay alive via their
    /// own heartbeat calls to the server's backward-compat endpoint.
    async fn poll_daemon(
        &self,
        daemon: &Daemon,
        email_service: Option<&crate::server::email::traits::EmailService>,
    ) -> Result<()> {
        Self::warn_if_insecure_daemon_url(&daemon.base.url);

        // Skip polling for legacy daemons - they don't have the new endpoints
        if !daemon.supports_full_server_poll() {
            tracing::debug!(
                daemon_id = %daemon.id,
                daemon_name = %daemon.base.name,
                version = ?daemon.base.version,
                "Skipping poll for legacy daemon (< v0.14.0) - polling endpoints not supported"
            );
            return Ok(());
        }

        tracing::debug!(
            daemon_id = %daemon.id,
            daemon_name = %daemon.base.name,
            daemon_url = %daemon.base.url,
            api_key_id = ?daemon.base.api_key_id,
            "Starting poll for daemon"
        );

        // Get the API key for this daemon
        let api_key = match self.get_daemon_api_key(daemon).await {
            Ok(key) => key,
            Err(e) => {
                // API key lookup failure is a configuration error, not a network error.
                // Log it clearly so the user can fix it.
                tracing::error!(
                    daemon_id = %daemon.id,
                    daemon_name = %daemon.base.name,
                    error = %e,
                    "Failed to get API key for daemon - check that daemon has api_key_id set and the key has plaintext stored"
                );
                return Err(e);
            }
        };

        // Check if this is first contact (last_seen was None)
        let is_first_contact = daemon.base.last_seen.is_none();

        // Get status - either via first contact (which assigns daemon ID) or regular poll
        let status = if is_first_contact {
            tracing::info!(
                daemon_id = %daemon.id,
                daemon_name = %daemon.base.name,
                "First contact with ServerPoll daemon - assigning ID"
            );

            // Send first contact to assign daemon its server-side ID
            // This must succeed before we can proceed - without the correct ID,
            // discovery updates from the daemon won't be recognized by the server
            match self.send_first_contact(daemon, &api_key).await {
                Ok(status) => status,
                Err(e) => {
                    // First contact failed - abort poll entirely
                    // No point continuing since discovery updates won't work without correct ID
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        daemon_name = %daemon.base.name,
                        error = %e,
                        "First contact failed - aborting poll (will retry next cycle)"
                    );
                    // Mark unreachable after threshold failures
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        daemon_name = %daemon.base.name,
                        "Marking daemon unreachable after {} failures",
                        UNREACHABLE_THRESHOLD
                    );
                    if let Err(mark_err) =
                        self.mark_daemon_unreachable(daemon.id, email_service).await
                    {
                        tracing::error!(
                            daemon_id = %daemon.id,
                            "Failed to mark daemon as unreachable: {}",
                            mark_err
                        );
                    }
                    return Err(e);
                }
            }
        } else {
            // Regular status poll (retry is built into the helper)
            match self.poll_status(daemon, &api_key).await {
                Ok(status) => status,
                Err(e) => {
                    // Backon exhausted retries - mark daemon unreachable
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        daemon_name = %daemon.base.name,
                        "Marking daemon unreachable after {} failures",
                        UNREACHABLE_THRESHOLD
                    );
                    if let Err(mark_err) =
                        self.mark_daemon_unreachable(daemon.id, email_service).await
                    {
                        tracing::error!(
                            daemon_id = %daemon.id,
                            "Failed to mark daemon as unreachable: {}",
                            mark_err
                        );
                    }
                    return Err(e);
                }
            }
        };

        let auth = AuthenticatedEntity::System;

        tracing::debug!(
            daemon_id = %daemon.id,
            daemon_name = %daemon.base.name,
            ready_for_work = status.ready_for_work,
            has_docker_socket = status.has_docker_socket,
            interfaced_subnet_count = status.interfaced_subnets.len(),
            "ServerPoll status received"
        );

        // Process status data
        if let Err(e) = self
            .process_status(daemon.id, status.clone(), auth.clone())
            .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = ?e,
                "Failed to process daemon status"
            );
        }

        // If daemon has a version and it's different from what we have, process startup
        // (process_startup handles migration from legacy to unified discovery internally)
        if let Some(version) = status.version.clone()
            && daemon.base.version.as_ref() != Some(&version)
            && let Err(e) = self
                .process_startup(daemon.id, version.clone(), auth.clone())
                .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = ?e,
                "Failed to process daemon startup"
            );
        }

        // Capabilities are now resolved inside process_status() — no separate call needed.

        // First contact - create default discovery jobs and emit telemetry
        if is_first_contact {
            tracing::info!(
                daemon_id = %daemon.id,
                daemon_name = %daemon.base.name,
                "First contact with ServerPoll daemon"
            );

            // Determine if org is on Free plan for discovery defaults
            let is_free_plan = if let Ok(Some(network)) = self
                .network_service
                .get_by_id(&daemon.base.network_id)
                .await
            {
                if let Ok(Some(org)) = self
                    .organization_service
                    .get_by_id(&network.base.organization_id)
                    .await
                {
                    matches!(org.base.plan, Some(BillingPlan::Free(_)))
                } else {
                    false
                }
            } else {
                false
            };

            // Create default discovery jobs
            if let Err(e) = self
                .create_default_discovery_jobs(
                    daemon.id,
                    daemon.base.network_id,
                    daemon.base.host_id,
                    status.has_docker_socket,
                    is_free_plan,
                )
                .await
            {
                tracing::warn!(
                    daemon_id = %daemon.id,
                    error = ?e,
                    "Failed to create default discovery jobs"
                );
            }

            // Emit telemetry
            if let Err(e) = self
                .emit_first_daemon_telemetry(daemon.id, daemon.base.network_id)
                .await
            {
                tracing::warn!(
                    daemon_id = %daemon.id,
                    error = ?e,
                    "Failed to emit first daemon telemetry"
                );
            }
        }

        // Poll discovery data
        match self.poll_discovery(daemon, &api_key).await {
            Ok(poll_response) => {
                let auth = AuthenticatedEntity::System;

                // Process progress update if available
                if let Some(progress) = poll_response.progress
                    && let Err(e) = self.process_discovery_progress(progress).await
                {
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        error = ?e,
                        "Failed to process discovery progress"
                    );
                }

                // Process entities if any
                if !poll_response.entities.is_empty() {
                    match self
                        .process_discovery_entities(poll_response.entities, auth.clone())
                        .await
                    {
                        Ok(created_entities) => {
                            // Send created entities back to daemon
                            if let Err(e) = self
                                .send_created_entities(daemon, &api_key, created_entities)
                                .await
                            {
                                tracing::warn!(
                                    daemon_id = %daemon.id,
                                    "Failed to send created entities to daemon: {}",
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                daemon_id = %daemon.id,
                                error = ?e,
                                "Failed to process discovery entities"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!(
                    daemon_id = %daemon.id,
                    "Failed to poll daemon discovery: {}",
                    e
                );
            }
        }

        // Check for pending work and initiate if daemon reports ready
        if status.ready_for_work
            && let Some(work) = self.get_pending_work(daemon.id).await
        {
            let pending = self
                .discovery_service
                .get_pending_credential_ids_for_session(&work.session_id)
                .await;
            let request = self
                .discovery_service
                .build_daemon_request(&work, work.network_id, &pending)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to build daemon request: {}", e);
                    DaemonDiscoveryRequest {
                        session_id: work.session_id,
                        discovery_type: work.discovery_type,
                        credential_mappings: vec![],
                    }
                });
            if let Err(e) = self
                .send_discovery_request_to_daemon(daemon, Some(&api_key), request)
                .await
            {
                tracing::warn!(
                    daemon_id = %daemon.id,
                    "Failed to initiate discovery: {}",
                    e
                );
            }
        }

        Ok(())
    }

    /// Get the API key for a daemon (from the linked api_key_id)
    pub async fn get_daemon_api_key(&self, daemon: &Daemon) -> Result<String> {
        let api_key_id = daemon
            .base
            .api_key_id
            .ok_or_else(|| anyhow::anyhow!("Daemon {} has no linked API key", daemon.id))?;

        let api_key = self
            .daemon_api_key_service
            .get_by_id(&api_key_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("API key {} not found", api_key_id))?;

        // Get the plaintext key (stored for ServerPoll daemons)
        api_key
            .base
            .plaintext
            .as_ref()
            .map(|s| s.expose_secret().to_string())
            .ok_or_else(|| anyhow::anyhow!("API key {} has no stored plaintext", api_key_id))
    }

    // ========================================================================
    // Inactivity standby
    // ========================================================================

    /// Check all active daemons for inactivity (no completed discovery in 30 days)
    /// and put them on standby, sending notification emails if email service is available.
    pub async fn check_daemon_inactivity(
        &self,
        email_service: Option<&crate::server::email::traits::EmailService>,
    ) -> Result<()> {
        let cutoff = Utc::now() - chrono::Duration::days(30);

        // Get all non-standby daemons created more than 30 days ago
        let active_daemons = self
            .get_all(StorableFilter::<Daemon>::new_for_active_daemons().created_before(cutoff))
            .await?;

        for mut daemon in active_daemons {
            // Check for historical discoveries completed by this daemon
            let filter = StorableFilter::<Discovery>::new_from_uuid_column("daemon_id", &daemon.id)
                .historical_discovery();
            let discoveries = self.discovery_service.get_all(filter).await?;

            // Find the most recent finished_at from Historical discoveries
            let last_finished = discoveries
                .iter()
                .filter_map(|d| {
                    if let RunType::Historical { ref results } = d.base.run_type {
                        results.finished_at
                    } else {
                        None
                    }
                })
                .max();

            let should_standby = match last_finished {
                Some(finished) => finished < cutoff,
                None => true, // No historical records at all
            };

            if should_standby {
                daemon.base.standby = true;
                self.update(&mut daemon, AuthenticatedEntity::System)
                    .await?;
                tracing::info!(
                    daemon_id = %daemon.id,
                    "Set daemon to standby (inactive for 30+ days)"
                );

                // Send notification emails if email service is available
                if let Some(email_service) = email_service
                    && let Err(e) = self.send_standby_notification(&daemon, email_service).await
                {
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        error = %e,
                        "Failed to send daemon standby notification email"
                    );
                }
            }
        }

        Ok(())
    }

    /// Send standby notification email to org owner and daemon installer
    async fn send_standby_notification(
        &self,
        daemon: &Daemon,
        email_service: &crate::server::email::traits::EmailService,
    ) -> Result<()> {
        let network = self
            .network_service
            .get_by_id(&daemon.base.network_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network not found"))?;

        let org_id = network.base.organization_id;
        let network_name = &network.base.name;
        let daemon_name = &daemon.base.name;

        // Send to org owner
        let owners = email_service
            .user_service
            .get_organization_owners(&org_id)
            .await?;
        let owner = owners
            .first()
            .ok_or_else(|| anyhow::anyhow!("No owner found for organization {}", org_id))?;
        email_service
            .send_daemon_standby_email(owner.base.email.clone(), daemon_name, network_name)
            .await?;

        // Also send to daemon installer if different from owner
        if daemon.base.user_id != owner.id
            && let Some(user) = email_service
                .user_service
                .get_by_id(&daemon.base.user_id)
                .await?
        {
            email_service
                .send_daemon_standby_email(user.base.email, daemon_name, network_name)
                .await?;
        }

        Ok(())
    }

    /// Send unreachable notification email to org owner and daemon installer
    async fn send_unreachable_notification(
        &self,
        daemon: &Daemon,
        email_service: &crate::server::email::traits::EmailService,
    ) -> Result<()> {
        let network = self
            .network_service
            .get_by_id(&daemon.base.network_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network not found"))?;

        let org_id = network.base.organization_id;
        let network_name = &network.base.name;
        let daemon_name = &daemon.base.name;

        // Send to org owner
        let owners = email_service
            .user_service
            .get_organization_owners(&org_id)
            .await?;
        let owner = owners
            .first()
            .ok_or_else(|| anyhow::anyhow!("No owner found for organization {}", org_id))?;
        email_service
            .send_daemon_unreachable_email(owner.base.email.clone(), daemon_name, network_name)
            .await?;

        // Also send to daemon installer if different from owner
        if daemon.base.user_id != owner.id
            && let Some(user) = email_service
                .user_service
                .get_by_id(&daemon.base.user_id)
                .await?
        {
            email_service
                .send_daemon_unreachable_email(user.base.email, daemon_name, network_name)
                .await?;
        }

        Ok(())
    }

    /// After discovery creates a host with a loopback interface, scope any localhost-targeted
    /// credentials to specifically the loopback interface instead of all interfaces.
    async fn scope_loopback_credentials(
        &self,
        host_id: &Uuid,
        host_service: &HostService,
    ) -> Result<()> {
        // Find the loopback interface for this host
        let interfaces = host_service.get_interfaces_for_host(host_id).await?;
        let loopback_interface = interfaces.iter().find(|i| i.base.ip_address.is_loopback());

        let Some(loopback_iface) = loopback_interface else {
            return Ok(()); // No loopback interface on this host
        };
        let loopback_id = loopback_iface.id;

        // Get current credential assignments
        let assignments = self
            .credential_service
            .get_credential_assignments_for_host(host_id)
            .await?;

        let mut updated = false;
        let mut new_assignments = Vec::new();
        for assignment in assignments {
            if assignment.interface_ids.is_some() {
                new_assignments.push(assignment);
                continue;
            }

            // Check if this credential targets localhost
            let is_loopback_cred = match self
                .credential_service
                .get_by_id(&assignment.credential_id)
                .await
            {
                Ok(Some(cred)) => cred
                    .base
                    .target_ips
                    .as_ref()
                    .is_some_and(|ips| ips.iter().any(|ip| ip.is_loopback())),
                _ => false,
            };

            if is_loopback_cred {
                new_assignments.push(CredentialAssignment {
                    credential_id: assignment.credential_id,
                    interface_ids: Some(vec![loopback_id]),
                });
                updated = true;
            } else {
                new_assignments.push(assignment);
            }
        }

        if updated {
            self.credential_service
                .set_host_credentials(host_id, &new_assignments)
                .await?;
            tracing::debug!(
                host_id = %host_id,
                loopback_interface_id = %loopback_id,
                "Scoped loopback credentials to loopback interface"
            );
        }

        Ok(())
    }
}
