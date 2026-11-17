use chrono::{DateTime, Utc};
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
        product_area::ProductArea,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

impl Storable for Discovery {
    const HAS_SCD2: bool = false;
    type BaseData = DiscoveryBase;

    fn table_name() -> &'static str {
        "discovery"
    }

    /// A run is looked up by its own name or by the daemon that produced it —
    /// "what did that box do last night" is the usual question of a history
    /// list. Discovery rows carry no description to match on.
    fn search_predicates() -> &'static [&'static str] {
        &[
            "discovery.name ILIKE {}",
            "EXISTS (SELECT 1 FROM daemons d WHERE d.id = discovery.daemon_id \
             AND d.name ILIKE {})",
        ]
    }

    fn new(base: Self::BaseData) -> Self {
        let now = crate::server::shared::clock::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
            scan_count: 0,
            force_full_scan: false,
            next_run_is_full_scan: false,
            integration_targets: vec![],
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
                    site_id,
                    tags: _, // Stored in entity_tags junction table
                },
            scan_count,
            force_full_scan,
            next_run_is_full_scan: _, // Computed on read, never stored
            integration_targets,
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "site_id",
                "daemon_id",
                "run_type",
                "discovery_type",
                "scan_count",
                "force_full_scan",
                "integration_targets",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::Uuid(site_id),
                SqlValue::Uuid(daemon_id),
                SqlValue::RunType(run_type),
                SqlValue::DiscoveryType(discovery_type),
                SqlValue::I32(scan_count as i32),
                SqlValue::Bool(force_full_scan),
                SqlValue::IntegrationTargets(integration_targets),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let discovery_type: DiscoveryType =
            serde_json::from_value(row.get::<serde_json::Value, _>("discovery_type"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize discovery_type: {}", e))?;

        let mut run_type: RunType =
            serde_json::from_value(row.get::<serde_json::Value, _>("run_type"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize run_type: {}", e))?;
        // Runs stored before `discovery_warnings` still carry their warnings in the JSON, and
        // those were copied into the table. The table is the one source: the service attaches its
        // rows, so the JSON copy is dropped here rather than shown alongside them.
        if let RunType::Historical { results } = &mut run_type {
            results.warnings.clear();
        }

        let integration_targets =
            serde_json::from_value(row.get::<serde_json::Value, _>("integration_targets"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize integration_targets: {}", e))?;

        let mut discovery = Discovery {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: DiscoveryBase {
                daemon_id: row.get("daemon_id"),
                name: row.get("name"),
                site_id: row.get("site_id"),
                run_type,
                discovery_type,
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
            scan_count: row.get::<i32, _>("scan_count") as u32,
            force_full_scan: row.get("force_full_scan"),
            next_run_is_full_scan: false,
            integration_targets,
        };
        discovery.next_run_is_full_scan = discovery.full_scan_is_due();
        Ok(discovery)
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

    const ENTITY_TYPE: EntityDiscriminants = EntityDiscriminants::Discovery;

    const ENTITY_NAME_SINGULAR: &'static str = "Discovery";
    const ENTITY_NAME_PLURAL: &'static str = "Discoveries";
    const ENTITY_DESCRIPTION: &'static str = "Network discovery operations. Trigger and monitor scans that detect hosts, services, and network topology.";

    fn product_area() -> ProductArea {
        ProductArea::Discovery
    }

    fn site_id(&self) -> Option<Uuid> {
        Some(self.base.site_id)
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
