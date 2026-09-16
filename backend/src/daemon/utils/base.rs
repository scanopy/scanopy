use crate::daemon::discovery::integration::container::ContainerRuntime;
use crate::server::interfaces::r#impl::base::{
    IfAdminStatus, IfOperStatus, Interface, InterfaceBase, if_type,
};
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue, mac_of,
};
use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::types::SubnetType;
use anyhow::Error;
use anyhow::anyhow;
use async_trait::async_trait;
use bollard::query_parameters::ListNetworksOptions;
use bollard::{API_DEFAULT_VERSION, Docker};
use cidr::IpCidr;
use local_ip_address::local_ip;
use mac_address::MacAddress;
// net-route has no BSD support; FreeBSD/OpenBSD use netdev instead (see
// `get_own_routing_table_gateway_ips`). Keep the import off BSD so net-route is never compiled there.
#[cfg(not(any(target_os = "freebsd", target_os = "openbsd")))]
use net_route::Handle;
use pnet::ipnetwork::IpNetwork;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

/// Negotiate the container daemon's API version after a successful ping.
///
/// bollard pins `API_DEFAULT_VERSION` and never negotiates, so it talks to a daemon's
/// Docker-compatible API using its newest response models. Podman's compat layer advertises
/// an older Docker API level, so those newer models can fail to deserialize — surfacing as
/// "no containers found." Negotiating downgrades the client to the daemon's advertised
/// version. `Docker` is cheaply cloneable (Arc internally), so we clone before negotiating
/// and fall back to the original default-version client if the `/version` call fails.
///
/// Best-effort and capped by a short timeout: the client is fully usable on the default
/// version, so a slow/unresponsive `/version` (e.g. a sluggish `podman machine` VM) must not
/// stall discovery. Without this cap the call inherits bollard's connect timeout (up to 120s),
/// which blocks every scan for two minutes before falling back.
const NEGOTIATE_VERSION_TIMEOUT: Duration = Duration::from_secs(5);

async fn negotiate_container_api_version(client: Docker) -> Docker {
    match tokio::time::timeout(
        NEGOTIATE_VERSION_TIMEOUT,
        client.clone().negotiate_version(),
    )
    .await
    {
        Ok(Ok(negotiated)) => negotiated,
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "Container API version negotiation failed; using default version");
            client
        }
        Err(_) => {
            tracing::warn!(
                timeout_secs = NEGOTIATE_VERSION_TIMEOUT.as_secs(),
                "Container API version negotiation timed out; using default version"
            );
            client
        }
    }
}

pub const SCAN_TIMEOUT: Duration = Duration::from_millis(800);

/// The daemon host's own NICs, after the `--interfaces` filter and the container-bridge skip.
///
/// Shared by `get_own_interfaces` (which keeps the addresses), `own_nics_as_interfaces` (which
/// keeps the ports), and the DCP sweep (`network::dcp`, which needs raw `NetworkInterface`s rather
/// than either derived shape) — `pub(crate)` so all three can never disagree about which NICs
/// belong to this host.
pub(crate) fn filtered_own_nics(
    interface_filter: &[String],
) -> Vec<pnet::datalink::NetworkInterface> {
    let all_interfaces = pnet::datalink::interfaces();

    let selected: Vec<_> = if interface_filter.is_empty() {
        all_interfaces
    } else {
        let filtered: Vec<_> = all_interfaces
            .into_iter()
            .filter(|iface| interface_filter.iter().any(|f| f == &iface.name))
            .collect();

        if filtered.is_empty() {
            tracing::warn!(
                filter = ?interface_filter,
                "No ip_addresses matched the filter. Check --ip_address argument."
            );
        } else {
            tracing::debug!(
                filter = ?interface_filter,
                matched = filtered.len(),
                "Filtered ip_addresses by --ip_addresses argument"
            );
        }

        filtered
    };

    selected
        .into_iter()
        // Container bridges on this host belong to the container integration,
        // which types them from the runtime API. Skip them here so an
        // unreachable runtime can't leave them to be created and scanned as
        // ordinary subnets (and so the heartbeat never reports them at all).
        .filter(|iface| {
            let reserved = ContainerRuntime::reserves_host_interface_name(&iface.name);
            if reserved {
                tracing::debug!(
                    interface = %iface.name,
                    "Skipping container-runtime bridge interface"
                );
            }
            !reserved
        })
        .collect()
}

/// One of this host's NICs, as an `Interface` row.
///
/// **Nothing here is typed as a physical port, deliberately.** `pnet` reports flags, not an
/// IANAifType, and the flags do not separate a real NIC from a virtual one: on a Mac, `en0` and
/// each of `en1`-`en6`, `bridge0`, `awdl0`, `llw0`, `ap1` and `anpi0/1/3` all report `8863`
/// (`UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST`) — identical. Typing them `ethernetCsmaCd` put
/// fifteen ports on the daemon host in the L2 view, of which one was real. Linux is milder but the
/// same shape (`veth` pairs, bond members, VLAN sub-interfaces).
///
/// So loopback and point-to-point are recorded honestly from the flags, and everything else is
/// `propVirtual` — an admission that we do not know. It costs nothing the rows exist for:
/// `find_host_by_mac`'s interface tier applies no `if_type` filter, so a chassis MAC still resolves
/// to this host. It only stops them being drawn, and `l2_builder` draws any interface that has a
/// resolved neighbour regardless of type — so a NIC lights up exactly when something authoritative
/// says it is cabled.
fn nic_to_interface(
    nic: &pnet::datalink::NetworkInterface,
    network_id: Uuid,
    host_id: Uuid,
) -> Interface {
    let if_type = if nic.is_loopback() {
        if_type::SOFTWARE_LOOPBACK
    } else if nic.is_point_to_point() {
        if_type::TUNNEL
    } else {
        if_type::PROP_VIRTUAL
    };

    Interface::new(InterfaceBase {
        host_id,
        network_id,
        if_index: Some(nic.index as i32),
        if_descr: Some(nic.name.clone()),
        if_name: Some(nic.name.clone()),
        if_alias: None,
        if_type: Some(if_type),
        speed_bps: None,
        admin_status: Some(IfAdminStatus::Up),
        oper_status: Some(if nic.is_up() {
            IfOperStatus::Up
        } else {
            IfOperStatus::Down
        }),
        // The daemon reading its own NIC — the host it runs on, not one it asked about.
        mac_address: nic_mac(nic)
            .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::DaemonSelfReport)),
        // Never set from here, exactly as the SNMP path doesn't (`snmp/mod.rs`). The ids the
        // daemon mints for its own `IPAddress` rows do not survive submission — the server matches
        // incoming addresses to the rows it already holds and keeps *their* ids. It builds an
        // `ip_address_id_remap` for precisely this (`hosts/service/create.rs`), but applies it to
        // credential assignments and bindings only, never to interfaces. Pointing at a minted id
        // therefore violates `interfaces_ip_address_id_fkey` and fails the whole submission, which
        // took self-report down with it: the phase never returned `Ok`, so `has_self_reported` was
        // never set, so every later scan retried self-report and the daemon-host interface phase
        // was never reached at all.
        ip_address_id: None,
        // Not an SNMP walk — no ipAddrTable to read, so this signal is unavailable here.
        ip_configured: false,
        neighbor_candidates: Vec::new(),
        fdb_macs: None,
        native_vlan_id: None,
        vlan_ids: None,
    })
}

/// A NIC's MAC, or `None` for the all-zero placeholder several platforms report.
fn nic_mac(iface: &pnet::datalink::NetworkInterface) -> Option<MacAddress> {
    match iface.mac {
        Some(mac) if !mac.octets().iter().all(|o| *o == 0) => Some(MacAddress::new(mac.octets())),
        _ => None,
    }
}

/// Cross-platform system utilities trait
#[async_trait]
pub trait DaemonUtils {
    fn new() -> Self;

    /// Get MAC address for an IP from ARP table
    async fn get_mac_address_for_ip(&self, ip: IpAddr) -> Result<Option<MacAddress>, Error>;

    fn get_fd_limit() -> Result<usize, Error>;

    fn get_own_ip_address(&self) -> Result<IpAddr, Error> {
        match local_ip() {
            Ok(ip) => {
                tracing::debug!(ip = %ip, "Detected local IP address");
                Ok(ip)
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to detect local IP address. This may occur in MACVLAN containers \
                     or environments without a default route."
                );
                Err(anyhow!("Failed to get local IP address: {}", e))
            }
        }
    }

    fn get_own_mac_address(&self) -> Result<Option<MacAddress>, Error> {
        mac_address::get_mac_address().map_err(|e| anyhow!("Failed to get own MAC address: {}", e))
    }

    fn get_own_hostname(&self) -> Option<String> {
        hostname::get()
            .ok()
            .map(|os_str| os_str.to_string_lossy().into_owned())
    }

    async fn get_own_interfaces(
        &self,
        network_id: Uuid,
        interface_filter: &[String],
    ) -> Result<
        (
            Vec<IPAddress>,
            Vec<Subnet>,
            HashMap<IpCidr, Option<MacAddress>>,
        ),
        Error,
    > {
        let ip_addresses = filtered_own_nics(interface_filter);

        tracing::debug!(
            interface_count = ip_addresses.len(),
            "Enumerating network ip_addresses"
        );

        for ip_address in &ip_addresses {
            tracing::debug!(
                name = %ip_address.name,
                index = ip_address.index,
                is_up = ip_address.is_up(),
                is_loopback = ip_address.is_loopback(),
                mac = ?ip_address.mac,
                ips = ?ip_address.ips,
                flags = ip_address.flags,
                "Found ip_address"
            );
        }

        // First pass: collect all interface data and potential subnets
        let mut potential_subnets: Vec<(String, IpNetwork)> = Vec::new();
        let mut interface_data: Vec<(String, IpAddr, Option<MacEvidence>)> = Vec::new();

        for ip_address in ip_addresses.into_iter() {
            let name = ip_address.name.clone();
            // The daemon reading its own NIC.
            let mac_address = nic_mac(&ip_address)
                .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::DaemonSelfReport));

            for ip in ip_address.ips.iter() {
                // APIPA (169.254.x.x) is defined as exactly /16 by RFC 3927.
                // Windows can report bogus prefixes (e.g. /0) via pnet — correct them.
                let ip = match ip {
                    IpNetwork::V4(v4)
                        if v4.ip().octets()[0] == 169
                            && v4.ip().octets()[1] == 254
                            && v4.prefix() != 16 =>
                    {
                        tracing::warn!(
                            ip = %v4.ip(),
                            reported_prefix = v4.prefix(),
                            "Correcting APIPA ip_address prefix to /16"
                        );
                        IpNetwork::V4(pnet::ipnetwork::Ipv4Network::new(v4.ip(), 16).unwrap_or(*v4))
                    }
                    other => *other,
                };
                interface_data.push((name.clone(), ip.ip(), mac_address.clone()));
                potential_subnets.push((name.clone(), ip));
            }
        }

        // Second pass: create unique subnets from valid networks
        let mut subnet_map: HashMap<IpCidr, Subnet> = HashMap::new();

        for (interface_name, ip_network) in potential_subnets {
            if let Some(subnet) = Subnet::from_discovery(interface_name, &ip_network, network_id) {
                subnet_map.entry(*subnet.base.cidr).or_insert(subnet);
            }
        }

        // Third pass: assign all ip_addresses to appropriate subnets
        let mut ip_addresses = Vec::new();
        let mut cidr_to_mac = HashMap::new();

        for (interface_name, ip_addr, mac_address) in interface_data {
            // Find which subnet this IP belongs to
            if let Some(subnet) = subnet_map
                .values()
                .filter(|s| s.base.cidr.contains(&ip_addr))
                .max_by_key(|s| s.base.cidr.network_length())
            {
                cidr_to_mac
                    .entry(*subnet.base.cidr)
                    .and_modify(|existing: &mut Option<MacAddress>| {
                        // Prefer a valid MAC over None
                        if existing.is_none() && mac_address.is_some() {
                            *existing = mac_of(&mac_address);
                        }
                    })
                    .or_insert(mac_of(&mac_address));

                ip_addresses.push(IPAddress::new(IPAddressBase {
                    network_id: subnet.base.network_id,
                    host_id: Uuid::nil(), // Placeholder - server will set correct host_id
                    name: Some(interface_name),
                    subnet_id: subnet.id,
                    ip_address: ip_addr,
                    mac_address,
                    position: ip_addresses.len() as i32,
                }));
            }
        }

        let subnets: Vec<Subnet> = subnet_map.into_values().collect();

        Ok((ip_addresses, subnets, cidr_to_mac))
    }

    /// The daemon host's own NICs as `Interface` rows, linked to the addresses they carry.
    ///
    /// The daemon host had none until now. Nothing else creates them: only the SNMP, UniFi and
    /// Instant On integrations build `InterfaceBase`, and none of those looks at the machine the
    /// daemon runs on. That absence is what breaks L2 resolution for the daemon host — a switch's
    /// `lldpRemChassisId` for a Linux server is lldpd's *primary* MAC, which need not belong to
    /// any NIC that carries an address Scanopy recorded, so `find_host_by_mac` falls through the
    /// `ip_addresses` tier and finds nothing on the `interfaces` tier to fall through *to*. That
    /// tier matches on MAC alone, with no `if_type` filter and collapsing to distinct hosts, so
    /// one row per NIC is enough for whichever MAC lldpd picked to name this host.
    ///
    /// See `nic_to_interface` for how a NIC is typed.
    fn own_nics_as_interfaces(
        &self,
        network_id: Uuid,
        host_id: Uuid,
        interface_filter: &[String],
    ) -> Vec<Interface> {
        filtered_own_nics(interface_filter)
            .into_iter()
            .map(|nic| nic_to_interface(&nic, network_id, host_id))
            .collect()
    }

    async fn new_docker_client(
        &self,
        docker_proxy: Result<Option<String>, Error>,
        docker_proxy_ssl_info: Result<Option<(String, String, String)>, Error>,
    ) -> Result<Docker, Error> {
        use tokio::time::timeout;

        const DOCKER_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

        tracing::debug!("Creating Docker client connection");
        let start = std::time::Instant::now();

        let client = if let Ok(Some(docker_proxy)) = docker_proxy {
            tracing::debug!(proxy = %docker_proxy, "Using Docker proxy");
            if docker_proxy.contains("https://")
                && let Ok(Some((cert, key, chain))) = docker_proxy_ssl_info
            {
                let cert_path = PathBuf::from(cert);
                let key_path = PathBuf::from(key);
                let chain_path = PathBuf::from(chain);

                Docker::connect_with_ssl(
                    &docker_proxy,
                    &key_path,
                    &cert_path,
                    &chain_path,
                    15,
                    API_DEFAULT_VERSION,
                )
                .map_err(|e| anyhow::Error::new(e).context("Failed to connect to Docker"))?
            } else {
                Docker::connect_with_http(&docker_proxy, 4, API_DEFAULT_VERSION)
                    .map_err(|e| anyhow::Error::new(e).context("Failed to connect to Docker"))?
            }
        } else {
            tracing::debug!("Using Docker local defaults");
            Docker::connect_with_local_defaults()
                .map_err(|e| anyhow::Error::new(e).context("Failed to connect to Docker"))?
        };

        // Ping Docker with retry and exponential backoff
        const MAX_PING_ATTEMPTS: u32 = 3;
        let mut last_error = None;
        for attempt in 1..=MAX_PING_ATTEMPTS {
            match timeout(DOCKER_CONNECT_TIMEOUT, client.ping()).await {
                Ok(Ok(_)) => {
                    tracing::debug!(
                        elapsed_ms = start.elapsed().as_millis(),
                        attempt,
                        "Docker client connected successfully"
                    );
                    return Ok(negotiate_container_api_version(client).await);
                }
                Ok(Err(e)) => {
                    let rendered = e.to_string();
                    // Keep the bollard error itself, not its text. `AttemptOutcome`'s
                    // classification downcasts to it to tell a refused credential from an
                    // unreachable socket, and a formatted string discards that.
                    last_error = Some(anyhow::Error::new(e).context("Docker ping failed"));
                    let e = rendered;
                    if attempt < MAX_PING_ATTEMPTS {
                        let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                        tracing::warn!(
                            attempt,
                            backoff_ms = backoff.as_millis(),
                            error = %e,
                            "Docker ping failed, retrying"
                        );
                        tokio::time::sleep(backoff).await;
                    }
                }
                Err(_) => {
                    last_error = Some(anyhow::anyhow!(
                        "Docker connection timed out after {DOCKER_CONNECT_TIMEOUT:?}"
                    ));
                    if attempt < MAX_PING_ATTEMPTS {
                        let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                        tracing::warn!(
                            attempt,
                            backoff_ms = backoff.as_millis(),
                            "Docker ping timed out, retrying"
                        );
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }
        tracing::warn!(
            elapsed_ms = start.elapsed().as_millis(),
            "Docker ping failed after {} attempts",
            MAX_PING_ATTEMPTS
        );
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Docker connection failed")))
    }

    /// Connect to a container runtime's local Unix socket via bollard's
    /// Docker-compatible client. `Some(path)` connects to an explicit socket.
    /// `None` falls back to bollard's Docker defaults (`DOCKER_HOST` /
    /// `/var/run/docker.sock`) — valid ONLY for `ContainerRuntime::Docker`. For
    /// `Podman`, a `None` path is an error: Podman must never silently connect to
    /// the Docker socket (which would discover Docker containers as Podman). Pings
    /// to verify before returning.
    async fn new_container_socket_client(
        &self,
        runtime: ContainerRuntime,
        socket_path: Option<String>,
    ) -> Result<Docker, Error> {
        use tokio::time::timeout;

        const DOCKER_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
        const MAX_PING_ATTEMPTS: u32 = 3;

        let start = std::time::Instant::now();
        let client = match (&socket_path, runtime) {
            (Some(path), _) => {
                tracing::debug!(socket = %path, runtime = runtime.label(), "Connecting to container socket");
                Docker::connect_with_socket(path, 120, API_DEFAULT_VERSION).map_err(|e| {
                    anyhow::anyhow!("Failed to connect to container socket {}: {}", path, e)
                })?
            }
            // No explicit path: only Docker may use bollard's local defaults.
            (None, ContainerRuntime::Docker) => {
                tracing::debug!("Using Docker local defaults");
                Docker::connect_with_local_defaults()
                    .map_err(|e| anyhow::Error::new(e).context("Failed to connect to Docker"))?
            }
            // Podman with no resolved socket: fail cleanly rather than fall back to
            // the Docker default socket.
            (None, ContainerRuntime::Podman) => {
                return Err(anyhow::anyhow!(
                    "No Podman socket found — checked CONTAINER_HOST, /run/podman/podman.sock, \
                     and $XDG_RUNTIME_DIR/podman/podman.sock. Set the credential's socket_path \
                     or CONTAINER_HOST to the Podman socket."
                ));
            }
        };

        let mut last_error = None;
        for attempt in 1..=MAX_PING_ATTEMPTS {
            match timeout(DOCKER_CONNECT_TIMEOUT, client.ping()).await {
                Ok(Ok(_)) => return Ok(negotiate_container_api_version(client).await),
                Ok(Err(e)) => {
                    last_error = Some(format!("Container socket ping failed: {}", e));
                }
                Err(_) => {
                    last_error = Some(format!(
                        "Container socket connection timed out after {:?}",
                        DOCKER_CONNECT_TIMEOUT
                    ));
                }
            }
            if attempt < MAX_PING_ATTEMPTS {
                let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                tokio::time::sleep(backoff).await;
            }
        }
        tracing::debug!(
            elapsed_ms = start.elapsed().as_millis(),
            "Container socket ping failed after {} attempts",
            MAX_PING_ATTEMPTS
        );
        Err(anyhow::anyhow!(last_error.unwrap_or_else(|| {
            "Container socket connection failed".to_string()
        })))
    }

    async fn get_subnets_from_docker_networks(
        &self,
        network_id: Uuid,
        client: &Docker,
        runtime: ContainerRuntime,
        runtime_service_id: Uuid,
    ) -> Result<Vec<Subnet>, Error> {
        let subnets: Vec<Subnet> = client
            .list_networks(None::<ListNetworksOptions>)
            .await?
            .into_iter()
            .filter_map(|n| {
                let driver = n.driver.clone().unwrap_or_else(|| "bridge".to_string());
                let network_name = n.name.clone().unwrap_or("Unknown Network".to_string());
                let configs = n.ipam.and_then(|ipam| ipam.config)?;
                Some((network_name, driver, configs))
            })
            .flat_map(|(network_name, driver, configs)| {
                configs
                    .iter()
                    .filter_map(|c| {
                        let cidr = IpCidr::from_str(c.subnet.as_ref()?).ok()?;
                        runtime.subnet_from_network(
                            network_id,
                            cidr,
                            network_name.clone(),
                            &driver,
                            runtime_service_id,
                        )
                    })
                    .collect::<Vec<Subnet>>()
            })
            .collect();

        Ok(subnets)
    }

    /// Collect all gateway IPs from the host routing table.
    ///
    /// Uses `net-route` on linux/macos/windows and `netdev` on FreeBSD/OpenBSD (net-route has no
    /// BSD cfg arm). Both arms return the same shape — every distinct routing-table gateway — so
    /// callers are platform-agnostic.
    #[cfg(not(any(target_os = "freebsd", target_os = "openbsd")))]
    async fn get_own_routing_table_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        let routing_handle = Handle::new()?;
        let routes = routing_handle.list().await?;

        Ok(routes
            .into_iter()
            .filter_map(|r| match r.gateway {
                Some(gateway) if gateway != r.destination => Some(gateway),
                _ => None,
            })
            .collect())
    }

    /// BSD variant: net-route lacks BSD cfg arms, so use netdev, which does BSD gateway discovery
    /// via `libc`/sysctl in-code. Each interface exposes its gateway's v4/v6 addresses; the union
    /// reproduces the "all routing-table gateways" set the net-route arm returns.
    ///
    /// Hardware-validation note: confirm on a multi-homed FreeBSD host that this matches the full
    /// gateway set; if netdev under-reports, fall back to a `sysctl(NET_RT_DUMP)` route reader
    /// isolated to this cfg arm.
    #[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
    async fn get_own_routing_table_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        let gateways = netdev::get_interfaces()
            .into_iter()
            .filter_map(|iface| iface.gateway)
            .flat_map(|gw| {
                gw.ipv4
                    .into_iter()
                    .map(IpAddr::V4)
                    .chain(gw.ipv6.into_iter().map(IpAddr::V6))
                    .collect::<Vec<_>>()
            })
            .collect();

        Ok(gateways)
    }

    /// Get optimal concurrency for ARP scanning (OS-specific due to BPF limits on macOS)
    fn get_optimal_arp_concurrency(&self) -> Result<usize, Error>;

    /// Get optimal concurrency for deep port scanning.
    ///
    /// # Arguments
    /// * `port_batch_size` - Number of ports scanned concurrently per host in deep scan
    /// * `arp_subnet_count` - Number of ARP datalink channels currently open (2 FDs each)
    fn get_optimal_deep_scan_concurrency(
        &self,
        port_batch_size: usize,
        arp_subnet_count: usize,
    ) -> Result<usize, Error>;
}

/// Merge host (physical) and Docker subnets, giving host subnets precedence.
/// - Host subnets are always kept
/// - Docker subnets with CIDRs matching host subnets are dropped (host wins),
///   but if the dropped one was a container *bridge*, the host record adopts its
///   `subnet_type` and `virtualization`
///
/// The host record wins on identity (its CIDR and interface-derived name are
/// what the rest of discovery keys off), but its type is only a guess from an
/// interface name, whereas the Docker record's came from the runtime API. For a
/// bridge — a distinct L3 network the runtime owns — that authoritative
/// classification carries over, or the bridge would keep the guess and lose the
/// `virtualization` that scopes it to its owning runtime service.
///
/// MacVLAN/IpVLAN are deliberately excluded: they are not separate networks but
/// overlays on the physical LAN the host is already on, so the host's own
/// classification is the correct one and adopting theirs would relabel a real
/// LAN as a container network.
///
/// Callers that don't want DockerBridge subnets (e.g., self-report) should
/// filter them separately after calling this function.
pub fn merge_host_and_docker_subnets(
    host_subnets: Vec<Subnet>,
    docker_subnets: Vec<Subnet>,
) -> Vec<Subnet> {
    let host_cidrs: HashSet<IpCidr> = host_subnets.iter().map(|s| *s.base.cidr).collect();

    let mut authoritative: HashMap<IpCidr, (SubnetType, Option<Uuid>)> = HashMap::new();

    let filtered_docker: Vec<Subnet> = docker_subnets
        .into_iter()
        .filter(|s| {
            let dominated_by_host = host_cidrs.contains(&s.base.cidr);
            if dominated_by_host {
                tracing::debug!(
                    cidr = %s.base.cidr,
                    subnet_type = ?s.base.subnet_type,
                    "Filtering out Docker subnet (host takes precedence)"
                );
                if s.base.subnet_type.is_container_bridge() {
                    authoritative.insert(
                        *s.base.cidr,
                        (s.base.subnet_type, s.base.virtualization_service_id),
                    );
                }
            }
            !dominated_by_host
        })
        .collect();

    let host_subnets: Vec<Subnet> = host_subnets
        .into_iter()
        .map(|mut s| {
            if let Some((subnet_type, virtualization_service_id)) =
                authoritative.remove(&s.base.cidr)
            {
                s.base.subnet_type = subnet_type;
                s.base.virtualization_service_id = virtualization_service_id;
            }
            s
        })
        .collect();

    [host_subnets, filtered_docker].concat()
}

#[cfg(target_os = "linux")]
use crate::daemon::utils::linux::LinuxDaemonUtils;
#[cfg(target_os = "linux")]
pub type PlatformDaemonUtils = LinuxDaemonUtils;

#[cfg(target_os = "macos")]
use crate::daemon::utils::macos::MacOsDaemonUtils;
#[cfg(target_os = "macos")]
pub type PlatformDaemonUtils = MacOsDaemonUtils;

#[cfg(target_family = "windows")]
use crate::daemon::utils::windows::WindowsDaemonUtils;
#[cfg(target_family = "windows")]
pub type PlatformDaemonUtils = WindowsDaemonUtils;

#[cfg(target_os = "freebsd")]
use crate::daemon::utils::bsd::BsdDaemonUtils;
#[cfg(target_os = "freebsd")]
pub type PlatformDaemonUtils = BsdDaemonUtils;

pub fn create_system_utils() -> PlatformDaemonUtils {
    PlatformDaemonUtils::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::shared::types::entities::EntitySource;
    use crate::server::subnets::r#impl::base::SubnetBase;
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
    use pnet::datalink::NetworkInterface;
    use std::str::FromStr;

    fn make_subnet(cidr: &str, subnet_type: SubnetType) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(IpCidr::from_str(cidr).unwrap()),
                AttributeSource::DaemonSelfReport,
            ),
            network_id: Uuid::nil(),
            name: String::new(),
            description: None,
            subnet_type,
            virtualization_service_id: None,
            source: EntitySource::Manual,
            tags: Vec::new(),
        })
    }

    #[test]
    fn macvlan_overlap_keeps_physical_only() {
        let host = vec![make_subnet("192.168.1.0/24", SubnetType::Lan)];
        let docker = vec![make_subnet("192.168.1.0/24", SubnetType::MacVlan)];

        let result = merge_host_and_docker_subnets(host, docker);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].base.subnet_type, SubnetType::Lan);
    }

    #[test]
    fn docker_bridge_kept_when_no_overlap() {
        let host = vec![];
        let docker = vec![make_subnet("172.17.0.0/16", SubnetType::DockerBridge)];

        let result = merge_host_and_docker_subnets(host, docker);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].base.subnet_type, SubnetType::DockerBridge);
    }

    /// A bridge the host is attached to appears twice: once from the host's own
    /// interface (typed by name, no virtualization) and once from the runtime
    /// API. The host row wins on identity, but must not keep the name guess —
    /// losing the API's type and virtualization would leave the bridge
    /// unrecognised and unscoped to its owning runtime service.
    #[test]
    fn bridge_overlap_adopts_authoritative_classification() {
        let service_id = Uuid::new_v4();
        let host = vec![make_subnet("172.17.0.0/16", SubnetType::Lan)];
        let mut bridge = make_subnet("172.17.0.0/16", SubnetType::DockerBridge);
        bridge.base.virtualization_service_id = Some(service_id);

        let result = merge_host_and_docker_subnets(host, vec![bridge]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].base.subnet_type, SubnetType::DockerBridge);
        assert_eq!(result[0].base.virtualization_service_id, Some(service_id));
    }

    // The classic `IFF_*` bits, which is what pnet puts in `NetworkInterface::flags` on every
    // platform it supports. Spelled out rather than taken from `libc`, which does not define them
    // on Windows — where this file still has to compile.
    const IFF_UP: u32 = 0x1;
    const IFF_LOOPBACK: u32 = 0x8;
    const IFF_POINTOPOINT: u32 = 0x10;

    fn make_nic(name: &str, index: u32, mac: Option<[u8; 6]>, flags: u32) -> NetworkInterface {
        NetworkInterface {
            name: name.to_string(),
            description: String::new(),
            index,
            mac: mac.map(|o| pnet::util::MacAddr::new(o[0], o[1], o[2], o[3], o[4], o[5])),
            ips: Vec::new(),
            flags,
        }
    }

    /// A NIC that carries no address Scanopy recorded still becomes an interface row.
    ///
    /// This is the whole point of collecting them. lldpd elects one of the host's MACs as its
    /// chassis id and a switch advertises that MAC back; on a multi-NIC server the elected NIC is
    /// often not one whose address falls in a scanned subnet, so it has no `ip_addresses` row to
    /// be found through. Dropping address-less NICs would leave exactly that MAC unfindable and
    /// the neighbour record unresolved — the bug this exists to close.
    #[test]
    fn a_nic_with_no_recorded_address_still_yields_a_row_carrying_its_mac() {
        let mac = [0x02, 0x42, 0xac, 0x11, 0x00, 0x02];
        let nic = make_nic("ens1f0np0", 7, Some(mac), IFF_UP);

        let entry = nic_to_interface(&nic, Uuid::new_v4(), Uuid::new_v4());

        assert_eq!(mac_of(&entry.base.mac_address), Some(MacAddress::new(mac)));
        assert_eq!(entry.base.if_index, Some(7));
        assert_eq!(entry.base.if_name.as_deref(), Some("ens1f0np0"));
    }

    /// No NIC is claimed to be a physical port, because the flags cannot tell us.
    ///
    /// A Mac reports `8863` for its real `en0` and for `bridge0`, `awdl0`, `ap1` and the
    /// `anpi`/Thunderbolt interfaces alike; asserting `ethernetCsmaCd` off the back of that put
    /// fifteen ports on the daemon host in L2, of which one was real. Asserted against
    /// `EXCLUDED_IF_TYPES` rather than the numbers, so it keeps meaning what it says if the list
    /// changes. Whether a NIC is *drawn* is `l2_builder`'s call, and it draws any interface that
    /// has a resolved neighbour whatever its type.
    #[test]
    fn no_nic_is_typed_as_a_physical_port() {
        let network_id = Uuid::new_v4();
        let host_id = Uuid::new_v4();

        for (name, flags) in [
            ("eno1", IFF_UP),
            ("lo", IFF_UP | IFF_LOOPBACK),
            ("utun4", IFF_UP | IFF_POINTOPOINT),
        ] {
            let entry = nic_to_interface(
                &make_nic(name, 2, Some([0, 1, 2, 3, 4, 5]), flags),
                network_id,
                host_id,
            );
            assert!(
                entry
                    .base
                    .if_type
                    .is_some_and(|t| if_type::EXCLUDED_IF_TYPES.contains(&t)),
                "{name} was typed {:?} — the daemon cannot know a NIC is a physical port",
                entry.base.if_type
            );
        }
    }

    #[test]
    fn no_overlap_keeps_both() {
        let host = vec![make_subnet("192.168.1.0/24", SubnetType::Lan)];
        let docker = vec![make_subnet("10.0.0.0/8", SubnetType::IpVlan)];

        let result = merge_host_and_docker_subnets(host, docker);
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn podman_socket_without_path_errors_never_uses_docker_defaults() {
        // Regression: a Podman socket credential with no resolvable socket path must
        // fail cleanly rather than fall back to Docker's local socket (which would
        // discover Docker containers stamped as Podman). The None+Podman arm returns
        // before any connection attempt, so this is deterministic with no I/O.
        let utils = create_system_utils();
        let result = utils
            .new_container_socket_client(ContainerRuntime::Podman, None)
            .await;
        assert!(result.is_err(), "Podman + no socket path must error");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Podman socket"),
            "error should name the missing Podman socket, got: {msg}"
        );
    }

    #[test]
    fn mixed_filters_only_overlapping_docker() {
        let host = vec![make_subnet("192.168.1.0/24", SubnetType::Lan)];
        let docker = vec![
            make_subnet("192.168.1.0/24", SubnetType::MacVlan),
            make_subnet("172.17.0.0/16", SubnetType::DockerBridge),
            make_subnet("10.0.0.0/8", SubnetType::IpVlan),
        ];

        let result = merge_host_and_docker_subnets(host, docker);
        // MacVlan dropped (overlaps host), Bridge + IpVlan kept (no overlap)
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].base.subnet_type, SubnetType::Lan);
        assert_eq!(result[1].base.subnet_type, SubnetType::DockerBridge);
        assert_eq!(result[2].base.subnet_type, SubnetType::IpVlan);
    }

    #[test]
    fn apipa_with_bogus_prefix_corrected_to_16() {
        use pnet::ipnetwork::{IpNetwork, Ipv4Network};
        use std::net::Ipv4Addr;

        // Simulate a Windows APIPA interface reporting /0
        let bogus = IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(169, 254, 1, 100), 0).unwrap());

        let corrected = match bogus {
            IpNetwork::V4(v4)
                if v4.ip().octets()[0] == 169
                    && v4.ip().octets()[1] == 254
                    && v4.prefix() != 16 =>
            {
                IpNetwork::V4(Ipv4Network::new(v4.ip(), 16).unwrap_or(v4))
            }
            other => other,
        };

        match corrected {
            IpNetwork::V4(v4) => assert_eq!(v4.prefix(), 16),
            _ => panic!("Expected V4"),
        }
    }

    #[test]
    fn apipa_with_correct_prefix_unchanged() {
        use pnet::ipnetwork::{IpNetwork, Ipv4Network};
        use std::net::Ipv4Addr;

        let correct = IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(169, 254, 1, 100), 16).unwrap());

        let result = match correct {
            IpNetwork::V4(v4)
                if v4.ip().octets()[0] == 169
                    && v4.ip().octets()[1] == 254
                    && v4.prefix() != 16 =>
            {
                IpNetwork::V4(Ipv4Network::new(v4.ip(), 16).unwrap_or(v4))
            }
            other => other,
        };

        match result {
            IpNetwork::V4(v4) => assert_eq!(v4.prefix(), 16),
            _ => panic!("Expected V4"),
        }
    }

    #[test]
    fn non_apipa_with_bogus_prefix_not_corrected() {
        use pnet::ipnetwork::{IpNetwork, Ipv4Network};
        use std::net::Ipv4Addr;

        // 10.0.0.1/8 should NOT be corrected (not APIPA)
        let normal = IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(10, 0, 0, 1), 8).unwrap());

        let result = match normal {
            IpNetwork::V4(v4)
                if v4.ip().octets()[0] == 169
                    && v4.ip().octets()[1] == 254
                    && v4.prefix() != 16 =>
            {
                IpNetwork::V4(Ipv4Network::new(v4.ip(), 16).unwrap_or(v4))
            }
            other => other,
        };

        match result {
            IpNetwork::V4(v4) => assert_eq!(v4.prefix(), 8),
            _ => panic!("Expected V4"),
        }
    }
}
