use std::collections::{HashMap, HashSet};

use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::shared::concepts::Concept;
use crate::server::shared::types::metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider};
use crate::server::shared::types::{Color, Icon};
use crate::server::topology::types::base::TopologyRequestOptions;
use crate::server::topology::types::nodes::Node;
use crate::server::topology::types::views::TopologyView;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;

/// What a rule decides about an entity's representation in the topology.
///
/// Each element rule produces a `PlacementDecision` for every entity it acts on.
/// The match on `ElementRule` is exhaustive — adding a new variant forces the
/// developer to decide what placement decisions it produces.
#[derive(Debug, Clone)]
pub enum PlacementDecision {
    /// No-op: entity keeps its own element node in its current container.
    Element,
    /// Entity becomes a subcontainer node created by this rule.
    /// The actual Node is in `RulePlacement::containers`; this just records the mapping.
    BecomeSubcontainer { container_id: Uuid },
    /// Element moved into a subcontainer created by this rule.
    PlaceInContainer { container_id: Uuid },
    /// Entity has no element — represented by another node for edge resolution.
    /// `inline_group` provides visual grouping metadata for the target element's renderer.
    InlineOn {
        node_id: Uuid,
        inline_group: Option<InlineGroup>,
    },
}

/// Visual grouping metadata for inlined entities.
/// Entities sharing the same `group_id` are rendered together in the element card.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct InlineGroup {
    /// The inlined entity's ID (e.g., service ID).
    pub entity_id: Uuid,
    /// Shared by all members of the visual group.
    pub group_id: Uuid,
    /// Whether this entity heads the inline group or is a member of it.
    pub role: InlineGroupRole,
}

/// Role of an inlined entity within its visual group.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum InlineGroupRole {
    /// Rendered as group title with icon (e.g., Docker runtime service)
    Header,
    /// Rendered as a service card within the group
    Member,
}

/// Complete output of one rule's placement computation.
pub struct RulePlacement {
    /// New subcontainer nodes created by this rule.
    pub containers: Vec<Node>,
    /// Per-entity placement decisions.
    pub placements: HashMap<Uuid, PlacementDecision>,
    /// Entity IDs claimed by this rule (first-match-wins).
    pub claimed: HashSet<Uuid>,
}

pub trait GraphRule {
    /// Whether edges targeting elements inside containers created by this rule
    /// should be elevated to target the container itself.
    fn will_accept_edges(&self) -> bool;

    /// Whether users can add this rule from the dropdown and remove it.
    fn is_removable(&self) -> bool;

    /// Whether users can reorder this rule relative to others.
    fn is_reorderable(&self) -> bool;

    /// Whether this rule has user-editable configuration (categories, tags, title).
    fn is_configurable(&self) -> bool;

    /// Whether multiple instances of this rule can coexist.
    fn allow_multiple(&self) -> bool;

    fn applicable_views(&self) -> &'static [TopologyView];
}

/// Generic wrapper that gives any rule type a stable UUID identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct IdentifiedRule<T: GraphRule> {
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// The rule being applied.
    pub rule: T,
}

impl<T: GraphRule> IdentifiedRule<T> {
    pub fn new(rule: T) -> Self {
        Self {
            id: Uuid::new_v4(),
            rule,
        }
    }
}

/// Rules that change which containers exist and how they nest.
/// Container titles are data-driven (subnet CIDR, host names), not user-configurable.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, EnumIter, IntoStaticStr,
)]
pub enum ContainerRule {
    /// One container per subnet.
    #[schema(title = "BySubnet")]
    BySubnet,
    /// Draw a host's container bridges as a single box rather than one each.
    #[schema(title = "MergeContainerBridges")]
    #[serde(alias = "MergeDockerBridges")]
    MergeContainerBridges,
    /// One container per application tag.
    #[schema(title = "ByApplication")]
    ByApplication {
        /// Application tags to draw containers for. Empty means every application tag.
        #[serde(default)]
        tag_ids: Vec<Uuid>,
    },
    /// One container per host.
    #[schema(title = "ByHost")]
    ByHost,
}

impl GraphRule for ContainerRule {
    fn applicable_views(&self) -> &'static [TopologyView] {
        match self {
            ContainerRule::BySubnet => &[TopologyView::L3Logical],
            ContainerRule::MergeContainerBridges => &[TopologyView::L3Logical],
            ContainerRule::ByApplication { .. } => &[TopologyView::Application],
            ContainerRule::ByHost => &[TopologyView::L2Physical, TopologyView::Workloads],
        }
    }

    fn will_accept_edges(&self) -> bool {
        match self {
            ContainerRule::MergeContainerBridges => true,
            ContainerRule::BySubnet
            | ContainerRule::ByApplication { .. }
            | ContainerRule::ByHost => false,
        }
    }

    fn is_removable(&self) -> bool {
        match self {
            ContainerRule::MergeContainerBridges => true,
            ContainerRule::BySubnet
            | ContainerRule::ByApplication { .. }
            | ContainerRule::ByHost => false,
        }
    }

    fn is_reorderable(&self) -> bool {
        match self {
            ContainerRule::MergeContainerBridges => true,
            ContainerRule::BySubnet
            | ContainerRule::ByApplication { .. }
            | ContainerRule::ByHost => false,
        }
    }

    fn is_configurable(&self) -> bool {
        match self {
            ContainerRule::BySubnet
            | ContainerRule::MergeContainerBridges
            | ContainerRule::ByApplication { .. }
            | ContainerRule::ByHost => false,
        }
    }

    fn allow_multiple(&self) -> bool {
        match self {
            ContainerRule::BySubnet
            | ContainerRule::MergeContainerBridges
            | ContainerRule::ByApplication { .. }
            | ContainerRule::ByHost => false,
        }
    }
}

impl HasId for ContainerRule {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ContainerRule {
    fn color(&self) -> Color {
        match self {
            ContainerRule::BySubnet => Color::Blue,
            ContainerRule::MergeContainerBridges => Color::Teal,
            ContainerRule::ByApplication { .. } => Concept::Application.color(),
            ContainerRule::ByHost => Concept::L2.color(),
        }
    }

    fn icon(&self) -> Icon {
        match self {
            ContainerRule::BySubnet => Icon::Network,
            ContainerRule::MergeContainerBridges => Icon::Boxes,
            ContainerRule::ByApplication { .. } => Concept::Application.icon(),
            ContainerRule::ByHost => Concept::L2.icon(),
        }
    }
}

impl TypeMetadataProvider for ContainerRule {
    fn name(&self) -> &'static str {
        match self {
            ContainerRule::BySubnet => "Subnet",
            ContainerRule::MergeContainerBridges => "Container Bridges",
            ContainerRule::ByApplication { .. } => "Application",
            ContainerRule::ByHost => "Host",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ContainerRule::BySubnet => "Group entities by subnet",
            ContainerRule::MergeContainerBridges => {
                "Merge a host's container runtime bridge subnets into a single group (e.g., Docker bridge networks). This prevents clutter from multiple bridge subnets on the same host."
            }
            ContainerRule::ByApplication { .. } => "Group services by application tag",
            ContainerRule::ByHost => "Group entities by host",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "is_removable": self.is_removable(),
            "is_reorderable": self.is_reorderable(),
            "is_configurable": self.is_configurable(),
            "allow_multiple": self.allow_multiple(),
            "views": self.applicable_views(),
            "will_accept_edges": self.will_accept_edges(),
        })
    }
}

/// The order nodes take inside each container.
///
/// Anything other than `Layout` is applied by the server when it builds the graph, and the
/// layout keeps that order on screen, reading left to right, top to bottom.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    IntoStaticStr,
)]
pub enum ElementSort {
    /// The layout places nodes to keep connections short.
    #[default]
    Layout,
    /// By IP address, numerically, IPv4 before IPv6.
    Address,
    /// By interface index, the order the device numbers its ports.
    PortIndex,
    /// By name, with runs of digits compared as numbers.
    Name,
    /// By service category, then name.
    Category,
}

impl ElementSort {
    pub fn applicable_views(&self) -> &'static [TopologyView] {
        match self {
            ElementSort::Layout | ElementSort::Name => &[
                TopologyView::L3Logical,
                TopologyView::L2Physical,
                TopologyView::Workloads,
                TopologyView::Application,
            ],
            ElementSort::Address => &[TopologyView::L3Logical],
            ElementSort::PortIndex => &[TopologyView::L2Physical],
            ElementSort::Category => &[TopologyView::Workloads, TopologyView::Application],
        }
    }
}

impl HasId for ElementSort {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ElementSort {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        match self {
            ElementSort::Layout => Icon::Waypoints,
            ElementSort::Address => Icon::Network,
            ElementSort::PortIndex => Icon::EthernetPort,
            ElementSort::Name => Icon::ArrowDownAZ,
            ElementSort::Category => Icon::Shapes,
        }
    }
}

impl TypeMetadataProvider for ElementSort {
    fn name(&self) -> &'static str {
        match self {
            ElementSort::Layout => "Layout",
            ElementSort::Address => "IP address",
            ElementSort::PortIndex => "Port number",
            ElementSort::Name => "Name",
            ElementSort::Category => "Category",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ElementSort::Layout => "Placed to keep connections short",
            ElementSort::Address => "Lowest address first, IPv4 before IPv6",
            ElementSort::PortIndex => "In the order the device numbers its ports",
            ElementSort::Name => "Alphabetical, with numbers in numeric order",
            ElementSort::Category => "Grouped by service category, then by name",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({ "views": self.applicable_views() })
    }
}

/// Rules that organize nodes within a container into sub-groups.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema, EnumIter, IntoStaticStr,
)]
pub enum ElementRule {
    /// One subcontainer per group of service categories.
    #[schema(title = "ByServiceCategory")]
    ByServiceCategory {
        /// Service categories to group into this subcontainer.
        ///
        /// Lenient on read so a category this build does not know drops out of the rule instead
        /// of failing the whole topology — see [`deserialize_known_categories`].
        #[serde(
            deserialize_with = "crate::server::services::r#impl::categories::deserialize_known_categories"
        )]
        categories: Vec<ServiceCategory>,
        /// Heading for the subcontainer. Defaults to the category name.
        title: Option<String>,
        /// Set by the backend on the default infrastructure rule.
        /// Frontend uses this to identify the infra container for auto-collapse.
        #[serde(default)]
        #[schema(read_only)]
        is_infra_rule: bool,
    },
    /// One subcontainer per group of tags.
    #[schema(title = "ByTag")]
    ByTag {
        /// Tags to group into this subcontainer.
        tag_ids: Vec<Uuid>,
        /// Heading for the subcontainer. Defaults to the tag name.
        title: Option<String>,
    },
    ByHypervisor,
    ByContainerRuntime,
    ByStack,
    /// Groups trunk ports (ports with tagged VLANs) into a "Trunk Ports" subcontainer.
    /// Higher priority than ByVLAN — prevents trunk ports from being grouped by VLAN.
    ByTrunkPort,
    /// Groups access ports by their native VLAN ID into per-VLAN subcontainers.
    ByVLAN,
    /// Groups ports by operational status (Up, Down, etc.) into per-status subcontainers.
    ByPortOpStatus,
}

impl GraphRule for ElementRule {
    fn will_accept_edges(&self) -> bool {
        match self {
            ElementRule::ByHypervisor | ElementRule::ByContainerRuntime | ElementRule::ByStack => {
                true
            }
            ElementRule::ByServiceCategory { .. }
            | ElementRule::ByTag { .. }
            | ElementRule::ByTrunkPort
            | ElementRule::ByVLAN
            | ElementRule::ByPortOpStatus => false,
        }
    }

    fn is_removable(&self) -> bool {
        match self {
            ElementRule::ByServiceCategory { .. } | ElementRule::ByTag { .. } => true,
            ElementRule::ByHypervisor
            | ElementRule::ByContainerRuntime
            | ElementRule::ByStack
            | ElementRule::ByTrunkPort
            | ElementRule::ByVLAN
            | ElementRule::ByPortOpStatus => false,
        }
    }

    fn is_reorderable(&self) -> bool {
        match self {
            ElementRule::ByServiceCategory { .. }
            | ElementRule::ByTag { .. }
            | ElementRule::ByHypervisor
            | ElementRule::ByContainerRuntime
            | ElementRule::ByStack => true,
            ElementRule::ByTrunkPort | ElementRule::ByVLAN | ElementRule::ByPortOpStatus => false,
        }
    }

    fn is_configurable(&self) -> bool {
        match self {
            ElementRule::ByServiceCategory { .. } | ElementRule::ByTag { .. } => true,
            ElementRule::ByHypervisor
            | ElementRule::ByContainerRuntime
            | ElementRule::ByStack
            | ElementRule::ByTrunkPort
            | ElementRule::ByVLAN
            | ElementRule::ByPortOpStatus => false,
        }
    }

    fn allow_multiple(&self) -> bool {
        match self {
            ElementRule::ByServiceCategory { .. } | ElementRule::ByTag { .. } => true,
            ElementRule::ByHypervisor
            | ElementRule::ByContainerRuntime
            | ElementRule::ByStack
            | ElementRule::ByTrunkPort
            | ElementRule::ByVLAN
            | ElementRule::ByPortOpStatus => false,
        }
    }

    fn applicable_views(&self) -> &'static [TopologyView] {
        match self {
            ElementRule::ByServiceCategory { .. } => {
                &[TopologyView::Application, TopologyView::Workloads]
            }
            ElementRule::ByTag { .. } => &[
                TopologyView::L3Logical,
                TopologyView::L2Physical,
                TopologyView::Workloads,
                TopologyView::Application,
            ],
            ElementRule::ByHypervisor => &[TopologyView::Workloads],
            ElementRule::ByContainerRuntime => &[TopologyView::Workloads],
            ElementRule::ByStack => &[TopologyView::L3Logical, TopologyView::Application],
            ElementRule::ByTrunkPort => &[TopologyView::L2Physical],
            ElementRule::ByVLAN => &[TopologyView::L2Physical],
            ElementRule::ByPortOpStatus => &[TopologyView::L2Physical],
        }
    }
}

impl HasId for ElementRule {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ElementRule {
    fn color(&self) -> Color {
        match self {
            ElementRule::ByServiceCategory { .. } => Color::Purple,
            ElementRule::ByTag { .. } => Color::Orange,
            ElementRule::ByHypervisor => Concept::Virtualization.color(),
            ElementRule::ByContainerRuntime => Concept::Containerization.color(),
            ElementRule::ByStack => Concept::Containerization.color(),
            ElementRule::ByTrunkPort => Color::Amber,
            ElementRule::ByVLAN => Color::Teal,
            ElementRule::ByPortOpStatus => Color::Gray,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            ElementRule::ByServiceCategory { .. } => Icon::Layers,
            ElementRule::ByTag { .. } => Icon::Tag,
            ElementRule::ByHypervisor => Concept::Virtualization.icon(),
            ElementRule::ByContainerRuntime => Concept::Containerization.icon(),
            ElementRule::ByStack => Concept::Containerization.icon(),
            ElementRule::ByTrunkPort => Icon::Network,
            ElementRule::ByVLAN => Icon::Network,
            ElementRule::ByPortOpStatus => Icon::Circle,
        }
    }
}

impl TypeMetadataProvider for ElementRule {
    fn name(&self) -> &'static str {
        match self {
            ElementRule::ByServiceCategory { .. } => "Service Category",
            ElementRule::ByTag { .. } => "Tag",
            ElementRule::ByHypervisor => "Hypervisor",
            ElementRule::ByContainerRuntime => "Container Runtime",
            ElementRule::ByStack => "Stack",
            ElementRule::ByTrunkPort => "Trunk Ports",
            ElementRule::ByVLAN => "VLAN",
            ElementRule::ByPortOpStatus => "Port Status",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            ElementRule::ByServiceCategory { .. } => "Group services by category",
            ElementRule::ByTag { .. } => "Group entities by tag",
            ElementRule::ByHypervisor => "Group VMs by hypervisor",
            ElementRule::ByContainerRuntime => "Group containers by runtime",
            ElementRule::ByStack => {
                "Group containers deployed together as a stack (e.g., Docker Compose, Podman Compose)"
            }
            ElementRule::ByTrunkPort => "Group trunk ports (ports carrying multiple VLANs)",
            ElementRule::ByVLAN => "Group access ports by native VLAN ID",
            ElementRule::ByPortOpStatus => "Group ports by operational status",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "is_removable": self.is_removable(),
            "is_reorderable": self.is_reorderable(),
            "is_configurable": self.is_configurable(),
            "allow_multiple": self.allow_multiple(),
            "views": self.applicable_views(),
            "will_accept_edges": self.will_accept_edges(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct GroupingConfig {
    pub container_rules: Vec<IdentifiedRule<ContainerRule>>,
    pub element_rules: Vec<IdentifiedRule<ElementRule>>,
    pub element_sort: ElementSort,
}

impl GroupingConfig {
    pub fn from_request_options(options: &TopologyRequestOptions, view: TopologyView) -> Self {
        // Container rules: look up current view directly (per-view HashMap)
        let container_rules = options
            .container_rules
            .get(&view)
            .cloned()
            .unwrap_or_default();

        // Element rules: filter shared set by applicable views
        let element_rules = options
            .element_rules
            .iter()
            .filter(|gr| gr.rule.applicable_views().contains(&view))
            .cloned()
            .collect();

        // A stored sort the view has no key for falls back to the layout's own order.
        let element_sort = options
            .element_sort
            .get(&view)
            .copied()
            .filter(|sort| sort.applicable_views().contains(&view))
            .unwrap_or_default();

        GroupingConfig {
            container_rules,
            element_rules,
            element_sort,
        }
    }

    pub fn should_group_container_bridges(&self) -> bool {
        self.container_rules
            .iter()
            .any(|r| matches!(r.rule, ContainerRule::MergeContainerBridges))
    }

    /// Whether a subnet is itself a container in this view, so an edge aimed at one address
    /// inside it elevates onto the subnet box.
    ///
    /// Container-runtime edges use this to decide how many to emit. Collapsing them to one per
    /// bridge subnet — with the lowest address as the representative — is only sound when that
    /// subnet is a container: every other member elevates onto the same box, so one edge covers
    /// them all. Where subnets are not containers (the Application view groups by application),
    /// members of one bridge land in different groups, and a single edge reaches only whichever
    /// group holds the representative, leaving the rest looking unconnected.
    pub fn subnets_are_containers(&self) -> bool {
        self.container_rules
            .iter()
            .any(|r| matches!(r.rule, ContainerRule::BySubnet))
    }

    pub fn has_application_rule(&self) -> bool {
        self.container_rules
            .iter()
            .any(|r| matches!(r.rule, ContainerRule::ByApplication { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::types::metadata::TypeMetadataProvider;
    use crate::server::topology::types::base::TopologyRequestOptions;

    #[test]
    fn element_sort_resolves_per_view_and_drops_inapplicable_sorts() {
        let options = TopologyRequestOptions {
            element_sort: HashMap::from([
                (TopologyView::L3Logical, ElementSort::Address),
                (TopologyView::Workloads, ElementSort::PortIndex),
            ]),
            ..Default::default()
        };
        let sort_for = |view| GroupingConfig::from_request_options(&options, view).element_sort;

        assert_eq!(sort_for(TopologyView::L3Logical), ElementSort::Address);
        // Workloads has no interfaces to index, so the stored sort cannot apply there.
        assert_eq!(sort_for(TopologyView::Workloads), ElementSort::Layout);
        assert_eq!(sort_for(TopologyView::Application), ElementSort::Layout);
    }

    #[test]
    fn test_metadata_includes_capability_flags() {
        let meta = ElementRule::ByStack.metadata();
        assert!(meta["is_removable"].is_boolean());
        assert!(meta["is_reorderable"].is_boolean());
        assert!(meta["is_configurable"].is_boolean());
        assert!(meta["allow_multiple"].is_boolean());
        assert!(meta["views"].is_array());
        assert!(meta["will_accept_edges"].is_boolean());
    }

    #[test]
    fn test_no_docker_grouping() {
        let mut options = TopologyRequestOptions::default();
        options.container_rules.insert(
            TopologyView::L3Logical,
            vec![IdentifiedRule::new(ContainerRule::BySubnet)],
        );
        let config = GroupingConfig::from_request_options(&options, TopologyView::L3Logical);

        assert!(!config.should_group_container_bridges());
    }

    #[test]
    fn test_serialization_round_trip_container_rules() {
        let rules = vec![
            IdentifiedRule::new(ContainerRule::BySubnet),
            IdentifiedRule::new(ContainerRule::MergeContainerBridges),
        ];

        let json = serde_json::to_string(&rules).unwrap();
        let deserialized: Vec<IdentifiedRule<ContainerRule>> = serde_json::from_str(&json).unwrap();
        assert_eq!(rules, deserialized);
    }

    #[test]
    fn test_merge_container_bridges_deserializes_legacy_alias() {
        // Persisted request-options written before the rename stored the variant
        // as "MergeDockerBridges"; the serde alias must still deserialize them.
        let legacy = r#"{"id":"00000000-0000-0000-0000-000000000001","rule":"MergeDockerBridges"}"#;
        let parsed: IdentifiedRule<ContainerRule> = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.rule, ContainerRule::MergeContainerBridges);
    }

    #[test]
    fn test_serialization_round_trip_element_rules() {
        let rules = vec![
            IdentifiedRule::new(ElementRule::ByServiceCategory {
                categories: vec![ServiceCategory::DNS, ServiceCategory::ReverseProxy],
                title: Some("Infrastructure".into()),
                is_infra_rule: false,
            }),
            IdentifiedRule::new(ElementRule::ByTag {
                tag_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
                title: Some("Tagged".into()),
            }),
            IdentifiedRule::new(ElementRule::ByStack),
        ];

        let json = serde_json::to_string(&rules).unwrap();
        let deserialized: Vec<IdentifiedRule<ElementRule>> = serde_json::from_str(&json).unwrap();
        assert_eq!(rules, deserialized);
    }

    #[test]
    fn test_by_stack_serde_round_trip() {
        let rule = IdentifiedRule::new(ElementRule::ByStack);
        let json = serde_json::to_string(&rule).unwrap();
        assert!(json.contains("ByStack"));
        let deserialized: IdentifiedRule<ElementRule> = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, deserialized);
    }
    /// Rollback safety: a rule saved by a newer build names `Industrial`; a binary without that
    /// variant must still be able to read the rule, minus the entry it cannot place.
    #[test]
    fn a_category_this_build_does_not_know_drops_out_of_the_rule() {
        let saved = serde_json::json!({
            "ByServiceCategory": {
                "categories": ["NetworkCore", "SomethingFromTheFuture", "Printer"],
                "title": "Infrastructure",
                "is_infra_rule": true
            }
        });

        let rule: ElementRule = serde_json::from_value(saved).expect("the rule still deserializes");

        let ElementRule::ByServiceCategory {
            categories, title, ..
        } = rule
        else {
            panic!("expected a ByServiceCategory rule");
        };
        assert_eq!(
            categories,
            vec![ServiceCategory::NetworkCore, ServiceCategory::Printer]
        );
        assert_eq!(title.as_deref(), Some("Infrastructure"));
    }
}
