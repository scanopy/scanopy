use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    discovery::r#impl::{
        base::{Discovery, DiscoveryBase},
        types::{DiscoveryType, RunType},
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for Discovery export
#[derive(Serialize)]
pub struct DiscoveryCsvRow {
    pub id: Uuid,
    pub name: String,
    pub discovery_type: String,
    pub run_type: String,
    pub daemon_id: Uuid,
    pub network_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Discovery {
    type BaseData = DiscoveryBase;

    fn table_name() -> &'static str {
        "discovery"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
            scan_count: 0,
            force_full_scan: false,
            pending_credential_ids: vec![],
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
                    discovery_type,
                    run_type,
                    name,
                    daemon_id,
                    network_id,
                    tags: _, // Stored in entity_tags junction table
                },
            scan_count,
            force_full_scan,
            pending_credential_ids,
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "network_id",
                "daemon_id",
                "run_type",
                "discovery_type",
                "scan_count",
                "force_full_scan",
                "pending_credential_ids",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::Uuid(network_id),
                SqlValue::Uuid(daemon_id),
                SqlValue::RunType(run_type),
                SqlValue::DiscoveryType(discovery_type),
                SqlValue::I32(scan_count as i32),
                SqlValue::Bool(force_full_scan),
                SqlValue::UuidArray(pending_credential_ids),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let discovery_type: DiscoveryType =
            serde_json::from_value(row.get::<serde_json::Value, _>("discovery_type"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize discovery_type: {}", e))?;

        let run_type: RunType = serde_json::from_value(row.get::<serde_json::Value, _>("run_type"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize run_type: {}", e))?;

        Ok(Discovery {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: DiscoveryBase {
                daemon_id: row.get("daemon_id"),
                name: row.get("name"),
                network_id: row.get("network_id"),
                run_type,
                discovery_type,
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
            scan_count: row.get::<i32, _>("scan_count") as u32,
            force_full_scan: row.get("force_full_scan"),
            pending_credential_ids: row.get("pending_credential_ids"),
        })
    }
}

impl Entity for Discovery {
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

    type CsvRow = DiscoveryCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        DiscoveryCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            discovery_type: format!("{:?}", self.base.discovery_type),
            run_type: format!("{:?}", self.base.run_type),
            daemon_id: self.base.daemon_id,
            network_id: self.base.network_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Discovery
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Discovery";
    const ENTITY_NAME_PLURAL: &'static str = "Discoveries";
    const ENTITY_DESCRIPTION: &'static str = "Network discovery operations. Trigger and monitor scans that detect hosts, services, and network topology.";

    fn entity_category() -> EntityCategory {
        EntityCategory::DiscoveryAndDaemons
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // scan_count is server-managed — never overwritten by API updates
        self.scan_count = existing.scan_count;
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

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
