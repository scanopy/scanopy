use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::shared::relations as R;
use crate::server::shared::storage::traits::{PaginatedResult, Storable, Unique};
use crate::server::tags::entity_tags::EntityTagService;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::{
        r#impl::base::{Binding, BindingType},
        service::BindingService,
    },
    dependencies::{r#impl::base::Dependency, service::DependencyService},
    hosts::{r#impl::base::Host, service::HostService},
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    services::r#impl::{base::Service, patterns::MatchDetails},
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{
            bus::EventBus,
            types::{DeletionReason, EntityOperation, UpdateReason},
        },
        position::next_position,
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
            traits::Storage,
        },
        types::{api::ValidationError, entities::EntitySource},
    },
    subnets::service::SubnetService,
};
use anyhow::anyhow;
use anyhow::{Error, Result};
use async_trait::async_trait;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

pub struct ServiceService {
    storage: Arc<GenericPostgresStorage<Service>>,
    binding_service: Arc<BindingService>,
    host_service: OnceLock<Arc<HostService>>,
    dependency_service: Arc<DependencyService>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    /// Deleting a service deletes the bridge subnets it owns.
    subnet_service: Arc<SubnetService>,
}

impl EventBusService<Service> for ServiceService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &Service) -> Option<Uuid> {
        Some(entity.base.site_id)
    }
    fn get_organization_id(&self, _entity: &Service) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Service> for ServiceService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Service>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Service>, anyhow::Error> {
        let service = self.storage().get_by_id(id).await?;
        match service {
            Some(mut s) => {
                s.base.bindings = self
                    .binding_service
                    .get_for(&R::BINDING_SERVICE.at(s.id))
                    .await?;
                self.hydrate_tags(&mut s).await?;
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }

    async fn get_all(
        &self,
        filter: StorableFilter<Service>,
    ) -> Result<Vec<Service>, anyhow::Error> {
        let mut services = self.storage().get_all(filter).await?;
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

    async fn get_unique(
        &self,
        filter: StorableFilter<Service>,
    ) -> Result<Unique<Service>, anyhow::Error> {
        match self.storage().get_unique(filter).await? {
            Unique::One(mut s) => {
                s.base.bindings = self
                    .binding_service
                    .get_for(&R::BINDING_SERVICE.at(s.id))
                    .await?;
                self.hydrate_tags(&mut s).await?;
                Ok(Unique::One(s))
            }
            Unique::None => Ok(Unique::None),
            Unique::Multiple => Ok(Unique::Multiple),
        }
    }

    async fn get_paginated(
        &self,
        filter: StorableFilter<Service>,
    ) -> Result<PaginatedResult<Service>, anyhow::Error> {
        let mut paginated = self
            .storage()
            .get_paginated(filter, "created_at ASC")
            .await?;

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

    async fn create(
        &self,
        service: Service,
        authentication: AuthenticatedEntity,
    ) -> Result<Service> {
        let mut service = if service.id == Uuid::nil() {
            Service::new(service.base)
        } else {
            service
        };

        // Deduplicate bindings before validation
        service.base.bindings = Self::deduplicate_bindings(service.base.bindings);

        // DB-level lock, scoped to the host: serializes the natural-key
        // dedup AND next_position across all backend instances. Keyed by
        // host (not service id) because a new service's fresh UUID would
        // never contend, and position assignment reads the whole per-host
        // list. Error paths release via Drop.
        let dedup_guard = self
            .storage
            .session_lock(
                LockKey::ServiceDedup {
                    host_id: service.base.host_id,
                },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?;

        // SCD2: only live services on this host are candidates for natural-key match.
        let filter = StorableFilter::<Service>::new_from_host_ids(&[service.base.host_id]).live();
        let existing_services = self.get_all(filter).await?;

        // Auto-assign position for new services (next available position on host)
        let next_pos = next_position(&existing_services);

        let service_from_storage = match existing_services
            .into_iter()
            .find(|existing: &Service| *existing == service)
        {
            // If both are from discovery, or if they have the same ID but for some reason the create route is being used, upsert data
            Some(existing_service)
                if (service.base.source.is_from_discovery()
                    && existing_service.base.source.is_from_discovery())
                    || service.id == existing_service.id =>
            {
                self.upsert_service(existing_service, service, authentication)
                    .await?
            }
            _ => {
                // Auto-assign position (users cannot set position via /api/services)
                service.base.position = next_pos;

                // Validate bindings don't conflict with each other before creating
                Self::validate_bindings_no_conflicts(&service.base.bindings)?;

                // Validate bindings reference ports/interfaces on the service's host
                self.validate_bindings_belong_to_host(
                    &service.base.host_id,
                    &service.base.bindings,
                )
                .await?;

                // For non-discovery sources, validate bindings aren't already claimed by other services
                // Discovery sources handle conflicts via partition_conflicting_bindings in create_with_children
                if !service.base.source.is_from_discovery() {
                    self.validate_bindings_available(
                        &service.base.host_id,
                        &service.id,
                        &service.base.bindings,
                    )
                    .await?;
                }

                // SCD2 origin: this row is being inserted for the first
                // time. Stamp created_at + valid_from to the entity's
                // already-refreshed `last_seen_at`. See
                // `DiscoveryTracked::originate_scan_timestamps`.
                use crate::server::shared::storage::snapshot::DiscoveryTracked;
                let mut service = service;
                service.originate_scan_timestamps(service.last_seen_at);
                let mut created = self.storage.create(&service).await?;

                // Save bindings to separate table with correct service_id and site_id
                let bindings_with_ids: Vec<Binding> = service
                    .base
                    .bindings
                    .iter()
                    .cloned()
                    .map(|b| b.with_service(created.id, created.base.site_id))
                    .collect();
                let saved_bindings = self
                    .binding_service
                    .sync_for(
                        &R::BINDING_SERVICE.at(created.id),
                        &bindings_with_ids,
                        authentication.clone(),
                        DeletionReason::EntityRequested,
                        UpdateReason::EntityRequested,
                    )
                    .await?;

                // Update service with the saved bindings (which have actual IDs)
                created.base.bindings = saved_bindings;

                // Save tags to junction table
                if let Some(tag_service) = self.entity_tag_service()
                    && let Some(org_id) = authentication.organization_id()
                {
                    tag_service
                        .set_tags(
                            created.id,
                            EntityDiscriminants::Service,
                            service.base.tags.clone(),
                            org_id,
                        )
                        .await?;
                    created.base.tags = service.base.tags;
                }

                let trigger_stale = created.triggers_staleness(None);

                if let Some(scope) = EntityScope::from_ids(
                    created.id,
                    created.clone().into(),
                    self.get_site_id(&created),
                    self.get_organization_id(&created),
                ) {
                    self.event_bus()
                        .publish(
                            Event::new(scope, EntityOperation::Created, authentication).with_flags(
                                EntityEventFlags {
                                    trigger_stale,
                                    ..Default::default()
                                },
                            ),
                        )
                        .await;
                }

                created
            }
        };

        dedup_guard.release().await?;
        Ok(service_from_storage)
    }

    async fn update(
        &self,
        service: &mut Service,
        authentication: AuthenticatedEntity,
        reason: UpdateReason,
    ) -> Result<Service> {
        let lock_guard = self
            .storage
            .session_lock(LockKey::Service(service.id), DEFAULT_LOCK_TIMEOUT)
            .await?;

        let current_service = self
            .get_by_id(&service.id)
            .await?
            .ok_or_else(|| anyhow!("Could not find service"))?;

        // Deduplicate bindings before validation
        service.base.bindings =
            Self::deduplicate_bindings(std::mem::take(&mut service.base.bindings));

        // Validate bindings don't conflict with each other
        Self::validate_bindings_no_conflicts(&service.base.bindings)?;

        // Validate bindings reference ports/interfaces on the service's host
        self.validate_bindings_belong_to_host(&service.base.host_id, &service.base.bindings)
            .await?;

        // Validate bindings aren't already claimed by other services on this host
        self.validate_bindings_available(
            &service.base.host_id,
            &service.id,
            &service.base.bindings,
        )
        .await?;

        self.update_dependency_members(&current_service, Some(service), authentication.clone())
            .await?;

        let mut updated = self.storage.update(service).await?;

        // Save bindings to separate table with correct service_id and site_id
        let bindings_with_ids: Vec<Binding> = service
            .base
            .bindings
            .iter()
            .cloned()
            .map(|b| b.with_service(updated.id, updated.base.site_id))
            .collect();
        let saved_bindings = self
            .binding_service
            .sync_for(
                &R::BINDING_SERVICE.at(updated.id),
                &bindings_with_ids,
                authentication.clone(),
                DeletionReason::EntityRequested,
                reason.clone(),
            )
            .await?;

        // Update service with the saved bindings (which have actual IDs and preserved created_at)
        updated.base.bindings = saved_bindings;

        // Update tags in junction table
        if let Some(tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
        {
            tag_service
                .set_tags(
                    updated.id,
                    EntityDiscriminants::Service,
                    updated.base.tags,
                    org_id,
                )
                .await?;
            updated.base.tags = service.base.tags.clone();
        }

        self.publish_updated(&current_service, &updated, authentication, reason)
            .await;

        lock_guard.release().await?;
        Ok(updated)
    }

    /// Each service through [`Self::delete`], so a bulk delete deletes what each one virtualizes
    /// and publishes its own `Deleted`, the same as deleting them one at a time. One that an
    /// earlier delete in the batch already took with it is skipped.
    async fn delete_many(
        &self,
        ids: &[Uuid],
        authentication: AuthenticatedEntity,
        reason: DeletionReason,
    ) -> Result<usize> {
        let mut deleted = 0;
        for id in ids {
            if self.get_by_id(id).await?.is_some() {
                self.delete(id, authentication.clone(), reason.clone())
                    .await?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    async fn delete(
        &self,
        id: &Uuid,
        authentication: AuthenticatedEntity,
        reason: DeletionReason,
    ) -> Result<()> {
        let lock_guard = self
            .storage
            .session_lock(LockKey::Service(*id), DEFAULT_LOCK_TIMEOUT)
            .await?;

        let service = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Service {} not found", id))?;

        self.delete_virtualized(&service, authentication.clone())
            .await?;

        // Its dependency memberships stay: the member names an id that no longer resolves, the
        // dependency service's subscriber reports it (`DependencyMemberChange`), and the user
        // decides what replaces it.

        // Remove tags from junction table
        if let Some(tag_service) = self.entity_tag_service() {
            tag_service
                .remove_all_for_entity(*id, EntityDiscriminants::Service)
                .await?;
        }

        self.storage.delete(id).await?;

        let trigger_stale = service.triggers_staleness(None);

        if let Some(scope) = EntityScope::from_ids(
            service.id,
            service.clone().into(),
            self.get_site_id(&service),
            self.get_organization_id(&service),
        ) {
            self.event_bus()
                .publish(
                    Event::new(
                        scope,
                        EntityOperation::Deleted {
                            reason: reason.clone(),
                        },
                        authentication,
                    )
                    .with_flags(EntityEventFlags {
                        trigger_stale,
                        ..Default::default()
                    }),
                )
                .await;
        }

        lock_guard.release().await?;
        Ok(())
    }
}

mod binding_mutation;
mod binding_validation;
mod dependencies;
mod lifecycle;
mod transfer;
mod upsert;
mod virtualized;
