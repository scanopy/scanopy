//! Taking a container's addresses off a host that also holds someone else's.
//!
//! A sweep can file a container's address on another host before the runtime reports it: an
//! ipvlan endpoint answers with its parent NIC's MAC, so the sweep keeps its address as a second
//! address on whichever host holds that MAC. The runtime's report names exactly the container's
//! addresses, so a report that matches such a host takes only those addresses (with what is bound
//! to them) to a host of its own, and leaves the rest where they were.
use super::*;
use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::ports::r#impl::base::PortBase;

impl HostMatch {
    /// The rows a container report takes from the host it matched, or `None` when it adopts that
    /// host as before.
    ///
    /// Taken when the report is a container's, the matched host is not a container, and the host
    /// holds an address beside the ones the report names. The report is the runtime's full list
    /// of the container's addresses, so the host's other addresses are another device's: in the
    /// lab, the VM whose second NIC the ipvlan endpoint shares. A matched host holding only the
    /// container's addresses is the container's own record from the sweep, and is adopted.
    pub(crate) fn rows_a_container_takes(
        &self,
        incoming_virtualization: Option<&HostVirtualization>,
        incoming: &[IPAddress],
    ) -> Option<HashSet<Uuid>> {
        if !incoming_virtualization.is_some_and(HostVirtualization::is_container)
            || self
                .host
                .base
                .virtualization_metadata
                .as_ref()
                .is_some_and(HostVirtualization::is_container)
        {
            return None;
        }
        let (named, others): (Vec<&IPAddress>, Vec<&IPAddress>) = self
            .ip_addresses
            .iter()
            .filter(|row| !should_skip_for_matching(row))
            .partition(|row| incoming.iter().any(|i| ip_addresses_share_address(i, row)));
        (!named.is_empty() && !others.is_empty())
            .then(|| named.into_iter().map(|row| row.id).collect())
    }
}

impl HostService {
    /// Move the `rows` of `from` to `to`, with what only they carry.
    ///
    /// - The address rows move with their ids.
    /// - A service on `from` bound only to moved rows moves, and its ports with it. A port another
    ///   service on `from` still binds stays, and `to` gets its own row for it.
    /// - A service bound to moved rows and others stays, without the bindings to moved rows.
    /// - An interface on `from` linked to a moved row stays and loses the link: the NIC is the
    ///   one the two share.
    /// - Credential assignments naming moved rows move those rows to `to`.
    pub(crate) async fn detach_addresses(
        &self,
        from: &Host,
        to: &Host,
        rows: &HashSet<Uuid>,
        authentication: &AuthenticatedEntity,
    ) -> Result<()> {
        let guard = self
            .storage()
            .session_lock(LockKey::Host(from.id), DEFAULT_LOCK_TIMEOUT)
            .await?;

        for mut row in self.ip_address_service.get_for_host(&from.id).await? {
            if rows.contains(&row.id) {
                row.base.host_id = to.id;
                self.ip_address_service
                    .update(&mut row, authentication.clone())
                    .await?;
            }
        }

        for mut interface in self.interface_service.get_for_host(&from.id).await? {
            if interface
                .base
                .ip_address_id
                .is_some_and(|id| rows.contains(&id))
            {
                interface.base.ip_address_id = None;
                self.interface_service
                    .update(&mut interface, authentication.clone())
                    .await?;
            }
        }

        let binds_moved_row = |binding: &Binding| match binding.base.binding_type {
            BindingType::IPAddress { ip_address_id } => rows.contains(&ip_address_id),
            BindingType::Port { ip_address_id, .. } => {
                ip_address_id.is_some_and(|id| rows.contains(&id))
            }
        };
        let services = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_host_ids(&[from.id]).live())
            .await?;
        let (moving, staying): (Vec<Service>, Vec<Service>) = services.into_iter().partition(|s| {
            !s.base.bindings.is_empty() && s.base.bindings.iter().all(binds_moved_row)
        });

        let ports_still_bound: HashSet<Uuid> = staying
            .iter()
            .flat_map(|s| &s.base.bindings)
            .filter(|b| !binds_moved_row(b))
            .filter_map(|b| match b.base.binding_type {
                BindingType::Port { port_id, .. } => Some(port_id),
                BindingType::IPAddress { .. } => None,
            })
            .collect();
        let from_ports = self.port_service.get_for_host(&from.id).await?;
        let mut port_id_map: HashMap<Uuid, Uuid> = HashMap::new();
        for port_id in moving
            .iter()
            .flat_map(|s| &s.base.bindings)
            .filter_map(|b| match b.base.binding_type {
                BindingType::Port { port_id, .. } => Some(port_id),
                BindingType::IPAddress { .. } => None,
            })
            .collect::<HashSet<Uuid>>()
        {
            let Some(port) = from_ports.iter().find(|p| p.id == port_id) else {
                continue;
            };
            let new_id = if ports_still_bound.contains(&port_id) {
                self.port_service
                    .create(
                        Port::new(PortBase::new(
                            to.id,
                            to.base.network_id,
                            port.base.port_type,
                        )),
                        authentication.clone(),
                    )
                    .await?
                    .id
            } else {
                let mut moved = port.with_host(to.id, to.base.network_id);
                self.port_service
                    .update(&mut moved, authentication.clone())
                    .await?;
                port_id
            };
            port_id_map.insert(port_id, new_id);
        }

        for mut service in moving {
            service.base.host_id = to.id;
            for binding in &mut service.base.bindings {
                if let BindingType::Port { port_id, .. } = &mut binding.base.binding_type
                    && let Some(&new_id) = port_id_map.get(port_id)
                {
                    *port_id = new_id;
                }
            }
            tracing::debug!(
                service = %service.base.name,
                from_host_id = %from.id,
                to_host_id = %to.id,
                "Moved a service bound only to a container's addresses"
            );
            self.service_service
                .update(&mut service, authentication.clone())
                .await?;
        }
        for mut service in staying {
            let before = service.base.bindings.len();
            service.base.bindings.retain(|b| !binds_moved_row(b));
            if service.base.bindings.len() != before {
                self.service_service
                    .update(&mut service, authentication.clone())
                    .await?;
            }
        }

        let assignments = self
            .credential_service
            .get_credential_assignments_for_host(&from.id)
            .await?;
        let mut kept: Vec<CredentialAssignment> = Vec::new();
        let mut taken: Vec<CredentialAssignment> = Vec::new();
        for assignment in &assignments {
            let Some(ids) = &assignment.ip_address_ids else {
                kept.push(assignment.clone());
                continue;
            };
            let (moved, remaining): (Vec<Uuid>, Vec<Uuid>) =
                ids.iter().partition(|id| rows.contains(id));
            if !moved.is_empty() {
                taken.push(CredentialAssignment {
                    credential_id: assignment.credential_id,
                    ip_address_ids: Some(moved),
                });
            }
            if !remaining.is_empty() {
                kept.push(CredentialAssignment {
                    credential_id: assignment.credential_id,
                    ip_address_ids: Some(remaining),
                });
            }
        }
        if !taken.is_empty() {
            self.credential_service
                .set_host_credentials(&from.id, &kept)
                .await?;
            self.credential_service
                .set_host_credentials(&to.id, &taken)
                .await?;
        }

        tracing::info!(
            from_host_id = %from.id,
            to_host_id = %to.id,
            addresses = rows.len(),
            "Took a container's addresses off a host that also holds another device's"
        );
        guard.release().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::virtualization::{
        ContainerHostVirtualization, ContainerNetworkType, ProxmoxVirtualization,
    };
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;

    fn row(host_id: Uuid, subnet_id: Uuid, ip: &str) -> IPAddress {
        IPAddress::new(IPAddressBase {
            host_id,
            subnet_id,
            ip_address: ip.parse().unwrap(),
            ..Default::default()
        })
    }

    fn container() -> HostVirtualization {
        HostVirtualization::Docker(ContainerHostVirtualization {
            container_name: Some("ipvlan-test".to_string()),
            container_id: Some("aec2a3e9".to_string()),
            compose_project: None,
            network_type: ContainerNetworkType::IpVlan,
        })
    }

    fn matched(virtualization: Option<HostVirtualization>, rows: Vec<IPAddress>) -> HostMatch {
        let mut host = Host::new(HostBase::default());
        host.base.virtualization_metadata = virtualization;
        HostMatch {
            host,
            ip_addresses: rows,
            same_device: vec![],
        }
    }

    /// The lab: the sweep kept the ipvlan endpoint's .231 beside the VM's .63 behind one MAC. The
    /// runtime's report for .231 takes .231 and leaves .63.
    #[test]
    fn a_container_report_takes_only_its_own_addresses() {
        let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let vm_address = row(host, lan, "192.168.4.63");
        let container_address = row(host, lan, "192.168.7.231");
        let m = matched(None, vec![vm_address, container_address.clone()]);
        let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];

        assert_eq!(
            m.rows_a_container_takes(Some(&container()), &incoming),
            Some(HashSet::from([container_address.id]))
        );
    }

    /// The same holds when the host the report matched is a hypervisor guest: a container is
    /// never that guest.
    #[test]
    fn a_guest_holding_a_container_address_gives_it_up() {
        let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let m = matched(
            Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                vm_name: Some("docker".to_string()),
                vm_id: Some("103".to_string()),
                guest_type: None,
            })),
            vec![
                row(host, lan, "192.168.4.63"),
                row(host, lan, "192.168.7.231"),
            ],
        );
        let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];
        assert!(
            m.rows_a_container_takes(Some(&container()), &incoming)
                .is_some()
        );
    }

    /// A host holding only the container's addresses is the container's record from the sweep.
    #[test]
    fn a_host_holding_only_the_containers_addresses_is_adopted() {
        let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let m = matched(None, vec![row(host, lan, "192.168.7.231")]);
        let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];
        assert_eq!(
            m.rows_a_container_takes(Some(&container()), &incoming),
            None
        );
    }

    /// A container's own host, whatever else it was given, is adopted: its report is a rescan.
    #[test]
    fn a_container_host_is_adopted() {
        let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let m = matched(
            Some(container()),
            vec![
                row(host, lan, "192.168.4.63"),
                row(host, lan, "192.168.7.231"),
            ],
        );
        let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];
        assert_eq!(
            m.rows_a_container_takes(Some(&container()), &incoming),
            None
        );
    }

    /// Only a container's report names a full address list; any other payload adopts as before.
    #[test]
    fn a_payload_that_is_not_a_container_adopts() {
        let (host, lan) = (Uuid::new_v4(), Uuid::new_v4());
        let m = matched(
            None,
            vec![
                row(host, lan, "192.168.4.63"),
                row(host, lan, "192.168.7.231"),
            ],
        );
        let incoming = [row(Uuid::nil(), lan, "192.168.7.231")];
        assert_eq!(m.rows_a_container_takes(None, &incoming), None);
    }
}
