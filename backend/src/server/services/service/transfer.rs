//! Service interface-binding reassignment on host change.
use super::*;

impl ServiceService {
    /// Update bindings to match ports and ip_addresses available on new host
    /// `original_interfaces` and `updated_interfaces` are the ip_addresses for the respective hosts
    /// `original_ports` and `updated_ports` are the ports for the respective hosts
    #[allow(clippy::too_many_arguments)]
    pub async fn reassign_service_interface_bindings(
        &self,
        service: Service,
        original_host: &Host,
        original_interfaces: &[IPAddress],
        original_ports: &[Port],
        updated_host: &Host,
        updated_interfaces: &[IPAddress],
        updated_ports: &[Port],
        interface_id_remap: &std::collections::HashMap<Uuid, Uuid>,
    ) -> Service {
        // Best-effort serialization vs concurrent update/delete of the same
        // service during host transfer. This method is infallible by
        // signature, so a lock failure logs and proceeds (the previous
        // in-memory lock provided no cross-process protection at all).
        // Held for the whole method; released via Drop (infallible signature).
        let _lock_guard = match self
            .storage
            .session_lock(LockKey::Service(service.id), DEFAULT_LOCK_TIMEOUT)
            .await
        {
            Ok(guard) => Some(guard),
            Err(e) => {
                tracing::warn!(
                    service_id = %service.id,
                    error = %e,
                    "Proceeding with binding reassignment without DB lock"
                );
                None
            }
        };

        tracing::trace!(
            "Preparing service {:?} for transfer from host {:?} to host {:?}",
            service,
            original_host,
            updated_host
        );

        let mut mutable_service = service.clone();

        let service_name = service.base.name.clone();
        let service_id = service.id;

        mutable_service.base.bindings = mutable_service
            .base
            .bindings
            .iter_mut()
            .filter_map(|b| {
                // Look up original interface from the provided slice
                let original_interface = b
                    .ip_address_id()
                    .and_then(|id| original_interfaces.iter().find(|i| i.id == id));

                match &mut b.base.binding_type {
                    BindingType::IPAddress { ip_address_id } => {
                        if let Some(original_ip_address) = original_interface {
                            let new_ip_address: Option<&IPAddress> =
                                updated_interfaces.iter().find(|i| *i == original_ip_address);

                            if let Some(new_ip_address) = new_ip_address {
                                *ip_address_id = new_ip_address.id;
                                return Some(*b);
                            }
                        }
                        // Structural match failed — try direct ID remap (handles subnet_id
                        // mismatch on second scan where scanner subnet UUIDs differ from DB)
                        if let Some(&new_id) = interface_id_remap.get(ip_address_id) {
                            *ip_address_id = new_id;
                            return Some(*b);
                        }
                        // Interface binding couldn't be matched - this can happen during consolidation
                        // when the source host's interface doesn't exist on the destination host.
                        // We drop the binding and warn.
                        tracing::warn!(
                            service_id = %service_id,
                            service_name = %service_name,
                            original_interface_id = ?b.ip_address_id(),
                            "Dropping ip_address binding during reassignment: \
                             no matching ip_address found on destination host"
                        );
                        None::<Binding>
                    }
                    BindingType::Port {
                        port_id,
                        ip_address_id,
                    } => {
                        if let Some(original_port) =
                            original_ports.iter().find(|p| p.id == *port_id)
                            && let Some(new_port) =
                                updated_ports.iter().find(|p| *p == original_port)
                        {
                            let new_ip_address: Option<Option<IPAddress>> = match original_interface
                            {
                                // None interface = listen on all ip_addresses, assume same for new host
                                None => Some(None),
                                Some(original_ip_address) => {
                                    match updated_interfaces
                                        .iter()
                                        .find(|i| *i == original_ip_address)
                                    {
                                        Some(found_ip_address) => {
                                            Some(Some(found_ip_address.clone()))
                                        }
                                        None => {
                                            // Structural match failed — try direct ID remap
                                            // (handles subnet_id mismatch on second scan)
                                            if let Some(&new_id) = interface_id_remap.get(&original_ip_address.id) {
                                                if let Some(found) = updated_interfaces.iter().find(|i| i.id == new_id) {
                                                    Some(Some(found.clone()))
                                                } else {
                                                    tracing::warn!(
                                                        service_id = %service_id,
                                                        service_name = %service_name,
                                                        port_number = %new_port.base.port_type.config().number,
                                                        remapped_interface_id = %new_id,
                                                        "Port binding ip_address remap target not found - \
                                                         falling back to 'all ip_addresses'"
                                                    );
                                                    Some(None)
                                                }
                                            } else {
                                                tracing::warn!(
                                                    service_id = %service_id,
                                                    service_name = %service_name,
                                                    port_number = %new_port.base.port_type.config().number,
                                                    original_interface_ip = %original_ip_address.base.ip_address,
                                                    "Port binding ip_address not found on destination host - \
                                                     falling back to 'all ip_addresses'"
                                                );
                                                Some(None)
                                            }
                                        }
                                    }
                                }
                            };

                            match new_ip_address {
                                None => return None,
                                Some(new_ip_address) => {
                                    *port_id = new_port.id;
                                    *ip_address_id = match new_ip_address {
                                        Some(new_ip_address) => Some(new_ip_address.id),
                                        None => None,
                                    };
                                    return Some(*b);
                                }
                            }
                        }
                        // Port not found on destination host - drop the binding
                        tracing::warn!(
                            service_id = %service_id,
                            service_name = %service_name,
                            original_port_id = %port_id,
                            "Dropping port binding during reassignment: \
                             no matching port found on destination host"
                        );
                        None::<Binding>
                    }
                };

                None
            })
            .collect();

        mutable_service.base.host_id = updated_host.id;

        mutable_service.base.site_id = updated_host.base.site_id;

        tracing::trace!(
            "Reassigned service {:?} bindings for from host {:?} to host {:?}",
            mutable_service,
            original_host,
            updated_host
        );

        mutable_service
    }
}

impl ServiceService {
    /// Point everything that references `old` at `new`, so `old` can be deleted without taking
    /// anything with it.
    ///
    /// For a host merge, where `old` duplicates `new` (same name and definition) on the host being
    /// merged away. Deleting `old` otherwise nulls the link of every container it runs
    /// (`services.virtualization_service_id`, `ON DELETE SET NULL`) and drops it from every
    /// dependency (`dependency_members`, `ON DELETE CASCADE`). `binding_map` carries `old`'s
    /// binding ids to `new`'s matching bindings; a binding with no match is dropped from a
    /// binding-level dependency, the same as deleting it would.
    pub async fn replace_service_references(
        &self,
        old: &Service,
        new: &Service,
        binding_map: &std::collections::HashMap<Uuid, Uuid>,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use crate::server::dependencies::r#impl::base::DependencyMembers;

        let site_id = old.base.site_id;
        let containers = self
            .get_all(
                StorableFilter::<Service>::new_from_site_ids(&[site_id])
                    .virtualization_service_in(&[old.id], false)
                    .live(),
            )
            .await?;
        for mut container in containers {
            container.base.virtualization_service_id = Some(new.id);
            self.update(&mut container, authentication.clone()).await?;
        }

        // Same lock `update_dependency_members` takes, held across the read-modify-write.
        let lock_guard = self
            .storage
            .session_lock(LockKey::DependencyMembers { site_id }, DEFAULT_LOCK_TIMEOUT)
            .await?;
        let dependencies = self
            .dependency_service
            .get_all(StorableFilter::<Dependency>::new_from_site_ids(&[site_id]))
            .await?;
        for mut dependency in dependencies {
            let changed = match &mut dependency.base.members {
                DependencyMembers::Services { service_ids } => {
                    let before = service_ids.clone();
                    let mut seen = std::collections::HashSet::new();
                    *service_ids = service_ids
                        .iter()
                        .map(|id| if *id == old.id { new.id } else { *id })
                        .filter(|id| seen.insert(*id))
                        .collect();
                    *service_ids != before
                }
                DependencyMembers::Bindings { binding_ids } => {
                    let old_bindings: Vec<Uuid> =
                        old.base.bindings.iter().map(|b| b.id()).collect();
                    let before = binding_ids.clone();
                    *binding_ids = binding_ids
                        .iter()
                        .filter_map(|id| {
                            if old_bindings.contains(id) {
                                binding_map.get(id).copied()
                            } else {
                                Some(*id)
                            }
                        })
                        .collect();
                    *binding_ids != before
                }
            };
            if changed {
                self.dependency_service
                    .update(&mut dependency, authentication.clone())
                    .await?;
            }
        }
        lock_guard.release().await?;
        Ok(())
    }
}
