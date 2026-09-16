//! Subnet-VLAN junction table and storage.
//!
//! Manages the many-to-many relationship between subnets and VLANs.

use std::collections::HashMap;
use std::fmt::Display;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
    snapshot::{FkMaps, Snapshotable},
    traits::{SqlValue, Storable, Storage},
};

/// The base data for a SubnetVlanRecord junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct SubnetVlanRecordBase {
    pub subnet_id: Uuid,
    pub vlan_id: Uuid,
}

impl SubnetVlanRecordBase {
    pub fn new(subnet_id: Uuid, vlan_id: Uuid) -> Self {
        Self { subnet_id, vlan_id }
    }
}

/// A junction record linking a subnet to a VLAN.
///
/// Snapshotable but **not** DiscoveryTracked: the SCD2 columns
/// (`valid_from`/`valid_to`/`lineage_id`) capture when an association
/// existed, which is the audit-trail need for the link itself. Per-link
/// freshness (`last_seen_at`) and per-link discovery FKs are intentionally
/// absent — knowing "when was this VLAN-subnet edge first observed" is
/// sufficient via `valid_from`, and "when did it stop being observed" via
/// `valid_to` (set on soft-close in `unlink`). The two endpoint entities
/// (Subnet, Vlan) carry their own DiscoveryTracked columns; the junction
/// itself doesn't need to repeat them.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct SubnetVlanRecord {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub valid_from: DateTime<Utc>,
    #[serde(default)]
    pub valid_to: Option<DateTime<Utc>>,
    #[serde(default)]
    pub lineage_id: Option<Uuid>,
    pub base: SubnetVlanRecordBase,
}

impl SubnetVlanRecord {
    pub fn new(base: SubnetVlanRecordBase) -> Self {
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

    pub fn subnet_id(&self) -> Uuid {
        self.base.subnet_id
    }

    pub fn vlan_id(&self) -> Uuid {
        self.base.vlan_id
    }
}

impl Display for SubnetVlanRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SubnetVlan(subnet={}, vlan={})",
            self.base.subnet_id, self.base.vlan_id
        )
    }
}

impl Storable for SubnetVlanRecord {
    type BaseData = SubnetVlanRecordBase;

    fn table_name() -> &'static str {
        "subnet_vlans"
    }

    const HAS_SCD2: bool = true;

    fn new(base: Self::BaseData) -> Self {
        SubnetVlanRecord::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec![
                "id",
                "subnet_id",
                "vlan_id",
                "created_at",
                "valid_from",
                "valid_to",
                "lineage_id",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.subnet_id),
                SqlValue::Uuid(self.base.vlan_id),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.valid_from),
                SqlValue::OptionTimestamp(self.valid_to),
                SqlValue::OptionalUuid(self.lineage_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(SubnetVlanRecord {
            id: row.get("id"),
            created_at: row.get("created_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            base: SubnetVlanRecordBase {
                subnet_id: row.get("subnet_id"),
                vlan_id: row.get("vlan_id"),
            },
        })
    }
}

impl Snapshotable for SubnetVlanRecord {
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
        if let Some(closed) = maps.subnets.get(&self.base.subnet_id) {
            self.base.subnet_id = *closed;
        }
        if let Some(closed) = maps.vlans.get(&self.base.vlan_id) {
            self.base.vlan_id = *closed;
        }
    }
}

/// Storage operations for subnet_vlans junction table.
pub struct SubnetVlanStorage {
    storage: GenericPostgresStorage<SubnetVlanRecord>,
}

impl SubnetVlanStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get all VLAN IDs linked to a subnet (live links only).
    pub async fn get_vlan_ids_for_subnet(&self, subnet_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("subnet_id", subnet_id).live();
        let records = self.storage.get_all(filter).await?;
        Ok(records.iter().map(|r| r.vlan_id()).collect())
    }

    /// Get all subnet IDs linked to a VLAN (live links only).
    pub async fn get_subnet_ids_for_vlan(&self, vlan_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("vlan_id", vlan_id).live();
        let records = self.storage.get_all(filter).await?;
        Ok(records.iter().map(|r| r.subnet_id()).collect())
    }

    /// Batch get: VLAN ID → subnet IDs for multiple VLANs (live links only).
    pub async fn get_for_vlans(&self, vlan_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if vlan_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuids_column("vlan_id", vlan_ids).live();
        let records = self.storage.get_all(filter).await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for record in records {
            result
                .entry(record.vlan_id())
                .or_default()
                .push(record.subnet_id());
        }

        Ok(result)
    }

    /// Bulk-insert pre-built subnet↔VLAN links in a single INSERT, with no
    /// per-subnet lock or delete-first pass. For seed paths (demo populate) on a
    /// freshly reset org. Callers must pre-dedup on `(subnet_id, vlan_id)` (the
    /// partial unique index allows at most one live link per pair).
    pub async fn create_many(&self, records: &[SubnetVlanRecord]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        self.storage.create_many(records).await?;
        Ok(())
    }

    /// Link a subnet to a VLAN (idempotent against live rows; the partial
    /// unique index `(subnet_id, vlan_id) WHERE valid_to IS NULL` enforces
    /// at-most-one live link per pair).
    pub async fn link(&self, subnet_id: &Uuid, vlan_id: &Uuid) -> Result<()> {
        let record = SubnetVlanRecord::new(SubnetVlanRecordBase::new(*subnet_id, *vlan_id));
        // Ignore unique constraint violations (idempotent)
        let _ = self.storage.create(&record).await;
        Ok(())
    }

    /// Soft-close the live link between a subnet and a VLAN.
    ///
    /// Sets `valid_to = NOW()` instead of hard-deleting so the historical
    /// "this VLAN was associated with this subnet during [valid_from, valid_to]"
    /// answer remains queryable via SCD2 `as_of` reads. Hard delete would
    /// erase that history.
    pub async fn unlink(&self, subnet_id: &Uuid, vlan_id: &Uuid) -> Result<()> {
        let filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("subnet_id", subnet_id)
                .uuid_column("vlan_id", vlan_id)
                .live();
        let mut rows = self.storage.get_all(filter).await?;
        if rows.is_empty() {
            return Ok(());
        }
        let now = Utc::now();
        for row in rows.iter_mut() {
            row.valid_to = Some(now);
        }
        self.storage.update_many(&rows).await?;
        Ok(())
    }

    /// Replace all VLAN links for a subnet. Soft-closes live links not in
    /// the new set, INSERTs new live links for additions, leaves unchanged
    /// links alone.
    pub async fn save_for_subnet(&self, subnet_id: &Uuid, vlan_ids: &[Uuid]) -> Result<()> {
        // Read-diff-write under a per-subnet advisory lock so concurrent
        // saves can't interleave into duplicate/merged link sets.
        let mut tx = self.storage.begin_transaction().await?;
        tx.lock(
            LockKey::JunctionSync {
                parent: EntityDiscriminants::Subnet,
                parent_id: *subnet_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        let existing_filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("subnet_id", subnet_id).live();
        let existing = tx.get_all(existing_filter).await?;

        let new_set: std::collections::HashSet<Uuid> = vlan_ids.iter().copied().collect();
        let existing_vlan_ids: std::collections::HashSet<Uuid> =
            existing.iter().map(|r| r.vlan_id()).collect();

        let now = Utc::now();

        // Soft-close removed links.
        for row in existing.iter().filter(|r| !new_set.contains(&r.vlan_id())) {
            let mut closed = row.clone();
            closed.valid_to = Some(now);
            tx.update(&mut closed).await?;
        }

        // INSERT new links.
        for vlan_id in vlan_ids.iter().filter(|v| !existing_vlan_ids.contains(v)) {
            let record = SubnetVlanRecord::new(SubnetVlanRecordBase::new(*subnet_id, *vlan_id));
            tx.create(&record).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Soft-close all subnet links for a VLAN (called on VLAN delete).
    /// Preserves historical association data via `valid_to`.
    pub async fn delete_for_vlan(&self, vlan_id: &Uuid) -> Result<()> {
        let filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("vlan_id", vlan_id).live();
        let mut rows = self.storage.get_all(filter).await?;
        if rows.is_empty() {
            return Ok(());
        }
        let now = Utc::now();
        for row in rows.iter_mut() {
            row.valid_to = Some(now);
        }
        self.storage.update_many(&rows).await?;
        Ok(())
    }
}
