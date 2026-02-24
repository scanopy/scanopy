//! Daemon service for managing daemon lifecycle, processing, and polling.
//!
//! This service consolidates:
//! - CRUD operations for daemons
//! - Processing logic for daemon data (formerly in processor.rs)
//! - Polling loop for ServerPoll mode (formerly in poller.rs)
//! - HTTP client for daemon communication

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Error, Result, anyhow};
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
use crate::server::daemon_api_keys::service::DaemonApiKeyService;
use crate::server::daemons::r#impl::api::{
    DaemonCapabilities, DaemonDiscoveryRequest, DaemonRegistrationRequest,
    DaemonRegistrationResponse, DiscoveryUpdatePayload, FirstContactRequest, ServerCapabilities,
};
use crate::server::daemons::r#impl::base::{Daemon, DaemonBase, DaemonMode};
use crate::server::daemons::r#impl::version::DaemonVersionPolicy;
use crate::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback, RunType};
use crate::server::discovery::service::DiscoveryService;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::service::HostService;
use crate::server::networks::r#impl::Network;
use crate::server::networks::service::NetworkService;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::types::{TelemetryEvent, TelemetryOperation};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::api::{ApiError, ApiResponse};
use crate::server::shared::types::entities::EntitySource;
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::snmp_credentials::r#impl::discovery::SnmpCredentialMapping;
use crate::server::subnets::service::SubnetService;
use crate::server::tags::entity_tags::EntityTagService;
use crate::server::users::service::UserService;
use axum::http::StatusCode;

/// Daily midnight cron schedule for default discovery jobs
const DAILY_MIDNIGHT_CRON: &str = "0 0 0 * * *";

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

    // TLS enforcement for daemon URLs in ServerPoll mode
    require_daemon_tls: bool,
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
}

impl DaemonService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        daemon_storage: Arc<GenericPostgresStorage<Daemon>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        discovery_service: Arc<DiscoveryService>,
        subnet_service: Arc<SubnetService>,
        network_service: Arc<NetworkService>,
        organization_service: Arc<OrganizationService>,
        user_service: Arc<UserService>,
        daemon_api_key_service: Arc<DaemonApiKeyService>,
        require_daemon_tls: bool,
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
            subnet_service,
            network_service,
            organization_service,
            user_service,
            daemon_api_key_service,
            host_service: std::sync::OnceLock::new(),
            poll_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_POLLS)),
            require_daemon_tls,
        }
    }

    /// Validates that a daemon URL uses HTTPS when TLS is required.
    /// Localhost addresses are exempt (development/integrated daemon).
    fn validate_daemon_url_tls(&self, url: &str) -> Result<()> {
        if !self.require_daemon_tls {
            return Ok(());
        }
        let parsed = url::Url::parse(url).map_err(|_| anyhow!("Invalid daemon URL"))?;
        if parsed.scheme() == "https" {
            return Ok(());
        }
        // Allow localhost exemption for integrated daemons
        if let Some(host) = parsed.host_str()
            && (host == "localhost" || host == "127.0.0.1" || host == "::1")
        {
            return Ok(());
        }
        Err(anyhow!("Daemon URL must use HTTPS when TLS is required"))
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
        // Validate TLS for daemon URL
        self.validate_daemon_url_tls(&daemon.base.url)?;

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %request.session_id,
            "Sending discovery request to daemon"
        );

        let _: Option<serde_json::Value> = self
            .post_to_daemon(daemon, api_key, "/api/discovery/initiate", &request)
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

    /// Process a heartbeat from a daemon
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
        let mut daemon = self
            .get_by_id(&daemon_id)
            .await?
            .ok_or_else(|| ApiError::entity_not_found::<Daemon>(daemon_id))?;

        daemon.base.version = Some(version.clone());
        daemon.base.last_seen = Some(Utc::now());

        self.update(&mut daemon, auth).await?;

        tracing::info!(daemon_id = %daemon_id, version = %version, "Daemon startup");

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

        // Check DaemonPoll restriction
        if request.mode == DaemonMode::DaemonPoll
            && let Some(plan) = &org.base.plan
            && !plan.features().daemon_poll
        {
            return Err(ApiError::coded(
                StatusCode::FORBIDDEN,
                ErrorCode::BillingFeatureNotAvailable {
                    feature: "DaemonPoll".into(),
                },
            ));
        }

        tracing::info!("{:?}", request);

        // Parse version early for use in server_capabilities
        let daemon_version = request
            .version
            .as_ref()
            .and_then(|v| semver::Version::parse(v).ok());

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
            if let Some(v) = daemon_version.clone() {
                existing_daemon.base.version = Some(v);
            }

            let updated_daemon = self.update(&mut existing_daemon, auth).await?;

            return Ok(DaemonRegistrationResponse {
                daemon: updated_daemon,
                host_id: existing_daemon.base.host_id,
                server_capabilities,
            });
        }

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
            snmp_credential_id: None,
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
        host_limit: Option<u64>,
    ) -> Result<CreatedEntitiesPayload, ApiError> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| ApiError::internal_error("HostService not initialized"))?;

        let mut created_hosts = Vec::new();
        let mut created_subnets = Vec::new();
        let mut host_failures = 0;
        let mut subnet_failures = 0;

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
                    host_limit,
                )
                .await
            {
                Ok(host_response) => {
                    created_hosts.push((pending_id, host_response));
                }
                Err(e) => {
                    host_failures += 1;
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
            && org.not_onboarded(&TelemetryOperation::FirstHostDiscovered)
        {
            let _ = self
                .event_bus
                .publish_telemetry(TelemetryEvent::new(
                    Uuid::new_v4(),
                    org.id,
                    TelemetryOperation::FirstHostDiscovered,
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
                cron_schedule: DAILY_MIDNIGHT_CRON.to_string(),
                last_run: None,
                enabled: true,
            }
        };

        // Create SelfReport discovery job
        let self_report_discovery_type = DiscoveryType::SelfReport { host_id };

        let self_report_discovery = self
            .discovery_service
            .create_discovery(
                Discovery::new(DiscoveryBase {
                    run_type: default_run_type.clone(),
                    discovery_type: self_report_discovery_type.clone(),
                    name: self_report_discovery_type.to_string(),
                    daemon_id,
                    network_id,
                    tags: Vec::new(),
                }),
                AuthenticatedEntity::System,
            )
            .await?;

        self.discovery_service
            .start_session(self_report_discovery, AuthenticatedEntity::System)
            .await?;

        // Create Docker discovery job if daemon has docker socket
        if has_docker_socket {
            let docker_discovery_type = DiscoveryType::Docker {
                host_id,
                host_naming_fallback: HostNamingFallback::BestService,
            };

            let docker_discovery = self
                .discovery_service
                .create_discovery(
                    Discovery::new(DiscoveryBase {
                        run_type: default_run_type.clone(),
                        discovery_type: docker_discovery_type.clone(),
                        name: docker_discovery_type.to_string(),
                        daemon_id,
                        network_id,
                        tags: Vec::new(),
                    }),
                    AuthenticatedEntity::System,
                )
                .await?;

            self.discovery_service
                .start_session(docker_discovery, AuthenticatedEntity::System)
                .await?;
        }

        // Create Network discovery job; snmp hydrated at session start
        let network_discovery_type = DiscoveryType::Network {
            subnet_ids: None,
            host_naming_fallback: HostNamingFallback::BestService,
            snmp_credentials: SnmpCredentialMapping::default(),
            probe_raw_socket_ports: false,
        };

        let network_discovery = self
            .discovery_service
            .create_discovery(
                Discovery::new(DiscoveryBase {
                    run_type: default_run_type.clone(),
                    discovery_type: network_discovery_type.clone(),
                    name: network_discovery_type.to_string(),
                    daemon_id,
                    network_id,
                    tags: Vec::new(),
                }),
                AuthenticatedEntity::System,
            )
            .await?;

        self.discovery_service
            .start_session(network_discovery, AuthenticatedEntity::System)
            .await?;

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

        if org.not_onboarded(&TelemetryOperation::FirstDaemonRegistered) {
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
                .publish_telemetry(TelemetryEvent {
                    id: Uuid::new_v4(),
                    organization_id: org.id,
                    operation: TelemetryOperation::FirstDaemonRegistered,
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
    pub async fn start_polling_loop(self: Arc<Self>) {
        let poll_interval = Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS);
        let mut interval = tokio::time::interval(poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            if let Err(e) = self.poll_all_daemons().await {
                tracing::warn!("Daemon poller cycle failed: {}", e);
            }
        }
    }

    /// Poll all ServerPoll-mode daemons in parallel with semaphore-limited concurrency.
    /// Uses backon for per-daemon retries - daemon is marked unreachable after exhausting retries.
    async fn poll_all_daemons(&self) -> Result<()> {
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
                    if let Err(e) = self.poll_daemon(&daemon).await {
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

    /// Mark a daemon as unreachable in the database
    async fn mark_daemon_unreachable(&self, daemon_id: Uuid) -> Result<()> {
        let mut daemon = self
            .get_by_id(&daemon_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Daemon {} not found", daemon_id))?;

        daemon.base.is_unreachable = true;

        self.update(&mut daemon, AuthenticatedEntity::System)
            .await?;

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
    async fn poll_daemon(&self, daemon: &Daemon) -> Result<()> {
        // Validate TLS for daemon URL
        self.validate_daemon_url_tls(&daemon.base.url)?;

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
                    if let Err(mark_err) = self.mark_daemon_unreachable(daemon.id).await {
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
                    if let Err(mark_err) = self.mark_daemon_unreachable(daemon.id).await {
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

        // If daemon has a version and we haven't recorded it yet, process startup
        if let Some(version) = status.version.clone()
            && daemon.base.version.is_none()
            && let Err(e) = self.process_startup(daemon.id, version, auth.clone()).await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = ?e,
                "Failed to process daemon startup"
            );
        }

        // Update capabilities if changed
        if daemon.base.capabilities != status.capabilities
            && let Err(e) = self
                .process_capabilities(daemon.id, status.capabilities.clone(), auth.clone())
                .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = ?e,
                "Failed to process daemon capabilities"
            );
        }

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
                    status.capabilities.has_docker_socket,
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
                        .process_discovery_entities(poll_response.entities, auth.clone(), None)
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

        // Check for pending work and initiate if available
        if let Some(work) = self.get_pending_work(daemon.id).await {
            let request = DaemonDiscoveryRequest {
                session_id: work.session_id,
                discovery_type: work.discovery_type,
            };
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
}
