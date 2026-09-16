//! Host updates and child (IP/port/service) synchronization.
use super::*;

impl HostService {
    /// Update a host from an UpdateHostRequest
    /// Optionally syncs ip_addresses and ports if provided in the request.
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
            virtualization_metadata,
            virtualization_service_id,
            hidden,
            tags,
            expected_updated_at: _,
            ip_addresses,
            ports,
            services,
            interfaces,
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

        // Same reason as the create path: an unresolvable virtualizing service must come back as
        // a validation error, not a foreign-key 500.
        self.validate_virtualization_service(virtualization_service_id)
            .await?;

        let mut updated_host = Host {
            id,
            created_at: existing.created_at,
            updated_at: Utc::now(),
            valid_from: existing.valid_from,
            valid_to: existing.valid_to,
            lineage_id: existing.lineage_id,
            last_seen_at: existing.last_seen_at,
            last_discovery_id: existing.last_discovery_id,
            first_discovery_id: existing.first_discovery_id,
            base: HostBase {
                // Carried over, then reconciled below — the request alone cannot say whether a
                // person renamed the host or merely saved some other field.
                name: existing.base.name.clone(),
                network_id,
                source: existing.base.source,
                // Carried over and reconciled below, for the same reason as the name.
                hostname: existing.base.hostname.clone(),
                description,
                virtualization_metadata,
                virtualization_service_id,
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
                firmware_revision: existing.base.firmware_revision.clone(),
                software_revision: existing.base.software_revision.clone(),
                credential_assignments: credential_assignments
                    .unwrap_or_else(|| existing.base.credential_assignments.clone()),
            },
        };

        // Only an actual change of the name is a person naming the host. The edit modal PUTs the
        // whole object, so stamping `Manual` on every save would freeze a derived name — a host
        // named after its detected service could then never adopt its controller's name because
        // someone once toggled "hidden".
        //
        // A blank name is a person handing naming back to discovery. `apply_name` would read it as
        // "no name to offer" and keep the stored one, so it is cleared explicitly.
        if name.trim().is_empty() {
            updated_host.base.clear_name();
        } else if updated_host.base.name.value().as_str() != name {
            updated_host.base.apply_name(HostName::manual(name));
        }

        // The same rule for the hostname: the edit modal sends the stored value back on every
        // save, so only an actual change is a person asserting one. Blank clears it.
        let requested_hostname = hostname
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty());
        if requested_hostname != attribution::text_of(&updated_host.base.hostname) {
            updated_host.base.hostname = requested_hostname
                .map(|h| Attributed::new(HostHostnameValue(h), AttributeSource::Manual));
        }

        if let Some(org_id) = authentication.organization_id() {
            self.entity_tag_service
                .set_tags(id, EntityDiscriminants::Host, tags, org_id)
                .await?;
        }

        let updated = self
            .update(&mut updated_host, authentication.clone())
            .await?;

        // Sync ip_addresses only if provided (None means preserve existing)
        if let Some(ip_addresses) = ip_addresses {
            self.sync_ip_addresses(
                &updated.id,
                &network_id,
                ip_addresses,
                authentication.clone(),
            )
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

        // Sync interfaces only if provided (None means preserve existing)
        if let Some(interfaces) = interfaces {
            self.sync_interfaces(&updated.id, &network_id, interfaces, authentication.clone())
                .await?;
        }

        // Load fresh children after sync
        let (ip_addresses, ports, services, interfaces) =
            self.load_children_for_host(&updated.id).await?;

        Ok(HostResponse::from_host_with_children(
            updated,
            ip_addresses,
            ports,
            services,
            interfaces,
        ))
    }

    /// Sync ip_addresses for a host: delete removed, update existing, create new.
    /// Client provides UUIDs - if ID exists for this host, update; if not, create.
    async fn sync_ip_addresses(
        &self,
        host_id: &Uuid,
        network_id: &Uuid,
        inputs: Vec<IPAddressInput>,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use std::collections::HashSet;

        // Get existing ip_addresses for this host (needed for position resolution)
        let existing = self.ip_address_service.get_for_host(host_id).await?;
        let existing_ids: HashSet<Uuid> = existing.iter().map(|i| i.id).collect();

        // Resolve and validate positions
        let mut inputs = inputs;
        resolve_and_validate_input_positions(&mut inputs, &existing, "ip_address")
            .map_err(|e| ValidationError::new(e.message))?;

        // All input IDs (client-provided)
        let input_ids: HashSet<Uuid> = inputs.iter().map(|i| i.id).collect();

        // Delete ip_addresses that are not in the input list
        let to_delete: Vec<Uuid> = existing_ids.difference(&input_ids).copied().collect();
        if !to_delete.is_empty() {
            self.ip_address_service
                .delete_many(&to_delete, authentication.clone())
                .await?;
        }

        // Process each input - create or update based on whether ID exists for this host
        for input in inputs {
            let id = input.id;
            let mut ip_address = input.into_ip_address(*host_id, *network_id);

            if existing_ids.contains(&id) {
                // Update existing interface - preserve created_at from existing
                if let Some(existing_iface) = existing.iter().find(|i| i.id == id) {
                    ip_address.preserve_immutable_fields(existing_iface);
                }

                self.ip_address_service
                    .update(&mut ip_address, authentication.clone())
                    .await?;
            } else {
                // Create new interface with client-provided ID
                self.ip_address_service
                    .create(ip_address, authentication.clone())
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

        // Get existing interfaces for this host
        let existing = self.interface_service.get_for_host(host_id).await?;
        let existing_ids: HashSet<Uuid> = existing.iter().map(|i| i.id).collect();

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
                // Get current port binding keys (port_id, ip_address_id)
                let current_ports: HashSet<_> = existing_svc
                    .base
                    .bindings
                    .iter()
                    .filter_map(|b| match &b.base.binding_type {
                        BindingType::Port {
                            port_id,
                            ip_address_id,
                        } => Some((*port_id, *ip_address_id)),
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
                            ip_address_id,
                            ..
                        } => Some((*port_id, *ip_address_id)),
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
}
