//! Construction and daemon-service dependency injection.
use super::*;

impl DiscoveryService {
    pub async fn new(
        discovery_storage: Arc<GenericPostgresStorage<Discovery>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        credential_service: Arc<CredentialService>,
        network_service: Arc<NetworkService>,
        organization_service: Arc<OrganizationService>,
    ) -> Result<Arc<Self>> {
        let (tx, _rx) = broadcast::channel(100); // Buffer 100 messages
        let scheduler = JobScheduler::new().await?;
        let scheduler = Some(Arc::new(scheduler));

        Ok(Arc::new_cyclic(|weak| Self {
            self_ref: weak.clone(),
            discovery_storage,
            sessions: RwLock::new(HashMap::new()),
            daemon_sessions: RwLock::new(HashMap::new()),
            discovery_sessions: RwLock::new(HashMap::new()),
            daemon_pull_cancellations: RwLock::new(HashMap::new()),
            running_snapshots: RwLock::new(HashSet::new()),
            superseded_wire_daemons: RwLock::new(HashSet::new()),
            session_last_updated: RwLock::new(HashMap::new()),
            update_tx: tx,
            scheduler,
            job_ids: RwLock::new(HashMap::new()),
            event_bus,
            entity_tag_service,
            credential_service,
            network_service,
            organization_service,
            daemon_service: std::sync::OnceLock::new(),
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
}
