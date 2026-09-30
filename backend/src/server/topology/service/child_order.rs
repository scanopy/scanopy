use std::cmp::Ordering;
use std::collections::HashMap;
use std::net::IpAddr;

use uuid::Uuid;

use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::topology::service::context::TopologyContext;
use crate::server::topology::types::grouping::ElementSort;
use crate::server::topology::types::nodes::{ElementEntityType, Node, NodeType};

/// What a node is ordered by under one `ElementSort`. Every node in a sort carries the same
/// variant, so the derived cross-variant order never decides anything.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SortKey {
    Address(IpAddr),
    Index(i32),
    Name(NaturalKey),
    Category(String, NaturalKey),
}

/// A string compared with runs of digits taken as numbers, so `eth2` comes before `eth10`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NaturalKey(Vec<Chunk>);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Chunk {
    /// Digits without leading zeros, compared by length first so any length compares correctly.
    Number(usize, String),
    Text(String),
}

impl NaturalKey {
    fn new(value: &str) -> Self {
        let mut chunks = Vec::new();
        let mut chars = value.chars().peekable();
        while let Some(&c) = chars.peek() {
            let is_digit = c.is_ascii_digit();
            let mut run = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() != is_digit {
                    break;
                }
                run.push(c);
                chars.next();
            }
            chunks.push(if is_digit {
                let digits = run.trim_start_matches('0').to_string();
                Chunk::Number(digits.len(), digits)
            } else {
                Chunk::Text(run.to_lowercase())
            });
        }
        Self(chunks)
    }
}

/// Order every node that sits inside a container by `sort`.
///
/// An element is keyed by its own entity: its address, interface index, name or service
/// category. A subcontainer (a tag or application group, a stack, a hypervisor, a VLAN) takes the
/// lowest key among the elements it holds, so it lands where its first member would, expanded or
/// collapsed. Nodes without a key go last, by header then id, so the order is stable across
/// builds. Top-level containers keep their positions.
///
/// Knows nothing about views: `GroupingConfig` has already dropped a sort the view cannot use.
pub fn order_children(nodes: &mut [Node], sort: ElementSort, ctx: &TopologyContext) {
    if sort == ElementSort::Layout {
        return;
    }

    let own_keys: HashMap<Uuid, SortKey> = {
        let lookups = Lookups::new(ctx);
        nodes
            .iter()
            .filter_map(|node| Some((node.id, lookups.element_key(node, sort)?)))
            .collect()
    };

    let mut parent_of: HashMap<Uuid, Uuid> = HashMap::new();
    for node in nodes.iter() {
        if let Some(parent) = parent_id(node) {
            parent_of.insert(node.id, parent);
        }
    }

    // Each keyed element lowers every ancestor's key to its own, which gives each container the
    // minimum over all its descendants.
    let mut keys = own_keys.clone();
    for (id, key) in &own_keys {
        let mut current = *id;
        while let Some(&parent) = parent_of.get(&current) {
            match keys.get(&parent) {
                Some(existing) if existing <= key => break,
                _ => {
                    keys.insert(parent, key.clone());
                }
            }
            current = parent;
        }
    }

    let slots: Vec<usize> = (0..nodes.len())
        .filter(|&i| parent_of.contains_key(&nodes[i].id))
        .collect();
    let mut children: Vec<Node> = slots.iter().map(|&i| nodes[i].clone()).collect();
    children.sort_by(|a, b| compare(a, b, &keys));
    for (slot, child) in slots.into_iter().zip(children) {
        nodes[slot] = child;
    }
}

fn compare(a: &Node, b: &Node, keys: &HashMap<Uuid, SortKey>) -> Ordering {
    match (keys.get(&a.id), keys.get(&b.id)) {
        (Some(ka), Some(kb)) => ka.cmp(kb),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
    .then_with(|| {
        let ha = a.header.as_deref().map(NaturalKey::new);
        let hb = b.header.as_deref().map(NaturalKey::new);
        ha.cmp(&hb)
    })
    .then_with(|| a.id.cmp(&b.id))
}

fn parent_id(node: &Node) -> Option<Uuid> {
    match &node.node_type {
        NodeType::Element { container_id, .. } => Some(*container_id),
        NodeType::Container {
            parent_container_id,
            ..
        } => *parent_container_id,
    }
}

/// Indexed entity lookups, built once per ordering pass rather than scanned per node.
struct Lookups<'a> {
    ctx: &'a TopologyContext<'a>,
    addresses: HashMap<Uuid, IpAddr>,
    interfaces: HashMap<Uuid, (Option<i32>, String)>,
    services: HashMap<Uuid, (&'static str, &'a str)>,
}

impl<'a> Lookups<'a> {
    fn new(ctx: &'a TopologyContext<'a>) -> Self {
        Self {
            ctx,
            addresses: ctx
                .ip_addresses
                .iter()
                .map(|ip| (ip.id, ip.base.ip_address))
                .collect(),
            interfaces: ctx
                .interfaces
                .iter()
                .map(|i| (i.id, (i.base.if_index, i.display_name())))
                .collect(),
            services: ctx
                .services
                .iter()
                .map(|s| {
                    (
                        s.id,
                        (
                            ServiceDefinition::category(&*s.base.service_definition).name(),
                            s.base.name.as_str(),
                        ),
                    )
                })
                .collect(),
        }
    }

    fn element_key(&self, node: &Node, sort: ElementSort) -> Option<SortKey> {
        let NodeType::Element {
            element, host_id, ..
        } = &node.node_type
        else {
            return None;
        };
        match sort {
            ElementSort::Layout => None,
            ElementSort::Address => match element {
                ElementEntityType::IPAddress { ip_address_id, .. } => ip_address_id
                    .and_then(|id| self.addresses.get(&id))
                    .map(|addr| SortKey::Address(*addr)),
                _ => None,
            },
            ElementSort::PortIndex => match element {
                ElementEntityType::Interface { interface_id } => self
                    .interfaces
                    .get(interface_id)
                    .and_then(|(index, _)| *index)
                    .map(SortKey::Index),
                _ => None,
            },
            ElementSort::Name => self
                .name(node, element, *host_id)
                .map(|name| SortKey::Name(NaturalKey::new(&name))),
            ElementSort::Category => match element {
                ElementEntityType::Service {} => {
                    self.services.get(&node.id).map(|(category, name)| {
                        SortKey::Category(category.to_string(), NaturalKey::new(name))
                    })
                }
                _ => None,
            },
        }
    }

    /// The node's title where the builder set one, otherwise its entity's own name.
    fn name(&self, node: &Node, element: &ElementEntityType, host_id: Uuid) -> Option<String> {
        if let Some(header) = node.header.as_ref().filter(|h| !h.is_empty()) {
            return Some(header.clone());
        }
        match element {
            ElementEntityType::Service {} => self
                .services
                .get(&node.id)
                .map(|(_, name)| name.to_string()),
            ElementEntityType::Interface { interface_id } => self
                .interfaces
                .get(interface_id)
                .map(|(_, name)| name.clone()),
            ElementEntityType::Host {} | ElementEntityType::IPAddress { .. } => {
                let host = self.ctx.get_host_by_id(host_id)?;
                self.ctx.host_container_header(host)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::Host;
    use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
    use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
    use crate::server::topology::types::base::TopologyOptions;
    use crate::server::topology::types::nodes::ContainerType;
    use crate::server::topology::types::views::TopologyView;

    fn container(id: Uuid, parent: Option<Uuid>, header: &str) -> Node {
        Node {
            id,
            node_type: NodeType::Container {
                container_type: if parent.is_some() {
                    ContainerType::NestedTag
                } else {
                    ContainerType::Subnet
                },
                parent_container_id: parent,
                entity_id: None,
                icon: None,
                color: None,
                associated_service_definition: None,
                element_rule_id: None,
                will_accept_edges: false,
            },
            position: Default::default(),
            size: Default::default(),
            header: Some(header.to_string()),
        }
    }

    fn address(host_id: Uuid, subnet_id: Uuid, addr: &str) -> IPAddress {
        IPAddress {
            id: Uuid::new_v4(),
            base: IPAddressBase {
                host_id,
                subnet_id,
                ip_address: addr.parse().unwrap(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn address_node(ip: &IPAddress, container_id: Uuid) -> Node {
        Node::element(
            ip.id,
            container_id,
            ip.base.host_id,
            ElementEntityType::IPAddress {
                subnet_id: ip.base.subnet_id,
                ip_address_id: Some(ip.id),
            },
        )
    }

    fn with_ctx<R>(
        ips: &[IPAddress],
        hosts: &[Host],
        interfaces: &[Interface],
        f: impl FnOnce(&TopologyContext) -> R,
    ) -> R {
        let options = TopologyOptions::default();
        let ctx = TopologyContext::new(
            hosts,
            ips,
            &[],
            &[],
            &[],
            &[],
            &[],
            interfaces,
            &[],
            &[],
            &options,
            TopologyView::L3Logical,
        );
        f(&ctx)
    }

    fn addresses_in(nodes: &[Node], ips: &[IPAddress], container: Uuid) -> Vec<String> {
        let by_id: HashMap<Uuid, &IPAddress> = ips.iter().map(|ip| (ip.id, ip)).collect();
        nodes
            .iter()
            .filter(|n| parent_id(n) == Some(container))
            .map(|n| match by_id.get(&n.id) {
                Some(ip) => ip.base.ip_address.to_string(),
                None => n.header.clone().unwrap_or_default(),
            })
            .collect()
    }

    #[test]
    fn addresses_compare_numerically_with_ipv4_first() {
        let subnet = Uuid::new_v4();
        let host = Uuid::new_v4();
        let ips: Vec<IPAddress> = ["fd00::1", "10.0.0.10", "10.0.0.9", "10.0.0.2"]
            .iter()
            .map(|a| address(host, subnet, a))
            .collect();
        let mut nodes = vec![container(subnet, None, "subnet")];
        nodes.extend(ips.iter().map(|ip| address_node(ip, subnet)));

        with_ctx(&ips, &[], &[], |ctx| {
            order_children(&mut nodes, ElementSort::Address, ctx)
        });

        assert_eq!(
            addresses_in(&nodes, &ips, subnet),
            ["10.0.0.2", "10.0.0.9", "10.0.0.10", "fd00::1"]
        );
    }

    #[test]
    fn a_group_sits_at_its_lowest_address_and_is_ordered_inside() {
        let subnet = Uuid::new_v4();
        let group = Uuid::new_v4();
        let host = Uuid::new_v4();
        let loose: Vec<IPAddress> = ["10.0.0.10", "10.0.0.3"]
            .iter()
            .map(|a| address(host, subnet, a))
            .collect();
        let grouped: Vec<IPAddress> = ["10.0.0.20", "10.0.0.5"]
            .iter()
            .map(|a| address(host, subnet, a))
            .collect();
        // The builder appends groups after the elements, as `apply_element_rules` does.
        let mut nodes = vec![container(subnet, None, "subnet")];
        nodes.extend(loose.iter().map(|ip| address_node(ip, subnet)));
        nodes.extend(grouped.iter().map(|ip| address_node(ip, group)));
        nodes.push(container(group, Some(subnet), "tagged"));
        let ips: Vec<IPAddress> = loose.into_iter().chain(grouped).collect();

        with_ctx(&ips, &[], &[], |ctx| {
            order_children(&mut nodes, ElementSort::Address, ctx)
        });

        assert_eq!(
            addresses_in(&nodes, &ips, subnet),
            ["10.0.0.3", "tagged", "10.0.0.10"]
        );
        assert_eq!(addresses_in(&nodes, &ips, group), ["10.0.0.5", "10.0.0.20"]);
    }

    #[test]
    fn nodes_without_a_key_go_last_by_header() {
        let subnet = Uuid::new_v4();
        let host = Uuid::new_v4();
        let ip = address(host, subnet, "10.0.0.7");
        let unknown = |header: &str| {
            let mut node = Node::element(
                Uuid::new_v4(),
                subnet,
                host,
                ElementEntityType::IPAddress {
                    subnet_id: subnet,
                    ip_address_id: None,
                },
            );
            node.header = Some(header.to_string());
            node
        };
        let mut nodes = vec![
            container(subnet, None, "subnet"),
            unknown("zeta"),
            unknown("alpha"),
            address_node(&ip, subnet),
        ];
        let ips = vec![ip];

        with_ctx(&ips, &[], &[], |ctx| {
            order_children(&mut nodes, ElementSort::Address, ctx)
        });

        assert_eq!(
            addresses_in(&nodes, &ips, subnet),
            ["10.0.0.7", "alpha", "zeta"]
        );
    }

    #[test]
    fn names_order_digit_runs_numerically() {
        let switch = Uuid::new_v4();
        let host = Host {
            id: Uuid::new_v4(),
            ..Default::default()
        };
        let interfaces: Vec<Interface> = ["eth10", "Eth2", "eth1"]
            .iter()
            .map(|name| Interface {
                id: Uuid::new_v4(),
                base: InterfaceBase {
                    host_id: host.id,
                    if_descr: Some(name.to_string()),
                    ..Default::default()
                },
                ..Default::default()
            })
            .collect();
        let mut nodes = vec![container(switch, None, "switch")];
        nodes.extend(interfaces.iter().map(|i| {
            Node::element(
                i.id,
                switch,
                host.id,
                ElementEntityType::Interface { interface_id: i.id },
            )
        }));
        let hosts = vec![host];

        with_ctx(&[], &hosts, &interfaces, |ctx| {
            order_children(&mut nodes, ElementSort::Name, ctx)
        });

        let names: HashMap<Uuid, String> = interfaces
            .iter()
            .map(|i| (i.id, i.display_name()))
            .collect();
        let ordered: Vec<&str> = nodes[1..].iter().map(|n| names[&n.id].as_str()).collect();
        assert_eq!(ordered, ["eth1", "Eth2", "eth10"]);
    }

    #[test]
    fn layout_keeps_the_builder_order() {
        let subnet = Uuid::new_v4();
        let host = Uuid::new_v4();
        let ips: Vec<IPAddress> = ["10.0.0.9", "10.0.0.2"]
            .iter()
            .map(|a| address(host, subnet, a))
            .collect();
        let mut nodes = vec![container(subnet, None, "subnet")];
        nodes.extend(ips.iter().map(|ip| address_node(ip, subnet)));
        let before: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();

        with_ctx(&ips, &[], &[], |ctx| {
            order_children(&mut nodes, ElementSort::Layout, ctx)
        });

        assert_eq!(nodes.iter().map(|n| n.id).collect::<Vec<_>>(), before);
    }
}
