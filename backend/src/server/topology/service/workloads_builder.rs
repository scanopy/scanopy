use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{
    context::TopologyContext,
    edge_builder::EdgeBuilder,
    element_rules::{
        ElementMatchData, InlineContext, TaggableLookups, apply_element_rules,
        resolve_element_tag_ids,
    },
    view::ViewBuilder,
};
use crate::server::{
    dependencies::r#impl::{base::DependencyMembers, types::DependencyType},
    interfaces::r#impl::base::Neighbor,
    services::r#impl::{categories::ServiceCategory, definitions::ServiceDefinitionExt},
    shared::entities::EntityDiscriminants,
    topology::types::{
        edges::{DiscoveryProtocol, Edge, EdgeHandle, EdgeType, EdgeViewConfig},
        grouping::{GroupingConfig, PlacementDecision},
        nodes::{ContainerType, ElementEntityType, Node, NodeType},
    },
};

pub struct WorkloadsBuilder;

impl WorkloadsBuilder {
    /// Generate a deterministic container UUID from host_id for the Workloads view.
    fn container_id_for_host(host_id: Uuid) -> Uuid {
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("workloads:host:{host_id}").as_bytes(),
        )
    }

    /// Build a map of virtualizer_service_id → service_definition_name for subcontainer titles.
    fn build_virtualizer_titles(ctx: &TopologyContext) -> HashMap<Uuid, String> {
        ctx.services
            .iter()
            .filter(|s| s.base.service_definition.virtualization_role().is_some())
            .map(|s| (s.id, s.base.service_definition.name().to_string()))
            .collect()
    }
}

impl ViewBuilder for WorkloadsBuilder {
    fn build(&self, ctx: &TopologyContext, grouping: &GroupingConfig) -> (Vec<Node>, Vec<Edge>) {
        let mut nodes = Vec::new();

        // --- Phase 1: Build lookup maps (provider-agnostic) ---

        // virtualizer_service_id → managed guest host_ids (VMs, LXC containers, container hosts)
        let mut virt_to_guest_hosts: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for host in ctx.hosts {
            if let Some(svc_id) = host.base.virtualization_service_id {
                virt_to_guest_hosts.entry(svc_id).or_default().push(host.id);
            }
        }

        // virtualizer_service_id → managed container service_ids
        let mut virt_to_container_svcs: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for service in ctx.services {
            if let Some(svc_id) = service.base.virtualization_service_id {
                virt_to_container_svcs
                    .entry(svc_id)
                    .or_default()
                    .push(service.id);
            }
        }

        // Services that are virtualizers (Docker daemon, Proxmox service, etc.)
        let virtualizer_service_ids: HashSet<Uuid> = ctx
            .services
            .iter()
            .filter(|s| s.base.service_definition.virtualization_role().is_some())
            .map(|s| s.id)
            .collect();

        // Services that are managed by a virtualizer (containers)
        let managed_service_ids: HashSet<Uuid> = virt_to_container_svcs
            .values()
            .flat_map(|ids| ids.iter().copied())
            .collect();

        // Host lookup for resolving names and data
        let host_lookup: HashMap<Uuid, &crate::server::hosts::r#impl::base::Host> =
            ctx.hosts.iter().map(|h| (h.id, h)).collect();

        // Service lookup
        let service_lookup: HashMap<Uuid, &crate::server::services::r#impl::base::Service> =
            ctx.services.iter().map(|s| (s.id, s)).collect();

        // --- Phase 2: Prepare inline context for element rules ---
        // InlineContext provides host/service data for InlineOn placement decisions
        // (e.g., ByHypervisor inlines VM services, ByContainerRuntime inlines Docker-on-VM).
        let inline_ctx = InlineContext {
            hosts: &host_lookup,
            service_lookup: &service_lookup,
            virt_to_container_svcs: &virt_to_container_svcs,
            virt_to_guest_hosts: &virt_to_guest_hosts,
        };

        // --- Phase 3: Create Host containers for hosts that are not guests ---

        for host in ctx.hosts {
            // Guests (VMs, LXC containers, container hosts) are elements only, never containers
            if host.base.virtualization_service_id.is_some() {
                continue;
            }

            let container_id = Self::container_id_for_host(host.id);
            nodes.push(Node {
                id: container_id,
                node_type: NodeType::Container {
                    container_type: ContainerType::Host,
                    parent_container_id: None,
                    entity_id: Some(host.id),
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

        // --- Phase 4: Create workload elements ---

        // The host container an element on `host_id` starts in: the host's own, or for a guest
        // the container of the first host up its virtualization chain that is not a guest. A
        // container on a Docker that runs in a VM starts in the VM's hypervisor's container;
        // apply_element_rules then inlines it on the VM. `None` when the chain leaves the graph.
        let start_container = |host_id: Uuid| -> Option<Uuid> {
            let mut current = host_id;
            // Bounded by the host count, so a virtualization cycle cannot loop forever.
            for _ in 0..=host_lookup.len() {
                match host_lookup.get(&current)?.base.virtualization_service_id {
                    None => return Some(Self::container_id_for_host(current)),
                    Some(virt_svc_id) => current = service_lookup.get(&virt_svc_id)?.base.host_id,
                }
            }
            None
        };

        // 4a: Guest host elements (VMs, LXC containers, container hosts) — placed in their
        // virtualizer's host container
        for (virt_svc_id, guest_host_ids) in &virt_to_guest_hosts {
            let Some(virt_svc) = service_lookup.get(virt_svc_id) else {
                continue;
            };
            let Some(container_id) = start_container(virt_svc.base.host_id) else {
                continue;
            };

            for &guest_host_id in guest_host_ids {
                let Some(guest_host) = host_lookup.get(&guest_host_id) else {
                    continue;
                };
                let mut node = Node::element(
                    guest_host_id,
                    container_id,
                    guest_host_id,
                    ElementEntityType::Host {},
                );
                node.header = ctx.host_container_header(guest_host);
                nodes.push(node);
            }
        }

        // 4b: Container elements — placed in their host's container.
        // For Docker on VMs, elements go in the hypervisor's host container temporarily;
        // apply_element_rules will produce InlineOn decisions to remove them.
        for (virt_svc_id, container_svc_ids) in &virt_to_container_svcs {
            let Some(virt_svc) = service_lookup.get(virt_svc_id) else {
                continue;
            };
            let Some(container_id) = start_container(virt_svc.base.host_id) else {
                continue;
            };

            for &svc_id in container_svc_ids {
                let Some(svc) = service_lookup.get(&svc_id) else {
                    continue;
                };
                let mut node = Node::element(
                    svc_id,
                    container_id,
                    svc.base.host_id,
                    ElementEntityType::Service {},
                );
                node.header = Some(svc.base.name.clone());
                nodes.push(node);
            }
        }

        // 4c: Remaining services — not a virtualizer, not managed by one.
        // All services get elements here (including VM services); apply_element_rules
        // will produce InlineOn decisions for services that shouldn't have elements,
        // and they'll be removed in Phase 5.5.
        for service in ctx.services {
            if virtualizer_service_ids.contains(&service.id)
                || managed_service_ids.contains(&service.id)
            {
                continue;
            }

            // Skip OpenPorts services — irrelevant noise in Workloads view
            if service.base.service_definition.category() == ServiceCategory::OpenPorts {
                continue;
            }

            // Guest hosts have no container of their own, so their services start in the
            // hypervisor's host container; apply_element_rules will InlineOn these. Services on
            // hosts not in the graph are skipped.
            let Some(container_id) = start_container(service.base.host_id) else {
                continue;
            };

            let mut node = Node::element(
                service.id,
                container_id,
                service.base.host_id,
                ElementEntityType::Service {},
            );
            node.header = Some(service.base.name.clone());
            nodes.push(node);
        }

        // --- Phase 5: Apply element rules ---
        // Unified: produces PlaceInContainer, BecomeSubcontainer, InlineOn, and Element
        // decisions for all entities via exhaustive ElementRule matching.

        let virtualizer_titles = Self::build_virtualizer_titles(ctx);
        let tag_lookups = TaggableLookups {
            hosts: Some(&host_lookup),
            services: Some(&service_lookup),
            subnets: None,
        };

        // The role of the service virtualizing an element decides which virtualizer rule takes it.
        let virtualizer_role = |virt_svc_id: Option<Uuid>| {
            service_lookup
                .get(&virt_svc_id?)?
                .base
                .service_definition
                .virtualization_role()
        };

        let placements = apply_element_rules(
            &mut nodes,
            &grouping.element_rules,
            |node| match &node.node_type {
                NodeType::Element {
                    element: ElementEntityType::Host {},
                    ..
                } => {
                    let host = host_lookup.get(&node.id)?;
                    let virtualizer_service_id = host.base.virtualization_service_id;
                    let tag_ids =
                        resolve_element_tag_ids(EntityDiscriminants::Host, node.id, &tag_lookups);
                    Some(ElementMatchData {
                        categories: HashSet::new(),
                        tag_ids,
                        element_entity: EntityDiscriminants::Host,
                        virtualizer_service_id,
                        virtualizer_role: virtualizer_role(virtualizer_service_id),
                        deployment_group: None,
                        native_vlan_id: None,
                        vlan_number: None,
                        vlan_name: None,
                        is_trunk_port: false,
                        oper_status: None,
                    })
                }
                NodeType::Element {
                    element: ElementEntityType::Service {},
                    ..
                } => {
                    let svc = service_lookup.get(&node.id)?;
                    let virtualizer_service_id = svc.base.virtualization_service_id;
                    let tag_ids = resolve_element_tag_ids(
                        EntityDiscriminants::Service,
                        node.id,
                        &tag_lookups,
                    );
                    let categories = [svc.base.service_definition.category()]
                        .into_iter()
                        .collect();
                    Some(ElementMatchData {
                        categories,
                        tag_ids,
                        element_entity: EntityDiscriminants::Service,
                        virtualizer_service_id,
                        virtualizer_role: virtualizer_role(virtualizer_service_id),
                        deployment_group: None,
                        native_vlan_id: None,
                        vlan_number: None,
                        vlan_name: None,
                        is_trunk_port: false,
                        oper_status: None,
                    })
                }
                _ => None,
            },
            Some(&virtualizer_titles),
            Some(&inline_ctx),
        );

        // --- Phase 5.5: Remove inlined elements and attach InlineGroup to target nodes ---
        let inlined_ids: HashSet<Uuid> = placements
            .iter()
            .filter(|(_, d)| matches!(d, PlacementDecision::InlineOn { .. }))
            .map(|(id, _)| *id)
            .collect();
        nodes.retain(|n| !inlined_ids.contains(&n.id));

        // Attach InlineGroup metadata to target element nodes
        for decision in placements.values() {
            if let PlacementDecision::InlineOn {
                node_id,
                inline_group: Some(group),
            } = decision
            {
                for node in nodes.iter_mut() {
                    if node.id == *node_id {
                        if let NodeType::Element {
                            ref mut inline_groups,
                            ..
                        } = node.node_type
                        {
                            inline_groups.push(group.clone());
                        }
                        break;
                    }
                }
            }
        }

        // --- Phase 6: Remove host containers with no workload elements ---
        // After element rules may have created subcontainers and reassigned elements,
        // find host containers that ended up with zero elements (directly or via subcontainers).

        let container_parents: HashMap<Uuid, Option<Uuid>> = nodes
            .iter()
            .filter_map(|n| {
                if let NodeType::Container {
                    parent_container_id,
                    ..
                } = &n.node_type
                {
                    Some((n.id, *parent_container_id))
                } else {
                    None
                }
            })
            .collect();

        let host_container_ids: HashSet<Uuid> = nodes
            .iter()
            .filter_map(|n| {
                if let NodeType::Container {
                    container_type: ContainerType::Host,
                    ..
                } = &n.node_type
                {
                    Some(n.id)
                } else {
                    None
                }
            })
            .collect();

        // Walk each element up to its root host container
        let mut occupied_hosts: HashSet<Uuid> = HashSet::new();
        for node in &nodes {
            if let NodeType::Element { container_id, .. } = &node.node_type {
                let mut current = *container_id;
                while !host_container_ids.contains(&current) {
                    if let Some(parent) = container_parents.get(&current).and_then(|p| *p) {
                        current = parent;
                    } else {
                        break;
                    }
                }
                if host_container_ids.contains(&current) {
                    occupied_hosts.insert(current);
                }
            }
        }

        // Collect IDs to remove: unoccupied host containers + their orphaned subcontainers
        let ids_to_remove: HashSet<Uuid> = nodes
            .iter()
            .filter_map(|n| {
                if let NodeType::Container {
                    container_type,
                    parent_container_id,
                    ..
                } = &n.node_type
                {
                    // Unoccupied host containers
                    if *container_type == ContainerType::Host && !occupied_hosts.contains(&n.id) {
                        return Some(n.id);
                    }
                    // Subcontainers whose root host is being removed
                    if parent_container_id.is_some() {
                        let mut current = n.id;
                        while let Some(parent) = container_parents.get(&current).and_then(|p| *p) {
                            current = parent;
                        }
                        if host_container_ids.contains(&current)
                            && !occupied_hosts.contains(&current)
                        {
                            return Some(n.id);
                        }
                    }
                }
                None
            })
            .collect();

        nodes.retain(|n| !ids_to_remove.contains(&n.id));

        // Physical link edges between host containers (LLDP/CDP discovered connections)
        // Build inline using host container IDs as source/target, since
        // create_physical_link_edges uses IP address IDs which don't exist in this view.
        let mut edges = Vec::new();
        let mut processed_pairs: HashSet<(Uuid, Uuid)> = HashSet::new();

        for row in ctx.get_interfaces_with_neighbor() {
            let source_id = row.interface_id;
            let target_interface_id = match row.neighbor {
                Neighbor::Interface(id) => id,
                Neighbor::Host(_) => continue,
            };

            let Some(source_entry) = ctx.get_interface_by_id(source_id) else {
                continue;
            };
            let target_entry = match ctx.get_interface_by_id(target_interface_id) {
                Some(e) => e,
                None => continue,
            };

            // Skip self-loops (same host)
            if source_entry.base.host_id == target_entry.base.host_id {
                continue;
            }

            // Skip edges referencing removed host containers
            if ids_to_remove.contains(&Self::container_id_for_host(source_entry.base.host_id))
                || ids_to_remove.contains(&Self::container_id_for_host(target_entry.base.host_id))
            {
                continue;
            }

            // Dedup bidirectional pairs (A→B and B→A are the same physical link)
            let pair_key = if source_id < target_interface_id {
                (source_id, target_interface_id)
            } else {
                (target_interface_id, source_id)
            };
            if !processed_pairs.insert(pair_key) {
                continue;
            }

            let label = Some(format!(
                "{} ↔ {}",
                source_entry.display_name(),
                target_entry.display_name()
            ));

            edges.push(Edge {
                id: Uuid::new_v4(),
                source: Self::container_id_for_host(source_entry.base.host_id),
                target: Self::container_id_for_host(target_entry.base.host_id),
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

        // Device-level adjacencies for neighbours that only resolved to a host. Same host
        // containers, same removal filter — a pair already joined by a physical link above is
        // skipped, so the precise edge wins wherever we have one.
        let linked_host_pairs = EdgeBuilder::physical_link_host_pairs(ctx, &edges);

        edges.extend(
            EdgeBuilder::create_neighbor_link_edges(
                ctx,
                Self::container_id_for_host,
                &linked_host_pairs,
            )
            .into_iter()
            .filter(|edge| {
                !ids_to_remove.contains(&edge.source) && !ids_to_remove.contains(&edge.target)
            }),
        );

        // --- Dependency edges (connecting service elements) ---

        let binding_to_service = ctx.build_binding_to_service_map();

        // Build service_to_node from unified placement decisions.
        // Each PlacementDecision variant maps to a specific node:
        let element_node_ids: HashSet<Uuid> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .map(|n| n.id)
            .collect();

        let mut service_to_node: HashMap<Uuid, Uuid> = HashMap::new();
        for service in ctx.services {
            if let Some(decision) = placements.get(&service.id) {
                match decision {
                    PlacementDecision::Element | PlacementDecision::PlaceInContainer { .. } => {
                        // Service has its own element node (possibly moved to a subcontainer)
                        if element_node_ids.contains(&service.id) {
                            service_to_node.insert(service.id, service.id);
                        }
                    }
                    PlacementDecision::BecomeSubcontainer { container_id } => {
                        // Virtualizer service → its subcontainer node
                        service_to_node.insert(service.id, *container_id);
                    }
                    PlacementDecision::InlineOn { node_id, .. } => {
                        // Service inlined on another node
                        if element_node_ids.contains(node_id) {
                            service_to_node.insert(service.id, *node_id);
                        }
                    }
                }
            } else if element_node_ids.contains(&service.id) {
                // Service not acted on by any rule — maps to itself if it's an element
                service_to_node.insert(service.id, service.id);
            }
            // Services not matching any case (OpenPorts, services on hosts not in graph)
            // are intentionally excluded — they have no node representation.
        }

        for dep in ctx.dependencies {
            let service_ids: Vec<Uuid> = match &dep.base.members {
                DependencyMembers::Services { service_ids } => service_ids
                    .iter()
                    .filter_map(|id| service_to_node.get(id).copied())
                    .collect(),
                DependencyMembers::Bindings { binding_ids } => {
                    let mut ids = Vec::new();
                    for binding_id in binding_ids {
                        if let Some(&service_id) = binding_to_service.get(binding_id)
                            && let Some(&node_id) = service_to_node.get(&service_id)
                            && ids.last() != Some(&node_id)
                        {
                            ids.push(node_id);
                        }
                    }
                    ids
                }
            };

            if service_ids.len() < 2 {
                continue;
            }

            match dep.base.dependency_type {
                DependencyType::RequestPath => {
                    for window in service_ids.windows(2) {
                        edges.push(Edge {
                            id: Uuid::new_v4(),
                            source: window[0],
                            target: window[1],
                            edge_type: EdgeType::RequestPath {
                                dependency_id: dep.id,
                                source_id: Uuid::nil(),
                                target_id: Uuid::nil(),
                            },
                            label: Some(dep.base.name.clone()),
                            source_handle: EdgeHandle::Bottom,
                            target_handle: EdgeHandle::Top,
                            is_multi_hop: false,
                            view_config: EdgeViewConfig::default(),
                            relation_key: None,
                        });
                    }
                }
                DependencyType::HubAndSpoke => {
                    if let Some((&hub_id, spokes)) = service_ids.split_first() {
                        for &spoke_id in spokes {
                            edges.push(Edge {
                                id: Uuid::new_v4(),
                                source: hub_id,
                                target: spoke_id,
                                edge_type: EdgeType::HubAndSpoke {
                                    dependency_id: dep.id,
                                    source_id: Uuid::nil(),
                                    target_id: Uuid::nil(),
                                },
                                label: Some(dep.base.name.clone()),
                                source_handle: EdgeHandle::Bottom,
                                target_handle: EdgeHandle::Top,
                                is_multi_hop: false,
                                view_config: EdgeViewConfig::default(),
                                relation_key: None,
                            });
                        }
                    }
                }
            }
        }

        (nodes, edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{
        hosts::r#impl::{
            base::{Host, HostBase},
            name::{HostName, HostNameSources},
            virtualization::{
                ContainerHostVirtualization, ContainerNetworkType, HostVirtualization,
                ProxmoxGuestType, ProxmoxVirtualization,
            },
        },
        services::r#impl::{
            base::{Service, ServiceBase},
            categories::ServiceCategory,
            definitions::ServiceDefinition,
            patterns::Pattern,
            virtualization::{DockerVirtualization, ServiceVirtualization},
        },
        topology::{
            service::context::TopologyContext,
            types::{
                base::TopologyOptions,
                grouping::{ElementRule, GroupingConfig, IdentifiedRule, InlineGroupRole},
                nodes::ContainerType,
            },
        },
    };
    use chrono::Utc;

    // --- Test service definitions ---

    #[derive(PartialEq, Eq, Hash, Clone)]
    struct ProxmoxDef;
    impl ServiceDefinition for ProxmoxDef {
        fn name(&self) -> &'static str {
            "Proxmox VE"
        }
        fn description(&self) -> &'static str {
            "Proxmox"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::Hypervisor
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    #[derive(PartialEq, Eq, Hash, Clone)]
    struct DockerDef;
    impl ServiceDefinition for DockerDef {
        fn name(&self) -> &'static str {
            "Docker"
        }
        fn description(&self) -> &'static str {
            "Docker"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::ContainerRuntime
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    #[derive(PartialEq, Eq, Hash, Clone)]
    struct RegularDef;
    impl ServiceDefinition for RegularDef {
        fn name(&self) -> &'static str {
            "Samba"
        }
        fn description(&self) -> &'static str {
            "Samba"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::Storage
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    #[derive(PartialEq, Eq, Hash, Clone)]
    struct GenericDef;
    impl ServiceDefinition for GenericDef {
        fn name(&self) -> &'static str {
            "SSH"
        }
        fn description(&self) -> &'static str {
            "SSH"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::RemoteAccess
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
        fn is_generic(&self) -> bool {
            true
        }
    }

    // --- Test helpers ---

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

    fn make_proxmox_vm(name: &str, proxmox_service_id: Uuid) -> Host {
        Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: HostName::manual(name.to_string()),
                virtualization_metadata: Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                    vm_name: Some(name.to_string()),
                    vm_id: None,
                    guest_type: None,
                })),
                virtualization_service_id: Some(proxmox_service_id),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_proxmox_service(host_id: Uuid) -> Service {
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: Box::new(ProxmoxDef),
                name: "Proxmox VE".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_docker_service(host_id: Uuid) -> Service {
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: Box::new(DockerDef),
                name: "Docker".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_docker_container(name: &str, host_id: Uuid, docker_service_id: Uuid) -> Service {
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: Box::new(RegularDef),
                name: name.to_string(),
                virtualization_metadata: Some(ServiceVirtualization::Docker(
                    DockerVirtualization {
                        container_name: Some(name.to_string()),
                        container_id: None,
                        compose_project: None,
                    },
                )),
                virtualization_service_id: Some(docker_service_id),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[derive(PartialEq, Eq, Hash, Clone)]
    struct OpenPortsDef;
    impl ServiceDefinition for OpenPortsDef {
        fn name(&self) -> &'static str {
            "Open Ports"
        }
        fn description(&self) -> &'static str {
            "Open Ports"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::OpenPorts
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    fn make_open_ports_service(name: &str, host_id: Uuid) -> Service {
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: Box::new(OpenPortsDef),
                name: name.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_regular_service(name: &str, host_id: Uuid) -> Service {
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: Box::new(RegularDef),
                name: name.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_generic_service(name: &str, host_id: Uuid) -> Service {
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: Box::new(GenericDef),
                name: name.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn workloads_grouping() -> GroupingConfig {
        GroupingConfig {
            container_rules: vec![],
            element_rules: vec![
                IdentifiedRule::new(ElementRule::ByHypervisor),
                IdentifiedRule::new(ElementRule::ByContainerRuntime),
            ],
            element_sort: Default::default(),
        }
    }

    fn build(hosts: &[Host], services: &[Service]) -> (Vec<Node>, Vec<Edge>) {
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            hosts,
            &[],
            &[],
            services,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );
        WorkloadsBuilder.build(&ctx, &workloads_grouping())
    }

    // --- Tests ---

    #[test]
    fn test_empty_topology() {
        let (nodes, edges) = build(&[], &[]);
        assert!(nodes.is_empty());
        assert!(edges.is_empty());
    }

    #[test]
    fn test_bare_metal_host_with_services() {
        let host = make_host("nas-01");
        let svc1 = make_regular_service("samba", host.id);
        let svc2 = make_regular_service("nfs", host.id);
        let (nodes, _edges) = build(&[host], &[svc1, svc2]);

        // 1 Host container + 2 Service elements
        let containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
            .collect();
        assert_eq!(containers.len(), 1);
        assert!(matches!(
            containers[0].node_type,
            NodeType::Container {
                container_type: ContainerType::Host,
                ..
            }
        ));
        assert_eq!(containers[0].header.as_deref(), Some("nas-01"));

        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 2);

        // Both elements are Service{} type
        for elem in &elements {
            assert!(matches!(
                elem.node_type,
                NodeType::Element {
                    element: ElementEntityType::Service {},
                    ..
                }
            ));
        }

        // Elements are in the host container
        let container_id = containers[0].id;
        for elem in &elements {
            if let NodeType::Element {
                container_id: cid, ..
            } = &elem.node_type
            {
                assert_eq!(*cid, container_id);
            }
        }
    }

    #[test]
    fn test_host_with_only_generic_services() {
        let host = make_host("router-01");
        let ssh = make_generic_service("SSH", host.id);

        let (nodes, _edges) = build(&[host], &[ssh]);

        // Host container IS created — generic filtering is a frontend concern
        let containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
            .collect();
        assert_eq!(containers.len(), 1);

        // SSH service included as element
        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 1);
    }

    #[test]
    fn test_proxmox_hypervisor_with_vms() {
        let hypervisor = make_host("pve-01");
        let proxmox_svc = make_proxmox_service(hypervisor.id);
        let vm1 = make_proxmox_vm("vm-web", proxmox_svc.id);
        let vm2 = make_proxmox_vm("vm-db", proxmox_svc.id);

        let (nodes, edges) = build(&[hypervisor.clone(), vm1, vm2], &[proxmox_svc]);

        // No edges
        assert!(edges.is_empty());

        // 1 Host container (hypervisor only, VMs don't get containers)
        let host_containers: Vec<&Node> = nodes
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
        assert_eq!(host_containers.len(), 1);
        assert_eq!(host_containers[0].header.as_deref(), Some("pve-01"));

        // 1 Hypervisor subcontainer (Proxmox VE)
        let hyp_containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::Hypervisor,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(hyp_containers.len(), 1);

        // 2 VM elements (Host{} type)
        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 2);
        for elem in &elements {
            assert!(matches!(
                elem.node_type,
                NodeType::Element {
                    element: ElementEntityType::Host {},
                    ..
                }
            ));
        }

        // VM elements should be inside the Hypervisor subcontainer
        let virt_id = hyp_containers[0].id;
        for elem in &elements {
            if let NodeType::Element { container_id, .. } = &elem.node_type {
                assert_eq!(*container_id, virt_id);
            }
        }
    }

    #[test]
    fn test_docker_host_with_containers() {
        let host = make_host("server-01");
        let docker_svc = make_docker_service(host.id);
        let c1 = make_docker_container("nginx", host.id, docker_svc.id);
        let c2 = make_docker_container("postgres", host.id, docker_svc.id);

        let (nodes, edges) = build(&[host], &[docker_svc, c1, c2]);

        assert!(edges.is_empty());

        // 1 Host container
        let host_containers: Vec<&Node> = nodes
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
        assert_eq!(host_containers.len(), 1);

        // 1 ContainerRuntime subcontainer (Docker)
        let rt_containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::ContainerRuntime,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(rt_containers.len(), 1);

        // 2 Service elements (containers)
        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 2);

        // Elements inside ContainerRuntime
        let virt_id = rt_containers[0].id;
        for elem in &elements {
            if let NodeType::Element { container_id, .. } = &elem.node_type {
                assert_eq!(*container_id, virt_id);
            }
        }
    }

    #[test]
    fn test_vm_not_a_container() {
        // VM hosts should NOT get their own Host container
        let hypervisor = make_host("pve-01");
        let proxmox_svc = make_proxmox_service(hypervisor.id);
        let vm = make_proxmox_vm("media-vm", proxmox_svc.id);

        // Add a service on the VM (e.g., Docker running on the VM)
        let vm_service = make_regular_service("plex", vm.id);

        let (nodes, _edges) = build(&[hypervisor, vm], &[proxmox_svc, vm_service]);

        // Only 1 Host container (hypervisor), NOT 2
        let host_containers: Vec<&Node> = nodes
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
        assert_eq!(host_containers.len(), 1);
        assert_eq!(host_containers[0].header.as_deref(), Some("pve-01"));

        // The VM's service (plex) is NOT shown since the VM has no container
        // Services on VM hosts are skipped because the VM has virtualization
        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        // Only the VM itself as an element, not plex
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].header.as_deref(), Some("media-vm"));
    }

    #[test]
    fn test_no_edges() {
        let hypervisor = make_host("pve-01");
        let proxmox_svc = make_proxmox_service(hypervisor.id);
        let vm = make_proxmox_vm("vm-1", proxmox_svc.id);

        let (_nodes, edges) = build(&[hypervisor, vm], &[proxmox_svc]);

        assert!(edges.is_empty());
    }

    #[test]
    fn test_mixed_environment() {
        // Hypervisor with VMs + bare metal hosts with services
        let hypervisor = make_host("pve-01");
        let proxmox_svc = make_proxmox_service(hypervisor.id);
        let vm1 = make_proxmox_vm("vm-1", proxmox_svc.id);
        let vm2 = make_proxmox_vm("vm-2", proxmox_svc.id);

        let bare = make_host("nas-01");
        let docker_svc = make_docker_service(bare.id);
        let container = make_docker_container("nginx", bare.id, docker_svc.id);
        let samba = make_regular_service("samba", bare.id);

        let (nodes, edges) = build(
            &[hypervisor, vm1, vm2, bare],
            &[proxmox_svc, docker_svc, container, samba],
        );

        assert!(edges.is_empty());

        // 2 Host containers (hypervisor + bare), VMs don't get containers
        let host_containers: Vec<&Node> = nodes
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
        assert_eq!(host_containers.len(), 2);

        // 1 Hypervisor subcontainer (Proxmox on hypervisor) + 1 ContainerRuntime (Docker on bare)
        let hyp_containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::Hypervisor,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(hyp_containers.len(), 1);

        let rt_containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::ContainerRuntime,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(rt_containers.len(), 1);

        // Elements: 2 VMs + 1 container + 1 samba = 4
        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 4);
    }

    #[test]
    fn test_open_ports_excluded() {
        let host = make_host("server-01");
        let svc = make_regular_service("nginx", host.id);
        let open_ports = make_open_ports_service("Open Ports: 80, 443", host.id);

        let (nodes, _edges) = build(&[host], &[svc, open_ports]);

        // Only the regular service appears as an element
        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].header.as_deref(), Some("nginx"));
    }

    #[test]
    fn test_host_with_only_open_ports_removed() {
        let host = make_host("router-01");
        let open_ports = make_open_ports_service("Open Ports: 22", host.id);

        let (nodes, _edges) = build(&[host], &[open_ports]);

        // Host container is removed because it has no elements
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_empty_host_removed() {
        let host = make_host("empty-host");
        let (nodes, _edges) = build(&[host], &[]);

        // No services → no elements → host container removed
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_mixed_empty_and_populated_hosts() {
        let host1 = make_host("populated");
        let host2 = make_host("empty");
        let svc = make_regular_service("nginx", host1.id);

        let (nodes, _edges) = build(&[host1, host2], &[svc]);

        // Only the populated host's container remains
        let containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
            .collect();
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].header.as_deref(), Some("populated"));

        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 1);
    }

    fn make_container_host(name: &str, docker_service_id: Uuid) -> Host {
        let mut host = make_host(name);
        host.base.virtualization_metadata =
            Some(HostVirtualization::Docker(ContainerHostVirtualization {
                container_name: Some(name.to_string()),
                container_id: None,
                compose_project: None,
                network_type: ContainerNetworkType::MacVlan,
            }));
        host.base.virtualization_service_id = Some(docker_service_id);
        host
    }

    fn containers_of_type(nodes: &[Node], wanted: ContainerType) -> Vec<&Node> {
        nodes
            .iter()
            .filter(|n| {
                matches!(n.node_type, NodeType::Container { container_type, .. }
                    if container_type == wanted)
            })
            .collect()
    }

    fn container_of(nodes: &[Node], id: Uuid) -> Option<Uuid> {
        nodes
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| match n.node_type {
                NodeType::Element { container_id, .. } => Some(container_id),
                _ => None,
            })
    }

    /// One Docker box holds both kinds of container: a bridge container (a service) and a
    /// macvlan container (a host of its own), whose service shows on that host rather than as
    /// a separate element.
    #[test]
    fn docker_box_holds_container_services_and_container_hosts() {
        let host = make_host("docker-prod01");
        let docker_svc = make_docker_service(host.id);
        let nginx = make_docker_container("nginx", host.id, docker_svc.id);
        let pihole = make_container_host("pihole", docker_svc.id);
        let dns = make_regular_service("pihole-dns", pihole.id);
        let (nginx_id, pihole_id, dns_id) = (nginx.id, pihole.id, dns.id);

        let (nodes, _edges) = build(&[host, pihole], &[docker_svc, nginx, dns]);

        assert_eq!(containers_of_type(&nodes, ContainerType::Host).len(), 1);
        assert!(containers_of_type(&nodes, ContainerType::Hypervisor).is_empty());
        let runtime_boxes = containers_of_type(&nodes, ContainerType::ContainerRuntime);
        assert_eq!(runtime_boxes.len(), 1);
        let runtime_box = runtime_boxes[0].id;

        assert_eq!(container_of(&nodes, nginx_id), Some(runtime_box));
        assert_eq!(container_of(&nodes, pihole_id), Some(runtime_box));
        assert!(
            nodes.iter().all(|n| n.id != dns_id),
            "a container host's service is inlined on the host, not its own element"
        );
    }

    /// A Docker running in a VM shows all its containers, bridge services and macvlan hosts
    /// alike, inlined on the VM as one group headed by the runtime.
    #[test]
    fn docker_in_vm_inlines_container_services_and_container_hosts() {
        let node = make_host("pve-01");
        let proxmox_svc = make_proxmox_service(node.id);
        let vm = make_proxmox_vm("docker-vm", proxmox_svc.id);
        let docker_svc = make_docker_service(vm.id);
        let nginx = make_docker_container("nginx", vm.id, docker_svc.id);
        let pihole = make_container_host("pihole", docker_svc.id);
        let dns = make_regular_service("pihole-dns", pihole.id);
        let (vm_id, docker_id, nginx_id, pihole_id) = (vm.id, docker_svc.id, nginx.id, pihole.id);

        let (nodes, _edges) = build(&[node, vm, pihole], &[proxmox_svc, docker_svc, nginx, dns]);

        assert!(containers_of_type(&nodes, ContainerType::ContainerRuntime).is_empty());
        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 1, "only the VM remains an element");
        assert_eq!(elements[0].id, vm_id);

        let NodeType::Element { inline_groups, .. } = &elements[0].node_type else {
            unreachable!()
        };
        assert!(inline_groups.iter().all(|g| g.group_id == docker_id));
        // Each member says which entity it names, so the card resolves a bridge container as a
        // service and a macvlan container as a host.
        let members: HashSet<(Uuid, EntityDiscriminants)> = inline_groups
            .iter()
            .filter(|g| g.role == InlineGroupRole::Member)
            .map(|g| (g.entity_id, g.entity_type))
            .collect();
        assert_eq!(
            members,
            HashSet::from([
                (nginx_id, EntityDiscriminants::Service),
                (pihole_id, EntityDiscriminants::Host)
            ])
        );
        assert!(
            inline_groups
                .iter()
                .any(|g| g.role == InlineGroupRole::Header && g.entity_id == docker_id)
        );
    }

    /// Proxmox VMs and LXC containers group under their hypervisor as before; container hosts
    /// take nothing from the hypervisor rule.
    #[test]
    fn proxmox_vms_and_lxc_containers_group_under_the_hypervisor() {
        let node = make_host("pve-01");
        let proxmox_svc = make_proxmox_service(node.id);
        let guest = |name: &str, guest_type: ProxmoxGuestType| {
            let mut host = make_proxmox_vm(name, proxmox_svc.id);
            if let Some(HostVirtualization::Proxmox(p)) = &mut host.base.virtualization_metadata {
                p.guest_type = Some(guest_type);
            }
            host
        };
        let vm = guest("vm-web", ProxmoxGuestType::Qemu);
        let lxc = guest("ct-dns", ProxmoxGuestType::Lxc);
        let (vm_id, lxc_id) = (vm.id, lxc.id);

        let (nodes, _edges) = build(&[node, vm, lxc], &[proxmox_svc]);

        assert!(containers_of_type(&nodes, ContainerType::ContainerRuntime).is_empty());
        let hypervisor_boxes = containers_of_type(&nodes, ContainerType::Hypervisor);
        assert_eq!(hypervisor_boxes.len(), 1);
        assert_eq!(container_of(&nodes, vm_id), Some(hypervisor_boxes[0].id));
        assert_eq!(container_of(&nodes, lxc_id), Some(hypervisor_boxes[0].id));
    }

    fn make_identities_service(host_id: Uuid) -> Service {
        use crate::server::services::definitions::network_identities::NetworkIdentities;
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: Box::new(NetworkIdentities),
                name: "Network Identities".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_identity_host(interface: &str, identities_service_id: Uuid) -> Host {
        use crate::server::hosts::r#impl::virtualization::NetworkIdentityVirtualization;
        let mut host = make_host(interface);
        host.base.virtualization_metadata = Some(HostVirtualization::NetworkIdentity(
            NetworkIdentityVirtualization {},
        ));
        host.base.virtualization_service_id = Some(identities_service_id);
        host
    }

    /// A VM's network identities show inside the VM, in the hypervisor's box: inlined on the VM
    /// as one group headed by its Network Identities service, the way a Docker in a VM shows its
    /// containers. An identity's own service follows it onto the VM.
    #[test]
    fn a_vms_network_identities_inline_on_the_vm() {
        let node = make_host("pve");
        let proxmox_svc = make_proxmox_service(node.id);
        let vm = make_proxmox_vm("snmp-lab", proxmox_svc.id);
        let identities_svc = make_identities_service(vm.id);
        let first = make_identity_host("mv-snmp1", identities_svc.id);
        let second = make_identity_host("mv-snmp2", identities_svc.id);
        let snmpd = make_regular_service("snmpd", first.id);
        let (vm_id, identities_id, first_id, second_id, snmpd_id) =
            (vm.id, identities_svc.id, first.id, second.id, snmpd.id);

        let (nodes, _edges) = build(
            &[node, vm, first, second],
            &[proxmox_svc, identities_svc, snmpd],
        );

        let hypervisor_boxes = containers_of_type(&nodes, ContainerType::Hypervisor);
        assert_eq!(
            hypervisor_boxes.len(),
            1,
            "no box of its own for the identities"
        );
        assert_eq!(container_of(&nodes, vm_id), Some(hypervisor_boxes[0].id));
        for id in [first_id, second_id, snmpd_id] {
            assert!(nodes.iter().all(|n| n.id != id), "inlined on the VM");
        }

        let vm_node = nodes.iter().find(|n| n.id == vm_id).unwrap();
        let NodeType::Element { inline_groups, .. } = &vm_node.node_type else {
            unreachable!()
        };
        assert!(inline_groups.iter().all(|g| g.group_id == identities_id));
        let members: HashSet<Uuid> = inline_groups
            .iter()
            .filter(|g| g.role == InlineGroupRole::Member)
            // Members name hosts, so the card looks them up as hosts rather than services.
            .filter(|g| g.entity_type == EntityDiscriminants::Host)
            .map(|g| g.entity_id)
            .collect();
        assert_eq!(members, HashSet::from([first_id, second_id]));
        assert!(
            inline_groups
                .iter()
                .any(|g| g.role == InlineGroupRole::Header && g.entity_id == identities_id)
        );
    }

    /// On a host that is not a guest, the identities get a box of their own in the host's box,
    /// titled with the service's name.
    #[test]
    fn a_hosts_network_identities_get_a_box_titled_by_the_service() {
        let host = make_host("router");
        let identities_svc = make_identities_service(host.id);
        let identity = make_identity_host("vip0", identities_svc.id);
        let identity_id = identity.id;

        let (nodes, _edges) = build(&[host, identity], &[identities_svc]);

        let boxes = containers_of_type(&nodes, ContainerType::Hypervisor);
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0].header.as_deref(), Some("Network Identities"));
        assert_eq!(container_of(&nodes, identity_id), Some(boxes[0].id));
    }
}
