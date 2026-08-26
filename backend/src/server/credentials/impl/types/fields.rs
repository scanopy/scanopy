use anyhow::Error;
use serde::Serialize;
use utoipa::ToSchema;

use super::CredentialType;

// ============================================================================
// FieldDefinition — metadata for dynamic frontend form generation
// ============================================================================

#[derive(Debug, Clone, Serialize, ToSchema)]
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
    /// Whether the value is a secret, so it is masked and never echoed back.
    pub secret: bool,
    /// Whether the field may be left empty.
    pub optional: bool,
    /// Explanatory text shown beneath the field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<&'static str>,
    /// Options for `Select` fields. Each option carries a wire `value` (the
    /// serialized enum variant) and a human-facing `label`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<&'static [SelectOption]>,
    /// Value pre-filled when the field is first shown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<&'static str>,
    /// For SecretPathOrInline fields: what format the inline value should be
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_format: Option<InlineFormat>,
    /// Optional group name for visually grouping fields in the UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<&'static str>,
}

/// A single choice for a `Select` field. `value` is the wire value (serialized
/// enum variant, e.g. "Sha256"); `label` is the human-facing display text.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct SelectOption {
    /// Value submitted when this option is chosen.
    pub value: &'static str,
    /// Human-facing option label.
    pub label: &'static str,
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
            Self::SnmpV1 { community: _ } | Self::SnmpV2c { community: _ } => {
                vec![FieldDefinition {
                    id: "community",
                    label: "Community String",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: Some("custom-community-string"),
                    secret: true,
                    optional: false,
                    help_text: Some(
                        "Custom SNMP community string. The default 'public' community is always tried automatically during scans. On Cisco switches, append '@' and a VLAN id — for example 'readonly@20' — to read that VLAN's MAC-address table, which the plain community does not return.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: None,
                }]
            }
            Self::Gnmi {
                port: _,
                username: _,
                password: _,
                // API-settable only for now: no boolean field type exists in this form
                // vocabulary, and the validated lab NOSes serve plaintext gRPC anyway.
                tls: _,
                skip_verify: _,
            } => vec![
                FieldDefinition {
                    id: "port",
                    label: "gNMI Port",
                    field_type: FieldType::String,
                    placeholder: Some("9339"),
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "9339 is the IANA gNMI port; some NOSes listen on 6030 or 57400 instead.",
                    ),
                    options: None,
                    default_value: Some("9339"),
                    inline_format: None,
                    group: Some("Connection"),
                },
                FieldDefinition {
                    id: "username",
                    label: "Username",
                    field_type: FieldType::String,
                    placeholder: Some("gnmi-user"),
                    secret: false,
                    optional: false,
                    help_text: Some(
                        "Sent as gRPC `username` metadata (the OpenConfig convention).",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Authentication"),
                },
                FieldDefinition {
                    id: "password",
                    label: "Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Sent as gRPC `password` metadata."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                },
            ],
            Self::SnmpV3 {
                security_name: _,
                auth_protocol: _,
                auth_password: _,
                priv_protocol: _,
                priv_password: _,
                context_name: _,
            } => vec![
                FieldDefinition {
                    id: "security_name",
                    label: "Security Name",
                    field_type: FieldType::String,
                    placeholder: Some("snmp-user"),
                    secret: false,
                    optional: false,
                    help_text: Some("SNMPv3 USM user name (security name)."),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Authentication"),
                },
                FieldDefinition {
                    id: "auth_protocol",
                    label: "Auth Protocol",
                    field_type: FieldType::Select,
                    placeholder: None,
                    secret: false,
                    optional: false,
                    help_text: Some("Authentication hash algorithm."),
                    options: Some(super::snmp::SnmpV3AuthProtocol::OPTIONS),
                    default_value: Some("Sha256"),
                    inline_format: None,
                    group: Some("Authentication"),
                },
                FieldDefinition {
                    id: "auth_password",
                    label: "Auth Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Authentication password (minimum 8 characters)."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                },
                FieldDefinition {
                    id: "priv_protocol",
                    label: "Privacy Protocol",
                    field_type: FieldType::Select,
                    placeholder: None,
                    secret: false,
                    optional: false,
                    help_text: Some("Privacy (encryption) algorithm."),
                    options: Some(super::snmp::SnmpV3PrivProtocol::OPTIONS),
                    default_value: Some("Aes128"),
                    inline_format: None,
                    group: Some("Privacy"),
                },
                FieldDefinition {
                    id: "priv_password",
                    label: "Privacy Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Privacy (encryption) password (minimum 8 characters)."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Privacy"),
                },
                FieldDefinition {
                    id: "context_name",
                    label: "Context Name",
                    field_type: FieldType::String,
                    placeholder: Some("(default)"),
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "Optional SNMPv3 context name, applied to bridge and VLAN queries only. Cisco and some other vendors keep a separate MAC-address table per VLAN in a named context; naming it here reads that table instead of the near-empty default one. Interface, LLDP and ARP data always come from the default context.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: None,
                },
            ],
            Self::DockerProxy { .. } => container_proxy_field_definitions(
                "Docker API Port",
                "Docker API port. Use 2375 for non-TLS proxies (Tecnativa, HAProxy) or 2376 for TLS.",
            ),
            Self::PodmanProxy { .. } => container_proxy_field_definitions(
                "Podman API Port",
                "Podman API port. Point at a TCP-exposed Podman service (e.g. `podman system service tcp:`) directly or behind a TLS proxy.",
            ),
            Self::DockerSocket { .. } => vec![socket_path_field(
                "/var/run/docker.sock",
                "Path to the Docker Unix socket. Leave blank to auto-detect (DOCKER_HOST or /var/run/docker.sock).",
            )],
            Self::PodmanSocket { .. } => vec![socket_path_field(
                "/run/podman/podman.sock",
                "Path to the Podman Unix socket. Leave blank to auto-detect (rootful /run/podman/podman.sock or the rootless $XDG_RUNTIME_DIR/podman/podman.sock).",
            )],
            Self::UnifiApiKey { .. } => {
                let mut fields = unifi_connection_fields();
                fields.push(FieldDefinition {
                    id: "api_key",
                    label: "API Key",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: false,
                    help_text: Some(
                        "Network Application API key (Settings → Control Plane → Integrations → Create API Key). Requires UniFi OS; the legacy self-hosted Network Application on 8443 has no API keys — use a UniFi Local Admin credential there instead.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                });
                fields
            }
            Self::UnifiLocalAdmin { .. } => {
                let mut fields = unifi_connection_fields();
                fields.push(FieldDefinition {
                    id: "username",
                    label: "Username",
                    field_type: FieldType::String,
                    placeholder: None,
                    secret: false,
                    optional: false,
                    help_text: Some(
                        "Controller admin username. Use a local-only admin account so multi-factor authentication does not block the login.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Authentication"),
                });
                fields.push(FieldDefinition {
                    id: "password",
                    label: "Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Password for the local admin account."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                });
                fields
            }
            Self::InstantOnAccount { .. } => vec![
                FieldDefinition {
                    id: "username",
                    label: "Portal Account",
                    field_type: FieldType::String,
                    placeholder: Some("scanopy@example.com"),
                    secret: false,
                    optional: false,
                    help_text: Some(
                        "Email address of an Instant On portal account with access to the site. Add a dedicated account with the read-only Viewer role (Site management → Accounts managing this site) rather than using your own administrator login, and make sure multi-factor authentication is disabled on it — the sign-in cannot answer an MFA prompt.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Authentication"),
                },
                FieldDefinition {
                    id: "password",
                    label: "Password",
                    field_type: FieldType::SecretPathOrInline,
                    placeholder: None,
                    secret: true,
                    optional: false,
                    help_text: Some("Password for that portal account."),
                    options: None,
                    default_value: None,
                    inline_format: Some(InlineFormat::Plain),
                    group: Some("Authentication"),
                },
                FieldDefinition {
                    id: "site",
                    label: "Site",
                    field_type: FieldType::String,
                    placeholder: Some("(all sites)"),
                    secret: false,
                    optional: true,
                    help_text: Some(
                        "Limit the fetch to one Instant On site by name. Leave blank to read every site this account can see. Assign this credential to a single switch — each host it is assigned to fetches the whole site again.",
                    ),
                    options: None,
                    default_value: None,
                    inline_format: None,
                    group: Some("Scope"),
                },
            ],
        }
    }
}

/// Connection fields shared by both UniFi transports. Only the auth fields differ, so the
/// port/site pair is defined once — a UniFi credential of either kind points at the same
/// controller endpoint.
fn unifi_connection_fields() -> Vec<FieldDefinition> {
    vec![
        FieldDefinition {
            id: "port",
            label: "Controller Port",
            field_type: FieldType::String,
            placeholder: Some("443"),
            secret: false,
            optional: true,
            help_text: Some(
                "443 for a UniFi OS console (Dream Machine, Cloud Key, Cloud Gateway), 11443 for a self-hosted UniFi OS Server, or 8443 for the legacy self-hosted Network Application. Check the port in your controller's URL — the wrong port fails to connect.",
            ),
            options: None,
            default_value: Some("443"),
            inline_format: None,
            group: Some("Connection"),
        },
        FieldDefinition {
            id: "site",
            label: "Site",
            field_type: FieldType::String,
            placeholder: Some("default"),
            secret: false,
            optional: true,
            help_text: Some(
                "Internal site name, taken from the controller URL (/manage/site/<name>) — not the site's display name. Most installations use 'default'.",
            ),
            options: None,
            default_value: Some("default"),
            inline_format: None,
            group: Some("Connection"),
        },
    ]
}

/// Optional, non-secret socket-path field for local container-socket credentials. The
/// `placeholder` shows the runtime's default socket so it differs for Docker vs Podman. Blank ⇒
/// the daemon auto-detects, so a socket credential needs no required config.
fn socket_path_field(placeholder: &'static str, help: &'static str) -> FieldDefinition {
    FieldDefinition {
        id: "socket_path",
        label: "Socket Path",
        field_type: FieldType::String,
        placeholder: Some(placeholder),
        secret: false,
        optional: true,
        help_text: Some(help),
        options: None,
        default_value: None,
        inline_format: None,
        group: Some("Connection"),
    }
}

/// Shared field definitions for a container-runtime (Docker/Podman) proxy
/// credential. Only the port label/help differ between runtimes; the path and
/// TLS fields are identical because both speak the Docker-compatible API.
fn container_proxy_field_definitions(
    port_label: &'static str,
    port_help: &'static str,
) -> Vec<FieldDefinition> {
    vec![
        FieldDefinition {
            id: "port",
            label: port_label,
            field_type: FieldType::String,
            placeholder: Some("2375"),
            secret: false,
            optional: true,
            help_text: Some(port_help),
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
            help_text: Some("PEM private key. All three TLS fields must be provided together."),
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
    ]
}
