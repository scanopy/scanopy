use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

use crate::server::{
    ports::r#impl::base::Port,
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{generic::GenericPostgresStorage, traits::Storage},
    },
    tags::entity_tags::EntityTagService,
};

pub struct PortService {
    storage: Arc<GenericPostgresStorage<Port>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<Port> for PortService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &Port) -> Option<Uuid> {
        Some(entity.base.site_id)
    }

    fn get_organization_id(&self, _entity: &Port) -> Option<Uuid> {
        None
    }
}

impl CrudService<Port> for PortService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Port>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl ChildCrudService<Port> for PortService {}

impl PortService {
    pub fn new(storage: Arc<GenericPostgresStorage<Port>>, event_bus: Arc<EventBus>) -> Self {
        Self { storage, event_bus }
    }

    /// Get all ports for a specific host (alias for get_for_parent)
    pub async fn get_for_host(&self, host_id: &Uuid) -> Result<Vec<Port>> {
        self.get_for_parent(host_id).await
    }

    /// Get ports for multiple hosts. `at = None` reads live rows; `Some(t)`
    /// reads SCD2 state as of `t` (snapshot-view hydration).
    pub async fn get_for_hosts(
        &self,
        host_ids: &[Uuid],
        at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<HashMap<Uuid, Vec<Port>>> {
        if host_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter =
            crate::server::shared::storage::filter::StorableFilter::<Port>::new_from_host_ids(
                host_ids,
            )
            .live_or_as_of(at);
        let ports = self.storage.get_all(filter).await?;

        let mut result: HashMap<Uuid, Vec<Port>> = HashMap::new();
        for port in ports {
            result.entry(port.base.host_id).or_default().push(port);
        }
        Ok(result)
    }
}
