use std::collections::{HashMap, HashSet};
use uuid::Uuid;

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
    /// The merged neighbour read-model (GH #701): every resolved adjacency in the network,
    /// `live_or_as_of`-pinned the same way `interfaces` is. Defaults to `&[]` via [`Self::new`];
    /// production callers attach the real set via [`Self::with_neighbours`]. A separate builder
    /// method rather than a 13th positional constructor argument, so the many test call sites that
    /// don't exercise neighbour behaviour don't all need updating.
    pub neighbours: &'a [InterfaceNeighborRow],
    /// Raw candidate rows, for the handful of callers that need per-candidate evidence (e.g. the
    /// LLDP-vs-CDP protocol label on a `NeighborLink` edge) rather than the resolved outcome.
    /// Defaults to `&[]`; attach via [`Self::with_candidates`].
    pub candidates: &'a [InterfaceNeighborCandidate],
    pub options: &'a TopologyOptions,
    /// View this context is building — drives grouping rule selection.
    pub view: TopologyView,
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
            options,
            view,
        }
    }

    pub fn with_neighbours(mut self, neighbours: &'a [InterfaceNeighborRow]) -> Self {
        self.neighbours = neighbours;
        self
    }

    pub fn with_candidates(mut self, candidates: &'a [InterfaceNeighborCandidate]) -> Self {
        self.candidates = candidates;
        self
    }

    /// Whether any of `interface_id`'s current candidates carry LLDP evidence. Approximates "this
    /// specific adjacency's own protocol" — the true per-row answer would need a protocol column
    /// on the resolved tables, which is a schema decision outside this pass's scope — but is exact
    /// for the common case of one candidate per port, and strictly better than the old
    /// interface-level `has_lldp_data()` it replaces.
    pub fn interface_has_lldp_evidence(&self, interface_id: Uuid) -> bool {
        self.candidates
            .iter()
            .filter(|c| c.base.interface_id == interface_id)
            .any(|c| c.base.evidence.has_lldp_data())
    }

    // ============================================================================
    // Data Access Methods
    // ============================================================================

    pub fn get_subnet_by_id(&self, subnet_id: Uuid) -> Option<&'a Subnet> {
        self.subnets.iter().find(|s| s.id == subnet_id)
    }

    pub fn get_host_by_id(&self, host_id: Uuid) -> Option<&'a Host> {
        self.hosts.iter().find(|h| h.id == host_id)
    }

    pub fn get_service_by_id(&self, service_id: Uuid) -> Option<&'a Service> {
        self.services.iter().find(|s| s.id == service_id)
    }

    pub fn get_ip_address_by_id(&self, ip_address_id: Option<Uuid>) -> Option<&'a IPAddress> {
        let id = ip_address_id?;
        self.ip_addresses.iter().find(|i| i.id == id)
    }

    pub fn get_ip_addresses_for_host(&self, host_id: Uuid) -> Vec<&'a IPAddress> {
        self.ip_addresses
            .iter()
            .filter(|i| i.base.host_id == host_id)
            .collect()
    }

    /// Heading for a container that stands for `host`.
    ///
    /// The ladder itself lives on [`Host::display_name`], so the host list and the same host drawn
    /// here cannot disagree about what it is called; this only supplies the addresses it needs.
    pub fn host_container_header(&self, host: &Host) -> Option<String> {
        host.display_name(self.get_ip_addresses_for_host(host.id))
    }

    pub fn get_services_for_host(&self, host_id: Uuid) -> Vec<&'a Service> {
        self.services
            .iter()
            .filter(|s| s.base.host_id == host_id)
            .collect()
    }

    /// Get the first non-docker-bridge interface for a host
    pub fn get_first_non_container_bridge_ip_address_for_host(
        &self,
        host_id: Uuid,
    ) -> Option<&'a IPAddress> {
        self.ip_addresses.iter().find(|ip_address| {
            if ip_address.base.host_id != host_id {
                return false;
            }
            if let Some(subnet) = self.get_subnet_by_id(ip_address.base.subnet_id) {
                return !subnet.base.subnet_type.is_container_bridge()
                    && !subnet.base.subnet_type.is_loopback();
            }
            false
        })
    }

    pub fn get_services_bound_to_ip_address(&self, ip_address_id: Uuid) -> Vec<&'a Service> {
        self.services
            .iter()
            .filter(|s| {
                s.to_bound_ip_address_ids()
                    .iter()
                    .any(|s| s.map(|id| id == ip_address_id).unwrap_or(false))
            })
            .collect()
    }

    pub fn get_subnet_from_ip_address_id(&self, ip_address_id: Uuid) -> Option<&'a Subnet> {
        let ip_address = self.ip_addresses.iter().find(|i| i.id == ip_address_id)?;
        self.get_subnet_by_id(ip_address.base.subnet_id)
    }

    pub fn get_host_from_ip_address_id(&self, ip_address_id: Uuid) -> Option<&'a Host> {
        let ip_address = self.ip_addresses.iter().find(|i| i.id == ip_address_id)?;
        self.hosts.iter().find(|h| h.id == ip_address.base.host_id)
    }

    // ============================================================================
    // Interface (SNMP Interface Table) Methods
    // ============================================================================

    pub fn get_interface_by_id(&self, id: Uuid) -> Option<&'a Interface> {
        self.interfaces.iter().find(|e| e.id == id)
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
        self.interfaces
            .iter()
            .filter(|e| e.base.host_id == host_id)
            .collect()
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
        self.neighbours
            .iter()
            .any(|n| n.interface_id == interface_id)
    }

    /// Whether `interface_id` is linked, judged in **both** directions.
    ///
    /// Delegates to [`InterfaceLinkState::classify`], so a port is classified the same way here as
    /// it is by the server-side metadata filter — the graph cannot drop a port the filter kept.
    /// Judging the outbound direction alone is the mistake that drew 11 edges where there were
    /// ~720; see `FilterValueContext`.
    pub fn interface_link_state(&self, interface_id: Uuid) -> InterfaceLinkState {
        InterfaceLinkState::classify(
            self.interface_has_neighbor(interface_id),
            self.neighbours
                .iter()
                .any(|n| n.neighbor.interface_id() == Some(interface_id)),
        )
    }

    // ============================================================================
    // Virtualization Relationship Methods
    // ============================================================================

    /// The hypervisor service this host runs on.
    ///
    /// Reads the column rather than matching a variant. This used to match `Proxmox` only, so a
    /// vCenter or ESXi guest reported no hypervisor at all despite carrying one — the kind of gap
    /// a single-variant match creates silently and a column cannot.
    pub fn get_host_is_virtualized_by(&self, host_id: &Uuid) -> Option<&Service> {
        let host = self.get_host_by_id(*host_id)?;
        let service_id = host.base.virtualization_service_id?;
        self.services.iter().find(|s| s.id == service_id)
    }

    /// The container runtime service hosting this service. Was `Docker`-only; Podman containers
    /// were invisible here for the same reason.
    pub fn get_service_is_containerized_by(&self, service_id: &Uuid) -> Option<&Service> {
        let service = self.get_service_by_id(*service_id)?;
        let runtime_service_id = service.base.virtualization_service_id?;
        self.services.iter().find(|s| s.id == runtime_service_id)
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
            let vertical_order_difference = source_subnet.base.subnet_type.vertical_order()
                as isize
                - target_subnet.base.subnet_type.vertical_order() as isize;

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
