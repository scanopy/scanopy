use std::collections::{BTreeMap, HashMap, HashSet};

use crate::server::{
    hosts::r#impl::virtualization::HostVirtualizationState,
    interfaces::r#impl::base::{IfOperStatus, InterfaceLinkState},
    services::r#impl::categories::ServiceCategory,
    shared::types::guest_state::GuestState,
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
    /// assigned to an entity in the topology's site.
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
    /// The metadata filters this view offers, each naming the entities it reads and hides.
    /// Applied regardless of an entity's role (container/element/inline): a Service filter
    /// renders under the Services section whether Service is an element entity
    /// (Workloads/Application) or an inline entity (L3). A filter naming several entities renders
    /// in a section of its own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata_filters: Vec<MetadataFilter>,
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
    /// The colours this view paints on its element cards, each tied to the filter value that
    /// explains it. A card shows a classification colour only through one of these, so every
    /// colour has a chip in the filter panel whose hover rings the same cards. Per channel, earlier
    /// marks win.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub element_marks: Vec<ElementMark>,
}

/// Where on an element card a mark paints.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema, EnumIter)]
pub enum MarkChannel {
    /// The whole card, when it draws as a bare box below the detail threshold.
    StateFill,
    /// A dot on an icon card's tile, top-left corner, or in a row card's trailing slot, in the
    /// value's colour; it expands to name the value on hover. Workloads uses it for a guest's run
    /// state, L2 for a port's oper status.
    Status,
}

/// A colour a view paints on its element cards, named by the filter value that decodes it.
///
/// The colour is that value's `FilterValue::color`; the mark carries none of its own, so the card
/// and the chip cannot disagree. A card is marked when its own entity, or its host, carries
/// `value` under `filter_type`, judged by the extractor the filter's hover uses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ElementMark {
    pub channel: MarkChannel,
    /// The entity whose filter value is read: the card's own, or `Host` for the card's host.
    pub entity: EntityDiscriminants,
    pub filter_type: MetadataFilterType,
    /// The `FilterValue::id` that paints.
    pub value: String,
}

impl ElementMark {
    fn new(
        channel: MarkChannel,
        entity: EntityDiscriminants,
        filter_type: MetadataFilterType,
        value: &impl HasId,
    ) -> Self {
        Self {
            channel,
            entity,
            filter_type,
            value: value.id().to_string(),
        }
    }
}

/// Entities the topology can filter by staleness: those with a stale badge in the inventory or
/// on a topology card. Port is left out because nothing badges it.
const STALENESS_FILTERED_ENTITIES: [EntityDiscriminants; 5] = [
    EntityDiscriminants::Host,
    EntityDiscriminants::Service,
    EntityDiscriminants::IPAddress,
    EntityDiscriminants::Interface,
    EntityDiscriminants::Subnet,
];

impl ViewElementConfig {
    /// The entity types this view draws: its container, its elements and their inline entities.
    pub fn drawn_entities(&self) -> Vec<EntityDiscriminants> {
        self.container_entity
            .into_iter()
            .chain(
                self.element_entities
                    .iter()
                    .flat_map(|e| std::iter::once(e.entity_type).chain(e.inline_entities.clone())),
            )
            .collect()
    }

    /// Attach the staleness filter to every entity in this view that both
    /// carries a freshness verdict and can actually be hidden by one.
    ///
    /// Derived from the view's own hierarchy rather than hand-written per view,
    /// so a new view — or a view that gains a Host/Service slot — picks the
    /// filter up automatically and cannot forget it.
    ///
    /// Scope: every entity in `STALENESS_FILTERED_ENTITIES`, in whatever role
    /// the view gives it: container (a Workloads or L2 host box, an L3 subnet
    /// box), element, or inline. Each entity is judged on its own
    /// `last_seen_at`, so a stale host's IP in L3 is hidden by the IP's own
    /// verdict rather than inherited from the host.
    fn add_staleness_filters(&mut self) {
        let is_container = |entity: EntityDiscriminants| self.container_entity == Some(entity);
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

        for entity in STALENESS_FILTERED_ENTITIES {
            if !is_container(entity) && !is_element(entity) && !is_inline(entity) {
                continue;
            }
            self.metadata_filters.push(MetadataFilter {
                filter_type: MetadataFilterType::Staleness,
                // One per entity: each is judged on its own `last_seen_at`, so hiding stale
                // services leaves stale hosts alone.
                entities: vec![entity],
                label: "By staleness".to_string(),
                // `New` is a digest-only bucket — it means "created during
                // the scan window being reported on" and has no meaning in
                // a live topology — so the values are enumerated rather
                // than taken from the whole enum.
                values: [EntityFreshness::Current, EntityFreshness::Stale]
                    .into_iter()
                    .map(|f| {
                        <EntityFreshness as MetadataProvider<TypeMetadata>>::to_metadata(&f).into()
                    })
                    .collect(),
                applies: FilterApplication::Client,
            });
        }
    }

    /// Add a Confirmation filter for each entity in `CONFIRMATION_FILTERED_ENTITIES` the view draws,
    /// in whatever role, the way `add_staleness_filters` does for staleness.
    fn add_confirmation_filters(&mut self) {
        let drawn = self.drawn_entities();
        for entity in CONFIRMATION_FILTERED_ENTITIES {
            if !drawn.contains(&entity) {
                continue;
            }
            self.metadata_filters.push(MetadataFilter {
                filter_type: MetadataFilterType::Confirmation,
                entities: vec![entity],
                label: "By confirmation".to_string(),
                values: filter_values_from_enum::<ConfirmationFilterValue>(),
                applies: FilterApplication::Client,
            });
        }
    }

    /// The filters that read and hide `entity`.
    pub fn filters_for(
        &self,
        entity: EntityDiscriminants,
    ) -> impl Iterator<Item = &MetadataFilter> {
        self.metadata_filters
            .iter()
            .filter(move |f| f.entities.contains(&entity))
    }

    /// The filter of `filter_type` covering `entity`. At most one exists per view (tested), which
    /// is what lets the hide-set be keyed by entity and filter type.
    pub fn filter(
        &self,
        entity: EntityDiscriminants,
        filter_type: MetadataFilterType,
    ) -> Option<&MetadataFilter> {
        self.filters_for(entity)
            .find(|f| f.filter_type == filter_type)
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

/// The type of metadata filter. One variant per conceptually-distinct filter
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
    /// site staleness window and the current time — so the frontend
    /// extractor for it takes the site as context.
    Staleness,
    /// A port's ifOperStatus. Declared on Interface in L2, where it is what the card's status dot
    /// and zoomed-out fill show.
    OperStatus,
    /// Whether a guest (a Proxmox guest or a container host) was running at the last scan.
    /// Declared on Host and Service in Workloads, and on Service in Application, where it is the
    /// status dot on an icon card.
    GuestState,
    /// Whether the entity is confirmed or known only from another device's report. Declared on
    /// Host and Subnet in every view that draws them; computed in the browser from the source's
    /// `unconfirmed` flag, as Staleness is computed there, since subnets carry no filter values.
    Confirmation,
}

impl HasId for MetadataFilterType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

/// The two values of the Confirmation filter. The verdict itself is
/// [`EntitySource::confirmation`](crate::server::shared::types::entities::EntitySource::confirmation);
/// this only names its two outcomes for the filter chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum ConfirmationFilterValue {
    Confirmed,
    Unconfirmed,
}

impl HasId for ConfirmationFilterValue {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ConfirmationFilterValue {
    fn color(&self) -> Color {
        match self {
            Self::Confirmed => Color::Green,
            // The Inferred source's colour: an unconfirmed entity is exactly an inferred one.
            Self::Unconfirmed => Color::Violet,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Confirmed => Icon::CircleCheck,
            Self::Unconfirmed => Icon::CircleDashedCheck,
        }
    }
}

impl TypeMetadataProvider for ConfirmationFilterValue {
    fn name(&self) -> &'static str {
        match self {
            Self::Confirmed => "Confirmed",
            Self::Unconfirmed => "Unconfirmed",
        }
    }
}

/// Entities that can be unconfirmed, and so carry the Confirmation filter wherever a view draws
/// them.
const CONFIRMATION_FILTERED_ENTITIES: [EntityDiscriminants; 2] =
    [EntityDiscriminants::Host, EntityDiscriminants::Subnet];

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
    /// The entities this filter reads and hides. Hovering a value rings every listed entity
    /// carrying it; hiding one writes it into the hide-set under each listed entity.
    pub entities: Vec<EntityDiscriminants>,
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
/// Deliberately carries topology-build state only — no site and no clock. A filter that needs
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
    /// Interfaces whose forwarding table learned a known host's MAC (`fdb::learned_hosts`). A
    /// device is reached through the port even where no LLDP/CDP or single-MAC link was resolved,
    /// so the port counts as linked.
    pub interfaces_learning_known_hosts: HashSet<Uuid>,
    /// Interfaces a port's forwarding table reached (`fdb::learned_interfaces`): the far end of a
    /// learned link, linked the way a port a neighbour row names is.
    pub interfaces_learned_by_ports: HashSet<Uuid>,
}

/// Entities that can surface metadata filter values. Returns the stable
/// `FilterValue::id` (per `MetadataFilterType`) for each filter the entity
/// participates in.
///
/// The server-side filter (`metadata_filter::retain_visible`) calls this directly. The topology
/// bundle also stamps the result on each wrapped host, service and interface as `filter_values`
/// (`TopologyData::stamp_filter_values`), which is what the frontend filters, hovers and colours
/// on. Staleness is the one filter absent here: it depends on the clock and the site window, so
/// the frontend computes it.
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
    Services,
    Dependencies,
    HostDetail,
    Virtualization,
    OtherInterfaces,
    PortBindings,
    ElementSummary,
    DependencySummary,
    Application,
}

impl HasId for InspectorSection {
    fn id(&self) -> &'static str {
        self.into()
    }
}

/// Each section's heading in the inspector: what it holds, drawn with the entity or concept it
/// is about.
impl EntityMetadataProvider for InspectorSection {
    fn color(&self) -> Color {
        match self {
            InspectorSection::Identity | InspectorSection::ElementSummary => Color::Gray,
            InspectorSection::Services => EntityDiscriminants::Service.color(),
            InspectorSection::Dependencies | InspectorSection::DependencySummary => {
                EntityDiscriminants::Dependency.color()
            }
            InspectorSection::HostDetail => EntityDiscriminants::Host.color(),
            InspectorSection::Virtualization => Concept::Virtualization.color(),
            InspectorSection::OtherInterfaces => EntityDiscriminants::IPAddress.color(),
            InspectorSection::PortBindings => EntityDiscriminants::Port.color(),
            InspectorSection::Application => Concept::Application.color(),
        }
    }

    fn icon(&self) -> Icon {
        match self {
            InspectorSection::Identity => Icon::Crosshair,
            InspectorSection::ElementSummary => Icon::PackageOpen,
            InspectorSection::Services => EntityDiscriminants::Service.icon(),
            InspectorSection::Dependencies | InspectorSection::DependencySummary => {
                EntityDiscriminants::Dependency.icon()
            }
            InspectorSection::HostDetail => EntityDiscriminants::Host.icon(),
            InspectorSection::Virtualization => Concept::Virtualization.icon(),
            InspectorSection::OtherInterfaces => EntityDiscriminants::IPAddress.icon(),
            InspectorSection::PortBindings => EntityDiscriminants::Port.icon(),
            InspectorSection::Application => Concept::Application.icon(),
        }
    }
}

impl TypeMetadataProvider for InspectorSection {
    fn name(&self) -> &'static str {
        match self {
            InspectorSection::Identity => "Selected",
            InspectorSection::Services => "Services",
            InspectorSection::Dependencies => "Dependencies",
            InspectorSection::HostDetail => "Host",
            InspectorSection::Virtualization => "Runs on",
            InspectorSection::OtherInterfaces => "IP addresses",
            InspectorSection::PortBindings => "Port bindings",
            InspectorSection::ElementSummary => "Contents",
            InspectorSection::DependencySummary => "Crossing dependencies",
            InspectorSection::Application => "Application",
        }
    }

    /// One line under the heading saying what the section holds and why it is here.
    fn description(&self) -> &'static str {
        match self {
            InspectorSection::Identity => "The entity you selected",
            InspectorSection::Services => {
                "Services on this host, and on the hosts it presents or runs"
            }
            InspectorSection::Dependencies => "Dependencies this service is part of",
            InspectorSection::HostDetail => "The host this element belongs to",
            InspectorSection::Virtualization => {
                "The hypervisor, container runtime or guest that this host runs under"
            }
            InspectorSection::OtherInterfaces => "Every address this host holds",
            InspectorSection::PortBindings => "Ports this service listens on",
            InspectorSection::ElementSummary => "What this box holds, counted by type",
            InspectorSection::DependencySummary => {
                "Dependencies with members both inside and outside this box"
            }
            InspectorSection::Application => "The application this service belongs to",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({ "shown_entity": self.shown_entity() })
    }
}

impl InspectorSection {
    /// The entity type whose cards this section draws. The selected entity's detail rows skip a
    /// field that points at an entity another section in the same panel already shows. `None`
    /// for sections that show the selection itself or only counts.
    pub fn shown_entity(&self) -> Option<EntityDiscriminants> {
        match self {
            InspectorSection::Identity
            | InspectorSection::ElementSummary
            | InspectorSection::PortBindings => None,
            InspectorSection::HostDetail => Some(EntityDiscriminants::Host),
            // The host's virtualizer is recorded as a service (`Host.virtualized_by`).
            InspectorSection::Services | InspectorSection::Virtualization => {
                Some(EntityDiscriminants::Service)
            }
            InspectorSection::OtherInterfaces => Some(EntityDiscriminants::IPAddress),
            InspectorSection::Dependencies | InspectorSection::DependencySummary => {
                Some(EntityDiscriminants::Dependency)
            }
            InspectorSection::Application => Some(EntityDiscriminants::Tag),
        }
    }
}

/// View-specific inspector panel configuration.
/// Determines which sections appear and in what order for each view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ViewInspectorConfig {
    /// Inspector sections shown when an element is selected.
    pub element_sections: Vec<InspectorSection>,
    /// Inspector sections shown when a container is selected.
    pub container_sections: Vec<InspectorSection>,
    /// What type of member a dependency can be drawn between in this view.
    pub dependency_creation: Option<DependencyMemberType>,
    /// Whether the inspector offers the application picker.
    pub show_application_picker: bool,
    /// Whether a service card shows the IP address and ports it is reached on. Application
    /// groups services by what they do, not where they listen, so it leaves these out.
    pub show_service_network_detail: bool,
}

// ---------------------------------------------------------------------------
// ViewLayoutConfig — how the UI lays out a view
// ---------------------------------------------------------------------------

/// Root-level layout algorithm for a view.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewLayoutAlgorithm {
    /// Layered left to right, with port-to-port edges between columns of containers and
    /// layered (rather than box-packed) children.
    LayeredRight,
    /// Layered top to bottom in model order, so the root order decides placement.
    Workloads,
    /// Rectangle packing: tiles mostly independent containers into a compact rectangle.
    RectPacking,
    /// Layered top to bottom with box-packed children.
    Default,
}

/// How edge endpoints pick the side of a node they attach to.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewHandleStrategy {
    /// Left or right side, facing the other endpoint.
    Horizontal,
    /// Shortest side pair that avoids crossing other nodes.
    Optimal,
}

/// How elements inside a container are ordered before layout.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewElementOrder {
    /// Grouped by the containers they connect to.
    Automatic,
    /// Up ports first, then down, then the rest; subcontainers ranked by how many up ports
    /// they hold.
    OperStatus,
}

/// How root containers are ordered before layout.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ViewRootOrder {
    /// The order the layout receives them in.
    Input,
    /// Hosts with the most application-relevant services and guest VMs first.
    WorkloadCount,
}

/// How the UI lays out a view. Read by the topology layout pipeline in place of branching on
/// the view id.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ViewLayoutConfig {
    pub algorithm: ViewLayoutAlgorithm,
    pub handle_strategy: ViewHandleStrategy,
    pub element_order: ViewElementOrder,
    pub root_order: ViewRootOrder,
    /// Whether edges between root containers are passed to the root layout.
    pub root_edges: bool,
    /// Whether root containers with no cross-container edges are moved beside the connected
    /// ones after layout.
    pub repack_disconnected: bool,
    /// Whether each column of root containers is aligned to the same top edge after layout.
    pub align_layers_top: bool,
    /// Whether the view switches to a force-directed overview when every root container is
    /// collapsed.
    pub force_overview: bool,
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
            "slug": self.slug(),
            "element_config": self.element_config(),
            "edge_view_configs": edge_view_configs,
            "inspector_config": self.inspector_config(),
            "layout": self.layout_config()
        })
    }
}

// ---------------------------------------------------------------------------
// TopologyView — typed methods
// ---------------------------------------------------------------------------

impl TopologyView {
    /// Every entity type some view draws, as container, element or inline entity. Read from each
    /// view's hierarchy, so a view that starts drawing a type adds it here.
    pub fn drawn_entities() -> HashSet<EntityDiscriminants> {
        Self::iter()
            .flat_map(|view| view.base_element_config().drawn_entities())
            .collect()
    }

    /// The entity hierarchy for this view: parent → element → inline
    pub fn element_config(&self) -> ViewElementConfig {
        let mut config = self.base_element_config();
        config.add_staleness_filters();
        config.add_confirmation_filters();
        config.default_hidden_values = self.default_hidden_values();
        config.element_marks = self.element_marks(&config);
        config
    }

    /// The colours this view paints on element cards, in priority order per channel.
    ///
    /// Every view fills a zoomed-out card amber when its element is stale, and nothing else: a
    /// zoomed-out card looks the same in every view. L2 used to fill a port by its oper status as
    /// well, which made it the one view whose boxes were green and red. L2 puts a status dot on
    /// each port's row in its oper-status colour, shown at full detail. Workloads and Application put a
    /// status dot on a guest's or container's icon card in its run-state colour. L3 marks no run
    /// state: it declares no guest state filter to explain one.
    fn element_marks(&self, config: &ViewElementConfig) -> Vec<ElementMark> {
        use EntityDiscriminants::{Host, Interface, Service};
        use MarkChannel::*;
        use MetadataFilterType::{GuestState as GuestStateFilter, OperStatus, Staleness};

        let stale_fill: Vec<ElementMark> = config
            .element_entities
            .iter()
            .filter(|e| STALENESS_FILTERED_ENTITIES.contains(&e.entity_type))
            .map(|e| ElementMark::new(StateFill, e.entity_type, Staleness, &EntityFreshness::Stale))
            .collect();
        let status = |channel, status: IfOperStatus| {
            ElementMark::new(channel, Interface, OperStatus, &status)
        };

        match self {
            Self::L2Physical => stale_fill
                .into_iter()
                .chain(IfOperStatus::iter().map(|s| status(Status, s)))
                .collect(),
            // A guest's run state, as a status dot on its icon card: a container's own (a bridge
            // container is a Service card), then a guest host's. Service first, so a container card
            // shows its own state over its host's. Anything with no run state (bare metal, a vCenter or
            // ESXi guest, a network identity, a service not in a container) is unmarked.
            Self::Workloads => [Service, Host]
                .into_iter()
                .flat_map(|entity| {
                    GuestState::iter()
                        .map(move |g| ElementMark::new(Status, entity, GuestStateFilter, &g))
                })
                .chain(stale_fill)
                .collect(),
            // A containerized service's run state, as its card's status dot.
            Self::Application => GuestState::iter()
                .map(|g| ElementMark::new(Status, Service, GuestStateFilter, &g))
                .chain(stale_fill)
                .collect(),
            Self::L3Logical => stale_fill,
        }
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
                    // An address's icon card draws its services as badges. Its ports have no
                    // place on the card, and only show in the inspector.
                    inline_entities: vec![EntityDiscriminants::Service],
                }],
                metadata_filters: vec![MetadataFilter {
                    filter_type: MetadataFilterType::Category,
                    entities: vec![EntityDiscriminants::Service],
                    label: "By category".to_string(),
                    values: filter_values_from_enum::<ServiceCategory>(),
                    applies: FilterApplication::Client,
                }],
                // Populated by `element_config`, which is the only public constructor.
                default_hidden_values: HashMap::new(),
                collective_noun: None,
                element_marks: Vec::new(),
            },
            Self::L2Physical => ViewElementConfig {
                container_entity: Some(EntityDiscriminants::Host),
                element_entities: vec![ViewElementEntityConfig {
                    entity_type: EntityDiscriminants::Interface,
                    inline_entities: vec![],
                }],
                metadata_filters: vec![
                    MetadataFilter {
                        filter_type: MetadataFilterType::LinkState,
                        entities: vec![EntityDiscriminants::Interface],
                        label: "By link".to_string(),
                        values: filter_values_from_enum::<InterfaceLinkState>(),
                        // The only server-side filter. It hides ~16,000 of L2's 19,095 interfaces —
                        // ifTable rows with no neighbour — which is the difference between a view that
                        // loads and one that exhausts browser memory. Link state is derivable from the
                        // topology being built, so it qualifies.
                        applies: FilterApplication::Server,
                    },
                    // Decodes the port status dot and zoomed-out fill (see `element_marks`).
                    MetadataFilter {
                        filter_type: MetadataFilterType::OperStatus,
                        entities: vec![EntityDiscriminants::Interface],
                        label: "By status".to_string(),
                        values: filter_values_from_enum::<IfOperStatus>(),
                        applies: FilterApplication::Client,
                    },
                ],
                // Populated by `element_config`, which is the only public constructor.
                default_hidden_values: HashMap::new(),
                collective_noun: None,
                element_marks: Vec::new(),
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
                metadata_filters: vec![
                    MetadataFilter {
                        filter_type: MetadataFilterType::Category,
                        entities: vec![EntityDiscriminants::Service],
                        label: "By category".to_string(),
                        values: filter_values_from_enum::<ServiceCategory>(),
                        applies: FilterApplication::Client,
                    },
                    // Both workload kinds: a guest or macvlan container is a Host card, a container
                    // on a bridge network is a Service card, and Containerized covers both.
                    MetadataFilter {
                        filter_type: MetadataFilterType::Virtualization,
                        entities: vec![EntityDiscriminants::Host, EntityDiscriminants::Service],
                        label: "By virtualization".to_string(),
                        values: filter_values_from_enum::<HostVirtualizationState>(),
                        applies: FilterApplication::Client,
                    },
                    MetadataFilter {
                        filter_type: MetadataFilterType::GuestState,
                        entities: vec![EntityDiscriminants::Host, EntityDiscriminants::Service],
                        label: "By guest state".to_string(),
                        values: filter_values_from_enum::<GuestState>(),
                        applies: FilterApplication::Client,
                    },
                ],
                // Populated by `element_config`, which is the only public constructor.
                default_hidden_values: HashMap::new(),
                collective_noun: Some("workload".to_string()),
                element_marks: Vec::new(),
            },
            Self::Application => ViewElementConfig {
                container_entity: None,
                element_entities: vec![ViewElementEntityConfig {
                    entity_type: EntityDiscriminants::Service,
                    inline_entities: vec![],
                }],
                metadata_filters: vec![
                    MetadataFilter {
                        filter_type: MetadataFilterType::Category,
                        entities: vec![EntityDiscriminants::Service],
                        label: "By category".to_string(),
                        values: filter_values_from_enum::<ServiceCategory>(),
                        applies: FilterApplication::Client,
                    },
                    // A containerized service's run state, which its card's status dot shows.
                    MetadataFilter {
                        filter_type: MetadataFilterType::GuestState,
                        entities: vec![EntityDiscriminants::Service],
                        label: "By guest state".to_string(),
                        values: filter_values_from_enum::<GuestState>(),
                        applies: FilterApplication::Client,
                    },
                ],
                // Populated by `element_config`, which is the only public constructor.
                default_hidden_values: HashMap::new(),
                collective_noun: None,
                element_marks: Vec::new(),
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
                affects_layout_within_container: false,
                default_visibility: visibility,
                stroke,
                highlight_behavior: highlight,
                will_target_container,
                show_directionality,
            }
        };
        // Places its endpoints next to each other inside the container they share, and moves no
        // container.
        let active_within_container =
            |visibility, stroke, highlight, will_target_container, show_directionality| {
                EdgeViewConfig::Active {
                    affects_layout: false,
                    affects_layout_within_container: true,
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
                // A dependency between two addresses in one subnet draws them side by side, but
                // the subnets stay arranged by addressing and runtime structure.
                RequestPath => active_within_container(Visible, Dashed, WhenVisible, false, true),
                HubAndSpoke => active_within_container(Visible, Dashed, WhenVisible, false, true),
                Hypervisor => active(false, Hidden, Dashed, WhenVisible, true, false),
                // Hypervisor's stroke and highlight, shown by default: without it an identity
                // host has no visible tie to the guest that presents it.
                NetworkIdentity => active(false, Visible, Dashed, WhenVisible, true, false),
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
                Hypervisor | NetworkIdentity | ContainerRuntime | RequestPath | HubAndSpoke
                | SameContainer => EdgeViewConfig::Disabled,
            },
            Self::Workloads => match edge_type {
                PhysicalLink => active(false, Hidden, Dashed, WhenVisible, false, false),
                NeighborLink => active(false, Hidden, Dashed, WhenVisible, false, false),
                RequestPath | HubAndSpoke => {
                    active(false, Hidden, Dashed, WhenVisible, false, true)
                }
                // Identities nest inside their guest's stack here; the nesting is the relationship.
                Hypervisor | NetworkIdentity | ContainerRuntime | SameHost | SameContainer => {
                    EdgeViewConfig::Disabled
                }
            },
            Self::Application => match edge_type {
                RequestPath => active(true, Visible, Solid, WhenVisible, false, true),
                HubAndSpoke => active(true, Visible, Solid, WhenVisible, false, true),
                ContainerRuntime => active(true, Hidden, Dashed, Always, true, false),
                SameHost | Hypervisor | NetworkIdentity | PhysicalLink | SameContainer
                | NeighborLink => EdgeViewConfig::Disabled,
            },
        }
    }

    /// Edge types this view hides until the user turns them on: those active here with
    /// `default_visibility: Hidden`. Read from [`Self::edge_view_config`], which also feeds the
    /// view fixture, so the frontend and backend share one definition.
    pub fn default_hidden_edge_types(&self) -> Vec<EdgeTypeDiscriminants> {
        EdgeTypeDiscriminants::iter()
            .filter(|&edge_type| {
                matches!(
                    self.edge_view_config(edge_type),
                    EdgeViewConfig::Active {
                        default_visibility: EdgeDefaultVisibility::Hidden,
                        ..
                    }
                )
            })
            .collect()
    }

    /// Inspector panel configuration for this view
    pub fn inspector_config(&self) -> ViewInspectorConfig {
        match self {
            Self::L3Logical => ViewInspectorConfig {
                element_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::HostDetail,
                    InspectorSection::Services,
                    InspectorSection::OtherInterfaces,
                ],
                container_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::ElementSummary,
                ],
                dependency_creation: Some(DependencyMemberType::Bindings),
                show_application_picker: false,
                show_service_network_detail: true,
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
                show_service_network_detail: false,
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
                show_service_network_detail: true,
            },
            Self::L2Physical => ViewInspectorConfig {
                element_sections: vec![InspectorSection::Identity],
                container_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::ElementSummary,
                ],
                dependency_creation: None,
                show_application_picker: false,
                show_service_network_detail: true,
            },
        }
    }

    /// Short token naming this view in export filenames (`scanopy-<slug>-<site>-<date>`).
    pub fn slug(&self) -> &'static str {
        match self {
            Self::L2Physical => "l2",
            Self::L3Logical => "l3",
            Self::Workloads => "infra",
            Self::Application => "app",
        }
    }

    /// How the UI lays out this view.
    ///
    /// L2 runs left to right so port-to-port links read across columns of switches and hosts,
    /// and keeps its column structure when fully collapsed. Workloads puts the busiest hosts
    /// first and also keeps its layout when collapsed. Application tiles its mostly unconnected
    /// groups, so root edges and the disconnected repack would only pull them apart.
    pub fn layout_config(&self) -> ViewLayoutConfig {
        use ViewElementOrder::*;
        use ViewHandleStrategy::*;
        use ViewRootOrder::*;
        match self {
            Self::L2Physical => ViewLayoutConfig {
                algorithm: ViewLayoutAlgorithm::LayeredRight,
                handle_strategy: Horizontal,
                element_order: OperStatus,
                root_order: Input,
                root_edges: true,
                repack_disconnected: false,
                align_layers_top: true,
                force_overview: false,
            },
            Self::L3Logical => ViewLayoutConfig {
                algorithm: ViewLayoutAlgorithm::Default,
                handle_strategy: Optimal,
                element_order: Automatic,
                root_order: Input,
                root_edges: true,
                repack_disconnected: true,
                align_layers_top: false,
                force_overview: true,
            },
            Self::Workloads => ViewLayoutConfig {
                algorithm: ViewLayoutAlgorithm::Workloads,
                handle_strategy: Optimal,
                element_order: Automatic,
                root_order: WorkloadCount,
                root_edges: true,
                repack_disconnected: false,
                align_layers_top: false,
                force_overview: false,
            },
            Self::Application => ViewLayoutConfig {
                algorithm: ViewLayoutAlgorithm::RectPacking,
                handle_strategy: Optimal,
                element_order: Automatic,
                root_order: Input,
                root_edges: false,
                repack_disconnected: false,
                align_layers_top: false,
                force_overview: true,
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

    /// Every section a view shows gets a heading that says what it holds, since the inspector
    /// draws the heading and description from this metadata rather than from each component.
    #[test]
    fn every_shown_inspector_section_has_a_heading_and_description() {
        for view in TopologyView::iter() {
            let config = view.inspector_config();
            for section in config
                .element_sections
                .iter()
                .chain(&config.container_sections)
            {
                assert!(!section.name().is_empty(), "{section:?} has no name");
                assert!(
                    !section.description().is_empty(),
                    "{section:?} has no description"
                );
            }
        }
    }
    use strum::IntoEnumIterator;

    /// A view can only paint a colour its filter panel explains: every element mark names a value
    /// of a filter the view declares for that entity.
    #[test]
    fn every_element_mark_names_a_declared_filter_value() {
        for view in TopologyView::iter() {
            let config = view.element_config();
            for mark in &config.element_marks {
                let declared = config
                    .filter(mark.entity, mark.filter_type)
                    .is_some_and(|f| f.values.iter().any(|v| v.id == mark.value));
                assert!(
                    declared,
                    "{view:?} paints {mark:?} with no filter value to decode it"
                );
            }
        }
    }

    /// The hide-set is keyed by entity and filter type, so a view may cover each pair with at most
    /// one filter, and every filter has to name what it covers.
    #[test]
    fn each_entity_and_filter_type_has_at_most_one_filter() {
        for view in TopologyView::iter() {
            let config = view.element_config();
            let mut seen = HashSet::new();
            for filter in &config.metadata_filters {
                assert!(
                    !filter.entities.is_empty(),
                    "{view:?} {:?} covers no entity",
                    filter.filter_type
                );
                for entity in &filter.entities {
                    assert!(
                        seen.insert((*entity, filter.filter_type)),
                        "{view:?} covers {entity:?}.{:?} twice",
                        filter.filter_type
                    );
                }
            }
        }
    }

    /// Workloads' Containerized value covers both kinds of container: a macvlan host and a
    /// service on a bridge network.
    #[test]
    fn workloads_containerized_covers_container_hosts_and_services() {
        use crate::server::hosts::r#impl::base::{Host, HostBase};
        use crate::server::hosts::r#impl::virtualization::{
            ContainerHostVirtualization, ContainerNetworkType, HostVirtualization,
        };
        use crate::server::services::r#impl::base::{Service, ServiceBase};
        use crate::server::services::r#impl::virtualization::{
            DockerVirtualization, ServiceVirtualization,
        };
        use crate::server::shared::storage::traits::Storable;

        let config = TopologyView::Workloads.element_config();
        let filter = config
            .filter(
                EntityDiscriminants::Service,
                MetadataFilterType::Virtualization,
            )
            .expect("Workloads filters services by virtualization");
        assert_eq!(
            config
                .filter(
                    EntityDiscriminants::Host,
                    MetadataFilterType::Virtualization
                )
                .map(|f| &f.entities),
            Some(&filter.entities),
            "hosts and services share one virtualization filter"
        );

        let ctx = FilterValueContext::default();
        let containerized = HostVirtualizationState::Containerized.id();
        let host = Host::new(HostBase {
            virtualization_metadata: Some(HostVirtualization::Docker(
                ContainerHostVirtualization {
                    state: Default::default(),
                    container_name: None,
                    container_id: None,
                    compose_project: None,
                    network_type: ContainerNetworkType::MacVlan,
                },
            )),
            ..Default::default()
        });
        let service = Service::new(ServiceBase {
            virtualization_metadata: Some(ServiceVirtualization::Docker(DockerVirtualization {
                state: Default::default(),
                container_name: None,
                container_id: None,
                compose_project: None,
            })),
            ..Default::default()
        });
        assert_eq!(
            host.filter_values(&ctx)[&MetadataFilterType::Virtualization],
            containerized
        );
        assert_eq!(
            service.filter_values(&ctx)[&MetadataFilterType::Virtualization],
            containerized
        );
        assert!(filter.values.iter().any(|v| v.id == containerized));
    }

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

    /// Export filenames are keyed by slug, so two views sharing one would overwrite each other's
    /// downloads.
    #[test]
    fn every_view_has_a_unique_slug() {
        let mut seen = HashSet::new();
        for view in TopologyView::iter() {
            let slug = view.slug();
            assert!(!slug.is_empty(), "{view:?} has no slug");
            assert!(seen.insert(slug), "{view:?} reuses slug {slug}");
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
