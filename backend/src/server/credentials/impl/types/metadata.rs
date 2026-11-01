use std::borrow::Cow;

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::{
    billing::types::{base::BillingPlan, features::Feature},
    credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants,
    daemons::r#impl::version::last_without_credential_wire,
    services::r#impl::definitions::ServiceDefinition,
    shared::types::{
        Color, Icon, Stability,
        metadata::{EntityMetadataProvider, HasId, MetadataProvider, TypeMetadata},
    },
};

use super::{
    CredentialType, CredentialTypeDiscriminants, OsFamily, ScriptSource, SecretValue,
    default_docker_port, default_gnmi_port, default_proxmox_port, default_ssh_port,
    default_ssh_timeout_seconds, default_unifi_port, default_unifi_site, default_wake_on_lan_port,
    default_wake_on_lan_wait_seconds,
};

/// Category grouping for credential types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, IntoStaticStr, ToSchema, PartialEq, Eq)]
pub enum CredentialCategory {
    /// Network monitoring protocols (SNMP, NetFlow, sFlow)
    #[strum(serialize = "Network Monitoring")]
    NetworkMonitoring,
    /// Container runtimes (Docker, Podman)
    #[strum(serialize = "Containers")]
    Containers,
    /// Hypervisors (Proxmox VE, libvirt)
    #[strum(serialize = "Virtualization")]
    Virtualization,
    /// Management controllers that hold an inventory of the devices they have adopted
    /// (UniFi, Omada, Meraki, Aruba Central). Distinct from `NetworkMonitoring`, which is
    /// for polling protocols — a controller is an API that reports someone else's devices.
    #[strum(serialize = "Network Controllers")]
    NetworkController,
    /// Access to the host itself: logging in to run a script (SSH), or powering it on
    /// (Wake-on-LAN).
    #[strum(serialize = "Host Management")]
    HostManagement,
    /// Accounts other credentials sign in with (SSH logins), assigned nowhere themselves.
    #[strum(serialize = "Logins")]
    Logins,
}

/// Whether the vendor publishes and supports the API a credential type talks to.
///
/// Deliberately *not* folded into [`Stability`], because the two describe different
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

/// How a credential type reaches its host, and which service owns the port it goes through
/// ([`CredentialTypeDiscriminants::reach`]). The dispatched payload states the same with its port
/// values ([`CredentialReach`](crate::server::credentials::r#impl::mapping::CredentialReach)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachDeclaration {
    /// Its own service listens on the TCP port held in its `field`: gNMI, UniFi, Proxmox VE, the
    /// Docker/Podman API proxy.
    OwnPortField { field: &'static str },
    /// Its own service listens on these fixed UDP ports: SNMP.
    FixedUdpPorts(&'static [u16]),
    /// Signs in through a login it names, whose service owns the port. Its own service owns none:
    /// SSH Script, libvirt.
    ThroughLogin,
    /// A login: the port in its `field` is the transport the credentials that name it sign in
    /// through. SSH login (password or key).
    LoginTransport { field: &'static str },
    /// Reaches the host without a port on it: a local socket, a cloud portal, a broadcast.
    NoPort,
}

/// What a credential is for: run against hosts by an integration, or a login other credentials
/// use to reach a host.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, IntoStaticStr, ToSchema, PartialEq, Eq)]
pub enum CredentialRole {
    /// Assigned to hosts or sites and sent to daemons, which run its integration.
    Dispatched,
    /// Who the daemon is on a host (an SSH account and its secret). Never assigned or sent on its
    /// own: a dispatched credential names it as its login, and the daemon receives it inside that
    /// credential.
    Login,
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
            Self::SnmpCommunity => CredentialType::SnmpCommunity {
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
                security: super::SnmpV3Security::AuthPriv {
                    auth_protocol: super::SnmpV3AuthProtocol::default(),
                    auth_password: SecretValue::Inline {
                        value: SecretString::from(String::new()),
                    },
                    priv_protocol: super::SnmpV3PrivProtocol::default(),
                    priv_password: SecretValue::Inline {
                        value: SecretString::from(String::new()),
                    },
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
                host_key_fingerprint: None,
            },
            Self::SshKey => CredentialType::SshKey {
                port: default_ssh_port(),
                username: String::new(),
                private_key: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
                passphrase: None,
                host_key_fingerprint: None,
            },
            Self::SshScript => CredentialType::SshScript {
                // The form opens on the default mode: a file already on the scanned host.
                script: ScriptSource::HostFile {
                    path: String::new().into(),
                },
                target_os: OsFamily::Unix,
                timeout_seconds: default_ssh_timeout_seconds(),
            },
            Self::Libvirt => CredentialType::Libvirt {
                connection_uri: super::libvirt::default_libvirt_uri(),
            },
            Self::WakeOnLan => CredentialType::WakeOnLan {
                port: default_wake_on_lan_port(),
                wait_seconds: default_wake_on_lan_wait_seconds(),
                broadcast_address: None,
                secure_on_password: None,
            },
            Self::ProxmoxApiToken => CredentialType::ProxmoxApiToken {
                port: default_proxmox_port(),
                token_id: String::new(),
                token_secret: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
        }
    }

    /// Whether the vendor publishes the API behind this credential type. Exhaustive, so a new
    /// credential type cannot compile without saying which it is.
    pub fn upstream_support(&self) -> UpstreamSupport {
        match self {
            // Standard protocol, or the vendor's own documented API.
            Self::SnmpCommunity
            | Self::SnmpV3
            | Self::Gnmi
            | Self::DockerProxy
            | Self::DockerSocket
            | Self::PodmanProxy
            | Self::PodmanSocket
            // Standard protocols: SSH (RFC 4253) and the AMD Magic Packet format.
            | Self::SshPassword
            | Self::SshKey
            | Self::SshScript
            // libvirt's `virsh` and its output formats are documented (libvirt.org/manpages).
            | Self::Libvirt
            | Self::WakeOnLan
            // The Proxmox VE API is documented (pve.proxmox.com/pve-docs/api-viewer).
            | Self::ProxmoxApiToken => UpstreamSupport::Vendor,
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
        match self.role() {
            CredentialRole::Login => Icon::UserKey,
            CredentialRole::Dispatched => self.integration().icon(),
        }
    }
}

impl CredentialTypeDiscriminants {
    /// Display name for this credential transport (e.g. "Docker Socket").
    pub(crate) fn display_name(&self) -> &'static str {
        match self {
            Self::SnmpCommunity => "SNMP v1/v2c",
            Self::SnmpV3 => "SNMP v3",
            Self::Gnmi => "gNMI",
            Self::DockerProxy => "Docker Proxy",
            Self::DockerSocket => "Docker Socket",
            Self::PodmanProxy => "Podman Proxy",
            Self::PodmanSocket => "Podman Socket",
            Self::UnifiApiKey => "UniFi API Key",
            Self::UnifiLocalAdmin => "UniFi Local Admin",
            Self::InstantOnAccount => "Instant On Portal Account",
            Self::SshPassword => "SSH Login (Password)",
            Self::SshKey => "SSH Login (Key)",
            Self::SshScript => "SSH Script",
            Self::Libvirt => "libvirt",
            Self::WakeOnLan => "Wake-on-LAN",
            Self::ProxmoxApiToken => "Proxmox VE API Token",
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
            Self::SnmpCommunity => {
                "Uses SNMPv2c, and SNMPv1 for a device that does not answer v2c."
            }
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
            Self::SshPassword => "A username and password other SSH credentials log in with.",
            Self::SshKey => "A private key other SSH credentials log in with.",
            Self::SshScript => "Connects over SSH with an SSH login.",
            Self::Libvirt => "Runs virsh over SSH, logged in with an SSH login.",
            Self::WakeOnLan => {
                "Sends a magic packet over UDP. The daemon must be on the host's network segment, or the router must forward directed broadcasts or relay the packet."
            }
            Self::ProxmoxApiToken => "Connects to a node's API over HTTPS with an API token.",
        }
    }

    /// Short transport label within an integration (e.g. "Socket", "Proxy", "v3").
    pub(crate) fn transport_label(&self) -> &'static str {
        match self {
            Self::SnmpCommunity => "v1/v2c",
            Self::SnmpV3 => "v3",
            Self::Gnmi => "gNMI",
            Self::DockerProxy | Self::PodmanProxy => "Proxy",
            Self::DockerSocket | Self::PodmanSocket => "Socket",
            Self::UnifiApiKey => "API Key",
            Self::UnifiLocalAdmin => "Local Admin",
            Self::InstantOnAccount => "Portal Account",
            Self::SshPassword => "Login (Password)",
            Self::SshKey => "Login (Key)",
            Self::SshScript => "Script",
            Self::Libvirt => "virsh over SSH",
            Self::WakeOnLan => "Magic Packet",
            Self::ProxmoxApiToken => "API Token",
        }
    }

    /// Full credential description shown in the wizard and `credential-types.json`:
    /// the canonical discovery text plus the transport note. Derived, never
    /// hand-written per transport, so the two cannot drift.
    pub(crate) fn full_description(&self) -> String {
        match self.role() {
            // A login discovers nothing itself: the credentials that run over it do.
            CredentialRole::Login => self.transport_note().to_string(),
            CredentialRole::Dispatched => format!(
                "{} {}",
                self.integration().discovers(),
                self.transport_note()
            ),
        }
    }

    fn category_str(&self) -> &'static str {
        match self.role() {
            CredentialRole::Login => CredentialCategory::Logins.into(),
            CredentialRole::Dispatched => self.integration().credential_category().into(),
        }
    }

    /// The last daemon release that cannot parse credential mappings of this type over the
    /// server→daemon wire; any newer daemon can. Exhaustive (no wildcard): a new credential
    /// variant will not compile until it declares its gate. Read by the `ParsesCredentialType`
    /// capability (`credentials/service/capability`), which withholds the whole credential from a
    /// daemon at or below it, and by the UI's compatibility hint.
    ///
    /// Only for parsing the type at all. Options or targets within a type that need a newer daemon
    /// are capabilities of their own, which withhold just that part (see `UsesSnmpV3SecurityOptions`).
    ///
    /// Gated on the `CredentialType` discriminant, NOT the collapsed `CredentialQueryPayload` wire
    /// tag: SnmpCommunity and SnmpV3 carry different gates despite sharing the `Snmp` wire variant.
    pub fn last_unsupported_daemon_version(&self) -> semver::Version {
        match self {
            // The unified credential wire shipped in 0.16.2. Older daemons ignore
            // `credential_mappings` via #[serde(default)], so filtering these out is harmless.
            // SnmpCommunity goes on that wire as v2c, so an older daemon still uses it, over v2c only.
            Self::SnmpCommunity | Self::DockerProxy | Self::DockerSocket => {
                last_without_credential_wire()
            }
            // The SnmpV3 value of the inner `SnmpVersion` shipped in 0.17.0, as
            // AuthPriv over SHA and AES. The security levels and MD5 that 0.18.1 added are the
            // `UsesSnmpV3SecurityOptions` capability, so an AuthPriv credential still reaches the
            // daemons that can run it.
            Self::SnmpV3 => semver::Version::new(0, 16, 2),
            // Podman variants shipped in 0.17.2.
            Self::PodmanProxy | Self::PodmanSocket => semver::Version::new(0, 17, 1),
            // UniFi variants shipped in 0.17.7.
            Self::UnifiApiKey | Self::UnifiLocalAdmin => semver::Version::new(0, 17, 6),
            // gNMI shipped in 0.17.16 (scanopy#690). An older daemon that receives this mapping
            // deserializes the unknown wire tag to `Unknown` and skips it.
            Self::Gnmi => semver::Version::new(0, 17, 15),
            // Instant On shipped in 0.17.11.
            Self::InstantOnAccount => semver::Version::new(0, 17, 10),
            // SSH and Wake-on-LAN shipped in 0.17.19. Daemons from 0.17.3 would parse them to
            // `Unknown` and skip them; the gate keeps them off older daemons entirely.
            Self::SshPassword | Self::SshKey | Self::SshScript | Self::WakeOnLan => {
                semver::Version::new(0, 17, 18)
            }
            // Proxmox shipped in 0.17.21. That daemon is also the first whose ServerPoll
            // `create_host` returns the server's child ids, which guest linking depends on.
            Self::ProxmoxApiToken => semver::Version::new(0, 17, 20),
            // libvirt ships after 0.19.5.
            Self::Libvirt => semver::Version::new(0, 19, 5),
        }
    }

    /// Release maturity of this credential type's integration. See [`Stability`].
    ///
    /// Exhaustive with no wildcard: adding a credential type forces an explicit maturity
    /// declaration rather than defaulting a brand-new, unvalidated integration to `Stable`.
    pub fn stability(&self) -> Stability {
        match self {
            Self::SnmpCommunity
            | Self::SnmpV3
            | Self::DockerProxy
            | Self::DockerSocket
            | Self::PodmanProxy
            | Self::PodmanSocket
            | Self::UnifiApiKey
            | Self::UnifiLocalAdmin
            | Self::SshPassword
            | Self::SshKey
            | Self::SshScript
            | Self::WakeOnLan => Stability::Stable,
            // New; validated against ArcOS and DNOS over plaintext only. TLS is wired but not
            // yet exercised against a TLS-enabled device, and `skip_verify` is not yet
            // supported, so the field shape may still move.
            Self::Gnmi => Stability::Beta,
            // New and validated against one operator's 1960s only; the field shape may still move
            // once other Instant On models' payloads are seen.
            Self::InstantOnAccount => Stability::Beta,
            // New; validated against one single-node PVE 8.4 lab. Multi-node clusters and PVE 9
            // are covered by the API's documented shape, not yet by a live cluster.
            Self::ProxmoxApiToken => Stability::Beta,
            Self::Libvirt => Stability::Beta,
        }
    }

    /// The plan feature an organization needs to create a credential of this type, or `None` when
    /// every plan can. Exhaustive with no wildcard: adding a credential type forces a decision.
    ///
    /// Gates creation only. A credential created before its type was gated, or before the
    /// organization's plan changed, keeps working and stays editable.
    pub fn required_feature(&self) -> Option<Feature> {
        match self {
            Self::SnmpCommunity
            | Self::SnmpV3
            | Self::Gnmi
            | Self::DockerProxy
            | Self::DockerSocket
            | Self::PodmanProxy
            | Self::PodmanSocket
            | Self::UnifiApiKey
            | Self::UnifiLocalAdmin
            | Self::SshPassword
            | Self::SshKey
            | Self::SshScript
            | Self::Libvirt
            | Self::WakeOnLan
            | Self::ProxmoxApiToken => None,
            Self::InstantOnAccount => Some(Feature::InstantOnIntegration),
        }
    }

    /// How a credential of this type reaches its host, and which service owns the port it goes
    /// through. The one declaration of whether a type signs in through a login, is a login, or owns
    /// ports of its own: [`Self::role`] and [`Self::requires_login`] read it, and
    /// `every_credential_type_dispatches_the_reach_it_declares` checks each type's fields and
    /// dispatched payload against it. Exhaustive, so a new type has to answer.
    pub fn reach(&self) -> ReachDeclaration {
        match self {
            Self::SnmpCommunity | Self::SnmpV3 => ReachDeclaration::FixedUdpPorts(&[161, 1161]),
            Self::Gnmi
            | Self::DockerProxy
            | Self::PodmanProxy
            | Self::UnifiApiKey
            | Self::UnifiLocalAdmin
            | Self::ProxmoxApiToken => ReachDeclaration::OwnPortField { field: "port" },
            Self::SshPassword | Self::SshKey => ReachDeclaration::LoginTransport { field: "port" },
            Self::SshScript | Self::Libvirt => ReachDeclaration::ThroughLogin,
            // A local socket, HPE's cloud portal, a broadcast packet. Wake-on-LAN's UDP port field
            // is the packet's destination, not a port it owns on the host.
            Self::DockerSocket | Self::PodmanSocket | Self::InstantOnAccount | Self::WakeOnLan => {
                ReachDeclaration::NoPort
            }
        }
    }

    /// What this type is for: a login is the type whose port is a transport others sign in
    /// through ([`Self::reach`]).
    pub fn role(&self) -> CredentialRole {
        match self.reach() {
            ReachDeclaration::LoginTransport { .. } => CredentialRole::Login,
            ReachDeclaration::OwnPortField { .. }
            | ReachDeclaration::FixedUdpPorts(_)
            | ReachDeclaration::ThroughLogin
            | ReachDeclaration::NoPort => CredentialRole::Dispatched,
        }
    }

    /// The wire tag a credential of this type goes out under, for what must name it without
    /// building the payload (a warning about a credential that was not sent).
    pub fn wire_discriminant(&self) -> CredentialQueryPayloadDiscriminants {
        use CredentialQueryPayloadDiscriminants as W;
        match self {
            Self::SnmpCommunity | Self::SnmpV3 => W::Snmp,
            Self::Gnmi => W::Gnmi,
            Self::DockerProxy => W::DockerProxy,
            Self::DockerSocket => W::DockerSocket,
            Self::PodmanProxy => W::PodmanProxy,
            Self::PodmanSocket => W::PodmanSocket,
            Self::UnifiApiKey | Self::UnifiLocalAdmin => W::UnifiController,
            Self::InstantOnAccount => W::InstantOn,
            Self::SshPassword | Self::SshKey | Self::SshScript => W::Ssh,
            Self::WakeOnLan => W::WakeOnLan,
            Self::Libvirt => W::Libvirt,
            Self::ProxmoxApiToken => W::Proxmox,
        }
    }

    /// Whether a credential of this type signs in through a login it names
    /// (`login_credential_id`), as its [`Self::reach`] declares.
    pub fn requires_login(&self) -> bool {
        matches!(self.reach(), ReachDeclaration::ThroughLogin)
    }

    /// Whether an organization on `plan` can create a credential of this type.
    pub fn available_on(&self, plan: &BillingPlan) -> bool {
        self.required_feature()
            .is_none_or(|feature| plan.has_feature(feature.id()))
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
            // The last daemon version that cannot receive this type. The frontend uses it to
            // disable the type in the discovery credential picker; the server's dispatch filter
            // is what actually withholds it.
            "last_unsupported_daemon_version": self.last_unsupported_daemon_version().to_string(),
            // Release maturity. The frontend renders a "Beta" tag; it is not a gate.
            "stability": self.stability(),
            // A login is never assigned: the form skips assignments and other pickers leave it
            // out. A type that requires one shows a login picker.
            "role": self.role(),
            "requires_login": self.requires_login(),
            // Whether the vendor publishes this API. Orthogonal to `stability` — an integration
            // can be fully validated and still be riding an undocumented endpoint.
            "upstream_support": self.upstream_support(),
            // The plan feature needed to create this type. The type picker shows an upgrade badge
            // when the organization's plan lacks it; the server refuses the create either way.
            "required_feature": self.required_feature().map(|f| f.id()),
            // Guide for this credential's integration, path-only. The form links it for every
            // type rather than the two it used to hardcode.
            "docs_path": integration.docs_path(),
            // Keys `credential-integrations.json`; the picker groups types on it.
            "integration": integration,
            // How this transport connects, without the integration's discovery text. The picker
            // shows it under an integration row that already carries that text; standalone
            // surfaces (the type dropdown) use the full `description`.
            "transport_note": self.transport_note(),
            "associated_service": ServiceDefinition::name(&*service),
            "logo": service.logo(),
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
