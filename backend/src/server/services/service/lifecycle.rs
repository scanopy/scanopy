//! Construction, ordered queries, locking, and host-service dependency injection.
use super::*;
use crate::server::shared::relations as R;

impl ServiceService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Service>>,
        binding_service: Arc<BindingService>,
        dependency_service: Arc<DependencyService>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        subnet_service: Arc<SubnetService>,
    ) -> Self {
        Self {
            storage,
            binding_service,
            dependency_service,
            host_service: OnceLock::new(),
            event_bus,
            entity_tag_service,
            subnet_service,
        }
    }

    /// Get all services matching filter, ordered by the specified column.
    /// Also loads bindings and tags for each service.
    pub async fn get_all_ordered(
        &self,
        filter: StorableFilter<Service>,
        order_by: &str,
    ) -> Result<Vec<Service>> {
        let mut services = self.storage.get_all_ordered(filter, order_by).await?;
        if services.is_empty() {
            return Ok(services);
        }

        let service_ids: Vec<Uuid> = services.iter().map(|s| s.id).collect();
        let bindings_map = self
            .binding_service
            .get_for_many(&R::BINDING_SERVICE, &service_ids)
            .await?;

        for service in &mut services {
            if let Some(bindings) = bindings_map.get(&service.id) {
                service.base.bindings = bindings.clone();
            }
        }

        self.bulk_hydrate_tags(&mut services, None).await?;

        Ok(services)
    }

    /// Get paginated services matching filter, ordered by the specified column.
    /// Also loads bindings and tags for each service.
    pub async fn get_paginated_ordered(
        &self,
        filter: StorableFilter<Service>,
        order_by: &str,
    ) -> Result<PaginatedResult<Service>> {
        let mut paginated = self.storage.get_paginated(filter, order_by).await?;

        if !paginated.items.is_empty() {
            let service_ids: Vec<Uuid> = paginated.items.iter().map(|s| s.id).collect();
            let bindings_map = self
                .binding_service
                .get_for_many(&R::BINDING_SERVICE, &service_ids)
                .await?;

            for service in &mut paginated.items {
                if let Some(bindings) = bindings_map.get(&service.id) {
                    service.base.bindings = bindings.clone();
                }
            }

            self.bulk_hydrate_tags(&mut paginated.items, None).await?;
        }

        Ok(paginated)
    }

    pub fn set_host_service(&self, host_service: Arc<HostService>) -> Result<(), Arc<HostService>> {
        self.host_service.set(host_service)
    }
}
