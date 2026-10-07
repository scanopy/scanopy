use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    dependencies::r#impl::{
        base::{Dependency, DependencyBase, DependencyMembers},
        types::DependencyType,
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            snapshot::Snapshotable,
            traits::{Entity, SqlValue, Storable},
        },
        types::entities::EntitySource,
    },
    topology::types::edges::EdgeStyle,
};

/// CSV row representation for Dependency export (excludes nested members)
#[derive(Serialize)]
pub struct DependencyCsvRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub dependency_type: String,
    pub color: String,
    pub site_id: Uuid,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Dependency {
    type BaseData = DependencyBase;

    fn table_name() -> &'static str {
        "dependencies"
    }

    const HAS_SCD2: bool = true;

    fn is_live_row(&self) -> bool {
        self.valid_to.is_none()
    }

    fn new(base: Self::BaseData) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
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
            valid_from,
            valid_to,
            lineage_id,
            base:
                Self::BaseData {
                    name,
                    site_id,
                    description,
                    dependency_type,
                    ref members, // member_type stored on table, IDs in junction table
                    source,
                    color,
                    edge_style,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();

        let dependency_type_str: &'static str = dependency_type.into();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "description",
                "site_id",
                "source",
                "dependency_type",
                "color",
                "edge_style",
                "member_type",
                "valid_from",
                "valid_to",
                "lineage_id",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                SqlValue::Uuid(site_id),
                SqlValue::EntitySource(source),
                SqlValue::String(dependency_type_str.to_string()),
                SqlValue::String(color.to_string()),
                SqlValue::String(serde_json::to_string(&edge_style)?),
                SqlValue::String(
                    if members.is_bindings() {
                        "Bindings"
                    } else {
                        "Services"
                    }
                    .to_string(),
                ),
                SqlValue::Timestamp(valid_from),
                SqlValue::OptionTimestamp(valid_to),
                SqlValue::OptionalUuid(lineage_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let dependency_type_str: String = row.get("dependency_type");
        let dependency_type = match dependency_type_str.as_str() {
            "RequestPath" => DependencyType::RequestPath,
            "HubAndSpoke" => DependencyType::HubAndSpoke,
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown dependency_type: {}",
                    dependency_type_str
                ));
            }
        };

        let source: EntitySource =
            serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?;

        let edge_style: EdgeStyle = serde_json::from_str(&row.get::<String, _>("edge_style"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize edge_style: {}", e))?;

        Ok(Dependency {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            base: DependencyBase {
                name: row.get("name"),
                description: row.get("description"),
                site_id: row.get("site_id"),
                source,
                edge_style,
                dependency_type,
                // member_type discriminant determines which variant; IDs hydrated by DependencyService
                members: match row.get::<String, _>("member_type").as_str() {
                    "Bindings" => DependencyMembers::Bindings {
                        binding_ids: Vec::new(),
                    },
                    _ => DependencyMembers::Services {
                        service_ids: Vec::new(),
                    },
                },
                color: row.get::<String, _>("color").parse().unwrap_or_default(),
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
        })
    }
}

impl Entity for Dependency {
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

    type CsvRow = DependencyCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        let dependency_type_str: &'static str = self.base.dependency_type.into();
        DependencyCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            description: self.base.description.clone(),
            dependency_type: dependency_type_str.to_string(),
            color: self.base.color.to_string(),
            site_id: self.base.site_id,
            source: format!("{:?}", self.base.source),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Dependency
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Dependency";
    const ENTITY_NAME_PLURAL: &'static str = "Dependencies";
    const ENTITY_DESCRIPTION: &'static str =
        "Service dependency relationships. Define how services depend on each other.";

    fn entity_category() -> EntityCategory {
        EntityCategory::Visualization
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

    fn set_source(&mut self, source: EntitySource) {
        self.base.source = source;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        self.base.source = existing.base.source.clone();
        self.created_at = existing.created_at;
        self.updated_at = existing.updated_at;
    }
}

impl Snapshotable for Dependency {
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
    // Dependency is top-level site-scoped — no within-tracked-set FKs to remap.
}
