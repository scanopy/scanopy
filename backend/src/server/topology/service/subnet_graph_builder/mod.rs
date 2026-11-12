use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use strum::IntoDiscriminant;
use uuid::Uuid;

use crate::server::{
    hosts::r#impl::{base::Host, device_type::DeviceType, name_ladder::HostNameRung},
    ip_addresses::r#impl::base::IPAddress,
    services::r#impl::{base::Service, definitions::ServiceDefinitionExt},
    shared::{entities::EntityDiscriminants, types::metadata::HasId},
    subnets::r#impl::types::{SubnetType, SubnetTypeDiscriminants},
    topology::{
        service::{
            context::TopologyContext,
            element_rules::{
                ElementMatchData, TaggableLookups, apply_element_rules, apply_stack_runtime_logos,
                resolve_element_tag_ids,
            },
        },
        types::{
            grouping::{ElementRule, GroupingConfig},
            nodes::{ContainerType, ElementEntityType, Node, NodeType},
        },
    },
};

/// Builder-internal struct for grouping IP address children by subnet.
#[derive(Debug, Clone)]
struct SubnetChildData {
    pub id: Uuid,
    pub header: Option<String>,
    pub host_id: Uuid,
    pub ip_address_id: Option<Uuid>,
}

pub struct SubnetGraphBuilder {
    consolidated_container_bridge_subnets: HashMap<Uuid, Vec<Uuid>>,
}

impl Default for SubnetGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubnetGraphBuilder {
    pub fn new() -> Self {
        Self {
            consolidated_container_bridge_subnets: HashMap::new(),
        }
    }

    /// Compute which subnet ID each host's container bridges should be grouped under.
    /// Keyed by (host_id, bridge runtime) so a host's Docker bridges and Podman
    /// bridges merge into separate containers. Value is the primary (first) bridge
    /// subnet found for that (host, runtime).
    fn compute_container_bridge_grouping(
        ctx: &TopologyContext,
    ) -> HashMap<(Uuid, SubnetTypeDiscriminants), Uuid> {
        let mut mapping: HashMap<(Uuid, SubnetTypeDiscriminants), Uuid> = HashMap::new();
        for ip_address in ctx.ip_addresses {
            let Some(subnet) = ctx.get_subnet_by_id(ip_address.base.subnet_id) else {
                continue;
            };
            if subnet.base.subnet_type().is_container_bridge() {
                mapping
                    .entry((
                        ip_address.base.host_id,
                        subnet.base.subnet_type().discriminant(),
                    ))
                    .or_insert(subnet.id);
            }
        }
        mapping
    }

    /// Main entry point: group children by subnet and create all child nodes
    pub fn create_subnet_child_nodes(
        &mut self,
        ctx: &TopologyContext,
        grouping: &GroupingConfig,
    ) -> (HashSet<Uuid>, Vec<Node>) {
        let container_bridge_grouping = if grouping.should_group_container_bridges() {
            Self::compute_container_bridge_grouping(ctx)
        } else {
            HashMap::new()
        };
        let children_by_subnet =
            self.group_children_by_subnet(ctx, grouping, container_bridge_grouping);
        let mut child_nodes = Vec::new();

        let subnet_ids: HashSet<Uuid> = children_by_subnet
            .iter()
            .map(|(subnet_id, children)| {
                self.create_child_nodes(*subnet_id, children, ctx, &mut child_nodes);
                *subnet_id
            })
            .collect();

        (subnet_ids, child_nodes)
    }

    fn determine_subnet_child_header_text(
        &self,
        ctx: &TopologyContext,
        ip_address: &IPAddress,
        host: &Host,
        subnet_type: &SubnetType,
    ) -> Option<String> {
        let host_interfaces = ctx.get_ip_addresses_for_host(host.id);
        // The host's title, from the display ladder: a name, or an identifier such as its
        // hostname. A host titled only by its address counts as having no name here, because
        // every branch below either shows the address itself or suppresses a header that would
        // repeat it.
        let title = host
            .resolved_name(host_interfaces.iter().copied())
            .filter(|(_, rung)| *rung != HostNameRung::Address)
            .map(|(value, _)| value);
        let host_has_name = title.is_some();
        let title = title.unwrap_or_default();

        // P1: container-bridge interfaces — always show "<Runtime> @", never VM header
        if let Some(runtime) = subnet_type.container_runtime_label() {
            let header_text = if host_has_name {
                Some(format!("{runtime} @ {title}"))
            } else {
                // Generate a label from a non-container-bridge ip_address, if there is one
                host_interfaces
                    .iter()
                    .find(|i| {
                        ctx.get_subnet_from_ip_address_id(i.id)
                            .map(|s| !s.base.subnet_type().is_container_bridge())
                            .unwrap_or(false)
                    })
                    .map(|i| format!("{runtime} @ {}", i.base.ip_address))
            };

            return header_text;
        }

        // P2: Virtualized hosts — show the VM's own hostname
        // (VM status is indicated via colored text in the frontend)
        if ctx.get_host_is_virtualized_by(&host.id).is_some() && host_has_name {
            return Some(title);
        }

        // P3: Show host if it differs from the first service name + isn't shown via interface edges
        // and if it also isn't just the interface IP
        let host_services = ctx.get_services_for_host(host.id);
        let first_service_name_matches_host_name = match host_services.first() {
            Some(first_service) => first_service.base.name == title,
            None => false,
        };

        let host_name_is_interface_ip = ip_address.base.ip_address.to_string() == title;

        // Count of other ip_addresses that will actually have a node (ie services on that interface > 0)
        // so an interface edge will be created
        let ip_addresses_with_node: Vec<&&IPAddress> = host_interfaces
            .iter()
            .filter(|i| !ctx.get_services_bound_to_ip_address(i.id).is_empty())
            .collect();

        if !host_name_is_interface_ip
            && !first_service_name_matches_host_name
            && host_has_name
            && ip_addresses_with_node.len() < 2
        {
            return Some(title);
        }

        None
    }

    /// Group host ip_addresses by subnet
    /// If container-bridge grouping is enabled, a host's container-bridge ip_addresses
    /// are consolidated into one subnet per (host, runtime).
    fn group_children_by_subnet(
        &mut self,
        ctx: &TopologyContext,
        grouping: &GroupingConfig,
        container_bridge_grouping: HashMap<(Uuid, SubnetTypeDiscriminants), Uuid>,
    ) -> HashMap<Uuid, Vec<SubnetChildData>> {
        let mut children_by_subnet: HashMap<Uuid, Vec<SubnetChildData>> = HashMap::new();

        // Track container-bridge ip_addresses by (host, primary subnet) (only used if
        // grouping is enabled). Map: (host_id, primary_subnet_id) -> Vec<subnet_id>
        let mut container_bridge_subnets_by_host: HashMap<(Uuid, Uuid), Vec<Uuid>> = HashMap::new();

        for ip_address in ctx.ip_addresses {
            let Some(host) = ctx.get_host_by_id(ip_address.base.host_id) else {
                continue;
            };
            let subnet = ctx.get_subnet_by_id(ip_address.base.subnet_id);
            if subnet
                .map(|s| s.base.subnet_type().exclude_from_topology())
                .unwrap_or(false)
            {
                continue;
            }
            let subnet_type = subnet.map(|s| s.base.subnet_type()).unwrap_or_default();

            let header_text =
                self.determine_subnet_child_header_text(ctx, ip_address, host, &subnet_type);

            let child = SubnetChildData {
                id: ip_address.id,
                host_id: host.id,
                header: header_text,
                ip_address_id: Some(ip_address.id),
            };

            // Special handling for container bridges (only if grouping is enabled):
            // consolidate under the (host, runtime) primary subnet.
            if grouping.should_group_container_bridges() && subnet_type.is_container_bridge() {
                if let Some(subnet_grouping_id) =
                    container_bridge_grouping.get(&(host.id, subnet_type.discriminant()))
                {
                    container_bridge_subnets_by_host
                        .entry((host.id, *subnet_grouping_id))
                        .or_default()
                        .push(ip_address.base.subnet_id);

                    children_by_subnet
                        .entry(*subnet_grouping_id)
                        .or_default()
                        .push(child);
                }
            } else {
                children_by_subnet
                    .entry(ip_address.base.subnet_id)
                    .or_default()
                    .push(child);
            }
        }

        // Consolidate container-bridge children into their primary subnet (only if grouping is enabled)
        if grouping.should_group_container_bridges() {
            for ((_, grouping_id), mut subnet_ids) in container_bridge_subnets_by_host {
                // Remove duplicates and sort for consistency
                subnet_ids.sort();
                subnet_ids.dedup();

                // Store the consolidation mapping
                self.consolidated_container_bridge_subnets
                    .insert(grouping_id, subnet_ids);
            }
        }

        children_by_subnet
    }

    /// Create child (element) nodes for a subnet
    fn create_child_nodes(
        &mut self,
        subnet_id: Uuid,
        children: &[SubnetChildData],
        ctx: &TopologyContext,
        child_nodes: &mut Vec<Node>,
    ) {
        // Create element nodes for all children
        // Positions are zeroed — the frontend computes layout via elkjs
        for child in children.iter() {
            let mut node = Node::element(
                child.id,
                subnet_id,
                child.host_id,
                ElementEntityType::IPAddress {
                    subnet_id,
                    ip_address_id: child.ip_address_id,
                },
            );
            node.header = child.header.clone();
            child_nodes.push(node);
        }

        // Create nested group containers for ByServiceCategory and ByTag rules
        self.create_nested_group_containers(subnet_id, children, ctx, child_nodes);
    }

    /// Create nested Container nodes for ByServiceCategory and ByTag grouping rules (ClientSide mode only).
    /// First-match-wins: nodes already claimed by an earlier rule are not reassigned.
    fn create_nested_group_containers(
        &self,
        subnet_id: Uuid,
        children: &[SubnetChildData],
        ctx: &TopologyContext,
        child_nodes: &mut Vec<Node>,
    ) {
        let grouping = GroupingConfig::from_request_options(&ctx.options.request, ctx.view);
        let children_by_id: HashMap<Uuid, &SubnetChildData> =
            children.iter().map(|c| (c.id, c)).collect();
        let host_lookup: HashMap<Uuid, &Host> = ctx.hosts.iter().map(|h| (h.id, h)).collect();
        let tag_lookups = TaggableLookups {
            hosts: Some(&host_lookup),
            services: None,
            subnets: None,
        };
        // Resolving device types walks every service, so it runs only when a rule groups by them.
        let device_types: HashMap<Uuid, DeviceType> = if grouping
            .element_rules
            .iter()
            .any(|r| matches!(r.rule, ElementRule::ByDeviceType))
        {
            let mut services_by_host: HashMap<Uuid, Vec<&Service>> = HashMap::new();
            for service in ctx.services {
                services_by_host
                    .entry(service.base.host_id)
                    .or_default()
                    .push(service);
            }
            let mut ip_addresses_by_host: HashMap<Uuid, Vec<&IPAddress>> = HashMap::new();
            for ip_address in ctx.ip_addresses {
                ip_addresses_by_host
                    .entry(ip_address.base.host_id)
                    .or_default()
                    .push(ip_address);
            }
            ctx.hosts
                .iter()
                .map(|h| {
                    let services = services_by_host.get(&h.id).into_iter().flatten().copied();
                    let ip_addresses = ip_addresses_by_host
                        .get(&h.id)
                        .into_iter()
                        .flatten()
                        .copied();
                    (h.id, h.device_type(services, ip_addresses))
                })
                .collect()
        } else {
            HashMap::new()
        };

        // Resolve the deployment group (compose project) only for elements
        // inside container subnets. LAN ip_addresses shouldn't be grouped by stack.
        let is_container_subnet = ctx
            .get_subnet_by_id(subnet_id)
            .map(|s| s.base.subnet_type().is_container_network())
            .unwrap_or(false);

        let _ = apply_element_rules(
            child_nodes,
            &grouping.element_rules,
            |node| {
                let child = children_by_id.get(&node.id)?;
                let host_services = ctx.get_services_for_host(child.host_id);
                let categories = host_services
                    .iter()
                    .map(|s| s.base.service_definition.category())
                    .collect();
                let tag_ids = resolve_element_tag_ids(
                    EntityDiscriminants::IPAddress,
                    child.host_id,
                    &tag_lookups,
                );
                // An address on a container subnet belongs to a container, and its host's device type
                // (nearly always Server) says nothing about it. Leaving it out keeps it free for
                // ByStack, which groups only container subnets, the inverse of this check.
                let device_type = if is_container_subnet {
                    None
                } else {
                    device_types.get(&child.host_id).copied()
                };
                let deployment_group = if !is_container_subnet {
                    None
                } else {
                    let mut projects: HashSet<&str> = HashSet::new();
                    let services_iter: Box<dyn Iterator<Item = _>> =
                        if let Some(iface_id) = child.ip_address_id {
                            // Interface-specific: only services bound to this interface
                            Box::new(host_services.iter().copied().filter(move |s| {
                                s.base
                                    .bindings
                                    .iter()
                                    .any(|b| b.ip_address_id() == Some(iface_id))
                            }))
                        } else {
                            // Fallback: all services on the host
                            Box::new(host_services.iter().copied())
                        };
                    // Runtime-agnostic: any container runtime (Docker, Podman, …) that
                    // carries a compose project participates in stack grouping.
                    for service in services_iter {
                        if let Some(project) = service
                            .base
                            .virtualization_metadata
                            .as_ref()
                            .and_then(|v| v.compose_project())
                        {
                            projects.insert(project);
                        }
                    }
                    if projects.len() == 1 {
                        projects.into_iter().next().map(String::from)
                    } else {
                        None
                    }
                };
                Some(ElementMatchData {
                    categories,
                    tag_ids,
                    element_entity: EntityDiscriminants::IPAddress,
                    virtualizer_service_id: None,
                    virtualizer_role: None,
                    deployment_group,
                    native_vlan_id: None,
                    vlan_number: None,
                    vlan_name: None,
                    is_trunk_port: false,
                    oper_status: None,
                    device_type,
                })
            },
            None,
            None,
        );

        // Post-process: stamp each Stack subcontainer's logo from the runtime of
        // the services in that deployment group (Docker, Podman, …).
        apply_stack_runtime_logos(child_nodes, ctx.services);
    }

    /// Service definition whose logo marks a container-bridge subnet container: the
    /// definition of the runtime service each bridge reports through
    /// `virtualization_service_id`. A merged container gets it only when every merged
    /// bridge resolves to the same definition; a bridge with no runtime service, a
    /// service missing from the context, or a definition without a logo yields `None`,
    /// leaving the subnet type's icon.
    fn bridge_runtime_definition(
        &self,
        ctx: &TopologyContext,
        subnet_id: Uuid,
    ) -> Option<&'static str> {
        let members = self
            .consolidated_container_bridge_subnets
            .get(&subnet_id)
            .map(Vec::as_slice)
            .unwrap_or(std::slice::from_ref(&subnet_id));
        let mut definitions = members.iter().map(|member_id| {
            let subnet = ctx.get_subnet_by_id(*member_id)?;
            if !subnet.base.subnet_type().is_container_bridge() {
                return None;
            }
            let service = ctx.get_service_by_id(subnet.base.virtualization_service_id?)?;
            let definition = &service.base.service_definition;
            definition.has_logo().then(|| definition.id())
        });
        let first = definitions.next()??;
        definitions.all(|d| d == Some(first)).then_some(first)
    }

    /// Create subnet container nodes
    /// Positions and sizes are zeroed — the frontend computes layout via elkjs
    pub fn create_subnet_nodes(
        &self,
        ctx: &TopologyContext,
        subnet_ids: &HashSet<Uuid>,
    ) -> Vec<Node> {
        subnet_ids
            .iter()
            .map(|subnet_id| {
                // Build display header from subnet metadata
                let header =
                    if let Some(cids) = self.consolidated_container_bridge_subnets.get(subnet_id) {
                        // Consolidation is per (host, runtime), so the merged subnets
                        // share a runtime — derive the label from any of them.
                        let runtime = ctx
                            .subnets
                            .iter()
                            .find(|s| cids.contains(&s.id))
                            .and_then(|s| s.base.subnet_type().container_runtime_label())
                            .unwrap_or("Container");
                        Some(format!(
                            "{runtime} Bridge: ({})",
                            ctx.subnets
                                .iter()
                                .filter(|s| cids.contains(&s.id))
                                .map(|s| s.base.cidr.to_string())
                                .join(", ")
                        ))
                    } else if let Some(subnet) = ctx.subnets.iter().find(|s| s.id == *subnet_id) {
                        use crate::server::shared::types::metadata::TypeMetadataProvider;
                        let type_name = subnet.base.subnet_type().name();
                        let cidr = subnet.base.cidr.to_string();
                        let show_label = subnet.base.subnet_type().show_label();
                        let name_or_type = if subnet.base.name() != cidr {
                            subnet.base.name().to_string()
                        } else if show_label {
                            type_name.to_string()
                        } else {
                            String::new()
                        };
                        Some(if name_or_type.is_empty() {
                            cidr
                        } else {
                            format!("{}: {}", name_or_type, cidr)
                        })
                    } else {
                        None
                    };

                // Container-runtime edges target the container box rather than each
                // container's individual IP address. That holds for a merged bridge group,
                // and equally for a lone bridge subnet when merging is switched off —
                // otherwise un-merging trades one edge per host for one per container.
                let will_accept_edges = self
                    .consolidated_container_bridge_subnets
                    .contains_key(subnet_id)
                    || ctx
                        .get_subnet_by_id(*subnet_id)
                        .is_some_and(|s| s.base.subnet_type().is_container_bridge());
                Node {
                    id: *subnet_id,
                    node_type: NodeType::Container {
                        container_type: ContainerType::Subnet,
                        parent_container_id: None,
                        entity_id: Some(*subnet_id),
                        icon: None,
                        color: None,
                        associated_service_definition: self
                            .bridge_runtime_definition(ctx, *subnet_id)
                            .map(str::to_string),
                        element_rule_id: None,
                        will_accept_edges,
                    },
                    position: Default::default(),
                    size: Default::default(),
                    header,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
