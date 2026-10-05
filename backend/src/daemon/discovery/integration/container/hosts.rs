//! Containers with their own identity on the LAN become hosts under their runtime.
//!
//! A container on a macvlan or ipvlan network has its own IP on the LAN (and, on macvlan, its own
//! MAC), so the network sweep finds it as a host. Reporting its services on the runtime's host
//! would put them at the wrong address. It is submitted as its own host instead, linked to the
//! runtime's stored service through `virtualization_service_id`, the way a Proxmox guest is linked
//! to its node.

use std::collections::HashMap;

use anyhow::{Error, Result};
use bollard::models::{ContainerInspectResponse, ContainerSummary};
use uuid::Uuid;

use crate::daemon::discovery::service::ops::HostData;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::name::{HostName, HostNameSources};
use crate::server::hosts::r#impl::virtualization::{
    ContainerHostVirtualization, ContainerNetworkType,
};
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::ports::r#impl::base::Port;
use crate::server::services::r#impl::base::ServiceMatchBaselineParams;
use crate::server::services::r#impl::definitions::ServiceDefinitionExt;
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::Subnet;

use super::ContainerRuntime;
use super::interfaces::{container_host_addresses, container_lan_network_type};
use super::scanner::{ContainerScanner, spread_bindings_across_endpoints};

/// One container as a host of its own, before service matching.
pub struct ContainerHostRecord {
    pub host: Host,
    /// Its macvlan/ipvlan addresses first, then any bridge addresses, each with the subnet it was
    /// placed in (`None` when the server is left to place it).
    pub addresses: Vec<(IPAddress, Option<Subnet>)>,
}

/// Build the host for a container with an endpoint on one of the runtime's macvlan/ipvlan
/// networks. `None` when no such endpoint carries an address.
///
/// `runtime_service_id` is the runtime's **stored** service id, read back from the server: the
/// server keeps a guest's link only when it names a real service.
#[allow(clippy::too_many_arguments)]
pub fn container_host_record(
    runtime: ContainerRuntime,
    container: &ContainerInspectResponse,
    network_type: ContainerNetworkType,
    lan_subnets: &[Subnet],
    bridge_subnets: &[Subnet],
    placement_subnets: &[Subnet],
    runtime_service_id: Uuid,
    network_id: Uuid,
) -> Option<ContainerHostRecord> {
    let addresses = container_host_addresses(
        runtime,
        container,
        lan_subnets,
        bridge_subnets,
        placement_subnets,
        network_id,
    );
    // Bridge addresses sort after the LAN ones, so a LAN address leads whenever there is one.
    let leads_with_lan = addresses
        .first()
        .is_some_and(|(_, s)| !s.as_ref().is_some_and(Subnet::is_container_bridge_subnet));
    if !leads_with_lan {
        return None;
    }

    let container_name = container
        .name
        .as_ref()
        .map(|n| n.trim_start_matches('/').to_string())
        .filter(|n| !n.is_empty());

    let mut host = Host::new(HostBase {
        network_id,
        source: EntitySource::Discovery,
        virtualization_metadata: Some(runtime.host_virtualization(ContainerHostVirtualization {
            container_name: container_name.clone(),
            container_id: container.id.clone(),
            compose_project: ContainerScanner::extract_compose_project(container),
            network_type,
        })),
        virtualization_service_id: Some(runtime_service_id),
        ..Default::default()
    });
    if let Some(name) = container_name {
        // A person named the container when they started it.
        host.base
            .apply_name(HostName::from_controller(name, runtime.client_probe()));
    }

    Some(ContainerHostRecord { host, addresses })
}

/// A container with an endpoint on one of the runtime's macvlan/ipvlan networks.
pub struct LanContainer {
    pub container: ContainerInspectResponse,
    pub summary: ContainerSummary,
    pub network_type: ContainerNetworkType,
    /// Containers that share its network namespace (`NetworkMode "container:<id>"`). They have
    /// no endpoints of their own and answer at its addresses, so their services go on its host.
    pub members: Vec<(ContainerInspectResponse, ContainerSummary)>,
}

/// Split a runtime's containers into those that are hosts of their own (with the members of
/// their network namespace) and those that stay services on the runtime's host.
pub fn partition_lan_containers(
    containers: Vec<(ContainerInspectResponse, ContainerSummary)>,
    lan_subnets: &[Subnet],
) -> (
    Vec<LanContainer>,
    Vec<(ContainerInspectResponse, ContainerSummary)>,
) {
    let mut lan_containers = Vec::new();
    let mut on_host = Vec::new();
    for (container, summary) in containers {
        match container_lan_network_type(&container, lan_subnets) {
            Some(network_type) => lan_containers.push(LanContainer {
                container,
                summary,
                network_type,
                members: Vec::new(),
            }),
            None => on_host.push((container, summary)),
        }
    }

    // A namespace member names its parent by a short or full id, as in `container_interfaces`.
    let mut remaining = Vec::with_capacity(on_host.len());
    for (container, summary) in on_host {
        let parent = shared_netns_reference(&container).and_then(|reference| {
            lan_containers.iter_mut().find(|lan| {
                lan.container
                    .id
                    .as_deref()
                    .is_some_and(|id| id.starts_with(reference))
            })
        });
        match parent {
            Some(parent) => parent.members.push((container, summary)),
            None => remaining.push((container, summary)),
        }
    }

    (lan_containers, remaining)
}

/// The container whose network namespace `container` shares, as its `NetworkMode` names it.
fn shared_netns_reference(container: &ContainerInspectResponse) -> Option<&str> {
    container
        .host_config
        .as_ref()?
        .network_mode
        .as_deref()?
        .strip_prefix("container:")
        .filter(|r| !r.is_empty())
}

/// What recording a runtime's container hosts produced for the runtime's own host.
#[derive(Default)]
pub struct ContainerHostsOutcome {
    /// Container hosts the server accepted.
    pub recorded: usize,
    /// Host ports published to those containers. They are open on the runtime's host, so they
    /// stay there.
    pub published_ports: Vec<Port>,
}

impl ContainerScanner<'_> {
    /// Submit the runtime's host with its runtime service, and return the id the server stored
    /// that service under. Container hosts link to it, and the server keeps a link only when it
    /// names a stored service.
    ///
    /// The scanned host's own submission later lands on the same host by id and address.
    pub async fn submit_runtime_host(&self, host_data: &HostData) -> Result<Option<Uuid>, Error> {
        let Some(runtime_service) = host_data
            .services
            .iter()
            .find(|s| s.id == self.runtime_service_id)
        else {
            return Ok(None);
        };
        let bound_port_ids: Vec<Uuid> = runtime_service
            .base
            .bindings
            .iter()
            .filter_map(|b| b.port_id())
            .collect();
        let ports: Vec<Port> = host_data
            .ports
            .iter()
            .filter(|p| bound_port_ids.contains(&p.id))
            .cloned()
            .collect();
        // Matched by address alone. The swept MAC can belong to more than this host: an ipvlan
        // endpoint answers ARP with its parent's MAC, and its host may already hold the address
        // this runtime's MAC was seen at. The full submission that follows carries the MACs.
        let ip_addresses: Vec<IPAddress> = host_data
            .ip_addresses
            .iter()
            .cloned()
            .map(|mut a| {
                a.base.mac_address = None;
                a
            })
            .collect();

        let response = self
            .ops
            .create_integration_host(
                self.integration,
                host_data.host.clone(),
                ip_addresses,
                ports,
                vec![runtime_service.clone()],
                vec![],
                host_data.subnets.clone(),
                // Only the runtime's own record is sent here; the host's interfaces arrive with
                // its full submission.
                false,
                InterfaceDataComplete::none(),
                self.cancel,
            )
            .await?;

        Ok(response
            .services
            .iter()
            .find(|s| {
                self.runtime
                    .is_runtime_role(s.base.service_definition.virtualization_role())
            })
            .map(|s| s.id))
    }

    /// Submit each container as its own host under the runtime service stored as
    /// `runtime_service_id`. Stops at `deadline`, at a container boundary.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_container_hosts(
        &self,
        containers: &[LanContainer],
        lan_subnets: &[Subnet],
        bridge_subnets: &[Subnet],
        placement_subnets: &[Subnet],
        runtime_service_id: Uuid,
        deadline: tokio::time::Instant,
    ) -> Result<ContainerHostsOutcome, Error> {
        let network_id = self.ops.network_id().await?;
        let daemon_id = self.ops.daemon_id().await?;
        let mut outcome = ContainerHostsOutcome::default();
        // Daemon-minted bridge subnet id → the id the server holds it under, or `None` when the
        // server could not record it.
        let mut stored_bridges: HashMap<Uuid, Option<Uuid>> = HashMap::new();

        for LanContainer {
            container,
            summary,
            network_type,
            members,
        } in containers
        {
            if self.cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }
            if tokio::time::Instant::now() >= deadline {
                tracing::warn!(
                    runtime = self.runtime.label(),
                    "Container scan hit its soft deadline before every container host was recorded"
                );
                break;
            }

            let Some(record) = container_host_record(
                self.runtime,
                container,
                *network_type,
                lan_subnets,
                bridge_subnets,
                placement_subnets,
                runtime_service_id,
                network_id,
            ) else {
                tracing::debug!(
                    container = ?container.name,
                    "Container has a LAN endpoint with no address; not recorded as a host"
                );
                continue;
            };

            let ContainerHostRecord { host, addresses } = record;
            let mut placed: Vec<(IPAddress, Option<Subnet>)> = Vec::new();
            for (mut address, subnet) in addresses {
                let on_bridge = subnet
                    .as_ref()
                    .filter(|s| s.is_container_bridge_subnet())
                    .cloned();
                if let Some(bridge) = on_bridge {
                    // The server holds a runtime's bridge under the stored runtime service, and
                    // dedups on that pair, so recording it under that owner returns the row the
                    // address belongs in.
                    let stored = match stored_bridges.get(&bridge.id) {
                        Some(stored) => *stored,
                        None => {
                            let mut owned = bridge.clone();
                            owned.base.virtualization_service_id = Some(runtime_service_id);
                            let stored = match self.ops.create_subnet(&owned, self.cancel).await {
                                Ok(s) => Some(s.id),
                                Err(e) => {
                                    tracing::warn!(
                                        cidr = %bridge.base.cidr,
                                        error = %e,
                                        "Failed to record a container bridge; container addresses on it are left out"
                                    );
                                    None
                                }
                            };
                            stored_bridges.insert(bridge.id, stored);
                            stored
                        }
                    };
                    let Some(stored) = stored else { continue };
                    address.base.subnet_id = stored;
                }
                placed.push((address, subnet));
            }

            let (mut services, mut ports, published_ports) = self
                .match_container_host_services(&host, container, summary, &placed, &daemon_id)
                .await?;
            outcome.published_ports.extend(published_ports);
            // Each member is matched on its own exposed ports (`exposed_port_scope`), at the
            // addresses it shares with the parent.
            for (member, member_summary) in members {
                let (member_services, member_ports, member_published) = self
                    .match_container_host_services(
                        &host,
                        member,
                        member_summary,
                        &placed,
                        &daemon_id,
                    )
                    .await?;
                services.extend(member_services);
                ports.extend(member_ports);
                outcome.published_ports.extend(member_published);
            }

            let ip_addresses: Vec<IPAddress> = placed.into_iter().map(|(a, _)| a).collect();
            match self
                .ops
                .create_integration_host(
                    self.integration,
                    host,
                    ip_addresses,
                    ports,
                    services,
                    vec![],
                    vec![],
                    // The runtime reports endpoints, never the container's interface table.
                    false,
                    InterfaceDataComplete::none(),
                    self.cancel,
                )
                .await
            {
                Ok(_) => outcome.recorded += 1,
                Err(e) => tracing::warn!(
                    container = ?container.name,
                    error = %e,
                    "Failed to record container host"
                ),
            }
        }

        Ok(outcome)
    }

    /// Match a container host's services the way a bridge container's are matched (the exec
    /// probe over loopback, plus the ports the runtime lists), against the container's own first
    /// address, then bind them at each of its addresses. Returns its services and ports, and the
    /// host ports published to it, which belong to the runtime's host.
    async fn match_container_host_services(
        &self,
        host: &Host,
        container: &ContainerInspectResponse,
        summary: &ContainerSummary,
        addresses: &[(IPAddress, Option<Subnet>)],
        daemon_id: &Uuid,
    ) -> Result<
        (
            Vec<crate::server::services::r#impl::base::Service>,
            Vec<Port>,
            Vec<Port>,
        ),
        Error,
    > {
        let Some((primary, primary_subnet)) = addresses.first() else {
            return Ok((vec![], vec![], vec![]));
        };
        // Matching needs a subnet. An address left for the server to place is matched against an
        // untyped one.
        let match_subnet = primary_subnet.clone().unwrap_or_default();

        let exposed_port_filter = ContainerScanner::exposed_port_scope(container);
        let with_subnets: Vec<(IPAddress, Subnet)> = addresses
            .iter()
            .map(|(a, _)| (a.clone(), match_subnet.clone()))
            .collect();
        let (host_ip_to_host_ports, container_ips_to_container_ports, _) =
            self.get_ports_from_container(summary, &with_subnets, exposed_port_filter.as_ref());

        let probed = match &container.name {
            Some(name) => {
                self.scan_container_endpoints(
                    name.trim_start_matches('/'),
                    self.cancel.clone(),
                    exposed_port_filter.as_ref(),
                )
                .await?
            }
            None => vec![],
        };
        let endpoint_responses = ContainerScanner::attribute_to_address(
            &probed,
            primary.base.ip_address,
            &HashMap::new(),
        );
        let all_ports = container_ips_to_container_ports
            .get(&primary.base.ip_address)
            .cloned()
            .unwrap_or_default();

        let (mut services, ports) = self.ops.match_services(
            host,
            &ServiceMatchBaselineParams {
                subnet: &match_subnet,
                ip_address: primary,
                all_ports: &all_ports,
                endpoint_responses: &endpoint_responses,
                // The host carries the container relationship; its services are the container's
                // own, not containers on the runtime's host.
                virtualization_metadata: &None,
                virtualization_service_id: None,
                client_responses: &HashMap::new(),
                managed_device: &None,
                // The daemon reaches it over the LAN, but no mDNS browse ran for it.
                dns_sd: &None,
            },
            &[],
            daemon_id,
            &host.base.network_id,
        )?;

        let address_ids: Vec<Uuid> = addresses.iter().map(|(a, _)| a.id).collect();
        spread_bindings_across_endpoints(&mut services, &address_ids, primary.id);

        let published_ports = host_ip_to_host_ports
            .into_values()
            .flatten()
            .map(Port::new_hostless)
            .collect();

        Ok((services, ports, published_ports))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::virtualization::HostVirtualization;
    use crate::server::subnets::r#impl::types::SubnetType;

    use super::super::interfaces::tests::{container, endpoint, subnet};
    use crate::server::credentials::r#impl::types::CredentialIntegration;
    use crate::server::hosts::r#impl::virtualization::undeclared_virtualization;

    /// Every container host either runtime builds, on either network type, is a kind of
    /// virtualization its integration declares reporting, which is what the docs list.
    #[test]
    fn every_container_host_is_declared_by_its_integration() {
        for runtime in [ContainerRuntime::Docker, ContainerRuntime::Podman] {
            let integration = match runtime {
                ContainerRuntime::Docker => CredentialIntegration::Docker,
                ContainerRuntime::Podman => CredentialIntegration::Podman,
            };
            for network_type in [ContainerNetworkType::MacVlan, ContainerNetworkType::IpVlan] {
                let lan_type = match network_type {
                    ContainerNetworkType::MacVlan => SubnetType::MacVlan,
                    ContainerNetworkType::IpVlan => SubnetType::IpVlan,
                };
                let lan_network = subnet("lan", "192.168.1.0/24", lan_type);
                let known_lan = subnet("Office LAN", "192.168.1.0/24", SubnetType::Lan);
                let web = container(
                    "4f1c2a",
                    "web",
                    vec![(
                        "lan",
                        endpoint("192.168.1.53", None, Some("02:42:c0:a8:01:35")),
                    )],
                );
                let record = container_host_record(
                    runtime,
                    &web,
                    network_type,
                    std::slice::from_ref(&lan_network),
                    &[],
                    std::slice::from_ref(&known_lan),
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                )
                .expect("a LAN container is a host");
                assert_eq!(
                    undeclared_virtualization(integration, &record.host),
                    None,
                    "{runtime:?} {network_type:?}"
                );
            }
        }
    }

    /// A macvlan container becomes a host under the runtime's stored service, named by the
    /// container and carrying only its own addresses.
    #[test]
    fn a_macvlan_container_host_links_to_its_runtime() {
        let runtime_service_id = Uuid::new_v4();
        let lan_network = subnet("lan", "192.168.1.0/24", SubnetType::MacVlan);
        let known_lan = subnet("Office LAN", "192.168.1.0/24", SubnetType::Lan);
        let mut pihole = container(
            "4f1c2a",
            "pihole",
            vec![(
                "lan",
                endpoint("192.168.1.53", None, Some("02:42:c0:a8:01:35")),
            )],
        );
        pihole.config = Some(bollard::models::ContainerConfig {
            labels: Some(HashMap::from([(
                "com.docker.compose.project".to_string(),
                "dns".to_string(),
            )])),
            ..Default::default()
        });

        let record = container_host_record(
            ContainerRuntime::Docker,
            &pihole,
            ContainerNetworkType::MacVlan,
            std::slice::from_ref(&lan_network),
            &[],
            std::slice::from_ref(&known_lan),
            runtime_service_id,
            Uuid::new_v4(),
        )
        .expect("a macvlan container is a host");

        assert_eq!(
            record.host.base.virtualization_metadata,
            Some(HostVirtualization::Docker(ContainerHostVirtualization {
                container_name: Some("pihole".into()),
                container_id: Some("4f1c2a".into()),
                compose_project: Some("dns".into()),
                network_type: ContainerNetworkType::MacVlan,
            }))
        );
        assert_eq!(
            record.host.base.virtualization_service_id,
            Some(runtime_service_id)
        );
        assert_eq!(record.host.base.name.value().to_string(), "pihole");

        let ips: Vec<String> = record
            .addresses
            .iter()
            .map(|(a, _)| a.base.ip_address.to_string())
            .collect();
        assert_eq!(
            ips,
            vec!["192.168.1.53"],
            "the runtime host's addresses stay off it"
        );
        assert_eq!(record.addresses[0].0.base.subnet_id, known_lan.id);
    }

    /// A pod member sharing a macvlan container's network namespace answers at that container's
    /// addresses, so it goes with it to the container host. A member of a bridge container stays
    /// on the runtime's host with its parent.
    #[test]
    fn a_namespace_member_follows_its_lan_parent_to_its_host() {
        let lan = vec![subnet("lan", "192.168.1.0/24", SubnetType::MacVlan)];
        let member_of = |id: &str, name: &str, parent: &str| {
            let mut c = container(id, name, vec![]);
            c.host_config = Some(bollard::models::HostConfig {
                network_mode: Some(format!("container:{parent}")),
                ..Default::default()
            });
            (c, ContainerSummary::default())
        };
        let containers = vec![
            (
                container(
                    "aaaa1111bbbb",
                    "pihole",
                    vec![("lan", endpoint("192.168.1.53", None, None))],
                ),
                ContainerSummary::default(),
            ),
            (
                container(
                    "cccc2222dddd",
                    "web",
                    vec![("app_default", endpoint("172.18.0.2", None, None))],
                ),
                ContainerSummary::default(),
            ),
            member_of("eeee3333ffff", "unbound", "aaaa1111"),
            member_of("9999888877776666", "web-sidecar", "cccc2222dddd"),
        ];

        let (lan_containers, on_host) = partition_lan_containers(containers, &lan);

        assert_eq!(lan_containers.len(), 1);
        let members: Vec<Option<&str>> = lan_containers[0]
            .members
            .iter()
            .map(|(c, _)| c.id.as_deref())
            .collect();
        assert_eq!(members, vec![Some("eeee3333ffff")]);

        let stays: Vec<Option<&str>> = on_host.iter().map(|(c, _)| c.id.as_deref()).collect();
        assert_eq!(stays, vec![Some("cccc2222dddd"), Some("9999888877776666")]);
    }

    /// A container whose only LAN endpoint has no address has nothing for the network to find,
    /// so it stays off the host path.
    #[test]
    fn a_lan_endpoint_without_an_address_is_no_host() {
        let lan_network = subnet("lan", "192.168.1.0/24", SubnetType::MacVlan);
        let bridge = subnet("app_default", "172.18.0.0/16", SubnetType::DockerBridge);
        let mut lan_endpoint = endpoint("", None, None);
        lan_endpoint.ip_address = None;
        let c = container(
            "a",
            "web",
            vec![
                ("app_default", endpoint("172.18.0.2", None, None)),
                ("lan", lan_endpoint),
            ],
        );

        assert!(
            container_host_record(
                ContainerRuntime::Podman,
                &c,
                ContainerNetworkType::MacVlan,
                std::slice::from_ref(&lan_network),
                std::slice::from_ref(&bridge),
                &[],
                Uuid::new_v4(),
                Uuid::new_v4(),
            )
            .is_none()
        );
    }
}
