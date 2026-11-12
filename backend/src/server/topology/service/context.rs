use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::fdb::LearnedHost;

use crate::server::{
    bindings::r#impl::base::Binding,
    dependencies::r#impl::base::Dependency,
    hosts::r#impl::base::Host,
    interface_neighbors::r#impl::base::{
        InterfaceNeighborCandidate, InterfaceNeighborRow, Neighbor,
    },
    interfaces::r#impl::base::{Interface, InterfaceLinkState},
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    subnets::r#impl::base::Subnet,
    tags::r#impl::base::Tag,
    topology::types::{
        base::TopologyOptions,
        edges::Edge,
        nodes::{ElementEntityType, Node, NodeType},
        views::TopologyView,
    },
    vlans::r#impl::base::Vlan,
};

/// Lookups over the context's slices, built once per context.
///
/// The builders ask for a host's addresses, a host's services or an interface by id once per
/// host, address or neighbour row. Answering each by scanning the slice made a build quadratic in
/// the site's size. Every list holds its rows in slice order and every id maps to the first row
/// carrying it, so a lookup returns what the scan returned.
struct ContextIndex<'a> {
    host_by_id: HashMap<Uuid, &'a Host>,
    subnet_by_id: HashMap<Uuid, &'a Subnet>,
    service_by_id: HashMap<Uuid, &'a Service>,
    ip_address_by_id: HashMap<Uuid, &'a IPAddress>,
    interface_by_id: HashMap<Uuid, &'a Interface>,
    ip_addresses_by_host: HashMap<Uuid, Vec<&'a IPAddress>>,
    services_by_host: HashMap<Uuid, Vec<&'a Service>>,
    services_by_bound_ip_address: HashMap<Uuid, Vec<&'a Service>>,
    interfaces_by_host: HashMap<Uuid, Vec<&'a Interface>>,
}

impl<'a> ContextIndex<'a> {
    fn new(
        hosts: &'a [Host],
        ip_addresses: &'a [IPAddress],
        subnets: &'a [Subnet],
        services: &'a [Service],
        interfaces: &'a [Interface],
    ) -> Self {
        let mut index = Self {
            host_by_id: HashMap::with_capacity(hosts.len()),
            subnet_by_id: HashMap::with_capacity(subnets.len()),
            service_by_id: HashMap::with_capacity(services.len()),
            ip_address_by_id: HashMap::with_capacity(ip_addresses.len()),
            interface_by_id: HashMap::with_capacity(interfaces.len()),
            ip_addresses_by_host: HashMap::new(),
            services_by_host: HashMap::new(),
            services_by_bound_ip_address: HashMap::new(),
            interfaces_by_host: HashMap::new(),
        };
        for host in hosts {
            index.host_by_id.entry(host.id).or_insert(host);
        }
        for subnet in subnets {
            index.subnet_by_id.entry(subnet.id).or_insert(subnet);
        }
        for ip_address in ip_addresses {
            index
                .ip_address_by_id
                .entry(ip_address.id)
                .or_insert(ip_address);
            index
                .ip_addresses_by_host
                .entry(ip_address.base.host_id)
                .or_default()
                .push(ip_address);
        }
        for service in services {
            index.service_by_id.entry(service.id).or_insert(service);
            index
                .services_by_host
                .entry(service.base.host_id)
                .or_default()
                .push(service);
            let mut bound: HashSet<Uuid> = HashSet::new();
            for ip_address_id in service.to_bound_ip_address_ids().into_iter().flatten() {
                if bound.insert(ip_address_id) {
                    index
                        .services_by_bound_ip_address
                        .entry(ip_address_id)
                        .or_default()
                        .push(service);
                }
            }
        }
        for interface in interfaces {
            index
                .interface_by_id
                .entry(interface.id)
                .or_insert(interface);
            index
                .interfaces_by_host
                .entry(interface.base.host_id)
                .or_default()
                .push(interface);
        }
        index
    }
}

/// Central context for topology building operations
/// Provides topology-specific business logic and data access
pub struct TopologyContext<'a> {
    pub hosts: &'a [Host],
    pub ip_addresses: &'a [IPAddress],
    pub subnets: &'a [Subnet],
    pub services: &'a [Service],
    pub dependencies: &'a [Dependency],
    pub ports: &'a [Port],
    pub bindings: &'a [Binding],
    pub interfaces: &'a [Interface],
    pub entity_tags: &'a [Tag],
    pub vlans: &'a [Vlan],
    /// The merged neighbour read-model (GH #701): every resolved adjacency in the site,
    /// `live_or_as_of`-pinned the same way `interfaces` is. Defaults to `&[]` via [`Self::new`];
    /// production callers attach the real set via [`Self::with_neighbours`]. A separate builder
    /// method rather than a 13th positional constructor argument, so the many test call sites that
    /// don't exercise neighbour behaviour don't all need updating.
    pub neighbours: &'a [InterfaceNeighborRow],
    /// Raw candidate rows, for the handful of callers that need per-candidate evidence (e.g. the
    /// LLDP-vs-CDP protocol label on a `NeighborLink` edge) rather than the resolved outcome.
    /// Defaults to `&[]`; attach via [`Self::with_candidates`].
    pub candidates: &'a [InterfaceNeighborCandidate],
    /// The organization's Application tag group; tags in it place entities in applications.
    /// `None` (the default) means no tag does. Attach via [`Self::with_application_group`].
    pub application_group_id: Option<Uuid>,
    pub options: &'a TopologyOptions,
    /// View this context is building — drives grouping rule selection.
    pub view: TopologyView,
    /// The known hosts each port's forwarding table learned, computed from `interfaces` and
    /// `ip_addresses` the way the server-side filter computes it (`fdb::learned_hosts`), so the
    /// graph and the filter agree on which ports are linked, and L2 draws those links.
    pub learned_by_port: HashMap<Uuid, Vec<LearnedHost>>,
    /// The far ends of those links (`fdb::learned_interfaces`).
    pub interfaces_learned_by_ports: HashSet<Uuid>,
    index: ContextIndex<'a>,
    /// Interfaces with a resolved neighbour row of their own, from `neighbours`.
    interfaces_with_neighbor: HashSet<Uuid>,
    /// Interfaces another interface's resolved neighbour row points at, from `neighbours`.
    interfaces_named_as_neighbor: HashSet<Uuid>,
    /// Interfaces with a candidate carrying LLDP evidence, from `candidates`.
    interfaces_with_lldp_evidence: HashSet<Uuid>,
}

impl<'a> TopologyContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        hosts: &'a [Host],
        ip_addresses: &'a [IPAddress],
        subnets: &'a [Subnet],
        services: &'a [Service],
        dependencies: &'a [Dependency],
        ports: &'a [Port],
        bindings: &'a [Binding],
        interfaces: &'a [Interface],
        entity_tags: &'a [Tag],
        vlans: &'a [Vlan],
        options: &'a TopologyOptions,
        view: TopologyView,
    ) -> Self {
        let learned_by_port = super::fdb::learned_hosts(interfaces, ip_addresses);
        Self {
            hosts,
            ip_addresses,
            subnets,
            services,
            dependencies,
            ports,
            bindings,
            interfaces,
            entity_tags,
            vlans,
            neighbours: &[],
            candidates: &[],
            application_group_id: None,
            options,
            view,
            interfaces_learned_by_ports: super::fdb::learned_interfaces(&learned_by_port),
            learned_by_port,
            index: ContextIndex::new(hosts, ip_addresses, subnets, services, interfaces),
            interfaces_with_neighbor: HashSet::new(),
            interfaces_named_as_neighbor: HashSet::new(),
            interfaces_with_lldp_evidence: HashSet::new(),
        }
    }

    pub fn with_neighbours(mut self, neighbours: &'a [InterfaceNeighborRow]) -> Self {
        self.neighbours = neighbours;
        self.interfaces_with_neighbor = neighbours.iter().map(|n| n.interface_id).collect();
        self.interfaces_named_as_neighbor = neighbours
            .iter()
            .filter_map(|n| n.neighbor.interface_id())
            .collect();
        self
    }

    pub fn with_candidates(mut self, candidates: &'a [InterfaceNeighborCandidate]) -> Self {
        self.candidates = candidates;
        self.interfaces_with_lldp_evidence = candidates
            .iter()
            .filter(|c| c.base.evidence.has_lldp_data())
            .map(|c| c.base.interface_id)
            .collect();
        self
    }

    pub fn with_application_group(mut self, application_group_id: Option<Uuid>) -> Self {
        self.application_group_id = application_group_id;
        self
    }

    /// Whether any of `interface_id`'s current candidates carry LLDP evidence. Approximates "this
    /// specific adjacency's own protocol" — the true per-row answer would need a protocol column
    /// on the resolved tables, which is a schema decision outside this pass's scope — but is exact
    /// for the common case of one candidate per port, and strictly better than the old
    /// interface-level `has_lldp_data()` it replaces.
    pub fn interface_has_lldp_evidence(&self, interface_id: Uuid) -> bool {
        self.interfaces_with_lldp_evidence.contains(&interface_id)
    }

    // ============================================================================
    // Data Access Methods
    // ============================================================================

    pub fn get_subnet_by_id(&self, subnet_id: Uuid) -> Option<&'a Subnet> {
        self.index.subnet_by_id.get(&subnet_id).copied()
    }

    pub fn get_host_by_id(&self, host_id: Uuid) -> Option<&'a Host> {
        self.index.host_by_id.get(&host_id).copied()
    }

    pub fn get_service_by_id(&self, service_id: Uuid) -> Option<&'a Service> {
        self.index.service_by_id.get(&service_id).copied()
    }

    pub fn get_ip_address_by_id(&self, ip_address_id: Option<Uuid>) -> Option<&'a IPAddress> {
        self.index.ip_address_by_id.get(&ip_address_id?).copied()
    }

    pub fn get_ip_addresses_for_host(&self, host_id: Uuid) -> Vec<&'a IPAddress> {
        self.index
            .ip_addresses_by_host
            .get(&host_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Heading for a container that stands for `host`.
    ///
    /// The ladder itself lives on [`Host::display_name`], so the host list and the same host drawn
    /// here cannot disagree about what it is called; this only supplies the addresses it needs.
    pub fn host_container_header(&self, host: &Host) -> Option<String> {
        host.display_name(self.get_ip_addresses_for_host(host.id))
    }

    pub fn get_services_for_host(&self, host_id: Uuid) -> Vec<&'a Service> {
        self.index
            .services_by_host
            .get(&host_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get the first non-docker-bridge interface for a host
    pub fn get_first_non_container_bridge_ip_address_for_host(
        &self,
        host_id: Uuid,
    ) -> Option<&'a IPAddress> {
        let ip_addresses = self.index.ip_addresses_by_host.get(&host_id)?;
        ip_addresses.iter().copied().find(|ip_address| {
            if let Some(subnet) = self.get_subnet_by_id(ip_address.base.subnet_id) {
                return !subnet.base.subnet_type().is_container_bridge()
                    && !subnet.base.subnet_type().is_loopback();
            }
            false
        })
    }

    pub fn get_services_bound_to_ip_address(&self, ip_address_id: Uuid) -> Vec<&'a Service> {
        self.index
            .services_by_bound_ip_address
            .get(&ip_address_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_subnet_from_ip_address_id(&self, ip_address_id: Uuid) -> Option<&'a Subnet> {
        let ip_address = self.get_ip_address_by_id(Some(ip_address_id))?;
        self.get_subnet_by_id(ip_address.base.subnet_id)
    }

    pub fn get_host_from_ip_address_id(&self, ip_address_id: Uuid) -> Option<&'a Host> {
        let ip_address = self.get_ip_address_by_id(Some(ip_address_id))?;
        self.get_host_by_id(ip_address.base.host_id)
    }

    // ============================================================================
    // Interface (SNMP Interface Table) Methods
    // ============================================================================

    pub fn get_interface_by_id(&self, id: Uuid) -> Option<&'a Interface> {
        self.index.interface_by_id.get(&id).copied()
    }

    /// Resolve an IP address ID from an Interface, with single-IP-address host fallback.
    /// Returns Some(ip_address_id) if:
    /// - The interface has ip_address_id set, OR
    /// - The interface's host has exactly one IP address
    pub fn resolve_ip_address_for_interface(&self, interface: &Interface) -> Option<Uuid> {
        // Direct resolution
        if let Some(ip_address_id) = interface.base.ip_address_id {
            return Some(ip_address_id);
        }

        // Single-interface host fallback
        let host_interfaces = self.get_ip_addresses_for_host(interface.base.host_id);
        if host_interfaces.len() == 1 {
            return Some(host_interfaces[0].id);
        }

        None
    }

    pub fn get_interfaces_for_host(&self, host_id: Uuid) -> Vec<&'a Interface> {
        self.index
            .interfaces_by_host
            .get(&host_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Every resolved full adjacency (specific remote port known) — one row per
    /// `(interface, neighbour interface)` pair, so a port may appear more than once here now
    /// (GH #701: a port can anchor several links).
    pub fn get_interfaces_with_neighbor(&self) -> Vec<&'a InterfaceNeighborRow> {
        self.neighbours
            .iter()
            .filter(|n| matches!(n.neighbor, Neighbor::Interface(_)))
            .collect()
    }

    /// Interfaces whose LLDP/CDP neighbour resolved to a host but not to a specific port.
    /// The adjacency is known, the remote port is not — see `EdgeType::NeighborLink`.
    pub fn get_interfaces_with_host_neighbor(&self) -> Vec<&'a InterfaceNeighborRow> {
        self.neighbours
            .iter()
            .filter(|n| matches!(n.neighbor, Neighbor::Host(_)))
            .collect()
    }

    /// Whether `interface_id` has at least one live row of its own in either resolved-neighbour
    /// table — the successor to reading `Interface.neighbor.is_some()` directly.
    ///
    /// The **outbound direction only**. Almost every caller wants
    /// [`Self::interface_link_state`] instead: a link is recorded on one side, so this answers
    /// `false` for the far end of most links.
    pub fn interface_has_neighbor(&self, interface_id: Uuid) -> bool {
        self.interfaces_with_neighbor.contains(&interface_id)
    }

    /// Whether `interface_id` is linked, judged in **both** directions.
    ///
    /// Delegates to [`InterfaceLinkState::classify`], so a port is classified the same way here as
    /// it is by the server-side metadata filter — the graph cannot drop a port the filter kept.
    /// Judging the outbound direction alone is the mistake that drew 11 edges where there were
    /// ~720; see `FilterValueContext`.
    pub fn interface_link_state(&self, interface_id: Uuid) -> InterfaceLinkState {
        InterfaceLinkState::classify(
            self.interface_has_neighbor(interface_id)
                || self.learned_by_port.contains_key(&interface_id),
            self.interfaces_named_as_neighbor.contains(&interface_id)
                || self.interfaces_learned_by_ports.contains(&interface_id),
        )
    }

    // ============================================================================
    // Virtualization Relationship Methods
    // ============================================================================

    /// The hypervisor service this host runs on.
    ///
    /// Reads the column rather than matching a variant. This used to match `Proxmox` only, so a
    /// vCenter or ESXi guest reported no hypervisor at all despite carrying one — the type of gap
    /// a single-variant match creates silently and a column cannot.
    pub fn get_host_is_virtualized_by(&self, host_id: &Uuid) -> Option<&Service> {
        let host = self.get_host_by_id(*host_id)?;
        self.get_service_by_id(host.base.virtualization_service_id?)
    }

    /// The container runtime service hosting this service. Was `Docker`-only; Podman containers
    /// were invisible here for the same reason.
    pub fn get_service_is_containerized_by(&self, service_id: &Uuid) -> Option<&Service> {
        let service = self.get_service_by_id(*service_id)?;
        self.get_service_by_id(service.base.virtualization_service_id?)
    }

    pub fn get_node_subnet(&self, node_id: Uuid, nodes: &[Node]) -> Option<Uuid> {
        nodes
            .iter()
            .find(|n| n.id == node_id)
            .and_then(|node| match &node.node_type {
                NodeType::Element {
                    element: ElementEntityType::IPAddress { subnet_id, .. },
                    ..
                } => Some(*subnet_id),
                NodeType::Container { .. } => Some(node.id),
                _ => None,
            })
    }

    // ============================================================================
    // VLAN Methods
    // ============================================================================

    /// Look up a VLAN by its entity UUID
    pub fn get_vlan_by_id(&self, vlan_id: Uuid) -> Option<&'a Vlan> {
        self.vlans.iter().find(|v| v.id == vlan_id)
    }

    // ============================================================================
    // Tag Methods
    // ============================================================================

    pub fn get_host_ids_with_tags(&self, tag_ids: &[Uuid]) -> HashSet<Uuid> {
        self.hosts
            .iter()
            .filter(|h| h.base.tags.iter().any(|t| tag_ids.contains(t)))
            .map(|h| h.id)
            .collect()
    }

    // ============================================================================
    // Edge Classification Methods
    // ============================================================================

    pub fn edge_is_intra_subnet(&self, edge: &Edge) -> bool {
        if let (Some(source_subnet), Some(target_subnet)) = (
            self.get_subnet_from_ip_address_id(edge.source),
            self.get_subnet_from_ip_address_id(edge.target),
        ) {
            return source_subnet.id == target_subnet.id;
        }
        false
    }

    pub fn edge_is_multi_hop(
        &self,
        source_ip_address_id: &Uuid,
        target_ip_address_id: &Uuid,
    ) -> bool {
        if let (Some(source_subnet), Some(target_subnet)) = (
            self.get_subnet_from_ip_address_id(*source_ip_address_id),
            self.get_subnet_from_ip_address_id(*target_ip_address_id),
        ) {
            let vertical_order_difference = source_subnet.base.subnet_type().vertical_order()
                as isize
                - target_subnet.base.subnet_type().vertical_order() as isize;

            return vertical_order_difference.abs() > 1;
        }
        false
    }

    // ============================================================================
    // Node Existence Checks
    // ============================================================================

    pub fn interface_will_have_node(&self, _ip_address_id: &Uuid) -> bool {
        true
    }

    pub fn service_will_have_node(&self, service_id: &Uuid) -> bool {
        self.get_service_by_id(*service_id)
            .map(|s| !s.base.bindings.is_empty())
            .unwrap_or(true)
    }

    /// Build a map of binding_id → service_id from all services.
    /// Used by builders that need to resolve dependency bindings to service IDs.
    pub fn build_binding_to_service_map(&self) -> HashMap<Uuid, Uuid> {
        self.services
            .iter()
            .flat_map(|s| s.base.bindings.iter().map(move |b| (b.id, s.id)))
            .collect()
    }
}
