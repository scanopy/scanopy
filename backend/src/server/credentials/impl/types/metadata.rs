use std::borrow::Cow;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::{
    services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt},
    shared::types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, MetadataProvider, TypeMetadata},
    },
};

use super::{
    CredentialType, CredentialTypeDiscriminants, OsFamily, ScriptSource, SecretValue,
    default_docker_port, default_gnmi_port, default_ssh_port, default_ssh_timeout_seconds,
    default_unifi_port, default_unifi_site, default_wake_on_lan_port,
    default_wake_on_lan_wait_seconds,
};

/// Category grouping for credential types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, IntoStaticStr, ToSchema, PartialEq, Eq)]
pub enum CredentialCategory {
    /// Network monitoring protocols (SNMP, NetFlow, sFlow)
    #[strum(serialize = "Network Monitoring")]
    NetworkMonitoring,
    /// Container and virtualization platforms (Docker, vSphere, ESXi)
    #[strum(serialize = "Container & Virtualization")]
    ContainerVirtualization,
    /// Management controllers that hold an inventory of the devices they have adopted
    /// (UniFi, Omada, Meraki, Aruba Central). Distinct from `NetworkMonitoring`, which is
    /// for polling protocols — a controller is an API that reports someone else's devices.
    #[strum(serialize = "Network Controllers")]
    NetworkController,
    /// Access to the host itself: logging in to run a script (SSH), or powering it on
    /// (Wake-on-LAN).
    #[strum(serialize = "Host Management")]
    HostManagement,
}

/// Release maturity of a credential type's integration.
///
/// Additive and exhaustive: a new credential variant will not compile until it declares its
/// stability, and every existing type is `Stable` by explicit arm rather than by wildcard, so
/// promoting an integration is a one-line reviewable change rather than a deletion nobody
/// notices. This is presentation metadata about the *code*, like `minimum_daemon_version` —
/// it is never stored on a credential row, so it carries no deploy-coexistence obligation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, IntoStaticStr, ToSchema, PartialEq, Eq)]
pub enum CredentialStability {
    /// Generally available.
    Stable,
    /// Shipped for validation. Data collection may be incomplete and the credential's field
    /// shape may change in a future release. Usable, but clearly marked in the UI.
    Beta,
}

/// `Beta < Stable`. Not derived: declaration order puts `Stable` first, which would order
/// these the other way round.
impl PartialOrd for CredentialStability {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        Some(match (self, other) {
            (Self::Beta, Self::Stable) => Ordering::Less,
            (Self::Stable, Self::Beta) => Ordering::Greater,
            (Self::Beta, Self::Beta) | (Self::Stable, Self::Stable) => Ordering::Equal,
        })
    }
}

/// Whether the vendor publishes and supports the API a credential type talks to.
///
/// Deliberately *not* folded into [`CredentialStability`], because the two describe different
/// things and change independently. Stability is about our own maturity and is meant to be retired
/// by promotion to `Stable`; an undocumented upstream is a permanent property of the vendor's API
/// that our promotion does not change. Collapsing them would force an integration built on a
/// reverse-engineered API to sit in `Beta` forever to keep the warning — or to reach `Stable` with
/// the warning silently dropped. UniFi is the proof that both combinations are real: it is
/// `Stable` and `Undocumented` today.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, IntoStaticStr, ToSchema, PartialEq, Eq)]
pub enum UpstreamSupport {
    /// The vendor publishes and supports this API.
    Vendor,
    /// Reverse-engineered from the vendor's own client. There is no published contract, so it can
    /// change or stop working without notice.
    Undocumented,
}

/// A credential assigned to a host, optionally limited to specific ip_addresses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct CredentialAssignment {
    /// The credential this entity refers to.
    pub credential_id: Uuid,
    /// Interface IDs to limit this credential to. None = all host ip_addresses.
    #[serde(default, alias = "interface_ids")]
    #[schema(required)]
    pub ip_address_ids: Option<Vec<Uuid>>,
}

/// Host-keyed mirror of [`CredentialAssignment`]: a host this credential is
/// assigned to, optionally limited to specific ip_addresses. Hydrated onto a
/// credential from the `host_credentials` junction (PerHost scope).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct CredentialHostAssignment {
    /// The host this entity belongs to.
    pub host_id: Uuid,
    /// IP address IDs to limit this credential to on the host. None = all host ip_addresses.
    #[serde(default)]
    #[schema(required)]
    pub ip_address_ids: Option<Vec<Uuid>>,
}

impl CredentialTypeDiscriminants {
    /// Create a `CredentialType` instance with default field values for this variant.
    /// Used by `generate-fixtures` and anywhere variant iteration is needed.
    pub fn to_credential_type(&self) -> CredentialType {
        match self {
            Self::SnmpV1 => CredentialType::SnmpV1 {
                community: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            Self::SnmpV2c => CredentialType::SnmpV2c {
                community: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            Self::Gnmi => CredentialType::Gnmi {
                port: default_gnmi_port(),
                username: String::new(),
                password: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
                tls: false,
                skip_verify: false,
            },
            Self::SnmpV3 => CredentialType::SnmpV3 {
                security_name: String::new(),
                auth_protocol: super::SnmpV3AuthProtocol::default(),
                auth_password: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
                priv_protocol: super::SnmpV3PrivProtocol::default(),
                priv_password: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
                context_name: None,
            },
            Self::DockerProxy => CredentialType::DockerProxy {
                port: default_docker_port(),
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
            Self::DockerSocket => CredentialType::DockerSocket { socket_path: None },
            Self::PodmanProxy => CredentialType::PodmanProxy {
                port: default_docker_port(),
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
            Self::PodmanSocket => CredentialType::PodmanSocket { socket_path: None },
            Self::UnifiApiKey => CredentialType::UnifiApiKey {
                port: default_unifi_port(),
                site: default_unifi_site(),
                api_key: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            Self::UnifiLocalAdmin => CredentialType::UnifiLocalAdmin {
                port: default_unifi_port(),
                site: default_unifi_site(),
                username: String::new(),
                password: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            Self::InstantOnAccount => CredentialType::InstantOnAccount {
                username: String::new(),
                password: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
                site: None,
            },
            Self::SshPassword => CredentialType::SshPassword {
                port: default_ssh_port(),
                username: String::new(),
                password: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
                // The form opens on the default mode: a file already on the scanned host.
                script: ScriptSource::HostFile {
                    path: String::new().into(),
                },
                target_os: OsFamily::Unix,
                timeout_seconds: default_ssh_timeout_seconds(),
                host_key_fingerprint: None,
            },
            Self::SshKey => CredentialType::SshKey {
                port: default_ssh_port(),
                username: String::new(),
                private_key: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
                passphrase: None,
                // The form opens on the default mode: a file already on the scanned host.
                script: ScriptSource::HostFile {
                    path: String::new().into(),
                },
                target_os: OsFamily::Unix,
                timeout_seconds: default_ssh_timeout_seconds(),
                host_key_fingerprint: None,
            },
            Self::WakeOnLan => CredentialType::WakeOnLan {
                port: default_wake_on_lan_port(),
                wait_seconds: default_wake_on_lan_wait_seconds(),
                broadcast_address: None,
                secure_on_password: None,
            },
        }
    }

    /// Whether the vendor publishes the API behind this credential type. Exhaustive, so a new
    /// credential type cannot compile without saying which it is.
    pub fn upstream_support(&self) -> UpstreamSupport {
        match self {
            // Standard protocol, or the vendor's own documented API.
            Self::SnmpV1
            | Self::SnmpV2c
            | Self::SnmpV3
            | Self::Gnmi
            | Self::DockerProxy
            | Self::DockerSocket
            | Self::PodmanProxy
            | Self::PodmanSocket
            // Standard protocols: SSH (RFC 4253) and the AMD Magic Packet format.
            | Self::SshPassword
            | Self::SshKey
            | Self::WakeOnLan => UpstreamSupport::Vendor,
            // Both UniFi transports read `/proxy/network/api/s/<site>/stat/device`, the legacy
            // Network API, not Ubiquiti's documented Integration API (`.../integration/v1/...`,
            // added with v9 API keys). Undocumented regardless of which transport authenticates.
            Self::UnifiApiKey | Self::UnifiLocalAdmin => UpstreamSupport::Undocumented,
            // HPE publishes APIs for Aruba Central, Instant AOS-8 and ArubaOS-Switch, but none
            // for Instant On; this is reverse-engineered from the portal's own web client.
            Self::InstantOnAccount => UpstreamSupport::Undocumented,
        }
    }
}

impl HasId for CredentialTypeDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for CredentialTypeDiscriminants {
    fn color(&self) -> Color {
        self.integration().color()
    }
    fn icon(&self) -> Icon {
        self.integration().icon()
    }
}

impl CredentialTypeDiscriminants {
    /// Display name for this credential transport (e.g. "Docker Socket").
    pub(crate) fn display_name(&self) -> &'static str {
        match self {
            Self::SnmpV1 => "SNMP v1",
            Self::SnmpV2c => "SNMP v2c",
            Self::SnmpV3 => "SNMP v3",
            Self::Gnmi => "gNMI",
            Self::DockerProxy => "Docker Proxy",
            Self::DockerSocket => "Docker Socket",
            Self::PodmanProxy => "Podman Proxy",
            Self::PodmanSocket => "Podman Socket",
            Self::UnifiApiKey => "UniFi API Key",
            Self::UnifiLocalAdmin => "UniFi Local Admin",
            Self::InstantOnAccount => "Instant On Portal Account",
            Self::SshPassword => "SSH Password",
            Self::SshKey => "SSH Key",
            Self::WakeOnLan => "Wake-on-LAN",
        }
    }

    /// Transport-specific note appended after the canonical discovery text. This is
    /// the only per-transport prose; the shared "what's discovered" stem lives in
    /// [`CredentialIntegration::discovers`](super::CredentialIntegration::discovers).
    ///
    /// One sentence saying **how it connects**, plus a compatibility caveat only when that
    /// caveat decides which transport the user should pick. See the writing guidance on
    /// [`CredentialIntegration::discovers`](super::CredentialIntegration::discovers) — in particular, credential setup
    /// belongs in field help text, not here.
    pub(crate) fn transport_note(&self) -> &'static str {
        match self {
            Self::SnmpV1 => "Uses SNMPv1.",
            Self::SnmpV2c => "Uses SNMPv2c.",
            Self::SnmpV3 => "Uses SNMPv3.",
            Self::Gnmi => "Connects over gRPC; username and password travel as request metadata.",
            Self::DockerProxy | Self::PodmanProxy => "Connects over TCP, optionally with TLS.",
            Self::DockerSocket | Self::PodmanSocket => "Connects via the daemon's local socket.",
            Self::UnifiApiKey => {
                "Connects with a controller API key. Requires UniFi OS; the legacy self-hosted Network Application does not support API keys."
            }
            Self::UnifiLocalAdmin => {
                "Connects with a local admin account. Works with every controller, including the legacy self-hosted Network Application."
            }
            Self::InstantOnAccount => {
                "Connects to the Instant On cloud portal with a site account."
            }
            Self::SshPassword => "Connects over SSH with a username and password.",
            Self::SshKey => "Connects over SSH with a private key.",
            Self::WakeOnLan => {
                "Sends a magic packet over UDP. The daemon must be on the host's network segment, or the router must forward directed broadcasts or relay the packet."
            }
        }
    }

    /// Short transport label within an integration (e.g. "Socket", "Proxy", "v2c").
    pub(crate) fn transport_label(&self) -> &'static str {
        match self {
            Self::SnmpV1 => "v1",
            Self::SnmpV2c => "v2c",
            Self::SnmpV3 => "v3",
            Self::Gnmi => "gNMI",
            Self::DockerProxy | Self::PodmanProxy => "Proxy",
            Self::DockerSocket | Self::PodmanSocket => "Socket",
            Self::UnifiApiKey => "API Key",
            Self::UnifiLocalAdmin => "Local Admin",
            Self::InstantOnAccount => "Portal Account",
            Self::SshPassword => "Password",
            Self::SshKey => "Key",
            Self::WakeOnLan => "Magic Packet",
        }
    }

    /// Full credential description shown in the wizard and `credential-types.json`:
    /// the canonical discovery text plus the transport note. Derived, never
    /// hand-written per transport, so the two cannot drift.
    pub(crate) fn full_description(&self) -> String {
        format!(
            "{} {}",
            self.integration().discovers(),
            self.transport_note()
        )
    }

    fn category_str(&self) -> &'static str {
        self.integration().credential_category().into()
    }

    /// Minimum daemon version that can safely receive credential mappings of this
    /// type over the server→daemon wire. Exhaustive (no wildcard): a new credential
    /// variant will not compile until it declares its floor. This single declaration
    /// drives server-side dispatch filtering (never send a mapping an older daemon
    /// can't deserialize), assignment-time rejection, and the UI compatibility gate.
    ///
    /// Gated on the 7-way `CredentialType` discriminant, NOT the collapsed
    /// `CredentialQueryPayload` wire tag: SnmpV1/V3 carry a higher floor than SnmpV2c
    /// despite all three sharing the single `Snmp` wire variant.
    ///
    /// Distinct from the global [`DaemonVersionPolicy::minimum_supported`] floor —
    /// same `semver` comparison, different purpose.
    pub fn minimum_daemon_version(&self) -> semver::Version {
        match self {
            // Unified credential-wire floor. Older daemons ignore `credential_mappings`
            // via #[serde(default)], so filtering these out is harmless.
            Self::SnmpV2c | Self::DockerProxy | Self::DockerSocket => {
                semver::Version::new(0, 16, 2)
            }
            // SnmpV1/SnmpV3 inner `SnmpVersion` values shipped in 0.17.0.
            Self::SnmpV1 | Self::SnmpV3 => semver::Version::new(0, 17, 0),
            // Podman variants shipped in 0.17.2.
            Self::PodmanProxy | Self::PodmanSocket => semver::Version::new(0, 17, 2),
            // UniFi variants ship in 0.17.7.
            Self::UnifiApiKey | Self::UnifiLocalAdmin => semver::Version::new(0, 17, 7),
            // gNMI ships in 0.17.16 (scanopy#690). An older daemon that receives this mapping
            // deserializes the unknown wire tag to `Unknown` and skips it.
            Self::Gnmi => semver::Version::new(0, 17, 16),
            // Instant On ships in 0.17.11.
            Self::InstantOnAccount => semver::Version::new(0, 17, 11),
            // SSH and Wake-on-LAN ship in 0.17.19. Daemons from 0.17.3 would parse them to
            // `Unknown` and skip them; the floor keeps them off older daemons entirely.
            Self::SshPassword | Self::SshKey | Self::WakeOnLan => semver::Version::new(0, 17, 19),
        }
    }

    /// Release maturity of this credential type's integration. See [`CredentialStability`].
    ///
    /// Exhaustive with no wildcard: adding a credential type forces an explicit maturity
    /// declaration rather than defaulting a brand-new, unvalidated integration to `Stable`.
    pub fn stability(&self) -> CredentialStability {
        match self {
            Self::SnmpV1
            | Self::SnmpV2c
            | Self::SnmpV3
            | Self::DockerProxy
            | Self::DockerSocket
            | Self::PodmanProxy
            | Self::PodmanSocket
            | Self::UnifiApiKey
            | Self::UnifiLocalAdmin => CredentialStability::Stable,
            // New; validated against ArcOS and DNOS over plaintext only. TLS is wired but not
            // yet exercised against a TLS-enabled device, and `skip_verify` is not yet
            // supported, so the field shape may still move.
            Self::Gnmi => CredentialStability::Beta,
            // New and validated against one operator's 1960s only; the field shape may still move
            // once other Instant On models' payloads are seen.
            Self::InstantOnAccount => CredentialStability::Beta,
            // New; the script contract and host-key handling may still move.
            Self::SshPassword | Self::SshKey | Self::WakeOnLan => CredentialStability::Beta,
        }
    }

    /// Whether a daemon at `daemon_version` can safely receive credential mappings of
    /// this type. A missing version is treated conservatively: only types at the
    /// 0.16.2 unified-wire floor are considered compatible. Shared by server-side
    /// dispatch filtering and the UI compatibility gate so the two never diverge.
    pub fn compatible_with_daemon(&self, daemon_version: Option<&semver::Version>) -> bool {
        match daemon_version {
            Some(v) => *v >= self.minimum_daemon_version(),
            None => self.minimum_daemon_version() <= semver::Version::new(0, 16, 2),
        }
    }

    fn metadata_json(&self) -> serde_json::Value {
        let ct = self.to_credential_type();
        let integration = self.integration();
        let service = integration.service();
        serde_json::json!({
            "fields": ct.field_definitions(),
            // Which fields depend on the daemon's and the scanned host's OS, for placing the pickers.
            "os_fields": ct.os_fields(),
            // The frontend derives "daemon-host-only" (former `is_local_auto`) from `targets`.
            "targets": ct.targets(),
            "requires_config": ct.requires_config(),
            "single_endpoint_per_host": ct.single_endpoint_per_host(),
            // Minimum daemon version that can receive this type (message-only on the
            // frontend; the actual gate uses the server-computed compat flag).
            "minimum_daemon_version": self.minimum_daemon_version().to_string(),
            // Release maturity. The frontend renders a "Beta" tag; it is not a gate.
            "stability": self.stability(),
            // Whether the vendor publishes this API. Orthogonal to `stability` — an integration
            // can be fully validated and still be riding an undocumented endpoint.
            "upstream_support": self.upstream_support(),
            // Guide for this credential's integration, path-only. The form links it for every
            // type rather than the two it used to hardcode.
            "docs_path": integration.docs_path(),
            // Keys `credential-integrations.json`; the picker groups types on it.
            "integration": integration,
            "associated_service": ServiceDefinition::name(&*service),
            "has_logo": service.has_logo(),
            "logo_ext": integration.logo_ext(),
            "logo_needs_white_background": service.logo_needs_white_background(),
        })
    }
}

// Credential types build their `TypeMetadata` directly (rather than via the
// `TypeMetadataProvider` blanket) because their description is composed at build
// time from the centralized integration text — see [`full_description`].
impl MetadataProvider<TypeMetadata> for CredentialTypeDiscriminants {
    fn to_metadata(&self) -> TypeMetadata {
        TypeMetadata {
            id: self.id(),
            name: Some(self.display_name()),
            description: Some(Cow::Owned(self.full_description())),
            category: Some(self.category_str()),
            icon: Some(self.icon()),
            color: self.color(),
            metadata: Some(self.metadata_json()),
        }
    }
}
