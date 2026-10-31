//! SNMP-specific credential types for discovery dispatch.

use crate::server::credentials::r#impl::mapping::{
    BannerField, BannerFieldValue, CredentialMapping, IpOverride, ResolvableSecret,
};
use crate::server::shared::types::field_definition::{
    FieldSetValue, SelectOption, field_set_options,
};
use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};
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

/// SNMPv3 USM authentication protocol. MD5 is offered for devices that support nothing else;
/// SHA-2 variants beyond SHA-256 are not. Serialized form (e.g. "Sha256") is the wire value
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
    strum::EnumIter,
    strum::IntoStaticStr,
    ToSchema,
)]
pub enum SnmpV3AuthProtocol {
    /// HMAC-MD5. Weaker than SHA; for devices that support nothing else.
    Md5,
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
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
        }
    }

    /// Select options surfaced to the frontend via `field_definitions()`.
    pub const OPTIONS: &'static [SelectOption] = &[
        SelectOption {
            value: "Md5",
            label: "MD5",
        },
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
    strum::EnumIter,
    strum::IntoStaticStr,
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

/// SNMPv3 USM security level (RFC 3414): whether requests are authenticated, and whether they
/// are also encrypted. Picks which of [`SnmpV3Security`]'s fields a credential carries.
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
    strum::EnumIter,
    strum::IntoStaticStr,
    ToSchema,
)]
pub enum SnmpV3SecurityLevel {
    /// User name only: no authentication, no encryption.
    NoAuthNoPriv,
    /// Authenticated, not encrypted.
    AuthNoPriv,
    /// Authenticated and encrypted. Every v3 credential stored before the level existed.
    #[default]
    AuthPriv,
}

impl FieldSetValue for SnmpV3SecurityLevel {
    fn label(&self) -> &'static str {
        match self {
            Self::NoAuthNoPriv => "No authentication, no privacy (noAuthNoPriv)",
            Self::AuthNoPriv => "Authentication, no privacy (authNoPriv)",
            Self::AuthPriv => "Authentication and privacy (authPriv)",
        }
    }

    fn options() -> &'static [SelectOption] {
        static OPTIONS: std::sync::LazyLock<Vec<SelectOption>> =
            std::sync::LazyLock::new(field_set_options::<SnmpV3SecurityLevel>);
        &OPTIONS
    }
}

/// The security material of an SNMPv3 credential, by security level. A level carries exactly
/// the fields it uses, so a credential cannot hold a privacy password it never sends.
///
/// `S` is the secret's representation: `SecretValue` where the credential is stored,
/// `ResolvableSecret` on the server→daemon wire. Flattened into its parent, so the stored JSON
/// keeps the flat shape it had before levels existed.
#[derive(Debug, Clone, Serialize, Eq, PartialEq, Hash, ToSchema)]
#[serde(tag = "security_level")]
pub enum SnmpV3Security<S> {
    /// User name only.
    #[schema(title = "NoAuthNoPriv")]
    NoAuthNoPriv,
    /// Authenticated, not encrypted.
    #[schema(title = "AuthNoPriv")]
    AuthNoPriv {
        /// Hash algorithm used for authentication.
        auth_protocol: SnmpV3AuthProtocol,
        /// Authentication passphrase.
        auth_password: S,
    },
    /// Authenticated and encrypted.
    #[schema(title = "AuthPriv")]
    AuthPriv {
        /// Hash algorithm used for authentication.
        auth_protocol: SnmpV3AuthProtocol,
        /// Authentication passphrase.
        auth_password: S,
        /// Cipher used for privacy (encryption).
        priv_protocol: SnmpV3PrivProtocol,
        /// Privacy passphrase.
        priv_password: S,
    },
}

/// Read by hand rather than derived, for one reason: a credential stored before security levels
/// existed has no `security_level`, and an internally tagged enum cannot default its tag. Those
/// rows were all AuthPriv, so a missing level reads as AuthPriv. Fields the level does not use
/// are dropped rather than refused, so a form that sent them back costs nothing.
impl<'de, S: Deserialize<'de>> Deserialize<'de> for SnmpV3Security<S> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        #[derive(Deserialize)]
        #[serde(bound = "S: Deserialize<'de>")]
        struct Fields<S> {
            #[serde(default)]
            security_level: SnmpV3SecurityLevel,
            auth_protocol: Option<SnmpV3AuthProtocol>,
            auth_password: Option<S>,
            priv_protocol: Option<SnmpV3PrivProtocol>,
            priv_password: Option<S>,
        }

        let fields = Fields::<S>::deserialize(deserializer)?;
        let level = fields.security_level;
        let missing = |field: &'static str| {
            D::Error::custom(format!("SNMPv3 {level:?} credential is missing `{field}`"))
        };
        Ok(match level {
            SnmpV3SecurityLevel::NoAuthNoPriv => Self::NoAuthNoPriv,
            SnmpV3SecurityLevel::AuthNoPriv => Self::AuthNoPriv {
                auth_protocol: fields
                    .auth_protocol
                    .ok_or_else(|| missing("auth_protocol"))?,
                auth_password: fields
                    .auth_password
                    .ok_or_else(|| missing("auth_password"))?,
            },
            SnmpV3SecurityLevel::AuthPriv => Self::AuthPriv {
                auth_protocol: fields
                    .auth_protocol
                    .ok_or_else(|| missing("auth_protocol"))?,
                auth_password: fields
                    .auth_password
                    .ok_or_else(|| missing("auth_password"))?,
                priv_protocol: fields
                    .priv_protocol
                    .ok_or_else(|| missing("priv_protocol"))?,
                priv_password: fields
                    .priv_password
                    .ok_or_else(|| missing("priv_password"))?,
            },
        })
    }
}

impl<S> SnmpV3Security<S> {
    pub fn level(&self) -> SnmpV3SecurityLevel {
        match self {
            Self::NoAuthNoPriv => SnmpV3SecurityLevel::NoAuthNoPriv,
            Self::AuthNoPriv { .. } => SnmpV3SecurityLevel::AuthNoPriv,
            Self::AuthPriv { .. } => SnmpV3SecurityLevel::AuthPriv,
        }
    }

    /// The same security material with each secret converted, e.g. a stored `SecretValue` to
    /// the `ResolvableSecret` the daemon receives.
    pub fn map_secrets<T>(&self, convert: impl Fn(&S) -> T) -> SnmpV3Security<T> {
        match self.try_map_secrets(|_, secret| Ok::<_, std::convert::Infallible>(convert(secret))) {
            Ok(mapped) => mapped,
        }
    }

    /// [`Self::map_secrets`] with a conversion that can fail, e.g. reading a secret's file.
    /// The conversion also receives the field id, for its error message.
    pub fn try_map_secrets<T, E>(
        &self,
        convert: impl Fn(&'static str, &S) -> Result<T, E>,
    ) -> Result<SnmpV3Security<T>, E> {
        Ok(match self {
            Self::NoAuthNoPriv => SnmpV3Security::NoAuthNoPriv,
            Self::AuthNoPriv {
                auth_protocol,
                auth_password,
            } => SnmpV3Security::AuthNoPriv {
                auth_protocol: *auth_protocol,
                auth_password: convert("auth_password", auth_password)?,
            },
            Self::AuthPriv {
                auth_protocol,
                auth_password,
                priv_protocol,
                priv_password,
            } => SnmpV3Security::AuthPriv {
                auth_protocol: *auth_protocol,
                auth_password: convert("auth_password", auth_password)?,
                priv_protocol: *priv_protocol,
                priv_password: convert("priv_password", priv_password)?,
            },
        })
    }

    /// Each secret by its field id, for the per-field passes over a credential (redaction merge,
    /// inline values, file paths).
    pub fn secrets(&self) -> Vec<(&'static str, &S)> {
        match self {
            Self::NoAuthNoPriv => vec![],
            Self::AuthNoPriv { auth_password, .. } => vec![("auth_password", auth_password)],
            Self::AuthPriv {
                auth_password,
                priv_password,
                ..
            } => vec![
                ("auth_password", auth_password),
                ("priv_password", priv_password),
            ],
        }
    }

    /// [`Self::secrets`], mutably.
    pub fn secrets_mut(&mut self) -> Vec<(&'static str, &mut S)> {
        match self {
            Self::NoAuthNoPriv => vec![],
            Self::AuthNoPriv { auth_password, .. } => vec![("auth_password", auth_password)],
            Self::AuthPriv {
                auth_password,
                priv_password,
                ..
            } => vec![
                ("auth_password", auth_password),
                ("priv_password", priv_password),
            ],
        }
    }
}

/// SNMPv3 USM parameters for daemon queries.
/// `community` on the parent struct is unused for v3; these params carry the
/// security material instead. Passwords are `ResolvableSecret` (inline or
/// daemon-read file path) and never logged in plaintext.
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct SnmpV3Params {
    /// SNMPv3 USM security (user) name.
    pub security_name: String,
    /// Security level and the authentication / privacy material it uses.
    #[serde(flatten)]
    pub security: SnmpV3Security<ResolvableSecret>,
    /// Context name, sent as the scoped PDU's contextName. Empty or unset addresses the default
    /// context. Cisco IOS-XE keeps its per-VLAN bridge forwarding database in a named context,
    /// so reading one without this returns the default context's near-empty table (GH #686).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_name: Option<String>,
}

impl std::fmt::Debug for SnmpV3Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `ResolvableSecret`'s own Debug redacts, so the security material prints safely.
        f.debug_struct("SnmpV3Params")
            .field("security_name", &self.security_name)
            .field("security", &self.security)
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

// The protocols reach the UI as metadata so it can write the net-snmp options for a stored
// credential: the check commands on an SNMP scan warning look the credential up by id and
// render its version, user, level, protocols and context, with every secret a placeholder.

impl HasId for SnmpV3AuthProtocol {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SnmpV3AuthProtocol {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        Icon::Lock
    }
}

impl TypeMetadataProvider for SnmpV3AuthProtocol {
    fn name(&self) -> &'static str {
        self.label()
    }

    /// The name net-snmp's `-a` takes for this protocol.
    fn metadata(&self) -> serde_json::Value {
        let net_snmp_name = match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA",
            Self::Sha256 => "SHA-256",
        };
        serde_json::json!({ "net_snmp_name": net_snmp_name })
    }
}

impl HasId for SnmpV3PrivProtocol {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SnmpV3PrivProtocol {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        Icon::Lock
    }
}

impl TypeMetadataProvider for SnmpV3PrivProtocol {
    fn name(&self) -> &'static str {
        self.label()
    }

    /// The name net-snmp's `-x` takes for this protocol. The daemon extends AES-256 keys the
    /// Blumenthal way, which is what net-snmp's `AES-256` does (`AES-256C` is the Reeder way).
    fn metadata(&self) -> serde_json::Value {
        let net_snmp_name = match self {
            Self::Aes128 => "AES",
            Self::Aes256 => "AES-256",
        };
        serde_json::json!({ "net_snmp_name": net_snmp_name })
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
                    label: "Security Level",
                    value: BannerFieldValue::Plain(format!("{:?}", v3.security.level())),
                });
                match &v3.security {
                    SnmpV3Security::NoAuthNoPriv => {}
                    SnmpV3Security::AuthNoPriv {
                        auth_protocol,
                        auth_password,
                    } => {
                        lines.push(BannerField {
                            label: "Auth Protocol",
                            value: BannerFieldValue::Plain(auth_protocol.label().to_string()),
                        });
                        lines.push(BannerField {
                            label: "Auth Password",
                            value: auth_password.banner_value(),
                        });
                    }
                    SnmpV3Security::AuthPriv {
                        auth_protocol,
                        auth_password,
                        priv_protocol,
                        priv_password,
                    } => {
                        lines.push(BannerField {
                            label: "Auth Protocol",
                            value: BannerFieldValue::Plain(auth_protocol.label().to_string()),
                        });
                        lines.push(BannerField {
                            label: "Auth Password",
                            value: auth_password.banner_value(),
                        });
                        lines.push(BannerField {
                            label: "Priv Protocol",
                            value: BannerFieldValue::Plain(priv_protocol.label().to_string()),
                        });
                        lines.push(BannerField {
                            label: "Priv Password",
                            value: priv_password.banner_value(),
                        });
                    }
                }
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

    /// A v3 credential row exactly as every release before security levels stored it: no
    /// `security_level`, all four AuthPriv fields. It must load as the AuthPriv credential it was.
    #[test]
    fn a_v3_row_stored_before_security_levels_loads_as_auth_priv() {
        use super::super::{CredentialType, SecretValue};
        use secrecy::ExposeSecret;

        let stored = r#"{
            "type": "SnmpV3",
            "security_name": "scanopy-ro",
            "auth_protocol": "Sha256",
            "auth_password": {"mode": "Inline", "value": "auth-secret"},
            "priv_protocol": "Aes128",
            "priv_password": {"mode": "FilePath", "path": "/etc/scanopy/snmpv3-priv"},
            "context_name": "vlan-20"
        }"#;
        let credential: CredentialType = serde_json::from_str(stored).expect("old row loads");

        let CredentialType::SnmpV3 {
            security_name,
            security:
                SnmpV3Security::AuthPriv {
                    auth_protocol,
                    auth_password: SecretValue::Inline { value },
                    priv_protocol,
                    priv_password: SecretValue::FilePath { path },
                },
            context_name,
        } = &credential
        else {
            panic!("expected an AuthPriv v3 credential, got {credential:?}");
        };
        assert_eq!(security_name, "scanopy-ro");
        assert_eq!(*auth_protocol, SnmpV3AuthProtocol::Sha256);
        assert_eq!(value.expose_secret(), "auth-secret");
        assert_eq!(*priv_protocol, SnmpV3PrivProtocol::Aes128);
        assert_eq!(path.as_str(), "/etc/scanopy/snmpv3-priv");
        assert_eq!(context_name.as_deref(), Some("vlan-20"));

        // And it reaches the daemon unchanged.
        let CredentialQueryPayload::Snmp(snmp) = credential.to_query_payload(None).unwrap() else {
            panic!("an SNMP credential");
        };
        assert_eq!(
            snmp.v3.expect("v3 params").security.level(),
            SnmpV3SecurityLevel::AuthPriv
        );
    }

    #[test]
    fn a_level_keeps_only_the_fields_it_uses() {
        // A form that sent the privacy fields back while AuthNoPriv was picked.
        let json = r#"{
            "security_level": "AuthNoPriv",
            "auth_protocol": "Md5",
            "auth_password": {"mode": "Value", "value": "a"},
            "priv_protocol": "Aes128",
            "priv_password": {"mode": "Value", "value": "p"}
        }"#;
        let security: SnmpV3Security<ResolvableSecret> = serde_json::from_str(json).unwrap();
        assert_eq!(security.secrets().len(), 1);
        let reserialized = serde_json::to_value(&security).unwrap();
        assert!(reserialized.get("priv_password").is_none());

        let no_auth: SnmpV3Security<ResolvableSecret> = serde_json::from_str(
            r#"{"security_level": "NoAuthNoPriv", "auth_password": {"mode": "Value", "value": "a"}}"#,
        )
        .unwrap();
        assert_eq!(no_auth, SnmpV3Security::NoAuthNoPriv);
    }

    #[test]
    fn a_level_missing_a_field_it_needs_is_refused() {
        let json = r#"{
            "security_level": "AuthPriv",
            "auth_protocol": "Sha1",
            "auth_password": {"mode": "Value", "value": "a"},
            "priv_protocol": "Aes128"
        }"#;
        let err = serde_json::from_str::<SnmpV3Security<ResolvableSecret>>(json).unwrap_err();
        assert!(err.to_string().contains("priv_password"), "{err}");
    }
}
