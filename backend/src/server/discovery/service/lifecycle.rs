//! Construction and lazy dependency injection.
use super::*;

impl DiscoveryService {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        discovery_storage: Arc<GenericPostgresStorage<Discovery>>,
        subnet_target_storage: Arc<GenericChildStorage<DiscoverySubnet>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        credential_service: Arc<CredentialService>,
        site_service: Arc<SiteService>,
        organization_service: Arc<OrganizationService>,
        warning_service: Arc<DiscoveryWarningService>,
        deployment_type: DeploymentType,
    ) -> Result<Arc<Self>> {
        let (tx, _rx) = broadcast::channel(100); // Buffer 100 messages
        let scheduler = JobScheduler::new().await?;
        let scheduler = Some(Arc::new(scheduler));

        Ok(Arc::new_cyclic(|weak| Self {
            self_ref: weak.clone(),
            discovery_storage,
            subnet_target_storage,
            sessions: RwLock::new(HashMap::new()),
            daemon_sessions: RwLock::new(HashMap::new()),
            discovery_sessions: RwLock::new(HashMap::new()),
            daemon_pull_cancellations: RwLock::new(HashMap::new()),
            running_snapshots: RwLock::new(HashSet::new()),
            superseded_wire_daemons: RwLock::new(HashSet::new()),
            touched_subnets: RwLock::new(HashMap::new()),
            withheld_credentials: RwLock::new(HashMap::new()),
            session_last_updated: RwLock::new(HashMap::new()),
            update_tx: tx,
            scheduler,
            job_ids: RwLock::new(HashMap::new()),
            event_bus,
            entity_tag_service,
            credential_service,
            site_service,
            organization_service,
            warning_service,
            deployment_type,
            daemon_service: std::sync::OnceLock::new(),
            host_service: std::sync::OnceLock::new(),
            digest_service: std::sync::OnceLock::new(),
        }))
    }

    /// Set the daemon service dependency after construction.
    /// This breaks the circular dependency: DaemonService holds Arc<DiscoveryService>,
    /// and DiscoveryService holds OnceLock<Arc<DaemonService>>.
    pub fn set_daemon_service(
        &self,
        service: Arc<DaemonService>,
    ) -> Result<(), Arc<DaemonService>> {
        self.daemon_service.set(service)
    }

    /// Set the host service after construction. `HostService` reaches `DiscoveryService`
    /// through `DaemonService`, so it is built later.
    pub fn set_host_service(&self, service: Arc<HostService>) -> Result<(), Arc<HostService>> {
        self.host_service.set(service)
    }

    /// Set the digest service after construction. `DiscoveryDigestService` holds
    /// `Arc<DiscoveryService>`.
    pub fn set_digest_service(
        &self,
        service: Arc<DiscoveryDigestService>,
    ) -> Result<(), Arc<DiscoveryDigestService>> {
        self.digest_service.set(service)
    }
}
