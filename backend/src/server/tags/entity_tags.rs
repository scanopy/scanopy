//! Entity tag junction table and service.
//!
//! Manages tag assignments to entities across the system.

use anyhow::{Error, Result, anyhow};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, postgres::PgRow};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
    snapshot::{FkMaps, Snapshotable},
    traits::{Entity, SqlValue, Storable, Storage},
};
use crate::server::shared::types::api::ApiError;
use crate::server::tags::service::TagService;

/// Build the `entity_tags` lookup filter for a batch of entities.
///
/// `snapshot_id = None` → live associations (`valid_to IS NULL`).
/// `snapshot_id = Some(id)` → the associations captured under that snapshot.
fn entity_tags_filter(
    entity_ids: &[Uuid],
    entity_type: &EntityDiscriminants,
    snapshot_id: Option<Uuid>,
) -> StorableFilter<EntityTag> {
    let base = StorableFilter::<EntityTag>::new_from_uuids_column("entity_id", entity_ids)
        .entity_type(entity_type);
    match snapshot_id {
        None => base.live(),
        Some(id) => base.snapshot_id(&id),
    }
}

// =============================================================================
// Entity Tag (Junction Table)
// =============================================================================

/// The base data for an EntityTag junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct EntityTagBase {
    pub entity_id: Uuid,
    pub entity_type: EntityDiscriminants,
    pub tag_id: Uuid,
}

impl EntityTagBase {
    pub fn new(entity_id: Uuid, entity_type: EntityDiscriminants, tag_id: Uuid) -> Self {
        Self {
            entity_id,
            entity_type,
            tag_id,
        }
    }
}

/// A junction record linking an entity to a tag
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct EntityTag {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub valid_from: DateTime<Utc>,
    #[serde(default)]
    pub valid_to: Option<DateTime<Utc>>,
    #[serde(default)]
    pub lineage_id: Option<Uuid>,
    pub base: EntityTagBase,
}

impl EntityTag {
    pub fn new(base: EntityTagBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            base,
        }
    }
}

impl Display for EntityTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EntityTag(entity={}, type={}, tag={})",
            self.base.entity_id, self.base.entity_type, self.base.tag_id
        )
    }
}

impl Storable for EntityTag {
    type BaseData = EntityTagBase;

    fn table_name() -> &'static str {
        "entity_tags"
    }

    const HAS_SCD2: bool = true;

    fn new(base: Self::BaseData) -> Self {
        EntityTag::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec![
                "id",
                "entity_id",
                "entity_type",
                "tag_id",
                "created_at",
                "valid_from",
                "valid_to",
                "lineage_id",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.entity_id),
                SqlValue::EntityDiscriminant(self.base.entity_type),
                SqlValue::Uuid(self.base.tag_id),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.valid_from),
                SqlValue::OptionTimestamp(self.valid_to),
                SqlValue::OptionalUuid(self.lineage_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        let entity_type: EntityDiscriminants =
            serde_json::from_str(&row.get::<String, _>("entity_type"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize entity_type: {}", e))?;

        Ok(EntityTag {
            id: row.get("id"),
            created_at: row.get("created_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            base: EntityTagBase {
                entity_id: row.get("entity_id"),
                entity_type,
                tag_id: row.get("tag_id"),
            },
        })
    }
}

impl Snapshotable for EntityTag {
    fn id_value(&self) -> Uuid {
        self.id
    }
    fn set_id_value(&mut self, id: Uuid) {
        self.id = id;
    }
    fn valid_from(&self) -> DateTime<Utc> {
        self.valid_from
    }
    fn valid_to(&self) -> Option<DateTime<Utc>> {
        self.valid_to
    }
    fn lineage_id(&self) -> Option<Uuid> {
        self.lineage_id
    }
    fn set_valid_from(&mut self, t: DateTime<Utc>) {
        self.valid_from = t;
    }
    fn set_valid_to(&mut self, t: Option<DateTime<Utc>>) {
        self.valid_to = t;
    }
    fn set_lineage_id(&mut self, id: Option<Uuid>) {
        self.lineage_id = id;
    }

    fn remap_fks_for_clone(&mut self, maps: &FkMaps) {
        // Only remap entity_id when the row's entity_type is one of the
        // network-scoped entities cloned at network snapshot. Org-scoped
        // entity_types (Daemon, User, DaemonApiKey, UserApiKey, etc.) are
        // filtered out at fetch time by SnapshotService — those rows aren't
        // cloned. tag_id stays pointing at the live tag (tags follow per-
        // action lifecycle, not network-snapshot lifecycle).
        if let Some(closed) = maps.lookup_by_entity_type(self.base.entity_type, self.base.entity_id)
        {
            self.base.entity_id = closed;
        }
    }
}

// =============================================================================
// Entity Tag Storage
// =============================================================================

/// Storage operations for the entity_tags junction table.
/// Manages tag assignments for all taggable entities.
pub struct EntityTagStorage {
    storage: GenericPostgresStorage<EntityTag>,
}

impl EntityTagStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get all tag IDs for a single entity. SCD2: live rows only.
    pub async fn get_for_entity(
        &self,
        entity_id: &Uuid,
        entity_type: &EntityDiscriminants,
    ) -> Result<Vec<Uuid>> {
        let filter = StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", entity_id)
            .entity_type(entity_type)
            .live();
        let records = self.storage.get_all(filter).await?;
        Ok(records.iter().map(|r| r.base.tag_id).collect())
    }

    /// Get tag IDs for multiple entities of the same type (batch loading).
    /// Returns a map of entity_id -> Vec<tag_id>.
    ///
    /// `snapshot_id = None` reads live associations (`valid_to IS NULL`).
    /// `snapshot_id = Some(id)` reads the closed copies captured under that
    /// snapshot — `entity_ids` are then the closed-copy entity ids (matching
    /// the snapshot entities the topology read loads).
    pub async fn get_for_entities(
        &self,
        entity_ids: &[Uuid],
        entity_type: &EntityDiscriminants,
        snapshot_id: Option<Uuid>,
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if entity_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let records = self
            .storage
            .get_all(entity_tags_filter(entity_ids, entity_type, snapshot_id))
            .await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for record in records {
            result
                .entry(record.base.entity_id)
                .or_default()
                .push(record.base.tag_id);
        }

        Ok(result)
    }

    /// Add a tag to an entity
    /// Returns Ok(true) if added, Ok(false) if already existed
    pub async fn add(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<bool> {
        let entity_tag = EntityTag::new(EntityTagBase::new(entity_id, entity_type, tag_id));

        match self.storage.create(&entity_tag).await {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a unique constraint violation (already exists)
                if e.to_string().contains("already exists") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Remove a tag from an entity (soft-close — `valid_to = NOW()` on the
    /// live junction row). Hard delete would break as-of joins for any
    /// snapshot taken while the association was live.
    /// Returns Ok(true) if a live row was closed, Ok(false) if none existed.
    pub async fn remove(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<bool> {
        let filter = StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", &entity_id)
            .entity_type(&entity_type)
            .tag_id(&tag_id)
            .live();

        let mut rows = self.storage.get_all(filter).await?;
        if rows.is_empty() {
            return Ok(false);
        }

        let now = chrono::Utc::now();
        for row in rows.iter_mut() {
            row.valid_to = Some(now);
        }
        self.storage.update_many(&rows).await?;
        Ok(true)
    }

    /// Replace all tags for an entity with a new set. Soft-closes existing
    /// live junction rows that aren't in the new set, INSERTs new live
    /// rows for additions, and leaves rows that are unchanged alone.
    ///
    /// The whole read-diff-write runs inside one transaction holding a
    /// per-parent advisory lock, so concurrent set calls for the same
    /// entity can't interleave into duplicate or merged tag sets. Tag sets
    /// per entity are small (typically < 10), so per-row inside the
    /// transaction is fine — the bulk-in-tx APIs used by SnapshotService
    /// aren't needed here.
    pub async fn set(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_ids: Vec<Uuid>,
    ) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;
        tx.lock(
            LockKey::JunctionSync {
                parent: entity_type,
                parent_id: entity_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        let existing_filter =
            StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", &entity_id)
                .entity_type(&entity_type)
                .live();
        let existing = tx.get_all(existing_filter).await?;

        let new_set: std::collections::HashSet<Uuid> = tag_ids.iter().copied().collect();
        let existing_tag_ids: std::collections::HashSet<Uuid> =
            existing.iter().map(|r| r.base.tag_id).collect();

        let now = chrono::Utc::now();

        // Soft-close rows for tags removed from the set.
        for row in existing
            .iter()
            .filter(|row| !new_set.contains(&row.base.tag_id))
        {
            let mut closed = row.clone();
            closed.valid_to = Some(now);
            tx.update(&mut closed).await?;
        }

        // INSERT live rows for tags newly assigned. Existing live rows whose
        // tag_id is still in the set are left untouched.
        for tid in tag_ids
            .into_iter()
            .filter(|t| !existing_tag_ids.contains(t))
        {
            let new_row = EntityTag::new(EntityTagBase::new(entity_id, entity_type, tid));
            tx.create(&new_row).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Remove all tags for an entity (used when an entity is deleted).
    /// Soft-close the live junction rows so as-of reads can still resolve
    /// the associations that existed before deletion.
    pub async fn remove_all_for_entity(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
    ) -> Result<()> {
        let filter = StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", &entity_id)
            .entity_type(&entity_type)
            .live();

        let mut rows = self.storage.get_all(filter).await?;
        if rows.is_empty() {
            return Ok(());
        }

        let now = chrono::Utc::now();
        for row in rows.iter_mut() {
            row.valid_to = Some(now);
        }
        self.storage.update_many(&rows).await?;
        Ok(())
    }

    /// Bulk insert pre-built EntityTag records. Skips validation — caller must
    /// ensure tags exist. Uses a single INSERT for all records.
    pub async fn create_many(&self, entity_tags: &[EntityTag]) -> Result<Vec<EntityTag>> {
        self.storage.create_many(entity_tags).await
    }

    /// Bulk add a tag to multiple entities.
    /// Silently skips entities that already have the tag.
    pub async fn bulk_add(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<usize> {
        if entity_ids.is_empty() {
            return Ok(0);
        }

        let mut added = 0;
        for entity_id in entity_ids {
            if self.add(*entity_id, entity_type, tag_id).await? {
                added += 1;
            }
        }

        Ok(added)
    }

    /// Bulk remove a tag from multiple entities (soft-close).
    /// Returns the number of live junction rows that were closed.
    pub async fn bulk_remove(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<usize> {
        if entity_ids.is_empty() {
            return Ok(0);
        }

        let filter = StorableFilter::<EntityTag>::new_from_uuids_column("entity_id", entity_ids)
            .entity_type(&entity_type)
            .tag_id(&tag_id)
            .live();

        let mut rows = self.storage.get_all(filter).await?;
        if rows.is_empty() {
            return Ok(0);
        }

        let now = chrono::Utc::now();
        for row in rows.iter_mut() {
            row.valid_to = Some(now);
        }
        let count = rows.len();
        self.storage.update_many(&rows).await?;
        Ok(count)
    }
}

// =============================================================================
// Entity Tag Service
// =============================================================================

/// Service for managing tag assignments to entities.
///
/// Provides:
/// - Tag hydration for entities retrieved from the database
/// - Tag assignment/removal with validation
/// - Bulk operations for efficient multi-entity updates
pub struct EntityTagService {
    storage: Arc<EntityTagStorage>,
    tag_service: Arc<TagService>,
}

impl EntityTagService {
    pub fn new(storage: Arc<EntityTagStorage>, tag_service: Arc<TagService>) -> Self {
        Self {
            storage,
            tag_service,
        }
    }

    // =========================================================================
    // Hydration Methods
    // =========================================================================

    /// Get tags for a single entity.
    pub async fn get_tags(
        &self,
        entity_id: &Uuid,
        entity_type: &EntityDiscriminants,
    ) -> Result<Vec<Uuid>, Error> {
        self.storage.get_for_entity(entity_id, entity_type).await
    }

    /// Hydrate tags for a single entity.
    pub async fn hydrate_tags<T: Entity>(&self, entity: &mut T) -> Result<()> {
        let tags = self
            .storage
            .get_for_entity(&entity.id(), &T::entity_type())
            .await?;
        entity.set_tags(tags);
        Ok(())
    }

    /// Hydrate tags for a batch of entities (single database query).
    ///
    /// This is the preferred method for list endpoints to avoid N+1 queries.
    pub async fn hydrate_tags_batch<T: Entity>(&self, entities: &mut [T]) -> Result<()> {
        if entities.is_empty() {
            return Ok(());
        }

        let ids: Vec<Uuid> = entities.iter().map(|e| e.id()).collect();
        let tags_map = self
            .storage
            .get_for_entities(&ids, &T::entity_type(), None)
            .await?;

        for entity in entities {
            let tags = tags_map.get(&entity.id()).cloned().unwrap_or_default();
            entity.set_tags(tags);
        }

        Ok(())
    }

    /// Get tags for multiple entities as a map (useful when building response types).
    ///
    /// `snapshot_id = None` reads live associations; `Some(id)` reads the
    /// associations captured under that snapshot (entity_ids are closed-copy ids).
    pub async fn get_tags_map(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
        snapshot_id: Option<Uuid>,
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        self.storage
            .get_for_entities(entity_ids, &entity_type, snapshot_id)
            .await
    }

    // =========================================================================
    // Assignment Methods
    // =========================================================================

    /// Add a tag to an entity.
    ///
    /// Validates that:
    /// - The tag exists
    /// - The tag belongs to the specified organization
    pub async fn add_tag(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
        organization_id: Uuid,
    ) -> Result<(), Error> {
        // Validate tag exists and belongs to organization
        let tag = self.validate_tag_full(tag_id, organization_id).await?;

        // Check application group constraint
        if tag.base.is_application {
            self.validate_single_app_tag(entity_id, &entity_type, Some(tag_id))
                .await?;
        }

        // Add to junction table
        self.storage
            .add(entity_id, entity_type, tag_id)
            .await
            .map_err(|e| anyhow!("Failed to add tag: {}", e))?;

        Ok(())
    }

    /// Remove a tag from an entity.
    pub async fn remove_tag(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<(), Error> {
        self.storage
            .remove(entity_id, entity_type, tag_id)
            .await
            .map_err(|e| anyhow!("Failed to remove tag: {}", e))?;

        Ok(())
    }

    /// Replace all tags for an entity.
    ///
    /// Validates all new tags before making changes.
    pub async fn set_tags(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_ids: Vec<Uuid>,
        organization_id: Uuid,
    ) -> Result<(), Error> {
        if tag_ids.is_empty() {
            return Ok(());
        }

        // Validate all tags and check application group constraint
        let mut app_count = 0;
        for tag_id in &tag_ids {
            let tag = self.validate_tag_full(*tag_id, organization_id).await?;
            if tag.base.is_application {
                app_count += 1;
            }
        }
        if app_count > 1 {
            return Err(anyhow!(
                "Only one application tag allowed per {}. Services inherit their host's application unless overridden with their own.",
                entity_type
            ));
        }

        // Replace tags
        self.storage
            .set(entity_id, entity_type, tag_ids)
            .await
            .map_err(|e| anyhow!("Failed to set tags: {}", e))?;

        Ok(())
    }

    /// Remove all tags when an entity is deleted.
    pub async fn remove_all_for_entity(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
    ) -> Result<()> {
        self.storage
            .remove_all_for_entity(entity_id, entity_type)
            .await
    }

    // =========================================================================
    // Bulk Operations
    // =========================================================================

    /// Bulk insert pre-built EntityTag records. Skips validation — caller must
    /// ensure tags exist. Single INSERT for all records.
    pub async fn create_many(&self, entity_tags: &[EntityTag]) -> Result<Vec<EntityTag>> {
        self.storage.create_many(entity_tags).await
    }

    /// Add a tag to multiple entities.
    ///
    /// Validates the tag once, then adds to all entities.
    pub async fn bulk_add_tag(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
        organization_id: Uuid,
    ) -> Result<usize, ApiError> {
        if entity_ids.is_empty() {
            return Ok(0);
        }

        // Validate tag exists and belongs to organization
        self.validate_tag_full(tag_id, organization_id).await?;

        // Bulk add
        let count = self
            .storage
            .bulk_add(entity_ids, entity_type, tag_id)
            .await
            .map_err(|e| ApiError::internal_error(&format!("Failed to bulk add tag: {}", e)))?;

        Ok(count)
    }

    /// Remove a tag from multiple entities.
    pub async fn bulk_remove_tag(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<usize, Error> {
        if entity_ids.is_empty() {
            return Ok(0);
        }

        let count = self
            .storage
            .bulk_remove(entity_ids, entity_type, tag_id)
            .await
            .map_err(|e| anyhow!("Failed to bulk remove tag: {}", e))?;

        Ok(count)
    }

    // =========================================================================
    // Validation Helpers
    // =========================================================================

    /// Validate that a tag exists and belongs to the specified organization.
    /// Returns the full Tag for further checks.
    async fn validate_tag_full(
        &self,
        tag_id: Uuid,
        organization_id: Uuid,
    ) -> Result<super::r#impl::base::Tag, Error> {
        use crate::server::shared::services::traits::CrudService;

        match self.tag_service.get_by_id(&tag_id).await {
            Ok(Some(tag)) => {
                if tag.base.organization_id != organization_id {
                    return Err(anyhow!(
                        "Tag {} does not belong to this organization",
                        tag_id
                    ));
                }
                Ok(tag)
            }
            Ok(None) => Err(anyhow!("Tag {} not found", tag_id)),
            Err(e) => Err(anyhow!("Failed to validate tag {}: {}", tag_id, e)),
        }
    }

    /// Validate that an entity doesn't already have a different application tag.
    /// `exclude_tag_id` is the tag being added (don't count it against the limit).
    async fn validate_single_app_tag(
        &self,
        entity_id: Uuid,
        entity_type: &EntityDiscriminants,
        exclude_tag_id: Option<Uuid>,
    ) -> Result<(), Error> {
        use crate::server::shared::services::traits::CrudService;

        let existing_tag_ids = self.storage.get_for_entity(&entity_id, entity_type).await?;
        for existing_id in &existing_tag_ids {
            if exclude_tag_id == Some(*existing_id) {
                continue;
            }
            if let Ok(Some(existing_tag)) = self.tag_service.get_by_id(existing_id).await
                && existing_tag.base.is_application
            {
                return Err(anyhow!(
                    "Only one application tag allowed per {}. Services inherit their host's application unless overridden with their own.",
                    entity_type
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod entity_tags_filter_tests {
    use super::*;

    #[test]
    fn live_filter_excludes_closed_and_omits_snapshot_id() {
        let ids = [Uuid::new_v4()];
        let where_clause =
            entity_tags_filter(&ids, &EntityDiscriminants::Host, None).to_where_clause();
        assert!(
            where_clause.contains("valid_to IS NULL"),
            "live filter must restrict to live rows: {where_clause}"
        );
        assert!(
            !where_clause.contains("snapshot_id"),
            "live filter must not filter by snapshot_id: {where_clause}"
        );
    }

    #[test]
    fn snapshot_filter_scopes_to_snapshot_id() {
        let ids = [Uuid::new_v4()];
        let snapshot_id = Uuid::new_v4();
        let where_clause = entity_tags_filter(&ids, &EntityDiscriminants::Host, Some(snapshot_id))
            .to_where_clause();
        assert!(
            where_clause.contains("snapshot_id"),
            "snapshot filter must scope by snapshot_id: {where_clause}"
        );
    }
}
