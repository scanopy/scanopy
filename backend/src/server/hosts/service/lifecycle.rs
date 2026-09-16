//! Constructor, query helpers, response builders, and child loading.
use super::*;

impl HostService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<GenericPostgresStorage<Host>>,
        ip_address_service: Arc<IPAddressService>,
        port_service: Arc<PortService>,
        service_service: Arc<ServiceService>,
        interface_service: Arc<InterfaceService>,
        interface_neighbor_service: Arc<InterfaceNeighborService>,
        daemon_service: Arc<DaemonService>,
        credential_service: Arc<CredentialService>,
        subnet_service: Arc<SubnetService>,
        vlan_service: Arc<VlanService>,
        network_service: Arc<NetworkService>,
        organization_service: Arc<OrganizationService>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            ip_address_service,
            port_service,
            service_service,
            interface_service,
            interface_neighbor_service,
            daemon_service,
            credential_service,
            subnet_service,
            vlan_service,
            network_service,
            organization_service,
            event_bus,
            entity_tag_service,
        }
    }

    /// Get ports for a specific host
    pub async fn get_ports_for_host(&self, host_id: &Uuid) -> Result<Vec<Port>> {
        self.port_service.get_for_host(host_id).await
    }

    /// Get ip_addresses for a specific host
    pub async fn get_ip_addresses_for_host(&self, host_id: &Uuid) -> Result<Vec<IPAddress>> {
        self.ip_address_service.get_for_host(host_id).await
    }

    // =========================================================================
    // HostResponse builders (load children for API responses)
    // =========================================================================

    /// Get a single host with all children hydrated for API response
    pub async fn get_host_response(&self, id: &Uuid) -> Result<Option<HostResponse>> {
        let mut host = match self.get_by_id(id).await? {
            Some(h) => h,
            None => return Ok(None),
        };

        // Hydrate tags from junction table
        let tags = self
            .entity_tag_service
            .get_tags(id, &EntityDiscriminants::Host)
            .await?;
        host.base.tags = tags;

        let (ip_addresses, ports, services, interfaces) =
            self.load_children_for_host(&host.id).await?;
        Ok(Some(HostResponse::from_host_with_children(
            host,
            ip_addresses,
            ports,
            services,
            interfaces,
        )))
    }

    /// Get all hosts with all children hydrated for API response
    pub async fn get_all_host_responses(
        &self,
        filter: StorableFilter<Host>,
    ) -> Result<Vec<HostResponse>> {
        let hosts = self.get_all(filter).await?;
        if hosts.is_empty() {
            return Ok(vec![]);
        }

        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let (ip_addresses_map, ports_map, services_map, interfaces_map) =
            self.load_children_for_hosts(&host_ids, None).await?;

        // Hydrate tags from junction table
        let tags_map = self
            .entity_tag_service
            .get_tags_map(&host_ids, EntityDiscriminants::Host, None)
            .await?;

        let responses = hosts
            .into_iter()
            .map(|mut host| {
                // Apply hydrated tags
                if let Some(tags) = tags_map.get(&host.id) {
                    host.base.tags = tags.clone();
                }
                let ip_addresses = ip_addresses_map.get(&host.id).cloned().unwrap_or_default();
                let ports = ports_map.get(&host.id).cloned().unwrap_or_default();
                let services = services_map.get(&host.id).cloned().unwrap_or_default();
                let interfaces = interfaces_map.get(&host.id).cloned().unwrap_or_default();
                HostResponse::from_host_with_children(
                    host,
                    ip_addresses,
                    ports,
                    services,
                    interfaces,
                )
            })
            .collect();

        Ok(responses)
    }

    /// Get paginated hosts for API response, with children hydrated unless
    /// `include_children` is false.
    ///
    /// The child collections dominate the payload — a host list carrying them is
    /// an order of magnitude larger than one without — so callers that only need
    /// host identity (name pickers, id→name lookups, counts) pass `false` and
    /// skip both the bytes and the four child queries. Tags are hydrated either
    /// way: they live on the host row's own junction table, callers filter and
    /// label by them, and they cost one query for the whole page.
    ///
    /// Supports custom ordering via the `order_by` parameter.
    pub async fn get_all_host_responses_paginated(
        &self,
        filter: StorableFilter<Host>,
        order_by: &str,
        at: Option<DateTime<Utc>>,
        include_children: bool,
    ) -> Result<PaginatedResult<HostResponse>> {
        let result = self.storage().get_paginated(filter, order_by).await?;

        if result.items.is_empty() {
            return Ok(PaginatedResult {
                items: vec![],
                total_count: result.total_count,
            });
        }

        let host_ids: Vec<Uuid> = result.items.iter().map(|h| h.id).collect();
        // Hydrate children as-of the same instant as the host rows so a snapshot
        // view shows a coherent point-in-time host + children bundle.
        let (ip_addresses_map, ports_map, services_map, interfaces_map) = if include_children {
            self.load_children_for_hosts(&host_ids, at).await?
        } else {
            Default::default()
        };

        // Hydrate tags from junction table
        let tags_map = self
            .entity_tag_service
            .get_tags_map(&host_ids, EntityDiscriminants::Host, None)
            .await?;

        let responses = result
            .items
            .into_iter()
            .map(|mut host| {
                // Apply hydrated tags
                if let Some(tags) = tags_map.get(&host.id) {
                    host.base.tags = tags.clone();
                }
                let ip_addresses = ip_addresses_map.get(&host.id).cloned().unwrap_or_default();
                let ports = ports_map.get(&host.id).cloned().unwrap_or_default();
                let services = services_map.get(&host.id).cloned().unwrap_or_default();
                let interfaces = interfaces_map.get(&host.id).cloned().unwrap_or_default();
                HostResponse::from_host_with_children(
                    host,
                    ip_addresses,
                    ports,
                    services,
                    interfaces,
                )
            })
            .collect();

        Ok(PaginatedResult {
            items: responses,
            total_count: result.total_count,
        })
    }

    /// Load all children for a single host.
    pub(crate) async fn load_children_for_host(
        &self,
        host_id: &Uuid,
    ) -> Result<(Vec<IPAddress>, Vec<Port>, Vec<Service>, Vec<Interface>)> {
        let ip_addresses = self.ip_address_service.get_for_host(host_id).await?;
        let ports = self.port_service.get_for_host(host_id).await?;
        // SCD2: live services only.
        let services = self
            .service_service
            .get_all_ordered(
                StorableFilter::<Service>::new_from_host_ids(&[*host_id]).live(),
                "position ASC",
            )
            .await?;
        let interfaces = self.interface_service.get_for_host(host_id).await?;

        Ok((ip_addresses, ports, services, interfaces))
    }

    /// Batch load all children for multiple hosts.
    /// `at = None` loads live children; `Some(t)` loads SCD2 state as of `t`.
    async fn load_children_for_hosts(
        &self,
        host_ids: &[Uuid],
        at: Option<DateTime<Utc>>,
    ) -> Result<(
        HashMap<Uuid, Vec<IPAddress>>,
        HashMap<Uuid, Vec<Port>>,
        HashMap<Uuid, Vec<Service>>,
        HashMap<Uuid, Vec<Interface>>,
    )> {
        let ip_addresses_map = self.ip_address_service.get_for_hosts(host_ids, at).await?;
        let ports_map = self.port_service.get_for_hosts(host_ids, at).await?;

        // Load services ordered by position and group by host_id.
        let services = self
            .service_service
            .get_all_ordered(
                StorableFilter::<Service>::new_from_host_ids(host_ids).live_or_as_of(at),
                "position ASC",
            )
            .await?;

        let mut services_map: HashMap<Uuid, Vec<Service>> = HashMap::new();
        for service in services {
            services_map
                .entry(service.base.host_id)
                .or_default()
                .push(service);
        }

        // Load interfaces and group by host_id
        let mut interfaces_map = self.interface_service.get_for_hosts(host_ids, at).await?;
        // Sort each host's entries by if_index
        for entries in interfaces_map.values_mut() {
            entries.sort_by_key(|e| e.base.if_index);
        }

        Ok((ip_addresses_map, ports_map, services_map, interfaces_map))
    }
}
