//! A host report that reached a container at its bridge address from outside its runtime.
//!
//! A daemon on the same Docker or Podman network as a container sweeps that network as its own
//! subnet, and reports what answers at the container's address as a host. The runtime host's
//! container scan already recorded that address on the runtime host, so the report matches the
//! runtime host. What it read describes the container, so the rule the runtime host's own daemon
//! applies at its bridge containers (`daemon::discovery::service::bridge_containers`) applies here:
//!
//! - its services stay, marked as the container's;
//! - its host facts, interfaces and other addresses are dropped;
//! - a credential assignment it earned is kept to the container's address.
use super::*;
use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::services::r#impl::virtualization::ServiceVirtualization;

/// The container a report reached, as its runtime host records it.
#[derive(Debug, PartialEq)]
pub(crate) struct ReachedContainer {
    /// The report's rows at the container's bridge addresses.
    address_ids: Vec<Uuid>,
    /// The container's virtualization and its runtime's service, from the stored service bound to
    /// that address. `None` when the runtime host holds the address with no container service on
    /// it, such as a container removed since.
    virtualization: Option<(ServiceVirtualization, Option<Uuid>)>,
}

impl HostMatch {
    /// The stored rows the report shares with this host: the same address on the same subnet.
    fn shared_rows<'a>(&'a self, incoming: &'a [IPAddress]) -> Vec<(&'a IPAddress, &'a IPAddress)> {
        incoming
            .iter()
            .filter(|i| !i.base.ip_address.is_loopback())
            .flat_map(|i| {
                self.ip_addresses
                    .iter()
                    .filter(move |row| ip_addresses_share_address(i, row))
                    .map(move |row| (i, row))
            })
            .collect()
    }

    /// Whether the report could be one that reached a container through this host's bridge: it is
    /// not itself a container's, this host is not a container, and it shares an address with this
    /// host. Checked before loading the site's subnets and the host's services.
    fn may_have_reached_container(
        &self,
        incoming_virtualization: Option<&HostVirtualization>,
        incoming: &[IPAddress],
    ) -> bool {
        !incoming_virtualization.is_some_and(HostVirtualization::is_container)
            && !self
                .host
                .base
                .virtualization_metadata
                .as_ref()
                .is_some_and(HostVirtualization::is_container)
            && !self.shared_rows(incoming).is_empty()
    }

    /// The container the report reached, when every address it shares with this host is one of
    /// the host's container bridge addresses. `None` when it shares any other address, as the
    /// runtime host's own report does with its LAN address.
    pub(crate) fn reached_container(
        &self,
        incoming_virtualization: Option<&HostVirtualization>,
        incoming: &[IPAddress],
        bridge_subnet_ids: &HashSet<Uuid>,
        stored_services: &[Service],
    ) -> Option<ReachedContainer> {
        if !self.may_have_reached_container(incoming_virtualization, incoming) {
            return None;
        }
        let shared = self.shared_rows(incoming);
        if !shared
            .iter()
            .all(|(_, row)| bridge_subnet_ids.contains(&row.base.subnet_id))
        {
            return None;
        }
        let virtualization = shared.iter().find_map(|(_, row)| {
            stored_services.iter().find_map(|s| {
                let bound_here = s
                    .base
                    .bindings
                    .iter()
                    .any(|b| b.ip_address_id() == Some(row.id));
                s.base
                    .virtualization_metadata
                    .clone()
                    .filter(|_| bound_here)
                    .map(|v| (v, s.base.virtualization_service_id))
            })
        });
        Some(ReachedContainer {
            address_ids: shared.iter().map(|(i, _)| i.id).collect(),
            virtualization,
        })
    }
}

impl ReachedContainer {
    /// Cut the report down to what describes the container, so its upsert onto `runtime_host`
    /// adds the container's services and changes nothing about the runtime host itself.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn confine(
        &self,
        runtime_host: &Host,
        host: &mut Host,
        ip_addresses: &mut Vec<IPAddress>,
        services: &mut Vec<Service>,
        interfaces: &mut Vec<Interface>,
        subnets: &mut Vec<Subnet>,
        interfaces_complete: &mut bool,
    ) {
        ip_addresses.retain(|i| self.address_ids.contains(&i.id));
        subnets.retain(|s| ip_addresses.iter().any(|i| i.base.subnet_id == s.id));
        // Not a complete ifTable of the runtime host, so its stored interfaces are not pruned.
        interfaces.clear();
        *interfaces_complete = false;

        for service in services.iter_mut() {
            service.base.bindings.retain(|b| {
                b.ip_address_id()
                    .is_none_or(|id| self.address_ids.contains(&id))
            });
            if let Some((virtualization, runtime_service_id)) = &self.virtualization {
                service.base.virtualization_metadata = Some(virtualization.clone());
                service.base.virtualization_service_id = *runtime_service_id;
            }
        }
        services.retain(|s| !s.base.bindings.is_empty());

        // An integration may report its credential for the whole host (SNMP does), but it reached
        // the container, not the runtime host.
        let assignments = std::mem::take(&mut host.base.credential_assignments)
            .into_iter()
            .map(|assignment| CredentialAssignment {
                ip_address_ids: Some(self.address_ids.clone()),
                ..assignment
            })
            .collect();
        host.base = HostBase {
            credential_assignments: assignments,
            ..runtime_host.base.clone()
        };
    }
}

impl HostService {
    /// [`HostMatch::reached_container`], loading the site's bridge subnets and the matched host's
    /// services only for a report that shares an address with a host that is not a container.
    pub(crate) async fn reached_container(
        &self,
        matched: &HostMatch,
        incoming_virtualization: Option<&HostVirtualization>,
        incoming: &[IPAddress],
    ) -> Result<Option<ReachedContainer>> {
        if !matched.may_have_reached_container(incoming_virtualization, incoming) {
            return Ok(None);
        }
        let bridge_subnet_ids: HashSet<Uuid> = self
            .subnet_service
            .get_all(
                StorableFilter::<Subnet>::new_from_site_ids(&[matched.host.base.site_id]).live(),
            )
            .await?
            .into_iter()
            .filter(|s| s.is_container_bridge_subnet())
            .map(|s| s.id)
            .collect();
        if bridge_subnet_ids.is_empty() {
            return Ok(None);
        }
        let stored_services = self
            .service_service
            .get_all(
                StorableFilter::<Service>::new_from_uuid_column("host_id", &matched.host.id).live(),
            )
            .await?;
        Ok(matched.reached_container(
            incoming_virtualization,
            incoming,
            &bridge_subnet_ids,
            &stored_services,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::bindings::r#impl::base::{Binding, BindingType};
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;
    use crate::server::services::r#impl::virtualization::DockerVirtualization;

    fn row(host_id: Uuid, subnet_id: Uuid, ip: &str) -> IPAddress {
        IPAddress::new(IPAddressBase {
            host_id,
            subnet_id,
            ip_address: ip.parse().unwrap(),
            ..Default::default()
        })
    }

    fn docker(name: &str) -> ServiceVirtualization {
        ServiceVirtualization::Docker(DockerVirtualization {
            container_name: Some(name.to_string()),
            container_id: Some(format!("{name}-id")),
            compose_project: None,
            state: Default::default(),
        })
    }

    fn bound_service(
        ip_address_id: Uuid,
        virtualization: Option<ServiceVirtualization>,
    ) -> Service {
        let mut service = Service {
            id: Uuid::new_v4(),
            ..Default::default()
        };
        service.base.bindings = vec![Binding::new_serviceless(BindingType::Port {
            port_id: Uuid::new_v4(),
            ip_address_id: Some(ip_address_id),
        })];
        service.base.virtualization_metadata = virtualization;
        service
    }

    /// The lab: a Docker host's LAN address and its `cc-snmp-sha` container's bridge address,
    /// with the container scan's service for that container.
    struct RuntimeHost {
        matched: HostMatch,
        lan: Uuid,
        bridge: Uuid,
        container: ServiceVirtualization,
        runtime_service_id: Uuid,
        stored_services: Vec<Service>,
    }

    fn runtime_host() -> RuntimeHost {
        let (host_id, lan, bridge) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let bridge_row = row(host_id, bridge, "10.77.0.2");
        let container = docker("cc-snmp-sha");
        let runtime_service_id = Uuid::new_v4();
        let mut placeholder = bound_service(bridge_row.id, Some(container.clone()));
        placeholder.base.virtualization_service_id = Some(runtime_service_id);
        let mut host = Host::new(HostBase {
            description: Some("docker-host".to_string()),
            ..Default::default()
        });
        host.id = host_id;
        RuntimeHost {
            matched: HostMatch {
                host,
                matched_by: HostMatchedBy::IpAddress {
                    ip: "10.77.0.2".parse().unwrap(),
                },
                ip_addresses: vec![row(host_id, lan, "192.168.65.3"), bridge_row],
                same_device: vec![],
                inferred_twins: vec![],
            },
            lan,
            bridge,
            container,
            runtime_service_id,
            stored_services: vec![placeholder],
        }
    }

    /// Another daemon's SNMP walk of the container: its address, its loopback, its ifTable, a
    /// sysName, the SNMP service on both addresses, and the credential for the whole host.
    #[test]
    fn a_report_reaching_a_container_keeps_only_the_containers_services() {
        let r = runtime_host();
        let credential_id = Uuid::new_v4();
        let at_bridge = row(Uuid::nil(), r.bridge, "10.77.0.2");
        let loopback = row(Uuid::nil(), Uuid::new_v4(), "127.0.0.1");
        let mut host = Host::new(HostBase {
            description: Some("snmp-container".to_string()),
            credential_assignments: vec![CredentialAssignment {
                credential_id,
                ip_address_ids: None,
            }],
            ..Default::default()
        });
        let mut ip_addresses = vec![at_bridge.clone(), loopback.clone()];
        let mut services = vec![
            bound_service(at_bridge.id, None),
            bound_service(loopback.id, None),
        ];
        let mut interfaces = vec![Interface::default(); 11];
        let mut subnets = vec![];
        let mut interfaces_complete = true;

        let reached = r
            .matched
            .reached_container(
                None,
                &ip_addresses,
                &HashSet::from([r.bridge]),
                &r.stored_services,
            )
            .expect("the report shares only the container's bridge address");
        reached.confine(
            &r.matched.host,
            &mut host,
            &mut ip_addresses,
            &mut services,
            &mut interfaces,
            &mut subnets,
            &mut interfaces_complete,
        );

        assert_eq!(ip_addresses, vec![at_bridge.clone()]);
        assert!(interfaces.is_empty() && !interfaces_complete);
        assert_eq!(host.base.description, r.matched.host.base.description);
        assert_eq!(
            host.base.credential_assignments,
            vec![CredentialAssignment {
                credential_id,
                ip_address_ids: Some(vec![at_bridge.id]),
            }]
        );
        assert_eq!(services.len(), 1);
        assert_eq!(
            services[0].base.virtualization_metadata,
            Some(r.container.clone())
        );
        assert_eq!(
            services[0].base.virtualization_service_id,
            Some(r.runtime_service_id)
        );
    }

    /// The runtime host's own report holds its LAN address too, so it is the host.
    #[test]
    fn the_runtime_hosts_own_report_is_the_host() {
        let r = runtime_host();
        let incoming = [
            row(Uuid::nil(), r.lan, "192.168.65.3"),
            row(Uuid::nil(), r.bridge, "10.77.0.2"),
        ];

        assert_eq!(
            r.matched.reached_container(
                None,
                &incoming,
                &HashSet::from([r.bridge]),
                &r.stored_services
            ),
            None
        );
    }
}
