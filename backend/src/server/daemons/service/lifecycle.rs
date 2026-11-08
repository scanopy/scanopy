//! Construction, limit checks, insecure-URL warning, and host-service dependency injection.
use super::*;

impl DaemonService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        daemon_storage: Arc<GenericPostgresStorage<Daemon>>,
        interfaced_subnet_storage: Arc<GenericChildStorage<DaemonInterfacedSubnet>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        discovery_service: Arc<DiscoveryService>,
        credential_service: Arc<CredentialService>,
        subnet_service: Arc<SubnetService>,
        vlan_service: Arc<VlanService>,
        site_service: Arc<SiteService>,
        organization_service: Arc<OrganizationService>,
        user_service: Arc<UserService>,
        daemon_api_key_service: Arc<DaemonApiKeyService>,
        deployment_type: crate::server::config::DeploymentType,
        trusted_ca: Option<&TrustedCaBundle>,
        has_email_service: bool,
    ) -> Self {
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
            vlan_service,
            site_service,
            organization_service,
            user_service,
            daemon_api_key_service,
            host_service: std::sync::OnceLock::new(),
            poll_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_POLLS)),
            deployment_type,
            has_email_service,
            live_update_tx: tokio::sync::broadcast::channel(100).0,
        }
    }

    /// Subscribe to daemon check-ins that changed a version or health.
    pub fn subscribe_live_daemon_updates(
        &self,
    ) -> tokio::sync::broadcast::Receiver<crate::server::live_updates::types::LiveDaemonUpdate>
    {
        self.live_update_tx.subscribe()
    }

    /// Subnet ids this daemon has interfaces on, from the
    /// `daemon_interfaced_subnets` junction. Errors degrade to an empty list — a
    /// junction read failure shouldn't fail a whole daemon fetch.
    pub async fn get_interfaced_subnet_ids(&self, daemon_id: &Uuid) -> Vec<Uuid> {
        let filter =
            StorableFilter::<DaemonInterfacedSubnet>::new_from_uuid_column("daemon_id", daemon_id);
        self.interfaced_subnet_storage
            .inner()
            .get_all_ordered(filter, "subnet_id ASC")
            .await
            .map(|records| records.into_iter().map(|r| r.subnet_id).collect())
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
        if daemon_ids.is_empty() {
            return std::collections::HashMap::new();
        }
        let filter = StorableFilter::<DaemonInterfacedSubnet>::new_from_uuids_column(
            "daemon_id",
            daemon_ids,
        );
        self.interfaced_subnet_storage
            .inner()
            .get_all_ordered(filter, "daemon_id ASC")
            .await
            .map(|records| {
                let mut map: std::collections::HashMap<Uuid, Vec<Uuid>> =
                    std::collections::HashMap::new();
                for record in records {
                    map.entry(record.daemon_id)
                        .or_default()
                        .push(record.subnet_id);
                }
                map
            })
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
        let rows: Vec<DaemonInterfacedSubnet> = subnet_ids
            .iter()
            .map(|subnet_id| DaemonInterfacedSubnet {
                daemon_id: *daemon_id,
                subnet_id: *subnet_id,
            })
            .collect();
        self.interfaced_subnet_storage
            .save_for_parent(
                &ParentRef::new(EntityDiscriminants::Daemon, *daemon_id),
                &rows,
            )
            .await
            .map(|_| ())
            .map_err(|e| {
                ApiError::internal_error(&format!("Failed to save interfaced subnets: {e}"))
            })
    }

    /// Which limit, if any, stops this org adding another daemon. An org gets one daemon
    /// while it is on Free, and one while no owner has verified their email. The first daemon
    /// is always allowed so users can experience core product value. Free comes first, because
    /// verifying an email would not lift it. The email limit is never reached on a server with
    /// no email transport, where no owner could receive a verification link.
    pub async fn daemon_limit_reached(
        &self,
        org_id: Uuid,
    ) -> Result<Option<DaemonLimit>, ApiError> {
        if !self.org_has_daemon(org_id).await {
            return Ok(None);
        }

        let on_free = self
            .organization_service
            .get_by_id(&org_id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?
            .is_some_and(|org| org.base.plan.is_free());
        if on_free {
            return Ok(Some(DaemonLimit::PlanRequired));
        }

        if !self.has_email_service {
            return Ok(None);
        }
        let owners = self
            .user_service
            .get_organization_owners(&org_id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
        if owners.iter().any(|u| u.base.email_verified) {
            return Ok(None);
        }
        Ok(Some(DaemonLimit::EmailVerification))
    }

    async fn org_has_daemon(&self, org_id: Uuid) -> bool {
        // Get sites for this org, then count daemons on those sites
        let sites = self
            .site_service
            .get_all(StorableFilter::<Site>::new_from_org_id(&org_id))
            .await
            .unwrap_or_default();

        let site_ids: Vec<Uuid> = sites.iter().map(|n| n.id).collect();
        if site_ids.is_empty() {
            return false;
        }

        !self
            .get_all(StorableFilter::<Daemon>::new_from_site_ids(&site_ids))
            .await
            .unwrap_or_default()
            .is_empty()
    }

    /// Refuse a new daemon while [`Self::daemon_limit_reached`].
    pub async fn check_daemon_limit(&self, org_id: Uuid) -> Result<(), ApiError> {
        use crate::server::shared::types::error_codes::ErrorCode;
        let code = match self.daemon_limit_reached(org_id).await? {
            None => return Ok(()),
            Some(DaemonLimit::PlanRequired) => ErrorCode::DaemonPlanRequired,
            Some(DaemonLimit::EmailVerification) => ErrorCode::AuthEmailVerificationRequired,
        };
        Err(ApiError::coded(axum::http::StatusCode::FORBIDDEN, code))
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
                 Ensure the connection is secured through other means (e.g., VPN, private site)"
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

    /// Takes the daemon off standby, so it picks up the work just queued for it. Grants the same
    /// grace window as the restart reactivation path, so the nightly inactivity check doesn't
    /// put it back on standby before that work completes. A no-op for an active daemon.
    pub async fn wake_from_standby(&self, daemon: &mut Daemon) -> Result<(), anyhow::Error> {
        if !daemon.base.standby {
            return Ok(());
        }
        daemon.base.standby = false;
        daemon.base.standby_cleared_at = Some(crate::server::shared::clock::now());
        self.update(
            daemon,
            AuthenticatedEntity::System,
            UpdateReason::DaemonStandbyChanged,
        )
        .await?;
        Ok(())
    }

    /// Marks an unreachable ServerPoll daemon reachable again, so the poll loop tries it on its
    /// next pass. A no-op for a daemon that is already reachable. The caller has checked that the
    /// daemon polls in ServerPoll mode.
    pub async fn retry_connection(
        &self,
        daemon: &mut Daemon,
        authentication: AuthenticatedEntity,
    ) -> Result<(), anyhow::Error> {
        if !daemon.base.is_unreachable {
            return Ok(());
        }
        daemon.base.is_unreachable = false;
        self.update(daemon, authentication, UpdateReason::DaemonRetryRequested)
            .await?;
        Ok(())
    }
}
