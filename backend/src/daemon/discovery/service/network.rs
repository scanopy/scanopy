use crate::daemon::discovery::service::base::{
    CreatesDiscoveredEntities, DiscoversNetworkedEntities, DiscoveryRunner, RunsDiscovery,
};
use crate::daemon::discovery::types::base::{DiscoveryCriticalError, DiscoverySessionUpdate};
use crate::daemon::utils::arp::{self, ArpScanResult};
use crate::daemon::utils::scanner::{
    ScanConcurrencyController, can_arp_scan, scan_endpoints, scan_tcp_ports, scan_udp_ports,
};
use crate::daemon::utils::snmp::{self, IfTableEntry};
use crate::server::credentials::r#impl::mapping::{
    DockerProxyQueryCredential, ResolvedCredential, SnmpCredentialMapping, SnmpQueryCredential,
};
use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::discovery::r#impl::scan_settings::{ScanSettings, defaults};
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback};
use crate::server::if_entries::r#impl::base::{
    IfAdminStatus, IfEntry, IfEntryBase, IfOperStatus, if_type,
};
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::{Service, ServiceMatchBaselineParams};
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::types::SubnetTypeDiscriminants;
use crate::{
    daemon::utils::base::DaemonUtils,
    server::{
        daemons::r#impl::{api::DaemonDiscoveryRequest, base::DaemonMode},
        hosts::r#impl::{
            api::{DiscoveryHostRequest, HostResponse},
            base::{Host, HostBase},
        },
        subnets::r#impl::base::Subnet,
    },
};
use anyhow::Error;
use async_trait::async_trait;
use cidr::IpCidr;
use futures::{StreamExt, future::try_join_all};
use mac_address::MacAddress;
use pnet::datalink;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{net::IpAddr, sync::Arc};
use strum::IntoDiscriminant;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Per-host data discovered during deep_scan_host().
/// Used by subsequent discovery phases (e.g., Docker container scanning) to link
/// containers to the correct virtualizing service and provide host interfaces.
#[derive(Debug, Clone, Default)]
pub struct DiscoveredHostData {
    pub docker_service_id: Option<Uuid>,
    pub interfaces: Vec<Interface>,
}

/// Grace period to wait for late ARP arrivals after the last deep scan completes
const LATE_ARRIVAL_GRACE_PERIOD: Duration = Duration::from_secs(30);

/// Hard maximum duration for a single discovery run (safety net)
const MAX_DISCOVERY_DURATION: Duration = Duration::from_secs(21600); // 6 hours

/// Maximum interval between progress reports (heartbeat even if progress unchanged)
const MAX_PROGRESS_REPORT_INTERVAL: Duration = Duration::from_secs(30);

// Progress phase weights (must sum to 100)
const PROGRESS_ARP_PHASE: u8 = 30; // 0-30%: ARP discovery
const PROGRESS_DEEP_SCAN_PHASE: u8 = 65; // 30-95%: Deep scanning
const PROGRESS_GRACE_PHASE: u8 = 5; // 95-100%: Grace period

/// Docker scan cost in batch-equivalents (~60s ≈ 60 batches)
const DOCKER_BATCH_WEIGHT: usize = 60;

#[derive(Default)]
pub struct NetworkScanDiscovery {
    subnet_ids: Option<Vec<Uuid>>,
    host_naming_fallback: HostNamingFallback,
    snmp_credentials: SnmpCredentialMapping,
    scan_settings: ScanSettings,
    /// Docker credentials indexed by target IP for remote Docker scanning
    docker_credentials: HashMap<IpAddr, ResolvedCredential<DockerProxyQueryCredential>>,
    /// Precomputed set of ports for light scans (discovery + credential ports)
    light_scan_ports: HashSet<u16>,
}

impl NetworkScanDiscovery {
    pub fn new(
        subnet_ids: Option<Vec<Uuid>>,
        host_naming_fallback: HostNamingFallback,
        snmp_credentials: SnmpCredentialMapping,
        scan_settings: ScanSettings,
        docker_credentials: HashMap<IpAddr, ResolvedCredential<DockerProxyQueryCredential>>,
    ) -> Self {
        // Build light scan port set: discovery ports + credential custom ports
        let mut light_scan_ports: HashSet<u16> = Service::all_discovery_ports()
            .iter()
            .filter(|p| p.is_tcp())
            .map(|p| p.number())
            .collect();

        // Add custom ports from DockerProxy credentials
        for cred in docker_credentials.values() {
            light_scan_ports.insert(cred.credential.port);
        }

        // Add SNMP standard ports if SNMP credentials are present
        if snmp_credentials.default_credential.is_some()
            || !snmp_credentials.ip_overrides.is_empty()
        {
            light_scan_ports.insert(161);
            light_scan_ports.insert(1161);
        }

        Self {
            subnet_ids,
            host_naming_fallback,
            snmp_credentials,
            scan_settings,
            docker_credentials,
            light_scan_ports,
        }
    }
}

pub struct DeepScanParams<'a> {
    ip: IpAddr,
    subnet: &'a Subnet,
    mac: Option<MacAddress>,
    cancel: CancellationToken,
    scan_rate_pps: u32,
    port_scan_batch_size: usize,
    gateway_ips: &'a [IpAddr],
    /// Optional counter for batch-level progress tracking
    batches_completed: Option<&'a Arc<AtomicUsize>>,
    /// Total batches counter - for non-interfaced hosts, we add to this AFTER
    /// the responsiveness check passes (so only responsive hosts are counted)
    total_batches: Option<&'a Arc<AtomicUsize>>,
    /// Hosts discovered counter - for non-interfaced hosts, we increment AFTER
    /// the responsiveness check passes (so only responsive hosts are counted)
    hosts_discovered: Option<&'a Arc<AtomicUsize>>,
    /// Number of batches expected for a full port scan of this host
    batches_per_host: usize,
    /// SNMP credentials ordered by specificity: IP override → network default → "public"
    snmp_credentials: Vec<ResolvedCredential<SnmpQueryCredential>>,
    /// Shared concurrency controller for graceful FD exhaustion handling
    scan_controller: Arc<ScanConcurrencyController>,
    /// Whether to probe raw-socket ports (9100-9107) during endpoint scanning
    probe_raw_socket_ports: bool,
    /// Host ID from early reporting — reused in final create_host to update rather than duplicate
    early_host_id: Uuid,
    /// Docker credential for this host, if any
    docker_credential: Option<ResolvedCredential<DockerProxyQueryCredential>>,
    /// Whether this is a full 65k port scan (vs light scan with discovery ports only)
    is_full_scan: bool,
    /// Precomputed light scan port set (used when is_full_scan is false)
    light_scan_ports: &'a HashSet<u16>,
}

impl CreatesDiscoveredEntities for DiscoveryRunner<NetworkScanDiscovery> {}

#[async_trait]
impl RunsDiscovery for DiscoveryRunner<NetworkScanDiscovery> {
    fn discovery_type(&self) -> DiscoveryType {
        DiscoveryType::Network {
            subnet_ids: self.domain.subnet_ids.clone(),
            host_naming_fallback: self.domain.host_naming_fallback,
            snmp_credentials: self.domain.snmp_credentials.clone(),
        }
    }

    async fn discover(
        &self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        // Ignore docker bridge subnets, they are discovered through Docker Discovery
        let subnets: Vec<Subnet> = match self.discover_create_subnets(&cancel).await {
            Ok(subnets) => subnets,
            Err(e) => {
                // Pre-start failure: initialize a minimal session so we can report the error
                let daemon_id = self.as_ref().config_store.get_id().await?;
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

        let discovery_result = self
            .scan_and_process_hosts(subnets, cancel.clone())
            .await
            .map(|_| ());

        self.finish_discovery(discovery_result, cancel.clone())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl DiscoversNetworkedEntities for DiscoveryRunner<NetworkScanDiscovery> {
    async fn get_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        self.as_ref()
            .utils
            .get_own_routing_table_gateway_ips()
            .await
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

        // Target specific subnets if provided in discovery type
        let subnets = if let Some(subnet_ids) = &self.domain.subnet_ids {
            let all_subnets = self.get_subnets().await?;
            all_subnets
                .into_iter()
                .filter(|s| subnet_ids.contains(&s.id))
                .collect()

        // Target all interfaced subnets if not
        } else {
            let interface_filter = self.as_ref().config_store.get_interfaces().await?;
            let (_, subnets, _) = self
                .as_ref()
                .utils
                .get_own_interfaces(
                    self.discovery_type(),
                    daemon_id,
                    network_id,
                    &interface_filter,
                )
                .await?;

            // Filter out docker bridge subnets (handled in docker discovery).
            // Size filtering for non-interfaced subnets is done later in
            // scan_and_process_hosts() where subnet_cidr_to_mac is available.
            let subnets: Vec<Subnet> = subnets
                .into_iter()
                .filter(|s| {
                    if s.base.subnet_type.discriminant() == SubnetTypeDiscriminants::DockerBridge {
                        tracing::warn!("Skipping {} with CIDR {}, docker bridge subnets are scanned in docker discovery", s.base.name, s.base.cidr);
                        return false
                    }

                    true
                })
                .collect();
            let subnet_futures = subnets
                .iter()
                .map(|subnet| self.create_subnet(subnet, cancel));
            try_join_all(subnet_futures).await?
        };

        Ok(subnets)
    }
}

impl DiscoveryRunner<NetworkScanDiscovery> {
    pub async fn scan_and_process_hosts(
        &self,
        subnets: Vec<Subnet>,
        cancel: CancellationToken,
    ) -> Result<Vec<(IpAddr, Host, DiscoveredHostData)>, Error> {
        let session = self.as_ref().get_session().await?;

        let interface_filter = self.as_ref().config_store.get_interfaces().await?;
        let (_, _, subnet_cidr_to_mac) = self
            .as_ref()
            .utils
            .get_own_interfaces(
                self.discovery_type(),
                session.info.daemon_id,
                session.info.network_id,
                &interface_filter,
            )
            .await?;

        // Filter non-interfaced subnets by size - ARP-scannable subnets of any
        // size are fine (ARP finds responsive hosts first), but non-interfaced
        // subnets require full port scanning of every IP which is too slow for
        // large CIDRs
        let subnets: Vec<Subnet> = subnets
            .into_iter()
            .filter(|s| {
                // Loopback subnets are not scannable
                if s.base.subnet_type.is_loopback() {
                    return false;
                }
                let is_interfaced = subnet_cidr_to_mac
                    .get(&s.base.cidr)
                    .and_then(|m| *m)
                    .is_some();
                if !is_interfaced && s.base.cidr.network_length() < 16 {
                    tracing::warn!(
                        subnet = %s.base.name,
                        cidr = %s.base.cidr,
                        "Skipping non-interfaced subnet larger than /16, port scanning would take too long"
                    );
                    return false;
                }
                true
            })
            .collect();

        let all_ips_with_subnets: Vec<(IpAddr, Subnet)> = subnets
            .iter()
            .flat_map(|subnet| {
                self.determine_scan_order(&subnet.base.cidr)
                    .map(move |ip| (ip, subnet.clone()))
            })
            .collect();

        let total_ips = all_ips_with_subnets.len();

        // Get scan settings from discovery request, falling back to defaults
        let use_npcap = self.domain.scan_settings.use_npcap_arp;
        let arp_retries = self
            .domain
            .scan_settings
            .arp_retries
            .unwrap_or(defaults::arp_retries());
        let arp_rate_pps = self
            .domain
            .scan_settings
            .arp_rate_pps
            .unwrap_or(defaults::arp_rate_pps());
        let scan_rate_pps = self
            .domain
            .scan_settings
            .scan_rate_pps
            .unwrap_or(defaults::scan_rate_pps());
        let port_scan_batch_size = self
            .domain
            .scan_settings
            .port_scan_batch_size
            .unwrap_or(defaults::port_scan_batch_size())
            .clamp(16, 1000);

        // Check ARP capability once before partitioning
        let arp_available = can_arp_scan(use_npcap);

        // Partition IPs - only use ARP path if we have capability
        let (interfaced_ips, non_interfaced_ips): (Vec<_>, Vec<_>) = if arp_available {
            all_ips_with_subnets.into_iter().partition(|(_, subnet)| {
                subnet_cidr_to_mac
                    .get(&subnet.base.cidr)
                    .and_then(|m| *m)
                    .is_some()
            })
        } else {
            // No ARP capability - treat all as non-interfaced (port scan only)
            (Vec::new(), all_ips_with_subnets)
        };

        // Calculate estimated ARP duration for progress reporting
        let arp_target_count = interfaced_ips.len() as u64;
        let total_rounds = 1 + arp_retries as u64;
        let send_time_per_round_secs = arp_target_count / arp_rate_pps.max(1) as u64;
        let estimated_arp_duration = Duration::from_secs(
            total_rounds * (send_time_per_round_secs + arp::ROUND_WAIT.as_secs())
                + arp::POST_SCAN_RECEIVE.as_secs(),
        );
        let pipeline_start = Instant::now();

        tracing::info!(
            total_ips = total_ips,
            interfaced_ips = interfaced_ips.len(),
            non_interfaced_ips = non_interfaced_ips.len(),
            estimated_arp_secs = estimated_arp_duration.as_secs(),
            arp_method = if cfg!(target_family = "windows") && !use_npcap {
                "SendARP"
            } else {
                "Broadcast"
            },
            "Starting continuous discovery pipeline"
        );

        self.report_discovery_update(DiscoverySessionUpdate::scanning(0))
            .await?;

        // Count unique subnets that will have ARP channels open
        let arp_subnet_count: usize = {
            let unique_cidrs: std::collections::HashSet<_> = interfaced_ips
                .iter()
                .map(|(_, subnet)| subnet.base.cidr)
                .collect();
            unique_cidrs.len()
        };

        // Use the port batch size from the coordinated calculation
        let effective_batch_size = port_scan_batch_size;

        // Calculate deep scan concurrency based on FDs available after ARP
        let mut deep_scan_concurrency = self
            .as_ref()
            .utils
            .get_optimal_deep_scan_concurrency(effective_batch_size, arp_subnet_count)?;

        // Create shared concurrency controller for graceful degradation
        let scan_controller = ScanConcurrencyController::new(effective_batch_size);

        let gateway_ips = self
            .as_ref()
            .utils
            .get_own_routing_table_gateway_ips()
            .await?;

        // Create async channel for discovered hosts
        // Buffer size allows ARP to run ahead while deep scanning catches up
        let (host_tx, mut host_rx) =
            tokio_mpsc::channel::<(IpAddr, Subnet, Option<MacAddress>)>(256);

        // Start ARP scanning for interfaced subnets
        if !interfaced_ips.is_empty() {
            // Group IPs by subnet for batch scanning
            let mut subnet_to_ips: HashMap<IpCidr, (Subnet, Vec<std::net::Ipv4Addr>)> =
                HashMap::new();
            for (ip, subnet) in &interfaced_ips {
                if let IpAddr::V4(ipv4) = ip {
                    subnet_to_ips
                        .entry(subnet.base.cidr)
                        .or_insert_with(|| (subnet.clone(), Vec::new()))
                        .1
                        .push(*ipv4);
                }
            }

            tracing::info!(
                subnets = subnet_to_ips.len(),
                total_ips = interfaced_ips.len(),
                arp_retries,
                arp_rate_pps,
                cidrs = ?subnet_to_ips.keys().map(|c| c.to_string()).collect::<Vec<_>>(),
                "Starting ARP discovery"
            );

            // Start ARP scan for each subnet and forward results to async channel
            for (cidr, (subnet, target_ips)) in subnet_to_ips {
                if cancel.is_cancelled() {
                    return Err(Error::msg("Discovery session was cancelled"));
                }

                let subnet_mac = subnet_cidr_to_mac.get(&cidr).and_then(|m| *m);

                let Some(source_mac) = subnet_mac else {
                    tracing::warn!(cidr = %cidr, "No MAC address found for subnet, skipping ARP scan");
                    continue;
                };

                // Find the network interface for this subnet
                // Match by both MAC and having an IP in the target subnet to handle
                // bridge setups where physical and bridge interfaces share the same MAC
                let pnet_source_mac = pnet::util::MacAddr::from(source_mac.bytes());
                let interface = datalink::interfaces().into_iter().find(|iface| {
                    iface.mac.unwrap_or_default() == pnet_source_mac
                        && iface.ips.iter().any(|ip| cidr.contains(&ip.ip()))
                });

                let Some(interface) = interface else {
                    tracing::warn!(mac = %source_mac, "No interface found for MAC, skipping ARP scan");
                    continue;
                };

                // Get an IPv4 address from this interface (prefer one on the target subnet)
                let source_ipv4 = interface
                    .ips
                    .iter()
                    .filter_map(|ip_net| match ip_net.ip() {
                        IpAddr::V4(ip) => Some(ip),
                        IpAddr::V6(_) => None,
                    })
                    .find(|ip| cidr.contains(&IpAddr::V4(*ip)))
                    .or_else(|| {
                        interface.ips.iter().find_map(|ip_net| match ip_net.ip() {
                            IpAddr::V4(ip) => Some(ip),
                            IpAddr::V6(_) => None,
                        })
                    });

                let Some(source_ipv4) = source_ipv4 else {
                    tracing::warn!(
                        interface = %interface.name,
                        cidr = %cidr,
                        "No IPv4 address found on interface, skipping ARP scan"
                    );
                    continue;
                };

                let target_count = target_ips.len();
                tracing::debug!(
                    cidr = %cidr,
                    interface = %interface.name,
                    source_ip = %source_ipv4,
                    source_mac = %source_mac,
                    targets = target_count,
                    arp_rate_pps,
                    "Starting ARP scan"
                );

                match arp::scan_subnet(
                    &interface,
                    source_ipv4,
                    source_mac,
                    target_ips,
                    use_npcap,
                    arp_retries,
                    arp_rate_pps,
                ) {
                    Ok(arp_rx) => {
                        // Spawn a task to forward ARP results to the async channel
                        // Use spawn_blocking since std::sync::mpsc::recv_timeout is blocking
                        let host_tx = host_tx.clone();
                        let subnet = subnet.clone();

                        // Use a background thread for the blocking recv, forward via channel.
                        // Hard timeout prevents infinite hangs if the ARP receiver thread
                        // gets stuck (e.g., on bridge interfaces with continuous traffic).
                        std::thread::spawn(move || {
                            let forwarder_start = Instant::now();
                            let forwarder_timeout = Duration::from_secs(600); // 10 minutes
                            let mut forwarded = 0u64;
                            loop {
                                if forwarder_start.elapsed() >= forwarder_timeout {
                                    tracing::warn!(
                                        cidr = %cidr,
                                        forwarded,
                                        elapsed_secs = forwarder_start.elapsed().as_secs(),
                                        "ARP forwarder hit timeout, forcing exit"
                                    );
                                    break;
                                }

                                match arp_rx.recv_timeout(Duration::from_millis(100)) {
                                    Ok(ArpScanResult { ip, mac }) => {
                                        // Use blocking_send since we're in a std thread
                                        if host_tx
                                            .blocking_send((
                                                IpAddr::V4(ip),
                                                subnet.clone(),
                                                Some(mac),
                                            ))
                                            .is_err()
                                        {
                                            // Receiver dropped, stop forwarding
                                            break;
                                        }
                                        forwarded += 1;
                                    }
                                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                                }
                            }
                            tracing::debug!(
                                cidr = %cidr,
                                forwarded,
                                "ARP forwarder completed"
                            );
                        });
                    }
                    Err(e) => {
                        if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                            tracing::error!(cidr = %cidr, error = %e, "Critical error starting ARP scan");
                        } else {
                            tracing::warn!(cidr = %cidr, error = %e, "ARP scan failed to start");
                        }
                    }
                }
            }
        }

        // Send all non-interfaced IPs directly to deep scanner (no discovery phase).
        // Key insight: ARP filters to responsive hosts before expensive port scanning.
        // For non-interfaced subnets where ARP isn't possible, just deep scan all IPs
        // directly - we're going to port scan them anyway.
        if !non_interfaced_ips.is_empty() {
            tracing::info!(
                count = non_interfaced_ips.len(),
                "Queuing non-interfaced IPs for deep scan (no ARP available)"
            );

            let host_tx = host_tx.clone();
            tokio::spawn(async move {
                for (ip, subnet) in non_interfaced_ips {
                    if host_tx.send((ip, subnet, None)).await.is_err() {
                        break; // Receiver dropped
                    }
                }
            });
        }

        // Drop our copy of the sender so the channel closes when all forwarders are done
        drop(host_tx);

        // =============================================================
        // CONTINUOUS PIPELINE: Deep scan hosts as they arrive
        // =============================================================
        tracing::info!(
            deep_scan_concurrency,
            grace_period_secs = LATE_ARRIVAL_GRACE_PERIOD.as_secs(),
            "Deep scanning hosts as they are discovered"
        );

        let hosts_discovered = Arc::new(AtomicUsize::new(0));
        let hosts_scanned = Arc::new(AtomicUsize::new(0));
        let last_activity = Arc::new(std::sync::Mutex::new(Instant::now()));
        let mut results: Vec<(IpAddr, Host, DiscoveredHostData)> = Vec::new();

        // Batch-level progress tracking for smoother UX
        // TCP port scanning is the bulk of deep scan work
        let is_full_scan = self.domain.scan_settings.is_full_scan;
        let scan_port_count = if is_full_scan {
            65535_usize
        } else {
            self.domain.light_scan_ports.len()
        };
        let batches_per_host = scan_port_count.div_ceil(effective_batch_size);
        let total_batches = Arc::new(AtomicUsize::new(0));
        let batches_completed = Arc::new(AtomicUsize::new(0));

        // Collect hosts into a stream and process with concurrency limit
        // Use trait objects to allow spawning from different code paths
        #[allow(clippy::type_complexity)]
        let mut pending_scans: futures::stream::FuturesUnordered<
            std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Option<(IpAddr, Host, DiscoveredHostData)>>
                        + Send,
                >,
            >,
        > = futures::stream::FuturesUnordered::new();
        let mut channel_closed = false;
        let mut last_progress_report = 0u8;
        let mut last_progress_time = Instant::now();
        let mut deep_scan_started_at: Option<Instant> = None;

        // Buffer for hosts waiting to be scanned when at concurrency limit
        let mut pending_hosts: Vec<(IpAddr, Subnet, Option<MacAddress>)> = Vec::new();

        // Use interval instead of sleep - interval persists across select iterations
        // whereas sleep creates a new future each time and gets dropped when other branches fire
        let mut progress_ticker = tokio::time::interval(Duration::from_secs(1));

        // Helper to calculate phase-weighted progress
        // Note: counters passed by value to avoid borrowing issues in closure
        let calculate_progress = |channel_closed: bool,
                                  has_pending_scans: bool,
                                  grace_elapsed: Duration,
                                  total_batches_val: usize,
                                  batches_completed_val: usize,
                                  hosts_discovered_val: usize,
                                  hosts_scanned_val: usize|
         -> u8 {
            if !channel_closed {
                // ARP phase (0-30%): Based on elapsed time vs estimated duration
                let arp_elapsed = pipeline_start.elapsed();
                let arp_progress = if estimated_arp_duration.as_secs() > 0 {
                    (arp_elapsed.as_secs_f64() / estimated_arp_duration.as_secs_f64()).min(1.0)
                } else {
                    1.0
                };
                (arp_progress * PROGRESS_ARP_PHASE as f64) as u8
            } else if total_batches_val > 0
                && (batches_completed_val < total_batches_val || has_pending_scans)
            {
                // Deep scan phase (30-95%): Based on batch completion ratio for smooth progress
                let scan_progress = batches_completed_val as f64 / total_batches_val as f64;
                PROGRESS_ARP_PHASE + (scan_progress * PROGRESS_DEEP_SCAN_PHASE as f64) as u8
            } else if has_pending_scans && total_batches_val == 0 && hosts_discovered_val > 0 {
                // Channel closed but no batch info yet - use host-level progress
                // to avoid getting stuck at 30% when batches haven't been registered
                let host_progress =
                    (hosts_scanned_val as f64 / hosts_discovered_val as f64).min(1.0);
                PROGRESS_ARP_PHASE + (host_progress * PROGRESS_DEEP_SCAN_PHASE as f64) as u8
            } else if has_pending_scans {
                // Deep scan with no batch info yet - show minimal progress
                PROGRESS_ARP_PHASE
            } else {
                // Grace period phase (95-100%): Based on grace period elapsed
                let grace_progress = (grace_elapsed.as_secs_f64()
                    / LATE_ARRIVAL_GRACE_PERIOD.as_secs_f64())
                .min(1.0);
                PROGRESS_ARP_PHASE
                    + PROGRESS_DEEP_SCAN_PHASE
                    + (grace_progress * PROGRESS_GRACE_PHASE as f64) as u8
            }
        };

        let mut early_reported_hosts: HashMap<
            IpAddr,
            tokio::task::JoinHandle<Result<Uuid, Error>>,
        > = HashMap::new();

        loop {
            tokio::select! {
                // Try to receive new hosts from the channel
                host = host_rx.recv(), if !channel_closed => {
                    match host {
                        Some((ip, subnet, mac)) => {
                            // Only count ARP-confirmed hosts immediately.
                            // Non-interfaced hosts are counted after responsiveness
                            // check passes in deep_scan_host().
                            if mac.is_some() {
                                hosts_discovered.fetch_add(1, Ordering::Relaxed);
                            }
                            *last_activity.lock().unwrap() = Instant::now();

                            // Early-report a minimal host so the UI shows it immediately.
                            // Only for interfaced hosts (mac.is_some()) — they're confirmed
                            // responsive via ARP. Non-interfaced hosts must pass the TCP
                            // responsiveness check in deep_scan_host() first.
                            if mac.is_some()
                                && let std::collections::hash_map::Entry::Vacant(e) = early_reported_hosts.entry(ip)
                            {
                                let early_subnet = subnet.clone();
                                let early_cancel = cancel.clone();
                                let service = self.service.clone();
                                let early_handle = tokio::spawn(async move {
                                    let host = Host::new(HostBase {
                                        name: ip.to_string(),
                                        network_id: early_subnet.base.network_id,
                                        source: EntitySource::Discovery { metadata: vec![] },
                                        ..Default::default()
                                    });
                                    let host_id = host.id;
                                    let interface = Interface::new(InterfaceBase {
                                        network_id: early_subnet.base.network_id,
                                        host_id: Uuid::nil(),
                                        name: None,
                                        subnet_id: early_subnet.id,
                                        ip_address: ip,
                                        mac_address: mac,
                                        position: 0,
                                    });
                                    let request = DiscoveryHostRequest {
                                        host,
                                        interfaces: vec![interface],
                                        ports: vec![],
                                        services: vec![],
                                        if_entries: vec![],
                                    };
                                    service.entity_buffer.push_host(request.clone()).await;
                                    let mode = service.config_store.get_mode().await?;
                                    match mode {
                                        DaemonMode::DaemonPoll => {
                                            let _response: HostResponse = service
                                                .api_client
                                                .post("/api/v1/hosts/discovery", &request, "Failed to create early host")
                                                .await?;
                                            Ok(host_id)
                                        }
                                        DaemonMode::ServerPoll => {
                                            let _actual = service
                                                .entity_buffer
                                                .await_host(&host_id, Duration::from_secs(120), &early_cancel)
                                                .await
                                                .ok_or_else(|| anyhow::anyhow!("Timeout waiting for early host creation"))?;
                                            Ok(host_id)
                                        }
                                    }
                                });
                                e.insert(early_handle);
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }

                            // Spawn deep scan if under concurrency limit, otherwise buffer
                            if pending_scans.len() < deep_scan_concurrency {
                                let cancel = cancel.clone();
                                let gateway_ips = gateway_ips.clone();
                                let hosts_scanned = hosts_scanned.clone();
                                let last_activity = last_activity.clone();
                                let batches_completed = batches_completed.clone();
                                let total_batches = total_batches.clone();
                                let hosts_discovered = hosts_discovered.clone();
                                let scan_controller = scan_controller.clone();

                                // Only count batches for hosts with MAC (known responsive from ARP).
                                // Non-interfaced hosts will have batches counted AFTER responsiveness check.
                                if mac.is_some() {
                                    let docker_weight = if self.domain.docker_credentials.contains_key(&ip) { DOCKER_BATCH_WEIGHT } else { 0 };
                                    total_batches.fetch_add(batches_per_host + docker_weight, Ordering::Relaxed);
                                }
                                let snmp_credentials = self.domain.snmp_credentials.get_credentials_by_specificity(&ip);
                                tracing::debug!(ip = %ip, credential_count = snmp_credentials.len(), "SNMP credentials resolved for host");
                                let docker_credential = self.domain.docker_credentials.get(&ip).cloned();
                                let probe_raw_socket_ports = self.domain.scan_settings.probe_raw_socket_ports;
                                let light_scan_ports = self.domain.light_scan_ports.clone();
                                let early_host_handle = early_reported_hosts.remove(&ip);
                                pending_scans.push(Box::pin(async move {
                                    let early_host_id = match early_host_handle {
                                        Some(handle) => match handle.await {
                                            Ok(Ok(id)) => id,
                                            _ => Uuid::new_v4(),
                                        },
                                        None => Uuid::new_v4(),
                                    };

                                    let result = self
                                        .deep_scan_host(DeepScanParams {
                                            ip,
                                            subnet: &subnet,
                                            mac,
                                            cancel,
                                            scan_rate_pps,
                                            port_scan_batch_size: effective_batch_size,
                                            gateway_ips: &gateway_ips,
                                            batches_completed: Some(&batches_completed),
                                            total_batches: Some(&total_batches),
                                            hosts_discovered: Some(&hosts_discovered),
                                            batches_per_host,
                                            snmp_credentials,
                                            scan_controller,
                                            probe_raw_socket_ports,
                                            early_host_id,
                                            docker_credential,
                                            is_full_scan,
                                            light_scan_ports: &light_scan_ports,
                                        })
                                        .await;

                                    hosts_scanned.fetch_add(1, Ordering::Relaxed);
                                    *last_activity.lock().unwrap() = Instant::now();

                                    match result {
                                        Ok(Some(host)) => Some(host),
                                        Ok(None) => None,
                                        Err(e) => {
                                            if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                                                tracing::error!(ip = %ip, error = %e, "Critical error in deep scan");
                                            } else {
                                                tracing::warn!(ip = %ip, error = %e, "Deep scan failed");
                                            }
                                            None
                                        }
                                    }
                                }));
                            } else {
                                // Only count batches for hosts with MAC (known responsive from ARP).
                                // Non-interfaced hosts will have batches counted AFTER responsiveness check.
                                if mac.is_some() {
                                    let docker_weight = if self.domain.docker_credentials.contains_key(&ip) { DOCKER_BATCH_WEIGHT } else { 0 };
                                    total_batches.fetch_add(batches_per_host + docker_weight, Ordering::Relaxed);
                                }
                                pending_hosts.push((ip, subnet, mac));
                            }
                        }
                        None => {
                            channel_closed = true;

                            tracing::info!(
                                hosts_discovered = hosts_discovered.load(Ordering::Relaxed),
                                pending_scans = pending_scans.len(),
                                pending_hosts = pending_hosts.len(),
                                total_batches = total_batches.load(Ordering::Relaxed),
                                batches_completed = batches_completed.load(Ordering::Relaxed),
                                elapsed_secs = pipeline_start.elapsed().as_secs(),
                                "Host discovery channel closed, transitioning to deep scan phase"
                            );

                            // ARP complete - recalculate concurrency without ARP FD reservation
                            // Those FDs (2 per subnet) are now available for deep scanning
                            if let Ok(new_concurrency) = self.as_ref().utils.get_optimal_deep_scan_concurrency(
                                effective_batch_size,
                                0, // No more ARP channels open
                            ) && new_concurrency > deep_scan_concurrency {
                                tracing::info!(
                                    old = deep_scan_concurrency,
                                    new = new_concurrency,
                                    "Increasing deep scan concurrency"
                                );
                                deep_scan_concurrency = new_concurrency;
                            }
                        }
                    }
                }

                // Collect completed deep scans and spawn buffered hosts
                Some(result) = pending_scans.next(), if !pending_scans.is_empty() => {
                    if let Some(host) = result {
                        results.push(host);
                    }

                    // Spawn next buffered host if available
                    // Note: batches only counted for MAC hosts when buffered; non-MAC hosts
                    // have batches counted in deep_scan_host after responsiveness check
                    if let Some((ip, subnet, mac)) = pending_hosts.pop() {
                        let cancel = cancel.clone();
                        let gateway_ips = gateway_ips.clone();
                        let hosts_scanned = hosts_scanned.clone();
                        let last_activity = last_activity.clone();
                        let batches_completed = batches_completed.clone();
                        let total_batches = total_batches.clone();
                        let hosts_discovered = hosts_discovered.clone();
                        let snmp_credentials = self.domain.snmp_credentials.get_credentials_by_specificity(&ip);
                        tracing::debug!(ip = %ip, credential_count = snmp_credentials.len(), "SNMP credentials resolved for buffered host");
                        let docker_credential = self.domain.docker_credentials.get(&ip).cloned();
                        let scan_controller = scan_controller.clone();
                        let probe_raw_socket_ports = self.domain.scan_settings.probe_raw_socket_ports;
                        let light_scan_ports = self.domain.light_scan_ports.clone();
                        let early_host_handle = early_reported_hosts.remove(&ip);

                        pending_scans.push(Box::pin(async move {
                            let early_host_id = match early_host_handle {
                                Some(handle) => match handle.await {
                                    Ok(Ok(id)) => id,
                                    _ => Uuid::new_v4(),
                                },
                                None => Uuid::new_v4(),
                            };

                            let result = self
                                .deep_scan_host(DeepScanParams {
                                    ip,
                                    subnet: &subnet,
                                    mac,
                                    cancel,
                                    scan_rate_pps,
                                    port_scan_batch_size: effective_batch_size,
                                    gateway_ips: &gateway_ips,
                                    batches_completed: Some(&batches_completed),
                                    total_batches: Some(&total_batches),
                                    hosts_discovered: Some(&hosts_discovered),
                                    batches_per_host,
                                    snmp_credentials,
                                    scan_controller,
                                    probe_raw_socket_ports,
                                    early_host_id,
                                    docker_credential,
                                    is_full_scan,
                                    light_scan_ports: &light_scan_ports,
                                })
                                .await;

                            hosts_scanned.fetch_add(1, Ordering::Relaxed);
                            *last_activity.lock().unwrap() = Instant::now();

                            match result {
                                Ok(Some(host)) => Some(host),
                                Ok(None) => None,
                                Err(e) => {
                                    if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                                        tracing::error!(ip = %ip, error = %e, "Critical error in deep scan");
                                    } else {
                                        tracing::warn!(ip = %ip, error = %e, "Deep scan failed");
                                    }
                                    None
                                }
                            }
                        }));
                    }
                }

                // Periodic progress update and grace period check
                _ = progress_ticker.tick() => {
                    let has_pending = !pending_scans.is_empty() || !pending_hosts.is_empty();
                    let grace_elapsed = last_activity.lock().unwrap().elapsed();
                    let total_batches_val = total_batches.load(Ordering::Relaxed);
                    let batches_completed_val = batches_completed.load(Ordering::Relaxed);
                    let hosts_discovered_val = hosts_discovered.load(Ordering::Relaxed);
                    let hosts_scanned_val = hosts_scanned.load(Ordering::Relaxed);

                    // Calculate and report progress (only if changed)
                    let progress = calculate_progress(
                        channel_closed,
                        has_pending,
                        grace_elapsed,
                        total_batches_val,
                        batches_completed_val,
                        hosts_discovered_val,
                        hosts_scanned_val,
                    );

                    // Update estimation atomics on the session
                    if let Ok(session) = self.as_ref().get_session().await {
                        session.hosts_discovered.store(hosts_discovered_val as u32, Ordering::Relaxed);

                        if channel_closed && hosts_scanned_val > 0 {
                            // Host-based estimation: uses actual per-host completion time
                            // which includes TCP + endpoints + SNMP + host creation — the
                            // real bottleneck, not just TCP port scanning batches.
                            let started = deep_scan_started_at.get_or_insert(Instant::now());
                            let deep_scan_elapsed = started.elapsed();
                            let time_per_host = deep_scan_elapsed.as_secs_f64() / hosts_scanned_val as f64;
                            let remaining_hosts = hosts_discovered_val.saturating_sub(hosts_scanned_val);
                            let remaining_secs = (remaining_hosts as f64 * time_per_host) as u32
                                + LATE_ARRIVAL_GRACE_PERIOD.as_secs() as u32;
                            session.estimated_remaining_secs.store(remaining_secs, Ordering::Relaxed);
                        } else if batches_completed_val > 0 {
                            // ARP phase still active — fall back to batch-based estimation
                            let started = deep_scan_started_at.get_or_insert(Instant::now());
                            let deep_scan_elapsed = started.elapsed();
                            let time_per_batch = deep_scan_elapsed.as_secs_f64() / batches_completed_val as f64;
                            let remaining_batches = total_batches_val.saturating_sub(batches_completed_val);
                            let remaining_secs = (remaining_batches as f64 * time_per_batch * 1.2) as u32
                                + LATE_ARRIVAL_GRACE_PERIOD.as_secs() as u32;
                            session.estimated_remaining_secs.store(remaining_secs, Ordering::Relaxed);
                        }
                    }

                    // Report progress if it changed OR if enough time has passed (heartbeat)
                    let time_since_last_report = last_progress_time.elapsed();
                    if progress != last_progress_report || time_since_last_report >= MAX_PROGRESS_REPORT_INTERVAL {
                        last_progress_report = progress;
                        last_progress_time = Instant::now();
                        let _ = self.report_scanning_progress(progress.min(99)).await;
                    }

                    // Check grace period expiry
                    if channel_closed && !has_pending && grace_elapsed >= LATE_ARRIVAL_GRACE_PERIOD {
                            tracing::debug!(
                                elapsed_secs = grace_elapsed.as_secs(),
                                "Grace period expired, ending discovery"
                            );
                            break;
                    }
                }
            }

            // Check for cancellation
            if cancel.is_cancelled() {
                tracing::info!("Discovery cancelled by user");
                return Err(Error::msg("Discovery session was cancelled"));
            }

            // Global timeout safety net
            if pipeline_start.elapsed() >= MAX_DISCOVERY_DURATION {
                tracing::error!(
                    elapsed_secs = pipeline_start.elapsed().as_secs(),
                    hosts_discovered = hosts_discovered.load(Ordering::Relaxed),
                    hosts_scanned = hosts_scanned.load(Ordering::Relaxed),
                    pending_scans = pending_scans.len(),
                    pending_hosts = pending_hosts.len(),
                    channel_closed,
                    "Discovery hit global timeout, forcing completion"
                );
                break;
            }

            // Exit when channel closed, no pending scans/hosts, and grace period expired
            if channel_closed && pending_scans.is_empty() && pending_hosts.is_empty() {
                let elapsed = last_activity.lock().unwrap().elapsed();

                if elapsed >= LATE_ARRIVAL_GRACE_PERIOD {
                    break;
                }

                // Log status while waiting
                let discovered = hosts_discovered.load(Ordering::Relaxed);
                if discovered > 0 {
                    tracing::debug!(
                        discovered,
                        scanned = hosts_scanned.load(Ordering::Relaxed),
                        results = results.len(),
                        grace_remaining_secs = (LATE_ARRIVAL_GRACE_PERIOD - elapsed).as_secs(),
                        "Waiting for late arrivals"
                    );
                }
            }
        }

        self.report_discovery_update(DiscoverySessionUpdate::scanning(100))
            .await?;

        let discovered = hosts_discovered.load(Ordering::Relaxed);
        tracing::info!(
            hosts_discovered = discovered,
            hosts_scanned = hosts_scanned.load(Ordering::Relaxed),
            results = results.len(),
            elapsed_secs = pipeline_start.elapsed().as_secs(),
            "Discovery pipeline complete"
        );

        Ok(results)
    }

    async fn deep_scan_host(
        &self,
        params: DeepScanParams<'_>,
    ) -> Result<Option<(IpAddr, Host, DiscoveredHostData)>, Error> {
        let DeepScanParams {
            ip,
            subnet,
            mac,
            cancel,
            scan_rate_pps,
            port_scan_batch_size,
            gateway_ips,
            batches_completed,
            total_batches,
            hosts_discovered,
            batches_per_host,
            snmp_credentials,
            scan_controller,
            probe_raw_socket_ports,
            early_host_id,
            docker_credential,
            is_full_scan,
            light_scan_ports,
        } = params;

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        // Use fixed batch size, limited by scan controller if FD exhaustion has occurred
        let effective_batch_size = port_scan_batch_size.min(scan_controller.batch_size());

        // For non-interfaced hosts (no MAC from ARP), check responsiveness first.
        // This avoids full 65k port scans on hosts that aren't online.
        let mut responsiveness_ports: HashSet<u16> = HashSet::new();
        if mac.is_none() {
            let discovery_ports: Vec<u16> = Service::all_discovery_ports()
                .iter()
                .filter(|p| p.is_tcp())
                .map(|p| p.number())
                .collect();

            tracing::debug!(
                ip = %ip,
                ports = discovery_ports.len(),
                "Checking responsiveness (non-interfaced host)"
            );

            let responsive_ports = scan_tcp_ports(
                ip,
                cancel.clone(),
                effective_batch_size,
                scan_rate_pps,
                discovery_ports,
                scan_controller.clone(),
            )
            .await?;

            if responsive_ports.is_empty() {
                tracing::debug!(ip = %ip, "Host unresponsive, skipping deep scan");
                return Ok(None);
            }

            // Host is responsive - NOW we count it in hosts_discovered and total_batches
            // This ensures only responsive hosts contribute to progress calculation
            if let Some(discovered) = hosts_discovered {
                discovered.fetch_add(1, Ordering::Relaxed);
            }
            if let Some(total) = total_batches {
                let docker_weight = if docker_credential.is_some() {
                    DOCKER_BATCH_WEIGHT
                } else {
                    0
                };
                total.fetch_add(batches_per_host + docker_weight, Ordering::Relaxed);
            }

            tracing::debug!(
                ip = %ip,
                open_ports = responsive_ports.len(),
                "Host responsive, proceeding with deep scan"
            );

            // Track discovered ports so we don't re-scan them
            responsiveness_ports.extend(responsive_ports.iter().map(|(p, _)| p.number()));
        }

        let remaining_tcp_ports: Vec<u16> = if is_full_scan {
            (1..=65535)
                .filter(|p| !responsiveness_ports.contains(p))
                .collect()
        } else {
            // Light scan: only discovery ports + credential custom ports
            light_scan_ports
                .iter()
                .copied()
                .filter(|p| !responsiveness_ports.contains(p))
                .collect()
        };

        tracing::debug!(
            ip = %ip,
            is_full_scan,
            responsiveness_ports = responsiveness_ports.len(),
            remaining_ports = remaining_tcp_ports.len(),
            snmp_enabled = !snmp_credentials.is_empty(),
            effective_batch_size,
            "Starting deep scan"
        );

        // Scan in batches with rate limiting and graceful degradation
        let mut all_tcp_ports = Vec::new();
        for chunk in remaining_tcp_ports.chunks(effective_batch_size) {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            let open_ports = scan_tcp_ports(
                ip,
                cancel.clone(),
                effective_batch_size,
                scan_rate_pps,
                chunk.to_vec(),
                scan_controller.clone(),
            )
            .await?;
            all_tcp_ports.extend(open_ports);

            // Update batch-level progress
            if let Some(counter) = batches_completed {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        let use_https_ports: HashMap<u16, bool> = all_tcp_ports
            .iter()
            .map(|(p, h)| (p.number(), *h))
            .collect();
        let mut open_ports: Vec<PortType> = all_tcp_ports.iter().map(|(p, _)| *p).collect();

        // Merge responsiveness check discovered ports (for non-interfaced hosts)
        for port_num in responsiveness_ports {
            let port = PortType::new_tcp(port_num);
            if !open_ports.contains(&port) {
                open_ports.push(port);
            }
        }
        open_ports.sort_by_key(|p| (p.number(), p.protocol()));
        open_ports.dedup();

        // UDP and endpoint scanning with rate limiting
        let plain_snmp_creds: Vec<SnmpQueryCredential> = snmp_credentials
            .iter()
            .map(|r| r.credential.clone())
            .collect();
        let udp_ports = scan_udp_ports(
            ip,
            cancel.clone(),
            effective_batch_size,
            scan_rate_pps,
            subnet.base.cidr,
            gateway_ips.to_vec(),
            &plain_snmp_creds,
        )
        .await?;
        open_ports.extend(udp_ports);

        let mut ports_to_check = open_ports.clone();
        let endpoint_only_ports = Service::endpoint_only_ports();
        ports_to_check.extend(endpoint_only_ports);
        ports_to_check.sort_by_key(|p| (p.number(), p.protocol()));
        ports_to_check.dedup();

        let accept_invalid_certs = self
            .as_ref()
            .config_store
            .get_accept_invalid_scan_certs()
            .await?;

        let endpoint_responses = scan_endpoints(
            ip,
            cancel.clone(),
            Some(ports_to_check),
            Some(use_https_ports),
            effective_batch_size,
            probe_raw_socket_ports,
            accept_invalid_certs,
        )
        .await?;

        for endpoint_response in &endpoint_responses {
            let port = endpoint_response.endpoint.port_type;
            if !open_ports.contains(&port) {
                open_ports.push(port);
            }
        }

        open_ports.sort_by_key(|p| (p.number(), p.protocol()));
        open_ports.dedup();

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        // SNMP polling - gather system info, interface table, and neighbor discovery
        // Only attempt if UDP 161 or 1161 is open (saves time on hosts without SNMP)
        // Credentials are tried in specificity order: IP override → network default → "public"
        let snmp_port = if open_ports.contains(&PortType::Snmp) {
            Some(161u16)
        } else if open_ports.iter().any(|p| p.number() == 1161 && p.is_udp()) {
            Some(1161u16)
        } else {
            None
        };
        let (
            snmp_system_info,
            snmp_if_entries,
            lldp_neighbors,
            cdp_neighbors,
            ip_addr_table,
            arp_entries,
            device_inventory,
            bridge_fdb,
            lldp_local,
            working_snmp_credential_id,
        ) = if let Some(port) = snmp_port {
            // Try each credential until system_info succeeds with actual data
            // (query_system_info returns Ok with empty fields on auth failure)
            let mut working_credential: Option<(
                snmp::SystemInfo,
                &ResolvedCredential<SnmpQueryCredential>,
            )> = None;
            for resolved in &snmp_credentials {
                match snmp::query_system_info(ip, &resolved.credential, port).await {
                    Ok(system_info)
                        if system_info.sys_descr.is_some()
                            || system_info.sys_name.is_some()
                            || system_info.sys_object_id.is_some() =>
                    {
                        working_credential = Some((system_info, resolved));
                        break;
                    }
                    Ok(_) => {
                        tracing::debug!(ip = %ip, "SNMP credential returned no data, trying next");
                    }
                    Err(e) => {
                        tracing::debug!(ip = %ip, error = %e, "SNMP credential failed, trying next");
                    }
                }
            }

            if let Some((system_info, resolved)) = working_credential {
                let credential = &resolved.credential;
                let snmp_cred_id = resolved.credential_id;
                tracing::debug!(
                    ip = %ip,
                    sys_name = ?system_info.sys_name,
                    "SNMP system info retrieved"
                );

                // Walk interface table
                let if_entries = match snmp::walk_if_table(ip, credential, port).await {
                    Ok(entries) => {
                        tracing::debug!(
                            ip = %ip,
                            if_count = entries.len(),
                            "SNMP ifTable walked"
                        );
                        entries
                    }
                    Err(e) => {
                        tracing::debug!(ip = %ip, error = %e, "SNMP ifTable walk failed");
                        Vec::new()
                    }
                };

                // Query LLDP neighbors
                let lldp = match snmp::query_lldp_neighbors(ip, credential, port).await {
                    Ok(neighbors) => {
                        tracing::debug!(
                            ip = %ip,
                            count = neighbors.len(),
                            "LLDP neighbors discovered"
                        );
                        neighbors
                    }
                    Err(e) => {
                        tracing::debug!(ip = %ip, error = %e, "LLDP query failed");
                        Vec::new()
                    }
                };

                // Query CDP neighbors (Cisco devices)
                let cdp = match snmp::query_cdp_neighbors(ip, credential, port).await {
                    Ok(neighbors) => {
                        tracing::debug!(
                            ip = %ip,
                            count = neighbors.len(),
                            "CDP neighbors discovered"
                        );
                        neighbors
                    }
                    Err(e) => {
                        tracing::debug!(ip = %ip, error = %e, "CDP query failed");
                        Vec::new()
                    }
                };

                // Query ipAddrTable for IP→ifIndex+netMask mappings
                let ip_addr_table = snmp::query_ip_addr_table(ip, credential, port)
                    .await
                    .unwrap_or_default();

                // Query ARP table for remote host discovery
                let arp_entries = snmp::query_arp_table(ip, credential, port)
                    .await
                    .unwrap_or_default();
                tracing::info!(
                    ip = %ip,
                    count = arp_entries.len(),
                    "ARP table entries collected"
                );

                // Query ENTITY-MIB for hardware inventory
                let device_inventory = snmp::query_entity_physical(ip, credential, port)
                    .await
                    .unwrap_or(None);
                tracing::info!(
                    ip = %ip,
                    has_inventory = device_inventory.is_some(),
                    "ENTITY-MIB inventory queried"
                );

                // Query bridge FDB for MAC-to-port mappings
                let bridge_fdb = snmp::query_bridge_fdb(ip, credential, port)
                    .await
                    .unwrap_or_default();
                tracing::info!(
                    ip = %ip,
                    count = bridge_fdb.len(),
                    "Bridge FDB entries collected"
                );

                // Query local LLDP identity
                let lldp_local = snmp::query_lldp_local(ip, credential, port)
                    .await
                    .unwrap_or(None);
                tracing::info!(
                    ip = %ip,
                    has_lldp_local = lldp_local.is_some(),
                    "LLDP local identity queried"
                );

                (
                    Some(system_info),
                    if_entries,
                    lldp,
                    cdp,
                    ip_addr_table,
                    arp_entries,
                    device_inventory,
                    bridge_fdb,
                    lldp_local,
                    snmp_cred_id,
                )
            } else {
                tracing::debug!(ip = %ip, "All SNMP credentials failed");
                (
                    None,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Default::default(),
                    Vec::new(),
                    None,
                    Vec::new(),
                    None,
                    None,
                )
            }
        } else {
            (
                None,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Default::default(),
                Vec::new(),
                None,
                Vec::new(),
                None,
                None,
            )
        };

        tracing::info!(
            ip = %ip,
            open_ports = open_ports.len(),
            endpoints = endpoint_responses.len(),
            snmp_interfaces = snmp_if_entries.len(),
            "Deep scan complete"
        );

        // Use SNMP sysName for hostname if DNS lookup fails
        let dns_hostname = self.get_hostname_for_ip(ip).await?;
        let hostname = dns_hostname.or_else(|| {
            snmp_system_info
                .as_ref()
                .and_then(|info| info.sys_name.clone())
        });

        // Enrich MAC from SNMP ipAddrTable when ARP didn't provide one.
        // ipAddrTable maps IP→ifIndex, and ifTable has ifIndex→MAC (ifPhysAddress).
        let mac = mac.or_else(|| {
            let ip_entry = ip_addr_table.get(&ip)?;
            let entry = snmp_if_entries
                .iter()
                .find(|e| e.if_index == ip_entry.if_index)?;
            tracing::debug!(
                ip = %ip,
                if_index = ip_entry.if_index,
                mac = ?entry.if_phys_address,
                "ipAddrTable MAC enrichment"
            );
            entry.if_phys_address
        });
        if mac.is_none() && !snmp_if_entries.is_empty() {
            tracing::debug!(
                ip = %ip,
                ip_addr_table_entries = ip_addr_table.len(),
                snmp_if_entries = snmp_if_entries.len(),
                "MAC enrichment failed - Interface will have no MAC"
            );
        }

        // Docker client probing — attempt connection if Docker credential exists for this IP
        let mut client_responses = std::collections::HashMap::new();
        let mut _docker_client_handle = None; // Keep client alive for run_docker_scan
        let mut _docker_ssl_handles: Vec<tempfile::NamedTempFile> = Vec::new();
        let mut working_docker_credential_id: Option<Uuid> = None;
        if let Some(resolved_docker) = &docker_credential {
            // Always attempt Docker probe when credential exists — the credential is explicit
            // user configuration, so we don't gate on port scanning heuristics. The Docker
            // client connection has its own timeout as a safety net.
            {
                // Build proxy URL
                let proxy_path = resolved_docker
                    .credential
                    .path
                    .as_deref()
                    .unwrap_or("")
                    .trim_start_matches('/');
                let has_ssl = resolved_docker.credential.ssl_cert.is_some();
                let scheme = if has_ssl { "https" } else { "http" };
                let host_str = match ip {
                    IpAddr::V6(v6) => format!("[{}]", v6),
                    _ => ip.to_string(),
                };
                let proxy_url = if proxy_path.is_empty() {
                    format!(
                        "{}://{}:{}",
                        scheme, host_str, resolved_docker.credential.port
                    )
                } else {
                    format!(
                        "{}://{}:{}/{}",
                        scheme, host_str, resolved_docker.credential.port, proxy_path
                    )
                };

                // Resolve SSL paths
                let label = "Docker proxy connection";
                let ssl_info: Result<Option<(String, String, String)>, Error> =
                    if let (Some(cert_rv), Some(key_rv), Some(chain_rv)) = (
                        &resolved_docker.credential.ssl_cert,
                        &resolved_docker.credential.ssl_key,
                        &resolved_docker.credential.ssl_chain,
                    ) {
                        let (cert_path, cert_handle) =
                            cert_rv.resolve_to_path("ssl_cert", label)?;
                        let (key_path, key_handle) = key_rv.resolve_to_path("ssl_key", label)?;
                        let (chain_path, chain_handle) =
                            chain_rv.resolve_to_path("ssl_chain", label)?;
                        _docker_ssl_handles.extend(cert_handle);
                        _docker_ssl_handles.extend(key_handle);
                        _docker_ssl_handles.extend(chain_handle);
                        Ok(Some((
                            cert_path.to_string_lossy().into_owned(),
                            key_path.to_string_lossy().into_owned(),
                            chain_path.to_string_lossy().into_owned(),
                        )))
                    } else {
                        Ok(None)
                    };

                tracing::info!(ip = %ip, proxy_url = %proxy_url, "Attempting Docker proxy probe");

                let ssl_paths = ssl_info?;
                const DOCKER_PROBE_MAX_ATTEMPTS: u32 = 3;
                for probe_attempt in 1..=DOCKER_PROBE_MAX_ATTEMPTS {
                    match self
                        .as_ref()
                        .utils
                        .new_docker_client(Ok(Some(proxy_url.clone())), Ok(ssl_paths.clone()))
                        .await
                    {
                        Ok(client) => {
                            tracing::info!(ip = %ip, proxy_url = %proxy_url, "Docker client probe succeeded");
                            client_responses.insert(
                                crate::server::services::r#impl::patterns::ClientProbe::Docker,
                                vec![PortType::new_tcp(resolved_docker.credential.port)],
                            );
                            _docker_client_handle = Some(client);
                            working_docker_credential_id = resolved_docker.credential_id;
                            break;
                        }
                        Err(e) => {
                            if probe_attempt < DOCKER_PROBE_MAX_ATTEMPTS {
                                tracing::debug!(
                                    ip = %ip,
                                    attempt = probe_attempt,
                                    error = %e,
                                    "Docker client probe failed, retrying"
                                );
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            } else {
                                tracing::debug!(
                                    ip = %ip,
                                    error = %e,
                                    "Docker client probe failed after {} attempts",
                                    DOCKER_PROBE_MAX_ATTEMPTS
                                );
                            }
                        }
                    }
                }

                // Mark Docker work as completed in progress tracking
                if let Some(counter) = batches_completed {
                    counter.fetch_add(DOCKER_BATCH_WEIGHT, Ordering::Relaxed);
                }
            }
        }

        if !client_responses.is_empty() {
            tracing::info!(
                ip = %ip,
                client_probes = client_responses.len(),
                "Client probes completed"
            );
        }

        // Ensure credential-probed ports are in open_ports so they flow into
        // all_ports for service matching (credential probe confirms reachability
        // even if the TCP scan missed the port)
        for port_type in client_responses.values().flatten() {
            if !open_ports.contains(port_type) {
                open_ports.push(*port_type);
            }
        }

        let interface = Interface::new(InterfaceBase {
            network_id: subnet.base.network_id,
            host_id: Uuid::nil(), // Placeholder - server will set correct host_id
            name: None,
            subnet_id: subnet.id,
            ip_address: ip,
            mac_address: mac,
            position: 0,
        });

        // Filter raw socket ports from service matching when probe_raw_socket_ports is off,
        // matching the same filtering applied in scan_endpoints() for endpoint probing.
        if !probe_raw_socket_ports {
            open_ports.retain(|p| !p.is_raw_socket());
        }

        if let Ok(Some((mut host, interfaces, ports, services))) = self
            .process_host(
                ServiceMatchBaselineParams {
                    subnet,
                    interface: &interface,
                    all_ports: &open_ports,
                    endpoint_responses: &endpoint_responses,
                    virtualization: &None,
                    client_responses: &client_responses,
                },
                hostname,
                self.domain.host_naming_fallback,
            )
            .await
        {
            // Reuse the early-reported host ID so the server updates the existing record
            host.id = early_host_id;

            // Add SNMP system info to host if available
            if let Some(ref info) = snmp_system_info {
                host.base.sys_descr = info.sys_descr.clone();
                host.base.sys_object_id = info.sys_object_id.clone();
                host.base.sys_location = info.sys_location.clone();
                host.base.sys_contact = info.sys_contact.clone();
                host.base.sys_name = info.sys_name.clone();
            }

            // Set chassis_id from LLDP local identity (canonical device identifier)
            if let Some(ref local) = lldp_local {
                use crate::server::snmp::resolution::lldp::LldpChassisId;
                if let Some(chassis) =
                    LldpChassisId::from_snmp(local.chassis_id_subtype, &local.chassis_id_bytes)
                {
                    host.base.chassis_id = Some(match &chassis {
                        LldpChassisId::NetworkAddress(ip) => ip.to_string(),
                        LldpChassisId::MacAddress(s)
                        | LldpChassisId::ChassisComponent(s)
                        | LldpChassisId::InterfaceAlias(s)
                        | LldpChassisId::PortComponent(s)
                        | LldpChassisId::InterfaceName(s)
                        | LldpChassisId::LocallyAssigned(s) => s.clone(),
                    });
                }
            }

            // Add ENTITY-MIB hardware inventory
            if let Some(ref inventory) = device_inventory {
                host.base.manufacturer = inventory.manufacturer.clone();
                host.base.model = inventory.model.clone();
                host.base.serial_number = inventory.serial_number.clone();
            }

            // Populate credential_assignments from successful SNMP credential
            if let Some(cred_id) = working_snmp_credential_id {
                host.base.credential_assignments.push(CredentialAssignment {
                    credential_id: cred_id,
                    interface_ids: None,
                });
            }

            // Populate credential_assignments from successful Docker credential,
            // scoped to the interface the Docker probe succeeded on
            if let Some(cred_id) = working_docker_credential_id {
                host.base.credential_assignments.push(CredentialAssignment {
                    credential_id: cred_id,
                    interface_ids: Some(vec![interface.id]),
                });
            }

            // Convert SNMP ifTable entries to IfEntry entities with LLDP/CDP/FDB data
            let if_entries: Vec<IfEntry> = snmp_if_entries
                .iter()
                .map(|entry| {
                    self.convert_snmp_if_entry(
                        entry,
                        subnet.base.network_id,
                        &lldp_neighbors,
                        &cdp_neighbors,
                        &bridge_fdb,
                    )
                })
                .collect();

            // Discover remote subnets from ipAddrTable (Part 1)
            let mut discovered_subnets: Vec<Subnet> = Vec::new();
            let mut extra_interfaces: Vec<Interface> = Vec::new();
            let daemon_id = self.as_ref().config_store.get_id().await?;
            let discovery_type = self.discovery_type();
            for (entry_ip, entry) in &ip_addr_table {
                let mask = match entry.net_mask {
                    Some(m) => m,
                    None => continue,
                };

                // Only handle IPv4
                let (entry_ipv4, mask_ipv4) = match (entry_ip, mask) {
                    (IpAddr::V4(eip), IpAddr::V4(mip)) => (*eip, mip),
                    _ => continue,
                };

                // Skip loopback, link-local
                let octets = entry_ipv4.octets();
                if octets[0] == 127 || (octets[0] == 169 && octets[1] == 254) {
                    continue;
                }

                // Skip /32 and /0
                let mask_octets = mask_ipv4.octets();
                let mask_u32 = u32::from_be_bytes(mask_octets);
                if mask_u32 == 0xFFFFFFFF || mask_u32 == 0 {
                    continue;
                }

                // Build network from IP + mask
                let ipv4_network = match ipnetwork::Ipv4Network::with_netmask(entry_ipv4, mask_ipv4)
                {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let ip_network = ipnetwork::IpNetwork::V4(ipv4_network);

                // Skip if this is the current scanning subnet
                let new_cidr_str = format!("{}/{}", ipv4_network.network(), ipv4_network.prefix());
                if new_cidr_str == subnet.base.cidr.to_string() {
                    continue;
                }

                // Get interface name for subnet typing
                let if_name = snmp_if_entries
                    .iter()
                    .find(|e| e.if_index == entry.if_index)
                    .and_then(|e| e.if_name.clone())
                    .unwrap_or_default();

                if let Some(new_subnet) = Subnet::from_discovery(
                    if_name,
                    &ip_network,
                    daemon_id,
                    &discovery_type,
                    subnet.base.network_id,
                ) {
                    tracing::info!(
                        ip = %ip,
                        cidr = %new_subnet.base.cidr,
                        "Discovered remote subnet via ipAddrTable"
                    );

                    match self.create_subnet(&new_subnet, &cancel).await {
                        Ok(created_subnet) => {
                            // Build an interface for the host on this subnet
                            let if_mac = snmp_if_entries
                                .iter()
                                .find(|e| e.if_index == entry.if_index)
                                .and_then(|e| e.if_phys_address);

                            extra_interfaces.push(Interface::new(InterfaceBase {
                                network_id: subnet.base.network_id,
                                host_id: Uuid::nil(),
                                name: None,
                                subnet_id: created_subnet.id,
                                ip_address: *entry_ip,
                                mac_address: if_mac,
                                position: 0,
                            }));

                            discovered_subnets.push(created_subnet);
                        }
                        Err(e) => {
                            tracing::warn!(
                                ip = %ip,
                                cidr = %new_subnet.base.cidr,
                                error = %e,
                                "Failed to create discovered subnet"
                            );
                        }
                    }
                }
            }

            // Add extra interfaces for remote subnets
            let mut interfaces = interfaces;
            interfaces.extend(extra_interfaces);

            // Create loopback interface if this host has a SOFTWARE_LOOPBACK ifEntry
            let has_loopback_if_entry = snmp_if_entries
                .iter()
                .any(|e| e.if_type == Some(if_type::SOFTWARE_LOOPBACK));
            if has_loopback_if_entry {
                let loopback_subnet = Subnet::from_discovery(
                    "lo".to_string(),
                    &ipnetwork::IpNetwork::V4(
                        ipnetwork::Ipv4Network::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 8)
                            .unwrap(),
                    ),
                    daemon_id,
                    &discovery_type,
                    subnet.base.network_id,
                );
                if let Some(loopback_subnet) = loopback_subnet {
                    match self.create_subnet(&loopback_subnet, &cancel).await {
                        Ok(created_loopback) => {
                            interfaces.push(Interface::new(InterfaceBase {
                                network_id: subnet.base.network_id,
                                host_id: Uuid::nil(),
                                name: Some("lo".to_string()),
                                subnet_id: created_loopback.id,
                                ip_address: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                                mac_address: None,
                                position: 0,
                            }));
                        }
                        Err(e) => {
                            tracing::debug!(
                                error = %e,
                                "Failed to create loopback subnet for SNMP host"
                            );
                        }
                    }
                }
            }

            // Discover remote hosts from ARP table (Part 2)
            // Only create hosts for ARP entries on SNMP-discovered remote subnets
            for arp_entry in &arp_entries {
                // Skip entries on the current scanning subnet
                if subnet.base.cidr.contains(&arp_entry.ip_address) {
                    continue;
                }

                // Find matching SNMP-discovered subnet
                let matching_subnet = discovered_subnets
                    .iter()
                    .find(|s| s.base.cidr.contains(&arp_entry.ip_address));

                if let Some(remote_subnet) = matching_subnet {
                    let arp_interface = Interface::new(InterfaceBase {
                        network_id: subnet.base.network_id,
                        host_id: Uuid::nil(),
                        name: None,
                        subnet_id: remote_subnet.id,
                        ip_address: arp_entry.ip_address,
                        mac_address: Some(arp_entry.mac_address),
                        position: 0,
                    });

                    let arp_host = Host::new(HostBase {
                        network_id: subnet.base.network_id,
                        source: EntitySource::Discovery { metadata: vec![] },
                        ..Default::default()
                    });

                    tracing::info!(
                        ip = %arp_entry.ip_address,
                        mac = %arp_entry.mac_address,
                        subnet = %remote_subnet.base.cidr,
                        "Discovered remote host via ARP table"
                    );

                    if let Err(e) = self
                        .create_host(
                            arp_host,
                            vec![arp_interface],
                            vec![],
                            vec![],
                            vec![],
                            &cancel,
                        )
                        .await
                    {
                        tracing::debug!(
                            ip = %arp_entry.ip_address,
                            error = %e,
                            "Failed to create ARP-discovered host"
                        );
                    }
                }
            }

            let services_count = services.len();
            let if_entries_count = if_entries.len();

            if let Ok(host_response) = self
                .create_host(host, interfaces, ports, services, if_entries, &cancel)
                .await
            {
                tracing::info!(
                    ip = %ip,
                    services = services_count,
                    if_entries = if_entries_count,
                    "Host created"
                );
                let host_data = DiscoveredHostData {
                    docker_service_id: host_response
                        .services
                        .iter()
                        .find(|s| s.base.service_definition.id() == "Docker")
                        .map(|s| s.id),
                    interfaces: host_response.interfaces.clone(),
                };
                return Ok(Some((ip, host_response.to_host(), host_data)));
            } else {
                tracing::warn!(ip = %ip, "Host creation failed");
            }
        } else {
            tracing::debug!(ip = %ip, "Host processing returned None");
        }

        Ok(None)
    }

    /// Convert SNMP ifTable entry to IfEntry entity with LLDP/CDP/FDB neighbor data
    /// Uses Uuid::nil() for host_id as placeholder - server will set correct host_id
    fn convert_snmp_if_entry(
        &self,
        entry: &IfTableEntry,
        network_id: Uuid,
        lldp_neighbors: &[snmp::LldpNeighbor],
        cdp_neighbors: &[snmp::CdpNeighbor],
        bridge_fdb: &[snmp::BridgeFdbEntry],
    ) -> IfEntry {
        use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};

        // Find LLDP neighbor data for this port (match by local_port_index == if_index)
        let lldp_neighbor = lldp_neighbors
            .iter()
            .find(|n| n.local_port_index == entry.if_index);

        // Find CDP neighbor data for this port
        let cdp_neighbor = cdp_neighbors
            .iter()
            .find(|n| n.local_port_index == entry.if_index);

        // Convert LLDP chassis ID using subtype + raw bytes via from_snmp()
        let lldp_chassis_id = lldp_neighbor.and_then(|n| {
            let subtype = n.remote_chassis_id_subtype?;
            let bytes = n.remote_chassis_id_bytes.as_ref()?;
            LldpChassisId::from_snmp(subtype, bytes)
        });

        // Convert LLDP port ID using subtype + raw bytes via from_snmp()
        let lldp_port_id = lldp_neighbor.and_then(|n| {
            let subtype = n.remote_port_id_subtype?;
            let bytes = n.remote_port_id_bytes.as_ref()?;
            LldpPortId::from_snmp(subtype, bytes)
        });

        // Collect learned MACs from bridge FDB for this port.
        // Single-MAC ports are used for neighbor resolution server-side;
        // multi-MAC ports indicate uplinks where LLDP/CDP is the better source
        // for direct neighbor identification.
        let fdb_macs: Vec<String> = bridge_fdb
            .iter()
            .filter(|fdb| fdb.if_index == Some(entry.if_index) && fdb.status == 3)
            .map(|fdb| fdb.mac_address.to_string())
            .collect();

        IfEntry::new(IfEntryBase {
            host_id: Uuid::nil(), // Placeholder - server will set correct host_id
            network_id,
            if_index: entry.if_index,
            if_descr: entry.if_descr.clone().unwrap_or_default(),
            if_name: entry.if_name.clone(),
            if_alias: entry.if_alias.clone(),
            if_type: entry.if_type.unwrap_or(1), // 1 = "other"
            speed_bps: entry.if_speed.map(|s| s as i64),
            admin_status: IfAdminStatus::from(entry.if_admin_status.unwrap_or(1)),
            oper_status: IfOperStatus::from(entry.if_oper_status.unwrap_or(1)),
            mac_address: entry.if_phys_address, // MAC from SNMP ifPhysAddress
            interface_id: None,                 // Linked server-side via MAC matching
            neighbor: None,                     // Resolved server-side from LLDP/CDP data
            // LLDP raw data
            lldp_chassis_id,
            lldp_port_id,
            lldp_sys_name: lldp_neighbor.and_then(|n| n.remote_sys_name.clone()),
            lldp_port_desc: lldp_neighbor.and_then(|n| n.remote_port_desc.clone()),
            lldp_mgmt_addr: lldp_neighbor.and_then(|n| n.remote_mgmt_addr),
            lldp_sys_desc: lldp_neighbor.and_then(|n| n.remote_sys_desc.clone()),
            // CDP raw data
            cdp_device_id: cdp_neighbor.and_then(|n| n.remote_device_id.clone()),
            cdp_port_id: cdp_neighbor.and_then(|n| n.remote_port_id.clone()),
            cdp_platform: cdp_neighbor.and_then(|n| n.remote_platform.clone()),
            cdp_address: cdp_neighbor.and_then(|n| n.remote_address),
            // Bridge FDB data
            fdb_macs: if fdb_macs.is_empty() {
                None
            } else {
                Some(fdb_macs)
            },
        })
    }

    async fn get_hostname_for_ip(&self, ip: IpAddr) -> Result<Option<String>, Error> {
        match timeout(Duration::from_millis(2000), async {
            tokio::task::spawn_blocking(move || dns_lookup::lookup_addr(&ip)).await?
        })
        .await
        {
            Ok(Ok(hostname)) => Ok(Some(hostname)),
            _ => Ok(None),
        }
    }

    /// Figure out what order to scan IPs in given allocation patterns
    fn determine_scan_order(&self, subnet: &IpCidr) -> impl Iterator<Item = IpAddr> {
        let mut ips: Vec<IpAddr> = subnet.iter().map(|ip| ip.address()).collect();

        // Sort by likelihood of being active hosts - highest probability first
        ips.sort_by_key(|ip| {
            let last_octet = match ip {
                IpAddr::V4(ipv4) => ipv4.octets()[3],
                IpAddr::V6(_) => return 9999, // IPv6 gets lowest priority for now
            };

            match last_octet {
                // Tier 1: Almost guaranteed to be active infrastructure
                1 => 1,   // Default gateway (.1)
                254 => 2, // Alternative gateway (.254)

                // Tier 2: Very common infrastructure and static assignments
                2 => 10,   // Secondary router/switch
                3 => 11,   // Tertiary infrastructure
                10 => 12,  // Common DHCP start
                100 => 13, // Common DHCP end
                253 => 14, // Alt gateway range
                252 => 15, // Alt gateway range

                // Tier 3: Common static device ranges
                4..=9 => 20 + last_octet as u16, // Infrastructure devices
                11..=20 => 30 + last_octet as u16, // Servers, printers
                21..=30 => 50 + last_octet as u16, // Network devices

                // Tier 4: Active DHCP ranges (most devices live here)
                31..=50 => 100 + last_octet as u16, // Early DHCP range
                51..=100 => 200 + last_octet as u16, // Mid DHCP range
                101..=150 => 400 + last_octet as u16, // Late DHCP range

                // Tier 5: Less common but still viable
                151..=200 => 600 + last_octet as u16, // Extended DHCP
                201..=251 => 800 + last_octet as u16, // High static range

                // Skip entirely - reserved addresses
                0 | 255 => 9998, // Network/broadcast addresses
            }
        });

        ips.into_iter()
    }

    async fn get_subnets(&self) -> Result<Vec<Subnet>, Error> {
        self.as_ref()
            .api_client
            .get("/api/v1/subnets", "Failed to get subnets")
            .await
    }
}
