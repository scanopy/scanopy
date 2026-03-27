use itertools::Itertools;
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

use crate::server::{
    hosts::r#impl::base::Host,
    interfaces::r#impl::base::Interface,
    services::r#impl::base::Service,
    subnets::r#impl::types::SubnetType,
    topology::{
        service::{
            context::TopologyContext,
            planner::{
                anchor_planner::ChildAnchorPlanner,
                child_planner::ChildNodePlanner,
                utils::{NODE_PADDING, PlannerUtils, SUBNET_PADDING},
            },
        },
        types::{
            edges::Edge,
            layout::{Ixy, NodeLayout, SubnetLayout, Uxy},
            nodes::{Node, NodeType, SubnetChild},
        },
    },
};

pub struct SubnetLayoutPlanner {
    consolidated_docker_subnets: HashMap<Uuid, Vec<Uuid>>,
}

impl Default for SubnetLayoutPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl SubnetLayoutPlanner {
    pub fn new() -> Self {
        Self {
            consolidated_docker_subnets: HashMap::new(),
        }
    }

    pub fn get_consolidated_docker_subnets(&self) -> &HashMap<Uuid, Vec<Uuid>> {
        &self.consolidated_docker_subnets
    }

    /// Main entry point: calculate subnet layouts and create all child nodes
    pub fn create_subnet_child_nodes(
        &mut self,
        ctx: &TopologyContext,
        all_edges: &mut [Edge],
        group_docker_bridges_by_host: bool,
        docker_bridge_host_subnet_id_to_group_on: HashMap<Uuid, Uuid>,
    ) -> (HashMap<Uuid, SubnetLayout>, Vec<Node>) {
        let children_by_subnet = self.group_children_by_subnet(
            ctx,
            all_edges,
            group_docker_bridges_by_host,
            docker_bridge_host_subnet_id_to_group_on,
        );
        let mut child_nodes = Vec::new();

        let subnet_sizes: HashMap<Uuid, SubnetLayout> = children_by_subnet
            .iter()
            .map(|(subnet_id, children)| {
                let (size, infra_width) =
                    self.calculate_subnet_size(*subnet_id, children, ctx, &mut child_nodes);
                (*subnet_id, SubnetLayout { size, infra_width })
            })
            .collect();

        (subnet_sizes, child_nodes)
    }

    fn determine_subnet_child_header_text(
        &self,
        ctx: &TopologyContext,
        interface: &Interface,
        host: &Host,
        subnet_type: &SubnetType,
    ) -> Option<String> {
        // P1: Show virtualization provider, if any
        let host_interfaces = ctx.get_interfaces_for_host(host.id);
        if let Some(service) = ctx.get_host_is_virtualized_by(&host.id) {
            let virtualization_service_host = ctx.get_host_by_id(service.base.host_id);

            let host_interface_subnet_ids: Vec<Uuid> =
                host_interfaces.iter().map(|i| i.base.subnet_id).collect();
            let virtualization_service_interface_subnet_ids: Vec<Uuid> = service
                .base
                .bindings
                .iter()
                .filter_map(|b| ctx.get_interface_by_id(b.interface_id()))
                .map(|i| i.base.subnet_id)
                .collect();

            // Find shared subnets to determine which interface to use
            let host_interface_subnet_ids_hashset: HashSet<&Uuid> =
                host_interface_subnet_ids.iter().collect();
            let virtualization_service_interface_subnet_ids_hashset: HashSet<&Uuid> =
                virtualization_service_interface_subnet_ids.iter().collect();

            let intersection: Vec<&Uuid> = host_interface_subnet_ids_hashset
                .intersection(&virtualization_service_interface_subnet_ids_hashset)
                .cloned()
                .collect();

            let hide_docker_bridge_vm_header = *subnet_type == SubnetType::DockerBridge
                && ctx.options.request.hide_vm_title_on_docker_container;

            if !hide_docker_bridge_vm_header {
                // If they have at least one interface on a common subnet
                // Use the IP address from that interface in the header text
                match intersection.first() {
                    Some(first) => {
                        if let Some(interface) =
                            host_interfaces.iter().find(|i| i.base.subnet_id == **first)
                            && host_interface_subnet_ids
                                .iter()
                                .filter(|i| i == first)
                                .count()
                                == 1
                            && virtualization_service_interface_subnet_ids
                                .iter()
                                .filter(|i| i == first)
                                .count()
                                == 1
                        {
                            let on = virtualization_service_host
                                .map(|h| h.base.name.clone())
                                .unwrap_or(interface.base.ip_address.to_string());

                            if on == service.base.name {
                                return Some(format!("VM: {}", service.base.name));
                            } else {
                                return Some(format!("VM: {} on {}", service.base.name, on));
                            }
                        }
                        return Some(format!("VM: {}", service.base.name));
                    }
                    _ => return Some(format!("VM: {}", service.base.name)),
                }
            }
        }

        let host_has_name = host.base.name != "Unknown Device" && !host.base.name.is_empty();

        // P2: Show docker container header with host name
        if *subnet_type == SubnetType::DockerBridge {
            let header_text = if host_has_name {
                Some("Docker @ ".to_owned() + &host.base.name.clone())
            } else {
                // Generate a label from non-docker interface, if there is one
                host_interfaces
                    .iter()
                    .find(|i| {
                        ctx.get_subnet_from_interface_id(i.id)
                            .map(|s| s.base.subnet_type != SubnetType::DockerBridge)
                            .unwrap_or(false)
                    })
                    .map(|i| "Docker @ ".to_owned() + &i.base.ip_address.to_string())
            };

            return header_text;
        }

        // P3: Show host if it differs from the first service name + isn't shown via interface edges
        // and if it also isn't just the interface IP
        let host_services = ctx.get_services_for_host(host.id);
        let first_service_name_matches_host_name = match host_services.first() {
            Some(first_service) => first_service.base.name == host.base.name,
            None => false,
        };

        let host_name_is_interface_ip = interface.base.ip_address.to_string() == host.base.name;

        // Count of other interfaces that will actually have a node (ie services on that interface > 0)
        // so an interface edge will be created
        let interfaces_with_node: Vec<&&Interface> = host_interfaces
            .iter()
            .filter(|i| !ctx.get_services_bound_to_interface(i.id).is_empty())
            .collect();

        if !host_name_is_interface_ip
            && !first_service_name_matches_host_name
            && host_has_name
            && interfaces_with_node.len() < 2
        {
            return Some(host.base.name.clone());
        }

        None
    }

    /// Group host interfaces by subnet
    /// If group_docker_bridges_by_host is true, all DockerBridge interfaces for a given host
    /// are consolidated into one subnet
    fn group_children_by_subnet(
        &mut self,
        ctx: &TopologyContext,
        all_edges: &mut [Edge],
        group_docker_bridges_by_host: bool,
        docker_bridge_host_subnet_id_to_group_on: HashMap<Uuid, Uuid>,
    ) -> HashMap<Uuid, Vec<SubnetChild>> {
        let mut children_by_subnet: HashMap<Uuid, Vec<SubnetChild>> = HashMap::new();

        // Track DockerBridge interfaces by host (only used if grouping is enabled)
        // Map: (host_id, primary_subnet_id) -> Vec<subnet_id>)
        let mut docker_subnets_by_host: HashMap<(Uuid, Uuid), Vec<Uuid>> = HashMap::new();

        for interface in ctx.interfaces {
            let Some(host) = ctx.get_host_by_id(interface.base.host_id) else {
                continue;
            };
            let subnet = ctx.get_subnet_by_id(interface.base.subnet_id);
            if subnet
                .map(|s| s.base.subnet_type.exclude_from_topology())
                .unwrap_or(false)
            {
                continue;
            }
            let subnet_type = subnet.map(|s| s.base.subnet_type).unwrap_or_default();
            let services = ctx.services;

            let interface_bound_services: Vec<&Service> = services
                .iter()
                .filter(|s| {
                    // Services with a binding to the interface
                    s.base.bindings.iter().any(|b| match b.interface_id() {
                        // Service is bound to interface if ID matches
                        Some(binding_interface_id) if binding_interface_id == interface.id => true,
                        // If there's no interface, it's an L4 binding bound to all interfaces
                        None => true,
                        _ => false,
                    })
                })
                .collect();

            // Update source/target handles for edges
            let edges = ChildAnchorPlanner::plan_anchors(interface.id, all_edges, ctx);

            let header_text =
                self.determine_subnet_child_header_text(ctx, interface, host, &subnet_type);

            let child = SubnetChild {
                id: interface.id,
                host_id: host.id,
                size: Uxy::subnet_child_size_from_service_count(
                    &interface_bound_services,
                    interface.id,
                    header_text.is_some(),
                    ctx.options.request.hide_ports,
                ),
                header: header_text,
                interface_id: Some(interface.id),
                edges,
            };

            // Special handling for DockerBridge (only if grouping is enabled)
            if group_docker_bridges_by_host && matches!(subnet_type, SubnetType::DockerBridge) {
                if let Some(subnet_grouping_id) =
                    docker_bridge_host_subnet_id_to_group_on.get(&host.id)
                {
                    docker_subnets_by_host
                        .entry((host.id, *subnet_grouping_id))
                        .or_default()
                        .push(interface.base.subnet_id);

                    children_by_subnet
                        .entry(*subnet_grouping_id)
                        .or_default()
                        .push(child);
                }
            } else {
                children_by_subnet
                    .entry(interface.base.subnet_id)
                    .or_default()
                    .push(child);
            }
        }

        // Consolidate all DockerBridge children into their primary subnet (only if grouping is enabled)
        if group_docker_bridges_by_host {
            for ((_, grouping_id), mut subnet_ids) in docker_subnets_by_host {
                // Remove duplicates and sort for consistency
                subnet_ids.sort();
                subnet_ids.dedup();

                // Store the consolidation mapping
                self.consolidated_docker_subnets
                    .insert(grouping_id, subnet_ids);
            }
        }

        children_by_subnet
    }

    /// Calculate the size and layout of a subnet, creating child nodes
    fn calculate_subnet_size(
        &mut self,
        subnet_id: Uuid,
        children: &[SubnetChild],
        ctx: &TopologyContext,
        child_nodes: &mut Vec<Node>,
    ) -> (Uxy, usize) {
        // Separate infrastructure from regular nodes
        let (infrastructure_children, regular_children) =
            if let Some(subnet) = ctx.get_subnet_by_id(subnet_id) {
                let infrastructure_interface_ids = ctx.get_interfaces_with_infra_service(subnet);

                let (infrastructure, regular): (Vec<SubnetChild>, Vec<SubnetChild>) = children
                    .iter()
                    .sorted_by_key(|c| c.size.y)
                    .cloned()
                    .partition(|c| infrastructure_interface_ids.contains(&c.interface_id));

                (infrastructure, regular)
            } else {
                (Vec::new(), children.to_vec())
            };

        // Calculate regular nodes layout using coordinate-based system
        let (regular_child_positions, regular_grid_size) = if !regular_children.is_empty() {
            let positions = ChildNodePlanner::calculate_anchor_based_positions(
                &regular_children,
                &NODE_PADDING,
                ctx,
            );

            let container_size =
                PlannerUtils::calculate_container_size_from_layouts(&positions, &NODE_PADDING);

            (positions, container_size)
        } else {
            // Return 0 size when no regular children
            (HashMap::new(), Uxy { x: 0, y: 0 })
        };

        // Calculate infrastructure nodes layout using coordinate-based system
        let infra_cols = if infrastructure_children.is_empty() {
            0
        } else {
            // Calculate infrastructure nodes layout
            let positions = ChildNodePlanner::calculate_anchor_based_positions(
                &infrastructure_children,
                &NODE_PADDING,
                ctx,
            );

            // Calculate how many "columns" of infra nodes we have
            let mut x_positions: Vec<isize> = positions.values().map(|l| l.position.x).collect();
            x_positions.sort_unstable();
            x_positions.dedup();
            x_positions.len()
        };

        let (infra_child_positions, infra_grid_size) = if !infrastructure_children.is_empty() {
            let positions = ChildNodePlanner::calculate_anchor_based_positions(
                &infrastructure_children,
                &NODE_PADDING,
                ctx,
            );

            let container_size =
                PlannerUtils::calculate_container_size_from_layouts(&positions, &NODE_PADDING);

            (positions, container_size)
        } else {
            (HashMap::new(), Uxy { x: 0, y: 0 })
        };

        // Create infrastructure nodes (using HashMap lookup instead of grid iteration)
        for child in infrastructure_children.iter() {
            if let Some(layout) = infra_child_positions.get(&child.id) {
                child_nodes.push(Node {
                    id: child.id,
                    node_type: NodeType::InterfaceNode {
                        subnet_id,
                        interface_id: child.interface_id,
                        host_id: child.host_id,
                        is_infra: true,
                    },
                    position: layout.position,
                    size: child.size,
                    header: child.header.clone(),
                });
            }
        }

        // Create regular nodes (offset by infrastructure width, using HashMap lookup)
        for child in regular_children.iter() {
            if let Some(layout) = regular_child_positions.get(&child.id) {
                let node_position = Ixy {
                    x: layout.position.x
                        + if infra_cols > 0 {
                            infra_grid_size.x as isize
                        } else {
                            0
                        },
                    y: layout.position.y,
                };

                child_nodes.push(Node {
                    id: child.id,
                    node_type: NodeType::InterfaceNode {
                        subnet_id,
                        interface_id: child.interface_id,
                        host_id: child.host_id,
                        is_infra: false,
                    },
                    position: node_position,
                    size: child.size,
                    header: child.header.clone(),
                });
            }
        }

        let total_size = Uxy {
            x: regular_grid_size.x + infra_grid_size.x,
            y: regular_grid_size.y.max(infra_grid_size.y),
        };

        (total_size, infra_grid_size.x)
    }

    /// Create subnet container nodes with calculated positions
    pub fn create_subnet_nodes(
        &self,
        ctx: &TopologyContext,
        layouts: &HashMap<Uuid, SubnetLayout>,
    ) -> Vec<Node> {
        let subnet_grid_positions = self.calculate_subnet_grid_positions_by_layer(ctx, layouts);
        let (positions, _) =
            PlannerUtils::calculate_container_size(subnet_grid_positions, &SUBNET_PADDING);

        layouts
            .iter()
            .filter_map(|(subnet_id, layout)| {
                if let Some(position) = positions.get(subnet_id) {
                    if let Some(consolidated_subnet_ids) =
                        self.consolidated_docker_subnets.get(subnet_id)
                    {
                        let header = "Docker Bridge: (".to_owned()
                            + &ctx
                                .subnets
                                .iter()
                                .filter(|s| consolidated_subnet_ids.contains(&s.id))
                                .map(|s| s.base.cidr.to_string())
                                .join(", ")
                            + ")";

                        return Some(Node {
                            id: *subnet_id,
                            node_type: NodeType::SubnetNode {
                                infra_width: layout.infra_width,
                            },
                            position: *position,
                            size: layout.size,
                            header: Some(header),
                        });
                    }

                    return Some(Node {
                        id: *subnet_id,
                        node_type: NodeType::SubnetNode {
                            infra_width: layout.infra_width,
                        },
                        position: *position,
                        size: layout.size,
                        header: None,
                    });
                }
                None
            })
            .collect()
    }

    /// Calculate positions of subnets given layer values
    fn calculate_subnet_grid_positions_by_layer(
        &self,
        ctx: &TopologyContext,
        layouts: &HashMap<Uuid, SubnetLayout>,
    ) -> Vec<Vec<(Uuid, NodeLayout)>> {
        let sorted: Vec<_> = ctx
            .subnets
            .iter()
            .sorted_by_key(|s| {
                (
                    s.base.subnet_type.vertical_order(),
                    s.base.subnet_type.horizontal_order(),
                    s.base.name.clone(),
                )
            })
            .filter_map(|s| layouts.get(&s.id).map(|layout| (s, layout)))
            .collect();

        let mut subnets_by_layer: BTreeMap<usize, Vec<(&Uuid, &SubnetLayout)>> = BTreeMap::new();
        for (subnet, layout) in sorted {
            subnets_by_layer
                .entry(subnet.base.subnet_type.vertical_order())
                .or_default()
                .push((&subnet.id, layout));
        }

        subnets_by_layer
            .into_values()
            .map(|row| {
                row.into_iter()
                    .map(|(id, layout)| {
                        (
                            *id,
                            NodeLayout {
                                size: layout.size,
                                position: Ixy { x: 0, y: 0 },
                            },
                        )
                    })
                    .collect()
            })
            .collect()
    }
}
