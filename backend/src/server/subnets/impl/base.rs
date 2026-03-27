use std::fmt::Display;
use std::net::Ipv4Addr;

use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::api::deserialize_empty_string_as_none;
use crate::server::shared::types::entities::{DiscoveryMetadata, EntitySource};
use crate::server::subnets::r#impl::types::SubnetType;
use chrono::{DateTime, Utc};
use cidr::{IpCidr, Ipv4Cidr};
use pnet::ipnetwork::IpNetwork;
use serde::de::Error as DeError;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{interfaces::r#impl::base::Interface, services::r#impl::base::Service};

fn deserialize_cidr<'de, D>(deserializer: D) -> Result<IpCidr, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<IpCidr>().map_err(|e| {
        let msg = e.to_string();
        if msg.contains("host part of address was not zero") {
            DeError::custom(format!(
                "Invalid CIDR '{}': address doesn't align with the subnet mask. Use a network address (e.g., for /24, the last octet should be 0).",
                s
            ))
        } else {
            DeError::custom(format!("Invalid CIDR '{}': {}", s, msg))
        }
    })
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct SubnetBase {
    #[schema(value_type = String)]
    #[serde(deserialize_with = "deserialize_cidr")]
    pub cidr: IpCidr,
    pub network_id: Uuid,
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    #[validate(length(min = 0, max = 500))]
    pub description: Option<String>,
    pub subnet_type: SubnetType,
    #[serde(default)]
    #[schema(required)]
    /// Will be automatically set to Manual for creation through API
    pub source: EntitySource,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

impl Default for SubnetBase {
    fn default() -> Self {
        Self {
            cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(192, 168, 4, 0), 24).unwrap()),
            name: "New Subnet".to_string(),
            network_id: Uuid::new_v4(),
            description: None,
            subnet_type: SubnetType::Unknown,
            source: EntitySource::Manual,
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::subnet)]
pub struct Subnet {
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
    pub base: SubnetBase,
}

impl Subnet {
    pub fn is_docker_bridge_subnet(&self) -> bool {
        self.base.subnet_type == SubnetType::DockerBridge
    }

    pub fn is_vpn_subnet(&self) -> bool {
        self.base.subnet_type == SubnetType::VpnTunnel
    }

    pub fn from_discovery(
        interface_name: String,
        ip_network: &IpNetwork,
        daemon_id: Uuid,
        discovery_type: &DiscoveryType,
        network_id: Uuid,
    ) -> Option<Self> {
        let mut subnet_type = SubnetType::from_interface_name(&interface_name);

        match ip_network {
            IpNetwork::V6(_) => None,
            IpNetwork::V4(ipv4_network) => {
                // Non-loopback CIDRs on loopback interfaces (e.g. 10.99.0.0/24 aliased
                // on lo0) are real networks, not loopback
                if subnet_type.is_loopback() && ipv4_network.ip().octets()[0] != 127 {
                    subnet_type = SubnetType::Unknown;
                }

                let (network_addr, prefix_len) = match (&subnet_type, ipv4_network.prefix()) {
                    // VPN tunnels with /32 -> expand to /24
                    (SubnetType::VpnTunnel, 32) => {
                        let ip_octets = ipv4_network.ip().octets();
                        let network_addr =
                            std::net::Ipv4Addr::new(ip_octets[0], ip_octets[1], ip_octets[2], 0);
                        (network_addr, 24)
                    }
                    // Skip other /32 single IPs
                    (_, 32) => return None,
                    // Normal case - use the network's actual network address and prefix
                    _ => (ipv4_network.network(), ipv4_network.prefix()),
                };

                let cidr = IpCidr::V4(Ipv4Cidr::new(network_addr, prefix_len).ok()?);

                Some(Subnet::new(SubnetBase {
                    cidr,
                    network_id,
                    description: None,
                    tags: Vec::new(),
                    name: cidr.to_string(),
                    subnet_type,
                    source: EntitySource::Discovery {
                        metadata: vec![DiscoveryMetadata::new(discovery_type.clone(), daemon_id)],
                    },
                }))
            }
        }
    }

    pub fn has_interface_with_service(
        &self,
        host_interfaces: &[&Interface],
        service: &Service,
    ) -> bool {
        service.base.bindings.iter().any(|binding| {
            host_interfaces.iter().any(|interface| {
                let interface_match = match binding.interface_id() {
                    Some(id) => interface.id == id,
                    None => true, // Listens on all interfaces
                };

                interface_match && interface.base.subnet_id == self.id
            })
        })
    }

    pub fn is_organizational_subnet(&self) -> bool {
        let organizational_cidr = IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 0).unwrap());
        self.base.cidr == organizational_cidr
    }
}

impl PartialEq for Subnet {
    fn eq(&self, other: &Self) -> bool {
        let network_match =
            self.base.cidr == other.base.cidr && self.base.network_id == other.base.network_id;

        network_match || self.id == other.id
    }
}

impl Hash for Subnet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.cidr.hash(state);
    }
}

impl Display for Subnet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Subnet {}: {}", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<Subnet> for Subnet {
    fn triggers_staleness(&self, _other: Option<Subnet>) -> bool {
        false
    }
}
