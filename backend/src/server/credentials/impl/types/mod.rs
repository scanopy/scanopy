use crate::server::{
    credentials::r#impl::mapping::{
        CredentialQueryPayload, DockerProxyQueryCredential, ResolvableSecret, ResolvableValue,
    },
    ports::r#impl::base::PortType,
    services::r#impl::definitions::ServiceDefinition,
};
use anyhow::Error;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, EnumIter};
use strum_macros::IntoStaticStr;
use utoipa::ToSchema;

pub mod docker_proxy;
pub mod snmp;

mod fields;
mod metadata;
mod secrets;

pub use fields::{FieldDefinition, FieldType, InlineFormat, PemTag};
pub use metadata::{CredentialAssignment, CredentialCategory, ScopeModel};
pub use secrets::{
    FileOrInline, REDACTED_SECRET_SENTINEL, SecretValue, StorageCredentialType,
    deserialize_optional_file_or_inline, deserialize_optional_secret_value,
};

// Re-export SnmpVersion from snmp submodule
pub use snmp::SnmpVersion;

fn default_docker_port() -> u16 {
    PortType::Docker.number()
}

/// Universal credential type — tagged enum stored as JSONB.
/// Each variant represents a different credential protocol/method.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, EnumDiscriminants, IntoStaticStr)]
#[strum_discriminants(derive(Display, Hash, Serialize, Deserialize, IntoStaticStr, EnumIter))]
#[serde(tag = "type")]
pub enum CredentialType {
    /// SNMPv2c community string for querying network devices
    SnmpV2c { community: SecretValue },
    /// Docker API proxy credentials. Target IP determined from host interfaces at scan time.
    DockerProxy {
        /// Port for the Docker API proxy (default 2375)
        #[serde(default = "default_docker_port")]
        port: u16,
        /// Optional URL path prefix (e.g. "/v1.43")
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// PEM-encoded public certificate — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_cert: Option<FileOrInline>,
        /// Private key — inline PEM content or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_secret_value"
        )]
        ssl_key: Option<SecretValue>,
        /// PEM-encoded CA chain — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_chain: Option<FileOrInline>,
    },
}

impl PartialEq for CredentialType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SnmpV2c { community: c1 }, Self::SnmpV2c { community: c2 }) => match (c1, c2) {
                (SecretValue::Inline { value: a }, SecretValue::Inline { value: b }) => {
                    a.expose_secret() == b.expose_secret()
                }
                (SecretValue::FilePath { path: a }, SecretValue::FilePath { path: b }) => a == b,
                _ => false,
            },
            (
                Self::DockerProxy {
                    port: p1,
                    path: pa1,
                    ssl_cert: c1,
                    ssl_key: k1,
                    ssl_chain: ch1,
                },
                Self::DockerProxy {
                    port: p2,
                    path: pa2,
                    ssl_cert: c2,
                    ssl_key: k2,
                    ssl_chain: ch2,
                },
            ) => {
                p1 == p2
                    && pa1 == pa2
                    && c1 == c2
                    && match (k1, k2) {
                        (
                            Some(SecretValue::Inline { value: a }),
                            Some(SecretValue::Inline { value: b }),
                        ) => a.expose_secret() == b.expose_secret(),
                        (
                            Some(SecretValue::FilePath { path: a }),
                            Some(SecretValue::FilePath { path: b }),
                        ) => a == b,
                        (None, None) => true,
                        _ => false,
                    }
                    && ch1 == ch2
            }
            _ => false,
        }
    }
}

/// Returns a discriminant string for `CredentialType` (the serde tag value).
impl CredentialType {
    /// Merge redacted sentinel values from the existing credential.
    /// When the API response redacts secrets to "********" and the UI sends that back,
    /// this restores the original secret from the existing record.
    pub fn merge_redacted_secrets(&mut self, existing: &CredentialType) {
        match (self, existing) {
            (
                Self::SnmpV2c { community },
                Self::SnmpV2c {
                    community: existing_community,
                },
            ) => {
                if community.is_redacted_sentinel() {
                    *community = existing_community.clone();
                }
            }
            (
                Self::DockerProxy { ssl_key, .. },
                Self::DockerProxy {
                    ssl_key: existing_key,
                    ..
                },
            ) => {
                if let Some(key) = ssl_key
                    && key.is_redacted_sentinel()
                {
                    *ssl_key = existing_key.clone();
                }
            }
            // Type changed — no merging needed
            _ => {}
        }
    }

    pub fn credential_category(&self) -> CredentialCategory {
        match self {
            Self::SnmpV2c { .. } => CredentialCategory::NetworkMonitoring,
            Self::DockerProxy { .. } => CredentialCategory::ContainerVirtualization,
        }
    }

    pub fn scope_models(&self) -> Vec<ScopeModel> {
        match self {
            Self::SnmpV2c { .. } => vec![ScopeModel::Broadcast, ScopeModel::PerHost],
            Self::DockerProxy { .. } => vec![ScopeModel::PerHost],
        }
    }

    /// Get the inline string value for a field by ID.
    /// Returns None for FilePath mode, None fields, or redacted sentinels.
    fn get_inline_value(&self, field_id: &str) -> Option<String> {
        match self {
            Self::SnmpV2c { community } => match field_id {
                "community" => match community {
                    SecretValue::Inline { value } => {
                        let v = value.expose_secret().to_string();
                        if v == REDACTED_SECRET_SENTINEL {
                            None
                        } else {
                            Some(v)
                        }
                    }
                    SecretValue::FilePath { .. } => None,
                },
                _ => None,
            },
            Self::DockerProxy {
                ssl_cert,
                ssl_key,
                ssl_chain,
                ..
            } => match field_id {
                "ssl_cert" => match ssl_cert.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                "ssl_key" => match ssl_key.as_ref()? {
                    SecretValue::Inline { value } => {
                        let v = value.expose_secret().to_string();
                        if v == REDACTED_SECRET_SENTINEL {
                            None
                        } else {
                            Some(v)
                        }
                    }
                    SecretValue::FilePath { .. } => None,
                },
                "ssl_chain" => match ssl_chain.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                _ => None,
            },
        }
    }

    /// Validate inline field values using field_definitions() metadata.
    /// Skips FilePath values (validated on daemon after read), redacted sentinels,
    /// and empty optionals.
    pub fn validate(&self) -> Result<(), Error> {
        for field in self.field_definitions() {
            if let Some(fmt) = &field.inline_format
                && let Some(value) = self.get_inline_value(field.id)
            {
                fmt.validate(&value, field.label)?;
            }
        }
        Ok(())
    }

    /// Returns the ServiceDefinition this credential type integrates with.
    /// Every credential type maps to exactly one service — used for logo display,
    /// metadata enrichment, and Phase 2 integration dispatch.
    pub fn associated_service(&self) -> Box<dyn ServiceDefinition> {
        match self {
            Self::SnmpV2c { .. } => Box::new(crate::server::services::definitions::snmp::Snmp),
            Self::DockerProxy { .. } => {
                Box::new(crate::server::services::definitions::docker_daemon::Docker)
            }
        }
    }

    /// Convert to wire format payload for daemon transmission.
    /// No wildcard match — compiler forces update when new variants added.
    pub fn to_query_payload(&self) -> CredentialQueryPayload {
        match self {
            CredentialType::SnmpV2c { community } => CredentialQueryPayload::Snmp(
                crate::server::credentials::r#impl::mapping::SnmpQueryCredential {
                    version: SnmpVersion::V2c,
                    community: match community {
                        SecretValue::Inline { value } => ResolvableSecret::Value {
                            value: value.expose_secret().to_string(),
                        },
                        SecretValue::FilePath { path } => {
                            ResolvableSecret::FilePath { path: path.clone() }
                        }
                    },
                },
            ),
            CredentialType::DockerProxy {
                port,
                path,
                ssl_cert,
                ssl_key,
                ssl_chain,
            } => CredentialQueryPayload::DockerProxy(DockerProxyQueryCredential {
                port: *port,
                path: path.clone(),
                ssl_cert: ssl_cert.as_ref().map(|f| match f {
                    FileOrInline::Inline { value } => ResolvableValue::Value {
                        value: value.clone(),
                    },
                    FileOrInline::FilePath { path } => {
                        ResolvableValue::FilePath { path: path.clone() }
                    }
                }),
                ssl_key: ssl_key.as_ref().map(|s| match s {
                    SecretValue::Inline { value } => ResolvableSecret::Value {
                        value: value.expose_secret().to_string(),
                    },
                    SecretValue::FilePath { path } => {
                        ResolvableSecret::FilePath { path: path.clone() }
                    }
                }),
                ssl_chain: ssl_chain.as_ref().map(|f| match f {
                    FileOrInline::Inline { value } => ResolvableValue::Value {
                        value: value.clone(),
                    },
                    FileOrInline::FilePath { path } => {
                        ResolvableValue::FilePath { path: path.clone() }
                    }
                }),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    fn snmp_cred(community: &str) -> CredentialType {
        CredentialType::SnmpV2c {
            community: SecretValue::Inline {
                value: SecretString::from(community.to_string()),
            },
        }
    }

    fn docker_cred(ssl_key: Option<&str>) -> CredentialType {
        CredentialType::DockerProxy {
            port: 2376,
            path: None,
            ssl_cert: None,
            ssl_key: ssl_key.map(|k| SecretValue::Inline {
                value: SecretString::from(k.to_string()),
            }),
            ssl_chain: None,
        }
    }

    #[test]
    fn merge_redacted_secrets_preserves_original_when_sentinel_sent() {
        let existing = snmp_cred("my-secret-community");
        let mut updated = snmp_cred(REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::SnmpV2c { community } = &updated {
            if let SecretValue::Inline { value } = community {
                assert_eq!(value.expose_secret(), "my-secret-community");
            } else {
                panic!("Expected Inline secret");
            }
        } else {
            panic!("Expected Snmp variant");
        }
    }

    #[test]
    fn merge_redacted_secrets_allows_actual_value_changes() {
        let existing = snmp_cred("old-community");
        let mut updated = snmp_cred("new-community");
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::SnmpV2c { community } = &updated {
            if let SecretValue::Inline { value } = community {
                assert_eq!(value.expose_secret(), "new-community");
            } else {
                panic!("Expected Inline secret");
            }
        } else {
            panic!("Expected Snmp variant");
        }
    }

    #[test]
    fn merge_redacted_secrets_handles_type_mismatch() {
        let existing = snmp_cred("secret");
        let mut updated = docker_cred(Some("new-key"));
        // Should be a no-op — types don't match
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::DockerProxy { ssl_key, .. } = &updated {
            if let Some(SecretValue::Inline { value }) = ssl_key {
                assert_eq!(value.expose_secret(), "new-key");
            } else {
                panic!("Expected Some(Inline)");
            }
        } else {
            panic!("Expected DockerProxy variant");
        }
    }

    #[test]
    fn merge_redacted_secrets_handles_docker_ssl_key() {
        let existing = docker_cred(Some("original-key"));
        let mut updated = docker_cred(Some(REDACTED_SECRET_SENTINEL));
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::DockerProxy { ssl_key, .. } = &updated {
            if let Some(SecretValue::Inline { value }) = ssl_key {
                assert_eq!(value.expose_secret(), "original-key");
            } else {
                panic!("Expected Some(Inline)");
            }
        } else {
            panic!("Expected DockerProxy variant");
        }
    }

    #[test]
    fn is_redacted_sentinel_detects_sentinel() {
        let sentinel = SecretValue::Inline {
            value: SecretString::from(REDACTED_SECRET_SENTINEL.to_string()),
        };
        assert!(sentinel.is_redacted_sentinel());

        let real = SecretValue::Inline {
            value: SecretString::from("real-secret".to_string()),
        };
        assert!(!real.is_redacted_sentinel());

        let filepath = SecretValue::FilePath {
            path: REDACTED_SECRET_SENTINEL.to_string(),
        };
        assert!(!filepath.is_redacted_sentinel());
    }
}
