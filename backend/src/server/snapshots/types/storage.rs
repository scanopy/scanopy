use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
    snapshots::types::base::{Snapshot, SnapshotBase},
};

#[derive(Serialize)]
pub struct SnapshotCsvRow {
    pub id: Uuid,
    pub network_id: Uuid,
    pub taken_at: DateTime<Utc>,
    pub created_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Snapshot {
    const HAS_SCD2: bool = false;
    type BaseData = SnapshotBase;

    fn table_name() -> &'static str {
        "snapshots"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    network_id,
                    taken_at,
                    created_by_user_id,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "network_id",
                "taken_at",
                "created_by_user_id",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Uuid(network_id),
                SqlValue::Timestamp(taken_at),
                SqlValue::OptionalUuid(created_by_user_id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(Snapshot {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: SnapshotBase {
                network_id: row.get("network_id"),
                taken_at: row.get("taken_at"),
                created_by_user_id: row.get("created_by_user_id"),
            },
        })
    }
}

impl Entity for Snapshot {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    type CsvRow = SnapshotCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        SnapshotCsvRow {
            id: self.id,
            network_id: self.base.network_id,
            taken_at: self.base.taken_at,
            created_by_user_id: self.base.created_by_user_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Snapshot
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Snapshot";
    const ENTITY_NAME_PLURAL: &'static str = "Snapshots";
    const ENTITY_DESCRIPTION: &'static str = "Point-in-time capture of a network's topology and entities. Created manually via the topology tab; loadable from the snapshots dropdown.";

    fn entity_category() -> EntityCategory {
        EntityCategory::Visualization
    }

    fn network_id(&self) -> Option<Uuid> {
        Some(self.base.network_id)
    }

    fn organization_id(&self) -> Option<Uuid> {
        None
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }
}
