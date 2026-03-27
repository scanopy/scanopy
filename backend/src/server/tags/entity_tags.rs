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
    traits::{Entity, SqlValue, Storable, Storage},
};
use crate::server::shared::types::api::ApiError;
use crate::server::tags::service::TagService;

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
    pub base: EntityTagBase,
}

impl EntityTag {
    pub fn new(base: EntityTagBase) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
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

    fn new(base: Self::BaseData) -> Self {
        EntityTag::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["id", "entity_id", "entity_type", "tag_id", "created_at"],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.entity_id),
                SqlValue::EntityDiscriminant(self.base.entity_type),
                SqlValue::Uuid(self.base.tag_id),
                SqlValue::Timestamp(self.created_at),
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
            base: EntityTagBase {
                entity_id: row.get("entity_id"),
                entity_type,
                tag_id: row.get("tag_id"),
            },
        })
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

    /// Get all tag IDs for a single entity
    pub async fn get_for_entity(
        &self,
        entity_id: &Uuid,
        entity_type: &EntityDiscriminants,
    ) -> Result<Vec<Uuid>> {
        let filter = StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", entity_id)
            .entity_type(entity_type);
        let records = self.storage.get_all(filter).await?;
        Ok(records.iter().map(|r| r.base.tag_id).collect())
    }

    /// Get tag IDs for multiple entities of the same type (batch loading)
    /// Returns a map of entity_id -> Vec<tag_id>
    pub async fn get_for_entities(
        &self,
        entity_ids: &[Uuid],
        entity_type: &EntityDiscriminants,
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if entity_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<EntityTag>::new_from_uuids_column("entity_id", entity_ids)
            .entity_type(entity_type);
        let records = self.storage.get_all(filter).await?;

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

    /// Remove a tag from an entity
    /// Returns Ok(true) if removed, Ok(false) if didn't exist
    pub async fn remove(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<bool> {
        let filter = StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", &entity_id)
            .entity_type(&entity_type)
            .tag_id(&tag_id);

        let deleted = self.storage.delete_by_filter(filter).await?;
        Ok(deleted > 0)
    }

    /// Replace all tags for an entity with a new set.
    /// Uses a transaction to ensure atomicity - if any insert fails, the delete is rolled back.
    pub async fn set(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_ids: Vec<Uuid>,
    ) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;

        // Delete existing tags
        let filter = StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", &entity_id)
            .entity_type(&entity_type);
        tx.delete_by_filter(filter).await?;

        // Insert new tags
        for tag_id in tag_ids {
            let entity_tag = EntityTag::new(EntityTagBase::new(entity_id, entity_type, tag_id));
            tx.create(&entity_tag).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Remove all tags for an entity (used when entity is deleted)
    pub async fn remove_all_for_entity(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
    ) -> Result<()> {
        let filter = StorableFilter::<EntityTag>::new_from_uuid_column("entity_id", &entity_id)
            .entity_type(&entity_type);

        self.storage.delete_by_filter(filter).await?;
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

    /// Bulk remove a tag from multiple entities
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
            .tag_id(&tag_id);

        self.storage.delete_by_filter(filter).await
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
            .get_for_entities(&ids, &T::entity_type())
            .await?;

        for entity in entities {
            let tags = tags_map.get(&entity.id()).cloned().unwrap_or_default();
            entity.set_tags(tags);
        }

        Ok(())
    }

    /// Get tags for multiple entities as a map (useful when building response types).
    pub async fn get_tags_map(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        self.storage
            .get_for_entities(entity_ids, &entity_type)
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
        self.validate_tag(tag_id, organization_id).await?;

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

        // Validate all tags
        for tag_id in &tag_ids {
            self.validate_tag(*tag_id, organization_id).await?;
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
        self.validate_tag(tag_id, organization_id).await?;

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
    async fn validate_tag(&self, tag_id: Uuid, organization_id: Uuid) -> Result<(), Error> {
        use crate::server::shared::services::traits::CrudService;

        match self.tag_service.get_by_id(&tag_id).await {
            Ok(Some(tag)) => {
                if tag.base.organization_id != organization_id {
                    return Err(anyhow!(
                        "Tag {} does not belong to this organization",
                        tag_id
                    ));
                }
                Ok(())
            }
            Ok(None) => Err(anyhow!("Tag {} not found", tag_id)),
            Err(e) => Err(anyhow!("Failed to validate tag {}: {}", tag_id, e)),
        }
    }
}
