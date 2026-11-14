//! Subnet-VLAN junction table.
//!
//! Manages the many-to-many relationship between subnets and VLANs.

use std::fmt::Display;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::shared::storage::{
    snapshot::{FkMaps, Snapshotable},
    traits::{SqlValue, Storable},
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

    fn relations() -> &'static [&'static crate::server::shared::storage::relation::Relation<Self>] {
        use crate::server::shared::relations as R;
        &[&R::SUBNET_VLAN_SUBNET, &R::SUBNET_VLAN_VLAN]
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
