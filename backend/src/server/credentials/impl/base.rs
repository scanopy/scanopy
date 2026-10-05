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
    /// validated for it, and a daemon on another OS cannot use the credential. Set exactly when
    /// the credential reads something on the daemon; `None` otherwise.
    #[serde(default)]
    pub daemon_os: Option<OsFamily>,
    /// Tags assigned to this entity.
    #[serde(default = "default_tags")]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Sites this credential is assigned to (Broadcast scope).
    /// Hydrated from the `site_credentials` junction table.
    #[serde(default)]
    #[schema(required)]
    pub assigned_site_ids: Vec<Uuid>,
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
            && self.assigned_site_ids == other.assigned_site_ids
            && self.host_assignments == other.host_assignments
    }
}

impl CredentialBase {
    /// Everything the API checks about a credential's settings before saving it: the inline
    /// formats its type declares, and every path for the OS of the machine that holds it.
    pub fn validate_settings(&self) -> Result<(), anyhow::Error> {
        self.credential_type.validate()?;
        match self.daemon_os {
            Some(os) => self.credential_type.validate_paths(os),
            None if self.credential_type.reads_daemon_paths() => Err(anyhow::anyhow!(
                "Choose the Daemon OS: this credential reads a file or socket on the daemon"
            )),
            // No daemon-side path, so any OS validates the scanned-host paths alike.
            None => self.credential_type.validate_paths(OsFamily::default()),
        }
    }

    /// Drop a `daemon_os` the credential has no use for, so it never blocks a daemon on an OS the
    /// credential does not care about.
    pub fn clear_unused_daemon_os(&mut self) {
        if !self.credential_type.reads_daemon_paths() {
            self.daemon_os = None;
        }
    }

    /// Why the daemon `daemon_name`, on `daemon_os`, cannot use this credential: both OSes are
    /// known and differ. `None` when it can.
    pub fn daemon_os_refusal(
        &self,
        daemon_name: &str,
        daemon_os: Option<crate::server::daemons::r#impl::base::DaemonOs>,
    ) -> Option<String> {
        use crate::server::shared::types::metadata::TypeMetadataProvider;
        match (self.daemon_os, daemon_os) {
            (Some(declared), Some(actual)) if declared != OsFamily::from(actual) => Some(format!(
                "Credential \"{}\" reads files on a {} daemon, and \"{daemon_name}\" runs {}. Set the credential's Daemon OS to match, or choose another credential.",
                self.name,
                declared.metadata().name,
                actual.name(),
            )),
            _ => None,
        }
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
            daemon_os: None,
            tags: Vec::new(),
            assigned_site_ids: Vec::new(),
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
