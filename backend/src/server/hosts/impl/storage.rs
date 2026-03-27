use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    hosts::r#impl::{
        base::{Host, HostBase},
        virtualization::HostVirtualization,
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
        types::entities::EntitySource,
    },
};

/// CSV row representation for Host export
#[derive(Serialize)]
pub struct HostCsvRow {
    pub id: Uuid,
    pub name: String,
    pub hostname: Option<String>,
    pub description: Option<String>,
    pub network_id: Uuid,
    pub source: String,
    pub hidden: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Host {
    type BaseData = HostBase;

    fn table_name() -> &'static str {
        "hosts"
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
        // Exhaustive destructuring ensures compile error if HostBase changes
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    name,
                    description,
                    hostname,
                    network_id,
                    hidden,
                    source,
                    virtualization,
                    tags: _, // Stored in entity_tags junction table
                    sys_descr,
                    sys_object_id,
                    sys_location,
                    sys_contact,
                    management_url,
                    chassis_id,
                    sys_name,
                    manufacturer,
                    model,
                    serial_number,
                    credential_assignments: _, // Stored in host_credentials junction table
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "description",
                "network_id",
                "source",
                "hostname",
                "hidden",
                "virtualization",
                "sys_descr",
                "sys_object_id",
                "sys_location",
                "sys_contact",
                "management_url",
                "chassis_id",
                "sys_name",
                "manufacturer",
                "model",
                "serial_number",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                SqlValue::Uuid(network_id),
                SqlValue::EntitySource(source),
                SqlValue::OptionalString(hostname),
                SqlValue::Bool(hidden),
                SqlValue::OptionalHostVirtualization(virtualization),
                SqlValue::OptionalString(sys_descr),
                SqlValue::OptionalString(sys_object_id),
                SqlValue::OptionalString(sys_location),
                SqlValue::OptionalString(sys_contact),
                SqlValue::OptionalString(management_url),
                SqlValue::OptionalString(chassis_id),
                SqlValue::OptionalString(sys_name),
                SqlValue::OptionalString(manufacturer),
                SqlValue::OptionalString(model),
                SqlValue::OptionalString(serial_number),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        // Parse JSON fields safely
        let source: EntitySource =
            serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?;
        let virtualization: Option<HostVirtualization> =
            serde_json::from_value(row.get::<serde_json::Value, _>("virtualization"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize virtualization: {}", e))?;

        Ok(Host {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: HostBase {
                name: row.get("name"),
                description: row.get("description"),
                network_id: row.get("network_id"),
                source,
                hostname: row.get("hostname"),
                hidden: row.get("hidden"),
                virtualization,
                tags: Vec::new(), // Hydrated from entity_tags junction table
                sys_descr: row.get("sys_descr"),
                sys_object_id: row.get("sys_object_id"),
                sys_location: row.get("sys_location"),
                sys_contact: row.get("sys_contact"),
                management_url: row.get("management_url"),
                chassis_id: row.get("chassis_id"),
                sys_name: row.get("sys_name"),
                manufacturer: row.get("manufacturer"),
                model: row.get("model"),
                serial_number: row.get("serial_number"),
                credential_assignments: Vec::new(), // Hydrated from host_credentials junction table
            },
        })
    }
}

impl Entity for Host {
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

    type CsvRow = HostCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        HostCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            hostname: self.base.hostname.clone(),
            description: self.base.description.clone(),
            network_id: self.base.network_id,
            source: format!("{:?}", self.base.source),
            hidden: self.base.hidden,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Host
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Host";
    const ENTITY_NAME_PLURAL: &'static str = "Hosts";
    const ENTITY_DESCRIPTION: &'static str =
        "Network hosts (devices). Manage discovered or manually created hosts on your network.";

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
        self.created_at = existing.created_at;
        self.updated_at = existing.updated_at;
    }
}
