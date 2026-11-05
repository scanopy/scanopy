use super::*;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::services::r#impl::base::{Service, ServiceBase};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{DeviceTypeImplication, ServiceDefinition};
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::services::r#impl::virtualization::{
    DockerVirtualization, ServiceVirtualization,
};
use crate::server::shared::types::Color;
use crate::server::shared::types::metadata::{EntityMetadataProvider, TypeMetadataProvider};
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase, SubnetTypeAttributed};
use crate::server::tags::r#impl::base::{Tag, TagBase};
use crate::server::topology::service::context::TopologyContext;
use crate::server::topology::types::base::TopologyOptions;
use crate::server::topology::types::grouping::{ElementRule, IdentifiedRule};

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
    fn device_type(&self) -> DeviceTypeImplication {
        DeviceTypeImplication::Server
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::None
    }
}

fn make_host(name: &str, tags: Vec<Uuid>) -> Host {
    Host {
        id: Uuid::new_v4(),
        created_at: crate::server::shared::clock::now(),
        updated_at: crate::server::shared::clock::now(),
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
        created_at: crate::server::shared::clock::now(),
        updated_at: crate::server::shared::clock::now(),
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
        created_at: crate::server::shared::clock::now(),
        updated_at: crate::server::shared::clock::now(),
        base: TagBase {
            name: name.to_string(),
            description: None,
            color: Color::Yellow,
            organization_id: Uuid::new_v4(),
            tag_group_id: None,
            position: None,
            icon: None,
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

/// Runs the L3 element rules over one subnet and returns each element's container and the
/// subcontainers the rules created.
fn group_subnet(
    rules: Vec<ElementRule>,
    hosts: &[Host],
    services: &[Service],
    subnets: &[Subnet],
    tags: &[Tag],
    subnet_id: Uuid,
    children: &[SubnetChildData],
) -> (HashMap<Uuid, Uuid>, Vec<Node>) {
    let mut options = TopologyOptions::default();
    options.request.element_rules = rules.into_iter().map(IdentifiedRule::new).collect();
    let ctx = TopologyContext::new(
        hosts,
        &[],
        subnets,
        services,
        &[],
        &[],
        &[],
        &[],
        tags,
        &[],
        &options,
        crate::server::topology::types::views::TopologyView::L3Logical,
    );
    let mut nodes: Vec<Node> = children
        .iter()
        .map(|c| make_element_node(c.id, c.host_id, subnet_id))
        .collect();
    SubnetGraphBuilder::new().create_nested_group_containers(subnet_id, children, &ctx, &mut nodes);
    let (elements, containers): (Vec<Node>, Vec<Node>) = nodes
        .into_iter()
        .partition(|n| matches!(n.node_type, NodeType::Element { .. }));
    let placement = elements
        .iter()
        .map(|n| match n.node_type {
            NodeType::Element { container_id, .. } => (n.id, container_id),
            _ => unreachable!(),
        })
        .collect();
    (placement, containers)
}

#[test]
fn by_device_type_groups_each_device_type_present() {
    let phone = make_host("Mayas-iPhone", vec![]);
    let laptop = make_host("Mayas-MacBook-Pro", vec![]);
    let desktop = make_host("Sams-iMac", vec![]);
    let unidentified = make_host("plain-host", vec![]);
    let subnet_id = Uuid::new_v4();
    let children: Vec<SubnetChildData> = [&phone, &laptop, &desktop, &unidentified]
        .iter()
        .map(|h| make_subnet_child(Uuid::new_v4(), h.id))
        .collect();
    let hosts = vec![phone, laptop, desktop, unidentified];

    let (placement, containers) = group_subnet(
        vec![ElementRule::ByDeviceType],
        &hosts,
        &[],
        &[],
        &[],
        subnet_id,
        &children,
    );

    assert_eq!(
        containers.len(),
        4,
        "one subcontainer per device type present, Unidentified included"
    );
    let container_for = |device_type: DeviceType| {
        containers
            .iter()
            .find(|n| n.header.as_deref() == Some(device_type.name()))
            .unwrap_or_else(|| panic!("no subcontainer for {device_type:?}"))
    };
    let mobile = container_for(DeviceType::Mobile);
    let laptop = container_for(DeviceType::Laptop);
    let desktop = container_for(DeviceType::Desktop);
    let unidentified = container_for(DeviceType::Unidentified);
    let NodeType::Container {
        container_type,
        icon,
        parent_container_id,
        ..
    } = &laptop.node_type
    else {
        panic!("expected a container");
    };
    assert_eq!(*container_type, ContainerType::DeviceType);
    assert_eq!(
        icon.as_deref(),
        Some(DeviceType::Laptop.icon().to_string().as_str())
    );
    assert_eq!(*parent_container_id, Some(subnet_id));

    assert_eq!(placement[&children[0].id], mobile.id);
    assert_eq!(placement[&children[1].id], laptop.id);
    assert_eq!(placement[&children[2].id], desktop.id);
    assert_eq!(
        placement[&children[3].id], unidentified.id,
        "a host with no evidence gets the Unidentified group"
    );
}

#[test]
fn by_device_type_yields_to_an_earlier_tag_rule_and_wins_when_first() {
    let tag = make_tag("Critical");
    let phone = make_host("Mayas-iPhone", vec![tag.id]);
    let laptop = make_host("Mayas-MacBook-Pro", vec![]);
    let subnet_id = Uuid::new_v4();
    let children = vec![
        make_subnet_child(Uuid::new_v4(), phone.id),
        make_subnet_child(Uuid::new_v4(), laptop.id),
    ];
    let hosts = vec![phone, laptop];
    let tags = vec![tag.clone()];
    let by_tag = ElementRule::ByTag {
        tag_ids: vec![tag.id],
        title: None,
    };
    let container_type_of = |containers: &[Node], id: Uuid| {
        containers
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| match n.node_type {
                NodeType::Container { container_type, .. } => Some(container_type),
                _ => None,
            })
    };

    let (placement, containers) = group_subnet(
        vec![by_tag.clone(), ElementRule::ByDeviceType],
        &hosts,
        &[],
        &[],
        &tags,
        subnet_id,
        &children,
    );
    assert_eq!(
        container_type_of(&containers, placement[&children[0].id]),
        Some(ContainerType::NestedTag)
    );
    assert_eq!(
        container_type_of(&containers, placement[&children[1].id]),
        Some(ContainerType::DeviceType)
    );

    let (placement, containers) = group_subnet(
        vec![ElementRule::ByDeviceType, by_tag],
        &hosts,
        &[],
        &[],
        &tags,
        subnet_id,
        &children,
    );
    assert_eq!(
        container_type_of(&containers, placement[&children[0].id]),
        Some(ContainerType::DeviceType)
    );
}

#[test]
fn by_device_type_leaves_container_subnets_to_by_stack() {
    let docker_host = make_host("docker-host", vec![]);
    let mut container = make_service(docker_host.id, Box::new(ReverseProxyServiceDef));
    container.base.virtualization_metadata =
        Some(ServiceVirtualization::Docker(DockerVirtualization {
            container_name: None,
            container_id: None,
            compose_project: Some("media".to_string()),
            state: Default::default(),
        }));
    let bridge = Subnet {
        id: Uuid::new_v4(),
        base: SubnetBase {
            subnet_type: SubnetTypeAttributed::unattributed(SubnetType::DockerBridge),
            ..Default::default()
        },
        ..Default::default()
    };
    let child = SubnetChildData {
        id: Uuid::new_v4(),
        host_id: docker_host.id,
        ip_address_id: None,
        header: None,
    };
    let hosts = vec![docker_host];
    let services = vec![container];

    let (placement, containers) = group_subnet(
        vec![ElementRule::ByDeviceType, ElementRule::ByStack],
        &hosts,
        &services,
        std::slice::from_ref(&bridge),
        &[],
        bridge.id,
        std::slice::from_ref(&child),
    );

    assert_eq!(containers.len(), 1);
    assert!(matches!(
        containers[0].node_type,
        NodeType::Container {
            container_type: ContainerType::Stack,
            ..
        }
    ));
    assert_eq!(placement[&child.id], containers[0].id);
}
