use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::Binding,
    groups::r#impl::base::Group,
    hosts::r#impl::{base::Host, virtualization::HostVirtualization},
    if_entries::r#impl::base::IfEntry,
    interfaces::r#impl::base::Interface,
    ports::r#impl::base::Port,
    services::r#impl::{
        base::Service, definitions::ServiceDefinitionExt, virtualization::ServiceVirtualization,
    },
    subnets::r#impl::base::Subnet,
    topology::types::{
        base::TopologyOptions,
        edges::Edge,
        nodes::{Node, NodeType},
    },
};

/// Central context for topology building operations
/// Provides topology-specific business logic and data access
pub struct TopologyContext<'a> {
    pub hosts: &'a [Host],
    pub interfaces: &'a [Interface],
    pub subnets: &'a [Subnet],
    pub services: &'a [Service],
    pub groups: &'a [Group],
    pub ports: &'a [Port],
    pub bindings: &'a [Binding],
    pub if_entries: &'a [IfEntry],
    pub options: &'a TopologyOptions,
}

impl<'a> TopologyContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        hosts: &'a [Host],
        interfaces: &'a [Interface],
        subnets: &'a [Subnet],
        services: &'a [Service],
        groups: &'a [Group],
        ports: &'a [Port],
        bindings: &'a [Binding],
        if_entries: &'a [IfEntry],
        options: &'a TopologyOptions,
    ) -> Self {
        Self {
            hosts,
            interfaces,
            subnets,
            services,
            groups,
            ports,
            bindings,
            if_entries,
            options,
        }
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

    pub fn get_interface_by_id(&self, interface_id: Option<Uuid>) -> Option<&'a Interface> {
        let id = interface_id?;
        self.interfaces.iter().find(|i| i.id == id)
    }

    pub fn get_interfaces_for_host(&self, host_id: Uuid) -> Vec<&'a Interface> {
        self.interfaces
            .iter()
            .filter(|i| i.base.host_id == host_id)
            .collect()
    }

    pub fn get_services_for_host(&self, host_id: Uuid) -> Vec<&'a Service> {
        self.services
            .iter()
            .filter(|s| s.base.host_id == host_id)
            .collect()
    }

    /// Get the first non-docker-bridge interface for a host
    pub fn get_first_non_docker_bridge_interface_for_host(
        &self,
        host_id: Uuid,
    ) -> Option<&'a Interface> {
        self.interfaces.iter().find(|interface| {
            if interface.base.host_id != host_id {
                return false;
            }
            if let Some(subnet) = self.get_subnet_by_id(interface.base.subnet_id) {
                return !subnet.base.subnet_type.is_docker_bridge()
                    && !subnet.base.subnet_type.is_loopback();
            }
            false
        })
    }

    pub fn get_services_bound_to_interface(&self, interface_id: Uuid) -> Vec<&'a Service> {
        self.services
            .iter()
            .filter(|s| {
                s.to_bound_interface_ids()
                    .iter()
                    .any(|s| s.map(|id| id == interface_id).unwrap_or(false))
            })
            .collect()
    }

    pub fn get_subnet_from_interface_id(&self, interface_id: Uuid) -> Option<&'a Subnet> {
        let interface = self.interfaces.iter().find(|i| i.id == interface_id)?;
        self.get_subnet_by_id(interface.base.subnet_id)
    }

    pub fn get_host_from_interface_id(&self, interface_id: Uuid) -> Option<&'a Host> {
        let interface = self.interfaces.iter().find(|i| i.id == interface_id)?;
        self.hosts.iter().find(|h| h.id == interface.base.host_id)
    }

    // ============================================================================
    // IfEntry (SNMP Interface Table) Methods
    // ============================================================================

    pub fn get_if_entry_by_id(&self, id: Uuid) -> Option<&'a IfEntry> {
        self.if_entries.iter().find(|e| e.id == id)
    }

    /// Resolve an interface ID from an IfEntry, with single-interface host fallback.
    /// Returns Some(interface_id) if:
    /// - The if_entry has interface_id set, OR
    /// - The if_entry's host has exactly one interface
    pub fn resolve_interface_for_if_entry(&self, if_entry: &IfEntry) -> Option<Uuid> {
        // Direct resolution
        if let Some(interface_id) = if_entry.base.interface_id {
            return Some(interface_id);
        }

        // Single-interface host fallback
        let host_interfaces = self.get_interfaces_for_host(if_entry.base.host_id);
        if host_interfaces.len() == 1 {
            return Some(host_interfaces[0].id);
        }

        None
    }

    pub fn get_if_entries_for_host(&self, host_id: Uuid) -> Vec<&'a IfEntry> {
        self.if_entries
            .iter()
            .filter(|e| e.base.host_id == host_id)
            .collect()
    }

    /// Get all if_entries that have a resolved neighbor (full resolution only)
    pub fn get_if_entries_with_neighbor(&self) -> Vec<&'a IfEntry> {
        use crate::server::if_entries::r#impl::base::Neighbor;
        self.if_entries
            .iter()
            .filter(|e| matches!(e.base.neighbor, Some(Neighbor::IfEntry(_))))
            .collect()
    }

    // ============================================================================
    // Virtualization Relationship Methods
    // ============================================================================

    pub fn get_host_is_virtualized_by(&self, host_id: &Uuid) -> Option<&Service> {
        if let Some(host) = self.get_host_by_id(*host_id)
            && let Some(HostVirtualization::Proxmox(proxmox_virtualization)) =
                &host.base.virtualization
        {
            return self
                .services
                .iter()
                .find(|s| s.id == proxmox_virtualization.service_id);
        }
        None
    }

    pub fn get_service_is_containerized_by(&self, service_id: &Uuid) -> Option<&Service> {
        if let Some(service) = self.get_service_by_id(*service_id)
            && let Some(ServiceVirtualization::Docker(docker_virtualization)) =
                &service.base.virtualization
        {
            return self
                .services
                .iter()
                .find(|s| s.id == docker_virtualization.service_id);
        }
        None
    }

    pub fn get_node_subnet(&self, node_id: Uuid, nodes: &[Node]) -> Option<Uuid> {
        nodes
            .iter()
            .find(|n| n.id == node_id)
            .map(|node| match node.node_type {
                NodeType::InterfaceNode { subnet_id, .. } => subnet_id,
                NodeType::SubnetNode { .. } => node.id,
            })
    }

    // ============================================================================
    // Infrastructure Zone Methods
    // ============================================================================

    pub fn get_interfaces_with_infra_service(&self, subnet: &Subnet) -> Vec<Option<Uuid>> {
        self.services
            .iter()
            .filter(|s| {
                if let Some(host) = self.hosts.iter().find(|h| h.id == s.base.host_id) {
                    let host_interfaces = self.get_interfaces_for_host(host.id);
                    return (self
                        .options
                        .request
                        .left_zone_service_categories
                        .contains(&s.base.service_definition.category())
                        || (self.options.request.show_gateway_in_left_zone
                            && s.base.service_definition.is_gateway()))
                        && subnet.has_interface_with_service(&host_interfaces, s);
                }
                false
            })
            .flat_map(|s| s.to_bound_interface_ids())
            .collect()
    }

    pub fn is_interface_infra(&self, interface_id: Uuid) -> bool {
        if let Some(subnet) = self.get_subnet_from_interface_id(interface_id) {
            let infra_interfaces = self.get_interfaces_with_infra_service(subnet);
            return infra_interfaces.contains(&Some(interface_id));
        }
        false
    }

    /// Check if a subnet has both infra and non-infra nodes
    /// If it only has one type, infra constraints are not necessary
    pub fn subnet_has_mixed_infra(&self, subnet: &Subnet) -> bool {
        let infra_interfaces = self.get_interfaces_with_infra_service(subnet);

        // Get all interfaces in this subnet
        let all_interfaces_in_subnet: Vec<Uuid> = self
            .interfaces
            .iter()
            .filter(|i| i.base.subnet_id == subnet.id)
            .map(|i| i.id)
            .collect();

        if all_interfaces_in_subnet.is_empty() {
            return false;
        }

        // Check if we have both infra and non-infra
        let has_infra = all_interfaces_in_subnet
            .iter()
            .any(|id| infra_interfaces.contains(&Some(*id)));

        let has_non_infra = all_interfaces_in_subnet
            .iter()
            .any(|id| !infra_interfaces.contains(&Some(*id)));

        has_infra && has_non_infra
    }

    // ============================================================================
    // Edge Classification Methods
    // ============================================================================

    pub fn edge_is_intra_subnet(&self, edge: &Edge) -> bool {
        if let (Some(source_subnet), Some(target_subnet)) = (
            self.get_subnet_from_interface_id(edge.source),
            self.get_subnet_from_interface_id(edge.target),
        ) {
            return source_subnet.id == target_subnet.id;
        }
        false
    }

    pub fn edge_is_multi_hop(
        &self,
        source_interface_id: &Uuid,
        target_interface_id: &Uuid,
    ) -> bool {
        if let (Some(source_subnet), Some(target_subnet)) = (
            self.get_subnet_from_interface_id(*source_interface_id),
            self.get_subnet_from_interface_id(*target_interface_id),
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

    pub fn interface_will_have_node(&self, _interface_id: &Uuid) -> bool {
        true
    }

    pub fn service_will_have_node(&self, service_id: &Uuid) -> bool {
        self.get_service_by_id(*service_id)
            .map(|s| !s.base.bindings.is_empty())
            .unwrap_or(true)
    }
}
