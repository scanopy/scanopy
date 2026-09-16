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
    nodes::{Node, NodeType},
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
        let (subnet_ids, child_nodes) =
            subnet_builder.create_subnet_child_nodes(ctx, &mut all_edges, grouping);

        let mut subnet_nodes = subnet_builder.create_subnet_nodes(ctx, &subnet_ids);

        // Set icon and color on container nodes from subnet metadata
        let subnet_map: HashMap<Uuid, &crate::server::subnets::r#impl::types::SubnetType> = ctx
            .subnets
            .iter()
            .map(|s| (s.id, &s.base.subnet_type))
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

        let all_nodes: Vec<Node> = subnet_nodes.into_iter().chain(child_nodes).collect();
        (all_nodes, all_edges)
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
    use crate::server::services::r#impl::definitions::ServiceDefinition;
    use crate::server::services::r#impl::patterns::Pattern;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::types::Color;
    use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
    use crate::server::tags::r#impl::base::{Tag, TagBase};
    use crate::server::topology::service::context::TopologyContext;
    use crate::server::topology::service::view::ViewBuilder;
    use crate::server::topology::types::base::TopologyOptions;
    use crate::server::topology::types::grouping::{ElementRule, IdentifiedRule};
    use crate::server::topology::types::nodes::{ContainerType, NodeType};
    use chrono::Utc;
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
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    #[test]
    fn test_l3_bytag_ignores_service_tags() {
        // Regression: L3Builder used to match IPAddress elements on service
        // tags via inheritance. Now only the host's own tags count.
        let network_id = Uuid::new_v4();
        let subnet_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();
        let ip_address_id = Uuid::new_v4();
        let tag_id = Uuid::new_v4();

        let host = Host {
            id: host_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: HostName::manual("test-host".to_string()),
                network_id,
                tags: vec![], // NO host tags
                ..Default::default()
            },
            ..Default::default()
        };

        let subnet = Subnet {
            id: subnet_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: SubnetBase {
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, 0, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                name: "test-subnet".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        let ip_address = IPAddress {
            id: ip_address_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: IPAddressBase {
                network_id,
                host_id,
                subnet_id,
                ip_address: std::net::IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
                ..Default::default()
            },
            ..Default::default()
        };

        let service = Service {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: ServiceBase {
                host_id,
                network_id,
                service_definition: Box::new(TestServiceDef),
                tags: vec![tag_id], // Service HAS the tag
                ..Default::default()
            },
            ..Default::default()
        };

        let tag = Tag {
            id: tag_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: TagBase {
                name: "ServiceTag".to_string(),
                description: None,
                color: Color::Orange,
                organization_id: Uuid::new_v4(),
                is_application: false,
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

        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        let host = Host {
            id: host_id,
            base: HostBase {
                name: HostName::manual("bridge-host".to_string()),
                network_id,
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
                network_id,
                name: name.to_string(),
                subnet_type: st,
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
                network_id,
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

        let network_id = Uuid::new_v4();
        let mgmt_subnet_id = Uuid::new_v4();
        let servers_subnet_id = Uuid::new_v4();
        let bridge_subnet_id = Uuid::new_v4();

        let make_host = |name: &str| Host {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: HostBase {
                name: HostName::manual(name.to_string()),
                network_id,
                ..Default::default()
            },
            ..Default::default()
        };
        let make_subnet = |id: Uuid, third: u8, subnet_type: SubnetType| Subnet {
            id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: SubnetBase {
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(
                        Ipv4Cidr::new(Ipv4Addr::new(10, 0, third, 0), 24).unwrap(),
                    )),
                    AttributeSource::DaemonSelfReport,
                ),
                network_id,
                name: format!("subnet-{third}"),
                subnet_type,
                ..Default::default()
            },
            ..Default::default()
        };
        let make_ip = |host_id: Uuid, subnet_id: Uuid, third: u8, last: u8| IPAddress {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: IPAddressBase {
                network_id,
                host_id,
                subnet_id,
                ip_address: std::net::IpAddr::V4(Ipv4Addr::new(10, 0, third, last)),
                ..Default::default()
            },
            ..Default::default()
        };
        let make_if = |host_id: Uuid, if_index: i32, ip_address_id: Uuid| Interface {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            base: InterfaceBase {
                host_id,
                network_id,
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
}
