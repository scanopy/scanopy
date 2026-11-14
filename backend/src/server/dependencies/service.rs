use crate::server::shared::relations as R;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::server::shared::storage::traits::Unique;

use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::service::BindingService,
    dependencies::{
        dependency_members::{DependencyMemberRecord, DependencyMemberRecordBase},
        r#impl::base::{Dependency, DependencyMembers},
    },
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        events::{
            bus::EventBus,
            types::{EntityOperation, UpdateReason},
        },
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
    member_storage: Arc<GenericPostgresStorage<DependencyMemberRecord>>,
    binding_service: Arc<BindingService>,
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
                d.base.members = self.hydrate_members(&d.id, &d.base.members).await?;
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
        self.hydrate_members_batch(&mut dependencies).await?;
        Ok(dependencies)
    }

    async fn get_unique(
        &self,
        filter: StorableFilter<Dependency>,
    ) -> Result<Unique<Dependency>, anyhow::Error> {
        match self.storage().get_unique(filter).await? {
            Unique::One(mut d) => {
                self.entity_tag_service.hydrate_tags(&mut d).await?;
                d.base.members = self.hydrate_members(&d.id, &d.base.members).await?;
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
        self.hydrate_members_batch(&mut paginated.items).await?;
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
        self.save_members_for_dependency(&created.id, &dependency.base.members)
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
                .await;
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
        reason: UpdateReason,
    ) -> Result<Dependency, anyhow::Error> {
        let current = self
            .get_by_id(&dependency.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not find dependency"))?;

        let updated = self.storage().update(dependency).await?;

        // Save members to junction table
        self.save_members_for_dependency(&updated.id, &dependency.base.members)
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

        // Return with members populated
        let mut result = updated;
        result.base.members = dependency.base.members.clone();

        self.publish_updated(&current, &result, authentication, reason)
            .await;

        Ok(result)
    }
}

impl DependencyService {
    pub fn new(
        dependency_storage: Arc<GenericPostgresStorage<Dependency>>,
        member_storage: Arc<GenericPostgresStorage<DependencyMemberRecord>>,
        binding_service: Arc<BindingService>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            dependency_storage,
            member_storage,
            binding_service,
            event_bus,
            entity_tag_service,
        }
    }

    /// A dependency's member records, in position order.
    async fn member_records(&self, dependency_id: &Uuid) -> Result<Vec<DependencyMemberRecord>> {
        let filter =
            StorableFilter::<DependencyMemberRecord>::new_from_dependency_ids(&[*dependency_id]);
        self.member_storage
            .get_all_ordered(filter, "position ASC")
            .await
    }

    /// Fill a dependency's members from its member records. `members` already has the right
    /// variant (from the `member_type` column); this fills in the ids.
    async fn hydrate_members(
        &self,
        dependency_id: &Uuid,
        members: &DependencyMembers,
    ) -> Result<DependencyMembers> {
        let records = self.member_records(dependency_id).await?;
        Ok(members_from_records(members, records.iter()))
    }

    /// [`Self::hydrate_members`] for a batch of dependencies, in one query.
    async fn hydrate_members_batch(&self, dependencies: &mut [Dependency]) -> Result<()> {
        if dependencies.is_empty() {
            return Ok(());
        }
        let dependency_ids: Vec<Uuid> = dependencies.iter().map(|d| d.id).collect();
        let filter =
            StorableFilter::<DependencyMemberRecord>::new_from_dependency_ids(&dependency_ids);
        let records = self
            .member_storage
            .get_all_ordered(filter, "dependency_id ASC, position ASC")
            .await?;

        let mut records_by_dependency: HashMap<Uuid, Vec<&DependencyMemberRecord>> = HashMap::new();
        for record in &records {
            records_by_dependency
                .entry(record.dependency_id())
                .or_default()
                .push(record);
        }
        for dependency in dependencies.iter_mut() {
            if let Some(records) = records_by_dependency.get(&dependency.id) {
                dependency.base.members =
                    members_from_records(&dependency.base.members, records.iter().copied());
            }
        }
        Ok(())
    }

    /// Persist dependency members to the junction table, replacing what it held.
    ///
    /// A `Services` member is stored with no binding, each service once, in the given order. A
    /// `Bindings` member is stored with its binding's service. A member can name a binding that
    /// no longer exists: members outlive what they name, and the user decides what replaces it.
    /// Such a binding keeps the service its row recorded; one never recorded is an error.
    pub async fn save_members_for_dependency(
        &self,
        dependency_id: &Uuid,
        members: &DependencyMembers,
    ) -> Result<()> {
        let rows: Vec<DependencyMemberRecord> = match members {
            DependencyMembers::Services { service_ids } => {
                let mut seen = HashSet::new();
                service_ids
                    .iter()
                    .filter(|id| seen.insert(**id))
                    .enumerate()
                    .map(|(position, service_id)| {
                        DependencyMemberRecord::new(DependencyMemberRecordBase::new(
                            *dependency_id,
                            *service_id,
                            None,
                            position as i32,
                        ))
                    })
                    .collect()
            }
            DependencyMembers::Bindings { binding_ids } => {
                let recorded_services: HashMap<Uuid, Uuid> = self
                    .member_records(dependency_id)
                    .await?
                    .iter()
                    .filter_map(|r| r.binding_id().map(|b| (b, r.service_id())))
                    .collect();
                let mut rows = Vec::with_capacity(binding_ids.len());
                for (position, binding_id) in binding_ids.iter().enumerate() {
                    let service_id = match self.binding_service.get_by_id(binding_id).await? {
                        Some(binding) => binding.base.service_id,
                        None => *recorded_services
                            .get(binding_id)
                            .ok_or_else(|| anyhow::anyhow!("Binding {} not found", binding_id))?,
                    };
                    rows.push(DependencyMemberRecord::new(
                        DependencyMemberRecordBase::new(
                            *dependency_id,
                            service_id,
                            Some(*binding_id),
                            position as i32,
                        ),
                    ));
                }
                rows
            }
        };

        self.member_storage
            .replace_for(&R::DEPENDENCY_MEMBER_DEPENDENCY.at(*dependency_id), &rows)
            .await?;
        Ok(())
    }

    /// Bulk-insert pre-built member records for many dependencies at once, skipping the
    /// per-dependency lock/delete. For seed paths on a freshly reset org, where there are no
    /// rows to replace and no concurrent writers. Callers supply records with
    /// `service_id`/`binding_id`/`position` already resolved.
    pub async fn create_members(&self, records: &[DependencyMemberRecord]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        self.member_storage.create_many(records).await?;
        Ok(())
    }

    /// Every live member naming `binding_id`, across dependencies. For the subscriber that
    /// reports members losing what they name.
    pub(crate) async fn member_records_for_binding(
        &self,
        binding_id: &Uuid,
    ) -> Result<Vec<DependencyMemberRecord>> {
        self.member_storage
            .get_all(
                StorableFilter::<DependencyMemberRecord>::new_from_binding_id(binding_id).live(),
            )
            .await
    }

    /// Every live member recorded against `service_id`: the service itself, or a binding of it.
    pub(crate) async fn member_records_for_service(
        &self,
        service_id: &Uuid,
    ) -> Result<Vec<DependencyMemberRecord>> {
        self.member_storage
            .get_all(
                StorableFilter::<DependencyMemberRecord>::new_from_service_id(service_id).live(),
            )
            .await
    }
}

/// `members`' variant filled from member records, in the order given.
fn members_from_records<'a>(
    members: &DependencyMembers,
    records: impl Iterator<Item = &'a DependencyMemberRecord>,
) -> DependencyMembers {
    match members {
        DependencyMembers::Services { .. } => DependencyMembers::Services {
            service_ids: records.map(|r| r.service_id()).collect(),
        },
        DependencyMembers::Bindings { .. } => DependencyMembers::Bindings {
            binding_ids: records.filter_map(|r| r.binding_id()).collect(),
        },
    }
}
