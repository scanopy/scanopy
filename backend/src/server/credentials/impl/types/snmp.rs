//! SNMP-specific credential types for discovery dispatch.

use crate::server::credentials::r#impl::mapping::{
    BannerField, BannerFieldValue, CredentialMapping, IpOverride, ResolvableSecret,
    ResolvedCredential,
};
use crate::server::shared::types::field_definition::SelectOption;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use utoipa::ToSchema;

// ============================================================================
// Core types
// ============================================================================

/// SNMP protocol version
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    Default,
    strum::VariantNames,
    ToSchema,
)]
pub enum SnmpVersion {
    /// SNMPv1 (community string based, distinct wire PDU set, no GET-BULK)
    V1,
    /// SNMPv2c (community string based)
    #[default]
    V2c,
    /// SNMPv3 (authentication + privacy)
    V3,
}

impl std::fmt::Display for SnmpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnmpVersion::V1 => write!(f, "V1"),
            SnmpVersion::V2c => write!(f, "V2c"),
            SnmpVersion::V3 => write!(f, "V3"),
        }
    }
}

impl std::str::FromStr for SnmpVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "V1" | "1" => Ok(SnmpVersion::V1),
            "V2C" | "2C" | "2" => Ok(SnmpVersion::V2c),
            "V3" | "3" => Ok(SnmpVersion::V3),
            _ => Err(anyhow::anyhow!("Invalid SNMP version: {}", s)),
        }
    }
}

/// SNMPv3 USM authentication protocol. Variants are limited to the modern,
/// secure set Scanopy supports; MD5 / SHA-2 variants beyond these are
/// intentionally excluded. Serialized form (e.g. "Sha256") is the wire value
/// stored in the credential and used as the frontend select option value.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    Default,
    strum::VariantNames,
    ToSchema,
)]
pub enum SnmpV3AuthProtocol {
    /// HMAC-SHA-1
    Sha1,
    /// HMAC-SHA-256
    #[default]
    Sha256,
}

impl SnmpV3AuthProtocol {
    /// Human-facing label for the credential form dropdown.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
        }
    }

    /// Select options surfaced to the frontend via `field_definitions()`.
    pub const OPTIONS: &'static [SelectOption] = &[
        SelectOption {
            value: "Sha1",
            label: "SHA-1",
        },
        SelectOption {
            value: "Sha256",
            label: "SHA-256",
        },
    ];
}

/// SNMPv3 USM privacy (encryption) protocol.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    Default,
    strum::VariantNames,
    ToSchema,
)]
pub enum SnmpV3PrivProtocol {
    /// AES-128-CFB
    #[default]
    Aes128,
    /// AES-256-CFB
    Aes256,
}

impl SnmpV3PrivProtocol {
    /// Human-facing label for the credential form dropdown.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Aes128 => "AES-128",
            Self::Aes256 => "AES-256",
        }
    }

    /// Select options surfaced to the frontend via `field_definitions()`.
    pub const OPTIONS: &'static [SelectOption] = &[
        SelectOption {
            value: "Aes128",
            label: "AES-128",
        },
        SelectOption {
            value: "Aes256",
            label: "AES-256",
        },
    ];
}

/// SNMPv3 USM AuthPriv parameters for daemon queries.
/// `community` on the parent struct is unused for v3; these params carry the
/// security material instead. Both passwords are `ResolvableSecret` (inline or
/// daemon-read file path) and never logged in plaintext.
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct SnmpV3Params {
    /// SNMPv3 USM security (user) name.
    pub security_name: String,
    /// Hash algorithm used for authentication.
    pub auth_protocol: SnmpV3AuthProtocol,
    /// Authentication passphrase.
    pub auth_password: ResolvableSecret,
    /// Cipher used for privacy (encryption).
    pub priv_protocol: SnmpV3PrivProtocol,
    /// Privacy passphrase.
    pub priv_password: ResolvableSecret,
    /// Context name, sent as the scoped PDU's contextName. Empty or unset addresses the default
    /// context. Cisco IOS-XE keeps its per-VLAN bridge forwarding database in a named context,
    /// so reading one without this returns the default context's near-empty table (GH #686).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_name: Option<String>,
}

impl std::fmt::Debug for SnmpV3Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnmpV3Params")
            .field("security_name", &self.security_name)
            .field("auth_protocol", &self.auth_protocol)
            .field("auth_password", &"********")
            .field("priv_protocol", &self.priv_protocol)
            .field("priv_password", &"********")
            .field("context_name", &self.context_name)
            .finish()
    }
}

/// SNMP credential for daemon queries. `community` carries the v1/v2c community
/// string; `v3` carries USM parameters when `version` is `V3`.
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct SnmpQueryCredential {
    /// SNMP version to query with.
    #[serde(default)]
    pub version: SnmpVersion,
    /// Community string, for v1 and v2c.
    pub community: ResolvableSecret,
    /// USM parameters, for v3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v3: Option<SnmpV3Params>,
}

impl Default for SnmpQueryCredential {
    fn default() -> Self {
        Self {
            version: SnmpVersion::default(),
            community: ResolvableSecret::Value {
                value: String::new(),
            },
            v3: None,
        }
    }
}

impl std::fmt::Debug for SnmpQueryCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnmpQueryCredential")
            .field("version", &self.version)
            .field("community", &"********")
            .field("v3", &self.v3)
            .finish()
    }
}

impl SnmpQueryCredential {
    pub fn public_default() -> Self {
        Self {
            version: SnmpVersion::default(),
            community: ResolvableSecret::Value {
                value: "public".to_string(),
            },
            v3: None,
        }
    }
}

// ============================================================================
// Banner / metadata
// ============================================================================

/// Banner lines for SNMP credentials
impl SnmpQueryCredential {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        let mut lines = vec![BannerField {
            label: "Version",
            value: BannerFieldValue::Plain(self.version.to_string()),
        }];
        match (&self.version, &self.v3) {
            (SnmpVersion::V3, Some(v3)) => {
                lines.push(BannerField {
                    label: "Security Name",
                    value: BannerFieldValue::Plain(v3.security_name.clone()),
                });
                lines.push(BannerField {
                    label: "Auth Protocol",
                    value: BannerFieldValue::Plain(v3.auth_protocol.label().to_string()),
                });
                lines.push(BannerField {
                    label: "Auth Password",
                    value: v3.auth_password.banner_value(),
                });
                lines.push(BannerField {
                    label: "Priv Protocol",
                    value: BannerFieldValue::Plain(v3.priv_protocol.label().to_string()),
                });
                lines.push(BannerField {
                    label: "Priv Password",
                    value: v3.priv_password.banner_value(),
                });
            }
            _ => {
                lines.push(BannerField {
                    label: "Community",
                    value: self.community.banner_value(),
                });
            }
        }
        lines
    }
}

// ============================================================================
// Legacy Daemon Support (pre-v0.15.0)
//
// These types support daemons < v0.15.0 using SnmpCredentialMapping in
// DiscoveryType::Network. Modern equivalent: `build_credential_mappings_for_discovery()`
// with CredentialQueryPayload. Remove when minimum daemon version >= 0.15.0.
// ============================================================================

/// Legacy: SNMP credential mapping type alias for pre-v0.15.0 daemon DiscoveryType::Network.
pub type SnmpCredentialMapping = CredentialMapping<SnmpQueryCredential>;

/// Legacy: SNMP-specific resolution: IP override → network default → "public" fallback.
/// Deduplicates by community string.
/// Returns `ResolvedCredential` wrappers that pair each credential with its server-side ID.
impl SnmpCredentialMapping {
    pub fn get_credentials_by_specificity(
        &self,
        ip: &IpAddr,
    ) -> Vec<ResolvedCredential<SnmpQueryCredential>> {
        let mut credentials: Vec<ResolvedCredential<SnmpQueryCredential>> = Vec::new();

        // 1. IP-specific override (most specific) — host-scoped, should be auto-assigned
        if let Some(override_cred) = self.ip_overrides.iter().find(|o| &o.ip == ip) {
            let cred_id = override_cred.credential_id;
            credentials.push(ResolvedCredential {
                credential: override_cred.credential.clone(),
                credential_id: if cred_id != uuid::Uuid::nil() {
                    Some(cred_id)
                } else {
                    None
                },
            });
        }

        // 2. Network default — already network-wide, don't auto-assign
        if let Some(ref default) = self.default_credential
            && !credentials
                .iter()
                .any(|c| c.credential.community == default.community)
        {
            credentials.push(ResolvedCredential {
                credential: default.clone(),
                credential_id: None,
            });
        }

        // 3. "public" fallback (least specific) — synthetic, no server-side credential
        let public_default = SnmpQueryCredential::public_default();
        if !credentials
            .iter()
            .any(|c| c.credential.community == public_default.community)
        {
            credentials.push(ResolvedCredential {
                credential: public_default,
                credential_id: None,
            });
        }

        credentials
    }
}

/// Legacy: Exposed SNMP credential for daemon serialization (plaintext secrets).
#[derive(Serialize)]
pub struct SnmpQueryCredentialExposed {
    pub version: SnmpVersion,
    pub community: String,
}

impl From<&SnmpQueryCredential> for SnmpQueryCredentialExposed {
    fn from(cred: &SnmpQueryCredential) -> Self {
        Self {
            version: cred.version,
            community: match &cred.community {
                ResolvableSecret::Value { value } => value.clone(),
                ResolvableSecret::FilePath { .. } => String::new(),
            },
        }
    }
}

/// Legacy: Exposed IP override for daemon serialization (plaintext secrets).
#[derive(Serialize)]
pub struct SnmpIpOverrideExposed {
    pub ip: IpAddr,
    pub credential: SnmpQueryCredentialExposed,
}

impl From<&IpOverride<SnmpQueryCredential>> for SnmpIpOverrideExposed {
    fn from(o: &IpOverride<SnmpQueryCredential>) -> Self {
        Self {
            ip: o.ip,
            credential: SnmpQueryCredentialExposed::from(&o.credential),
        }
    }
}

/// Legacy: Exposed credential mapping for daemon serialization (plaintext secrets).
#[derive(Serialize)]
pub struct SnmpCredentialMappingExposed {
    pub default_credential: Option<SnmpQueryCredentialExposed>,
    pub ip_overrides: Vec<SnmpIpOverrideExposed>,
}

impl From<&SnmpCredentialMapping> for SnmpCredentialMappingExposed {
    fn from(mapping: &SnmpCredentialMapping) -> Self {
        Self {
            default_credential: mapping.default_credential.as_ref().map(Into::into),
            ip_overrides: mapping.ip_overrides.iter().map(Into::into).collect(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;
    use std::net::IpAddr;
    use uuid::Uuid;

    fn cred(community: &str) -> SnmpQueryCredential {
        SnmpQueryCredential {
            version: SnmpVersion::V2c,
            community: ResolvableSecret::Value {
                value: community.to_string(),
            },
            v3: None,
        }
    }

    fn community_value(cred: &SnmpQueryCredential) -> &str {
        match &cred.community {
            ResolvableSecret::Value { value } => value,
            ResolvableSecret::FilePath { path } => path.as_str(),
        }
    }

    #[test]
    fn exposed_serialization_contains_plaintext() {
        let original = SnmpCredentialMapping {
            default_credential: Some(cred("my-secret")),
            ..Default::default()
        };

        let exposed = SnmpCredentialMappingExposed::from(&original);
        let json = serde_json::to_string(&exposed).unwrap();
        assert!(json.contains("my-secret"));
    }

    #[test]
    fn resolvable_secret_roundtrip() {
        let original = cred("my-secret");
        let json = serde_json::to_string(&original).unwrap();
        let roundtripped: SnmpQueryCredential = serde_json::from_str(&json).unwrap();
        assert_eq!(community_value(&roundtripped), "my-secret");
    }

    #[test]
    fn legacy_string_community_deserializes() {
        // Pre-v0.15.0 discovery_type JSONB stored community as a plain string
        // (redacted via Secret<String> serialize). The custom ResolvableSecret
        // deserializer must accept this format.
        let json = r#"{"version":"V2c","community":"********"}"#;
        let cred: SnmpQueryCredential = serde_json::from_str(json).unwrap();
        assert_eq!(
            cred.community,
            ResolvableSecret::Value {
                value: "********".to_string()
            }
        );
    }

    #[test]
    fn specificity_ordering() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let other_ip: IpAddr = "10.0.0.2".parse().unwrap();
        let cred_id = Uuid::new_v4();

        let mapping = SnmpCredentialMapping {
            default_credential: Some(cred("default-community")),
            ip_overrides: vec![IpOverride {
                ip,
                credential: cred("override-community"),
                credential_id: cred_id,
                host_id: None,
                mac_address: None,
            }],
            ..Default::default()
        };

        // IP with override: override first, then default, then public
        let creds = mapping.get_credentials_by_specificity(&ip);
        assert_eq!(creds.len(), 3);
        assert_eq!(community_value(&creds[0].credential), "override-community");
        assert_eq!(creds[0].credential_id, Some(cred_id)); // IP override has credential_id
        assert_eq!(community_value(&creds[1].credential), "default-community");
        assert_eq!(creds[1].credential_id, None); // Network default has no credential_id
        assert_eq!(community_value(&creds[2].credential), "public");
        assert_eq!(creds[2].credential_id, None); // Fallback has no credential_id

        // IP without override: default, then public
        let creds = mapping.get_credentials_by_specificity(&other_ip);
        assert_eq!(creds.len(), 2);
        assert_eq!(community_value(&creds[0].credential), "default-community");
        assert_eq!(community_value(&creds[1].credential), "public");
    }

    #[test]
    fn specificity_deduplicates() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        // Override and default are both "public" — should not duplicate
        let mapping = SnmpCredentialMapping {
            default_credential: Some(cred("public")),
            ip_overrides: vec![IpOverride {
                ip,
                credential: cred("public"),
                credential_id: Uuid::nil(),
                host_id: None,
                mac_address: None,
            }],
            ..Default::default()
        };

        let creds = mapping.get_credentials_by_specificity(&ip);
        assert_eq!(creds.len(), 1);
        assert_eq!(community_value(&creds[0].credential), "public");
        // nil UUID override should result in None credential_id
        assert_eq!(creds[0].credential_id, None);
    }

    #[test]
    fn specificity_nil_credential_id_becomes_none() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();

        let mapping = SnmpCredentialMapping {
            ip_overrides: vec![IpOverride {
                ip,
                credential: cred("secret"),
                credential_id: Uuid::nil(),
                host_id: None,
                mac_address: None,
            }],
            ..Default::default()
        };

        let creds = mapping.get_credentials_by_specificity(&ip);
        assert_eq!(creds[0].credential_id, None);
    }

    #[test]
    fn banner_lines_snmp() {
        let payload = CredentialQueryPayload::Snmp(cred("my-community"));
        let lines = payload.banner_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].label, "Version");
        assert!(matches!(&lines[0].value, BannerFieldValue::Plain(v) if v == "V2c"));
        assert_eq!(lines[1].label, "Community");
        assert!(matches!(
            lines[1].value,
            BannerFieldValue::RedactedInline(12)
        )); // "my-community".len()
    }
}
