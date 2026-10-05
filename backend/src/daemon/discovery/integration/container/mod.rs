//! Shared container-runtime discovery machinery (Docker + Podman).
//!
//! Docker and Podman both expose a Docker-compatible REST API, so a single
//! scanner and two transports (HTTP(S) proxy + local Unix socket) serve both.
//! The [`ContainerRuntime`] type selects the runtime-specific bits — which
//! daemon service the discovered containers belong to, which `ClientProbe`
//! feeds service matching, and which virtualization variants get stamped onto
//! services and subnets. The `docker` and `podman` integration modules are thin
//! wrappers that call into here with the appropriate runtime.

pub mod hosts;
pub mod interfaces;
pub mod scanner;

use std::net::IpAddr;
use std::time::Duration;

use anyhow::{Error, Result};
use bollard::Docker;
use cidr::IpCidr;
use uuid::Uuid;

use crate::daemon::utils::base::DaemonUtils;
use crate::server::credentials::r#impl::mapping::ContainerProxyQueryCredential;
use crate::server::hosts::r#impl::virtualization::{
    ContainerHostVirtualization, HostVirtualization, HostVirtualizationDiscriminants,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::definitions::{ServiceDefinition, VirtualizationRole};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::services::r#impl::virtualization::{
    DockerVirtualization, PodmanVirtualization, ServiceVirtualization,
};
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase, SubnetCidr, SubnetCidrValue};
use crate::server::subnets::r#impl::types::SubnetType;

use super::{
    Checkpoint, CollectionShortfall, Completeness, ProbeContext, ProbeFailure, ProbeSuccess,
    merge_subnets,
};
use crate::daemon::discovery::service::warnings::AttemptOutcome;
use crate::server::hosts::r#impl::attributes::HostOsValue;
use crate::server::hosts::r#impl::os::{HostOs, HostOsFamily};
use crate::server::shared::attribution::{AttributeSource, Attributed};

const CONTAINER_PROBE_MAX_ATTEMPTS: u32 = 3;

/// Hard cap on a container scan, shared by all four container integrations (Docker and Podman,
/// socket and proxy).
///
/// One constant rather than four inline literals that have to agree. After the probe rework this
/// leaves roughly a 10x margin for several hundred containers, and the soft deadline derived from
/// it ([`CONTAINER_SCAN_SOFT_DEADLINE_FRACTION`]) means a slow scan stops itself and says so
/// rather than letting this fire. Raising it was never the fix — a bigger number only lengthens
/// the stall before the same loss.
pub const CONTAINER_SCAN_TIMEOUT: Duration = Duration::from_secs(300);

/// How much of [`CONTAINER_SCAN_TIMEOUT`] the container loop may spend before it stops at a
/// container boundary and reports what it got. Derived rather than a second literal so the two
/// cannot drift.
const CONTAINER_SCAN_SOFT_DEADLINE_FRACTION: f32 = 0.8;

/// Which container runtime an integration targets. Selects every
/// runtime-specific decision in the otherwise-shared scanning machinery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

impl ContainerRuntime {
    /// The daemon service definition these containers belong to. `execute`
    /// matches it in `host_data` by id to recover its `service_id`.
    pub fn service_def(&self) -> Box<dyn ServiceDefinition> {
        match self {
            Self::Docker => Box::new(crate::server::services::definitions::docker_daemon::Docker),
            Self::Podman => Box::new(crate::server::services::definitions::podman::Podman),
        }
    }

    /// The definition for a container serving this runtime's API on a TCP port.
    pub fn api_proxy_def(&self) -> Box<dyn ServiceDefinition> {
        match self {
            Self::Docker => {
                Box::new(crate::server::services::definitions::docker_api_proxy::DockerApiProxy)
            }
            Self::Podman => {
                Box::new(crate::server::services::definitions::podman_api_proxy::PodmanApiProxy)
            }
        }
    }

    /// The client-probe value fed into service matching for this runtime.
    pub fn client_probe(&self) -> ClientProbe {
        match self {
            Self::Docker => ClientProbe::Docker,
            Self::Podman => ClientProbe::Podman,
        }
    }

    /// The `SubnetType` stamped on this runtime's bridge networks.
    pub fn bridge_subnet_type(&self) -> SubnetType {
        match self {
            Self::Docker => SubnetType::DockerBridge,
            Self::Podman => SubnetType::PodmanBridge,
        }
    }

    /// Build a subnet from one IPAM entry of a network reported by the runtime's
    /// own API.
    ///
    /// This is the **only** constructor permitted to produce a container-runtime
    /// subnet type. `driver` is the evidence: it comes from the runtime, not from
    /// guessing at an interface name (`SubnetType::from_interface_name`
    /// deliberately cannot return these types — see #663). Bridge networks also
    /// carry `virtualization_service_id`, which scopes an otherwise-ambiguous CIDR
    /// like `172.17.0.0/16` to the runtime service that owns it.
    ///
    /// `None` for drivers that own no routable L3 network (`host`, `none`, `null`).
    pub fn subnet_from_network(
        &self,
        site_id: Uuid,
        cidr: IpCidr,
        name: String,
        driver: &str,
        runtime_service_id: Uuid,
    ) -> Option<Subnet> {
        let bridge_subnet_type = self.bridge_subnet_type();
        let subnet_type = match driver {
            "bridge" | "overlay" => bridge_subnet_type,
            "macvlan" => SubnetType::MacVlan,
            "ipvlan" => SubnetType::IpVlan,
            _ => {
                tracing::trace!(
                    network_name = %name,
                    driver = driver,
                    "Skipping unsupported container network driver"
                );
                return None;
            }
        };

        // MacVLAN/IpVLAN sit on a physical LAN the host shares with everything
        // else, so they are not host-scoped and take no owning runtime.
        let virtualization_service_id =
            (subnet_type == bridge_subnet_type).then_some(runtime_service_id);

        Some(Subnet::new(SubnetBase {
            // The runtime's own IPAM config: it defined this bridge, but it is describing
            // something it manages rather than itself, which is what `Probe` means here.
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr),
                AttributeSource::Probe(self.client_probe()),
            ),
            description: None,
            tags: Vec::new(),
            site_id,
            name,
            subnet_type,
            virtualization_service_id,
            source: EntitySource::Discovery,
        }))
    }

    /// Whether `interface_name` is one a container runtime reserves for its own
    /// bridges on the host it runs on.
    ///
    /// This is a **filter, never a label** — it keeps the daemon from creating
    /// and scanning its own host's container bridges, which matters most when
    /// the runtime API is unreachable and no authoritative record will arrive.
    /// Classification is not allowed to consult it: a false negative here just
    /// means a bridge shows up as an ordinary subnet, whereas a false positive
    /// in classification labels an unrelated network "Docker" (#663).
    pub fn reserves_host_interface_name(interface_name: &str) -> bool {
        let name = interface_name.to_lowercase();

        // Docker's per-network bridges are `br-` + the first 12 hex chars of the
        // network id. Anything else after `br-` is someone else's bridge.
        if let Some(suffix) = name.strip_prefix("br-") {
            return suffix.len() == 12 && suffix.chars().all(|c| c.is_ascii_hexdigit());
        }

        // Swarm's ingress bridge, plus the default bridges: docker0, podman0,
        // cni-podman0.
        name == "docker_gwbridge"
            || ["docker", "podman", "cni-podman"].iter().any(|prefix| {
                name.strip_prefix(prefix).is_some_and(|rest| {
                    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
                })
            })
    }

    /// Build the container identity stamped onto a discovered container service.
    ///
    /// The owning runtime is carried separately, as `Service::virtualization_service_id`.
    pub fn service_virtualization(
        &self,
        container_name: Option<String>,
        container_id: Option<String>,
        compose_project: Option<String>,
    ) -> ServiceVirtualization {
        match self {
            Self::Docker => ServiceVirtualization::Docker(DockerVirtualization {
                container_name,
                container_id,
                compose_project,
            }),
            Self::Podman => ServiceVirtualization::Podman(PodmanVirtualization {
                container_name,
                container_id,
                compose_project,
            }),
        }
    }

    /// The `HostVirtualization` variant for a container that is a host of its own.
    pub fn host_virtualization(
        &self,
        container: ContainerHostVirtualization,
    ) -> HostVirtualization {
        match self {
            Self::Docker => HostVirtualization::Docker(container),
            Self::Podman => HostVirtualization::Podman(container),
        }
    }

    /// Whether `role` is this runtime's: the role of the service its container hosts link to.
    pub fn is_runtime_role(&self, role: Option<VirtualizationRole>) -> bool {
        let hosts = match self {
            Self::Docker => HostVirtualizationDiscriminants::Docker,
            Self::Podman => HostVirtualizationDiscriminants::Podman,
        };
        matches!(role, Some(VirtualizationRole::ContainerRuntime { hosts: h, .. }) if h == hosts)
    }

    /// Human-facing label for logs/diagnostics.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Docker => "Docker",
            Self::Podman => "Podman",
        }
    }
}

/// Handle returned by a successful container-runtime probe (Docker or Podman).
pub struct ContainerProbeHandle {
    pub client: Docker,
    pub port: u16,
    /// Must stay alive until `client` is dropped — bollard reads certs lazily.
    pub _ssl_temp_handles: Vec<tempfile::NamedTempFile>,
}

pub type SslPaths = Option<(String, String, String)>;

/// Build a proxy URL from a container-runtime proxy credential and target IP.
pub fn build_container_proxy_url(ip: IpAddr, cred: &ContainerProxyQueryCredential) -> String {
    let proxy_path = cred.path.as_deref().unwrap_or("").trim_start_matches('/');
    let has_ssl = cred.ssl_cert.is_some();
    let scheme = if has_ssl { "https" } else { "http" };
    let host_str = match ip {
        IpAddr::V6(v6) => format!("[{}]", v6),
        _ => ip.to_string(),
    };
    if proxy_path.is_empty() {
        format!("{}://{}:{}", scheme, host_str, cred.port)
    } else {
        format!("{}://{}:{}/{}", scheme, host_str, cred.port, proxy_path)
    }
}

/// Resolve SSL paths from a proxy credential, returning (cert, key, chain) paths
/// and temp file handles that must outlive the client.
pub fn resolve_container_ssl(
    cred: &ContainerProxyQueryCredential,
    label: &str,
) -> Result<(SslPaths, Vec<tempfile::NamedTempFile>), Error> {
    let mut temp_handles = Vec::new();

    let ssl_info = if let (Some(cert_rv), Some(key_rv), Some(chain_rv)) =
        (&cred.ssl_cert, &cred.ssl_key, &cred.ssl_chain)
    {
        let (cert_path, cert_handle) = cert_rv.resolve_to_path("ssl_cert", label)?;
        let (key_path, key_handle) = key_rv.resolve_to_path("ssl_key", label)?;
        let (chain_path, chain_handle) = chain_rv.resolve_to_path("ssl_chain", label)?;
        temp_handles.extend(cert_handle);
        temp_handles.extend(key_handle);
        temp_handles.extend(chain_handle);
        Some((
            cert_path.to_string_lossy().into_owned(),
            key_path.to_string_lossy().into_owned(),
            chain_path.to_string_lossy().into_owned(),
        ))
    } else {
        None
    };

    Ok((ssl_info, temp_handles))
}

/// Probe a container-runtime proxy endpoint (Docker or Podman).
pub async fn probe_proxy(
    ctx: &ProbeContext<'_>,
    runtime: ContainerRuntime,
) -> Result<ProbeSuccess, ProbeFailure> {
    let cred = ctx.credential.as_container_proxy().ok_or_else(|| {
        ProbeFailure::malformed(format!("Expected {} proxy credential", runtime.label()))
    })?;

    let proxy_url = build_container_proxy_url(ctx.ip, cred);
    let label = ctx.credential.discovery_label();
    let (ssl_paths, ssl_temp_handles) = resolve_container_ssl(cred, label).map_err(|e| {
        // The certificate material on the credential could not be read or parsed. That is our
        // stored configuration, not the remote host, so it is the one failure here an operator
        // fixes in Scanopy.
        ProbeFailure::malformed(format!("Failed to resolve {} SSL: {}", runtime.label(), e))
    })?;

    tracing::info!(ip = %ctx.ip, proxy_url = %proxy_url, runtime = runtime.label(), "Attempting container proxy probe");

    for attempt in 1..=CONTAINER_PROBE_MAX_ATTEMPTS {
        if ctx.cancel.is_cancelled() {
            // Never reported. The operator stopped the scan; telling them their credential was
            // rejected — which is what this used to do — is worse than saying nothing.
            return Err(ProbeFailure::cancelled());
        }

        match ctx
            .utils
            .new_docker_client(Ok(Some(proxy_url.clone())), Ok(ssl_paths.clone()))
            .await
        {
            Ok(client) => {
                tracing::info!(ip = %ctx.ip, proxy_url = %proxy_url, runtime = runtime.label(), "Container client probe succeeded");
                return Ok(ProbeSuccess {
                    client_probe: Some(runtime.client_probe()),
                    ports: vec![PortType::new_tcp(cred.port)],
                    handle: Some(Box::new(ContainerProbeHandle {
                        client,
                        port: cred.port,
                        _ssl_temp_handles: ssl_temp_handles,
                    })),
                });
            }
            Err(e) => {
                if attempt < CONTAINER_PROBE_MAX_ATTEMPTS {
                    tracing::debug!(ip = %ctx.ip, attempt, error = %e, "Container client probe failed, retrying");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                } else {
                    return Err(classify_container_error(&e).with_context(format!(
                        "{} probe failed after {} attempts",
                        runtime.label(),
                        CONTAINER_PROBE_MAX_ATTEMPTS
                    )));
                }
            }
        }
    }

    Err(ProbeFailure::unreachable(format!(
        "{} probe exhausted all attempts",
        runtime.label()
    )))
}

/// Classify a container-client failure.
///
/// `new_docker_client` returns `anyhow::Error`, but it now preserves the underlying
/// `bollard::errors::Error` in the chain rather than formatting it into a string — so a socket
/// that refused us (fix the credential) is distinguishable from one nothing is listening on (fix
/// the address), which the single "probe failed after N attempts" message could never say.
fn classify_container_error(error: &anyhow::Error) -> ProbeFailure {
    let outcome = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<bollard::errors::Error>())
        .map(AttemptOutcome::from)
        .unwrap_or(AttemptOutcome::Unreachable);
    ProbeFailure::with_outcome(outcome, error.to_string())
}

/// Probe a container-runtime local Unix socket. `socket_path` of `None` uses the
/// Docker defaults (DOCKER_HOST / `/var/run/docker.sock`); `Some(path)` connects
/// to an explicit socket (e.g. the Podman socket).
pub async fn probe_socket(
    ctx: &ProbeContext<'_>,
    runtime: ContainerRuntime,
    socket_path: Option<String>,
) -> Result<ProbeSuccess, ProbeFailure> {
    match ctx
        .utils
        .new_container_socket_client(runtime, socket_path.clone())
        .await
    {
        Ok(client) => {
            tracing::debug!(runtime = runtime.label(), socket = ?socket_path, "Container socket probe succeeded");
            Ok(ProbeSuccess {
                client_probe: Some(runtime.client_probe()),
                ports: vec![],
                handle: Some(Box::new(ContainerProbeHandle {
                    client,
                    port: 0,
                    _ssl_temp_handles: vec![],
                })),
            })
        }
        Err(e) => Err(classify_container_error(&e)
            .with_context(format!("{} socket connection failed", runtime.label()))),
    }
}

/// Execute container scanning after a successful probe, enriching `host_data`
/// with discovered container services/ports/ip_addresses and bridge subnets.
pub async fn execute(
    ctx: &super::IntegrationContext<'_>,
    host_data: &mut crate::daemon::discovery::service::ops::HostData,
    _checkpoint: &Checkpoint<'_>,
    runtime: ContainerRuntime,
) -> Result<Completeness, super::IntegrationFailure> {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU8;

    use scanner::ContainerScanner;

    let handle = ctx
        .probe_handle
        .and_then(|h| h.downcast_ref::<ContainerProbeHandle>())
        .ok_or_else(|| anyhow::anyhow!("Missing ContainerProbeHandle"))?;

    // Find the runtime daemon service from host_data by id (matched via the
    // runtime's client-probe). Its service_id stamps every container/subnet.
    let runtime_def_id = runtime.service_def().id();
    let runtime_service_id = host_data
        .services
        .iter()
        .find(|s| s.base.service_definition.id() == runtime_def_id)
        .map(|s| s.id)
        .ok_or_else(|| {
            anyhow::anyhow!("{} daemon service not found in host_data", runtime.label())
        })?;

    let scanner = ContainerScanner {
        runtime,
        client: &handle.client,
        runtime_service_id,
        host_ip: ctx.ip,
        host_naming_fallback: ctx.host_naming_fallback,
        ops: ctx.ops,
        integration: ctx.integration,
        api_port: ctx.credential.as_container_proxy().map(|c| c.port),
        cancel: ctx.cancel,
        accept_invalid_certs: ctx.accept_invalid_certs,
        utils: ctx.utils,
    };

    // Bridge subnets are collected here because container→subnet mapping needs them below, but
    // they are NOT written into host_data until the containers are in hand — see the merge at the
    // end. Writing them here is what GH #650 actually was: the scan between this point and there
    // takes minutes, and a timeout in the middle left the host holding every bridge subnet and
    // none of the containers that give them meaning, reading as a clean success.
    //
    // The macvlan/ipvlan networks are kept apart: they never reach host_data, and only say which
    // containers have an address of their own on the LAN.
    let (bridge_subnets, lan_subnets): (Vec<Subnet>, Vec<Subnet>) = scanner
        .create_network_subnets()
        .await?
        .into_iter()
        .partition(Subnet::is_container_bridge_subnet);
    ctx.ops.report_progress(10).await.ok();

    // The engine reports the machine it runs on. Read before the scan so a slow container pass
    // cannot time it out; a failed read only means no OS from this source.
    let engine_os = match handle.client.info().await {
        Ok(info) => engine_host_os(&info),
        Err(e) => {
            tracing::debug!(error = %e, runtime = runtime.label(), "Container engine info unavailable");
            None
        }
    };

    let containers = scanner.get_containers_and_summaries().await?;
    let container_count = containers.len();
    ctx.ops.report_progress(20).await.ok();

    // A container with a macvlan/ipvlan endpoint is a host of its own under the runtime. Every
    // other container stays a service on the runtime's host, except the members of a LAN
    // container's network namespace, whose services go on its host.
    let (lan_containers, containers_on_host) =
        hosts::partition_lan_containers(containers, &lan_subnets);
    let lan_container_count = lan_containers.len();
    let bridge_count = containers_on_host.len();

    let mut host_interfaces = host_data.ip_addresses.clone();
    let containers_interfaces_and_subnets = interfaces::container_interfaces(
        runtime,
        &containers_on_host,
        &bridge_subnets,
        ctx.known_subnets,
        &mut host_interfaces,
    );

    let deadline = tokio::time::Instant::now()
        + CONTAINER_SCAN_TIMEOUT.mul_f32(CONTAINER_SCAN_SOFT_DEADLINE_FRACTION);
    let scan = scanner
        .scan_and_process_containers(
            containers_on_host,
            &containers_interfaces_and_subnets,
            Arc::new(AtomicU8::new(0)),
            deadline,
        )
        .await?;
    ctx.ops.report_progress(70).await.ok();

    let container_hosts = if lan_containers.is_empty() {
        hosts::ContainerHostsOutcome::default()
    } else {
        // Container hosts link to the runtime's stored service, so the runtime's host goes first.
        match scanner.submit_runtime_host(host_data).await {
            Ok(Some(stored_runtime_service_id)) => {
                // A LAN address belongs on the LAN, never on a container bridge.
                let placement_subnets: Vec<Subnet> =
                    merge_subnets(ctx.known_subnets, ctx.scanning_subnet, &host_data.subnets)
                        .into_iter()
                        .filter(|s| !s.is_container_bridge_subnet())
                        .collect();
                scanner
                    .record_container_hosts(
                        &lan_containers,
                        &lan_subnets,
                        &bridge_subnets,
                        &placement_subnets,
                        stored_runtime_service_id,
                        deadline,
                    )
                    .await?
            }
            Ok(None) => {
                tracing::warn!(
                    runtime = runtime.label(),
                    "The server returned no runtime service; containers with their own LAN address were not recorded"
                );
                hosts::ContainerHostsOutcome::default()
            }
            Err(e) => {
                tracing::warn!(
                    runtime = runtime.label(),
                    error = %e,
                    "Failed to record the runtime's host; containers with their own LAN address were not recorded"
                );
                hosts::ContainerHostsOutcome::default()
            }
        }
    };
    ctx.ops.report_progress(90).await.ok();

    tracing::info!(
        discovered = %scan.results.len(),
        container_hosts = container_hosts.recorded,
        total_containers = container_count,
        reached = scan.reached,
        runtime = runtime.label(),
        "Container scanning complete"
    );

    // Everything lands here, in one synchronous block with no await in it, so there is no point
    // at which a dropped future can catch this half-done. The scratch buffer the caller owns
    // makes that guarantee structural; keeping the writes together is what makes the code shape
    // match it. Container hosts are their own submissions and were sent above.
    for subnet in bridge_subnets {
        host_data.add_subnet(subnet);
    }
    if let Some(os) = engine_os {
        host_data.offer_os(Attributed::new(
            HostOsValue(os),
            AttributeSource::ContainerRuntimeInfo,
        ));
    }
    for result in scan.results {
        for service in result.services {
            host_data.add_service(service);
        }
        for port in result.ports {
            host_data.add_port(port);
        }
        for ip_address in result.ip_addresses {
            host_data.add_ip_address(ip_address);
        }
    }
    for port in container_hosts.published_ports {
        host_data.add_port(port);
    }
    // The API proxy owns the credential's port: the runtime is reached through it, not listening
    // on it, so the runtime service gives up its binding there.
    let api_proxy_id = runtime.api_proxy_def().id();
    let proxy_found = host_data
        .services
        .iter()
        .any(|s| s.base.service_definition.id() == api_proxy_id);
    if proxy_found && let Some(api_port) = scanner.api_port {
        release_runtime_api_port(host_data, runtime_service_id, api_port);
    }

    // A scan stopped by the soft deadline has a coherent subset — every container it did reach is
    // whole — so it is worth keeping, but only if the operator is told it is a subset.
    Ok(if scan.reached < bridge_count {
        Completeness::Partial(CollectionShortfall {
            what: "containers",
            collected: scan.reached,
            expected: bridge_count,
        })
    } else if container_hosts.recorded < lan_container_count {
        Completeness::Partial(CollectionShortfall {
            what: "container hosts",
            collected: container_hosts.recorded,
            expected: lan_container_count,
        })
    } else {
        Completeness::Complete
    })
}

/// Remove the runtime service's binding on `api_port`, which the container fronting its API owns.
fn release_runtime_api_port(
    host_data: &mut crate::daemon::discovery::service::ops::HostData,
    runtime_service_id: Uuid,
    api_port: u16,
) {
    let api_port_ids: Vec<Uuid> = host_data
        .ports
        .iter()
        .filter(|p| p.base.port_type == PortType::new_tcp(api_port))
        .map(|p| p.id)
        .collect();
    if let Some(runtime) = host_data
        .services
        .iter_mut()
        .find(|s| s.id == runtime_service_id)
    {
        runtime
            .base
            .bindings
            .retain(|b| !b.port_id().is_some_and(|id| api_port_ids.contains(&id)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::services::r#impl::definitions::ServiceDefinition;

    #[test]
    fn runtime_selects_distinct_service_definitions() {
        assert_eq!(ContainerRuntime::Docker.service_def().name(), "Docker");
        assert_eq!(ContainerRuntime::Podman.service_def().name(), "Podman");
        assert_ne!(
            ContainerRuntime::Docker.service_def().id(),
            ContainerRuntime::Podman.service_def().id()
        );
    }

    #[test]
    fn runtime_selects_distinct_client_probes() {
        assert_eq!(ContainerRuntime::Docker.client_probe(), ClientProbe::Docker);
        assert_eq!(ContainerRuntime::Podman.client_probe(), ClientProbe::Podman);
    }

    #[test]
    fn runtime_selects_distinct_bridge_subnet_types() {
        assert_eq!(
            ContainerRuntime::Docker.bridge_subnet_type(),
            SubnetType::DockerBridge
        );
        assert_eq!(
            ContainerRuntime::Podman.bridge_subnet_type(),
            SubnetType::PodmanBridge
        );
    }

    fn cidr(s: &str) -> IpCidr {
        s.parse().unwrap()
    }

    /// The runtime API is the only evidence that can produce a container subnet
    /// type, and `driver` is that evidence. Bridges are host-scoped so they name
    /// their owning runtime; MacVLAN/IpVLAN sit on the shared physical LAN and don't.
    #[test]
    fn subnet_from_network_types_by_driver() {
        let id = Uuid::new_v4();
        let build = |runtime: ContainerRuntime, driver: &str| {
            runtime.subnet_from_network(
                Uuid::nil(),
                cidr("172.17.0.0/16"),
                "net".into(),
                driver,
                id,
            )
        };

        let docker = build(ContainerRuntime::Docker, "bridge").expect("bridge is a network");
        assert_eq!(docker.base.subnet_type, SubnetType::DockerBridge);
        assert_eq!(docker.base.virtualization_service_id, Some(id));

        let podman = build(ContainerRuntime::Podman, "overlay").expect("overlay is a network");
        assert_eq!(podman.base.subnet_type, SubnetType::PodmanBridge);
        assert_eq!(podman.base.virtualization_service_id, Some(id));

        let macvlan = build(ContainerRuntime::Docker, "macvlan").expect("macvlan is a network");
        assert_eq!(macvlan.base.subnet_type, SubnetType::MacVlan);
        assert!(macvlan.base.virtualization_service_id.is_none());

        // Drivers with no routable L3 network of their own.
        assert!(build(ContainerRuntime::Docker, "host").is_none());
        assert!(build(ContainerRuntime::Docker, "none").is_none());
    }

    /// The filter must catch the bridges a runtime actually creates without
    /// claiming bridges it doesn't. `br-<12 hex>` is Docker's naming convention;
    /// a router's `br-guest`/`br-lan` is someone else's bridge, and treating it
    /// as Docker's is what produced #663.
    #[test]
    fn reserved_interface_names_exclude_foreign_bridges() {
        for reserved in [
            "docker0",
            "podman0",
            "cni-podman0",
            "docker_gwbridge",
            "br-1a2b3c4d5e6f",
            "BR-1A2B3C4D5E6F",
        ] {
            assert!(
                ContainerRuntime::reserves_host_interface_name(reserved),
                "{reserved} should be recognised as a container bridge"
            );
        }

        for foreign in [
            "br-guest",
            "br-lan",
            "br-iot",
            "br0",
            "eth0",
            "wlan0",
            "lo",
            "dockerhub",
        ] {
            assert!(
                !ContainerRuntime::reserves_host_interface_name(foreign),
                "{foreign} must not be claimed as a container bridge"
            );
        }
    }

    #[test]
    fn runtime_stamps_matching_service_virtualization() {
        let docker = ContainerRuntime::Docker.service_virtualization(
            Some("web".into()),
            Some("abc".into()),
            Some("proj".into()),
        );
        assert!(matches!(docker, ServiceVirtualization::Docker(_)));
        assert_eq!(docker.container_name(), Some("web"));

        let podman = ContainerRuntime::Podman.service_virtualization(
            Some("web".into()),
            Some("abc".into()),
            None,
        );
        assert!(matches!(podman, ServiceVirtualization::Podman(_)));
    }
}

/// The OS of the machine a container engine runs on, from its `info` response.
///
/// `OSType` is the Go runtime's `GOOS`, which says the family; `OperatingSystem` names the product
/// ("Ubuntu 24.04.1 LTS", or "Docker Desktop" for the VM Docker Desktop runs in) and `OSVersion` the
/// release. A type other than `linux` or `windows` names no family this build knows, so it writes
/// no OS.
fn engine_host_os(info: &bollard::models::SystemInfo) -> Option<HostOs> {
    let family = match info.os_type.as_deref()? {
        "linux" => HostOsFamily::Linux,
        "windows" => HostOsFamily::Windows,
        _ => return None,
    };
    let present = |s: &Option<String>| {
        s.as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let name = present(&info.operating_system);
    // `OperatingSystem` usually already carries the release ("Ubuntu 24.04.1 LTS" beside "24.04"),
    // so a second copy would only repeat it.
    let version = present(&info.os_version)
        .filter(|v| !name.as_deref().is_some_and(|n| n.contains(v.as_str())));
    Some(HostOs {
        family,
        name,
        version,
        edition: None,
        codename: None,
        kernel_version: present(&info.kernel_version),
    })
}

#[cfg(test)]
mod engine_os_tests {
    use super::*;

    fn info(
        os_type: &str,
        operating_system: &str,
        os_version: &str,
    ) -> bollard::models::SystemInfo {
        bollard::models::SystemInfo {
            os_type: Some(os_type.to_string()),
            operating_system: Some(operating_system.to_string()),
            os_version: Some(os_version.to_string()),
            kernel_version: Some("6.8.0-45-generic".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn the_engine_names_its_host_without_repeating_the_release() {
        let os = engine_host_os(&info("linux", "Ubuntu 24.04.1 LTS", "24.04")).unwrap();
        assert_eq!(os.family, HostOsFamily::Linux);
        assert_eq!(os.name.as_deref(), Some("Ubuntu 24.04.1 LTS"));
        assert_eq!(os.version, None);
        assert_eq!(os.kernel_version.as_deref(), Some("6.8.0-45-generic"));
    }

    #[test]
    fn a_windows_engine_keeps_a_release_its_name_does_not_carry() {
        let os = engine_host_os(&info(
            "windows",
            "Windows Server 2022 Datacenter",
            "10.0 20348",
        ))
        .unwrap();
        assert_eq!(os.family, HostOsFamily::Windows);
        assert_eq!(os.version.as_deref(), Some("10.0 20348"));
    }

    #[test]
    fn an_unknown_os_type_or_a_missing_one_writes_no_os() {
        assert_eq!(engine_host_os(&info("plan9", "Plan 9", "4")), None);
        assert_eq!(
            engine_host_os(&bollard::models::SystemInfo::default()),
            None
        );
    }
}
