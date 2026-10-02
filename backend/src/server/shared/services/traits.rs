use anyhow::{Error, anyhow};
use async_trait::async_trait;
use std::{fmt::Display, sync::Arc};
use uuid::Uuid;

use std::collections::HashMap;

use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event as TypedEvent};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::types::api::{ApiError, GroupCount},
    shared::{
        entities::{ChangeTriggersTopologyStaleness, Entity as EntityEnum},
        events::{bus::EventBus, types::EntityOperation},
        storage::{
            child::ChildStorableEntity,
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, PaginatedResult, Storage, Unique},
        },
    },
    tags::entity_tags::EntityTagService,
};

pub trait EventBusService<T: Into<EntityEnum> + Default + Clone> {
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

/// Build a typed entity event from the per-service identity helpers + entity.
/// Returns `None` if neither network_id nor organization_id is available — in
/// that case the publish should be skipped.
fn build_entity_event<T, S>(
    bus_service: &S,
    entity: T,
    entity_id: Uuid,
    operation: EntityOperation,
    flags: EntityEventFlags,
    authentication: AuthenticatedEntity,
) -> Option<TypedEvent<EntityOperation>>
where
    T: Into<EntityEnum> + Default + Clone,
    S: EventBusService<T> + ?Sized,
{
    let network_id = bus_service.get_network_id(&entity);
    let organization_id = bus_service.get_organization_id(&entity);
    let scope = EntityScope::from_ids(entity_id, entity.into(), network_id, organization_id)?;
    Some(TypedEvent::new(scope, operation, authentication).with_flags(flags))
}

/// Helper trait for services that use generic storage
/// Provides default implementations for common CRUD operations
/// The entity `create_base` stores: server-owned id and timestamps.
///
/// A nil id gets a fresh entity. A client that mints its own id (the credential wizard, so its
/// targets can name the credential before it exists) keeps that id, but still sends the UI's
/// placeholder timestamps (the Unix epoch), which are replaced: creation order decides which of two
/// equally specific credentials a scan uses. Real timestamps a caller set on purpose are kept.
fn ready_for_create<T: Entity>(entity: T) -> T {
    if entity.id() == Uuid::nil() {
        T::new(entity.get_base())
    } else if entity.created_at() == chrono::DateTime::<chrono::Utc>::UNIX_EPOCH {
        let mut stamped = T::new(entity.get_base());
        stamped.set_id(entity.id());
        stamped
    } else {
        entity
    }
}

#[cfg(test)]
mod ready_for_create_tests {
    use super::*;
    use crate::server::credentials::r#impl::base::Credential;

    /// The wizard's own id survives, and the placeholder date does not reach storage.
    #[test]
    fn a_client_id_keeps_its_id_and_gets_real_timestamps() {
        let id = Uuid::new_v4();
        let sent = Credential {
            id,
            created_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
            updated_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
            ..Default::default()
        };
        let stored = ready_for_create(sent);
        assert_eq!(stored.id, id);
        assert!(stored.created_at > chrono::DateTime::<chrono::Utc>::UNIX_EPOCH);
    }

    #[test]
    fn a_real_timestamp_is_kept() {
        let when = chrono::Utc::now() - chrono::Duration::days(3);
        let sent = Credential {
            id: Uuid::new_v4(),
            created_at: when,
            updated_at: when,
            ..Default::default()
        };
        assert_eq!(ready_for_create(sent).created_at, when);
    }
}

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

    /// Hydrate tags from junction table for multiple entities, if they support it.
    ///
    /// `snapshot_id = None` hydrates from live associations; `Some(id)` hydrates
    /// from the associations captured under that snapshot (the entities must
    /// then be the snapshot's closed copies, so their ids match the closed
    /// `entity_tags`).
    async fn bulk_hydrate_tags(
        &self,
        entities: &mut [T],
        snapshot_id: Option<Uuid>,
    ) -> Result<(), Error> {
        if let Some(entity_tag_service) = self.entity_tag_service() {
            let ids: Vec<Uuid> = entities.iter().map(|e| e.id()).collect();
            let tags_map = entity_tag_service
                .get_tags_map(&ids, T::entity_type(), snapshot_id)
                .await?;
            for entity in entities {
                if let Some(tags) = tags_map.get(&entity.id()) {
                    entity.set_tags(tags.clone());
                }
            }
        }
        Ok(())
    }

    /// Get all entities with filter (live tag hydration).
    async fn get_all(&self, filter: StorableFilter<T>) -> Result<Vec<T>, anyhow::Error> {
        let mut all = self.storage().get_all(filter).await?;
        self.bulk_hydrate_tags(&mut all, None).await?;
        Ok(all)
    }

    /// Get all entities with filter, hydrating tags as-of a snapshot.
    ///
    /// Use when `filter` is already snapshot-scoped (so the rows are closed
    /// copies): tag associations hydrate from the same snapshot rather than
    /// from live `entity_tags`, which reference live (different) entity ids.
    ///
    /// Delegates to `get_all` (NOT `storage().get_all`) so per-service
    /// hydration overrides — e.g. `ServiceService::get_all` populating
    /// `bindings`, which the topology edge builder needs — still run. Then it
    /// re-hydrates tags as-of the snapshot, overwriting the live tags `get_all`
    /// applied. For the live view (`snapshot_id = None`) it is exactly `get_all`.
    async fn get_all_as_of_snapshot(
        &self,
        filter: StorableFilter<T>,
        snapshot_id: Option<Uuid>,
    ) -> Result<Vec<T>, anyhow::Error> {
        let mut all = self.get_all(filter).await?;
        if snapshot_id.is_some() {
            self.bulk_hydrate_tags(&mut all, snapshot_id).await?;
        }
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
        self.bulk_hydrate_tags(&mut entities, None).await?;
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
        self.bulk_hydrate_tags(&mut paginated.items, None).await?;
        Ok(paginated)
    }

    /// Count rows for the given networks. Scope is standardized here (not built
    /// at the call site): SCD2 entities are narrowed to live rows so snapshot
    /// closed-copies aren't counted; non-SCD2 entities get a plain count. For
    /// scopes more custom than network/org (e.g. parent-scoped junctions), drop
    /// to `storage().count(filter)` with a bespoke filter.
    async fn count_for_networks(&self, network_ids: &[Uuid]) -> Result<u64, anyhow::Error> {
        let mut filter = StorableFilter::<T>::new_from_network_ids(network_ids);
        if T::HAS_SCD2 {
            filter = filter.live();
        }
        self.storage().count(filter).await
    }

    /// Per-group totals for a grouped list, under the filter the list query
    /// already built. `group_sql` is the group field's ORDER BY expression, so
    /// the filter must already carry any JOIN that expression needs.
    async fn count_by_group(
        &self,
        filter: StorableFilter<T>,
        group_sql: &str,
    ) -> Result<Vec<GroupCount>, anyhow::Error> {
        Ok(self
            .storage()
            .count_by_group(filter, group_sql)
            .await?
            .into_iter()
            .map(GroupCount::from)
            .collect())
    }

    /// Count rows for an organization. Same SCD2-aware live narrowing as
    /// [`count_for_networks`].
    async fn count_for_org(&self, organization_id: &Uuid) -> Result<u64, anyhow::Error> {
        let mut filter = StorableFilter::<T>::new_from_org_id(organization_id);
        if T::HAS_SCD2 {
            filter = filter.live();
        }
        self.storage().count(filter).await
    }

    /// Whether any row matches.
    ///
    /// For guards that ask "does this exist" rather than "which one is it" — deleting a host with
    /// a daemon on it, rejecting a duplicate. Distinct from [`Self::get_unique`] on purpose: an
    /// existence check has nothing to say about uniqueness, and answering it with a lookup that
    /// treats several matches as an error turns a clear "delete the daemon first" into an
    /// internal error on exactly the data that most needs the guard.
    async fn exists(&self, filter: StorableFilter<T>) -> Result<bool, anyhow::Error> {
        self.storage().exists(filter).await
    }

    /// Fetch the one entity a filter identifies, with its tags hydrated.
    ///
    /// See [`Storage::get_unique`] for why this returns [`Unique`] rather than `Option`.
    async fn get_unique(&self, filter: StorableFilter<T>) -> Result<Unique<T>, anyhow::Error> {
        match self.storage().get_unique(filter).await? {
            Unique::One(mut entity) => {
                self.hydrate_tags(&mut entity).await?;
                Ok(Unique::One(entity))
            }
            Unique::None => Ok(Unique::None),
            Unique::Multiple => Ok(Unique::Multiple),
        }
    }

    /// Validate that every id in `ids` refers to a live entity owned by `org_id`.
    ///
    /// Loads the referenced rows and rejects if any requested id is missing
    /// (does not exist / is not visible) or belongs to a different organization.
    /// Call this before persisting caller-supplied foreign-key references (e.g.
    /// junction rows) so a caller cannot reference another tenant's entities by
    /// id. Returns `entity_access_denied` for the first offending id — the same
    /// response for "missing" and "wrong org", so it is not an existence oracle.
    async fn validate_ids_in_org(&self, ids: &[Uuid], org_id: Uuid) -> Result<(), ApiError> {
        let unique: Vec<Uuid> = ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<Uuid>>()
            .into_iter()
            .collect();
        if unique.is_empty() {
            return Ok(());
        }
        let filter = StorableFilter::<T>::new_from_entity_ids(&unique);
        let entities = self.get_all(filter).await.map_err(ApiError::from)?;
        let owned: std::collections::HashSet<Uuid> = entities
            .iter()
            .filter(|e| self.get_organization_id(e) == Some(org_id))
            .map(|e| e.id())
            .collect();
        if let Some(missing) = unique.iter().find(|id| !owned.contains(id)) {
            return Err(ApiError::entity_access_denied::<T>(*missing));
        }
        Ok(())
    }

    /// Validate that every id in `ids` refers to a live entity on a network the
    /// caller can access. Network-keyed analogue of [`validate_ids_in_org`], for
    /// entities scoped by `network_id` rather than `organization_id`.
    async fn validate_ids_in_networks(
        &self,
        ids: &[Uuid],
        user_network_ids: &[Uuid],
    ) -> Result<(), ApiError> {
        let unique: Vec<Uuid> = ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<Uuid>>()
            .into_iter()
            .collect();
        if unique.is_empty() {
            return Ok(());
        }
        let filter = StorableFilter::<T>::new_from_entity_ids(&unique);
        let entities = self.get_all(filter).await.map_err(ApiError::from)?;
        let accessible: std::collections::HashSet<Uuid> = entities
            .iter()
            .filter(|e| {
                self.get_network_id(e)
                    .is_some_and(|n| user_network_ids.contains(&n))
            })
            .map(|e| e.id())
            .collect();
        if let Some(missing) = unique.iter().find(|id| !accessible.contains(id)) {
            return Err(ApiError::entity_access_denied::<T>(*missing));
        }
        Ok(())
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

            if let Some(event) = build_entity_event(
                self,
                entity,
                *id,
                EntityOperation::Deleted,
                EntityEventFlags {
                    trigger_stale,
                    suppress_logs,
                    ..Default::default()
                },
                authentication,
            ) {
                self.event_bus().publish(event).await?;
            }

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
        let entity = ready_for_create(entity);

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

        if let Some(event) = build_entity_event(
            self,
            created.clone(),
            created.id(),
            EntityOperation::Created,
            EntityEventFlags {
                trigger_stale,
                suppress_logs,
                ..Default::default()
            },
            authentication,
        ) {
            self.event_bus().publish(event).await?;
        }

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
            if let Some(event) = build_entity_event(
                self,
                entity.clone(),
                entity.id(),
                EntityOperation::Created,
                EntityEventFlags::default(),
                authentication.clone(),
            ) {
                let _ = self.event_bus().publish(event).await;
            }
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

        if let Some(event) = build_entity_event(
            self,
            updated.clone(),
            updated.id(),
            EntityOperation::Updated,
            EntityEventFlags {
                trigger_stale,
                suppress_logs,
                ..Default::default()
            },
            authentication,
        ) {
            self.event_bus().publish(event).await?;
        }

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

                if let Some(event) = build_entity_event(
                    self,
                    entity,
                    *id,
                    EntityOperation::Deleted,
                    EntityEventFlags {
                        trigger_stale,
                        suppress_logs,
                        ..Default::default()
                    },
                    authentication.clone(),
                ) {
                    self.event_bus().publish(event).await?;
                }
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

            if let Some(event) = build_entity_event(
                self,
                entity.clone(),
                entity.id(),
                EntityOperation::Deleted,
                EntityEventFlags {
                    trigger_stale,
                    suppress_logs,
                    ..Default::default()
                },
                authentication.clone(),
            ) {
                self.event_bus().publish(event).await?;
            }
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

            if let Some(scope) = EntityScope::from_ids(
                entity.id(),
                entity.clone().into(),
                entity.network_id(),
                entity.organization_id(),
            ) {
                let event =
                    TypedEvent::new(scope, EntityOperation::Deleted, authentication.clone())
                        .with_flags(EntityEventFlags {
                            trigger_stale,
                            suppress_logs,
                            ..Default::default()
                        });
                self.event_bus().publish(event).await?;
            }
        }

        self.storage().delete_by_filter(filter).await
    }
}

// =============================================================================
// SCD2 service-trait extensions
// =============================================================================

use crate::server::daemons::r#impl::api::ScannedEntityIds;
use crate::server::shared::storage::snapshot::{DiscoveryTracked, Snapshotable};

/// Per-action close-and-clone for service-driven mutations
/// (`TagService::update`, `EntityTagsService` lifecycle, etc.).
///
/// Closes the existing live row by inserting a closed historical copy with a
/// synthetic id and `lineage_id` pointing back to the live row, then advances
/// the live row's `valid_from` to NOW and applies the new field values. The
/// live row's id is stable across mutations — external FKs continue to work.
#[async_trait]
pub trait SnapshotMutator<E: Snapshotable + Display> {
    async fn close_and_clone(&self, new_entity: E) -> Result<E, Error>;
}

#[async_trait]
impl<S, E> SnapshotMutator<E> for S
where
    S: CrudService<E> + Sync,
    E: Entity
        + Into<EntityEnum>
        + Default
        + Display
        + ChangeTriggersTopologyStaleness<E>
        + Snapshotable
        + Send
        + Sync,
{
    async fn close_and_clone(&self, mut new_entity: E) -> Result<E, Error> {
        let mut tx = self.storage().begin_transaction().await?;
        // FOR UPDATE: concurrent close-and-clones of the same entity
        // serialize on the live row, so each change gets exactly one closed
        // historical copy instead of two copies of the same interval.
        let live = tx
            .get_by_id_for_update(&Snapshotable::id_value(&new_entity))
            .await?
            .ok_or_else(|| anyhow!("close_and_clone: live row not found"))?;
        let now = chrono::Utc::now();
        // Closed historical copy: synthetic id, lineage_id pointing at live id.
        let mut closed = live.make_closed_copy(now);
        closed.set_id_value(uuid::Uuid::new_v4());
        tx.create(&closed).await?;
        // Advance live row's valid_from; new field values come from the input.
        new_entity.set_valid_from(now);
        let updated = tx.update(&mut new_entity).await?;
        tx.commit().await?;
        Ok(updated)
    }
}

/// Pull the in-memory `ScannedEntityIds` payload off an
/// `EntityOperation::Created` event for the historical Discovery row, plus
/// the row's id (used as `last_discovery_id` / `first_discovery_id`).
///
/// Returns `None` when the event isn't for a Discovery, isn't a
/// `RunType::Historical`, or carries `scanned: None` (which happens for
/// rows read back from the DB after the strip in
/// `SqlValue::RunType::bind_value`). Per-entity subscribers iterate events
/// and skip on `None`.
pub fn extract_scanned_from_discovery_event(
    event: &crate::server::shared::events::traits::Event<
        crate::server::shared::events::types::EntityOperation,
    >,
) -> Option<(&crate::server::daemons::r#impl::api::ScannedEntityIds, Uuid)> {
    use crate::server::shared::entities::{Entity, EntityDiscriminants};

    if event.scope.entity_discriminant() != EntityDiscriminants::Discovery {
        return None;
    }
    let Entity::Discovery(discovery) = event.scope.entity_type() else {
        return None;
    };
    let results = discovery.base.run_type.historical_results()?;
    let scanned = results.scanned.as_ref()?;
    Some((scanned, event.scope.entity_id()))
}

/// Subscriber-driven post-terminal FK update for `DiscoveryTracked` entities.
/// Called from per-entity-service subscribers on the `EntityOperation::Created`
/// event published for the historical Discovery row (whose scope carries
/// `Entity::Discovery(...)` with `run_type::Historical { results: { scanned, .. } }`)
/// to set `last_discovery_id` (always) and `first_discovery_id` (only when
/// currently NULL).
#[async_trait]
pub trait DiscoveryFkUpdater<E: DiscoveryTracked + Display> {
    async fn update_discovery_fks(
        &self,
        scanned: &ScannedEntityIds,
        discovery_id: Uuid,
    ) -> Result<(), Error>;
}

#[async_trait]
impl<S, E> DiscoveryFkUpdater<E> for S
where
    S: CrudService<E> + Sync,
    E: Entity
        + Into<EntityEnum>
        + Default
        + Display
        + ChangeTriggersTopologyStaleness<E>
        + DiscoveryTracked
        + Send
        + Sync,
{
    async fn update_discovery_fks(
        &self,
        scanned: &ScannedEntityIds,
        discovery_id: Uuid,
    ) -> Result<(), Error> {
        let filter = E::scanned_in_session_filter(scanned);
        let mut entities = self.storage().get_all(filter).await?;
        if entities.is_empty() {
            return Ok(());
        }
        for e in entities.iter_mut() {
            e.set_last_discovery_id(Some(discovery_id));
            if e.first_discovery_id().is_none() {
                e.set_first_discovery_id(Some(discovery_id));
            }
        }
        self.storage().update_many(&entities).await?;
        Ok(())
    }
}
