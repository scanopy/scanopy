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
    user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase},
    users::r#impl::permissions::UserOrgPermissions,
};

/// CSV row representation for UserApiKey export (excludes sensitive key field)
#[derive(Serialize)]
pub struct UserApiKeyCsvRow {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub permissions: String,
    pub is_enabled: bool,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for UserApiKey {
    type BaseData = UserApiKeyBase;

    fn table_name() -> &'static str {
        "user_api_keys"
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
                    key,
                    name,
                    user_id,
                    organization_id,
                    permissions,
                    last_used,
                    expires_at,
                    is_enabled,
                    tags: _,        // Stored in entity_tags junction table
                    network_ids: _, // Stored in junction table, not here
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "key",
                "name",
                "user_id",
                "organization_id",
                "permissions",
                "last_used",
                "expires_at",
                "is_enabled",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(key),
                SqlValue::String(name),
                SqlValue::Uuid(user_id),
                SqlValue::Uuid(organization_id),
                SqlValue::String(permissions.to_string()),
                SqlValue::OptionTimestamp(last_used),
                SqlValue::OptionTimestamp(expires_at),
                SqlValue::Bool(is_enabled),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let permissions_str: String = row.get("permissions");
        let permissions = permissions_str
            .parse::<UserOrgPermissions>()
            .unwrap_or_default();

        Ok(UserApiKey {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: UserApiKeyBase {
                key: row.get("key"),
                name: row.get("name"),
                user_id: row.get("user_id"),
                organization_id: row.get("organization_id"),
                permissions,
                last_used: row.get("last_used"),
                expires_at: row.get("expires_at"),
                is_enabled: row.get("is_enabled"),
                tags: Vec::new(),        // Hydrated from entity_tags junction table
                network_ids: Vec::new(), // Hydrated separately from junction table
            },
        })
    }
}

impl Entity for UserApiKey {
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

    type CsvRow = UserApiKeyCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        UserApiKeyCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            user_id: self.base.user_id,
            organization_id: self.base.organization_id,
            permissions: format!("{:?}", self.base.permissions),
            is_enabled: self.base.is_enabled,
            last_used: self.base.last_used,
            expires_at: self.base.expires_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::UserApiKey
    }

    const ENTITY_NAME_SINGULAR: &'static str = "User API Key";
    const ENTITY_NAME_PLURAL: &'static str = "User API Keys";
    const ENTITY_DESCRIPTION: &'static str = "User API keys for programmatic access. Create and manage personal API keys with scoped permissions.";

    fn entity_category() -> EntityCategory {
        EntityCategory::OrganizationsAndUsers
    }

    fn network_id(&self) -> Option<Uuid> {
        // User API keys use a junction table for network access
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

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // key hash cannot be changed via update (use rotate endpoint instead)
        self.base.key = existing.base.key.clone();
        // last_used is server-set only
        self.base.last_used = existing.base.last_used;
        // user_id and organization_id cannot be changed
        self.base.user_id = existing.base.user_id;
        self.base.organization_id = existing.base.organization_id;
        self.created_at = existing.created_at;
        self.id = existing.id;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
