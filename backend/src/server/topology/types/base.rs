use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::shared::entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants};
use crate::server::topology::types::edges::{EdgeHandle, EdgeTypeDiscriminants};
use crate::server::topology::types::grouping::{
    ContainerRule, ElementRule, ElementSort, GraphRule, IdentifiedRule,
};
use crate::server::topology::types::layout::{Ixy, Uxy};
use crate::server::topology::types::views::{
    MetadataFilterType, TopologyView, TopologyViewSupport,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};
use strum::IntoEnumIterator;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema, Validate)]
pub struct Topology {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: TopologyBase,
}

impl Topology {
    /// Resolve the available views for a share, filtering by data availability.
    /// If `configured` is None or empty, all data-supported views are returned.
    /// If `configured` is Some(non-empty list), returns the intersection preserving list order.
    ///
    /// `support` must be computed from raw entity data (see
    /// `TopologyService::get_view_support`) — NOT from this topology's
    /// persisted graph, because the graph is rebuilt per-view and its
    /// edges/entity_tags reflect only the most recently rendered view.
    pub fn resolve_available_views(
        &self,
        configured: &Option<Vec<TopologyView>>,
        support: &TopologyViewSupport,
    ) -> Vec<TopologyView> {
        let data_supported: Vec<TopologyView> = TopologyView::iter()
            .filter(|v| v.is_supported(support))
            .collect();

        match configured {
            None => data_supported,
            Some(list) if list.is_empty() => data_supported,
            Some(list) => list
                .iter()
                .filter(|v| data_supported.contains(v))
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Default, ToSchema)]
pub struct TopologyBase {
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// Saved layout and view settings for this topology.
    pub options: TopologyOptions,
    // The per-view node/edge graph is no longer persisted — it's a pure
    // function of entities + `options` and is built on request by the read
    // path (`build_all_view_graphs`). The row holds only the user's grouping
    // `options`; snapshots build their graph on request from closed copies, so
    // there are no snapshot-pinned topology rows (and no `snapshot_id`).
}

impl TopologyBase {
    pub fn new(network_id: Uuid) -> Self {
        Self {
            network_id,
            options: TopologyOptions::default(),
        }
    }
}

impl ChangeTriggersTopologyStaleness<Topology> for Topology {
    fn triggers_staleness(&self, other: Option<Topology>) -> bool {
        if let Some(other_topology) = other {
            // Switching the active view is a client-side slice selection (all
            // views are pre-built on the row), not an options change — request
            // options no longer carry a view scalar. Grouping/hide-rule edits
            // still trip this.
            self.base.options.request != other_topology.base.options.request
        } else {
            false
        }
    }
}

impl Display for Topology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Topology {{ id: {} }}", self.id)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, ToSchema)]
pub struct TopologyOptions {
    // `#[serde(default)]` keeps deserialization lenient (missing -> default), but
    // these are always present in responses, so mark them required in the schema.
    /// Settings applied in the viewer, which do not change what the server returns.
    #[serde(default)]
    #[schema(required)]
    pub local: TopologyLocalOptions,
    /// Settings that change how the server builds the graph.
    #[serde(default)]
    #[schema(required)]
    pub request: TopologyRequestOptions,
}

/// Filter settings for hiding entities by tag in topology visualization.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default, ToSchema)]
pub struct TopologyTagFilter {
    /// Host tag IDs to hide (hosts with these tags will fade out)
    #[serde(default)]
    pub hidden_host_tag_ids: Vec<Uuid>,
    /// Service tag IDs to hide (services with these tags will be hidden from nodes)
    #[serde(default)]
    pub hidden_service_tag_ids: Vec<Uuid>,
    /// Subnet tag IDs to hide (subnets with these tags will fade out)
    #[serde(default)]
    pub hidden_subnet_tag_ids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, ToSchema)]
#[serde(default)]
pub struct TopologyLocalOptions {
    /// Keep unrelated edges at full opacity when something is selected.
    pub no_fade_edges: bool,
    /// Edge types to leave out of the drawing.
    pub hide_edge_types: Vec<EdgeTypeDiscriminants>,
    /// Restrict the view to entities carrying these tags.
    #[serde(default)]
    pub tag_filter: TopologyTagFilter,
    /// Show the minimap.
    #[serde(default = "default_true")]
    pub show_minimap: bool,
    /// Collapse parallel edges between the same pair of nodes into one.
    #[serde(default = "default_true")]
    pub bundle_edges: bool,
}

fn default_true() -> bool {
    true
}

impl Default for TopologyLocalOptions {
    fn default() -> Self {
        Self {
            no_fade_edges: false,
            hide_edge_types: vec![EdgeTypeDiscriminants::Hypervisor],
            tag_filter: TopologyTagFilter::default(),
            show_minimap: true,
            bundle_edges: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
#[serde(default)]
pub struct TopologyRequestOptions {
    /// Entity types hidden per view. Keyed by TopologyView, values are entity
    /// types (matching those declared as container/element/inline in the
    /// view's element_config). Hides every manifestation of the entity in
    /// that view — element nodes, container nodes, and inline rows on
    /// element cards. Supersedes the old `hide_ports` (L3-only, inline-only).
    #[serde(default)]
    pub hide_entities: HashMap<TopologyView, Vec<EntityDiscriminants>>,
    /// Generic per-(view, entity, filter) hide-set for metadata filters
    /// (Category, Virtualization, etc). Supersedes the old
    /// `hide_service_categories`; nested so JSON keys are strings all the
    /// way down.
    #[serde(default = "default_hide_metadata_values")]
    pub hide_metadata_values: HashMap<
        TopologyView,
        HashMap<EntityDiscriminants, HashMap<MetadataFilterType, Vec<String>>>,
    >,
    /// Rules deciding how nodes are grouped into containers.
    #[serde(default = "default_container_rules")]
    pub container_rules: HashMap<TopologyView, Vec<IdentifiedRule<ContainerRule>>>,
    /// Rules deciding how entities are placed and inlined within containers.
    #[serde(default = "default_element_rules")]
    pub element_rules: Vec<IdentifiedRule<ElementRule>>,
    /// How nodes are ordered inside containers, per view. A view with no entry uses the
    /// layout's own order.
    #[serde(default)]
    pub element_sort: HashMap<TopologyView, ElementSort>,
}

/// The hide-set a topology starts with when the user has expressed no preference.
///
/// Sourced from each view's own `default_hidden_values` so there is exactly one definition of
/// what counts as a product default — the frontend reads the same list out of the generated view
/// fixture.
fn default_hide_metadata_values()
-> HashMap<TopologyView, HashMap<EntityDiscriminants, HashMap<MetadataFilterType, Vec<String>>>> {
    TopologyView::iter()
        .map(|view| (view, view.element_config().default_hidden_values))
        .collect()
}

impl TopologyRequestOptions {
    /// Adopt any default hide-set entry the stored options have no opinion about.
    ///
    /// `#[serde(default = ...)]` is a whole-field fallback: options that already carry a
    /// `hide_metadata_values` map take it verbatim, so a newly added default reaches new installs
    /// only. Every existing user would keep a map written before the filter existed and never be
    /// offered it — which for the L2 link-state filter means keeping a view that renders every
    /// ifTable row, the condition this default exists to fix.
    ///
    /// Merging is per `(view, entity, filter)` key and only where the key is **absent**. A user
    /// who has toggled a filter has an opinion recorded under that key and it is left exactly as
    /// it stands, including an explicit empty list — that is "show everything", not "unset". Only
    /// a filter they have never been offered is filled in.
    pub fn merge_missing_hide_defaults(&mut self) {
        for (view, by_entity) in default_hide_metadata_values() {
            let stored_view = self.hide_metadata_values.entry(view).or_default();
            for (entity, by_filter) in by_entity {
                let stored_entity = stored_view.entry(entity).or_default();
                for (filter_type, values) in by_filter {
                    stored_entity.entry(filter_type).or_insert(values);
                }
            }
        }
    }
}

fn default_container_rules() -> HashMap<TopologyView, Vec<IdentifiedRule<ContainerRule>>> {
    use ContainerRule::*;

    // Build from applicable_views: for each rule type, add it to every view it applies to
    let all_rules: Vec<IdentifiedRule<ContainerRule>> = vec![
        IdentifiedRule::new(BySubnet),
        IdentifiedRule::new(MergeContainerBridges),
        IdentifiedRule::new(ByApplication { tag_ids: vec![] }),
        IdentifiedRule::new(ByHost),
    ];

    let mut map: HashMap<TopologyView, Vec<IdentifiedRule<ContainerRule>>> =
        TopologyView::iter().map(|p| (p, vec![])).collect();

    for gr in all_rules {
        for &view in gr.rule.applicable_views() {
            map.entry(view).or_default().push(gr.clone());
        }
    }

    map
}

fn default_element_rules() -> Vec<IdentifiedRule<ElementRule>> {
    vec![
        IdentifiedRule::new(ElementRule::ByTrunkPort),
        IdentifiedRule::new(ElementRule::ByVLAN),
        IdentifiedRule::new(ElementRule::ByPortOpStatus),
        IdentifiedRule::new(ElementRule::ByServiceCategory {
            categories: ServiceCategory::iter()
                .filter(|c| c.application_relevant_use_cases().is_empty())
                .collect(),
            title: Some("Infrastructure".into()),
            is_infra_rule: true,
        }),
        IdentifiedRule::new(ElementRule::ByTag {
            tag_ids: vec![],
            title: None,
        }),
        IdentifiedRule::new(ElementRule::ByHypervisor),
        IdentifiedRule::new(ElementRule::ByContainerRuntime),
        IdentifiedRule::new(ElementRule::ByStack),
    ]
}

impl Default for TopologyRequestOptions {
    fn default() -> Self {
        Self {
            hide_entities: HashMap::new(),
            hide_metadata_values: default_hide_metadata_values(),
            container_rules: default_container_rules(),
            element_rules: default_element_rules(),
            element_sort: HashMap::new(),
        }
    }
}

/// Lightweight request type for updating a single node's position.
///
/// Used for drag operations - instead of sending the entire topology (which can be
/// several megabytes for large networks), only sends the node ID and new position.
/// Fixes HTTP 413 errors on drag operations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyNodePositionUpdate {
    /// Network ID for authorization
    pub network_id: Uuid,
    /// View whose node/edge slice this update targets
    pub view: TopologyView,
    /// ID of the node to update
    pub node_id: Uuid,
    /// New position for the node
    pub position: Ixy,
}

/// Lightweight request type for updating an edge's handles.
///
/// Used for edge reconnect operations - instead of sending the entire topology,
/// only sends the edge ID and new handle positions.
/// Fixes HTTP 413 errors on edge reconnect operations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyEdgeHandleUpdate {
    /// Network ID for authorization
    pub network_id: Uuid,
    /// View whose node/edge slice this update targets
    pub view: TopologyView,
    /// ID of the edge to update
    pub edge_id: Uuid,
    /// New source handle position
    pub source_handle: EdgeHandle,
    /// New target handle position
    pub target_handle: EdgeHandle,
}

/// Lightweight request type for updating a node's size and position.
///
/// Used for subnet resize operations - instead of sending the entire topology,
/// only sends the node ID, new size, and new position.
/// Fixes HTTP 413 errors on resize operations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyNodeResizeUpdate {
    /// Network ID for authorization
    pub network_id: Uuid,
    /// View whose node/edge slice this update targets
    pub view: TopologyView,
    /// ID of the node to update
    pub node_id: Uuid,
    /// New size for the node
    pub size: Uxy,
    /// New position for the node
    pub position: Ixy,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stored map that predates a filter must adopt its default, or every existing topology
    /// keeps rendering as though the filter were switched off. This is the whole reason the merge
    /// exists: `#[serde(default)]` is a whole-field fallback and cannot fill a missing key.
    #[test]
    fn absent_filter_adopts_its_default() {
        let mut options = TopologyRequestOptions {
            hide_metadata_values: HashMap::from([(
                TopologyView::L2Physical,
                HashMap::from([(
                    EntityDiscriminants::Service,
                    HashMap::from([(MetadataFilterType::Category, vec!["OpenPorts".to_string()])]),
                )]),
            )]),
            ..Default::default()
        };

        options.merge_missing_hide_defaults();

        let interface = &options.hide_metadata_values[&TopologyView::L2Physical]
            [&EntityDiscriminants::Interface][&MetadataFilterType::LinkState];
        assert_eq!(interface, &vec!["Unlinked".to_string()]);
    }

    /// An empty list is an opinion — "show everything" — recorded when the user clears a filter.
    /// Treating it as unset would re-hide what they just chose to see, on the next page load.
    #[test]
    fn cleared_filter_is_not_refilled() {
        let mut options = TopologyRequestOptions {
            hide_metadata_values: HashMap::from([(
                TopologyView::L2Physical,
                HashMap::from([(
                    EntityDiscriminants::Interface,
                    HashMap::from([(MetadataFilterType::LinkState, vec![])]),
                )]),
            )]),
            ..Default::default()
        };

        options.merge_missing_hide_defaults();

        assert!(
            options.hide_metadata_values[&TopologyView::L2Physical]
                [&EntityDiscriminants::Interface][&MetadataFilterType::LinkState]
                .is_empty()
        );
    }

    /// Merging must not disturb a filter the user has actually set.
    #[test]
    fn user_selection_survives_the_merge() {
        let mut options = TopologyRequestOptions {
            hide_metadata_values: HashMap::from([(
                TopologyView::L2Physical,
                HashMap::from([(
                    EntityDiscriminants::Interface,
                    HashMap::from([(MetadataFilterType::LinkState, vec!["Linked".to_string()])]),
                )]),
            )]),
            ..Default::default()
        };

        options.merge_missing_hide_defaults();

        assert_eq!(
            options.hide_metadata_values[&TopologyView::L2Physical]
                [&EntityDiscriminants::Interface][&MetadataFilterType::LinkState],
            vec!["Linked".to_string()]
        );
    }

    /// Every value a view hides by default has to be one the view actually offers, or the chip
    /// that would turn it back on is never rendered and the hide becomes permanent and invisible.
    #[test]
    fn every_default_hidden_value_is_offered_by_its_view() {
        for view in TopologyView::iter() {
            let config = view.element_config();
            for (entity, by_filter) in &config.default_hidden_values {
                for (filter_type, values) in by_filter {
                    let declared = config
                        .metadata_filters
                        .get(entity)
                        .and_then(|filters| filters.iter().find(|f| f.filter_type == *filter_type));

                    // Service.Category is declared in every view but only *rendered* where
                    // Service has an element or inline role, so a view without that role is
                    // allowed to hide a value it does not offer.
                    let Some(filter) = declared else { continue };

                    for value in values {
                        assert!(
                            filter.values.iter().any(|v| v.id == *value),
                            "{view:?} hides {entity:?}.{filter_type:?}={value} but does not offer it"
                        );
                    }
                }
            }
        }
    }
}
