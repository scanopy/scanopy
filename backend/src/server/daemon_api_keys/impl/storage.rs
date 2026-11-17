use chrono::{DateTime, Utc};
use secrecy::SecretString;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase},
    shared::{
        entities::EntityDiscriminants,
        product_area::ProductArea,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

impl Storable for DaemonApiKey {
    const HAS_SCD2: bool = false;
    type BaseData = DaemonApiKeyBase;

    fn table_name() -> &'static str {
        "api_keys"
    }

    fn search_predicates() -> &'static [&'static str] {
        &["api_keys.name ILIKE {}"]
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
                    site_id,
                    is_enabled,
                    tags: _, // Stored in entity_tags junction table
                    daemon_id,
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
                "site_id",
                "name",
                "is_enabled",
                "key",
                "daemon_id",
                "plaintext",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::OptionTimestamp(last_used),
                SqlValue::OptionTimestamp(expires_at),
                SqlValue::Uuid(site_id),
                SqlValue::String(name),
                SqlValue::Bool(is_enabled),
                SqlValue::String(key),
                SqlValue::OptionalUuid(daemon_id),
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
                site_id: row.get("site_id"),
                tags: Vec::new(), // Hydrated from entity_tags junction table
                daemon_id: row.get("daemon_id"),
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

    const ENTITY_TYPE: EntityDiscriminants = EntityDiscriminants::DaemonApiKey;

    const ENTITY_NAME_SINGULAR: &'static str = "Daemon API Key";
    const ENTITY_NAME_PLURAL: &'static str = "Daemon API Keys";
    const ENTITY_DESCRIPTION: &'static str = "API keys for daemon authentication. Create and manage keys that allow daemons to communicate with the server.";

    fn product_area() -> ProductArea {
        ProductArea::Discovery
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    fn key(name: &str, plaintext: Option<&str>) -> DaemonApiKey {
        DaemonApiKey {
            base: DaemonApiKeyBase {
                name: name.to_string(),
                plaintext: plaintext.map(|s| SecretString::from(s.to_string())),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// A ServerPoll key stores its plaintext so the server can dial the daemon. Update
    /// requests omit it (`#[serde(skip)]` => None), so preserve_immutable_fields must
    /// restore it from the stored record — otherwise a rename/enable-disable/Manage-key PUT
    /// would persist NULL and break ServerPoll polling ("has no stored plaintext").
    #[test]
    fn update_preserves_stored_plaintext_when_request_omits_it() {
        let existing = key("old-name", Some("scp_d_secret"));
        let mut incoming = key("new-name", None); // rename; serde dropped the plaintext
        incoming.preserve_immutable_fields(&existing);
        assert_eq!(incoming.base.name, "new-name"); // mutable field still applies
        assert_eq!(
            incoming
                .base
                .plaintext
                .as_ref()
                .expect("plaintext preserved across update")
                .expose_secret(),
            "scp_d_secret"
        );
    }

    /// An explicit new plaintext (as rotate sets before persisting) is not clobbered by the
    /// old stored value.
    #[test]
    fn update_keeps_explicit_new_plaintext() {
        let existing = key("k", Some("old-secret"));
        let mut incoming = key("k", Some("new-secret"));
        incoming.preserve_immutable_fields(&existing);
        assert_eq!(
            incoming.base.plaintext.as_ref().unwrap().expose_secret(),
            "new-secret"
        );
    }
}
