use std::collections::HashMap;
use uuid::Uuid;

use super::{
    context::TopologyContext, edge_builder::EdgeBuilder, subnet_graph_builder::SubnetGraphBuilder,
    view::ViewBuilder,
};
use crate::server::shared::types::metadata::EntityMetadataProvider;
use crate::server::topology::types::{
    edges::Edge,
    grouping::GroupingConfig,
    nodes::{ContainerType, Node, NodeType},
};

pub struct L3Builder;

impl ViewBuilder for L3Builder {
    fn build(&self, ctx: &TopologyContext, grouping: &GroupingConfig) -> (Vec<Node>, Vec<Edge>) {
        // Create all edges
        let mut all_edges = Vec::new();

        all_edges.extend(EdgeBuilder::create_interface_edges(ctx));
        all_edges.extend(EdgeBuilder::create_dependency_edges(ctx));
        all_edges.extend(EdgeBuilder::create_vm_host_edges(ctx));
        all_edges.extend(EdgeBuilder::create_containerized_service_edges(
            ctx, grouping,
        ));
        all_edges.extend(EdgeBuilder::create_same_container_edges(ctx));
        all_edges.extend(EdgeBuilder::create_physical_link_edges(ctx));

        // Create nodes (positions zeroed — frontend computes layout via elkjs)
        let mut subnet_builder = SubnetGraphBuilder::new();
        let (subnet_ids, mut child_nodes) =
            subnet_builder.create_subnet_child_nodes(ctx, &mut all_edges, grouping);

        let mut subnet_nodes = subnet_builder.create_subnet_nodes(ctx, &subnet_ids);

        // Set icon and color on container nodes from subnet metadata
        let subnet_map: HashMap<Uuid, crate::server::subnets::r#impl::types::SubnetType> = ctx
            .subnets
            .iter()
            .map(|s| (s.id, s.base.subnet_type()))
            .collect();
        for node in &mut subnet_nodes {
            if let NodeType::Container {
                ref mut icon,
                ref mut color,
                ..
            } = node.node_type
                && let Some(subnet_type) = subnet_map.get(&node.id)
            {
                *icon = Some(subnet_type.icon().to_string());
                *color = Some(subnet_type.color());
            }
        }

        clear_stack_logos_repeating_bridge(&subnet_nodes, &mut child_nodes);

        let all_nodes: Vec<Node> = subnet_nodes.into_iter().chain(child_nodes).collect();
        (all_nodes, all_edges)
    }
}

/// Drop a Stack subcontainer's runtime logo when the bridge subnet it sits in already shows
/// the same one: every stack inside a Docker bridge is a Docker stack. A stack on any other
/// subnet, or inside a bridge whose runtime didn't resolve, keeps its logo.
fn clear_stack_logos_repeating_bridge(subnet_nodes: &[Node], child_nodes: &mut [Node]) {
    let subnet_logos: HashMap<Uuid, &str> = subnet_nodes
        .iter()
        .filter_map(|node| match &node.node_type {
            NodeType::Container {
                associated_service_definition: Some(definition),
                ..
            } => Some((node.id, definition.as_str())),
            _ => None,
        })
        .collect();
    for node in child_nodes.iter_mut() {
        if let NodeType::Container {
            container_type: ContainerType::Stack,
            parent_container_id: Some(parent_id),
            associated_service_definition,
            ..
        } = &mut node.node_type
            && associated_service_definition.as_deref() == subnet_logos.get(parent_id).copied()
        {
            *associated_service_definition = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::{Host, HostBase};
    use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
    use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
    use crate::server::services::r#impl::base::{Service, ServiceBase};
    use crate::server::services::r#impl::categories::ServiceCategory;
    use crate::server::services::r#impl::definitions::{DeviceTypeImplication, ServiceDefinition};
    use crate::server::services::r#impl::patterns::Pattern;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::types::Color;
    use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
    use crate::server::subnets::r#impl::base::{SubnetName, SubnetTypeAttributed};
    use crate::server::tags::r#impl::base::{Tag, TagBase};
    use crate::server::topology::service::context::TopologyContext;
    use crate::server::topology::service::view::ViewBuilder;
    use crate::server::topology::types::base::TopologyOptions;
    use crate::server::topology::types::grouping::{ElementRule, IdentifiedRule};
    use crate::server::topology::types::nodes::{ContainerType, NodeType};

    use cidr::{IpCidr, Ipv4Cidr};
    use std::net::Ipv4Addr;

    #[derive(PartialEq, Eq, Hash, Clone)]
    struct TestServiceDef;

    impl ServiceDefinition for TestServiceDef {
        fn name(&self) -> &'static str {
            "TestService"
        }
        fn description(&self) -> &'static str {
            "Test"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::Development
        }
        fn device_type(&self) -> DeviceTypeImplication {
            DeviceTypeImplication::Server
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    #[test]
    fn test_l3_bytag_ignores_service_tags() {
        // Regression: L3Builder used to match IPAddress elements on service
        // tags via inheritance. Now only the host's own tags count.
        let site_id = Uuid::new_v4();
        let subnet_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let ip_address_id = Uuid::new_v4();
        let tag_id = Uuid::new_v4();

        let host = Host {
            id: host_id,
            created_at: crate::server::shared::clock::now(),
            updated_at: crate::server::shared::clock::now(),
            base: HostBase {
                name: HostName::manual("test-host".to_string()),
                site_id,
                tags: vec![], // NO host tags
                ..Default::default()
            },
            ..Default::default()
        };

        let subnet = Subnet {
            id: subnet_id,
            created_at: crate::server::shared::clock::now(),
            updated_at: crate::server::shared::clock::now(),
            base: SubnetBase {
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                site_id,
                name: SubnetName::unattributed("test-subnet".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let ip_address = IPAddress {
            id: ip_address_id,
            created_at: crate::server::shared::clock::now(),
            updated_at: crate::server::shared::clock::now(),
            base: IPAddressBase {
                site_id,
                host_id,
                subnet_id,
                ip_address: std::net::IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
                ..Default::default()
            },
            ..Default::default()
        };

        let service = Service {
            id: Uuid::new_v4(),
            created_at: crate::server::shared::clock::now(),
            updated_at: crate::server::shared::clock::now(),
            base: ServiceBase {
                host_id,
                site_id,
                service_definition: Box::new(TestServiceDef),
                tags: vec![tag_id], // Service HAS the tag
                ..Default::default()
            },
            ..Default::default()
        };

        let tag = Tag {
            id: tag_id,
            created_at: crate::server::shared::clock::now(),
            updated_at: crate::server::shared::clock::now(),
            base: TagBase {
                name: "ServiceTag".to_string(),
                description: None,
                color: Color::Orange,
                organization_id: Uuid::new_v4(),
                tag_group_id: None,
                position: None,
                icon: None,
                source: crate::server::shared::types::entities::EntitySource::Manual,
            },
            ..Default::default()
        };

        let mut options = TopologyOptions::default();
        options.request.element_rules = vec![IdentifiedRule::new(ElementRule::ByTag {
            tag_ids: vec![tag_id],
            title: Some("Tagged".to_string()),
        })];

        let hosts = vec![host];
        let ip_addresses = vec![ip_address];
        let subnets = vec![subnet];
        let services = vec![service];
        let tags = vec![tag];

        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
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

        let grouping = GroupingConfig::from_request_options(
            &ctx.options.request,
            crate::server::topology::types::views::TopologyView::L3Logical,
        );
        let builder = L3Builder;
        let (nodes, _edges) = builder.build(&ctx, &grouping);

        // ByTag on a tag applied only to a service (not the host) should NOT
        // create a NestedTag container — IP addresses resolve their tag target
        // to the owning Host, and the host has no tags.
        assert!(
            !nodes.iter().any(|n| matches!(
                n.node_type,
                NodeType::Container {
                    container_type: ContainerType::NestedTag,
                    ..
                }
            )),
            "ByTag should not match IP addresses on service-only tags"
        );
    }

    #[test]
    fn test_l3_merges_container_bridges_per_runtime() {
        use crate::server::subnets::r#impl::types::SubnetType;
        use crate::server::topology::types::grouping::ContainerRule;
        use crate::server::topology::types::views::TopologyView;

        let site_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        let host = Host {
            id: host_id,
            base: HostBase {
                name: HostName::manual("bridge-host".to_string()),
                site_id,
                ..Default::default()
            },
            ..Default::default()
        };

        // Same host runs two Podman bridges and one Docker bridge. Per-runtime
        // merging must collapse the two Podman bridges into one container and
        // keep the Docker bridge as a separate one.
        let make_bridge = |st: SubnetType, cidr: Ipv4Cidr, name: &str| Subnet {
            id: Uuid::new_v4(),
            base: SubnetBase {
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(cidr)),
                    AttributeSource::DaemonSelfReport,
                ),
                site_id,
                name: SubnetName::unattributed(name.to_string()),
                subnet_type: SubnetTypeAttributed::unattributed(st),
                ..Default::default()
            },
            ..Default::default()
        };
        let podman_a = make_bridge(
            SubnetType::PodmanBridge,
            Ipv4Cidr::new(Ipv4Addr::new(10, 88, 0, 0), 16).unwrap(),
            "podman0",
        );
        let podman_b = make_bridge(
            SubnetType::PodmanBridge,
            Ipv4Cidr::new(Ipv4Addr::new(10, 89, 0, 0), 24).unwrap(),
            "podman1",
        );
        let docker = make_bridge(
            SubnetType::DockerBridge,
            Ipv4Cidr::new(Ipv4Addr::new(172, 17, 0, 0), 16).unwrap(),
            "docker0",
        );

        let make_ip = |subnet_id: Uuid, addr: Ipv4Addr| IPAddress {
            id: Uuid::new_v4(),
            base: IPAddressBase {
                site_id,
                host_id,
                subnet_id,
                ip_address: std::net::IpAddr::V4(addr),
                ..Default::default()
            },
            ..Default::default()
        };
        let ip_addresses = vec![
            make_ip(podman_a.id, Ipv4Addr::new(10, 88, 0, 2)),
            make_ip(podman_b.id, Ipv4Addr::new(10, 89, 0, 2)),
            make_ip(docker.id, Ipv4Addr::new(172, 17, 0, 2)),
        ];

        let mut options = TopologyOptions::default();
        options.request.container_rules.insert(
            TopologyView::L3Logical,
            vec![
                IdentifiedRule::new(ContainerRule::BySubnet),
                IdentifiedRule::new(ContainerRule::MergeContainerBridges),
            ],
        );

        let hosts = vec![host];
        let subnets = vec![podman_a, podman_b, docker];

        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );
        let grouping =
            GroupingConfig::from_request_options(&ctx.options.request, TopologyView::L3Logical);
        let (nodes, _edges) = L3Builder.build(&ctx, &grouping);

        let headers: Vec<&str> = nodes.iter().filter_map(|n| n.header.as_deref()).collect();

        let podman_headers: Vec<&&str> = headers
            .iter()
            .filter(|h| h.starts_with("Podman Bridge:"))
            .collect();
        assert_eq!(
            podman_headers.len(),
            1,
            "the two Podman bridges should merge into one container, got {headers:?}"
        );
        // Both Podman CIDRs are consolidated into that single container's header.
        assert!(podman_headers[0].contains("10.88"));
        assert!(podman_headers[0].contains("10.89"));

        assert_eq!(
            headers
                .iter()
                .filter(|h| h.starts_with("Docker Bridge:"))
                .count(),
            1,
            "the Docker bridge stays a separate container, got {headers:?}"
        );
    }

    /// A physical link's endpoints are IP addresses, and `add_edges_to_graph` drops — silently —
    /// any edge whose endpoint is not a node. A host with several addresses is where that is
    /// easiest to get wrong: the link resolves through the interface's own address, which has to
    /// be the one L3 renders, not a bridge address that got merged into another box.
    #[test]
    fn l3_physical_link_endpoints_are_rendered_nodes() {
        use crate::server::interface_neighbors::r#impl::base::{InterfaceNeighborRow, Neighbor};
        use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
        use crate::server::subnets::r#impl::types::SubnetType;
        use crate::server::topology::types::edges::EdgeType;
        use crate::server::topology::types::views::TopologyView;

        let site_id = Uuid::new_v4();
        let mgmt_subnet_id = Uuid::new_v4();
        let servers_subnet_id = Uuid::new_v4();
        let bridge_subnet_id = Uuid::new_v4();

        let make_host = |name: &str| Host {
            id: Uuid::new_v4(),
            created_at: crate::server::shared::clock::now(),
            updated_at: crate::server::shared::clock::now(),
            base: HostBase {
                name: HostName::manual(name.to_string()),
                site_id,
                ..Default::default()
            },
            ..Default::default()
        };
        let make_subnet = |id: Uuid, third: u8, subnet_type: SubnetType| Subnet {
            id,
            created_at: crate::server::shared::clock::now(),
            updated_at: crate::server::shared::clock::now(),
            base: SubnetBase {
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, third, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                site_id,
                name: SubnetName::unattributed(format!("subnet-{third}")),
                subnet_type: SubnetTypeAttributed::unattributed(subnet_type),
                ..Default::default()
            },
            ..Default::default()
        };
        let make_ip = |host_id: Uuid, subnet_id: Uuid, third: u8, last: u8| IPAddress {
            id: Uuid::new_v4(),
            created_at: crate::server::shared::clock::now(),
            updated_at: crate::server::shared::clock::now(),
            base: IPAddressBase {
                site_id,
                host_id,
                subnet_id,
                ip_address: std::net::IpAddr::V4(Ipv4Addr::new(10, 0, third, last)),
                ..Default::default()
            },
            ..Default::default()
        };
        let make_if = |host_id: Uuid, if_index: i32, ip_address_id: Uuid| Interface {
            id: Uuid::new_v4(),
            created_at: crate::server::shared::clock::now(),
            updated_at: crate::server::shared::clock::now(),
            base: InterfaceBase {
                host_id,
                site_id,
                if_index: Some(if_index),
                if_descr: Some(format!("eth{if_index}")),
                if_type: Some(6),
                ip_address_id: Some(ip_address_id),
                ..Default::default()
            },
            ..Default::default()
        };

        let switch = make_host("switch");
        let container_host = make_host("docker-host");

        // The container host carries a bridge address alongside its real one, so the graph has
        // to pick the right address for the cable.
        let switch_ip = make_ip(switch.id, mgmt_subnet_id, 1, 3);
        let host_ip = make_ip(container_host.id, servers_subnet_id, 20, 20);
        let host_bridge_ip = make_ip(container_host.id, bridge_subnet_id, 30, 1);

        let host_if = make_if(container_host.id, 1, host_ip.id);
        let switch_if = make_if(switch.id, 5, switch_ip.id);
        let neighbours = vec![InterfaceNeighborRow {
            id: Uuid::new_v4(),
            interface_id: switch_if.id,
            neighbor: Neighbor::Interface(host_if.id),
            neighbor_seen_at: None,
        }];

        let hosts = vec![switch, container_host];
        let ip_addresses = vec![switch_ip, host_ip, host_bridge_ip];
        let subnets = vec![
            make_subnet(mgmt_subnet_id, 1, SubnetType::default()),
            make_subnet(servers_subnet_id, 20, SubnetType::default()),
            make_subnet(bridge_subnet_id, 30, SubnetType::DockerBridge),
        ];
        let interfaces = vec![switch_if, host_if];

        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            &hosts,
            &ip_addresses,
            &subnets,
            &[],
            &[],
            &[],
            &[],
            &interfaces,
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        )
        .with_neighbours(&neighbours);
        let grouping =
            GroupingConfig::from_request_options(&ctx.options.request, TopologyView::L3Logical);
        let (nodes, edges) = L3Builder.build(&ctx, &grouping);

        let node_ids: std::collections::HashSet<Uuid> = nodes.iter().map(|n| n.id).collect();
        let links: Vec<&Edge> = edges
            .iter()
            .filter(|e| matches!(e.edge_type, EdgeType::PhysicalLink { .. }))
            .collect();

        assert_eq!(links.len(), 1, "expected the one cable, got {links:?}");
        for link in links {
            assert!(
                node_ids.contains(&link.source) && node_ids.contains(&link.target),
                "physical link {:?} → {:?} has no node to land on, so the graph drops it",
                link.source,
                link.target
            );
        }
    }

    mod bridge_runtime_logo {
        use super::*;
        use crate::server::bindings::r#impl::base::Binding;
        use crate::server::services::definitions::docker_daemon::Docker;
        use crate::server::services::definitions::podman::Podman;
        use crate::server::services::r#impl::virtualization::{
            DockerVirtualization, ServiceVirtualization,
        };
        use crate::server::subnets::r#impl::types::SubnetType;
        use crate::server::topology::types::grouping::ContainerRule;
        use crate::server::topology::types::views::TopologyView;

        fn host() -> Host {
            Host {
                id: Uuid::new_v4(),
                base: HostBase {
                    name: HostName::manual("container-host".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            }
        }

        fn subnet(subnet_type: SubnetType, runtime: Option<Uuid>, second_octet: u8) -> Subnet {
            Subnet {
                id: Uuid::new_v4(),
                base: SubnetBase {
                    cidr: SubnetCidr::new(
                        SubnetCidrValue(IpCidr::V4(
                            Ipv4Cidr::new(Ipv4Addr::new(172, second_octet, 0, 0), 16).unwrap(),
                        )),
                        AttributeSource::DaemonSelfReport,
                    ),
                    name: SubnetName::unattributed(format!("net{second_octet}")),
                    subnet_type: SubnetTypeAttributed::unattributed(subnet_type),
                    virtualization_service_id: runtime,
                    ..Default::default()
                },
                ..Default::default()
            }
        }

        /// The host's address on `subnet`. The builder groups by `subnet_id`, so the
        /// address value itself doesn't need to fall inside the CIDR.
        fn ip(host: &Host, subnet: &Subnet) -> IPAddress {
            IPAddress {
                id: Uuid::new_v4(),
                base: IPAddressBase {
                    host_id: host.id,
                    subnet_id: subnet.id,
                    ip_address: std::net::IpAddr::V4(Ipv4Addr::new(172, 16, 0, 2)),
                    ..Default::default()
                },
                ..Default::default()
            }
        }

        fn service(host: &Host, definition: Box<dyn ServiceDefinition>) -> Service {
            Service {
                id: Uuid::new_v4(),
                base: ServiceBase {
                    host_id: host.id,
                    service_definition: definition,
                    ..Default::default()
                },
                ..Default::default()
            }
        }

        /// A container in compose project "media", bound to `ip`, so ByStack groups it.
        fn stack_member(host: &Host, ip: &IPAddress, runtime_id: Uuid) -> Service {
            let mut svc = service(host, Box::new(TestServiceDef));
            svc.base.bindings = vec![Binding::new_ip_address_serviceless(ip.id)];
            svc.base.virtualization_service_id = Some(runtime_id);
            svc.base.virtualization_metadata =
                Some(ServiceVirtualization::Docker(DockerVirtualization {
                    state: Default::default(),
                    container_name: Some("jellyfin".to_string()),
                    container_id: Some("abc123".to_string()),
                    compose_project: Some("media".to_string()),
                }));
            svc
        }

        fn build(
            hosts: &[Host],
            subnets: &[Subnet],
            ip_addresses: &[IPAddress],
            services: &[Service],
            by_stack: bool,
        ) -> Vec<Node> {
            let mut options = TopologyOptions::default();
            options.request.container_rules.insert(
                TopologyView::L3Logical,
                vec![
                    IdentifiedRule::new(ContainerRule::BySubnet),
                    IdentifiedRule::new(ContainerRule::MergeContainerBridges),
                ],
            );
            options.request.element_rules = if by_stack {
                vec![IdentifiedRule::new(ElementRule::ByStack)]
            } else {
                vec![]
            };
            let ctx = TopologyContext::new(
                hosts,
                ip_addresses,
                subnets,
                services,
                &[],
                &[],
                &[],
                &[],
                &[],
                &[],
                &options,
                TopologyView::L3Logical,
            );
            let grouping =
                GroupingConfig::from_request_options(&ctx.options.request, TopologyView::L3Logical);
            L3Builder.build(&ctx, &grouping).0
        }

        fn container(nodes: &[Node], id: Uuid) -> &NodeType {
            &nodes
                .iter()
                .find(|n| n.id == id)
                .expect("container node")
                .node_type
        }

        fn logo(nodes: &[Node], id: Uuid) -> Option<&str> {
            match container(nodes, id) {
                NodeType::Container {
                    associated_service_definition,
                    ..
                } => associated_service_definition.as_deref(),
                _ => panic!("expected container"),
            }
        }

        fn stack_logo_in(nodes: &[Node], subnet_id: Uuid) -> Option<&str> {
            nodes
                .iter()
                .find_map(|n| match &n.node_type {
                    NodeType::Container {
                        container_type: ContainerType::Stack,
                        parent_container_id: Some(parent),
                        associated_service_definition,
                        ..
                    } if *parent == subnet_id => Some(associated_service_definition.as_deref()),
                    _ => None,
                })
                .expect("stack subcontainer in subnet")
        }

        #[test]
        fn bridges_take_their_runtime_logo_and_other_subnets_do_not() {
            let h = host();
            let docker = service(&h, Box::new(Docker));
            let podman = service(&h, Box::new(Podman));
            let docker_bridge = subnet(SubnetType::DockerBridge, Some(docker.id), 17);
            let podman_bridge = subnet(SubnetType::PodmanBridge, Some(podman.id), 88);
            // A non-bridge subnet linked to a runtime still keeps its own icon.
            let macvlan = subnet(SubnetType::MacVlan, Some(docker.id), 30);
            let ips = [
                ip(&h, &docker_bridge),
                ip(&h, &podman_bridge),
                ip(&h, &macvlan),
            ];

            let nodes = build(
                &[h],
                &[
                    docker_bridge.clone(),
                    podman_bridge.clone(),
                    macvlan.clone(),
                ],
                &ips,
                &[docker, podman],
                false,
            );

            assert_eq!(logo(&nodes, docker_bridge.id), Some("Docker"));
            assert_eq!(logo(&nodes, podman_bridge.id), Some("Podman"));
            assert_eq!(logo(&nodes, macvlan.id), None);
        }

        #[test]
        fn bridge_without_resolvable_runtime_keeps_icon() {
            let (h1, h2, h3) = (host(), host(), host());
            let no_logo_runtime = service(&h3, Box::new(TestServiceDef));
            let unlinked = subnet(SubnetType::DockerBridge, None, 17);
            let missing_service = subnet(SubnetType::DockerBridge, Some(Uuid::new_v4()), 18);
            let logoless = subnet(SubnetType::DockerBridge, Some(no_logo_runtime.id), 19);
            let ips = [
                ip(&h1, &unlinked),
                ip(&h2, &missing_service),
                ip(&h3, &logoless),
            ];

            let nodes = build(
                &[h1, h2, h3],
                &[unlinked.clone(), missing_service.clone(), logoless.clone()],
                &ips,
                &[no_logo_runtime],
                false,
            );

            for bridge in [&unlinked, &missing_service, &logoless] {
                assert_eq!(logo(&nodes, bridge.id), None);
                assert!(matches!(
                    container(&nodes, bridge.id),
                    NodeType::Container { icon: Some(i), .. }
                        if *i == SubnetType::DockerBridge.icon().to_string()
                ));
            }
        }

        #[test]
        fn merged_bridges_show_logo_only_when_every_bridge_resolves_to_one_runtime() {
            let (shared, partial) = (host(), host());
            let docker_a = service(&shared, Box::new(Docker));
            let docker_b = service(&shared, Box::new(Docker));
            let docker_c = service(&partial, Box::new(Docker));
            // Two Docker daemons on one host still name one runtime.
            let shared_0 = subnet(SubnetType::DockerBridge, Some(docker_a.id), 17);
            let shared_1 = subnet(SubnetType::DockerBridge, Some(docker_b.id), 18);
            let partial_0 = subnet(SubnetType::DockerBridge, Some(docker_c.id), 19);
            let partial_1 = subnet(SubnetType::DockerBridge, None, 20);
            let ips = [
                ip(&shared, &shared_0),
                ip(&shared, &shared_1),
                ip(&partial, &partial_0),
                ip(&partial, &partial_1),
            ];
            let subnets = [shared_0, shared_1, partial_0, partial_1];

            let nodes = build(
                &[shared, partial],
                &subnets,
                &ips,
                &[docker_a, docker_b, docker_c],
                false,
            );

            let merged_logos: Vec<Option<&str>> = nodes
                .iter()
                .filter(|n| {
                    n.header
                        .as_deref()
                        .is_some_and(|h| h.starts_with("Docker Bridge:") && h.contains(", "))
                })
                .map(|n| logo(&nodes, n.id))
                .collect();
            assert_eq!(merged_logos.len(), 2, "one merged container per host");
            assert!(merged_logos.contains(&Some("Docker")));
            assert!(merged_logos.contains(&None));
        }

        #[test]
        fn stack_logo_drops_only_where_its_bridge_already_shows_it() {
            let (in_bridge, on_macvlan, in_unresolved) = (host(), host(), host());
            let docker_1 = service(&in_bridge, Box::new(Docker));
            let docker_2 = service(&on_macvlan, Box::new(Docker));
            let docker_3 = service(&in_unresolved, Box::new(Docker));
            let bridge = subnet(SubnetType::DockerBridge, Some(docker_1.id), 17);
            let macvlan = subnet(SubnetType::MacVlan, None, 30);
            let unresolved = subnet(SubnetType::DockerBridge, None, 18);
            let bridge_ip = ip(&in_bridge, &bridge);
            let macvlan_ip = ip(&on_macvlan, &macvlan);
            let unresolved_ip = ip(&in_unresolved, &unresolved);
            let services = [
                stack_member(&in_bridge, &bridge_ip, docker_1.id),
                stack_member(&on_macvlan, &macvlan_ip, docker_2.id),
                stack_member(&in_unresolved, &unresolved_ip, docker_3.id),
                docker_1,
                docker_2,
                docker_3,
            ];

            let nodes = build(
                &[in_bridge, on_macvlan, in_unresolved],
                &[bridge.clone(), macvlan.clone(), unresolved.clone()],
                &[bridge_ip, macvlan_ip, unresolved_ip],
                &services,
                true,
            );

            assert_eq!(logo(&nodes, bridge.id), Some("Docker"));
            assert_eq!(stack_logo_in(&nodes, bridge.id), None);
            assert_eq!(stack_logo_in(&nodes, macvlan.id), Some("Docker"));
            assert_eq!(stack_logo_in(&nodes, unresolved.id), Some("Docker"));
        }
    }
}
