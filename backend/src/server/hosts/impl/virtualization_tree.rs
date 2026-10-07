//! Where a host sits in the virtualization tree: which host runs it, which host tops its chain,
//! and how deep it is.
//!
//! A host's parent is the host running the service that virtualizes it
//! (`hosts.virtualization_service_id` → `services.host_id`). Chains nest: a Proxmox node runs a
//! VM, which runs a Docker engine, which runs a macvlan container with an address of its own.
//!
//! Two implementations of one rule, like the name ladder: [`tree_position`] for the response a
//! row carries, and [`VIRTUALIZATION_TREE_JOIN`] for ordering and grouping a paginated host list,
//! which only the database can do across pages it hasn't sent. The rule both follow:
//!
//! - walk up through live services and live hosts, at most [`DEPTH_CAP`] steps, never revisiting
//!   a host (a cycle stops the walk where it would repeat);
//! - the root is the last host the walk reached;
//! - a host with no parent is a root only when some other live host runs under one of its live
//!   services. A host that neither runs under nor runs anything is in no tree at all.

use std::sync::LazyLock;

use uuid::Uuid;

use crate::server::hosts::r#impl::name_ladder::display_name_sql;

/// The most ancestors a walk follows. Real chains are three deep; the cap is what keeps a cycle
/// the data should never hold from walking forever, in both implementations.
pub const DEPTH_CAP: usize = 8;

/// A host's place in the virtualization tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreePosition {
    /// The host this one runs under, if any.
    pub parent_host_id: Option<Uuid>,
    /// The host at the top of this host's chain. Itself when it is a root.
    pub root_host_id: Uuid,
    /// How many ancestors sit above it. A root is 0.
    pub depth: u32,
}

/// Where `host_id` sits, or `None` when it is in no tree.
///
/// `parent_of` answers the host a host runs under, already restricted to live services and live
/// hosts and never the host itself. `has_children` says whether another live host runs under one
/// of `host_id`'s live services.
pub fn tree_position(
    host_id: Uuid,
    parent_of: impl Fn(Uuid) -> Option<Uuid>,
    has_children: bool,
) -> Option<TreePosition> {
    let mut chain = vec![host_id];
    while chain.len() <= DEPTH_CAP {
        let current = *chain.last().expect("the chain starts with the host");
        match parent_of(current) {
            Some(parent) if !chain.contains(&parent) => chain.push(parent),
            _ => break,
        }
    }

    let depth = chain.len() - 1;
    if depth == 0 && !has_children {
        return None;
    }

    Some(TreePosition {
        parent_host_id: chain.get(1).copied(),
        root_host_id: *chain.last().expect("the chain starts with the host"),
        depth: depth as u32,
    })
}

/// The lateral join that places each `hosts` row in its tree, as `virt_tree.root_id` (NULL when
/// the host is in no tree) and `virt_tree.path`.
///
/// `path` is one key per host from the root down to this row, each the host's lowercased title
/// then its id. Ordering by it lists a tree parent first with every subtree directly under its
/// root, siblings by title: an array that is a prefix of another sorts before it.
///
/// The chain is built by prepending, so `ids[1]` of the deepest step is the root.
pub static VIRTUALIZATION_TREE_JOIN: LazyLock<String> = LazyLock::new(|| {
    let title = display_name_sql("ancestor", "ancestor_ip");
    format!(
        "LEFT JOIN LATERAL (\
            WITH RECURSIVE chain(host_id, depth, ids) AS (\
                SELECT hosts.id, 0, ARRAY[hosts.id] \
                UNION ALL \
                SELECT parent.id, chain.depth + 1, parent.id || chain.ids \
                FROM chain \
                JOIN hosts child ON child.id = chain.host_id \
                JOIN services link ON link.id = child.virtualization_service_id \
                    AND link.valid_to IS NULL \
                JOIN hosts parent ON parent.id = link.host_id AND parent.valid_to IS NULL \
                WHERE chain.depth < {cap} AND NOT parent.id = ANY(chain.ids)\
            ), top AS (\
                SELECT depth, ids FROM chain ORDER BY depth DESC LIMIT 1\
            ) \
            SELECT \
                CASE WHEN top.depth > 0 OR EXISTS (\
                    SELECT 1 FROM services own \
                    JOIN hosts guest ON guest.virtualization_service_id = own.id \
                        AND guest.valid_to IS NULL \
                    WHERE own.host_id = hosts.id AND own.valid_to IS NULL \
                        AND guest.id <> hosts.id\
                ) THEN top.ids[1] END AS root_id, \
                ARRAY(\
                    SELECT lower({title}) || ' ' || ancestor.id::text \
                    FROM unnest(top.ids) WITH ORDINALITY AS step(id, ord) \
                    JOIN hosts ancestor ON ancestor.id = step.id \
                    LEFT JOIN LATERAL (\
                        SELECT ip_address FROM ip_addresses \
                        WHERE ip_addresses.host_id = ancestor.id AND ip_addresses.valid_to IS NULL \
                        ORDER BY last_seen_at DESC, position ASC LIMIT 1\
                    ) AS ancestor_ip ON TRUE \
                    ORDER BY step.ord\
                ) AS path \
            FROM top\
        ) AS virt_tree ON TRUE",
        cap = DEPTH_CAP,
    )
});

/// The group key [`VIRTUALIZATION_TREE_JOIN`] yields: the root's id, `''` for a host in no tree.
pub const VIRTUALIZATION_TREE_GROUP_SQL: &str = "COALESCE(virt_tree.root_id::text, '')";

/// The order inside a group: parent first, subtrees under their parents, siblings by title.
pub const VIRTUALIZATION_TREE_ORDER_SQL: &str = "virt_tree.path";

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn ids<const N: usize>() -> [Uuid; N] {
        std::array::from_fn(|_| Uuid::new_v4())
    }

    fn parents(edges: &[(Uuid, Uuid)]) -> impl Fn(Uuid) -> Option<Uuid> + '_ {
        let map: HashMap<Uuid, Uuid> = edges.iter().copied().collect();
        move |id| map.get(&id).copied()
    }

    #[test]
    fn a_three_level_chain_roots_every_host_at_the_top() {
        let [node, vm, container] = ids();
        let edges = [(vm, node), (container, vm)];

        assert_eq!(
            tree_position(container, parents(&edges), false),
            Some(TreePosition {
                parent_host_id: Some(vm),
                root_host_id: node,
                depth: 2
            })
        );
        assert_eq!(
            tree_position(vm, parents(&edges), true),
            Some(TreePosition {
                parent_host_id: Some(node),
                root_host_id: node,
                depth: 1
            })
        );
    }

    #[test]
    fn a_host_running_others_with_no_parent_is_its_own_root() {
        let [node] = ids();
        assert_eq!(
            tree_position(node, |_| None, true),
            Some(TreePosition {
                parent_host_id: None,
                root_host_id: node,
                depth: 0
            })
        );
    }

    #[test]
    fn a_host_that_neither_runs_under_nor_runs_anything_is_in_no_tree() {
        let [lone] = ids();
        assert_eq!(tree_position(lone, |_| None, false), None);
    }

    #[test]
    fn a_cycle_stops_where_it_would_repeat() {
        let [a, b] = ids();
        let edges = [(a, b), (b, a)];

        let position = tree_position(a, parents(&edges), true).expect("a has a parent");
        assert_eq!(position.root_host_id, b);
        assert_eq!(position.depth, 1);
    }

    #[test]
    fn a_chain_longer_than_the_cap_stops_at_the_cap() {
        let chain: Vec<Uuid> = (0..DEPTH_CAP + 4).map(|_| Uuid::new_v4()).collect();
        let edges: Vec<(Uuid, Uuid)> = chain.windows(2).map(|w| (w[0], w[1])).collect();

        let position = tree_position(chain[0], parents(&edges), false).expect("has parents");
        assert_eq!(position.depth as usize, DEPTH_CAP);
        assert_eq!(position.root_host_id, chain[DEPTH_CAP]);
    }
}
