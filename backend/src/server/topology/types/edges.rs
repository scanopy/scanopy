use crate::server::shared::types::entity_field::{EntityField, FieldGroup, Hop, MetadataFixture};
use crate::server::{
    dependencies::r#impl::types::DependencyTypeDiscriminants,
    shared::{
        concepts::Concept,
        entities::EntityDiscriminants,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumDiscriminants, EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;
use uuid::Uuid;

/// Protocol that discovered the physical link between network devices
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    Default,
    ToSchema,
    IntoStaticStr,
    EnumIter,
)]
pub enum DiscoveryProtocol {
    /// Link Layer Discovery Protocol (IEEE 802.1AB)
    #[default]
    LLDP,
    /// Cisco Discovery Protocol (Cisco proprietary)
    CDP,
    /// A switch's forwarding table learned the device's MAC on this port. The device is reached
    /// through the port but may sit further behind it, past a switch or access point nothing
    /// reported.
    ForwardingTable,
}

impl HasId for DiscoveryProtocol {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for DiscoveryProtocol {
    fn color(&self) -> Color {
        match self {
            DiscoveryProtocol::LLDP => Color::Green,
            DiscoveryProtocol::CDP => Color::Blue,
            DiscoveryProtocol::ForwardingTable => Color::Gray,
        }
    }
    fn icon(&self) -> Icon {
        EntityDiscriminants::Interface.icon()
    }
}

impl TypeMetadataProvider for DiscoveryProtocol {
    fn name(&self) -> &'static str {
        match self {
            DiscoveryProtocol::LLDP => "LLDP",
            DiscoveryProtocol::CDP => "CDP",
            DiscoveryProtocol::ForwardingTable => "Forwarding table",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            DiscoveryProtocol::LLDP => {
                "A device's LLDP (Link Layer Discovery Protocol) neighbor table"
            }
            DiscoveryProtocol::CDP => "A device's CDP (Cisco Discovery Protocol) neighbor table",
            DiscoveryProtocol::ForwardingTable => {
                "Learned from a switch's forwarding table, which saw the device's MAC behind one of its ports. The device may sit further away than a direct cable"
            }
        }
    }
}

/// Whether an edge is visible by default or hidden behind a toggle
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeDefaultVisibility {
    #[default]
    Visible,
    Hidden,
}

/// Visual stroke style for an edge
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeStroke {
    #[default]
    Solid,
    Dashed,
    /// Finer break-up than `Dashed`, for edges that annotate the graph rather than
    /// structure it (see `SameContainer`).
    Dotted,
}

/// Controls when an edge contributes to node highlighting on selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeHighlightBehavior {
    /// Highlights connected nodes when the edge is visible (not hidden by toggle)
    #[default]
    WhenVisible,
    /// Always highlights connected nodes regardless of visibility
    Always,
    /// Never highlights connected nodes
    Never,
}

/// What a click on an edge highlights.
///
/// An edge is one segment of a relation — a dependency's chain, a host's addresses, a
/// container's addresses, a runtime's bridges — and a click either lights up the whole
/// relation or only the segment that was clicked. Generic: any current or future edge type
/// picks one, and the selection code reads the property rather than branching on edge type.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EdgeSelectionScope {
    /// Highlight every node connected by any segment of the same relation — the segments that
    /// share this edge's `relation_key`.
    ConnectedNodes,
    /// Highlight only this edge's own two endpoints.
    #[default]
    Segment,
}

/// Per-view configuration for an edge: disabled (not in this view) or active with properties
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EdgeViewConfig {
    /// Edge is not available in this view
    #[default]
    #[schema(title = "Disabled")]
    Disabled,
    /// Edge is active in this view with specific properties
    #[schema(title = "Active")]
    Active {
        /// Whether ELK should use this edge for layout positioning
        affects_layout: bool,
        /// Whether ELK should use this edge to place its endpoints next to each other inside the
        /// container they share, without it moving any container. Implied by `affects_layout`.
        /// Defaulted so edges persisted before the field existed still deserialize.
        #[serde(default)]
        affects_layout_within_container: bool,
        /// Whether the edge is shown by default or hidden behind a toggle
        default_visibility: EdgeDefaultVisibility,
        /// Visual stroke style
        stroke: EdgeStroke,
        /// When this edge contributes to node highlighting on selection
        highlight_behavior: EdgeHighlightBehavior,
        /// Whether this edge should be elevated to target an accepting container
        /// instead of the element inside it
        will_target_container: bool,
        /// Whether this edge should show directional animation when highlighted
        show_directionality: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, ToSchema)]
pub struct Edge {
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// Node the edge starts at.
    pub source: Uuid,
    /// Node the edge ends at.
    pub target: Uuid,
    /// What relationship this edge represents, and the entities behind it.
    #[serde(flatten)]
    pub edge_type: EdgeType,
    /// Text drawn on the edge.
    #[schema(required)]
    pub label: Option<String>,
    /// The names of the two ends, source first, for edges that join two named things. The UI
    /// draws each name at its own end; `label` carries the same names joined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_labels: Option<[String; 2]>,
    /// Which side of the source node the edge leaves from.
    pub source_handle: EdgeHandle,
    /// Which side of the target node the edge arrives at.
    pub target_handle: EdgeHandle,
    /// Whether the edge stands in for a path that crosses intermediate nodes.
    pub is_multi_hop: bool,
    /// Per-view overrides for how this edge is drawn.
    #[serde(default)]
    pub view_config: EdgeViewConfig,
    /// Identity of the relation this edge stands for — see [`EdgeType::relation_key`]. Stamped
    /// centrally from `edge_type` once the graph is built, so no construction site can forget
    /// it. `None` marks an edge as interchangeable with its like.
    #[serde(default)]
    #[schema(required)]
    pub relation_key: Option<String>,
}

#[derive(
    Serialize,
    Copy,
    Deserialize,
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    ToSchema,
)]
pub enum EdgeHandle {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(
    Serialize,
    Copy,
    Deserialize,
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Default,
    IntoStaticStr,
    Display,
    VariantNames,
    ToSchema,
)]
pub enum EdgeStyle {
    Straight,
    #[serde(alias = "Step")]
    SmoothStep,
    #[default]
    #[serde(alias = "SimpleBezier")]
    Bezier,
}

/// Read a saved list of edge types, dropping any this binary doesn't know.
///
/// A topology's options persist the edge types a user chose to hide. Read strictly, one unknown
/// value (an edge type added by a newer release, met by an older one during a rollback or a
/// mixed-version window) failed the whole topology. Same reasoning and same narrow scope as
/// `deserialize_known_categories`: only stored lists are lenient, the enum itself stays strict.
pub fn deserialize_known_edge_types<'de, D>(
    deserializer: D,
) -> Result<Vec<EdgeTypeDiscriminants>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Vec::<serde_json::Value>::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .filter_map(
            |value| match serde_json::from_value::<EdgeTypeDiscriminants>(value.clone()) {
                Ok(edge_type) => Some(edge_type),
                Err(_) => {
                    tracing::warn!(%value, "Unrecognized edge type in topology options; dropping it");
                    None
                }
            },
        )
        .collect())
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    EnumDiscriminants,
    IntoStaticStr,
    EnumIter,
    ToSchema,
)]
#[strum_discriminants(derive(Display, Hash, Serialize, Deserialize, EnumIter, ToSchema))]
#[serde(tag = "edge_type")]
pub enum EdgeType {
    #[schema(title = "SameHost")]
    SameHost {
        /// The host both endpoints sit on.
        host_id: Uuid,
    },
    #[schema(title = "Hypervisor")]
    Hypervisor {
        /// The hypervisor service running the guest.
        hypervisor_service_id: Uuid,
    },
    /// A guest's own address to one of its network identities on the same subnet: an address
    /// and MAC the guest presents from an interface of its own beyond its configured NICs.
    #[schema(title = "NetworkIdentity")]
    NetworkIdentity {
        /// The guest's Network Identities service, which owns the identity host.
        identities_service_id: Uuid,
    },
    #[schema(title = "ContainerRuntime")]
    ContainerRuntime {
        /// The host running the container runtime.
        host_id: Uuid,
        /// The container runtime service itself.
        service_id: Uuid,
        /// The bridge subnet(s) this edge reaches: one when they render as their own boxes,
        /// all of them when merged into a single box. Resolved here rather than in the
        /// inspector, which cannot tell which subnet an elevated edge landed on. For an edge to a
        /// container host (macvlan, ipvlan), the subnet holding the address it ends on.
        subnet_ids: Vec<Uuid>,
        /// The containerized services this edge stands for — the ones on those subnets. Empty on
        /// an edge to a container host.
        containerized_service_ids: Vec<Uuid>,
    },
    /// One container reachable at several of its host's container-bridge subnets. Ties the
    /// container's addresses together so a multi-attached container reads as one thing rather
    /// than as unrelated cards in separate subnet boxes.
    #[schema(title = "SameContainer")]
    SameContainer {
        /// The containerized service reachable at several addresses.
        service_id: Uuid,
    },
    #[schema(title = "RequestPath")]
    RequestPath {
        /// The dependency this edge was drawn from.
        dependency_id: Uuid,
        /// Member the request starts at.
        source_id: Uuid,
        /// Member the request arrives at.
        target_id: Uuid,
    },
    #[schema(title = "HubAndSpoke")]
    HubAndSpoke {
        /// The dependency this edge was drawn from.
        dependency_id: Uuid,
        /// The hub member.
        source_id: Uuid,
        /// The spoke member.
        target_id: Uuid,
    },
    /// Physical link discovered via LLDP/CDP neighbor discovery
    #[schema(title = "PhysicalLink")]
    PhysicalLink {
        /// Interface at one end of the cable.
        source_entity_id: Uuid,
        /// Interface at the other end.
        target_entity_id: Uuid,
        /// Neighbour-discovery protocol the link was learned from.
        protocol: DiscoveryProtocol,
    },
    /// Device-level adjacency from LLDP/CDP: the neighbour resolved to a host, but the remote
    /// port could not be pinned down (a locally-assigned port id that matches nothing, or a
    /// neighbour entry carrying no port id at all). The two devices are provably adjacent;
    /// which cables they meet on is unknown. `PhysicalLink` is the port-precise sibling.
    #[schema(title = "NeighborLink")]
    NeighborLink {
        /// One of the adjacent devices.
        source_host_id: Uuid,
        /// The other adjacent device.
        target_host_id: Uuid,
        /// Neighbour-discovery protocol the adjacency was learned from.
        protocol: DiscoveryProtocol,
    },
}

impl HasId for EdgeType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EdgeType {
    /// What a click on this edge highlights. Edges that stand for one segment of a wider
    /// relation light up the whole relation; edges that are a relationship in their own
    /// right light up their endpoints.
    pub fn selection_scope(&self) -> EdgeSelectionScope {
        use EdgeSelectionScope::*;
        match self {
            // Every segment of the dependency's chain.
            EdgeType::RequestPath { .. } | EdgeType::HubAndSpoke { .. } => ConnectedNodes,
            // Every address of the host.
            EdgeType::SameHost { .. } => ConnectedNodes,
            // Every address of the container.
            EdgeType::SameContainer { .. } => ConnectedNodes,
            // A runtime's edges each reach a different bridge, and a hypervisor's each reach a
            // different VM — they are separate connections that happen to share an origin, not
            // segments of one thing, so a click stays on the one that was clicked. A physical
            // link is likewise the whole relationship, not a segment of one.
            EdgeType::ContainerRuntime { .. }
            | EdgeType::Hypervisor { .. }
            | EdgeType::NetworkIdentity { .. }
            | EdgeType::PhysicalLink { .. }
            | EdgeType::NeighborLink { .. } => Segment,
        }
    }

    /// Order-independent key for a pair of ids, so a relationship reported from either end
    /// reads as the same one.
    fn pair_key(a: Uuid, b: Uuid) -> String {
        if a < b {
            format!("{a}:{b}")
        } else {
            format!("{b}:{a}")
        }
    }

    /// The identity of the relation this edge stands for, or `None` when the edge is one of
    /// several interchangeable connections of its type.
    ///
    /// Two edges sharing a relation key are one thing drawn twice: safe for the canvas to merge
    /// into a single line, and a click on either means both. Two edges *without* a shared key
    /// are different things and must each keep their own line — merging them leaves a picture
    /// asserting "these two boxes are connected" while silently dropping which cable, host or
    /// dependency connects them.
    ///
    /// Derived by destructuring rather than by naming payload fields, so renaming a field or
    /// adding a variant is a compile error rather than a silent behaviour change.
    pub fn relation_key(&self) -> Option<String> {
        match self {
            // Every segment of one dependency's chain.
            EdgeType::RequestPath { dependency_id, .. }
            | EdgeType::HubAndSpoke { dependency_id, .. } => Some(dependency_id.to_string()),
            // Every address of one host, or of one container.
            EdgeType::SameHost { host_id } => Some(host_id.to_string()),
            EdgeType::SameContainer { service_id } => Some(service_id.to_string()),
            // A cable is the pair of ports it joins; a device-level adjacency, the pair of
            // devices. Two cables between the same pair of boxes are still two cables.
            EdgeType::PhysicalLink {
                source_entity_id,
                target_entity_id,
                ..
            } => Some(Self::pair_key(*source_entity_id, *target_entity_id)),
            EdgeType::NeighborLink {
                source_host_id,
                target_host_id,
                ..
            } => Some(Self::pair_key(*source_host_id, *target_host_id)),
            // These elevate onto their containers (`will_target_container`), so several of them
            // between the same pair of boxes land on identical endpoints and draw as one line
            // over another. Merging them into a single counted line is the only way to show
            // there is more than one; expanding the bundle fans them back out.
            EdgeType::Hypervisor { .. }
            | EdgeType::NetworkIdentity { .. }
            | EdgeType::ContainerRuntime { .. } => None,
        }
    }
}

impl EntityMetadataProvider for EdgeType {
    fn color(&self) -> Color {
        match self {
            EdgeType::RequestPath { .. } => EntityDiscriminants::Dependency.color(),
            EdgeType::HubAndSpoke { .. } => EntityDiscriminants::Dependency.color(),
            EdgeType::SameHost { .. } => EntityDiscriminants::Host.color(),
            EdgeType::Hypervisor { .. } => Concept::Virtualization.color(),
            // An identity is one interface the guest presents. Virtualization's colour is the
            // host's, which would draw it like the SameHost and Hypervisor edges beside it.
            EdgeType::NetworkIdentity { .. } => EntityDiscriminants::Interface.color(),
            EdgeType::ContainerRuntime { .. } => Concept::Containerization.color(),
            EdgeType::SameContainer { .. } => Concept::Containerization.color(),
            EdgeType::PhysicalLink { .. } => EntityDiscriminants::Interface.color(),
            // Joins two hosts, not two ports — colouring it as a host also keeps it visually
            // distinct from the port-precise link it sits beside.
            EdgeType::NeighborLink { .. } => EntityDiscriminants::Host.color(),
        }
    }

    fn icon(&self) -> Icon {
        match self {
            EdgeType::RequestPath { .. } => DependencyTypeDiscriminants::RequestPath.icon(),
            EdgeType::HubAndSpoke { .. } => DependencyTypeDiscriminants::HubAndSpoke.icon(),
            EdgeType::SameHost { .. } => EntityDiscriminants::Host.icon(),
            EdgeType::Hypervisor { .. } => Concept::Virtualization.icon(),
            EdgeType::NetworkIdentity { .. } => Icon::FingerprintPattern,
            EdgeType::ContainerRuntime { .. } => Concept::Containerization.icon(),
            EdgeType::SameContainer { .. } => Concept::Containerization.icon(),
            EdgeType::PhysicalLink { .. } => EntityDiscriminants::Interface.icon(),
            EdgeType::NeighborLink { .. } => EntityDiscriminants::Host.icon(),
        }
    }
}

impl TypeMetadataProvider for EdgeType {
    fn name(&self) -> &'static str {
        match self {
            EdgeType::RequestPath { .. } => DependencyTypeDiscriminants::RequestPath.name(),
            EdgeType::HubAndSpoke { .. } => DependencyTypeDiscriminants::HubAndSpoke.name(),
            EdgeType::SameHost { .. } => "Same Host",
            EdgeType::Hypervisor { .. } => "Hypervisor",
            EdgeType::NetworkIdentity { .. } => "Network Identity",
            EdgeType::ContainerRuntime { .. } => "Container Runtime",
            EdgeType::SameContainer { .. } => "Same Container",
            EdgeType::PhysicalLink { .. } => "Physical Link",
            EdgeType::NeighborLink { .. } => "Neighbor Link",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let edge_style: &str = match &self {
            EdgeType::RequestPath { .. } => EdgeStyle::Bezier.into(),
            EdgeType::HubAndSpoke { .. } => EdgeStyle::Bezier.into(),
            EdgeType::SameHost { .. } => EdgeStyle::Bezier.into(),
            EdgeType::Hypervisor { .. } => EdgeStyle::Bezier.into(),
            EdgeType::NetworkIdentity { .. } => EdgeStyle::Bezier.into(),
            EdgeType::ContainerRuntime { .. } => EdgeStyle::Bezier.into(),
            EdgeType::SameContainer { .. } => EdgeStyle::Bezier.into(),
            EdgeType::PhysicalLink { .. } => EdgeStyle::Bezier.into(),
            EdgeType::NeighborLink { .. } => EdgeStyle::Bezier.into(),
        };

        let has_start_marker = false;

        let has_end_marker = match &self {
            EdgeType::RequestPath { .. } => true,
            EdgeType::HubAndSpoke { .. } => true,
            EdgeType::SameHost { .. } => false,
            EdgeType::Hypervisor { .. } => false,
            EdgeType::NetworkIdentity { .. } => false,
            EdgeType::ContainerRuntime { .. } => false,
            EdgeType::SameContainer { .. } => false,
            EdgeType::PhysicalLink { .. } => false, // No markers - bidirectional link
            EdgeType::NeighborLink { .. } => false, // No markers - bidirectional adjacency
        };

        let is_host_edge = matches!(
            self,
            EdgeType::SameHost { .. } | EdgeType::ContainerRuntime { .. }
        );
        let is_dependency_edge = matches!(
            self,
            EdgeType::RequestPath { .. } | EdgeType::HubAndSpoke { .. }
        );
        let is_physical_edge = matches!(
            self,
            EdgeType::PhysicalLink { .. } | EdgeType::NeighborLink { .. }
        );

        serde_json::json!({
            "has_start_marker": has_start_marker,
            "has_end_marker": has_end_marker,
            "edge_style": edge_style,
            "is_host_edge": is_host_edge,
            "is_dependency_edge": is_dependency_edge,
            "is_physical_edge": is_physical_edge,
            "selection_scope": self.selection_scope(),
            "fields": self.fields()
        })
    }
}

use crate::server::shared::{relations as R, storage::relation::Reference};
use EntityDiscriminants as E;

const DEPENDENCY_FIELDS: &[EntityField] =
    &[
        EntityField::new("dependency_id", "Dependency", FieldGroup::Relationships)
            .entity(&[Hop::parent(R::DEPENDENCY_ID)])
            .summary(),
    ];
const SAME_HOST_FIELDS: &[EntityField] =
    &[
        EntityField::new("host_id", "Host", FieldGroup::Relationships)
            .entity(&[Hop::parent(R::HOST_ID)])
            .summary(),
    ];
const HYPERVISOR_FIELDS: &[EntityField] = &[EntityField::new(
    "hypervisor_service_id",
    "Hypervisor",
    FieldGroup::Virtualization,
)
.entity(&[Hop::parent(Reference::new(
    E::Service,
    "hypervisor_service_id",
))])
.summary()];
const NETWORK_IDENTITY_FIELDS: &[EntityField] = &[EntityField::new(
    "identities_service_id",
    "Identity Service",
    FieldGroup::Relationships,
)
.entity(&[Hop::parent(Reference::new(
    E::Service,
    "identities_service_id",
))])
.summary()];
const CONTAINER_RUNTIME_FIELDS: &[EntityField] = &[
    EntityField::new("host_id", "Host", FieldGroup::Relationships)
        .entity(&[Hop::parent(R::HOST_ID)]),
    EntityField::new("service_id", "Runtime", FieldGroup::Virtualization)
        .entity(&[Hop::parent(R::SERVICE_ID)])
        .summary(),
    EntityField::new(
        "containerized_service_ids",
        "Containers",
        FieldGroup::Virtualization,
    )
    .entity(&[Hop::parent(Reference::new(
        E::Service,
        "containerized_service_ids",
    ))])
    .many(),
    EntityField::new("subnet_ids", "Bridge Subnets", FieldGroup::Network)
        .entity(&[Hop::parent(R::SUBNET_IDS)])
        .many(),
];
const SAME_CONTAINER_FIELDS: &[EntityField] =
    &[
        EntityField::new("service_id", "Service", FieldGroup::Relationships)
            .entity(&[Hop::parent(R::SERVICE_ID)])
            .summary(),
    ];
const PROTOCOL: EntityField = EntityField::new("protocol", "Protocol", FieldGroup::Network)
    .metadata(MetadataFixture::DiscoveryProtocols);
const PHYSICAL_LINK_FIELDS: &[EntityField] = &[
    EntityField::new("source_entity_id", "From Interface", FieldGroup::Network)
        .entity(&[Hop::parent(Reference::new(
            E::Interface,
            "source_entity_id",
        ))])
        .summary(),
    EntityField::new("target_entity_id", "To Interface", FieldGroup::Network)
        .entity(&[Hop::parent(Reference::new(
            E::Interface,
            "target_entity_id",
        ))])
        .summary(),
    PROTOCOL,
];
const NEIGHBOR_LINK_FIELDS: &[EntityField] = &[
    EntityField::new("source_host_id", "From Host", FieldGroup::Relationships)
        .entity(&[Hop::parent(Reference::new(E::Host, "source_host_id"))])
        .summary(),
    EntityField::new("target_host_id", "To Host", FieldGroup::Relationships)
        .entity(&[Hop::parent(Reference::new(E::Host, "target_host_id"))])
        .summary(),
    PROTOCOL,
];

impl EdgeType {
    /// The fields an edge of this type is described by, read from the edge's own data. See
    /// `shared::types::entity_field`.
    pub fn fields(&self) -> &'static [EntityField] {
        match self {
            EdgeType::RequestPath { .. } | EdgeType::HubAndSpoke { .. } => DEPENDENCY_FIELDS,
            EdgeType::SameHost { .. } => SAME_HOST_FIELDS,
            EdgeType::Hypervisor { .. } => HYPERVISOR_FIELDS,
            EdgeType::NetworkIdentity { .. } => NETWORK_IDENTITY_FIELDS,
            EdgeType::ContainerRuntime { .. } => CONTAINER_RUNTIME_FIELDS,
            EdgeType::SameContainer { .. } => SAME_CONTAINER_FIELDS,
            EdgeType::PhysicalLink { .. } => PHYSICAL_LINK_FIELDS,
            EdgeType::NeighborLink { .. } => NEIGHBOR_LINK_FIELDS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::dependencies::r#impl::types::DependencyTypeDiscriminants;
    use crate::server::topology::types::views::TopologyView;
    use strum::IntoEnumIterator;

    #[test]
    fn edge_type_matches_dependency_type() {
        // This will fail to compile if DependencyType adds/removes variants
        // without updating EdgeType
        let dependency_types: Vec<DependencyTypeDiscriminants> =
            DependencyTypeDiscriminants::iter().collect();

        assert_eq!(
            dependency_types.len(),
            2,
            "Update EdgeType to match DependencyType variants!"
        );
        assert!(dependency_types.contains(&DependencyTypeDiscriminants::RequestPath));
        assert!(dependency_types.contains(&DependencyTypeDiscriminants::HubAndSpoke));
    }

    #[test]
    fn edge_view_config_serde_round_trips() {
        // Disabled variant
        let disabled = EdgeViewConfig::Disabled;
        let json = serde_json::to_value(disabled).unwrap();
        assert_eq!(json["type"], "disabled");
        let deserialized: EdgeViewConfig = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, disabled);

        // Active variant
        let active = EdgeViewConfig::Active {
            affects_layout: true,
            affects_layout_within_container: true,
            default_visibility: EdgeDefaultVisibility::Hidden,
            stroke: EdgeStroke::Dashed,
            highlight_behavior: EdgeHighlightBehavior::Always,
            will_target_container: true,
            show_directionality: true,
        };
        let json = serde_json::to_value(active).unwrap();
        assert_eq!(json["type"], "active");
        assert_eq!(json["affects_layout"], true);
        assert_eq!(json["default_visibility"], "hidden");
        assert_eq!(json["stroke"], "dashed");
        assert_eq!(json["highlight_behavior"], "always");
        assert_eq!(json["will_target_container"], true);
        assert_eq!(json["show_directionality"], true);
        let deserialized: EdgeViewConfig = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, active);
    }

    /// Topologies persist their edges with each view's config, so a row written before
    /// `affects_layout_within_container` existed has to keep loading.
    #[test]
    fn active_view_config_persisted_without_within_container_flag_still_loads() {
        let stored = serde_json::json!({
            "type": "active",
            "affects_layout": false,
            "default_visibility": "visible",
            "stroke": "dashed",
            "highlight_behavior": "when_visible",
            "will_target_container": false,
            "show_directionality": true
        });
        let config: EdgeViewConfig = serde_json::from_value(stored).unwrap();
        assert!(matches!(
            config,
            EdgeViewConfig::Active {
                affects_layout_within_container: false,
                ..
            }
        ));
    }

    #[test]
    fn view_config_default_is_disabled() {
        assert_eq!(EdgeViewConfig::default(), EdgeViewConfig::Disabled);
    }

    /// An edge with no relation key is interchangeable with its like: the canvas is free to
    /// merge several of them into one counted line. That is only ever right when they land on
    /// identical endpoints anyway, which is what elevating onto a container guarantees. An edge
    /// that draws to its own endpoints and gets merged disappears instead — the bug this rule
    /// exists to prevent.
    #[test]
    fn interchangeable_edges_are_the_ones_elevated_onto_containers() {
        for edge_type in EdgeType::iter() {
            if edge_type.relation_key().is_some() {
                continue;
            }
            for view in TopologyView::iter() {
                let EdgeViewConfig::Active {
                    will_target_container,
                    ..
                } = view.edge_view_config((&edge_type).into())
                else {
                    continue;
                };
                assert!(
                    will_target_container,
                    "{edge_type:?} has no relation key, so {view:?} may merge several of them \
                     into one line — but it draws to its own endpoints there, so merging hides \
                     all but the first. Give it a relation key or elevate it."
                );
            }
        }
    }
}
