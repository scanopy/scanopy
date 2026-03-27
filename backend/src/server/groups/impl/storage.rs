use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    groups::r#impl::{
        base::{Group, GroupBase},
        types::GroupType,
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
        types::entities::EntitySource,
    },
    topology::types::edges::EdgeStyle,
};

/// CSV row representation for Group export (excludes nested binding_ids)
#[derive(Serialize)]
pub struct GroupCsvRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub group_type: String,
    pub color: String,
    pub network_id: Uuid,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Group {
    type BaseData = GroupBase;

    fn table_name() -> &'static str {
        "groups"
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
                    description,
                    group_type,
                    binding_ids: _, // Stored in group_bindings junction table
                    source,
                    color,
                    edge_style,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();

        // GroupType is now stored as TEXT (just the variant name)
        let group_type_str: &'static str = group_type.into();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "description",
                "network_id",
                "source",
                "group_type",
                "color",
                "edge_style",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                SqlValue::Uuid(network_id),
                SqlValue::EntitySource(source),
                SqlValue::String(group_type_str.to_string()),
                SqlValue::String(color.to_string()),
                SqlValue::String(serde_json::to_string(&edge_style)?),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        // GroupType is now stored as TEXT (variant name like "RequestPath" or "HubAndSpoke")
        let group_type_str: String = row.get("group_type");
        let group_type = match group_type_str.as_str() {
            "RequestPath" => GroupType::RequestPath,
            "HubAndSpoke" => GroupType::HubAndSpoke,
            _ => return Err(anyhow::anyhow!("Unknown group_type: {}", group_type_str)),
        };

        let source: EntitySource =
            serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?;

        let edge_style: EdgeStyle = serde_json::from_str(&row.get::<String, _>("edge_style"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize edge_style: {}", e))?;

        Ok(Group {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: GroupBase {
                name: row.get("name"),
                description: row.get("description"),
                network_id: row.get("network_id"),
                source,
                edge_style,
                group_type,
                binding_ids: Vec::new(), // Hydrated by GroupService via GroupBindingStorage
                color: row.get::<String, _>("color").parse().unwrap_or_default(),
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
        })
    }
}

impl Entity for Group {
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

    type CsvRow = GroupCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        let group_type_str: &'static str = self.base.group_type.into();
        GroupCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            description: self.base.description.clone(),
            group_type: group_type_str.to_string(),
            color: self.base.color.to_string(),
            network_id: self.base.network_id,
            source: format!("{:?}", self.base.source),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Group
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Group";
    const ENTITY_NAME_PLURAL: &'static str = "Groups";
    const ENTITY_DESCRIPTION: &'static str = "Logical groupings of hosts. Organize hosts into groups for easier management and visualization.";

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
