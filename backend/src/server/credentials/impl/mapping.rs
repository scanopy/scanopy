//! Generic credential mapping for discovery dispatch.
//!
//! The mapping types define how credentials are resolved per-IP during discovery.
//! `CredentialMapping<T>` is generic over the query credential type.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};
use strum::EnumDiscriminants;
use tempfile::NamedTempFile;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::shared::types::metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider};
use crate::server::shared::types::{Color, Icon};

// Re-export type-specific types so external imports don't break
pub use super::types::container_proxy::ContainerProxyQueryCredential;
pub use super::types::instant_on::InstantOnQueryCredential;
pub use super::types::unifi::{UnifiAuth, UnifiQueryCredential};

/// Container-runtime (Docker/Podman) socket query credential. The daemon connects via a local
/// Unix socket; `socket_path` optionally repoints it (e.g. rootless Podman at
/// `$XDG_RUNTIME_DIR/podman/podman.sock`, a non-default `DOCKER_HOST`). Blank ⇒ the daemon
/// auto-detects (bollard defaults for Docker, `resolve_podman_socket_path()` for Podman).
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default)]
pub struct ContainerSocketQueryCredential {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_path: Option<String>,
}
/// gNMI query credential the daemon dials with. Username/password travel as gRPC metadata;
/// the password uses the same [`ResolvableSecret`] resolution SNMP communities do.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct GnmiQueryCredential {
    pub port: u16,
    pub username: String,
    pub password: ResolvableSecret,
    pub tls: bool,
    pub skip_verify: bool,
}

pub use super::types::snmp::{
    SnmpCredentialMapping, SnmpCredentialMappingExposed, SnmpIpOverrideExposed,
    SnmpQueryCredential, SnmpQueryCredentialExposed, SnmpV3AuthProtocol, SnmpV3Params,
    SnmpV3PrivProtocol, SnmpVersion,
};

// ============================================================================
// Generic Credential Mapping
// ============================================================================

/// Generic credential mapping: a default credential for the network
/// plus per-IP overrides for specific hosts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CredentialMapping<T> {
    #[serde(default)]
    pub default_credential: Option<T>,
    #[serde(default)]
    pub ip_overrides: Vec<IpOverride<T>>,
}

/// IP-specific credential override
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct IpOverride<T> {
    pub ip: IpAddr,
    pub credential: T,
    /// Credential ID for tracking which credential was used during discovery.
    #[serde(default)]
    pub credential_id: Uuid,
}

impl<T> IpOverride<T> {
    /// Check if this override targets localhost (127.0.0.1 or ::1).
    pub fn is_localhost(&self) -> bool {
        self.ip == IpAddr::V4(Ipv4Addr::LOCALHOST) || self.ip == IpAddr::V6(Ipv6Addr::LOCALHOST)
    }
}

impl<T> CredentialMapping<T> {
    /// Check if any credentials are configured
    pub fn is_enabled(&self) -> bool {
        self.default_credential.is_some() || !self.ip_overrides.is_empty()
    }

    /// Get credential for a specific IP, falling back to default
    pub fn get_credential_for_ip(&self, ip: &IpAddr) -> Option<&T> {
        self.ip_overrides
            .iter()
            .find(|o| &o.ip == ip)
            .map(|o| &o.credential)
            .or(self.default_credential.as_ref())
    }

    /// Collect all unique credential IDs referenced in this mapping's IP overrides.
    /// Excludes nil UUIDs (which indicate no server-side credential).
    pub fn credential_ids(&self) -> Vec<Uuid> {
        self.ip_overrides
            .iter()
            .map(|o| o.credential_id)
            .filter(|id| *id != Uuid::nil())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
}

/// A credential payload paired with its server-side ID (if host-assignable).
/// `credential_id` is Some for host-scoped credentials (IP overrides from host assignments).
/// None for network-level defaults and fallbacks — those don't get auto-assigned
/// to discovered hosts because they're already available network-wide.
#[derive(Debug, Clone)]
pub struct ResolvedCredential<T> {
    pub credential: T,
    pub credential_id: Option<Uuid>,
}

/// Per-daemon integration targeting, stored on the `Discovery` entity and delivered via the
/// init command at registration. Each entry references exactly one stored credential and says
/// where it applies on this daemon. This is the single home for cred↔IP targeting — it replaces
/// the global, race-prone `credential.target_ips`.
///
/// The variants ARE the scopes; their strum [`Target`] discriminants are the capability enum that
/// `CredentialType::targets()` returns and validates against (single source of truth). Every
/// target carries a real `credential_id` — there is no credential-less branch and no nil
/// sentinel; a local socket is just a credential whose type targets only the daemon host.
#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema, EnumDiscriminants,
)]
// `Target` is the capability enum returned by `CredentialType::targets()`: where a credential
// can apply (DaemonHost / Network / Hosts). It's the strum discriminant of `IntegrationTarget`.
#[strum_discriminants(
    name(Target),
    derive(Serialize, Deserialize, Hash, ToSchema, strum::VariantNames)
)]
#[serde(tag = "scope")]
pub enum IntegrationTarget {
    /// The daemon's own host — realized as a 127.0.0.1 IP-override (e.g. a local Docker/Podman
    /// socket, or any credential the user pins to the daemon host without naming its IP).
    #[schema(title = "DaemonHost")]
    DaemonHost {
        /// Credential to use on the daemon host.
        credential_id: Uuid,
    },
    /// All hosts on the network — a broadcast default credential.
    #[schema(title = "Network")]
    Network {
        /// Credential to use across the network.
        credential_id: Uuid,
    },
    /// Specific host IPs — one IP-override per address.
    #[schema(title = "Hosts")]
    Hosts {
        /// Credential to use on the listed addresses.
        credential_id: Uuid,
        /// The host addresses this credential applies to.
        #[schema(value_type = Vec<String>)]
        ips: Vec<IpAddr>,
    },
}

impl IntegrationTarget {
    /// The stored credential this target references (present in every variant).
    pub fn credential_id(&self) -> Uuid {
        match self {
            Self::DaemonHost { credential_id }
            | Self::Network { credential_id }
            | Self::Hosts { credential_id, .. } => *credential_id,
        }
    }
}

/// The compact token grammar the daemon accepts via `--credential-id` /
/// `SCANOPY_CREDENTIAL_IDS`. Inverse of `parse_integration_target_tokens` in
/// `daemon/shared/config.rs`, which is what the daemon parses these back with.
impl std::fmt::Display for IntegrationTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // A sole loopback target *is* the daemon-host scope, per the parser.
            Self::DaemonHost { credential_id } => write!(f, "{credential_id}@127.0.0.1"),
            Self::Network { credential_id } => write!(f, "{credential_id}"),
            Self::Hosts { credential_id, ips } => {
                write!(f, "{credential_id}@")?;
                for (i, ip) in ips.iter().enumerate() {
                    if i > 0 {
                        write!(f, "+")?;
                    }
                    write!(f, "{ip}")?;
                }
                Ok(())
            }
        }
    }
}

// ============================================================================
// Generic Credential Query Types (wire format for unified discovery)
// ============================================================================

/// Credential payload sent to daemon with secrets exposed.
/// Each variant corresponds to a CredentialType variant.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, EnumDiscriminants)]
// The discriminant is the integration's stable identity: it labels the discovery-warning metric
// and rides on every coded credential warning, which is why it needs serde and a schema of its own
// rather than only `Display`. The display names live in metadata, never here.
#[strum_discriminants(
    derive(
        Hash,
        PartialOrd,
        Ord,
        Serialize,
        Deserialize,
        ToSchema,
        strum::Display,
        strum::EnumIter,
        strum::IntoStaticStr,
        strum::VariantNames
    ),
    strum(serialize_all = "PascalCase")
)]
#[serde(tag = "type")]
pub enum CredentialQueryPayload {
    Snmp(SnmpQueryCredential),
    DockerProxy(ContainerProxyQueryCredential),
    DockerSocket(ContainerSocketQueryCredential),
    PodmanProxy(ContainerProxyQueryCredential),
    PodmanSocket(ContainerSocketQueryCredential),
    /// Both UniFi transports (API key and local admin) share this payload; the auth
    /// material is discriminated inside `UnifiAuth`.
    UnifiController(UnifiQueryCredential),
    /// HPE Networking Instant On cloud portal. The only payload here whose endpoint is off the
    /// operator's network entirely — the daemon authenticates to HPE's cloud and reads the site
    /// inventory, while the credential stays bound to the switch it reports on.
    InstantOn(InstantOnQueryCredential),
    /// gNMI (OpenConfig) devices — openconfig-interfaces and openconfig-lldp collection.
    Gnmi(GnmiQueryCredential),
    /// Forward-compat fallback: a credential type from a newer server that this
    /// daemon doesn't recognize. `#[serde(other)]` deserializes any unknown `type`
    /// tag here (a unit variant, the only shape allowed for `other` on an
    /// internally-tagged enum — mirrors `EntitySource`/`SubnetType`) instead of
    /// hard-failing the whole discovery request. The daemon's dispatch skips it.
    #[serde(other)]
    Unknown,
}

impl Default for CredentialQueryPayload {
    fn default() -> Self {
        Self::Snmp(SnmpQueryCredential::default())
    }
}

impl From<CredentialQueryPayloadDiscriminants> for super::types::CredentialTypeDiscriminants {
    fn from(d: CredentialQueryPayloadDiscriminants) -> Self {
        match d {
            CredentialQueryPayloadDiscriminants::Snmp => Self::SnmpV2c,
            CredentialQueryPayloadDiscriminants::DockerProxy => Self::DockerProxy,
            CredentialQueryPayloadDiscriminants::DockerSocket => Self::DockerSocket,
            CredentialQueryPayloadDiscriminants::PodmanProxy => Self::PodmanProxy,
            CredentialQueryPayloadDiscriminants::PodmanSocket => Self::PodmanSocket,
            // Lossy but harmless: this reverse map only picks a representative
            // `CredentialType` for a wire tag, and both UniFi transports share one.
            CredentialQueryPayloadDiscriminants::Gnmi => Self::Gnmi,
            CredentialQueryPayloadDiscriminants::UnifiController => Self::UnifiApiKey,
            CredentialQueryPayloadDiscriminants::InstantOn => Self::InstantOnAccount,
            // `Unknown` is the daemon-side forward-compat sentinel; the server only
            // ever builds `CredentialQueryPayload` from a known `CredentialType`, so
            // this reverse conversion never sees it. Fall back to the SNMP default to
            // keep the mapping total (unreachable server-side).
            CredentialQueryPayloadDiscriminants::Unknown => Self::SnmpV2c,
        }
    }
}

impl CredentialQueryPayload {
    /// The proxy credential for either container-runtime proxy variant
    /// (Docker/Podman), which share the same Docker-compatible API shape.
    pub fn as_container_proxy(&self) -> Option<&ContainerProxyQueryCredential> {
        match self {
            Self::DockerProxy(c) | Self::PodmanProxy(c) => Some(c),
            _ => None,
        }
    }

    /// Ports that should be included in light scans for this credential type.
    /// Used by network scanning to ensure integration-relevant ports are always scanned.
    pub fn required_scan_ports(&self) -> Vec<u16> {
        match self {
            Self::Snmp(_) => vec![161, 1161],
            Self::Gnmi(g) => vec![g.port],
            Self::DockerProxy(d) | Self::PodmanProxy(d) => vec![d.port],
            Self::DockerSocket(_) | Self::PodmanSocket(_) => vec![],
            Self::UnifiController(u) => vec![u.port],
            // Nothing to scan for: the endpoint is HPE's cloud, and the switch this credential is
            // bound to does not have to expose any port for the fetch to work.
            Self::InstantOn(_) => vec![],
            Self::Unknown => vec![],
        }
    }

    pub fn discovery_label(&self) -> &'static str {
        match self {
            Self::Snmp(_) => "SNMP queries",
            Self::Gnmi(_) => "gNMI queries",
            Self::DockerProxy(_) => "Docker proxy connection",
            Self::DockerSocket(_) => "Docker socket connection",
            Self::PodmanProxy(_) => "Podman proxy connection",
            Self::PodmanSocket(_) => "Podman socket connection",
            Self::UnifiController(_) => "UniFi controller connection",
            Self::InstantOn(_) => "Instant On portal connection",
            Self::Unknown => "unknown credential",
        }
    }
}

/// Display metadata for the integration behind a credential.
///
/// The discriminant is what a coded scan warning carries, and it needs a name an operator
/// recognises — "SNMP", not `Snmp`. Distinct from `integrations.json`, which is keyed by display
/// name and covers the five *integrations*, and from `credential-types.json`, which is keyed by
/// `CredentialType` (ten variants: SnmpV1/V2c/V3, UnifiApiKey/UnifiLocalAdmin, …). Neither is keyed
/// by these eight values, so neither can resolve them: reusing the credential-type fixture happened
/// to work for `DockerProxy` and silently rendered `Snmp`, `UnifiController` and `InstantOn` as
/// their raw discriminants.
///
/// Named to sit inside "the {name} credential", which is the phrasing every credential warning
/// uses and the one the prose these codes replaced used before them.
impl HasId for CredentialQueryPayloadDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for CredentialQueryPayloadDiscriminants {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        Icon::KeyRound
    }
}

impl TypeMetadataProvider for CredentialQueryPayloadDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            Self::Snmp => "SNMP",
            Self::DockerProxy => "Docker proxy",
            Self::DockerSocket => "Docker socket",
            Self::PodmanProxy => "Podman proxy",
            Self::PodmanSocket => "Podman socket",
            Self::UnifiController => "UniFi controller",
            Self::InstantOn => "Instant On portal",
            Self::Gnmi => "gNMI",
            // Reachable only from a warning written by a newer binary than this one.
            Self::Unknown => "unrecognised",
        }
    }
}

impl CredentialQueryPayload {
    /// Resolve all FilePath fields to Value by reading from disk,
    /// then validate PEM contents for fields that require it.
    pub fn resolve_file_paths(&self) -> Result<Self, anyhow::Error> {
        use super::types::InlineFormat;

        let label = self.discovery_label();
        match self {
            Self::Snmp(snmp) => {
                let v3 = snmp
                    .v3
                    .as_ref()
                    .map(|v3| -> Result<_, anyhow::Error> {
                        Ok(super::types::snmp::SnmpV3Params {
                            security_name: v3.security_name.clone(),
                            auth_protocol: v3.auth_protocol,
                            auth_password: v3
                                .auth_password
                                .resolve_to_value("auth_password", label)?,
                            priv_protocol: v3.priv_protocol,
                            priv_password: v3
                                .priv_password
                                .resolve_to_value("priv_password", label)?,
                            context_name: v3.context_name.clone(),
                        })
                    })
                    .transpose()?;
                Ok(Self::Snmp(SnmpQueryCredential {
                    version: snmp.version,
                    community: snmp.community.resolve_to_value("community", label)?,
                    v3,
                }))
            }
            Self::DockerProxy(d) | Self::PodmanProxy(d) => {
                let ssl_cert = d
                    .ssl_cert
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_cert", label))
                    .transpose()?;
                let ssl_key = d
                    .ssl_key
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_key", label))
                    .transpose()?;
                let ssl_chain = d
                    .ssl_chain
                    .as_ref()
                    .map(|v| v.resolve_to_value("ssl_chain", label))
                    .transpose()?;

                // Validate resolved PEM contents
                if let Some(ResolvableValue::Value { value }) = &ssl_cert {
                    InlineFormat::PemCertificate.validate(value, "SSL Certificate")?;
                }
                if let Some(ResolvableSecret::Value { value }) = &ssl_key {
                    InlineFormat::PemPrivateKey.validate(value, "SSL Private Key")?;
                }
                if let Some(ResolvableValue::Value { value }) = &ssl_chain {
                    InlineFormat::PemCertificate.validate(value, "SSL CA Chain")?;
                }

                let resolved = ContainerProxyQueryCredential {
                    port: d.port,
                    path: d.path.clone(),
                    ssl_cert,
                    ssl_key,
                    ssl_chain,
                };
                Ok(match self {
                    Self::PodmanProxy(_) => Self::PodmanProxy(resolved),
                    _ => Self::DockerProxy(resolved),
                })
            }
            Self::DockerSocket(d) => Ok(Self::DockerSocket(d.clone())),
            Self::PodmanSocket(d) => Ok(Self::PodmanSocket(d.clone())),
            // No PEM validation — UniFi secrets are opaque plain strings.
            Self::UnifiController(u) => Ok(Self::UnifiController(UnifiQueryCredential {
                port: u.port,
                site: u.site.clone(),
                auth: match &u.auth {
                    UnifiAuth::ApiKey { api_key } => UnifiAuth::ApiKey {
                        api_key: api_key.resolve_to_value("api_key", label)?,
                    },
                    UnifiAuth::LocalAdmin { username, password } => UnifiAuth::LocalAdmin {
                        username: username.clone(),
                        password: password.resolve_to_value("password", label)?,
                    },
                },
            })),
            // No PEM validation — a portal password is an opaque plain string.
            Self::InstantOn(i) => Ok(Self::InstantOn(InstantOnQueryCredential {
                username: i.username.clone(),
                password: i.password.resolve_to_value("password", label)?,
                site: i.site.clone(),
            })),
            Self::Gnmi(g) => Ok(Self::Gnmi(GnmiQueryCredential {
                port: g.port,
                username: g.username.clone(),
                password: g.password.resolve_to_value("password", label)?,
                tls: g.tls,
                skip_verify: g.skip_verify,
            })),
            Self::Unknown => Ok(Self::Unknown),
        }
    }

    pub fn banner_lines(&self) -> Vec<BannerField> {
        match self {
            Self::Gnmi(_) => vec![],
            Self::Snmp(snmp) => snmp.banner_lines(),
            Self::DockerProxy(c) | Self::PodmanProxy(c) => c.banner_lines(),
            Self::DockerSocket(_) | Self::PodmanSocket(_) => vec![],
            Self::UnifiController(u) => u.banner_lines(),
            Self::InstantOn(i) => i.banner_lines(),
            Self::Unknown => vec![],
        }
    }
}

/// Non-secret value — inline or file path. Daemon can log freely.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(tag = "mode")]
pub enum ResolvableValue {
    Value { value: String },
    FilePath { path: String },
}

/// Secret value — inline or file path. Daemon wraps resolved value in Secret<String>.
/// Never logged in plaintext.
///
/// Custom Deserialize accepts both the current tagged-enum format
/// (`{"mode":"Value","value":"..."}`) and legacy plain strings (`"********"`)
/// from pre-v0.15.0 discovery_type JSONB. Legacy strings deserialize as
/// `Value { value: string }`.
#[derive(Clone, Serialize, Eq, PartialEq, Hash, ToSchema)]
#[serde(tag = "mode")]
pub enum ResolvableSecret {
    Value { value: String },
    FilePath { path: String },
}

/// Redacts the secret rather than deriving `Debug`, so *holding* one of these is enough to be
/// safe in a log line. `SnmpV3Params` and `SnmpQueryCredential` hand-write redacting impls for
/// the same reason; doing it here as well means a payload that forgets to — as
/// `UnifiQueryCredential` did, and as the Instant On payload would have — cannot leak. A file
/// path is not a secret and stays legible, which is what makes a misconfigured path debuggable.
impl std::fmt::Debug for ResolvableSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value { value } => f
                .debug_struct("Value")
                .field("value", &format_args!("******** ({} chars)", value.len()))
                .finish(),
            Self::FilePath { path } => f.debug_struct("FilePath").field("path", path).finish(),
        }
    }
}

impl<'de> Deserialize<'de> for ResolvableSecret {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match &value {
            serde_json::Value::String(s) => Ok(ResolvableSecret::Value { value: s.clone() }),
            serde_json::Value::Object(_) => {
                #[derive(Deserialize)]
                #[serde(tag = "mode")]
                enum Tagged {
                    Value { value: String },
                    FilePath { path: String },
                }
                let tagged: Tagged =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(match tagged {
                    Tagged::Value { value } => ResolvableSecret::Value { value },
                    Tagged::FilePath { path } => ResolvableSecret::FilePath { path },
                })
            }
            _ => Err(serde::de::Error::custom(
                "expected string or object for ResolvableSecret",
            )),
        }
    }
}

impl ResolvableValue {
    /// Resolve to a string value. FilePath variant reads from disk.
    pub fn resolve(&self, field_name: &str, label: &str) -> Result<String, anyhow::Error> {
        match self {
            Self::Value { value } => Ok(value.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} from {} for {}", field_name, path, label);
                std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })
            }
        }
    }

    /// Read FilePath from disk and return Value. Value variants pass through.
    pub fn resolve_to_value(&self, field_name: &str, label: &str) -> Result<Self, anyhow::Error> {
        match self {
            Self::Value { .. } => Ok(self.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} from {} for {}", field_name, path, label);
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })?;
                Ok(Self::Value { value: contents })
            }
        }
    }

    /// Resolve to a filesystem path. FilePath returns the path directly.
    /// Value writes content to a temp file (caller must hold the handle to keep it alive).
    pub fn resolve_to_path(
        &self,
        field_name: &str,
        label: &str,
    ) -> Result<(PathBuf, Option<NamedTempFile>), anyhow::Error> {
        match self {
            Self::FilePath { path } => Ok((PathBuf::from(path), None)),
            Self::Value { value } => {
                let mut tmp = NamedTempFile::new().map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create temp file for {} ({}): {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.write_all(value.as_bytes()).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to write {} to temp file for {}: {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.flush()?;
                let path = tmp.path().to_path_buf();
                Ok((path, Some(tmp)))
            }
        }
    }
}

impl ResolvableSecret {
    /// Resolve to a Secret<String>. FilePath variant reads from disk.
    pub fn resolve(
        &self,
        field_name: &str,
        label: &str,
    ) -> Result<redact::Secret<String>, anyhow::Error> {
        match self {
            Self::Value { value } => Ok(redact::Secret::from(value.clone())),
            Self::FilePath { path } => {
                tracing::info!("Read {} (********) from {} for {}", field_name, path, label);
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })?;
                Ok(redact::Secret::from(contents))
            }
        }
    }

    /// Read FilePath from disk and return Value. Value variants pass through.
    pub fn resolve_to_value(&self, field_name: &str, label: &str) -> Result<Self, anyhow::Error> {
        match self {
            Self::Value { .. } => Ok(self.clone()),
            Self::FilePath { path } => {
                tracing::info!("Read {} (********) from {} for {}", field_name, path, label);
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to read {} from {} for {}: {}",
                        field_name,
                        path,
                        label,
                        e
                    )
                })?;
                Ok(Self::Value { value: contents })
            }
        }
    }

    /// Resolve to a filesystem path. FilePath returns the path directly.
    /// Value writes content to a temp file (caller must hold the handle to keep it alive).
    pub fn resolve_to_path(
        &self,
        field_name: &str,
        label: &str,
    ) -> Result<(PathBuf, Option<NamedTempFile>), anyhow::Error> {
        match self {
            Self::FilePath { path } => Ok((PathBuf::from(path), None)),
            Self::Value { value } => {
                let mut tmp = NamedTempFile::new().map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to create temp file for {} ({}): {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.write_all(value.as_bytes()).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to write {} to temp file for {}: {}",
                        field_name,
                        label,
                        e
                    )
                })?;
                tmp.flush()?;
                let path = tmp.path().to_path_buf();
                Ok((path, Some(tmp)))
            }
        }
    }
}

// ============================================================================
// Banner display types for credential logging
// ============================================================================

/// One line in the credential banner.
pub struct BannerField {
    pub label: &'static str,
    pub value: BannerFieldValue,
}

pub enum BannerFieldValue {
    /// Non-secret inline value — show directly (e.g., port "2376", version "v2c")
    Plain(String),
    /// Long inline value — show "<inline, N chars>" instead of dumping content
    InlineSummary(usize),
    /// Inline secret — show "******** (N chars)"
    RedactedInline(usize),
    /// File path that exists — show "successfully read from /path"
    FileOk(String),
    /// File path that doesn't exist — show "failed to read from /path"
    FileFailed(String),
}

impl BannerFieldValue {
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::FileFailed(_))
    }
}

impl std::fmt::Display for BannerFieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plain(v) => write!(f, "{}", v),
            Self::InlineSummary(len) => write!(f, "<inline, {} chars>", len),
            Self::RedactedInline(len) => write!(f, "******** ({} chars)", len),
            Self::FileOk(path) => write!(f, "successfully read from {}", path),
            Self::FileFailed(path) => write!(f, "failed to read from {}", path),
        }
    }
}

impl ResolvableValue {
    pub fn banner_value(&self) -> BannerFieldValue {
        match self {
            Self::Value { value } => {
                if value.len() > 64 {
                    BannerFieldValue::InlineSummary(value.len())
                } else {
                    BannerFieldValue::Plain(value.clone())
                }
            }
            Self::FilePath { path } => {
                if Path::new(path).exists() {
                    BannerFieldValue::FileOk(path.clone())
                } else {
                    BannerFieldValue::FileFailed(path.clone())
                }
            }
        }
    }
}

impl ResolvableSecret {
    pub fn banner_value(&self) -> BannerFieldValue {
        match self {
            Self::Value { value } => BannerFieldValue::RedactedInline(value.len()),
            Self::FilePath { path } => {
                if Path::new(path).exists() {
                    BannerFieldValue::FileOk(path.clone())
                } else {
                    BannerFieldValue::FileFailed(path.clone())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snmp_cred(community: &str) -> SnmpQueryCredential {
        SnmpQueryCredential {
            version: SnmpVersion::V2c,
            community: ResolvableSecret::Value {
                value: community.to_string(),
            },
            v3: None,
        }
    }

    fn make_override(ip: IpAddr, cred_id: Uuid) -> IpOverride<SnmpQueryCredential> {
        IpOverride {
            ip,
            credential: make_snmp_cred("public"),
            credential_id: cred_id,
        }
    }

    // -- credential_ids --

    #[test]
    fn credential_ids_filters_nil_uuids() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ip_overrides: vec![
                make_override("10.0.0.1".parse().unwrap(), Uuid::nil()),
                make_override("10.0.0.2".parse().unwrap(), Uuid::new_v4()),
            ],
        };
        let ids = mapping.credential_ids();
        assert_eq!(ids.len(), 1);
        assert_ne!(ids[0], Uuid::nil());
    }

    #[test]
    fn credential_ids_deduplicates() {
        let shared_id = Uuid::new_v4();
        let mapping = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![
                make_override("10.0.0.1".parse().unwrap(), shared_id),
                make_override("10.0.0.2".parse().unwrap(), shared_id),
            ],
        };
        let ids = mapping.credential_ids();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], shared_id);
    }

    #[test]
    fn credential_ids_empty_when_no_overrides() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ip_overrides: vec![],
        };
        assert!(mapping.credential_ids().is_empty());
    }

    // -- is_enabled --

    #[test]
    fn is_enabled_default_only() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ip_overrides: vec![],
        };
        assert!(mapping.is_enabled());
    }

    #[test]
    fn is_enabled_overrides_only() {
        let mapping = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
        };
        assert!(mapping.is_enabled());
    }

    #[test]
    fn is_enabled_empty() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping::default();
        assert!(!mapping.is_enabled());
    }

    // -- get_credential_for_ip --

    #[test]
    fn get_credential_for_ip_override_match() {
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("default")),
            ip_overrides: vec![IpOverride {
                ip,
                credential: make_snmp_cred("override"),
                credential_id: Uuid::new_v4(),
            }],
        };
        let cred = mapping.get_credential_for_ip(&ip).unwrap();
        assert_eq!(
            cred.community,
            ResolvableSecret::Value {
                value: "override".to_string()
            }
        );
    }

    #[test]
    fn get_credential_for_ip_fallback_to_default() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("default")),
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
        };
        let other_ip: IpAddr = "10.0.0.99".parse().unwrap();
        let cred = mapping.get_credential_for_ip(&other_ip).unwrap();
        assert_eq!(
            cred.community,
            ResolvableSecret::Value {
                value: "default".to_string()
            }
        );
    }

    #[test]
    fn get_credential_for_ip_no_match() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
        };
        let other_ip: IpAddr = "10.0.0.99".parse().unwrap();
        assert!(mapping.get_credential_for_ip(&other_ip).is_none());
    }

    // -- is_localhost --

    #[test]
    fn is_localhost_v4() {
        let o = make_override("127.0.0.1".parse().unwrap(), Uuid::new_v4());
        assert!(o.is_localhost());
    }

    #[test]
    fn is_localhost_v6() {
        let o = make_override("::1".parse().unwrap(), Uuid::new_v4());
        assert!(o.is_localhost());
    }

    #[test]
    fn is_localhost_non_local() {
        let o = make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4());
        assert!(!o.is_localhost());
    }
}
