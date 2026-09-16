use std::fmt::Display;

use crate::server::credentials::r#impl::types::REDACTED_SECRET_SENTINEL;
use crate::server::shared::{
    entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
    entity_metadata::EntityCategory,
    storage::traits::{Entity, SqlValue, Storable},
};
use crate::server::topology::types::views::TopologyView;
use chrono::{DateTime, Utc};
use secrecy::SecretString;
use serde::{Deserialize, Serialize, Serializer};
use sqlx::Row;
use sqlx::postgres::PgRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Serializer for `Option<SecretString>` that emits the redaction sentinel when set,
/// `null` when unset — never exposing plaintext.
fn redact_optional_password<S: Serializer>(
    value: &Option<SecretString>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        None => serializer.serialize_none(),
        Some(_) => serializer.serialize_some(REDACTED_SECRET_SENTINEL),
    }
}

/// CSV row representation for Share export (excludes password_hash)
#[derive(Serialize)]
pub struct ShareCsvRow {
    pub id: Uuid,
    pub name: String,
    pub topology_id: Uuid,
    pub network_id: Uuid,
    pub created_by: Uuid,
    pub is_enabled: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub has_password: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Share display options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct ShareOptions {
    /// Viewer can open the inspector for a selected element.
    #[schema(required)]
    pub show_inspect_panel: bool,
    /// Viewer sees the zoom controls.
    #[schema(required)]
    pub show_zoom_controls: bool,
    /// Viewer sees the export button.
    #[schema(required)]
    pub show_export_button: bool,
    /// Viewer sees the minimap.
    #[serde(default)]
    #[schema(required)]
    pub show_minimap: bool,
}

impl Default for ShareOptions {
    fn default() -> Self {
        Self {
            show_inspect_panel: true,
            show_zoom_controls: true,
            show_export_button: true,
            show_minimap: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Validate)]
pub struct ShareBase {
    /// The topology this share exposes.
    pub topology_id: Uuid,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// User who created the share.
    pub created_by: Uuid,
    /// Human-facing name for this share.
    pub name: String,
    /// Whether the link still resolves. Disabled shares return 404.
    pub is_enabled: bool,
    /// When this record stops being valid.
    #[schema(required)]
    pub expires_at: Option<DateTime<Utc>>,
    /// Password hash — never sent to client, never accepted from client.
    #[serde(skip)]
    pub password_hash: Option<String>,
    /// Plaintext password on ingest; redacted sentinel (`"********"`) or `None` on egress.
    /// Never stored — `password_hash` is the DB column. Wrapped in `SecretString` so
    /// `Debug`/logging shows `[REDACTED]` during the window between request
    /// deserialization and hashing.
    #[serde(default, serialize_with = "redact_optional_password")]
    #[schema(value_type = Option<String>, format = "password")]
    pub password: Option<SecretString>,
    /// Domains permitted to embed this share. Empty means no restriction.
    #[schema(required)]
    pub allowed_domains: Option<Vec<String>>,
    /// What the viewer can see and do.
    pub options: ShareOptions,
    /// Which topology views are enabled for this share.
    /// None = all views (subject to data availability). Some(list) = only these views in order.
    /// First element is the default view shown on load.
    #[serde(default)]
    #[schema(required)]
    pub enabled_views: Option<Vec<TopologyView>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Validate)]
pub struct Share {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: ShareBase,
}

impl Display for Share {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Share {} ({})", self.id, self.base.name)
    }
}

impl Share {
    /// Check if the share is currently valid (enabled and not expired)
    pub fn is_valid(&self) -> bool {
        if !self.base.is_enabled {
            return false;
        }
        if let Some(expires_at) = self.base.expires_at
            && Utc::now() > expires_at
        {
            return false;
        }
        true
    }

    /// Check if this share requires a password
    pub fn requires_password(&self) -> bool {
        self.base.password_hash.is_some()
    }

    /// Check if this share has domain restrictions configured
    pub fn has_domain_restrictions(&self) -> bool {
        self.base
            .allowed_domains
            .as_ref()
            .is_some_and(|d| !d.is_empty())
    }
}

impl ShareBase {
    /// Normalize `password` so the response carries only `Some(sentinel)` (when a
    /// password is set) or `None` — never the plaintext the client sent. Call this
    /// in every handler that mutates `password_hash` before serializing a response.
    pub fn redact_password(&mut self) {
        self.password = if self.password_hash.is_some() {
            Some(SecretString::new(
                REDACTED_SECRET_SENTINEL.to_string().into(),
            ))
        } else {
            None
        };
    }
}

// Hand-rolled equality/hash because `SecretString` doesn't implement them. Compare
// the stored hash (not the transient `password` field) — two shares with identical
// stored state are equal regardless of what the request-side plaintext buffer holds.
impl PartialEq for ShareBase {
    fn eq(&self, other: &Self) -> bool {
        self.topology_id == other.topology_id
            && self.network_id == other.network_id
            && self.created_by == other.created_by
            && self.name == other.name
            && self.is_enabled == other.is_enabled
            && self.expires_at == other.expires_at
            && self.password_hash == other.password_hash
            && self.allowed_domains == other.allowed_domains
            && self.options == other.options
            && self.enabled_views == other.enabled_views
    }
}
impl Eq for ShareBase {}
impl std::hash::Hash for ShareBase {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.topology_id.hash(state);
        self.network_id.hash(state);
        self.created_by.hash(state);
        self.name.hash(state);
        self.is_enabled.hash(state);
        self.expires_at.hash(state);
        self.password_hash.hash(state);
        self.allowed_domains.hash(state);
        self.options.hash(state);
        self.enabled_views.hash(state);
    }
}

impl PartialEq for Share {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.created_at == other.created_at
            && self.updated_at == other.updated_at
            && self.base == other.base
    }
}
impl Eq for Share {}
impl std::hash::Hash for Share {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.created_at.hash(state);
        self.updated_at.hash(state);
        self.base.hash(state);
    }
}

impl ChangeTriggersTopologyStaleness<Share> for Share {
    fn triggers_staleness(&self, _other: Option<Share>) -> bool {
        false
    }
}

impl Storable for Share {
    const HAS_SCD2: bool = false;
    type BaseData = ShareBase;

    fn table_name() -> &'static str {
        "shares"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = Utc::now();
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
        Ok((
            vec![
                "id",
                "topology_id",
                "network_id",
                "created_by",
                "name",
                "is_enabled",
                "expires_at",
                "password_hash",
                "allowed_domains",
                "options",
                "enabled_views",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.topology_id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::Uuid(self.base.created_by),
                SqlValue::String(self.base.name.clone()),
                SqlValue::Bool(self.base.is_enabled),
                SqlValue::OptionTimestamp(self.base.expires_at),
                SqlValue::OptionalString(self.base.password_hash.clone()),
                SqlValue::OptionalStringArray(self.base.allowed_domains.clone()),
                SqlValue::ShareOptions(self.base.options.clone()),
                SqlValue::EnabledViews(self.base.enabled_views.clone()),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let options_value: serde_json::Value = row.get("options");
        let options: ShareOptions = serde_json::from_value(options_value)?;

        let enabled_views: Option<Vec<TopologyView>> = row
            .try_get::<Option<serde_json::Value>, _>("enabled_views")
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_value(v).ok());

        let password_hash: Option<String> = row.get("password_hash");
        let password = password_hash
            .as_ref()
            .map(|_| SecretString::new(REDACTED_SECRET_SENTINEL.to_string().into()));

        Ok(Share {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: ShareBase {
                topology_id: row.get("topology_id"),
                network_id: row.get("network_id"),
                created_by: row.get("created_by"),
                name: row.get("name"),
                is_enabled: row.get("is_enabled"),
                expires_at: row.get("expires_at"),
                password_hash,
                password,
                allowed_domains: row.get("allowed_domains"),
                options,
                enabled_views,
            },
        })
    }
}

impl Entity for Share {
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

    type CsvRow = ShareCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        ShareCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            topology_id: self.base.topology_id,
            network_id: self.base.network_id,
            created_by: self.base.created_by,
            is_enabled: self.base.is_enabled,
            expires_at: self.base.expires_at,
            has_password: self.requires_password(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Share
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Share";
    const ENTITY_NAME_PLURAL: &'static str = "Shares";
    const ENTITY_DESCRIPTION: &'static str =
        "Shared network views. Create read-only shareable links to your network topology.";

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
}
