use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    services::r#impl::{
        base::{Service, ServiceBase},
        definitions::ServiceDefinition,
        virtualization::ServiceVirtualization,
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            child::ChildStorableEntity,
            traits::{Entity, SqlValue, Storable},
        },
        types::entities::EntitySource,
    },
};

/// CSV row representation for Service export (excludes nested bindings)
#[derive(Serialize)]
pub struct ServiceCsvRow {
    pub id: Uuid,
    pub name: String,
    pub service_definition: String,
    pub host_id: Uuid,
    pub network_id: Uuid,
    pub source: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Service {
    type BaseData = ServiceBase;

    fn table_name() -> &'static str {
        "services"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = chrono::Utc::now();

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
                    name,
                    network_id,
                    host_id,
                    service_definition,
                    virtualization,
                    bindings: _, // Bindings stored in separate table, managed by BindingStorage
                    source,
                    tags: _, // Stored in entity_tags junction table
                    position,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "network_id",
                "host_id",
                "service_definition",
                "virtualization",
                "source",
                "position",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::Uuid(network_id),
                SqlValue::Uuid(host_id),
                SqlValue::ServiceDefinition(service_definition),
                SqlValue::OptionalServiceVirtualization(virtualization),
                SqlValue::EntitySource(source),
                SqlValue::I32(position),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let service_definition: Box<dyn ServiceDefinition> =
            serde_json::from_str(&row.get::<String, _>("service_definition"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize service_definition: {}", e))?;
        let virtualization: Option<ServiceVirtualization> =
            serde_json::from_value(row.get::<serde_json::Value, _>("virtualization"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize virtualization: {}", e))?;
        let source: EntitySource =
            serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?;

        Ok(Service {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: ServiceBase {
                name: row.get("name"),
                network_id: row.get("network_id"),
                host_id: row.get("host_id"),
                service_definition,
                virtualization,
                bindings: Vec::new(), // Bindings loaded separately by ServiceService via BindingStorage
                tags: Vec::new(),     // Hydrated from entity_tags junction table
                source,
                position: row.get("position"),
            },
        })
    }
}

impl Entity for Service {
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

    type CsvRow = ServiceCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        ServiceCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            service_definition: self.base.service_definition.id().to_string(),
            host_id: self.base.host_id,
            network_id: self.base.network_id,
            source: format!("{:?}", self.base.source),
            position: self.base.position,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Service
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Service";
    const ENTITY_NAME_PLURAL: &'static str = "Services";
    const ENTITY_DESCRIPTION: &'static str = "Services running on hosts. Detected or manually added services like databases, web servers, etc.";

    fn entity_category() -> EntityCategory {
        EntityCategory::NetworkInfrastructure
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

    fn set_source(&mut self, source: EntitySource) {
        self.base.source = source;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // source is set at creation time (Manual or Discovery), cannot be changed
        self.base.source = existing.base.source.clone();
        // Preserve virtualization if not explicitly set (discovery-managed field)
        if self.base.virtualization.is_none() {
            self.base.virtualization = existing.base.virtualization.clone();
        }
    }
}

impl ChildStorableEntity for Service {
    fn parent_column() -> &'static str {
        "host_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.host_id
    }
}
