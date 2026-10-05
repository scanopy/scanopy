use std::fmt::Display;

use crate::server::shared::api_key_common::{ApiKeyCommon, ApiKeyType};
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::types::api::serialize_sensitive_info;
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct UserApiKeyBase {
    /// The stored key. Returned redacted except on creation and rotation.
    #[serde(default)]
    #[serde(serialize_with = "serialize_sensitive_info")]
    #[schema(read_only, required)]
    pub key: String,
    /// Human-facing name for this key.
    pub name: String,
    /// User the key acts on behalf of.
    pub user_id: Uuid,
    /// The organization that owns this record.
    pub organization_id: Uuid,
    /// Role the key is limited to, which cannot exceed the user's own.
    #[serde(default)]
    pub permissions: UserOrgPermissions,
    /// When this key was last used to authenticate.
    #[serde(default)]
    #[schema(read_only, required)]
    pub last_used: Option<DateTime<Utc>>,
    /// When this record stops being valid.
    pub expires_at: Option<DateTime<Utc>>,
    /// Whether the key may still be used. Disabled keys are rejected.
    #[serde(default)]
    pub is_enabled: bool,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Site IDs this key has access to (hydrated from junction table)
    #[serde(default)]
    pub site_ids: Vec<Uuid>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct UserApiKey {
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
    pub base: UserApiKeyBase,
}

impl UserApiKey {
    /// Check if the key changes should suppress logging
    /// (only logs significant changes, not just last_used updates)
    pub fn suppress_logs(&self, other: &Self) -> bool {
        self.base.key == other.base.key
            && self.base.name == other.base.name
            && self.base.expires_at == other.base.expires_at
            && self.base.is_enabled == other.base.is_enabled
            && self.base.permissions == other.base.permissions
            && self.base.site_ids == other.base.site_ids
    }
}

impl Display for UserApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<UserApiKey> for UserApiKey {
    fn triggers_staleness(&self, _other: Option<UserApiKey>) -> bool {
        false
    }
}

impl ApiKeyCommon for UserApiKey {
    const KEY_TYPE: ApiKeyType = ApiKeyType::User;

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
}
