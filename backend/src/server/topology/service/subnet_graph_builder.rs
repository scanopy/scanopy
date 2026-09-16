use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use strum::IntoDiscriminant;
use uuid::Uuid;

use crate::server::{
    hosts::r#impl::{base::Host, name_ladder::HostNameRung},
    ip_addresses::r#impl::base::IPAddress,
    shared::entities::EntityDiscriminants,
    subnets::r#impl::types::{SubnetType, SubnetTypeDiscriminants},
    topology::{
        service::{
            anchor_planner::ChildAnchorPlanner,
            context::TopologyContext,
            element_rules::{
                ElementMatchData, TaggableLookups, apply_element_rules, apply_stack_runtime_logos,
                resolve_element_tag_ids,
            },
        },
        types::{
            edges::Edge,
            grouping::GroupingConfig,
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
            if subnet.base.subnet_type.is_container_bridge() {
                mapping
                    .entry((
                        ip_address.base.host_id,
                        subnet.base.subnet_type.discriminant(),
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
        all_edges: &mut [Edge],
        grouping: &GroupingConfig,
    ) -> (HashSet<Uuid>, Vec<Node>) {
        let container_bridge_grouping = if grouping.should_group_container_bridges() {
            Self::compute_container_bridge_grouping(ctx)
        } else {
            HashMap::new()
        };
        let children_by_subnet =
            self.group_children_by_subnet(ctx, all_edges, grouping, container_bridge_grouping);
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
                            .map(|s| !s.base.subnet_type.is_container_bridge())
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
        all_edges: &mut [Edge],
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
                .map(|s| s.base.subnet_type.exclude_from_topology())
                .unwrap_or(false)
            {
                continue;
            }
            let subnet_type = subnet.map(|s| s.base.subnet_type).unwrap_or_default();

            // Update source/target handles for edges
            ChildAnchorPlanner::plan_anchors(ip_address.id, all_edges, ctx);

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
        _subnet_id: Uuid,
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

        let _ = apply_element_rules(
            child_nodes,
            &grouping.element_rules,
            |node| {
                let child = children_by_id.get(&node.id)?;
                let categories = ctx
                    .services
                    .iter()
                    .filter(|s| s.base.host_id == child.host_id)
                    .map(|s| s.base.service_definition.category())
                    .collect();
                let tag_ids = resolve_element_tag_ids(
                    EntityDiscriminants::IPAddress,
                    child.host_id,
                    &tag_lookups,
                );
                // Resolve the deployment group (compose project) only for elements
                // inside container subnets. LAN ip_addresses shouldn't be grouped by stack.
                let is_container_subnet = ctx
                    .subnets
                    .iter()
                    .find(|s| s.id == _subnet_id)
                    .map(|s| s.base.subnet_type.is_container_network())
                    .unwrap_or(false);
                let deployment_group = if !is_container_subnet {
                    None
                } else {
                    let mut projects: HashSet<&str> = HashSet::new();
                    let services_iter: Box<dyn Iterator<Item = _>> =
                        if let Some(iface_id) = child.ip_address_id {
                            // Interface-specific: only services bound to this interface
                            Box::new(ctx.services.iter().filter(move |s| {
                                s.base.host_id == child.host_id
                                    && s.base
                                        .bindings
                                        .iter()
                                        .any(|b| b.ip_address_id() == Some(iface_id))
                            }))
                        } else {
                            // Fallback: all services on the host
                            Box::new(
                                ctx.services
                                    .iter()
                                    .filter(|s| s.base.host_id == child.host_id),
                            )
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
                    deployment_group,
                    native_vlan_id: None,
                    vlan_number: None,
                    vlan_name: None,
                    is_trunk_port: false,
                    oper_status: None,
                })
            },
            None,
            None,
        );

        // Post-process: stamp each Stack subcontainer's logo from the runtime of
        // the services in that deployment group (Docker, Podman, …).
        apply_stack_runtime_logos(child_nodes, ctx.services);
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
                            .and_then(|s| s.base.subnet_type.container_runtime_label())
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
                        let type_name = subnet.base.subnet_type.name();
                        let cidr = subnet.base.cidr.to_string();
                        let show_label = subnet.base.subnet_type.show_label();
                        let name_or_type = if subnet.base.name != cidr {
                            subnet.base.name.clone()
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
                        .is_some_and(|s| s.base.subnet_type.is_container_bridge());
                Node {
                    id: *subnet_id,
                    node_type: NodeType::Container {
                        container_type: ContainerType::Subnet,
                        parent_container_id: None,
                        entity_id: Some(*subnet_id),
                        icon: None,
                        color: None,
                        associated_service_definition: None,
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
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::{Host, HostBase};
    use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
    use crate::server::services::r#impl::base::{Service, ServiceBase};
    use crate::server::services::r#impl::categories::ServiceCategory;
    use crate::server::services::r#impl::definitions::ServiceDefinition;
    use crate::server::services::r#impl::patterns::Pattern;
    use crate::server::shared::types::Color;
    use crate::server::tags::r#impl::base::{Tag, TagBase};
    use crate::server::topology::service::context::TopologyContext;
    use crate::server::topology::types::base::TopologyOptions;
    use crate::server::topology::types::grouping::{ElementRule, IdentifiedRule};
    use chrono::Utc;

    /// Test service definition that returns ReverseProxy category
    #[derive(PartialEq, Eq, Hash, Clone)]
    struct ReverseProxyServiceDef;

    impl ServiceDefinition for ReverseProxyServiceDef {
        fn name(&self) -> &'static str {
            "TestReverseProxy"
        }
        fn description(&self) -> &'static str {
            "Test"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::ReverseProxy
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    fn make_host(name: &str, tags: Vec<Uuid>) -> Host {
        Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: HostName::manual(name.to_string()),
                tags,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_service(host_id: Uuid, def: Box<dyn ServiceDefinition>) -> Service {
        make_service_with_tags(host_id, def, vec![])
    }

    fn make_service_with_tags(
        host_id: Uuid,
        def: Box<dyn ServiceDefinition>,
        tags: Vec<Uuid>,
    ) -> Service {
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: def,
                tags,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_tag(name: &str) -> Tag {
        Tag {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: TagBase {
                name: name.to_string(),
                description: None,
                color: Color::Yellow,
                organization_id: Uuid::new_v4(),
                is_application: false,
            },
            ..Default::default()
        }
    }

    fn make_element_node(id: Uuid, host_id: Uuid, container_id: Uuid) -> Node {
        Node::element(
            id,
            container_id,
            host_id,
            ElementEntityType::IPAddress {
                subnet_id: container_id,
                ip_address_id: Some(id),
            },
        )
    }

    fn make_subnet_child(id: Uuid, host_id: Uuid) -> SubnetChildData {
        SubnetChildData {
            id,
            host_id,
            ip_address_id: Some(id),
            header: None,
        }
    }

    #[test]
    fn test_nested_group_first_match_wins() {
        let tag = make_tag("MyTag");
        // Host that matches both ByServiceCategory(ReverseProxy) AND ByTag(tag)
        let host_both = make_host("host-both", vec![tag.id]);
        // Host that only matches the tag
        let host_tag_only = make_host("host-tag-only", vec![tag.id]);

        let svc = make_service(host_both.id, Box::new(ReverseProxyServiceDef));

        let subnet_id = Uuid::new_v4();
        let child_both_id = Uuid::new_v4();
        let child_tag_id = Uuid::new_v4();

        let children = vec![
            make_subnet_child(child_both_id, host_both.id),
            make_subnet_child(child_tag_id, host_tag_only.id),
        ];

        let mut child_nodes = vec![
            make_element_node(child_both_id, host_both.id, subnet_id),
            make_element_node(child_tag_id, host_tag_only.id, subnet_id),
        ];

        // Rules: ByServiceCategory first, then ByTag
        // Use Application view so ByServiceCategory is applicable
        let mut options = TopologyOptions::default();
        options.request.element_rules = vec![
            IdentifiedRule::new(ElementRule::ByServiceCategory {
                categories: vec![ServiceCategory::ReverseProxy],
                title: Some("Infra".to_string()),
                is_infra_rule: false,
            }),
            IdentifiedRule::new(ElementRule::ByTag {
                tag_ids: vec![tag.id],
                title: None,
            }),
        ];

        let hosts = vec![host_both.clone(), host_tag_only.clone()];
        let services = vec![svc];
        let tags = vec![tag.clone()];

        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &services,
            &[],
            &[],
            &[],
            &[],
            &tags,
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::Application,
        );

        let planner = SubnetGraphBuilder::new();
        planner.create_nested_group_containers(subnet_id, &children, &ctx, &mut child_nodes);

        // Find group containers
        let groups: Vec<&Node> = child_nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
            .collect();
        assert_eq!(groups.len(), 2, "Should create two group containers");

        let cat_group = groups
            .iter()
            .find(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::NestedServiceCategory,
                        ..
                    }
                )
            })
            .expect("Should have NestedServiceCategory");

        let tag_group = groups
            .iter()
            .find(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::NestedTag,
                        ..
                    }
                )
            })
            .expect("Should have NestedTag");

        // First-match-wins: host_both should be in the category group (first rule)
        let both_node = child_nodes.iter().find(|n| n.id == child_both_id).unwrap();
        if let NodeType::Element { container_id, .. } = &both_node.node_type {
            assert_eq!(
                *container_id, cat_group.id,
                "Overlapping host should be in first-match container (NestedServiceCategory)"
            );
        }

        // host_tag_only should be in the tag group (only matches tag rule)
        let tag_node = child_nodes.iter().find(|n| n.id == child_tag_id).unwrap();
        if let NodeType::Element { container_id, .. } = &tag_node.node_type {
            assert_eq!(
                *container_id, tag_group.id,
                "Tag-only host should be in NestedTag"
            );
        }

        // Verify headers contain custom titles
        assert_eq!(
            cat_group.header.as_deref(),
            Some("Infra"),
            "Category group header should be custom title"
        );
        assert!(
            tag_group.header.is_none(),
            "Tag group with no custom title should have no header"
        );

        // Verify element_rule_id is set on the container
        assert!(
            matches!(&cat_group.node_type, NodeType::Container { element_rule_id, .. } if element_rule_id.is_some()),
            "Category group should have element_rule_id"
        );
        assert!(
            matches!(&tag_group.node_type, NodeType::Container { element_rule_id, .. } if element_rule_id.is_some()),
            "Tag group should have element_rule_id"
        );
    }

    #[test]
    fn test_nested_group_reversed_order_flips_priority() {
        let tag = make_tag("TestTag");
        let host = make_host("overlap-host", vec![tag.id]);
        let svc = make_service(host.id, Box::new(ReverseProxyServiceDef));

        let subnet_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let children = vec![make_subnet_child(child_id, host.id)];

        // This time: ByTag FIRST, then ByServiceCategory
        let mut options = TopologyOptions::default();
        options.request.element_rules = vec![
            IdentifiedRule::new(ElementRule::ByTag {
                tag_ids: vec![tag.id],
                title: Some("Tagged".to_string()),
            }),
            IdentifiedRule::new(ElementRule::ByServiceCategory {
                categories: vec![ServiceCategory::ReverseProxy],
                title: Some("Infra".to_string()),
                is_infra_rule: false,
            }),
        ];

        let hosts = vec![host.clone()];
        let services = vec![svc];
        let tags = vec![tag.clone()];

        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &services,
            &[],
            &[],
            &[],
            &[],
            &tags,
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let mut child_nodes = vec![make_element_node(child_id, host.id, subnet_id)];

        let planner = SubnetGraphBuilder::new();
        planner.create_nested_group_containers(subnet_id, &children, &ctx, &mut child_nodes);

        // Find the tag group (should be first match now)
        let tag_group = child_nodes
            .iter()
            .find(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::NestedTag,
                        ..
                    }
                )
            })
            .expect("Should have NestedTag");

        // Host should be in tag group (first rule wins)
        let element = child_nodes.iter().find(|n| n.id == child_id).unwrap();
        if let NodeType::Element { container_id, .. } = &element.node_type {
            assert_eq!(
                *container_id, tag_group.id,
                "When ByTag is first, overlapping host should be in NestedTag"
            );
        }

        // NestedServiceCategory should not be created when all its matches are already claimed
        // (since the only matching host was claimed by tag rule)
        let cat_group = child_nodes.iter().find(|n| {
            matches!(
                n.node_type,
                NodeType::Container {
                    container_type: ContainerType::NestedServiceCategory,
                    ..
                }
            )
        });
        assert!(
            cat_group.is_none(),
            "NestedServiceCategory should not be created when all its matches are already claimed"
        );
    }

    #[test]
    fn test_bytag_does_not_match_on_service_tags_in_l3() {
        // Regression: ByTag rules in L3 used to match IPAddress elements via
        // service-tag inheritance (all service tags on the IP's host were unioned
        // into the element's tag_ids). That conflated service tags with host tags.
        // Now the element resolves only its taggable ancestor (Host), so a tag
        // applied only to a service on the host should NOT match.
        let tag = make_tag("ServiceTag");
        let host = make_host("host-no-tags", vec![]); // Host has NO tags
        let svc = make_service_with_tags(host.id, Box::new(ReverseProxyServiceDef), vec![tag.id]);

        let subnet_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let children = vec![make_subnet_child(child_id, host.id)];
        let mut child_nodes = vec![make_element_node(child_id, host.id, subnet_id)];

        let mut options = TopologyOptions::default();
        options.request.element_rules = vec![IdentifiedRule::new(ElementRule::ByTag {
            tag_ids: vec![tag.id],
            title: Some("ServiceTagGroup".to_string()),
        })];

        let hosts = vec![host.clone()];
        let services = vec![svc];
        let tags = vec![tag.clone()];

        let ctx = TopologyContext::new(
            &hosts,
            &[],
            &[],
            &services,
            &[],
            &[],
            &[],
            &[],
            &tags,
            &[],
            &options,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );

        let planner = SubnetGraphBuilder::new();
        planner.create_nested_group_containers(subnet_id, &children, &ctx, &mut child_nodes);

        // No NestedTag container should be created — the rule's tag is on the
        // service, not the host, so the IP address does not match.
        assert!(
            !child_nodes.iter().any(|n| matches!(
                n.node_type,
                NodeType::Container {
                    container_type: ContainerType::NestedTag,
                    ..
                }
            )),
            "ByTag should not match service-only tags via inheritance"
        );
    }
}
