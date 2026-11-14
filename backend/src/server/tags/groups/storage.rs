use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use strum::IntoEnumIterator;
use uuid::Uuid;

use crate::server::{
    shared::{
        entities::EntityDiscriminants,
        product_area::ProductArea,
        storage::traits::{Entity, SqlValue, Storable},
    },
    tags::groups::base::{TagGroup, TagGroupBase, TagGroupType},
};

impl Storable for TagGroup {
    const HAS_SCD2: bool = false;
    type BaseData = TagGroupBase;

    fn table_name() -> &'static str {
        "tag_groups"
    }

    fn search_predicates() -> &'static [&'static str] {
        &[
            "tag_groups.name ILIKE {}",
            "tag_groups.description ILIKE {}",
        ]
    }

    fn new(base: Self::BaseData) -> Self {
        let now = crate::server::shared::clock::now();
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
                    organization_id,
                    group_type,
                    entity_types,
                    ordered,
                    // Derived from `group_type` on every read; not a column.
                    scope_computed: _,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "organization_id",
                "name",
                "description",
                "group_type",
                "entity_types",
                "ordered",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Uuid(organization_id),
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                SqlValue::String(group_type.to_string()),
                // A computed scope is never stored: it is read from entity metadata on every load.
                SqlValue::OptionalStringArray(
                    entity_types
                        .filter(|_| !group_type.has_computed_scope())
                        .map(|types| types.iter().map(|t| t.to_string()).collect()),
                ),
                SqlValue::Bool(ordered),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let group_type: String = row.get("group_type");
        let group_type: TagGroupType = group_type
            .parse()
            .map_err(|_| anyhow::anyhow!("Unknown tag group type {group_type}"))?;
        Ok(TagGroup {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: TagGroupBase {
                name: row.get("name"),
                description: row.get("description"),
                organization_id: row.get("organization_id"),
                group_type,
                ordered: row.get("ordered"),
                scope_computed: group_type.has_computed_scope(),
                // A computed scope is read from entity metadata, not the row. A type name this build
                // no longer knows is dropped rather than failing the list.
                entity_types: group_type.computed_entity_types().or_else(|| {
                    row.get::<Option<Vec<String>>, _>("entity_types")
                        .map(|names| {
                            names
                                .iter()
                                .filter_map(|name| {
                                    EntityDiscriminants::iter().find(|t| t.to_string() == *name)
                                })
                                .collect()
                        })
                }),
            },
        })
    }
}

impl Entity for TagGroup {
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

    const ENTITY_TYPE: EntityDiscriminants = EntityDiscriminants::TagGroup;

    const ENTITY_NAME_SINGULAR: &'static str = "Tag Group";
    const ENTITY_NAME_PLURAL: &'static str = "Tag Groups";
    const ENTITY_DESCRIPTION: &'static str = "Sets of tags an entity holds one of, in a set order, optionally limited to some entity types.";

    fn product_area() -> ProductArea {
        ProductArea::Platform
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
