use crate::server::topology::types::views::{
    FilterValueContext, HasFilterValues, MetadataFilterType,
};
use crate::server::{
    bindings::r#impl::base::Binding,
    dependencies::r#impl::base::Dependency,
    hosts::r#impl::{base::Host, device_type::DeviceType},
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
    /// What this host is, as a device: the device type its icon shows. See [`Host::device_type`].
    #[serde(default)]
    #[schema(read_only)]
    pub device_type: DeviceType,
    /// This host's value for each metadata filter it takes part in, keyed by filter. See
    /// [`TopologyData::stamp_filter_values`].
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schema(read_only)]
    pub filter_values: BTreeMap<MetadataFilterType, String>,
}

/// A [`Service`] as the topology bundle ships it: the domain row plus its metadata filter values.
///
/// A wrapper rather than a field on [`Service`] itself: `Service` is the row every other endpoint
/// and the daemon protocol serialize, where a topology-only value would always be absent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyService {
    #[serde(flatten)]
    pub service: Service,
    /// This service's value for each metadata filter it takes part in, keyed by filter. See
    /// [`TopologyData::stamp_filter_values`].
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schema(read_only)]
    pub filter_values: BTreeMap<MetadataFilterType, String>,
}

/// An [`Interface`] as the topology bundle ships it: the domain row plus its metadata filter
/// values.
///
/// A wrapper rather than a field on [`Interface`] itself, for the same reason as
/// [`TopologyService`]: the shared row is serialized by every interface endpoint, where a
/// topology-only value would always be absent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyInterface {
    #[serde(flatten)]
    pub interface: Interface,
    /// This interface's value for each metadata filter it takes part in, keyed by filter. See
    /// [`TopologyData::stamp_filter_values`].
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schema(read_only)]
    pub filter_values: BTreeMap<MetadataFilterType, String>,
}

impl From<Service> for TopologyService {
    fn from(service: Service) -> Self {
        Self {
            service,
            filter_values: BTreeMap::new(),
        }
    }
}

impl From<Interface> for TopologyInterface {
    fn from(interface: Interface) -> Self {
        Self {
            interface,
            filter_values: BTreeMap::new(),
        }
    }
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
    pub fn wrap_all(
        hosts: Vec<Host>,
        ip_addresses: &[IPAddress],
        services: &[Service],
    ) -> Vec<Self> {
        let mut by_host: HashMap<Uuid, Vec<&IPAddress>> = HashMap::new();
        for ip in ip_addresses {
            by_host.entry(ip.base.host_id).or_default().push(ip);
        }
        let mut services_by_host: HashMap<Uuid, Vec<&Service>> = HashMap::new();
        for service in services {
            services_by_host
                .entry(service.base.host_id)
                .or_default()
                .push(service);
        }

        hosts
            .into_iter()
            .map(|host| {
                let display_name =
                    host.display_name(by_host.get(&host.id).into_iter().flatten().copied());
                let device_type = host.device_type(
                    services_by_host
                        .get(&host.id)
                        .into_iter()
                        .flatten()
                        .copied(),
                    by_host.get(&host.id).into_iter().flatten().copied(),
                );
                Self {
                    host,
                    display_name,
                    device_type,
                    filter_values: BTreeMap::new(),
                }
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

impl std::ops::Deref for TopologyService {
    type Target = Service;

    fn deref(&self) -> &Service {
        &self.service
    }
}

impl std::ops::Deref for TopologyInterface {
    type Target = Interface;

    fn deref(&self) -> &Interface {
        &self.interface
    }
}

/// Server-side metadata filters run over the bundle, which holds wrapped entities. The values
/// they filter on are the entity's own, computed rather than read from the stamped field, so the
/// filter does not depend on stamping having run first.
impl HasFilterValues for TopologyHost {
    fn filter_values(&self, ctx: &FilterValueContext) -> BTreeMap<MetadataFilterType, String> {
        self.host.filter_values(ctx)
    }
}

impl HasFilterValues for TopologyService {
    fn filter_values(&self, ctx: &FilterValueContext) -> BTreeMap<MetadataFilterType, String> {
        self.service.filter_values(ctx)
    }
}

impl HasFilterValues for TopologyInterface {
    fn filter_values(&self, ctx: &FilterValueContext) -> BTreeMap<MetadataFilterType, String> {
        self.interface.filter_values(ctx)
    }
}

/// Bundle of entities + the built graph that feed the topology render, export,
/// and share pipelines.
///
/// Loaded by [`crate::server::topology::service::main::TopologyService::get_topology_data`]
/// for either the live view (`snapshot_id = None`) or a point-in-time snapshot
/// (`snapshot_id = Some(id)`). The per-view `nodes`/`edges` are built on request
/// from these entities + the site's grouping options
/// (`build_view_graphs`) — they are not persisted. The frontend selects the
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
    /// Interfaces included in this topology, each carrying its metadata filter values.
    pub interfaces: Vec<TopologyInterface>,
    /// Resolved LLDP/CDP neighbour adjacencies (GH #701): the merged read-model of
    /// `interface_neighbor_interfaces` + `interface_neighbor_hosts`, replacing the old
    /// `Interface.neighbor` scalar now that a port can anchor several links.
    #[serde(default)]
    pub neighbours: Vec<InterfaceNeighborRow>,
    /// Raw LLDP/CDP candidate evidence behind `neighbours` (GH #701), for per-adjacency protocol
    /// derivation (`TopologyContext::interface_has_lldp_evidence`), which the resolved rows alone
    /// cannot answer (they carry no raw evidence, by design — see `interface_neighbors`).
    ///
    /// Server-side only: no topology view reads it, so it never goes over the wire. The host
    /// editor reads one interface's evidence from `GET /interfaces/{id}/neighbours`.
    #[serde(skip)]
    pub candidates: Vec<InterfaceNeighborCandidate>,
    /// Services included in this topology, each carrying its metadata filter values.
    pub services: Vec<TopologyService>,
    /// VLANs included in this topology.
    pub vlans: Vec<Vlan>,
    /// Tags assigned to this entity.
    pub tags: Vec<Tag>,
    /// The organization's Application tag group. Its tags place entities in applications.
    /// Shipped here because share and embed viewers can't list tag groups.
    #[serde(default)]
    #[schema(required)]
    pub application_group_id: Option<Uuid>,
    /// Per-view graph built on request from the entities above + grouping
    /// options. Keyed by view so switching the active perspective is a
    /// client-side slice selection.
    #[serde(default)]
    pub nodes: HashMap<TopologyView, Vec<Node>>,
    /// Connections between the nodes of the built graph.
    #[serde(default)]
    pub edges: HashMap<TopologyView, Vec<Edge>>,
    /// Per view, the entities each node stands for beyond its own `entity_id`, keyed by node id:
    /// the addresses reached through an L2 port, the services bound to all of a host's addresses
    /// on its IP cards, the members inlined on a card. The topology search selects a node when it
    /// matches any of them. Nodes with none are absent.
    #[serde(default)]
    pub node_entities: HashMap<TopologyView, HashMap<Uuid, Vec<Uuid>>>,
    /// Per view, how many entities of each type a Visibility tag left out that the view would
    /// have drawn. They never reach the browser, so this is how the filter panel knows to offer
    /// the Visibility filter only in a view, and for a type, where it hid something. Views and
    /// types with none are absent.
    #[serde(default)]
    pub hidden_by_tag: HashMap<TopologyView, HashMap<EntityDiscriminants, usize>>,
    /// How many entities of each type a server-side metadata filter removed, by the filter that
    /// removed them.
    ///
    /// Server-filtered entities never reach the browser, so this is the only way the frontend can
    /// say "171 interfaces hidden by By link" rather than presenting an empty view as an empty
    /// site. Keyed by entity and filter only, with no view: the hide-set is per view but a drop
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
    /// Whether a person added an interface-reporting credential to the organization: the L2
    /// view's setup prompt reads it. Set by the topology data endpoint, `false` elsewhere.
    #[serde(default)]
    #[schema(required)]
    pub has_interface_credential: bool,
}

impl TopologyData {
    /// Stamp every host, service and interface with its metadata filter values, so the frontend
    /// reads them off the entity instead of re-deriving them.
    ///
    /// `ctx` must be built from this bundle's own `neighbours`: an interface's link state depends
    /// on which ports point at it, and a context built from anything else would stamp a value the
    /// server-side filter never saw.
    pub fn stamp_filter_values(&mut self, ctx: &FilterValueContext) {
        for host in &mut self.hosts {
            host.filter_values = host.host.filter_values(ctx);
        }
        for service in &mut self.services {
            service.filter_values = service.service.filter_values(ctx);
        }
        for interface in &mut self.interfaces {
            interface.filter_values = interface.interface.filter_values(ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::HostBase;
    use crate::server::interface_neighbors::r#impl::base::Neighbor;
    use crate::server::interfaces::r#impl::base::{
        IfOperStatus, InterfaceBase, InterfaceLinkState,
    };
    use crate::server::services::r#impl::base::ServiceBase;
    use crate::server::services::r#impl::virtualization::{
        DockerVirtualization, ServiceVirtualization,
    };
    use crate::server::shared::storage::traits::Storable;
    use crate::server::shared::types::metadata::HasId;

    fn interface(oper_status: Option<IfOperStatus>) -> Interface {
        Interface::new(InterfaceBase {
            oper_status,
            ..Default::default()
        })
    }

    /// A bundle covering each value source: a linked port (outbound), a port linked only because
    /// another points at it, an unlinked port with no status, a containerized service and a plain
    /// one.
    fn bundle() -> TopologyData {
        let outbound = interface(Some(IfOperStatus::Up));
        let inbound = interface(Some(IfOperStatus::Down));
        let unlinked = interface(None);
        let neighbours = vec![InterfaceNeighborRow {
            id: Uuid::new_v4(),
            interface_id: outbound.id,
            neighbor: Neighbor::Interface(inbound.id),
            neighbor_seen_at: None,
        }];
        let containerized = Service::new(ServiceBase {
            virtualization_metadata: Some(ServiceVirtualization::Docker(DockerVirtualization {
                state: Default::default(),
                container_name: None,
                container_id: None,
                compose_project: None,
            })),
            ..Default::default()
        });
        TopologyData {
            hosts: TopologyHost::wrap_all(vec![Host::new(HostBase::default())], &[], &[]),
            services: vec![
                containerized.into(),
                Service::new(ServiceBase::default()).into(),
            ],
            interfaces: vec![outbound.into(), inbound.into(), unlinked.into()],
            neighbours,
            ..Default::default()
        }
    }

    /// The stamped field is what the frontend filters and colours on, and the server-side filter
    /// computes the same values afresh. They must agree for every entity, or a value the browser
    /// offers as a chip is not the value the server drops by.
    #[test]
    fn stamped_filter_values_match_each_entitys_own() {
        let mut data = bundle();
        let ctx = FilterValueContext::from_neighbours(&data.neighbours);
        data.stamp_filter_values(&ctx);

        for host in &data.hosts {
            assert!(!host.filter_values.is_empty());
            assert_eq!(host.filter_values, host.host.filter_values(&ctx));
        }
        for service in &data.services {
            assert!(!service.filter_values.is_empty());
            assert_eq!(service.filter_values, service.service.filter_values(&ctx));
        }
        for interface in &data.interfaces {
            assert!(!interface.filter_values.is_empty());
            assert_eq!(
                interface.filter_values,
                interface.interface.filter_values(&ctx)
            );
        }
    }

    /// The frontend reads `entity.filter_values[filter]` off the same object as the entity's own
    /// fields, so the wrapper has to serialize flat, keyed by the filter's id.
    #[test]
    fn filter_values_serialize_beside_the_entity_fields() {
        let mut data = bundle();
        let ctx = FilterValueContext::from_neighbours(&data.neighbours);
        data.stamp_filter_values(&ctx);

        let inbound = &data.interfaces[1];
        let json = serde_json::to_value(inbound).unwrap();
        assert_eq!(json["id"], serde_json::json!(inbound.id));
        assert_eq!(
            json["filter_values"][MetadataFilterType::LinkState.id()],
            serde_json::json!(InterfaceLinkState::Linked.id())
        );
        let round_trip: TopologyInterface = serde_json::from_value(json).unwrap();
        assert_eq!(round_trip.filter_values, inbound.filter_values);
    }
}
