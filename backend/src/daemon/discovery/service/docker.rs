use anyhow::anyhow;
use anyhow::{Error, Result};
use async_trait::async_trait;
use bollard::{
    Docker,
    models::{ContainerInspectResponse, ContainerSummary, PortSummaryTypeEnum},
    query_parameters::{InspectContainerOptions, ListContainersOptions, ListNetworksOptions},
};
use futures::future::try_join_all;
use futures::stream::{self, StreamExt};
use mac_address::MacAddress;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::{collections::HashMap, net::IpAddr, sync::OnceLock};
use strum::IntoDiscriminant;
use tokio_util::sync::CancellationToken;

use crate::daemon::discovery::service::base::RunsDiscovery;
use crate::daemon::utils::base::{DaemonUtils, merge_host_and_docker_subnets};
use crate::daemon::utils::scanner::scan_endpoints;
use crate::server::bindings::r#impl::base::{Binding, BindingDiscriminants};
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback};
use crate::server::hosts::r#impl::base::HostBase;
use crate::server::interfaces::r#impl::base::ALL_INTERFACES_IP;
use crate::server::ports::r#impl::base::Port;
use crate::server::services::r#impl::base::{Service, ServiceBase, ServiceMatchBaselineParams};
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::endpoints::{Endpoint, EndpointResponse};
use crate::server::services::r#impl::patterns::MatchDetails;
use crate::server::services::r#impl::virtualization::{
    DockerVirtualization, ServiceVirtualization,
};
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::{DiscoveryMetadata, EntitySource};
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::types::SubnetTypeDiscriminants;
use crate::{
    daemon::discovery::service::base::{
        CreatesDiscoveredEntities, DiscoversNetworkedEntities, DiscoveryRunner,
    },
    server::{
        daemons::r#impl::api::DaemonDiscoveryRequest,
        hosts::r#impl::base::Host,
        interfaces::r#impl::base::{Interface, InterfaceBase},
        ports::r#impl::base::PortType,
    },
};
use uuid::Uuid;

type IpPortHashMap = HashMap<IpAddr, Vec<PortType>>;

pub struct DockerScanDiscovery {
    pub docker_client: OnceLock<Docker>,
    pub host_id: Uuid,
    pub docker_service_id: OnceLock<Uuid>,
    pub host_naming_fallback: HostNamingFallback,
    /// IP address of the Docker host. Used for endpoint probing when containers
    /// publish ports on 0.0.0.0. For local scanning this is the daemon's own IP;
    /// for remote scanning this is the remote host's IP from the credential.
    pub host_ip: IpAddr,
}

pub struct ProcessContainerParams<'a> {
    pub containers_interfaces_and_subnets: &'a HashMap<String, Vec<(Interface, Subnet)>>,
    pub container: &'a ContainerInspectResponse,
    pub container_summary: &'a ContainerSummary,
    pub docker_service_id: &'a Uuid,
    pub cancel: CancellationToken,
}

#[async_trait]
impl RunsDiscovery for DiscoveryRunner<DockerScanDiscovery> {
    fn discovery_type(&self) -> DiscoveryType {
        DiscoveryType::Docker {
            host_id: self.domain.host_id,
            host_naming_fallback: self.domain.host_naming_fallback,
        }
    }

    async fn discover(
        &self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let docker_proxy = self.as_ref().config_store.get_docker_proxy().await;
        let docker_proxy_ssl_info = self.as_ref().config_store.get_docker_proxy_ssl_info().await;

        // Pre-start work: connect to Docker and list containers
        // If this fails, report it to the server instead of silently swallowing
        let (_docker, container_list) = match async {
            let docker = self
                .as_ref()
                .utils
                .new_docker_client(docker_proxy, docker_proxy_ssl_info)
                .await?;
            self.domain
                .docker_client
                .set(docker.clone())
                .map_err(|_| anyhow!("Failed to set docker client"))?;
            let container_list = self.get_containers_to_scan().await?;
            Ok::<_, Error>((docker, container_list))
        }
        .await
        {
            Ok(result) => result,
            Err(e) => {
                // Pre-start failure: initialize a minimal session so we can report the error
                if let Err(init_err) = self.initialize_discovery_session(request, daemon_id).await {
                    tracing::error!(
                        "Failed to initialize session for error reporting: {}",
                        init_err
                    );
                    return Err(e);
                }
                self.finish_discovery(Err(e), cancel).await?;
                return Ok(());
            }
        };

        self.start_discovery(request).await?;

        // Get and create docker and host subnets
        let subnets = self.discover_create_subnets(&cancel).await?;

        // Get host interfaces (needed for docker daemon service host matching)
        let interface_filter = self.as_ref().config_store.get_interfaces().await?;
        let (mut host_interfaces, _, _) = self
            .as_ref()
            .utils
            .get_own_interfaces(
                self.discovery_type(),
                daemon_id,
                network_id,
                &interface_filter,
            )
            .await?;

        // Update interface subnet IDs to match created subnets (they may differ if subnets already existed)
        for interface in &mut host_interfaces {
            if let Some(subnet) = subnets
                .iter()
                .find(|s| s.base.cidr.contains(&interface.base.ip_address))
            {
                interface.base.subnet_id = subnet.id;
            }
        }

        // Create service for docker daemon (pass interfaces for proper host matching)
        let (_, services) = self
            .create_docker_daemon_service(&host_interfaces, &cancel)
            .await?;

        let docker_daemon_service = services
            .first()
            .ok_or_else(|| anyhow!("Docker daemon service was not created, aborting"))?;

        // Set docker_service_id on domain for scan_and_process_containers
        self.domain
            .docker_service_id
            .set(docker_daemon_service.id)
            .map_err(|_| anyhow!("Docker service ID already set"))?;

        // Get container info
        let containers = self.get_containers_and_summaries().await?;

        // Combine host interfaces + subnets to get a map of containers to the interfaces they have + subnets those interfaces are for
        let containers_interfaces_and_subnets =
            self.get_container_interfaces(&containers, &subnets, &mut host_interfaces);

        let discovered_hosts_services = self
            .scan_and_process_containers(
                cancel.clone(),
                containers,
                &containers_interfaces_and_subnets,
                Arc::new(AtomicU8::new(0)),
            )
            .await;

        if let Ok(ref container_data) = discovered_hosts_services {
            tracing::info!(
                total_containers = %container_list.len(),
                discovered = %container_data.len(),
                "Docker scan complete"
            );
        }

        let discovery_result = if discovered_hosts_services.is_ok() {
            Ok(())
        } else {
            Err(anyhow::Error::msg(""))
        };

        self.finish_discovery(discovery_result, cancel.clone())
            .await?;

        Ok(())
    }
}

impl DockerScanDiscovery {
    pub fn new(
        host_id: Uuid,
        docker_service_id: Uuid,
        host_naming_fallback: HostNamingFallback,
        host_ip: IpAddr,
    ) -> Self {
        let service_id_lock = OnceLock::new();
        let _ = service_id_lock.set(docker_service_id);
        Self {
            docker_client: OnceLock::new(),
            host_id,
            docker_service_id: service_id_lock,
            host_naming_fallback,
            host_ip,
        }
    }

    pub fn new_deferred(
        host_id: Uuid,
        host_naming_fallback: HostNamingFallback,
        host_ip: IpAddr,
    ) -> Self {
        Self {
            docker_client: OnceLock::new(),
            host_id,
            docker_service_id: OnceLock::new(),
            host_naming_fallback,
            host_ip,
        }
    }
}

impl CreatesDiscoveredEntities for DiscoveryRunner<DockerScanDiscovery> {}

#[async_trait]
impl DiscoversNetworkedEntities for DiscoveryRunner<DockerScanDiscovery> {
    async fn get_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        let docker = self
            .domain
            .docker_client
            .get()
            .ok_or_else(|| anyhow!("Docker client unavailable"))?;

        let gateway_ips: Vec<IpAddr> = docker
            .list_networks(None::<ListNetworksOptions>)
            .await?
            .iter()
            .filter_map(|n| {
                if let Some(ipam) = &n.ipam
                    && let Some(config) = &ipam.config
                {
                    return Some(
                        config
                            .iter()
                            .filter_map(|c| c.gateway.as_ref())
                            .filter_map(|g| g.parse::<IpAddr>().ok())
                            .collect::<Vec<IpAddr>>(),
                    );
                }
                None
            })
            .flatten()
            .collect();

        Ok(gateway_ips)
    }

    async fn discover_create_subnets(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error> {
        let daemon_id = self.as_ref().config_store.get_id().await?;

        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let interface_filter = self.as_ref().config_store.get_interfaces().await?;
        let (_, host_subnets, _) = self
            .as_ref()
            .utils
            .get_own_interfaces(
                self.discovery_type(),
                daemon_id,
                network_id,
                &interface_filter,
            )
            .await?;

        let docker = self
            .domain
            .docker_client
            .get()
            .ok_or_else(|| anyhow!("Docker client unavailable"))?;

        let docker_subnets = self
            .as_ref()
            .utils
            .get_subnets_from_docker_networks(daemon_id, network_id, docker, self.discovery_type())
            .await?;

        // Merge host and Docker subnets — host subnets take precedence on CIDR overlap
        // DockerBridge subnets are kept here since Docker discovery owns them
        let subnets = merge_host_and_docker_subnets(host_subnets, docker_subnets);

        let subnet_futures = subnets
            .iter()
            .map(|subnet| self.create_subnet(subnet, cancel));
        let subnets = try_join_all(subnet_futures).await?;

        Ok(subnets)
    }
}

impl DiscoveryRunner<DockerScanDiscovery> {
    /// Create docker daemon service which has container relationship with docker daemon service
    /// Takes host_interfaces to enable proper host matching via MAC/IP addresses
    pub async fn create_docker_daemon_service(
        &self,
        host_interfaces: &[Interface],
        cancel: &CancellationToken,
    ) -> Result<(Host, Vec<Service>), Error> {
        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let host_id = self.domain.host_id;

        let docker_service_definition = crate::server::services::definitions::docker_daemon::Docker;

        let docker_service = Service::new(ServiceBase {
            name: ServiceDefinition::name(&docker_service_definition).to_string(),
            service_definition: Box::new(docker_service_definition),
            bindings: vec![],
            host_id,
            tags: Vec::new(),
            network_id,
            virtualization: None,
            source: EntitySource::DiscoveryWithMatch {
                metadata: vec![DiscoveryMetadata::new(
                    DiscoveryType::SelfReport { host_id },
                    daemon_id,
                )],
                details: MatchDetails::new_certain("Docker daemon self-report"),
            },
            position: 0,
        });

        let mut temp_docker_daemon_host = Host::new(HostBase {
            name: "Docker Daemon Host".to_string(),
            network_id,
            hostname: None,
            description: None,
            source: EntitySource::Discovery {
                metadata: vec![DiscoveryMetadata::new(self.discovery_type(), daemon_id)],
            },
            virtualization: None,
            hidden: false,
            tags: Vec::new(),
            // SNMP fields - not applicable to docker discovery
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            credential_assignments: vec![],
        });
        temp_docker_daemon_host.id = self.domain.host_id;

        // Pass host_interfaces separately - server will create them with the correct host_id
        let host_response = self
            .create_host(
                temp_docker_daemon_host,
                host_interfaces.to_vec(),
                vec![], // No ports for docker daemon host
                vec![docker_service],
                vec![], // No SNMP if_entries for docker discovery
                cancel,
            )
            .await?;

        Ok((host_response.to_host(), host_response.services))
    }

    /// Create Docker bridge subnets from Docker networks.
    /// Returns the created subnets (with server-assigned IDs) for use in container interface resolution.
    pub async fn create_docker_bridge_subnets(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error> {
        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let docker_client = self
            .domain
            .docker_client
            .get()
            .ok_or_else(|| anyhow::anyhow!("Docker client not set"))?;

        let host_id = self.domain.host_id;
        let discovery_type = DiscoveryType::Docker {
            host_id,
            host_naming_fallback: self.domain.host_naming_fallback,
        };

        let docker_subnets = self
            .as_ref()
            .utils
            .get_subnets_from_docker_networks(daemon_id, network_id, docker_client, discovery_type)
            .await
            .unwrap_or_default();

        let bridge_subnet_futures = docker_subnets
            .iter()
            .filter(|s| s.is_docker_bridge_subnet())
            .map(|s| self.create_subnet(s, cancel));

        let created_subnets: Vec<Subnet> = futures::future::join_all(bridge_subnet_futures)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        Ok(created_subnets)
    }

    pub async fn scan_and_process_containers(
        &self,
        cancel: CancellationToken,
        containers: Vec<(ContainerInspectResponse, ContainerSummary)>,
        containers_interfaces_and_subnets: &HashMap<String, Vec<(Interface, Subnet)>>,
        progress: Arc<AtomicU8>,
    ) -> Result<Vec<(Host, Vec<Service>)>> {
        let docker_service_id = self
            .domain
            .docker_service_id
            .get()
            .ok_or_else(|| anyhow!("Docker service ID not set"))?;
        let concurrent_scans = 15usize;
        let total = containers.len().max(1);

        // Process containers concurrently using streams
        let results = stream::iter(containers.into_iter())
            .map(|(container, container_summary)| {
                let cancel = cancel.clone();

                async move {
                    self.process_single_container(&ProcessContainerParams {
                        containers_interfaces_and_subnets,
                        container: &container,
                        container_summary: &container_summary,
                        docker_service_id,
                        cancel,
                    })
                    .await
                }
            })
            .buffer_unordered(concurrent_scans);

        let mut stream_pin = Box::pin(results);
        let mut all_container_data = Vec::new();
        let mut completed = 0usize;

        while let Some(result) = stream_pin.next().await {
            if cancel.is_cancelled() {
                tracing::warn!("Docker discovery session was cancelled");
                return Err(Error::msg("Docker discovery session was cancelled"));
            }

            completed += 1;
            progress.store(
                ((completed as f64 / total as f64) * 99.0) as u8,
                Ordering::Relaxed,
            );

            match result {
                Ok(Some((host, services))) => all_container_data.push((host, services)),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        phase = "container_processing",
                        "Container processing error"
                    );
                }
            }
        }

        Ok(all_container_data)
    }

    async fn process_single_container(
        &self,
        params: &ProcessContainerParams<'_>,
    ) -> Result<Option<(Host, Vec<Service>)>> {
        let ProcessContainerParams {
            container,
            container_summary,
            cancel,
            ..
        } = params;

        if let Some(container_id) = container.id.clone() {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            if container_id != container_summary.id.clone().unwrap_or_default() {
                tracing::warn!(
                    "Container inspection failure; inspected container does not match container summary"
                );
                return Ok(None);
            }

            let host_networking_mode = container
                .host_config
                .as_ref()
                .and_then(|c| c.network_mode.clone())
                .unwrap_or_default()
                == "host";

            if host_networking_mode {
                return self
                    .process_host_mode_container(params, &container_id)
                    .await;
            } else {
                return self
                    .process_bridge_mode_container(params, &container_id)
                    .await;
            }
        }

        Ok(None)
    }

    async fn process_host_mode_container(
        &self,
        params: &ProcessContainerParams<'_>,
        container_id: &String,
    ) -> Result<Option<(Host, Vec<Service>)>> {
        let ProcessContainerParams {
            containers_interfaces_and_subnets,
            container,
            cancel,
            docker_service_id,
            ..
        } = params;

        tracing::info!(
            "Processing host mode container {}",
            container
                .name
                .as_ref()
                .unwrap_or(&"Unknown Container Name".to_string())
        );

        let host_ip = self.domain.host_ip;

        let open_ports: Vec<PortType> = container
            .config
            .as_ref()
            .and_then(|c| c.exposed_ports.as_ref())
            .map(|p| {
                p.iter()
                    .filter_map(|v| PortType::from_str(v).ok())
                    .collect()
            })
            .unwrap_or_default();

        // Scan endpoints for exposed ports if any are declared
        let endpoint_responses = if !open_ports.is_empty() {
            let port_scan_batch_size = 200usize.clamp(16, 1000);
            let accept_invalid_certs = self
                .as_ref()
                .config_store
                .get_accept_invalid_scan_certs()
                .await?;
            tokio::spawn(scan_endpoints(
                host_ip,
                cancel.clone(),
                Some(open_ports.clone()),
                None,
                port_scan_batch_size,
                true,
                accept_invalid_certs,
            ))
            .await
            .map_err(|e| anyhow!("Scan task panicked: {}", e))?
            .map_err(|e| anyhow!("Endpoint scanning error: {}", e))?
        } else {
            vec![]
        };

        let empty_vec_ref = &vec![];

        let container_interfaces_and_subnets = containers_interfaces_and_subnets
            .get(container_id)
            .unwrap_or(empty_vec_ref);

        for (interface, subnet) in container_interfaces_and_subnets {
            let empty_client_responses = std::collections::HashMap::new();
            let params = ServiceMatchBaselineParams {
                subnet,
                interface,
                all_ports: &open_ports,
                endpoint_responses: &endpoint_responses,
                virtualization: &Some(ServiceVirtualization::Docker(DockerVirtualization {
                    container_name: container
                        .name
                        .clone()
                        .map(|n| n.trim_start_matches("/").to_string()),
                    container_id: container.id.clone(),
                    service_id: **docker_service_id,
                })),
                client_responses: &empty_client_responses,
            };

            if let Ok(Some((mut host, interfaces, ports, services))) = self
                .process_host(params, None, self.domain.host_naming_fallback)
                .await
            {
                host.id = self.domain.host_id;

                if let Ok(host_response) = self
                    .create_host(host, interfaces, ports, services, vec![], cancel)
                    .await
                {
                    return Ok::<Option<(Host, Vec<Service>)>, Error>(Some((
                        host_response.to_host(),
                        host_response.services,
                    )));
                }
                return Ok(None);
            }
        }
        Ok(None)
    }

    async fn process_bridge_mode_container(
        &self,
        params: &ProcessContainerParams<'_>,
        container_id: &String,
    ) -> Result<Option<(Host, Vec<Service>)>> {
        let ProcessContainerParams {
            containers_interfaces_and_subnets,
            container,
            container_summary,
            cancel,
            docker_service_id,
            ..
        } = params;

        tracing::info!(
            "Processing bridge mode container {}",
            container
                .name
                .as_ref()
                .unwrap_or(&"Unknown Container Name".to_string())
        );

        let empty_vec_ref = &vec![];

        let container_interfaces_and_subnets = containers_interfaces_and_subnets
            .get(container_id)
            .unwrap_or(empty_vec_ref);

        let (host_ip_to_host_ports, container_ips_to_container_ports, host_to_container_port_map) =
            self.get_ports_from_container(container_summary, container_interfaces_and_subnets);

        if container_interfaces_and_subnets.is_empty() {
            tracing::warn!(
                container = ?container.name,
                "No interfaces found for bridge container - Docker bridge subnets may not have been created"
            );
            return Ok(None);
        }

        for (interface, subnet) in container_interfaces_and_subnets {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            let mut endpoint_responses = if let Some(name) = &container.name {
                self.scan_container_endpoints(
                    interface,
                    &host_to_container_port_map,
                    name.trim_start_matches("/"),
                    cancel.clone(),
                )
                .await?
            } else {
                vec![]
            };

            // Always try external probing when published ports exist. Exec-based results
            // may contain partial responses (e.g., from bash /dev/tcp raw sockets) that
            // don't match specific service patterns but would prevent a fallback-only
            // approach from firing. Merging both sets gives the pattern matcher the best
            // chance of identifying the specific service.
            if !host_to_container_port_map.is_empty() {
                let accept_invalid_certs = self
                    .as_ref()
                    .config_store
                    .get_accept_invalid_scan_certs()
                    .await?;
                let external_responses = self
                    .scan_container_endpoints_external(
                        interface,
                        &host_to_container_port_map,
                        cancel.clone(),
                        accept_invalid_certs,
                    )
                    .await?;
                if !external_responses.is_empty() {
                    tracing::debug!(
                        "External endpoint probing found {} responses for container at {}",
                        external_responses.len(),
                        interface.base.ip_address
                    );
                    endpoint_responses.extend(external_responses);
                }
            }

            if !endpoint_responses.is_empty() {
                tracing::debug!(
                    "Found {} endpoint responses for container at {}",
                    endpoint_responses.len(),
                    interface.base.ip_address
                );
            }

            let empty_vec_ref: &Vec<_> = &Vec::new();
            let container_ports_on_interface = container_ips_to_container_ports
                .get(&interface.base.ip_address)
                .unwrap_or(empty_vec_ref);

            let empty_client_responses = std::collections::HashMap::new();
            if let Ok(Some((mut host, mut interfaces, mut ports, mut services))) = self
                .process_host(
                    ServiceMatchBaselineParams {
                        subnet,
                        interface,
                        all_ports: container_ports_on_interface,
                        endpoint_responses: &endpoint_responses,
                        virtualization: &Some(ServiceVirtualization::Docker(
                            DockerVirtualization {
                                container_name: container
                                    .name
                                    .clone()
                                    .map(|n| n.trim_start_matches("/").to_string()),
                                container_id: container.id.clone(),
                                service_id: **docker_service_id,
                            },
                        )),
                        client_responses: &empty_client_responses,
                    },
                    None,
                    self.domain.host_naming_fallback,
                )
                .await
            {
                // Add information that we have from docker context to processed host + services

                host.id = self.domain.host_id;

                // Add all interfaces relevant to container to the interfaces vec
                container_interfaces_and_subnets.iter().for_each(|(i, _)| {
                    if !interfaces.contains(i) {
                        interfaces.push(i.clone())
                    }
                });

                let docker_bridge_subnet_ids: Vec<Uuid> = container_interfaces_and_subnets
                    .iter()
                    .filter(|(_, subnet)| {
                        subnet.base.subnet_type.discriminant()
                            == SubnetTypeDiscriminants::DockerBridge
                    })
                    .map(|(_, subnet)| subnet.id)
                    .collect();

                services.iter_mut().for_each(|s| {
                    // Add all host port + IPs and any container ports which weren't matched
                    // We know they are open on this host even if no services matched them
                    container_ports_on_interface
                        .iter()
                        .for_each(|container_port| {
                            // Add bindings for container ports which weren't matched
                            match ports.iter().find(|p| p.base.port_type == *container_port) {
                                Some(unmatched_container_port)
                                    if !s
                                        .base
                                        .bindings
                                        .iter()
                                        .filter_map(|b| b.port_id())
                                        .any(|port_id| port_id == unmatched_container_port.id) =>
                                {
                                    s.base.bindings.push(Binding::new_port_serviceless(
                                        unmatched_container_port.id,
                                        Some(interface.id),
                                    ))
                                }
                                _ => (),
                            }
                        });

                    // Add bindings for all host ports, provided there's an interface
                    host_ip_to_host_ports.iter().for_each(|(ip, pbs)| {
                        pbs.iter().for_each(|pb| {
                            // If there's an existing port and existing non-docker bindings, they'll need to be replaced if listener is on all interfaces otherwise there'll be duplicate bindings
                            let (port, existing_non_docker_bindings) =
                                match ports.iter().find(|p| p.base.port_type == *pb) {
                                    // Port exists on host, so get IDs of existing non-Docker bridge service bindings
                                    Some(existing_port) => (
                                        *existing_port,
                                        s.base
                                            .bindings
                                            .iter()
                                            .filter_map(|b| {
                                                if let Some(port_id) = b.port_id()
                                                    && port_id == existing_port.id
                                                {
                                                    // Only include if it's NOT on a Docker bridge
                                                    // Look up interface in the interfaces vec
                                                    if let Some(interface_id) = b.interface_id()
                                                        && let Some(interface) = interfaces
                                                            .iter()
                                                            .find(|i| i.id == interface_id)
                                                        && !docker_bridge_subnet_ids
                                                            .contains(&interface.base.subnet_id)
                                                    {
                                                        return Some(b.id());
                                                    }
                                                }
                                                None
                                            })
                                            .collect(),
                                    ),
                                    // Port doesn't exist on host yet, so it can't have been bound by service
                                    None => (Port::new_hostless(*pb), vec![]),
                                };

                            // Get host interface from the interfaces vec
                            let host_interface =
                                interfaces.iter().find(|i| i.base.ip_address == *ip);

                            // Add binding to specific interface, or all interfaces if it's on ALL_INTERFACES_IP
                            match host_interface {
                                Some(host_interface) => {
                                    s.base.bindings.push(Binding::new_port_serviceless(
                                        port.id,
                                        Some(host_interface.id),
                                    ));
                                    ports.push(port);
                                }
                                None if *ip == ALL_INTERFACES_IP => {
                                    // Remove existing non-Docker bridge bindings for this port
                                    s.base.bindings = s
                                        .base
                                        .bindings
                                        .iter()
                                        .filter(|b| !existing_non_docker_bindings.contains(&b.id()))
                                        .cloned()
                                        .collect();

                                    // Add bindings for all non-Docker bridge interfaces
                                    // Use the interface ID from the `interfaces` list (not container_interfaces_and_subnets)
                                    // because Interface::eq deduplication at lines 617-621 may have matched
                                    // different interface objects with different UUIDs
                                    for (interface, subnet) in container_interfaces_and_subnets {
                                        if subnet.base.subnet_type.discriminant()
                                            != SubnetTypeDiscriminants::DockerBridge
                                        {
                                            // Find the matching interface in the interfaces list
                                            if let Some(matched_interface) =
                                                interfaces.iter().find(|i| *i == interface)
                                            {
                                                s.base.bindings.push(
                                                    Binding::new_port_serviceless(
                                                        port.id,
                                                        Some(matched_interface.id),
                                                    ),
                                                );
                                            }
                                        }
                                    }

                                    ports.push(port);
                                }
                                _ => {}
                            }
                        });
                    });

                    // Remove any interface bindings which are now superceded by port bindings
                    // (interface binding is implicit in port binding)
                    let interface_ids_with_port_binding: Vec<Uuid> = s
                        .base
                        .bindings
                        .clone()
                        .into_iter()
                        .filter_map(|b| {
                            if b.base.binding_type.discriminant() == BindingDiscriminants::Port
                                && let Some(interface_id) = b.interface_id()
                            {
                                return Some(interface_id);
                            }
                            None
                        })
                        .collect();

                    s.base.bindings.retain(|b| {
                        b.base.binding_type.discriminant() == BindingDiscriminants::Port
                            || !interface_ids_with_port_binding
                                .contains(&b.interface_id().unwrap_or_default())
                    });
                });

                if let Ok(host_response) = self
                    .create_host(host, interfaces, ports, services.clone(), vec![], cancel)
                    .await
                {
                    return Ok::<Option<(Host, Vec<Service>)>, Error>(Some((
                        host_response.to_host(),
                        host_response.services,
                    )));
                }
                return Ok(None);
            }
        }

        Ok(None)
    }

    pub async fn get_containers_to_scan(&self) -> Result<Vec<ContainerSummary>, Error> {
        let docker = self
            .domain
            .docker_client
            .get()
            .ok_or_else(|| anyhow!("Docker client unavailable"))?;

        docker
            .list_containers(None::<ListContainersOptions>)
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn get_containers_and_summaries(
        &self,
    ) -> Result<Vec<(ContainerInspectResponse, ContainerSummary)>, Error> {
        let docker = self
            .domain
            .docker_client
            .get()
            .ok_or_else(|| anyhow!("Docker client unavailable"))?;

        let container_summaries = self.get_containers_to_scan().await?;

        let containers_to_inspect: Vec<_> = container_summaries
            .iter()
            .filter_map(|c| {
                if let Some(id) = &c.id {
                    return Some(docker.inspect_container(id, None::<InspectContainerOptions>));
                }
                None
            })
            .collect();

        let inspected_containers: Vec<ContainerInspectResponse> =
            try_join_all(containers_to_inspect).await?;

        Ok(inspected_containers
            .into_iter()
            .zip(container_summaries)
            .collect())
    }

    async fn scan_container_endpoints(
        &self,
        interface: &Interface,
        host_to_container_port_map: &HashMap<(IpAddr, u16), u16>,
        container_name: &str,
        cancel: CancellationToken,
    ) -> Result<Vec<EndpointResponse>, Error> {
        use std::collections::HashMap;

        // Build inverse map: (container_port) -> Vec<(host_ip, host_port)>
        let mut container_to_host_port_map: HashMap<u16, Vec<(IpAddr, u16)>> = HashMap::new();
        for ((host_ip, host_port), container_port) in host_to_container_port_map {
            container_to_host_port_map
                .entry(*container_port)
                .or_default()
                .push((*host_ip, *host_port));
        }

        let docker = self
            .domain
            .docker_client
            .get()
            .ok_or_else(|| anyhow!("Docker client unavailable"))?;

        let all_endpoints = Service::all_discovery_endpoints();

        let mut endpoint_responses = Vec::new();

        for endpoint in all_endpoints {
            if cancel.is_cancelled() {
                tracing::debug!(
                    "Container endpoint scanning cancelled for {}",
                    container_name
                );
                break;
            }

            // Build command with multiple fallback options
            // Test both HTTP and HTTPS
            let requests = [
                // curl - HTTP
                format!(
                    "curl -i -s -m 1 -L --max-redirs 2 http://127.0.0.1:{}{}",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // curl - HTTPS (with -k for self-signed certs)
                format!(
                    "curl -k -i -s -m 1 -L --max-redirs 2 https://127.0.0.1:{}{}",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // wget - HTTP
                format!(
                    "wget -S -q -O- -T 1 http://127.0.0.1:{}{}",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // wget - HTTPS (with --no-check-certificate)
                format!(
                    "wget --no-check-certificate -S -q -O- -T 1 https://127.0.0.1:{}{}",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // Python - HTTP
                format!(
                    "python3 -c \"import urllib.request; req = urllib.request.Request('http://127.0.0.1:{}{}'); \
                    exec(\\\"try:\\\\n resp = urllib.request.urlopen(req)\\\\n print('HTTP/1.1', resp.status, resp.msg)\\\\n \
                    for h in resp.headers: print(h + ':', resp.headers[h])\\\\n print()\\\\n \
                    print(resp.read().decode('utf-8'))\\\\nexcept urllib.error.HTTPError as e:\\\\n \
                    print('HTTP/1.1', e.code, e.msg)\\\\n for h in e.headers: print(h + ':', e.headers[h])\\\\n \
                    print()\\\\n print(e.read().decode('utf-8'))\\\")\"",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // Python - HTTPS (with unverified SSL context)
                format!(
                    "python3 -c \"import urllib.request, ssl; \
                    ctx = ssl._create_unverified_context(); \
                    req = urllib.request.Request('https://127.0.0.1:{}{}'); \
                    exec(\\\"try:\\\\n resp = urllib.request.urlopen(req, context=ctx)\\\\n print('HTTP/1.1', resp.status, resp.msg)\\\\n \
                    for h in resp.headers: print(h + ':', resp.headers[h])\\\\n print()\\\\n \
                    print(resp.read().decode('utf-8'))\\\\nexcept urllib.error.HTTPError as e:\\\\n \
                    print('HTTP/1.1', e.code, e.msg)\\\\n for h in e.headers: print(h + ':', e.headers[h])\\\\n \
                    print()\\\\n print(e.read().decode('utf-8'))\\\")\"",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // bash /dev/tcp - only supports HTTP (no TLS)
                format!(
                    "bash -c \"exec 3<>/dev/tcp/127.0.0.1/{} && echo -e 'GET {} HTTP/1.0\\r\\nHost: 127.0.0.1\\r\\n\\r\\n' >&3 && cat <&3\"",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
            ];

            // Join with || to try each in order, fallback to empty string
            let command = format!("{} || echo ''", requests.join(" 2>/dev/null || "));

            // Execute curl with command that works for environment
            let exec = docker
                .create_exec(
                    container_name,
                    bollard::exec::CreateExecOptions {
                        cmd: Some(vec!["sh", "-c", &command]),
                        attach_stdout: Some(true),
                        attach_stderr: Some(true),
                        ..Default::default()
                    },
                )
                .await;

            let Ok(exec_result) = exec else {
                continue;
            };

            if let Ok(bollard::exec::StartExecResults::Attached { mut output, .. }) =
                docker.start_exec(&exec_result.id, None).await
            {
                use futures::StreamExt;
                let mut full_response = String::new();

                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => {
                            tracing::debug!("Endpoint scan cancelled for container {}", container_name);
                            break;
                        }
                        msg = output.next() => {
                            match msg {
                                Some(Ok(bollard::container::LogOutput::StdOut { message })) => {
                                    full_response.push_str(&String::from_utf8_lossy(&message));
                                }
                                Some(Ok(bollard::container::LogOutput::StdErr { message })) => {
                                    // wget outputs headers to stderr with -S flag
                                    full_response.push_str(&String::from_utf8_lossy(&message));
                                }
                                Some(Ok(_)) => {}
                                Some(Err(e)) => {
                                    tracing::warn!("Error reading docker exec output: {}", e);
                                    break;
                                }
                                None => break,
                            }
                        }
                    }
                }

                let full_response = full_response.trim();

                // Parse response to check status code and extract body
                if let Some((status, body, headers)) = Self::parse_http_response(full_response) {
                    // Map back to the host-visible endpoint
                    if let Some(host_mappings) =
                        container_to_host_port_map.get(&endpoint.port_type.number())
                    {
                        for (host_ip, host_port) in host_mappings {
                            let host_endpoint = Endpoint {
                                ip: Some(*host_ip),
                                port_type: PortType::new_tcp(*host_port),
                                protocol: endpoint.protocol,
                                path: endpoint.path.clone(),
                            };

                            endpoint_responses.push(EndpointResponse {
                                endpoint: host_endpoint,
                                body: body.clone(),
                                status,
                                headers: headers.clone(),
                            });
                        }
                    }

                    // Also add the container-internal endpoint
                    let container_endpoint = Endpoint {
                        ip: Some(interface.base.ip_address), // Container's IP on the bridge network
                        port_type: PortType::new_tcp(endpoint.port_type.number()), // Container port, not host port
                        protocol: endpoint.protocol,
                        path: endpoint.path.clone(),
                    };

                    endpoint_responses.push(EndpointResponse {
                        endpoint: container_endpoint,
                        body: body.clone(),
                        status,
                        headers: headers.clone(),
                    });
                }
            }
        }

        Ok(endpoint_responses)
    }

    /// Fallback endpoint scanning for bridge-mode containers that lack HTTP tools.
    /// Probes host-published ports externally via reqwest, then remaps responses
    /// back to container ports for the pattern matcher.
    async fn scan_container_endpoints_external(
        &self,
        interface: &Interface,
        host_to_container_port_map: &HashMap<(IpAddr, u16), u16>,
        cancel: CancellationToken,
        accept_invalid_certs: bool,
    ) -> Result<Vec<EndpointResponse>, Error> {
        // Build inverse map: container_port -> Vec<(host_ip, host_port)>
        let mut container_to_host_port_map: HashMap<u16, Vec<(IpAddr, u16)>> = HashMap::new();
        for ((host_ip, host_port), container_port) in host_to_container_port_map {
            container_to_host_port_map
                .entry(*container_port)
                .or_default()
                .push((*host_ip, *host_port));
        }

        let all_endpoints = Service::all_discovery_endpoints();

        // Filter to endpoints whose port matches a container port with a host mapping
        let probeable_endpoints: Vec<_> = all_endpoints
            .into_iter()
            .filter(|e| container_to_host_port_map.contains_key(&e.port_type.number()))
            .collect();

        if probeable_endpoints.is_empty() {
            return Ok(vec![]);
        }

        let client = reqwest::Client::builder()
            .connect_timeout(crate::daemon::utils::scanner::SCAN_TIMEOUT)
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()
            .map_err(|e| anyhow!("Could not build client: {}", e))?;

        let mut endpoint_responses = Vec::new();

        for endpoint in &probeable_endpoints {
            if cancel.is_cancelled() {
                break;
            }

            let Some(host_mappings) = container_to_host_port_map.get(&endpoint.port_type.number())
            else {
                continue;
            };

            for (host_ip, host_port) in host_mappings {
                if cancel.is_cancelled() {
                    break;
                }

                // Resolve 0.0.0.0 to the Docker host's IP
                let probe_ip = if *host_ip == ALL_INTERFACES_IP {
                    self.domain.host_ip
                } else {
                    *host_ip
                };

                // Try HTTP and HTTPS, same pattern as scan_endpoints in scanner.rs
                let http_url = format!("http://{}:{}{}", probe_ip, host_port, endpoint.path);
                let https_url = format!("https://{}:{}{}", probe_ip, host_port, endpoint.path);

                let urls = [http_url, https_url];

                for url in &urls {
                    tracing::trace!("Docker external probe: {}", url);

                    // Timeout covers connect + headers; body has its own deadline.
                    match tokio::time::timeout(
                        crate::daemon::utils::scanner::SCAN_TIMEOUT,
                        client.get(url).send(),
                    )
                    .await
                    {
                        Ok(Ok(response)) => {
                            let status = response.status().as_u16();
                            let headers: HashMap<String, String> = response
                                .headers()
                                .iter()
                                .filter_map(|(name, value)| {
                                    value
                                        .to_str()
                                        .ok()
                                        .map(|v| (name.as_str().to_lowercase(), v.to_string()))
                                })
                                .collect();

                            let deadline = tokio::time::Instant::now()
                                + crate::daemon::utils::scanner::SCAN_TIMEOUT;
                            let body =
                                crate::daemon::utils::scanner::read_response_body_until_deadline(
                                    response, deadline,
                                )
                                .await;

                            tracing::debug!(
                                "Docker external probe {} returned {} (length: {})",
                                url,
                                status,
                                body.len()
                            );

                            // Container-port response for pattern matching
                            endpoint_responses.push(EndpointResponse {
                                endpoint: Endpoint {
                                    ip: Some(interface.base.ip_address),
                                    port_type: endpoint.port_type,
                                    protocol: endpoint.protocol,
                                    path: endpoint.path.clone(),
                                },
                                body: body.clone(),
                                status,
                                headers: headers.clone(),
                            });

                            // Host-port response for downstream binding logic
                            endpoint_responses.push(EndpointResponse {
                                endpoint: Endpoint {
                                    ip: Some(*host_ip),
                                    port_type: PortType::new_tcp(*host_port),
                                    protocol: endpoint.protocol,
                                    path: endpoint.path.clone(),
                                },
                                body,
                                status,
                                headers,
                            });

                            // Got a response, no need to try HTTPS
                            break;
                        }
                        Ok(Err(e)) => {
                            tracing::trace!("Docker external probe {} failed: {}", url, e);
                            continue;
                        }
                        Err(_) => {
                            tracing::trace!(
                                "Docker external probe {} timed out waiting for headers",
                                url
                            );
                            continue;
                        }
                    }
                }
            }
        }

        Ok(endpoint_responses)
    }

    /// Parse HTTP response to extract status code and body
    /// Returns (status_code, body) if successful
    fn parse_http_response(response: &str) -> Option<(u16, String, HashMap<String, String>)> {
        if response.is_empty() {
            return None;
        }

        let response_bytes = response.as_bytes();

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut parsed_response = httparse::Response::new(&mut headers);

        match parsed_response.parse(response_bytes) {
            Ok(httparse::Status::Complete(headers_len)) => {
                let status = parsed_response.code?;
                let body = &response_bytes[headers_len..];
                let body = String::from_utf8_lossy(body).to_string();
                let headers: HashMap<String, String> = parsed_response
                    .headers
                    .iter()
                    .filter_map(|header| {
                        // Convert header value bytes to string
                        std::str::from_utf8(header.value).ok().map(|value| {
                            (
                                header.name.to_lowercase(), // Normalize to lowercase
                                value.to_string(),
                            )
                        })
                    })
                    .collect();

                Some((status, body, headers))
            }
            Ok(httparse::Status::Partial) => {
                // Not enough data, might be incomplete response
                tracing::debug!("Partial HTTP response received");
                None
            }
            Err(_) => None,
        }
    }

    fn get_ports_from_container(
        &self,
        container_summary: &ContainerSummary,
        container_interfaces_and_subnets: &[(Interface, Subnet)],
    ) -> (IpPortHashMap, IpPortHashMap, HashMap<(IpAddr, u16), u16>) {
        let mut host_ip_to_host_ports: IpPortHashMap = HashMap::new();
        let mut container_ips_to_container_ports: IpPortHashMap = HashMap::new();
        let mut host_to_container_port_map: HashMap<(IpAddr, u16), u16> = HashMap::new();

        let container_ips: Vec<IpAddr> = container_interfaces_and_subnets
            .iter()
            .map(|(i, _)| i.base.ip_address)
            .collect();

        if let Some(ports) = &container_summary.ports {
            ports.iter().for_each(|p| {
                // Handle ports regardless of whether ip is set
                if let Some(port_type @ (PortSummaryTypeEnum::TCP | PortSummaryTypeEnum::UDP)) =
                    p.typ
                {
                    let private_port = match port_type {
                        PortSummaryTypeEnum::TCP => PortType::new_tcp(p.private_port),
                        PortSummaryTypeEnum::UDP => PortType::new_udp(p.private_port),
                        _ => unreachable!("Already matched TCP/UDP in outer pattern"),
                    };

                    // Always add the private port to all container IPs
                    container_ips.iter().for_each(|ip| {
                        container_ips_to_container_ports
                            .entry(*ip)
                            .or_default()
                            .push(private_port);
                    });

                    // Only handle host port mapping if we have both ip and public_port
                    if let (Some(ip_str), Some(public)) = (&p.ip, p.public_port)
                        && let Ok(ip) = ip_str.parse::<IpAddr>()
                    {
                        let public_port = match port_type {
                            PortSummaryTypeEnum::TCP => PortType::new_tcp(public),
                            PortSummaryTypeEnum::UDP => PortType::new_udp(public),
                            _ => unreachable!("Already matched TCP/UDP in outer pattern"),
                        };

                        host_ip_to_host_ports
                            .entry(ip)
                            .or_default()
                            .push(public_port);

                        host_to_container_port_map.insert((ip, public), p.private_port);
                    }
                }
            });
        }

        (
            host_ip_to_host_ports,
            container_ips_to_container_ports,
            host_to_container_port_map,
        )
    }

    pub fn get_container_interfaces(
        &self,
        containers: &[(ContainerInspectResponse, ContainerSummary)],
        subnets: &[Subnet],
        host_interfaces: &mut [Interface],
    ) -> HashMap<String, Vec<(Interface, Subnet)>> {
        // Created subnets may differ from discovered if there are existing subnets with the same CIDR, so we need to update interface subnet_id references
        let host_interfaces_and_subnets = host_interfaces
            .iter_mut()
            .filter_map(|i| {
                if let Some(subnet) = subnets
                    .iter()
                    .find(|s| s.base.cidr.contains(&i.base.ip_address))
                {
                    i.base.subnet_id = subnet.id;

                    return Some((i.clone(), subnet.clone()));
                }

                None
            })
            .collect::<Vec<(Interface, Subnet)>>();

        // Collect interfaces from containers
        containers
            .iter()
            .filter_map(|(container, _)| {
                let host_networking_mode = container
                    .host_config
                    .as_ref()
                    .and_then(|c| c.network_mode.clone())
                    .unwrap_or_default()
                    == "host";

                let mut interfaces_and_subnets: Vec<(Interface, Subnet)> = if host_networking_mode {
                    host_interfaces_and_subnets.clone()
                }
                // Containers not in host networking mode
                else if let Some(network_settings) = &container.network_settings {
                    if let Some(networks) = &network_settings.networks {
                        networks
                            .iter()
                            .filter_map(|(network_name, endpoint)| {
                                // Parse interface if IP
                                if let Some(ip_string) = &endpoint.ip_address {
                                    let ip_address = ip_string.parse::<IpAddr>().ok();

                                    if let Some(ip_address) = ip_address
                                        && let Some(subnet) = subnets
                                            .iter()
                                            .find(|s| s.base.cidr.contains(&ip_address))
                                    {
                                        // Parse MAC address from Docker network endpoint
                                        let mac_address = endpoint
                                            .mac_address
                                            .as_ref()
                                            .and_then(|mac_str| mac_str.parse::<MacAddress>().ok());

                                        return Some((
                                            Interface::new(InterfaceBase {
                                                network_id: subnet.base.network_id,
                                                host_id: Uuid::nil(), // Placeholder - server will set correct host_id
                                                subnet_id: subnet.id,
                                                ip_address,
                                                mac_address,
                                                name: Some(network_name.to_owned()),
                                                position: 0,
                                            }),
                                            subnet.clone(),
                                        ));
                                    }
                                }
                                tracing::warn!(
                                    "No matching subnet found for container {:?} on network '{}'",
                                    container.name,
                                    network_name
                                );

                                None
                            })
                            .collect::<Vec<(Interface, Subnet)>>()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Merge in host interfaces
                interfaces_and_subnets.extend(host_interfaces_and_subnets.clone());

                container
                    .id
                    .as_ref()
                    .map(|id| (id.clone(), interfaces_and_subnets))
            })
            .collect()
    }
}
