use chrono::{DateTime, Utc};
use secrecy::SecretString;
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for DaemonApiKey export (excludes sensitive key field)
#[derive(Serialize)]
pub struct DaemonApiKeyCsvRow {
    pub id: Uuid,
    pub name: String,
    pub network_id: Uuid,
    pub is_enabled: bool,
    pub last_used: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for DaemonApiKey {
    type BaseData = DaemonApiKeyBase;

    fn table_name() -> &'static str {
        "api_keys"
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
        use secrecy::ExposeSecret;

        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    key,
                    name,
                    last_used,
                    expires_at,
                    network_id,
                    is_enabled,
                    tags: _, // Stored in entity_tags junction table
                    plaintext,
                },
        } = self.clone();

        // Extract plaintext secret for storage (only for ServerPoll keys)
        let plaintext_value = plaintext.map(|s| s.expose_secret().to_string());

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "last_used",
                "expires_at",
                "network_id",
                "name",
                "is_enabled",
                "key",
                "plaintext",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::OptionTimestamp(last_used),
                SqlValue::OptionTimestamp(expires_at),
                SqlValue::Uuid(network_id),
                SqlValue::String(name),
                SqlValue::Bool(is_enabled),
                SqlValue::String(key),
                SqlValue::OptionalString(plaintext_value),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        // Wrap plaintext in SecretString for in-memory protection
        let plaintext: Option<SecretString> = row
            .get::<Option<String>, _>("plaintext")
            .map(SecretString::from);

        Ok(DaemonApiKey {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: DaemonApiKeyBase {
                last_used: row.get("last_used"),
                expires_at: row.get("expires_at"),
                name: row.get("name"),
                key: row.get("key"),
                is_enabled: row.get("is_enabled"),
                network_id: row.get("network_id"),
                tags: Vec::new(), // Hydrated from entity_tags junction table
                plaintext,
            },
        })
    }
}

impl Entity for DaemonApiKey {
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

    type CsvRow = DaemonApiKeyCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        DaemonApiKeyCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            network_id: self.base.network_id,
            is_enabled: self.base.is_enabled,
            last_used: self.base.last_used,
            expires_at: self.base.expires_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::DaemonApiKey
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Daemon API Key";
    const ENTITY_NAME_PLURAL: &'static str = "Daemon API Keys";
    const ENTITY_DESCRIPTION: &'static str = "API keys for daemon authentication. Create and manage keys that allow daemons to communicate with the server.";

    fn entity_category() -> EntityCategory {
        EntityCategory::DiscoveryAndDaemons
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

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // key hash cannot be changed via update (use rotate endpoint instead)
        self.base.key = existing.base.key.clone();
        // last_used is server-set only
        self.base.last_used = existing.base.last_used;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
