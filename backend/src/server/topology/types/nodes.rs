use crate::server::shared::concepts::Concept;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::types::metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider};
use crate::server::shared::types::{Color, Icon};
use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::topology::types::grouping::InlineGroup;
use crate::server::topology::types::layout::{Ixy, Uxy};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, ToSchema)]
pub struct Node {
    /// Whether this node is a container or an element, and what it stands for.
    #[serde(flatten)]
    pub node_type: NodeType,
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// Where the node sits in the layout.
    pub position: Ixy,
    /// Width and height of the node.
    pub size: Uxy,
    /// Heading drawn at the top of a container node.
    pub header: Option<String>,
}

impl Node {
    pub fn element(
        id: Uuid,
        container_id: Uuid,
        host_id: Uuid,
        element: ElementEntityType,
    ) -> Self {
        Self {
            id,
            node_type: NodeType::Element {
                container_id,
                host_id,
                element,
                inline_groups: Vec::new(),
            },
            position: Ixy::default(),
            size: Uxy::default(),
            header: None,
        }
    }
}

/// How the container's title is rendered in the topology viewer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub enum TitleStyle {
    /// Card/pill positioned above the container (subnets)
    External,
    /// Inside the container's top padding area (subcontainers)
    Inline,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    ToSchema,
    EnumIter,
    IntoStaticStr,
)]
pub enum ContainerType {
    // Top-level containers
    #[default]
    Subnet,
    ServiceCategory,
    Application,
    /// Application-view container for services with no application tag. Same
    /// display/layout as `Application` but collapsed by default (it tends to be
    /// large and noisy). See `metadata()` `collapsed_by_default`.
    ApplicationUngrouped,
    /// Generic root container for perspectives without structural container rules.
    Root,

    /// Host container for L2 Physical view (groups ports by their network device)
    Host,

    // Subcontainers (nested inside a top-level container)
    NestedTag,
    NestedServiceCategory,
    Hypervisor,
    ContainerRuntime,
    Stack,
    TrunkPort,
    VLAN,
    PortOpStatus,
}

impl HasId for ContainerType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ContainerType {
    fn color(&self) -> Color {
        match self {
            ContainerType::Subnet => Color::Blue,
            ContainerType::ServiceCategory => EntityDiscriminants::Service.color(),
            ContainerType::Application | ContainerType::ApplicationUngrouped => {
                Concept::Application.color()
            }
            ContainerType::Root => Color::Gray,
            ContainerType::Host => EntityDiscriminants::Host.color(),
            ContainerType::NestedTag => Color::Orange,
            ContainerType::NestedServiceCategory => Color::Purple,
            ContainerType::Hypervisor => Concept::Virtualization.color(),
            ContainerType::ContainerRuntime => Concept::Containerization.color(),
            ContainerType::Stack => Concept::Containerization.color(),
            ContainerType::TrunkPort => Color::Amber,
            ContainerType::VLAN => Color::Teal,
            ContainerType::PortOpStatus => Color::Gray,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            ContainerType::Subnet => Icon::Network,
            ContainerType::ServiceCategory => EntityDiscriminants::Service.icon(),
            ContainerType::Application | ContainerType::ApplicationUngrouped => {
                Concept::Application.icon()
            }
            ContainerType::Root => Icon::Layers,
            ContainerType::Host => Concept::L2.icon(),
            ContainerType::NestedTag => Icon::Tag,
            ContainerType::NestedServiceCategory => Icon::Layers,
            ContainerType::Hypervisor => Concept::Virtualization.icon(),
            ContainerType::ContainerRuntime => Concept::Containerization.icon(),
            ContainerType::Stack => Concept::Containerization.icon(),
            ContainerType::TrunkPort => Icon::Network,
            ContainerType::VLAN => Icon::Network,
            ContainerType::PortOpStatus => Icon::Circle,
        }
    }
}

impl TypeMetadataProvider for ContainerType {
    fn name(&self) -> &'static str {
        match self {
            ContainerType::Subnet => "Subnet",
            ContainerType::ServiceCategory => "Service category",
            ContainerType::Application => "Application",
            ContainerType::ApplicationUngrouped => "Ungrouped",
            ContainerType::Root => "Root",
            ContainerType::Host => "Host",
            ContainerType::NestedTag => "Tag container",
            ContainerType::NestedServiceCategory => "Service category container",
            ContainerType::Hypervisor => "Hypervisor",
            ContainerType::ContainerRuntime => "Container runtime",
            ContainerType::Stack => "Stack",
            ContainerType::TrunkPort => "Trunk ports",
            ContainerType::VLAN => "VLAN",
            ContainerType::PortOpStatus => "Port status",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ContainerType::Subnet => "Network subnet container",
            ContainerType::ServiceCategory => "Services grouped by category",
            ContainerType::Application => "Services grouped by application tag",
            ContainerType::ApplicationUngrouped => "Services not assigned to an application",
            ContainerType::Root => "Root container",
            ContainerType::Host => "Physical network device",
            ContainerType::NestedTag => "Elements grouped by tag",
            ContainerType::NestedServiceCategory => "Elements grouped by service category",
            ContainerType::Hypervisor => "VMs grouped by hypervisor",
            ContainerType::ContainerRuntime => "Containers grouped by runtime",
            ContainerType::Stack => "Workloads deployed together as one unit",
            ContainerType::TrunkPort => "Trunk ports carrying multiple VLANs",
            ContainerType::VLAN => "Access ports grouped by native VLAN",
            ContainerType::PortOpStatus => "Ports grouped by operational status",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let is_subcontainer = matches!(
            self,
            ContainerType::NestedTag
                | ContainerType::NestedServiceCategory
                | ContainerType::Hypervisor
                | ContainerType::ContainerRuntime
                | ContainerType::Stack
                | ContainerType::TrunkPort
                | ContainerType::VLAN
                | ContainerType::PortOpStatus
        );
        let title_style = if is_subcontainer {
            TitleStyle::Inline
        } else {
            TitleStyle::External
        };
        let padding_top = if is_subcontainer { 50 } else { 25 };
        let (collapsed_width, collapsed_height) = if is_subcontainer {
            (250, 40)
        } else {
            (200, 80)
        };
        let fill_icon = matches!(self, ContainerType::PortOpStatus);
        let collapsed_by_default = matches!(
            self,
            ContainerType::PortOpStatus | ContainerType::ApplicationUngrouped
        );
        serde_json::json!({
            "title_style": title_style,
            "is_subcontainer": is_subcontainer,
            "is_collapsible": true,
            "has_border": true,
            "fill_icon": fill_icon,
            "collapsed_by_default": collapsed_by_default,
            "padding": { "top": padding_top, "left": 25, "bottom": 25, "right": 25 },
            "collapsed_size": { "width": collapsed_width, "height": collapsed_height },
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
#[serde(tag = "element_type")]
pub enum ElementEntityType {
    #[schema(title = "IPAddress")]
    IPAddress {
        /// Subnet the address sits in.
        subnet_id: Uuid,
        /// The IP address itself, when one is known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ip_address_id: Option<Uuid>,
    },
    #[schema(title = "Service")]
    Service {},
    #[schema(title = "Host")]
    Host {},
    #[schema(title = "Interface")]
    Interface {
        /// The interface this element stands for.
        interface_id: Uuid,
    },
}

impl Default for ElementEntityType {
    fn default() -> Self {
        Self::IPAddress {
            subnet_id: Uuid::nil(),
            ip_address_id: None,
        }
    }
}

impl From<&ElementEntityType> for EntityDiscriminants {
    fn from(eet: &ElementEntityType) -> Self {
        match eet {
            ElementEntityType::IPAddress { .. } => EntityDiscriminants::IPAddress,
            ElementEntityType::Service {} => EntityDiscriminants::Service,
            ElementEntityType::Host {} => EntityDiscriminants::Host,
            ElementEntityType::Interface { .. } => EntityDiscriminants::Interface,
        }
    }
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
    #[schema(title = "Container")]
    Container {
        /// What this container groups — a host, a subnet, an application, and so on.
        #[serde(default)]
        container_type: ContainerType,
        /// Container this one nests inside, for subcontainers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_container_id: Option<Uuid>,
        /// The entity this container represents (e.g. host ID for Host containers,
        /// subnet ID for Subnet containers). Used for ownership mapping on the frontend.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        entity_id: Option<Uuid>,
        /// Display icon name (set by graph builder from the source entity, e.g. subnet type)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        /// Display color (set by graph builder from the source entity, e.g. subnet type)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<Color>,
        /// Service definition ID for logo rendering (e.g. "Docker", "Proxmox VE").
        /// Used by Hypervisor and Stack subcontainers to show the service's logo.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        associated_service_definition: Option<String>,
        /// ID of the element rule that created this container (for subcontainers like NestedTag, Hypervisor, etc.)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        element_rule_id: Option<Uuid>,
        /// When true, this container accepts edges with `will_target_container`, causing
        /// them to visually attach here instead of at elements inside.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        will_accept_edges: bool,
    },
    #[schema(title = "Element")]
    Element {
        /// Container this element is drawn inside.
        #[serde(default)]
        container_id: Uuid,
        /// Host the element belongs to.
        host_id: Uuid,
        #[serde(flatten)]
        element: ElementEntityType,
        /// Visual grouping metadata for services inlined on this element.
        /// Populated by element rules (e.g., Docker containers on a VM host
        /// get InlineGroups with Header/Member roles for dotted-border rendering).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        inline_groups: Vec<InlineGroup>,
    },
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
            SubnetType::PodmanBridge => 3,
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
            SubnetType::PodmanBridge => 2,
            SubnetType::MacVlan => 3,
            SubnetType::IpVlan => 4,

            // Special
            SubnetType::Loopback => 999,
            SubnetType::Unknown => 999,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_ungrouped_is_collapsed_by_default() {
        // Ungrouped tiles/renders like a top-level Application container but
        // starts collapsed; the rest of its metadata must match Application.
        let meta = ContainerType::ApplicationUngrouped.metadata();
        assert_eq!(meta["collapsed_by_default"], true);
        assert_eq!(meta["is_subcontainer"], false);
        let app = ContainerType::Application.metadata();
        assert_eq!(meta["padding"], app["padding"]);
        assert_eq!(meta["collapsed_size"], app["collapsed_size"]);
        // Application stays expandable-by-default — only the Ungrouped variant flips.
        assert_eq!(app["collapsed_by_default"], false);
    }

    #[test]
    fn test_container_round_trip() {
        let node_type = NodeType::Container {
            container_type: ContainerType::Subnet,
            parent_container_id: None,
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["node_type"], "Container");
        assert_eq!(json["container_type"], "Subnet");
        assert!(json.get("parent_container_id").is_none());

        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_container_with_parent() {
        let parent_id = Uuid::new_v4();
        let node_type = NodeType::Container {
            container_type: ContainerType::NestedServiceCategory,
            parent_container_id: Some(parent_id),
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["container_type"], "NestedServiceCategory");
        assert_eq!(json["parent_container_id"], parent_id.to_string());

        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_container_element_rule_id_round_trip() {
        let rule_id = Uuid::new_v4();
        let node_type = NodeType::Container {
            container_type: ContainerType::NestedTag,
            parent_container_id: None,
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: Some(rule_id),
            will_accept_edges: true,
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["element_rule_id"], rule_id.to_string());
        assert_eq!(json["will_accept_edges"], true);

        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_element_ip_address_round_trip() {
        let container_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let subnet_id = Uuid::new_v4();
        let iface_id = Uuid::new_v4();
        let node_type = NodeType::Element {
            container_id,
            host_id,
            element: ElementEntityType::IPAddress {
                subnet_id,
                ip_address_id: Some(iface_id),
            },
            inline_groups: Vec::new(),
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["node_type"], "Element");
        assert_eq!(json["element_type"], "IPAddress");
        assert_eq!(json["subnet_id"], subnet_id.to_string());
        assert_eq!(json["host_id"], host_id.to_string());
        assert_eq!(json["ip_address_id"], iface_id.to_string());

        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_element_service_round_trip() {
        let container_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let node_type = NodeType::Element {
            container_id,
            host_id,
            element: ElementEntityType::Service {},
            inline_groups: Vec::new(),
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["node_type"], "Element");
        assert_eq!(json["element_type"], "Service");
        // Service elements don't have subnet_id or ip_address_id
        assert!(json.get("subnet_id").is_none());
        assert!(json.get("ip_address_id").is_none());

        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_element_interface_backward_compat() {
        // Verify that Interface elements serialize the same as before the restructure
        let container_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let subnet_id = Uuid::new_v4();
        let node_type = NodeType::Element {
            container_id,
            host_id,
            element: ElementEntityType::IPAddress {
                subnet_id,
                ip_address_id: None,
            },
            inline_groups: Vec::new(),
        };
        let json = serde_json::to_value(&node_type).unwrap();
        // All fields should be at the top level (flattened)
        assert_eq!(json["container_id"], container_id.to_string());
        assert_eq!(json["host_id"], host_id.to_string());
        assert_eq!(json["subnet_id"], subnet_id.to_string());
        assert!(json.get("ip_address_id").is_none());
    }

    #[test]
    fn test_container_types() {
        let tag = NodeType::Container {
            container_type: ContainerType::NestedTag,
            parent_container_id: Some(Uuid::new_v4()),
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        };
        let json = serde_json::to_value(&tag).unwrap();
        assert_eq!(json["container_type"], "NestedTag");

        let svc = NodeType::Container {
            container_type: ContainerType::NestedServiceCategory,
            parent_container_id: Some(Uuid::new_v4()),
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        };
        let json = serde_json::to_value(&svc).unwrap();
        assert_eq!(json["container_type"], "NestedServiceCategory");
    }

    #[test]
    fn test_service_category_container() {
        let node_type = NodeType::Container {
            container_type: ContainerType::ServiceCategory,
            parent_container_id: None,
            entity_id: None,
            icon: Some("Zap".to_string()),
            color: Some(Color::Purple),
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["container_type"], "ServiceCategory");
        assert!(json.get("parent_container_id").is_none());
    }

    #[test]
    fn test_node_element_constructor() {
        let id = Uuid::new_v4();
        let container_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let node = Node::element(id, container_id, host_id, ElementEntityType::Service {});
        assert_eq!(node.id, id);
        assert!(node.header.is_none());
        assert!(matches!(node.node_type, NodeType::Element { .. }));
    }

    #[test]
    fn test_entity_discriminant_mapping() {
        assert_eq!(
            EntityDiscriminants::from(&ElementEntityType::IPAddress {
                subnet_id: Uuid::nil(),
                ip_address_id: None,
            }),
            EntityDiscriminants::IPAddress
        );
        assert_eq!(
            EntityDiscriminants::from(&ElementEntityType::Service {}),
            EntityDiscriminants::Service
        );
        assert_eq!(
            EntityDiscriminants::from(&ElementEntityType::Host {}),
            EntityDiscriminants::Host
        );
        assert_eq!(
            EntityDiscriminants::from(&ElementEntityType::Interface {
                interface_id: Uuid::nil(),
            }),
            EntityDiscriminants::Interface
        );
    }

    #[test]
    fn test_element_interface_round_trip() {
        let container_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let interface_id = Uuid::new_v4();
        let node_type = NodeType::Element {
            container_id,
            host_id,
            element: ElementEntityType::Interface { interface_id },
            inline_groups: Vec::new(),
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["node_type"], "Element");
        assert_eq!(json["element_type"], "Interface");
        assert_eq!(json["interface_id"], interface_id.to_string());

        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_host_container_round_trip() {
        let node_type = NodeType::Container {
            container_type: ContainerType::Host,
            parent_container_id: None,
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["container_type"], "Host");
        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_element_host_round_trip() {
        let container_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let node_type = NodeType::Element {
            container_id,
            host_id,
            element: ElementEntityType::Host {},
            inline_groups: Vec::new(),
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["node_type"], "Element");
        assert_eq!(json["element_type"], "Host");
        assert!(json.get("subnet_id").is_none());

        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_hypervisor_container_round_trip() {
        let node_type = NodeType::Container {
            container_type: ContainerType::Hypervisor,
            parent_container_id: None,
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["container_type"], "Hypervisor");
        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_container_runtime_container_round_trip() {
        let node_type = NodeType::Container {
            container_type: ContainerType::ContainerRuntime,
            parent_container_id: None,
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["container_type"], "ContainerRuntime");
        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }

    #[test]
    fn test_stack_container_round_trip() {
        let parent_id = Uuid::new_v4();
        let node_type = NodeType::Container {
            container_type: ContainerType::Stack,
            parent_container_id: Some(parent_id),
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        };
        let json = serde_json::to_value(&node_type).unwrap();
        assert_eq!(json["container_type"], "Stack");
        let deserialized: NodeType = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, node_type);
    }
}
