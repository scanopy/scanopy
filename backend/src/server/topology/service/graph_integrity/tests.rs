use std::collections::HashSet;

use strum::IntoEnumIterator;
use uuid::Uuid;

use super::drop_dangling_nodes;
use crate::server::{
    bindings::r#impl::base::Binding,
    hosts::r#impl::{
        base::{Host, HostBase},
        name::{HostName, HostNameSources},
    },
    interface_neighbors::r#impl::base::{
        InterfaceNeighborCandidate, InterfaceNeighborCandidateBase, InterfaceNeighborEvidence,
        InterfaceNeighborRow, Neighbor,
    },
    interfaces::r#impl::base::{FdbMacs, FdbMacsValue, IfOperStatus, Interface, InterfaceBase},
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue},
    services::r#impl::base::{Service, ServiceBase},
    shared::{attribution::AttributeSource, entities::EntityDiscriminants},
    subnets::r#impl::base::Subnet,
    topology::{
        service::{
            context::TopologyContext,
            main::{TagHidden, TopologyService},
            view::builder_for_view,
        },
        types::{
            api::{TopologyData, TopologyHost},
            base::TopologyOptions,
            grouping::GroupingConfig,
            nodes::{ContainerType, ElementEntityType, Node, NodeType},
            views::TopologyView,
        },
    },
};

const X_MAC: &str = "f8:bc:12:dc:14:01";

fn host(name: &str) -> Host {
    Host {
        id: Uuid::new_v4(),
        base: HostBase {
            name: HostName::manual(name.to_string()),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn make_subnet() -> Subnet {
    Subnet {
        id: Uuid::new_v4(),
        ..Default::default()
    }
}

fn port(host_id: Uuid, if_index: i32) -> Interface {
    Interface::new(InterfaceBase {
        host_id,
        if_index: Some(if_index),
        if_name: Some(format!("Gi0/{if_index}")),
        if_type: Some(6),
        // Up, so the default ByPortOpStatus rule nests ports in a subcontainer under the host.
        oper_status: Some(IfOperStatus::Up),
        ..Default::default()
    })
}

fn address(host_id: Uuid, subnet_id: Uuid) -> IPAddress {
    IPAddress::new(IPAddressBase {
        host_id,
        subnet_id,
        ..Default::default()
    })
}

fn service(host_id: Uuid, ip_address_id: Uuid) -> (Service, Binding) {
    let mut service = Service {
        id: Uuid::new_v4(),
        base: ServiceBase {
            host_id,
            name: "web".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    let binding = Binding::new_ip_address(service.id, Uuid::nil(), ip_address_id);
    service.base.bindings = vec![binding.clone()];
    (service, binding)
}

fn neighbour(interface_id: Uuid, neighbor: Neighbor) -> InterfaceNeighborRow {
    InterfaceNeighborRow {
        id: Uuid::new_v4(),
        interface_id,
        neighbor,
        neighbor_seen_at: None,
    }
}

fn candidate(interface_id: Uuid) -> InterfaceNeighborCandidate {
    InterfaceNeighborCandidate::new(InterfaceNeighborCandidateBase::new(
        Uuid::nil(),
        interface_id,
        InterfaceNeighborEvidence::default(),
    ))
}

#[derive(Clone, Copy, Debug)]
enum Removal {
    Dismissed,
    VisibilityHidden,
}

/// A switch and a server, and host X that has left the bundle: a live port, a port-level and a
/// device-level neighbour, a switch port that learned its MAC, an address and a service. A
/// Visibility-hidden subnet holds a second address of the server.
struct Fixture {
    data: TopologyData,
    tag_hidden: TagHidden,
    x: Uuid,
    /// X and every row of it, none of which any view may draw.
    x_rows: HashSet<Uuid>,
    hidden_subnet: Uuid,
}

fn fixture(removal: Removal) -> Fixture {
    let switch = host("switch-01");
    let server = host("server-01");
    let x = host("x-01");
    let subnet = make_subnet();
    let hidden_subnet = make_subnet();

    let switch_to_server = port(switch.id, 1);
    let switch_to_x = port(switch.id, 2);
    let mut switch_learning_x = port(switch.id, 3);
    switch_learning_x.base.fdb_macs = Some(FdbMacs::new(
        FdbMacsValue(vec![X_MAC.to_string()]),
        AttributeSource::ForwardingTable,
    ));
    let server_port = port(server.id, 1);
    let mut x_port = port(x.id, 1);
    x_port.base.mac_address = Some(MacEvidence::new(
        MacEvidenceValue(X_MAC.parse().unwrap()),
        AttributeSource::ArpReply,
    ));

    let server_address = address(server.id, subnet.id);
    let server_hidden_address = address(server.id, hidden_subnet.id);
    let x_address = address(x.id, subnet.id);
    let (server_service, server_binding) = service(server.id, server_address.id);
    let (x_service, x_binding) = service(x.id, x_address.id);

    let neighbours = vec![
        neighbour(server_port.id, Neighbor::Interface(switch_to_server.id)),
        neighbour(switch_to_x.id, Neighbor::Interface(x_port.id)),
        neighbour(x_port.id, Neighbor::Interface(switch_to_x.id)),
        neighbour(x_port.id, Neighbor::Host(switch.id)),
    ];
    let candidates = vec![candidate(server_port.id), candidate(x_port.id)];

    let ip_addresses = vec![server_address, server_hidden_address, x_address.clone()];
    let services = vec![server_service, x_service.clone()];
    let mut tag_hidden = TagHidden {
        subnets: vec![hidden_subnet.clone()],
        ..Default::default()
    };
    // What `get_topology_data` does: a dismissed host is dropped, a hidden one set aside.
    if let Removal::VisibilityHidden = removal {
        tag_hidden.hosts.push(x.clone());
    }
    let mut data = TopologyData {
        hosts: TopologyHost::wrap_all(vec![switch, server], &ip_addresses, &services),
        ip_addresses,
        subnets: vec![subnet],
        bindings: vec![server_binding, x_binding.clone()],
        interfaces: [
            switch_to_server,
            switch_to_x,
            switch_learning_x,
            server_port,
            x_port.clone(),
        ]
        .into_iter()
        .map(Into::into)
        .collect(),
        neighbours,
        candidates,
        services: services.into_iter().map(Into::into).collect(),
        ..Default::default()
    };
    tag_hidden.orphans = data.remove_orphans();

    Fixture {
        data,
        tag_hidden,
        x: x.id,
        x_rows: HashSet::from([x.id, x_port.id, x_address.id, x_service.id, x_binding.id()]),
        hidden_subnet: hidden_subnet.id,
    }
}

/// Every reference in one view's graph that doesn't resolve in the same response.
fn dangling_references(data: &TopologyData, view: TopologyView) -> Vec<String> {
    let nodes = &data.nodes[&view];
    let node_ids: HashSet<Uuid> = nodes.iter().map(|n| n.id).collect();
    let container_ids: HashSet<Uuid> = nodes
        .iter()
        .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
        .map(|n| n.id)
        .collect();
    let host_ids: HashSet<Uuid> = data.hosts.iter().map(|h| h.host.id).collect();
    let subnet_ids: HashSet<Uuid> = data.subnets.iter().map(|s| s.id).collect();

    let mut dangling = Vec::new();
    for node in nodes {
        match &node.node_type {
            NodeType::Element {
                container_id,
                host_id,
                ..
            } => {
                if !container_ids.contains(container_id) {
                    dangling.push(format!(
                        "element {} in missing container {container_id}",
                        node.id
                    ));
                }
                if !host_ids.contains(host_id) {
                    dangling.push(format!("element {} of missing host {host_id}", node.id));
                }
            }
            NodeType::Container {
                container_type,
                parent_container_id,
                entity_id,
                ..
            } => {
                if let Some(parent) = parent_container_id
                    && !container_ids.contains(parent)
                {
                    dangling.push(format!(
                        "container {} in missing container {parent}",
                        node.id
                    ));
                }
                let missing_entity = match (container_type, entity_id) {
                    (ContainerType::Host, Some(id)) => !host_ids.contains(id),
                    (ContainerType::Subnet, Some(id)) => !subnet_ids.contains(id),
                    _ => false,
                };
                if missing_entity {
                    dangling.push(format!(
                        "{container_type:?} container {} for missing entity {entity_id:?}",
                        node.id
                    ));
                }
            }
        }
    }
    for edge in &data.edges[&view] {
        for end in [edge.source, edge.target] {
            if !node_ids.contains(&end) {
                dangling.push(format!("edge {} to missing node {end}", edge.id));
            }
        }
    }
    dangling
}

/// The ids a node stands for.
fn node_refs(node: &Node) -> Vec<Uuid> {
    match &node.node_type {
        NodeType::Element {
            host_id,
            entity_id,
            inline_groups,
            ..
        } => std::iter::once(*host_id)
            .chain(*entity_id)
            .chain(inline_groups.iter().map(|g| g.entity_id))
            .collect(),
        NodeType::Container { entity_id, .. } => entity_id.iter().copied().collect(),
    }
}

fn render(fixture: Fixture) -> (TopologyData, Uuid, HashSet<Uuid>, Uuid) {
    let views: Vec<TopologyView> = TopologyView::iter().collect();
    let Fixture {
        data,
        tag_hidden,
        x,
        x_rows,
        hidden_subnet,
    } = fixture;
    let rendered =
        TopologyService::render_loaded(data, tag_hidden, &TopologyOptions::default(), &views);
    (rendered, x, x_rows, hidden_subnet)
}

/// GH #852: a host that left the bundle, dismissed or hidden, leaves no reference behind in any
/// view, and nothing of it reaches the response.
#[test]
fn every_view_resolves_every_reference_when_a_linked_host_has_left_the_bundle() {
    for removal in [Removal::Dismissed, Removal::VisibilityHidden] {
        let (rendered, _, x_rows, hidden_subnet) = render(fixture(removal));

        for view in TopologyView::iter() {
            let dangling = dangling_references(&rendered, view);
            assert!(dangling.is_empty(), "{removal:?}, {view:?}: {dangling:#?}");

            for node in &rendered.nodes[&view] {
                let refs = node_refs(node);
                assert!(
                    !refs
                        .iter()
                        .any(|id| x_rows.contains(id) || *id == hidden_subnet),
                    "{removal:?}, {view:?}: node {node:?} draws a row that left the bundle"
                );
            }
        }

        let shipped: HashSet<Uuid> = rendered
            .interfaces
            .iter()
            .map(|i| i.interface.id)
            .chain(rendered.ip_addresses.iter().map(|ip| ip.id))
            .chain(rendered.services.iter().map(|s| s.service.id))
            .chain(rendered.bindings.iter().map(|b| b.id()))
            .collect();
        assert!(
            shipped.is_disjoint(&x_rows),
            "{removal:?}: the bundle still ships rows of the host that left it"
        );
    }
}

/// The switch side of the link to the removed host still draws, as it does for a neighbour on a
/// host the bundle never held; only the edge to the removed port goes.
#[test]
fn the_near_side_of_a_link_to_a_removed_host_still_draws() {
    let (rendered, x, _, _) = render(fixture(Removal::Dismissed));
    let l2 = &rendered.nodes[&TopologyView::L2Physical];
    let drawn_hosts: HashSet<Uuid> = l2
        .iter()
        .filter_map(|n| match n.node_type {
            NodeType::Container {
                container_type: ContainerType::Host,
                entity_id,
                ..
            } => entity_id,
            _ => None,
        })
        .collect();
    let present: HashSet<Uuid> = rendered.hosts.iter().map(|h| h.host.id).collect();
    assert_eq!(drawn_hosts, present);
    assert!(!drawn_hosts.contains(&x));
}

/// A hidden host counts once where it would have drawn; a dismissed one counts nowhere, since
/// nobody can show it again with the Visibility filter.
#[test]
fn only_a_hidden_host_counts_toward_the_visibility_filter() {
    let (hidden, ..) = render(fixture(Removal::VisibilityHidden));
    let l2 = &hidden.hidden_by_tag[&TopologyView::L2Physical];
    assert_eq!(l2.get(&EntityDiscriminants::Host), Some(&1));
    let l3 = &hidden.hidden_by_tag[&TopologyView::L3Logical];
    assert_eq!(l3.get(&EntityDiscriminants::Subnet), Some(&1));

    let (dismissed, ..) = render(fixture(Removal::Dismissed));
    for (view, counts) in &dismissed.hidden_by_tag {
        assert_eq!(
            counts.get(&EntityDiscriminants::Host),
            None,
            "{view:?} counts a dismissed host as hidden"
        );
    }
}

/// The L2 builder on its own, handed interfaces and neighbours of a host it wasn't handed, draws
/// none of that host's ports.
#[test]
fn l2_draws_no_port_of_a_host_missing_from_its_context() {
    let switch = host("switch-01");
    let x = host("x-01");
    let switch_port = port(switch.id, 1);
    let x_port = port(x.id, 1);
    let neighbours = vec![
        neighbour(switch_port.id, Neighbor::Interface(x_port.id)),
        neighbour(x_port.id, Neighbor::Interface(switch_port.id)),
    ];
    let hosts = vec![switch];
    let interfaces = vec![switch_port, x_port.clone()];
    let options = TopologyOptions::default();
    let ctx = TopologyContext::new(
        &hosts,
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &interfaces,
        &[],
        &[],
        &options,
        TopologyView::L2Physical,
    )
    .with_neighbours(&neighbours);
    let grouping = GroupingConfig::from_request_options(&options.request, TopologyView::L2Physical);

    let (mut nodes, _) = builder_for_view(TopologyView::L2Physical).build(&ctx, &grouping);

    assert!(!nodes.iter().any(|n| n.id == x_port.id));
    assert_eq!(drop_dangling_nodes(&mut nodes), Vec::<Uuid>::new());
}

fn container(id: Uuid, parent: Option<Uuid>) -> Node {
    Node {
        id,
        node_type: NodeType::Container {
            container_type: ContainerType::Host,
            parent_container_id: parent,
            entity_id: None,
            icon: None,
            color: None,
            associated_service_definition: None,
            element_rule_id: None,
            will_accept_edges: false,
        },
        position: Default::default(),
        size: Default::default(),
        header: None,
    }
}

fn element(container_id: Uuid) -> Node {
    Node::element(
        Uuid::new_v4(),
        container_id,
        Uuid::new_v4(),
        ElementEntityType::Service {},
    )
}

/// A container whose chain leaves the graph goes with everything inside it; the rest stays.
#[test]
fn the_guard_drops_what_sits_in_a_missing_container_and_keeps_the_rest() {
    let root = Uuid::new_v4();
    let sub = Uuid::new_v4();
    let orphan_sub = Uuid::new_v4();
    let in_sub = element(sub);
    let in_orphan_sub = element(orphan_sub);
    let in_missing = element(Uuid::new_v4());
    let mut nodes = vec![
        container(root, None),
        container(sub, Some(root)),
        container(orphan_sub, Some(Uuid::new_v4())),
        in_sub.clone(),
        in_orphan_sub.clone(),
        in_missing.clone(),
    ];

    let dropped: HashSet<Uuid> = drop_dangling_nodes(&mut nodes).into_iter().collect();

    assert_eq!(
        dropped,
        HashSet::from([orphan_sub, in_orphan_sub.id, in_missing.id])
    );
    let kept: HashSet<Uuid> = nodes.iter().map(|n| n.id).collect();
    assert_eq!(kept, HashSet::from([root, sub, in_sub.id]));
}

/// The cascade reaches the end of each chain in one pass, and leaves optional links alone.
#[test]
fn removing_a_parent_removes_its_children_and_theirs() {
    let kept_host = host("kept");
    let gone_host = host("gone");
    let subnet = make_subnet();
    let gone_subnet_id = Uuid::new_v4();

    let gone_port = port(gone_host.id, 1);
    let kept_port = port(kept_host.id, 1);
    let address_on_gone_subnet = address(kept_host.id, gone_subnet_id);
    let kept_address = address(kept_host.id, subnet.id);
    // Optional link to an address that leaves: the interface stays.
    let mut kept_port_on_gone_address = port(kept_host.id, 2);
    kept_port_on_gone_address.base.ip_address_id = Some(address_on_gone_subnet.id);
    let (kept_service, binding_to_gone_address) = service(kept_host.id, address_on_gone_subnet.id);
    let (gone_service, binding_of_gone_service) = service(gone_host.id, kept_address.id);
    let gone_row = neighbour(gone_port.id, Neighbor::Interface(kept_port.id));
    // The far end is a reference: the row stays and still marks the kept port linked.
    let row_to_gone_port = neighbour(kept_port.id, Neighbor::Interface(gone_port.id));
    let gone_candidate = candidate(gone_port.id);

    let ip_addresses = vec![address_on_gone_subnet.clone(), kept_address.clone()];
    let services = vec![kept_service.clone(), gone_service.clone()];
    let mut data = TopologyData {
        hosts: TopologyHost::wrap_all(vec![kept_host], &ip_addresses, &services),
        ip_addresses,
        subnets: vec![subnet],
        bindings: vec![
            binding_to_gone_address.clone(),
            binding_of_gone_service.clone(),
        ],
        interfaces: [
            gone_port.clone(),
            kept_port.clone(),
            kept_port_on_gone_address.clone(),
        ]
        .into_iter()
        .map(Into::into)
        .collect(),
        neighbours: vec![gone_row.clone(), row_to_gone_port.clone()],
        candidates: vec![gone_candidate.clone()],
        services: services.into_iter().map(Into::into).collect(),
        ..Default::default()
    };

    let removed = data.remove_orphans();

    let ids = |v: Vec<Uuid>| v.into_iter().collect::<HashSet<_>>();
    assert_eq!(
        ids(removed.ip_addresses.iter().map(|ip| ip.id).collect()),
        HashSet::from([address_on_gone_subnet.id])
    );
    assert_eq!(
        ids(removed.interfaces.iter().map(|i| i.interface.id).collect()),
        HashSet::from([gone_port.id])
    );
    assert_eq!(
        ids(removed.services.iter().map(|s| s.service.id).collect()),
        HashSet::from([gone_service.id])
    );
    assert_eq!(
        ids(removed.bindings.iter().map(|b| b.id()).collect()),
        HashSet::from([binding_to_gone_address.id(), binding_of_gone_service.id()])
    );
    assert_eq!(
        ids(removed.neighbours.iter().map(|r| r.id).collect()),
        HashSet::from([gone_row.id])
    );
    assert_eq!(
        ids(removed.candidates.iter().map(|c| c.id).collect()),
        HashSet::from([gone_candidate.id])
    );
    assert_eq!(
        ids(data.interfaces.iter().map(|i| i.interface.id).collect()),
        HashSet::from([kept_port.id, kept_port_on_gone_address.id])
    );
    assert_eq!(
        ids(data.neighbours.iter().map(|r| r.id).collect()),
        HashSet::from([row_to_gone_port.id])
    );
    assert_eq!(
        ids(data.services.iter().map(|s| s.service.id).collect()),
        HashSet::from([kept_service.id])
    );
}
