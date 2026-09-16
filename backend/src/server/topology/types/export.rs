use super::api::TopologyData;
use super::edges::{Edge, EdgeType};
use super::nodes::{ElementEntityType, Node, NodeType};
use std::collections::HashMap;
use std::fmt::Write;
use uuid::Uuid;

/// A host the export could not resolve at all — a node pointing at an id the bundle doesn't carry.
const UNKNOWN_HOST: &str = "Unknown Host";

/// A host the export resolved, that has nothing on any rung of the title ladder.
///
/// Distinct from [`UNKNOWN_HOST`]: one means the export lost the host, the other means the host is
/// there and has never been given a name. Matches the `hosts_unnamedHost` string the UI renders for
/// the same state — an export and the screen it was taken from should not use different words.
/// Literal rather than translated because these artifacts (Mermaid, Confluence markup) are emitted
/// server-side, where the requester's locale isn't in scope.
const UNNAMED_HOST: &str = "Unnamed host";

fn short_id(id: &Uuid) -> String {
    id.to_string().replace('-', "")[..8].to_string()
}

fn mermaid_escape(s: &str) -> String {
    s.replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('[', "&#91;")
        .replace(']', "&#93;")
}

fn edge_type_name(edge_type: &EdgeType) -> &'static str {
    match edge_type {
        EdgeType::SameHost { .. } => "Same Host",
        EdgeType::Hypervisor { .. } => "Hypervisor",
        EdgeType::ContainerRuntime { .. } => "Container Runtime",
        EdgeType::SameContainer { .. } => "Same Container",
        EdgeType::RequestPath { .. } => "Request Path",
        EdgeType::HubAndSpoke { .. } => "Hub & Spoke",
        EdgeType::PhysicalLink { .. } => "Physical Link",
        EdgeType::NeighborLink { .. } => "Neighbor Link",
    }
}

pub fn topology_to_mermaid(nodes: &[Node], edges: &[Edge], data: &TopologyData) -> String {
    let mut output = String::new();
    writeln!(output, "flowchart TD").unwrap();

    // Build lookup maps
    let subnets: HashMap<Uuid, _> = data.subnets.iter().map(|s| (s.id, s)).collect();
    let hosts: HashMap<Uuid, _> = data.hosts.iter().map(|h| (h.id, h)).collect();
    let ip_addresses: HashMap<Uuid, _> = data.ip_addresses.iter().map(|i| (i.id, i)).collect();

    // Group Element nodes by subnet_id (only Interface elements have subnet_id)
    let mut nodes_by_subnet: HashMap<Uuid, Vec<_>> = HashMap::new();
    for node in nodes {
        if let NodeType::Element {
            element: ElementEntityType::IPAddress { subnet_id, .. },
            ..
        } = &node.node_type
        {
            nodes_by_subnet.entry(*subnet_id).or_default().push(node);
        }
    }

    // Generate subgraphs for each subnet
    for (subnet_id, nodes) in &nodes_by_subnet {
        if let Some(subnet) = subnets.get(subnet_id) {
            let sub_label = format!(
                "{} - {}",
                mermaid_escape(&subnet.base.name),
                subnet.base.cidr
            );
            writeln!(
                output,
                "    subgraph sub_{}[\"{}\"]",
                short_id(subnet_id),
                sub_label
            )
            .unwrap();

            for node in nodes {
                if let NodeType::Element {
                    host_id,
                    element: ElementEntityType::IPAddress { ip_address_id, .. },
                    ..
                } = &node.node_type
                {
                    let host_name = hosts
                        .get(host_id)
                        .map(|h| h.display_name.as_deref().unwrap_or(UNNAMED_HOST))
                        .unwrap_or(UNKNOWN_HOST);

                    let ip = ip_address_id
                        .and_then(|iid| ip_addresses.get(&iid))
                        .map(|i| i.base.ip_address.to_string())
                        .unwrap_or_default();

                    let label = if ip.is_empty() {
                        mermaid_escape(host_name)
                    } else {
                        format!("{}<br/>{}", mermaid_escape(host_name), ip)
                    };

                    writeln!(output, "        n_{}[\"{}\"]", short_id(&node.id), label).unwrap();
                }
            }

            writeln!(output, "    end").unwrap();
        }
    }

    // Build set of Container IDs (their node ID == subnet ID, rendered as subgraphs)
    let subnet_node_ids: std::collections::HashSet<Uuid> = nodes
        .iter()
        .filter(|n| matches!(n.node_type, NodeType::Container { .. }))
        .map(|n| n.id)
        .collect();

    // Generate edges — use sub_ prefix for subgraph nodes, n_ for interface nodes
    for edge in edges {
        let arrow = match &edge.edge_type {
            EdgeType::RequestPath { .. } | EdgeType::HubAndSpoke { .. } => "-->",
            EdgeType::SameHost { .. }
            | EdgeType::PhysicalLink { .. }
            | EdgeType::SameContainer { .. } => "---",
            // Dotted: adjacent devices, exact ports unknown.
            EdgeType::NeighborLink { .. } => "-.-",
            EdgeType::Hypervisor { .. } | EdgeType::ContainerRuntime { .. } => "-.->",
        };

        let label_str = edge
            .label
            .as_ref()
            .map(|l| format!("|{}|", mermaid_escape(l)))
            .unwrap_or_default();

        let source_prefix = if subnet_node_ids.contains(&edge.source) {
            "sub"
        } else {
            "n"
        };
        let target_prefix = if subnet_node_ids.contains(&edge.target) {
            "sub"
        } else {
            "n"
        };

        writeln!(
            output,
            "    {}_{} {}{} {}_{}",
            source_prefix,
            short_id(&edge.source),
            arrow,
            label_str,
            target_prefix,
            short_id(&edge.target)
        )
        .unwrap();
    }

    output
}

pub fn topology_to_confluence(nodes: &[Node], edges: &[Edge], data: &TopologyData) -> String {
    let mut output = String::new();

    // Header
    writeln!(output, "h1. Network Topology").unwrap();
    writeln!(output).unwrap();

    // Subnets table
    writeln!(output, "h2. Subnets").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "|| Name || CIDR || Type || Description ||").unwrap();
    for subnet in &data.subnets {
        let description = subnet.base.description.as_deref().unwrap_or("");
        writeln!(
            output,
            "| {} | {} | {:?} | {} |",
            subnet.base.name, subnet.base.cidr, subnet.base.subnet_type, description
        )
        .unwrap();
    }
    writeln!(output).unwrap();

    // Build lookup maps for hosts table
    let mut ip_addresses_by_host: HashMap<Uuid, Vec<String>> = HashMap::new();
    for iface in &data.ip_addresses {
        ip_addresses_by_host
            .entry(iface.base.host_id)
            .or_default()
            .push(iface.base.ip_address.to_string());
    }

    let mut services_by_host: HashMap<Uuid, Vec<String>> = HashMap::new();
    for svc in &data.services {
        services_by_host
            .entry(svc.base.host_id)
            .or_default()
            .push(svc.base.name.clone());
    }

    // Hosts table
    writeln!(output, "h2. Hosts").unwrap();
    writeln!(output).unwrap();
    writeln!(output, "|| Name || Hostname || IP Addresses || Services ||").unwrap();
    for host in &data.hosts {
        let hostname =
            crate::server::shared::attribution::text_of(&host.base.hostname).unwrap_or_default();
        let ips = ip_addresses_by_host
            .get(&host.id)
            .map(|v| v.join(", "))
            .unwrap_or_default();
        let services = services_by_host
            .get(&host.id)
            .map(|v| v.join(", "))
            .unwrap_or_default();

        writeln!(
            output,
            "| {} | {} | {} | {} |",
            host.display_name.as_deref().unwrap_or(UNNAMED_HOST),
            hostname,
            ips,
            services
        )
        .unwrap();
    }
    writeln!(output).unwrap();

    // Connections
    writeln!(output, "h2. Connections").unwrap();
    writeln!(output).unwrap();

    // Build node_id -> host title map. The same ladder the `NodeType::Container` arm below reads
    // off `n.header` — an element and the container it sits in must not name their host differently
    // in one table.
    let hosts_map: HashMap<Uuid, &str> = data
        .hosts
        .iter()
        .map(|h| (h.id, h.display_name.as_deref().unwrap_or(UNNAMED_HOST)))
        .collect();
    let nodes_map: HashMap<Uuid, _> = nodes.iter().map(|n| (n.id, n)).collect();

    for edge in edges {
        let source_host = nodes_map
            .get(&edge.source)
            .and_then(|n| match &n.node_type {
                NodeType::Element { host_id, .. } => hosts_map.get(host_id).copied(),
                NodeType::Container { .. } => n.header.as_deref(),
            })
            .unwrap_or("Unknown");

        let target_host = nodes_map
            .get(&edge.target)
            .and_then(|n| match &n.node_type {
                NodeType::Element { host_id, .. } => hosts_map.get(host_id).copied(),
                NodeType::Container { .. } => n.header.as_deref(),
            })
            .unwrap_or("Unknown");

        let type_name = edge_type_name(&edge.edge_type);

        writeln!(
            output,
            "* {} -> {} ({})",
            source_host, target_host, type_name
        )
        .unwrap();
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mermaid_empty_topology() {
        let data = TopologyData::default();

        let result = topology_to_mermaid(&[], &[], &data);
        assert!(result.contains("flowchart TD"));
    }

    #[test]
    fn test_confluence_empty_topology() {
        let data = TopologyData::default();

        let result = topology_to_confluence(&[], &[], &data);
        assert!(result.contains("h1. Network Topology"));
        assert!(result.contains("|| Name || CIDR || Type || Description ||"));
        assert!(result.contains("|| Name || Hostname || IP Addresses || Services ||"));
    }

    #[test]
    fn test_mermaid_escape_special_chars() {
        assert_eq!(mermaid_escape("test\"value"), "test&quot;value");
        assert_eq!(mermaid_escape("test<value>"), "test&lt;value&gt;");
        assert_eq!(mermaid_escape("test[value]"), "test&#91;value&#93;");
    }

    #[test]
    fn test_short_id_format() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let result = short_id(&id);
        assert_eq!(result.len(), 8);
        assert_eq!(result, "550e8400");
    }
}
