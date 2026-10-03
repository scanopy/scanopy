//! Construction, limit checks, insecure-URL warning, and host-service dependency injection.
use super::*;

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
        deployment_type: crate::server::config::DeploymentType,
        trusted_ca: Option<&TrustedCaBundle>,
    ) -> Self {
        let interfaced_subnet_storage =
            DaemonInterfacedSubnetStorage::new(daemon_storage.pool().clone());
        Self {
            daemon_storage,
            interfaced_subnet_storage,
            client: TrustedCaBundle::apply(
                trusted_ca,
                reqwest::Client::builder().timeout(Duration::from_secs(10)),
            )
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
            deployment_type,
        }
    }

    /// Subnet ids this daemon has interfaces on, from the
    /// `daemon_interfaced_subnets` junction. Errors degrade to an empty list — a
    /// junction read failure shouldn't fail a whole daemon fetch.
    pub async fn get_interfaced_subnet_ids(&self, daemon_id: &Uuid) -> Vec<Uuid> {
        self.interfaced_subnet_storage
            .get_subnet_ids_for_daemon(daemon_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = ?e, "Failed to load interfaced subnet ids");
                Vec::new()
            })
    }

    /// Batch variant of [`Self::get_interfaced_subnet_ids`] for list endpoints
    /// (avoids N+1).
    pub async fn get_interfaced_subnet_ids_batch(
        &self,
        daemon_ids: &[Uuid],
    ) -> std::collections::HashMap<Uuid, Vec<Uuid>> {
        self.interfaced_subnet_storage
            .get_subnet_ids_for_daemons(daemon_ids)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(error = ?e, "Failed to batch-load interfaced subnet ids");
                std::collections::HashMap::new()
            })
    }

    /// Replace this daemon's set of interfaced subnets in the
    /// `daemon_interfaced_subnets` junction. Every subnet id must already exist.
    pub async fn set_interfaced_subnet_ids(
        &self,
        daemon_id: &Uuid,
        subnet_ids: &[Uuid],
    ) -> Result<(), ApiError> {
        self.interfaced_subnet_storage
            .save_interfaced_subnets_for_daemon(daemon_id, subnet_ids)
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!("Failed to save interfaced subnets: {e}"))
            })
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
    pub(crate) fn warn_if_insecure_daemon_url(url: &str) {
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
}
