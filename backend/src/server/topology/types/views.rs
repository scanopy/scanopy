use std::collections::{BTreeMap, HashMap, HashSet};

use crate::server::{
    hosts::r#impl::virtualization::HostVirtualizationState,
    interfaces::r#impl::base::InterfaceLinkState,
    services::r#impl::categories::ServiceCategory,
    shared::{
        concepts::Concept,
        entities::EntityDiscriminants,
        types::entities::EntityFreshness,
        types::{
            Color, Icon,
            metadata::{
                EntityMetadataProvider, HasId, MetadataProvider, TypeMetadata, TypeMetadataProvider,
            },
        },
    },
};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;
use uuid::Uuid;

use super::edges::{
    EdgeDefaultVisibility, EdgeHighlightBehavior, EdgeStroke, EdgeTypeDiscriminants, EdgeViewConfig,
};

// ---------------------------------------------------------------------------
// TopologyView enum
// ---------------------------------------------------------------------------

/// Which topology view is being rendered
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Default,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum TopologyView {
    L2Physical,
    #[default]
    L3Logical,
    Workloads,
    Application,
}

impl HasId for TopologyView {
    fn id(&self) -> &'static str {
        self.into()
    }
}

// ---------------------------------------------------------------------------
// TopologyViewSupport — per-view data-availability flags
// ---------------------------------------------------------------------------

/// Per-view data-availability flags, computed from raw entity tables at
/// share-read time. Decoupled from the persisted topology graph because
/// that graph is view-specific — rebuilt under one view doesn't contain
/// the other views' nodes/edges.
///
/// Add a field per view-specific data requirement. Views that always
/// have the data they need (L3Logical / Workloads today) don't get a
/// field — `TopologyView::is_supported` just returns `true` for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopologyViewSupport {
    /// L2Physical requires interface-level neighbor discovery (LLDP/CDP).
    pub l2_physical: bool,
    /// Application requires at least one application-flagged tag to be
    /// assigned to an entity in the topology's network.
    pub application: bool,
}

impl TopologyView {
    /// Whether a topology with the given support flags has enough data to
    /// render this view. Exhaustive by design — adding a new `TopologyView`
    /// variant will fail compilation here until its eligibility rule is
    /// declared.
    pub fn is_supported(&self, support: &TopologyViewSupport) -> bool {
        match self {
            Self::L3Logical => true,
            Self::L2Physical => support.l2_physical,
            Self::Workloads => true,
            Self::Application => support.application,
        }
    }
}

// ---------------------------------------------------------------------------
// ViewElementConfig — defines how a view structures its elements
// ---------------------------------------------------------------------------

/// Defines the entity hierarchy for a topology view:
/// container (grouping box) → element (rendered as nodes) → inline (shown inside nodes)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ViewElementConfig {
    /// Entity rendered as the container/grouping box (e.g. Subnet in L3, Host in L2/Workloads)
    pub container_entity: Option<EntityDiscriminants>,
    /// Per-element-entity config: the entity types rendered as element nodes and
    /// (for each) which other entity types are shown *inside* those cards.
    /// Replaces the old flat `inline_entities` so views like Workloads — where
    /// Host elements inline services but Service elements inline nothing — can
    /// be expressed correctly.
    pub element_entities: Vec<ViewElementEntityConfig>,
    /// Generic metadata filters keyed by the entity they apply to in this
    /// view. Applied regardless of the entity's role (container/element/
    /// inline) — a Service metadata filter renders under the Services
    /// section whether Service is an element entity (Workloads/Application)
    /// or an inline entity (L3).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata_filters: HashMap<EntityDiscriminants, Vec<MetadataFilter>>,
    /// Filter values this view hides out of the box, keyed the same way as the
    /// hide-set in request options.
    ///
    /// These are product defaults rather than user filters: they keep a view
    /// legible before anyone has touched it, so "clear all filters" preserves
    /// them and the "filters applied" count ignores them. Declared here so the
    /// backend's initial hide-set and the frontend's notion of what counts as a
    /// default are the same list — they were separately hardcoded before, in
    /// `default_hide_metadata_values` and again in the options panel.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub default_hidden_values:
        HashMap<EntityDiscriminants, HashMap<MetadataFilterType, Vec<String>>>,
    /// Single noun spanning all element entities. Used in summaries when the
    /// per-entity breakdown would be confusing (e.g. mixed Host+Service in
    /// Workloads, where everything is conceptually a "workload"). Singular;
    /// the UI pluralizes as needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collective_noun: Option<String>,
}

impl ViewElementConfig {
    /// Attach the staleness filter to every entity in this view that both
    /// carries a freshness verdict and can actually be hidden by one.
    ///
    /// Derived from the view's own hierarchy rather than hand-written per view,
    /// so a new view — or a view that gains a Host/Service slot — picks the
    /// filter up automatically and cannot forget it.
    ///
    /// Scope: Host and Service only. They are the two entities with a
    /// `source` (so a freshness verdict is meaningful — see
    /// `DiscoveryTracked::is_discovery_managed`) and the two the inventory
    /// already badges. Restricted to entities in an element or inline role
    /// because the generic hide path only removes element nodes and inline
    /// services; declaring it for a container-role Host would render a filter
    /// that silently does nothing.
    fn add_staleness_filters(&mut self) {
        let is_element = |entity: EntityDiscriminants| {
            self.element_entities
                .iter()
                .any(|e| e.entity_type == entity)
        };
        let is_inline = |entity: EntityDiscriminants| {
            self.element_entities
                .iter()
                .any(|e| e.inline_entities.contains(&entity))
        };

        for entity in [EntityDiscriminants::Host, EntityDiscriminants::Service] {
            if !is_element(entity) && !is_inline(entity) {
                continue;
            }
            self.metadata_filters
                .entry(entity)
                .or_default()
                .push(MetadataFilter {
                    filter_type: MetadataFilterType::Staleness,
                    label: "By staleness".to_string(),
                    // `New` is a digest-only bucket — it means "created during
                    // the scan window being reported on" and has no meaning in
                    // a live topology — so the values are enumerated rather
                    // than taken from the whole enum.
                    values: [EntityFreshness::Current, EntityFreshness::Stale]
                        .into_iter()
                        .map(|f| {
                            <EntityFreshness as MetadataProvider<TypeMetadata>>::to_metadata(&f)
                                .into()
                        })
                        .collect(),
                    applies: FilterApplication::Client,
                });
        }
    }
}

/// An element-entity slot inside a view, plus the set of entity types that
/// render inline on that element's card. Card rendering (what services/ports
/// show up inside an element) and layout re-trigger fingerprinting both read
/// from this list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ViewElementEntityConfig {
    /// Entity type drawn as an element in this view.
    pub entity_type: EntityDiscriminants,
    /// Entity types folded into that element rather than drawn separately.
    pub inline_entities: Vec<EntityDiscriminants>,
}

// ---------------------------------------------------------------------------
// MetadataFilter — declared per (view, element entity) on the backend; drives
// both the filter-panel chip UI and the hide-set stored in request options.
// ---------------------------------------------------------------------------

/// The kind of metadata filter. One variant per conceptually-distinct filter
/// across the app — Category (on Service), Virtualization (on Host), and so
/// on. Kept narrow on purpose: adding a new filter means adding a variant
/// here + a `HasFilterValues` impl on the relevant entity.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Ord,
    PartialOrd,
    ToSchema,
    EnumIter,
    IntoStaticStr,
)]
pub enum MetadataFilterType {
    Category,
    Virtualization,
    /// Whether a port resolved to a neighbour. Declared on Interface in L2, where it is the
    /// difference between the handful of ports that carry the fabric and the ifTable rows that
    /// merely exist.
    LinkState,
    /// How recently discovery observed the entity. Unlike the others, this
    /// value is not intrinsic to the entity — it depends on the entity's
    /// network staleness window and the current time — so the frontend
    /// extractor for it takes the network as context.
    Staleness,
}

impl HasId for MetadataFilterType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

/// Where a filter is applied — and therefore what toggling it costs.
///
/// Filtering on the client makes a toggle instant, because the entities are already there. Filtering
/// on the server makes it a round trip, but keeps the hidden entities out of the response entirely.
/// That is a good trade only when the hidden set is large: L2 ships 19,095 interfaces to render
/// 2,872, and a customer's browser runs out of memory holding the difference.
///
/// **A filter may only be `Server` if its value is computable from the topology the backend is
/// already building.** `Staleness` fails that test — it depends on the current time, so an identical
/// request would return different results as time passes, and a cached response would be wrong. It
/// stays `Client` permanently; this is a property of the filter, not a migration left half-done.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    EnumIter,
    IntoStaticStr,
)]
pub enum FilterApplication {
    /// Applied in the browser against entities the response already contains.
    Client,
    /// Applied while building the topology; hidden entities never reach the client.
    Server,
}

impl HasId for FilterApplication {
    fn id(&self) -> &'static str {
        self.into()
    }
}

/// A declared metadata filter on an element entity inside a view.
///
/// `FilterValue::id` is the **stable identifier** — it's what each entity
/// emits via `HasFilterValues`, what the hide-set in request options stores,
/// and what matches entities on toggle. Changing an `id` silently clears
/// users' persisted hide entries for that value; `label` is display-only
/// and can change freely.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetadataFilter {
    /// What the filter narrows by.
    pub filter_type: MetadataFilterType,
    /// User-facing sub-section label (e.g. "By Category", "By Virtualization").
    pub label: String,
    /// The choices offered for this filter.
    pub values: Vec<FilterValue>,
    /// Where this filter runs. Deliberately not defaulted: a new filter has to state which side it
    /// belongs on, and the compiler is a better place to force that decision than a review comment.
    /// See `FilterApplication` for when `Server` is permissible.
    pub applies: FilterApplication,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FilterValue {
    /// Server-assigned unique identifier.
    pub id: String,
    /// Human-facing label for this choice.
    pub label: String,
    /// Colour shown on the filter chip.
    pub color: Color,
    /// Icon shown on the filter chip.
    #[schema(value_type = Option<String>, required)]
    pub icon: Option<Icon>,
}

/// `TypeMetadata` → `FilterValue`. Any enum whose values already implement
/// `TypeMetadataProvider` (ServiceCategory, HostVirtualizationState, etc.)
/// gets a ready-made `FilterValue` for free via `.to_metadata().into()` —
/// no hand-maintained parallel list.
impl From<TypeMetadata> for FilterValue {
    fn from(tm: TypeMetadata) -> Self {
        FilterValue {
            id: tm.id.to_string(),
            label: tm.name.unwrap_or(tm.id).to_string(),
            color: tm.color,
            icon: tm.icon,
        }
    }
}

/// Enumerate a `TypeMetadataProvider` strum enum straight into `FilterValue`s.
pub fn filter_values_from_enum<T>() -> Vec<FilterValue>
where
    T: TypeMetadataProvider + IntoEnumIterator,
{
    T::iter()
        .map(|v| <T as MetadataProvider<TypeMetadata>>::to_metadata(&v).into())
        .collect()
}

/// Cross-entity state a filter value may depend on.
///
/// Deliberately carries topology-build state only — no network and no clock. A filter that needs
/// either is a filter that cannot run on the server (see `FilterApplication`), so admitting them
/// here would only make it easy to build something that returns different answers over time.
#[derive(Debug, Default)]
pub struct FilterValueContext {
    /// Interfaces named as another interface's neighbour.
    ///
    /// A link is recorded on one side only, so an interface with no `neighbor` of its own is still
    /// linked if something points at it. Judging the outbound direction alone is a real bug and not
    /// a theoretical one: the frontend shipped it and drew 11 edges where it should have drawn 720.
    pub interfaces_referenced_as_neighbours: HashSet<Uuid>,
    /// Interfaces with at least one live row of their own in either resolved-neighbour table
    /// (`interface_neighbor_interfaces` / `interface_neighbor_hosts`).
    ///
    /// The successor to reading `Interface.neighbor.is_some()` directly, now that a port's
    /// resolved adjacencies live in a separate `Vec` rather than a scalar field on the interface.
    pub interfaces_with_neighbours: HashSet<Uuid>,
}

/// Entities that can surface metadata filter values. Returns the stable
/// `FilterValue::id` (per `MetadataFilterType`) for each filter the entity
/// participates in. Serialized alongside the entity so the frontend
/// doesn't need any extractor logic.
pub trait HasFilterValues {
    fn filter_values(&self, ctx: &FilterValueContext) -> BTreeMap<MetadataFilterType, String>;
}

// ---------------------------------------------------------------------------
// Inspector types
// ---------------------------------------------------------------------------

/// Whether dependencies are tracked by service or by specific binding
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    EnumIter,
    IntoStaticStr,
)]
pub enum DependencyMemberType {
    Services,
    Bindings,
}

/// A section that can appear in the inspector panel
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    EnumIter,
    IntoStaticStr,
)]
pub enum InspectorSection {
    Identity,
    IfEntryData,
    Services,
    Dependencies,
    HostDetail,
    Virtualization,
    OtherInterfaces,
    PortBindings,
    SubnetDetail,
    ElementSummary,
    DependencySummary,
    Application,
}

/// View-specific inspector panel configuration.
/// Determines which sections appear and in what order for each view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ViewInspectorConfig {
    /// Inspector sections shown when an element is selected.
    pub element_sections: Vec<InspectorSection>,
    /// Inspector sections shown when a container is selected.
    pub container_sections: Vec<InspectorSection>,
    /// What kind of member a dependency can be drawn between in this view.
    pub dependency_creation: Option<DependencyMemberType>,
    /// Whether the inspector offers the application picker.
    pub show_application_picker: bool,
}

// ---------------------------------------------------------------------------
// TopologyView — metadata impls
// ---------------------------------------------------------------------------

impl EntityMetadataProvider for TopologyView {
    fn color(&self) -> Color {
        match self {
            Self::L2Physical => Concept::L2.color(),
            Self::L3Logical => Concept::L3.color(),
            Self::Workloads => Concept::Workloads.color(),
            Self::Application => Concept::Application.color(),
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::L2Physical => Concept::L2.icon(),
            Self::L3Logical => Concept::L3.icon(),
            Self::Workloads => Concept::Workloads.icon(),
            Self::Application => Concept::Application.icon(),
        }
    }
}

impl TypeMetadataProvider for TopologyView {
    fn name(&self) -> &'static str {
        match self {
            Self::L2Physical => "L2 Physical",
            Self::L3Logical => "L3 Logical",
            Self::Workloads => "Workloads",
            Self::Application => "Applications",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::L2Physical => "Interfaces and physical links between hosts",
            Self::L3Logical => "IP addresses and connectivity across subnets",
            Self::Workloads => "Services, VMs, and containers per host",
            Self::Application => "Services per application",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let edge_view_configs: serde_json::Map<String, serde_json::Value> =
            EdgeTypeDiscriminants::iter()
                .map(|et| {
                    (
                        et.to_string(),
                        serde_json::to_value(self.edge_view_config(et)).unwrap(),
                    )
                })
                .collect();

        serde_json::json!({
            "element_config": self.element_config(),
            "edge_view_configs": edge_view_configs,
            "inspector_config": self.inspector_config()
        })
    }
}

// ---------------------------------------------------------------------------
// TopologyView — typed methods
// ---------------------------------------------------------------------------

impl TopologyView {
    /// The entity hierarchy for this view: parent → element → inline
    pub fn element_config(&self) -> ViewElementConfig {
        let mut config = self.base_element_config();
        config.add_staleness_filters();
        config.default_hidden_values = self.default_hidden_values();
        config
    }

    /// Filter values hidden out of the box in this view.
    ///
    /// The single definition of what counts as a product default. `TopologyRequestOptions`
    /// seeds a new topology's hide-set from it, and the frontend reads the same list out of the
    /// generated view fixture to decide which chips "clear all" preserves and which ones the
    /// "filters applied" badge ignores.
    fn default_hidden_values(
        &self,
    ) -> HashMap<EntityDiscriminants, HashMap<MetadataFilterType, Vec<String>>> {
        let mut by_entity: HashMap<EntityDiscriminants, HashMap<MetadataFilterType, Vec<String>>> =
            HashMap::new();

        // Open ports are the noisiest thing a scan produces and are covered by the
        // ByServiceCategory element rule; this is the chip-level toggle state.
        by_entity.insert(
            EntityDiscriminants::Service,
            HashMap::from([(
                MetadataFilterType::Category,
                vec![ServiceCategory::OpenPorts.id().to_string()],
            )]),
        );

        // L2 draws one element per ifTable row, so unlinked ports dominate its node count while
        // carrying none of its adjacency. Hidden by default, one click from being shown again.
        if matches!(self, Self::L2Physical) {
            by_entity.insert(
                EntityDiscriminants::Interface,
                HashMap::from([(
                    MetadataFilterType::LinkState,
                    vec![InterfaceLinkState::Unlinked.id().to_string()],
                )]),
            );
        }

        by_entity
    }

    /// Per-view hierarchy without the filters that are derived from it.
    fn base_element_config(&self) -> ViewElementConfig {
        match self {
            Self::L3Logical => ViewElementConfig {
                container_entity: Some(EntityDiscriminants::Subnet),
                element_entities: vec![ViewElementEntityConfig {
                    entity_type: EntityDiscriminants::IPAddress,
                    // Port first so it surfaces above Service in the filter
                    // panel (Service's category chips push it below the fold
                    // otherwise). Card rendering reads this list as a set,
                    // so order is irrelevant there.
                    inline_entities: vec![EntityDiscriminants::Port, EntityDiscriminants::Service],
                }],
                metadata_filters: [(
                    EntityDiscriminants::Service,
                    vec![MetadataFilter {
                        filter_type: MetadataFilterType::Category,
                        label: "By category".to_string(),
                        values: filter_values_from_enum::<ServiceCategory>(),
                        applies: FilterApplication::Client,
                    }],
                )]
                .into_iter()
                .collect(),
                // Populated by `element_config`, which is the only public constructor.
                default_hidden_values: HashMap::new(),
                collective_noun: None,
            },
            Self::L2Physical => ViewElementConfig {
                container_entity: Some(EntityDiscriminants::Host),
                element_entities: vec![ViewElementEntityConfig {
                    entity_type: EntityDiscriminants::Interface,
                    inline_entities: vec![],
                }],
                metadata_filters: [(
                    EntityDiscriminants::Interface,
                    vec![MetadataFilter {
                        filter_type: MetadataFilterType::LinkState,
                        label: "By link".to_string(),
                        values: filter_values_from_enum::<InterfaceLinkState>(),
                        // The only server-side filter. It hides ~16,000 of L2's 19,095 interfaces —
                        // ifTable rows with no neighbour — which is the difference between a view that
                        // loads and one that exhausts browser memory. Link state is derivable from the
                        // topology being built, so it qualifies.
                        applies: FilterApplication::Server,
                    }],
                )]
                .into_iter()
                .collect(),
                // Populated by `element_config`, which is the only public constructor.
                default_hidden_values: HashMap::new(),
                collective_noun: None,
            },
            Self::Workloads => ViewElementConfig {
                container_entity: Some(EntityDiscriminants::Host),
                element_entities: vec![
                    ViewElementEntityConfig {
                        entity_type: EntityDiscriminants::Service,
                        inline_entities: vec![],
                    },
                    ViewElementEntityConfig {
                        entity_type: EntityDiscriminants::Host,
                        inline_entities: vec![EntityDiscriminants::Service],
                    },
                ],
                metadata_filters: [
                    (
                        EntityDiscriminants::Service,
                        vec![MetadataFilter {
                            filter_type: MetadataFilterType::Category,
                            label: "By category".to_string(),
                            values: filter_values_from_enum::<ServiceCategory>(),
                            applies: FilterApplication::Client,
                        }],
                    ),
                    (
                        EntityDiscriminants::Host,
                        vec![MetadataFilter {
                            filter_type: MetadataFilterType::Virtualization,
                            label: "By virtualization".to_string(),
                            values: filter_values_from_enum::<HostVirtualizationState>(),
                            applies: FilterApplication::Client,
                        }],
                    ),
                ]
                .into_iter()
                .collect(),
                // Populated by `element_config`, which is the only public constructor.
                default_hidden_values: HashMap::new(),
                collective_noun: Some("workload".to_string()),
            },
            Self::Application => ViewElementConfig {
                container_entity: None,
                element_entities: vec![ViewElementEntityConfig {
                    entity_type: EntityDiscriminants::Service,
                    inline_entities: vec![],
                }],
                metadata_filters: [(
                    EntityDiscriminants::Service,
                    vec![MetadataFilter {
                        filter_type: MetadataFilterType::Category,
                        label: "By category".to_string(),
                        values: filter_values_from_enum::<ServiceCategory>(),
                        applies: FilterApplication::Client,
                    }],
                )]
                .into_iter()
                .collect(),
                // Populated by `element_config`, which is the only public constructor.
                default_hidden_values: HashMap::new(),
                collective_noun: None,
            },
        }
    }

    /// Per-view configuration for each edge type.
    /// All match arms are exhaustive — adding a new EdgeTypeDiscriminants variant
    /// will cause a compile error here, forcing a configuration decision.
    pub fn edge_view_config(&self, edge_type: EdgeTypeDiscriminants) -> EdgeViewConfig {
        use EdgeDefaultVisibility::*;
        use EdgeHighlightBehavior::*;
        use EdgeStroke::*;
        use EdgeTypeDiscriminants::*;

        let active = |affects_layout,
                      visibility,
                      stroke,
                      highlight,
                      will_target_container,
                      show_directionality| {
            EdgeViewConfig::Active {
                affects_layout,
                default_visibility: visibility,
                stroke,
                highlight_behavior: highlight,
                will_target_container,
                show_directionality,
            }
        };

        match self {
            Self::L3Logical => match edge_type {
                SameHost => active(true, Visible, Solid, WhenVisible, false, false),
                ContainerRuntime => active(true, Visible, Solid, WhenVisible, true, false),
                RequestPath => active(false, Visible, Dashed, WhenVisible, false, true),
                HubAndSpoke => active(false, Visible, Dashed, WhenVisible, false, true),
                Hypervisor => active(false, Hidden, Dashed, WhenVisible, true, false),
                PhysicalLink => EdgeViewConfig::Disabled,
                // Connects two hosts, and this view has no host node to land on — its
                // elements are IP addresses.
                NeighborLink => EdgeViewConfig::Disabled,
                // Annotates the graph rather than structuring it: no layout influence, off by
                // default, and never elevated to the subnet box — the point is that one
                // container spans them, which is lost if the edge targets the boxes.
                SameContainer => active(false, Hidden, Dotted, WhenVisible, false, false),
            },
            Self::L2Physical => match edge_type {
                PhysicalLink => active(true, Visible, Solid, WhenVisible, false, false),
                // Home view for an adjacency we can only place at device level: dashed marks
                // it as the approximate one, and it still steers layout so the neighbour is
                // drawn beside the device it neighbours.
                NeighborLink => active(true, Visible, Dashed, WhenVisible, false, false),
                SameHost => active(false, Hidden, Dashed, WhenVisible, false, false),
                Hypervisor | ContainerRuntime | RequestPath | HubAndSpoke | SameContainer => {
                    EdgeViewConfig::Disabled
                }
            },
            Self::Workloads => match edge_type {
                PhysicalLink => active(false, Hidden, Dashed, WhenVisible, false, false),
                NeighborLink => active(false, Hidden, Dashed, WhenVisible, false, false),
                RequestPath | HubAndSpoke => {
                    active(false, Hidden, Dashed, WhenVisible, false, true)
                }
                Hypervisor | ContainerRuntime | SameHost | SameContainer => {
                    EdgeViewConfig::Disabled
                }
            },
            Self::Application => match edge_type {
                RequestPath => active(true, Visible, Solid, WhenVisible, false, true),
                HubAndSpoke => active(true, Visible, Solid, WhenVisible, false, true),
                ContainerRuntime => active(true, Hidden, Dashed, Always, true, false),
                SameHost | Hypervisor | PhysicalLink | SameContainer | NeighborLink => {
                    EdgeViewConfig::Disabled
                }
            },
        }
    }

    /// Inspector panel configuration for this view
    pub fn inspector_config(&self) -> ViewInspectorConfig {
        match self {
            Self::L3Logical => ViewInspectorConfig {
                element_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::HostDetail,
                    InspectorSection::IfEntryData,
                    InspectorSection::Services,
                    InspectorSection::OtherInterfaces,
                ],
                container_sections: vec![
                    InspectorSection::SubnetDetail,
                    InspectorSection::ElementSummary,
                ],
                dependency_creation: Some(DependencyMemberType::Bindings),
                show_application_picker: false,
            },
            Self::Application => ViewInspectorConfig {
                element_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::Dependencies,
                    InspectorSection::Application,
                ],
                container_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::ElementSummary,
                    InspectorSection::DependencySummary,
                ],
                dependency_creation: Some(DependencyMemberType::Services),
                show_application_picker: true,
            },
            Self::Workloads => ViewInspectorConfig {
                element_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::HostDetail,
                    InspectorSection::Virtualization,
                    InspectorSection::Services,
                    InspectorSection::OtherInterfaces,
                ],
                container_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::ElementSummary,
                ],
                dependency_creation: Some(DependencyMemberType::Services),
                show_application_picker: false,
            },
            Self::L2Physical => ViewInspectorConfig {
                element_sections: vec![InspectorSection::Identity, InspectorSection::IfEntryData],
                container_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::ElementSummary,
                ],
                dependency_creation: None,
                show_application_picker: false,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn topology_view_serde_round_trip() {
        let json = serde_json::to_value(TopologyView::L2Physical).unwrap();
        assert_eq!(json, "L2Physical");
        let deserialized: TopologyView = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, TopologyView::L2Physical);
    }

    #[test]
    fn all_views_have_non_empty_sections() {
        for view in TopologyView::iter() {
            let config = view.inspector_config();
            assert!(
                !config.element_sections.is_empty(),
                "{:?} has no element sections",
                view
            );
            assert!(
                !config.container_sections.is_empty(),
                "{:?} has no container sections",
                view
            );
        }
    }

    #[test]
    fn serde_roundtrip_inspector_section() {
        let section = InspectorSection::Dependencies;
        let json = serde_json::to_string(&section).unwrap();
        let deserialized: InspectorSection = serde_json::from_str(&json).unwrap();
        assert_eq!(section, deserialized);
    }

    #[test]
    fn serde_roundtrip_config() {
        let config = TopologyView::Application.inspector_config();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ViewInspectorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
