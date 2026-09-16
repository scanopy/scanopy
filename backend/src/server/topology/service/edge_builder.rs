use itertools::Itertools;
use petgraph::{Graph, graph::NodeIndex};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::IpAddr;
use uuid::Uuid;

use crate::server::{
    dependencies::r#impl::{base::Dependency, types::DependencyType},
    interfaces::r#impl::base::Neighbor,
    topology::{
        service::context::TopologyContext,
        types::{
            edges::{DiscoveryProtocol, Edge, EdgeHandle, EdgeType, EdgeViewConfig},
            grouping::GroupingConfig,
            nodes::Node,
        },
    },
};

pub struct EdgeBuilder;

impl EdgeBuilder {
    /// Create dependency edges (connecting services in a dependency's service chain)
    pub fn create_dependency_edges(ctx: &TopologyContext) -> Vec<Edge> {
        ctx.dependencies
            .iter()
            .flat_map(|dependency| {
                let binding_ids = dependency.binding_ids();
                match &dependency.base.dependency_type {
                    DependencyType::RequestPath => binding_ids
                        .windows(2)
                        .filter_map(|window| {
                            EdgeBuilder::edge_from_service_bindings(
                                ctx, window[0], window[1], dependency,
                            )
                        })
                        .collect::<Vec<Edge>>(),
                    DependencyType::HubAndSpoke => {
                        let mut binding_ids = binding_ids.clone();
                        binding_ids.reverse();
                        if let Some(hub_binding_id) = binding_ids.pop() {
                            return binding_ids
                                .iter()
                                .filter_map(|spoke_binding| {
                                    EdgeBuilder::edge_from_service_bindings(
                                        ctx,
                                        hub_binding_id,
                                        *spoke_binding,
                                        dependency,
                                    )
                                })
                                .collect::<Vec<Edge>>();
                        }
                        Vec::new()
                    }
                }
            })
            .collect()
    }

    /// Create edges to connect a host that virtualizes containers via Docker
    /// to the containerized services on Docker bridge subnets.
    ///
    /// Each edge names the containerized services it stands for, so the inspector lists
    /// exactly that set rather than re-deriving it from an endpoint the frontend may have
    /// elevated onto a subnet box.
    pub fn create_containerized_service_edges(
        ctx: &TopologyContext,
        grouping: &GroupingConfig,
    ) -> Vec<Edge> {
        let mut docker_service_to_containerized_service_ids: HashMap<Uuid, Vec<Uuid>> =
            HashMap::new();

        ctx.services.iter().for_each(|s| {
            // Any container-runtime manager (Docker, Podman, …), read off the column rather
            // than a variant.
            if let Some(runtime_service_id) = s.base.virtualization_service_id {
                let entry = docker_service_to_containerized_service_ids
                    .entry(runtime_service_id)
                    .or_default();
                if !entry.contains(&s.id) {
                    entry.push(s.id);
                }
            }
        });

        ctx.services
            .iter()
            .filter(|s| {
                docker_service_to_containerized_service_ids
                    .keys()
                    .contains(&s.id)
            })
            .filter_map(|s| {
                let host = ctx.get_host_by_id(s.base.host_id)?;
                let origin_ip_address =
                    ctx.get_first_non_container_bridge_ip_address_for_host(host.id)?;
                Some((s, host, origin_ip_address))
            })
            .flat_map(|(s, host, origin_ip_address)| {
                // Bridge-subnet IP addresses on this host, with the subnet each sits in.
                let bridge_ips: HashMap<Uuid, (Uuid, IpAddr)> = ctx
                    .get_ip_addresses_for_host(host.id)
                    .iter()
                    .filter_map(|i| {
                        let subnet = ctx.get_subnet_by_id(i.base.subnet_id)?;
                        subnet
                            .base
                            .subnet_type
                            .is_container_bridge()
                            .then_some((i.id, (subnet.id, i.base.ip_address)))
                    })
                    .collect();

                // One edge per bridge subnet the runtime's containers actually sit in, rather
                // than one per container. A container attached to several subnets would
                // otherwise anchor to whichever of its bindings came first, leaving every other
                // subnet it belongs to with no connection to its host. Edge count now scales
                // with subnets-per-host instead of containers-per-host, and each edge elevates
                // onto the subnet container (which accepts edges) rather than an element card.
                // The lowest address in each subnet is the representative, so the target is
                // stable across renders instead of following binding order.
                let mut target_by_subnet: BTreeMap<Uuid, (IpAddr, Uuid)> = BTreeMap::new();
                // The containers reachable on each of those subnets — what the edge represents.
                let mut containers_by_subnet: BTreeMap<Uuid, Vec<Uuid>> = BTreeMap::new();
                // Every (bridge subnet, address) a container is actually attached at, keyed so
                // iteration is deterministic. Needed for the views where subnets are not
                // containers and each attachment has to carry its own edge.
                let mut containers_by_attachment: BTreeMap<(Uuid, Uuid), Vec<Uuid>> =
                    BTreeMap::new();
                for containerized_id in docker_service_to_containerized_service_ids
                    .get(&s.id)
                    .unwrap_or(&Vec::new())
                {
                    let Some(containerized) = ctx.get_service_by_id(*containerized_id) else {
                        continue;
                    };
                    for ip_address_id in containerized
                        .base
                        .bindings
                        .iter()
                        .filter_map(|b| b.ip_address_id())
                    {
                        let Some((subnet_id, address)) = bridge_ips.get(&ip_address_id) else {
                            continue;
                        };
                        let candidate = (*address, ip_address_id);
                        target_by_subnet
                            .entry(*subnet_id)
                            .and_modify(|current| {
                                if candidate < *current {
                                    *current = candidate;
                                }
                            })
                            .or_insert(candidate);
                        let containers = containers_by_subnet.entry(*subnet_id).or_default();
                        if !containers.contains(containerized_id) {
                            containers.push(*containerized_id);
                        }
                        let at_address = containers_by_attachment
                            .entry((*subnet_id, ip_address_id))
                            .or_default();
                        if !at_address.contains(containerized_id) {
                            at_address.push(*containerized_id);
                        }
                    }
                }

                // With bridges merged, every one of this runtime's subnets renders as a single
                // box, so the per-subnet edges would stack into identical overlapping lines on
                // it. Collapse them into the one edge the user actually sees, carrying the
                // union of what those subnets hold.
                let per_edge: Vec<(Uuid, Vec<Uuid>, Vec<Uuid>)> =
                    if grouping.should_group_container_bridges() {
                        let Some((_, target)) = target_by_subnet.values().min().copied() else {
                            return Vec::new();
                        };
                        let mut containers: Vec<Uuid> = Vec::new();
                        for subnet_containers in containers_by_subnet.values() {
                            for id in subnet_containers {
                                if !containers.contains(id) {
                                    containers.push(*id);
                                }
                            }
                        }
                        vec![(
                            target,
                            target_by_subnet.keys().copied().collect(),
                            containers,
                        )]
                    } else if grouping.subnets_are_containers() {
                        target_by_subnet
                            .iter()
                            .map(|(subnet_id, (_, target))| {
                                (
                                    *target,
                                    vec![*subnet_id],
                                    containers_by_subnet
                                        .get(subnet_id)
                                        .cloned()
                                        .unwrap_or_default(),
                                )
                            })
                            .collect()
                    } else {
                        // Subnets are not containers here, so there is no shared box for one
                        // representative address to stand for the rest — each attachment needs
                        // its own edge, or every container that is not the representative
                        // renders unconnected to the runtime that owns it.
                        containers_by_attachment
                            .iter()
                            .map(|((subnet_id, ip_address_id), containers)| {
                                (*ip_address_id, vec![*subnet_id], containers.clone())
                            })
                            .collect()
                    };

                per_edge
                    .into_iter()
                    .map(|(target, subnet_ids, containerized_service_ids)| {
                        let is_multi_hop = ctx.edge_is_multi_hop(&origin_ip_address.id, &target);

                        Edge {
                            id: Uuid::new_v4(),
                            source: origin_ip_address.id,
                            target,
                            edge_type: EdgeType::ContainerRuntime {
                                service_id: s.id,
                                host_id: host.id,
                                subnet_ids,
                                containerized_service_ids,
                            },
                            label: Some(format!(
                                "{} on {}",
                                s.base.name,
                                ctx.host_container_header(host).unwrap_or_default()
                            )),
                            source_handle: EdgeHandle::Bottom,
                            target_handle: EdgeHandle::Top,
                            is_multi_hop,
                            view_config: EdgeViewConfig::default(),
                            relation_key: None,
                        }
                    })
                    .collect::<Vec<Edge>>()
            })
            .collect()
    }

    /// Tie together the addresses of a container that sits on several of its host's
    /// container-bridge subnets, so a multi-attached container reads as one thing rather than as
    /// unrelated cards in separate subnet boxes.
    ///
    /// Hub-and-spoke from the container's lowest address rather than a mesh: a container on N
    /// subnets produces N-1 edges instead of N(N-1)/2, and anchoring on the lowest address keeps
    /// the shape stable across renders.
    pub fn create_same_container_edges(ctx: &TopologyContext) -> Vec<Edge> {
        ctx.services
            .iter()
            .filter(|s| s.base.virtualization_metadata.is_some())
            .flat_map(|containerized| {
                // Addresses of this container that sit on one of its host's bridge subnets.
                let mut bridge_addresses: Vec<(IpAddr, Uuid)> = containerized
                    .base
                    .bindings
                    .iter()
                    .filter_map(|b| b.ip_address_id())
                    .unique()
                    .filter_map(|ip_address_id| {
                        let ip_address = ctx.get_ip_address_by_id(Some(ip_address_id))?;
                        let subnet = ctx.get_subnet_by_id(ip_address.base.subnet_id)?;
                        subnet
                            .base
                            .subnet_type
                            .is_container_bridge()
                            .then_some((ip_address.base.ip_address, ip_address_id))
                    })
                    .collect();

                // Nothing to tie together unless it spans more than one.
                if bridge_addresses.len() < 2 {
                    return Vec::new();
                }
                bridge_addresses.sort();

                let (_, anchor) = bridge_addresses[0];
                bridge_addresses[1..]
                    .iter()
                    .map(|(_, target)| Edge {
                        id: Uuid::new_v4(),
                        source: anchor,
                        target: *target,
                        edge_type: EdgeType::SameContainer {
                            service_id: containerized.id,
                        },
                        label: Some(containerized.base.name.clone()),
                        source_handle: EdgeHandle::Bottom,
                        target_handle: EdgeHandle::Top,
                        is_multi_hop: ctx.edge_is_multi_hop(&anchor, target),
                        view_config: EdgeViewConfig::default(),
                        relation_key: None,
                    })
                    .collect::<Vec<Edge>>()
            })
            .collect()
    }

    // Create edges to connect a host that virtualizes other hosts as VMs
    pub fn create_vm_host_edges(ctx: &TopologyContext) -> Vec<Edge> {
        // Proxmox service interface binding that is present for a given subnet.
        // There could be multiple host ip_addresses with a given subnet, we arbitrarily choose the first one so there's
        // one clustering hub rather than multiple hubs
        // (subnet_id, proxmox_service_id) : (ip_address_id)
        let mut subnet_to_promxox_host_ip_address_id: HashMap<(Uuid, Uuid), Uuid> = HashMap::new();

        // Hosts VMs managed by a given proxmox service
        let mut vm_host_id_to_proxmox_service: HashMap<Uuid, Uuid> = HashMap::new();

        ctx.hosts.iter().for_each(|h| {
            // Any host-level virtualization manager (Proxmox, vCenter, ESXi, …), read off the
            // column rather than a variant.
            if let Some(vm_manager_service_id) = h.base.virtualization_service_id {
                // Create mapping between subnet and hypervisor interface(s) on that subnet
                if let Some(promxox_service) = ctx.get_service_by_id(vm_manager_service_id) {
                    promxox_service
                        .base
                        .bindings
                        .iter()
                        .filter_map(|b| b.ip_address_id())
                        .for_each(|i| {
                            if let Some(subnet) = ctx.get_subnet_from_ip_address_id(i)
                                && !subnet_to_promxox_host_ip_address_id
                                    .contains_key(&(subnet.id, promxox_service.id))
                            {
                                subnet_to_promxox_host_ip_address_id
                                    .entry((subnet.id, promxox_service.id))
                                    .insert_entry(i);
                            }
                        });
                }

                vm_host_id_to_proxmox_service.insert(h.id, vm_manager_service_id);
            }
        });

        // Creates edges between interface that proxmox service has on a given subnet with ip_addresses that the virtualized host has on the subnet
        ctx.hosts
            .iter()
            .flat_map(|h| {
                if let Some(proxmox_service_id) = vm_host_id_to_proxmox_service.get(&h.id) {
                    let host_interfaces = ctx.get_ip_addresses_for_host(h.id);
                    return host_interfaces
                        .into_iter()
                        .filter_map(|i| {
                            if let Some(proxmox_service_ip_address_id) =
                                subnet_to_promxox_host_ip_address_id
                                    .get(&(i.base.subnet_id, *proxmox_service_id))
                            {
                                let is_multi_hop =
                                    ctx.edge_is_multi_hop(proxmox_service_ip_address_id, &i.id);

                                return Some(Edge {
                                    id: Uuid::new_v4(),
                                    source: *proxmox_service_ip_address_id,
                                    target: i.id,
                                    edge_type: EdgeType::Hypervisor {
                                        hypervisor_service_id: *proxmox_service_id,
                                    },
                                    label: None,
                                    source_handle: EdgeHandle::Bottom,
                                    target_handle: EdgeHandle::Top,
                                    is_multi_hop,
                                    view_config: EdgeViewConfig::default(),
                                    relation_key: None,
                                });
                            }
                            None
                        })
                        .collect();
                }
                Vec::new()
            })
            .collect()
    }

    /// Create host-level Hypervisor edges (hypervisor host → VM host).
    /// Unlike `create_vm_host_edges` which uses interface IDs, this uses host IDs
    /// as source/target for views where elements are hosts (e.g. Infrastructure).
    pub fn create_vm_host_edges_by_host(ctx: &TopologyContext) -> Vec<Edge> {
        ctx.hosts
            .iter()
            .filter_map(|h| {
                let vm_manager_service_id = h.base.virtualization_service_id?;
                let proxmox_service = ctx.get_service_by_id(vm_manager_service_id)?;
                Some(Edge {
                    id: Uuid::new_v4(),
                    source: proxmox_service.base.host_id,
                    target: h.id,
                    edge_type: EdgeType::Hypervisor {
                        hypervisor_service_id: vm_manager_service_id,
                    },
                    label: None,
                    source_handle: EdgeHandle::Bottom,
                    target_handle: EdgeHandle::Top,
                    is_multi_hop: false,
                    view_config: EdgeViewConfig::default(),
                    relation_key: None,
                })
            })
            .collect()
    }

    /// Create interface edges (connecting multiple ip_addresses on the same host)
    pub fn create_interface_edges(ctx: &TopologyContext) -> Vec<Edge> {
        ctx.hosts
            .iter()
            .flat_map(|host| {
                let host_interfaces = ctx.get_ip_addresses_for_host(host.id);

                // Use the first non-container-bridge interface as the origin
                // This ensures we don't try to create edges FROM a container-bridge interface
                if let Some(origin_ip_address) =
                    ctx.get_first_non_container_bridge_ip_address_for_host(host.id)
                {
                    host_interfaces
                        .iter()
                        .filter(|ip_address| ip_address.id != origin_ip_address.id)
                        .filter_map(|ip_address| {
                            let source_subnet =
                                ctx.get_subnet_by_id(origin_ip_address.base.subnet_id);
                            let target_subnet = ctx.get_subnet_by_id(ip_address.base.subnet_id);

                            if let Some(source_subnet) = source_subnet
                                && source_subnet.base.subnet_type.is_container_bridge()
                            {
                                return None;
                            }

                            if let Some(target_subnet) = target_subnet
                                && target_subnet.base.subnet_type.is_container_bridge()
                            {
                                return None;
                            }

                            let is_multi_hop =
                                ctx.edge_is_multi_hop(&origin_ip_address.id, &ip_address.id);

                            // Hide label if both ip_addresses are on the same subnet
                            let label =
                                if origin_ip_address.base.subnet_id == ip_address.base.subnet_id {
                                    None
                                } else {
                                    ctx.host_container_header(host)
                                };

                            Some(Edge {
                                id: Uuid::new_v4(),
                                source: origin_ip_address.id,
                                target: ip_address.id,
                                edge_type: EdgeType::SameHost { host_id: host.id },
                                label,
                                source_handle: EdgeHandle::Bottom,
                                target_handle: EdgeHandle::Top,
                                is_multi_hop,
                                view_config: EdgeViewConfig::default(),
                                relation_key: None,
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            })
            .collect()
    }

    /// Create physical link edges from LLDP/CDP neighbor discovery
    /// Only creates edges when both endpoints have associated ip_addresses (nodes)
    ///
    /// Two dedup passes, not one. The first collapses two reports of the same *interface* pair
    /// (A→B and B→A). The second collapses onto the resolved *ip_address* pair: GH #701 lets a
    /// port anchor several adjacencies, so two distinct interface pairs between the same host
    /// pair can now both survive the first dedup and then resolve onto the same ip pair via
    /// `resolve_ip_address_for_interface`'s single-IP-host fallback — previously impossible, since
    /// a port could anchor at most one edge.
    pub fn create_physical_link_edges(ctx: &TopologyContext) -> Vec<Edge> {
        // Track processed pairs to avoid duplicate edges (A→B and B→A)
        let mut processed_interface_pairs: HashSet<(Uuid, Uuid)> = HashSet::new();
        let mut processed_ip_pairs: HashSet<(Uuid, Uuid)> = HashSet::new();

        ctx.get_interfaces_with_neighbor()
            .into_iter()
            .filter_map(|row| {
                let source_id = row.interface_id;
                // Get the target Interface ID from resolved neighbor
                let target_interface_id = match row.neighbor {
                    Neighbor::Interface(id) => id,
                    Neighbor::Host(_) => return None, // Already filtered by get_interfaces_with_neighbor
                };

                // Skip if we've already processed this pair (in either direction)
                let pair_key = if source_id < target_interface_id {
                    (source_id, target_interface_id)
                } else {
                    (target_interface_id, source_id)
                };

                if !processed_interface_pairs.insert(pair_key) {
                    return None;
                }

                let source_entry = ctx.get_interface_by_id(source_id)?;
                // Resolve interface IDs with single-interface host fallback
                let source_ip_address_id = ctx.resolve_ip_address_for_interface(source_entry)?;
                let target_entry = ctx.get_interface_by_id(target_interface_id)?;
                let target_ip_address_id = ctx.resolve_ip_address_for_interface(target_entry)?;

                let ip_pair_key = if source_ip_address_id < target_ip_address_id {
                    (source_ip_address_id, target_ip_address_id)
                } else {
                    (target_ip_address_id, source_ip_address_id)
                };
                if !processed_ip_pairs.insert(ip_pair_key) {
                    return None;
                }

                let is_multi_hop =
                    ctx.edge_is_multi_hop(&source_ip_address_id, &target_ip_address_id);

                // Build label from port descriptions: "Gi0/1 ↔ Gi0/2"
                let label = Some(format!(
                    "{} ↔ {}",
                    source_entry.display_name(),
                    target_entry.display_name()
                ));

                // The specific adjacency row's own evidence, not an interface-level predicate —
                // see `TopologyContext::interface_has_lldp_evidence`.
                let protocol = if ctx.interface_has_lldp_evidence(source_id) {
                    DiscoveryProtocol::LLDP
                } else {
                    DiscoveryProtocol::CDP
                };

                Some(Edge {
                    id: Uuid::new_v4(),
                    source: source_ip_address_id,
                    target: target_ip_address_id,
                    edge_type: EdgeType::PhysicalLink {
                        source_entity_id: source_entry.id,
                        target_entity_id: target_entry.id,
                        protocol,
                    },
                    label,
                    source_handle: EdgeHandle::Bottom,
                    target_handle: EdgeHandle::Top,
                    is_multi_hop,
                    view_config: EdgeViewConfig::default(),
                    relation_key: None,
                })
            })
            .collect()
    }

    /// Order-independent key for a pair of hosts, so an adjacency reported from either end
    /// collapses to one entry.
    fn host_pair_key(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
        if a < b { (a, b) } else { (b, a) }
    }

    /// The host pairs `edges` already joins with a port-precise `PhysicalLink`. Pass to
    /// `create_neighbor_link_edges` so the approximate edge is not drawn over the exact one.
    pub fn physical_link_host_pairs(
        ctx: &TopologyContext,
        edges: &[Edge],
    ) -> HashSet<(Uuid, Uuid)> {
        edges
            .iter()
            .filter_map(|edge| match &edge.edge_type {
                EdgeType::PhysicalLink {
                    source_entity_id,
                    target_entity_id,
                    ..
                } => {
                    let source = ctx.get_interface_by_id(*source_entity_id)?;
                    let target = ctx.get_interface_by_id(*target_entity_id)?;
                    Some(Self::host_pair_key(
                        source.base.host_id,
                        target.base.host_id,
                    ))
                }
                _ => None,
            })
            .collect()
    }

    /// Create device-level adjacency edges for LLDP/CDP neighbours that resolved to a host but
    /// not to a port. Without these the neighbour contributes nothing to the graph at all and
    /// the device it identifies never appears (GH #649).
    ///
    /// `node_for_host` maps a host to the node the calling view anchors on — each view renders
    /// against its own node-id space, so the caller owns that mapping. `linked_host_pairs` are
    /// pairs already joined by a `PhysicalLink`: the precise edge supersedes the approximate
    /// one, so those are skipped rather than drawn on top of it.
    pub fn create_neighbor_link_edges(
        ctx: &TopologyContext,
        node_for_host: impl Fn(Uuid) -> Uuid,
        linked_host_pairs: &HashSet<(Uuid, Uuid)>,
    ) -> Vec<Edge> {
        let mut processed_pairs: HashSet<(Uuid, Uuid)> = HashSet::new();

        ctx.get_interfaces_with_host_neighbor()
            .into_iter()
            .filter_map(|row| {
                let target_host_id = match row.neighbor {
                    Neighbor::Host(id) => id,
                    Neighbor::Interface(_) => return None, // Already filtered by get_interfaces_with_host_neighbor
                };
                let source_entry = ctx.get_interface_by_id(row.interface_id)?;
                let source_host_id = source_entry.base.host_id;
                if source_host_id == target_host_id {
                    return None;
                }

                // Both ends must be in this topology, or the edge dangles.
                let source_host = ctx.get_host_by_id(source_host_id)?;
                let target_host = ctx.get_host_by_id(target_host_id)?;

                let pair_key = Self::host_pair_key(source_host_id, target_host_id);
                if linked_host_pairs.contains(&pair_key) {
                    return None;
                }
                if !processed_pairs.insert(pair_key) {
                    return None;
                }

                // The specific adjacency row's own evidence, not an interface-level predicate — see
                // `TopologyContext::interface_has_lldp_evidence`.
                let protocol = if ctx.interface_has_lldp_evidence(row.interface_id) {
                    DiscoveryProtocol::LLDP
                } else {
                    DiscoveryProtocol::CDP
                };

                Some(Edge {
                    id: Uuid::new_v4(),
                    source: node_for_host(source_host_id),
                    target: node_for_host(target_host_id),
                    edge_type: EdgeType::NeighborLink {
                        source_host_id,
                        target_host_id,
                        protocol,
                    },
                    label: Some(format!(
                        "{} ↔ {}",
                        ctx.host_container_header(source_host).unwrap_or_default(),
                        ctx.host_container_header(target_host).unwrap_or_default()
                    )),
                    source_handle: EdgeHandle::Bottom,
                    target_handle: EdgeHandle::Top,
                    is_multi_hop: false,
                    view_config: EdgeViewConfig::default(),
                    relation_key: None,
                })
            })
            .collect()
    }

    /// Add edges to a petgraph Graph
    pub fn add_edges_to_graph(
        graph: &mut Graph<Node, Edge>,
        node_indices: &HashMap<Uuid, NodeIndex>,
        edges: Vec<Edge>,
    ) {
        for edge in edges {
            if let (Some(&src_idx), Some(&tgt_idx)) = (
                node_indices.get(&edge.source),
                node_indices.get(&edge.target),
            ) {
                graph.add_edge(src_idx, tgt_idx, edge);
            }
        }
    }

    pub fn edge_from_service_bindings(
        ctx: &TopologyContext,
        source_id: Uuid,
        target_id: Uuid,
        dependency: &Dependency,
    ) -> Option<Edge> {
        let source_ip_address = ctx.services.iter().find_map(|s| {
            if let Some(source_binding) = s.get_binding(source_id) {
                return Some(source_binding.ip_address_id());
            }
            None
        });

        let target_ip_address = ctx.services.iter().find_map(|s| {
            if let Some(target_binding) = s.get_binding(target_id) {
                return Some(target_binding.ip_address_id());
            }
            None
        });

        let (Some(Some(source_ip_address)), Some(Some(target_ip_address))) =
            (source_ip_address, target_ip_address)
        else {
            return None;
        };

        let is_multi_hop = ctx.edge_is_multi_hop(&source_ip_address, &target_ip_address);

        // If edge is intra-subnet, don't label - gets too messy
        let label = if ctx
            .get_subnet_from_ip_address_id(source_ip_address)
            .map(|s| s.id)
            == ctx
                .get_subnet_from_ip_address_id(target_ip_address)
                .map(|s| s.id)
        {
            None
        } else {
            Some(dependency.base.name.to_string())
        };

        Some(Edge {
            id: Uuid::new_v4(),
            source: source_ip_address,
            target: target_ip_address,
            edge_type: match dependency.base.dependency_type {
                DependencyType::HubAndSpoke => EdgeType::HubAndSpoke {
                    source_id,
                    target_id,
                    dependency_id: dependency.id,
                },
                DependencyType::RequestPath => EdgeType::RequestPath {
                    source_id,
                    target_id,
                    dependency_id: dependency.id,
                },
            },
            label,
            source_handle: EdgeHandle::Bottom,
            target_handle: EdgeHandle::Top,
            is_multi_hop,
            view_config: EdgeViewConfig::default(),
            relation_key: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::bindings::r#impl::base::Binding;
    use crate::server::hosts::r#impl::base::{Host, HostBase};
    use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
    use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
    use crate::server::services::r#impl::base::{Service, ServiceBase};
    use crate::server::services::r#impl::virtualization::{
        DockerVirtualization, ServiceVirtualization,
    };
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
    use crate::server::subnets::r#impl::types::SubnetType;
    use crate::server::topology::service::context::TopologyContext;
    use crate::server::topology::types::base::TopologyOptions;
    use crate::server::topology::types::grouping::{ContainerRule, IdentifiedRule};
    use crate::server::topology::types::views::TopologyView;
    use cidr::{IpCidr, Ipv4Cidr};
    use std::net::Ipv4Addr;

    /// Bridges rendered as their own boxes — one runtime edge per bridge subnet.
    ///
    /// `BySubnet` is what makes a subnet a container, and so what makes one representative
    /// address able to stand for every other member of it. The fixture used to leave
    /// `container_rules` empty, which described no view at all: nothing rendered subnets as
    /// boxes, yet the assertions expected the per-subnet collapsing that only a box justifies.
    fn ungrouped() -> GroupingConfig {
        GroupingConfig {
            container_rules: vec![IdentifiedRule::new(ContainerRule::BySubnet)],
            element_rules: vec![],
        }
    }

    /// Elements grouped by application, as in the Application view — subnets are not containers,
    /// so there is no shared box and every attachment carries its own edge.
    fn by_application() -> GroupingConfig {
        GroupingConfig {
            container_rules: vec![IdentifiedRule::new(ContainerRule::ByApplication {
                tag_ids: vec![],
            })],
            element_rules: vec![],
        }
    }

    /// Bridges merged into one box — one runtime edge for the box. Carries `BySubnet` too,
    /// because `MergeContainerBridges` only ever applies in the view that has it.
    fn merged() -> GroupingConfig {
        GroupingConfig {
            container_rules: vec![
                IdentifiedRule::new(ContainerRule::BySubnet),
                IdentifiedRule::new(ContainerRule::MergeContainerBridges),
            ],
            element_rules: vec![],
        }
    }

    /// The containerized services an edge says it stands for.
    fn containerized_ids(edge: &Edge) -> Vec<Uuid> {
        match &edge.edge_type {
            EdgeType::ContainerRuntime {
                containerized_service_ids,
                ..
            } => containerized_service_ids.clone(),
            other => panic!("expected a ContainerRuntime edge, got {other:?}"),
        }
    }

    /// The bridge subnets an edge says it reaches.
    fn subnet_ids(edge: &Edge) -> Vec<Uuid> {
        match &edge.edge_type {
            EdgeType::ContainerRuntime { subnet_ids, .. } => subnet_ids.clone(),
            other => panic!("expected a ContainerRuntime edge, got {other:?}"),
        }
    }

    fn subnet(network_id: Uuid, name: &str, third_octet: u8, subnet_type: SubnetType) -> Subnet {
        Subnet {
            id: Uuid::new_v4(),
            base: SubnetBase {
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(172, third_octet, 0, 0), 16).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                name: name.to_string(),
                subnet_type,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn ip(network_id: Uuid, host_id: Uuid, subnet_id: Uuid, addr: Ipv4Addr) -> IPAddress {
        IPAddress {
            id: Uuid::new_v4(),
            base: IPAddressBase {
                network_id,
                host_id,
                subnet_id,
                ip_address: std::net::IpAddr::V4(addr),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// A container attached to several bridge subnets must connect its runtime to EVERY subnet it
    /// sits in. Anchoring one edge to the container's first binding left the other subnets with
    /// members but no link to their host — the box looked orphaned in the L3 view.
    #[test]
    fn runtime_edges_reach_every_bridge_subnet_a_container_sits_in() {
        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        let lan = subnet(network_id, "lan", 30, SubnetType::Lan);
        let db_net = subnet(network_id, "db-net", 20, SubnetType::DockerBridge);
        let mgmt_net = subnet(network_id, "mgmt-net", 21, SubnetType::DockerBridge);

        let host_ip = ip(network_id, host_id, lan.id, Ipv4Addr::new(172, 30, 0, 5));
        let db_ip = ip(network_id, host_id, db_net.id, Ipv4Addr::new(172, 20, 0, 2));
        let mgmt_ip = ip(
            network_id,
            host_id,
            mgmt_net.id,
            Ipv4Addr::new(172, 21, 0, 2),
        );

        let host = Host {
            id: host_id,
            base: HostBase {
                name: HostName::manual("docker-host".to_string()),
                network_id,
                ..Default::default()
            },
            ..Default::default()
        };

        let runtime = Service {
            id: Uuid::new_v4(),
            base: ServiceBase {
                host_id,
                network_id,
                name: "Docker".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        // One container, attached to both bridge subnets.
        let container = Service {
            id: Uuid::new_v4(),
            base: ServiceBase {
                host_id,
                network_id,
                name: "multi-attached".to_string(),
                virtualization_metadata: Some(ServiceVirtualization::Docker(
                    DockerVirtualization {
                        container_name: Some("multi-attached".to_string()),
                        container_id: Some("abc123".to_string()),
                        compose_project: None,
                    },
                )),
                virtualization_service_id: Some(runtime.id),
                bindings: vec![
                    Binding::new_ip_address_serviceless(db_ip.id),
                    Binding::new_ip_address_serviceless(mgmt_ip.id),
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        let container_id = container.id;
        let hosts = vec![host];
        let ip_addresses = vec![host_ip.clone(), db_ip.clone(), mgmt_ip.clone()];
        let subnets = vec![lan, db_net.clone(), mgmt_net.clone()];
        let services = vec![runtime, container];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
            &services,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );

        let edges = EdgeBuilder::create_containerized_service_edges(&ctx, &ungrouped());

        let targets: HashSet<Uuid> = edges.iter().map(|e| e.target).collect();
        assert_eq!(
            targets,
            HashSet::from([db_ip.id, mgmt_ip.id]),
            "each bridge subnet the container sits in needs its own runtime edge"
        );
        assert!(
            edges.iter().all(|e| e.source == host_ip.id),
            "edges originate from the host's non-bridge address"
        );
        assert!(
            edges
                .iter()
                .all(|e| containerized_ids(e) == vec![container_id]),
            "the container sits in both subnets, so both edges stand for it"
        );

        // Merging the bridges into one box leaves one edge for it, standing for the same set.
        let merged_edges = EdgeBuilder::create_containerized_service_edges(&ctx, &merged());
        assert_eq!(
            merged_edges.len(),
            1,
            "merged bridges render as one box, so they get one edge"
        );
        assert_eq!(containerized_ids(&merged_edges[0]), vec![container_id]);
    }

    /// A container on several bridge subnets gets its addresses tied together, hub-and-spoke from
    /// the lowest — N-1 edges, not a mesh — so the same container reads as one thing across boxes.
    #[test]
    fn same_container_ties_a_containers_addresses_together() {
        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        let lan = subnet(network_id, "lan", 30, SubnetType::Lan);
        let db_net = subnet(network_id, "db-net", 20, SubnetType::DockerBridge);
        let mgmt_net = subnet(network_id, "mgmt-net", 21, SubnetType::DockerBridge);
        let proxy_net = subnet(network_id, "proxy-net", 19, SubnetType::DockerBridge);

        // Lowest address is on proxy-net (172.19.x), so that is the anchor.
        let proxy_ip = ip(
            network_id,
            host_id,
            proxy_net.id,
            Ipv4Addr::new(172, 19, 0, 3),
        );
        let db_ip = ip(network_id, host_id, db_net.id, Ipv4Addr::new(172, 20, 0, 3));
        let mgmt_ip = ip(
            network_id,
            host_id,
            mgmt_net.id,
            Ipv4Addr::new(172, 21, 0, 2),
        );
        // A non-bridge address on the same host must not be tied in.
        let host_ip = ip(network_id, host_id, lan.id, Ipv4Addr::new(172, 30, 0, 5));

        let host = Host {
            id: host_id,
            base: HostBase {
                name: HostName::manual("docker-host".to_string()),
                network_id,
                ..Default::default()
            },
            ..Default::default()
        };
        let runtime_id = Uuid::new_v4();
        let container = Service {
            id: Uuid::new_v4(),
            base: ServiceBase {
                host_id,
                network_id,
                name: "multi-attached".to_string(),
                virtualization_metadata: Some(ServiceVirtualization::Docker(
                    DockerVirtualization {
                        container_name: Some("multi-attached".to_string()),
                        container_id: Some("abc123".to_string()),
                        compose_project: None,
                    },
                )),
                virtualization_service_id: Some(runtime_id),
                bindings: vec![
                    Binding::new_ip_address_serviceless(db_ip.id),
                    Binding::new_ip_address_serviceless(mgmt_ip.id),
                    Binding::new_ip_address_serviceless(proxy_ip.id),
                    Binding::new_ip_address_serviceless(host_ip.id),
                ],
                ..Default::default()
            },
            ..Default::default()
        };

        let hosts = vec![host];
        let ip_addresses = vec![
            host_ip.clone(),
            proxy_ip.clone(),
            db_ip.clone(),
            mgmt_ip.clone(),
        ];
        let subnets = vec![lan, db_net, mgmt_net, proxy_net];
        let services = vec![container];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
            &services,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );

        let edges = EdgeBuilder::create_same_container_edges(&ctx);

        assert_eq!(edges.len(), 2, "three subnets means two edges, not a mesh");
        assert!(
            edges.iter().all(|e| e.source == proxy_ip.id),
            "all edges anchor on the container's lowest address"
        );
        assert_eq!(
            edges.iter().map(|e| e.target).collect::<HashSet<_>>(),
            HashSet::from([db_ip.id, mgmt_ip.id])
        );
        assert!(
            !edges
                .iter()
                .any(|e| e.target == host_ip.id || e.source == host_ip.id),
            "the host's own non-bridge address is not part of the container"
        );
    }

    /// A container on a single bridge subnet has nothing to tie together.
    #[test]
    fn same_container_needs_more_than_one_subnet() {
        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        let only = subnet(network_id, "only", 20, SubnetType::DockerBridge);
        let only_ip = ip(network_id, host_id, only.id, Ipv4Addr::new(172, 20, 0, 2));

        let host = Host {
            id: host_id,
            base: HostBase {
                name: HostName::manual("docker-host".to_string()),
                network_id,
                ..Default::default()
            },
            ..Default::default()
        };
        let container = Service {
            id: Uuid::new_v4(),
            base: ServiceBase {
                host_id,
                network_id,
                name: "single".to_string(),
                virtualization_metadata: Some(ServiceVirtualization::Docker(
                    DockerVirtualization {
                        container_name: Some("single".to_string()),
                        container_id: Some("def456".to_string()),
                        compose_project: None,
                    },
                )),
                virtualization_service_id: Some(Uuid::new_v4()),
                bindings: vec![Binding::new_ip_address_serviceless(only_ip.id)],
                ..Default::default()
            },
            ..Default::default()
        };

        let hosts = vec![host];
        let ip_addresses = vec![only_ip];
        let subnets = vec![only];
        let services = vec![container];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
            &services,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );

        assert!(EdgeBuilder::create_same_container_edges(&ctx).is_empty());
    }

    /// Several containers sharing a subnet collapse to a single edge for it, so edge count tracks
    /// subnets on the host rather than containers on it.
    #[test]
    fn containers_sharing_a_subnet_produce_one_edge() {
        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        let lan = subnet(network_id, "lan", 30, SubnetType::Lan);
        let shared = subnet(network_id, "shared", 20, SubnetType::DockerBridge);

        let host_ip = ip(network_id, host_id, lan.id, Ipv4Addr::new(172, 30, 0, 5));
        let first = ip(network_id, host_id, shared.id, Ipv4Addr::new(172, 20, 0, 2));
        let second = ip(network_id, host_id, shared.id, Ipv4Addr::new(172, 20, 0, 3));

        let host = Host {
            id: host_id,
            base: HostBase {
                name: HostName::manual("docker-host".to_string()),
                network_id,
                ..Default::default()
            },
            ..Default::default()
        };
        let runtime = Service {
            id: Uuid::new_v4(),
            base: ServiceBase {
                host_id,
                network_id,
                name: "Docker".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let runtime_id = runtime.id;
        let containerized = |name: &str, ip_id: Uuid| Service {
            id: Uuid::new_v4(),
            base: ServiceBase {
                host_id,
                network_id,
                name: name.to_string(),
                virtualization_metadata: Some(ServiceVirtualization::Docker(
                    DockerVirtualization {
                        container_name: Some(name.to_string()),
                        container_id: Some(name.to_string()),
                        compose_project: None,
                    },
                )),
                virtualization_service_id: Some(runtime_id),
                bindings: vec![Binding::new_ip_address_serviceless(ip_id)],
                ..Default::default()
            },
            ..Default::default()
        };

        let hosts = vec![host];
        let ip_addresses = vec![host_ip.clone(), first.clone(), second.clone()];
        let subnets = vec![lan, shared];
        let services = vec![
            runtime,
            containerized("one", first.id),
            containerized("two", second.id),
        ];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
            &services,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );

        let edges = EdgeBuilder::create_containerized_service_edges(&ctx, &ungrouped());
        assert_eq!(edges.len(), 1, "one edge per subnet, not per container");
        // Lowest address in the subnet is the stable representative.
        assert_eq!(edges[0].target, first.id);
        assert_eq!(
            containerized_ids(&edges[0]).len(),
            2,
            "the subnet's edge stands for both containers on it"
        );
    }

    /// Reported against the Application view: a Docker host drew a runtime edge to every
    /// container except one. Both sat on the same bridge; the lower-addressed one won the single
    /// per-subnet edge, and because neither carried a compose project they grouped separately, so
    /// nothing reached the other. Collapsing to one representative is only sound where the subnet
    /// is itself a box every member elevates onto.
    #[test]
    fn every_container_gets_an_edge_where_subnets_are_not_containers() {
        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let runtime_id = Uuid::new_v4();

        let lan = subnet(network_id, "lan", 30, SubnetType::Lan);
        let shared = subnet(network_id, "bridge", 17, SubnetType::DockerBridge);

        let host_ip = ip(network_id, host_id, lan.id, Ipv4Addr::new(172, 30, 0, 1));
        // Deliberately in the order the reported data had them: the lower address is the one
        // that used to win, and the higher one is the container that vanished.
        let lower = ip(network_id, host_id, shared.id, Ipv4Addr::new(172, 17, 0, 2));
        let higher = ip(network_id, host_id, shared.id, Ipv4Addr::new(172, 17, 0, 3));

        let host = Host {
            id: host_id,
            base: HostBase {
                network_id,
                name: HostName::manual("docker-host".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let runtime = Service {
            id: runtime_id,
            base: ServiceBase {
                network_id,
                host_id,
                name: "Docker".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let containerized = |name: &str, ip_id: Uuid| Service {
            id: Uuid::new_v4(),
            base: ServiceBase {
                network_id,
                host_id,
                name: name.to_string(),
                virtualization_metadata: Some(ServiceVirtualization::Docker(
                    DockerVirtualization {
                        container_id: Some(name.to_string()),
                        container_name: Some(name.to_string()),
                        compose_project: None,
                    },
                )),
                virtualization_service_id: Some(runtime_id),
                bindings: vec![Binding::new_ip_address_serviceless(ip_id)],
                ..Default::default()
            },
            ..Default::default()
        };

        let hosts = vec![host];
        let ip_addresses = vec![host_ip.clone(), lower.clone(), higher.clone()];
        let subnets = vec![lan, shared];
        let services = vec![
            runtime,
            containerized("buildkit", lower.id),
            containerized("postgres", higher.id),
        ];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
            &services,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::Application,
        );

        let edges = EdgeBuilder::create_containerized_service_edges(&ctx, &by_application());

        let mut targets: Vec<Uuid> = edges.iter().map(|e| e.target).collect();
        targets.sort();
        let mut expected = vec![lower.id, higher.id];
        expected.sort();
        assert_eq!(
            targets, expected,
            "both containers need their own edge when no subnet box can stand for them"
        );

        // Where the subnet *is* a box, the collapsing still holds — one edge, lowest address.
        let collapsed = EdgeBuilder::create_containerized_service_edges(&ctx, &ungrouped());
        assert_eq!(collapsed.len(), 1);
        assert_eq!(collapsed[0].target, lower.id);
    }

    /// The set an edge stands for is scoped to the bridge subnet it reaches, not to everything
    /// the runtime hosts. Clicking the edge into one bridge box listed the whole host's
    /// containers, so a box holding one container claimed to hold all of them.
    #[test]
    fn each_edge_names_only_the_containers_on_its_own_subnet() {
        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        let lan = subnet(network_id, "lan", 30, SubnetType::Lan);
        let db_net = subnet(network_id, "db-net", 20, SubnetType::DockerBridge);
        let web_net = subnet(network_id, "web-net", 21, SubnetType::DockerBridge);
        let db_net_id = db_net.id;
        let web_net_id = web_net.id;

        let host_ip = ip(network_id, host_id, lan.id, Ipv4Addr::new(172, 30, 0, 5));
        let db_ip = ip(network_id, host_id, db_net.id, Ipv4Addr::new(172, 20, 0, 2));
        let web_ip = ip(
            network_id,
            host_id,
            web_net.id,
            Ipv4Addr::new(172, 21, 0, 2),
        );

        let host = Host {
            id: host_id,
            base: HostBase {
                name: HostName::manual("docker-host".to_string()),
                network_id,
                ..Default::default()
            },
            ..Default::default()
        };
        let runtime = Service {
            id: Uuid::new_v4(),
            base: ServiceBase {
                host_id,
                network_id,
                name: "Docker".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };
        let runtime_id = runtime.id;
        let container = |name: &str, ip_id: Uuid| Service {
            id: Uuid::new_v4(),
            base: ServiceBase {
                host_id,
                network_id,
                name: name.to_string(),
                virtualization_metadata: Some(ServiceVirtualization::Docker(
                    DockerVirtualization {
                        container_name: Some(name.to_string()),
                        container_id: Some(name.to_string()),
                        compose_project: None,
                    },
                )),
                virtualization_service_id: Some(runtime_id),
                bindings: vec![Binding::new_ip_address_serviceless(ip_id)],
                ..Default::default()
            },
            ..Default::default()
        };
        let postgres = container("postgres", db_ip.id);
        let nginx = container("nginx", web_ip.id);
        let postgres_id = postgres.id;
        let nginx_id = nginx.id;

        let hosts = vec![host];
        let ip_addresses = vec![host_ip.clone(), db_ip.clone(), web_ip.clone()];
        let subnets = vec![lan, db_net, web_net];
        let services = vec![runtime, postgres, nginx];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
            &services,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );

        let edges = EdgeBuilder::create_containerized_service_edges(&ctx, &ungrouped());
        let by_target: HashMap<Uuid, (Vec<Uuid>, Vec<Uuid>)> = edges
            .iter()
            .map(|e| (e.target, (subnet_ids(e), containerized_ids(e))))
            .collect();
        assert_eq!(
            by_target.get(&db_ip.id),
            Some(&(vec![db_net_id], vec![postgres_id]))
        );
        assert_eq!(
            by_target.get(&web_ip.id),
            Some(&(vec![web_net_id], vec![nginx_id]))
        );

        // Merged, the two boxes become one and its edge stands for everything inside it.
        let merged_edges = EdgeBuilder::create_containerized_service_edges(&ctx, &merged());
        assert_eq!(merged_edges.len(), 1);
        let sorted = |mut ids: Vec<Uuid>| {
            ids.sort();
            ids
        };
        assert_eq!(
            sorted(containerized_ids(&merged_edges[0])),
            sorted(vec![postgres_id, nginx_id])
        );
        assert_eq!(
            sorted(subnet_ids(&merged_edges[0])),
            sorted(vec![db_net_id, web_net_id]),
            "the merged edge reaches every bridge in the box"
        );
    }
}
