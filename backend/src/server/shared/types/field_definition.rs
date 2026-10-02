use anyhow::Error;
use serde::Serialize;
use utoipa::ToSchema;

/// Field types for dynamic form generation.
///
/// The serialized lowercase name is the wire contract with the frontend, which switches on it to
/// pick a renderer and a validator. Adding a variant means teaching the frontend about it.
#[derive(Serialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    /// Single-line text input.
    String,
    /// Multi-line text area.
    Text,
    /// Numeric input with no implied range.
    Number,
    /// Checkbox.
    Boolean,
    /// Fixed choice from `options`.
    Select,
    /// Fixed choice from `options`, every option shown at once. For a short list where seeing the
    /// alternatives is part of choosing, such as the shell an SSH script runs in.
    Radio,
    /// TCP/UDP port. Distinct from `Number` because it carries a 1-65535 range the frontend
    /// validator enforces — and because declaring it is what stops the form guessing "is this a
    /// port?" from the field's label.
    Port,
    /// Secret value supplied either inline or as a path to a file the daemon reads.
    SecretPathOrInline,
    /// Non-secret value supplied either inline or as a path to a file the daemon reads.
    PathOrInline,
    /// An SSH script: a file on the scanned host, a file on the daemon, or text entered here.
    ScriptSource,
}

/// Definition of a form field for dynamic UI rendering.
///
/// One definition serves every dynamic form: credential types and discovery scan settings both
/// build these, and the frontend renders them through the same switch. Reaches the UI through
/// build-time fixtures (`credential-types.json`, `integrations.json`, `scan-settings.json`) and,
/// for the generated TypeScript union, through the OpenAPI schema.
#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct FieldDefinition {
    /// Server-assigned unique identifier.
    pub id: &'static str,
    /// Human-facing field label.
    pub label: &'static str,
    /// How the field should be rendered and validated.
    pub field_type: FieldType,
    /// Placeholder text for the input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
    /// Placeholders that replace `placeholder` while another field holds a given value, e.g. a
    /// Windows path once the OS picker says Windows. `field` is a sibling field id, or `daemon_os`
    /// for the credential's own daemon OS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder_by: Option<&'static [DependentPlaceholder]>,
    /// Whether the value is a secret, so it is masked and never echoed back.
    pub secret: bool,
    /// Whether the field may be left empty.
    pub optional: bool,
    /// Explanatory text shown beneath the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<&'static str>,
    /// Choices for `Select` fields. Each option carries a wire `value` (the serialized enum
    /// variant) and a human-facing `label`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<&'static [SelectOption]>,
    /// Value pre-filled when the field is first shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<&'static str>,
    /// For `PathOrInline` and `SecretPathOrInline` fields: what format the inline value must be.
    /// Also drives server-side validation in `CredentialType::validate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_format: Option<InlineFormat>,
    /// Grouping label used to section a long form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<&'static str>,
    /// For a field that can be read from a file on the daemon: an example file name, which the
    /// form joins to the Daemon OS's example directory for the path placeholder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<&'static str>,
}

/// A placeholder that applies while `depends_on` holds `value`.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct DependentPlaceholder {
    /// Id of the form field this placeholder depends on (or `daemon_os` for the credential's
    /// Daemon OS).
    pub depends_on: &'static str,
    /// Value that field must hold for this placeholder to apply.
    pub value: &'static str,
    /// Placeholder shown while it does.
    pub placeholder: &'static str,
}

/// A single choice for a `Select` field. `value` is the wire value (serialized enum variant, e.g.
/// "Sha256"); `label` is the human-facing display text.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct SelectOption {
    /// Value submitted when this option is chosen.
    pub value: &'static str,
    /// Human-facing option label.
    pub label: &'static str,
}

/// Format hint for inline values in `PathOrInline` and `SecretPathOrInline` fields.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum InlineFormat {
    /// Plain text (e.g. SNMP community string, API key)
    Plain,
    /// PEM-encoded private key
    PemPrivateKey,
    /// PEM-encoded certificate (public, non-secret)
    PemCertificate,
    /// SSH private key: OpenSSH format (`ssh-keygen`'s default) or PKCS#8 / RSA / EC PEM.
    SshPrivateKey,
    /// Six bytes written as a MAC address (e.g. a Wake-on-LAN SecureOn password).
    MacAddress,
}

/// PEM block tag — the label between `-----BEGIN` and `-----`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PemTag {
    Certificate,
    PrivateKey,
    RsaPrivateKey,
    EcPrivateKey,
    OpensshPrivateKey,
}

impl PemTag {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Certificate => "CERTIFICATE",
            Self::PrivateKey => "PRIVATE KEY",
            Self::RsaPrivateKey => "RSA PRIVATE KEY",
            Self::EcPrivateKey => "EC PRIVATE KEY",
            Self::OpensshPrivateKey => "OPENSSH PRIVATE KEY",
        }
    }
}

impl InlineFormat {
    /// PEM tags accepted by this format, or empty for non-PEM formats.
    pub fn allowed_pem_tags(&self) -> &'static [PemTag] {
        match self {
            Self::Plain | Self::MacAddress => &[],
            Self::PemCertificate => &[PemTag::Certificate],
            Self::PemPrivateKey => &[
                PemTag::PrivateKey,
                PemTag::RsaPrivateKey,
                PemTag::EcPrivateKey,
            ],
            Self::SshPrivateKey => &[
                PemTag::OpensshPrivateKey,
                PemTag::PrivateKey,
                PemTag::RsaPrivateKey,
                PemTag::EcPrivateKey,
            ],
        }
    }

    /// Validate a resolved value matches the expected format.
    /// Returns Ok(()) for Plain format (no validation needed).
    pub fn validate(&self, value: &str, field_name: &str) -> Result<(), Error> {
        if let Self::MacAddress = self {
            let trimmed = value.trim();
            if !trimmed.is_empty() && trimmed.parse::<mac_address::MacAddress>().is_err() {
                crate::bail_validation!(
                    "{} must be six bytes written as a MAC address, e.g. 01:23:45:67:89:ab",
                    field_name
                );
            }
            return Ok(());
        }
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
