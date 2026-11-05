//! Dependency member junction table.
//!
//! Manages the ordered list of service members (with optional binding refinement) for each dependency.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, postgres::PgRow};
use std::fmt::Display;
use uuid::Uuid;

use crate::server::shared::{
    position::Positioned,
    storage::{
        child::ChildStorableEntity,
        traits::{SqlValue, Storable},
    },
};

// =============================================================================
// Dependency Member (Junction Table)
// =============================================================================

/// The base data for a DependencyMemberRecord junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct DependencyMemberRecordBase {
    pub dependency_id: Uuid,
    pub service_id: Uuid,
    pub binding_id: Option<Uuid>,
    pub position: i32,
}

impl DependencyMemberRecordBase {
    pub fn new(
        dependency_id: Uuid,
        service_id: Uuid,
        binding_id: Option<Uuid>,
        position: i32,
    ) -> Self {
        Self {
            dependency_id,
            service_id,
            binding_id,
            position,
        }
    }
}

/// A junction record linking a dependency to a service member with a position
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct DependencyMemberRecord {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub valid_from: DateTime<Utc>,
    #[serde(default)]
    pub valid_to: Option<DateTime<Utc>>,
    #[serde(default)]
    pub lineage_id: Option<Uuid>,
    pub base: DependencyMemberRecordBase,
}

impl DependencyMemberRecord {
    pub fn new(base: DependencyMemberRecordBase) -> Self {
        let now = crate::server::shared::clock::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            base,
        }
    }

    pub fn dependency_id(&self) -> Uuid {
        self.base.dependency_id
    }

    pub fn service_id(&self) -> Uuid {
        self.base.service_id
    }

    pub fn binding_id(&self) -> Option<Uuid> {
        self.base.binding_id
    }
}

impl Display for DependencyMemberRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DependencyMember(dep={}, service={}, binding={:?}, pos={})",
            self.base.dependency_id, self.base.service_id, self.base.binding_id, self.base.position
        )
    }
}

impl Storable for DependencyMemberRecord {
    type BaseData = DependencyMemberRecordBase;

    fn table_name() -> &'static str {
        "dependency_members"
    }

    /// `dependency_members` carries the full SCD2 set (`valid_from` / `valid_to` /
    /// `lineage_id` / `snapshot_id`). This went undeclared while `GenericChildStorage` filtered
    /// every child to live rows unconditionally; now that the filter consults this, the
    /// declaration is load-bearing.
    const HAS_SCD2: bool = true;

    fn new(base: Self::BaseData) -> Self {
        DependencyMemberRecord::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec![
                "id",
                "dependency_id",
                "service_id",
                "binding_id",
                "position",
                "created_at",
                "valid_from",
                "valid_to",
                "lineage_id",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.dependency_id),
                SqlValue::Uuid(self.base.service_id),
                SqlValue::OptionalUuid(self.base.binding_id),
                SqlValue::I32(self.base.position),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.valid_from),
                SqlValue::OptionTimestamp(self.valid_to),
                SqlValue::OptionalUuid(self.lineage_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(DependencyMemberRecord {
            id: row.get("id"),
            created_at: row.get("created_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            base: DependencyMemberRecordBase {
                dependency_id: row.get("dependency_id"),
                service_id: row.get("service_id"),
                binding_id: row.get("binding_id"),
                position: row.get("position"),
            },
        })
    }
}

impl crate::server::shared::storage::snapshot::Snapshotable for DependencyMemberRecord {
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

    fn remap_fks_for_clone(&mut self, maps: &crate::server::shared::storage::snapshot::FkMaps) {
        if let Some(closed) = maps.dependencies.get(&self.base.dependency_id) {
            self.base.dependency_id = *closed;
        }
        if let Some(closed) = maps.services.get(&self.base.service_id) {
            self.base.service_id = *closed;
        }
        // Bindings are snapshotted earlier in CLONE_ORDER, so the closed-id
        // is in the map. Without this remap, an as-of-T join through the
        // closed dep_member would land on the live binding row whose state
        // has moved past T.
        if let Some(binding_id) = self.base.binding_id
            && let Some(closed) = maps.bindings.get(&binding_id)
        {
            self.base.binding_id = Some(*closed);
        }
    }
}

impl ChildStorableEntity for DependencyMemberRecord {
    fn parent_column() -> &'static str {
        "dependency_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.dependency_id
    }
}

impl Positioned for DependencyMemberRecord {
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
        "dependency member"
    }
}
