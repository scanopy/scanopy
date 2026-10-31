use crate::server::hosts::r#impl::os::recog::RecogDatabase;
use crate::server::shared::attribution::AttributeMethod;
use crate::server::shared::oui;
use crate::server::{
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::{
            base::{
                DiscoverySessionServiceMatchParams, ServiceMatchBaselineParams,
                ServiceMatchServiceParams,
            },
            virtualization::ServiceVirtualization,
        },
    },
    shared::types::metadata::TypeMetadataProvider,
    subnets::r#impl::types::SubnetType,
};
use anyhow::{Error, anyhow};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::{net::IpAddr, ops::Range};
use strum_macros::{Display, EnumDiscriminants, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::{ports::r#impl::base::PortType, services::r#impl::endpoints::Endpoint};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MatchResult {
    pub ports: Vec<PortType>,
    pub endpoint: Option<Endpoint>,
    pub mac_vendor: Option<String>,
    pub details: MatchDetails,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MatchDetails {
    /// Why the service was matched to this definition.
    pub reason: MatchReason,
    /// How strong the match is.
    pub confidence: MatchConfidence,
}

impl MatchDetails {
    pub fn new_certain(reason_str: &str) -> Self {
        Self {
            reason: MatchReason::Reason(reason_str.to_string()),
            confidence: MatchConfidence::Certain,
        }
    }

    pub fn reason_string(&self) -> String {
        match &self.reason {
            MatchReason::Container(string, _) => string.clone(),
            MatchReason::Reason(string) => string.clone(),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "lowercase")]
pub enum MatchReason {
    Reason(String),
    #[serde(rename = "container")]
    Container(String, Vec<MatchReason>),
}

/// Manual ToSchema for MatchReason since internally-tagged enums with tuple variants aren't supported
impl utoipa::PartialSchema for MatchReason {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
        use utoipa::openapi::schema::{
            ArrayBuilder, ArrayItems, ObjectBuilder, OneOfBuilder, SchemaType, Type,
        };
        use utoipa::openapi::{Ref, RefOr, Schema};

        // Variant 1: { type: "reason", data: string }
        let reason_variant = ObjectBuilder::new()
            .title(Some("Reason"))
            .property(
                "type",
                ObjectBuilder::new()
                    .schema_type(SchemaType::new(Type::String))
                    .enum_values(Some(vec!["reason"])),
            )
            .required("type")
            .property(
                "data",
                ObjectBuilder::new()
                    .schema_type(SchemaType::new(Type::String))
                    .description(Some("Why the service was matched.")),
            )
            .required("data")
            .build();

        // Variant 2: { type: "container", data: [string, MatchReason[]] }, as a two-item tuple
        let container_data = ArrayBuilder::new()
            .prefix_items([
                Schema::Object(
                    ObjectBuilder::new()
                        .schema_type(SchemaType::new(Type::String))
                        .description(Some("The rule that grouped these reasons."))
                        .build(),
                ),
                Schema::Array(
                    ArrayBuilder::new()
                        .items(RefOr::Ref(Ref::from_schema_name("MatchReason")))
                        .description(Some("The reasons it grouped."))
                        .build(),
                ),
            ])
            .items(ArrayItems::False)
            .min_items(Some(2))
            .max_items(Some(2))
            .description(Some("Tuple of [name: string, children: MatchReason[]]"));
        let container_variant = ObjectBuilder::new()
            .title(Some("Container"))
            .property(
                "type",
                ObjectBuilder::new()
                    .schema_type(SchemaType::new(Type::String))
                    .enum_values(Some(vec!["container"])),
            )
            .required("type")
            .property("data", container_data)
            .required("data")
            .build();

        RefOr::T(Schema::OneOf(
            OneOfBuilder::new()
                .item(reason_variant)
                .item(container_variant)
                .description(Some("Match reason - either a simple reason string or a container with nested reasons"))
                .build(),
        ))
    }
}

impl utoipa::ToSchema for MatchReason {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("MatchReason")
    }
}

#[derive(
    Debug,
    Clone,
    Hash,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    ToSchema,
    strum::EnumIter,
    IntoStaticStr,
)]
pub enum MatchConfidence {
    NotApplicable = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Certain = 4,
}

impl MatchConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchConfidence::NotApplicable => "Not Applicable",
            MatchConfidence::Low => "Low",
            MatchConfidence::Medium => "Medium",
            MatchConfidence::High => "High",
            MatchConfidence::Certain => "Certain",
        }
    }
}

/// Types of client probes that run before service matching.
/// The probe result (success/failure) is pre-computed; Pattern::ClientResponse
/// just checks whether it succeeded.
///
/// Two types of producer fill this: the credentialed `DiscoveryIntegration`s, and the
/// non-credentialed [`AppProbe`](crate::daemon::utils::app_probe::AppProbe)s. Every variant needs
/// one — a variant nothing produces gives a service definition a pattern that can never match, and
/// `every_client_probe_variant_has_a_producer` is what now says so.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    strum_macros::EnumIter,
    strum_macros::EnumString,
    strum_macros::IntoStaticStr,
    strum_macros::VariantNames,
)]
pub enum ClientProbe {
    Docker,
    /// A completed gNMI `Get` against the device. A gRPC listener accepts a TCP connect whatever
    /// it serves, so the completed call is the only evidence that the target is a gNMI endpoint.
    Gnmi,
    Podman,
    Snmp,
    UnifiController,
    /// Authenticated to the Instant On cloud portal with the credential bound to this host.
    /// Unlike the others this proves nothing is listening *on* the host — it proves the portal
    /// account works, which is what gates the integration's `execute`.
    InstantOn,
    /// The Proxmox VE API answered `/version` with the credential's API token.
    Proxmox,
    /// `virsh` answered over SSH with the libvirt connection the credential names.
    Libvirt,
    /// A well-formed MBAP frame came back with our transaction ID echoed. Says nothing about
    /// whether the device answered `0x2B` — a `0xAB` exception is still Modbus.
    ModbusTcp,
    /// The OPC UA binary transport answered a `HEL` with an `ACK` or an `ERR`.
    OpcUa,
    /// A CIP ListIdentity reply came back with our sender context echoed.
    EtherNetIp,
    /// An `OPTIONS` request came back as a `SIP/2.0` status line. Any final response counts: a
    /// stack that parsed the request and refused it is still a stack.
    Sip,
    /// The server sent an `SSH-` identification string, which RFC 4253 requires before anything
    /// else happens.
    Ssh,
    /// An FTP greeting: `220` ready, or `421` refusing this client.
    Ftp,
    /// Telnet option negotiation — an `IAC` command byte in the opening bytes.
    Telnet,
    /// An `OPTIONS` request came back as an `RTSP/1.0` status line.
    Rtsp,
    /// A NUT server answered its text protocol rather than an error of another shape.
    Nut,
    /// An EPICS Channel Access server answered a `CA_PROTO_VERSION` header with its own.
    EpicsCa,
    /// A Zabbix agent answered a passive check with its `ZBXD` header.
    ZabbixAgent,
    /// A Check_MK agent dumped its section list, which opens `<<<check_mk>>>`.
    CheckMkAgent,
    /// An SMB2 NEGOTIATE was answered with an SMB2 header.
    Smb,
    /// An LDAP search of the root DSE came back as a BER-encoded searchResEntry or a result code.
    Ldap,
    /// A Kerberos AS-REQ came back as a KRB-ERROR, which is what an unauthenticated request earns
    /// and is proof of a KDC.
    Kerberos,
    /// MySQL's server greeting packet, which it sends before the client says anything.
    MySql,
    /// PostgreSQL answered an SSLRequest with `S` or `N`.
    PostgreSql,
    /// A TDS PRELOGIN response.
    MsSql,
    /// MongoDB answered a `hello` command.
    MongoDb,
    /// Redis answered `PING` with `+PONG`, or refused it with `-NOAUTH`, which is still Redis.
    Redis,
    /// A CQL OPTIONS frame came back as SUPPORTED.
    Cassandra,
    /// A Kafka ApiVersions request came back with our correlation ID echoed.
    Kafka,
    /// The AMQP protocol header was answered, either by agreement or by a version rejection.
    Amqp,
    /// An MQTT CONNECT came back as a CONNACK, whatever return code it carries.
    Mqtt,
    /// An Oracle TNS connect packet came back as a TNS packet.
    OracleTns,
    /// An X.224 connection request came back as a connection confirm.
    Rdp,
    /// An ONC RPC NULL call to the NFS program came back as an RPC reply.
    Nfs,
    /// A DNS query over TCP came back as a response carrying our transaction id.
    DnsTcp,
    /// A TLS handshake reached the server's certificate, and that certificate's subject carries the
    /// organizational unit Docker issues to swarm nodes. Names the swarm, not just the transport.
    DockerSwarm,
    /// A TLS handshake reached the server's certificate. Presence of TLS only — used where the
    /// certificate carries nothing dependable to match on.
    Tls,
    /// An IKE_SA_INIT came back as an IKE response, including a rejection notify.
    Ike,
    /// An OpenVPN hard reset was answered with the server's own hard reset.
    OpenVpn,
    /// A ZMTP greeting came back naming a security mechanism, in cleartext, before any encryption.
    Zmtp,
    /// A Bacula `Hello` was answered with the plaintext CRAM-MD5 challenge.
    Bacula,
    /// An SSH identification string whose software name is Beszel's.
    BeszelAgent,
    /// A Q.931 `SETUP` was answered with a Q.931 message carrying our call reference.
    H323,
    /// An SNMP agent answered SNMPv3 engine discovery with a Report naming its engine, which it
    /// sends before any credential is involved. Not [`Self::Snmp`], which means a credential worked.
    SnmpEngine,
    /// A Docker Engine API answered `GET /_ping` with its `Server: Docker/` token, before any auth.
    /// Not [`Self::Docker`], which means a credential reached the daemon.
    DockerPing,
    /// Podman's API service answered `GET /_ping` with its `Server: Libpod/` token, before any
    /// auth. Not [`Self::Podman`], which means a credential reached it.
    PodmanPing,
    /// A `Get-Printer-Attributes` request came back as an `application/ipp` message, whatever its
    /// status.
    Ipp,
    /// An LPD "send queue state" request came back as the plain-text status RFC 1179 describes.
    Lpd,
    /// A SMUX `OpenPDU` came back as a `ClosePDU` (RFC 1227): the agent's multiplexing port
    /// refusing a subagent it was not configured for, which is SMUX answering.
    Smux,
    /// An ONC RPC NULL call to the portmapper program came back as an RPC reply with our XID.
    Rpcbind,
    /// An LLMNR query over TCP for the reverse name of the probed address was answered.
    Llmnr,
    /// A Spotify Connect receiver answered `getInfo` on the port and path its mDNS advertisement
    /// names, with the JSON every Spotify Connect implementation returns. The reply states the
    /// device's brand and model.
    SpotifyConnect,
    /// The port opened with an RFB protocol version, `RFB 003.008` and similar: a VNC server's
    /// greeting.
    Vnc,
    /// The port answered a lockdown `QueryType` with `com.apple.mobile.lockdown`: the pairing and
    /// sync service on iPhones and iPads.
    AppleLockdown,
    /// A DNS query over UDP was answered.
    DnsUdp,
    /// An SNTP request was answered with a time.
    Ntp,
    /// A DHCP DISCOVER broadcast on the subnet drew an OFFER.
    Dhcp,
    /// A BACnet Who-Is came back as a BVLC message from the address asked.
    Bacnet,
    /// A TFTP read request for a missing file came back as an `ERROR` or `DATA` packet.
    Tftp,
    /// An MGCP `AUEP` came back as a response line carrying our transaction id.
    Mgcp,
    /// `/api/v1/status` returned the Google Wifi private API shape — `setupState`,
    /// `software.softwareVersion`, `system` — from which the probe reads the unit's board and
    /// firmware. Router and mesh point both answer; `Not(IsGateway)` separates them.
    GoogleWifi,
}

impl ClientProbe {
    /// Where this probe's own structured answers sit on the provenance ladder.
    ///
    /// Exhaustive and with no default, so adding a probe forces the tier decision here — at the
    /// probe's own definition — rather than in `AttributeSource::method`, where it would be easy
    /// to add a variant and never revisit the match.
    pub fn method(&self) -> AttributeMethod {
        use AttributeMethod as M;
        match self {
            // A runtime or controller describing something it manages: known speaker, not the
            // subject.
            Self::Docker
            | Self::Podman
            | Self::UnifiController
            | Self::InstantOn
            | Self::Proxmox
            | Self::Libvirt => M::Reported,
            // We chose the address and the transport correlated the answer.
            Self::Snmp | Self::Gnmi => M::Queried,
            // Same, over the device's own product protocol.
            Self::ModbusTcp | Self::EtherNetIp | Self::OpcUa => M::Native,
            // The presence probes: we chose the address and spoke the service's own protocol to
            // it. Most establish only that the protocol answered. SMB is the one that reads a value
            // under this tier: the Windows release its NTLMSSP challenge states, which the server
            // emits about itself. (The SSH banner's OS is filed as `SshBannerMatch`, because it is
            // our match of a package tag rather than a field the server fills.)
            Self::Sip
            | Self::Ssh
            | Self::Ftp
            | Self::Telnet
            | Self::Rtsp
            | Self::Nut
            | Self::EpicsCa
            | Self::ZabbixAgent
            | Self::CheckMkAgent
            | Self::Smb
            | Self::Ldap
            | Self::Kerberos
            | Self::MySql
            | Self::PostgreSql
            | Self::MsSql
            | Self::MongoDb
            | Self::Redis
            | Self::Cassandra
            | Self::Kafka
            | Self::Amqp
            | Self::Mqtt
            | Self::OracleTns
            | Self::Rdp
            | Self::Nfs
            | Self::DnsTcp
            | Self::DockerSwarm
            | Self::Tls
            | Self::Ike
            | Self::OpenVpn
            | Self::Zmtp
            | Self::Bacula
            | Self::BeszelAgent
            | Self::H323
            | Self::SnmpEngine
            | Self::DockerPing
            | Self::PodmanPing
            | Self::Ipp
            | Self::Lpd
            | Self::Smux
            | Self::Rpcbind
            | Self::Llmnr
            | Self::SpotifyConnect
            | Self::Vnc
            | Self::AppleLockdown
            | Self::DnsUdp
            | Self::Ntp
            | Self::Dhcp
            | Self::Bacnet
            | Self::Tftp
            | Self::Mgcp
            | Self::GoogleWifi => M::Native,
        }
    }

    /// The ports this probe runs on, and so the ports a [`Pattern::ClientResponse`] reading it
    /// scans. The one place a probe's port is written: the probe runner pairs each
    /// [`AppProbe`](crate::daemon::utils::app_probe::AppProbe) with every port here.
    ///
    /// Empty for the evidence a credentialed integration produces, and for Spotify Connect, whose
    /// port comes from the device's own mDNS advertisement. Neither runs as an app probe.
    pub fn ports(&self) -> Vec<PortType> {
        use crate::daemon::utils::app_probe::{
            beszel_agent::BESZEL_AGENT_PORT,
            epics_ca::CA_SERVER_PORT,
            h323::H323_PORT,
            mgcp::{MGCP_CALL_AGENT_PORT, MGCP_GATEWAY_PORT},
        };
        use crate::server::services::definitions::saltmaster::{
            SALT_PUBLISH_PORT, SALT_REQUEST_PORT,
        };
        match self {
            Self::Docker
            | Self::Gnmi
            | Self::Podman
            | Self::Snmp
            | Self::UnifiController
            | Self::InstantOn
            | Self::Proxmox
            | Self::Libvirt
            | Self::SpotifyConnect => vec![],
            Self::ModbusTcp => vec![PortType::ModbusTcp],
            Self::OpcUa => vec![PortType::OpcUa],
            Self::EtherNetIp => vec![PortType::EtherNetIp],
            Self::Sip => vec![PortType::Sip],
            // 2222 is the usual second sshd, and the banner tells it from anything else there.
            Self::Ssh => vec![PortType::Ssh, PortType::new_tcp(2222)],
            Self::Ftp => vec![PortType::Ftp],
            Self::Telnet => vec![PortType::Telnet],
            Self::Rtsp => vec![PortType::Rtsp],
            Self::Nut => vec![PortType::new_tcp(3493)],
            Self::EpicsCa => vec![PortType::new_tcp(CA_SERVER_PORT)],
            Self::ZabbixAgent => vec![PortType::new_tcp(10050)],
            Self::CheckMkAgent => vec![PortType::new_tcp(6556)],
            Self::Smb => vec![PortType::Samba],
            Self::Ldap => vec![PortType::Ldap],
            Self::Kerberos => vec![PortType::Kerberos],
            Self::MySql => vec![PortType::MySql],
            Self::PostgreSql => vec![PortType::PostgreSQL],
            Self::MsSql => vec![PortType::MsSql],
            Self::MongoDb => vec![PortType::MongoDB],
            Self::Redis => vec![PortType::Redis],
            Self::Cassandra => vec![PortType::Cassandra],
            Self::Kafka => vec![PortType::Kafka],
            Self::Amqp => vec![PortType::AMQP],
            Self::Mqtt => vec![PortType::Mqtt],
            Self::OracleTns => vec![PortType::new_tcp(1521)],
            Self::Rdp => vec![PortType::Rdp],
            Self::Nfs => vec![PortType::Nfs],
            Self::DnsTcp => vec![PortType::DnsTcp],
            Self::DockerSwarm => vec![PortType::new_tcp(2377)],
            // Unbound's remote-control port: the TLS it speaks there is all it offers unauthenticated.
            Self::Tls => vec![PortType::new_tcp(8953)],
            Self::Ike => vec![PortType::new_udp(500)],
            Self::OpenVpn => vec![PortType::OpenVPN],
            Self::Zmtp => vec![
                PortType::new_tcp(SALT_PUBLISH_PORT),
                PortType::new_tcp(SALT_REQUEST_PORT),
            ],
            Self::Bacula => vec![PortType::new_tcp(9101)],
            Self::BeszelAgent => vec![PortType::new_tcp(BESZEL_AGENT_PORT)],
            Self::H323 => vec![PortType::new_tcp(H323_PORT)],
            Self::SnmpEngine => vec![PortType::Snmp],
            Self::DockerPing => vec![PortType::Docker],
            Self::PodmanPing => vec![PortType::Http8888],
            Self::Ipp => vec![PortType::Ipp],
            Self::Lpd => vec![PortType::LdpTcp],
            Self::Smux => vec![PortType::new_tcp(199)],
            Self::Rpcbind => vec![PortType::new_tcp(111)],
            Self::Llmnr => vec![PortType::new_tcp(5355)],
            Self::Vnc => vec![PortType::new_tcp(5900)],
            Self::AppleLockdown => vec![PortType::new_tcp(62078)],
            Self::DnsUdp => vec![PortType::DnsUdp],
            Self::Ntp => vec![PortType::Ntp],
            Self::Dhcp => vec![PortType::Dhcp],
            Self::Bacnet => vec![PortType::BACnet],
            Self::Tftp => vec![PortType::new_udp(69)],
            Self::Mgcp => vec![
                PortType::new_udp(MGCP_GATEWAY_PORT),
                PortType::new_udp(MGCP_CALL_AGENT_PORT),
            ],
            Self::GoogleWifi => vec![PortType::Http],
        }
    }
}

/// A device reported by a management controller the daemon authenticated to, rather than one
/// probed directly over the network.
///
/// The controller is authoritative for the devices it has adopted, so what it says a device
/// *is* counts as match evidence — the same way [`Pattern::ContainerVirtualization`] treats the
/// container runtime's own inventory as evidence. This keeps controller-sourced devices inside
/// the normal matcher instead of having integrations construct `Service` rows directly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManagedDevice {
    /// Vendor-specific device class, verbatim from the controller (e.g. UniFi's `"usw"`).
    /// Compared against the vendor-namespaced constants below.
    pub device_type: String,
}

/// UniFi's `stat/device` `type` values.
/// unpoller: the discriminator it switches on to pick `USW` / `UAP` / `USG` / `UDM`.
pub struct UnifiDeviceType;
impl UnifiDeviceType {
    pub const ACCESS_POINT: &'static str = "uap";
    pub const SWITCH: &'static str = "usw";
    pub const GATEWAY: &'static str = "ugw";
    pub const DREAM_MACHINE: &'static str = "udm";
    /// The Next-Gen Gateway line (UXG-Lite, UXG-Pro, UXG-Max). unpoller: `UXG.Type`.
    pub const NEXT_GEN_GATEWAY: &'static str = "uxg";
}

/// Instant On's inventory `deviceType` values — the four members of the portal's own
/// `device-type-enum`, read from its web client.
///
/// `STACK` is a stack of switches the portal presents as a single device, so it resolves to the
/// same service as `SWITCH` rather than getting one of its own.
pub struct InstantOnDeviceType;
impl InstantOnDeviceType {
    pub const ACCESS_POINT: &'static str = "ACCESS_POINT";
    pub const SWITCH: &'static str = "SWITCH";
    pub const STACK: &'static str = "STACK";
    pub const GATEWAY: &'static str = "GATEWAY";
}

/// DNS-SD service types, as a device advertises them on 5353.
///
/// Named here rather than inline in definitions for the same reason as the vendor blocks above:
/// the string is a protocol identifier, and one typo produces a pattern that silently never
/// matches. Written without the `.local` domain, which is how the browse normalises them.
///
/// These reach devices the port scan cannot: a HomeKit sensor or a HomePod may expose no TCP port
/// worth scanning and still announce itself here.
pub struct DnsSdServiceType;
impl DnsSdServiceType {
    /// Chromecast and Google Home. TXT `md=` carries the model, which is what separates the two.
    pub const GOOGLE_CAST: &'static str = "_googlecast._tcp";
    /// Apple TV, HomePod, AirPort — anything that can receive AirPlay.
    pub const AIRPLAY: &'static str = "_airplay._tcp";
    /// AirPlay audio (Remote Audio Output Protocol). HomePods and AirPlay speakers.
    pub const RAOP: &'static str = "_raop._tcp";
    /// Apple's device-to-device pairing channel, on Apple TVs and HomePods.
    pub const COMPANION_LINK: &'static str = "_companion-link._tcp";
    /// Apple device metadata; TXT `model=` carries the hardware identifier.
    pub const DEVICE_INFO: &'static str = "_device-info._tcp";
    /// HomeKit Accessory Protocol. TXT `ci=` carries the accessory category.
    pub const HOMEKIT: &'static str = "_hap._tcp";
    /// Internet Printing Protocol — the AirPrint advertisement.
    pub const IPP: &'static str = "_ipp._tcp";
    /// Raw page-description printing, advertised alongside IPP by most network printers.
    pub const PDL_DATASTREAM: &'static str = "_pdl-datastream._tcp";
    /// Sonos speakers.
    pub const SONOS: &'static str = "_sonos._tcp";
    /// Philips Hue bridge.
    pub const HUE: &'static str = "_hue._tcp";
    /// Home Assistant.
    pub const HOME_ASSISTANT: &'static str = "_home-assistant._tcp";
    /// Spotify Connect receivers. The SRV record names the port and TXT `CPath` the path of the
    /// `getInfo` endpoint; both differ by vendor (Bose `8200` `/zc`, Sonos `1400` `/spotifyzc`).
    pub const SPOTIFY_CONNECT: &'static str = "_spotify-connect._tcp";
}

#[derive(Debug, Clone, EnumDiscriminants)]
#[strum_discriminants(derive(IntoStaticStr))]
pub enum Pattern<'a> {
    /// Match any of the listed patterns
    AnyOf(Vec<Pattern<'a>>),

    /// Must match all of the listed patterns
    AllOf(Vec<Pattern<'a>>),

    /// Inverse of pattern
    Not(Box<Pattern<'a>>),

    /// Always matches, and binds the inner pattern's ports when the inner pattern matches. For the
    /// ports a product opens only sometimes, placed in an [`Pattern::AllOf`] beside the evidence
    /// that names it: the evidence decides the match, and this only adds what it finds. On its own,
    /// or as an [`Pattern::AnyOf`] arm, it would match every host, which
    /// `optional_patterns_only_sit_beside_evidence` refuses.
    Optional(Box<Pattern<'a>>),

    /// Whether or not a specific port is open on the host
    Port(PortType),

    /// Whether or not an endpoint provided a specific response
    /// PortType
    /// path: &str - ie "/", "/admin", etc
    /// body response: &str - String to match on in response
    /// status_code: optional, defaults to 199..400 (any ok or redirect)
    Endpoint(PortType, &'a str, &'a str, Option<Range<u16>>),

    /// Whether or not reseponse headers from the host
    /// PortType: If provided, check headers on a response from the specific port. Otherwise, use any port.
    /// header: &str - Header name
    /// value: &str - string to match on in value
    /// status_code: optional, defaults to 200..300 (any ok or redirect)
    Header(Option<PortType>, &'a str, &'a str, Option<Range<u16>>),

    /// Whether the subnet that the host was found on matches a subnet type
    SubnetIsType(SubnetType),

    /// Whether the host IP is found in the daemon's routing table.
    IsGateway,

    /// Whether the vendor derived from the mac address (https://gist.github.com/aallan/b4bb86db86079509e6159810ae9bd3e4) matches the provided str
    MacVendor(&'static str),

    /// Custom evaluation of discovery match params
    /// fn - constraint function
    /// &'a str - match reason (describe what it means if function evaluates true)
    /// &'a str - no match reason (describe what it means if function evaluates false)
    /// MatchConfdence - confidence level that match uniquely identifies service
    Custom(
        fn(&DiscoverySessionServiceMatchParams) -> bool,
        fn(&DiscoverySessionServiceMatchParams) -> Vec<PortType>,
        &'a str,
        &'a str,
        MatchConfidence,
    ),

    /// Whether a credentialed client probe succeeded for this host.
    /// The probe (connection + ping) runs before service matching;
    /// the pattern just checks the pre-computed result.
    ClientResponse(ClientProbe),

    /// Whether the service runs in a container (Docker or Podman). Runtime-agnostic;
    /// per-runtime container definitions narrow to their virtualization via a custom check.
    ContainerVirtualization,

    /// Whether the host's mDNS announcement carries a DNS-SD service type, optionally with a
    /// constraint on that service's TXT data.
    ///
    /// * `&str` — the service type (see [`DnsSdServiceType`]).
    /// * `Option<(&str, &str)>` — a TXT key and the prefix its value must start with.
    ///
    /// One variant rather than a separate TXT pattern, because TXT records belong to a service
    /// rather than to a host: a Sonos advertises `_airplay._tcp` with `model=Five` *and*
    /// `_spotify-connect._tcp` with an unrelated set, so a TXT match that was not scoped to a
    /// service type could be satisfied by the wrong one.
    ///
    /// The TXT half is what makes this able to identify a device rather than a capability.
    /// `_airplay._tcp` alone means "can receive AirPlay", which is true of Apple TVs, HomePods,
    /// Macs and most third-party speakers; `model` starting `AppleTV` is an Apple TV.
    ///
    /// Only matches when the mDNS browse reached the host's broadcast domain — mDNS is link-local,
    /// so this is evidence a daemon can only gather about the segments it sits on.
    DnsSd(&'a str, Option<(&'a str, &'a str)>),

    /// Whether a certificate on any of the host's TLS ports was issued by a CA whose name contains
    /// the string (case-insensitive), e.g. `OU=Cast` or `O=Bose Corporation`.
    ///
    /// A vendor's devices sign every TLS port with the vendor's CA, including ports whose numbers
    /// differ between units, so the issuer names the product where a port list cannot. Read across
    /// every certificate the host presented, whichever service holds the port, so the answer does
    /// not depend on match order and `Not(CertificateIssuer(..))` asks about the host. Binds the
    /// ports with a matching certificate that are still unbound.
    ///
    /// The ports are where this issuer's certificates are known to appear. They are scanned, so a
    /// light scan reaches them, and are not evidence: the certificate is. A certificate on a port
    /// not listed still matches when a full scan finds it.
    CertificateIssuer(&'a str, &'static [u16]),

    /// Whether a Recog fingerprint of something one of the host's ports emitted (a page title, a
    /// cookie, a login challenge, a favicon, a certificate) names one of these `service.vendor` and
    /// `service.product` pairs, exactly as Recog writes them (the vendor empty where Recog gives none).
    ///
    /// Detection is Recog's: the definition names products and never copies a regex or a hash, so
    /// a Recog refresh updates it. Binds the unbound ports a fingerprint matched on.
    ///
    /// The ports are where the product's web interface is known to listen. They are scanned and
    /// requested at `/`, so a light scan reaches them, and are not evidence: the fingerprint is.
    Recog(&'static [(&'static str, &'static str)], &'static [u16]),

    /// Whether a management controller reported this device as the given device class.
    /// Takes a vendor-specific class string (see [`UnifiDeviceType`]), mirroring how
    /// [`Pattern::MacVendor`] takes a vendor string. Only matches when the host was
    /// discovered via a controller integration that supplied a [`ManagedDevice`].
    ManagedDeviceType(&'static str),

    /// No match pattern (only added manually or by the system)
    None,
}

// https://gist.github.com/aallan/b4bb86db86079509e6159810ae9bd3e4
pub struct Vendor;
impl Vendor {
    pub const PHILIPS: &'static str = "Philips Lighting BV";
    pub const HP: &'static str = "HP Inc.";
    pub const EERO: &'static str = "eero Inc";
    pub const TPLINK: &'static str = "TP-LINK TECHNOLOGIES CO.,LTD";
    pub const UBIQUITI: &'static str = "Ubiquiti Inc";
    pub const GOOGLE: &'static str = "Google, Inc.";
    pub const NEST: &'static str = "Nest Labs Inc.";
    pub const AMAZON: &'static str = "Amazon Technologies Inc.";
    pub const SONOS: &'static str = "Sonos, Inc.";
    pub const ECOBEE: &'static str = "ecobee inc";
    pub const ROKU: &'static str = "Roku, Inc";
    pub const ROBOROCK: &'static str = "Beijing Roborock Technology Co., Ltd.";
    pub const WIZ: &'static str = "WiZ";
    pub const TUYASMART: &'static str = "Tuya Smart Inc.";
    /// The registrant on TP-Link's newer OUIs, which Kasa plugs and switches ship with.
    pub const TPLINK_SYSTEMS: &'static str = "TP-Link Systems Inc.";
    /// Bosch, Siemens, Neff, Gaggenau and Thermador appliances.
    pub const BSH: &'static str = "BSH Hausgeräte GmbH";
    /// Chamberlain and LiftMaster, including myQ.
    pub const CHAMBERLAIN: &'static str = "The Chamberlain Group, Inc";
    pub const BOSE: &'static str = "Bose Corporation";
    /// ESP8266 and ESP32 Wi-Fi chips, inside most hobbyist and many commercial IoT devices.
    pub const ESPRESSIF: &'static str = "Espressif Inc.";
    pub const BROTHER: &'static str = "Brother industries, LTD.";
}

impl PartialEq for Pattern<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Pattern::AnyOf(a), Pattern::AnyOf(b)) => a == b,
            (Pattern::AllOf(a), Pattern::AllOf(b)) => a == b,
            (Pattern::Not(a), Pattern::Not(b)) => a == b,
            (Pattern::Optional(a), Pattern::Optional(b)) => a == b,
            (Pattern::Port(a), Pattern::Port(b)) => a == b,
            (
                Pattern::Endpoint(port_a, path_a, match_a, range_a),
                Pattern::Endpoint(port_b, path_b, match_b, range_b),
            ) => port_a == port_b && path_a == path_b && match_a == match_b && range_a == range_b,
            (
                Pattern::Header(port_a, header_a, value_a, range_a),
                Pattern::Header(port_b, header_b, value_b, range_b),
            ) => {
                port_a == port_b && header_a == header_b && value_a == value_b && range_a == range_b
            }
            (Pattern::SubnetIsType(a), Pattern::SubnetIsType(b)) => a == b,
            (Pattern::IsGateway, Pattern::IsGateway) => true,
            (Pattern::MacVendor(a), Pattern::MacVendor(b)) => a == b,
            (
                Pattern::Custom(con_fn_a, port_fn_a, match_a, no_match_a, conf_a),
                Pattern::Custom(con_fn_b, port_fn_b, match_b, no_match_b, conf_b),
            ) => {
                // Compare function pointers by address and compare other fields
                (*con_fn_a as usize) == (*con_fn_b as usize)
                    && (*port_fn_a as usize) == (*port_fn_b as usize)
                    && match_a == match_b
                    && no_match_a == no_match_b
                    && conf_a == conf_b
            }
            (Pattern::ClientResponse(a), Pattern::ClientResponse(b)) => a == b,
            (Pattern::ContainerVirtualization, Pattern::ContainerVirtualization) => true,
            (Pattern::ManagedDeviceType(a), Pattern::ManagedDeviceType(b)) => a == b,
            (Pattern::DnsSd(a, a_txt), Pattern::DnsSd(b, b_txt)) => a == b && a_txt == b_txt,
            (Pattern::CertificateIssuer(a, a_ports), Pattern::CertificateIssuer(b, b_ports)) => {
                a == b && a_ports == b_ports
            }
            (Pattern::Recog(a, a_ports), Pattern::Recog(b, b_ports)) => {
                a == b && a_ports == b_ports
            }
            (Pattern::None, Pattern::None) => true,
            _ => false,
        }
    }
}

impl Eq for Pattern<'_> {}

impl Display for Pattern<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pattern::AnyOf(patterns) => {
                let pattern_strings = patterns.iter().map(|p| p.to_string()).join(", ");
                write!(f, "Any of: ({})", pattern_strings)
            }
            Pattern::AllOf(patterns) => {
                let pattern_strings = patterns.iter().map(|p| p.to_string()).join(", ");
                write!(f, "All of: ({})", pattern_strings)
            }
            Pattern::Not(pattern) => write!(f, "Not ({})", pattern),
            Pattern::Optional(pattern) => write!(f, "Optionally ({})", pattern),
            Pattern::Port(port_base) => write!(f, "{} is open", port_base),
            Pattern::Endpoint(port_base, path, match_string, range) => {
                if let Some(range) = range {
                    write!(
                        f,
                        "Endpoint response status is between {} and {}, and response body from <ip>:{}{} contains \"{}\"",
                        range.start,
                        range.end,
                        port_base.number(),
                        path,
                        match_string
                    )
                } else {
                    write!(
                        f,
                        "Endpoint response body from <ip>:{}{} contains \"{}\"",
                        port_base.number(),
                        path,
                        match_string
                    )
                }
            }
            Pattern::Header(port_base, header, value, range) => {
                let ip_str = if let Some(port_base) = port_base {
                    format!("<ip>:{}", port_base.number())
                } else {
                    "<ip>".to_string()
                };
                if let Some(range) = range {
                    write!(
                        f,
                        "Endpoint response status is between {} and {}, and response from {} has header \"{}\" with value \"{}\"",
                        range.start, range.end, ip_str, header, value
                    )
                } else {
                    write!(
                        f,
                        "Endpoint response from {} has header \"{}\" with value \"{}\"",
                        ip_str, header, value
                    )
                }
            }
            Pattern::SubnetIsType(subnet_type) => write!(f, "Subnet is type {:?}", subnet_type),
            Pattern::IsGateway => write!(
                f,
                "Host IP is a gateway in daemon's routing tables, or ends in .1 or .254."
            ),
            Pattern::MacVendor(vendor) => write!(f, "MAC Address belongs to {}", vendor),
            Pattern::Custom(_, _, _, _, _) => {
                write!(f, "A custom match pattern evaluated at runtime")
            }
            Pattern::ClientResponse(probe) => write!(f, "Client probe {:?} succeeded", probe),
            Pattern::ContainerVirtualization => write!(f, "Service is running in a container"),
            Pattern::ManagedDeviceType(device_type) => {
                write!(f, "Controller reports device type '{}'", device_type)
            }
            Pattern::DnsSd(service_type, txt) => match txt {
                Some((key, value)) => write!(
                    f,
                    "Host advertises '{}' over mDNS with {}={}…",
                    service_type, key, value
                ),
                None => write!(f, "Host advertises '{}' over mDNS", service_type),
            },
            Pattern::CertificateIssuer(issuer, _) => {
                write!(
                    f,
                    "A TLS certificate on the host was issued by '{}'",
                    issuer
                )
            }
            Pattern::Recog(products, _) => {
                write!(
                    f,
                    "A fingerprint of the host's web interface names {}",
                    recog_product_names(products)
                )
            }
            Pattern::None => write!(f, "No match pattern provided"),
        }
    }
}
/// The software a `Pattern::Recog` names, as a reader would: "Citrix NetScaler or F5 BIG-IP LTM".
fn recog_product_names(products: &[(&str, &str)]) -> String {
    products
        .iter()
        .map(|(vendor, product)| {
            if vendor.is_empty() || product.contains(vendor) {
                product.to_string()
            } else {
                format!("{vendor} {product}")
            }
        })
        .join(" or ")
}

/// Whether an address can be a gateway at all: a router another host reaches. A loopback, unspecified
/// or multicast address never is. macOS lists `127.0.0.1` as the gateway of its loopback routes, and
/// `127.0.0.1` also ends in .1, so without this the daemon's own host matched the Gateway service
/// by both of [`Pattern::IsGateway`]'s rules. An IPv6 link-local gateway (`fe80::1`) is a real
/// router and passes.
pub fn is_router_gateway(ip: &IpAddr) -> bool {
    !ip.is_loopback() && !ip.is_unspecified() && !ip.is_multicast()
}

impl Pattern<'_> {
    pub fn matches(
        &self,
        params: &DiscoverySessionServiceMatchParams,
    ) -> Result<MatchResult, Error> {
        // Return ports + endpoint that matched, if any

        let DiscoverySessionServiceMatchParams {
            gateway_ips,
            baseline_params,
            service_params,
            daemon_id,
            ..
        } = params;

        let ServiceMatchBaselineParams {
            subnet,
            ip_address,
            endpoint_responses,
            virtualization_metadata,
            managed_device,
            dns_sd,
            certificates,
            fingerprints,
            ..
        } = baseline_params;

        let ServiceMatchServiceParams {
            unbound_ports,
            claimed_ports,
            service_definition,
            ..
        } = service_params;

        match self {
            Pattern::Port(port_base) => {
                if let Some(matched_port) = unbound_ports.iter().find(|p| **p == *port_base) {
                    let mut all_other_services_ports: Vec<PortType> =
                        ServiceDefinitionRegistry::all_service_definitions()
                            .iter()
                            .filter(|s| s.id() != service_definition.id())
                            .flat_map(|s| s.discovery_pattern().ports())
                            .collect();

                    all_other_services_ports.sort_by_key(|p| (p.number(), p.protocol()));
                    all_other_services_ports.dedup();

                    let is_unique_to_service =
                        port_base.is_custom() && !all_other_services_ports.contains(port_base);

                    let (reason, confidence) = if port_base.is_custom() && is_unique_to_service {
                        (
                            format!(
                                "Port {} is open and is not used in other service match patterns",
                                port_base,
                            ),
                            MatchConfidence::Medium,
                        )
                    } else {
                        (format!("Port {} is open", port_base), MatchConfidence::Low)
                    };

                    Ok(MatchResult {
                        ports: vec![*matched_port],
                        endpoint: None,
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Reason(reason),
                            confidence,
                        },
                    })
                } else {
                    Err(anyhow!("Port {} is not open", port_base))
                }
            }

            Pattern::Header(
                port_base,
                expected_header,
                expected_value,
                expected_status_code_range,
            ) => {
                let match_result = endpoint_responses
                    .iter()
                    .filter(|actual| {
                        let is_same_endpoint = port_base
                            .map(|p| actual.endpoint.port_type == p)
                            .unwrap_or(true);

                        let expected_range =
                            expected_status_code_range.as_ref().unwrap_or(&(200..400));
                        let status_code_in_range = expected_range.contains(&actual.status);

                        let headers_contain_value = actual.headers.iter().any(|(header, value)| {
                            header.to_lowercase() == expected_header.to_lowercase()
                                && value
                                    .to_lowercase()
                                    .contains(&expected_value.to_lowercase())
                        });

                        is_same_endpoint && status_code_in_range && headers_contain_value
                    })
                    .map(|actual| {
                        let mut match_reason = Vec::new();

                        match_reason.push(format!(
                            "header {} contained \"{}\"",
                            expected_header, expected_value
                        ));

                        if let Some(expected_status_code_range) = expected_status_code_range {
                            // Only add this as a reason if expected status code range is anything other than successful
                            match_reason.push(format!(
                                "status code {} was in range {:?}",
                                actual.status, expected_status_code_range
                            ));
                        }

                        if let Some(port_base) = port_base {
                            (
                                actual,
                                format!(
                                    "Response from {} {}",
                                    port_base.number(),
                                    match_reason.join(" and ")
                                ),
                            )
                        } else {
                            (actual, format!("Response {}", match_reason.join(" and ")))
                        }
                    })
                    .next();

                match match_result {
                    Some((response, reason)) => Ok(MatchResult {
                        ports: vec![response.endpoint.port_type],
                        endpoint: Some(response.endpoint.clone()),
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Reason(reason),
                            confidence: MatchConfidence::High,
                        },
                    }),
                    None => Err(anyhow!(
                        "Could not find an header response on port {}",
                        port_base.unwrap_or_default().number()
                    )),
                }
            }

            Pattern::Endpoint(
                port_base,
                path,
                expected_body_match_string,
                expected_status_code_range,
            ) => {
                let endpoint = Endpoint::for_pattern(*port_base, path);

                let match_result = endpoint_responses
                    .iter()
                    .filter(|actual| {
                        let is_same_endpoint = actual.endpoint.protocol == endpoint.protocol
                        // Compare number + protocol instead of port_base and port_base 
                        // because ports are dynamically recreated during discovery 
                        // and named enums like Http9000 won't match new_tcp(9000)
                            && actual.endpoint.port_type.number() == endpoint.port_type.number()
                            && actual.endpoint.port_type.protocol() == endpoint.port_type.protocol()
                            && actual.endpoint.path == endpoint.path;

                        let expected_range =
                            expected_status_code_range.as_ref().unwrap_or(&(200..400));
                        let status_code_in_range = expected_range.contains(&actual.status);

                        let body_contains_match_string =
                            actual.body_contains_beyond_request(expected_body_match_string);

                        is_same_endpoint && status_code_in_range && body_contains_match_string
                    })
                    .map(|actual| {
                        let mut match_reason = Vec::new();

                        match_reason.push(format!(
                            "contained \"{}\" in body",
                            expected_body_match_string
                        ));

                        if let Some(expected_status_code_range) = expected_status_code_range {
                            // Only add this as a reson if expected status code range is anything other than successful
                            match_reason.push(format!(
                                "status code was {} was in range {:?}",
                                actual.status, expected_status_code_range
                            ));
                        }

                        (
                            actual,
                            format!(
                                "Response for {}:{}{} {}",
                                ip_address.base.ip_address,
                                port_base.number(),
                                path,
                                match_reason.join(" and ")
                            ),
                        )
                    })
                    .next();

                match match_result {
                    Some((response, reason)) => Ok(MatchResult {
                        ports: vec![response.endpoint.port_type],
                        endpoint: Some(response.endpoint.clone()),
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Reason(reason),
                            confidence: MatchConfidence::High,
                        },
                    }),
                    None => Err(anyhow!(
                        "Could not find an endpoint response containing {}",
                        expected_body_match_string
                    )),
                }
            }

            Pattern::MacVendor(vendor_string) => {
                if let Some(mac_address) =
                    crate::server::ip_addresses::r#impl::base::mac_of(&ip_address.base.mac_address)
                {
                    let mac_str = mac_address.to_string();
                    let Some(entry) = oui::lookup_by_mac(&mac_str) else {
                        return Err(anyhow!(
                            "Could not find vendor for mac address in OUI database"
                        ));
                    };

                    let normalize = |s: &str| -> String {
                        s.trim()
                            .to_lowercase()
                            .chars()
                            .filter(|c| c.is_alphanumeric())
                            .collect()
                    };

                    let vendor_string = normalize(vendor_string);
                    let entry_string = normalize(&entry.company_name);

                    if vendor_string == entry_string {
                        Ok(MatchResult {
                            ports: vec![],
                            endpoint: None,
                            mac_vendor: Some(entry.company_name.clone()),
                            details: MatchDetails {
                                reason: MatchReason::Reason(format!(
                                    "Mac address is from vendor {}",
                                    entry.company_name
                                )),
                                confidence: MatchConfidence::Medium,
                            },
                        })
                    } else {
                        Err(anyhow!("Mac address is not from vendor {}", vendor_string))
                    }
                } else {
                    Err(anyhow!(
                        "IPAddress {} does not have a mac address",
                        ip_address.base.ip_address
                    ))
                }
            }

            // `Not(DnsSd(..))` asks whether the host advertises the service at all, so it reads
            // the advertisement directly: a DnsSd match can stand down because another service
            // holds its port, and that must not read as the host not advertising it.
            Pattern::Not(pattern) => match match pattern.as_ref() {
                Pattern::DnsSd(expected, txt) => {
                    if dns_sd
                        .as_ref()
                        .is_some_and(|h| h.advertises(expected, *txt))
                    {
                        Ok(format!("Host advertises '{expected}' over mDNS"))
                    } else {
                        Err(anyhow!("Host did not advertise '{expected}' over mDNS"))
                    }
                }
                other => other
                    .matches(params)
                    .map(|result| result.details.reason.to_string()),
            } {
                Ok(reason) => Err(anyhow!("{reason}")),
                Err(e) => Ok(MatchResult {
                    ports: vec![],
                    endpoint: None,
                    mac_vendor: None,
                    details: MatchDetails {
                        reason: MatchReason::Reason(format!("{}", e)),
                        confidence: MatchConfidence::Low,
                    },
                }),
            },

            // Never fails, and never raises the confidence of the match it sits in: the evidence beside
            // it decided that. It contributes the ports its inner pattern bound, and a reason either way.
            Pattern::Optional(pattern) => Ok(match pattern.matches(params) {
                Ok(result) => MatchResult {
                    details: MatchDetails {
                        reason: result.details.reason,
                        confidence: MatchConfidence::NotApplicable,
                    },
                    ..result
                },
                Err(e) => MatchResult {
                    ports: vec![],
                    endpoint: None,
                    mac_vendor: None,
                    details: MatchDetails {
                        reason: MatchReason::Reason(format!("Optional, absent: {e}")),
                        confidence: MatchConfidence::NotApplicable,
                    },
                },
            }),

            Pattern::AnyOf(patterns) => {
                let mut ports = Vec::new();
                let mut endpoint = None;
                let mut mac_vendor = None;
                let mut any_matched = false;
                let mut confidence = MatchConfidence::Low;
                let mut reasons = Vec::new();
                let mut no_match_errors = String::new();
                patterns.iter().for_each(|p| match p.matches(params) {
                    Ok(result) => {
                        any_matched = true;
                        ports.extend(result.ports);
                        reasons.push(result.details.reason);

                        if result.endpoint.is_some() && endpoint.is_none() {
                            endpoint = result.endpoint;
                        }

                        if result.mac_vendor.is_some() && mac_vendor.is_none() {
                            mac_vendor = result.mac_vendor;
                        }

                        if result.details.confidence > confidence {
                            confidence = result.details.confidence;
                        }
                    }
                    Err(e) => {
                        no_match_errors = no_match_errors.clone() + ", " + &e.to_string();
                    }
                });

                ports.sort_by_key(|p| (p.number(), p.protocol()));
                ports.dedup();

                if any_matched {
                    Ok(MatchResult {
                        ports,
                        endpoint: None,
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Container("Any of".to_string(), reasons),
                            confidence,
                        },
                    })
                } else {
                    Err(anyhow!(no_match_errors))
                }
            }

            Pattern::AllOf(patterns) => {
                let mut all_matched = true;
                let mut ports = Vec::new();
                let mut endpoint = None;
                let mut mac_vendor = None;
                let mut matched_confidences = Vec::new();
                let mut reasons = Vec::new();
                let mut no_match_errors = String::new();
                patterns.iter().for_each(|p| match p.matches(params) {
                    Ok(result) => {
                        ports.extend(result.ports);
                        reasons.push(result.details.reason);
                        matched_confidences.push(result.details.confidence);

                        if result.endpoint.is_some() && endpoint.is_none() {
                            endpoint = result.endpoint;
                        }

                        if result.mac_vendor.is_some() && mac_vendor.is_none() {
                            mac_vendor = result.mac_vendor;
                        }
                    }
                    Err(e) => {
                        all_matched = false;
                        no_match_errors = no_match_errors.clone() + ", " + &e.to_string();
                    }
                });

                if all_matched {
                    matched_confidences.sort();

                    let max_confidence =
                        matched_confidences.last().unwrap_or(&MatchConfidence::Low);

                    // Boost confidence if multiple lower-confidence patterns are matched. An
                    // optional arm is not one: it is `NotApplicable`, and corroborates nothing.
                    let corroborating = matched_confidences
                        .iter()
                        .filter(|c| **c != MatchConfidence::NotApplicable)
                        .count();
                    let confidence = if matches!(
                        max_confidence,
                        MatchConfidence::Low | MatchConfidence::Medium
                    ) && corroborating > 3
                    {
                        match max_confidence {
                            MatchConfidence::Low => MatchConfidence::Medium,
                            MatchConfidence::Medium => MatchConfidence::High,
                            _ => *max_confidence,
                        }
                    } else {
                        *max_confidence
                    };

                    ports.sort_by_key(|p| (p.number(), p.protocol()));
                    ports.dedup();

                    Ok(MatchResult {
                        ports,
                        endpoint: None,
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Container("All of".to_string(), reasons),
                            confidence,
                        },
                    })
                } else {
                    Err(anyhow!(no_match_errors))
                }
            }

            Pattern::IsGateway => {
                if !is_router_gateway(&ip_address.base.ip_address) {
                    return Err(anyhow!(
                        "A loopback, unspecified or multicast address is never a gateway"
                    ));
                }
                let gateway_ips_in_subnet: Vec<_> = gateway_ips
                    .iter()
                    .filter(|g| subnet.base.cidr.contains(g))
                    .collect();

                let count_gateways_in_subnet = gateway_ips_in_subnet.len();
                let host_ip_in_routing_table =
                    gateway_ips_in_subnet.contains(&&ip_address.base.ip_address);

                let last_octet_1_or_254 = match ip_address.base.ip_address {
                    IpAddr::V4(ipv4) => {
                        let octets = ipv4.octets();
                        octets[3] == 1 || octets[3] == 254
                    }
                    IpAddr::V6(ipv6) => {
                        let segments = ipv6.segments();
                        segments[7] == 1 || segments[7] == 254
                    }
                };

                let mut reason = String::new();

                let is_gateway = if host_ip_in_routing_table {
                    reason = format!(
                        "Host IP address is in routing table of daemon {}",
                        daemon_id
                    );
                    true
                } else if last_octet_1_or_254 && count_gateways_in_subnet == 0 {
                    // Likely a gateway if common IP and no other gateways found
                    reason = format!(
                        "No other gateways in subnet {} and IP address ends in 1 or 254",
                        subnet.base.cidr
                    );
                    true
                } else {
                    false
                };

                if is_gateway {
                    Ok(MatchResult {
                        ports: vec![],
                        endpoint: None,
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Reason(reason),
                            confidence: MatchConfidence::High,
                        },
                    })
                } else {
                    Err(anyhow!(
                        "IP address is not in routing table, and does not end in 1 or 254 with no other gateways identified in subnet"
                    ))
                }
            }

            Pattern::SubnetIsType(subnet_type) => {
                if &subnet.base.subnet_type() == subnet_type {
                    Ok(MatchResult {
                        ports: vec![],
                        endpoint: None,
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Reason(format!(
                                "Subnet {} is type {}",
                                subnet.base.cidr,
                                subnet_type.name()
                            )),
                            confidence: MatchConfidence::Low,
                        },
                    })
                } else {
                    Err(anyhow!(
                        "Subnet {} is not type {}",
                        subnet.base.cidr,
                        subnet_type.name()
                    ))
                }
            }

            Pattern::Custom(
                constraint_function,
                port_function,
                reason,
                no_match_reason,
                confidence,
            ) => {
                if constraint_function(params) {
                    Ok(MatchResult {
                        ports: port_function(params),
                        endpoint: None,
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Reason(reason.to_string()),
                            confidence: *confidence,
                        },
                    })
                } else {
                    let no_match_reason = no_match_reason.to_string();
                    Err(anyhow!(no_match_reason))
                }
            }

            Pattern::ClientResponse(probe) => {
                if let Some(ports) = baseline_params.client_responses.get(probe) {
                    Ok(MatchResult {
                        ports: ports.clone(),
                        endpoint: None,
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Reason(format!(
                                "Client probe {:?} succeeded",
                                probe
                            )),
                            confidence: MatchConfidence::Certain,
                        },
                    })
                } else {
                    Err(anyhow!("Client probe {:?} did not succeed", probe))
                }
            }

            Pattern::ContainerVirtualization => match virtualization_metadata {
                Some(ServiceVirtualization::Docker(..))
                | Some(ServiceVirtualization::Podman(..)) => Ok(MatchResult {
                    ports: vec![],
                    endpoint: None,
                    mac_vendor: None,
                    details: MatchDetails {
                        reason: MatchReason::Reason(
                            "Service is running in a container".to_string(),
                        ),
                        confidence: MatchConfidence::Low,
                    },
                }),
                _ => Err(anyhow!("Service is not running in a container")),
            },

            // The controller authenticated to us and named this device's class. That is a
            // direct statement from the device's own management plane, not an inference from
            // a banner, so it is `Certain` — the same confidence a credentialed client probe
            // earns. It is still ANDed with a MacVendor guard in the service definitions.
            Pattern::ManagedDeviceType(expected) => match managed_device {
                Some(device) if device.device_type.eq_ignore_ascii_case(expected) => {
                    Ok(MatchResult {
                        ports: vec![],
                        endpoint: None,
                        mac_vendor: None,
                        details: MatchDetails {
                            reason: MatchReason::Reason(format!(
                                "Controller reported device type '{}'",
                                device.device_type
                            )),
                            confidence: MatchConfidence::Certain,
                        },
                    })
                }
                Some(device) => Err(anyhow!(
                    "Controller reported device type '{}', not '{}'",
                    device.device_type,
                    expected
                )),
                None => Err(anyhow!("Host was not reported by a management controller")),
            },

            // A device asserting its own service type is as direct as a controller naming a
            // device class, but nobody authenticated to us to say it — an mDNS announcement is
            // unauthenticated and trivially spoofable by anything on the link. `High` rather than
            // the `Certain` that `ManagedDeviceType` earns, and definitions AND it with a vendor
            // or port arm where one exists.
            // Binds the port the advertisement's SRV record names: the scanned port when the scan
            // found it, otherwise a port taken from the advertisement itself. A port a service
            // matched earlier this round already holds is that service's, and the earlier match
            // is the more specific one (non-generic definitions run first), so this one stands
            // down rather than add a second service on the same port.
            Pattern::DnsSd(expected, txt) => match dns_sd {
                Some(host) if host.advertises(expected, *txt) => Ok(MatchResult {
                    ports: match host.advertised_port(expected) {
                        None => vec![],
                        Some(port) if claimed_ports.contains(&port) => {
                            return Err(anyhow!(
                                "Host advertises '{expected}' on {port}, which another service holds"
                            ));
                        }
                        Some(port) => vec![
                            unbound_ports
                                .iter()
                                .find(|p| **p == port)
                                .copied()
                                .unwrap_or(port),
                        ],
                    },
                    endpoint: None,
                    mac_vendor: None,
                    details: MatchDetails {
                        reason: MatchReason::Reason(match txt {
                            Some((key, value)) => format!(
                                "Host advertises '{expected}' over mDNS with {key}={value}…"
                            ),
                            None => format!("Host advertises '{expected}' over mDNS"),
                        }),
                        confidence: MatchConfidence::High,
                    },
                }),
                Some(_) => Err(anyhow!("Host did not advertise '{expected}' over mDNS")),
                // Not the same statement as "it didn't advertise it": mDNS is link-local, so a
                // host on a routed subnet can run the service and still never be asked.
                None => Err(anyhow!("No mDNS response was collected for this host")),
            },

            // A vendor CA signing the device's certificate is the vendor's own statement, made
            // before the device shipped, so `High`. The evidence is read across every certificate
            // on the host; only the ports still unbound are bound.
            Pattern::CertificateIssuer(expected, _) => {
                let expected_lower = expected.to_lowercase();
                let matching: Vec<_> = certificates
                    .iter()
                    .filter(|cert| cert.issuer.to_lowercase().contains(&expected_lower))
                    .collect();
                let Some(first) = matching.first() else {
                    return Err(anyhow!(
                        "No TLS certificate on the host was issued by '{expected}'"
                    ));
                };
                Ok(MatchResult {
                    ports: matching
                        .iter()
                        .map(|cert| cert.port)
                        .filter(|port| unbound_ports.contains(port))
                        .collect(),
                    endpoint: None,
                    mac_vendor: None,
                    details: MatchDetails {
                        reason: MatchReason::Reason(format!(
                            "TLS certificate on {} issued by {}",
                            first.port, first.issuer
                        )),
                        confidence: MatchConfidence::High,
                    },
                })
            }

            // A title, cookie or certificate naming the product is the product's own wording, so
            // `High`. A favicon alone is `Medium`: forks and rebrands keep the icon they started
            // from, which is why Recog grades every favicon match 0.5.
            Pattern::Recog(products, _) => {
                let matching: Vec<_> = fingerprints
                    .iter()
                    .filter(|fp| {
                        fp.matched.software.as_ref().is_some_and(|software| {
                            products
                                .contains(&(software.vendor.as_str(), software.product.as_str()))
                        })
                    })
                    .collect();
                let Some(first) = matching.first() else {
                    return Err(anyhow!(
                        "No fingerprint on the host names {}",
                        recog_product_names(products)
                    ));
                };
                let confidence = if matching
                    .iter()
                    .any(|fp| fp.database != RecogDatabase::Favicon)
                {
                    MatchConfidence::High
                } else {
                    MatchConfidence::Medium
                };
                let mut ports: Vec<PortType> = matching
                    .iter()
                    .map(|fp| fp.port)
                    .filter(|port| unbound_ports.contains(port))
                    .collect();
                ports.dedup();
                Ok(MatchResult {
                    ports,
                    endpoint: None,
                    mac_vendor: None,
                    details: MatchDetails {
                        reason: MatchReason::Reason(format!(
                            "{} on {} names {}",
                            first.source(),
                            first.port,
                            first
                                .matched
                                .software
                                .as_ref()
                                .map(|s| s.product.as_str())
                                .unwrap_or_default()
                        )),
                        confidence,
                    },
                })
            }

            Pattern::None => Err(anyhow!("No match pattern provided")),
        }
    }

    /// Get all ports which need to be scanned for a given service's match pattern
    /// This skips ports from endpoints/headers because we don't want to scan a port if it's just being used in an endpoint (unnecessary network request)
    /// There's logic to add any endpoint-specific ports into scanning in scan_ports_and_endpoints and the docker discovery equivalent
    pub fn ports(&self) -> Vec<PortType> {
        match self {
            Pattern::Port(port) => vec![*port],
            Pattern::AnyOf(patterns) | Pattern::AllOf(patterns) => {
                patterns.iter().flat_map(|p| p.ports().to_vec()).collect()
            }
            Pattern::Optional(pattern) => pattern.ports(),
            Pattern::ClientResponse(probe) => probe.ports(),
            Pattern::CertificateIssuer(_, ports) | Pattern::Recog(_, ports) => {
                ports.iter().map(|port| PortType::new_tcp(*port)).collect()
            }
            _ => vec![],
        }
    }

    /// The probe evidence this pattern reads as a positive signal.
    pub fn client_probes(&self) -> Vec<ClientProbe> {
        match self {
            Pattern::ClientResponse(probe) => vec![*probe],
            Pattern::AnyOf(patterns) | Pattern::AllOf(patterns) => {
                patterns.iter().flat_map(|p| p.client_probes()).collect()
            }
            Pattern::Optional(pattern) => pattern.client_probes(),
            _ => vec![],
        }
    }

    /// Get all endpoints which need to be scanned for a given service's match pattern
    pub fn endpoints(&self) -> Vec<Endpoint> {
        match self {
            Pattern::Endpoint(port_base, path, ..) => {
                vec![Endpoint::for_pattern(*port_base, path)]
            }
            Pattern::Header(port_base_opt, ..) => {
                // If a specific port is specified, create an endpoint for it
                // If no port is specified, we need at least one endpoint to exist
                // The actual endpoint will be provided by other patterns (Endpoint patterns)
                // or we'll use a default HTTP endpoint on port 80
                if let Some(port_base) = port_base_opt {
                    vec![Endpoint::for_pattern(*port_base, "/")]
                } else {
                    // Port-agnostic header check - needs at least one endpoint
                    // Return a default HTTP endpoint to ensure something gets scanned
                    vec![Endpoint::for_pattern(PortType::Http, "/")]
                }
            }
            Pattern::AnyOf(patterns) | Pattern::AllOf(patterns) => patterns
                .iter()
                .flat_map(|p| p.endpoints().to_vec())
                .collect(),
            Pattern::Optional(pattern) => pattern.endpoints(),
            // The page whose title, cookies and challenge the fingerprints read.
            Pattern::Recog(_, ports) => ports
                .iter()
                .map(|port| Endpoint::for_pattern(PortType::new_tcp(*port), "/"))
                .collect(),
            _ => vec![],
        }
    }

    /// Whether the pattern includes HTTP endpoint probes on raw-socket ports.
    /// Used to flag services whose detection depends on the `probe_raw_socket_ports` toggle.
    /// Whether `probe_raw_socket_ports` being off would stop this pattern matching.
    ///
    /// That setting drops 9100-9107 from the scan results entirely, so it governs any reference to
    /// one of those ports — a bare `Pattern::Port`, as JetDirect uses, as much as an `Endpoint`.
    /// Reading only the endpoints under-reported it: the setting exists because writing to 9100
    /// prints, and JetDirect was missing from the list of what it gates.
    pub fn gated_by_raw_socket_scanning(&self) -> bool {
        self.ports().iter().any(PortType::is_raw_socket)
            || self.endpoints().iter().any(|e| e.port_type.is_raw_socket())
    }

    /// Whether service uses IsGateway as a positive match signal -> service is_gateway = trues
    pub fn contains_gateway_ip_pattern(&self) -> bool {
        match self {
            Pattern::IsGateway => true,
            Pattern::AllOf(patterns) | Pattern::AnyOf(patterns) => {
                patterns.iter().any(|p| p.contains_gateway_ip_pattern())
            }
            _ => false,
        }
    }

    /// Whether a completed TCP connection to `port`, and nothing else, is enough for this pattern
    /// to match.
    ///
    /// This is the property a middlebox exploits. A firewall session helper, transparent proxy or
    /// IPS completes the handshake on behalf of an address that holds no device, and a definition
    /// resting on the connect alone then names a service there (GH: FortiGate SIP ALG report). A
    /// definition that reads a protocol response cannot be fooled the same way, because the
    /// middlebox would have to speak the protocol to satisfy it.
    ///
    /// Derived from the pattern rather than from a maintained list, so a definition added with a
    /// bare [`Pattern::Port`] is covered the day it lands and one that validates its port never is.
    /// [`ServiceDefinition::connect_only_rationale`] is what declares the deliberate exceptions,
    /// and the two are held equal by a test.
    ///
    /// UDP is out of scope: there is no connect to complete, and a UDP port is only ever reported
    /// open by an `AppProbe` that already validated it.
    pub fn matches_on_connect_alone(&self, port: PortType) -> bool {
        if !port.is_tcp() {
            return false;
        }
        match self {
            Pattern::Port(p) => *p == port,
            Pattern::AnyOf(patterns) => patterns.iter().any(|p| p.matches_on_connect_alone(port)),
            // Every arm has to be satisfiable by connects for the whole to be, and one of them has
            // to be the port in question — otherwise `AllOf([Port(80), Port(443)])` would report
            // itself connect-only for a port it never looks at.
            Pattern::AllOf(patterns) => {
                patterns.iter().all(|p| p.satisfiable_by_connect())
                    && patterns.iter().any(|p| p.references_port(port))
            }
            _ => false,
        }
    }

    /// Whether this pattern can be satisfied without evidence beyond completed TCP connections.
    ///
    /// The [`Pattern::AllOf`] recursion for [`Self::matches_on_connect_alone`]. Everything not
    /// listed is `false` because it demands evidence a middlebox cannot manufacture: an HTTP body
    /// or header match, a credentialed or application-probe response, an mDNS advert, a controller
    /// inventory entry. [`Pattern::MacVendor`] is `false` for the same reason from the other
    /// direction — it needs a MAC, which a routed address never has, so an OUI-gated definition is
    /// already self-limiting off-link. [`Pattern::Custom`] is `false` because its predicate is
    /// opaque; that also keeps the "Unclaimed Open Ports" catch-all from marking every port on the
    /// network as connect-only.
    fn satisfiable_by_connect(&self) -> bool {
        match self {
            Pattern::Port(_) => true,
            // Satisfied when the inner pattern *fails*, which needs no evidence at all.
            Pattern::Not(_) => true,
            // Satisfied whatever the inner pattern finds.
            Pattern::Optional(_) => true,
            Pattern::AnyOf(patterns) => patterns.iter().any(|p| p.satisfiable_by_connect()),
            Pattern::AllOf(patterns) => patterns.iter().all(|p| p.satisfiable_by_connect()),
            _ => false,
        }
    }

    /// Whether `port` appears anywhere in this pattern as a scanned port.
    fn references_port(&self, port: PortType) -> bool {
        self.ports().contains(&port)
    }
}

impl<'a> Pattern<'a> {
    /// The DNS-SD service types this pattern reads, so the mDNS browse can ask for each by name
    /// rather than rely on another device's enumeration reply to mention it.
    pub fn dns_sd_types(&self) -> Vec<&'a str> {
        match self {
            Pattern::DnsSd(service_type, _) => vec![*service_type],
            Pattern::AnyOf(patterns) | Pattern::AllOf(patterns) => {
                patterns.iter().flat_map(|p| p.dns_sd_types()).collect()
            }
            Pattern::Not(pattern) => pattern.dns_sd_types(),
            Pattern::Optional(pattern) => pattern.dns_sd_types(),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests;
