//! Port-binding removal, conflict partitioning, and availability validation.
use super::*;
use crate::server::shared::relations as R;

impl ServiceService {
    /// Remove specific port bindings from a service and sync to database.
    ///
    /// Used during discovery conflict resolution to reclaim ports from generic services
    /// (e.g., Unclaimed Open Ports) when a specific service definition now matches.
    /// Returns the remaining bindings after removal.
    /// Remove port bindings that overlap with the given claims.
    ///
    /// `claims_to_remove` contains `(port_id, claimer_ip_address_id)` pairs.
    /// A binding is removed if its port_id matches a claim AND the ip_addresses
    /// overlap (None overlaps anything, Some(a) overlaps Some(a)).
    ///
    /// Note: the daemon's OpenPorts upsert later in the batch sets the
    /// authoritative final state, so this only needs to clear conflicts
    /// to unblock service creation — no splitting needed.
    pub async fn remove_port_bindings(
        &self,
        service_id: &Uuid,
        claims_to_remove: &[(Uuid, Option<Uuid>)],
        authentication: AuthenticatedEntity,
        reason: DeletionReason,
    ) -> Result<Vec<Binding>> {
        let service = self
            .get_by_id(service_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Service {} not found", service_id))?;

        let remaining_bindings: Vec<Binding> = service
            .base
            .bindings
            .into_iter()
            .filter(|b| {
                let Some(port_id) = b.port_id() else {
                    return true; // Keep interface-only bindings
                };
                let bind_iface = b.ip_address_id();
                // Keep if no claim overlaps this binding
                !claims_to_remove.iter().any(|(claim_port, claim_iface)| {
                    *claim_port == port_id
                        && match (claim_iface, &bind_iface) {
                            (None, _) | (_, None) => true,
                            (Some(a), Some(b)) => a == b,
                        }
                })
            })
            .collect();

        self.binding_service
            .sync_for(
                &R::BINDING_SERVICE.at(*service_id),
                &remaining_bindings,
                authentication,
                reason,
                UpdateReason::EntityMaintained,
            )
            .await
    }

    /// Remove every binding a service holds, leaving the service and its dependency memberships.
    /// For a service about to be folded into another, whose bindings would otherwise conflict
    /// with the one taking them.
    pub async fn clear_bindings(
        &self,
        service_id: &Uuid,
        authentication: AuthenticatedEntity,
        reason: DeletionReason,
    ) -> Result<()> {
        self.binding_service
            .sync_for(
                &R::BINDING_SERVICE.at(*service_id),
                &[],
                authentication,
                reason,
                UpdateReason::EntityMaintained,
            )
            .await?;
        Ok(())
    }

    /// Partition bindings into non-conflicting and conflicting sets.
    ///
    /// A binding conflicts if another service on the same host already has a port binding
    /// to the same port on the same interface (or either is "all ip_addresses").
    ///
    /// Also checks against `batch_claimed` for in-batch conflict detection during discovery.
    ///
    /// Returns: (valid_bindings, conflicting_bindings)
    pub async fn partition_conflicting_bindings(
        &self,
        host_id: &Uuid,
        service_id: &Uuid,
        bindings: Vec<Binding>,
        batch_claimed: &[(Uuid, Option<Uuid>)],
    ) -> Result<(Vec<Binding>, Vec<Binding>)> {
        if bindings.is_empty() {
            return Ok((vec![], vec![]));
        }

        // Get existing claimed bindings from database. SCD2: live services only.
        let filter = StorableFilter::<Service>::new_from_host_ids(&[*host_id]).live();
        let db_claimed: Vec<(Uuid, Option<Uuid>)> = self
            .get_all(filter)
            .await?
            .into_iter()
            .filter(|s| s.id != *service_id)
            .flat_map(|s| {
                s.base.bindings.into_iter().filter_map(|b| {
                    if let BindingType::Port {
                        port_id,
                        ip_address_id,
                    } = b.base.binding_type
                    {
                        Some((port_id, ip_address_id))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Combine DB claims with batch claims
        let all_claimed: Vec<_> = db_claimed
            .iter()
            .chain(batch_claimed.iter())
            .cloned()
            .collect();

        if all_claimed.is_empty() {
            return Ok((bindings, vec![]));
        }

        let mut valid = Vec::new();
        let mut conflicting = Vec::new();

        for binding in bindings {
            if let BindingType::Port {
                port_id,
                ip_address_id,
            } = &binding.base.binding_type
            {
                let has_conflict = all_claimed.iter().any(|(claimed_port, claimed_iface)| {
                    if claimed_port != port_id {
                        return false;
                    }
                    // Conflict if same port AND ip_addresses overlap:
                    // - Either is "all ip_addresses" (None) -> conflict
                    // - Both specific and same interface -> conflict
                    match (ip_address_id, claimed_iface) {
                        (None, _) | (_, None) => true,
                        (Some(a), Some(b)) => a == b,
                    }
                });

                if has_conflict {
                    conflicting.push(binding);
                } else {
                    valid.push(binding);
                }
            } else {
                // Interface bindings don't conflict cross-service
                valid.push(binding);
            }
        }

        Ok((valid, conflicting))
    }

    /// Validate that proposed bindings don't conflict with other services on the same host.
    /// Returns error with helpful message identifying the conflicting service.
    /// Used for manual service creation/update validation.
    pub(crate) async fn validate_bindings_available(
        &self,
        host_id: &Uuid,
        service_id: &Uuid,
        bindings: &[Binding],
    ) -> Result<()> {
        if bindings.is_empty() {
            return Ok(());
        }

        // SCD2: only live services compete for binding ownership.
        let filter = StorableFilter::<Service>::new_from_host_ids(&[*host_id]).live();
        let other_services: Vec<_> = self
            .get_all(filter)
            .await?
            .into_iter()
            .filter(|s| s.id != *service_id)
            .collect();

        for binding in bindings {
            if let BindingType::Port {
                port_id,
                ip_address_id,
            } = &binding.base.binding_type
            {
                let conflicting_service = other_services.iter().find(|s| {
                    s.base.bindings.iter().any(|b| {
                        if let BindingType::Port {
                            port_id: existing_port,
                            ip_address_id: existing_iface,
                        } = &b.base.binding_type
                        {
                            if existing_port != port_id {
                                return false;
                            }
                            match (ip_address_id, existing_iface) {
                                (None, _) | (_, None) => true,
                                (Some(a), Some(b)) => a == b,
                            }
                        } else {
                            false
                        }
                    })
                });

                if let Some(owner) = conflicting_service {
                    let host_service = self
                        .host_service
                        .get()
                        .expect("host_service not initialized");

                    let ports = host_service.get_ports_for_host(host_id).await?;
                    let port_display = ports
                        .iter()
                        .find(|p| p.id == *port_id)
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| port_id.to_string());

                    return Err(ValidationError::new(format!(
                        "Port {} is already bound to '{}' on this host. \
                         Use 'Transfer Ports' to reassign it, or remove the binding from '{}' first.",
                        port_display, owner.base.name, owner.base.name,
                    ))
                    .into());
                }
            }
        }

        Ok(())
    }
}
