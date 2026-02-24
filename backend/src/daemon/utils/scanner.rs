use crate::daemon::discovery::types::base::DiscoveryCriticalError;
use crate::server::services::r#impl::base::Service;
use crate::server::services::r#impl::endpoints::{Endpoint, EndpointResponse};
use anyhow::anyhow;
use anyhow::{Error, Result};
use cidr::IpCidr;
use dhcproto::Encodable;
use dhcproto::v4::{self, Decodable, Encoder, Message, MessageType};
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use hickory_resolver::Resolver;
use hickory_resolver::config::{NameServerConfig, ResolverConfig};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::xfer::Protocol;
use rand::{Rng, SeedableRng};
use rsntp::AsyncSntpClient;
use snmp2::{AsyncSession, Oid};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::{net::TcpStream, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::server::ports::r#impl::base::{PortType, TransportProtocol};

pub const SCAN_TIMEOUT: Duration = Duration::from_millis(800);

/// Default port scan batch size - number of ports scanned concurrently per host
pub const PORT_SCAN_BATCH_SIZE: usize = 200;

/// Minimum batch size floor to prevent degradation to unusably slow scanning
pub const PORT_SCAN_BATCH_MIN: usize = 16;

/// Number of consecutive successes required before attempting recovery
const RECOVERY_THRESHOLD: usize = 50;

/// Minimum time between degradation events (milliseconds) to prevent cascading
const DEGRADATION_COOLDOWN_MS: u64 = 500;

/// EMFILE error code on Unix systems (Too many open files)
#[cfg(unix)]
const EMFILE: i32 = 24;

/// Controller for dynamically adjusting scan concurrency when FD exhaustion occurs.
///
/// This provides graceful degradation: when "Too many open files" errors are detected,
/// the batch size is halved (down to a minimum floor). After sustained success,
/// batch size gradually recovers.
#[derive(Debug)]
pub struct ScanConcurrencyController {
    /// Current active batch size
    current_batch_size: AtomicUsize,
    /// Original target batch size (for recovery)
    target_batch_size: usize,
    /// Whether we're currently in degraded mode
    degraded: AtomicBool,
    /// Consecutive successful operations since last degradation
    success_streak: AtomicUsize,
    /// Timestamp of last degradation (ms since controller creation) for rate limiting
    last_degradation_ms: AtomicU64,
    /// Controller creation time for computing relative timestamps
    created_at: Instant,
}

impl ScanConcurrencyController {
    /// Create a new controller with the given initial batch size
    pub fn new(initial_batch_size: usize) -> Arc<Self> {
        Arc::new(Self {
            current_batch_size: AtomicUsize::new(initial_batch_size),
            target_batch_size: initial_batch_size,
            degraded: AtomicBool::new(false),
            success_streak: AtomicUsize::new(0),
            last_degradation_ms: AtomicU64::new(0),
            created_at: Instant::now(),
        })
    }

    /// Get the current recommended batch size
    pub fn batch_size(&self) -> usize {
        self.current_batch_size.load(Ordering::Relaxed)
    }

    /// Check if currently operating in degraded mode
    pub fn is_degraded(&self) -> bool {
        self.degraded.load(Ordering::Relaxed)
    }

    /// Called when an FD exhaustion error (EMFILE) is detected.
    /// Halves the batch size (minimum PORT_SCAN_BATCH_MIN) and resets success streak.
    /// Uses compare-and-swap to ensure only one caller succeeds per degradation level.
    /// Rate-limited to prevent cascading degradation from concurrent errors.
    pub fn on_fd_exhaustion(&self) {
        let now_ms = self.created_at.elapsed().as_millis() as u64;
        let last_ms = self.last_degradation_ms.load(Ordering::Relaxed);

        // Rate limit: skip if we degraded very recently (concurrent errors from same spike)
        // Allow first degradation by checking if last_ms > 0 (meaning we've degraded before)
        if last_ms > 0 && now_ms.saturating_sub(last_ms) < DEGRADATION_COOLDOWN_MS {
            // Still mark as degraded and reset streak, but don't reduce further
            self.degraded.store(true, Ordering::Relaxed);
            self.success_streak.store(0, Ordering::Relaxed);
            tracing::debug!(
                "FD exhaustion skipped (rate limited), {} errors within cooldown period",
                DEGRADATION_COOLDOWN_MS
            );
            return;
        }

        // Use compare_exchange to atomically reduce - only the "winner" logs
        loop {
            let current = self.current_batch_size.load(Ordering::Relaxed);
            let new_size = (current / 2).max(PORT_SCAN_BATCH_MIN);

            // If already at floor, just ensure we're marked as degraded
            if current == new_size && current == PORT_SCAN_BATCH_MIN {
                self.degraded.store(true, Ordering::Relaxed);
                return;
            }

            // Try to be the one to reduce the batch size
            match self.current_batch_size.compare_exchange(
                current,
                new_size,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    // We won the race - log and update state
                    self.degraded.store(true, Ordering::Relaxed);
                    self.success_streak.store(0, Ordering::Relaxed);
                    // Use max(1, now_ms) so we never store 0 (which means "never degraded")
                    self.last_degradation_ms
                        .store(now_ms.max(1), Ordering::Relaxed);

                    tracing::warn!(
                        previous_batch_size = current,
                        new_batch_size = new_size,
                        floor = PORT_SCAN_BATCH_MIN,
                        "FD exhaustion detected, reducing batch size"
                    );
                    return;
                }
                Err(_) => {
                    // Another thread already reduced it, retry with new value
                    continue;
                }
            }
        }
    }

    /// Called after a successful batch of operations.
    /// Tracks success streak and attempts gradual recovery after threshold.
    pub fn on_success(&self) {
        if !self.degraded.load(Ordering::Relaxed) {
            return;
        }

        let streak = self.success_streak.fetch_add(1, Ordering::Relaxed) + 1;

        if streak >= RECOVERY_THRESHOLD {
            let current = self.current_batch_size.load(Ordering::Relaxed);

            // Recover by 25%, but don't exceed target
            let new_size = ((current * 125) / 100).min(self.target_batch_size);

            if new_size > current {
                self.current_batch_size.store(new_size, Ordering::Relaxed);
                self.success_streak.store(0, Ordering::Relaxed);

                // Check if we've fully recovered
                if new_size >= self.target_batch_size {
                    self.degraded.store(false, Ordering::Relaxed);
                    tracing::info!(
                        previous_batch_size = current,
                        recovered_batch_size = new_size,
                        "Batch size fully recovered from FD exhaustion"
                    );
                } else {
                    tracing::info!(
                        previous_batch_size = current,
                        new_batch_size = new_size,
                        target = self.target_batch_size,
                        "Batch size partially recovering after sustained success"
                    );
                }
            }
        }
    }

    /// Check if an error indicates FD exhaustion and handle it.
    /// Returns true if this was an FD exhaustion error that was handled.
    #[cfg(unix)]
    pub fn check_and_handle_error(&self, error: &std::io::Error) -> bool {
        if error.raw_os_error() == Some(EMFILE) || error.kind() == ErrorKind::Other {
            // Also check error message for "Too many open files"
            let msg = error.to_string().to_lowercase();
            if error.raw_os_error() == Some(EMFILE) || msg.contains("too many open files") {
                self.on_fd_exhaustion();
                return true;
            }
        }
        false
    }

    #[cfg(not(unix))]
    pub fn check_and_handle_error(&self, error: &std::io::Error) -> bool {
        // On Windows, check for equivalent error
        let msg = error.to_string().to_lowercase();
        if msg.contains("too many open files") || msg.contains("no more file handles") {
            self.on_fd_exhaustion();
            return true;
        }
        false
    }
}

/// Generic batch scanner that maintains constant parallelism with rate limiting
/// This is the core RustScan pattern extracted into a reusable function
///
/// # Arguments
/// * `items` - Items to scan
/// * `batch_size` - Number of concurrent operations to maintain
/// * `scan_rate_pps` - Maximum probes per second (0 = unlimited)
/// * `cancel` - Cancellation token
/// * `scan_fn` - Async function that scans an item and returns Option<Result>
///
/// # Returns
/// Vector of successfully scanned results
async fn batch_scan<T, O, F, Fut>(
    items: Vec<T>,
    batch_size: usize,
    scan_rate_pps: u32,
    cancel: CancellationToken,
    scan_fn: F,
) -> Vec<O>
where
    T: Send + 'static,
    O: Send + 'static,
    F: Fn(T) -> Fut,
    Fut: std::future::Future<Output = Option<O>> + Send + 'static,
{
    let mut results = Vec::new();
    let mut item_iter = items.into_iter();

    // Calculate stagger delay from rate limit
    let stagger_delay = if scan_rate_pps > 0 {
        Duration::from_micros(1_000_000 / scan_rate_pps as u64)
    } else {
        Duration::ZERO
    };

    let mut futures: FuturesUnordered<Pin<Box<dyn Future<Output = Option<O>> + Send>>> =
        FuturesUnordered::new();

    // Initial seeding with staggered starts
    for _ in 0..batch_size {
        if cancel.is_cancelled() {
            break;
        }

        if let Some(item) = item_iter.next() {
            futures.push(Box::pin(scan_fn(item)));
            // Stagger connection starts to avoid SYN burst
            if !stagger_delay.is_zero() {
                tokio::time::sleep(stagger_delay).await;
            }
        } else {
            break;
        }
    }

    while let Some(result) = futures.next().await {
        if cancel.is_cancelled() {
            break;
        }

        if let Some(output) = result {
            results.push(output);
        }

        while futures.len() < batch_size && !cancel.is_cancelled() {
            if let Some(item) = item_iter.next() {
                futures.push(Box::pin(scan_fn(item)));
                // Stagger connection starts to avoid SYN burst
                if !stagger_delay.is_zero() {
                    tokio::time::sleep(stagger_delay).await;
                }
            } else {
                break;
            }
        }
    }

    results
}

/// Check if ARP scanning is available on this platform.
///
/// # Arguments
/// * `use_npcap` - (Windows only) Check for Npcap availability instead of SendARP
pub fn can_arp_scan(use_npcap: bool) -> bool {
    let available = crate::daemon::utils::arp::is_available(use_npcap);

    if available {
        tracing::info!("ARP scanning capability confirmed. Fast host discovery enabled.");
    } else {
        tracing::warn!(
            "ARP scanning not available. Will fall back to TCP port scanning for host discovery. \
             For MACVLAN deployments, ensure: (1) container has NET_RAW and NET_ADMIN capabilities, \
             (2) network interface is properly configured with a MAC address."
        );
    }

    available
}

#[allow(clippy::too_many_arguments)]
pub async fn scan_ports_and_endpoints(
    ip: IpAddr,
    cancel: CancellationToken,
    port_scan_batch_size: usize,
    scan_rate_pps: u32,
    cidr: IpCidr,
    gateway_ips: Vec<IpAddr>,
    tcp_ports_to_check: Vec<u16>,
    accept_invalid_certs: bool,
) -> Result<(Vec<PortType>, Vec<EndpointResponse>), Error> {
    if cancel.is_cancelled() {
        return Err(anyhow!("Operation cancelled"));
    }

    let mut open_ports = Vec::new();
    let mut endpoint_responses = Vec::new();

    // Scan TCP ports with batching and rate limiting
    let controller = ScanConcurrencyController::new(port_scan_batch_size);
    let tcp_ports = scan_tcp_ports(
        ip,
        cancel.clone(),
        port_scan_batch_size,
        scan_rate_pps,
        tcp_ports_to_check,
        controller,
    )
    .await?;

    let use_https_ports: HashMap<u16, bool> =
        tcp_ports.iter().map(|(p, h)| (p.number(), *h)).collect();
    let tcp_ports: Vec<PortType> = tcp_ports.iter().map(|(p, _)| *p).collect();

    open_ports.extend(tcp_ports.clone());

    if cancel.is_cancelled() {
        return Err(anyhow!("Operation cancelled"));
    }

    // Scan UDP ports with batching and rate limiting
    let udp_ports = scan_udp_ports(
        ip,
        cancel.clone(),
        port_scan_batch_size,
        scan_rate_pps,
        cidr,
        gateway_ips,
    )
    .await?;
    open_ports.extend(udp_ports);

    if cancel.is_cancelled() {
        return Err(anyhow!("Operation cancelled"));
    }

    // Scan endpoints - check on ALL open TCP ports, not just filtered ones
    let mut ports_to_check = tcp_ports.clone();

    // Also add endpoint-only ports that we didn't scan during port scanning
    let endpoint_only_ports = Service::endpoint_only_ports();
    ports_to_check.extend(endpoint_only_ports);
    ports_to_check.sort_by_key(|p| (p.number(), p.protocol()));
    ports_to_check.dedup();

    let endpoints = scan_endpoints(
        ip,
        cancel.clone(),
        Some(ports_to_check),
        Some(use_https_ports),
        port_scan_batch_size,
        false,
        accept_invalid_certs,
    )
    .await?;
    endpoint_responses.extend(endpoints);

    // Add any ports that had endpoint responses but weren't in open_ports
    // This handles cases where we got HTTP response but port scan didn't detect it
    for endpoint_response in &endpoint_responses {
        let port = endpoint_response.endpoint.port_type;
        if !open_ports.contains(&port) {
            tracing::debug!(
                "Adding port {} to open ports based on successful endpoint response",
                port
            );
            open_ports.push(port);
        }
    }

    // Deduplicate ports (sort first for consistent deduplication)
    open_ports.sort_by_key(|p| (p.number(), p.protocol()));
    open_ports.dedup();

    tracing::info!(
        ip = %ip,
        open_ports = %open_ports.len(),
        endpoint_responses = %endpoint_responses.len(),
        "Host scan complete"
    );

    Ok((open_ports, endpoint_responses))
}

/// Scan TCP ports with graceful FD exhaustion handling.
///
/// When FD exhaustion is detected, the controller automatically reduces batch size
/// and logs a warning. The scan continues with reduced concurrency rather than failing.
pub async fn scan_tcp_ports(
    ip: IpAddr,
    cancel: CancellationToken,
    batch_size: usize,
    scan_rate_pps: u32,
    tcp_ports_to_check: Vec<u16>,
    controller: Arc<ScanConcurrencyController>,
) -> Result<Vec<(PortType, bool)>, Error> {
    let ports: Vec<PortType> = tcp_ports_to_check
        .iter()
        .map(|p| PortType::new_tcp(*p))
        .collect();

    // Use controller's batch size if in degraded mode
    let effective_batch_size = batch_size.min(controller.batch_size());
    let controller_for_log = controller.clone();

    let open_ports = batch_scan(
        ports.clone(),
        effective_batch_size,
        scan_rate_pps,
        cancel,
        move |port| {
            let controller = controller.clone();
            async move {
                let socket = SocketAddr::new(ip, port.number());

                // Try connection with timeout, retry once on timeout for slow hosts
                let mut attempts = 0;
                let max_attempts = 2;

                loop {
                    attempts += 1;
                    let start = std::time::Instant::now();

                    match timeout(SCAN_TIMEOUT, TcpStream::connect(socket)).await {
                        Ok(Ok(stream)) => {
                            controller.on_success();

                            let connect_time = start.elapsed();

                            // Try to peek at the connection to detect immediate disconnects
                            let mut buf = [0u8; 1];
                            let peek_result =
                                timeout(Duration::from_millis(50), stream.peek(&mut buf)).await;

                            let use_https = match peek_result {
                                Ok(Ok(0)) => true,   // HTTPS (immediate close)
                                Ok(Ok(_)) => false,  // Got bytes
                                Ok(Err(_)) => false, // Peek error
                                Err(_) => false,     // No immediate response
                            };

                            tracing::debug!(
                                "Found open TCP port {}:{} (took {:?})",
                                ip,
                                port,
                                connect_time
                            );

                            drop(stream);
                            return Some((
                                PortType::new_tcp(port.number()),
                                use_https || port.is_https(),
                            ));
                        }
                        Ok(Err(e)) => {
                            // Check for FD exhaustion and handle gracefully
                            if controller.check_and_handle_error(&e) {
                                // FD exhaustion - continue scanning with reduced batch
                                // Return None for this port but don't fail the scan
                                return None;
                            }

                            if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                                tracing::error!(
                                    "Critical error scanning {}:{}: {}",
                                    socket.ip(),
                                    port,
                                    e
                                );
                            }
                            return None;
                        }
                        Err(_) => {
                            let elapsed = start.elapsed();

                            if attempts < max_attempts {
                                tracing::trace!(
                                    "Port {}:{} timeout attempt {}/{} (took {:?}), retrying...",
                                    ip,
                                    port,
                                    attempts,
                                    max_attempts,
                                    elapsed
                                );
                                tokio::time::sleep(Duration::from_millis(100)).await;
                                continue;
                            } else {
                                tracing::trace!(
                                    "Port {}:{} timeout after {} attempts",
                                    ip,
                                    port,
                                    attempts
                                );
                                return None;
                            }
                        }
                    }
                }
            }
        },
    )
    .await;

    tracing::debug!(
        ip = %ip,
        ports_scanned = %ports.len(),
        responses = %open_ports.len(),
        effective_batch_size,
        degraded = controller_for_log.is_degraded(),
        "TCP ports scanned"
    );

    Ok(open_ports)
}

pub async fn scan_udp_ports(
    ip: IpAddr,
    cancel: CancellationToken,
    batch_size: usize,
    scan_rate_pps: u32,
    cidr: IpCidr,
    gateway_ips: Vec<IpAddr>,
) -> Result<Vec<PortType>, Error> {
    let discovery_ports = Service::all_discovery_ports();
    let ports: Vec<u16> = discovery_ports
        .iter()
        .filter(|p| p.protocol() == TransportProtocol::Udp)
        .map(|p| p.number())
        .collect();

    // UDP is slower and less reliable, cap at 10 concurrent
    let udp_batch_size = std::cmp::min(batch_size, 10);

    let is_gateway = gateway_ips.contains(&ip);

    // UDP rate limiting is less critical but still useful
    let open_ports = batch_scan(
        ports.clone(),
        udp_batch_size,
        scan_rate_pps,
        cancel,
        |port| async move {
            let result = match port {
                53 => test_dns_service(ip).await,
                123 => test_ntp_service(ip).await,
                161 => test_snmp_service(ip).await,
                67 => {
                    if is_gateway {
                        test_dhcp_service(ip, &cidr).await
                    } else {
                        Ok(None)
                    }
                }
                47808 => test_bacnet_service(ip).await,
                _ => Ok(None),
            };

            match result {
                Ok(Some(detected_port)) => {
                    tracing::trace!("Found open UDP port {}:{}", ip, detected_port);
                    Some(PortType::new_udp(detected_port))
                }
                Ok(None) => None,
                Err(e) => {
                    if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                        tracing::error!("Critical error scanning UDP {}:{}: {}", ip, port, e);
                    }
                    None
                }
            }
        },
    )
    .await;

    tracing::debug!(
        ip = %ip,
        ports_scanned = %ports.len(),
        responses = %open_ports.len(),
        "UDP ports scanned"
    );

    Ok(open_ports)
}

pub async fn scan_endpoints(
    ip: IpAddr,
    cancel: CancellationToken,
    filter_ports: Option<Vec<PortType>>,
    use_https_ports: Option<HashMap<u16, bool>>,
    batch_size: usize,
    probe_raw_socket_ports: bool,
    accept_invalid_certs: bool,
) -> Result<Vec<EndpointResponse>, Error> {
    use std::collections::HashMap;

    let client = reqwest::Client::builder()
        .timeout(SCAN_TIMEOUT)
        .danger_accept_invalid_certs(accept_invalid_certs)
        .build()
        .map_err(|e| anyhow!("Could not build client {}", e))?;

    let all_endpoints: Vec<Endpoint> = Service::all_discovery_endpoints()
        .into_iter()
        .filter_map(|e| {
            if !probe_raw_socket_ports && e.port_type.is_raw_socket() {
                return None;
            }
            if let Some(filter_ports) = &filter_ports {
                if filter_ports.contains(&e.port_type) {
                    return Some(e);
                }
                None
            } else {
                Some(e)
            }
        })
        .collect();

    // Group endpoints by (port, path) to avoid duplicate requests
    let mut unique_endpoints: HashMap<(u16, String), Endpoint> = HashMap::new();
    for endpoint in all_endpoints {
        let key = (endpoint.port_type.number(), endpoint.path.clone());
        unique_endpoints.entry(key).or_insert(endpoint);
    }

    let endpoints: Vec<Endpoint> = unique_endpoints.into_values().collect();
    let total_endpoints = endpoints.len();

    let endpoint_batch_size = std::cmp::min(batch_size / 2, 50);

    let use_https_ports_is_none = use_https_ports.is_none();
    let https_ports = use_https_ports.unwrap_or_default();

    // Endpoint scanning uses HTTP client with connection pooling, rate limiting less critical
    let responses = batch_scan(endpoints, endpoint_batch_size, 0, cancel, move |endpoint| {
        let client = client.clone();
        let https_ports = https_ports.clone();
        async move {
            let endpoint_with_ip = endpoint.use_ip(ip);

            // Common HTTPS ports
            let use_https = https_ports
                .get(&endpoint.port_type.number())
                .unwrap_or(&false);
            let url = format!(
                "{}:{}{}",
                ip,
                endpoint_with_ip.port_type.number(),
                endpoint_with_ip.path
            );
            let http_url = format!("http://{}", url);
            let https_url = format!("https://{}", url);

            // Decide which of HTTP or HTTPS to try first
            let urls = if use_https_ports_is_none {
                // No info = try both
                vec![http_url, https_url]
            } else if *use_https {
                vec![https_url, http_url]
            } else {
                vec![http_url, https_url]
            };

            for url in urls {
                tracing::trace!("Trying endpoint: {}", url);

                match client.get(&url).send().await {
                    Ok(response) => {
                        let status = response.status().as_u16();

                        let headers = response
                            .headers()
                            .iter()
                            .filter_map(|(name, value)| {
                                // Convert HeaderValue to string
                                value.to_str().ok().map(|v| {
                                    (
                                        name.as_str().to_lowercase(), // Normalize to lowercase
                                        v.to_string(),
                                    )
                                })
                            })
                            .collect();

                        match response.text().await {
                            Ok(body) => {
                                tracing::debug!(
                                    "Endpoint {} returned {} (length: {})",
                                    url,
                                    status,
                                    body.len()
                                );
                                return Some(EndpointResponse {
                                    endpoint: endpoint_with_ip,
                                    headers,
                                    body,
                                    status,
                                });
                            }
                            Err(e) => {
                                tracing::trace!("Failed to read response from {}: {}", url, e);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::trace!("Endpoint {} failed: {}", url, e);
                        if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                            tracing::error!("Critical error scanning endpoint {}: {}", url, e);
                        }
                        continue;
                    }
                }
            }

            None
        }
    })
    .await;

    tracing::debug!(
        ip = %ip,
        endpoints_scanned = %total_endpoints,
        responses = %responses.len(),
        "Endpoint scan complete"
    );

    Ok(responses)
}

pub async fn test_dns_service(ip: IpAddr) -> Result<Option<u16>, Error> {
    let mut config = ResolverConfig::new();
    let name_server = NameServerConfig::new(SocketAddr::new(ip, 53), Protocol::Udp);
    config.add_name_server(name_server);

    let resolver =
        Resolver::builder_with_config(config, TokioConnectionProvider::default()).build();

    match timeout(
        Duration::from_millis(2000),
        resolver.lookup_ip("google.com"),
    )
    .await
    {
        Ok(Ok(_)) => Ok(Some(53)),
        _ => Ok(None),
    }
}

pub async fn test_ntp_service(ip: IpAddr) -> Result<Option<u16>, Error> {
    let client = AsyncSntpClient::new();
    let server_addr = format!("{}:123", ip);

    match timeout(
        Duration::from_millis(2000),
        client.synchronize(&server_addr),
    )
    .await
    {
        Ok(Ok(result)) => {
            // Validate that we got a meaningful time response
            if let Ok(datetime) = result.datetime().unix_timestamp() {
                if datetime > Duration::from_secs(0) {
                    Ok(Some(123))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
        Ok(Err(_)) => Ok(None),
        Err(_) => Ok(None),
    }
}

pub async fn test_snmp_service(ip: IpAddr) -> Result<Option<u16>, Error> {
    let target = format!("{}:161", ip);
    let community = b"public";

    // Wrap session creation with timeout to prevent hanging
    let session_result = timeout(
        Duration::from_millis(2000),
        AsyncSession::new_v2c(&target, community, 0),
    )
    .await;

    match session_result {
        Ok(Ok(mut session)) => {
            let sys_descr_oid = Oid::from(&[1, 3, 6, 1, 2, 1, 1, 1, 0])
                .map_err(|e| anyhow!("Invalid Oid: {:?}", e))?;

            match timeout(Duration::from_millis(2000), session.get(&sys_descr_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some(_varbind) = response.varbinds.next() {
                        Ok(Some(161))
                    } else {
                        Ok(None)
                    }
                }
                Ok(Err(_)) => Ok(None),
                Err(_) => Ok(None),
            }
        }
        Ok(Err(_)) => Ok(None),
        Err(_) => Ok(None), // Session creation timed out
    }
}

/// Test if a host is running a DHCP server on port 67
pub async fn test_dhcp_service(ip: IpAddr, subnet_cidr: &IpCidr) -> Result<Option<u16>, Error> {
    let socket = match UdpSocket::bind("0.0.0.0:68").await {
        Ok(s) => s,
        Err(_) => {
            // If port 68 is busy (another DHCP client), try random port
            match UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => s,
                Err(_) => {
                    return Ok(None);
                }
            }
        }
    };

    if socket.set_broadcast(true).is_err() {
        return Ok(None);
    }

    // Calculate broadcast address for this subnet
    let broadcast_addr = match subnet_cidr {
        IpCidr::V4(cidr) => {
            let broadcast_ip = cidr.last_address();
            SocketAddr::new(IpAddr::V4(broadcast_ip), 67)
        }
        IpCidr::V6(_) => {
            return Ok(None);
        }
    };

    // Create a more complete DHCP DISCOVER message
    let mut rng = rand::rngs::StdRng::from_os_rng();
    let mac_addr: [u8; 6] = rng.random();
    let transaction_id = rng.random::<u32>();

    let mut msg = Message::default();
    msg.set_opcode(v4::Opcode::BootRequest)
        .set_htype(v4::HType::Eth)
        .set_xid(transaction_id)
        .set_flags(v4::Flags::default().set_broadcast())
        .set_chaddr(&mac_addr);

    // Add required and common DHCP options
    msg.opts_mut()
        .insert(v4::DhcpOption::MessageType(MessageType::Discover));

    // Add parameter request list (commonly requested by clients)
    msg.opts_mut()
        .insert(v4::DhcpOption::ParameterRequestList(vec![
            v4::OptionCode::SubnetMask,
            v4::OptionCode::Router,
            v4::OptionCode::DomainNameServer,
            v4::OptionCode::DomainName,
        ]));

    // Encode DHCP DISCOVER packet
    let mut buf = Vec::new();
    let mut encoder = Encoder::new(&mut buf);
    msg.encode(&mut encoder)?;

    if socket.send_to(&buf, broadcast_addr).await.is_ok()
        && let Some(port) = wait_for_dhcp_responses(&socket, ip, transaction_id, 3).await?
    {
        return Ok(Some(port));
    }

    // Fall back to unicast
    let unicast_addr = SocketAddr::new(ip, 67);

    if socket.send_to(&buf, unicast_addr).await.is_ok()
        && let Some(port) = wait_for_dhcp_responses(&socket, ip, transaction_id, 3).await?
    {
        return Ok(Some(port));
    }

    Ok(None)
}

/// Helper function to wait for and validate DHCP responses (checks multiple times)
async fn wait_for_dhcp_responses(
    socket: &UdpSocket,
    expected_ip: IpAddr,
    expected_xid: u32,
    max_attempts: usize,
) -> Result<Option<u16>, Error> {
    let mut response_buf = [0u8; 1500];

    for _ in 1..=max_attempts {
        match timeout(
            Duration::from_millis(2000), // Longer timeout per attempt
            socket.recv_from(&mut response_buf),
        )
        .await
        {
            Ok(Ok((len, from))) => {
                if len == 0 {
                    continue;
                }

                let response_ip = from.ip();

                // Check if response came from the IP we're testing
                if response_ip != expected_ip {
                    continue; // Keep trying - might get another response
                }

                // Parse and validate DHCP message
                match Message::decode(&mut dhcproto::Decoder::new(&response_buf[..len])) {
                    Ok(response_msg) => {
                        // Verify transaction ID matches
                        if response_msg.xid() != expected_xid {
                            continue;
                        }

                        // Check for valid DHCP response type
                        let message_type = response_msg.opts().iter().find_map(|(_, opt)| {
                            if let v4::DhcpOption::MessageType(msg_type) = opt {
                                Some(msg_type)
                            } else {
                                None
                            }
                        });

                        let is_valid = matches!(
                            message_type,
                            Some(&MessageType::Offer) | Some(&MessageType::Ack)
                        );

                        if is_valid {
                            return Ok(Some(67));
                        } else {
                            continue;
                        }
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
            Ok(Err(_)) => {
                break; // Socket error, no point continuing
            }
            Err(_) => {
                // Timeout - continue to next attempt
            }
        }
    }

    Ok(None)
}

/// Test if a host is running a BACnet service on UDP port 47808
pub async fn test_bacnet_service(ip: IpAddr) -> Result<Option<u16>, Error> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let target = SocketAddr::new(ip, 47808);

    // BACnet Who-Is probe packet
    // BVLC header + NPDU + Who-Is APDU
    let bacnet_probe: [u8; 12] = [
        0x81, // BVLC type indicator
        0x0a, // Original-Unicast-NPDU
        0x00, 0x0c, // Length: 12 bytes (big-endian)
        0x01, // NPDU version 1
        0x04, // NPDU control: expecting reply, no DNET/DLEN/DADR
        0x00, // Hop count (unused for unicast)
        0x00, // Reserved
        0x10, // APDU type: Unconfirmed service request
        0x08, // Service choice: Who-Is
        0x00, // No device instance range (optional field)
        0x00, // Padding to reach 12 bytes
    ];

    if socket.send_to(&bacnet_probe, target).await.is_err() {
        return Ok(None);
    }

    let mut response_buf = [0u8; 512];
    match timeout(
        Duration::from_millis(2000),
        socket.recv_from(&mut response_buf),
    )
    .await
    {
        Ok(Ok((len, from))) => {
            // Verify response is from the target IP
            if from.ip() != ip {
                return Ok(None);
            }

            // Check for valid BACnet response:
            // - At least 4 bytes (minimum BVLC header)
            // - First byte is 0x81 (BVLC type indicator)
            if len >= 4 && response_buf[0] == 0x81 {
                tracing::debug!("BACnet service detected on {}:47808", ip);
                return Ok(Some(47808));
            }

            Ok(None)
        }
        Ok(Err(_)) => Ok(None),
        Err(_) => Ok(None), // Timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_controller_initial_state() {
        let controller = ScanConcurrencyController::new(200);
        assert_eq!(controller.batch_size(), 200);
        assert!(!controller.is_degraded());
    }

    #[test]
    fn test_scan_controller_degradation_halves_batch_size() {
        let controller = ScanConcurrencyController::new(200);

        controller.on_fd_exhaustion();
        assert_eq!(controller.batch_size(), 100);
        assert!(controller.is_degraded());

        // Wait for cooldown before next degradation (rate limiting prevents cascading)
        std::thread::sleep(std::time::Duration::from_millis(
            DEGRADATION_COOLDOWN_MS + 10,
        ));

        controller.on_fd_exhaustion();
        assert_eq!(controller.batch_size(), 50);

        std::thread::sleep(std::time::Duration::from_millis(
            DEGRADATION_COOLDOWN_MS + 10,
        ));

        controller.on_fd_exhaustion();
        assert_eq!(controller.batch_size(), 25);

        std::thread::sleep(std::time::Duration::from_millis(
            DEGRADATION_COOLDOWN_MS + 10,
        ));

        controller.on_fd_exhaustion();
        assert_eq!(controller.batch_size(), 16); // Minimum floor
    }

    #[test]
    fn test_scan_controller_min_floor_enforced() {
        let controller = ScanConcurrencyController::new(32);

        controller.on_fd_exhaustion();
        assert_eq!(controller.batch_size(), 16); // 32/2 = 16, at floor

        std::thread::sleep(std::time::Duration::from_millis(
            DEGRADATION_COOLDOWN_MS + 10,
        ));

        controller.on_fd_exhaustion();
        assert_eq!(controller.batch_size(), 16); // Should stay at floor
    }

    #[test]
    fn test_scan_controller_recovery_after_threshold() {
        let controller = ScanConcurrencyController::new(200);

        // Degrade to 100
        controller.on_fd_exhaustion();
        assert_eq!(controller.batch_size(), 100);
        assert!(controller.is_degraded());

        // 49 successes - not enough
        for _ in 0..49 {
            controller.on_success();
        }
        assert_eq!(controller.batch_size(), 100); // No change yet

        // 50th success triggers recovery (25% increase: 100 -> 125)
        controller.on_success();
        assert_eq!(controller.batch_size(), 125);
        assert!(controller.is_degraded()); // Still degraded, not at target

        // More successes to continue recovery
        for _ in 0..50 {
            controller.on_success();
        }
        assert_eq!(controller.batch_size(), 156); // 125 * 1.25 = 156

        // Keep going until full recovery
        for _ in 0..50 {
            controller.on_success();
        }
        assert_eq!(controller.batch_size(), 195); // 156 * 1.25 = 195

        for _ in 0..50 {
            controller.on_success();
        }
        assert_eq!(controller.batch_size(), 200); // Capped at target
        assert!(!controller.is_degraded()); // Fully recovered
    }

    #[test]
    fn test_scan_controller_success_resets_streak_on_degradation() {
        let controller = ScanConcurrencyController::new(200);

        // Degrade
        controller.on_fd_exhaustion();
        assert_eq!(controller.batch_size(), 100);

        // Build up a streak
        for _ in 0..40 {
            controller.on_success();
        }

        // Wait for cooldown before another degradation
        std::thread::sleep(std::time::Duration::from_millis(
            DEGRADATION_COOLDOWN_MS + 10,
        ));

        // Another FD exhaustion resets everything
        controller.on_fd_exhaustion();
        assert_eq!(controller.batch_size(), 50);

        // Need full 50 successes again
        for _ in 0..49 {
            controller.on_success();
        }
        assert_eq!(controller.batch_size(), 50); // Not recovered yet

        controller.on_success();
        assert_eq!(controller.batch_size(), 62); // 50 * 1.25 = 62
    }

    #[test]
    fn test_scan_controller_success_ignored_when_not_degraded() {
        let controller = ScanConcurrencyController::new(200);
        assert!(!controller.is_degraded());

        // Success calls should be no-ops when not degraded
        for _ in 0..100 {
            controller.on_success();
        }

        assert_eq!(controller.batch_size(), 200);
        assert!(!controller.is_degraded());
    }

    #[cfg(unix)]
    #[test]
    fn test_scan_controller_emfile_detection() {
        let controller = ScanConcurrencyController::new(200);

        // Create an EMFILE error (error code 24)
        let emfile_error = std::io::Error::from_raw_os_error(24);

        assert!(controller.check_and_handle_error(&emfile_error));
        assert_eq!(controller.batch_size(), 100);
        assert!(controller.is_degraded());
    }

    #[test]
    fn test_scan_controller_non_emfile_error_ignored() {
        let controller = ScanConcurrencyController::new(200);

        // Connection refused error - should not trigger degradation
        let conn_refused = std::io::Error::new(ErrorKind::ConnectionRefused, "connection refused");

        assert!(!controller.check_and_handle_error(&conn_refused));
        assert_eq!(controller.batch_size(), 200);
        assert!(!controller.is_degraded());
    }
}
