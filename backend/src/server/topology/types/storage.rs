use crate::server::shared::entities::EntityDiscriminants;
use crate::server::{
    shared::{
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
    topology::types::base::{Topology, TopologyBase, TopologyOptions},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

/// CSV row representation for Topology export (slimmed: `options` lives in JSONB;
/// the per-view graph is built on request and never persisted).
#[derive(Serialize)]
pub struct TopologyCsvRow {
    pub id: Uuid,
    pub network_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Topology {
    const HAS_SCD2: bool = false;
    type BaseData = TopologyBase;

    fn table_name() -> &'static str {
        "topologies"
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
                    options,
                },
        } = self.clone();

        Ok((
            vec!["id", "created_at", "updated_at", "network_id", "options"],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::Uuid(network_id),
                SqlValue::TopologyOptions(options),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let options: TopologyOptions =
            serde_json::from_value(row.get::<serde_json::Value, _>("options"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize options: {}", e))?;

        Ok(Topology {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: TopologyBase {
                network_id: row.get("network_id"),
                options,
            },
        })
    }
}

impl Entity for Topology {
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

    type CsvRow = TopologyCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        TopologyCsvRow {
            id: self.id,
            network_id: self.base.network_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Topology
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Topology";
    const ENTITY_NAME_PLURAL: &'static str = "Topologies";
    const ENTITY_DESCRIPTION: &'static str =
        "Network topology maps showing host relationships and connections.";

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

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        self.id = existing.id;
        self.created_at = existing.created_at;
        self.updated_at = existing.updated_at;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        None
    }

    fn set_tags(&mut self, _tags: Vec<Uuid>) {
        // Topology no longer carries tags directly; entity-specific tags live
        // on hosts/services/subnets and surface via TopologyData.
    }
}
