use chrono::{DateTime, Utc};
use semver::Version;
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    daemons::r#impl::{
        api::DaemonCapabilities,
        base::{Daemon, DaemonBase, DaemonMode},
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for Daemon export (excludes sensitive url field)
#[derive(Serialize)]
pub struct DaemonCsvRow {
    pub id: Uuid,
    pub name: String,
    pub mode: String,
    pub version: Option<String>,
    pub host_id: Uuid,
    pub network_id: Uuid,
    pub user_id: Uuid,
    pub last_seen: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Daemon {
    type BaseData = DaemonBase;

    fn table_name() -> &'static str {
        "daemons"
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
                    network_id,
                    host_id,
                    capabilities,
                    last_seen,
                    mode,
                    url,
                    name,
                    tags: _, // Stored in entity_tags junction table
                    version,
                    user_id,
                    api_key_id,
                    is_unreachable,
                    standby,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "last_seen",
                "network_id",
                "host_id",
                "capabilities",
                "url",
                "name",
                "mode",
                "version",
                "user_id",
                "api_key_id",
                "is_unreachable",
                "standby",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::OptionTimestamp(last_seen),
                SqlValue::Uuid(network_id),
                SqlValue::Uuid(host_id),
                SqlValue::DaemonCapabilities(capabilities),
                SqlValue::String(url),
                SqlValue::String(name),
                SqlValue::DaemonMode(mode),
                SqlValue::OptionalString(version.map(|v| v.to_string())),
                SqlValue::Uuid(user_id),
                SqlValue::OptionalUuid(api_key_id),
                SqlValue::Bool(is_unreachable),
                SqlValue::Bool(standby),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let mode: DaemonMode = serde_json::from_str(&row.get::<String, _>("mode"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize mode: {}", e))?;

        let capabilities: DaemonCapabilities =
            serde_json::from_value(row.get::<serde_json::Value, _>("capabilities"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize capabilities: {}", e))?;

        // Parse version from string, ignoring parse errors (version may be invalid)
        let version: Option<Version> = row
            .get::<Option<String>, _>("version")
            .and_then(|s| Version::parse(&s).ok());

        Ok(Daemon {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: DaemonBase {
                url: row.get("url"),
                last_seen: row.get("last_seen"),
                host_id: row.get("host_id"),
                network_id: row.get("network_id"),
                name: row.get("name"),
                mode,
                capabilities,
                tags: Vec::new(), // Hydrated from entity_tags junction table
                version,
                user_id: row.get("user_id"),
                api_key_id: row.get("api_key_id"),
                is_unreachable: row.get("is_unreachable"),
                standby: row.get("standby"),
            },
        })
    }
}

impl Entity for Daemon {
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

    type CsvRow = DaemonCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        DaemonCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            mode: format!("{:?}", self.base.mode),
            version: self.base.version.as_ref().map(|v| v.to_string()),
            host_id: self.base.host_id,
            network_id: self.base.network_id,
            user_id: self.base.user_id,
            last_seen: self.base.last_seen,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Daemon
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Daemon";
    const ENTITY_NAME_PLURAL: &'static str = "Daemons";
    const ENTITY_DESCRIPTION: &'static str =
        "Daemons are scanning agents that connect to the server to perform network discovery.";

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

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // url is set at registration time, cannot be changed via update
        self.base.url = existing.base.url.clone();
        // last_seen is server-set only
        self.base.last_seen = existing.base.last_seen;
        // capabilities are reported by the daemon, not user-editable
        self.base.capabilities = existing.base.capabilities.clone();
        // standby is managed by billing plan restrictions, not user-editable
        self.base.standby = existing.base.standby;
    }
}
