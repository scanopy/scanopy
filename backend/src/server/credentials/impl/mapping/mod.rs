//! Generic credential mapping for discovery dispatch.
//!
//! The mapping types define how credentials are resolved per-IP during discovery.
//! `CredentialMapping<T>` is generic over the query credential type.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use strum::EnumDiscriminants;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::shared::types::metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider};
use crate::server::shared::types::{Color, Icon};

mod resolvable;
pub use resolvable::*;

// Re-export type-specific types so external imports don't break
pub use super::types::container_proxy::ContainerProxyQueryCredential;
pub use super::types::instant_on::InstantOnQueryCredential;
pub use super::types::proxmox::ProxmoxQueryCredential;
pub use super::types::ssh::{ScriptSource, SshAuth, SshQueryCredential};
pub use super::types::unifi::{UnifiAuth, UnifiQueryCredential};
pub use super::types::wake_on_lan::WakeOnLanQueryCredential;

/// Container-runtime (Docker/Podman) socket query credential. The daemon connects via a local
/// Unix socket; `socket_path` optionally repoints it (e.g. rootless Podman at
/// `$XDG_RUNTIME_DIR/podman/podman.sock`, a non-default `DOCKER_HOST`). Blank ⇒ the daemon
/// auto-detects (bollard defaults for Docker, `resolve_podman_socket_path()` for Podman).
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default)]
pub struct ContainerSocketQueryCredential {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub socket_path: Option<super::types::paths::DaemonSocket>,
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

/// Generic credential mapping: a default credential for the site
/// plus per-IP overrides for specific hosts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct CredentialMapping<T> {
    #[serde(default)]
    pub default_credential: Option<T>,
    /// The stored credential `default_credential` came from.
    ///
    /// The accumulator this is built in is keyed by credential id — one mapping per credential —
    /// and the key used to be dropped on the way out, which left a broadcast default anonymous by
    /// the time it reached the daemon. A credential warning could then name the address it failed
    /// at but never the record to go and fix, which is the whole point of carrying it.
    ///
    /// `None` on a mapping from a server too old to send it, and on the daemon's own injected
    /// "public" fallback, which has no stored row behind it.
    #[serde(default)]
    pub default_credential_id: Option<Uuid>,
    /// The OS the credential's files and sockets were declared for, `None` when it reads nothing
    /// on the daemon. A daemon on another OS drops the mapping and warns, rather than reading a
    /// path written for a different OS.
    #[serde(default)]
    pub daemon_os: Option<super::types::paths::OsFamily>,
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
    /// The host this override was expanded from, where it came from a host assignment.
    ///
    /// A host assignment means "use this credential on this device", and it fans out to one
    /// override per address the host holds. Flattened to addresses alone, the daemon could not
    /// tell that two of them were the same device, and reported a multi-homed host's unscanned
    /// address as an untried credential even though the credential had just worked at the host's
    /// other address. Carrying the host is what lets those siblings be recognised.
    ///
    /// `None` where there is no host behind the override — an integration target names addresses
    /// directly, and so does the legacy SNMP mapping — and on a mapping from an older server.
    /// Those keep the per-address rule they have always had.
    #[serde(default)]
    pub host_id: Option<Uuid>,
    /// The MAC the server holds for this address, where the override came from a host assignment.
    ///
    /// Wake-on-LAN addresses a sleeping host by MAC: it has no IP stack running to answer at `ip`.
    /// `None` where the server has no MAC for the address, where there is no host behind the
    /// override, and on a mapping from an older server. Older daemons ignore the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac_address: Option<mac_address::MacAddress>,
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
/// None for site-level defaults and fallbacks — those don't get auto-assigned
/// to discovered hosts because they're already available site-wide.
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
// can apply (DaemonHost / Site / Hosts). It's the strum discriminant of `IntegrationTarget`.
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
    /// All hosts on the site, as a broadcast default credential. Rows written before the
    /// site rename hold `"scope":"Network"`; daemons older than
    /// `minimum_site_wire` are sent that spelling.
    #[schema(title = "Site")]
    #[serde(alias = "Network")]
    Site {
        /// Credential to use across the site.
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
            | Self::Site { credential_id }
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
            Self::Site { credential_id } => write!(f, "{credential_id}"),
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
    /// SSH: run the credential's script on the host and apply its JSON output. Both SSH
    /// transports (password and key) share this payload; the auth is discriminated in `SshAuth`.
    Ssh(SshQueryCredential),
    /// Wake-on-LAN: wake the assigned hosts before the sweep. Never probed or executed per host.
    WakeOnLan(WakeOnLanQueryCredential),
    /// Proxmox VE API token: the node's API reports every node and guest in its cluster.
    Proxmox(ProxmoxQueryCredential),
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
            CredentialQueryPayloadDiscriminants::Ssh => Self::SshKey,
            CredentialQueryPayloadDiscriminants::WakeOnLan => Self::WakeOnLan,
            CredentialQueryPayloadDiscriminants::Proxmox => Self::ProxmoxApiToken,
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
            Self::Ssh(s) => vec![s.port],
            // The packet is UDP to a broadcast address; a sleeping host has no port to find open.
            Self::WakeOnLan(_) => vec![],
            Self::Proxmox(p) => vec![p.port],
            Self::Unknown => vec![],
        }
    }

    /// Whether this credential reads anything off the daemon's own machine: a file-backed secret
    /// or value, a daemon-side script, or a socket path. Those are what `daemon_os` describes, so
    /// only these credentials are checked against the daemon's OS. Exhaustive, so a new payload
    /// cannot skip the question.
    pub fn reads_daemon_paths(&self) -> bool {
        let secret = |s: &ResolvableSecret| matches!(s, ResolvableSecret::FilePath { .. });
        let value = |v: &ResolvableValue| matches!(v, ResolvableValue::FilePath { .. });
        match self {
            Self::Snmp(s) => {
                secret(&s.community)
                    || s.v3
                        .as_ref()
                        .is_some_and(|v3| secret(&v3.auth_password) || secret(&v3.priv_password))
            }
            Self::DockerProxy(d) | Self::PodmanProxy(d) => {
                d.ssl_cert.as_ref().is_some_and(value)
                    || d.ssl_key.as_ref().is_some_and(secret)
                    || d.ssl_chain.as_ref().is_some_and(value)
            }
            Self::DockerSocket(c) | Self::PodmanSocket(c) => {
                c.socket_path.as_ref().is_some_and(|s| !s.is_blank())
            }
            Self::UnifiController(u) => match &u.auth {
                UnifiAuth::ApiKey { api_key } => secret(api_key),
                UnifiAuth::LocalAdmin { password, .. } => secret(password),
            },
            Self::InstantOn(i) => secret(&i.password),
            Self::Gnmi(g) => secret(&g.password),
            Self::Ssh(s) => {
                matches!(s.script, ScriptSource::DaemonFile { .. })
                    || match &s.auth {
                        SshAuth::Password { password } => secret(password),
                        SshAuth::PrivateKey {
                            private_key,
                            passphrase,
                        } => secret(private_key) || passphrase.as_ref().is_some_and(secret),
                    }
            }
            Self::WakeOnLan(w) => w.secure_on_password.as_ref().is_some_and(secret),
            Self::Proxmox(p) => secret(&p.token_secret),
            Self::Unknown => false,
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
            Self::Ssh(_) => "SSH script",
            Self::WakeOnLan(_) => "Wake-on-LAN",
            Self::Proxmox(_) => "Proxmox VE API connection",
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
            Self::Ssh => "SSH",
            Self::WakeOnLan => "Wake-on-LAN",
            Self::Proxmox => "Proxmox VE API",
            // Reachable only from a warning written by a newer binary than this one.
            Self::Unknown => "unrecognised",
        }
    }
}

impl CredentialQueryPayload {
    /// Resolve all FilePath fields to Value by reading from disk,
    /// then validate PEM contents for fields that require it.
    pub fn resolve_file_paths(&self) -> Result<Self, anyhow::Error> {
        use crate::server::shared::types::field_definition::InlineFormat;

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
            Self::Ssh(s) => {
                let auth = match &s.auth {
                    SshAuth::Password { password } => SshAuth::Password {
                        password: password.resolve_to_value("password", label)?,
                    },
                    SshAuth::PrivateKey {
                        private_key,
                        passphrase,
                    } => {
                        let private_key = private_key.resolve_to_value("private_key", label)?;
                        if let ResolvableSecret::Value { value } = &private_key {
                            InlineFormat::SshPrivateKey.validate(value, "SSH Private Key")?;
                        }
                        SshAuth::PrivateKey {
                            private_key,
                            passphrase: passphrase
                                .as_ref()
                                .map(|p| p.resolve_to_value("passphrase", label))
                                .transpose()?,
                        }
                    }
                };
                Ok(Self::Ssh(SshQueryCredential {
                    auth,
                    script: s.script.resolve_daemon_file()?,
                    ..s.clone()
                }))
            }
            Self::WakeOnLan(w) => {
                let secure_on_password = w
                    .secure_on_password
                    .as_ref()
                    .map(|p| p.resolve_to_value("secure_on_password", label))
                    .transpose()?;
                if let Some(ResolvableSecret::Value { value }) = &secure_on_password {
                    InlineFormat::MacAddress.validate(value, "SecureOn Password")?;
                }
                Ok(Self::WakeOnLan(WakeOnLanQueryCredential {
                    secure_on_password,
                    ..w.clone()
                }))
            }
            // No format validation: a token secret is an opaque string.
            Self::Proxmox(p) => Ok(Self::Proxmox(ProxmoxQueryCredential {
                token_secret: p.token_secret.resolve_to_value("token_secret", label)?,
                ..p.clone()
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
            Self::Ssh(s) => s.banner_lines(),
            Self::WakeOnLan(w) => w.banner_lines(),
            Self::Proxmox(p) => p.banner_lines(),
            Self::Unknown => vec![],
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
            host_id: None,
            mac_address: None,
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
            ..Default::default()
        };
        let ids = mapping.credential_ids();
        assert_eq!(ids.len(), 1);
        assert_ne!(ids[0], Uuid::nil());
    }

    #[test]
    fn credential_ids_deduplicates() {
        let shared_id = Uuid::new_v4();
        let mapping = CredentialMapping {
            ip_overrides: vec![
                make_override("10.0.0.1".parse().unwrap(), shared_id),
                make_override("10.0.0.2".parse().unwrap(), shared_id),
            ],
            ..Default::default()
        };
        let ids = mapping.credential_ids();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], shared_id);
    }

    #[test]
    fn credential_ids_empty_when_no_overrides() {
        let mapping: CredentialMapping<SnmpQueryCredential> = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ..Default::default()
        };
        assert!(mapping.credential_ids().is_empty());
    }

    // -- is_enabled --

    #[test]
    fn is_enabled_default_only() {
        let mapping = CredentialMapping {
            default_credential: Some(make_snmp_cred("public")),
            ..Default::default()
        };
        assert!(mapping.is_enabled());
    }

    #[test]
    fn is_enabled_overrides_only() {
        let mapping = CredentialMapping {
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
            ..Default::default()
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
                host_id: None,
                mac_address: None,
            }],
            ..Default::default()
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
            ..Default::default()
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
            ip_overrides: vec![make_override("10.0.0.1".parse().unwrap(), Uuid::new_v4())],
            ..Default::default()
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

    /// What a daemon from 0.17.3 to 0.17.18 does with an SSH or Wake-on-LAN credential, should one
    /// get past the version filter: a credential type it does not know, carrying fields it does not
    /// know, lands on `Unknown` instead of failing the whole discovery request. The forward-compat
    /// suite only skews a bare `{"type": ...}`, which says nothing about the fields.
    #[test]
    fn an_unknown_type_with_fields_parses_to_unknown() {
        let future = serde_json::json!({
            "type": "SomethingNewer",
            "port": 22,
            "auth": {"mode": "Password", "password": {"mode": "Value", "value": "x"}},
            "script": "uname -a",
        });
        let mapping: CredentialMapping<CredentialQueryPayload> =
            serde_json::from_value(serde_json::json!({
                "default_credential": null,
                "ip_overrides": [{"ip": "10.0.0.1", "credential": future, "mac_address": "3c:ec:ef:12:34:56"}],
            }))
            .expect("an unknown credential type must not fail the mapping");
        assert_eq!(
            mapping.ip_overrides[0].credential,
            CredentialQueryPayload::Unknown
        );
    }
}
