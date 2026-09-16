use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{
    context::TopologyContext,
    edge_builder::EdgeBuilder,
    element_rules::{
        ElementMatchData, TaggableLookups, apply_element_rules, resolve_element_tag_ids,
    },
    view::ViewBuilder,
};
use crate::server::{
    interfaces::r#impl::base::{InterfaceLinkState, Neighbor, if_type::EXCLUDED_IF_TYPES},
    shared::entities::EntityDiscriminants,
    topology::types::{
        edges::{DiscoveryProtocol, Edge, EdgeHandle, EdgeType, EdgeViewConfig},
        grouping::GroupingConfig,
        nodes::{ContainerType, ElementEntityType, Node, NodeType},
    },
};

pub struct L2Builder;

impl L2Builder {
    /// Generate a deterministic container UUID from host_id for the L2 view.
    fn container_id_for_host(host_id: Uuid) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("l2:{host_id}").as_bytes())
    }
}

impl ViewBuilder for L2Builder {
    fn build(&self, ctx: &TopologyContext, grouping: &GroupingConfig) -> (Vec<Node>, Vec<Edge>) {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // 1. Build PhysicalLink edges using interface_id as source/target
        //    (unlike create_physical_link_edges which uses ip_address_id)
        let mut processed_pairs: HashSet<(Uuid, Uuid)> = HashSet::new();

        for row in ctx.get_interfaces_with_neighbor() {
            let source_id = row.interface_id;
            let target_interface_id = match row.neighbor {
                Neighbor::Interface(id) => id,
                Neighbor::Host(_) => continue,
            };

            // Dedup bidirectional pairs
            let pair_key = if source_id < target_interface_id {
                (source_id, target_interface_id)
            } else {
                (target_interface_id, source_id)
            };
            if !processed_pairs.insert(pair_key) {
                continue;
            }

            let Some(source_entry) = ctx.get_interface_by_id(source_id) else {
                continue;
            };
            let target_entry = match ctx.get_interface_by_id(target_interface_id) {
                Some(e) => e,
                None => continue,
            };

            // Skip self-loops
            if source_entry.base.host_id == target_entry.base.host_id {
                continue;
            }

            let label = Some(format!(
                "{} ↔ {}",
                source_entry.display_name(),
                target_entry.display_name()
            ));

            edges.push(Edge {
                id: Uuid::new_v4(),
                source: source_entry.id, // interface_id, not ip_address_id
                target: target_entry.id, // interface_id, not ip_address_id
                edge_type: EdgeType::PhysicalLink {
                    source_entity_id: source_entry.id,
                    target_entity_id: target_entry.id,
                    protocol: DiscoveryProtocol::default(),
                },
                label,
                source_handle: EdgeHandle::Bottom,
                target_handle: EdgeHandle::Top,
                is_multi_hop: false,
                view_config: EdgeViewConfig::default(),
                relation_key: None,
            });
        }

        // 1b. Device-level adjacencies: neighbours that resolved to a host but not to a port.
        //     Anchored on the Host containers, since there is no port to attach them to.
        let linked_host_pairs = EdgeBuilder::physical_link_host_pairs(ctx, &edges);

        let neighbor_link_edges = EdgeBuilder::create_neighbor_link_edges(
            ctx,
            Self::container_id_for_host,
            &linked_host_pairs,
        );

        // 2. Determine qualifying hosts:
        //    - Hosts with any Interface that has LLDP/CDP neighbor data
        //    - Hosts that are targets of physical links
        //    - Both ends of a device-level adjacency, so the neighbour we could only identify
        //      at host level still gets a container to draw the edge to (GH #649)
        let mut qualifying_host_ids: HashSet<Uuid> = HashSet::new();

        // Hosts with neighbor data
        for row in ctx.get_interfaces_with_neighbor() {
            if let Some(entry) = ctx.get_interface_by_id(row.interface_id) {
                qualifying_host_ids.insert(entry.base.host_id);
            }
        }

        for edge in &neighbor_link_edges {
            if let EdgeType::NeighborLink {
                source_host_id,
                target_host_id,
                ..
            } = &edge.edge_type
            {
                qualifying_host_ids.insert(*source_host_id);
                qualifying_host_ids.insert(*target_host_id);
            }
        }

        edges.extend(neighbor_link_edges);

        // Hosts that are targets (look up target interface → host_id)
        for edge in &edges {
            if let EdgeType::PhysicalLink {
                target_entity_id, ..
            } = &edge.edge_type
                && let Some(entry) = ctx.get_interface_by_id(*target_entity_id)
            {
                qualifying_host_ids.insert(entry.base.host_id);
            }
        }

        // Hosts with no IP address at all, identified only by an interface carrying MAC
        // evidence — a PROFINET DCP identify is the case this exists for. Neither of the two
        // conditions above ever fires for such a host: it has no neighbour data (DCP is a
        // broadcast identify, not a directed LLDP/CDP exchange) and is never the target of a
        // resolved link, so without this it appears in **no view at all** — L3 is structurally
        // blind to it too, since `subnet_graph_builder` iterates `ip_addresses`.
        //
        // Gated on "no IP" specifically so this never changes behaviour for any host that
        // already has one: those are already visible via L3, and already require neighbour data
        // to additionally qualify for L2 — that rule is untouched. Drawn as a standalone
        // container with no edges, the same as any other qualifying host whose interfaces happen
        // to carry no resolved neighbour.
        for host in ctx.hosts {
            if ctx.get_ip_addresses_for_host(host.id).is_empty()
                && ctx
                    .get_interfaces_for_host(host.id)
                    .iter()
                    .any(|entry| entry.base.mac_address.is_some())
            {
                qualifying_host_ids.insert(host.id);
            }
        }

        // 3. Create Host containers for qualifying hosts
        let host_lookup: HashMap<Uuid, &crate::server::hosts::r#impl::base::Host> =
            ctx.hosts.iter().map(|h| (h.id, h)).collect();

        for &host_id in &qualifying_host_ids {
            let Some(host) = host_lookup.get(&host_id) else {
                continue;
            };

            let container_id = Self::container_id_for_host(host_id);
            nodes.push(Node {
                id: container_id,
                node_type: NodeType::Container {
                    container_type: ContainerType::Host,
                    parent_container_id: None,
                    entity_id: Some(host_id),
                    icon: None,
                    color: None,
                    associated_service_definition: None,
                    element_rule_id: None,
                    will_accept_edges: false,
                },
                position: Default::default(),
                size: Default::default(),
                header: ctx.host_container_header(host),
            });
        }

        // 4. Create Port elements for qualifying hosts' IfEntries
        for &host_id in &qualifying_host_ids {
            let container_id = Self::container_id_for_host(host_id);
            for entry in ctx.get_interfaces_for_host(host_id) {
                // Skip virtual/software interface types — unless this one has a resolved
                // neighbour, which is evidence it is a real port whatever its declared type says.
                // Two reasons. A daemon host's own NICs are recorded `propVirtual` because `pnet`
                // cannot tell a physical NIC from a virtual one, so without this they could never
                // be drawn even once a switch names them. And an edge is built from
                // `interfaces.neighbor` with no type check at all (see the loop above), so a
                // neighbour that resolved onto a switch's VLAN or bridge interface used to produce
                // an edge pointing at a node that was never created.
                // An unread type is not an excluded one. A port learned from a neighbour's
                // advertisement has no ifType, and dropping it for that would remove exactly the
                // far ends this view exists to draw.
                // Linked in *either* direction, not just outbound: a link is recorded on one side,
                // so the far end of nearly every link reports no neighbour of its own. Judging the
                // outbound direction here dropped exactly the ports the metadata filter had kept,
                // and `EdgeBuilder` then dropped the link that named them.
                if entry
                    .base
                    .if_type
                    .is_some_and(|if_type| EXCLUDED_IF_TYPES.contains(&if_type))
                    && ctx.interface_link_state(entry.id) == InterfaceLinkState::Unlinked
                {
                    continue;
                }

                let mut node = Node::element(
                    entry.id,
                    container_id,
                    host_id,
                    ElementEntityType::Interface {
                        interface_id: entry.id,
                    },
                );
                node.header = Some(entry.display_name().to_string());
                nodes.push(node);
            }
        }

        // 5. Apply element rules (ByTag already has L2Physical in applicable_views)
        let if_entry_lookup: std::collections::HashMap<
            Uuid,
            &crate::server::interfaces::r#impl::base::Interface,
        > = ctx.interfaces.iter().map(|e| (e.id, e)).collect();
        let tag_lookups = TaggableLookups {
            hosts: Some(&host_lookup),
            services: None,
            subnets: None,
        };
        let _ = apply_element_rules(
            &mut nodes,
            &grouping.element_rules,
            |node| {
                if let NodeType::Element { host_id, .. } = &node.node_type {
                    let tag_ids = resolve_element_tag_ids(
                        EntityDiscriminants::Interface,
                        *host_id,
                        &tag_lookups,
                    );
                    let interface = if_entry_lookup.get(&node.id);
                    let native_vlan_id = interface.and_then(|e| e.base.native_vlan_id);
                    let resolved_vlan = native_vlan_id.and_then(|vid| ctx.get_vlan_by_id(vid));
                    Some(ElementMatchData {
                        categories: HashSet::new(),
                        tag_ids,
                        element_entity: EntityDiscriminants::Interface,
                        virtualizer_service_id: None,
                        deployment_group: None,
                        native_vlan_id,
                        vlan_number: resolved_vlan.map(|v| v.base.vlan_number),
                        vlan_name: resolved_vlan.map(|v| v.base.name.clone()),
                        is_trunk_port: interface
                            .and_then(|e| e.base.vlan_ids.as_ref())
                            .is_some_and(|v| !v.is_empty()),
                        oper_status: interface.and_then(|e| e.base.oper_status),
                    })
                } else {
                    None
                }
            },
            None,
            None,
        );

        (nodes, edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::attributes::HostChassisIdValue;
    use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
    use crate::server::interface_neighbors::r#impl::base::{
        InterfaceNeighborCandidate, InterfaceNeighborCandidateBase, InterfaceNeighborEvidence,
        InterfaceNeighborRow,
    };
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::shared::attribution::{AttributeSource, Attributed};
    use crate::server::{
        hosts::r#impl::base::{Host, HostBase},
        interfaces::r#impl::base::{Interface, InterfaceBase, Neighbor, if_type},
        ip_addresses::r#impl::base::{IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue},
        lldp::LldpChassisId,
        topology::{
            service::context::TopologyContext,
            types::{
                base::TopologyOptions,
                grouping::GroupingConfig,
                nodes::{ContainerType, NodeType},
            },
        },
    };
    use chrono::Utc;

    fn make_host(name: &str) -> Host {
        Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: HostName::manual(name.to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_if_entry(host_id: Uuid, if_index: i32, if_type: i32) -> Interface {
        Interface {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: InterfaceBase {
                host_id,
                if_index: Some(if_index),
                if_descr: Some(format!("GigabitEthernet0/{if_index}")),
                if_name: Some(format!("Gi0/{if_index}")),
                if_type: Some(if_type),
                speed_bps: Some(1_000_000_000),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// A resolved-neighbour row for `interface_id`, matching the shape `TopologyContext.neighbours`
    /// now carries instead of `Interface.base.neighbor`.
    fn neighbor_row(interface_id: Uuid, neighbor: Neighbor) -> InterfaceNeighborRow {
        InterfaceNeighborRow {
            id: Uuid::new_v4(),
            interface_id,
            neighbor,
            neighbor_seen_at: None,
        }
    }

    /// A candidate carrying LLDP evidence for `interface_id`, for tests asserting the protocol a
    /// `NeighborLink` edge is labelled with (`TopologyContext::interface_has_lldp_evidence`).
    fn lldp_candidate(interface_id: Uuid) -> InterfaceNeighborCandidate {
        InterfaceNeighborCandidate::new(InterfaceNeighborCandidateBase::new(
            Uuid::nil(),
            interface_id,
            InterfaceNeighborEvidence {
                lldp_chassis_id: Some(LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".to_string())),
                ..Default::default()
            },
        ))
    }

    fn l2_grouping() -> GroupingConfig {
        GroupingConfig {
            container_rules: vec![],
            element_rules: vec![],
        }
    }

    #[test]
    fn test_empty_topology() {
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );
        let builder = L2Builder;
        let (nodes, edges) = builder.build(&ctx, &l2_grouping());
        assert!(nodes.is_empty());
        assert!(edges.is_empty());
    }

    #[test]
    fn test_hosts_without_neighbors_excluded() {
        let h1 = make_host("server-1");
        let ie1 = make_if_entry(h1.id, 1, 6);
        let hosts = vec![h1];
        let interfaces = vec![ie1];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let builder = L2Builder;
        let (nodes, edges) = builder.build(&ctx, &l2_grouping());
        // No LLDP neighbors → no qualifying hosts → empty
        assert!(nodes.is_empty());
        assert!(edges.is_empty());
    }

    /// A host with no IP address at all and no neighbour data — a PROFINET DCP identify, the
    /// case §7 of the DCP plan exists for — still qualifies for L2, drawn as a standalone
    /// container with no edges. The interface carries only a MAC, no neighbour: the "hosts with
    /// neighbor data" condition above never fires for it, and it is never a link target, so
    /// without this it would join `test_hosts_without_neighbors_excluded`'s host in appearing in
    /// no view at all — except that host also has no MAC, which is the one thing distinguishing
    /// the two cases.
    #[test]
    fn a_no_ip_host_identified_only_by_a_mac_still_gets_a_standalone_container() {
        let h1 = make_host("press-line-3");
        let mut ie1 = make_if_entry(h1.id, 1, 6);
        ie1.base.if_type = None; // DCP reports no ifType — unread, not a claim
        ie1.base.mac_address = Some(MacEvidence::new(
            MacEvidenceValue("00:ad:24:af:4e:00".parse().unwrap()),
            AttributeSource::ProfinetDcp,
        ));
        let hosts = vec![h1.clone()];
        let interfaces = vec![ie1];
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &hosts,
            &[], // no ip_addresses — the condition this test exists for
            &[],
            &[],
            &[],
            &[],
            &[],
            &interfaces,
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let builder = L2Builder;
        let (nodes, edges) = builder.build(&ctx, &l2_grouping());

        assert!(edges.is_empty(), "no neighbour, so no edge to draw");
        let container = nodes
            .iter()
            .find(|n| matches!(n.node_type, NodeType::Container { entity_id: Some(id), .. } if id == h1.id))
            .expect("the no-IP, MAC-identified host must still get a container");
        assert_eq!(container.header.as_deref(), Some("press-line-3"));
        let port = nodes
            .iter()
            .find(|n| matches!(n.node_type, NodeType::Element { host_id, .. } if host_id == h1.id));
        assert!(port.is_some(), "its interface must still render as a port");
    }

    /// A host container says which device it is even when the host carries no name.
    ///
    /// `HostName::unnamed()` formats as the empty string, so the header used to be `Some("")`. That
    /// is not absence: every consumer reads it with `??`, so the container drew a blank title and
    /// no fallback ever fired. Unnamed hosts are a live state — controller-imported devices with
    /// no assigned name sit at exactly that rung — and L2 is where they cluster.
    #[test]
    fn an_unnamed_host_container_falls_back_to_the_evidence_it_has() {
        let mut by_chassis = make_host("");
        by_chassis.base.name = HostName::unnamed();
        by_chassis.base.chassis_id = Some(Attributed::new(
            HostChassisIdValue("00:11:22:33:44:55".to_string()),
            AttributeSource::Probe(ClientProbe::Snmp),
        ));

        let mut by_address = make_host("");
        by_address.base.name = HostName::unnamed();

        let mut anonymous = make_host("");
        anonymous.base.name = HostName::unnamed();

        // A neighbour on each of the other two qualifies all three: the far end is a link target.
        let anchor = make_if_entry(by_chassis.id, 1, if_type::ETHERNET_CSMA_CD);
        let from_address = make_if_entry(by_address.id, 1, if_type::ETHERNET_CSMA_CD);
        let from_anonymous = make_if_entry(anonymous.id, 2, if_type::ETHERNET_CSMA_CD);
        let neighbours = vec![
            neighbor_row(from_address.id, Neighbor::Interface(anchor.id)),
            neighbor_row(from_anonymous.id, Neighbor::Interface(anchor.id)),
        ];

        let address = IPAddress::new(IPAddressBase {
            network_id: Uuid::nil(),
            host_id: by_address.id,
            subnet_id: Uuid::new_v4(),
            ip_address: "10.0.0.7".parse().unwrap(),
            mac_address: None,
            name: None,
            position: 0,
        });

        let by_chassis_id = by_chassis.id;
        let by_address_id = by_address.id;
        let anonymous_id = anonymous.id;
        let hosts = vec![by_chassis, by_address, anonymous];
        let interfaces = vec![anchor, from_address, from_anonymous];
        let ip_addresses = vec![address];
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &[],
            &[],
            &[],
            &[],
            &[],
            &interfaces,
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L2Physical,
        )
        .with_neighbours(&neighbours);

        let (nodes, _) = L2Builder.build(&ctx, &l2_grouping());
        let header_for = |host_id: Uuid| -> Option<String> {
            nodes
                .iter()
                .find(|n| {
                    matches!(
                        n.node_type,
                        NodeType::Container {
                            container_type: ContainerType::Host,
                            entity_id: Some(id),
                            ..
                        } if id == host_id
                    )
                })
                .expect("every qualifying host gets a container")
                .header
                .clone()
        };

        assert_eq!(
            header_for(by_chassis_id).as_deref(),
            Some("00:11:22:33:44:55"),
            "a device known only through LLDP still has a chassis id to be called by"
        );
        assert_eq!(
            header_for(by_address_id).as_deref(),
            Some("10.0.0.7"),
            "an address is a poor name but it is an identity"
        );
        assert_eq!(
            header_for(anonymous_id),
            None,
            "with nothing to say the header must be absent, not an empty string a `??` reads as present"
        );
    }

    /// A virtual-typed interface is drawn when — and only when — it has a resolved neighbour.
    ///
    /// Both halves matter. A daemon host's own NICs are recorded `propVirtual` because `pnet`
    /// cannot tell a physical NIC from a virtual one, so without the neighbour exemption they
    /// could never appear even once a switch names them. And an edge is built from
    /// `interfaces.neighbor` with no type check, so before this a neighbour resolving onto a
    /// bridge or VLAN interface produced an edge whose endpoint node was never created.
    #[test]
    fn a_virtual_interface_is_drawn_only_once_it_has_a_neighbour() {
        let h1 = make_host("switch-1");
        let h2 = make_host("daemon-host");

        // The far end is an ordinary port; the near end is virtual-typed, as a daemon host's is.
        let physical = make_if_entry(h1.id, 1, if_type::ETHERNET_CSMA_CD);
        let virtual_linked = make_if_entry(h2.id, 1, if_type::PROP_VIRTUAL);
        let virtual_bare = make_if_entry(h2.id, 2, if_type::PROP_VIRTUAL);
        let neighbours = vec![neighbor_row(
            virtual_linked.id,
            Neighbor::Interface(physical.id),
        )];

        let hosts = vec![h1, h2];
        let interfaces = vec![physical, virtual_linked.clone(), virtual_bare.clone()];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let (nodes, edges) = L2Builder.build(&ctx, &l2_grouping());
        let drawn: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();

        assert!(
            drawn.contains(&virtual_linked.id),
            "a virtual interface with a resolved neighbour must be drawn, or its edge dangles"
        );
        assert!(
            !drawn.contains(&virtual_bare.id),
            "a virtual interface with no neighbour is still noise and must stay hidden"
        );
        // Every edge endpoint must be a node that exists.
        for edge in &edges {
            assert!(drawn.contains(&edge.source), "edge source has no node");
            assert!(drawn.contains(&edge.target), "edge target has no node");
        }
    }

    /// The far end of a link is virtual-typed and reports no neighbour of its own.
    ///
    /// A link is recorded on one side, so the remote port of most links has no row of its own —
    /// judging the virtual-type exemption on the outbound direction alone therefore drew no node
    /// for it, and `EdgeBuilder` then dropped the very link that named it. The metadata filter had
    /// already classified that port `Linked` and kept it in the bundle, so the graph and the
    /// filter disagreed about the same port.
    #[test]
    fn a_virtual_interface_named_only_as_a_neighbour_is_drawn() {
        let h1 = make_host("switch-1");
        let h2 = make_host("westermo");

        let physical = make_if_entry(h1.id, 1, if_type::ETHERNET_CSMA_CD);
        // Named by the switch, reports nothing itself, and carries an excluded ifType.
        let virtual_far_end = make_if_entry(h2.id, 1, if_type::PROP_VIRTUAL);
        let neighbours = vec![neighbor_row(
            physical.id,
            Neighbor::Interface(virtual_far_end.id),
        )];

        let hosts = vec![h1, h2];
        let interfaces = vec![physical.clone(), virtual_far_end.clone()];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let (nodes, edges) = L2Builder.build(&ctx, &l2_grouping());
        let drawn: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();

        assert!(
            drawn.contains(&virtual_far_end.id),
            "a port named as another port's neighbour is linked and must be drawn"
        );
        assert!(
            edges
                .iter()
                .any(|e| e.source == physical.id && e.target == virtual_far_end.id),
            "the link that named it must survive"
        );
    }

    /// The default hide-set in front of the graph builder — the pairing the two halves are each
    /// tested for separately and neither covers.
    ///
    /// `retain_visible` runs over the bundle before any view is built, so `L2Builder` sees only
    /// the ports that survived it. If the two disagree about which ports are linked, the view
    /// draws host containers with nothing inside them: the hosts qualify off the neighbour rows,
    /// which are never filtered, while every port they own has already been dropped.
    #[test]
    fn hiding_unlinked_ports_still_draws_the_linked_ones() {
        let h1 = make_host("switch-1");
        let h2 = make_host("switch-2");

        let linked_out = make_if_entry(h1.id, 1, if_type::ETHERNET_CSMA_CD);
        let unlinked = make_if_entry(h1.id, 2, if_type::ETHERNET_CSMA_CD);
        // The far end: named by `linked_out`, reports no neighbour of its own.
        let linked_in = make_if_entry(h2.id, 1, if_type::ETHERNET_CSMA_CD);

        let neighbours = vec![neighbor_row(
            linked_out.id,
            Neighbor::Interface(linked_in.id),
        )];

        let mut interfaces = vec![linked_out.clone(), unlinked.clone(), linked_in.clone()];
        let options = TopologyOptions::default();

        // Exactly what `apply_server_metadata_filters` does to the bundle first.
        let ctx_values = crate::server::topology::types::views::FilterValueContext {
            interfaces_referenced_as_neighbours:
                crate::server::topology::service::metadata_filter::referenced_neighbour_interfaces(
                    neighbours.iter(),
                ),
            interfaces_with_neighbours:
                crate::server::topology::service::metadata_filter::interfaces_with_neighbours(
                    neighbours.iter(),
                ),
        };
        let hide_sets =
            crate::server::topology::service::metadata_filter::server_hide_sets(&options);
        crate::server::topology::service::metadata_filter::retain_visible(
            &mut interfaces,
            hide_sets.get(&EntityDiscriminants::Interface),
            &ctx_values,
        );

        assert_eq!(
            interfaces.len(),
            2,
            "only the unlinked port should have been dropped"
        );

        let hosts = vec![h1.clone(), h2.clone()];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let (nodes, _edges) = L2Builder.build(&ctx, &l2_grouping());
        let drawn: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();

        assert!(
            drawn.contains(&linked_out.id),
            "a linked port must survive both the filter and the builder"
        );
        assert!(
            drawn.contains(&linked_in.id),
            "so must the far end it names"
        );
        assert!(!drawn.contains(&unlinked.id));
    }

    #[test]
    fn test_physical_link_creates_containers_and_edges() {
        let h1 = make_host("switch-1");
        let h2 = make_host("switch-2");

        let ie1 = make_if_entry(h1.id, 1, 6);
        let ie2 = make_if_entry(h2.id, 1, 6);
        let neighbours = vec![neighbor_row(ie1.id, Neighbor::Interface(ie2.id))];

        let hosts = vec![h1, h2];
        let interfaces = vec![ie1, ie2];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let builder = L2Builder;
        let (nodes, edges) = builder.build(&ctx, &l2_grouping());

        // 2 Host containers + 2 Port elements
        let containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::Host,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(containers.len(), 2);

        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 2);

        // 1 PhysicalLink edge
        assert_eq!(edges.len(), 1);
        assert!(matches!(edges[0].edge_type, EdgeType::PhysicalLink { .. }));
    }

    /// An LLDP neighbour that identifies the remote device but not the remote port used to
    /// contribute nothing: no edge, and — because host qualification ran off resolved links
    /// only — no container either, so the neighbour was absent from L2 Physical entirely
    /// (GH #649). It now draws a device-level adjacency between the two Host containers.
    #[test]
    fn host_only_neighbor_draws_a_device_level_edge_between_both_hosts() {
        let h1 = make_host("switch-aruba-01");
        let h2 = make_host("switch-netgear-01");

        let ie1 = make_if_entry(h1.id, 1, 6);
        let neighbours = vec![neighbor_row(ie1.id, Neighbor::Host(h2.id))];
        let candidates = vec![lldp_candidate(ie1.id)];

        let hosts = vec![h1.clone(), h2.clone()];
        let interfaces = vec![ie1];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours)
        .with_candidates(&candidates);

        let builder = L2Builder;
        let (nodes, edges) = builder.build(&ctx, &l2_grouping());

        assert_eq!(edges.len(), 1);
        match edges[0].edge_type {
            EdgeType::NeighborLink {
                source_host_id,
                target_host_id,
                protocol,
            } => {
                assert_eq!(source_host_id, h1.id);
                assert_eq!(target_host_id, h2.id);
                assert_eq!(protocol, DiscoveryProtocol::LLDP);
            }
            ref other => panic!("expected a NeighborLink, got {other:?}"),
        }

        // Both devices get a container, including the one we could only name.
        let container_entity_ids: HashSet<Uuid> = nodes
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
        assert!(container_entity_ids.contains(&h1.id));
        assert!(container_entity_ids.contains(&h2.id));

        // The edge lands on those containers, not on ports.
        assert!(nodes.iter().any(|n| n.id == edges[0].source));
        assert!(nodes.iter().any(|n| n.id == edges[0].target));
    }

    /// The precise link supersedes the approximate one: a second neighbour entry between the
    /// same two devices that stopped at the host must not draw a vague edge on top of a link
    /// we already resolved port to port.
    #[test]
    fn a_host_pair_already_joined_by_a_physical_link_draws_no_neighbor_link() {
        let h1 = make_host("switch-1");
        let h2 = make_host("switch-2");

        let ie2 = make_if_entry(h2.id, 1, 6);
        let ie1 = make_if_entry(h1.id, 1, 6);
        // A second port on the same switch pair, resolved only as far as the device.
        let ie3 = make_if_entry(h1.id, 2, 6);
        let neighbours = vec![
            neighbor_row(ie1.id, Neighbor::Interface(ie2.id)),
            neighbor_row(ie3.id, Neighbor::Host(h2.id)),
        ];

        let hosts = vec![h1, h2];
        let interfaces = vec![ie1, ie2, ie3];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let builder = L2Builder;
        let (_nodes, edges) = builder.build(&ctx, &l2_grouping());

        assert_eq!(edges.len(), 1);
        assert!(matches!(edges[0].edge_type, EdgeType::PhysicalLink { .. }));
    }

    #[test]
    fn bidirectional_host_only_neighbors_collapse_to_one_edge() {
        let h1 = make_host("switch-1");
        let h2 = make_host("switch-2");

        let ie1 = make_if_entry(h1.id, 1, 6);
        let ie2 = make_if_entry(h2.id, 1, 6);
        let neighbours = vec![
            neighbor_row(ie1.id, Neighbor::Host(h2.id)),
            neighbor_row(ie2.id, Neighbor::Host(h1.id)),
        ];

        let hosts = vec![h1, h2];
        let interfaces = vec![ie1, ie2];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let builder = L2Builder;
        let (_nodes, edges) = builder.build(&ctx, &l2_grouping());

        assert_eq!(edges.len(), 1);
        assert!(matches!(edges[0].edge_type, EdgeType::NeighborLink { .. }));
    }

    /// The neighbour may name a device that this topology does not contain — a host on another
    /// network, or one filtered out. Drawing to it would leave an edge with no node.
    #[test]
    fn a_neighbor_host_outside_the_topology_draws_no_edge() {
        let h1 = make_host("switch-1");
        let ie1 = make_if_entry(h1.id, 1, 6);
        let neighbours = vec![neighbor_row(ie1.id, Neighbor::Host(Uuid::new_v4()))];

        let hosts = vec![h1];
        let interfaces = vec![ie1];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let builder = L2Builder;
        let (_nodes, edges) = builder.build(&ctx, &l2_grouping());

        assert!(edges.is_empty());
    }

    #[test]
    fn test_virtual_if_types_excluded() {
        let h1 = make_host("switch-1");
        let h2 = make_host("switch-2");

        let ie_eth = make_if_entry(h1.id, 1, 6); // ethernet - included
        let ie_lo = make_if_entry(h1.id, 2, 24); // loopback - excluded
        let ie_vlan = make_if_entry(h1.id, 3, 135); // l2vlan - excluded
        let ie_tun = make_if_entry(h1.id, 4, 131); // tunnel - excluded
        let ie2 = make_if_entry(h2.id, 1, 6);

        // Create neighbor link
        let neighbours = vec![neighbor_row(ie_eth.id, Neighbor::Interface(ie2.id))];

        let hosts = vec![h1, h2];
        let interfaces = vec![ie_eth, ie_lo, ie_vlan, ie_tun, ie2];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let builder = L2Builder;
        let (nodes, _edges) = builder.build(&ctx, &l2_grouping());

        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        // h1: only ethernet port (lo, vlan, tunnel excluded)
        // h2: only ethernet port
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn test_bidirectional_links_deduped() {
        let h1 = make_host("switch-1");
        let h2 = make_host("switch-2");

        let ie1 = make_if_entry(h1.id, 1, 6);
        let ie2 = make_if_entry(h2.id, 1, 6);

        // Both entries point to each other (bidirectional LLDP)
        let neighbours = vec![
            neighbor_row(ie1.id, Neighbor::Interface(ie2.id)),
            neighbor_row(ie2.id, Neighbor::Interface(ie1.id)),
        ];

        let hosts = vec![h1, h2];
        let interfaces = vec![ie1, ie2];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let builder = L2Builder;
        let (_nodes, edges) = builder.build(&ctx, &l2_grouping());

        // Only 1 edge despite bidirectional discovery
        assert_eq!(edges.len(), 1);
    }

    /// GH #701: a shared L2 segment where a router and two hosts behind a bridge each hear both
    /// others. Before the multi-neighbour schema this rendered 2 edges (a port could anchor only
    /// one link) and flipped between scans (the reciprocal tier's adjacency map was keyed on local
    /// interface id alone). With one resolved row per interface-neighbour pair, all three pairwise
    /// adjacencies must render as distinct `PhysicalLink` edges.
    #[test]
    fn a_three_node_shared_segment_renders_three_physical_links() {
        let router = make_host("edge-router");
        let host_a = make_host("mcast-rcv");
        let host_b = make_host("mcast-src");

        let router_port = make_if_entry(router.id, 1, 6);
        let host_a_port = make_if_entry(host_a.id, 1, 6);
        let host_b_port = make_if_entry(host_b.id, 1, 6);

        // Every port resolved a full Interface adjacency to each of the other two — the shape a
        // real reciprocal-tier resolution pass produces once each side names the other.
        let neighbours = vec![
            neighbor_row(router_port.id, Neighbor::Interface(host_a_port.id)),
            neighbor_row(router_port.id, Neighbor::Interface(host_b_port.id)),
            neighbor_row(host_a_port.id, Neighbor::Interface(router_port.id)),
            neighbor_row(host_a_port.id, Neighbor::Interface(host_b_port.id)),
            neighbor_row(host_b_port.id, Neighbor::Interface(router_port.id)),
            neighbor_row(host_b_port.id, Neighbor::Interface(host_a_port.id)),
        ];

        let hosts = vec![router, host_a, host_b];
        let interfaces = vec![router_port, host_a_port, host_b_port];
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
            crate::server::topology::types::views::TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);

        let (_nodes, edges) = L2Builder.build(&ctx, &l2_grouping());

        let physical_links: Vec<&Edge> = edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::PhysicalLink { .. }))
            .collect();
        assert_eq!(
            physical_links.len(),
            3,
            "a shared segment must render all three pairwise adjacencies, not collapse any far-end \
             port onto a single link: {edges:?}"
        );
    }
}
