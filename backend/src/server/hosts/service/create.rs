//! Host creation from API requests and the create-with-children path.
use super::open_ports::{held_by_open_ports, port_claim};
use super::stale_ports::{overlaps, stored_container};
use super::*;
use crate::server::shared::attribution::AttributeSource;

use crate::server::subnets::service::Placement;

/// Decide whether the end-of-scan interface prune (delete interfaces no longer reported for a
/// host) should run for this upsert. It runs only when ALL of these hold:
///   - `interfaces_complete`: the incoming set is an authoritative, complete ifTable. A partial
///     SNMP walk cut short by timeout/error reports fewer interfaces than really exist; pruning
///     against it deletes live interfaces and their server-resolved L2 neighbors (GH #649).
///   - `persisted_count > 0`: a total SNMP failure / credential removal yields zero interfaces,
///     which means "none observed", not "all removed".
///   - `skipped_count == 0`: every reported interface actually persisted. If some were skipped,
///     what we hold is a subset of what the device reported, so it is no more authoritative than
///     a partial walk — same rule, different cause.
///
/// Returning false preserves existing interface rows — the safe direction (stale beats deleted).
pub(crate) fn should_prune_interfaces(
    interfaces_complete: bool,
    persisted_count: usize,
    skipped_count: usize,
) -> bool {
    interfaces_complete && persisted_count > 0 && skipped_count == 0
}

/// Resolve an incoming `virtualization_service_id` to a service that actually exists, or `None`.
///
/// The daemon mints a fresh UUID for every service it matches, so an owner reference arrives
/// naming an id that is either (a) already a live service, (b) one this submission is about to
/// remap onto an existing row, or (c) nothing at all — a service that was dropped during
/// conflict resolution, or a stale id from an earlier scan.
///
/// Case (c) used to be stored verbatim: harmless-looking, but it silently defeated the
/// container-bridge dedup, which is how GH #650 accumulated 353 subnet rows for 70 CIDRs. Now
/// the column carries a foreign key, so writing it would abort the entire host instead. Neither
/// is acceptable, hence: resolve if we can, otherwise degrade this one reference to `None` and
/// let the caller log it. One subnet without an owner beats a host that cannot be created.
pub(crate) fn resolve_owner_service_id(
    incoming: Option<Uuid>,
    remap: &std::collections::HashMap<Uuid, Uuid>,
    live_service_ids: &std::collections::HashSet<Uuid>,
) -> Option<Uuid> {
    let id = incoming?;
    if let Some(&remapped) = remap.get(&id) {
        return Some(remapped);
    }
    live_service_ids.contains(&id).then_some(id)
}

impl HostService {
    /// Whether `interface` is a port the forwarding-table pass recorded that a switch port on its
    /// site still learns. A device's own walk doesn't report such a port (a virtual MAC, a NIC the
    /// walk missed), so pruning it there would only see the next pass record it again; it goes
    /// once no switch learns its MAC.
    pub(crate) async fn still_learned_by_a_switch(&self, interface: &Interface) -> Result<bool> {
        if interface.base.interface_source != AttributeSource::ForwardingTable {
            return Ok(false);
        }
        let Some(mac) = &interface.base.mac_address else {
            return Ok(false);
        };
        let learning = self
            .interface_service
            .get_all(
                StorableFilter::<Interface>::new_from_site_ids(&[interface.base.site_id])
                    .live()
                    .fdb_macs_contain(&[mac.value().0.to_string().to_lowercase()]),
            )
            .await?;
        Ok(!learning.is_empty())
    }

    /// Keep a discovery-supplied hypervisor service only if it is a real service on the host's
    /// own site; otherwise drop it and say so.
    ///
    /// An integration that reports guests (Proxmox) creates the hypervisor's host first and reads
    /// the service id back from the create response, so a well-behaved daemon only ever sends a
    /// stored id. The site check is tenant isolation: the handler has already pinned the host
    /// to the daemon's site, and the hypervisor must sit on that same site. A daemon-minted
    /// id (never stored) or one from elsewhere degrades to `None` rather than reaching Postgres as
    /// a foreign-key failure that would abort the whole host.
    pub(crate) async fn accept_discovered_virtualization_service(&self, host: &mut Host) {
        let Some(id) = host.base.virtualization_service_id else {
            return;
        };
        let keep = match self.service_service.get_by_id(&id).await {
            Ok(Some(service)) => service.base.site_id == host.base.site_id,
            Ok(None) => false,
            Err(e) => {
                tracing::warn!(error = ?e, %id, "Could not look up a discovered hypervisor service");
                false
            }
        };
        if !keep {
            host.base.virtualization_service_id = None;
            tracing::warn!(
                host_name = %host.base.name,
                %id,
                "Discovery payload named a hypervisor service that is not a stored service on \
                 this host's site; ignoring it"
            );
        }
    }

    /// Reject an API-supplied virtualizing service that does not exist.
    ///
    /// Discovery's copy goes through `accept_discovered_virtualization_service` instead, which
    /// drops a bad id rather than failing the host. Left unchecked, a bad id reaches Postgres and
    /// comes back as an opaque foreign-key 500.
    pub(crate) async fn validate_virtualization_service(
        &self,
        virtualization_service_id: Option<Uuid>,
    ) -> Result<()> {
        let Some(id) = virtualization_service_id else {
            return Ok(());
        };
        if self.service_service.get_by_id(&id).await?.is_none() {
            return Err(ValidationError::new(format!(
                "virtualization_service_id {id} does not match any service"
            ))
            .into());
        }
        Ok(())
    }

    /// Why `interface_id` cannot be the interface that presents a host owned by
    /// `virtualization_service_id` on `site_id`, or `None` when it can.
    ///
    /// The interface must exist, sit on the host's site, and belong to the host that runs the
    /// owning service: a network identity is presented by its own guest, never by some other host.
    async fn virtualization_interface_problem(
        &self,
        site_id: Uuid,
        virtualization_service_id: Option<Uuid>,
        interface_id: Uuid,
    ) -> Result<Option<String>> {
        let Some(interface) = self.interface_service.get_by_id(&interface_id).await? else {
            return Ok(Some(format!(
                "virtualization_interface_id {interface_id} does not match any interface"
            )));
        };
        if interface.base.site_id != site_id {
            return Ok(Some(format!(
                "virtualization_interface_id {interface_id} is on another site"
            )));
        }
        let owner_host = match virtualization_service_id {
            Some(id) => self
                .service_service
                .get_by_id(&id)
                .await?
                .map(|s| s.base.host_id),
            None => None,
        };
        if owner_host != Some(interface.base.host_id) {
            return Ok(Some(format!(
                "virtualization_interface_id {interface_id} is not an interface of the host running \
                 the virtualizing service"
            )));
        }
        Ok(None)
    }

    /// Keep a discovery-supplied presenting interface only if it is valid for this host's owner;
    /// otherwise drop it and say so. Runs after [`Self::accept_discovered_virtualization_service`],
    /// so the owner it checks against has already been vetted.
    pub(crate) async fn accept_discovered_virtualization_interface(&self, host: &mut Host) {
        let Some(id) = host.base.virtualization_interface_id else {
            return;
        };
        let problem = match self
            .virtualization_interface_problem(
                host.base.site_id,
                host.base.virtualization_service_id,
                id,
            )
            .await
        {
            Ok(problem) => problem,
            Err(e) => Some(format!("could not look it up: {e}")),
        };
        if let Some(problem) = problem {
            host.base.virtualization_interface_id = None;
            tracing::warn!(
                host_name = %host.base.name,
                %id,
                %problem,
                "Discovery payload named a presenting interface it cannot use; ignoring it"
            );
        }
    }

    /// Reject an API-supplied presenting interface that is not valid for the host's owner, the
    /// API counterpart of [`Self::accept_discovered_virtualization_interface`].
    pub(crate) async fn validate_virtualization_interface(
        &self,
        site_id: Uuid,
        virtualization_service_id: Option<Uuid>,
        virtualization_interface_id: Option<Uuid>,
    ) -> Result<()> {
        let Some(id) = virtualization_interface_id else {
            return Ok(());
        };
        if let Some(problem) = self
            .virtualization_interface_problem(site_id, virtualization_service_id, id)
            .await?
        {
            return Err(ValidationError::new(problem).into());
        }
        Ok(())
    }

    // =========================================================================
    // Host creation with children
    // =========================================================================

    /// Create a host with all its children (ip_addresses, ports, services, interfaces) from API request.
    /// Client provides UUIDs for all entities, enabling services to reference ip_addresses/ports.
    /// For API users: errors if a host with matching ip_addresses already exists.
    pub async fn create_from_request(
        &self,
        request: CreateHostRequest,
        authentication: AuthenticatedEntity,
    ) -> Result<HostResponse> {
        // Destructure request to ensure compile error if fields change
        let CreateHostRequest {
            name,
            site_id,
            hostname,
            description,
            virtualization_metadata,
            virtualization_service_id,
            virtualization_interface_id,
            tags,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            credential_assignments,
            ip_addresses: ip_address_inputs,
            ports: port_inputs,
            services: service_inputs,
            interfaces: interface_inputs,
        } = request;

        // Resolve and validate positions (no existing entities for create)
        let empty_ip_addresses: Vec<IPAddress> = vec![];
        let empty_services: Vec<Service> = vec![];
        let mut ip_address_inputs = ip_address_inputs;
        let mut service_inputs = service_inputs;
        resolve_and_validate_input_positions(
            &mut ip_address_inputs,
            &empty_ip_addresses,
            "ip_address",
        )
        .map_err(|e| ValidationError::new(e.message))?;
        resolve_and_validate_input_positions(&mut service_inputs, &empty_services, "service")
            .map_err(|e| ValidationError::new(e.message))?;

        // The virtualizing service must exist. Without this the insert fails
        // `hosts_virtualization_service_id_fkey` and surfaces as a 500, when the caller sent a
        // bad id and deserves to be told so.
        self.validate_virtualization_service(virtualization_service_id)
            .await?;
        self.validate_virtualization_interface(
            site_id,
            virtualization_service_id,
            virtualization_interface_id,
        )
        .await?;

        // Auto-set source to Manual for API-created entities
        let source = EntitySource::Manual;

        // Create host base with SNMP fields. The name is applied below rather than assigned
        // here: a person typed it, and that is the top of the ladder.
        let mut host_base = HostBase {
            name: HostName::unnamed(),
            site_id,
            hostname: hostname
                .filter(|v| !v.trim().is_empty())
                .map(|v| Attributed::new(HostHostnameValue(v), AttributeSource::Manual)),
            description,
            source: source.clone(),
            virtualization_metadata,
            virtualization_service_id,
            virtualization_interface_id,
            tags,
            // A person filling in the create form is the definition of `Manual`: nothing discovery
            // reads may displace what they typed.
            sys_descr: sys_descr
                .map(|v| Attributed::new(HostSysDescrValue(v), AttributeSource::Manual)),
            sys_object_id: sys_object_id
                .map(|v| Attributed::new(HostSysObjectIdValue(v), AttributeSource::Manual)),
            sys_location: sys_location
                .map(|v| Attributed::new(HostSysLocationValue(v), AttributeSource::Manual)),
            sys_contact: sys_contact
                .map(|v| Attributed::new(HostSysContactValue(v), AttributeSource::Manual)),
            management_url: management_url
                .map(|v| Attributed::new(HostManagementUrlValue(v), AttributeSource::Manual)),
            chassis_id: chassis_id
                .map(|v| Attributed::new(HostChassisIdValue(v), AttributeSource::Manual)),
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            asset_tag: None,
            firmware_revision: None,
            software_revision: None,
            os: None,
            matched_device_type: None,
            credential_assignments,
        };
        host_base.apply_name(HostName::manual(name.clone()));
        let host = Host::new(host_base);

        // Build ip_addresses with client-provided IDs
        let ip_addresses: Vec<IPAddress> = ip_address_inputs
            .into_iter()
            .map(|input| input.into_ip_address(host.id, site_id))
            .collect();

        // Build ports with client-provided IDs
        let ports: Vec<Port> = port_inputs
            .into_iter()
            .map(|input| input.into_port(host.id, site_id))
            .collect();

        // Build services with client-provided IDs
        let services: Vec<Service> = service_inputs
            .into_iter()
            .map(|input| input.into_service(host.id, site_id, source.clone()))
            .collect();

        // Build interfaces (server assigns UUIDs)
        let interfaces: Vec<Interface> = interface_inputs
            .into_iter()
            .map(|input| input.into_interface(host.id, site_id))
            .collect();

        // Use unified creation with Error behavior for API users
        self.create_with_children(
            host,
            ip_addresses,
            ports,
            services,
            interfaces,
            vec![], // No integration-derived subnets for API creates
            ConflictBehavior::Error,
            authentication,
            None, // limit checked in handler
            // A manual API create provides the full, authoritative interface list...
            true,
            // ...and carries no SNMP-derived neighbour data, so there is nothing to preserve.
            InterfaceDataComplete::default(),
            &crate::server::ports::r#impl::coverage::PortCoverage::default(),
        )
        .await
        .map(|created| created.host)
    }

    /// Create-or-upsert body of `CrudService::create`, WITHOUT the
    /// `HostDedup` advisory lock. Only two callers:
    /// - `CrudService::create` (mod.rs), which wraps it in the lock;
    /// - `create_with_children`, which holds the same lock across the wider
    ///   IP/MAC-match → create → IP-insertion window (re-acquiring here on a
    ///   second connection would self-deadlock).
    ///
    /// `matched_by` is how `create_with_children` matched an existing host before aligning the
    /// id; `None` when the caller sent an id that is already the host's.
    pub(crate) async fn create_unlocked(
        &self,
        host: Host,
        authentication: AuthenticatedEntity,
        matched_by: Option<HostMatchedBy>,
    ) -> Result<Host> {
        let host = if host.id == Uuid::nil() {
            Host::new(host.base.clone())
        } else {
            host
        };

        tracing::trace!("Creating host {:?}", host);

        // SCD2: only live hosts are eligible for the natural-key match.
        let filter = StorableFilter::<Host>::new_from_site_ids(&[host.base.site_id]).live();
        let all_hosts = self.get_all(filter).await?;

        // Find existing host by ID (Host::eq only compares IDs)
        // For discovery, create_with_children already set host.id to the existing host's ID
        // if an IP-address match was found, so this will find the match
        let host_from_storage = match all_hosts.into_iter().find(|h| host.eq(h)) {
            // Upsert if both are discovery sources, or if IDs match exactly
            Some(existing_host)
                if (host.base.source.discriminant() == EntitySourceDiscriminants::Discovery
                    && existing_host.base.source.discriminant()
                        == EntitySourceDiscriminants::Discovery)
                    || host.id == existing_host.id =>
            {
                let matched_by = matched_by.unwrap_or(HostMatchedBy::DaemonReportedId);
                self.upsert_host(
                    existing_host,
                    host,
                    authentication,
                    UpdateReason::HostRediscovered { matched_by },
                )
                .await?
            }
            _ => {
                if let Some(existing_host) = self.get_by_id(&host.id).await? {
                    return Err(ValidationError::new(format!(
                        "Site mismatch: Daemon is trying to update host '{}' (id: {}) but cannot proceed. \
                        The host belongs to site {} while the daemon is assigned to site {}. \
                        To resolve this, either reassign the daemon to the correct site or delete the mismatched host.",
                        existing_host.base.name,
                        host.id,
                        existing_host.base.site_id,
                        host.base.site_id
                    )).into());
                }

                // SCD2 origin: this row is being inserted for the first
                // time. Stamp created_at + valid_from to the entity's
                // already-refreshed `last_seen_at` so all four temporal
                // columns line up at one canonical scan_time. See
                // `DiscoveryTracked::originate_scan_timestamps`.
                use crate::server::shared::storage::snapshot::DiscoveryTracked;
                let mut host = host;
                host.originate_scan_timestamps(host.last_seen_at);
                let created = self.storage().create(&host).await?;
                let trigger_stale = created.triggers_staleness(None);

                if let Some(scope) = EntityScope::from_ids(
                    created.id(),
                    created.clone().into(),
                    self.get_site_id(&created),
                    self.get_organization_id(&created),
                ) {
                    self.event_bus()
                        .publish(
                            Event::new(scope, EntityOperation::Created, authentication).with_flags(
                                EntityEventFlags {
                                    trigger_stale,
                                    ..Default::default()
                                },
                            ),
                        )
                        .await;
                }

                host
            }
        };

        Ok(host_from_storage)
    }

    /// Create a host with all children, handling conflicts according to behavior.
    /// This is the unified internal method used by both API and discovery paths.
    ///
    /// ## Host Deduplication Flow
    ///
    /// Host deduplication happens in two stages:
    ///
    /// 1. **IP-address-based matching** (this method): `find_matching_host_by_ip_addresses` compares
    ///    incoming IP addresses against existing hosts using MAC address or subnet+IP matching.
    ///    - For API users (ConflictBehavior::Error): Returns an error telling them to edit the existing host.
    ///    - For discovery (ConflictBehavior::Upsert): Sets `host.id = existing_host.id` so the
    ///      subsequent create() call will recognize this as an existing host.
    ///
    /// 2. **ID-based matching** (in `create_unlocked()`): Uses `Host::eq` which only compares
    ///    IDs. Since we set `host.id = existing_host.id` in step 1, it will find a match and
    ///    call `upsert_host()` to merge discovery data. Both stages run under one
    ///    `HostDedup` advisory lock held until children are persisted (see below).
    ///
    /// This two-stage approach means:
    /// - IP-address matching handles the "is this the same physical host?" question
    /// - ID matching handles the "should we upsert?" question (relies on ID being set correctly)
    /// - Discovery always upserts when IP addresses match, even if daemon reported a different host ID
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn create_with_children(
        &self,
        mut host: Host,
        ip_addresses: Vec<IPAddress>,
        ports: Vec<Port>,
        services: Vec<Service>,
        interfaces: Vec<Interface>,
        subnets: Vec<Subnet>,
        conflict_behavior: ConflictBehavior,
        authentication: AuthenticatedEntity,
        limit_ctx: Option<&HostLimitContext>,
        // Whether `interfaces` is an authoritative, complete ifTable. When false (a partial SNMP
        // walk), the stale-interface prune is skipped so a transient partial scan cannot delete
        // interfaces and their resolved L2 neighbors (GH #649).
        interfaces_complete: bool,
        interface_data_complete: InterfaceDataComplete,
        // What this scan probed and got an answer from. A stored port binding changes hands only
        // where it was covered; empty coverage (an API create, an older daemon) moves nothing.
        port_coverage: &crate::server::ports::r#impl::coverage::PortCoverage,
    ) -> Result<DiscoveredHost> {
        // For advancing `last_seen_at` on matched-existing child rows (see the upsert
        // branches below); mirrors how upsert_host refreshes the host's freshness signal.
        use crate::server::shared::storage::snapshot::DiscoveryTracked;

        // DB-level dedup lock, held from the IP/MAC natural-key match until
        // all children are persisted. The match reads the ip_addresses
        // table, so a competing submission of the same NEW device only
        // becomes visible once this one's host AND ip_addresses rows are
        // inserted — locking `create_unlocked` alone (ID-based check) cannot
        // catch two fresh-UUID submissions of one device. Error paths
        // release via Drop.
        let dedup_guard = self
            .storage()
            .session_lock(
                LockKey::HostDedup {
                    site_id: host.base.site_id,
                },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?;

        // Stage 1: IP-address-based collision detection
        // Compares MAC addresses and subnet+IP to find hosts that represent the same physical machine
        let matching_result = self
            .find_matching_host_by_ip_addresses(
                &host.base.site_id,
                &ip_addresses,
                &interfaces,
                host.base.chassis_id.as_ref().map(|c| c.value().0.as_str()),
            )
            .await?;

        // A container report matching a host that also holds another device's addresses takes its
        // own addresses to a new host instead of adopting that one (`rows_a_container_takes`).
        let mut detach_from: Option<(Host, HashSet<Uuid>)> = None;
        let matching_result = match matching_result {
            Some(m) if matches!(conflict_behavior, ConflictBehavior::Upsert) => {
                match m.rows_a_container_takes(
                    host.base.virtualization_metadata.as_ref(),
                    &ip_addresses,
                ) {
                    Some(rows) => {
                        detach_from = Some((m.host, rows));
                        None
                    }
                    None => Some(m),
                }
            }
            other => other,
        };

        // A guest whose hypervisor names its NICs gives up rows another NIC answered for, each to
        // a host of its own (`rows_a_guest_sheds`).
        let shed_devices: Vec<HashSet<Uuid>> = match &matching_result {
            Some(m) if matches!(conflict_behavior, ConflictBehavior::Upsert) => {
                m.rows_a_guest_sheds(&ip_addresses, &interfaces)
            }
            _ => Vec::new(),
        };

        // A daemon's report of its own host retires the addresses it no longer has
        // (`stale_self_reported_rows`).
        let retired_self_reported: HashSet<Uuid> = match &matching_result {
            Some(m) if matches!(conflict_behavior, ConflictBehavior::Upsert) => {
                let stale =
                    super::self_report::stale_self_reported_rows(&m.ip_addresses, &ip_addresses);
                if !stale.is_empty()
                    && self
                        .daemon_service
                        .exists(StorableFilter::<Daemon>::new_from_host_ids(&[m.host.id]))
                        .await?
                {
                    stale
                } else {
                    HashSet::new()
                }
            }
            _ => HashSet::new(),
        };

        let is_new_host = matching_result.is_none();
        let mut same_device: Vec<SameDevice> = Vec::new();
        let mut host_matched_by: Option<HostMatchedBy> = None;

        if let Some(HostMatch {
            host: existing_host,
            matched_by,
            same_device: proven,
            ..
        }) = matching_result
        {
            match conflict_behavior {
                ConflictBehavior::Error => {
                    // API users should edit the existing host rather than create a duplicate
                    return Err(ValidationError::new(format!(
                        "A host with matching IP addresses already exists: '{}' (id: {}). \
                         Edit the existing host instead of creating a new one.",
                        existing_host.base.name, existing_host.id
                    ))
                    .into());
                }
                ConflictBehavior::Upsert => {
                    // For discovery: align the incoming host ID with the existing host
                    // This ensures create() will match via Host::eq (which compares IDs)
                    // and trigger upsert_host() to merge discovery metadata
                    host.id = existing_host.id;
                    host_matched_by = Some(matched_by);
                    same_device = proven;
                }
            }
        }

        // A payload whose only identity is a MAC may not conjure a host on a weak claim.
        //
        // Everything with an address or a chassis id is untouched: those are the identities every
        // existing path carries, and this gate never sees them. What it does see is a device known
        // only at the link layer, where the two failure modes §6 separates both land on a row an
        // operator then has to disprove — a weak *value* mints a fresh host on every MAC rotation,
        // and a weak *provenance* mints a ghost for a device nothing has ever contacted, which is
        // GH #668 from the LLDP side.
        //
        // Discovery only. A person creating a host with no addresses is asserting it themselves,
        // which is the top of the ladder, not a claim to be graded.
        if is_new_host && host.base.source.is_from_discovery() {
            mac_identity::require_minting_identity(&host, &ip_addresses, &interfaces)?;
        }

        // Check host limit for new hosts (not upserts)
        if is_new_host && let Some(ctx) = limit_ctx {
            let current_hosts = self.live_host_count(ctx).await?;
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
        let original_ip_addresses = ip_addresses.clone();
        let original_ports = ports.clone();

        // Stage 2: Create or upsert host via ID matching
        // If host.id was set to an existing host's ID above, this will trigger upsert_host()
        // Unlocked variant: this task already holds the HostDedup lock.
        let mut created_host = self
            .create_unlocked(host, authentication.clone(), host_matched_by)
            .await?;

        // Before the address loop, so the moved rows are found on this host and refreshed in place.
        if let Some((from, rows)) = &detach_from {
            self.detach_addresses(from, &created_host, rows, &authentication)
                .await?;
        }
        // Each device shed is a new host, so the plan limit applies as it does to any other.
        let shed_devices = match limit_ctx {
            Some(ctx)
                if !shed_devices.is_empty()
                    && self.live_host_count(ctx).await? + shed_devices.len() as u64 > ctx.limit =>
            {
                tracing::warn!(
                    host_id = %created_host.id,
                    devices = shed_devices.len(),
                    "Host limit reached; leaving addresses its NICs do not carry on this guest"
                );
                Vec::new()
            }
            _ => shed_devices,
        };
        self.shed_guest_addresses(&created_host, &shed_devices, &authentication)
            .await?;
        self.retire_self_reported_addresses(&created_host, &retired_self_reported, &authentication)
            .await?;

        // Capture daemon interface ID → IP mapping before ip_addresses are consumed.
        // Used later to remap credential assignment ip_address_ids to server-assigned IDs.
        let daemon_ip_address_ips: Vec<(Uuid, IpAddr)> = ip_addresses
            .iter()
            .map(|i| (i.id, i.base.ip_address))
            .collect();

        // Order: ports → subnets → ip_addresses → services
        // Subnets before ip_addresses (FK: ip_addresses.subnet_id → subnets.id).
        // Interfaces before services (binding validation queries host ip_addresses).
        // Subnet virtualization.service_id needs the real (deduped) service ID, but
        // services aren't created yet. Solved by pre-computing service_id_remap from
        // existing_services_for_match before creating subnets.

        // Create ports with correct host_id
        // For Upsert: deduplicate by checking existing ports first
        // For Error: just create (will fail on duplicate constraint)
        let mut created_ports = Vec::new();
        // Matched-existing children are pushed unchanged below; collect them here to
        // advance their `last_seen_at` to this scan's time (they were still observed),
        // then persist in one write — otherwise a repeat scan freezes child freshness
        // even though the FK subscriber advances last_discovery_id.
        let mut ports_to_refresh: Vec<Port> = Vec::new();
        for port in ports {
            let port_with_host = port.with_host(created_host.id, created_host.base.site_id);

            if matches!(conflict_behavior, ConflictBehavior::Upsert) {
                // Check if port already exists by ID
                if let Some(mut existing_port) =
                    self.port_service.get_by_id(&port_with_host.id).await?
                {
                    // The id comes from the daemon, so it can name a row on another site.
                    if existing_port.base.site_id != created_host.base.site_id {
                        tracing::warn!(
                            port_id = %existing_port.id,
                            site_id = %created_host.base.site_id,
                            "Discovered port names a row on another site; dropping it"
                        );
                        continue;
                    }
                    existing_port.set_last_seen_at(port_with_host.last_seen_at);
                    existing_port.base.port_source = existing_port
                        .base
                        .port_source
                        .keep_or_heal(port_with_host.base.port_source);
                    ports_to_refresh.push(existing_port);
                    created_ports.push(existing_port);
                    continue;
                }

                // Check by unique constraint (host_id, port_number, protocol)
                let existing_ports = self.port_service.get_for_host(&created_host.id).await?;
                let port_config = port_with_host.base.port_type.config();
                if let Some(mut existing_port) = existing_ports.into_iter().find(|p| {
                    let existing_config = p.base.port_type.config();
                    existing_config.number == port_config.number
                        && existing_config.protocol == port_config.protocol
                }) {
                    existing_port.set_last_seen_at(port_with_host.last_seen_at);
                    existing_port.base.port_source = existing_port
                        .base
                        .port_source
                        .keep_or_heal(port_with_host.base.port_source);
                    ports_to_refresh.push(existing_port);
                    created_ports.push(existing_port);
                    continue;
                }
            }

            // SCD2 origin: dedup didn't match, so this row is being
            // inserted. Stamp created_at + valid_from to the entity's
            // already-refreshed `last_seen_at` so all four temporal
            // columns line up at one canonical scan_time.
            let mut port_with_host = port_with_host;
            port_with_host.originate_scan_timestamps(port_with_host.last_seen_at);
            let created = self
                .port_service
                .create(port_with_host, authentication.clone())
                .await?;
            created_ports.push(created);
        }
        // Persist the refreshed last_seen_at for matched ports (storage-level, so no
        // Updated event is fired for a no-op freshness bump — same as upsert_host).
        if !ports_to_refresh.is_empty() {
            self.port_service
                .storage()
                .update_many(&ports_to_refresh)
                .await?;
        }

        // Order: subnets → ip_addresses → services
        // Subnets before ip_addresses (FK constraint: ip_addresses.subnet_id → subnets.id).
        // Interfaces before services (binding validation queries host ip_addresses from DB).
        // Subnets need the real service_id for dedup, but services haven't been created yet.
        // Solution: pre-compute service_id_remap by matching incoming services against
        // existing ones, then apply it to subnet virtualization before creation.

        // Pre-fetch existing services for ID alignment and service_id pre-computation.
        // SCD2: only live services are candidates for natural-key match.
        let mut existing_services_for_match =
            if matches!(conflict_behavior, ConflictBehavior::Upsert) {
                self.service_service
                    .get_all(
                        StorableFilter::<Service>::new_from_host_ids(&[created_host.id]).live(),
                    )
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            };

        // Pre-compute service_id_remap: match incoming services to existing ones
        // using the same PartialEq logic (host_id + service_definition) that the
        // service creation loop uses for ID alignment. This lets us patch subnet
        // virtualization.service_id before creating subnets.
        let mut service_id_remap: std::collections::HashMap<Uuid, Uuid> =
            std::collections::HashMap::new();
        // Stored services this submission found again. They answered, so they keep their ports
        // whatever else claims them (see `stale_ports`).
        let mut rediscovered: HashSet<Uuid> = HashSet::new();
        for svc in &services {
            let mut probe = svc.clone();
            probe.base.host_id = created_host.id;
            if let Some(existing) = existing_services_for_match
                .iter()
                .find(|e| **e == probe)
                .or_else(|| stored_container(&existing_services_for_match, &probe))
            {
                rediscovered.insert(existing.id);
                if existing.id != svc.id {
                    service_id_remap.insert(svc.id, existing.id);
                }
            }
        }

        // --- Owner-service row phase -------------------------------------------------------
        //
        // Subnets are written before services, but a container-bridge subnet names the runtime
        // service that owns it, and that id is a UUID the daemon minted in memory. Writing the
        // subnet first therefore violates `subnets_virtualization_service_id_fkey` (GH #650).
        //
        // The `services` row references only `hosts` — it is *bindings*, a separate table, that
        // reference ports and ip_addresses. So the cycle is broken by materialising the owner's
        // row here, with no bindings, and letting the main service loop below attach them: it
        // natural-key matches this row and `upsert_service` merges bindings additively
        // (`services/service/upsert.rs`), so nothing is lost.
        //
        // Only services actually named as an owner are hoisted, and only non-generic ones.
        // `Service::eq` resolves a generic service by shared port bindings, so a binding-less
        // generic row would fail to match itself on the next scan and duplicate — the invariant
        // is asserted registry-wide by `a_virtualizer_definition_is_never_generic`.
        let owner_ids: std::collections::HashSet<Uuid> = subnets
            .iter()
            .filter_map(|s| s.base.virtualization_service_id)
            .collect();

        let mut live_service_ids: std::collections::HashSet<Uuid> =
            existing_services_for_match.iter().map(|s| s.id).collect();

        for svc in &services {
            if !owner_ids.contains(&svc.id)
                || ServiceDefinitionExt::is_generic(&svc.base.service_definition)
            {
                continue;
            }

            let mut probe = svc.clone();
            probe.base.host_id = created_host.id;
            probe.base.site_id = created_host.base.site_id;

            if let Some(existing) = existing_services_for_match.iter().find(|e| **e == probe) {
                // Already persisted by an earlier scan — nothing to insert, just make the id
                // resolvable for the subnet loop.
                if existing.id != svc.id {
                    service_id_remap.insert(svc.id, existing.id);
                }
                live_service_ids.insert(existing.id);
                continue;
            }

            // New this scan. Insert the bare row so the FK resolves; the main loop attaches its
            // bindings. `virtualization_service_id` is stripped because a container runtime can
            // itself name an owner, and that id may not be resolvable yet — the repoint pass
            // after the service loop sets it.
            let mut row = probe;
            row.base.bindings = Vec::new();
            row.base.virtualization_service_id = None;

            match self
                .service_service
                .create(row, authentication.clone())
                .await
            {
                Ok(created) => {
                    if created.id != svc.id {
                        service_id_remap.insert(svc.id, created.id);
                    }
                    live_service_ids.insert(created.id);
                    existing_services_for_match.push(created);
                }
                Err(e) => {
                    // Not fatal: the subnets naming it degrade to an unowned bridge rather than
                    // the whole host failing.
                    tracing::warn!(
                        host_id = %created_host.id,
                        service = %svc.base.service_definition.name(),
                        error = %e,
                        "Failed to pre-create container runtime service; bridge subnets it owns \
                         will be recorded without an owner"
                    );
                }
            }
        }

        // Create integration-derived subnets (e.g., Docker bridges)
        // Patch virtualization.service_id using the pre-computed remap
        let mut subnet_id_remap: std::collections::HashMap<Uuid, Uuid> =
            std::collections::HashMap::new();
        // Subnet ids known-good because this call just created (or deduped to) them.
        // An ip_address referencing one of these needs no dangling-subnet repair,
        // so the common path skips the live-subnet lookup entirely.
        let mut created_subnet_ids: std::collections::HashSet<Uuid> =
            std::collections::HashSet::new();
        // Subnets server-side placement filed an address into: the scan saw an address there.
        let mut placed_subnet_ids: std::collections::HashSet<Uuid> =
            std::collections::HashSet::new();
        for mut subnet in subnets {
            // Force the subnet onto the resolved host's site. The daemon's
            // discovery payload is only authenticated for its own site, but
            // subnets ride in the host request with a caller-supplied
            // site_id; without this a daemon could write subnets into any
            // site. Mirrors how ports/interfaces force host_id + site_id.
            subnet.base.site_id = created_host.base.site_id;

            subnet.base.virtualization_service_id = resolve_owner_service_id(
                subnet.base.virtualization_service_id,
                &service_id_remap,
                &live_service_ids,
            );
            let original_id = subnet.id;
            let created = self
                .subnet_service
                .create(subnet, authentication.clone())
                .await?;
            if created.id != original_id {
                subnet_id_remap.insert(original_id, created.id);
            }
            created_subnet_ids.insert(created.id);
        }

        // Create ip_addresses with correct host_id
        // For Upsert: deduplicate by checking existing ip_addresses first
        // Apply subnet_id_remap for virtualized subnets whose IDs changed during dedup

        // Count how many incoming ip_addresses share each MAC address.
        // Multiple incoming ip_addresses with the same MAC = VLAN sub-interfaces (or bridge/bond
        // members) sharing a parent's MAC. These are distinct ip_addresses and must not be
        // collapsed via MAC matching. A unique MAC (count == 1) indicates a standalone interface
        // that may have moved subnets (e.g., Docker container with DHCP, subnet reconfiguration).
        let incoming_mac_counts: HashMap<MacAddress, usize> = ip_addresses
            .iter()
            .filter_map(|i| mac_of(&i.base.mac_address))
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            });
        // Every address this payload reports. A stored row still holding one of them did not move,
        // so the MAC match below must not take it as the old home of a different address.
        let incoming_ips: HashSet<IpAddr> =
            ip_addresses.iter().map(|i| i.base.ip_address).collect();

        // Live subnets for this host's site, loaded lazily to repair any
        // ip_address whose subnet_id references no live subnet row — e.g. an old
        // daemon reporting the server-seeded loopback subnet under a UUID this
        // server never minted (see HostService::seed_loopback). Resolve-only:
        // never creates a subnet; on no CIDR match the insert error surfaces.
        let mut site_live_subnets: Option<Vec<Subnet>> = None;

        let mut created_ip_addresses = Vec::new();
        // The submitted id each entry of `created_ip_addresses` came from, pushed with it. An
        // address the loop drops pushes neither, so the pairing never shifts.
        let mut stored_from: Vec<Uuid> = Vec::new();
        // Matched-existing ip_addresses are pushed unchanged below; collect them to
        // advance last_seen_at (see the ports loop for rationale) and persist in one write.
        let mut ip_addresses_to_refresh: Vec<IPAddress> = Vec::new();
        // Rows that moved to a new address: a real change, written with an event per row.
        let mut ip_addresses_rehomed: Vec<IPAddress> = Vec::new();
        for mut ip_address in ip_addresses {
            let original_id = ip_address.id;
            ip_address.base.host_id = created_host.id;
            // Force onto the host's site (see subnet loop above): the payload
            // site_id is caller-supplied and must not be trusted.
            ip_address.base.site_id = created_host.base.site_id;

            // Remap subnet_id if the subnet was deduped to an existing one
            if let Some(&new_subnet_id) = subnet_id_remap.get(&ip_address.base.subnet_id) {
                ip_address.base.subnet_id = new_subnet_id;
            }

            // Repair a dangling subnet_id (one referencing no live subnet row) by
            // mapping the IP to the most-specific live subnet on the site that
            // contains it. Guards the ip_addresses.subnet_id -> subnets FK against
            // stale references (e.g. an old daemon reporting the server-seeded
            // loopback subnet under a UUID this server re-minted; see seed_loopback).
            //
            // A subnet_id this call just created/deduped is known-good, so the
            // common host (every IP references an in-request subnet) skips the
            // live-subnet lookup entirely. The lookup is lazy and loads once.
            if !created_subnet_ids.contains(&ip_address.base.subnet_id) {
                if site_live_subnets.is_none() {
                    site_live_subnets = Some(
                        self.subnet_service
                            .get_all(
                                StorableFilter::<Subnet>::new_from_site_ids(&[created_host
                                    .base
                                    .site_id])
                                .live(),
                            )
                            .await?,
                    );
                }
                let live_subnets = site_live_subnets.as_ref().expect("loaded above");

                // A daemon that has not upgraded yet still picks the subnet itself, and picks
                // wrongly: its list includes the `0.0.0.0/0` organizational rows, which contain
                // every IPv4 address, so it stamps whichever it saw first. Re-placing such a row
                // is scoped to discovery payloads — a person filing a host under Internet or
                // Remote through the API is making a deliberate choice and is never second-guessed.
                let names_a_catch_all = matches!(conflict_behavior, ConflictBehavior::Upsert)
                    && live_subnets
                        .iter()
                        .any(|s| s.id == ip_address.base.subnet_id && s.is_organizational_subnet());
                // A subnet id that is deleted, or on another site, fails the foreign key or files
                // the address on another tenant's subnet if it reaches the insert.
                let names_a_live_subnet = live_subnets
                    .iter()
                    .any(|s| s.id == ip_address.base.subnet_id);

                if needs_placement(live_subnets, &ip_address) || names_a_catch_all {
                    match self
                        .subnet_service
                        .place_address(created_host.base.site_id, ip_address.base.ip_address)
                        .await?
                    {
                        Placement::Existing(subnet_id) | Placement::Inferred(subnet_id) => {
                            tracing::debug!(
                                ip = %ip_address.base.ip_address,
                                from_subnet_id = %ip_address.base.subnet_id,
                                subnet_id = %subnet_id,
                                "Placed ip_address server-side"
                            );
                            ip_address.base.subnet_id = subnet_id;
                            placed_subnet_ids.insert(subnet_id);
                        }
                        // Loopback, a public address or IPv6 global unicast: no subnet of this
                        // site may hold it. A live subnet named for it stays as named.
                        Placement::Unplaceable if names_a_live_subnet => tracing::warn!(
                            ip = %ip_address.base.ip_address,
                            "No subnet holds this address and none may be inferred for it"
                        ),
                        // Named by a subnet id that is gone or on another site.
                        Placement::Unplaceable => match conflict_behavior {
                            // A daemon learned the id earlier (a subnet POST, the loopback seeded
                            // at registration) and the subnet was deleted since. The address is
                            // dropped, as a rescan would not send it, and the rest of the host
                            // lands. Same for an id on another site.
                            ConflictBehavior::Upsert => {
                                tracing::debug!(
                                    ip = %ip_address.base.ip_address,
                                    subnet_id = %ip_address.base.subnet_id,
                                    "Discovered address names no subnet on this site and none \
                                     holds it; dropping the address"
                                );
                                continue;
                            }
                            ConflictBehavior::Error => {
                                return Err(ValidationError::new(format!(
                                    "subnet_id {} names no subnet on this host's site, and no \
                                     subnet holds {}",
                                    ip_address.base.subnet_id, ip_address.base.ip_address
                                ))
                                .into());
                            }
                        },
                    }
                }
            }

            if matches!(conflict_behavior, ConflictBehavior::Upsert) {
                // Each match below writes the stored row back, so the incoming MAC is merged into
                // it by rung first. Without that the reading was dropped whole: an ARP reply could
                // neither fill an address stored without a MAC nor lift one stored `Unspecified`
                // (GH #718).
                //
                // Check if interface already exists by ID
                if let Some(mut existing_iface) =
                    self.ip_address_service.get_by_id(&ip_address.id).await?
                {
                    // The id comes from the daemon, so it can name a row on another site.
                    if existing_iface.base.site_id != created_host.base.site_id {
                        tracing::warn!(
                            ip_address_id = %existing_iface.id,
                            site_id = %created_host.base.site_id,
                            "Discovered address names a row on another site; dropping it"
                        );
                        continue;
                    }
                    existing_iface.base.merge_discovered(&ip_address.base);
                    existing_iface.set_last_seen_at(ip_address.last_seen_at);
                    ip_addresses_to_refresh.push(existing_iface.clone());
                    stored_from.push(original_id);
                    created_ip_addresses.push(existing_iface);
                    continue;
                }

                // Check by unique constraint (host_id, subnet_id, ip_address).
                // SCD2: only live rows; the partial unique index is also
                // `WHERE valid_to IS NULL`, so this matches index semantics.
                let filter =
                    StorableFilter::<IPAddress>::new_from_host_ids(&[ip_address.base.host_id])
                        .subnet_id(&ip_address.base.subnet_id)
                        .live();
                let existing_by_key: Vec<IPAddress> =
                    self.ip_address_service.get_all(filter).await?;
                if let Some(mut existing_iface) = existing_by_key
                    .into_iter()
                    .find(|i| i.base.ip_address == ip_address.base.ip_address)
                {
                    existing_iface.base.merge_discovered(&ip_address.base);
                    existing_iface.set_last_seen_at(ip_address.last_seen_at);
                    ip_addresses_to_refresh.push(existing_iface.clone());
                    stored_from.push(original_id);
                    created_ip_addresses.push(existing_iface);
                    continue;
                }

                // MAC fallback: find by (host_id, mac_address) when subnet differs.
                // Designed for the case where an interface moved between subnets across
                // discovery runs (e.g., Docker container with DHCP, subnet reconfiguration).
                //
                // Dual guard to prevent VLAN sub-interface collapse:
                // - incoming_mac_counts == 1: this MAC is unique in the incoming batch,
                //   so it's a standalone ip_address, not a VLAN sub-interface
                // - existing_by_mac.len() == 1: only one existing interface has this MAC,
                //   so there's an unambiguous 1:1 match (not a N:1 VLAN consolidation)
                if let Some(mac) = mac_of(&ip_address.base.mac_address)
                    && incoming_mac_counts.get(&mac).copied().unwrap_or(0) == 1
                {
                    let mac_filter =
                        StorableFilter::<IPAddress>::new_from_host_ids(&[ip_address.base.host_id])
                            .mac_address(&mac)
                            .live();
                    let existing_by_mac: Vec<IPAddress> =
                        self.ip_address_service.get_all(mac_filter).await?;
                    // A row the payload still reports at its own address is that address, not this
                    // one before a move: a host with one NIC per address has each answer for the
                    // other's MAC under ARP flux, and matching on it drops the new address.
                    let still_reported = existing_by_mac
                        .first()
                        .is_some_and(|row| incoming_ips.contains(&row.base.ip_address));
                    if existing_by_mac.len() == 1 && !still_reported {
                        // Needed to ask whether the old subnet still holds the new address.
                        if site_live_subnets.is_none() {
                            site_live_subnets = Some(
                                self.subnet_service
                                    .get_all(
                                        StorableFilter::<Subnet>::new_from_site_ids(&[
                                            created_host.base.site_id,
                                        ])
                                        .live(),
                                    )
                                    .await?,
                            );
                        }
                        let live_subnets_for_move =
                            site_live_subnets.as_ref().expect("loaded above");

                        let mut existing_iface = existing_by_mac.into_iter().next().unwrap();
                        // Only a NIC re-homed to a subnet that no longer holds its old address is
                        // the same row. Within one range, the same MAC at a different address is
                        // ambiguous (a DHCP renewal, a second NIC answering under ARP flux, a
                        // secondary IP), and folding it in discarded the reported address; it is
                        // recorded as its own row below instead. Keeping a stale renewal address
                        // beside the new one is the cost, where folding lost the live one.
                        if !interface_moved(&existing_iface, &ip_address, live_subnets_for_move) {
                            tracing::debug!(
                                interface_ip = %ip_address.base.ip_address,
                                interface_mac = %mac,
                                existing_ip = %existing_iface.base.ip_address,
                                "Same MAC at another address in range; recording it as its own row"
                            );
                        } else {
                            existing_iface.base.ip_address = ip_address.base.ip_address;
                            existing_iface.base.subnet_id = ip_address.base.subnet_id;
                            existing_iface.base.merge_discovered(&ip_address.base);
                            existing_iface.set_last_seen_at(ip_address.last_seen_at);
                            ip_addresses_rehomed.push(existing_iface.clone());
                            stored_from.push(original_id);
                            created_ip_addresses.push(existing_iface);
                            continue;
                        }
                    }
                }
            }

            // SCD2 origin: dedup didn't match, so this row is being
            // inserted. Stamp created_at + valid_from to the entity's
            // already-refreshed `last_seen_at`.
            let mut ip_address = ip_address;
            ip_address.originate_scan_timestamps(ip_address.last_seen_at);
            let created = self
                .ip_address_service
                .create(ip_address, authentication.clone())
                .await?;
            stored_from.push(original_id);
            created_ip_addresses.push(created);
        }
        // Persist refreshed last_seen_at for matched ip_addresses (storage-level, no event).
        if !ip_addresses_to_refresh.is_empty() {
            self.ip_address_service
                .storage()
                .update_many(&ip_addresses_to_refresh)
                .await?;
        }
        // Re-homed rows: their updates log the old and new address.
        self.ip_address_service
            .update_many_base(
                &ip_addresses_rehomed,
                authentication.clone(),
                UpdateReason::IpAddressRehomed,
            )
            .await?;

        // Scanner→DB ip_address id mapping. A dropped address has no entry, so a binding naming
        // it is dropped or widened by `reassign_service_interface_bindings`.
        let ip_address_id_remap: std::collections::HashMap<Uuid, Uuid> = stored_from
            .iter()
            .zip(created_ip_addresses.iter())
            .filter(|(orig, created)| **orig != created.id)
            .map(|(orig, created)| (*orig, created.id))
            .collect();

        // Create services with bindings reassigned (for discovery where IDs may change)
        // Track claimed bindings in this batch to detect in-batch conflicts
        let mut batch_claimed: Vec<(Uuid, Option<Uuid>)> = Vec::new();
        // Collect orphaned bindings from dropped services to assign to OpenPorts
        let mut orphaned_bindings: Vec<Binding> = Vec::new();
        let mut created_services = Vec::new();
        let mut turnover = self
            .port_turnover(&created_host.id, port_coverage, rediscovered)
            .await?;

        for service in services {
            let mut reassigned = self
                .service_service
                .reassign_service_interface_bindings(
                    service,
                    &original_host,
                    &original_ip_addresses,
                    &original_ports,
                    &created_host,
                    &created_ip_addresses,
                    &created_ports,
                    &ip_address_id_remap,
                )
                .await;

            // Align service ID with existing match so conflict check excludes its bindings
            let original_service_id = reassigned.id;
            if let Some(existing) = existing_services_for_match
                .iter()
                .find(|e| **e == reassigned)
                .or_else(|| stored_container(&existing_services_for_match, &reassigned))
            {
                reassigned.id = existing.id;
            }

            // Track service ID remapping for subnet virtualization patching
            if reassigned.id != original_service_id {
                service_id_remap.insert(original_service_id, reassigned.id);
            }

            // Resolve the owner to its stored id now, not just before the insert: the conflict
            // handling below compares it against the host's stored services, and the daemon's
            // minted id matches none of them.
            reassigned.base.virtualization_service_id = resolve_owner_service_id(
                reassigned.base.virtualization_service_id,
                &service_id_remap,
                &live_service_ids,
            );

            // A container now identified takes over the generic placeholder stored for it, before
            // the placeholder's bindings count as conflicts.
            let absorbed = self
                .absorb_container_placeholders(
                    &turnover,
                    &mut reassigned,
                    &mut existing_services_for_match,
                    authentication.clone(),
                )
                .await?;

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

                // Services a generic container could be reconciled onto: claiming a conflicting
                // port, and either not virtualized or already carrying this container (a rescan).
                // Port-only matches such as PostgreSQL count; Unclaimed Open Ports does not, its
                // ports are reclaimed instead. Never the container's own runtime: a container
                // publishing the port its runtime is reached on (an API proxy) would otherwise
                // become the runtime's identity, and the runtime its own container. Cloned
                // because later branches mutate existing_services_for_match.
                let conflicting_port_ids: Vec<Uuid> = conflicting_bindings
                    .iter()
                    .filter_map(|b| b.port_id())
                    .collect();
                let container_id = reassigned
                    .base
                    .virtualization_metadata
                    .as_ref()
                    .and_then(|v| v.container_id());
                let enrichable_services: Vec<Service> = existing_services_for_match
                    .iter()
                    .filter(|s| {
                        Some(s.id) != reassigned.base.virtualization_service_id
                            && !ServiceDefinitionExt::is_open_ports(&s.base.service_definition)
                            && s.base.virtualization_metadata.as_ref().is_none_or(|v| {
                                container_id.is_some() && v.container_id() == container_id
                            })
                            && s.base.bindings.iter().any(|b| {
                                b.port_id()
                                    .is_some_and(|pid| conflicting_port_ids.contains(&pid))
                            })
                    })
                    .cloned()
                    .collect();

                if matches_existing_service {
                    // Conflicts with Unclaimed Open Ports are taken back, and so are covered ports
                    // whose holders are no longer there (`stale_ports`). Any other conflict leaves
                    // the port with its owner.
                    let (handed_over, conflicting_bindings): (Vec<Binding>, Vec<Binding>) =
                        conflicting_bindings.into_iter().partition(|b| {
                            port_claim(b).is_some_and(|claim| {
                                !held_by_open_ports(&claim, &existing_services_for_match)
                                    && turnover
                                        .holders_handing_over(
                                            &[claim],
                                            &existing_services_for_match,
                                            &reassigned,
                                        )
                                        .is_some_and(|holders| !holders.is_empty())
                            })
                        });
                    let handed_over_claims: Vec<(Uuid, Option<Uuid>)> =
                        handed_over.iter().filter_map(port_claim).collect();
                    if let Some(holders) = turnover.holders_handing_over(
                        &handed_over_claims,
                        &existing_services_for_match,
                        &reassigned,
                    ) && !holders.is_empty()
                    {
                        self.take_ports(
                            &holders,
                            &handed_over_claims,
                            &mut existing_services_for_match,
                            handed_over_to(&reassigned),
                            authentication.clone(),
                        )
                        .await?;
                    }
                    let mut valid_bindings = valid_bindings;
                    valid_bindings.extend(handed_over);
                    let (reclaimed, _dropped): (Vec<Binding>, Vec<Binding>) =
                        if ServiceDefinitionExt::is_open_ports(&reassigned.base.service_definition)
                        {
                            (vec![], conflicting_bindings)
                        } else {
                            conflicting_bindings.into_iter().partition(|b| {
                                port_claim(b).is_some_and(|claim| {
                                    held_by_open_ports(&claim, &existing_services_for_match)
                                })
                            })
                        };
                    let claims: Vec<(Uuid, Option<Uuid>)> =
                        reclaimed.iter().filter_map(port_claim).collect();
                    if !claims.is_empty() {
                        self.reclaim_from_open_ports(
                            &claims,
                            &mut existing_services_for_match,
                            &reassigned,
                            authentication.clone(),
                        )
                        .await;
                    }
                    reassigned.base.bindings = valid_bindings;
                    reassigned.base.bindings.extend(reclaimed);
                } else if reassigned.base.virtualization_metadata.is_some()
                    && ServiceDefinitionExt::is_generic(&reassigned.base.service_definition)
                    && !enrichable_services.is_empty()
                {
                    // Safety net for Docker container → specific service reconciliation.
                    //
                    // When the Docker scan can't identify a container's specific service
                    // (e.g., exec-based and external endpoint probing both fail to match),
                    // it creates a generic "Docker Container" service. This conflicts with
                    // the specific service already found by the network scan (same port).
                    //
                    // Rather than dropping the Docker Container and losing its virtualization
                    // metadata (container name, container ID, Docker daemon linkage), we set
                    // the Docker virtualization on the specific service directly. The network
                    // scan already identified the service; we're just adding the container
                    // metadata. When one service takes the container, it also takes the
                    // container's other bindings (its loopback and bridge addresses), which would
                    // otherwise be lost with the container. A container with no such service
                    // falls through, so one whose ports sit only in Unclaimed Open Ports reclaims
                    // them below (GH #784).
                    let takes_other_bindings = enrichable_services.len() == 1;
                    for mut updated in enrichable_services {
                        updated.base.virtualization_metadata =
                            reassigned.base.virtualization_metadata.clone();
                        updated.base.virtualization_service_id =
                            reassigned.base.virtualization_service_id;
                        if takes_other_bindings {
                            updated.base.bindings.extend(valid_bindings.iter().cloned());
                        }
                        // Mirror the stored result so later services in the payload see it.
                        if let Ok(saved) = self
                            .service_service
                            .update(
                                &mut updated,
                                authentication.clone(),
                                UpdateReason::ServiceContainerAdopted,
                            )
                            .await
                            && let Some(stored) = existing_services_for_match
                                .iter_mut()
                                .find(|s| s.id == saved.id)
                        {
                            *stored = saved;
                        }
                    }
                    continue;
                } else if let Some(owner_id) = reassigned.base.virtualization_service_id
                    && let Some(owner) = existing_services_for_match
                        .iter()
                        .find(|s| s.id == owner_id)
                    && conflicting_bindings.iter().all(|b| {
                        b.port_id().is_some_and(|pid| {
                            owner
                                .base
                                .bindings
                                .iter()
                                .any(|ob| ob.port_id() == Some(pid))
                        })
                    })
                {
                    // A container claiming ports its own runtime holds: the runtime is reached
                    // through the container (an API proxy publishing the port the credential
                    // connects to), so the container owns them and the runtime gives them up.
                    // Covers bindings the runtime kept from earlier scans, and older daemons.
                    let claims: Vec<(Uuid, Option<Uuid>)> = conflicting_bindings
                        .iter()
                        .filter_map(|b| match &b.base.binding_type {
                            BindingType::Port {
                                port_id,
                                ip_address_id,
                            } => Some((*port_id, *ip_address_id)),
                            BindingType::IPAddress { .. } => None,
                        })
                        .collect();
                    self.service_service
                        .remove_port_bindings(
                            &owner_id,
                            &claims,
                            authentication.clone(),
                            handed_over_to(&reassigned),
                        )
                        .await?;
                    if let Some(owner) = existing_services_for_match
                        .iter_mut()
                        .find(|s| s.id == owner_id)
                    {
                        owner.base.bindings.retain(|b| {
                            let Some(port_id) = b.port_id() else {
                                return true;
                            };
                            let bind_iface = b.ip_address_id();
                            !claims
                                .iter()
                                .any(|(cp, ci)| *cp == port_id && bindings_overlap(ci, &bind_iface))
                        });
                    }
                    let mut full_bindings = valid_bindings;
                    full_bindings.extend(conflicting_bindings);
                    reassigned.base.bindings = full_bindings;
                } else {
                    // Check if all conflicts are with the Unclaimed Open Ports service.
                    // When a new service definition is added and a host is re-scanned,
                    // the new service's ports conflict with OpenPorts from the prior scan.
                    // The specific service should reclaim those ports.
                    let conflicting_claims: Vec<(Uuid, Option<Uuid>)> =
                        conflicting_bindings.iter().filter_map(port_claim).collect();

                    // Each conflict is held by Unclaimed Open Ports, by services that hand the
                    // port over on a covered scan (`stale_ports`), or both. Anything else keeps
                    // the port and the new service is dropped.
                    let handing_over = turnover
                        .holders_handing_over(
                            &conflicting_claims,
                            &existing_services_for_match,
                            &reassigned,
                        )
                        .filter(|holders| {
                            !conflicting_claims.is_empty()
                                && conflicting_claims.iter().all(|claim| {
                                    held_by_open_ports(claim, &existing_services_for_match)
                                        || existing_services_for_match.iter().any(|s| {
                                            holders.contains(&s.id)
                                                && s.base
                                                    .bindings
                                                    .iter()
                                                    .any(|b| overlaps(b, claim))
                                        })
                                })
                        });

                    if let Some(holders) = handing_over {
                        // The daemon's OpenPorts upsert later in the batch sets the
                        // authoritative final state — this just clears DB conflicts
                        // so the new service can be created.
                        let open_ports_claims: Vec<(Uuid, Option<Uuid>)> = conflicting_claims
                            .iter()
                            .filter(|claim| held_by_open_ports(claim, &existing_services_for_match))
                            .copied()
                            .collect();
                        if !open_ports_claims.is_empty() {
                            self.reclaim_from_open_ports(
                                &open_ports_claims,
                                &mut existing_services_for_match,
                                &reassigned,
                                authentication.clone(),
                            )
                            .await;
                        }
                        if !holders.is_empty() {
                            self.take_ports(
                                &holders,
                                &conflicting_claims,
                                &mut existing_services_for_match,
                                handed_over_to(&reassigned),
                                authentication.clone(),
                            )
                            .await?;
                        }

                        // Restore full bindings on the incoming service
                        let mut full_bindings = valid_bindings;
                        full_bindings.extend(conflicting_bindings);
                        reassigned.base.bindings = full_bindings;

                    // Fall through to service creation below
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

                        // A dropped container's other bindings are its own (its bridge address,
                        // its loopback publish), not ports nobody claimed.
                        if reassigned.base.virtualization_metadata.is_none() {
                            orphaned_bindings.extend(valid_bindings);
                        }
                        continue;
                    }
                }
            }

            // Track this service's port bindings for in-batch conflict detection
            for binding in &reassigned.base.bindings {
                if let BindingType::Port {
                    port_id,
                    ip_address_id,
                } = &binding.base.binding_type
                {
                    batch_claimed.push((*port_id, *ip_address_id));
                }
            }

            // A container service names the runtime service that hosts it, and that reference
            // arrives as the daemon's freshly minted id. Resolve it against the remap and the
            // live set before the insert, or `services_virtualization_service_id_fkey` aborts
            // the whole host — the same failure the bridge subnets had (GH #650).
            reassigned.base.virtualization_service_id = resolve_owner_service_id(
                reassigned.base.virtualization_service_id,
                &service_id_remap,
                &live_service_ids,
            );
            // Aligned onto its own owner (a container answering as its runtime): a service never
            // runs inside itself, and the container details are the other container's.
            if reassigned.base.virtualization_service_id == Some(reassigned.id) {
                tracing::warn!(
                    service_name = %reassigned.base.name,
                    "Dropping container details whose owner is the service itself"
                );
                reassigned.base.virtualization_service_id = None;
                reassigned.base.virtualization_metadata = None;
            }

            let reassigned_id = reassigned.id;
            let created = self
                .service_service
                .create(reassigned, authentication.clone())
                .await?;
            // `create` can hand back a different id than it was given — a singleton upsert, or
            // Docker-Container reconciling onto an existing specific service. That hop was never
            // recorded anywhere, so anything referencing the old id (bridge subnets, container
            // service virtualization) was left pointing at nothing. Record it here, where it is
            // the only place the two ids are both in scope (GH #650).
            if created.id != reassigned_id {
                service_id_remap.insert(reassigned_id, created.id);
            }
            // Services later in this loop may name this one as their owner.
            live_service_ids.insert(created.id);
            // Add to existing_services_for_match so subsequent services in this batch
            // can find it for ID alignment and Docker Container → specific service reconciliation
            existing_services_for_match.push(created.clone());
            turnover.mark_present(created.id);
            self.finish_absorbing(absorbed, &created, authentication.clone())
                .await?;
            created_services.push(created);
        }

        // Covered ports nothing answered on, once stale, leave their holders (`stale_ports`).
        let answered: HashSet<Uuid> = created_ports.iter().map(|p| p.id).collect();
        self.release_silent_ports(
            &turnover,
            &created_host.base.site_id,
            &answered,
            &mut existing_services_for_match,
            authentication.clone(),
        )
        .await?;

        // If we have orphaned bindings, assign them to OpenPorts service
        if !orphaned_bindings.is_empty() {
            use crate::server::services::definitions::open_ports::OpenPorts as OpenPortsDef;
            use crate::server::services::r#impl::base::ServiceBase;

            let open_ports_service = Service::new(ServiceBase {
                host_id: created_host.id,
                site_id: created_host.base.site_id,
                service_definition: Box::new(OpenPortsDef),
                name: "Unclaimed Open Ports".to_string(),
                bindings: orphaned_bindings,
                virtualization_metadata: None,
                virtualization_service_id: None,
                source: EntitySource::Discovery,
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

        // Net for a service whose owner id only moved *after* it was inserted — a later
        // iteration reconciling onto an existing row, say. The common case is already handled
        // before the insert, so this rarely fires.
        //
        // Errors propagate rather than being discarded: this writes an FK-bearing column, and a
        // silent failure here is exactly the drift the column was introduced to make impossible.
        for svc in &created_services {
            let resolved = resolve_owner_service_id(
                svc.base.virtualization_service_id,
                &service_id_remap,
                &live_service_ids,
            );
            if resolved != svc.base.virtualization_service_id {
                let mut updated = svc.clone();
                updated.base.virtualization_service_id = resolved;
                self.service_service
                    .update(
                        &mut updated,
                        authentication.clone(),
                        UpdateReason::EntityMaintained,
                    )
                    .await?;
            }
        }

        // No after-the-fact repair for bridge subnets: the runtime services they name are
        // materialised before the subnet loop runs, so a bridge is written with a final owner id
        // rather than one that has to be chased afterwards.

        // Binding fixup: remap provisional daemon interface/port IDs to server-assigned IDs.
        // This handles the case where interface or port UUIDs changed during dedup (upsert).
        // Idempotent — no-op if no IDs changed.
        // Reuse the ip_address_id_remap built before service creation.
        // The old structural-matching approach (ip_address + subnet_id) failed on second
        // scan because original_ip_addresses retain scanner subnet_ids while created_ip_addresses
        // have DB subnet_ids.
        {
            let port_id_remap: std::collections::HashMap<Uuid, Uuid> = original_ports
                .iter()
                .filter_map(|orig| {
                    created_ports
                        .iter()
                        .find(|c| c.base.port_type == orig.base.port_type)
                        .and_then(|created| {
                            if created.id != orig.id {
                                Some((orig.id, created.id))
                            } else {
                                None
                            }
                        })
                })
                .collect();

            if !ip_address_id_remap.is_empty() || !port_id_remap.is_empty() {
                for svc in &created_services {
                    let needs_update = svc.base.bindings.iter().any(|b| {
                        b.ip_address_id()
                            .is_some_and(|id| ip_address_id_remap.contains_key(&id))
                            || b.port_id()
                                .is_some_and(|id| port_id_remap.contains_key(&id))
                    });
                    if needs_update {
                        let mut updated = svc.clone();
                        for binding in &mut updated.base.bindings {
                            match &mut binding.base.binding_type {
                                BindingType::IPAddress { ip_address_id } => {
                                    if let Some(&new_id) = ip_address_id_remap.get(ip_address_id) {
                                        *ip_address_id = new_id;
                                    }
                                }
                                BindingType::Port {
                                    port_id,
                                    ip_address_id,
                                } => {
                                    if let Some(&new_id) = port_id_remap.get(port_id) {
                                        *port_id = new_id;
                                    }
                                    if let Some(iface_id) = ip_address_id
                                        && let Some(&new_id) = ip_address_id_remap.get(iface_id)
                                    {
                                        *iface_id = new_id;
                                    }
                                }
                            }
                        }
                        let _ = self
                            .service_service
                            .update(
                                &mut updated,
                                authentication.clone(),
                                UpdateReason::EntityMaintained,
                            )
                            .await;
                    }
                }
            }
        }

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

        // Create interfaces with correct host_id
        // Uses create_or_update_from_discovery for tiered dedup on
        // (host_id, if_name) → (host_id, if_index) → (host_id, mac_address).
        //
        // `claimed` tracks rows already matched/created in this batch so two
        // incoming ifTable entries can't collapse onto one existing row — e.g. an
        // L2 switch whose IP-less ports all share the chassis MAC and lack ifName
        // would otherwise all tier-3 onto the management interface (issue #614).
        //
        // A single interface that fails validation (e.g. it collides with a unique index) is
        // skipped rather than propagated: `?` here would abandon the rest of the ifTable *and*
        // every child created after this loop, silently, on every scan. Only validation failures
        // are tolerated — anything else (a lost connection, a failed lock) is a systemic problem
        // and must still abort the host.
        let mut created_interfaces = Vec::new();
        let mut skipped_interfaces = 0usize;
        let mut claimed: HashSet<Uuid> = HashSet::new();
        // A native VLAN id comes from the daemon, which learned it from its VLAN batch. One deleted
        // since fails `interfaces_native_vlan_id_fkey`, and one on another site would link this
        // site's subnets to it. Both are cleared.
        let mut live_vlan_ids: Vec<Uuid> = interfaces
            .iter()
            .filter_map(|i| i.base.native_vlan_id)
            .collect::<HashSet<Uuid>>()
            .into_iter()
            .collect();
        self.vlan_service
            .retain_on_site(&mut live_vlan_ids, created_host.base.site_id)
            .await?;
        for mut entry in interfaces {
            entry.base.host_id = created_host.id;
            entry.base.site_id = created_host.base.site_id;
            if let Some(vlan_id) = entry.base.native_vlan_id
                && !live_vlan_ids.contains(&vlan_id)
            {
                tracing::debug!(
                    host_id = %created_host.id,
                    if_index = entry.base.if_index,
                    %vlan_id,
                    "Discovered interface names a native VLAN not on this site; clearing it"
                );
                entry.base.native_vlan_id = None;
            }
            // A daemon that knows which of its submitted addresses an interface carries names it by
            // the id it submitted: a gateway's per-network interfaces share one MAC, so MAC
            // matching (`link_interfaces_to_ip_addresses`) cannot pair them. Point the link at the
            // row the address landed on, and drop one that names no address of this host.
            entry.base.ip_address_id = entry.base.ip_address_id.and_then(|id| {
                let id = ip_address_id_remap.get(&id).copied().unwrap_or(id);
                created_ip_addresses
                    .iter()
                    .any(|ip| ip.id == id)
                    .then_some(id)
            });
            let if_index = entry.base.if_index;

            match self
                .interface_service
                .create_or_update_from_discovery(
                    entry,
                    &claimed,
                    interface_data_complete,
                    authentication.clone(),
                )
                .await
            {
                Ok(created) => {
                    claimed.insert(created.id);
                    created_interfaces.push(created);
                }
                Err(e) if e.downcast_ref::<ValidationError>().is_some() => {
                    skipped_interfaces += 1;
                    tracing::warn!(
                        host_id = %created_host.id,
                        if_index = if_index,
                        error = %e,
                        "Skipped an interface that failed validation; continuing with the rest of the ifTable"
                    );
                }
                Err(e) => return Err(e),
            }
        }

        // Full-sweep prune of interfaces no longer reported for this host.
        //
        // Only runs when the incoming interface set is an authoritative, COMPLETE ifTable
        // (`interfaces_complete`). An SNMP walk cut short by timeout/error returns a partial,
        // smaller-than-real ifTable (see daemon `walk_if_table`); pruning against it would delete
        // live interfaces and the server-resolved L2 neighbors on them, tearing switches off the
        // L2 topology map on every scan that hiccups (GH #649). Three guards, all required:
        //   - `interfaces_complete`: skip pruning on a partial walk (daemon-signalled).
        //   - non-empty set: a total SNMP failure / credential removal yields zero interfaces,
        //     which is "none observed", not "all removed".
        //   - nothing skipped: an interface that failed to persist means what we hold is a subset
        //     of what the device reported — as non-authoritative as a partial walk.
        if should_prune_interfaces(
            interfaces_complete,
            created_interfaces.len(),
            skipped_interfaces,
        ) {
            let kept_ids: HashSet<Uuid> = created_interfaces.iter().map(|i| i.id).collect();
            let existing = self
                .interface_service
                .get_for_host(&created_host.id)
                .await?;
            for iface in existing {
                if kept_ids.contains(&iface.id) || self.still_learned_by_a_switch(&iface).await? {
                    continue;
                }
                if let Err(e) = self
                    .interface_service
                    .delete(
                        &iface.id,
                        authentication.clone(),
                        DeletionReason::EntityNoLongerReported,
                    )
                    .await
                {
                    tracing::warn!(
                        host_id = %created_host.id,
                        interface_id = %iface.id,
                        error = %e,
                        "Failed to prune orphan interface"
                    );
                }
            }
        } else if !created_interfaces.is_empty() {
            // Non-empty incoming set that we chose NOT to prune against ⇒ the set isn't
            // authoritative: an incomplete (partial) walk, or an interface that failed to persist.
            // Surface it so a self-hosted operator can see why stale interfaces persist;
            // `interfaces_complete` and `skipped` distinguish which of the two guards fired.
            tracing::debug!(
                host_id = %created_host.id,
                interfaces_complete = interfaces_complete,
                skipped = skipped_interfaces,
                incoming = created_interfaces.len(),
                "Skipped interface prune: incoming ifTable is not authoritative (partial walk, or an interface failed to persist) — preserving existing interfaces and L2 links"
            );
        }

        // Remap credential assignment ip_address_ids from daemon UUIDs to server UUIDs
        // and persist to the host_credentials junction table.
        // credential_assignments is transient on HostBase (not stored in hosts table),
        // so we must persist via set_host_credentials and use the original input host's
        // assignments (created_host.base.credential_assignments is empty after DB round-trip).
        let mut remapped_assignments = original_host.base.credential_assignments.clone();
        for assignment in &mut remapped_assignments {
            remap_assignment_ip_ids(assignment, &daemon_ip_address_ips, &created_ip_addresses);
        }
        // The credential ids come from the payload. One deleted since fails the junction's foreign
        // key and takes every assignment of this write with it; one from another organization
        // would be stored as a cross-tenant assignment. Both are dropped.
        if !remapped_assignments.is_empty() {
            let organization_id = self
                .site_service
                .get_by_id(&created_host.base.site_id)
                .await?
                .map(|site| site.base.organization_id);
            let mut usable: Vec<Uuid> = remapped_assignments
                .iter()
                .map(|a| a.credential_id)
                .collect();
            match organization_id {
                Some(org_id) => {
                    self.credential_service
                        .retain_in_org(&mut usable, org_id)
                        .await?
                }
                None => usable.clear(),
            }
            remapped_assignments.retain(|assignment| {
                let keep = usable.contains(&assignment.credential_id);
                if !keep {
                    tracing::debug!(
                        host_id = %created_host.id,
                        credential_id = %assignment.credential_id,
                        "Credential assignment names no credential in this organization; dropping it"
                    );
                }
                keep
            });
        }
        // MERGE (not replace): discovery self-reports only credentials that probed successfully,
        // so replacing would prune user/init-assigned daemon-host creds that didn't probe this
        // scan. Merge adds the freshly-discovered assignments without deleting existing ones.
        if !remapped_assignments.is_empty()
            && let Err(e) = self
                .credential_service
                .merge_host_credentials(&created_host.id, &remapped_assignments)
                .await
        {
            tracing::warn!(
                host_id = %created_host.id,
                error = ?e,
                "Failed to persist credential assignments during discover_host"
            );
        }
        created_host.base.credential_assignments = remapped_assignments;

        // Hosts this payload proves are the same device (`hosts_proven_same_device`) merge into the
        // one it matched, now that its children are written, and still under the dedup lock so no
        // other submission can match the merged-away host halfway through.
        let mut merged_response = None;
        for proven in same_device {
            match self
                .merge_same_device(&created_host.id, &proven, &authentication)
                .await
            {
                Ok(response) => merged_response = Some(response),
                Err(e) => tracing::warn!(
                    host_id = %created_host.id,
                    other_host_id = %proven.host_id,
                    error = ?e,
                    "Could not merge a host this payload proves is the same device; leaving both"
                ),
            }
        }

        dedup_guard.release().await?;

        // The subnets were stored before any merge, so their ids hold either way.
        let subnet_ids = created_subnet_ids
            .union(&placed_subnet_ids)
            .copied()
            .collect();
        let host = match merged_response {
            Some(response) => response,
            None => HostResponse::from_host_with_children(
                created_host,
                created_ip_addresses,
                created_ports,
                created_services,
                created_interfaces,
            ),
        };
        Ok(DiscoveredHost { host, subnet_ids })
    }

    /// The live hosts across the organization's sites, the count a plan's host limit applies to.
    /// SCD2: closed historical copies don't count.
    async fn live_host_count(&self, ctx: &HostLimitContext) -> Result<u64> {
        let filter = StorableFilter::<Host>::new_from_site_ids(&ctx.org_site_ids).live();
        Ok(self.get_all(filter).await?.len() as u64)
    }

    /// Merge `proven` into `destination` with [`Self::merge_hosts`], the same merge a person
    /// triggers from the API.
    async fn merge_same_device(
        &self,
        destination_id: &Uuid,
        proven: &SameDevice,
        authentication: &AuthenticatedEntity,
    ) -> Result<HostResponse> {
        let destination = self
            .get_by_id(destination_id)
            .await?
            .ok_or_else(|| anyhow!("host {destination_id} is no longer live"))?;
        let other = self
            .get_by_id(&proven.host_id)
            .await?
            .ok_or_else(|| anyhow!("host {} is no longer live", proven.host_id))?;
        let reason = DeletionReason::HostMergedInto {
            host_id: destination.id,
            proven_by: proven.addresses.clone(),
        };
        self.merge_hosts(destination, other, authentication.clone(), reason)
            .await
    }
}

/// Whether a MAC-matched row is the *same* interface at a new address, or a second address on the
/// same MAC.
///
/// The two are indistinguishable from the uniqueness guards alone — a host gaining a second address
/// on one MAC still presents as exactly one existing row the first time it appears — so this asks
/// the one question that separates them: has the interface left the range it used to be in? If the
/// old subnet still contains the new address, nothing says the old address is gone, and collapsing
/// them would silently discard a genuine second address (a VLAN sub-interface, an IP alias).
///
/// Deliberately conservative in the DHCP-within-one-subnet case, which stays exactly as it was:
/// this fixes the case the block was written for and invents no new behaviour for the one it
/// cannot tell apart.
fn interface_moved(existing: &IPAddress, incoming: &IPAddress, live_subnets: &[Subnet]) -> bool {
    if existing.base.subnet_id == incoming.base.subnet_id {
        return false;
    }
    live_subnets
        .iter()
        .find(|s| s.id == existing.base.subnet_id)
        .is_none_or(|s| !s.base.cidr.contains(&incoming.base.ip_address))
}

/// Whether this address needs the server to choose its subnet.
///
/// True when the id names no live subnet at all — nil, or a row this server never minted — which
/// covers both an integration that deliberately leaves placement to the server and an old daemon
/// reporting a stale id.
///
/// Also true when the named subnet no longer *contains* the address, which is the same test
/// `interface_moved` makes below. That happens when a real netmask narrows a range Scanopy had only
/// inferred: the correction re-files what it displaces, but an address it could not place keeps a
/// subnet that no longer covers it, and identity alone would never notice.
fn needs_placement(live_subnets: &[Subnet], ip_address: &IPAddress) -> bool {
    live_subnets
        .iter()
        .find(|s| s.id == ip_address.base.subnet_id)
        .is_none_or(|s| !s.base.cidr.contains(&ip_address.base.ip_address))
}

/// Rewrite a credential assignment's `ip_address_ids` from the daemon's own
/// interface UUIDs to the server-assigned ones, matching on the address itself.
///
/// A scoped assignment whose ids all fail to resolve widens to host-wide
/// (`None`) rather than collapsing to `Some(vec![])`. An empty id list is not
/// "no restriction" downstream — `build_all_credential_mappings` treats it as a
/// restriction that matches no address, so the credential silently stops being
/// dispatched. Since discovery only ever reports a credential that *probed
/// successfully on this host*, host-wide is the honest fallback; SNMP already
/// reports its assignments that way.
pub(crate) fn remap_assignment_ip_ids(
    assignment: &mut crate::server::credentials::r#impl::types::CredentialAssignment,
    daemon_ip_address_ips: &[(Uuid, IpAddr)],
    created_ip_addresses: &[IPAddress],
) {
    let Some(ids) = assignment.ip_address_ids.as_ref() else {
        return;
    };
    let remapped: Vec<Uuid> = ids
        .iter()
        .filter_map(|daemon_id| {
            let ip = daemon_ip_address_ips
                .iter()
                .find(|(id, _)| id == daemon_id)
                .map(|(_, ip)| *ip)?;
            created_ip_addresses
                .iter()
                .find(|i| i.base.ip_address == ip)
                .map(|i| i.id)
        })
        .collect();

    if remapped.is_empty() && !ids.is_empty() {
        tracing::warn!(
            credential_id = %assignment.credential_id,
            "No reported address for this credential assignment resolved to a stored IP; \
             widening it to the whole host so the credential keeps being dispatched"
        );
        assignment.ip_address_ids = None;
        return;
    }
    assignment.ip_address_ids = Some(remapped);
}

/// Why a holder gives up the ports `claimant` takes.
fn handed_over_to(claimant: &Service) -> DeletionReason {
    DeletionReason::ServicePortsHandedOver {
        to_service_id: claimant.id,
        to: claimant.base.service_definition.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::types::CredentialAssignment;
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;
    use crate::server::subnets::r#impl::base::SubnetBase;
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};

    fn subnet(id: Uuid, cidr: &str) -> Subnet {
        Subnet {
            id,
            base: SubnetBase {
                cidr: SubnetCidr::new(
                    SubnetCidrValue(cidr.parse().expect("valid test CIDR")),
                    AttributeSource::DaemonSelfReport,
                ),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// An address that actually sits inside the `127.0.0.0/8` these tests use as their live subnet.
    ///
    /// The default is `0.0.0.0`, which is in no subnet at all — fine while `needs_placement` only
    /// compared ids, and a false negative now that it also asks whether the subnet covers the
    /// address.
    fn ip_address_on(subnet_id: Uuid) -> IPAddress {
        IPAddress {
            base: IPAddressBase {
                subnet_id,
                ip_address: "127.0.0.1".parse().expect("valid test IP"),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    fn ip(s: &str) -> std::net::IpAddr {
        s.parse().expect("valid test IP")
    }

    fn ip_address_at(subnet_id: Uuid, ip: &str) -> IPAddress {
        IPAddress {
            base: IPAddressBase {
                subnet_id,
                ip_address: ip.parse().expect("valid test IP"),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// The case the block was written for and never carried out: the interface left the range it
    /// was in, so the row must follow it rather than keep an address that is gone while its
    /// `last_seen_at` advances and makes the stale value look current.
    #[test]
    fn an_interface_that_left_its_range_counts_as_moved() {
        let old_subnet = Uuid::new_v4();
        let live = vec![subnet(old_subnet, "10.0.0.0/24")];

        let existing = ip_address_at(old_subnet, "10.0.0.5");
        let incoming = ip_address_at(Uuid::new_v4(), "192.168.1.7");

        assert!(interface_moved(&existing, &incoming, &live));
    }

    /// And the case the uniqueness guards cannot tell apart from it: a host gaining a *second*
    /// address on one MAC presents as exactly one existing row the first time it appears. The old
    /// subnet still holds the new address, so nothing says the old one is gone, and treating it as
    /// a move would silently discard a genuine VLAN sub-interface or IP alias.
    #[test]
    fn a_second_address_inside_the_same_range_is_not_a_move() {
        let old_subnet = Uuid::new_v4();
        let live = vec![subnet(old_subnet, "10.0.0.0/24")];

        let existing = ip_address_at(old_subnet, "10.0.0.5");
        // A different subnet row covering the same range — the shape a re-created subnet takes.
        let incoming = ip_address_at(Uuid::new_v4(), "10.0.0.9");

        assert!(!interface_moved(&existing, &incoming, &live));
    }

    /// Re-reporting the same subnet is never a move, whatever the address does — that is the
    /// DHCP-within-one-subnet case, left exactly as it was.
    #[test]
    fn the_same_subnet_is_never_a_move() {
        let same = Uuid::new_v4();
        let live = vec![subnet(same, "10.0.0.0/24")];

        assert!(!interface_moved(
            &ip_address_at(same, "10.0.0.5"),
            &ip_address_at(same, "10.0.0.9"),
            &live
        ));
    }

    /// An existing row whose subnet no longer exists has nothing to say about where the interface
    /// was, so the reported address is the better answer.
    #[test]
    fn a_vanished_old_subnet_counts_as_moved() {
        assert!(interface_moved(
            &ip_address_at(Uuid::new_v4(), "10.0.0.5"),
            &ip_address_at(Uuid::new_v4(), "192.168.1.7"),
            &[]
        ));
    }

    /// The trigger, not the rule: an id naming no live subnet is the server's to choose — a nil
    /// sentinel from an integration that leaves placement to us, or a stale id from a daemon
    /// reporting a subnet this server never minted.
    ///
    /// The rule itself — longest prefix, never a `0.0.0.0/0` catch-all — lives with the subnets it
    /// chooses among and is tested there.
    #[test]
    fn an_id_naming_no_live_subnet_needs_placement() {
        let live = vec![subnet(Uuid::new_v4(), "127.0.0.0/8")];

        assert!(needs_placement(&live, &ip_address_on(Uuid::new_v4())));
        assert!(needs_placement(&live, &ip_address_on(Uuid::nil())));
    }

    /// And one that does name a live subnet is left alone, so an ordinary rescan re-places nothing.
    #[test]
    fn an_id_naming_a_live_subnet_is_left_alone() {
        let valid = Uuid::new_v4();
        let live = vec![subnet(valid, "127.0.0.0/8")];

        assert!(!needs_placement(&live, &ip_address_on(valid)));
    }

    /// A subnet that still exists but no longer covers the address needs re-placing.
    ///
    /// This is what a narrowed range leaves behind: a real netmask shrinks a range Scanopy had only
    /// inferred, and an address the correction could not re-file keeps pointing at it. Asking only
    /// whether the id names a live subnet would never notice.
    #[test]
    fn an_address_its_subnet_no_longer_covers_needs_placement() {
        let narrowed = Uuid::new_v4();
        let live = vec![subnet(narrowed, "10.20.30.0/24")];

        let mut stranded = ip_address_on(narrowed);
        stranded.base.ip_address = "10.20.31.5".parse().unwrap();

        assert!(needs_placement(&live, &stranded));
    }

    fn assignment(ip_address_ids: Option<Vec<Uuid>>) -> CredentialAssignment {
        CredentialAssignment {
            credential_id: Uuid::new_v4(),
            ip_address_ids,
        }
    }

    fn stored_ip(id: Uuid, addr: &str) -> IPAddress {
        IPAddress {
            id,
            base: crate::server::ip_addresses::r#impl::base::IPAddressBase {
                ip_address: ip(addr),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn assignment_ip_ids_are_rewritten_to_the_stored_rows() {
        let daemon_id = Uuid::new_v4();
        let stored_id = Uuid::new_v4();
        let mut a = assignment(Some(vec![daemon_id]));

        remap_assignment_ip_ids(
            &mut a,
            &[(daemon_id, ip("127.0.0.1"))],
            &[stored_ip(stored_id, "127.0.0.1")],
        );

        assert_eq!(a.ip_address_ids, Some(vec![stored_id]));
    }

    // A scoped assignment whose addresses all fail to resolve must widen to the whole host, not
    // collapse to an empty list: downstream, `Some(vec![])` is a restriction matching no address
    // at all, so the credential would silently stop being dispatched — and with discovery's
    // one-shot integration targets now pruned at completion, there is nothing left to re-add it.
    #[test]
    fn an_unresolvable_assignment_widens_to_the_host_instead_of_matching_nothing() {
        let mut a = assignment(Some(vec![Uuid::new_v4()]));

        remap_assignment_ip_ids(&mut a, &[], &[stored_ip(Uuid::new_v4(), "127.0.0.1")]);

        assert_eq!(a.ip_address_ids, None);
    }

    #[test]
    fn a_host_wide_assignment_stays_host_wide() {
        let mut a = assignment(None);

        remap_assignment_ip_ids(&mut a, &[], &[]);

        assert_eq!(a.ip_address_ids, None);
    }

    // GH #649: the interface prune must NOT run on a partial SNMP walk, or a transient timeout
    // deletes live switch ports and their resolved L2 neighbors every scan. These lock the gate.
    #[test]
    fn partial_walk_never_prunes_even_with_interfaces_present() {
        // The bug: an incomplete walk reported some (but not all) interfaces; pruning against it
        // would delete every interface it missed. It must be skipped.
        assert!(!should_prune_interfaces(false, 5, 0));
    }

    #[test]
    fn complete_walk_with_interfaces_prunes() {
        // An authoritative full ifTable is the only time stale interfaces may be removed.
        assert!(should_prune_interfaces(true, 5, 0));
    }

    #[test]
    fn empty_set_never_prunes_regardless_of_completeness() {
        // Zero interfaces = "none observed" (total SNMP failure / cred removal), not "all removed".
        assert!(!should_prune_interfaces(true, 0, 0));
        assert!(!should_prune_interfaces(false, 0, 0));
    }

    #[test]
    fn complete_walk_with_a_skipped_interface_never_prunes() {
        // An interface that failed to persist means the set we hold is a subset of what the
        // device reported — pruning against it would delete ports that are genuinely still there.
        assert!(!should_prune_interfaces(true, 5, 1));
    }
}
