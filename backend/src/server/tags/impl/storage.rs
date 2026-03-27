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
    tags::r#impl::base::{Tag, TagBase},
};

/// CSV row representation for Tag export
#[derive(Serialize)]
pub struct TagCsvRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub organization_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Tag {
    type BaseData = TagBase;

    fn table_name() -> &'static str {
        "tags"
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
                    description,
                    color,
                    organization_id,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "name",
                "description",
                "color",
                "organization_id",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                SqlValue::String(color.to_string()),
                SqlValue::Uuid(organization_id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(Tag {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: TagBase {
                name: row.get("name"),
                description: row.get("description"),
                organization_id: row.get("organization_id"),
                color: row.get::<String, _>("color").parse().unwrap_or_default(),
            },
        })
    }
}

impl Entity for Tag {
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

    type CsvRow = TagCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        TagCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            description: self.base.description.clone(),
            color: self.base.color.to_string(),
            organization_id: self.base.organization_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Tag
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Tag";
    const ENTITY_NAME_PLURAL: &'static str = "Tags";
    const ENTITY_DESCRIPTION: &'static str =
        "Custom tags for categorization. Apply labels to entities for filtering and organization.";

    fn entity_category() -> EntityCategory {
        EntityCategory::Metadata
    }

    fn network_id(&self) -> Option<Uuid> {
        None
    }

    fn organization_id(&self) -> Option<Uuid> {
        Some(self.base.organization_id)
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }
}
