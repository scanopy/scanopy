use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::shared::storage::traits::Unique;

use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    dependencies::{
        dependency_members::{DependencyMemberRecord, DependencyMemberStorage},
        r#impl::base::{Dependency, DependencyMembers},
    },
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        events::{bus::EventBus, types::EntityOperation},
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{PaginatedResult, Storable, Storage},
        },
    },
    tags::entity_tags::EntityTagService,
};

pub struct DependencyService {
    dependency_storage: Arc<GenericPostgresStorage<Dependency>>,
    member_storage: Arc<DependencyMemberStorage>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<Dependency> for DependencyService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &Dependency) -> Option<Uuid> {
        Some(entity.base.site_id)
    }
    fn get_organization_id(&self, _entity: &Dependency) -> Option<Uuid> {
        None
    }
}

/// Helper: hydrate members for a batch of dependencies.
async fn hydrate_members_for_batch(
    member_storage: &DependencyMemberStorage,
    dependencies: &mut [Dependency],
) -> Result<()> {
    if dependencies.is_empty() {
        return Ok(());
    }
    let pairs: Vec<(Uuid, &DependencyMembers)> = dependencies
        .iter()
        .map(|d| (d.id, &d.base.members))
        .collect();
    let members_map = member_storage.hydrate_members_batch(&pairs).await?;
    for dep in dependencies.iter_mut() {
        if let Some(members) = members_map.get(&dep.id) {
            dep.base.members = members.clone();
        }
    }
    Ok(())
}

#[async_trait]
impl CrudService<Dependency> for DependencyService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Dependency>> {
        &self.dependency_storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Dependency>, anyhow::Error> {
        let dependency = self.storage().get_by_id(id).await?;
        match dependency {
            Some(mut d) => {
                self.entity_tag_service.hydrate_tags(&mut d).await?;
                d.base.members = self
                    .member_storage
                    .hydrate_members(&d.id, &d.base.members)
                    .await?;
                Ok(Some(d))
            }
            None => Ok(None),
        }
    }

    async fn get_all(
        &self,
        filter: StorableFilter<Dependency>,
    ) -> Result<Vec<Dependency>, anyhow::Error> {
        let mut dependencies = self.storage().get_all(filter).await?;
        self.entity_tag_service
            .hydrate_tags_batch(&mut dependencies)
            .await?;
        hydrate_members_for_batch(&self.member_storage, &mut dependencies).await?;
        Ok(dependencies)
    }

    async fn get_unique(
        &self,
        filter: StorableFilter<Dependency>,
    ) -> Result<Unique<Dependency>, anyhow::Error> {
        match self.storage().get_unique(filter).await? {
            Unique::One(mut d) => {
                self.entity_tag_service.hydrate_tags(&mut d).await?;
                d.base.members = self
                    .member_storage
                    .hydrate_members(&d.id, &d.base.members)
                    .await?;
                Ok(Unique::One(d))
            }
            Unique::None => Ok(Unique::None),
            Unique::Multiple => Ok(Unique::Multiple),
        }
    }

    async fn get_paginated(
        &self,
        filter: StorableFilter<Dependency>,
    ) -> Result<PaginatedResult<Dependency>, anyhow::Error> {
        self.get_paginated_ordered(filter, "created_at ASC").await
    }

    async fn get_paginated_ordered(
        &self,
        filter: StorableFilter<Dependency>,
        order_by: &str,
    ) -> Result<PaginatedResult<Dependency>, anyhow::Error> {
        let mut paginated = self.storage().get_paginated(filter, order_by).await?;
        self.entity_tag_service
            .hydrate_tags_batch(&mut paginated.items)
            .await?;
        hydrate_members_for_batch(&self.member_storage, &mut paginated.items).await?;
        Ok(paginated)
    }

    async fn create(
        &self,
        dependency: Dependency,
        authentication: AuthenticatedEntity,
    ) -> Result<Dependency, anyhow::Error> {
        let dependency = if dependency.id == Uuid::nil() {
            Dependency::new(dependency.base)
        } else {
            dependency
        };

        let created = self.storage().create(&dependency).await?;

        // Save members to junction table (service_ids_for_bindings = None for Services,
        // derived by handler for Bindings)
        self.member_storage
            .save_for_dependency(&created.id, &dependency.base.members)
            .await?;

        // Save tags to junction table
        if let Some(tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
        {
            tag_service
                .set_tags(
                    created.id,
                    EntityDiscriminants::Dependency,
                    dependency.base.tags,
                    org_id,
                )
                .await?;
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
                .await?;
        }

        // Return with members populated
        let mut result = created;
        result.base.members = dependency.base.members;
        Ok(result)
    }

    async fn update(
        &self,
        dependency: &mut Dependency,
        authentication: AuthenticatedEntity,
    ) -> Result<Dependency, anyhow::Error> {
        let current = self
            .get_by_id(&dependency.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not find dependency"))?;

        let updated = self.storage().update(dependency).await?;

        // Save members to junction table
        self.member_storage
            .save_for_dependency(&updated.id, &dependency.base.members)
            .await?;

        // Update tags in junction table
        if let Some(entity_tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
        {
            entity_tag_service
                .set_tags(
                    updated.id,
                    EntityDiscriminants::Dependency,
                    dependency.base.tags.clone(),
                    org_id,
                )
                .await?;
        }

        let trigger_stale = updated.triggers_staleness(Some(current));

        if let Some(scope) = EntityScope::from_ids(
            updated.id,
            updated.clone().into(),
            self.get_site_id(&updated),
            self.get_organization_id(&updated),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Updated, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        // Return with members populated
        let mut result = updated;
        result.base.members = dependency.base.members.clone();
        Ok(result)
    }
}

impl DependencyService {
    pub fn new(
        dependency_storage: Arc<GenericPostgresStorage<Dependency>>,
        member_storage: Arc<DependencyMemberStorage>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            dependency_storage,
            member_storage,
            event_bus,
            entity_tag_service,
        }
    }

    /// Persist dependency members to the junction table.
    /// Used by bulk creation paths (e.g. demo data) where `create_many`
    /// bypasses per-entity service logic.
    pub async fn save_members_for_dependency(
        &self,
        dependency_id: &Uuid,
        members: &DependencyMembers,
    ) -> Result<()> {
        self.member_storage
            .save_for_dependency(dependency_id, members)
            .await
    }

    /// Bulk-insert pre-built member records for many dependencies at once,
    /// skipping the per-dependency lock/delete. For seed paths on a freshly
    /// reset org. See [`DependencyMemberStorage::create_many`].
    pub async fn create_members(&self, records: &[DependencyMemberRecord]) -> Result<()> {
        self.member_storage.create_many(records).await
    }
}
