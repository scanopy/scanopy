use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::net::IpAddr;
use strum::IntoDiscriminant;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

fn default_tags() -> Vec<Uuid> {
    Vec::new()
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, ToSchema)]
pub struct CredentialBase {
    pub organization_id: Uuid,
    #[validate(length(
        min = 1,
        max = 100,
        message = "Credential name must be between 1 and 100 characters"
    ))]
    pub name: String,
    pub credential_type: CredentialType,
    /// Ephemeral bootstrap IPs for pre-discovery credential resolution.
    /// Write-only — skipped in API GET responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(write_only, value_type = Option<Vec<String>>)]
    pub target_ips: Option<Vec<IpAddr>>,
    #[serde(default = "default_tags")]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

impl PartialEq for CredentialBase {
    fn eq(&self, other: &Self) -> bool {
        self.organization_id == other.organization_id
            && self.name == other.name
            && self.credential_type == other.credential_type
            && self.target_ips == other.target_ips
            && self.tags == other.tags
    }
}

impl Default for CredentialBase {
    fn default() -> Self {
        use crate::server::credentials::r#impl::types::SecretValue;
        use secrecy::SecretString;
        Self {
            organization_id: Uuid::nil(),
            name: "New Credential".to_string(),
            credential_type: CredentialType::SnmpV2c {
                community: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            target_ips: None,
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema, Validate)]
pub struct Credential {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
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
