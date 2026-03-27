use anyhow::Error;
use serde::Serialize;
use utoipa::ToSchema;

use super::CredentialType;

// ============================================================================
// FieldDefinition — metadata for dynamic frontend form generation
// ============================================================================

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FieldDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub field_type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
    pub secret: bool,
    pub optional: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<&'static [&'static str]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<&'static str>,
    /// For SecretPathOrInline fields: what format the inline value should be
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_format: Option<InlineFormat>,
    /// Optional group name for visually grouping fields in the UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<&'static str>,
}

/// Format hint for inline values in PathOrInline and SecretPathOrInline fields.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum InlineFormat {
    /// Plain text (e.g. SNMP community string, API key)
    Plain,
    /// PEM-encoded private key
    PemPrivateKey,
    /// PEM-encoded certificate (public, non-secret)
    PemCertificate,
}

/// PEM block tag — the label between `-----BEGIN` and `-----`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PemTag {
    Certificate,
    PrivateKey,
    RsaPrivateKey,
    EcPrivateKey,
}

impl PemTag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Certificate => "CERTIFICATE",
            Self::PrivateKey => "PRIVATE KEY",
            Self::RsaPrivateKey => "RSA PRIVATE KEY",
            Self::EcPrivateKey => "EC PRIVATE KEY",
        }
    }
}

impl InlineFormat {
    /// PEM tags accepted by this format, or empty for non-PEM formats.
    pub fn allowed_pem_tags(&self) -> &'static [PemTag] {
        match self {
            Self::Plain => &[],
            Self::PemCertificate => &[PemTag::Certificate],
            Self::PemPrivateKey => &[
                PemTag::PrivateKey,
                PemTag::RsaPrivateKey,
                PemTag::EcPrivateKey,
            ],
        }
    }

    /// Validate a resolved value matches the expected format.
    /// Returns Ok(()) for Plain format (no validation needed).
    pub fn validate(&self, value: &str, field_name: &str) -> Result<(), Error> {
        let tags = self.allowed_pem_tags();
        if tags.is_empty() {
            return Ok(());
        }
        validate_pem(value, field_name, tags)
    }
}

/// Parse PEM and verify at least one entry has a tag in `allowed_tags`.
fn validate_pem(value: &str, field_name: &str, allowed_tags: &[PemTag]) -> Result<(), Error> {
    use crate::server::shared::types::api::ValidationError;

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let entries = pem::parse_many(trimmed)
        .map_err(|e| ValidationError::new(format!("{} is not valid PEM: {}", field_name, e)))?;
    if entries.is_empty() {
        crate::bail_validation!("{} contains no PEM data", field_name);
    }
    if !entries
        .iter()
        .any(|p| allowed_tags.iter().any(|t| t.as_str() == p.tag()))
    {
        let expected = allowed_tags
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(" or ");
        crate::bail_validation!(
            "{} must contain a {} PEM block, found: {}",
            field_name,
            expected,
            entries
                .iter()
                .map(|p| p.tag().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Text,
    Select,
    SecretPathOrInline,
    PathOrInline,
}

impl CredentialType {
    /// Returns field definitions for this credential type.
    /// Uses exhaustive destructuring for compile-time enforcement:
    /// adding a field to the enum variant without updating this method causes a compile error.
    pub fn field_definitions(&self) -> Vec<FieldDefinition> {
        match self {
            Self::SnmpV2c { community: _ } => vec![FieldDefinition {
                id: "community",
                label: "Community String",
                field_type: FieldType::SecretPathOrInline,
                placeholder: Some("custom-community-string"),
                secret: true,
                optional: false,
                help_text: Some(
                    "Custom SNMP community string. The default 'public' community is always tried automatically during scans.",
                ),
                options: None,
                default_value: None,
                inline_format: Some(InlineFormat::Plain),
                group: None,
            }],
            Self::DockerProxy {
                port: _,
                path: _,
                ssl_cert: _,
                ssl_key: _,
                ssl_chain: _,
            } => vec![
                FieldDefinition {
                    id: "port",
                    label: "Docker API Port",
                    field_type: FieldType::String,
                    placeholder: Some("2375"),
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "Docker API port. Use 2375 for non-TLS proxies (Tecnativa, HAProxy) or 2376 for TLS.",
                    ),
                    options: None,
                    default_value: Some("2375"),
                    inline_format: None,
                    group: Some("Connection"),
                },
                FieldDefinition {
                    id: "path",
                    label: "URL Path Prefix",
                    field_type: FieldType::String,
                    placeholder: Some("/"),
                    secret: false,
                    optional: true,
                    help_text: Some("Optional URL path prefix appended after the port"),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Connection"),
                },
                FieldDefinition {
                    id: "ssl_cert",
                    label: "SSL Certificate",
                    field_type: FieldType::PathOrInline,
                    placeholder: Some("-----BEGIN CERTIFICATE-----"),
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "PEM-encoded client certificate. All three TLS fields (cert, key, CA chain) must be provided together.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::PemCertificate),
                    group: Some("TLS"),
                },
                FieldDefinition {
                    id: "ssl_key",
                    label: "SSL Private Key",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: true,
                    help_text: Some(
                        "PEM private key. All three TLS fields must be provided together.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::PemPrivateKey),
                    group: Some("TLS"),
                },
                FieldDefinition {
                    id: "ssl_chain",
                    label: "SSL CA Chain",
                    field_type: FieldType::PathOrInline,
                    placeholder: Some("-----BEGIN CERTIFICATE-----"),
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "PEM-encoded CA certificate chain. All three TLS fields must be provided together.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::PemCertificate),
                    group: Some("TLS"),
                },
            ],
        }
    }
}
