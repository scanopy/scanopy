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

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::Binding,
    interface_neighbors::r#impl::base::{InterfaceNeighborCandidate, InterfaceNeighborRow},
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    shared::{
        entities::EntityDiscriminants as E,
        storage::{
            relation::{Ownership, Target},
            traits::Storable,
        },
    },
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
    /// A row's owners are its [`Ownership::Owner`] relations ([`Storable::relations`], declared in
    /// `shared/relations.rs`): an address leaves with its host or its subnet, a binding with its
    /// service, address or port. References, such as an interface's `ip_address_id`, stay.
    ///
    /// Runs parents first, so one pass reaches the end of each chain.
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

        let mut present = Present::default();
        present.hold(E::Host, hosts.iter().map(|h| h.host.id));
        present.hold(E::Subnet, subnets.iter().map(|s| s.id));

        let removed_ip_addresses = present.split_unowned(ip_addresses, |ip| ip);
        let removed_interfaces = present.split_unowned(interfaces, |i| &i.interface);
        let removed_services = present.split_unowned(services, |s| &s.service);
        let removed_ports = present.split_unowned(ports, |p| p);

        present.hold(E::IPAddress, ip_addresses.iter().map(|ip| ip.id));
        present.hold(E::Interface, interfaces.iter().map(|i| i.interface.id));
        present.hold(E::Service, services.iter().map(|s| s.service.id));
        present.hold(E::Port, ports.iter().map(|p| p.id));

        let removed_bindings = present.split_unowned(bindings, |b| b);
        let removed_candidates = present.split_unowned(candidates, |c| c);
        // A neighbour row is resolved from `interface_neighbor_interfaces` or `_hosts`, so it has
        // no relations of its own. Both tables' local end (`NEIGHBOR_INTERFACE_LOCAL`,
        // `NEIGHBOR_HOST_LOCAL`) is an owner. Their far end is `OwnerKeptWhenHidden`: the row
        // still says the local port is linked (the "By link" filter reads it), and the builders
        // draw no edge to a far end the bundle doesn't hold.
        let removed_neighbours = split_off(neighbours, |row| {
            present.holds(E::Interface, row.interface_id) != Some(false)
        });

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

/// The ids of each entity type the bundle still holds, filled parents first.
#[derive(Default)]
struct Present(HashMap<E, HashSet<Uuid>>);

impl Present {
    fn hold(&mut self, entity: E, ids: impl Iterator<Item = Uuid>) {
        self.0.entry(entity).or_default().extend(ids);
    }

    /// Whether the bundle holds `id`, or `None` when it carries no rows of that type at all and
    /// so can't judge.
    fn holds(&self, entity: E, id: Uuid) -> Option<bool> {
        self.0.get(&entity).map(|ids| ids.contains(&id))
    }

    /// Move the rows of `rows` with an owner the bundle no longer holds into the returned vector.
    /// `row` reaches the stored row inside a wrapper such as `TopologyInterface`.
    fn split_unowned<W, T: Storable>(&self, rows: &mut Vec<W>, row: impl Fn(&W) -> &T) -> Vec<W> {
        split_off(rows, |wrapper| {
            let row = row(wrapper);
            T::relations().iter().all(|relation| {
                let Target::Entity(entity) = relation.target else {
                    return true;
                };
                relation.ownership != Ownership::Owner
                    || relation
                        .ids_of(row)
                        .into_iter()
                        .all(|id| self.holds(entity, id) != Some(false))
            })
        })
    }
}
