//! Dependency member junction table and storage.
//!
//! Manages the ordered list of service members (with optional binding refinement) for each dependency.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, postgres::PgRow};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

use crate::server::bindings::r#impl::base::Binding;

use crate::server::{
    dependencies::r#impl::base::DependencyMembers,
    shared::{
        entities::EntityDiscriminants,
        position::Positioned,
        storage::{
            child::ChildStorableEntity,
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
            traits::{SqlValue, Storable, Storage},
        },
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
    /// `lineage_id` / `snapshot_id`). This went undeclared while `ChildStorage` filtered
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

// =============================================================================
// Dependency Member Storage
// =============================================================================

/// Storage operations for dependency_members junction table.
/// Manages the ordered list of service members for each dependency.
pub struct DependencyMemberStorage {
    storage: GenericPostgresStorage<DependencyMemberRecord>,
    pool: PgPool,
}

impl DependencyMemberStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool.clone()),
            pool,
        }
    }

    /// Get ordered records for a single dependency
    async fn get_records_for_dependency(
        &self,
        dependency_id: &Uuid,
    ) -> Result<Vec<DependencyMemberRecord>> {
        let filter =
            StorableFilter::<DependencyMemberRecord>::new_from_dependency_ids(&[*dependency_id]);
        self.storage.get_all_ordered(filter, "position ASC").await
    }

    /// Hydrate the DependencyMembers enum from junction table records.
    /// The `members` field already has the correct variant (from member_type column);
    /// this fills in the IDs.
    pub async fn hydrate_members(
        &self,
        dependency_id: &Uuid,
        members: &DependencyMembers,
    ) -> Result<DependencyMembers> {
        let records = self.get_records_for_dependency(dependency_id).await?;
        Ok(match members {
            DependencyMembers::Services { .. } => DependencyMembers::Services {
                service_ids: records.iter().map(|r| r.service_id()).collect(),
            },
            DependencyMembers::Bindings { .. } => DependencyMembers::Bindings {
                binding_ids: records.iter().filter_map(|r| r.binding_id()).collect(),
            },
        })
    }

    /// Batch hydrate members for multiple dependencies.
    /// Each dependency's `members` field must already have the correct variant.
    pub async fn hydrate_members_batch(
        &self,
        dependencies: &[(Uuid, &DependencyMembers)],
    ) -> Result<HashMap<Uuid, DependencyMembers>> {
        if dependencies.is_empty() {
            return Ok(HashMap::new());
        }

        let dep_ids: Vec<Uuid> = dependencies.iter().map(|(id, _)| *id).collect();
        let filter = StorableFilter::<DependencyMemberRecord>::new_from_dependency_ids(&dep_ids);
        let records = self
            .storage
            .get_all_ordered(filter, "dependency_id ASC, position ASC")
            .await?;

        // Group records by dependency_id
        let mut records_by_dep: HashMap<Uuid, Vec<&DependencyMemberRecord>> = HashMap::new();
        for r in &records {
            records_by_dep.entry(r.dependency_id()).or_default().push(r);
        }

        // Build variant-correct members for each dependency
        let default_members = DependencyMembers::default();
        let variant_map: HashMap<Uuid, &DependencyMembers> = dependencies.iter().cloned().collect();
        let mut result = HashMap::new();
        for (dep_id, dep_records) in &records_by_dep {
            let variant = variant_map.get(dep_id).copied().unwrap_or(&default_members);
            let members = match variant {
                DependencyMembers::Services { .. } => DependencyMembers::Services {
                    service_ids: dep_records.iter().map(|r| r.service_id()).collect(),
                },
                DependencyMembers::Bindings { .. } => DependencyMembers::Bindings {
                    binding_ids: dep_records.iter().filter_map(|r| r.binding_id()).collect(),
                },
            };
            result.insert(*dep_id, members);
        }

        Ok(result)
    }

    /// Save members for a dependency (replaces all existing).
    /// For Services variant: stores service_ids with binding_id = NULL.
    /// For Bindings variant: looks up service_id for each binding and stores both.
    pub async fn save_for_dependency(
        &self,
        dependency_id: &Uuid,
        members: &DependencyMembers,
    ) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;
        // Serialize concurrent delete-all + re-insert syncs for one
        // dependency (interleaving would duplicate member rows).
        tx.lock(
            LockKey::JunctionSync {
                parent: EntityDiscriminants::Dependency,
                parent_id: *dependency_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        // Delete existing members
        let filter =
            StorableFilter::<DependencyMemberRecord>::new_from_dependency_ids(&[*dependency_id]);
        tx.delete_by_filter(filter).await?;

        match members {
            DependencyMembers::Services { service_ids } => {
                let mut seen = std::collections::HashSet::new();
                let unique: Vec<&Uuid> = service_ids.iter().filter(|id| seen.insert(*id)).collect();
                for (position, service_id) in unique.iter().enumerate() {
                    let record = DependencyMemberRecord::new(DependencyMemberRecordBase::new(
                        *dependency_id,
                        **service_id,
                        None,
                        position as i32,
                    ));
                    tx.create(&record).await?;
                }
            }
            DependencyMembers::Bindings { binding_ids } => {
                // Look up service_id for each binding
                let binding_storage = GenericPostgresStorage::<Binding>::new(self.pool.clone());
                for (position, binding_id) in binding_ids.iter().enumerate() {
                    let filter = StorableFilter::<Binding>::new_from_entity_id(binding_id);
                    let binding = binding_storage
                        .get_unique(filter)
                        .await?
                        .at_most_one()?
                        .ok_or_else(|| anyhow::anyhow!("Binding {} not found", binding_id))?;
                    let record = DependencyMemberRecord::new(DependencyMemberRecordBase::new(
                        *dependency_id,
                        binding.base.service_id,
                        Some(*binding_id),
                        position as i32,
                    ));
                    tx.create(&record).await?;
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Bulk-insert fully-formed member records across many dependencies in a
    /// single INSERT, with no per-dependency lock or delete-first pass. For
    /// seed paths (demo populate) where the org was just reset — there are no
    /// existing rows to replace and no concurrent writers to serialize against,
    /// so the `save_for_dependency` locking is unnecessary overhead. Callers
    /// must supply records with `service_id`/`binding_id`/`position` already
    /// resolved.
    pub async fn create_many(&self, records: &[DependencyMemberRecord]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        self.storage.create_many(records).await?;
        Ok(())
    }

    /// Delete all member associations for a dependency
    pub async fn delete_for_dependency(&self, dependency_id: &Uuid) -> Result<()> {
        let filter =
            StorableFilter::<DependencyMemberRecord>::new_from_dependency_ids(&[*dependency_id]);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }

    /// Remove a specific binding from all dependencies.
    /// Deletes the member row entirely (unlike service-only which would keep it).
    pub async fn remove_binding(&self, binding_id: &Uuid) -> Result<()> {
        let filter = StorableFilter::<DependencyMemberRecord>::new_from_binding_id(binding_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }

    /// Remove a specific service from all dependencies
    pub async fn remove_service(&self, service_id: &Uuid) -> Result<()> {
        let filter = StorableFilter::<DependencyMemberRecord>::new_from_service_id(service_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }
}
