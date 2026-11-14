//! Keep the topology bundle closed: no row survives the removal of the row it belongs to.
//!
//! Hosts, subnets, services and interfaces leave the bundle for several reasons: a person
//! dismissed the host, a Visibility tag hid it, a server-side metadata filter dropped it. Each of
//! those paths removes the parent row only. Without this pass its interfaces, addresses, services
//! and neighbour rows stay behind, and every view builder has to notice on its own that their
//! parent is gone. L2 didn't, and drew ports inside a host container it never emitted, which the
//! client's layout rejects wholesale (GH #852).
//!
//! Every removal path runs [`TopologyData::remove_orphans`] after it removes rows, so builders
//! only see a bundle whose required references resolve.

use std::collections::HashSet;

use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::Binding,
    interface_neighbors::r#impl::base::{InterfaceNeighborCandidate, InterfaceNeighborRow},
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    topology::types::api::{TopologyData, TopologyInterface, TopologyService},
};

/// Rows [`TopologyData::remove_orphans`] removed, kept so a caller that restores their parents
/// (the Visibility counts) can restore them too.
#[derive(Debug, Clone, Default)]
pub struct OrphanRows {
    pub ip_addresses: Vec<IPAddress>,
    pub interfaces: Vec<TopologyInterface>,
    pub services: Vec<TopologyService>,
    pub ports: Vec<Port>,
    pub bindings: Vec<Binding>,
    pub neighbours: Vec<InterfaceNeighborRow>,
    pub candidates: Vec<InterfaceNeighborCandidate>,
}

impl OrphanRows {
    /// Put the rows back into `data`. Run [`TopologyData::remove_orphans`] afterwards: rows whose
    /// parent is still absent leave again.
    pub fn restore_into(self, data: &mut TopologyData) {
        data.ip_addresses.extend(self.ip_addresses);
        data.interfaces.extend(self.interfaces);
        data.services.extend(self.services);
        data.ports.extend(self.ports);
        data.bindings.extend(self.bindings);
        data.neighbours.extend(self.neighbours);
        data.candidates.extend(self.candidates);
    }

    pub fn extend(&mut self, other: OrphanRows) {
        self.ip_addresses.extend(other.ip_addresses);
        self.interfaces.extend(other.interfaces);
        self.services.extend(other.services);
        self.ports.extend(other.ports);
        self.bindings.extend(other.bindings);
        self.neighbours.extend(other.neighbours);
        self.candidates.extend(other.candidates);
    }
}

/// Move the rows of `rows` that fail `keep` into the returned vector.
fn split_off<T>(rows: &mut Vec<T>, keep: impl Fn(&T) -> bool) -> Vec<T> {
    let (kept, removed) = std::mem::take(rows).into_iter().partition(keep);
    *rows = kept;
    removed
}

impl TopologyData {
    /// Remove every row whose required parent is absent from the bundle, and return them.
    ///
    /// Runs parents first, so one pass reaches the end of each chain: an address leaves with its
    /// host or subnet; an interface, service or port with its host; a binding with its service,
    /// address or port; a neighbour row with its local interface; a candidate
    /// with its interface. Optional links, such as an interface's `ip_address_id`, stay.
    pub fn remove_orphans(&mut self) -> OrphanRows {
        // No `..`: a collection added to the bundle doesn't compile here until it states what it
        // belongs to.
        let TopologyData {
            hosts,
            ip_addresses,
            subnets,
            // A dependency only joins services or bindings with an edge, and an edge to an absent
            // node is dropped when the graph is assembled.
            dependencies: _,
            ports,
            bindings,
            interfaces,
            neighbours,
            candidates,
            services,
            // Site-level rows, owned by no host.
            vlans: _,
            tags: _,
            application_group_id: _,
            // Derived from the rows above, after this pass.
            nodes: _,
            edges: _,
            node_entities: _,
            hidden_by_tag: _,
            filtered_out: _,
            available_views: _,
            has_interface_credential: _,
        } = self;

        let host_ids: HashSet<Uuid> = hosts.iter().map(|h| h.host.id).collect();
        let subnet_ids: HashSet<Uuid> = subnets.iter().map(|s| s.id).collect();

        let removed_ip_addresses = split_off(ip_addresses, |ip| {
            host_ids.contains(&ip.base.host_id) && subnet_ids.contains(&ip.base.subnet_id)
        });
        let removed_interfaces =
            split_off(interfaces, |i| host_ids.contains(&i.interface.base.host_id));
        let removed_services = split_off(services, |s| host_ids.contains(&s.service.base.host_id));
        let removed_ports = split_off(ports, |p| host_ids.contains(&p.base.host_id));

        let ip_address_ids: HashSet<Uuid> = ip_addresses.iter().map(|ip| ip.id).collect();
        let interface_ids: HashSet<Uuid> = interfaces.iter().map(|i| i.interface.id).collect();
        let service_ids: HashSet<Uuid> = services.iter().map(|s| s.service.id).collect();
        let port_ids: HashSet<Uuid> = ports.iter().map(|p| p.id).collect();

        let removed_bindings = split_off(bindings, |b| {
            service_ids.contains(&b.service_id())
                && b.ip_address_id()
                    .is_none_or(|id| ip_address_ids.contains(&id))
                && b.port_id().is_none_or(|id| port_ids.contains(&id))
        });
        // The local interface is the row's parent. The far end is a reference: the row still says
        // the local port is linked (the "By link" filter reads it), and the builders draw no edge
        // to a far end the bundle doesn't hold.
        let removed_neighbours =
            split_off(neighbours, |row| interface_ids.contains(&row.interface_id));
        let removed_candidates =
            split_off(candidates, |c| interface_ids.contains(&c.base.interface_id));

        OrphanRows {
            ip_addresses: removed_ip_addresses,
            interfaces: removed_interfaces,
            services: removed_services,
            ports: removed_ports,
            bindings: removed_bindings,
            neighbours: removed_neighbours,
            candidates: removed_candidates,
        }
    }
}
