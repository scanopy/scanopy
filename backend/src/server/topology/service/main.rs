use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use anyhow::Error;
use async_trait::async_trait;
use petgraph::{Graph, graph::NodeIndex};
use strum::IntoEnumIterator;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::metadata_filter;
use crate::server::shared::entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants};
use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::topology::types::views::{FilterValueContext, MetadataFilterType};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::{r#impl::base::Binding, service::BindingService},
    dependencies::{r#impl::base::Dependency, service::DependencyService},
    hosts::{r#impl::base::Host, service::HostService},
    interface_neighbors::{
        r#impl::base::{InterfaceNeighborCandidate, InterfaceNeighborRow},
        service::InterfaceNeighborService,
    },
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    ip_addresses::{r#impl::base::IPAddress, service::IPAddressService},
    networks::service::NetworkService,
    ports::{r#impl::base::Port, service::PortService},
    services::{r#impl::base::Service, service::ServiceService},
    shared::{
        events::{bus::EventBus, types::EntityOperation},
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, Storable, Storage},
        },
    },
    subnets::{r#impl::base::Subnet, service::SubnetService},
    tags::{entity_tags::EntityTagService, r#impl::base::Tag, service::TagService},
    topology::{
        service::{context::TopologyContext, edge_builder::EdgeBuilder},
        types::{
            api::{TopologyData, TopologyHost},
            base::{Topology, TopologyOptions},
            edges::Edge,
            grouping::{ContainerRule, ElementRule, GroupingConfig},
            nodes::Node,
            views::{TopologyView, TopologyViewSupport},
        },
    },
    vlans::{r#impl::base::Vlan, service::VlanService},
};

pub struct TopologyService {
    storage: Arc<GenericPostgresStorage<Topology>>,
    pub(crate) host_service: Arc<HostService>,
    pub(crate) ip_address_service: Arc<IPAddressService>,
    pub(crate) subnet_service: Arc<SubnetService>,
    pub(crate) dependency_service: Arc<DependencyService>,
    pub(crate) service_service: Arc<ServiceService>,
    pub(crate) port_service: Arc<PortService>,
    pub(crate) binding_service: Arc<BindingService>,
    pub(crate) interface_service: Arc<InterfaceService>,
    pub(crate) interface_neighbor_service: Arc<InterfaceNeighborService>,
    pub(crate) tag_service: Arc<TagService>,
    pub(crate) vlan_service: Arc<VlanService>,
    pub(crate) network_service: Arc<NetworkService>,
    event_bus: Arc<EventBus>,
    /// Broadcast channel emitting `network_id`s whose live entity set has
    /// just changed. Frontend SSE consumers refetch the live topology row +
    /// entity data on receipt. Replaces the legacy staleness state machine.
    pub live_update_tx: broadcast::Sender<Uuid>,
}

impl EventBusService<Topology> for TopologyService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Topology) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Topology) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Topology> for TopologyService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Topology>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }

    /// Create a topology row.
    ///
    /// The row holds only the user's grouping `options`; the per-view graph is
    /// built on request from entities + options (see `build_all_view_graphs`),
    /// so there's nothing to seed here. One live row per network.
    async fn create(
        &self,
        entity: Topology,
        authentication: AuthenticatedEntity,
    ) -> Result<Topology, anyhow::Error> {
        let topology = if entity.id() == Uuid::nil() {
            Topology::new(entity.get_base())
        } else {
            entity
        };

        let created = self.storage().create(&topology).await?;

        if let Some(scope) = EntityScope::from_ids(
            created.id(),
            created.clone().into(),
            self.get_network_id(&created),
            self.get_organization_id(&created),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Created, authentication)
                        .with_flags(EntityEventFlags::default()),
                )
                .await?;
        }

        Ok(created)
    }

    /// Persist a topology update (grouping `options` only).
    ///
    /// The graph is no longer stored — grouping/hide-rule edits change
    /// `options.request` and take effect on the next on-request build, so there
    /// is nothing to rebuild here. `trigger_stale` still rides on the published
    /// event so any interested consumer can tell a grouping change from a no-op.
    /// View switching is a client-side slice selection and does NOT reach here.
    async fn update(
        &self,
        entity: &mut Topology,
        authentication: AuthenticatedEntity,
    ) -> Result<Topology, Error> {
        let current = self
            .get_by_id(&entity.id())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not find topology {}", entity.id()))?;

        let trigger_stale = entity.triggers_staleness(Some(current.clone()));

        let updated = self.storage().update(entity).await?;

        if let Some(scope) = EntityScope::from_ids(
            updated.id(),
            updated.clone().into(),
            self.get_network_id(&updated),
            self.get_organization_id(&updated),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Updated, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(updated)
    }
}

pub struct BuildGraphParams<'a> {
    pub options: &'a TopologyOptions,
    pub hosts: &'a [Host],
    pub ip_addresses: &'a [IPAddress],
    pub subnets: &'a [Subnet],
    pub services: &'a [Service],
    pub dependencies: &'a [Dependency],
    pub ports: &'a [Port],
    pub bindings: &'a [Binding],
    pub interfaces: &'a [Interface],
    /// GH #701: the merged neighbour read-model — see `TopologyContext::neighbours`.
    pub neighbours: &'a [InterfaceNeighborRow],
    /// Raw candidate evidence — see `TopologyContext::candidates`.
    pub candidates: &'a [InterfaceNeighborCandidate],
    pub entity_tags: &'a [Tag],
    pub vlans: &'a [Vlan],
    pub old_nodes: &'a [Node],
    pub old_edges: &'a [Edge],
    pub old_view: Option<TopologyView>,
    /// View this graph is being built for — selects the builder, grouping, and
    /// per-edge view config.
    pub view: TopologyView,
}

/// Whether any interface in this set qualifies its network for the L2 Physical view.
///
/// Two conditions, matching `l2_builder.rs`'s `qualifying_host_ids` exactly (kept in sync
/// deliberately — this answers "should the tab even be offered", that answers "which hosts does
/// it draw", and a network can only offer the tab honestly if at least one host would qualify):
/// a resolved neighbour (port-precise or device-level), or a host with no IP address at all,
/// identified only by an interface's MAC — the PROFINET DCP identify case, which has neither a
/// neighbour nor an IP for `subnet_graph_builder.rs` to place it by.
///
/// Neighbours arrive as `has_resolved_neighbours` rather than being read off the interfaces:
/// they live in their own tables now (GH #701), so the caller resolves them from
/// `InterfaceNeighborService` and passes the answer in.
fn any_interface_qualifies_l2_physical(
    interfaces: &[Interface],
    ip_addresses: &[IPAddress],
    has_resolved_neighbours: bool,
) -> bool {
    if has_resolved_neighbours {
        return true;
    }
    let hosts_with_ip: HashSet<Uuid> = ip_addresses.iter().map(|ip| ip.base.host_id).collect();
    interfaces
        .iter()
        .any(|i| i.base.mac_address.is_some() && !hosts_with_ip.contains(&i.base.host_id))
}

impl TopologyService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host_service: Arc<HostService>,
        ip_address_service: Arc<IPAddressService>,
        subnet_service: Arc<SubnetService>,
        dependency_service: Arc<DependencyService>,
        service_service: Arc<ServiceService>,
        port_service: Arc<PortService>,
        binding_service: Arc<BindingService>,
        interface_service: Arc<InterfaceService>,
        interface_neighbor_service: Arc<InterfaceNeighborService>,
        tag_service: Arc<TagService>,
        vlan_service: Arc<VlanService>,
        network_service: Arc<NetworkService>,
        storage: Arc<GenericPostgresStorage<Topology>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        let (live_update_tx, _) = broadcast::channel(100);
        Self {
            host_service,
            ip_address_service,
            subnet_service,
            dependency_service,
            service_service,
            storage,
            port_service,
            binding_service,
            interface_service,
            interface_neighbor_service,
            tag_service,
            vlan_service,
            network_service,
            event_bus,
            live_update_tx,
        }
    }

    /// Subscribe to live-topology update pings. Each emitted `Uuid` is a
    /// network whose live entity set just changed; consumers refetch the
    /// live topology row + entity data.
    pub fn subscribe_live_topology_updates(&self) -> broadcast::Receiver<Uuid> {
        self.live_update_tx.subscribe()
    }

    /// Unified entity-set loader for live + snapshot topology reads.
    ///
    /// `snapshot_id = None` reads live entity rows (`valid_to IS NULL`).
    /// `snapshot_id = Some(id)` reads closed copies stamped at that snapshot
    /// (`snapshot_id = id`) — those have distinct ids from their live
    /// counterparts and survive live-row deletion.
    pub async fn get_topology_data(
        &self,
        network_id: Uuid,
        snapshot_id: Option<Uuid>,
    ) -> Result<TopologyData, Error> {
        // Hosts/services/subnets carry the tags that drive topology grouping +
        // display, so hydrate their tag associations as-of the snapshot (from
        // the closed `entity_tags`), not from live `entity_tags` (which key on
        // live, not closed, entity ids). Other entity types don't surface tags
        // in the topology, so plain `get_all` (live hydration) is fine for them.
        let hosts = self
            .host_service
            .get_all_as_of_snapshot(
                apply_snapshot(
                    StorableFilter::<Host>::new_from_network_ids(&[network_id]).hidden_is(false),
                    snapshot_id,
                ),
                snapshot_id,
            )
            .await?;
        let ip_addresses = self
            .ip_address_service
            .get_all(apply_snapshot(
                StorableFilter::<IPAddress>::new_from_network_ids(&[network_id]),
                snapshot_id,
            ))
            .await?;
        let subnets = self
            .subnet_service
            .get_all_as_of_snapshot(
                apply_snapshot(
                    StorableFilter::<Subnet>::new_from_network_ids(&[network_id]),
                    snapshot_id,
                ),
                snapshot_id,
            )
            .await?;
        let dependencies = self
            .dependency_service
            .get_all(apply_snapshot(
                StorableFilter::<Dependency>::new_from_network_ids(&[network_id]),
                snapshot_id,
            ))
            .await?;
        let ports = self
            .port_service
            .get_all(apply_snapshot(
                StorableFilter::<Port>::new_from_network_ids(&[network_id]),
                snapshot_id,
            ))
            .await?;
        let bindings = self
            .binding_service
            .get_all(apply_snapshot(
                StorableFilter::<Binding>::new_from_network_ids(&[network_id]),
                snapshot_id,
            ))
            .await?;
        let interfaces = self
            .interface_service
            .get_all(apply_snapshot(
                StorableFilter::<Interface>::new_from_network_ids(&[network_id]),
                snapshot_id,
            ))
            .await?;
        // GH #701: the merged neighbour read-model, pinned to the same live/snapshot state as
        // `interfaces` above. `candidates` are never Snapshotable (disposable, replaced wholesale
        // every scan — see `interface_neighbors`), so they're always read live regardless of
        // `snapshot_id`; a historical snapshot's L2 view renders from `neighbours` alone.
        let neighbours = self
            .interface_neighbor_service
            .resolved_for_network(network_id, snapshot_id)
            .await?;
        let candidates = self
            .interface_neighbor_service
            .candidates_for_network(network_id)
            .await?;
        let services = self
            .service_service
            .get_all_as_of_snapshot(
                apply_snapshot(
                    StorableFilter::<Service>::new_from_network_ids(&[network_id]),
                    snapshot_id,
                ),
                snapshot_id,
            )
            .await?;
        let vlans = self
            .vlan_service
            .get_all(apply_snapshot(
                StorableFilter::<Vlan>::new_from_uuid_column("network_id", &network_id),
                snapshot_id,
            ))
            .await?;
        let tags = self.get_entity_tags(&hosts, &services, &subnets).await?;

        // Which views have data in THIS entity set (snapshot-aware). Reuses the
        // same `TopologyViewSupport` / `is_supported` logic that governs shares,
        // but computed from the entities just loaded (so a snapshot reflects its
        // captured data, not live). The topology tab uses this to hide views a
        // snapshot can't populate (no LLDP neighbors → no L2; no app tags → no
        // Application).
        let support = TopologyViewSupport {
            // Any resolved neighbour, port-precise or device-level. A network whose links have
            // all degraded to `Neighbor::Host` still has an L2 topology to show — dashed
            // `NeighborLink` edges between host containers — and hiding the view is the one
            // outcome that leaves the operator nothing to look at. Failing that, a no-IP host
            // identified only by MAC evidence still qualifies.
            l2_physical: any_interface_qualifies_l2_physical(
                &interfaces,
                &ip_addresses,
                !neighbours.is_empty(),
            ),
            application: tags.iter().any(|t| t.base.is_application),
        };
        let available_views: Vec<TopologyView> = TopologyView::iter()
            .filter(|v| v.is_supported(&support))
            .collect();

        // Title the hosts once, here, from the addresses this same bundle carries — so the map,
        // the host list and every by-id lookup on the frontend read one value rather than each
        // re-deriving the ladder. Last, because `get_entity_tags` above wants plain `&[Host]`.
        let hosts = TopologyHost::wrap_all(hosts, &ip_addresses);

        Ok(TopologyData {
            hosts,
            ip_addresses,
            subnets,
            dependencies,
            ports,
            bindings,
            interfaces,
            neighbours,
            candidates,
            services,
            vlans,
            tags,
            available_views,
            // Built on request by `get_topology_render_data`; the bare entity
            // loader leaves them empty.
            nodes: HashMap::new(),
            edges: HashMap::new(),
            // Filled by `apply_server_metadata_filters`, which runs on the way to the render
            // path; nothing has been dropped yet at this point.
            filtered_out: HashMap::new(),
        })
    }

    /// Fetch tag *definitions* for all tags referenced by hosts, services, and
    /// subnets (the referenced ids come from each entity's hydrated `tags`).
    ///
    /// Always read **live** — tag definitions are org-scoped and are NOT cloned
    /// at snapshot time, so there is no snapshot-pinned tag row to read. The
    /// per-snapshot part is the *association* (the closed `entity_tags`, which
    /// populate the entities' `tags` upstream); the definition (name/color) is
    /// shown as it is now, consistent with the "inspector entity details are
    /// always live" model.
    pub async fn get_entity_tags(
        &self,
        hosts: &[Host],
        services: &[Service],
        subnets: &[Subnet],
    ) -> Result<Vec<Tag>, Error> {
        let mut tag_ids: Vec<Uuid> = Vec::new();
        for host in hosts {
            tag_ids.extend(&host.base.tags);
        }
        for service in services {
            tag_ids.extend(&service.base.tags);
        }
        for subnet in subnets {
            tag_ids.extend(&subnet.base.tags);
        }

        tag_ids.sort();
        tag_ids.dedup();

        if tag_ids.is_empty() {
            return Ok(vec![]);
        }

        let filter = StorableFilter::<Tag>::new_from_entity_ids(&tag_ids).live();
        let tags = self.tag_service.get_all(filter).await?;

        Ok(tags)
    }

    /// Compute per-view data-support flags for a network's topology by
    /// querying raw entity tables — independent of whatever the topology
    /// was last rebuilt under.
    pub async fn get_view_support(&self, network_id: Uuid) -> Result<TopologyViewSupport, Error> {
        // Device-level neighbours count too — see the equivalent in `get_topology_data`. Live
        // only: this check has no snapshot context, and the live view is what it governs.
        let has_resolved_neighbours = !self
            .interface_neighbor_service
            .resolved_for_network(network_id, None)
            .await?
            .is_empty();
        let interfaces = self
            .interface_service
            .get_all(StorableFilter::<Interface>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;
        let ip_addresses = self
            .ip_address_service
            .get_all(StorableFilter::<IPAddress>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;
        let l2_physical = any_interface_qualifies_l2_physical(
            &interfaces,
            &ip_addresses,
            has_resolved_neighbours,
        );

        let application = match self.network_service.get_by_id(&network_id).await? {
            Some(network) => self
                .tag_service
                .get_all(StorableFilter::<Tag>::new_from_org_id(
                    &network.base.organization_id,
                ))
                .await?
                .iter()
                .any(|t| t.base.is_application),
            None => false,
        };

        Ok(TopologyViewSupport {
            l2_physical,
            application,
        })
    }

    /// Load the entity set for `(network_id, snapshot_id)` and build the
    /// per-view graph on request from it + the network's grouping options.
    /// Single source for the render, export, and share paths now that the graph
    /// is no longer persisted. Snapshots use the network's (live) options —
    /// the same behaviour the former `build_snapshot_topology` had.
    pub async fn get_topology_render_data(
        &self,
        network_id: Uuid,
        snapshot_id: Option<Uuid>,
    ) -> Result<TopologyData, Error> {
        let options = self.network_topology_options(network_id).await?;
        let mut data = self.get_topology_data(network_id, snapshot_id).await?;
        // `data.tags` only carries tags applied to entities. A grouping rule can
        // reference a tag applied to nothing (e.g. ByTag on an unused tag); the
        // frontend needs its name/color to label the group, so ship it too.
        self.augment_grouping_rule_tags(&mut data, &options).await?;
        Self::apply_server_metadata_filters(&mut data, &options);
        let (nodes, edges) = self.build_all_view_graphs(&data, &options);
        data.nodes = nodes;
        data.edges = edges;
        Ok(data)
    }

    /// Drop entities the user has hidden through a `Server`-side metadata filter.
    ///
    /// Runs before any view is built, so the builders never see them and nodes, edges and the
    /// entity bundle all shrink together. The bundle is what makes this worth doing: a `Node` is an
    /// id and a position, while the entity records are the payload — L2 ships 19,095 interfaces to
    /// render 2,872, and the difference is what exhausts a customer's browser.
    ///
    /// Silent when nothing is hidden server-side, which is the common case.
    fn apply_server_metadata_filters(data: &mut TopologyData, options: &TopologyOptions) {
        let hide_sets = metadata_filter::server_hide_sets(options);
        if hide_sets.is_empty() {
            return;
        }

        let ctx = FilterValueContext {
            interfaces_referenced_as_neighbours: metadata_filter::referenced_neighbour_interfaces(
                data.neighbours.iter(),
            ),
            interfaces_with_neighbours: metadata_filter::interfaces_with_neighbours(
                data.neighbours.iter(),
            ),
        };

        let mut filtered_out: HashMap<EntityDiscriminants, BTreeMap<MetadataFilterType, usize>> =
            HashMap::new();
        let mut record = |entity, tally: BTreeMap<MetadataFilterType, usize>| {
            if !tally.is_empty() {
                filtered_out.insert(entity, tally);
            }
        };

        record(
            EntityDiscriminants::Interface,
            metadata_filter::retain_visible(
                &mut data.interfaces,
                hide_sets.get(&EntityDiscriminants::Interface),
                &ctx,
            ),
        );
        record(
            EntityDiscriminants::Host,
            metadata_filter::retain_visible(
                &mut data.hosts,
                hide_sets.get(&EntityDiscriminants::Host),
                &ctx,
            ),
        );
        record(
            EntityDiscriminants::Service,
            metadata_filter::retain_visible(
                &mut data.services,
                hide_sets.get(&EntityDiscriminants::Service),
                &ctx,
            ),
        );

        let dropped: usize = filtered_out
            .values()
            .flat_map(|by_filter| by_filter.values())
            .sum();
        if dropped > 0 {
            tracing::debug!(dropped, "server-side metadata filters removed entities");
        }
        data.filtered_out = filtered_out;
    }

    /// Add tags referenced by grouping rules (ByTag element rules, ByApplication
    /// container rules) to `data.tags` when not already present, so the frontend
    /// can resolve their name/color. Tag definitions are org-scoped and never
    /// snapshot-cloned, so these are always loaded live.
    async fn augment_grouping_rule_tags(
        &self,
        data: &mut TopologyData,
        options: &TopologyOptions,
    ) -> Result<(), Error> {
        let mut rule_tag_ids: Vec<Uuid> = Vec::new();
        for r in &options.request.element_rules {
            if let ElementRule::ByTag { tag_ids, .. } = &r.rule {
                rule_tag_ids.extend(tag_ids);
            }
        }
        for rules in options.request.container_rules.values() {
            for r in rules {
                if let ContainerRule::ByApplication { tag_ids } = &r.rule {
                    rule_tag_ids.extend(tag_ids);
                }
            }
        }

        let present: HashSet<Uuid> = data.tags.iter().map(|t| t.id).collect();
        let mut missing: Vec<Uuid> = rule_tag_ids
            .into_iter()
            .filter(|id| !present.contains(id))
            .collect();
        missing.sort();
        missing.dedup();
        if missing.is_empty() {
            return Ok(());
        }

        let extra = self
            .tag_service
            .get_all(StorableFilter::<Tag>::new_from_entity_ids(&missing).live())
            .await?;
        data.tags.extend(extra);
        Ok(())
    }

    /// The network's single live topology row holds the user's grouping
    /// `options`. Defaults if the row is somehow absent.
    pub async fn network_topology_options(
        &self,
        network_id: Uuid,
    ) -> Result<TopologyOptions, Error> {
        let rows = self
            .get_all(StorableFilter::<Topology>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;
        Ok(rows
            .into_iter()
            .next()
            .map(|t| t.base.options)
            .unwrap_or_default())
    }

    /// Build a node/edge set for every view from a single entity snapshot.
    ///
    /// Pure function of `data` + `options` — called on request by the read,
    /// export, and share paths (the graph is no longer persisted). Returns one
    /// node/edge slice per view so the client can switch views without a
    /// refetch.
    pub fn build_all_view_graphs(
        &self,
        data: &TopologyData,
        options: &TopologyOptions,
    ) -> (
        HashMap<TopologyView, Vec<Node>>,
        HashMap<TopologyView, Vec<Edge>>,
    ) {
        let mut nodes_by_view = HashMap::new();
        let mut edges_by_view = HashMap::new();

        // The builders take plain domain hosts; the bundle carries them wrapped with the title
        // computed at load. Unwrap once here rather than threading the wrapper through
        // `TopologyContext` and every builder — and once for all views, not once each.
        let hosts: Vec<Host> = data.hosts.iter().map(|h| h.host.clone()).collect();

        for view in TopologyView::iter() {
            let (nodes, edges) = self.build_graph(BuildGraphParams {
                hosts: &hosts,
                ip_addresses: &data.ip_addresses,
                services: &data.services,
                subnets: &data.subnets,
                dependencies: &data.dependencies,
                ports: &data.ports,
                bindings: &data.bindings,
                interfaces: &data.interfaces,
                neighbours: &data.neighbours,
                candidates: &data.candidates,
                entity_tags: &data.tags,
                vlans: &data.vlans,
                // No stored prior graph to preserve handles from — overrides
                // aren't persisted (handle-preservation is disabled below).
                old_nodes: &[],
                old_edges: &[],
                options,
                old_view: None,
                view,
            });

            nodes_by_view.insert(view, nodes);
            edges_by_view.insert(view, edges);
        }

        (nodes_by_view, edges_by_view)
    }

    pub fn build_graph(&self, params: BuildGraphParams) -> (Vec<Node>, Vec<Edge>) {
        let BuildGraphParams {
            hosts,
            ip_addresses,
            subnets,
            services,
            dependencies,
            ports,
            bindings,
            interfaces,
            neighbours,
            candidates,
            entity_tags,
            vlans,
            old_edges,
            old_nodes,
            options,
            old_view,
            view,
        } = params;

        // Create context to avoid parameter passing
        let ctx = TopologyContext::new(
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
            options,
            view,
        )
        .with_neighbours(neighbours)
        .with_candidates(candidates);

        // Build grouping config from request options
        let grouping = GroupingConfig::from_request_options(&options.request, view);

        // Select builder by view and build nodes + edges
        let builder = super::view::builder_for_view(view);
        let (all_nodes, mut all_edges) = builder.build(&ctx, &grouping);

        // Set per-view edge configuration and the view-independent relation identity
        for edge in &mut all_edges {
            edge.view_config = view.edge_view_config((&edge.edge_type).into());
            edge.relation_key = edge.edge_type.relation_key();
        }

        let final_edges = all_edges;

        // Build graph
        let mut graph: Graph<Node, Edge> = Graph::new();
        let node_indices: HashMap<Uuid, NodeIndex> = all_nodes
            .into_iter()
            .map(|node| {
                let node_id = node.id;
                let node_idx = graph.add_node(node);
                (node_id, node_idx)
            })
            .collect();

        // Add edges to graph
        EdgeBuilder::add_edges_to_graph(&mut graph, &node_indices, final_edges);

        // User layout overrides (edge handle reconnect) are no longer persisted,
        // so there's no prior graph to carry handles forward from. The
        // handle-preservation pass below is DISABLED — kept (commented) for
        // revival if override persistence is reintroduced. `old_nodes` /
        // `old_edges` / `old_view` are fed empty by `build_all_view_graphs`.
        let _ = (old_nodes, old_edges, old_view);
        /*
        // Skip handle preservation when view has changed — old handles are not meaningful
        let view_unchanged = match old_view {
            Some(old_v) => old_v == view,
            None => true,
        };

        if view_unchanged {
            // Build previous graph to compare and determine if user edits should be persisted
            // If nodes have changed edges, assume they have moved and user edits are no longer applicable
            let mut old_graph: Graph<Node, Edge> = Graph::new();
            let old_node_indices: HashMap<Uuid, NodeIndex> = old_nodes
                .iter()
                .map(|node| {
                    let node_id = node.id;
                    let node_idx = old_graph.add_node(node.clone());
                    (node_id, node_idx)
                })
                .collect();

            EdgeBuilder::add_edges_to_graph(&mut old_graph, &old_node_indices, old_edges.to_vec());

            // Create a map of old edges by their source/target for quick lookup
            let mut old_edges_map: HashMap<(Uuid, Uuid), &Edge> = HashMap::new();
            for edge_ref in old_graph.edge_references() {
                let edge = edge_ref.weight();
                old_edges_map.insert((edge.source, edge.target), edge);
            }

            // Preserve handles for nodes with unchanged edge count
            let mut edges_to_update: Vec<(petgraph::prelude::EdgeIndex, EdgeHandle, EdgeHandle)> =
                Vec::new();

            for node in graph.node_weights() {
                if let Some(old_idx) = old_node_indices.get(&node.id)
                    && let Some(new_idx) = node_indices.get(&node.id)
                {
                    let old_edge_count = old_graph.edges(*old_idx).count();
                    let new_edge_count = graph.edges(*new_idx).count();

                    if old_edge_count == new_edge_count {
                        for edge_ref in graph.edges(*new_idx) {
                            let new_edge = edge_ref.weight();
                            if let Some(old_edge) =
                                old_edges_map.get(&(new_edge.source, new_edge.target))
                            {
                                edges_to_update.push((
                                    edge_ref.id(),
                                    old_edge.source_handle,
                                    old_edge.target_handle,
                                ));
                            }
                        }
                    }
                }
            }

            // Now apply the updates
            for (edge_idx, source_handle, target_handle) in edges_to_update {
                if let Some(edge) = graph.edge_weight_mut(edge_idx) {
                    edge.source_handle = source_handle;
                    edge.target_handle = target_handle;
                }
            }
        }
        */

        (
            graph.node_weights().cloned().collect(),
            graph.edge_weights().cloned().collect(),
        )
    }
}

/// Switch a filter between live and snapshot modes based on `snapshot_id`.
/// Live view: live rows (`valid_to IS NULL`). Snapshot view: closed copies
/// stamped with the snapshot's id (distinct rows from the live versions, and
/// survive live-row hard-deletes).
fn apply_snapshot<T: Storable>(
    f: StorableFilter<T>,
    snapshot_id: Option<Uuid>,
) -> StorableFilter<T> {
    match snapshot_id {
        None => f.live(),
        Some(id) => f.snapshot_id(&id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::interfaces::r#impl::base::InterfaceBase;
    use crate::server::ip_addresses::r#impl::base::{IPAddressBase, MacEvidence, MacEvidenceValue};
    use crate::server::shared::attribution::AttributeSource;

    fn interface(host_id: Uuid, mac: Option<&str>) -> Interface {
        Interface::new(InterfaceBase {
            host_id,
            mac_address: mac.map(|m| {
                MacEvidence::new(
                    MacEvidenceValue(m.parse().unwrap()),
                    AttributeSource::ProfinetDcp,
                )
            }),
            ..Default::default()
        })
    }

    fn ip_address(host_id: Uuid) -> IPAddress {
        IPAddress::new(IPAddressBase {
            host_id,
            ip_address: "10.0.0.1".parse().unwrap(),
            ..Default::default()
        })
    }

    /// The case this function exists for: a network whose only L2-relevant data is a PROFINET
    /// DCP-identified host (no IP, no neighbour) must still offer the L2 Physical tab —
    /// otherwise the view `l2_builder.rs` would draw a container in is never reachable.
    #[test]
    fn a_no_ip_mac_only_host_qualifies_the_network_for_l2() {
        let host_id = Uuid::new_v4();
        let interfaces = vec![interface(host_id, Some("aa:bb:cc:dd:ee:ff"))];
        assert!(any_interface_qualifies_l2_physical(&interfaces, &[], false));
    }

    /// The condition is "no IP *at all*", not "this interface has no IP" — a host with an IP
    /// recorded elsewhere (its own `IPAddress` row) doesn't qualify the network on its MAC
    /// alone; it's already visible via L3, and must still require a neighbour like any other.
    #[test]
    fn a_mac_carrying_interface_on_a_host_that_has_an_ip_does_not_qualify_on_its_own() {
        let host_id = Uuid::new_v4();
        let interfaces = vec![interface(host_id, Some("aa:bb:cc:dd:ee:ff"))];
        let ip_addresses = vec![ip_address(host_id)];
        assert!(!any_interface_qualifies_l2_physical(
            &interfaces,
            &ip_addresses,
            false
        ));
    }

    #[test]
    fn no_mac_and_no_neighbour_does_not_qualify() {
        let interfaces = vec![interface(Uuid::new_v4(), None)];
        assert!(!any_interface_qualifies_l2_physical(
            &interfaces,
            &[],
            false
        ));
    }

    /// The other half of the predicate, which moved out of `Interface` and into its own tables
    /// (GH #701): a resolved neighbour qualifies the network on its own, whatever the interfaces
    /// carry. A host with an IP and no MAC fails every other condition.
    #[test]
    fn a_resolved_neighbour_qualifies_the_network_on_its_own() {
        let host_id = Uuid::new_v4();
        let interfaces = vec![interface(host_id, None)];
        let ip_addresses = vec![ip_address(host_id)];
        assert!(any_interface_qualifies_l2_physical(
            &interfaces,
            &ip_addresses,
            true
        ));
    }
}
