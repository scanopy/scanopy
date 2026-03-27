use anyhow::{Error, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use std::{fmt::Display, sync::Arc};
use uuid::Uuid;

use std::collections::HashMap;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, Entity as EntityEnum},
        events::{
            bus::EventBus,
            types::{EntityEvent, EntityOperation},
        },
        storage::{
            child::ChildStorableEntity,
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, PaginatedResult, Storage},
        },
    },
    tags::entity_tags::EntityTagService,
};

pub trait EventBusService<T: Into<EntityEnum> + Default> {
    /// Event bus and helpers
    fn event_bus(&self) -> &Arc<EventBus>;

    fn get_network_id(&self, entity: &T) -> Option<Uuid>;
    fn get_organization_id(&self, entity: &T) -> Option<Uuid>;

    /// Whether to suppress activity logs for this operation.
    /// Returns true if only non-significant fields changed (e.g., last_seen timestamps).
    /// - Create: current is None, updated is Some
    /// - Update: both are Some
    /// - Delete: current is Some, updated is None
    ///
    /// Default implementation returns false (don't suppress).
    fn suppress_logs(&self, _current: Option<&T>, _updated: Option<&T>) -> bool {
        false
    }
}

/// Helper trait for services that use generic storage
/// Provides default implementations for common CRUD operations
#[async_trait]
pub trait CrudService<T: Entity + Into<EntityEnum> + Default>: EventBusService<T>
where
    T: Display + ChangeTriggersTopologyStaleness<T>,
{
    /// Get reference to the storage
    fn storage(&self) -> &Arc<GenericPostgresStorage<T>>;

    /// Get entity tag service, if it supports tags
    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>>;

    /// Get entity by ID
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<T>, anyhow::Error> {
        if let Some(mut entity) = self.storage().get_by_id(id).await? {
            self.hydrate_tags(&mut entity).await?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }

    async fn hydrate_tags(&self, entity: &mut T) -> Result<(), Error> {
        if let Some(entity_tag_service) = self.entity_tag_service() {
            let tags = entity_tag_service
                .get_tags(&entity.id(), &T::entity_type())
                .await?;
            entity.set_tags(tags);
        }
        Ok(())
    }

    /// Hydrate tags from junction table for multiple entities, if they support it
    async fn bulk_hydrate_tags(&self, entities: &mut [T]) -> Result<(), Error> {
        if let Some(entity_tag_service) = self.entity_tag_service() {
            let ids: Vec<Uuid> = entities.iter().map(|e| e.id()).collect();
            let tags_map = entity_tag_service
                .get_tags_map(&ids, T::entity_type())
                .await?;
            for entity in entities {
                if let Some(tags) = tags_map.get(&entity.id()) {
                    entity.set_tags(tags.clone());
                }
            }
        }
        Ok(())
    }

    /// Get all entities with filter
    async fn get_all(&self, filter: StorableFilter<T>) -> Result<Vec<T>, anyhow::Error> {
        let mut all = self.storage().get_all(filter).await?;
        self.bulk_hydrate_tags(&mut all).await?;
        Ok(all)
    }

    /// Get entities with pagination, returning items and total count.
    async fn get_paginated(
        &self,
        filter: StorableFilter<T>,
    ) -> Result<PaginatedResult<T>, anyhow::Error> {
        let mut paginated = self
            .storage()
            .get_paginated(filter, "created_at ASC")
            .await?;
        let mut entities = paginated.items;
        self.bulk_hydrate_tags(&mut entities).await?;
        paginated.items = entities;
        Ok(paginated)
    }

    /// Get entities with pagination and custom ordering, returning items and total count.
    /// Hydrates tags from junction table.
    async fn get_paginated_ordered(
        &self,
        filter: StorableFilter<T>,
        order_by: &str,
    ) -> Result<PaginatedResult<T>, anyhow::Error> {
        let mut paginated = self.storage().get_paginated(filter, order_by).await?;
        self.bulk_hydrate_tags(&mut paginated.items).await?;
        Ok(paginated)
    }

    /// Get one entity with filter
    async fn get_one(&self, filter: StorableFilter<T>) -> Result<Option<T>, anyhow::Error> {
        if let Some(mut entity) = self.storage().get_one(filter).await? {
            self.hydrate_tags(&mut entity).await?;
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }

    /// Delete entity by ID
    async fn delete(
        &self,
        id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), anyhow::Error> {
        if let Some(entity) = self.get_by_id(id).await? {
            self.storage().delete(id).await?;

            let trigger_stale = entity.triggers_staleness(None);
            let suppress_logs = self.suppress_logs(Some(&entity), None);

            if let Some(entity_tag_service) = self.entity_tag_service() {
                entity_tag_service
                    .remove_all_for_entity(entity.id(), T::entity_type())
                    .await?;
            }

            self.event_bus()
                .publish_entity(EntityEvent {
                    id: Uuid::new_v4(),
                    entity_id: *id,
                    network_id: self.get_network_id(&entity),
                    organization_id: self.get_organization_id(&entity),
                    entity_type: entity.into(),
                    operation: EntityOperation::Deleted,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({
                        "trigger_stale": trigger_stale,
                        "suppress_logs": suppress_logs
                    }),
                    authentication,
                })
                .await?;

            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "{} with id {} not found",
                T::table_name(),
                id
            ))
        }
    }

    /// Base create implementation. Services that override `create()` should
    /// delegate to this method for the standard storage + event logic.
    async fn create_base(
        &self,
        entity: T,
        authentication: AuthenticatedEntity,
    ) -> Result<T, anyhow::Error> {
        let entity = if entity.id() == Uuid::nil() {
            T::new(entity.get_base())
        } else {
            entity
        };

        let created = self.storage().create(&entity).await?;
        let trigger_stale = created.triggers_staleness(None);
        let suppress_logs = self.suppress_logs(None, Some(&created));

        if let Some(entity_tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
            && let Some(tags) = created.get_tags()
        {
            entity_tag_service
                .set_tags(created.id(), T::entity_type(), tags.clone(), org_id)
                .await?;
        }

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: created.id(),
                network_id: self.get_network_id(&created),
                organization_id: self.get_organization_id(&created),
                entity_type: created.clone().into(),
                operation: EntityOperation::Created,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale,
                    "suppress_logs": suppress_logs
                }),
                authentication,
            })
            .await?;

        Ok(created)
    }

    /// Create entity. Override this to add post-creation logic (e.g. onboarding
    /// events), delegating to `create_base()` for the standard behavior.
    async fn create(
        &self,
        entity: T,
        authentication: AuthenticatedEntity,
    ) -> Result<T, anyhow::Error> {
        self.create_base(entity, authentication).await
    }

    /// Bulk create entities. Bypasses per-entity service logic for speed,
    /// but publishes a Created event for each entity so subscribers
    /// (e.g. topology) pick up the new data.
    async fn create_many(
        &self,
        entities: &[T],
        authentication: AuthenticatedEntity,
    ) -> Result<Vec<T>, anyhow::Error> {
        let created = self.storage().create_many(entities).await?;

        for entity in &created {
            let _ = self
                .event_bus()
                .publish_entity(EntityEvent {
                    id: Uuid::new_v4(),
                    entity_id: entity.id(),
                    network_id: self.get_network_id(entity),
                    organization_id: self.get_organization_id(entity),
                    entity_type: entity.clone().into(),
                    operation: EntityOperation::Created,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({}),
                    authentication: authentication.clone(),
                })
                .await;
        }

        Ok(created)
    }

    /// Update entity
    async fn update(
        &self,
        entity: &mut T,
        authentication: AuthenticatedEntity,
    ) -> Result<T, anyhow::Error> {
        let current = self
            .get_by_id(&entity.id())
            .await?
            .ok_or_else(|| anyhow!("Could not find {}", entity))?;
        let updated = self.storage().update(entity).await?;

        let trigger_stale = updated.triggers_staleness(Some(current.clone()));
        let suppress_logs = self.suppress_logs(Some(&current), Some(&updated));

        if let Some(entity_tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
            && let Some(tags) = updated.get_tags()
        {
            entity_tag_service
                .set_tags(updated.id(), T::entity_type(), tags.clone(), org_id)
                .await?;
        }

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: updated.id(),
                network_id: self.get_network_id(&updated),
                organization_id: self.get_organization_id(&updated),
                entity_type: updated.clone().into(),
                operation: EntityOperation::Updated,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale,
                    "suppress_logs": suppress_logs
                }),
                authentication,
            })
            .await?;

        Ok(updated)
    }

    async fn delete_many(
        &self,
        ids: &[Uuid],
        authentication: AuthenticatedEntity,
    ) -> Result<usize, anyhow::Error> {
        if ids.is_empty() {
            return Ok(0);
        }

        // Log which entities are being deleted
        for id in ids {
            if let Some(entity) = self.get_by_id(id).await? {
                let trigger_stale = entity.triggers_staleness(None);
                let suppress_logs = self.suppress_logs(Some(&entity), None);

                if let Some(entity_tag_service) = self.entity_tag_service() {
                    entity_tag_service
                        .remove_all_for_entity(entity.id(), T::entity_type())
                        .await?;
                }

                self.event_bus()
                    .publish_entity(EntityEvent {
                        id: Uuid::new_v4(),
                        entity_id: *id,
                        network_id: self.get_network_id(&entity),
                        organization_id: self.get_organization_id(&entity),
                        entity_type: entity.into(),
                        operation: EntityOperation::Deleted,
                        timestamp: Utc::now(),
                        metadata: serde_json::json!({
                            "trigger_stale": trigger_stale,
                            "suppress_logs": suppress_logs
                        }),
                        authentication: authentication.clone(),
                    })
                    .await?;
            }
        }

        let deleted_count = self.storage().delete_many(ids).await?;

        Ok(deleted_count)
    }

    /// Delete all entities for an organization
    async fn delete_all_for_org(
        &self,
        organization_id: &Uuid,
        network_ids: &[Uuid],
        authentication: AuthenticatedEntity,
    ) -> Result<usize, anyhow::Error> {
        let filter = if T::is_network_keyed() {
            StorableFilter::<T>::new_from_network_ids(network_ids)
        } else {
            StorableFilter::<T>::new_from_org_id(organization_id)
        };

        // Get entities for event publishing before deletion
        let entities = self.get_all(filter.clone()).await?;

        // Publish delete events
        for entity in &entities {
            let trigger_stale = entity.triggers_staleness(None);
            let suppress_logs = self.suppress_logs(Some(entity), None);

            if let Some(entity_tag_service) = self.entity_tag_service() {
                entity_tag_service
                    .remove_all_for_entity(entity.id(), T::entity_type())
                    .await?;
            }

            self.event_bus()
                .publish_entity(EntityEvent {
                    id: Uuid::new_v4(),
                    entity_id: entity.id(),
                    network_id: self.get_network_id(entity),
                    organization_id: self.get_organization_id(entity),
                    entity_type: entity.clone().into(),
                    operation: EntityOperation::Deleted,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({
                        "trigger_stale": trigger_stale,
                        "suppress_logs": suppress_logs
                    }),
                    authentication: authentication.clone(),
                })
                .await?;
        }

        // Delete all matching entities
        self.storage().delete_by_filter(filter).await
    }
}

/// Extension trait for services that manage child entities.
/// Provides parent-based query methods using the entity's ChildStorableEntity implementation.
#[async_trait]
pub trait ChildCrudService<T>: CrudService<T>
where
    T: ChildStorableEntity
        + Entity
        + Into<EntityEnum>
        + Default
        + Display
        + ChangeTriggersTopologyStaleness<T>,
{
    /// Get all entities for a single parent
    async fn get_for_parent(&self, parent_id: &Uuid) -> Result<Vec<T>, anyhow::Error> {
        let filter = StorableFilter::<T>::new_from_uuid_column(T::parent_column(), parent_id);
        self.get_all(filter).await
    }

    /// Get entities for multiple parents, grouped by parent_id
    async fn get_for_parents(
        &self,
        parent_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<T>>, anyhow::Error> {
        if parent_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<T>::new_from_uuids_column(T::parent_column(), parent_ids);
        let entities = self.get_all(filter).await?;
        // Note: get_all already hydrates tags, no need to call bulk_hydrate_tags again

        let mut result: HashMap<Uuid, Vec<T>> = HashMap::new();
        for entity in entities {
            result.entry(entity.parent_id()).or_default().push(entity);
        }

        Ok(result)
    }

    /// Save children for a parent (syncs children, preserving IDs where possible)
    ///
    /// This uses a sync pattern instead of delete-all + insert-all to preserve
    /// existing entity IDs. This is important for entities with foreign key
    /// references (like bindings referenced by group_bindings with ON DELETE CASCADE).
    ///
    /// Also preserves `created_at` timestamps for existing children and generates
    /// new UUIDs for children with nil IDs.
    ///
    /// Returns the saved entities with their actual IDs (including generated ones).
    async fn save_for_parent(
        &self,
        parent_id: &Uuid,
        children: &[T],
        authentication: AuthenticatedEntity,
    ) -> Result<Vec<T>, Error> {
        // Fetch full existing children to get their created_at timestamps
        let existing_children = self.get_for_parent(parent_id).await?;
        let existing_by_id: std::collections::HashMap<Uuid, T> =
            existing_children.into_iter().map(|c| (c.id(), c)).collect();

        let current_ids: std::collections::HashSet<Uuid> = existing_by_id.keys().cloned().collect();

        // Compute which IDs are in the new set (excluding nil UUIDs which will get new IDs)
        let new_ids: std::collections::HashSet<Uuid> = children
            .iter()
            .filter(|c| !c.id().is_nil())
            .map(|c| c.id())
            .collect();

        // Delete only children that are no longer present
        let ids_to_delete: Vec<Uuid> = current_ids.difference(&new_ids).cloned().collect();

        if !ids_to_delete.is_empty() {
            self.delete_many(&ids_to_delete, authentication.clone())
                .await?;
        }

        // Upsert children (insert or update), collecting the saved entities
        let mut saved: Vec<T> = Vec::with_capacity(children.len());
        for child in children {
            let mut child_clone = child.clone();

            let saved_child = if child.id().is_nil() {
                // New child with nil UUID - generate a proper ID
                child_clone.set_id(Uuid::new_v4());
                self.create(child_clone, authentication.clone()).await?
            } else if let Some(existing) = existing_by_id.get(&child.id()) {
                // Existing child - preserve created_at from database
                child_clone.set_created_at(existing.created_at());
                self.update(&mut child_clone, authentication.clone())
                    .await?
            } else {
                // New child with explicit ID
                self.create(child_clone, authentication.clone()).await?
            };

            saved.push(saved_child);
        }

        Ok(saved)
    }

    /// Delete all entities for a parent
    async fn delete_for_parent(
        &self,
        parent_id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<usize, anyhow::Error> {
        let filter = StorableFilter::<T>::new_from_uuid_column(T::parent_column(), parent_id);

        let entities = self.storage().get_all(filter.clone()).await?;

        for entity in entities {
            let trigger_stale = entity.triggers_staleness(None);
            let suppress_logs = self.suppress_logs(Some(&entity), None);

            if let Some(entity_tag_service) = self.entity_tag_service() {
                entity_tag_service
                    .remove_all_for_entity(entity.id(), T::entity_type())
                    .await?;
            }

            self.event_bus()
                .publish_entity(EntityEvent {
                    id: Uuid::new_v4(),
                    entity_id: entity.id(),
                    network_id: entity.network_id(),
                    organization_id: entity.organization_id(),
                    entity_type: entity.clone().into(),
                    operation: EntityOperation::Deleted,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({
                        "trigger_stale": trigger_stale,
                        "suppress_logs": suppress_logs
                    }),
                    authentication: authentication.clone(),
                })
                .await?;
        }

        self.storage().delete_by_filter(filter).await
    }
}
