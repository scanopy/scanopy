//! Apply the user's metadata hide-set to the entity bundle, before any view is built.
//!
//! # Why this runs on entities rather than nodes
//!
//! The response is an entity bundle (`TopologyData`: hosts, interfaces, services, …) *plus* a
//! per-view slice of `Node`s. A `Node` is only an id, a type, a position and a header — the data
//! lives in the bundle. Dropping nodes after a view is built would shrink the wrong thing.
//!
//! Filtering the entities first means the builders never see them, so nodes, edges and bundle all
//! shrink together and no builder needs to know this exists. Edges come for free:
//! `EdgeBuilder::add_edges_to_graph` already drops any edge whose endpoints are absent from the
//! node index.
//!
//! # Why an entity must be hidden in *every* view to be dropped
//!
//! One bundle serves all four views, and the hide-set is per view. An interface hidden in L2 but
//! rendered in some other view has to survive, or that view silently loses nodes it never filtered.
//! Interfaces happen to appear only in L2, so today the quantifier never bites — which is exactly
//! why it needs to be written down rather than discovered later.
//!
//! # Scope
//!
//! Only filters declared `FilterApplication::Server` are applied here; the rest stay in the browser
//! where toggling them costs nothing. See `FilterApplication` for the rule governing which may be
//! which.

use std::collections::{BTreeMap, HashMap, HashSet};

use uuid::Uuid;

use crate::server::interface_neighbors::r#impl::base::InterfaceNeighborRow;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::topology::types::base::TopologyOptions;
use crate::server::topology::types::views::{
    FilterApplication, FilterValueContext, HasFilterValues, MetadataFilterType, TopologyView,
};
use strum::IntoEnumIterator;

/// Values hidden for one entity type, keyed by filter, unioned across every view that hides them.
///
/// `views_hiding` counts how many views hide a given value so the "every view" test can be applied
/// without re-walking the view configs per entity.
#[derive(Debug, Default)]
pub struct ServerHideSet {
    /// filter → value → number of views in which that value is hidden by a `Server` filter.
    by_filter: HashMap<MetadataFilterType, HashMap<String, usize>>,
    /// Views that declare *any* `Server` filter for this entity type — the denominator.
    rendering_views: usize,
}

impl ServerHideSet {
    /// Which filter drops an entity carrying these filter values, if any.
    ///
    /// `Some` only when some value it holds is hidden in every view that could render it. Naming
    /// the filter rather than answering yes/no is what lets the caller tally drops per filter, so
    /// the response can say *which* control emptied a view rather than only that something did.
    fn hidden_by(
        &self,
        values: &std::collections::BTreeMap<MetadataFilterType, String>,
    ) -> Option<MetadataFilterType> {
        if self.rendering_views == 0 {
            return None;
        }
        values.iter().find_map(|(filter, value)| {
            self.by_filter
                .get(filter)
                .and_then(|vals| vals.get(value))
                .is_some_and(|hiding| *hiding >= self.rendering_views)
                .then_some(*filter)
        })
    }
}

/// Build the per-entity-type hide-set from the view configs and the user's stored hide-set.
///
/// Returns an empty map when nothing is hidden server-side, which lets the caller skip the work
/// entirely rather than walking every entity to discover there was nothing to do.
pub fn server_hide_sets(options: &TopologyOptions) -> HashMap<EntityDiscriminants, ServerHideSet> {
    let mut out: HashMap<EntityDiscriminants, ServerHideSet> = HashMap::new();

    for view in TopologyView::iter() {
        let config = view.element_config();
        for (entity, filters) in &config.metadata_filters {
            let server_filters: Vec<_> = filters
                .iter()
                .filter(|f| f.applies == FilterApplication::Server)
                .collect();
            if server_filters.is_empty() {
                continue;
            }

            let entry = out.entry(*entity).or_default();
            entry.rendering_views += 1;

            let hidden_here = options
                .request
                .hide_metadata_values
                .get(&view)
                .and_then(|by_entity| by_entity.get(entity));

            for filter in server_filters {
                let Some(values) =
                    hidden_here.and_then(|by_filter| by_filter.get(&filter.filter_type))
                else {
                    continue;
                };
                let counts = entry.by_filter.entry(filter.filter_type).or_default();
                for value in values {
                    *counts.entry(value.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    out.retain(|_, set| !set.by_filter.is_empty());
    out
}

/// Interfaces named as another interface's neighbour.
///
/// Built once and handed to every `filter_values` call. See `FilterValueContext` for why judging a
/// port's own resolved rows alone is wrong. GH #701: reads the merged neighbour read-model
/// (`InterfaceNeighborRow`) instead of `Interface.base.neighbor` — a port's adjacencies are a `Vec`
/// now, so this is a plain map over every resolved row rather than a filter over interfaces.
pub fn referenced_neighbour_interfaces<'a>(
    neighbours: impl Iterator<Item = &'a InterfaceNeighborRow>,
) -> HashSet<Uuid> {
    neighbours
        .filter_map(|row| row.neighbor.interface_id())
        .collect()
}

/// Interfaces with at least one live row of their own in either resolved-neighbour table — the
/// successor to reading `Interface.neighbor.is_some()` directly.
pub fn interfaces_with_neighbours<'a>(
    neighbours: impl Iterator<Item = &'a InterfaceNeighborRow>,
) -> HashSet<Uuid> {
    neighbours.map(|row| row.interface_id).collect()
}

/// Drop entities of one type that every rendering view hides, tallied by the filter responsible.
///
/// Generic over the entity so this file names no entity type; the caller supplies the vector and
/// the id accessor. The tally is what the response carries back: an entity dropped here never
/// reaches the browser, so nothing downstream can count it or say what removed it.
pub fn retain_visible<T: HasFilterValues>(
    entities: &mut Vec<T>,
    hide_set: Option<&ServerHideSet>,
    ctx: &FilterValueContext,
) -> BTreeMap<MetadataFilterType, usize> {
    let mut dropped = BTreeMap::new();
    let Some(hide_set) = hide_set else {
        return dropped;
    };
    entities.retain(
        |entity| match hide_set.hidden_by(&entity.filter_values(ctx)) {
            Some(filter) => {
                *dropped.entry(filter).or_insert(0) += 1;
                false
            }
            None => true,
        },
    );
    dropped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::interface_neighbors::r#impl::base::Neighbor;
    use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
    use crate::server::topology::types::base::{TopologyOptions, TopologyRequestOptions};

    fn options_hiding(view: TopologyView, values: &[&str]) -> TopologyOptions {
        let mut request = TopologyRequestOptions::default();
        request.hide_metadata_values = HashMap::from([(
            view,
            HashMap::from([(
                EntityDiscriminants::Interface,
                HashMap::from([(
                    MetadataFilterType::LinkState,
                    values.iter().map(|v| v.to_string()).collect(),
                )]),
            )]),
        )]);
        TopologyOptions {
            request,
            ..Default::default()
        }
    }

    fn interface(id: Uuid) -> Interface {
        let mut iface = Interface::new(InterfaceBase::default());
        iface.id = id;
        iface
    }

    fn neighbor_row(interface_id: Uuid, neighbor: Neighbor) -> InterfaceNeighborRow {
        InterfaceNeighborRow {
            id: Uuid::new_v4(),
            interface_id,
            neighbor,
            neighbor_seen_at: None,
        }
    }

    fn ctx(neighbours: &[InterfaceNeighborRow]) -> FilterValueContext {
        FilterValueContext {
            interfaces_referenced_as_neighbours: referenced_neighbour_interfaces(neighbours.iter()),
            interfaces_with_neighbours: interfaces_with_neighbours(neighbours.iter()),
        }
    }

    fn total(dropped: &BTreeMap<MetadataFilterType, usize>) -> usize {
        dropped.values().sum()
    }

    /// The behaviour the whole feature turns on: a port nothing points at, and which points at
    /// nothing, is the one that gets dropped.
    #[test]
    fn drops_only_ports_with_no_link_in_either_direction() {
        let hide_sets = server_hide_sets(&options_hiding(TopologyView::L2Physical, &["Unlinked"]));

        let linked_out = Uuid::new_v4();
        let linked_in = Uuid::new_v4();
        let unlinked = Uuid::new_v4();

        let mut interfaces = vec![
            interface(linked_out),
            interface(linked_in),
            interface(unlinked),
        ];
        let neighbours = vec![neighbor_row(linked_out, Neighbor::Interface(linked_in))];
        let ctx = ctx(&neighbours);

        let dropped = retain_visible(
            &mut interfaces,
            hide_sets.get(&EntityDiscriminants::Interface),
            &ctx,
        );

        // Attributed to the filter that did it, not just counted: this is what lets an emptied
        // view name the control responsible instead of reporting the network as empty.
        assert_eq!(
            dropped,
            BTreeMap::from([(MetadataFilterType::LinkState, 1)])
        );
        let kept: Vec<_> = interfaces.iter().map(|i| i.id).collect();
        assert!(kept.contains(&linked_out));
        // Named as a neighbour but reports none of its own — dropping this is the bug the
        // frontend shipped, which drew 11 edges instead of 720.
        assert!(kept.contains(&linked_in));
        assert!(!kept.contains(&unlinked));
    }

    /// One bundle serves every view, so hiding a value in one view must not remove it from the
    /// others. Today interfaces render only in L2 so this never bites in production — which is
    /// exactly why it needs a test rather than a comment.
    #[test]
    fn keeps_entities_a_view_does_not_hide() {
        // Hidden in a view that declares no server-side filter for Interface at all.
        let hide_sets = server_hide_sets(&options_hiding(TopologyView::Workloads, &["Unlinked"]));

        let mut interfaces = vec![interface(Uuid::new_v4())];
        let dropped = retain_visible(
            &mut interfaces,
            hide_sets.get(&EntityDiscriminants::Interface),
            &FilterValueContext::default(),
        );

        assert_eq!(
            total(&dropped),
            0,
            "a view that does not hide the value must keep it"
        );
    }

    /// The defaults ship `Interface/LinkState = [Unlinked]`, so this is live on every install from
    /// the moment it lands — no setting to discover. Users see no change, because the frontend
    /// already hid those ports; what changes is that they stop being sent at all.
    ///
    /// Asserted rather than assumed: it is the difference between a fix a customer has to opt into
    /// and one that reaches them on upgrade.
    #[test]
    fn product_defaults_hide_unlinked_ports() {
        let hide_sets = server_hide_sets(&TopologyOptions::default());
        let mut interfaces = vec![interface(Uuid::new_v4())];
        let dropped = retain_visible(
            &mut interfaces,
            hide_sets.get(&EntityDiscriminants::Interface),
            &FilterValueContext::default(),
        );
        assert_eq!(total(&dropped), 1);
    }

    /// Clearing the hide-set has to bring them back, which is the only way a user can inspect a
    /// port the default hides.
    #[test]
    fn an_empty_hide_set_keeps_everything() {
        let hide_sets = server_hide_sets(&options_hiding(TopologyView::L2Physical, &[]));
        let mut interfaces = vec![interface(Uuid::new_v4())];
        let dropped = retain_visible(
            &mut interfaces,
            hide_sets.get(&EntityDiscriminants::Interface),
            &FilterValueContext::default(),
        );
        assert_eq!(total(&dropped), 0);
        assert_eq!(interfaces.len(), 1);
    }
}
