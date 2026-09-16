use crate::server::topology::types::views::{
    FilterValueContext, HasFilterValues, MetadataFilterType,
};
use crate::server::{
    bindings::r#impl::base::Binding,
    dependencies::r#impl::base::Dependency,
    hosts::r#impl::base::Host,
    interface_neighbors::r#impl::base::{InterfaceNeighborCandidate, InterfaceNeighborRow},
    interfaces::r#impl::base::Interface,
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    services::r#impl::base::Service,
    shared::entities::EntityDiscriminants,
    subnets::r#impl::base::Subnet,
    tags::r#impl::base::Tag,
    topology::types::{edges::Edge, nodes::Node, views::TopologyView},
    vlans::r#impl::base::Vlan,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use utoipa::ToSchema;
use uuid::Uuid;

/// A [`Host`] as the topology bundle ships it: the domain row plus the title every surface has to
/// agree on.
///
/// The ladder itself stays on [`Host::display_name`] — this only carries its result. The bundle
/// has to carry it per host because the frontend looks a host up *by id* at sites where no node
/// for that host is guaranteed to be in the current view (a service card naming its parent, a
/// `hostA ↔ hostB` edge label, a dependency target), so [`Node::header`] cannot be the only place
/// the name exists.
///
/// A wrapper rather than a field on [`Host`] itself: `Host` is the row every other endpoint and
/// the daemon protocol serialize, where a computed title would always be absent and read as "this
/// host has no name".
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyHost {
    #[serde(flatten)]
    pub host: Host,
    /// What to call this host when `name` is empty: its hostname, sysName, chassis id or first
    /// address, whichever it has. `None` when nothing identifies it.
    ///
    /// The same ladder, and the same value, as `HostResponse.display_name` and the host
    /// container's `header` — a host cannot be called one thing on the map and another in the
    /// list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub display_name: Option<String>,
}

impl TopologyHost {
    /// Title every host in the bundle against the bundle's own addresses.
    ///
    /// Groups the addresses by host first. [`TopologyContext::get_ip_addresses_for_host`] answers
    /// the same question by rescanning the whole address list per host, and reusing it here would
    /// make titling a bundle quadratic in a place that already holds every address. Order within
    /// a host is preserved, so the ladder's "first address" rung picks the address the container
    /// header picks — by construction, not by luck.
    ///
    /// [`TopologyContext::get_ip_addresses_for_host`]: crate::server::topology::service::context::TopologyContext::get_ip_addresses_for_host
    pub fn wrap_all(hosts: Vec<Host>, ip_addresses: &[IPAddress]) -> Vec<Self> {
        let mut by_host: HashMap<Uuid, Vec<&IPAddress>> = HashMap::new();
        for ip in ip_addresses {
            by_host.entry(ip.base.host_id).or_default().push(ip);
        }

        hosts
            .into_iter()
            .map(|host| {
                let display_name =
                    host.display_name(by_host.get(&host.id).into_iter().flatten().copied());
                Self { host, display_name }
            })
            .collect()
    }
}

/// Read-through to the host: the wrapper adds a title, it does not hide the entity. This is what
/// keeps the graph builders reading `h.id` and `h.base.*` off a bundle host unchanged.
impl std::ops::Deref for TopologyHost {
    type Target = Host;

    fn deref(&self) -> &Host {
        &self.host
    }
}

/// Server-side metadata filters run over the bundle, which holds wrapped hosts. The values they
/// filter on are the host's own.
impl HasFilterValues for TopologyHost {
    fn filter_values(&self, ctx: &FilterValueContext) -> BTreeMap<MetadataFilterType, String> {
        self.host.filter_values(ctx)
    }
}

/// Bundle of entities + the built graph that feed the topology render, export,
/// and share pipelines.
///
/// Loaded by [`crate::server::topology::service::main::TopologyService::get_topology_data`]
/// for either the live view (`snapshot_id = None`) or a point-in-time snapshot
/// (`snapshot_id = Some(id)`). The per-view `nodes`/`edges` are built on request
/// from these entities + the network's grouping options
/// (`build_all_view_graphs`) — they are not persisted. The frontend selects the
/// active view's slice client-side.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TopologyData {
    /// Hosts included in this topology, each carrying the title the map and the host list share.
    pub hosts: Vec<TopologyHost>,
    /// IP addresses included in this topology.
    pub ip_addresses: Vec<IPAddress>,
    /// Subnets included in this topology.
    pub subnets: Vec<Subnet>,
    /// Dependencies included in this topology.
    pub dependencies: Vec<Dependency>,
    /// Ports included in this topology.
    pub ports: Vec<Port>,
    /// Service bindings included in this topology.
    pub bindings: Vec<Binding>,
    /// Interfaces included in this topology.
    pub interfaces: Vec<Interface>,
    /// Resolved LLDP/CDP neighbour adjacencies (GH #701): the merged read-model of
    /// `interface_neighbor_interfaces` + `interface_neighbor_hosts`, replacing the old
    /// `Interface.neighbor` scalar now that a port can anchor several links.
    #[serde(default)]
    pub neighbours: Vec<InterfaceNeighborRow>,
    /// Raw LLDP/CDP candidate evidence behind `neighbours` (GH #701) — for the admin/debug
    /// unresolved-evidence display and per-adjacency protocol derivation
    /// (`TopologyContext::interface_has_lldp_evidence`), neither of which the resolved rows alone
    /// can answer (they carry no raw evidence, by design — see `interface_neighbors`).
    #[serde(default)]
    pub candidates: Vec<InterfaceNeighborCandidate>,
    /// Services included in this topology.
    pub services: Vec<Service>,
    /// VLANs included in this topology.
    pub vlans: Vec<Vlan>,
    /// Tags assigned to this entity.
    pub tags: Vec<Tag>,
    /// Per-view graph built on request from the entities above + grouping
    /// options. Keyed by view so switching the active perspective is a
    /// client-side slice selection.
    #[serde(default)]
    pub nodes: HashMap<TopologyView, Vec<Node>>,
    /// Connections between the nodes of the built graph.
    #[serde(default)]
    pub edges: HashMap<TopologyView, Vec<Edge>>,
    /// How many entities of each type a server-side metadata filter removed, by the filter that
    /// removed them.
    ///
    /// Server-filtered entities never reach the browser, so this is the only way the frontend can
    /// say "171 interfaces hidden by By link" rather than presenting an empty view as an empty
    /// network. Keyed by entity and filter only, with no view: the hide-set is per view but a drop
    /// is not — an entity is removed from the one shared bundle only when *every* view that could
    /// render it hides it (see `metadata_filter`).
    ///
    /// Empty when nothing was filtered, which is the common case.
    #[serde(default)]
    pub filtered_out: HashMap<EntityDiscriminants, BTreeMap<MetadataFilterType, usize>>,
    /// Views whose data is present in this entity set (L3/Workloads always;
    /// L2 Physical iff LLDP/CDP neighbors exist; Application iff app-flagged
    /// tags are used). The topology tab restricts a snapshot's view picker to
    /// these — you can't set up SNMP or create app tags on a historical
    /// snapshot — while the live view shows all views with setup prompts.
    #[serde(default)]
    pub available_views: Vec<TopologyView>,
}
