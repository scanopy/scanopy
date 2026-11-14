use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use anyhow::Error;
use async_trait::async_trait;
use petgraph::{Graph, graph::NodeIndex};
use strum::IntoEnumIterator;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::bundle_closure::OrphanRows;
use super::metadata_filter;
use crate::server::shared::entities::EntityDiscriminants;
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
    organizations::{r#impl::base::UseCase, service::OrganizationService},
    ports::{r#impl::base::Port, service::PortService},
    services::{r#impl::base::Service, service::ServiceService},
    shared::{
        events::{
            bus::EventBus,
            types::{EntityOperation, UpdateReason},
        },
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, Storable, Storage},
        },
    },
    sites::service::SiteService,
    subnets::{r#impl::base::Subnet, service::SubnetService},
    tags::{entity_tags::EntityTagService, r#impl::base::Tag, service::TagService},
    topology::{
        service::{context::TopologyContext, edge_builder::EdgeBuilder},
        types::{
            api::{
                TopologyData,
                TopologyHost,
                TopologyInterface,
                // The bundle row, not this module's `TopologyService` (the service itself).
                TopologyService as TopologyServiceRow,
            },
            base::{Topology, TopologyOptions, TopologyRequestOptions},
            edges::Edge,
            grouping::{ContainerRule, ElementRule, GroupingConfig},
            nodes::{Node, NodeType},
            views::{TopologyView, TopologyViewSupport},
        },
    },
    vlans::{r#impl::base::Vlan, service::VlanService},
};

/// The rows a Visibility tag left out of a bundle. Set aside rather than dropped, so each view can
/// count the ones it would have drawn: a hidden host with no services is nothing Workloads would
/// show, and the Workloads filter panel shouldn't offer to bring it back.
#[derive(Debug, Default)]
pub struct TagHidden {
    pub hosts: Vec<Host>,
    pub subnets: Vec<Subnet>,
    pub services: Vec<Service>,
    /// The rows that left the bundle with a removed parent: the hidden rows' children, which come
    /// back with them, and a dismissed host's, which stay out because their host never does.
    pub orphans: OrphanRows,
}

impl TagHidden {
    fn is_empty(&self) -> bool {
        self.hosts.is_empty() && self.subnets.is_empty() && self.services.is_empty()
    }
}

/// A bundle with its Visibility-hidden rows put back, for building the views they would be in.
struct TagHiddenBundle {
    data: TopologyData,
    hidden_types: HashMap<Uuid, EntityDiscriminants>,
}

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
    pub(crate) site_service: Arc<SiteService>,
    pub(crate) organization_service: Arc<OrganizationService>,
    event_bus: Arc<EventBus>,
    /// Broadcast channel emitting `site_id`s whose live entity set has
    /// just changed. Frontend SSE consumers refetch the live topology row +
    /// entity data on receipt. Replaces the legacy staleness state machine.
    pub live_update_tx: broadcast::Sender<Uuid>,
}

impl EventBusService<Topology> for TopologyService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &Topology) -> Option<Uuid> {
        Some(entity.base.site_id)
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
    /// built on request from entities + options (see `build_view_graphs`),
    /// so there's nothing to seed here. One live row per site.
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
            self.get_site_id(&created),
            self.get_organization_id(&created),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Created, authentication)
                        .with_flags(EntityEventFlags::default()),
                )
                .await;
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
        reason: UpdateReason,
    ) -> Result<Topology, Error> {
        let current = self
            .get_by_id(&entity.id())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not find topology {}", entity.id()))?;

        let updated = self.storage().update(entity).await?;

        self.publish_updated(&current, &updated, authentication, reason)
            .await;

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
    /// The organization's Application tag group: tags in it place entities in applications.
    pub application_group_id: Option<Uuid>,
    pub vlans: &'a [Vlan],
    pub old_nodes: &'a [Node],
    pub old_edges: &'a [Edge],
    pub old_view: Option<TopologyView>,
    /// View this graph is being built for — selects the builder, grouping, and
    /// per-edge view config.
    pub view: TopologyView,
}

/// Whether any interface in this set qualifies its site for the L2 Physical view.
///
/// Two conditions, matching `l2_builder.rs`'s `qualifying_host_ids` exactly (kept in sync
/// deliberately — this answers "should the tab even be offered", that answers "which hosts does
/// it draw", and a site can only offer the tab honestly if at least one host would qualify):
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
        site_service: Arc<SiteService>,
        organization_service: Arc<OrganizationService>,
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
            site_service,
            organization_service,
            event_bus,
            live_update_tx,
        }
    }

    /// The use case `organization_id` chose at signup.
    pub async fn organization_use_case(&self, organization_id: Uuid) -> Result<UseCase, Error> {
        self.organization_service
            .get_by_id(&organization_id)
            .await?
            .map(|org| org.base.use_case)
            .ok_or_else(|| anyhow::anyhow!("Organization {organization_id} not found"))
    }

    /// The request options a new topology in `organization_id` starts with.
    pub async fn default_request_options(
        &self,
        organization_id: Uuid,
    ) -> Result<TopologyRequestOptions, Error> {
        Ok(TopologyRequestOptions::for_use_case(
            self.organization_use_case(organization_id).await?,
        ))
    }

    /// Subscribe to live-topology update pings. Each emitted `Uuid` is a
    /// site whose live entity set just changed; consumers refetch the
    /// live topology row + entity data.
    pub fn subscribe_live_topology_updates(&self) -> broadcast::Receiver<Uuid> {
        self.live_update_tx.subscribe()
    }

    /// Announce a live-topology change, as the event subscriber does.
    #[cfg(test)]
    pub fn broadcast_live_update(&self, site_id: Uuid) {
        let _ = self.live_update_tx.send(site_id);
    }

    /// Unified entity-set loader for live + snapshot topology reads.
    ///
    /// `snapshot_id = None` reads live entity rows (`valid_to IS NULL`).
    /// `snapshot_id = Some(id)` reads closed copies stamped at that snapshot
    /// (`snapshot_id = id`) — those have distinct ids from their live
    /// counterparts and survive live-row deletion.
    pub async fn get_topology_data(
        &self,
        site_id: Uuid,
        snapshot_id: Option<Uuid>,
    ) -> Result<(TopologyData, TagHidden), Error> {
        // Hosts/services/subnets carry the tags that drive topology grouping +
        // display, so hydrate their tag associations as-of the snapshot (from
        // the closed `entity_tags`), not from live `entity_tags` (which key on
        // live, not closed, entity ids). Other entity types don't surface tags
        // in the topology, so plain `get_all` (live hydration) is fine for them.
        // The organization's Visibility tags. A host, service or subnet holding one (as of the
        // snapshot, for a snapshot read) is left out; a hidden host takes what it holds with it.
        let hiding = match self.site_service.get_by_id(&site_id).await? {
            Some(site) => {
                self.tag_service
                    .hiding_tag_ids(&site.base.organization_id)
                    .await?
            }
            None => vec![],
        };
        let is_hidden = |tags: &[Uuid]| tags.iter().any(|tag| hiding.contains(tag));
        let mut hosts = self
            .host_service
            .get_all_as_of_snapshot(
                apply_snapshot(
                    StorableFilter::<Host>::new_from_site_ids(&[site_id]),
                    snapshot_id,
                ),
                snapshot_id,
            )
            .await?;
        // A dismissed host is one a person said is not worth showing; it leaves the map the way a
        // hidden one does.
        // What a Visibility tag left out is set aside, so each view can count the rows it would
        // have drawn and the filter panel offers that filter only where it removed something.
        hosts.retain(|host| host.dismissed_at.is_none());
        let (hidden_hosts, hosts): (Vec<Host>, Vec<Host>) = hosts
            .into_iter()
            .partition(|host| is_hidden(&host.base.tags));
        let ip_addresses = self
            .ip_address_service
            .get_all(apply_snapshot(
                StorableFilter::<IPAddress>::new_from_site_ids(&[site_id]),
                snapshot_id,
            ))
            .await?;
        let subnets = self
            .subnet_service
            .get_all_as_of_snapshot(
                apply_snapshot(
                    StorableFilter::<Subnet>::new_from_site_ids(&[site_id]),
                    snapshot_id,
                ),
                snapshot_id,
            )
            .await?;
        let (hidden_subnets, subnets): (Vec<Subnet>, Vec<Subnet>) = subnets
            .into_iter()
            .partition(|subnet| is_hidden(&subnet.base.tags));
        let dependencies = self
            .dependency_service
            .get_all(apply_snapshot(
                StorableFilter::<Dependency>::new_from_site_ids(&[site_id]),
                snapshot_id,
            ))
            .await?;
        let ports = self
            .port_service
            .get_all(apply_snapshot(
                StorableFilter::<Port>::new_from_site_ids(&[site_id]),
                snapshot_id,
            ))
            .await?;
        let bindings = self
            .binding_service
            .get_all(apply_snapshot(
                StorableFilter::<Binding>::new_from_site_ids(&[site_id]),
                snapshot_id,
            ))
            .await?;
        let interfaces = self
            .interface_service
            .get_all(apply_snapshot(
                StorableFilter::<Interface>::new_from_site_ids(&[site_id]),
                snapshot_id,
            ))
            .await?;
        // GH #701: the merged neighbour read-model, pinned to the same live/snapshot state as
        // `interfaces` above. `candidates` are never Snapshotable (disposable, replaced wholesale
        // every scan — see `interface_neighbors`), so they're always read live regardless of
        // `snapshot_id`; a historical snapshot's L2 view renders from `neighbours` alone.
        let neighbours = self
            .interface_neighbor_service
            .resolved_for_site(site_id, snapshot_id)
            .await?;
        let candidates = self
            .interface_neighbor_service
            .candidates_for_site(site_id)
            .await?;
        let services = self
            .service_service
            .get_all_as_of_snapshot(
                apply_snapshot(
                    StorableFilter::<Service>::new_from_site_ids(&[site_id]),
                    snapshot_id,
                ),
                snapshot_id,
            )
            .await?;
        let (hidden_services, services): (Vec<Service>, Vec<Service>) = services
            .into_iter()
            .partition(|service| is_hidden(&service.base.tags));
        let mut tag_hidden = TagHidden {
            hosts: hidden_hosts,
            subnets: hidden_subnets,
            services: hidden_services,
            orphans: OrphanRows::default(),
        };
        let vlans = self
            .vlan_service
            .get_all(apply_snapshot(
                StorableFilter::<Vlan>::new_from_uuid_column("site_id", &site_id),
                snapshot_id,
            ))
            .await?;
        let tags = self.get_entity_tags(&hosts, &services, &subnets).await?;
        let application_group_id = self.application_group_id(site_id).await?;

        // Which views have data in THIS entity set (snapshot-aware). Reuses the
        // same `TopologyViewSupport` / `is_supported` logic that governs shares,
        // but computed from the entities just loaded (so a snapshot reflects its
        // captured data, not live). The topology tab uses this to hide views a
        // snapshot can't populate (no LLDP neighbors → no L2; no app tags → no
        // Application).
        let support = TopologyViewSupport {
            // Any resolved neighbour, port-precise or device-level. A site whose links have
            // all degraded to `Neighbor::Host` still has an L2 topology to show — dashed
            // `NeighborLink` edges between host containers — and hiding the view is the one
            // outcome that leaves the operator nothing to look at. Failing that, a no-IP host
            // identified only by MAC evidence still qualifies.
            l2_physical: any_interface_qualifies_l2_physical(
                &interfaces,
                &ip_addresses,
                !neighbours.is_empty(),
            ),
            application: tags.iter().any(|t| t.in_group(application_group_id)),
        };
        let available_views: Vec<TopologyView> = TopologyView::iter()
            .filter(|v| v.is_supported(&support))
            .collect();

        // Title the hosts once, here, from the addresses this same bundle carries — so the map,
        // the host list and every by-id lookup on the frontend read one value rather than each
        // re-deriving the ladder. The device type comes from the same bundle's services. Last,
        // because `get_entity_tags` above wants plain `&[Host]`.
        let hosts = TopologyHost::wrap_all(hosts, &ip_addresses, &services);
        // Interfaces likewise, as `HostResponse` titles them: the inspector cards label an
        // interface with `display_name`, and without it every one reads "Unnamed interface".
        // Filter values are stamped later, by `get_topology_render_data`, once the server-side
        // filter has run against the same context.
        let interfaces: Vec<TopologyInterface> = interfaces
            .into_iter()
            .map(|mut interface| {
                interface.display_name = Some(interface.display_name());
                interface.into()
            })
            .collect();
        let services: Vec<TopologyServiceRow> = services.into_iter().map(Into::into).collect();

        let mut data = TopologyData {
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
            application_group_id,
            available_views,
            // Built on request by `get_topology_render_data`; the bare entity
            // loader leaves them empty.
            nodes: HashMap::new(),
            edges: HashMap::new(),
            node_entities: HashMap::new(),
            // Counted per view by `get_topology_render_data`, from `TagHidden`.
            hidden_by_tag: HashMap::new(),
            // Filled by `apply_server_metadata_filters`, which runs on the way to the render
            // path; nothing has been dropped yet at this point.
            filtered_out: HashMap::new(),
            // Set by the topology data endpoint, which knows the caller's organization.
            has_interface_credential: false,
        };
        // Dismissed and hidden rows take their interfaces, addresses, services and neighbour rows
        // with them, so no builder meets a row whose parent is gone (GH #852).
        tag_hidden.orphans = data.remove_orphans();
        Ok((data, tag_hidden))
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

    /// Compute per-view data-support flags for a site's topology by
    /// querying raw entity tables — independent of whatever the topology
    /// was last rebuilt under.
    /// The Application tag group of the organization that owns `site_id`.
    async fn application_group_id(&self, site_id: Uuid) -> Result<Option<Uuid>, Error> {
        match self.site_service.get_by_id(&site_id).await? {
            Some(site) => {
                self.tag_service
                    .application_group_id(&site.base.organization_id)
                    .await
            }
            None => Ok(None),
        }
    }

    pub async fn get_view_support(&self, site_id: Uuid) -> Result<TopologyViewSupport, Error> {
        // Device-level neighbours count too — see the equivalent in `get_topology_data`. Live
        // only: this check has no snapshot context, and the live view is what it governs.
        let has_resolved_neighbours = !self
            .interface_neighbor_service
            .resolved_for_site(site_id, None)
            .await?
            .is_empty();
        let interfaces = self
            .interface_service
            .get_all(StorableFilter::<Interface>::new_from_site_ids(&[site_id]))
            .await?;
        let ip_addresses = self
            .ip_address_service
            .get_all(StorableFilter::<IPAddress>::new_from_site_ids(&[site_id]))
            .await?;
        let l2_physical = any_interface_qualifies_l2_physical(
            &interfaces,
            &ip_addresses,
            has_resolved_neighbours,
        );

        let application = match self.site_service.get_by_id(&site_id).await? {
            Some(site) => {
                let application_group_id = self
                    .tag_service
                    .application_group_id(&site.base.organization_id)
                    .await?;
                self.tag_service
                    .get_all(
                        StorableFilter::<Tag>::new_from_org_id(&site.base.organization_id).live(),
                    )
                    .await?
                    .iter()
                    .any(|t| t.in_group(application_group_id))
            }
            None => false,
        };

        Ok(TopologyViewSupport {
            l2_physical,
            application,
        })
    }

    /// Load the entity set for `(site_id, snapshot_id)` and build the graph of each of `views`
    /// on request from it + the site's grouping options.
    /// Single source for the render, export, and share paths now that the graph
    /// is no longer persisted. Snapshots use the site's (live) options —
    /// the same behaviour the former `build_snapshot_topology` had.
    ///
    /// Callers that draw or export one view pass only that view: each build costs a full pass
    /// over the site's entities, and on a large site the graphs of the views nobody looks at
    /// are most of the response.
    pub async fn get_topology_render_data(
        &self,
        site_id: Uuid,
        snapshot_id: Option<Uuid>,
        views: &[TopologyView],
    ) -> Result<TopologyData, Error> {
        let started = Instant::now();
        let options = self.site_topology_options(site_id).await?;
        let (mut data, tag_hidden) = self.get_topology_data(site_id, snapshot_id).await?;
        let loaded = started.elapsed();
        // `data.tags` only carries tags applied to entities. A grouping rule can
        // reference a tag applied to nothing (e.g. ByTag on an unused tag); the
        // frontend needs its name/color to label the group, so ship it too.
        self.augment_grouping_rule_tags(&mut data, &options).await?;
        let data = Self::render_loaded(data, tag_hidden, &options, views);
        tracing::debug!(
            %site_id,
            ?views,
            hosts = data.hosts.len(),
            load_ms = loaded.as_millis(),
            build_ms = (started.elapsed() - loaded).as_millis(),
            "Built topology render data"
        );
        Ok(data)
    }

    /// Filter a loaded bundle and build each of `views` from it: everything the render path does
    /// once the rows are read.
    pub(crate) fn render_loaded(
        mut data: TopologyData,
        tag_hidden: TagHidden,
        options: &TopologyOptions,
        views: &[TopologyView],
    ) -> TopologyData {
        let started = Instant::now();
        let tag_hidden_bundle = (!tag_hidden.is_empty()).then(|| {
            let mut bundle = Self::with_tag_hidden(&data, tag_hidden);
            let ctx = FilterValueContext::for_bundle(
                &bundle.data.neighbours,
                bundle.data.interfaces.iter().map(|i| &i.interface),
                &bundle.data.ip_addresses,
            );
            Self::apply_server_metadata_filters(&mut bundle.data, &options, &ctx);
            bundle
        });
        // One context for both the server-side drop and the values the frontend reads, so a
        // value offered as a chip is the value the server filters by.
        let filter_ctx = FilterValueContext::for_bundle(
            &data.neighbours,
            data.interfaces.iter().map(|i| &i.interface),
            &data.ip_addresses,
        );
        Self::apply_server_metadata_filters(&mut data, &options, &filter_ctx);
        data.stamp_filter_values(&filter_ctx);
        if let Some(bundle) = tag_hidden_bundle {
            data.hidden_by_tag = Self::hidden_by_tag_per_view(&bundle, options, views);
        }
        let filtered = started.elapsed();
        let (nodes, edges) = Self::build_view_graphs(&data, options, views);
        let built = started.elapsed();
        data.node_entities = nodes
            .iter()
            .map(|(view, view_nodes)| {
                (
                    *view,
                    super::node_entities::node_entities(view_nodes, &data),
                )
            })
            .collect();
        data.nodes = nodes;
        data.edges = edges;
        tracing::debug!(
            ?views,
            filter_ms = filtered.as_millis(),
            build_ms = (built - filtered).as_millis(),
            node_entities_ms = (started.elapsed() - built).as_millis(),
            "Rendered topology bundle"
        );
        data
    }

    /// `data` with the Visibility-hidden rows put back, and which type each of those rows is.
    fn with_tag_hidden(data: &TopologyData, hidden: TagHidden) -> TagHiddenBundle {
        let mut hidden_types: HashMap<Uuid, EntityDiscriminants> = HashMap::new();
        hidden_types.extend(
            hidden
                .hosts
                .iter()
                .map(|h| (h.id, EntityDiscriminants::Host)),
        );
        hidden_types.extend(
            hidden
                .subnets
                .iter()
                .map(|s| (s.id, EntityDiscriminants::Subnet)),
        );
        hidden_types.extend(
            hidden
                .services
                .iter()
                .map(|s| (s.id, EntityDiscriminants::Service)),
        );

        let mut with_hidden = data.clone();
        with_hidden.subnets.extend(hidden.subnets);
        with_hidden
            .services
            .extend(hidden.services.into_iter().map(Into::into));
        hidden.orphans.restore_into(&mut with_hidden);
        let services: Vec<Service> = with_hidden
            .services
            .iter()
            .map(|s| s.service.clone())
            .collect();
        with_hidden.hosts.extend(TopologyHost::wrap_all(
            hidden.hosts,
            &with_hidden.ip_addresses,
            &services,
        ));
        // A dismissed host's rows came back with the hidden ones; their host didn't.
        with_hidden.remove_orphans();
        TagHiddenBundle {
            data: with_hidden,
            hidden_types,
        }
    }

    /// Per view, how many Visibility-hidden rows of each type it would have drawn. A view that
    /// would draw none of them has no entry.
    fn hidden_by_tag_per_view(
        bundle: &TagHiddenBundle,
        options: &TopologyOptions,
        views: &[TopologyView],
    ) -> HashMap<TopologyView, HashMap<EntityDiscriminants, usize>> {
        let (nodes, _) = Self::build_view_graphs(&bundle.data, options, views);
        nodes
            .into_iter()
            .filter_map(|(view, view_nodes)| {
                let counts = drawn_hidden_counts(&view_nodes, &bundle.hidden_types);
                (!counts.is_empty()).then_some((view, counts))
            })
            .collect()
    }

    /// Drop entities the user has hidden through a `Server`-side metadata filter.
    ///
    /// Runs before any view is built, so the builders never see them and nodes, edges and the
    /// entity bundle all shrink together. The bundle is what makes this worth doing: a `Node` is an
    /// id and a position, while the entity records are the payload — L2 ships 19,095 interfaces to
    /// render 2,872, and the difference is what exhausts a customer's browser.
    ///
    /// Silent when nothing is hidden server-side, which is the common case.
    fn apply_server_metadata_filters(
        data: &mut TopologyData,
        options: &TopologyOptions,
        ctx: &FilterValueContext,
    ) {
        let hide_sets = metadata_filter::server_hide_sets(options);
        if hide_sets.is_empty() {
            return;
        }

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
                ctx,
            ),
        );
        record(
            EntityDiscriminants::Host,
            metadata_filter::retain_visible(
                &mut data.hosts,
                hide_sets.get(&EntityDiscriminants::Host),
                ctx,
            ),
        );
        record(
            EntityDiscriminants::Service,
            metadata_filter::retain_visible(
                &mut data.services,
                hide_sets.get(&EntityDiscriminants::Service),
                ctx,
            ),
        );

        // What a filter dropped takes its children with it. `filtered_out` counts only the rows a
        // filter matched.
        data.remove_orphans();

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

    /// The site's single live topology row holds the user's grouping
    /// `options`. Defaults if the row is somehow absent.
    pub async fn site_topology_options(&self, site_id: Uuid) -> Result<TopologyOptions, Error> {
        let rows = self
            .get_all(StorableFilter::<Topology>::new_from_site_ids(&[site_id]))
            .await?;
        Ok(rows
            .into_iter()
            .next()
            .map(|t| t.base.options)
            .unwrap_or_default())
    }

    /// Build a node/edge set for each of `views` from a single entity snapshot.
    ///
    /// Pure function of `data` + `options` — called on request by the read,
    /// export, and share paths (the graph is no longer persisted). Returns one
    /// node/edge slice per requested view.
    pub fn build_view_graphs(
        data: &TopologyData,
        options: &TopologyOptions,
        views: &[TopologyView],
    ) -> (
        HashMap<TopologyView, Vec<Node>>,
        HashMap<TopologyView, Vec<Edge>>,
    ) {
        let mut nodes_by_view = HashMap::new();
        let mut edges_by_view = HashMap::new();

        // The builders take plain domain rows; the bundle carries hosts, services and interfaces
        // wrapped with values computed at load. Unwrap once here rather than threading the
        // wrappers through `TopologyContext` and every builder — and once for all views, not once
        // each.
        let hosts: Vec<Host> = data.hosts.iter().map(|h| h.host.clone()).collect();
        let services: Vec<Service> = data.services.iter().map(|s| s.service.clone()).collect();
        let interfaces: Vec<Interface> = data
            .interfaces
            .iter()
            .map(|i| i.interface.clone())
            .collect();

        for &view in views {
            let (nodes, edges) = Self::build_graph(BuildGraphParams {
                hosts: &hosts,
                ip_addresses: &data.ip_addresses,
                services: &services,
                subnets: &data.subnets,
                dependencies: &data.dependencies,
                ports: &data.ports,
                bindings: &data.bindings,
                interfaces: &interfaces,
                neighbours: &data.neighbours,
                candidates: &data.candidates,
                entity_tags: &data.tags,
                application_group_id: data.application_group_id,
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

    pub fn build_graph(params: BuildGraphParams) -> (Vec<Node>, Vec<Edge>) {
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
            application_group_id,
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
        .with_candidates(candidates)
        .with_application_group(application_group_id);

        // Build grouping config from request options
        let grouping = GroupingConfig::from_request_options(&options.request, view);

        // Select builder by view and build nodes + edges
        let builder = super::view::builder_for_view(view);
        let (mut all_nodes, mut all_edges) = builder.build(&ctx, &grouping);
        // Every view passes here, so a builder that names a container it never emitted costs the
        // map those elements and logs an error, instead of failing the client's layout whole.
        let dropped = super::graph_integrity::drop_dangling_nodes(&mut all_nodes);
        if !dropped.is_empty() {
            tracing::error!(
                ?view,
                count = dropped.len(),
                node_ids = ?dropped.iter().take(20).collect::<Vec<_>>(),
                "Topology builder emitted nodes inside containers it never emitted; dropped them"
            );
        }
        super::child_order::order_children(&mut all_nodes, grouping.element_sort, &ctx);

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
        // `old_edges` / `old_view` are fed empty by `build_view_graphs`.
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

    /// The case this function exists for: a site whose only L2-relevant data is a PROFINET
    /// DCP-identified host (no IP, no neighbour) must still offer the L2 Physical tab —
    /// otherwise the view `l2_builder.rs` would draw a container in is never reachable.
    #[test]
    fn a_no_ip_mac_only_host_qualifies_the_site_for_l2() {
        let host_id = Uuid::new_v4();
        let interfaces = vec![interface(host_id, Some("aa:bb:cc:dd:ee:ff"))];
        assert!(any_interface_qualifies_l2_physical(&interfaces, &[], false));
    }

    /// The condition is "no IP *at all*", not "this interface has no IP" — a host with an IP
    /// recorded elsewhere (its own `IPAddress` row) doesn't qualify the site on its MAC
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
    /// (GH #701): a resolved neighbour qualifies the site on its own, whatever the interfaces
    /// carry. A host with an IP and no MAC fails every other condition.
    #[test]
    fn a_resolved_neighbour_qualifies_the_site_on_its_own() {
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

/// How many of `hidden` (id → type) a view's `nodes` draw: as a node's own entity, inline on a
/// card, or as the host of a drawn element (a hidden host's addresses in L3).
fn drawn_hidden_counts(
    nodes: &[Node],
    hidden: &HashMap<Uuid, EntityDiscriminants>,
) -> HashMap<EntityDiscriminants, usize> {
    let mut drawn: HashSet<Uuid> = HashSet::new();
    for node in nodes {
        match &node.node_type {
            NodeType::Element {
                host_id,
                entity_id,
                inline_groups,
                ..
            } => {
                drawn.insert(*host_id);
                drawn.extend(*entity_id);
                drawn.extend(inline_groups.iter().map(|g| g.entity_id));
            }
            NodeType::Container { entity_id, .. } => drawn.extend(*entity_id),
        }
    }
    let mut counts: HashMap<EntityDiscriminants, usize> = HashMap::new();
    for id in drawn {
        if let Some(entity_type) = hidden.get(&id) {
            *counts.entry(*entity_type).or_default() += 1;
        }
    }
    counts
}

#[cfg(test)]
mod hidden_by_tag_tests {
    use super::*;
    use crate::server::topology::types::nodes::ElementEntityType;

    /// A hidden host whose address L3 draws counts there; a view that draws nothing of it, as
    /// Workloads draws nothing of a host with no services, has no count.
    #[test]
    fn a_hidden_host_counts_only_where_its_elements_are_drawn() {
        let host = Uuid::new_v4();
        let address = Uuid::new_v4();
        let hidden = HashMap::from([(host, EntityDiscriminants::Host)]);
        let l3 = [Node::element(
            Uuid::new_v4(),
            Uuid::nil(),
            host,
            ElementEntityType::IPAddress {
                subnet_id: Uuid::new_v4(),
                ip_address_id: Some(address),
            },
        )];
        let other_host_service = [Node::element(
            Uuid::new_v4(),
            Uuid::nil(),
            Uuid::new_v4(),
            ElementEntityType::Service {},
        )];

        assert_eq!(
            drawn_hidden_counts(&l3, &hidden),
            HashMap::from([(EntityDiscriminants::Host, 1)])
        );
        assert!(drawn_hidden_counts(&other_host_service, &hidden).is_empty());
    }
}
