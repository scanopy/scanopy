use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::{fmt::Display, str::FromStr};
use strum_macros::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants};
use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

#[derive(
    Copy,
    Debug,
    Clone,
    PartialOrd,
    Ord,
    Default,
    Display,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
)]
pub enum TransportProtocol {
    #[default]
    Udp,
    Tcp,
}

/// The base data for a Port entity (everything except id, created_at, updated_at)
#[derive(Copy, Debug, Clone, Eq, Validate, Serialize, Deserialize, ToSchema)]
pub struct PortBase {
    pub host_id: Uuid,
    pub network_id: Uuid,
    #[serde(flatten)]
    #[schema(required)]
    pub port_type: PortType,
}

impl PortBase {
    pub fn new(host_id: Uuid, network_id: Uuid, port_type: PortType) -> Self {
        Self {
            host_id,
            network_id,
            port_type,
        }
    }

    /// Create a PortBase without host/network (will be set by server)
    pub fn new_hostless(port_type: PortType) -> Self {
        Self {
            host_id: Uuid::nil(),
            network_id: Uuid::nil(),
            port_type,
        }
    }
}

impl Default for PortBase {
    fn default() -> Self {
        Self::new_hostless(PortType::default())
    }
}

impl PartialEq for PortBase {
    fn eq(&self, other: &Self) -> bool {
        self.port_type == other.port_type
    }
}

impl Hash for PortBase {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.port_type.hash(state);
    }
}

/// Port entity with custom serialization that flattens PortType fields.
#[derive(Copy, Debug, Validate, Clone, Eq, Serialize, Deserialize, ToSchema)]
#[schema(example = crate::server::shared::types::examples::port)]
pub struct Port {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: PortBase,
}

impl Hash for Port {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.hash(state);
    }
}

impl PartialEq for Port {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}

impl Default for Port {
    fn default() -> Self {
        Self::new_hostless(PortType::default())
    }
}

impl ChangeTriggersTopologyStaleness<Port> for Port {
    fn triggers_staleness(&self, other: Option<Port>) -> bool {
        if let Some(other_port) = other {
            self.base.port_type != other_port.base.port_type
                || self.base.host_id != other_port.base.host_id
                || self.base.port_type.config() != other_port.base.port_type.config()
        } else {
            true // New or deleted port triggers staleness
        }
    }
}

impl Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.base.port_type, self.id)
    }
}

impl Port {
    pub fn new(base: PortBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }

    /// Create a Port with just a PortType (host_id/network_id set to nil).
    /// Use this for ports created during discovery before host assignment.
    pub fn new_hostless(port_type: PortType) -> Self {
        Self::new(PortBase::new_hostless(port_type))
    }

    // Convenience accessors
    pub fn host_id(&self) -> Uuid {
        self.base.host_id
    }

    pub fn network_id(&self) -> Uuid {
        self.base.network_id
    }

    pub fn port_type(&self) -> PortType {
        self.base.port_type
    }

    /// Set the host_id and network_id (for hostless ports that get resolved later)
    pub fn with_host(mut self, host_id: Uuid, network_id: Uuid) -> Self {
        self.base.host_id = host_id;
        self.base.network_id = network_id;
        self
    }
}

/// The type of port - predefined well-known ports or custom
/// Custom serialization outputs: {number, protocol, type} for flattening into Port
#[derive(Copy, Debug, Clone, Eq, EnumDiscriminants, EnumIter, IntoStaticStr, Default)]
#[strum_discriminants(derive(Display, Hash, EnumIter))]
pub enum PortType {
    Ssh,
    Telnet,
    DnsUdp,
    DnsTcp,
    Samba,
    Nfs,
    Ftp,
    Ipp,
    LdpTcp,
    LdpUdp,
    Ldap,
    Ldaps,
    Kerberos,
    Snmp,
    SnmpAlt,
    Rdp,
    Ntp,
    Sip,
    SipTls,
    Rtsp,
    Dhcp,
    #[default]
    Http,
    MySql,
    PostgreSQL,
    MongoDB,
    Redis,
    MsSql,
    Docker,
    DockerTls,
    Kubernetes,
    RabbitMqMgmt,
    Cassandra,
    Elasticsearch,
    InfluxDb,
    CouchDb,
    Kafka,
    Http3000,
    Http5000,
    Http8080,
    Http8081,
    Http8082,
    Http8888,
    Http9000,
    Https,
    Https8443,
    Https9443,
    Https10443,
    Mqtt,
    MqttTls,
    AMQP,
    AMQPTls,
    Wireguard,
    OpenVPN,
    BACnet,
    JetDirect,
    Custom(PortConfig),
}

impl Hash for PortType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.config().hash(state);
    }
}

impl PartialEq for PortType {
    fn eq(&self, other: &Self) -> bool {
        self.config() == other.config()
    }
}

impl Display for PortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}",
            self.number(),
            self.protocol().to_string().to_lowercase()
        )
    }
}

impl FromStr for PortType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"(?i)\b(\d+)\s*[/\-\s:]*\s*(tcp|udp)\b").map_err(|e| e.to_string())?;

        if let Some(caps) = re.captures(s.trim()) {
            let number = caps
                .get(1)
                .ok_or("Missing port number")?
                .as_str()
                .parse::<u16>()
                .map_err(|_| "Invalid port number")?;

            let proto_string = caps
                .get(2)
                .ok_or("Missing protocol")?
                .as_str()
                .to_lowercase();

            let protocol = match proto_string.as_str() {
                "tcp" => TransportProtocol::Tcp,
                "udp" => TransportProtocol::Udp,
                _ => return Err("Unknown protocol".into()),
            };

            Ok(PortType::Custom(PortConfig { number, protocol }))
        } else {
            Err("Failed to parse port and protocol".into())
        }
    }
}

#[derive(Copy, Debug, Clone, Validate, Default, Eq, Serialize, Deserialize, ToSchema)]
pub struct PortConfig {
    #[validate(range(min = 1, max = 65535))]
    pub number: u16,
    pub protocol: TransportProtocol,
}

impl PartialEq for PortConfig {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number && self.protocol == other.protocol
    }
}

impl Hash for PortConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.number.hash(state);
        self.protocol.hash(state);
    }
}

impl PortType {
    pub fn new(number: u16, protocol: TransportProtocol) -> Self {
        PortType::Custom(PortConfig { number, protocol })
    }

    pub fn is_tcp(&self) -> bool {
        self.protocol() == TransportProtocol::Tcp
    }

    pub fn is_udp(&self) -> bool {
        self.protocol() == TransportProtocol::Udp
    }

    pub fn new_tcp(number: u16) -> Self {
        PortType::Custom(PortConfig {
            number,
            protocol: TransportProtocol::Tcp,
        })
    }

    pub fn new_udp(number: u16) -> Self {
        PortType::Custom(PortConfig {
            number,
            protocol: TransportProtocol::Udp,
        })
    }

    pub fn protocol(&self) -> TransportProtocol {
        self.config().protocol
    }

    pub fn number(&self) -> u16 {
        self.config().number
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, PortType::Custom(_))
    }

    pub fn is_https(&self) -> bool {
        matches!(
            self,
            PortType::Https | PortType::Https10443 | PortType::Https8443 | PortType::Https9443
        )
    }

    /// Ports with raw-socket protocols that interpret any TCP data as input.
    /// HTTP probes on these ports cause unintended side effects (e.g. ghost printing).
    /// Matches nmap's Exclude T:9100-9107.
    pub fn is_raw_socket(&self) -> bool {
        (9100..=9107).contains(&self.number())
    }

    pub fn config(&self) -> PortConfig {
        match &self {
            PortType::Ssh => PortConfig {
                number: 22,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Telnet => PortConfig {
                number: 23,
                protocol: TransportProtocol::Tcp,
            },
            PortType::DnsTcp => PortConfig {
                number: 53,
                protocol: TransportProtocol::Tcp,
            },
            PortType::DnsUdp => PortConfig {
                number: 53,
                protocol: TransportProtocol::Udp,
            },
            PortType::Samba => PortConfig {
                number: 445,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Sip => PortConfig {
                number: 5060,
                protocol: TransportProtocol::Tcp,
            },
            PortType::SipTls => PortConfig {
                number: 5061,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Ldap => PortConfig {
                number: 389,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Ldaps => PortConfig {
                number: 636,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Kerberos => PortConfig {
                number: 88,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Nfs => PortConfig {
                number: 2049,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Ftp => PortConfig {
                number: 21,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Ipp => PortConfig {
                number: 631,
                protocol: TransportProtocol::Tcp,
            },
            PortType::LdpTcp => PortConfig {
                number: 515,
                protocol: TransportProtocol::Tcp,
            },
            PortType::LdpUdp => PortConfig {
                number: 515,
                protocol: TransportProtocol::Udp,
            },
            PortType::Snmp => PortConfig {
                number: 161,
                protocol: TransportProtocol::Udp,
            },
            PortType::SnmpAlt => PortConfig {
                number: 1161,
                protocol: TransportProtocol::Udp,
            },
            PortType::Rdp => PortConfig {
                number: 3389,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Ntp => PortConfig {
                number: 123,
                protocol: TransportProtocol::Udp,
            },
            PortType::Rtsp => PortConfig {
                number: 554,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Dhcp => PortConfig {
                number: 67,
                protocol: TransportProtocol::Udp,
            },
            PortType::Http => PortConfig {
                number: 80,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Http8080 => PortConfig {
                number: 8080,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Https => PortConfig {
                number: 443,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Https8443 => PortConfig {
                number: 8443,
                protocol: TransportProtocol::Tcp,
            },
            PortType::MySql => PortConfig {
                number: 3306,
                protocol: TransportProtocol::Tcp,
            },
            PortType::PostgreSQL => PortConfig {
                number: 5432,
                protocol: TransportProtocol::Tcp,
            },
            PortType::MongoDB => PortConfig {
                number: 27017,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Redis => PortConfig {
                number: 6379,
                protocol: TransportProtocol::Tcp,
            },
            PortType::MsSql => PortConfig {
                number: 1433,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Docker => PortConfig {
                number: 2375,
                protocol: TransportProtocol::Tcp,
            },
            PortType::DockerTls => PortConfig {
                number: 2376,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Kubernetes => PortConfig {
                number: 6443,
                protocol: TransportProtocol::Tcp,
            },
            PortType::RabbitMqMgmt => PortConfig {
                number: 15672,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Kafka => PortConfig {
                number: 9092,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Http3000 => PortConfig {
                number: 3000,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Http5000 => PortConfig {
                number: 5000,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Http8081 => PortConfig {
                number: 8081,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Http8082 => PortConfig {
                number: 8082,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Http8888 => PortConfig {
                number: 8888,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Http9000 => PortConfig {
                number: 9000,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Https9443 => PortConfig {
                number: 9443,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Https10443 => PortConfig {
                number: 10443,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Cassandra => PortConfig {
                number: 9042,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Elasticsearch => PortConfig {
                number: 9200,
                protocol: TransportProtocol::Tcp,
            },
            PortType::InfluxDb => PortConfig {
                number: 8086,
                protocol: TransportProtocol::Tcp,
            },
            PortType::CouchDb => PortConfig {
                number: 5984,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Custom(config) => *config,
            PortType::Mqtt => PortConfig {
                number: 1883,
                protocol: TransportProtocol::Tcp,
            },
            PortType::MqttTls => PortConfig {
                number: 8883,
                protocol: TransportProtocol::Tcp,
            },
            PortType::AMQP => PortConfig {
                number: 5672,
                protocol: TransportProtocol::Tcp,
            },
            PortType::AMQPTls => PortConfig {
                number: 5671,
                protocol: TransportProtocol::Tcp,
            },
            PortType::Wireguard => PortConfig {
                number: 51820,
                protocol: TransportProtocol::Udp,
            },
            PortType::OpenVPN => PortConfig {
                number: 1194,
                protocol: TransportProtocol::Udp,
            },
            PortType::BACnet => PortConfig {
                number: 47808,
                protocol: TransportProtocol::Udp,
            },
            PortType::JetDirect => PortConfig {
                number: 9100,
                protocol: TransportProtocol::Tcp,
            },
        }
    }
}

impl HasId for PortType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for PortType {
    fn color(&self) -> Color {
        EntityDiscriminants::Port.color()
    }
    fn icon(&self) -> Icon {
        EntityDiscriminants::Port.icon()
    }
}

impl TypeMetadataProvider for PortType {
    fn name(&self) -> &'static str {
        match self {
            PortType::Ssh => "SSH",
            PortType::Telnet => "Telnet",
            PortType::DnsUdp => "DNS (UDP)",
            PortType::DnsTcp => "DNS (TCP)",
            PortType::Samba => "Samba",
            PortType::Nfs => "NFS",
            PortType::Ftp => "FTP",
            PortType::Ipp => "IPP",
            PortType::LdpTcp => "LDP (TCP)",
            PortType::LdpUdp => "LDP (UDP)",
            PortType::Snmp => "SNMP",
            PortType::SnmpAlt => "SNMP (Alt)",
            PortType::Ldap => "LDAP",
            PortType::Ldaps => "LDAP TLS",
            PortType::Kerberos => "Kerberos",
            PortType::Rdp => "RDP",
            PortType::Ntp => "NTP",
            PortType::Sip => "SIP",
            PortType::SipTls => "SIP TLS",
            PortType::Rtsp => "RTSP",
            PortType::Dhcp => "DHCP",
            PortType::Http => "HTTP",
            PortType::Http8080 => "HTTP 8080",
            PortType::Https => "HTTPS",
            PortType::Https8443 => "HTTPS 8443",
            PortType::Custom(_) => "Custom",
            PortType::MySql => "MySql",
            PortType::PostgreSQL => "PostgreSql",
            PortType::MongoDB => "MongoDB",
            PortType::Redis => "Redis",
            PortType::MsSql => "MicrosoftSql",
            PortType::Docker => "Docker",
            PortType::DockerTls => "Docker TLS",
            PortType::Kubernetes => "Kubernetes",
            PortType::RabbitMqMgmt => "RabbitMq Management",
            PortType::Kafka => "Kafka",
            PortType::Http3000 => "HTTP 3000",
            PortType::Http5000 => "HTTP 5000",
            PortType::Http8081 => "HTTP 8081",
            PortType::Http8082 => "HTTP 8082",
            PortType::Http8888 => "HTTP 8888",
            PortType::Http9000 => "HTTP 9000",
            PortType::Https9443 => "HTTP 9443",
            PortType::Https10443 => "HTTP 10443",
            PortType::Cassandra => "Cassandra",
            PortType::Elasticsearch => "Elastic Search",
            PortType::InfluxDb => "InfluxDB",
            PortType::CouchDb => "CouchDB",
            PortType::Mqtt => "MQTT",
            PortType::MqttTls => "MQTT TLS",
            PortType::AMQP => "AMQP",
            PortType::AMQPTls => "AMQP TLS",
            PortType::Wireguard => "Wireguard",
            PortType::OpenVPN => "OpenVPN",
            PortType::BACnet => "BACnet",
            PortType::JetDirect => "JetDirect",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            PortType::Ssh => "Secure Shell",
            PortType::Telnet => "Telnet Protocol",
            PortType::DnsUdp => "Domain Name System (UDP)",
            PortType::DnsTcp => "Domain Name System (TCP)",
            PortType::Samba => "Samba File Sharing",
            PortType::Nfs => "Network File System",
            PortType::Ftp => "File Transfer Protocol",
            PortType::Ipp => "Internet Printing Protocol",
            PortType::Ldap => "Lightweight Directory Access Protocol",
            PortType::Ldaps => "Lightweight Directory Access Protocol using TLS",
            PortType::Sip => "Session Initiation Protocol",
            PortType::SipTls => "Session Initiation Protocol using TLS",
            PortType::Kerberos => "Kerberos Authentication Protocol",
            PortType::LdpTcp => "Line Printer Daemon (TCP)",
            PortType::LdpUdp => "Line Printer Daemon (UDP)",
            PortType::Snmp => "Simple Network Management Protocol",
            PortType::SnmpAlt => {
                "Simple Network Management Protocol (non-privileged alternate port)"
            }
            PortType::Rdp => "Remote Desktop Protocol",
            PortType::Ntp => "Network Time Protocol",
            PortType::Rtsp => "Real-Time Streaming Protocol",
            PortType::Dhcp => "Dynamic Host Configuration Protocol",
            PortType::Http => "Hypertext Transfer Protocol",
            PortType::Http8080 => "HTTP 8080",
            PortType::Https => "Hypertext Transfer Protocol Secure",
            PortType::Https8443 => "Alternative HTTPS Port",
            PortType::MySql => "MySQL Database",
            PortType::PostgreSQL => "PostgreSQL Database",
            PortType::MongoDB => "MongoDB",
            PortType::Redis => "Redis Database",
            PortType::MsSql => "MicrosoftSQL Database",
            PortType::Docker => "Docker",
            PortType::DockerTls => "Docker using TLS",
            PortType::Kubernetes => "Kubernetes",
            PortType::RabbitMqMgmt => "RabbitMQ Management",
            PortType::Kafka => "Kafka",
            PortType::Http3000 => "HTTP 3000",
            PortType::Http5000 => "HTTP 5000",
            PortType::Http8081 => "HTTP 8081",
            PortType::Http8082 => "HTTP 8082",
            PortType::Http8888 => "HTTP 8888",
            PortType::Http9000 => "HTTP 9000",
            PortType::Https9443 => "HTTPS 9443",
            PortType::Https10443 => "HTTPS 10443",
            PortType::Custom(_) => "Custom Port Configuration",
            PortType::Cassandra => "Cassandra",
            PortType::Elasticsearch => "Elastic Search",
            PortType::InfluxDb => "Influx Database",
            PortType::CouchDb => "Couch Database",
            PortType::Mqtt => "MQTT",
            PortType::MqttTls => "MQTT using TLS",
            PortType::AMQP => "Advanced Message Queuing Protocol",
            PortType::AMQPTls => "Advanced Message Queuing Protocol using TLS",
            PortType::Wireguard => "Wireguard VPN",
            PortType::OpenVPN => "OpenVPN",
            PortType::BACnet => "Building Automation and Control Network",
            PortType::JetDirect => "JetDirect RAW Printing",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let is_management = matches!(
            self,
            PortType::Ssh
                | PortType::Telnet
                | PortType::Rdp
                | PortType::Snmp
                | PortType::SnmpAlt
                | PortType::Http
                | PortType::Https
                | PortType::Http8080
                | PortType::Https8443
        );

        let is_dns = matches!(self, PortType::DnsUdp | PortType::DnsTcp);

        let number = self.number();
        let protocol = self.protocol();

        let can_be_added = !matches!(self, PortType::Custom(_));

        let is_custom = self.is_custom();

        serde_json::json!({
            "is_management": is_management,
            "is_custom": is_custom,
            "is_dns": is_dns,
            "can_be_added": can_be_added,
            "number": number,
            "protocol": protocol
        })
    }
}

impl Serialize for PortType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let config = self.config();
        let mut state = serializer.serialize_struct("PortType", 3)?;
        state.serialize_field("number", &config.number)?;
        state.serialize_field("protocol", &config.protocol)?;
        state.serialize_field("type", &self.id())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PortType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use strum::IntoEnumIterator;

        #[derive(Deserialize)]
        struct TempPortType {
            number: u16,
            protocol: TransportProtocol,
            #[serde(rename = "type")]
            #[allow(dead_code)]
            port_type: String,
        }

        let temp = TempPortType::deserialize(deserializer)?;

        // Try to find a matching predefined port type
        let port_type = PortType::iter()
            .find(|variant| {
                // Skip Custom variants during iteration
                if matches!(variant, PortType::Custom(_)) {
                    return false;
                }
                let config = variant.config();
                config.number == temp.number && config.protocol == temp.protocol
            })
            .unwrap_or({
                // If no predefined port matches, create a Custom variant
                PortType::Custom(PortConfig {
                    number: temp.number,
                    protocol: temp.protocol,
                })
            });

        Ok(port_type)
    }
}

/// Manual ToSchema implementation for PortType since it has custom serialization
/// PortType serializes to {number: u16, protocol: TransportProtocol, type: String}
/// On create, only number+protocol are required; type is auto-derived from them.
impl utoipa::PartialSchema for PortType {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
        use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
        use utoipa::openapi::{RefOr, Schema};

        RefOr::T(Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::new(Type::Object))
                .property(
                    "number",
                    ObjectBuilder::new()
                        .schema_type(SchemaType::new(Type::Integer))
                        .build(),
                )
                .property("protocol", TransportProtocol::schema())
                .property(
                    "type",
                    ObjectBuilder::new()
                        .schema_type(SchemaType::new(Type::String))
                        .description(Some(
                            "Auto-derived from number+protocol; optional on create",
                        ))
                        .build(),
                )
                .required("number")
                .required("protocol")
                .description(Some(
                    "Port type with number, protocol, and optional type identifier",
                ))
                .build(),
        ))
    }
}

impl utoipa::ToSchema for PortType {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("PortType")
    }
}
