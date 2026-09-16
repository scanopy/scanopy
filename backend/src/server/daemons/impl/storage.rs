use chrono::{DateTime, Utc};
use semver::Version;
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    daemons::r#impl::base::{Daemon, DaemonBase, DaemonMode},
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
    const HAS_SCD2: bool = false;
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
                    standby_cleared_at,
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
                "url",
                "name",
                "mode",
                "version",
                "user_id",
                "api_key_id",
                "is_unreachable",
                "standby",
                "standby_cleared_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::OptionTimestamp(last_seen),
                SqlValue::Uuid(network_id),
                SqlValue::Uuid(host_id),
                SqlValue::String(url),
                SqlValue::String(name),
                SqlValue::DaemonMode(mode),
                SqlValue::OptionalString(version.map(|v| v.to_string())),
                SqlValue::Uuid(user_id),
                SqlValue::OptionalUuid(api_key_id),
                SqlValue::Bool(is_unreachable),
                SqlValue::Bool(standby),
                SqlValue::OptionTimestamp(standby_cleared_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let mode: DaemonMode = serde_json::from_str(&row.get::<String, _>("mode"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize mode: {}", e))?;

        // Parse the stored version. A stored-but-unparseable string is logged
        // (not silently dropped) so a corrupt row is visible rather than quietly
        // read as `None` — which would then be treated as an Unknown/unsupported
        // daemon once the floor advances.
        let version: Option<Version> = match row.get::<Option<String>, _>("version") {
            Some(s) => match Version::parse(&s) {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::warn!(
                        daemon_id = %row.get::<Uuid, _>("id"),
                        stored_version = %s,
                        error = %e,
                        "Stored daemon version is not valid semver; treating as unknown"
                    );
                    None
                }
            },
            None => None,
        };

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
                tags: Vec::new(), // Hydrated from entity_tags junction table
                version,
                user_id: row.get("user_id"),
                api_key_id: row.get("api_key_id"),
                is_unreachable: row.get("is_unreachable"),
                standby: row.get("standby"),
                standby_cleared_at: row.get("standby_cleared_at"),
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
        // `url` is editable for ServerPoll (it is the address the server dials, and a daemon
        // can move); the update handler rejects changing it in DaemonPoll mode, where it is
        // unused. Everything below is genuinely server-owned.
        //
        // network_id is the tenancy boundary — the daemon's key, host, seeded loopback and
        // discovery jobs were all created against it. mode decides polling enrolment and the
        // shape of the daemon's own on-disk config. Both are also overwritten by the daemon's
        // handshake (see daemons/service/processing.rs), so a user edit would silently revert.
        self.base.network_id = existing.base.network_id;
        self.base.mode = existing.base.mode;
        // host_id is the daemon's own Host record, created at provision time.
        self.base.host_id = existing.base.host_id;
        // api_key_id is the 1:1 key binding, maintained by provisioning.
        self.base.api_key_id = existing.base.api_key_id;
        // version is self-reported by the daemon on every handshake.
        self.base.version = existing.base.version.clone();
        // last_seen is server-set only
        self.base.last_seen = existing.base.last_seen;
        // standby is managed by the inactivity background task and the
        // reactivation paths (startup + discovery queue); not user-editable.
        self.base.standby = existing.base.standby;
        // standby_cleared_at is server-managed alongside standby.
        self.base.standby_cleared_at = existing.base.standby_cleared_at;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::daemons::r#impl::base::{DaemonBase, DaemonMode};

    fn daemon(mode: DaemonMode, network_id: Uuid) -> Daemon {
        Daemon::new(DaemonBase {
            host_id: Uuid::new_v4(),
            network_id,
            url: "https://edge.corp:60073".to_string(),
            last_seen: None,
            mode,
            name: "edge-01".to_string(),
            tags: Vec::new(),
            version: Some(semver::Version::new(0, 17, 0)),
            user_id: Uuid::new_v4(),
            api_key_id: Some(Uuid::new_v4()),
            is_unreachable: false,
            standby: false,
            standby_cleared_at: None,
        })
    }

    /// The daemon update endpoint accepts a whole `Daemon` body, so this guard is what stops a
    /// caller moving a daemon to another network (its tenancy boundary), flipping its mode
    /// (which decides polling enrolment), or re-pointing its 1:1 key binding. Editable fields
    /// must still get through.
    #[test]
    fn update_cannot_change_identity_or_server_managed_fields() {
        let network = Uuid::new_v4();
        let existing = daemon(DaemonMode::ServerPoll, network);

        let mut request = existing.clone();
        // Fields a caller may legitimately edit.
        request.base.name = "renamed".to_string();
        request.base.url = "https://moved.corp:60073".to_string();
        request.base.user_id = Uuid::new_v4();
        request.base.tags = vec![Uuid::new_v4()];
        // Fields a caller must not be able to touch.
        request.base.network_id = Uuid::new_v4();
        request.base.mode = DaemonMode::DaemonPoll;
        request.base.host_id = Uuid::new_v4();
        request.base.api_key_id = Some(Uuid::new_v4());
        request.base.version = Some(semver::Version::new(9, 9, 9));
        request.base.standby = true;

        request.preserve_immutable_fields(&existing);

        assert_eq!(request.base.name, "renamed");
        assert_eq!(request.base.url, "https://moved.corp:60073");
        assert_ne!(request.base.user_id, existing.base.user_id);
        assert_eq!(request.base.tags.len(), 1);

        assert_eq!(request.base.network_id, network);
        assert_eq!(request.base.mode, DaemonMode::ServerPoll);
        assert_eq!(request.base.host_id, existing.base.host_id);
        assert_eq!(request.base.api_key_id, existing.base.api_key_id);
        assert_eq!(request.base.version, existing.base.version);
        assert!(!request.base.standby);
    }
}
