use crate::daemon::{
    discovery::{
        buffer::EntityBuffer, manager::DaemonDiscoverySessionManager,
        service::base::DaemonDiscoveryService,
    },
    runtime::{service::DaemonRuntimeService, state::DaemonState},
    shared::config::ConfigStore,
    utils::base::create_system_utils,
};
use anyhow::Result;
use std::sync::Arc;

pub struct DaemonServiceFactory {
    pub discovery_service: Arc<DaemonDiscoveryService>,
    pub discovery_manager: Arc<DaemonDiscoverySessionManager>,
    pub runtime_service: Arc<DaemonRuntimeService>,
    pub entity_buffer: Arc<EntityBuffer>,
    pub daemon_state: Arc<DaemonState>,
}

impl DaemonServiceFactory {
    pub async fn new(config: Arc<ConfigStore>) -> Result<Self> {
        // Initialize services with proper dependencies

        // Create entity buffer first - shared between discovery service and daemon state
        let entity_buffer = Arc::new(EntityBuffer::new());

        let discovery_service = Arc::new(DaemonDiscoveryService::new(
            config.clone(),
            entity_buffer.clone(),
        ));
        let discovery_manager = Arc::new(DaemonDiscoverySessionManager::new(
            discovery_service.clone(),
        ));
        let runtime_service = Arc::new(DaemonRuntimeService::new(
            config.clone(),
            discovery_manager.clone(),
        ));
        let daemon_state = Arc::new(DaemonState::new(
            config.clone(),
            create_system_utils(),
            discovery_service.clone(),
            entity_buffer.clone(),
            discovery_manager.clone(),
        ));

        Ok(Self {
            discovery_service,
            discovery_manager,
            runtime_service,
            entity_buffer,
            daemon_state,
        })
    }
}
