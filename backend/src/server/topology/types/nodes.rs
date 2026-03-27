use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::topology::types::edges::Edge;
use crate::server::topology::types::layout::{Ixy, Uxy};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, ToSchema)]
pub struct Node {
    #[serde(flatten)]
    pub node_type: NodeType,
    pub id: Uuid,
    pub position: Ixy,
    pub size: Uxy,
    pub header: Option<String>,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumDiscriminants,
    IntoStaticStr,
    ToSchema,
)]
#[serde(tag = "node_type")]
#[strum_discriminants(derive(Display, Hash, Serialize, Deserialize, EnumIter))]
pub enum NodeType {
    SubnetNode {
        infra_width: usize,
    },
    InterfaceNode {
        subnet_id: Uuid,
        host_id: Uuid,
        interface_id: Option<Uuid>,
        is_infra: bool,
    },
}

#[derive(Debug, Clone)]
pub struct SubnetChild {
    pub id: Uuid,
    pub header: Option<String>,
    pub host_id: Uuid,
    pub interface_id: Option<Uuid>,
    pub size: Uxy,
    pub edges: Vec<Edge>,
}

impl SubnetType {
    pub fn vertical_order(&self) -> usize {
        match self {
            // Layer 0: External
            SubnetType::Internet => 0,
            SubnetType::Remote => 0,

            // Layer 1: Gateway/DMZ
            SubnetType::Gateway => 1,
            SubnetType::Dmz => 1, // Same layer as Gateway
            SubnetType::VpnTunnel => 1,

            // Layer 2: Internal
            SubnetType::Lan => 2,
            SubnetType::WiFi => 2,
            SubnetType::Guest => 2,
            SubnetType::IoT => 2,

            // Layer 3: Infrastructure
            SubnetType::DockerBridge => 3,
            SubnetType::MacVlan => 3,
            SubnetType::IpVlan => 3,
            SubnetType::Management => 3,
            SubnetType::Storage => 3,

            // Special
            SubnetType::Loopback => 999,
            SubnetType::Unknown => 999,
        }
    }

    pub fn horizontal_order(&self) -> usize {
        match self {
            // Layer 0
            SubnetType::Internet => 0,
            SubnetType::Remote => 1,

            // Layer 1 - Gateway is central, DMZ to the side
            SubnetType::Gateway => 0,   // Center/left
            SubnetType::Dmz => 1,       // Right of gateway
            SubnetType::VpnTunnel => 2, // Further right

            // Layer 2
            SubnetType::Lan => 0,
            SubnetType::WiFi => 1,
            SubnetType::IoT => 2,
            SubnetType::Guest => 3,

            // Layer 3
            SubnetType::Storage => 0,
            SubnetType::Management => 1,
            SubnetType::DockerBridge => 2,
            SubnetType::MacVlan => 3,
            SubnetType::IpVlan => 4,

            // Special
            SubnetType::Loopback => 999,
            SubnetType::Unknown => 999,
        }
    }
}
