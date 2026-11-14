//! The one structural rule every view's graph keeps: each node sits inside a container the same
//! graph holds. The client's layout imports the graph whole, and one element naming a missing
//! container fails that import and leaves the map blank (GH #852).

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::server::topology::types::nodes::{Node, NodeType};

/// The container a node sits in, if any.
fn parent_of(node: &Node) -> Option<Uuid> {
    match &node.node_type {
        NodeType::Element { container_id, .. } => Some(*container_id),
        NodeType::Container {
            parent_container_id,
            ..
        } => *parent_container_id,
    }
}

/// Remove every node whose chain of containers leaves `nodes`, and return their ids.
///
/// A container whose parent is missing goes, and so does everything inside it. Edges to a removed
/// node are dropped later, when the graph is assembled.
pub fn drop_dangling_nodes(nodes: &mut Vec<Node>) -> Vec<Uuid> {
    let container_parent: HashMap<Uuid, Option<Uuid>> = nodes
        .iter()
        .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
        .map(|n| (n.id, parent_of(n)))
        .collect();

    // A container is rooted when its chain reaches a container with no parent. The visited set
    // stops a cycle, which no builder makes today, from looping.
    let mut rooted: HashMap<Uuid, bool> = HashMap::new();
    let mut is_rooted = |start: Uuid| -> bool {
        let mut chain = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(start);
        let answer = loop {
            let Some(id) = current else { break true };
            if let Some(&known) = rooted.get(&id) {
                break known;
            }
            if !seen.insert(id) {
                break false;
            }
            let Some(parent) = container_parent.get(&id) else {
                break false;
            };
            chain.push(id);
            current = *parent;
        };
        for id in chain {
            rooted.insert(id, answer);
        }
        answer
    };

    let mut dropped = Vec::new();
    nodes.retain(|node| {
        let keep = match parent_of(node) {
            None => true,
            Some(parent) => is_rooted(parent),
        };
        if !keep {
            dropped.push(node.id);
        }
        keep
    });
    dropped
}

#[cfg(test)]
mod tests;
