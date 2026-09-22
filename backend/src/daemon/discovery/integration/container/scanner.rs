use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::shared::attribution::AttributeSource;
use anyhow::{Error, Result, anyhow};
use bollard::{
    Docker,
    models::{ContainerInspectResponse, ContainerSummary, PortSummaryTypeEnum},
    query_parameters::{InspectContainerOptions, ListContainersOptions},
};
use futures::stream::{self, StreamExt};
use mac_address::MacAddress;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
};
use strum::IntoDiscriminant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{DaemonUtils, PlatformDaemonUtils};
use crate::daemon::utils::scanner::scan_endpoints;
use crate::server::bindings::r#impl::base::{Binding, BindingDiscriminants};
use crate::server::discovery::r#impl::types::HostNamingFallback;
use crate::server::ip_addresses::r#impl::base::{ALL_IP_ADDRESSES_IP, IPAddress, IPAddressBase};
use crate::server::ports::r#impl::base::{Port, PortType};
use crate::server::services::r#impl::base::{Service, ServiceMatchBaselineParams};
use crate::server::services::r#impl::endpoints::{ApplicationProtocol, Endpoint, EndpointResponse};
use crate::server::subnets::r#impl::base::Subnet;

use super::ContainerRuntime;

type IpPortHashMap = HashMap<IpAddr, Vec<PortType>>;

/// How many containers are inspected or scanned at once.
///
/// One socket serves all of them, so this bounds concurrent requests against a single container
/// daemon rather than expressing any parallelism the host has.
const CONCURRENT_CONTAINER_SCANS: usize = 15;

/// Bytes kept per probed endpoint. Service patterns match on a page's opening markup and headers,
/// so this is far more than enough, and it stops one batch dragging megabytes of HTML back
/// through the container socket.
const PROBE_BODY_LIMIT: usize = 65536;

/// How long an exec inside a container gets to finish writing its output. The commands run are a
/// `cat` of the socket tables and a batch of loopback HTTP probes; both finish in seconds.
const EXEC_OUTPUT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// One `(port, path)` a container answered on, before it is attributed to any address.
///
/// The probe runs over loopback inside the container, so its answers are the same whichever of
/// the container's addresses you ask about. Keeping them address-free until
/// [`ContainerScanner::attribute_to_address`] is what lets the probe run once per container
/// rather than once per attached bridge network.
pub(crate) struct ProbedEndpoint {
    pub port: u16,
    pub path: String,
    pub protocol: ApplicationProtocol,
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

/// Result of scanning a single container — services, ports, and ip_addresses
/// to be merged into the parent host's HostData rather than creating separate host entities.
pub struct ContainerScanResult {
    pub services: Vec<Service>,
    pub ports: Vec<Port>,
    pub ip_addresses: Vec<IPAddress>,
}

/// What a container sweep got through, and how far it got.
///
/// `reached` is separate from `results.len()` because a container can be reached and yield
/// nothing (no interfaces, no match), and only the caller comparing `reached` against the total
/// can tell "scanned everything, found little" from "ran out of time".
pub struct ContainerScanOutcome {
    pub results: Vec<ContainerScanResult>,
    pub reached: usize,
}

pub struct ProcessContainerParams<'a> {
    pub containers_interfaces_and_subnets: &'a HashMap<String, Vec<(IPAddress, Subnet)>>,
    pub container: &'a ContainerInspectResponse,
    pub container_summary: &'a ContainerSummary,
    pub runtime_service_id: &'a Uuid,
    pub cancel: CancellationToken,
}

/// Scans a container runtime (Docker or Podman) over its Docker-compatible API.
/// `runtime` selects the virtualization variants stamped onto discovered
/// services and subnets.
pub struct ContainerScanner<'a> {
    pub runtime: ContainerRuntime,
    pub client: &'a Docker,
    pub runtime_service_id: Uuid,
    pub host_ip: IpAddr,
    pub host_naming_fallback: HostNamingFallback,
    pub ops: &'a DiscoveryOps,
    pub cancel: &'a CancellationToken,
    pub accept_invalid_certs: bool,
    pub utils: &'a PlatformDaemonUtils,
}

impl<'a> ContainerScanner<'a> {
    /// Create bridge subnets from the runtime's networks.
    /// Returns the bridge subnets locally for use in container interface resolution.
    pub async fn create_bridge_subnets(&self) -> Result<Vec<Subnet>, Error> {
        let network_id = self.ops.network_id().await?;

        let subnets = self
            .utils
            .get_subnets_from_docker_networks(
                network_id,
                self.client,
                self.runtime,
                self.runtime_service_id,
            )
            .await
            .unwrap_or_else(|e| {
                // A failed/mis-deserialized networks listing silently empties bridge
                // subnets, degrading container→subnet mapping. Log it rather than swallow.
                tracing::warn!(
                    runtime = self.runtime.label(),
                    error = %e,
                    "Failed to list container networks; bridge subnets will be empty"
                );
                Vec::new()
            });

        // Return bridge subnets locally — they'll be created on the server
        // during create_host after service dedup (so service_id can be patched)
        Ok(subnets
            .into_iter()
            .filter(|s| s.is_container_bridge_subnet())
            .collect())
    }

    pub async fn scan_and_process_containers(
        &self,
        containers: Vec<(ContainerInspectResponse, ContainerSummary)>,
        containers_interfaces_and_subnets: &HashMap<String, Vec<(IPAddress, Subnet)>>,
        progress: Arc<AtomicU8>,
        deadline: tokio::time::Instant,
    ) -> Result<ContainerScanOutcome> {
        let containers_len = containers.len();
        let total = containers_len.max(1);

        // Process containers concurrently using streams
        let results = stream::iter(containers.into_iter())
            .map(|(container, container_summary)| {
                let cancel = self.cancel.clone();

                async move {
                    self.process_single_container(&ProcessContainerParams {
                        containers_interfaces_and_subnets,
                        container: &container,
                        container_summary: &container_summary,
                        runtime_service_id: &self.runtime_service_id,
                        cancel,
                    })
                    .await
                }
            })
            .buffer_unordered(CONCURRENT_CONTAINER_SCANS);

        let mut stream_pin = Box::pin(results);
        let mut all_container_data = Vec::new();
        let mut completed = 0usize;

        while let Some(result) = stream_pin.next().await {
            if self.cancel.is_cancelled() {
                tracing::warn!("Docker discovery session was cancelled");
                return Err(Error::msg("Docker discovery session was cancelled"));
            }

            completed += 1;
            progress.store(
                ((completed as f64 / total as f64) * 99.0) as u8,
                Ordering::Relaxed,
            );

            match result {
                Ok(Some(container_result)) => all_container_data.push(container_result),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        phase = "container_processing",
                        "Container processing error"
                    );
                }
            }

            // Stop at a container boundary rather than letting the integration's hard cap fire
            // mid-container. What we have is a coherent subset; what the cap produces is a
            // dropped future and nothing at all. The caller turns the shortfall into a warning.
            if tokio::time::Instant::now() >= deadline {
                tracing::warn!(
                    reached = completed,
                    total = containers_len,
                    runtime = self.runtime.label(),
                    "Container scan hit its soft deadline; reporting the containers read so far"
                );
                break;
            }
        }

        Ok(ContainerScanOutcome {
            results: all_container_data,
            reached: completed,
        })
    }

    async fn process_single_container(
        &self,
        params: &ProcessContainerParams<'_>,
    ) -> Result<Option<ContainerScanResult>> {
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
    ) -> Result<Option<ContainerScanResult>> {
        let ProcessContainerParams {
            containers_interfaces_and_subnets,
            container,
            cancel,
            runtime_service_id,
            ..
        } = params;

        tracing::info!(
            "Processing host mode container {}",
            container
                .name
                .as_ref()
                .unwrap_or(&"Unknown Container Name".to_string())
        );

        let host_ip = self.host_ip;

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
            let accept_invalid_certs = self.accept_invalid_certs;
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

        for (ip_address, subnet) in container_interfaces_and_subnets {
            let empty_client_responses = std::collections::HashMap::new();
            let params = ServiceMatchBaselineParams {
                subnet,
                ip_address,
                all_ports: &open_ports,
                endpoint_responses: &endpoint_responses,
                virtualization_metadata: &Some(
                    self.runtime.service_virtualization(
                        container
                            .name
                            .clone()
                            .map(|n| n.trim_start_matches("/").to_string()),
                        container.id.clone(),
                        Self::extract_compose_project(container),
                    ),
                ),
                virtualization_service_id: Some(**runtime_service_id),
                client_responses: &empty_client_responses,
                // Container inventory, not a management controller's device inventory.
                managed_device: &None,
                // ...and not swept over multicast either.
                dns_sd: &None,
            };

            if let Ok(Some(host_data)) = self
                .ops
                .build_host_from_scan(params, None, self.host_naming_fallback)
                .await
            {
                return Ok(Some(ContainerScanResult {
                    services: host_data.services,
                    ports: host_data.ports,
                    ip_addresses: host_data.ip_addresses,
                }));
            }
        }
        Ok(None)
    }

    async fn process_bridge_mode_container(
        &self,
        params: &ProcessContainerParams<'_>,
        container_id: &String,
    ) -> Result<Option<ContainerScanResult>> {
        let ProcessContainerParams {
            containers_interfaces_and_subnets,
            container,
            container_summary,
            cancel,
            runtime_service_id,
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

        // Pod members / infra containers share a netns and report the pod's full published ports;
        // scope them so each container is attributed only its own services (members → their own
        // exposed ports; the infra/pause container → empty → a portless generic container).
        let exposed_port_filter: Option<HashSet<u16>> = Self::exposed_port_scope(container);

        let (host_ip_to_host_ports, container_ips_to_container_ports, host_to_container_port_map) =
            self.get_ports_from_container(
                container_summary,
                container_interfaces_and_subnets,
                exposed_port_filter.as_ref(),
            );

        if container_interfaces_and_subnets.is_empty() {
            tracing::warn!(
                container = ?container.name,
                "No ip_addresses found for bridge container - Docker bridge subnets may not have been created"
            );
            return Ok(None);
        }

        // Probe once for the whole container, above the per-network loop. Every request goes to
        // 127.0.0.1 inside the container, so a container on three bridges was running three
        // identical sweeps and discarding two of them.
        let probed = if let Some(name) = &container.name {
            self.scan_container_endpoints(
                name.trim_start_matches("/"),
                cancel.clone(),
                exposed_port_filter.as_ref(),
            )
            .await?
        } else {
            vec![]
        };

        for (ip_address, subnet) in container_interfaces_and_subnets {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            let mut endpoint_responses = Self::attribute_to_address(
                &probed,
                ip_address.base.ip_address,
                &host_to_container_port_map,
            );

            // Fall back to probing published ports from outside for the ports the exec probe
            // could not answer for — an image with no HTTP client, or one whose service only
            // binds the published interface. Ports the exec already answered on are skipped:
            // both sets used to run unconditionally, and the external one is the expensive
            // half at 800ms per attempt, serially, over every path on the port.
            let answered_ports: HashSet<u16> = probed.iter().map(|p| p.port).collect();
            let unanswered: HashMap<(IpAddr, u16), u16> = host_to_container_port_map
                .iter()
                .filter(|(_, container_port)| !answered_ports.contains(container_port))
                .map(|(k, v)| (*k, *v))
                .collect();
            if !unanswered.is_empty() {
                let accept_invalid_certs = self.accept_invalid_certs;
                let external_responses = self
                    .scan_container_endpoints_external(
                        ip_address,
                        &unanswered,
                        cancel.clone(),
                        accept_invalid_certs,
                    )
                    .await?;
                if !external_responses.is_empty() {
                    tracing::debug!(
                        "External endpoint probing found {} responses for container at {}",
                        external_responses.len(),
                        ip_address.base.ip_address
                    );
                    endpoint_responses.extend(external_responses);
                }
            }

            if !endpoint_responses.is_empty() {
                tracing::debug!(
                    "Found {} endpoint responses for container at {}",
                    endpoint_responses.len(),
                    ip_address.base.ip_address
                );
            }

            let empty_vec_ref: &Vec<_> = &Vec::new();
            let container_ports_on_ip_address = container_ips_to_container_ports
                .get(&ip_address.base.ip_address)
                .unwrap_or(empty_vec_ref);

            let empty_client_responses = std::collections::HashMap::new();
            if let Ok(Some(mut host_data)) = self
                .ops
                .build_host_from_scan(
                    ServiceMatchBaselineParams {
                        subnet,
                        ip_address,
                        all_ports: container_ports_on_ip_address,
                        endpoint_responses: &endpoint_responses,
                        virtualization_metadata: &Some(
                            self.runtime.service_virtualization(
                                container
                                    .name
                                    .clone()
                                    .map(|n| n.trim_start_matches("/").to_string()),
                                container.id.clone(),
                                Self::extract_compose_project(container),
                            ),
                        ),
                        virtualization_service_id: Some(**runtime_service_id),
                        client_responses: &empty_client_responses,
                        // Container inventory, not a management controller's device inventory.
                        managed_device: &None,
                        // ...and not swept over multicast either.
                        dns_sd: &None,
                    },
                    None,
                    self.host_naming_fallback,
                )
                .await
            {
                // Add all ip_addresses relevant to container to the ip_addresses vec
                container_interfaces_and_subnets.iter().for_each(|(i, _)| {
                    if !host_data.ip_addresses.contains(i) {
                        host_data.ip_addresses.push(i.clone())
                    }
                });

                // Container-runtime bridge subnets (Docker OR Podman) — used to exclude
                // container-internal bindings from host-port placement below.
                let container_bridge_subnet_ids: Vec<Uuid> = container_interfaces_and_subnets
                    .iter()
                    .filter(|(_, subnet)| subnet.is_container_bridge_subnet())
                    .map(|(_, subnet)| subnet.id)
                    .collect();

                host_data.services.iter_mut().for_each(|s| {
                    // Add all host port + IPs and any container ports which weren't matched
                    // We know they are open on this host even if no services matched them
                    container_ports_on_ip_address
                        .iter()
                        .for_each(|container_port| {
                            // Add bindings for container ports which weren't matched
                            match host_data
                                .ports
                                .iter()
                                .find(|p| p.base.port_type == *container_port)
                            {
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
                                        Some(ip_address.id),
                                    ))
                                }
                                _ => (),
                            }
                        });

                    // Add bindings for all host ports, provided there's an interface
                    host_ip_to_host_ports.iter().for_each(|(ip, pbs)| {
                        pbs.iter().for_each(|pb| {
                            // If there's an existing port and existing non-docker bindings, they'll need to be replaced if listener is on all ip_addresses otherwise there'll be duplicate bindings
                            let (port, existing_non_docker_bindings) =
                                match host_data.ports.iter().find(|p| p.base.port_type == *pb) {
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
                                                    // Look up interface in the ip_addresses vec
                                                    if let Some(ip_address_id) = b.ip_address_id()
                                                        && let Some(ip_address) = host_data
                                                            .ip_addresses
                                                            .iter()
                                                            .find(|i| i.id == ip_address_id)
                                                        && !container_bridge_subnet_ids
                                                            .contains(&ip_address.base.subnet_id)
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

                            // Get host interface from the ip_addresses vec
                            let host_interface = host_data
                                .ip_addresses
                                .iter()
                                .find(|i| i.base.ip_address == *ip);

                            // Add binding to specific ip_address, or all ip_addresses if it's on ALL_IP_ADDRESSES_IP
                            match host_interface {
                                Some(host_ip_address) => {
                                    s.base.bindings.push(Binding::new_port_serviceless(
                                        port.id,
                                        Some(host_ip_address.id),
                                    ));
                                    host_data.ports.push(port);
                                }
                                None if *ip == ALL_IP_ADDRESSES_IP => {
                                    // Remove existing non-Docker bridge bindings for this port
                                    s.base.bindings = s
                                        .base
                                        .bindings
                                        .iter()
                                        .filter(|b| !existing_non_docker_bindings.contains(&b.id()))
                                        .cloned()
                                        .collect();

                                    // Add bindings for all non-Docker bridge ip_addresses
                                    // Use the interface ID from the `interfaces` list (not container_interfaces_and_subnets)
                                    // because Interface::eq deduplication at lines 617-621 may have matched
                                    // different interface objects with different UUIDs
                                    let mut bound_to_interface = false;
                                    for (ip_address, subnet) in container_interfaces_and_subnets {
                                        if !subnet.is_container_bridge_subnet() {
                                            // Find the matching interface in the ip_addresses list
                                            if let Some(matched_ip_address) = host_data
                                                .ip_addresses
                                                .iter()
                                                .find(|i| *i == ip_address)
                                            {
                                                s.base.bindings.push(
                                                    Binding::new_port_serviceless(
                                                        port.id,
                                                        Some(matched_ip_address.id),
                                                    ),
                                                );
                                                bound_to_interface = true;
                                            }
                                        }
                                    }

                                    // Published on 0.0.0.0 with no resolvable daemon-host interface
                                    // (e.g. the daemon host isn't on the container bridge subnet):
                                    // bind the host port to the container service on all addresses
                                    // (interface-less) so it isn't left orphaned on the host.
                                    if !bound_to_interface {
                                        s.base
                                            .bindings
                                            .push(Binding::new_port_serviceless(port.id, None));
                                    }

                                    host_data.ports.push(port);
                                }
                                _ => {}
                            }
                        });
                    });

                    // Remove any interface bindings which are now superceded by port bindings
                    // (interface binding is implicit in port binding)
                    let ip_address_ids_with_port_binding: Vec<Uuid> = s
                        .base
                        .bindings
                        .clone()
                        .into_iter()
                        .filter_map(|b| {
                            if b.base.binding_type.discriminant() == BindingDiscriminants::Port
                                && let Some(ip_address_id) = b.ip_address_id()
                            {
                                return Some(ip_address_id);
                            }
                            None
                        })
                        .collect();

                    s.base.bindings.retain(|b| {
                        b.base.binding_type.discriminant() == BindingDiscriminants::Port
                            || !ip_address_ids_with_port_binding
                                .contains(&b.ip_address_id().unwrap_or_default())
                    });
                });

                // Service matching above ran against a single endpoint. A container attached
                // to several bridge subnets (e.g. a proxy subnet and a private database
                // subnet) has an endpoint on each, and is equally present at all of them —
                // spread the bindings so it resolves inside every subnet it is attached to.
                let bridge_endpoint_ids: Vec<Uuid> = container_interfaces_and_subnets
                    .iter()
                    .filter(|(_, subnet)| subnet.is_container_bridge_subnet())
                    .map(|(i, _)| i.id)
                    .collect();

                spread_bindings_across_endpoints(
                    &mut host_data.services,
                    &bridge_endpoint_ids,
                    ip_address.id,
                );

                return Ok(Some(ContainerScanResult {
                    services: host_data.services,
                    ports: host_data.ports,
                    ip_addresses: host_data.ip_addresses,
                }));
            }
        }

        Ok(None)
    }

    /// Image-declared exposed port numbers for a container (`config.exposed_ports` keys like
    /// "80/tcp"). Empty when the image declares none (e.g. a pod infra/pause container).
    fn exposed_port_numbers(container: &ContainerInspectResponse) -> HashSet<u16> {
        container
            .config
            .as_ref()
            .and_then(|c| c.exposed_ports.as_ref())
            .map(|p| {
                p.iter()
                    .filter_map(|k| PortType::from_str(k).ok().map(|pt| pt.number()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The port set a bridge container's discovery is scoped to (`Some`), or `None` to use its
    /// reported/published ports as-is.
    ///
    /// Pod members and the pod infra container share one netns and each report the pod's FULL
    /// published port set, so without scoping every one matches every co-pod service:
    /// - A pod infra/pause container (name "<id>-infra") runs no workload of its own (the members
    ///   own the ports), so it is scoped to an EMPTY set → matches nothing → generic container.
    /// - A pod member (`NetworkMode "container:<id>"`) is scoped to its OWN image-declared exposed
    ///   ports.
    /// - Any other bridge container: `None` (use the ports it reports).
    fn exposed_port_scope(container: &ContainerInspectResponse) -> Option<HashSet<u16>> {
        let is_infra = container
            .name
            .as_deref()
            .is_some_and(|n| n.ends_with("-infra"));
        if is_infra {
            return Some(HashSet::new());
        }
        let shared_netns = container
            .host_config
            .as_ref()
            .and_then(|c| c.network_mode.as_deref())
            .is_some_and(|m| m.starts_with("container:"));
        if shared_netns {
            return Some(Self::exposed_port_numbers(container));
        }
        None
    }

    /// Probe a container's own listening ports, once, from inside its network namespace.
    ///
    /// Address-agnostic on purpose: everything here talks to the container over loopback, so the
    /// answers are the same whichever bridge network you ask about. They used to be collected
    /// once per attached network, running an identical sweep two or three times for a container
    /// on several bridges. [`Self::attribute_to_address`] stamps the addresses afterwards.
    ///
    /// Two execs total, against the ~150 (endpoint × 2 round-trips × networks) this replaces:
    /// one to find out what is actually listening, one to ask all of it at once.
    async fn scan_container_endpoints(
        &self,
        container_name: &str,
        cancel: CancellationToken,
        exposed_port_filter: Option<&HashSet<u16>>,
    ) -> Result<Vec<ProbedEndpoint>, Error> {
        let Some(listening) = self.listening_ports(container_name, cancel.clone()).await else {
            // No way to tell what is listening — a distroless image with no shell, or /proc not
            // mounted. Probing all ~150 endpoints blind is what made this slow; the external
            // probe over published ports is the documented fallback for exactly this case.
            tracing::debug!(
                container = container_name,
                "Could not read listening ports; leaving this container to the external probe"
            );
            return Ok(vec![]);
        };

        // A shared network namespace means /proc/net/tcp shows the whole pod's listeners, which
        // is precisely the over-attribution `exposed_port_scope` exists to stop. Intersect, so a
        // pod member is credited only with its own ports and an infra container (empty filter)
        // is skipped entirely rather than probed on everything.
        let ports: HashSet<u16> = match exposed_port_filter {
            Some(allowed) => listening.intersection(allowed).copied().collect(),
            None => listening,
        };
        if ports.is_empty() {
            return Ok(vec![]);
        }

        // Only endpoints on a port something is actually listening on. This is the whole
        // performance fix: a container listening on one port was being asked about all ~150
        // (port, path) pairs, 55 of which are on port 80 alone.
        let mut targets: Vec<(u16, String, ApplicationProtocol)> = Vec::new();
        let mut seen: HashSet<(u16, String)> = HashSet::new();
        for endpoint in Service::all_discovery_endpoints() {
            let port = endpoint.port_type.number();
            if !ports.contains(&port) {
                continue;
            }
            // `all_discovery_endpoints` dedups by (protocol, port, path), so the same (port,
            // path) recurs across protocols; one request answers all of them.
            if seen.insert((port, endpoint.path.clone())) {
                targets.push((port, endpoint.path, endpoint.protocol));
            }
        }
        if targets.is_empty() {
            return Ok(vec![]);
        }

        tracing::debug!(
            container = container_name,
            listening = ports.len(),
            endpoints = targets.len(),
            "Probing container endpoints scoped to its listening ports"
        );

        self.probe_endpoints_batched(container_name, &targets, cancel)
            .await
    }

    /// The ports something inside the container is listening on, or `None` if we could not tell.
    ///
    /// Reads `/proc/net/tcp` **and** `/proc/net/tcp6`. Both are required: a dual-stack listener —
    /// anything on the JVM, Node, or Go binding `:::8080` — appears only in the v6 table, so
    /// reading one file would silently lose a large fraction of real containers. State `0A` is
    /// `TCP_LISTEN`; the rest are established or closing connections.
    ///
    /// `None` is distinct from an empty set: empty means the container is genuinely listening on
    /// nothing, `None` means the read failed and the caller must not conclude anything.
    async fn listening_ports(
        &self,
        container_name: &str,
        cancel: CancellationToken,
    ) -> Option<HashSet<u16>> {
        let output = self
            .exec_capture(
                container_name,
                "cat /proc/net/tcp /proc/net/tcp6 2>/dev/null",
                cancel,
            )
            .await?;

        let ports = Self::parse_listening_ports(&output);
        if ports.is_empty() && !output.contains("local_address") {
            // No header line either — the read produced nothing usable rather than a genuinely
            // idle container.
            return None;
        }
        Some(ports)
    }

    /// Pull the listening ports out of `/proc/net/tcp`-format text.
    pub(crate) fn parse_listening_ports(procfs: &str) -> HashSet<u16> {
        const TCP_LISTEN: &str = "0A";

        procfs
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                // sl, local_address, rem_address, st
                let _sl = fields.next()?;
                let local = fields.next()?;
                let _rem = fields.next()?;
                if fields.next()? != TCP_LISTEN {
                    return None;
                }
                let (_addr, port) = local.rsplit_once(':')?;
                u16::from_str_radix(port, 16).ok()
            })
            .collect()
    }

    /// Ask every surviving `(port, path)` in one exec.
    ///
    /// One process spawn and one pair of socket round-trips for the whole container, rather than
    /// per endpoint — and the curl/wget/python availability chain is resolved once at the top
    /// instead of being re-run from scratch on all ~150.
    async fn probe_endpoints_batched(
        &self,
        container_name: &str,
        targets: &[(u16, String, ApplicationProtocol)],
        cancel: CancellationToken,
    ) -> Result<Vec<ProbedEndpoint>, Error> {
        // Random per container: the delimiter has to be something no response body can contain,
        // and a fixed marker is guessable by a page that echoes its own query.
        let nonce = format!("SCANOPY{}", Uuid::new_v4().simple());
        let script = Self::build_probe_script(&nonce, targets);

        let Some(output) = self.exec_capture(container_name, &script, cancel).await else {
            return Ok(vec![]);
        };

        Ok(Self::parse_batched_probe_output(&nonce, targets, &output))
    }

    /// The shell program run inside the container.
    ///
    /// `exec 2>&1` on the first line is load-bearing. `wget -S` writes its headers to stderr, and
    /// bollard delivers stdout and stderr as separate frames that can interleave — harmless when
    /// each exec carried one response, corrupting for everything downstream once one exec carries
    /// all of them. Folding the streams in the shell means the daemon reads one ordered stream.
    ///
    /// `head -c` bounds each body: port 80 alone carries ~55 paths, and an unbounded batch can
    /// return megabytes of HTML through the socket.
    pub(crate) fn build_probe_script(
        nonce: &str,
        targets: &[(u16, String, ApplicationProtocol)],
    ) -> String {
        let mut script = String::from("exec 2>&1\n");
        script.push_str(
            "if command -v curl >/dev/null 2>&1; then \
               fetch() { curl -k -i -s -m 1 -L --max-redirs 2 \"$1\"; }; \
             elif command -v wget >/dev/null 2>&1; then \
               fetch() { wget --no-check-certificate -S -q -O- -T 1 \"$1\"; }; \
             elif command -v python3 >/dev/null 2>&1; then \
               fetch() { python3 -c \"import sys,urllib.request,ssl;\
c=ssl._create_unverified_context();\
r=urllib.request.Request(sys.argv[1]);\
exec(\\\"try:\\\\n p=urllib.request.urlopen(r,context=c,timeout=1)\\\\nexcept Exception as e:\\\\n p=getattr(e,'file',None) or getattr(e,'fp',None)\\\\n if p is None: raise\\\\nprint('HTTP/1.1', p.status if hasattr(p,'status') else p.code)\\\\nfor h in p.headers: print(h + ':', p.headers[h])\\\\nprint()\\\\nprint(p.read().decode('utf-8','replace'))\\\")\" \"$1\"; }; \
             else \
               fetch() { return 1; }; \
             fi\n",
        );

        for (port, path, _) in targets {
            // The marker names the target, so a probe that produces no output at all still has a
            // segment and cannot shift the rest onto the wrong endpoint.
            script.push_str(&format!(
                "printf '\\n%s %s %s\\n' \"{nonce}\" \"{port}\" \"{path}\"\n\
                 fetch \"http://127.0.0.1:{port}{path}\" 2>/dev/null | head -c {PROBE_BODY_LIMIT}\n"
            ));
        }
        script
    }

    /// Split one batched exec's output back into per-endpoint responses.
    pub(crate) fn parse_batched_probe_output(
        nonce: &str,
        targets: &[(u16, String, ApplicationProtocol)],
        output: &str,
    ) -> Vec<ProbedEndpoint> {
        let protocol_for: HashMap<(u16, &str), ApplicationProtocol> = targets
            .iter()
            .map(|(port, path, protocol)| ((*port, path.as_str()), *protocol))
            .collect();

        // Split first, interpret second. Flushing inline meant the last segment — which has no
        // following marker to trigger it — was silently dropped.
        let mut segments: Vec<((u16, String), String)> = Vec::new();
        let mut current: Option<(u16, String)> = None;
        let mut body = String::new();

        for line in output.lines() {
            if let Some(rest) = line.strip_prefix(nonce) {
                if let Some(key) = current.take() {
                    segments.push((key, std::mem::take(&mut body)));
                }
                body.clear();

                let mut header = rest.split_whitespace();
                current = match (header.next(), header.next()) {
                    (Some(port), Some(path)) => {
                        port.parse().ok().map(|port| (port, path.to_string()))
                    }
                    // A marker with nothing after the port — not a target we sent.
                    _ => None,
                };
                continue;
            }
            if current.is_some() {
                body.push_str(line);
                body.push('\n');
            }
        }
        if let Some(key) = current.take() {
            segments.push((key, body));
        }

        segments
            .into_iter()
            .filter_map(|((port, path), raw)| {
                let (status, body, headers) = Self::parse_http_response(raw.trim())?;
                let protocol = protocol_for
                    .get(&(port, path.as_str()))
                    .copied()
                    .unwrap_or_default();
                Some(ProbedEndpoint {
                    port,
                    path,
                    protocol,
                    status,
                    body,
                    headers,
                })
            })
            .collect()
    }

    /// Run a shell command inside the container and return everything it wrote.
    ///
    /// `None` when the container has no shell, the exec could not be created, or the runtime
    /// refused — all cases where the caller must not read an empty result as "nothing there".
    async fn exec_capture(
        &self,
        container_name: &str,
        command: &str,
        cancel: CancellationToken,
    ) -> Option<String> {
        if cancel.is_cancelled() {
            return None;
        }

        let exec = self
            .client
            .create_exec(
                container_name,
                bollard::exec::CreateExecOptions {
                    cmd: Some(vec!["sh", "-c", command]),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    ..Default::default()
                },
            )
            .await
            .ok()?;

        let bollard::exec::StartExecResults::Attached { mut output, .. } =
            self.client.start_exec(&exec.id, None).await.ok()?
        else {
            return None;
        };

        use futures::StreamExt;
        let mut captured = String::new();
        // Creating and starting the exec are bounded by the client's own timeout; reading its
        // output was not, so an exec that never closed its stream held the container scan until
        // the integration's five-minute cap. Stopping early returns `None` rather than the partial
        // output: a truncated `/proc/net/tcp` parses as a shorter, wrong port list.
        let deadline = tokio::time::sleep(EXEC_OUTPUT_TIMEOUT);
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    tracing::debug!(container = container_name, "Container exec cancelled");
                    return None;
                }
                _ = &mut deadline => {
                    tracing::warn!(
                        container = container_name,
                        timeout_secs = EXEC_OUTPUT_TIMEOUT.as_secs(),
                        "Container exec produced no end of output in time; probing its ports \
                         from outside instead"
                    );
                    return None;
                }
                msg = output.next() => {
                    match msg {
                        Some(Ok(bollard::container::LogOutput::StdOut { message }))
                        | Some(Ok(bollard::container::LogOutput::StdErr { message })) => {
                            captured.push_str(&String::from_utf8_lossy(&message));
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            tracing::warn!(container = container_name, error = %e, "Error reading container exec output");
                            break;
                        }
                        None => break,
                    }
                }
            }
        }

        Some(captured)
    }

    /// Attribute address-agnostic probe results to one of the container's addresses.
    ///
    /// Each answer becomes the container-internal endpoint on `container_ip`, plus one per host
    /// port published to it, which is what the pattern matcher and the binding logic downstream
    /// each expect to find.
    fn attribute_to_address(
        probed: &[ProbedEndpoint],
        container_ip: IpAddr,
        host_to_container_port_map: &HashMap<(IpAddr, u16), u16>,
    ) -> Vec<EndpointResponse> {
        let mut container_to_host_port_map: HashMap<u16, Vec<(IpAddr, u16)>> = HashMap::new();
        for ((host_ip, host_port), container_port) in host_to_container_port_map {
            container_to_host_port_map
                .entry(*container_port)
                .or_default()
                .push((*host_ip, *host_port));
        }

        let mut responses = Vec::new();
        for answer in probed {
            if let Some(host_mappings) = container_to_host_port_map.get(&answer.port) {
                for (host_ip, host_port) in host_mappings {
                    responses.push(EndpointResponse {
                        endpoint: Endpoint {
                            ip: Some(*host_ip),
                            port_type: PortType::new_tcp(*host_port),
                            protocol: answer.protocol,
                            path: answer.path.clone(),
                        },
                        body: answer.body.clone(),
                        status: answer.status,
                        headers: answer.headers.clone(),
                    });
                }
            }

            responses.push(EndpointResponse {
                endpoint: Endpoint {
                    ip: Some(container_ip),
                    port_type: PortType::new_tcp(answer.port),
                    protocol: answer.protocol,
                    path: answer.path.clone(),
                },
                body: answer.body.clone(),
                status: answer.status,
                headers: answer.headers.clone(),
            });
        }
        responses
    }

    /// Fallback endpoint scanning for bridge-mode containers that lack HTTP tools.
    /// Probes host-published ports externally via reqwest, then remaps responses
    /// back to container ports for the pattern matcher.
    async fn scan_container_endpoints_external(
        &self,
        ip_address: &IPAddress,
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
                let probe_ip = if *host_ip == ALL_IP_ADDRESSES_IP {
                    self.host_ip
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
                                    ip: Some(ip_address.base.ip_address),
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

    fn extract_compose_project(container: &ContainerInspectResponse) -> Option<String> {
        container
            .config
            .as_ref()
            .and_then(|c| c.labels.as_ref())
            .and_then(|l| l.get("com.docker.compose.project").cloned())
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
        container_interfaces_and_subnets: &[(IPAddress, Subnet)],
        exposed_port_filter: Option<&HashSet<u16>>,
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
                // Shared-netns members (and pod infra containers) report the pod's full published
                // port set; scope to the container's own image-declared exposed ports so each is
                // attributed only its own services.
                if let Some(allowed) = exposed_port_filter
                    && !allowed.contains(&p.private_port)
                {
                    return;
                }
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

    /// The subnet a container endpoint on `network_name` belongs to.
    ///
    /// Bound by the runtime's own network **identity**, not by which CIDR happens to contain the
    /// address. The API already told us which network the endpoint is on and
    /// `create_bridge_subnets` names each subnet after it, so re-deriving that by address would
    /// trade a fact for a guess — and a guess that goes wrong in two ways: the network-wide list
    /// carries `0.0.0.0/0` catch-alls that contain every IPv4 address, and a bridge is host-scoped,
    /// so two daemons legitimately hold the same `172.17.0.0/16` and containment cannot tell them
    /// apart.
    ///
    /// A network with several IPAM pools yields several subnets sharing a name, so containment
    /// chooses among *that network's* subnets — never outside them.
    fn subnet_for_container_network<'s>(
        bridge_subnets: &'s [Subnet],
        network_name: &str,
        ip_address: IpAddr,
    ) -> Option<&'s Subnet> {
        let mut named = bridge_subnets
            .iter()
            .filter(|s| s.base.name == network_name)
            .peekable();
        named.peek()?;
        let candidates: Vec<&Subnet> = named.collect();
        candidates
            .iter()
            .find(|s| s.base.cidr.contains(&ip_address))
            .or_else(|| candidates.first())
            .copied()
    }

    pub fn get_container_interfaces(
        &self,
        containers: &[(ContainerInspectResponse, ContainerSummary)],
        bridge_subnets: &'a [Subnet],
        known_subnets: &[Subnet],
        host_interfaces: &mut [IPAddress],
    ) -> HashMap<String, Vec<(IPAddress, Subnet)>> {
        // The host's own addresses are a genuine address lookup — nothing names a network for them
        // — so they are placed by the shared rule: longest prefix, never a `0.0.0.0/0` catch-all.
        let host_interfaces_and_subnets = host_interfaces
            .iter_mut()
            .filter_map(|i| {
                let placed = crate::server::subnets::r#impl::inference::placeable_subnet(
                    known_subnets,
                    i.base.ip_address,
                )
                .or_else(|| {
                    crate::server::subnets::r#impl::inference::placeable_subnet(
                        bridge_subnets,
                        i.base.ip_address,
                    )
                });

                match placed {
                    Some(subnet) => {
                        i.base.subnet_id = subnet.id;
                        Some((i.clone(), subnet.clone()))
                    }
                    None => {
                        tracing::warn!(
                            ip = %i.base.ip_address,
                            "No subnet holds this host address; it is left unplaced"
                        );
                        None
                    }
                }
            })
            .collect::<Vec<(IPAddress, Subnet)>>();

        // Collect ip_addresses from containers
        let mut interfaces_by_id: HashMap<String, Vec<(IPAddress, Subnet)>> = containers
            .iter()
            .filter_map(|(container, _)| {
                let host_networking_mode = container
                    .host_config
                    .as_ref()
                    .and_then(|c| c.network_mode.clone())
                    .unwrap_or_default()
                    == "host";

                let mut ip_addresses_and_subnets: Vec<(IPAddress, Subnet)> = if host_networking_mode
                {
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
                                        && let Some(subnet) = Self::subnet_for_container_network(
                                            bridge_subnets,
                                            network_name,
                                            ip_address,
                                        )
                                    {
                                        // Parse MAC address from Docker network endpoint
                                        // The runtime's own record of the endpoint it created.
                                        let mac_address = endpoint
                                            .mac_address
                                            .as_ref()
                                            .and_then(|mac_str| mac_str.parse::<MacAddress>().ok())
                                            .map(|m| {
                                                MacEvidence::new(
                                                    MacEvidenceValue(m),
                                                    AttributeSource::Probe(
                                                        self.runtime.client_probe(),
                                                    ),
                                                )
                                            });

                                        return Some((
                                            IPAddress::new(IPAddressBase {
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
                            .collect::<Vec<(IPAddress, Subnet)>>()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // The runtime reports attachments in a HashMap, so iteration order varies run
                // to run. Downstream the first entry becomes the container's primary endpoint
                // (the one service matching is anchored to, and the one the container-runtime
                // edge targets), so sort by network name to keep both stable across scans.
                ip_addresses_and_subnets.sort_by(|(a, _), (b, _)| {
                    a.base
                        .name
                        .cmp(&b.base.name)
                        .then_with(|| a.base.ip_address.cmp(&b.base.ip_address))
                });

                // Merge in host ip_addresses
                ip_addresses_and_subnets.extend(host_interfaces_and_subnets.clone());

                container
                    .id
                    .as_ref()
                    .map(|id| (id.clone(), ip_addresses_and_subnets))
            })
            .collect();

        // Pod / shared-netns members run with NetworkMode "container:<id>" — they share the
        // referenced container's network namespace and report no networks of their own (so the
        // pass above leaves them with empty interfaces and they'd be dropped). Inherit the
        // referenced container's interfaces so the member is still discovered (e.g. a pod's
        // nginx member sharing the infra container's IP). The reference may be a short or full id.
        let shared_netns_members: Vec<(String, String)> = containers
            .iter()
            .filter_map(|(container, _)| {
                let mode = container
                    .host_config
                    .as_ref()
                    .and_then(|c| c.network_mode.clone())
                    .unwrap_or_default();
                let reference = mode.strip_prefix("container:")?.to_string();
                Some((container.id.clone()?, reference))
            })
            .collect();

        for (member_id, reference) in shared_netns_members {
            if let Some(parent_id) = interfaces_by_id
                .keys()
                .find(|k| k.starts_with(&reference))
                .cloned()
                && let Some(parent_ifaces) = interfaces_by_id.get(&parent_id).cloned()
                && !parent_ifaces.is_empty()
            {
                interfaces_by_id.insert(member_id, parent_ifaces);
            }
        }

        interfaces_by_id
    }

    /// List every container and inspect each one, pairing each inspection with the summary it
    /// came from.
    ///
    /// The pairing used to be positional: summaries without an id were filtered out of the
    /// inspect list, and the results were then zipped against the *unfiltered* summaries. One
    /// id-less summary therefore shifted every container after it onto the wrong summary, and the
    /// id-mismatch guard in `process_single_container` silently dropped all of them as an
    /// "inspection failure". Carrying the summary alongside its own future makes the pairing
    /// correct by construction.
    ///
    /// Bounded concurrency for the same reason the scan loop is bounded: this is one socket, and
    /// an unbounded `try_join_all` opened a request per container — several hundred at once on a
    /// large host. `try_join_all` also aborted the whole batch on a single failed inspect, where
    /// dropping just that container is plainly better.
    pub async fn get_containers_and_summaries(
        &self,
    ) -> Result<Vec<(ContainerInspectResponse, ContainerSummary)>, Error> {
        let container_summaries = self
            .client
            .list_containers(None::<ListContainersOptions>)
            .await
            .map_err(|e| anyhow!(e))?;

        let inspections = stream::iter(container_summaries.into_iter().filter_map(|summary| {
            let id = summary.id.clone()?;
            Some(async move {
                match self
                    .client
                    .inspect_container(&id, None::<InspectContainerOptions>)
                    .await
                {
                    Ok(inspected) => Some((inspected, summary)),
                    Err(e) => {
                        tracing::warn!(
                            container_id = %id,
                            error = %e,
                            "Failed to inspect container; skipping it"
                        );
                        None
                    }
                }
            })
        }))
        .buffer_unordered(CONCURRENT_CONTAINER_SCANS)
        .collect::<Vec<_>>()
        .await;

        Ok(inspections.into_iter().flatten().collect())
    }
}

/// Mirror each service's bindings from `primary_endpoint_id` onto the container's other
/// `endpoint_ids`.
///
/// Service matching runs once per container, against whichever endpoint resolved first, so its
/// bindings all point at that one endpoint. A container attached to several bridge subnets is
/// reachable at its endpoint on each of them, and callers that ask "what is at this IP address?"
/// — topology element cards, the container-runtime edge — resolve through bindings. Without this
/// the container only ever surfaces in one of the subnets it belongs to.
///
/// Idempotent: bindings compare by type, so re-running adds nothing.
fn spread_bindings_across_endpoints(
    services: &mut [Service],
    endpoint_ids: &[Uuid],
    primary_endpoint_id: Uuid,
) {
    let other_endpoint_ids: Vec<Uuid> = endpoint_ids
        .iter()
        .copied()
        .filter(|id| *id != primary_endpoint_id)
        .collect();

    if other_endpoint_ids.is_empty() {
        return;
    }

    for service in services.iter_mut() {
        let primary_bindings: Vec<Binding> = service
            .base
            .bindings
            .iter()
            .filter(|b| b.ip_address_id() == Some(primary_endpoint_id))
            .copied()
            .collect();

        for endpoint_id in &other_endpoint_ids {
            for binding in &primary_bindings {
                let mirrored = binding.rebound_to_ip_address(*endpoint_id);
                if !service.base.bindings.contains(&mirrored) {
                    service.base.bindings.push(mirrored);
                }
            }
        }
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    /// A dual-stack listener — anything on the JVM, Node, or Go binding `:::8080` — appears only
    /// in `/proc/net/tcp6`. Reading just the v4 table would drop it, and with it every service
    /// discovered inside such a container.
    #[test]
    fn listening_ports_come_from_both_ip_stacks() {
        let procfs = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 1 1
   1: 0100007F:8AE2 0100007F:1F90 01 00000000:00000000 00:00000000 00000000     0        0 2 1
  sl  local_address                         remote_address                        st
   0: 00000000000000000000000000000000:1F90 00000000000000000000000000000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 3 1
";

        let ports = ContainerScanner::parse_listening_ports(procfs);

        assert!(ports.contains(&80), "IPv4 listener missing");
        assert!(
            ports.contains(&8080),
            "IPv6 dual-stack listener missing — this is the common case for JVM/Node/Go images"
        );
        assert!(
            !ports.contains(&35554),
            "an established connection is not something listening"
        );
        assert_eq!(ports.len(), 2);
    }

    /// The batch carries every endpoint's response in one stream, so the framing is the only
    /// thing keeping them apart. A body that itself contains an HTTP status line must not be
    /// able to split a segment.
    #[test]
    fn a_batched_probe_response_is_split_per_endpoint() {
        let nonce = "SCANOPYtestnonce";
        let targets = vec![
            (80u16, "/".to_string(), ApplicationProtocol::Http),
            (80u16, "/status".to_string(), ApplicationProtocol::Http),
            (9000u16, "/api".to_string(), ApplicationProtocol::Http),
        ];

        // The middle body quotes an HTTP response, which is exactly what a status page or an
        // error-echoing endpoint returns.
        let output = format!(
            "\n{nonce} 80 /\n\
             HTTP/1.1 200 OK\r\nServer: nginx\r\n\r\n<html>welcome</html>\n\
             \n{nonce} 80 /status\n\
             HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nlast upstream said HTTP/1.1 500 Internal Server Error\n\
             \n{nonce} 9000 /api\n\
             HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\n\r\n{{\"error\":\"nope\"}}\n"
        );

        let probed = ContainerScanner::parse_batched_probe_output(nonce, &targets, &output);

        assert_eq!(probed.len(), 3, "every segment must survive the split");

        assert_eq!((probed[0].port, probed[0].path.as_str()), (80, "/"));
        assert!(probed[0].body.contains("welcome"));

        assert_eq!((probed[1].port, probed[1].path.as_str()), (80, "/status"));
        assert_eq!(
            probed[1].status, 200,
            "the status comes from the segment's own response line, not a status line quoted \
             inside its body"
        );
        assert!(probed[1].body.contains("last upstream said"));

        assert_eq!((probed[2].port, probed[2].path.as_str()), (9000, "/api"));
        assert_eq!(probed[2].status, 404);
    }

    /// A probe that returns nothing still gets a marker, so the endpoints after it stay on their
    /// own segments instead of shifting up onto the wrong one.
    #[test]
    fn an_endpoint_that_answered_nothing_does_not_shift_the_others() {
        let nonce = "SCANOPYtestnonce";
        let targets = vec![
            (80u16, "/".to_string(), ApplicationProtocol::Http),
            (8080u16, "/health".to_string(), ApplicationProtocol::Http),
        ];

        let output = format!(
            "\n{nonce} 80 /\n\
             \n{nonce} 8080 /health\n\
             HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"ok\":true}}\n"
        );

        let probed = ContainerScanner::parse_batched_probe_output(nonce, &targets, &output);

        assert_eq!(probed.len(), 1, "the silent endpoint contributes nothing");
        assert_eq!(
            (probed[0].port, probed[0].path.as_str()),
            (8080, "/health"),
            "the answering endpoint must keep its own identity"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::services::r#impl::base::ServiceBase;
    use crate::server::services::r#impl::categories::ServiceCategory;
    use crate::server::services::r#impl::definitions::ServiceDefinition;
    use crate::server::services::r#impl::patterns::Pattern;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};

    #[derive(PartialEq, Eq, Hash, Clone)]
    struct TestServiceDef;

    impl ServiceDefinition for TestServiceDef {
        fn name(&self) -> &'static str {
            "TestService"
        }
        fn description(&self) -> &'static str {
            "Test"
        }
        fn category(&self) -> ServiceCategory {
            ServiceCategory::Development
        }
        fn discovery_pattern(&self) -> Pattern<'_> {
            Pattern::None
        }
    }

    fn service_with(bindings: Vec<Binding>) -> Service {
        Service {
            base: ServiceBase {
                service_definition: Box::new(TestServiceDef),
                bindings,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Bindings anchored to a given endpoint, as (port_id, ip_address_id) pairs.
    fn port_bindings(service: &Service) -> Vec<(Option<Uuid>, Option<Uuid>)> {
        service
            .base
            .bindings
            .iter()
            .map(|b| (b.port_id(), b.ip_address_id()))
            .collect()
    }

    #[test]
    fn container_on_several_subnets_is_reachable_at_every_endpoint() {
        let (primary, second, third) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let port_id = Uuid::new_v4();
        let mut services = vec![service_with(vec![Binding::new_port_serviceless(
            port_id,
            Some(primary),
        )])];

        spread_bindings_across_endpoints(&mut services, &[primary, second, third], primary);

        let bound_to: HashSet<Uuid> = services[0]
            .base
            .bindings
            .iter()
            .filter(|b| b.port_id() == Some(port_id))
            .filter_map(|b| b.ip_address_id())
            .collect();
        assert_eq!(
            bound_to,
            HashSet::from([primary, second, third]),
            "the container's port should resolve at each subnet it is attached to"
        );
    }

    #[test]
    fn single_attachment_is_left_alone() {
        let primary = Uuid::new_v4();
        let mut services = vec![service_with(vec![Binding::new_port_serviceless(
            Uuid::new_v4(),
            Some(primary),
        )])];
        let before = port_bindings(&services[0]);

        spread_bindings_across_endpoints(&mut services, &[primary], primary);

        assert_eq!(port_bindings(&services[0]), before);
    }

    #[test]
    fn portless_container_is_reachable_at_every_endpoint() {
        // A container exposing no ports is bound by IP address rather than by port
        // (see `Service::from_discovery`); it is still present on every subnet.
        let (primary, second) = (Uuid::new_v4(), Uuid::new_v4());
        let mut services = vec![service_with(vec![Binding::new_ip_address_serviceless(
            primary,
        )])];

        spread_bindings_across_endpoints(&mut services, &[primary, second], primary);

        let bound_to: HashSet<Uuid> = services[0]
            .base
            .bindings
            .iter()
            .filter(|b| b.port_id().is_none())
            .filter_map(|b| b.ip_address_id())
            .collect();
        assert_eq!(bound_to, HashSet::from([primary, second]));
    }

    #[test]
    fn bindings_on_other_addresses_are_not_spread() {
        // A published host port is bound to the host's own address, not to a container
        // endpoint — it must not be duplicated onto the container's bridge endpoints.
        let (primary, second, host_ip) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let host_port_id = Uuid::new_v4();
        let mut services = vec![service_with(vec![
            Binding::new_port_serviceless(Uuid::new_v4(), Some(primary)),
            Binding::new_port_serviceless(host_port_id, Some(host_ip)),
        ])];

        spread_bindings_across_endpoints(&mut services, &[primary, second], primary);

        let host_port_addresses: Vec<Option<Uuid>> = services[0]
            .base
            .bindings
            .iter()
            .filter(|b| b.port_id() == Some(host_port_id))
            .map(|b| b.ip_address_id())
            .collect();
        assert_eq!(host_port_addresses, vec![Some(host_ip)]);
    }

    #[test]
    fn rescanning_does_not_accumulate_bindings() {
        let (primary, second) = (Uuid::new_v4(), Uuid::new_v4());
        let mut services = vec![service_with(vec![Binding::new_port_serviceless(
            Uuid::new_v4(),
            Some(primary),
        )])];

        spread_bindings_across_endpoints(&mut services, &[primary, second], primary);
        let after_first = port_bindings(&services[0]);
        spread_bindings_across_endpoints(&mut services, &[primary, second], primary);

        assert_eq!(port_bindings(&services[0]), after_first);
    }

    /// The hazard binding-by-identity removes. A container bridge is host-scoped, so two daemons
    /// legitimately hold the same `172.17.0.0/16` — containment cannot tell them apart, and the
    /// address is identical on both. Only the network the runtime named can.
    #[test]
    fn an_endpoint_binds_to_its_own_daemons_bridge_when_two_share_a_cidr() {
        let ours = bridge_subnet("scanopy_default", "172.17.0.0/16");
        let theirs = bridge_subnet("other_default", "172.17.0.0/16");
        let subnets = vec![theirs.clone(), ours.clone()];

        let bound = ContainerScanner::subnet_for_container_network(
            &subnets,
            "scanopy_default",
            "172.17.0.2".parse().unwrap(),
        )
        .expect("the named network");

        assert_eq!(bound.id, ours.id);
    }

    /// A Docker network with several IPAM pools yields several subnets sharing a name, so
    /// containment chooses among *that network's* subnets — never outside them.
    #[test]
    fn containment_only_chooses_among_the_named_networks_own_pools() {
        let v4 = bridge_subnet("dual", "172.20.0.0/16");
        let v6 = bridge_subnet("dual", "fd00:dead:beef::/64");
        let unrelated = bridge_subnet("other", "10.0.0.0/8");
        let subnets = vec![unrelated, v4.clone(), v6.clone()];

        let bound = ContainerScanner::subnet_for_container_network(
            &subnets,
            "dual",
            "fd00:dead:beef::2".parse().unwrap(),
        )
        .expect("the named network");

        assert_eq!(bound.id, v6.id);
    }

    /// An endpoint on a network no bridge subnet was created for has nowhere to go, and saying so
    /// is better than filing it under whichever range happens to contain the address.
    #[test]
    fn an_endpoint_on_an_unknown_network_binds_to_nothing() {
        let subnets = vec![bridge_subnet("scanopy_default", "172.17.0.0/16")];

        assert!(
            ContainerScanner::subnet_for_container_network(
                &subnets,
                "a_network_we_never_listed",
                "172.17.0.2".parse().unwrap(),
            )
            .is_none()
        );
    }

    fn bridge_subnet(name: &str, cidr: &str) -> Subnet {
        Subnet {
            id: Uuid::new_v4(),
            base: crate::server::subnets::r#impl::base::SubnetBase {
                name: name.to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(cidr.parse().expect("valid test CIDR")),
                    AttributeSource::DaemonSelfReport,
                ),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
