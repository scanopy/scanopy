use crate::server::credentials::r#impl::types::{
    CredentialHostAssignment, CredentialType, OsFamily,
};
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::types::api::deserialize_empty_string_as_none;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum::IntoDiscriminant;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

fn default_tags() -> Vec<Uuid> {
    Vec::new()
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, ToSchema)]
pub struct CredentialBase {
    /// The organization that owns this record.
    pub organization_id: Uuid,
    /// Human-facing name for this credential.
    #[validate(length(
        min = 1,
        max = 100,
        message = "Credential name must be between 1 and 100 characters"
    ))]
    pub name: String,
    /// Free-text notes about the credential: what it is for, who owns it.
    #[validate(length(min = 0, max = 500))]
    #[serde(default, deserialize_with = "deserialize_empty_string_as_none")]
    #[schema(required)]
    pub description: Option<String>,
    /// Protocol this credential authenticates with, and its settings.
    pub credential_type: CredentialType,
    /// The OS of the daemons that will read this credential's files and sockets. Paths are
    /// validated for it, and a daemon on another OS skips the credential with a warning.
    #[serde(default)]
    pub daemon_os: OsFamily,
    /// Tags assigned to this entity.
    #[serde(default = "default_tags")]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Networks this credential is assigned to (Broadcast scope).
    /// Hydrated from the `network_credentials` junction table.
    #[serde(default)]
    #[schema(required)]
    pub assigned_network_ids: Vec<Uuid>,
    /// Hosts this credential is assigned to (PerHost scope), with optional IP scoping.
    /// Hydrated from the `host_credentials` junction table.
    #[serde(default)]
    #[schema(required)]
    pub host_assignments: Vec<CredentialHostAssignment>,
}

impl PartialEq for CredentialBase {
    fn eq(&self, other: &Self) -> bool {
        self.organization_id == other.organization_id
            && self.name == other.name
            && self.description == other.description
            && self.credential_type == other.credential_type
            && self.daemon_os == other.daemon_os
            && self.tags == other.tags
            && self.assigned_network_ids == other.assigned_network_ids
            && self.host_assignments == other.host_assignments
    }
}

impl CredentialBase {
    /// Everything the API checks about a credential's settings before saving it: the inline
    /// formats its type declares, and every path for the OS of the machine that holds it.
    pub fn validate_settings(&self) -> Result<(), anyhow::Error> {
        self.credential_type.validate()?;
        self.credential_type.validate_paths(self.daemon_os)
    }
}

impl Default for CredentialBase {
    fn default() -> Self {
        use crate::server::credentials::r#impl::types::SecretValue;
        use secrecy::SecretString;
        Self {
            organization_id: Uuid::nil(),
            name: "New Credential".to_string(),
            description: None,
            credential_type: CredentialType::SnmpV2c {
                community: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            daemon_os: OsFamily::default(),
            tags: Vec::new(),
            assigned_network_ids: Vec::new(),
            host_assignments: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Validate)]
pub struct Credential {
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
    pub base: CredentialBase,
}

impl PartialEq for Credential {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.created_at == other.created_at
            && self.updated_at == other.updated_at
            && self.base == other.base
    }
}

impl ChangeTriggersTopologyStaleness<Credential> for Credential {
    fn triggers_staleness(&self, _other: Option<Credential>) -> bool {
        false
    }
}

impl Display for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Credential {}: {} ({})",
            self.id,
            self.base.name,
            self.base.credential_type.discriminant()
        )
    }
}

impl Credential {
    pub fn new(base: CredentialBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }
}
