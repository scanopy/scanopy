use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            snapshot::Snapshotable,
            traits::{Entity, SqlValue, Storable},
        },
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
    pub is_application: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Tag {
    type BaseData = TagBase;

    fn table_name() -> &'static str {
        "tags"
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
                    description,
                    color,
                    organization_id,
                    is_application,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "name",
                "description",
                "color",
                "organization_id",
                "is_application",
                "created_at",
                "updated_at",
                "valid_from",
                "valid_to",
                "lineage_id",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                SqlValue::String(color.to_string()),
                SqlValue::Uuid(organization_id),
                SqlValue::Bool(is_application),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::Timestamp(valid_from),
                SqlValue::OptionTimestamp(valid_to),
                SqlValue::OptionalUuid(lineage_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(Tag {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            base: TagBase {
                name: row.get("name"),
                description: row.get("description"),
                organization_id: row.get("organization_id"),
                color: row.get::<String, _>("color").parse().unwrap_or_default(),
                is_application: row.get("is_application"),
            },
        })
    }
}

impl Snapshotable for Tag {
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
    // Tag is org-scoped — no within-tracked-set FKs to remap.
    // Lifecycle: per-action close-and-clone on rename via TagService::update.
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
            is_application: self.base.is_application,
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

    fn site_id(&self) -> Option<Uuid> {
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
