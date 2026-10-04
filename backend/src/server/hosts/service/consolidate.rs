//! IP/MAC-based host matching, locking, upsert, and consolidation.
use super::*;
use crate::server::interfaces::service::match_existing_interface;
use crate::server::ip_addresses::r#impl::base::MacEvidence;

impl HostService {
    /// Find an existing host that matches based on IP-address data (subnet+IP or MAC address).
    ///
    /// This assembles the candidate set; the decision itself lives in `select_matching_host`,
    /// which is pure and carries the matching rules (including the VRRP/CARP/HSRP handling)
    /// plus their unit tests.
    ///
    /// Returns the matched host together with its **full, unfiltered** live IP rows —
    /// discovery derives `previous_subnets` from them for subnet↔VLAN reconciliation, so
    /// loopback and virtual-router rows must stay in — and the other hosts the payload proves
    /// are the same device ([`hosts_proven_same_device`]), which the caller merges.
    pub(crate) async fn find_matching_host_by_ip_addresses(
        &self,
        network_id: &Uuid,
        incoming_ip_addresses: &[IPAddress],
        incoming_interfaces: &[Interface],
        incoming_chassis_id: Option<&str>,
    ) -> Result<Option<HostMatch>> {
        // Every identity this payload offers, in the order the tiers below consult them. A MAC is
        // the last of the three and the only one that survives a device having no address at all,
        // which is the whole reason it is here.
        let incoming_macs = mac_identity::payload_macs(incoming_ip_addresses, incoming_interfaces);

        // A payload with no address, no chassis id and no MAC carries no identity to match on.
        if incoming_ip_addresses.is_empty()
            && incoming_chassis_id.is_none()
            && incoming_macs.is_empty()
        {
            return Ok(None);
        }

        // SCD2: only match against live rows. Closed historical copies
        // (set when a snapshot fires) must not influence reconciliation.
        let filter = StorableFilter::<Host>::new_from_network_ids(&[*network_id]).live();
        let mut all_hosts = self.get_all(filter).await?;

        if all_hosts.is_empty() {
            return Ok(None);
        }

        // Oldest-first, with an id tiebreak: storage orders by `created_at ASC` only, and
        // every host created in one daemon batch shares the batch's scan_time, so without
        // the tiebreak "first match wins" would be resolved arbitrarily by the database.
        all_hosts.sort_by_key(|h| (h.created_at, h.id));

        let host_ids: Vec<Uuid> = all_hosts.iter().map(|h| h.id).collect();
        let mut ip_addresses_by_host = self
            .ip_address_service
            .get_for_hosts(&host_ids, None)
            .await?;

        let candidates: Vec<HostCandidate> = all_hosts
            .iter()
            .map(|h| HostCandidate {
                id: h.id,
                chassis_id: crate::server::shared::attribution::text_of(&h.base.chassis_id),
                ip_addresses: ip_addresses_by_host.remove(&h.id).unwrap_or_default(),
                virtualization: h.base.virtualization_metadata.clone(),
            })
            .collect();

        let matched_id =
            match select_matching_host(incoming_ip_addresses, incoming_chassis_id, &candidates) {
                Some(id) => id,
                // The MAC tier, consulted only once addresses and the chassis id have both failed.
                // That ordering is what keeps MAC off the fleet-wide identity path: a device with an
                // address matches on IP and subnet long before it reaches here, so nothing changes for
                // it. What arrives is a device with no address at all, or one whose address moved.
                None => match self
                    .find_host_id_by_mac(
                        network_id,
                        &incoming_macs,
                        incoming_ip_addresses,
                        &candidates,
                    )
                    .await?
                {
                    Some(id) => id,
                    None => return Ok(None),
                },
            };

        let same_device = self
            .find_same_device(network_id, incoming_ip_addresses, matched_id, &candidates)
            .await?;

        let host_ip_addresses = candidates
            .into_iter()
            .find(|c| c.id == matched_id)
            .map(|c| c.ip_addresses)
            .unwrap_or_default();

        Ok(all_hosts
            .into_iter()
            .find(|h| h.id == matched_id)
            .map(|host| {
                tracing::debug!(
                    existing_host_id = %host.id,
                    existing_host_name = %host.base.name,
                    "Matched incoming IP addresses to existing host"
                );
                HostMatch {
                    host,
                    ip_addresses: host_ip_addresses,
                    same_device,
                }
            }))
    }

    /// The other hosts this payload proves are the same device as `matched`
    /// ([`hosts_proven_same_device`]).
    ///
    /// The subnet and daemon reads it needs are skipped unless another candidate holds one of the
    /// payload's addresses, which on almost every submission none does.
    async fn find_same_device(
        &self,
        network_id: &Uuid,
        incoming_ip_addresses: &[IPAddress],
        matched: Uuid,
        candidates: &[HostCandidate],
    ) -> Result<Vec<SameDevice>> {
        let shares_an_address = candidates.iter().any(|c| {
            c.id != matched
                && c.ip_addresses.iter().any(|row| {
                    incoming_ip_addresses.iter().any(|i| {
                        i.base.ip_address == row.base.ip_address
                            && i.base.subnet_id == row.base.subnet_id
                    })
                })
        });
        if !shares_an_address {
            return Ok(Vec::new());
        }

        let internal_subnet_ids: HashSet<Uuid> = self
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[*network_id]).live())
            .await?
            .into_iter()
            .filter(|s| s.is_container_bridge_subnet())
            .map(|s| s.id)
            .collect();
        let daemon_host_ids: HashSet<Uuid> = self
            .daemon_service
            .get_all(StorableFilter::<Daemon>::new_from_network_ids(&[
                *network_id,
            ]))
            .await?
            .into_iter()
            .map(|d| d.base.host_id)
            .collect();

        Ok(hosts_proven_same_device(
            incoming_ip_addresses,
            matched,
            candidates,
            &internal_subnet_ids,
            &daemon_host_ids,
        ))
    }

    /// The host on this network already carrying one of these MACs, or `None`.
    ///
    /// Looked up by targeted query rather than by widening the candidate load above. That load
    /// already fetches every live host and all their addresses, and this function runs twice per
    /// host per scan — once for `previous_subnets` and again inside `create_with_children` — so
    /// pulling every interface in the network alongside it would multiply the most-repeated read
    /// in discovery by the port count of the biggest switch on it. Two indexed reads keyed on the
    /// handful of addresses actually in the payload cost the same whatever the fleet looks like.
    ///
    /// Both tables, because a MAC-identified device may carry its address on either: one reached
    /// by an address puts it on an `ip_addresses` row, one known only at the link layer on an
    /// interface. Both are the same claim about the same NIC.
    ///
    /// Restricted to the candidate set so a row belonging to a host this network no longer holds
    /// live cannot resolve, and via each entity's service rather than its storage.
    async fn find_host_id_by_mac(
        &self,
        network_id: &Uuid,
        incoming_macs: &[MacEvidence],
        incoming_ip_addresses: &[IPAddress],
        candidates: &[HostCandidate],
    ) -> Result<Option<Uuid>> {
        // Every NIC address is worth a query except a group or virtual-router MAC, which is never
        // a NIC. A weak (locally administered) incoming MAC still matches a row whose stored copy
        // came from a hypervisor's config; `select_matching_host_by_mac` decides that per pair.
        let anchors: Vec<MacAddress> = incoming_macs
            .iter()
            .filter(|e| mac_identity::grade(e) != mac_identity::MacQuality::Excluded)
            .map(|e| e.value().0)
            .collect();
        if anchors.is_empty() {
            return Ok(None);
        }

        let ip_rows = self
            .ip_address_service
            .get_all(
                StorableFilter::<IPAddress>::new_from_network_ids(&[*network_id])
                    .mac_address_in(&anchors)
                    .live(),
            )
            .await?;
        let interface_rows = self
            .interface_service
            .get_all(
                StorableFilter::<Interface>::new_from_network_ids(&[*network_id])
                    .mac_address_in(&anchors)
                    .live(),
            )
            .await?;

        Ok(mac_identity::select_matching_host_by_mac(
            incoming_macs,
            &mac_carriers(&ip_rows, &interface_rows, incoming_ip_addresses, candidates),
        ))
    }

    /// Merge new discovery data with existing host
    pub(crate) async fn upsert_host(
        &self,
        mut existing_host: Host,
        new_host_data: Host,
        authentication: AuthenticatedEntity,
    ) -> Result<Host> {
        let host_before_updates = existing_host.clone();
        let mut has_updates = false;

        // SCD2 semantics: every successful natural-key match advances
        // last_seen_at to the scan time, regardless of whether other fields
        // change. The incoming `new_host_data` was pre-stamped by
        // `discover_host` (see `ScanContext`) so all entities from one
        // submission share one timestamp; without ScanContext the value is
        // whatever the caller put on the entity (likely a per-entity
        // Utc::now()).
        //
        // The later of the two, because a merge passes the other *stored* host here, and that
        // one can be older than the host it merges into.
        existing_host.last_seen_at = existing_host.last_seen_at.max(new_host_data.last_seen_at);

        tracing::trace!(
            "Upserting new host data {:?} to host {:?}",
            new_host_data,
            existing_host
        );

        // The name. Only names travel through here: the hostname and the other identifiers are
        // merged with the attributes below by source rank, and are never copied into `name` (see
        // the placement rule in `hosts::impl::name`). The incoming name carries the rank of
        // whatever produced it, so a controller's name refreshes on every sync and a name a person
        // typed is never touched.
        if existing_host
            .base
            .apply_name(new_host_data.base.name.clone())
        {
            has_updates = true;
        }

        if existing_host
            .base
            .apply_attributes_from(&new_host_data.base)
        {
            has_updates = true;
        }

        if existing_host
            .base
            .fill_virtualization_from(&new_host_data.base)
        {
            // A host never runs inside one of its own services: an ipvlan container's address once
            // merged into its runtime's host left that host owned by its own Docker service.
            if self.owned_by_own_service(&existing_host).await? {
                tracing::warn!(
                    host_id = %existing_host.id,
                    host_name = %existing_host.base.name,
                    "Ignoring a virtualization owner that runs on the host itself"
                );
                existing_host.base.virtualization_service_id =
                    host_before_updates.base.virtualization_service_id;
                existing_host.base.virtualization_metadata =
                    host_before_updates.base.virtualization_metadata.clone();
                existing_host.base.virtualization_interface_id =
                    host_before_updates.base.virtualization_interface_id;
            } else {
                has_updates = true;
            }
        }

        // EntitySource merge: previously concatenated discovery metadata vecs
        // here. With the metadata field removed, source is just the variant
        // discriminant — propagate the new value if it's Discovery, else keep
        // existing. Discovery context is now tracked via FK columns
        // (last_discovery_id / first_discovery_id) populated post-terminal.
        existing_host.base.source = match (existing_host.base.source, new_host_data.base.source) {
            (EntitySource::Discovery, EntitySource::Discovery) => EntitySource::Discovery,
            (_, EntitySource::Discovery) => {
                has_updates = true;
                EntitySource::Discovery
            }
            (existing_source, _) => existing_source,
        };

        // Always write — even when no field changes, last_seen_at was
        // advanced above and must persist. The Updated event is only
        // published when there's a substantive field change, so consumers
        // that listen for "real" updates aren't woken by every refresh.
        self.storage().update(&mut existing_host).await?;

        if has_updates {
            let trigger_stale = existing_host.triggers_staleness(Some(host_before_updates));

            if let Some(scope) = EntityScope::from_ids(
                existing_host.id(),
                existing_host.clone().into(),
                self.get_network_id(&existing_host),
                self.get_organization_id(&existing_host),
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
        } else {
            tracing::debug!(
                "No new data to upsert from host {} to {} (last_seen_at refresh only)",
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

        if self.daemon_service.exists(daemon_filter).await? {
            return Err(ValidationError::new(
                "Can't consolidate a host that has a daemon associated with it. \
                 Consolidate the other host into the host with the daemon instead.",
            )
            .into());
        }

        // Lock BOTH hosts (in canonical order via session_lock_many): a
        // concurrent update/delete of either host must not interleave with
        // the merge. `delete_host_inner` below runs under these guards.
        let lock_guards = self
            .storage()
            .session_lock_many(
                &[
                    LockKey::Host(destination_host.id),
                    LockKey::Host(other_host.id),
                ],
                CONSOLIDATE_LOCK_TIMEOUT,
            )
            .await?;

        tracing::trace!(
            "Consolidating host {:?} into host {:?}",
            other_host,
            destination_host
        );

        // Get ip_addresses and ports for both hosts
        let dest_interfaces = self
            .ip_address_service
            .get_for_host(&destination_host.id)
            .await?;
        let other_interfaces = self.ip_address_service.get_for_host(&other_host.id).await?;

        let dest_ports = self.port_service.get_for_host(&destination_host.id).await?;
        let other_ports = self.port_service.get_for_host(&other_host.id).await?;

        // Build interface ID mapping: source interface ID -> dest interface ID
        // Transfer non-conflicting ip_addresses to destination

        // Count MACs per host to detect VLAN sub-interfaces. MAC-based conflict detection
        // is only safe when both sides have a unique MAC (count == 1). If either host has
        // multiple ip_addresses sharing a MAC (VLANs/bridges/bonds), MAC matching would
        // incorrectly collapse distinct sub-interfaces during the merge.
        let dest_mac_counts: HashMap<MacAddress, usize> = dest_interfaces
            .iter()
            .filter_map(|i| mac_of(&i.base.mac_address))
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            });
        let other_mac_counts: HashMap<MacAddress, usize> = other_interfaces
            .iter()
            .filter_map(|i| mac_of(&i.base.mac_address))
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            });

        let mut interface_id_map: HashMap<Uuid, Uuid> = HashMap::new();
        for other_iface in &other_interfaces {
            // Check for conflict: same (subnet_id + ip_address) or same MAC (when 1:1). The same
            // address is looked for first, across every destination row: under ARP flux another
            // destination row can carry this one's MAC, and taking that MAC match first folds this
            // address into a different one and loses it.
            let matching_dest_iface = dest_interfaces
                .iter()
                .find(|dest_iface| {
                    dest_iface.base.subnet_id == other_iface.base.subnet_id
                        && dest_iface.base.ip_address == other_iface.base.ip_address
                })
                .or_else(|| {
                    dest_interfaces.iter().find(|dest_iface| {
                        matches_destination_ip_address(
                            dest_iface,
                            other_iface,
                            &dest_mac_counts,
                            &other_mac_counts,
                        )
                    })
                });

            if let Some(dest_iface) = matching_dest_iface {
                // Conflict: map source ID to destination ID
                tracing::debug!(
                    source_interface_id = %other_iface.id,
                    dest_interface_id = %dest_iface.id,
                    ip = %other_iface.base.ip_address,
                    "IP address conflict - mapping to existing destination ip_address"
                );
                interface_id_map.insert(other_iface.id, dest_iface.id);
            } else {
                // No conflict: transfer interface to destination host
                let mut transferred = other_iface.clone();
                transferred.base.host_id = destination_host.id;
                self.ip_address_service
                    .update(&mut transferred, authentication.clone())
                    .await?;
                tracing::debug!(
                    ip_address_id = %other_iface.id,
                    ip = %other_iface.base.ip_address,
                    "Transferred ip_address to destination host"
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

        // Interfaces move with their ids, so their adjacencies and the hosts they present
        // (`virtualization_interface_id`) move with them. One matching an interface the
        // destination already has maps onto it instead, by the same ladder discovery uses.
        let dest_link_interfaces = self
            .interface_service
            .get_for_host(&destination_host.id)
            .await?;
        let mut claimed: HashSet<Uuid> = HashSet::new();
        for other_if in self.interface_service.get_for_host(&other_host.id).await? {
            match match_existing_interface(&other_if, &dest_link_interfaces, &claimed) {
                Some(dest_if_id) => {
                    claimed.insert(dest_if_id);
                    self.interface_neighbor_service
                        .repoint_interface(
                            destination_host.base.network_id,
                            other_if.id,
                            dest_if_id,
                        )
                        .await?;
                    self.repoint_virtualization_interface(
                        &other_if.id,
                        dest_if_id,
                        &authentication,
                    )
                    .await?;
                }
                None => {
                    let mut moved = other_if.clone();
                    moved.base.host_id = destination_host.id;
                    // A row that moved keeps its id; one folded into a destination row may carry
                    // different MAC evidence, which the interface's own link would then contradict.
                    // Discovery's interface linking restores that link on the next scan.
                    moved.base.ip_address_id = moved
                        .base
                        .ip_address_id
                        .filter(|id| interface_id_map.get(id) == Some(id));
                    self.interface_service
                        .update(&mut moved, authentication.clone())
                        .await?;
                }
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

        // Get services for both hosts. SCD2: live rows only.
        let destination_services = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_host_ids(&[destination_host.id]).live())
            .await?;

        let other_services = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_host_ids(&[other_host.id]).live())
            .await?;

        // Transfer services, updating binding IDs using the maps
        for mut service in other_services {
            // Check for duplicate by name + service_definition
            let duplicate_of = destination_services.iter().find(|dest_svc| {
                dest_svc.base.name == service.base.name
                    && dest_svc.base.service_definition.id() == service.base.service_definition.id()
            });

            if let Some(dest_svc) = duplicate_of {
                tracing::debug!(
                    service_name = %service.base.name,
                    service_def = %service.base.service_definition.id(),
                    "Skipping duplicate service during consolidation"
                );
                // The duplicate is deleted with its host. Everything that points at it moves to
                // the destination's copy first, or deleting it would unlink its guests and
                // containers and drop it from dependencies.
                let binding_map =
                    duplicate_binding_map(&service, dest_svc, &interface_id_map, &port_id_map);
                self.service_service
                    .replace_service_references(
                        &service,
                        dest_svc,
                        &binding_map,
                        authentication.clone(),
                    )
                    .await?;
                self.repoint_virtualization_service(
                    &service.id,
                    dest_svc.id,
                    &updated_host.base.network_id,
                    &authentication,
                )
                .await?;
                continue;
            }

            // Update host_id
            service.base.host_id = updated_host.id;
            service.base.network_id = updated_host.base.network_id;

            // Remap binding IDs using our maps
            for binding in &mut service.base.bindings {
                match &mut binding.base.binding_type {
                    BindingType::IPAddress { ip_address_id } => {
                        if let Some(&new_id) = interface_id_map.get(ip_address_id) {
                            *ip_address_id = new_id;
                        } else {
                            tracing::warn!(
                                service = %service.base.name,
                                ip_address_id = %ip_address_id,
                                "IP address not found in mapping during consolidation"
                            );
                        }
                    }
                    BindingType::Port {
                        port_id,
                        ip_address_id,
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
                        if let Some(iface_id) = ip_address_id {
                            if let Some(&new_iface_id) = interface_id_map.get(iface_id) {
                                *ip_address_id = Some(new_iface_id);
                            } else {
                                tracing::warn!(
                                    service = %service.base.name,
                                    ip_address_id = %iface_id,
                                    "IP address not found in mapping, falling back to all-ip_addresses"
                                );
                                *ip_address_id = None;
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
                // Remap ip_address_ids if present
                let remapped_iface_ids = match &other.ip_address_ids {
                    None => None,
                    Some(ids) => {
                        let remapped: Vec<Uuid> = ids
                            .iter()
                            .filter_map(|id| interface_id_map.get(id).copied())
                            .collect();
                        if remapped.is_empty() {
                            // All ip_addresses were dropped — skip this assignment
                            continue;
                        }
                        Some(remapped)
                    }
                };

                if let Some(dest_assignment) = dest_cred_map.get(&other.credential_id) {
                    // Both hosts have this credential — merge with broadest-scope-wins
                    let merged_iface_ids =
                        match (&dest_assignment.ip_address_ids, &remapped_iface_ids) {
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
                        entry.ip_address_ids = merged_iface_ids;
                    }
                } else {
                    // Only on other host — add to merged list
                    merged.push(CredentialAssignment {
                        credential_id: other.credential_id,
                        ip_address_ids: remapped_iface_ids,
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

        // Adjacencies elsewhere that name the other host as their neighbour now name the
        // destination. Its interfaces have all moved or mapped by now, so a row local to one of
        // them would be the destination naming itself, and is dropped.
        let dest_interface_ids: HashSet<Uuid> = self
            .interface_service
            .get_for_host(&updated_host.id)
            .await?
            .into_iter()
            .map(|i| i.id)
            .collect();
        self.interface_neighbor_service
            .repoint_neighbor_host(
                updated_host.base.network_id,
                other_host.id,
                updated_host.id,
                &dest_interface_ids,
            )
            .await?;

        // Tags: deleting the host clears its junction rows, so copy them across first. A tag the
        // destination cannot take (a second application tag) is left behind and logged.
        let organization_id = self
            .network_service
            .get_by_id(&updated_host.base.network_id)
            .await?
            .map(|n| n.base.organization_id);
        if let Some(organization_id) = organization_id {
            let dest_tags: HashSet<Uuid> = self
                .entity_tag_service
                .get_tags(&updated_host.id, &EntityDiscriminants::Host)
                .await?
                .into_iter()
                .collect();
            for tag_id in self
                .entity_tag_service
                .get_tags(&other_host.id, &EntityDiscriminants::Host)
                .await?
            {
                if dest_tags.contains(&tag_id) {
                    continue;
                }
                if let Err(e) = self
                    .entity_tag_service
                    .add_tag(
                        updated_host.id,
                        EntityDiscriminants::Host,
                        tag_id,
                        organization_id,
                    )
                    .await
                {
                    tracing::warn!(
                        %tag_id,
                        dest_host_id = %updated_host.id,
                        error = %e,
                        "Could not carry a tag across a host merge"
                    );
                }
            }
        }

        // Delete other host (remaining children that weren't transferred will
        // cascade). Inner variant: this task already holds Host(other_host.id).
        self.delete_host_inner(&other_host.id, authentication)
            .await?;

        tracing::info!(
            source_host_id = %other_host.id,
            source_host_name = %other_host.base.name,
            dest_host_id = %updated_host.id,
            dest_host_name = %updated_host.base.name,
            ip_addresses_mapped = %interface_id_map.len(),
            ports_mapped = %port_id_map.len(),
            "Hosts consolidated"
        );

        crate::server::shared::storage::lock::release_all(lock_guards).await?;

        // Return response with hydrated children
        let (ip_addresses, ports, services, interfaces) =
            self.load_children_for_host(&updated_host.id).await?;
        Ok(HostResponse::from_host_with_children(
            updated_host,
            ip_addresses,
            ports,
            services,
            interfaces,
        ))
    }

    /// Whether `host`'s virtualization owner is a service running on `host` itself.
    async fn owned_by_own_service(&self, host: &Host) -> Result<bool> {
        let Some(owner) = host.base.virtualization_service_id else {
            return Ok(false);
        };
        Ok(self
            .service_service
            .get_by_id(&owner)
            .await?
            .is_some_and(|s| s.base.host_id == host.id))
    }

    /// Point every host presented by interface `from` at interface `to`, for a merge that maps
    /// `from` onto `to`. Deleting `from` would otherwise null the reference.
    async fn repoint_virtualization_interface(
        &self,
        from: &Uuid,
        to: Uuid,
        authentication: &AuthenticatedEntity,
    ) -> Result<()> {
        let presented = self
            .get_all(
                StorableFilter::<Host>::new()
                    .virtualization_interface_id(from)
                    .live(),
            )
            .await?;
        for mut host in presented {
            host.base.virtualization_interface_id = Some(to);
            self.update(&mut host, authentication.clone()).await?;
        }
        Ok(())
    }

    /// Point every host and subnet owned by service `from` at service `to`, for a merge that
    /// deletes `from` as a duplicate of `to`. Containers it runs are re-pointed by
    /// `ServiceService::replace_service_references`.
    async fn repoint_virtualization_service(
        &self,
        from: &Uuid,
        to: Uuid,
        network_id: &Uuid,
        authentication: &AuthenticatedEntity,
    ) -> Result<()> {
        let guests = self
            .get_all(
                StorableFilter::<Host>::new_from_network_ids(&[*network_id])
                    .virtualization_service_in(&[*from], false)
                    .live(),
            )
            .await?;
        for mut guest in guests {
            guest.base.virtualization_service_id = Some(to);
            self.update(&mut guest, authentication.clone()).await?;
        }
        let subnets = self
            .subnet_service
            .get_all(
                StorableFilter::<Subnet>::new_from_network_ids(&[*network_id])
                    .virtualization_service_in(&[*from], false)
                    .live(),
            )
            .await?;
        for mut subnet in subnets {
            subnet.base.virtualization_service_id = Some(to);
            self.subnet_service
                .update(&mut subnet, authentication.clone())
                .await?;
        }
        Ok(())
    }
}

/// `duplicate`'s binding ids mapped to `kept`'s matching bindings, for a merge that deletes
/// `duplicate`. A binding matches when it binds the same address or port once the merge's
/// address and port remapping is applied; one with no match is left out.
fn duplicate_binding_map(
    duplicate: &Service,
    kept: &Service,
    ip_address_id_map: &HashMap<Uuid, Uuid>,
    port_id_map: &HashMap<Uuid, Uuid>,
) -> HashMap<Uuid, Uuid> {
    let remap = |id: &Uuid, map: &HashMap<Uuid, Uuid>| map.get(id).copied().unwrap_or(*id);
    duplicate
        .base
        .bindings
        .iter()
        .filter_map(|binding| {
            let remapped = match binding.base.binding_type {
                BindingType::IPAddress { ip_address_id } => BindingType::IPAddress {
                    ip_address_id: remap(&ip_address_id, ip_address_id_map),
                },
                BindingType::Port {
                    port_id,
                    ip_address_id,
                } => BindingType::Port {
                    port_id: remap(&port_id, port_id_map),
                    ip_address_id: ip_address_id.map(|id| remap(&id, ip_address_id_map)),
                },
            };
            kept.base
                .bindings
                .iter()
                .find(|k| k.base.binding_type == remapped)
                .map(|k| (binding.id(), k.id()))
        })
        .collect()
}

/// The `(host, stored MAC)` pairs the MAC tier may match against: rows from either table that
/// belong to a live candidate whose MACs identify it. An ipvlan container host's rows carry its
/// runtime's MAC and are left out, so that MAC resolves to the runtime alone. So is a host whose
/// hypervisor listed its addresses for an incoming address's MAC without that address
/// (`HostCandidate::lists_other_addresses_for`); a payload with no addresses excludes nothing.
pub(crate) fn mac_carriers(
    ip_rows: &[IPAddress],
    interface_rows: &[Interface],
    incoming_ip_addresses: &[IPAddress],
    candidates: &[HostCandidate],
) -> Vec<(Uuid, MacEvidence)> {
    let identifying_host_ids: HashSet<Uuid> = candidates
        .iter()
        .filter(|c| c.macs_identify_host())
        .filter(|c| {
            !incoming_ip_addresses
                .iter()
                .any(|incoming| c.lists_other_addresses_for(incoming))
        })
        .map(|c| c.id)
        .collect();
    ip_rows
        .iter()
        .filter_map(|r| r.base.mac_address.clone().map(|m| (r.base.host_id, m)))
        .chain(
            interface_rows
                .iter()
                .filter_map(|r| r.base.mac_address.clone().map(|m| (r.base.host_id, m))),
        )
        .filter(|(host_id, _)| identifying_host_ids.contains(host_id))
        .collect()
}

/// Whether `other_iface`, on the host being merged away, is already represented by `dest_iface` on
/// the destination host, and so should map onto that row rather than transfer beside it.
///
/// A free function rather than a closure so it can be tested without a database — the same shape
/// `matches_existing_subnet` uses for subnet dedup, and for the same reason: this is the rule that
/// decides whether a merge leaves the destination host holding two rows for one NIC, and it was
/// reachable only through `consolidate_hosts`, which needs a database.
pub(crate) fn matches_destination_ip_address(
    dest_iface: &IPAddress,
    other_iface: &IPAddress,
    dest_mac_counts: &HashMap<MacAddress, usize>,
    other_mac_counts: &HashMap<MacAddress, usize>,
) -> bool {
    // Match by subnet + IP (always safe — same logical ip_address)
    if dest_iface.base.subnet_id == other_iface.base.subnet_id
        && dest_iface.base.ip_address == other_iface.base.ip_address
    {
        return true;
    }

    // Match by MAC only when both hosts have a single interface with this MAC.
    // Multiple ip_addresses sharing a MAC = VLAN sub-interfaces that should
    // be preserved separately, not collapsed during merge.
    //
    // On the MAC value, never on the carrier: `MacEvidence` is an `Attributed`, which compares
    // value and source together, so a bare `==` would ask whether both hosts learned the address
    // the same way. They routinely have not — a stored row stamped `Unspecified` against a
    // daemon's `ArpReply`, or a forwarding table against an ARP reply — and the question here is
    // whether it is the same NIC. Missing the conflict transfers a second row for one NIC onto the
    // destination host.
    mac_of(&dest_iface.base.mac_address)
        .zip(mac_of(&other_iface.base.mac_address))
        .is_some_and(|(dest_mac, other_mac)| {
            dest_mac == other_mac
                && dest_mac_counts.get(&dest_mac).copied().unwrap_or(0) == 1
                && other_mac_counts.get(&other_mac).copied().unwrap_or(0) == 1
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::{IPAddressBase, MacEvidenceValue};

    fn ip_address(subnet_id: Uuid, ip: &str, mac: Option<(&str, AttributeSource)>) -> IPAddress {
        IPAddress::new(IPAddressBase {
            subnet_id,
            ip_address: ip.parse().expect("valid test IP"),
            mac_address: mac.map(|(mac, source)| {
                MacEvidence::new(
                    MacEvidenceValue(mac.parse::<MacAddress>().expect("valid test MAC")),
                    source,
                )
            }),
            ..Default::default()
        })
    }

    /// The same count `merge_hosts` builds from a host's live rows, so a fixture cannot be counted
    /// a way the caller never counts it.
    fn mac_counts(addresses: &[IPAddress]) -> HashMap<MacAddress, usize> {
        addresses
            .iter()
            .filter_map(|i| mac_of(&i.base.mac_address))
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            })
    }

    /// The regression. One NIC known to both hosts by different routes — a stored row that predates
    /// the provenance column against a daemon's ARP reply — carries the same address under
    /// different sources. Comparing the `MacEvidence` carriers asks whether the two hosts learned
    /// it the same way, which is not the question, and answering "no" transfers a second row for
    /// that NIC onto the destination host.
    #[test]
    fn different_mac_sources_for_one_nic_still_conflict() {
        let mac = "a4:bb:6d:12:34:56";
        let dest = ip_address(
            Uuid::new_v4(),
            "10.0.0.5",
            Some((mac, AttributeSource::Unspecified)),
        );
        let other = ip_address(
            Uuid::new_v4(),
            "10.0.9.9",
            Some((mac, AttributeSource::ArpReply)),
        );

        assert!(matches_destination_ip_address(
            &dest,
            &other,
            &mac_counts(std::slice::from_ref(&dest)),
            &mac_counts(std::slice::from_ref(&other)),
        ));
    }

    /// The subnet+IP tier stands on its own: the same logical address is the same row whatever the
    /// MACs say about it, including when they disagree.
    #[test]
    fn same_subnet_and_ip_conflict_whatever_the_macs_say() {
        let subnet = Uuid::new_v4();
        let dest = ip_address(
            subnet,
            "10.0.0.5",
            Some(("a4:bb:6d:12:34:56", AttributeSource::ArpReply)),
        );
        let other = ip_address(
            subnet,
            "10.0.0.5",
            Some(("a4:bb:6d:99:99:99", AttributeSource::ForwardingTable)),
        );

        assert!(matches_destination_ip_address(
            &dest,
            &other,
            &mac_counts(std::slice::from_ref(&dest)),
            &mac_counts(std::slice::from_ref(&other)),
        ));
    }

    /// VLAN sub-interfaces, bridge members and bond members share a parent's MAC while holding
    /// distinct addresses. A MAC that more than one row on either host carries identifies no single
    /// NIC, so the MAC tier must not fire and those rows transfer separately.
    #[test]
    fn a_mac_held_by_several_interfaces_does_not_conflict() {
        let mac = "a4:bb:6d:12:34:56";
        let dest = ip_address(
            Uuid::new_v4(),
            "10.0.0.5",
            Some((mac, AttributeSource::ArpReply)),
        );
        let dest_vlan = ip_address(
            Uuid::new_v4(),
            "10.0.1.5",
            Some((mac, AttributeSource::ArpReply)),
        );
        let other = ip_address(
            Uuid::new_v4(),
            "10.0.9.9",
            Some((mac, AttributeSource::ArpReply)),
        );

        assert!(!matches_destination_ip_address(
            &dest,
            &other,
            &mac_counts(&[dest.clone(), dest_vlan]),
            &mac_counts(std::slice::from_ref(&other)),
        ));
    }

    /// Two genuinely separate NICs sharing no address must stay separate rows.
    #[test]
    fn different_nics_with_no_shared_address_do_not_conflict() {
        let dest = ip_address(
            Uuid::new_v4(),
            "10.0.0.5",
            Some(("a4:bb:6d:12:34:56", AttributeSource::ArpReply)),
        );
        let other = ip_address(
            Uuid::new_v4(),
            "10.0.9.9",
            Some(("a4:bb:6d:99:99:99", AttributeSource::ArpReply)),
        );

        assert!(!matches_destination_ip_address(
            &dest,
            &other,
            &mac_counts(std::slice::from_ref(&dest)),
            &mac_counts(std::slice::from_ref(&other)),
        ));
    }
}
