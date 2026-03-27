//! Group binding junction table and storage.
//!
//! Manages the ordered list of binding IDs for each group.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, postgres::PgRow};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

use crate::server::shared::{
    position::Positioned,
    storage::{
        child::ChildStorableEntity,
        filter::StorableFilter,
        generic::GenericPostgresStorage,
        traits::{SqlValue, Storable, Storage},
    },
};

// =============================================================================
// Group Binding (Junction Table)
// =============================================================================

/// The base data for a GroupBinding junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct GroupBindingBase {
    pub group_id: Uuid,
    pub binding_id: Uuid,
    pub position: i32,
}

impl GroupBindingBase {
    pub fn new(group_id: Uuid, binding_id: Uuid, position: i32) -> Self {
        Self {
            group_id,
            binding_id,
            position,
        }
    }
}

/// A junction record linking a group to a binding with a position
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct GroupBinding {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub base: GroupBindingBase,
}

impl GroupBinding {
    pub fn new(base: GroupBindingBase) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            base,
        }
    }

    pub fn group_id(&self) -> Uuid {
        self.base.group_id
    }

    pub fn binding_id(&self) -> Uuid {
        self.base.binding_id
    }

    pub fn position(&self) -> i32 {
        self.base.position
    }
}

impl Display for GroupBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GroupBinding(group={}, binding={}, pos={})",
            self.base.group_id, self.base.binding_id, self.base.position
        )
    }
}

impl Storable for GroupBinding {
    type BaseData = GroupBindingBase;

    fn table_name() -> &'static str {
        "group_bindings"
    }

    fn new(base: Self::BaseData) -> Self {
        GroupBinding::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["id", "group_id", "binding_id", "position", "created_at"],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.group_id),
                SqlValue::Uuid(self.base.binding_id),
                SqlValue::I32(self.base.position),
                SqlValue::Timestamp(self.created_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(GroupBinding {
            id: row.get("id"),
            created_at: row.get("created_at"),
            base: GroupBindingBase {
                group_id: row.get("group_id"),
                binding_id: row.get("binding_id"),
                position: row.get("position"),
            },
        })
    }
}

impl ChildStorableEntity for GroupBinding {
    fn parent_column() -> &'static str {
        "group_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.group_id
    }
}

impl Positioned for GroupBinding {
    fn position(&self) -> i32 {
        self.base.position
    }

    fn set_position(&mut self, position: i32) {
        self.base.position = position;
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn entity_name() -> &'static str {
        "group binding"
    }
}

// =============================================================================
// Group Binding Storage
// =============================================================================

/// Storage operations for group_bindings junction table.
/// Manages the ordered list of binding IDs for each group.
pub struct GroupBindingStorage {
    storage: GenericPostgresStorage<GroupBinding>,
}

impl GroupBindingStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get all binding IDs for a single group, ordered by position
    pub async fn get_for_group(&self, group_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter = StorableFilter::<GroupBinding>::new_from_group_ids(&[*group_id]);
        let group_bindings = self.storage.get_all_ordered(filter, "position ASC").await?;
        Ok(group_bindings.iter().map(|gb| gb.binding_id()).collect())
    }

    /// Get binding IDs for multiple groups (batch loading)
    pub async fn get_for_groups(&self, group_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if group_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<GroupBinding>::new_from_group_ids(group_ids);
        let group_bindings = self
            .storage
            .get_all_ordered(filter, "group_id ASC, position ASC")
            .await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for gb in group_bindings {
            result
                .entry(gb.group_id())
                .or_default()
                .push(gb.binding_id());
        }

        Ok(result)
    }

    /// Save binding IDs for a group (replaces all existing).
    /// Uses a transaction to ensure atomicity - if any insert fails, the delete is rolled back.
    pub async fn save_for_group(&self, group_id: &Uuid, binding_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;

        // Delete existing bindings for this group
        let filter = StorableFilter::<GroupBinding>::new_from_group_ids(&[*group_id]);
        tx.delete_by_filter(filter).await?;

        // Deduplicate binding_ids while preserving order
        let mut seen = std::collections::HashSet::new();
        let unique_bindings: Vec<&Uuid> =
            binding_ids.iter().filter(|id| seen.insert(*id)).collect();

        // Insert new bindings with position
        for (position, binding_id) in unique_bindings.iter().enumerate() {
            let group_binding = GroupBinding::new(GroupBindingBase::new(
                *group_id,
                **binding_id,
                position as i32,
            ));
            tx.create(&group_binding).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Delete all binding associations for a group
    pub async fn delete_for_group(&self, group_id: &Uuid) -> Result<()> {
        let filter = StorableFilter::<GroupBinding>::new_from_group_ids(&[*group_id]);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }

    /// Remove a specific binding from all groups
    pub async fn remove_binding(&self, binding_id: &Uuid) -> Result<()> {
        let filter = StorableFilter::<GroupBinding>::new_from_binding_id(binding_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }
}
