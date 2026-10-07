use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::{
    context::TopologyContext,
    element_rules::{
        ElementMatchData, TaggableLookups, apply_element_rules, apply_stack_runtime_logos,
        resolve_element_tag_ids,
    },
    view::ViewBuilder,
};
use crate::server::{
    dependencies::r#impl::{base::DependencyMembers, types::DependencyType},
    services::r#impl::categories::ServiceCategory,
    shared::{
        concepts::Concept, entities::EntityDiscriminants, types::metadata::EntityMetadataProvider,
    },
    tags::r#impl::base::Tag,
    topology::types::{
        edges::{Edge, EdgeHandle, EdgeType, EdgeViewConfig},
        grouping::GroupingConfig,
        nodes::{ContainerType, ElementEntityType, Node, NodeType},
    },
};

/// Fixed UUID for the "Ungrouped" container
const UNGROUPED_CONTAINER_ID: Uuid = Uuid::from_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
]);

/// Namespace for deterministic topology container UUIDs.
pub const TOPOLOGY_CONTAINER_NAMESPACE: Uuid = Uuid::from_bytes([
    0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6, 0x47, 0x89, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78,
]);

/// Create a deterministic container UUID from a tag UUID.
fn container_id_for_tag(tag_id: &Uuid) -> Uuid {
    Uuid::new_v5(&TOPOLOGY_CONTAINER_NAMESPACE, tag_id.as_bytes())
}

pub struct ApplicationBuilder;

impl ApplicationBuilder {
    /// Find the application tag for a service: direct tag first, then inherit from host.
    fn find_app_tag<'a>(
        service: &crate::server::services::r#impl::base::Service,
        hosts: &[crate::server::hosts::r#impl::base::Host],
        app_tags: &'a HashMap<Uuid, &'a Tag>,
    ) -> Option<&'a Tag> {
        // Check service's own tags first
        for tag_id in &service.base.tags {
            if let Some(tag) = app_tags.get(tag_id) {
                return Some(tag);
            }
        }
        // Inherit from host
        if let Some(host) = hosts.iter().find(|h| h.id == service.base.host_id) {
            for tag_id in &host.base.tags {
                if let Some(tag) = app_tags.get(tag_id) {
                    return Some(tag);
                }
            }
        }
        None
    }
}

impl ViewBuilder for ApplicationBuilder {
    fn build(&self, ctx: &TopologyContext, grouping: &GroupingConfig) -> (Vec<Node>, Vec<Edge>) {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Build app tag lookup from entity_tags
        let app_tags: HashMap<Uuid, &Tag> = ctx
            .entity_tags
            .iter()
            .filter(|t| t.is_application())
            .map(|t| (t.id, t))
            .collect();

        // Collect services with at least one binding, excluding OpenPorts
        let eligible_services: Vec<&crate::server::services::r#impl::base::Service> = ctx
            .services
            .iter()
            .filter(|s| !s.base.bindings.is_empty())
            .filter(|s| s.base.service_definition.category() != ServiceCategory::OpenPorts)
            .collect();

        // service_id → node exists (for edge creation)
        let mut service_node_ids: HashMap<Uuid, bool> = HashMap::new();

        // ApplicationBuilder is only invoked when view==Application (see view.rs),
        // in which case the ByApplication container rule is always present and
        // can't be removed. Group all services by their application tag (or into
        // the "Ungrouped" container if none match).
        let mut services_by_tag: HashMap<
            Uuid,
            Vec<&crate::server::services::r#impl::base::Service>,
        > = HashMap::new();
        let mut ungrouped_services: Vec<&crate::server::services::r#impl::base::Service> =
            Vec::new();

        for service in &eligible_services {
            match Self::find_app_tag(service, ctx.hosts, &app_tags) {
                Some(tag) => {
                    services_by_tag.entry(tag.id).or_default().push(service);
                }
                None => {
                    ungrouped_services.push(service);
                }
            }
        }

        // Create containers for each app tag
        for (tag_id, services) in &services_by_tag {
            let container_id = container_id_for_tag(tag_id);
            let tag = app_tags[tag_id];

            nodes.push(Node {
                id: container_id,
                node_type: NodeType::Container {
                    container_type: ContainerType::Application,
                    parent_container_id: None,
                    entity_id: Some(*tag_id),
                    icon: Some(Concept::Application.icon().to_string()),
                    color: Some(tag.base.color),
                    associated_service_definition: None,
                    element_rule_id: None,
                    will_accept_edges: false,
                },
                position: Default::default(),
                size: Default::default(),
                header: Some(tag.base.name.clone()),
            });

            for service in services {
                let mut node = Node::element(
                    service.id,
                    container_id,
                    service.base.host_id,
                    ElementEntityType::Service {},
                );
                node.header = Some(service.base.name.clone());
                nodes.push(node);
                service_node_ids.insert(service.id, true);
            }
        }

        // Create "Ungrouped" container if needed
        if !ungrouped_services.is_empty() {
            nodes.push(Node {
                id: UNGROUPED_CONTAINER_ID,
                node_type: NodeType::Container {
                    container_type: ContainerType::ApplicationUngrouped,
                    parent_container_id: None,
                    entity_id: None,
                    icon: Some(Concept::Application.icon().to_string()),
                    color: Some(Concept::Application.color()),
                    associated_service_definition: None,
                    element_rule_id: None,
                    will_accept_edges: false,
                },
                position: Default::default(),
                size: Default::default(),
                header: Some("Ungrouped".to_string()),
            });

            for service in &ungrouped_services {
                let mut node = Node::element(
                    service.id,
                    UNGROUPED_CONTAINER_ID,
                    service.base.host_id,
                    ElementEntityType::Service {},
                );
                node.header = Some(service.base.name.clone());
                nodes.push(node);
                service_node_ids.insert(service.id, true);
            }
        }

        // Apply element rules (ByServiceCategory, ByTag) to create nested subcontainers
        let service_lookup: HashMap<Uuid, &crate::server::services::r#impl::base::Service> =
            eligible_services.iter().map(|s| (s.id, *s)).collect();
        let tag_lookups = TaggableLookups {
            hosts: None,
            services: Some(&service_lookup),
            subnets: None,
        };
        let _ = apply_element_rules(
            &mut nodes,
            &grouping.element_rules,
            |node| {
                let service = service_lookup.get(&node.id)?;
                let categories = HashSet::from([service.base.service_definition.category()]);
                let tag_ids =
                    resolve_element_tag_ids(EntityDiscriminants::Service, service.id, &tag_lookups);
                let deployment_group = service
                    .base
                    .virtualization_metadata
                    .as_ref()
                    .and_then(|v| v.compose_project().map(str::to_string));
                Some(ElementMatchData {
                    categories,
                    tag_ids,
                    element_entity: EntityDiscriminants::Service,
                    virtualizer_service_id: None,
                    virtualizer_role: None,
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
        apply_stack_runtime_logos(&mut nodes, ctx.services);

        // Build binding_id → service_id lookup (for Bindings variant backward compat)
        let binding_to_service = ctx.build_binding_to_service_map();

        // Create service-level flow edges from dependencies
        for dep in ctx.dependencies {
            // Resolve to ordered service IDs based on member type
            let is_bindings = matches!(dep.base.members, DependencyMembers::Bindings { .. });
            let service_ids: Vec<Uuid> = match &dep.base.members {
                DependencyMembers::Services { service_ids } => service_ids.clone(),
                DependencyMembers::Bindings { binding_ids } => {
                    let mut ids = Vec::new();
                    for binding_id in binding_ids {
                        if let Some(&service_id) = binding_to_service.get(binding_id)
                            && ids.last() != Some(&service_id)
                        {
                            ids.push(service_id);
                        } else if !binding_to_service.contains_key(binding_id) {
                            tracing::warn!(
                                dep_id = %dep.id, dep_name = %dep.base.name,
                                binding_id = %binding_id,
                                "Binding ID not found in any service's bindings"
                            );
                        }
                    }
                    ids
                }
            };

            if is_bindings && service_ids.len() < 2 {
                tracing::warn!(
                    dep_id = %dep.id, dep_name = %dep.base.name,
                    resolved = service_ids.len(),
                    member_type = "Bindings",
                    "Dependency resolved to < 2 services — no edges will be created"
                );
            }

            // Only create edges between services that have nodes
            match dep.base.dependency_type {
                DependencyType::RequestPath => {
                    for window in service_ids.windows(2) {
                        let (source_id, target_id) = (window[0], window[1]);
                        let source_has_node = service_node_ids.contains_key(&source_id);
                        let target_has_node = service_node_ids.contains_key(&target_id);
                        if !source_has_node || !target_has_node {
                            tracing::warn!(
                                dep_id = %dep.id, dep_name = %dep.base.name,
                                source_id = %source_id, source_has_node,
                                target_id = %target_id, target_has_node,
                                "RequestPath edge dropped — service missing from graph"
                            );
                        }
                        if source_has_node && target_has_node {
                            edges.push(Edge {
                                id: Uuid::new_v4(),
                                source: source_id,
                                target: target_id,
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
                }
                DependencyType::HubAndSpoke => {
                    if let Some((&hub_id, spokes)) = service_ids.split_first()
                        && service_node_ids.contains_key(&hub_id)
                    {
                        for &spoke_id in spokes {
                            if service_node_ids.contains_key(&spoke_id) {
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
        }

        // Create ContainerRuntime overlay edges (runtime-agnostic: Docker, Podman, …)
        for service in ctx.services {
            if let Some(runtime_service_id) = service.base.virtualization_service_id
                && service_node_ids.contains_key(&service.id)
                && service_node_ids.contains_key(&runtime_service_id)
            {
                edges.push(Edge {
                    id: Uuid::new_v4(),
                    source: runtime_service_id,
                    target: service.id,
                    edge_type: EdgeType::ContainerRuntime {
                        service_id: runtime_service_id,
                        host_id: service.base.host_id,
                        // This view has no subnets — services are the elements — and it draws
                        // one edge per container, so the edge stands for the container it
                        // targets and nothing else.
                        subnet_ids: vec![],
                        containerized_service_ids: vec![service.id],
                    },
                    label: None,
                    source_handle: EdgeHandle::Bottom,
                    target_handle: EdgeHandle::Top,
                    is_multi_hop: false,
                    view_config: EdgeViewConfig::default(),
                    relation_key: None,
                });
            }
        }

        (nodes, edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
    use crate::server::{
        bindings::r#impl::base::{Binding, BindingBase, BindingType},
        dependencies::r#impl::base::{Dependency, DependencyBase, DependencyMembers},
        hosts::r#impl::base::{Host, HostBase},
        services::r#impl::{
            base::{Service, ServiceBase},
            categories::ServiceCategory,
            definitions::ServiceDefinition,
            patterns::Pattern,
        },
        shared::types::Color,
        tags::r#impl::base::{Tag, TagBase},
        topology::{
            service::context::TopologyContext,
            types::{
                base::TopologyOptions,
                grouping::{ContainerRule, ElementRule, GroupingConfig, IdentifiedRule},
                nodes::NodeType,
                views::TopologyView,
            },
        },
    };
    use chrono::Utc;

    #[derive(PartialEq, Eq, Hash, Clone)]
    struct TestServiceDef {
        category: ServiceCategory,
        name: &'static str,
    }

    impl ServiceDefinition for TestServiceDef {
        fn name(&self) -> &'static str {
            self.name
        }
        fn description(&self) -> &'static str {
            "Test"
        }
        fn category(&self) -> ServiceCategory {
            self.category
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    fn make_service(
        host_id: Uuid,
        category: ServiceCategory,
        name: &'static str,
        bindings: Vec<Binding>,
    ) -> Service {
        Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                service_definition: Box::new(TestServiceDef { category, name }),
                name: name.to_string(),
                bindings,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_binding(service_id: Uuid) -> Binding {
        Binding {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: BindingBase {
                service_id,
                site_id: Uuid::new_v4(),
                binding_type: BindingType::IPAddress {
                    ip_address_id: Uuid::new_v4(),
                },
            },
            ..Default::default()
        }
    }

    fn make_dependency(dep_type: DependencyType, members: DependencyMembers) -> Dependency {
        Dependency {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: DependencyBase {
                name: "Test Dependency".to_string(),
                dependency_type: dep_type,
                members,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_services_without_app_tags_go_to_ungrouped() {
        let host_id = Uuid::new_v4();
        let binding = make_binding(Uuid::nil());
        let svc = make_service(
            host_id,
            ServiceCategory::Database,
            "PostgreSQL",
            vec![binding],
        );
        let services = vec![svc];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &[],
            &[],
            &[],
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

        let builder = ApplicationBuilder;
        let (nodes, _edges) = builder.build(
            &ctx,
            &GroupingConfig::from_request_options(&options.request, TopologyView::L3Logical),
        );

        // Should have one "Ungrouped" container + one element
        let containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
            .collect();
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].header.as_deref(), Some("Ungrouped"));

        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].header.as_deref(), Some("PostgreSQL"));
    }

    #[test]
    fn test_skips_services_without_bindings() {
        let host_id = Uuid::new_v4();
        let svc_no_bindings =
            make_service(host_id, ServiceCategory::Database, "NoBindings", vec![]);
        let services = vec![svc_no_bindings];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &[],
            &[],
            &[],
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

        let builder = ApplicationBuilder;
        let (nodes, _edges) = builder.build(
            &ctx,
            &GroupingConfig::from_request_options(&options.request, TopologyView::L3Logical),
        );
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_request_path_edges() {
        let host_id = Uuid::new_v4();

        let b1 = make_binding(Uuid::nil());
        let b2 = make_binding(Uuid::nil());
        let b3 = make_binding(Uuid::nil());
        let b1_id = b1.id;
        let b2_id = b2.id;
        let b3_id = b3.id;

        let svc1 = make_service(host_id, ServiceCategory::ReverseProxy, "Nginx", vec![b1]);
        let svc2 = make_service(host_id, ServiceCategory::Development, "App", vec![b2]);
        let svc3 = make_service(host_id, ServiceCategory::Database, "Postgres", vec![b3]);

        let dep = make_dependency(
            DependencyType::RequestPath,
            DependencyMembers::Bindings {
                binding_ids: vec![b1_id, b2_id, b3_id],
            },
        );

        let services = vec![svc1, svc2, svc3];
        let deps = vec![dep];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &[],
            &[],
            &[],
            &services,
            &deps,
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );

        let builder = ApplicationBuilder;
        let (nodes, edges) = builder.build(
            &ctx,
            &GroupingConfig::from_request_options(&options.request, TopologyView::L3Logical),
        );

        // No app tags applied — all services land in the single "Ungrouped" container.
        // Subcontainer count varies with default element rules (e.g. ByServiceCategory).
        let element_count = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .count();
        let container_count = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
            .count();
        assert_eq!(element_count, 3, "Should have 3 service elements");
        assert!(
            container_count >= 1,
            "Should have at least 1 container (Ungrouped), got {}",
            container_count
        );

        // 2 request path edges: svc1→svc2, svc2→svc3
        let flow_edges: Vec<&Edge> = edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::RequestPath { .. }))
            .collect();
        assert_eq!(flow_edges.len(), 2);
    }

    #[test]
    fn test_hub_and_spoke_edges() {
        let host_id = Uuid::new_v4();

        let b1 = make_binding(Uuid::nil());
        let b2 = make_binding(Uuid::nil());
        let b3 = make_binding(Uuid::nil());
        let b1_id = b1.id;
        let b2_id = b2.id;
        let b3_id = b3.id;

        let svc1 = make_service(host_id, ServiceCategory::ReverseProxy, "LB", vec![b1]);
        let svc2 = make_service(host_id, ServiceCategory::Development, "App1", vec![b2]);
        let svc3 = make_service(host_id, ServiceCategory::Development, "App2", vec![b3]);

        let dep = make_dependency(
            DependencyType::HubAndSpoke,
            DependencyMembers::Bindings {
                binding_ids: vec![b1_id, b2_id, b3_id],
            },
        );

        let services = vec![svc1, svc2, svc3];
        let deps = vec![dep];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &[],
            &[],
            &[],
            &services,
            &deps,
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );

        let builder = ApplicationBuilder;
        let (_nodes, edges) = builder.build(
            &ctx,
            &GroupingConfig::from_request_options(&options.request, TopologyView::L3Logical),
        );

        // Hub (svc1) → svc2, svc1 → svc3
        let spoke_edges: Vec<&Edge> = edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::HubAndSpoke { .. }))
            .collect();
        assert_eq!(spoke_edges.len(), 2);
    }

    #[test]
    fn test_deduplicates_bindings_to_services() {
        let host_id = Uuid::new_v4();

        // Service with 2 bindings in the same dependency
        let b1 = make_binding(Uuid::nil());
        let b2 = make_binding(Uuid::nil());
        let b3 = make_binding(Uuid::nil());
        let b1_id = b1.id;
        let b2_id = b2.id;
        let b3_id = b3.id;

        let svc1 = make_service(
            host_id,
            ServiceCategory::ReverseProxy,
            "Nginx",
            vec![b1, b2],
        );
        let svc2 = make_service(host_id, ServiceCategory::Database, "DB", vec![b3]);

        // Dependency: b1 (svc1) → b2 (svc1) → b3 (svc2)
        // Should deduplicate to: svc1 → svc2
        let dep = make_dependency(
            DependencyType::RequestPath,
            DependencyMembers::Bindings {
                binding_ids: vec![b1_id, b2_id, b3_id],
            },
        );

        let services = vec![svc1, svc2];
        let deps = vec![dep];
        let options = TopologyOptions::default();

        let ctx = TopologyContext::new(
            &[],
            &[],
            &[],
            &services,
            &deps,
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );

        let builder = ApplicationBuilder;
        let (_nodes, edges) = builder.build(
            &ctx,
            &GroupingConfig::from_request_options(&options.request, TopologyView::L3Logical),
        );

        let flow_edges: Vec<&Edge> = edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::RequestPath { .. }))
            .collect();
        assert_eq!(
            flow_edges.len(),
            1,
            "Should deduplicate consecutive same-service bindings"
        );
    }

    #[test]
    fn test_element_rules_across_apps() {
        // 2 hosts with different app tags → 2 Application containers
        // Both services have a shared non-app tag "monitoring"
        // ByTag element rule should create subcontainers in BOTH containers
        let monitoring_tag_id = Uuid::new_v4();
        let app_tag_a_id = Uuid::new_v4();
        let app_tag_b_id = Uuid::new_v4();

        let host_a = Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: HostName::manual("host-a".to_string()),
                tags: vec![app_tag_a_id],
                ..Default::default()
            },
            ..Default::default()
        };
        let host_b = Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: HostName::manual("host-b".to_string()),
                tags: vec![app_tag_b_id],
                ..Default::default()
            },
            ..Default::default()
        };

        let b1 = make_binding(Uuid::nil());
        let b2 = make_binding(Uuid::nil());

        let mut svc_a = make_service(host_a.id, ServiceCategory::Development, "AppA", vec![b1]);
        svc_a.base.tags = vec![monitoring_tag_id];

        let mut svc_b = make_service(host_b.id, ServiceCategory::Development, "AppB", vec![b2]);
        svc_b.base.tags = vec![monitoring_tag_id];

        let app_tag_a = Tag {
            id: app_tag_a_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: TagBase {
                name: "Group A".to_string(),
                description: None,
                color: Color::Blue,
                organization_id: Uuid::new_v4(),
                tag_group: Some(crate::server::tags::r#impl::base::TagGroup::Application),
                icon: None,
            },
            ..Default::default()
        };
        let app_tag_b = Tag {
            id: app_tag_b_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: TagBase {
                name: "Group B".to_string(),
                description: None,
                color: Color::Green,
                organization_id: Uuid::new_v4(),
                tag_group: Some(crate::server::tags::r#impl::base::TagGroup::Application),
                icon: None,
            },
            ..Default::default()
        };

        let services = vec![svc_a, svc_b];
        let hosts = vec![host_a, host_b];
        let tags = vec![app_tag_a, app_tag_b];

        let mut options = TopologyOptions::default();
        options.request.container_rules.insert(
            TopologyView::Application,
            vec![IdentifiedRule::new(ContainerRule::ByApplication {
                tag_ids: vec![],
            })],
        );
        options.request.element_rules = vec![IdentifiedRule::new(ElementRule::ByTag {
            tag_ids: vec![monitoring_tag_id],
            title: Some("Monitored".to_string()),
        })];

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
            TopologyView::Application,
        );

        let builder = ApplicationBuilder;
        let (nodes, _edges) = builder.build(
            &ctx,
            &GroupingConfig::from_request_options(&options.request, TopologyView::Application),
        );

        // 2 AppGroup containers + 2 NestedTag subcontainers + 2 element nodes = 6
        let containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
            .collect();
        let nested: Vec<&Node> = containers
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::NestedTag,
                        ..
                    }
                )
            })
            .copied()
            .collect();
        assert_eq!(
            containers.len(),
            4,
            "Should have 2 AppGroup + 2 NestedTag containers"
        );
        assert_eq!(nested.len(), 2, "Should have 2 NestedTag subcontainers");

        // Both services should be inside their respective NestedTag subcontainers
        let elements: Vec<&Node> = nodes
            .iter()
            .filter(|n| matches!(n.node_type, NodeType::Element { .. }))
            .collect();
        assert_eq!(elements.len(), 2);

        for element in &elements {
            if let NodeType::Element { container_id, .. } = &element.node_type {
                assert!(
                    nested.iter().any(|n| n.id == *container_id),
                    "Element {} should be inside a NestedTag subcontainer, but container_id is {}",
                    element.id,
                    container_id
                );
            }
        }
    }

    #[test]
    fn test_bytag_rule_does_not_inherit_host_tags() {
        // Regression: ByTag element rules in the Application view used to match
        // services via host-tag inheritance, causing services to be grouped under
        // tags applied only to their host. Host-tag inheritance is reserved for
        // the top-level ByApplication grouping (see find_app_tag). Element rules
        // should match on the service's own tags, symmetric with WorkloadsBuilder.
        let monitoring_tag_id = Uuid::new_v4();
        let app_tag_a_id = Uuid::new_v4();
        let app_tag_b_id = Uuid::new_v4();

        // Both hosts have the monitoring tag AND their app tag
        let host_a = Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: HostName::manual("host-a".to_string()),
                tags: vec![app_tag_a_id, monitoring_tag_id],
                ..Default::default()
            },
            ..Default::default()
        };
        let host_b = Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: HostName::manual("host-b".to_string()),
                tags: vec![app_tag_b_id, monitoring_tag_id],
                ..Default::default()
            },
            ..Default::default()
        };

        let b1 = make_binding(Uuid::nil());
        let b2 = make_binding(Uuid::nil());

        // Services do NOT have the monitoring tag — they inherit from hosts
        let svc_a = make_service(host_a.id, ServiceCategory::Development, "AppA", vec![b1]);
        let svc_b = make_service(host_b.id, ServiceCategory::Development, "AppB", vec![b2]);

        let app_tag_a = Tag {
            id: app_tag_a_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: TagBase {
                name: "Group A".to_string(),
                description: None,
                color: Color::Blue,
                organization_id: Uuid::new_v4(),
                tag_group: Some(crate::server::tags::r#impl::base::TagGroup::Application),
                icon: None,
            },
            ..Default::default()
        };
        let app_tag_b = Tag {
            id: app_tag_b_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: TagBase {
                name: "Group B".to_string(),
                description: None,
                color: Color::Green,
                organization_id: Uuid::new_v4(),
                tag_group: Some(crate::server::tags::r#impl::base::TagGroup::Application),
                icon: None,
            },
            ..Default::default()
        };

        let services = vec![svc_a, svc_b];
        let hosts = vec![host_a, host_b];
        let tags = vec![app_tag_a, app_tag_b];

        let mut options = TopologyOptions::default();
        options.request.container_rules.insert(
            TopologyView::Application,
            vec![IdentifiedRule::new(ContainerRule::ByApplication {
                tag_ids: vec![],
            })],
        );
        // Default ByServiceCategory PLUS a ByTag rule
        options.request.element_rules = vec![
            IdentifiedRule::new(ElementRule::ByServiceCategory {
                categories: vec![ServiceCategory::DNS, ServiceCategory::ReverseProxy],
                title: Some("Infrastructure".to_string()),
                is_infra_rule: false,
            }),
            IdentifiedRule::new(ElementRule::ByTag {
                tag_ids: vec![monitoring_tag_id],
                title: Some("Monitored".to_string()),
            }),
        ];

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
            TopologyView::Application,
        );

        let builder = ApplicationBuilder;
        let (nodes, _edges) = builder.build(
            &ctx,
            &GroupingConfig::from_request_options(&options.request, TopologyView::Application),
        );

        // Services don't have monitoring_tag_id; only hosts do. ByTag rule
        // should produce no NestedTag subcontainers — host-tag inheritance
        // does not apply to element rules.
        let nested_tags: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::NestedTag,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(
            nested_tags.len(),
            0,
            "ByTag should not match services by inherited host tags"
        );

        // Services should still be grouped into Application containers by their host's app tag
        // (via find_app_tag, which is the intended host→service inheritance for ByApplication).
        let app_containers: Vec<&Node> = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.node_type,
                    NodeType::Container {
                        container_type: ContainerType::Application,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(
            app_containers.len(),
            2,
            "Should have one Application container per app tag"
        );
    }
}
