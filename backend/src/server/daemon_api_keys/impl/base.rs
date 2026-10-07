use std::fmt::Display;

use crate::server::shared::api_key_common::{ApiKeyCommon, ApiKeyType};
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::types::api::serialize_sensitive_info;
use chrono::{DateTime, Utc};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, Validate)]
pub struct DaemonApiKeyBase {
    /// The stored key. Returned redacted except on creation and rotation.
    #[serde(default)]
    #[serde(serialize_with = "serialize_sensitive_info")]
    #[schema(read_only, required)]
    pub key: String,
    /// Human-facing name for this key.
    pub name: String,
    /// When a daemon last authenticated with this key.
    #[serde(default)]
    #[schema(read_only, required)]
    pub last_used: Option<DateTime<Utc>>,
    /// When this record stops being valid.
    pub expires_at: Option<DateTime<Utc>>,
    /// The site this entity belongs to.
    pub site_id: Uuid,
    /// Whether the key may still be used. Disabled keys are rejected.
    #[serde(default)]
    pub is_enabled: bool,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Daemon this key is bound to 1:1, when provisioned server-side.
    /// NULL for legacy site-shared keys created before 1:1 provisioning,
    /// which resolve daemon identity from the X-Daemon-ID header instead.
    #[serde(default)]
    #[schema(read_only)]
    pub daemon_id: Option<Uuid>,
    /// Plaintext API key for ServerPoll mode daemons only.
    /// Never serialized or logged - wrapped in SecretString for protection.
    /// NULL for DaemonPoll daemons (server doesn't need to send key).
    #[serde(skip)]
    #[validate(skip)]
    pub plaintext: Option<SecretString>,
}

// PartialEq ignores plaintext - we never compare secrets
impl PartialEq for DaemonApiKeyBase {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.name == other.name
            && self.last_used == other.last_used
            && self.expires_at == other.expires_at
            && self.site_id == other.site_id
            && self.is_enabled == other.is_enabled
            && self.tags == other.tags
            && self.daemon_id == other.daemon_id
    }
}

impl Eq for DaemonApiKeyBase {}

impl std::hash::Hash for DaemonApiKeyBase {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.name.hash(state);
        self.last_used.hash(state);
        self.expires_at.hash(state);
        self.site_id.hash(state);
        self.is_enabled.hash(state);
        self.tags.hash(state);
        self.daemon_id.hash(state);
        // plaintext intentionally excluded from hash
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct DaemonApiKey {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: DaemonApiKeyBase,
}

impl DaemonApiKey {
    pub fn suppress_logs(&self, other: &Self) -> bool {
        self.base.key == other.base.key
            && self.base.name == other.base.name
            && self.base.expires_at == other.base.expires_at
            && self.base.site_id == other.base.site_id
            && self.base.is_enabled == other.base.is_enabled
    }
}

impl Display for DaemonApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<DaemonApiKey> for DaemonApiKey {
    fn triggers_staleness(&self, _other: Option<DaemonApiKey>) -> bool {
        false
    }
}

impl ApiKeyCommon for DaemonApiKey {
    const KEY_TYPE: ApiKeyType = ApiKeyType::Daemon;

    fn key(&self) -> &str {
        &self.base.key
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_enabled(&self) -> bool {
        self.base.is_enabled
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.base.expires_at
    }

    fn last_used(&self) -> Option<DateTime<Utc>> {
        self.base.last_used
    }

    fn tags(&self) -> &[Uuid] {
        &self.base.tags
    }

    fn set_key(&mut self, key: String) {
        self.base.key = key;
    }

    fn set_is_enabled(&mut self, enabled: bool) {
        self.base.is_enabled = enabled;
    }

    fn set_last_used(&mut self, time: Option<DateTime<Utc>>) {
        self.base.last_used = time;
    }

    fn refresh_stored_plaintext(&mut self, plaintext: &str) {
        // Only ServerPoll keys hold a plaintext copy; leave DaemonPoll keys (None) alone.
        if self.base.plaintext.is_some() {
            self.base.plaintext = Some(SecretString::from(plaintext.to_string()));
        }
    }
}
