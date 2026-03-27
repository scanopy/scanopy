use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::r#impl::base::{Binding, BindingType},
    credentials::service::CredentialService,
    daemons::{r#impl::base::Daemon, service::DaemonService},
    hosts::r#impl::{
        api::{
            BindingInput, ConflictBehavior, CreateHostRequest, HostResponse, InterfaceInput,
            PortInput, ServiceInput, UpdateHostRequest,
        },
        base::{Host, HostBase},
    },
    if_entries::{r#impl::base::IfEntry, service::IfEntryService},
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    ports::{r#impl::base::Port, service::PortService},
    services::{
        r#impl::{base::Service, definitions::ServiceDefinitionExt},
        service::ServiceService,
    },
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        events::{
            bus::EventBus,
            types::{EntityEvent, EntityOperation},
        },
        position::resolve_and_validate_input_positions,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, PaginatedResult, Storable, Storage},
        },
        types::{
            api::ValidationError,
            entities::{EntitySource, EntitySourceDiscriminants},
        },
    },
    snmp::resolution::{lldp::LldpResolver, resolver::LldpResolverImpl},
    tags::entity_tags::EntityTagService,
};
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use chrono::Utc;
use std::{collections::HashMap, net::IpAddr, sync::Arc};
use strum::IntoDiscriminant;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct HostLimitContext {
    pub limit: u64,
    pub org_id: Uuid,
    pub org_network_ids: Vec<Uuid>,
}

pub struct HostService {
    storage: Arc<GenericPostgresStorage<Host>>,
    interface_service: Arc<InterfaceService>,
    port_service: Arc<PortService>,
    service_service: Arc<ServiceService>,
    if_entry_service: Arc<IfEntryService>,
    pub daemon_service: Arc<DaemonService>,
    credential_service: Arc<CredentialService>,
    host_locks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<Host> for HostService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Host) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Host) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Host> for HostService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Host>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    /// Create a new host, or upsert if a matching host exists.
    ///
    /// This method uses `Host::eq` (ID comparison) to find existing hosts.
    /// For discovery workflows, `create_with_children` sets the incoming host's ID
    /// to match an existing host found via interface comparison, so this method
    /// will find the match and trigger `upsert_host()`.
    ///
    /// Upsert conditions:
    /// - Both hosts are from discovery (merges discovery metadata)
    /// - OR the IDs already match (handles re-discovery of known hosts)
    async fn create(&self, host: Host, authentication: AuthenticatedEntity) -> Result<Host> {
        let host = if host.id == Uuid::nil() {
            Host::new(host.base.clone())
        } else {
            host
        };

        let lock = self.get_host_lock(&host.id).await;
        let _guard = lock.lock().await;

        tracing::trace!("Creating host {:?}", host);

        let filter = StorableFilter::<Host>::new_from_network_ids(&[host.base.network_id]);
        let all_hosts = self.get_all(filter).await?;

        // Find existing host by ID (Host::eq only compares IDs)
        // For discovery, create_with_children already set host.id to the existing host's ID
        // if an interface match was found, so this will find the match
        let host_from_storage = match all_hosts.into_iter().find(|h| host.eq(h)) {
            // Upsert if both are discovery sources, or if IDs match exactly
            Some(existing_host)
                if (host.base.source.discriminant() == EntitySourceDiscriminants::Discovery
                    && existing_host.base.source.discriminant()
                        == EntitySourceDiscriminants::Discovery)
                    || host.id == existing_host.id =>
            {
                if host.id != existing_host.id {
                    tracing::warn!(
                        incoming_host_id = %host.id,
                        matched_host_id = %existing_host.id,
                        matched_host_name = %existing_host.base.name,
                        "Host matched via MAC/IP address but discovery reported a different host ID. \
                         This may indicate a daemon is using a stale configuration. \
                         To fix, update the daemon's config file with: host_id = \"{}\"",
                        existing_host.id
                    );
                }

                tracing::debug!(
                    "Duplicate host for {}: {} found, {}: {} - upserting discovery data...",
                    host.base.name,
                    host.id,
                    existing_host.base.name,
                    existing_host.id
                );

                self.upsert_host(existing_host, host, authentication)
                    .await?
            }
            _ => {
                if let Some(existing_host) = self.get_by_id(&host.id).await? {
                    return Err(ValidationError::new(format!(
                        "Network mismatch: Daemon is trying to update host '{}' (id: {}) but cannot proceed. \
                        The host belongs to network {} while the daemon is assigned to network {}. \
                        To resolve this, either reassign the daemon to the correct network or delete the mismatched host.",
                        existing_host.base.name,
                        host.id,
                        existing_host.base.network_id,
                        host.base.network_id
                    )).into());
                }

                let created = self.storage().create(&host).await?;
                let trigger_stale = created.triggers_staleness(None);

                self.event_bus()
                    .publish_entity(EntityEvent {
                        id: Uuid::new_v4(),
                        entity_id: created.id(),
                        network_id: self.get_network_id(&created),
                        organization_id: self.get_organization_id(&created),
                        entity_type: created.into(),
                        operation: EntityOperation::Created,
                        timestamp: Utc::now(),
                        metadata: serde_json::json!({
                            "trigger_stale": trigger_stale
                        }),

                        authentication,
                    })
                    .await?;

                host
            }
        };

        Ok(host_from_storage)
    }

    async fn update(
        &self,
        updates: &mut Host,
        authentication: AuthenticatedEntity,
    ) -> Result<Host, Error> {
        let lock = self.get_host_lock(&updates.id).await;
        let _guard = lock.lock().await;

        let current_host = self
            .get_by_id(&updates.id)
            .await?
            .ok_or_else(|| anyhow!("Host '{}' not found", updates.id))?;

        let updated = self.storage().update(updates).await?;
        let trigger_stale = updated.triggers_staleness(Some(current_host));

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: updated.id(),
                network_id: self.get_network_id(&updated),
                organization_id: self.get_organization_id(&updated),
                entity_type: updated.clone().into(),
                operation: EntityOperation::Updated,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale
                }),

                authentication,
            })
            .await?;

        Ok(updated)
    }
}

impl HostService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<GenericPostgresStorage<Host>>,
        interface_service: Arc<InterfaceService>,
        port_service: Arc<PortService>,
        service_service: Arc<ServiceService>,
        if_entry_service: Arc<IfEntryService>,
        daemon_service: Arc<DaemonService>,
        credential_service: Arc<CredentialService>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            interface_service,
            port_service,
            service_service,
            if_entry_service,
            daemon_service,
            credential_service,
            host_locks: Arc::new(Mutex::new(HashMap::new())),
            event_bus,
            entity_tag_service,
        }
    }

    /// Get ports for a specific host
    pub async fn get_ports_for_host(&self, host_id: &Uuid) -> Result<Vec<Port>> {
        self.port_service.get_for_host(host_id).await
    }

    /// Get interfaces for a specific host
    pub async fn get_interfaces_for_host(&self, host_id: &Uuid) -> Result<Vec<Interface>> {
        self.interface_service.get_for_host(host_id).await
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

        let (interfaces, ports, services, if_entries) =
            self.load_children_for_host(&host.id).await?;
        Ok(Some(HostResponse::from_host_with_children(
            host, interfaces, ports, services, if_entries,
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
        let (interfaces_map, ports_map, services_map, if_entries_map) =
            self.load_children_for_hosts(&host_ids).await?;

        // Hydrate tags from junction table
        let tags_map = self
            .entity_tag_service
            .get_tags_map(&host_ids, EntityDiscriminants::Host)
            .await?;

        let responses = hosts
            .into_iter()
            .map(|mut host| {
                // Apply hydrated tags
                if let Some(tags) = tags_map.get(&host.id) {
                    host.base.tags = tags.clone();
                }
                let interfaces = interfaces_map.get(&host.id).cloned().unwrap_or_default();
                let ports = ports_map.get(&host.id).cloned().unwrap_or_default();
                let services = services_map.get(&host.id).cloned().unwrap_or_default();
                let if_entries = if_entries_map.get(&host.id).cloned().unwrap_or_default();
                HostResponse::from_host_with_children(host, interfaces, ports, services, if_entries)
            })
            .collect();

        Ok(responses)
    }

    /// Get paginated hosts with all children hydrated for API response.
    /// Supports custom ordering via the `order_by` parameter.
    pub async fn get_all_host_responses_paginated(
        &self,
        filter: StorableFilter<Host>,
        order_by: &str,
    ) -> Result<PaginatedResult<HostResponse>> {
        let result = self.storage().get_paginated(filter, order_by).await?;

        if result.items.is_empty() {
            return Ok(PaginatedResult {
                items: vec![],
                total_count: result.total_count,
            });
        }

        let host_ids: Vec<Uuid> = result.items.iter().map(|h| h.id).collect();
        let (interfaces_map, ports_map, services_map, if_entries_map) =
            self.load_children_for_hosts(&host_ids).await?;

        // Hydrate tags from junction table
        let tags_map = self
            .entity_tag_service
            .get_tags_map(&host_ids, EntityDiscriminants::Host)
            .await?;

        let responses = result
            .items
            .into_iter()
            .map(|mut host| {
                // Apply hydrated tags
                if let Some(tags) = tags_map.get(&host.id) {
                    host.base.tags = tags.clone();
                }
                let interfaces = interfaces_map.get(&host.id).cloned().unwrap_or_default();
                let ports = ports_map.get(&host.id).cloned().unwrap_or_default();
                let services = services_map.get(&host.id).cloned().unwrap_or_default();
                let if_entries = if_entries_map.get(&host.id).cloned().unwrap_or_default();
                HostResponse::from_host_with_children(host, interfaces, ports, services, if_entries)
            })
            .collect();

        Ok(PaginatedResult {
            items: responses,
            total_count: result.total_count,
        })
    }

    /// Load all children for a single host.
    async fn load_children_for_host(
        &self,
        host_id: &Uuid,
    ) -> Result<(Vec<Interface>, Vec<Port>, Vec<Service>, Vec<IfEntry>)> {
        let interfaces = self.interface_service.get_for_host(host_id).await?;
        let ports = self.port_service.get_for_host(host_id).await?;
        let services = self
            .service_service
            .get_all_ordered(
                StorableFilter::<Service>::new_from_host_ids(&[*host_id]),
                "position ASC",
            )
            .await?;
        let if_entries = self.if_entry_service.get_for_host(host_id).await?;

        Ok((interfaces, ports, services, if_entries))
    }

    /// Batch load all children for multiple hosts.
    async fn load_children_for_hosts(
        &self,
        host_ids: &[Uuid],
    ) -> Result<(
        HashMap<Uuid, Vec<Interface>>,
        HashMap<Uuid, Vec<Port>>,
        HashMap<Uuid, Vec<Service>>,
        HashMap<Uuid, Vec<IfEntry>>,
    )> {
        let interfaces_map = self.interface_service.get_for_hosts(host_ids).await?;
        let ports_map = self.port_service.get_for_hosts(host_ids).await?;

        // Load services ordered by position and group by host_id
        let services = self
            .service_service
            .get_all_ordered(
                StorableFilter::<Service>::new_from_host_ids(host_ids),
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

        // Load if_entries and group by host_id
        let mut if_entries_map = self.if_entry_service.get_for_hosts(host_ids).await?;
        // Sort each host's entries by if_index
        for entries in if_entries_map.values_mut() {
            entries.sort_by_key(|e| e.base.if_index);
        }

        Ok((interfaces_map, ports_map, services_map, if_entries_map))
    }

    // =========================================================================
    // Host creation with children
    // =========================================================================

    /// Create a host with all its children (interfaces, ports, services, if_entries) from API request.
    /// Client provides UUIDs for all entities, enabling services to reference interfaces/ports.
    /// For API users: errors if a host with matching interfaces already exists.
    pub async fn create_from_request(
        &self,
        request: CreateHostRequest,
        authentication: AuthenticatedEntity,
    ) -> Result<HostResponse> {
        // Destructure request to ensure compile error if fields change
        let CreateHostRequest {
            name,
            network_id,
            hostname,
            description,
            virtualization,
            hidden,
            tags,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            credential_assignments,
            interfaces: interface_inputs,
            ports: port_inputs,
            services: service_inputs,
            if_entries: if_entry_inputs,
        } = request;

        // Resolve and validate positions (no existing entities for create)
        let empty_interfaces: Vec<Interface> = vec![];
        let empty_services: Vec<Service> = vec![];
        let mut interface_inputs = interface_inputs;
        let mut service_inputs = service_inputs;
        resolve_and_validate_input_positions(&mut interface_inputs, &empty_interfaces, "interface")
            .map_err(|e| ValidationError::new(e.message))?;
        resolve_and_validate_input_positions(&mut service_inputs, &empty_services, "service")
            .map_err(|e| ValidationError::new(e.message))?;

        // Auto-set source to Manual for API-created entities
        let source = EntitySource::Manual;

        // Create host base with SNMP fields
        let host_base = HostBase {
            name: name.clone(),
            network_id,
            hostname,
            description,
            source: source.clone(),
            virtualization,
            hidden,
            tags,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            credential_assignments,
        };
        let host = Host::new(host_base);

        // Build interfaces with client-provided IDs
        let interfaces: Vec<Interface> = interface_inputs
            .into_iter()
            .map(|input| input.into_interface(host.id, network_id))
            .collect();

        // Build ports with client-provided IDs
        let ports: Vec<Port> = port_inputs
            .into_iter()
            .map(|input| input.into_port(host.id, network_id))
            .collect();

        // Build services with client-provided IDs
        let services: Vec<Service> = service_inputs
            .into_iter()
            .map(|input| input.into_service(host.id, network_id, source.clone()))
            .collect();

        // Build if_entries (server assigns UUIDs)
        let if_entries: Vec<IfEntry> = if_entry_inputs
            .into_iter()
            .map(|input| input.into_if_entry(host.id, network_id))
            .collect();

        // Use unified creation with Error behavior for API users
        self.create_with_children(
            host,
            interfaces,
            ports,
            services,
            if_entries,
            ConflictBehavior::Error,
            authentication,
            None, // limit checked in handler
        )
        .await
    }

    /// Create a host with all children, handling conflicts according to behavior.
    /// This is the unified internal method used by both API and discovery paths.
    ///
    /// ## Host Deduplication Flow
    ///
    /// Host deduplication happens in two stages:
    ///
    /// 1. **Interface-based matching** (this method): `find_matching_host_by_interfaces` compares
    ///    incoming interfaces against existing hosts using MAC address or subnet+IP matching.
    ///    - For API users (ConflictBehavior::Error): Returns an error telling them to edit the existing host.
    ///    - For discovery (ConflictBehavior::Upsert): Sets `host.id = existing_host.id` so the
    ///      subsequent create() call will recognize this as an existing host.
    ///
    /// 2. **ID-based matching** (in `create()`): Uses `Host::eq` which only compares IDs.
    ///    Since we set `host.id = existing_host.id` in step 1, the create() method will find
    ///    a match and call `upsert_host()` to merge discovery data.
    ///
    /// This two-stage approach means:
    /// - Interface matching handles the "is this the same physical host?" question
    /// - ID matching handles the "should we upsert?" question (relies on ID being set correctly)
    /// - Discovery always upserts when interfaces match, even if daemon reported a different host ID
    #[allow(clippy::too_many_arguments)]
    async fn create_with_children(
        &self,
        mut host: Host,
        interfaces: Vec<Interface>,
        ports: Vec<Port>,
        services: Vec<Service>,
        if_entries: Vec<IfEntry>,
        conflict_behavior: ConflictBehavior,
        authentication: AuthenticatedEntity,
        limit_ctx: Option<&HostLimitContext>,
    ) -> Result<HostResponse> {
        // Stage 1: Interface-based collision detection
        // Compares MAC addresses and subnet+IP to find hosts that represent the same physical machine
        let matching_result = self
            .find_matching_host_by_interfaces(&host.base.network_id, &interfaces)
            .await?;

        let is_new_host = matching_result.is_none();

        if let Some((existing_host, _)) = matching_result {
            match conflict_behavior {
                ConflictBehavior::Error => {
                    // API users should edit the existing host rather than create a duplicate
                    return Err(ValidationError::new(format!(
                        "A host with matching interfaces already exists: '{}' (id: {}). \
                         Edit the existing host instead of creating a new one.",
                        existing_host.base.name, existing_host.id
                    ))
                    .into());
                }
                ConflictBehavior::Upsert => {
                    // For discovery: align the incoming host ID with the existing host
                    // This ensures create() will match via Host::eq (which compares IDs)
                    // and trigger upsert_host() to merge discovery metadata
                    if host.id != existing_host.id {
                        tracing::debug!(
                            incoming_host_id = %host.id,
                            matched_host_id = %existing_host.id,
                            matched_host_name = %existing_host.base.name,
                            "Setting host ID to match existing host found via interface comparison"
                        );
                        host.id = existing_host.id;
                    }
                }
            }
        }

        // Check host limit for new hosts (not upserts)
        if is_new_host && let Some(ctx) = limit_ctx {
            let filter = StorableFilter::<Host>::new_from_network_ids(&ctx.org_network_ids);
            let current_hosts = self.get_all(filter).await?.len() as u64;
            if current_hosts >= ctx.limit {
                return Err(anyhow!(
                    "Host limit reached ({}/{}). Upgrade your plan for unlimited hosts.",
                    current_hosts,
                    ctx.limit
                ));
            }
        }

        // Store original entities for binding reassignment (discovery case)
        // These are needed because interface/port IDs may change during creation,
        // and service bindings need to be remapped to the new IDs
        let original_host = host.clone();
        let original_interfaces = interfaces.clone();
        let original_ports = ports.clone();

        // Stage 2: Create or upsert host via ID matching
        // If host.id was set to an existing host's ID above, this will trigger upsert_host()
        let mut created_host = self.create(host, authentication.clone()).await?;

        // Capture daemon interface ID → IP mapping before interfaces are consumed.
        // Used later to remap credential assignment interface_ids to server-assigned IDs.
        let daemon_interface_ips: Vec<(Uuid, IpAddr)> = interfaces
            .iter()
            .map(|i| (i.id, i.base.ip_address))
            .collect();

        // Create interfaces with correct host_id
        // For Upsert: deduplicate by checking existing interfaces first
        // For Error: just create (will fail on duplicate constraint)
        let mut created_interfaces = Vec::new();
        for mut interface in interfaces {
            interface.base.host_id = created_host.id;

            if matches!(conflict_behavior, ConflictBehavior::Upsert) {
                // Check if interface already exists by ID
                if let Some(existing_iface) =
                    self.interface_service.get_by_id(&interface.id).await?
                {
                    created_interfaces.push(existing_iface);
                    continue;
                }

                // Check by unique constraint (host_id, subnet_id, ip_address)
                let filter =
                    StorableFilter::<Interface>::new_from_host_ids(&[interface.base.host_id])
                        .subnet_id(&interface.base.subnet_id);
                let existing_by_key: Vec<Interface> =
                    self.interface_service.get_all(filter).await?;
                if let Some(existing_iface) = existing_by_key
                    .into_iter()
                    .find(|i| i.base.ip_address == interface.base.ip_address)
                {
                    created_interfaces.push(existing_iface);
                    continue;
                }

                // MAC fallback: find by (host_id, mac_address) when subnet differs
                // This handles cases where subnet_id changed between discovery runs
                if let Some(mac) = &interface.base.mac_address {
                    let mac_filter =
                        StorableFilter::<Interface>::new_from_host_ids(&[interface.base.host_id])
                            .mac_address(mac);
                    let existing_by_mac: Vec<Interface> =
                        self.interface_service.get_all(mac_filter).await?;
                    if let Some(existing_iface) = existing_by_mac.into_iter().next() {
                        tracing::debug!(
                            interface_ip = %interface.base.ip_address,
                            interface_mac = %mac,
                            existing_subnet_id = %existing_iface.base.subnet_id,
                            incoming_subnet_id = %interface.base.subnet_id,
                            "Found existing interface by MAC address (subnet_id differs)"
                        );
                        created_interfaces.push(existing_iface);
                        continue;
                    }
                }
            }

            let created = self
                .interface_service
                .create(interface, authentication.clone())
                .await?;
            created_interfaces.push(created);
        }

        // Create ports with correct host_id
        // For Upsert: deduplicate by checking existing ports first
        // For Error: just create (will fail on duplicate constraint)
        let mut created_ports = Vec::new();
        for port in ports {
            let port_with_host = port.with_host(created_host.id, created_host.base.network_id);

            if matches!(conflict_behavior, ConflictBehavior::Upsert) {
                // Check if port already exists by ID
                if let Some(existing_port) = self.port_service.get_by_id(&port_with_host.id).await?
                {
                    created_ports.push(existing_port);
                    continue;
                }

                // Check by unique constraint (host_id, port_number, protocol)
                let existing_ports = self.port_service.get_for_host(&created_host.id).await?;
                let port_config = port_with_host.base.port_type.config();
                if let Some(existing_port) = existing_ports.into_iter().find(|p| {
                    let existing_config = p.base.port_type.config();
                    existing_config.number == port_config.number
                        && existing_config.protocol == port_config.protocol
                }) {
                    created_ports.push(existing_port);
                    continue;
                }
            }

            let created = self
                .port_service
                .create(port_with_host, authentication.clone())
                .await?;
            created_ports.push(created);
        }

        // Pre-fetch existing services for ID alignment (same pattern as host at line 596-604).
        // When the same service is re-discovered from a different scan phase (e.g., network scan
        // then Docker scan), aligning IDs ensures partition_conflicting_bindings excludes the
        // existing service's bindings and create() reaches the upsert path.
        let existing_services_for_match = if matches!(conflict_behavior, ConflictBehavior::Upsert) {
            self.service_service
                .get_all(StorableFilter::<Service>::new_from_host_ids(&[
                    created_host.id,
                ]))
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };

        // Create services with bindings reassigned (for discovery where IDs may change)
        // Track claimed bindings in this batch to detect in-batch conflicts
        let mut batch_claimed: Vec<(Uuid, Option<Uuid>)> = Vec::new();
        // Collect orphaned bindings from dropped services to assign to OpenPorts
        let mut orphaned_bindings: Vec<Binding> = Vec::new();
        let mut created_services = Vec::new();

        for service in services {
            let mut reassigned = self
                .service_service
                .reassign_service_interface_bindings(
                    service,
                    &original_host,
                    &original_interfaces,
                    &original_ports,
                    &created_host,
                    &created_interfaces,
                    &created_ports,
                )
                .await;

            // Align service ID with existing match so conflict check excludes its bindings
            if let Some(existing) = existing_services_for_match
                .iter()
                .find(|e| **e == reassigned)
            {
                reassigned.id = existing.id;
            }

            // Check for binding conflicts with other services (DB + batch)
            let (valid_bindings, conflicting_bindings) = self
                .service_service
                .partition_conflicting_bindings(
                    &created_host.id,
                    &reassigned.id,
                    reassigned.base.bindings.clone(),
                    &batch_claimed,
                )
                .await?;

            if !conflicting_bindings.is_empty() {
                // Check if this service matches an existing one on this host (ID was
                // aligned earlier to enable upsert). When true, partial conflicts are
                // expected — the service is being re-discovered from a different scan
                // phase (e.g., Docker scan after network scan) and some of its new
                // bindings may conflict with other services like Unclaimed Open Ports.
                // We proceed with non-conflicting bindings so the upsert can merge
                // metadata (e.g., Docker virtualization).
                let matches_existing_service = existing_services_for_match
                    .iter()
                    .any(|e| e.id == reassigned.id);

                if matches_existing_service {
                    tracing::debug!(
                        service_name = %reassigned.base.name,
                        service_definition = %reassigned.base.service_definition.name(),
                        conflicting_count = conflicting_bindings.len(),
                        valid_count = valid_bindings.len(),
                        "Re-discovered service has partial binding conflicts - proceeding with valid bindings for upsert"
                    );
                    reassigned.base.bindings = valid_bindings;
                } else if reassigned.base.virtualization.is_some()
                    && ServiceDefinitionExt::is_generic(&reassigned.base.service_definition)
                {
                    // Safety net for Docker container → specific service reconciliation.
                    //
                    // When the Docker scan can't identify a container's specific service
                    // (e.g., exec-based and external endpoint probing both fail to match),
                    // it creates a generic "Docker Container" service. This conflicts with
                    // the specific service already found by the network scan (same port).
                    //
                    // Rather than dropping the Docker Container and losing its virtualization
                    // metadata (container name, container ID, Docker daemon linkage), we find
                    // the specific service that claims the conflicting port and set the Docker
                    // virtualization on it directly. The network scan already correctly
                    // identified the service; we're just adding the Docker container metadata.
                    let conflicting_port_ids: Vec<Uuid> = conflicting_bindings
                        .iter()
                        .filter_map(|b| b.port_id())
                        .collect();

                    // Find non-generic services on this host that claim the conflicting ports
                    let enrichable_services: Vec<&Service> = existing_services_for_match
                        .iter()
                        .filter(|s| {
                            !ServiceDefinitionExt::is_generic(&s.base.service_definition)
                                && s.base.virtualization.is_none()
                                && s.base.bindings.iter().any(|b| {
                                    b.port_id()
                                        .is_some_and(|pid| conflicting_port_ids.contains(&pid))
                                })
                        })
                        .collect();

                    if !enrichable_services.is_empty() {
                        for existing_svc in enrichable_services {
                            tracing::info!(
                                service_name = %existing_svc.base.name,
                                service_definition = %existing_svc.base.service_definition.name(),
                                container_service = %reassigned.base.name,
                                "Setting Docker virtualization on existing service from conflicting Docker Container"
                            );
                            let mut updated = existing_svc.clone();
                            updated.base.virtualization = reassigned.base.virtualization.clone();
                            let _ = self.service_service.storage().update(&mut updated).await;
                        }
                    }

                    // Still drop the generic Docker Container service itself
                    continue;
                } else {
                    let conflicting_ports: Vec<_> = conflicting_bindings
                        .iter()
                        .filter_map(|b| {
                            if let BindingType::Port { port_id, .. } = &b.base.binding_type {
                                created_ports
                                    .iter()
                                    .find(|p| p.id == *port_id)
                                    .map(|p| p.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();

                    tracing::warn!(
                        service_name = %reassigned.base.name,
                        service_definition = %reassigned.base.service_definition.name(),
                        host_id = %created_host.id,
                        conflicting_ports = ?conflicting_ports,
                        valid_binding_count = valid_bindings.len(),
                        "Discovery found service with conflicting port bindings - dropping service"
                    );

                    orphaned_bindings.extend(valid_bindings);
                    continue;
                }
            }

            // Track this service's port bindings for in-batch conflict detection
            for binding in &reassigned.base.bindings {
                if let BindingType::Port {
                    port_id,
                    interface_id,
                } = &binding.base.binding_type
                {
                    batch_claimed.push((*port_id, *interface_id));
                }
            }

            let created = self
                .service_service
                .create(reassigned, authentication.clone())
                .await?;
            created_services.push(created);
        }

        // If we have orphaned bindings, assign them to OpenPorts service
        if !orphaned_bindings.is_empty() {
            use crate::server::services::definitions::open_ports::OpenPorts as OpenPortsDef;
            use crate::server::services::r#impl::base::ServiceBase;

            tracing::info!(
                host_id = %created_host.id,
                orphaned_binding_count = orphaned_bindings.len(),
                "Assigning orphaned bindings to OpenPorts service"
            );

            let open_ports_service = Service::new(ServiceBase {
                host_id: created_host.id,
                network_id: created_host.base.network_id,
                service_definition: Box::new(OpenPortsDef),
                name: "Unclaimed Open Ports".to_string(),
                bindings: orphaned_bindings,
                virtualization: None,
                source: EntitySource::Discovery { metadata: vec![] },
                tags: Vec::new(),
                position: 0,
            });

            // The singleton upsert in service.create() will merge bindings
            // if an OpenPorts service already exists on this host
            let created = self
                .service_service
                .create(open_ports_service, authentication.clone())
                .await?;
            created_services.push(created);
        }

        tracing::info!(
            host_id = %created_host.id,
            host_name = %created_host.base.name,
            interface_count = %created_interfaces.len(),
            port_count = %created_ports.len(),
            service_count = %created_services.len(),
            "Created host with children"
        );

        if let Some(org_id) = authentication.organization_id() {
            self.entity_tag_service
                .set_tags(
                    created_host.id,
                    EntityDiscriminants::Host,
                    created_host.base.tags.clone(),
                    org_id,
                )
                .await?;
        }

        // Create if_entries with correct host_id
        // Uses create_or_update_by_if_index for deduplication on (host_id, if_index)
        let mut created_if_entries = Vec::new();
        for mut entry in if_entries {
            entry.base.host_id = created_host.id;
            entry.base.network_id = created_host.base.network_id;

            let created = self
                .if_entry_service
                .create_or_update_by_if_index(entry, authentication.clone())
                .await?;
            created_if_entries.push(created);
        }

        // Remap credential assignment interface_ids from daemon UUIDs to server UUIDs
        // and persist to the host_credentials junction table.
        // credential_assignments is transient on HostBase (not stored in hosts table),
        // so we must persist via set_host_credentials and use the original input host's
        // assignments (created_host.base.credential_assignments is empty after DB round-trip).
        let mut remapped_assignments = original_host.base.credential_assignments.clone();
        for assignment in &mut remapped_assignments {
            if let Some(ref mut ids) = assignment.interface_ids {
                *ids = ids
                    .iter()
                    .filter_map(|daemon_id| {
                        let ip = daemon_interface_ips
                            .iter()
                            .find(|(id, _)| id == daemon_id)
                            .map(|(_, ip)| *ip)?;
                        created_interfaces
                            .iter()
                            .find(|i| i.base.ip_address == ip)
                            .map(|i| i.id)
                    })
                    .collect();
            }
        }
        if !remapped_assignments.is_empty()
            && let Err(e) = self
                .credential_service
                .set_host_credentials(&created_host.id, &remapped_assignments)
                .await
        {
            tracing::warn!(
                host_id = %created_host.id,
                error = ?e,
                "Failed to persist credential assignments during discover_host"
            );
        }
        created_host.base.credential_assignments = remapped_assignments;

        Ok(HostResponse::from_host_with_children(
            created_host,
            created_interfaces,
            created_ports,
            created_services,
            created_if_entries,
        ))
    }

    /// Update a host from an UpdateHostRequest
    /// Optionally syncs interfaces and ports if provided in the request.
    pub async fn update_from_request(
        &self,
        request: UpdateHostRequest,
        authentication: AuthenticatedEntity,
    ) -> Result<HostResponse> {
        // Get existing host
        let existing = self
            .get_by_id(&request.id)
            .await?
            .ok_or_else(|| anyhow!("Host '{}' not found", request.id))?;

        let network_id = existing.base.network_id;
        let UpdateHostRequest {
            id,
            name,
            hostname,
            description,
            virtualization,
            hidden,
            tags,
            expected_updated_at: _,
            interfaces,
            ports,
            services,
            credential_assignments,
        } = request;

        // Optimistic locking: check if host was modified since user loaded it
        // Compare at microsecond precision since PostgreSQL TIMESTAMPTZ truncates nanoseconds
        if let Some(expected) = request.expected_updated_at
            && existing.updated_at.timestamp_micros() != expected.timestamp_micros()
        {
            tracing::warn!(
                host_id = %id,
                expected = %expected,
                actual = %existing.updated_at,
                "Host update conflict - host was modified since user loaded it"
            );
            return Err(ValidationError::new(format!(
                "Host was modified by another process (possibly discovery). \
                     Please reload and try again. Expected: {}, Actual: {}",
                expected, existing.updated_at
            ))
            .into());
        }

        let mut updated_host = Host {
            id,
            created_at: existing.created_at,
            updated_at: Utc::now(),
            base: HostBase {
                name,
                network_id,
                source: existing.base.source,
                hostname,
                description,
                virtualization,
                hidden,
                tags: tags.clone(),
                // Preserve existing SNMP fields on update
                sys_descr: existing.base.sys_descr.clone(),
                sys_object_id: existing.base.sys_object_id.clone(),
                sys_location: existing.base.sys_location.clone(),
                sys_contact: existing.base.sys_contact.clone(),
                management_url: existing.base.management_url.clone(),
                chassis_id: existing.base.chassis_id.clone(),
                sys_name: existing.base.sys_name.clone(),
                manufacturer: existing.base.manufacturer.clone(),
                model: existing.base.model.clone(),
                serial_number: existing.base.serial_number.clone(),
                credential_assignments: credential_assignments
                    .unwrap_or_else(|| existing.base.credential_assignments.clone()),
            },
        };

        if let Some(org_id) = authentication.organization_id() {
            self.entity_tag_service
                .set_tags(id, EntityDiscriminants::Host, tags, org_id)
                .await?;
        }

        let updated = self
            .update(&mut updated_host, authentication.clone())
            .await?;

        // Sync interfaces only if provided (None means preserve existing)
        if let Some(interfaces) = interfaces {
            self.sync_interfaces(&updated.id, &network_id, interfaces, authentication.clone())
                .await?;
        }

        // Sync ports only if provided (None means preserve existing)
        if let Some(ports) = ports {
            self.sync_ports(&updated.id, &network_id, ports, authentication.clone())
                .await?;
        }

        // Sync services only if provided (None means preserve existing)
        if let Some(services) = services {
            self.sync_services(&updated.id, &network_id, services, authentication.clone())
                .await?;
        }

        // Load fresh children after sync
        let (interfaces, ports, services, if_entries) =
            self.load_children_for_host(&updated.id).await?;

        Ok(HostResponse::from_host_with_children(
            updated, interfaces, ports, services, if_entries,
        ))
    }

    /// Sync interfaces for a host: delete removed, update existing, create new.
    /// Client provides UUIDs - if ID exists for this host, update; if not, create.
    async fn sync_interfaces(
        &self,
        host_id: &Uuid,
        network_id: &Uuid,
        inputs: Vec<InterfaceInput>,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use std::collections::HashSet;

        // Get existing interfaces for this host (needed for position resolution)
        let existing = self.interface_service.get_for_host(host_id).await?;
        let existing_ids: HashSet<Uuid> = existing.iter().map(|i| i.id).collect();

        // Resolve and validate positions
        let mut inputs = inputs;
        resolve_and_validate_input_positions(&mut inputs, &existing, "interface")
            .map_err(|e| ValidationError::new(e.message))?;

        // All input IDs (client-provided)
        let input_ids: HashSet<Uuid> = inputs.iter().map(|i| i.id).collect();

        // Delete interfaces that are not in the input list
        let to_delete: Vec<Uuid> = existing_ids.difference(&input_ids).copied().collect();
        if !to_delete.is_empty() {
            self.interface_service
                .delete_many(&to_delete, authentication.clone())
                .await?;
        }

        // Process each input - create or update based on whether ID exists for this host
        for input in inputs {
            let id = input.id;
            let mut interface = input.into_interface(*host_id, *network_id);

            if existing_ids.contains(&id) {
                // Update existing interface - preserve created_at from existing
                if let Some(existing_iface) = existing.iter().find(|i| i.id == id) {
                    interface.preserve_immutable_fields(existing_iface);
                }

                self.interface_service
                    .update(&mut interface, authentication.clone())
                    .await?;
            } else {
                // Create new interface with client-provided ID
                self.interface_service
                    .create(interface, authentication.clone())
                    .await?;
            }
        }

        Ok(())
    }

    /// Sync ports for a host: delete removed, create new, update existing.
    /// Client provides UUIDs - if ID exists for this host, update; if not, create.
    async fn sync_ports(
        &self,
        host_id: &Uuid,
        network_id: &Uuid,
        inputs: Vec<PortInput>,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use std::collections::HashSet;

        // Get existing ports for this host
        let existing = self.port_service.get_for_host(host_id).await?;
        let existing_ids: HashSet<Uuid> = existing.iter().map(|p| p.id).collect();

        // All input IDs (client-provided)
        let input_ids: HashSet<Uuid> = inputs.iter().map(|p| p.id).collect();

        // Delete ports that are not in the input list
        let to_delete: Vec<Uuid> = existing_ids.difference(&input_ids).copied().collect();
        if !to_delete.is_empty() {
            self.port_service
                .delete_many(&to_delete, authentication.clone())
                .await?;
        }

        // Process each input - create or update based on whether ID exists for this host
        for input in inputs {
            let id = input.id;
            let mut port = input.into_port(*host_id, *network_id);

            if existing_ids.contains(&id) {
                // Update existing port - preserve created_at from existing
                if let Some(existing_port) = existing.iter().find(|p| p.id == id) {
                    port.preserve_immutable_fields(existing_port);
                }

                self.port_service
                    .update(&mut port, authentication.clone())
                    .await?;
            } else {
                // Create new port with client-provided ID
                self.port_service
                    .create(port, authentication.clone())
                    .await?;
            }
        }

        Ok(())
    }

    /// Sync services for a host: delete removed, update existing, create new.
    /// Client provides UUIDs - if ID exists for this host, update; if not, create.
    async fn sync_services(
        &self,
        host_id: &Uuid,
        network_id: &Uuid,
        inputs: Vec<ServiceInput>,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use std::collections::HashSet;

        // Get existing services for this host (needed for position resolution)
        let existing = self.service_service.get_for_parent(host_id).await?;
        let existing_ids: HashSet<Uuid> = existing.iter().map(|s| s.id).collect();

        // Resolve and validate positions
        let mut inputs = inputs;
        resolve_and_validate_input_positions(&mut inputs, &existing, "service")
            .map_err(|e| ValidationError::new(e.message))?;

        // All input IDs (client-provided)
        let input_ids: HashSet<Uuid> = inputs.iter().map(|s| s.id).collect();

        // Delete services that are not in the input list
        let to_delete: Vec<Uuid> = existing_ids.difference(&input_ids).copied().collect();
        if !to_delete.is_empty() {
            self.service_service
                .delete_many(&to_delete, authentication.clone())
                .await?;
        }

        // Partition inputs: services losing port bindings must be processed first.
        // This ensures bindings are freed in DB before other services try to claim them,
        // which is required for port transfers between services to work correctly.
        let (losing_bindings, others): (Vec<_>, Vec<_>) = inputs.into_iter().partition(|input| {
            if let Some(existing_svc) = existing.iter().find(|s| s.id == input.id) {
                // Get current port binding keys (port_id, interface_id)
                let current_ports: HashSet<_> = existing_svc
                    .base
                    .bindings
                    .iter()
                    .filter_map(|b| match &b.base.binding_type {
                        BindingType::Port {
                            port_id,
                            interface_id,
                        } => Some((*port_id, *interface_id)),
                        _ => None,
                    })
                    .collect();

                // Get input port binding keys
                let input_ports: HashSet<_> = input
                    .bindings
                    .iter()
                    .filter_map(|b| match b {
                        BindingInput::Port {
                            port_id,
                            interface_id,
                            ..
                        } => Some((*port_id, *interface_id)),
                        _ => None,
                    })
                    .collect();

                // Service is "losing" if it has ports in DB that aren't in input
                current_ports.difference(&input_ports).next().is_some()
            } else {
                false // New service, not losing anything
            }
        });

        // Process losing-bindings services first, then others
        let ordered_inputs = losing_bindings.into_iter().chain(others);

        // Process each input - create or update based on whether ID exists for this host
        for input in ordered_inputs {
            let id = input.id;
            // For new services, source is Manual (API-created)
            // For existing services, we'll preserve their source below
            let mut service = input.into_service(*host_id, *network_id, EntitySource::Manual);

            if existing_ids.contains(&id) {
                // Update existing service - preserve immutable fields
                if let Some(existing_svc) = existing.iter().find(|s| s.id == id) {
                    service.preserve_immutable_fields(existing_svc);
                    // Also preserve source - can't change via API
                    service.base.source = existing_svc.base.source.clone();
                }

                self.service_service
                    .update(&mut service, authentication.clone())
                    .await?;
            } else {
                // Create new service with client-provided ID
                self.service_service
                    .create(service, authentication.clone())
                    .await?;
            }
        }

        Ok(())
    }

    // =========================================================================
    // Discovery support (internal API)
    // =========================================================================

    /// Create or update a host from daemon discovery data.
    /// This handles interface/port matching for host deduplication and upserts on conflict.
    #[allow(clippy::too_many_arguments)]
    pub async fn discover_host(
        &self,
        host: Host,
        interfaces: Vec<Interface>,
        ports: Vec<Port>,
        services: Vec<Service>,
        if_entries: Vec<crate::server::if_entries::r#impl::base::IfEntry>,
        authentication: AuthenticatedEntity,
        limit_ctx: Option<&HostLimitContext>,
    ) -> Result<HostResponse> {
        let host_response = self
            .create_with_children(
                host,
                interfaces,
                ports,
                services,
                if_entries.clone(),
                ConflictBehavior::Upsert,
                authentication.clone(),
                limit_ctx,
            )
            .await?;

        // Link IfEntries to Interfaces via MAC address matching (if any were created)
        if !if_entries.is_empty()
            && let Err(e) = self
                .link_if_entries_to_interfaces(&host_response.id, authentication)
                .await
        {
            tracing::warn!(error = %e, "Failed to link IfEntries to Interfaces");
        }

        Ok(host_response)
    }

    /// Link IfEntry records to Interface records for a host by matching MAC addresses.
    ///
    /// For each IfEntry with a MAC address, finds an Interface on the same host with
    /// the same MAC address and sets `if_entry.interface_id = interface.id`.
    /// This enables PhysicalLink topology edges to have source/target Interface IDs.
    async fn link_if_entries_to_interfaces(
        &self,
        host_id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use crate::server::if_entries::r#impl::base::if_type;

        // Get all interfaces for this host
        let interfaces = self.interface_service.get_for_host(host_id).await?;

        // Build MAC -> interface_id lookup
        let mac_to_interface: std::collections::HashMap<_, _> = interfaces
            .iter()
            .filter_map(|iface| iface.base.mac_address.map(|mac| (mac, iface.id)))
            .collect();

        // Find loopback interface (by IP address)
        let loopback_interface_id = interfaces
            .iter()
            .find(|iface| iface.base.ip_address.is_loopback())
            .map(|iface| iface.id);

        // Get all IfEntries for this host
        let if_entries = self.if_entry_service.get_for_host(host_id).await?;

        let mut linked_count = 0;
        for mut if_entry in if_entries {
            // Skip if already linked
            if if_entry.base.interface_id.is_some() {
                continue;
            }

            // Try loopback linking by if_type
            let matched_interface_id = if if_entry.base.if_type == if_type::SOFTWARE_LOOPBACK {
                loopback_interface_id
            } else {
                // Try MAC-based linking
                if_entry
                    .base
                    .mac_address
                    .and_then(|mac| mac_to_interface.get(&mac).copied())
            };

            if let Some(interface_id) = matched_interface_id {
                if_entry.base.interface_id = Some(interface_id);
                if let Err(e) = self
                    .if_entry_service
                    .update(&mut if_entry, authentication.clone())
                    .await
                {
                    tracing::warn!(
                        if_entry_id = %if_entry.id,
                        error = %e,
                        "Failed to link IfEntry to Interface"
                    );
                } else {
                    linked_count += 1;
                }
            }
        }

        if linked_count > 0 {
            tracing::debug!(
                host_id = %host_id,
                linked = linked_count,
                "Linked IfEntries to Interfaces via MAC address and loopback type"
            );
        }

        Ok(())
    }

    /// Find an existing host that matches based on interface data (MAC address or subnet+IP).
    pub async fn find_matching_host_by_interfaces(
        &self,
        network_id: &Uuid,
        incoming_interfaces: &[Interface],
    ) -> Result<Option<(Host, Vec<Interface>)>> {
        if incoming_interfaces.is_empty() {
            return Ok(None);
        }

        let filter = StorableFilter::<Host>::new_from_network_ids(&[*network_id]);
        let all_hosts = self.get_all(filter).await?;

        if all_hosts.is_empty() {
            return Ok(None);
        }

        let host_ids: Vec<Uuid> = all_hosts.iter().map(|h| h.id).collect();
        let interfaces_by_host = self.interface_service.get_for_hosts(&host_ids).await?;

        // Exclude loopback interfaces from matching — every host has 127.0.0.1
        // on the shared loopback subnet, so they would falsely match all hosts
        let non_loopback_incoming: Vec<_> = incoming_interfaces
            .iter()
            .filter(|i| !i.base.ip_address.is_loopback())
            .collect();

        if non_loopback_incoming.is_empty() {
            return Ok(None);
        }

        for host in all_hosts {
            let host_interfaces = interfaces_by_host
                .get(&host.id)
                .cloned()
                .unwrap_or_default();

            for incoming_iface in &non_loopback_incoming {
                for existing_iface in &host_interfaces {
                    if existing_iface.base.ip_address.is_loopback() {
                        continue;
                    }
                    if *incoming_iface == existing_iface {
                        tracing::debug!(
                            incoming_ip = %incoming_iface.base.ip_address,
                            existing_ip = %existing_iface.base.ip_address,
                            existing_host_id = %host.id,
                            existing_host_name = %host.base.name,
                            "Found matching host via interface comparison"
                        );
                        return Ok(Some((host, host_interfaces)));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn get_host_lock(&self, host_id: &Uuid) -> Arc<Mutex<()>> {
        let mut locks = self.host_locks.lock().await;
        locks
            .entry(*host_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Merge new discovery data with existing host
    async fn upsert_host(
        &self,
        mut existing_host: Host,
        new_host_data: Host,
        authentication: AuthenticatedEntity,
    ) -> Result<Host> {
        let host_before_updates = existing_host.clone();
        let mut has_updates = false;

        tracing::trace!(
            "Upserting new host data {:?} to host {:?}",
            new_host_data,
            existing_host
        );

        // Update hostname if not set
        if existing_host.base.hostname.is_none()
            && new_host_data
                .base
                .hostname
                .as_ref()
                .is_some_and(|h| !h.is_empty())
        {
            has_updates = true;
            existing_host.base.hostname = new_host_data.base.hostname.clone();

            // Also update display name if it was auto-set to IP (not manually renamed)
            if existing_host.base.name.parse::<std::net::IpAddr>().is_ok()
                && let Some(ref hostname) = existing_host.base.hostname
            {
                existing_host.base.name = hostname.clone();
            }
        }

        // Update SNMP fields if not set
        if existing_host.base.sys_descr.is_none() && new_host_data.base.sys_descr.is_some() {
            has_updates = true;
            existing_host.base.sys_descr = new_host_data.base.sys_descr;
        }
        if existing_host.base.sys_object_id.is_none() && new_host_data.base.sys_object_id.is_some()
        {
            has_updates = true;
            existing_host.base.sys_object_id = new_host_data.base.sys_object_id;
        }
        if existing_host.base.sys_location.is_none() && new_host_data.base.sys_location.is_some() {
            has_updates = true;
            existing_host.base.sys_location = new_host_data.base.sys_location;
        }
        if existing_host.base.sys_contact.is_none() && new_host_data.base.sys_contact.is_some() {
            has_updates = true;
            existing_host.base.sys_contact = new_host_data.base.sys_contact;
        }
        if existing_host.base.management_url.is_none()
            && new_host_data.base.management_url.is_some()
        {
            has_updates = true;
            existing_host.base.management_url = new_host_data.base.management_url;
        }
        if existing_host.base.chassis_id.is_none() && new_host_data.base.chassis_id.is_some() {
            has_updates = true;
            existing_host.base.chassis_id = new_host_data.base.chassis_id;
        }
        if existing_host.base.sys_name.is_none() && new_host_data.base.sys_name.is_some() {
            has_updates = true;
            existing_host.base.sys_name = new_host_data.base.sys_name;
        }
        if existing_host.base.manufacturer.is_none() && new_host_data.base.manufacturer.is_some() {
            has_updates = true;
            existing_host.base.manufacturer = new_host_data.base.manufacturer;
        }
        if existing_host.base.model.is_none() && new_host_data.base.model.is_some() {
            has_updates = true;
            existing_host.base.model = new_host_data.base.model;
        }
        if existing_host.base.serial_number.is_none() && new_host_data.base.serial_number.is_some()
        {
            has_updates = true;
            existing_host.base.serial_number = new_host_data.base.serial_number;
        }

        // Merge entity source metadata
        existing_host.base.source = match (existing_host.base.source, new_host_data.base.source) {
            (
                EntitySource::Discovery {
                    metadata: existing_metadata,
                },
                EntitySource::Discovery {
                    metadata: new_metadata,
                },
            ) => {
                has_updates = true;
                EntitySource::Discovery {
                    metadata: [new_metadata, existing_metadata].concat(),
                }
                .cap_metadata()
            }
            (
                _,
                EntitySource::Discovery {
                    metadata: new_metadata,
                },
            ) => {
                has_updates = true;
                EntitySource::Discovery {
                    metadata: new_metadata,
                }
            }
            (existing_source, _) => existing_source,
        };

        if has_updates {
            self.storage().update(&mut existing_host).await?;

            let trigger_stale = existing_host.triggers_staleness(Some(host_before_updates));

            self.event_bus()
                .publish_entity(EntityEvent {
                    id: Uuid::new_v4(),
                    entity_id: existing_host.id(),
                    network_id: self.get_network_id(&existing_host),
                    organization_id: self.get_organization_id(&existing_host),
                    entity_type: existing_host.clone().into(),
                    operation: EntityOperation::Updated,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({
                        "trigger_stale": trigger_stale
                    }),

                    authentication,
                })
                .await?;
        } else {
            tracing::debug!(
                "No new data to upsert from host {} to {}",
                new_host_data.base.name,
                existing_host.base.name
            );
        }

        Ok(existing_host)
    }

    pub async fn consolidate_hosts(
        &self,
        destination_host: Host,
        other_host: Host,
        authentication: AuthenticatedEntity,
    ) -> Result<HostResponse> {
        if destination_host.id == other_host.id {
            return Err(ValidationError::new("Can't consolidate a host with itself").into());
        }

        let daemon_filter = StorableFilter::<Daemon>::new_from_host_ids(&[other_host.id]);

        if self.daemon_service.get_one(daemon_filter).await?.is_some() {
            return Err(ValidationError::new(
                "Can't consolidate a host that has a daemon associated with it. \
                 Consolidate the other host into the host with the daemon instead.",
            )
            .into());
        }

        let lock = self.get_host_lock(&destination_host.id).await;
        let _guard1 = lock.lock().await;

        tracing::trace!(
            "Consolidating host {:?} into host {:?}",
            other_host,
            destination_host
        );

        // Get interfaces and ports for both hosts
        let dest_interfaces = self
            .interface_service
            .get_for_host(&destination_host.id)
            .await?;
        let other_interfaces = self.interface_service.get_for_host(&other_host.id).await?;

        let dest_ports = self.port_service.get_for_host(&destination_host.id).await?;
        let other_ports = self.port_service.get_for_host(&other_host.id).await?;

        // Build interface ID mapping: source_interface_id -> dest_interface_id
        // Transfer non-conflicting interfaces to destination
        let mut interface_id_map: HashMap<Uuid, Uuid> = HashMap::new();
        for other_iface in &other_interfaces {
            // Check for conflict: same (subnet_id + ip_address) or same MAC address
            let matching_dest_iface = dest_interfaces.iter().find(|dest_iface| {
                // Match by subnet + IP
                (dest_iface.base.subnet_id == other_iface.base.subnet_id
                    && dest_iface.base.ip_address == other_iface.base.ip_address)
                    // Or match by MAC address if both have one
                    || (dest_iface.base.mac_address.is_some()
                        && dest_iface.base.mac_address == other_iface.base.mac_address)
            });

            if let Some(dest_iface) = matching_dest_iface {
                // Conflict: map source ID to destination ID
                tracing::debug!(
                    source_interface_id = %other_iface.id,
                    dest_interface_id = %dest_iface.id,
                    ip = %other_iface.base.ip_address,
                    "Interface conflict - mapping to existing destination interface"
                );
                interface_id_map.insert(other_iface.id, dest_iface.id);
            } else {
                // No conflict: transfer interface to destination host
                let mut transferred = other_iface.clone();
                transferred.base.host_id = destination_host.id;
                self.interface_service
                    .update(&mut transferred, authentication.clone())
                    .await?;
                tracing::debug!(
                    interface_id = %other_iface.id,
                    ip = %other_iface.base.ip_address,
                    "Transferred interface to destination host"
                );
                // Map to itself (ID unchanged, just host_id changed)
                interface_id_map.insert(other_iface.id, other_iface.id);
            }
        }

        // Build port ID mapping: source_port_id -> dest_port_id
        // Transfer non-conflicting ports to destination
        let mut port_id_map: HashMap<Uuid, Uuid> = HashMap::new();
        for other_port in &other_ports {
            let other_config = other_port.base.port_type.config();

            // Check for conflict: same (number + protocol)
            let matching_dest_port = dest_ports.iter().find(|dest_port| {
                let dest_config = dest_port.base.port_type.config();
                dest_config.number == other_config.number
                    && dest_config.protocol == other_config.protocol
            });

            if let Some(dest_port) = matching_dest_port {
                // Conflict: map source ID to destination ID
                tracing::debug!(
                    source_port_id = %other_port.id,
                    dest_port_id = %dest_port.id,
                    port = %other_config.number,
                    "Port conflict - mapping to existing destination port"
                );
                port_id_map.insert(other_port.id, dest_port.id);
            } else {
                // No conflict: transfer port to destination host
                let mut transferred =
                    other_port.with_host(destination_host.id, destination_host.base.network_id);
                self.port_service
                    .update(&mut transferred, authentication.clone())
                    .await?;
                tracing::debug!(
                    port_id = %other_port.id,
                    port = %other_config.number,
                    "Transferred port to destination host"
                );
                // Map to itself (ID unchanged, just host_id changed)
                port_id_map.insert(other_port.id, other_port.id);
            }
        }

        // Upsert host data (metadata merge)
        let updated_host = self
            .upsert_host(
                destination_host.clone(),
                other_host.clone(),
                authentication.clone(),
            )
            .await?;

        // Get services for both hosts
        let destination_services = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_host_ids(&[
                destination_host.id,
            ]))
            .await?;

        let other_services = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_host_ids(&[
                other_host.id
            ]))
            .await?;

        // Transfer services, updating binding IDs using the maps
        for mut service in other_services {
            // Check for duplicate by name + service_definition
            let is_duplicate = destination_services.iter().any(|dest_svc| {
                dest_svc.base.name == service.base.name
                    && dest_svc.base.service_definition.id() == service.base.service_definition.id()
            });

            if is_duplicate {
                tracing::debug!(
                    service_name = %service.base.name,
                    service_def = %service.base.service_definition.id(),
                    "Skipping duplicate service during consolidation"
                );
                continue;
            }

            // Update host_id
            service.base.host_id = updated_host.id;
            service.base.network_id = updated_host.base.network_id;

            // Remap binding IDs using our maps
            for binding in &mut service.base.bindings {
                match &mut binding.base.binding_type {
                    BindingType::Interface { interface_id } => {
                        if let Some(&new_id) = interface_id_map.get(interface_id) {
                            *interface_id = new_id;
                        } else {
                            tracing::warn!(
                                service = %service.base.name,
                                interface_id = %interface_id,
                                "Interface not found in mapping during consolidation"
                            );
                        }
                    }
                    BindingType::Port {
                        port_id,
                        interface_id,
                    } => {
                        if let Some(&new_port_id) = port_id_map.get(port_id) {
                            *port_id = new_port_id;
                        } else {
                            tracing::warn!(
                                service = %service.base.name,
                                port_id = %port_id,
                                "Port not found in mapping during consolidation"
                            );
                        }
                        if let Some(iface_id) = interface_id {
                            if let Some(&new_iface_id) = interface_id_map.get(iface_id) {
                                *interface_id = Some(new_iface_id);
                            } else {
                                tracing::warn!(
                                    service = %service.base.name,
                                    interface_id = %iface_id,
                                    "Interface not found in mapping, falling back to all-interfaces"
                                );
                                *interface_id = None;
                            }
                        }
                    }
                }
            }

            self.service_service
                .update(&mut service, authentication.clone())
                .await
                .map_err(|e| {
                    tracing::error!(
                        service_id = %service.id,
                        service_name = %service.base.name,
                        "Failed to update service during consolidation: {}",
                        e
                    );
                    anyhow!(
                        "Failed to update service '{}' during consolidation: {}",
                        service.base.name,
                        e
                    )
                })?;
        }

        // Migrate credential assignments from other host to destination host
        let other_assignments = self
            .credential_service
            .get_credential_assignments_for_host(&other_host.id)
            .await?;

        if !other_assignments.is_empty() {
            use crate::server::credentials::r#impl::types::CredentialAssignment;

            let dest_assignments = self
                .credential_service
                .get_credential_assignments_for_host(&updated_host.id)
                .await?;

            let dest_cred_map: HashMap<Uuid, &CredentialAssignment> = dest_assignments
                .iter()
                .map(|a| (a.credential_id, a))
                .collect();

            let mut merged: Vec<CredentialAssignment> = dest_assignments.clone();
            let mut migrated_count = 0usize;

            for other in &other_assignments {
                // Remap interface_ids if present
                let remapped_iface_ids = match &other.interface_ids {
                    None => None,
                    Some(ids) => {
                        let remapped: Vec<Uuid> = ids
                            .iter()
                            .filter_map(|id| interface_id_map.get(id).copied())
                            .collect();
                        if remapped.is_empty() {
                            // All interfaces were dropped — skip this assignment
                            continue;
                        }
                        Some(remapped)
                    }
                };

                if let Some(dest_assignment) = dest_cred_map.get(&other.credential_id) {
                    // Both hosts have this credential — merge with broadest-scope-wins
                    let merged_iface_ids =
                        match (&dest_assignment.interface_ids, &remapped_iface_ids) {
                            (None, _) | (_, None) => None, // Either is all-interfaces → all
                            (Some(dest_ids), Some(other_ids)) => {
                                let mut union = dest_ids.clone();
                                for id in other_ids {
                                    if !union.contains(id) {
                                        union.push(*id);
                                    }
                                }
                                Some(union)
                            }
                        };

                    // Update the existing dest entry in merged list
                    if let Some(entry) = merged
                        .iter_mut()
                        .find(|a| a.credential_id == other.credential_id)
                    {
                        entry.interface_ids = merged_iface_ids;
                    }
                } else {
                    // Only on other host — add to merged list
                    merged.push(CredentialAssignment {
                        credential_id: other.credential_id,
                        interface_ids: remapped_iface_ids,
                    });
                }
                migrated_count += 1;
            }

            self.credential_service
                .set_host_credentials(&updated_host.id, &merged)
                .await?;

            tracing::info!(
                migrated = migrated_count,
                source_host_id = %other_host.id,
                dest_host_id = %updated_host.id,
                "Migrated credential assignments during consolidation"
            );
        }

        // Delete other host (remaining children that weren't transferred will cascade)
        self.delete_host(&other_host.id, authentication).await?;

        tracing::info!(
            source_host_id = %other_host.id,
            source_host_name = %other_host.base.name,
            dest_host_id = %updated_host.id,
            dest_host_name = %updated_host.base.name,
            interfaces_mapped = %interface_id_map.len(),
            ports_mapped = %port_id_map.len(),
            "Hosts consolidated"
        );

        // Return response with hydrated children
        let (interfaces, ports, services, if_entries) =
            self.load_children_for_host(&updated_host.id).await?;
        Ok(HostResponse::from_host_with_children(
            updated_host,
            interfaces,
            ports,
            services,
            if_entries,
        ))
    }

    // =========================================================================
    // LLDP link resolution
    // =========================================================================

    /// Resolve LLDP links for all if_entries in a network.
    ///
    /// Called by DiscoveryService when a discovery session completes successfully.
    /// This resolves LLDP neighbor data (chassis ID, port ID) to actual database
    /// entity references via the Neighbor enum.
    ///
    /// Resolution states:
    /// - Full resolution: Both host and port identified → `Neighbor::IfEntry(id)`
    /// - Partial resolution: Only host identified → `Neighbor::Host(id)`
    ///
    /// Returns statistics about the resolution process.
    pub async fn resolve_lldp_links(&self, network_id: Uuid) -> Result<LldpResolutionStats> {
        use crate::server::if_entries::r#impl::base::Neighbor;

        let resolver = LldpResolverImpl::new(
            self.if_entry_service.clone(),
            self.interface_service.clone(),
            self.storage.clone(),
        );

        // Get all if_entries with unresolved LLDP/CDP neighbors in this network
        let filter = StorableFilter::<IfEntry>::new_for_unresolved_lldp_in_network(network_id);
        let unresolved = self.if_entry_service.get_all(filter).await?;

        let mut stats = LldpResolutionStats::default();

        for mut if_entry in unresolved {
            stats.total += 1;

            // Try LLDP resolution first (more detailed data)
            // Only use chassis_id and port_id for neighbor resolution - these represent
            // actual physical connections. lldp_mgmt_addr is where you manage the device,
            // not necessarily the physical connection point.
            let resolved_neighbor = if let Some(ref chassis_id) = if_entry.base.lldp_chassis_id {
                // Resolve host from LLDP chassis ID
                if let Some(host_id) = chassis_id.resolve_host_id(&resolver, network_id).await {
                    stats.hosts_resolved += 1;

                    // Try to resolve specific port
                    if let Some(ref port_id) = if_entry.base.lldp_port_id
                        && let Some(remote_if_entry_id) =
                            port_id.resolve_if_entry_id(&resolver, host_id).await
                    {
                        stats.ports_resolved += 1;
                        Some(Neighbor::IfEntry(remote_if_entry_id))
                    } else {
                        Some(Neighbor::Host(host_id))
                    }
                } else {
                    None
                }
            } else if let Some(ref device_id) = if_entry.base.cdp_device_id {
                // CDP device_id is typically sysName, resolve against sys_name field
                // Don't fall back to cdp_address - it's management address, not physical connection
                if let Some(host_id) = resolver.find_host_by_sys_name(device_id, network_id).await {
                    stats.hosts_resolved += 1;

                    // Try CDP port resolution using cdp_port_id (long ifDescr format)
                    if let Some(ref port_id) = if_entry.base.cdp_port_id
                        && let Some(remote_if_entry_id) =
                            resolver.find_if_entry_by_name(port_id, host_id).await
                    {
                        stats.ports_resolved += 1;
                        Some(Neighbor::IfEntry(remote_if_entry_id))
                    } else {
                        Some(Neighbor::Host(host_id))
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Persist resolved neighbor
            if let Some(neighbor) = resolved_neighbor {
                if_entry.base.neighbor = Some(neighbor);
                self.if_entry_service
                    .update(&mut if_entry, AuthenticatedEntity::System)
                    .await?;
            }
        }

        tracing::info!(
            network_id = %network_id,
            total = stats.total,
            hosts_resolved = stats.hosts_resolved,
            ports_resolved = stats.ports_resolved,
            "LLDP/CDP link resolution complete"
        );

        Ok(stats)
    }

    /// Resolve FDB (bridge forwarding database) single-MAC ports to neighbor links.
    /// Called after resolve_lldp_links — only processes ports without LLDP/CDP data
    /// that have exactly one learned MAC address (direct physical connection).
    pub async fn resolve_fdb_links(&self, network_id: Uuid) -> Result<u32> {
        use crate::server::if_entries::r#impl::base::Neighbor;

        let resolver = LldpResolverImpl::new(
            self.if_entry_service.clone(),
            self.interface_service.clone(),
            self.storage.clone(),
        );

        let filter = StorableFilter::<IfEntry>::new_for_unresolved_fdb_in_network(network_id);
        let unresolved = self.if_entry_service.get_all(filter).await?;

        let mut resolved_count: u32 = 0;

        for mut if_entry in unresolved {
            let mac = match &if_entry.base.fdb_macs {
                Some(macs) if macs.len() == 1 => &macs[0],
                _ => continue,
            };

            // Try to find host by MAC
            let host_id = match resolver.find_host_by_mac(mac, network_id).await {
                Some(id) => id,
                None => continue,
            };

            // Try full resolution (specific port)
            let neighbor =
                if let Some(if_entry_id) = resolver.find_if_entry_by_mac(mac, host_id).await {
                    Neighbor::IfEntry(if_entry_id)
                } else {
                    Neighbor::Host(host_id)
                };

            if_entry.base.neighbor = Some(neighbor);
            self.if_entry_service
                .update(&mut if_entry, AuthenticatedEntity::System)
                .await?;
            resolved_count += 1;
        }

        if resolved_count > 0 {
            tracing::info!(
                network_id = %network_id,
                resolved = resolved_count,
                "FDB link resolution complete"
            );
        }

        Ok(resolved_count)
    }

    /// Delete a host (children cascade via FK)
    pub async fn delete_host(&self, id: &Uuid, authentication: AuthenticatedEntity) -> Result<()> {
        // Can't delete host with daemon
        if self
            .daemon_service
            .get_one(StorableFilter::<Daemon>::new_from_host_ids(&[*id]))
            .await?
            .is_some()
        {
            return Err(ValidationError::new(
                "Can't delete a host with an associated daemon. Delete the daemon first.",
            )
            .into());
        }

        let host = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Host {} not found", id))?;

        let lock = self.get_host_lock(id).await;
        let _guard = lock.lock().await;

        // Remove tags from junction table
        if let Some(tag_service) = self.entity_tag_service() {
            tag_service
                .remove_all_for_entity(*id, EntityDiscriminants::Host)
                .await?;
        }

        // Delete host - children cascade via ON DELETE CASCADE
        self.storage().delete(id).await?;

        let trigger_stale = host.triggers_staleness(None);

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: host.id(),
                network_id: self.get_network_id(&host),
                organization_id: self.get_organization_id(&host),
                entity_type: host.into(),
                operation: EntityOperation::Deleted,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale
                }),

                authentication,
            })
            .await?;

        Ok(())
    }
}

/// Statistics from LLDP link resolution.
#[derive(Default, Debug)]
pub struct LldpResolutionStats {
    /// Total number of if_entries with unresolved LLDP data
    pub total: usize,
    /// Number of if_entries where remote host was resolved
    pub hosts_resolved: usize,
    /// Number of if_entries where remote port (if_entry) was resolved
    pub ports_resolved: usize,
}
