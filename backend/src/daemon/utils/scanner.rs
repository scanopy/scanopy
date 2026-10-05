use crate::daemon::discovery::service::warnings::AttemptOutcome;
use crate::daemon::discovery::types::base::DiscoveryCriticalError;
use crate::server::services::r#impl::base::Service;
use crate::server::services::r#impl::endpoints::{Endpoint, EndpointResponse};
use anyhow::anyhow;
use anyhow::{Error, Result};
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use snmp2::{AsyncSession, Oid};
use std::collections::HashMap;
#[cfg(any(unix, test))]
use std::io::ErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::{net::TcpStream, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::server::credentials::r#impl::mapping::{SnmpQueryCredential, SnmpVersion};
use crate::server::ports::r#impl::base::PortType;

pub const SCAN_TIMEOUT: Duration = Duration::from_millis(800);

/// Read response body until a deadline, returning whatever was downloaded.
/// Service identification only needs enough body to find identifying strings
/// (e.g., "portainer.io" in a 22KB page). Streaming with a deadline ensures
/// we get partial data instead of failing entirely when large responses
/// exceed the timeout.
pub async fn read_response_body_until_deadline(
    response: reqwest::Response,
    deadline: tokio::time::Instant,
) -> String {
    use futures::StreamExt;
    let mut body_bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Ok(Some(Ok(chunk))) = tokio::time::timeout_at(deadline, stream.next()).await {
        body_bytes.extend_from_slice(&chunk);
    }
    String::from_utf8_lossy(&body_bytes).to_string()
}

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
pub(crate) async fn batch_scan<T, O, F, Fut>(
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
    let available = crate::daemon::discovery::service::network::arp::is_available(use_npcap);

    if available {
        tracing::info!("ARP scanning capability confirmed. Fast host discovery enabled.");
    } else {
        tracing::warn!(
            "ARP scanning not available. Will fall back to TCP port scanning for host discovery. \
             For MACVLAN deployments, ensure: (1) container has NET_RAW and NET_ADMIN capabilities, \
             (2) network ip_address is properly configured with a MAC address."
        );
    }

    available
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

    tracing::trace!(
        ip = %ip,
        ports_scanned = %ports.len(),
        responses = %open_ports.len(),
        effective_batch_size,
        degraded = controller_for_log.is_degraded(),
        "TCP ports scanned"
    );

    Ok(open_ports)
}

/// Probe the two SNMP ports, one session at a time per host.
///
/// What is left of `scan_udp_ports` after the four non-credentialed probes moved onto the
/// application-probe stage. It keeps its own function because SNMP is not batched with anything:
/// sessions are sequential per host, and credentials are tried in specificity order.
///
/// **Not dead when `snmp_credentials` is empty.** The network scan passes `&[]` because
/// `SnmpIntegration` owns credentialed probing, and this still tries the hardcoded `public`
/// community on both ports — which is the only reason an SNMP device with a default community is
/// found on a network with no credential configured at all.
pub async fn probe_snmp_ports(
    ip: IpAddr,
    cancel: CancellationToken,
    snmp_credentials: &[SnmpQueryCredential],
) -> Result<Vec<PortType>, Error> {
    let snmp_port_numbers: Vec<u16> = vec![161, 1161];
    let mut open_ports: Vec<PortType> = Vec::new();

    // Don't short-circuit: both ports could be open, and different communities can expose
    // different MIB views.
    if !cancel.is_cancelled() {
        for &port in &snmp_port_numbers {
            if cancel.is_cancelled() {
                break;
            }

            let mut port_detected = false;

            // Try each credential in specificity order (IP override → site default → public)
            for cred in snmp_credentials {
                if let SnmpProbeOutcome::Answered(p) =
                    try_snmp_with_credential_on_port(ip, cred, port).await
                {
                    if !port_detected {
                        open_ports.push(PortType::new_udp(p));
                        port_detected = true;
                    }
                    break; // port detected with this credential, move to next port
                }
            }

            // If no configured credential worked, try hardcoded "public"
            // (covers case where snmp_credentials is empty)
            if !port_detected && let Ok(Some(p)) = try_snmp_with_public_on_port(ip, port).await {
                open_ports.push(PortType::new_udp(p));
            }
        }
    }

    tracing::debug!(
        ip = %ip,
        ports_scanned = %snmp_port_numbers.len(),
        responses = %open_ports.len(),
        "SNMP ports probed"
    );

    Ok(open_ports)
}

/// Requests `endpoint` at each of `urls` in order and returns the first response.
///
/// A `400 Bad Request` is held back while another URL remains. TLS listeners (cockpit-tls,
/// nginx, Go's net/http) answer a plain-HTTP request with 400, and the TCP peek in
/// `scan_tcp_ports` does not reliably flag those ports as HTTPS, so the HTTP attempt can come
/// first and reach nothing of the application. The next URL's response wins whatever its
/// status; if it gets none, the 400 is returned.
async fn fetch_endpoint(
    client: &reqwest::Client,
    endpoint: Endpoint,
    urls: Vec<String>,
) -> Option<EndpointResponse> {
    let mut bad_request = None;
    let attempts = urls.len();
    for (attempt, url) in urls.into_iter().enumerate() {
        tracing::trace!("Trying endpoint: {}", url);

        // Timeout covers TCP connect + TLS handshake + response headers.
        // Body streaming has its own deadline via read_response_body_until_deadline.
        // Without this, non-HTTP services (e.g. Chromecast port 8009) that accept
        // TCP but never send HTTP headers would block .send() indefinitely.
        match timeout(SCAN_TIMEOUT, client.get(&url).send()).await {
            Ok(Ok(response)) => {
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

                let deadline = tokio::time::Instant::now() + SCAN_TIMEOUT;
                let body = read_response_body_until_deadline(response, deadline).await;
                tracing::debug!(
                    "Endpoint {} returned {} (length: {})",
                    url,
                    status,
                    body.len()
                );
                let response = EndpointResponse {
                    endpoint: endpoint.clone(),
                    headers,
                    body,
                    status,
                };
                if status == reqwest::StatusCode::BAD_REQUEST.as_u16()
                    && bad_request.is_none()
                    && attempt + 1 < attempts
                {
                    bad_request = Some(response);
                    continue;
                }
                return Some(response);
            }
            Ok(Err(e)) => {
                tracing::trace!("Endpoint {} failed: {}", url, e);
                if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                    tracing::error!("Critical error scanning endpoint {}: {}", url, e);
                }
                continue;
            }
            Err(_) => {
                tracing::trace!("Endpoint {} timed out waiting for response headers", url);
                continue;
            }
        }
    }

    bad_request
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
        .connect_timeout(SCAN_TIMEOUT)
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

            fetch_endpoint(&client, endpoint_with_ip, urls).await
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

/// Try an SNMP GET on a specific port using a credential
/// What an SNMP liveness probe established, beyond "it did not work".
///
/// Every arm of this used to collapse into `Ok(None)`, and the caller reported "SNMP not
/// responding with any credential". For SNMPv3 that was actively wrong: `create_session`
/// performs engine discovery *and authentication*, so it had already produced "Failed SNMPv3
/// engine discovery / authentication" — and an operator with a mistyped password was sent to
/// check whether the device was online.
pub enum SnmpProbeOutcome {
    /// Answered sysDescr on this port.
    Answered(u16),
    Failed(AttemptOutcome, String),
}

pub async fn try_snmp_with_credential_on_port(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> SnmpProbeOutcome {
    let sys_descr_oid = match Oid::from(&[1, 3, 6, 1, 2, 1, 1, 1, 0]) {
        Ok(oid) => oid,
        Err(e) => {
            return SnmpProbeOutcome::Failed(
                AttemptOutcome::Malformed,
                format!("Invalid OID: {e:?}"),
            );
        }
    };

    // Liveness probe: the default context, because that is where sysDescr lives on every device.
    // A credential naming a bridge context must not be judged unreachable because that context
    // holds no system MIB.
    let mut session = match crate::daemon::discovery::integration::snmp::session::create_session(
        ip,
        credential,
        port,
        crate::daemon::discovery::integration::snmp::session::SnmpContext::Default,
    )
    .await
    {
        Ok(session) => session,
        // v3 authenticates during engine discovery, so a failure here *is* the credential's
        // answer. v1/v2c carry no handshake — a failure is the socket, not the community.
        //
        // Both readings assume the credential reached the wire. `for_credential_error` is what
        // says when it did not: a field we could not read fails before the socket is opened, and
        // carries no verdict on the device at all (GH #668).
        Err(e) => {
            let otherwise = if matches!(credential.version, SnmpVersion::V3) {
                AttemptOutcome::Rejected
            } else {
                AttemptOutcome::Unreachable
            };
            return SnmpProbeOutcome::Failed(
                AttemptOutcome::for_credential_error(&e, otherwise),
                e.to_string(),
            );
        }
    };

    match timeout(Duration::from_millis(2000), session.get(&sys_descr_oid)).await {
        Ok(Ok(mut response)) => {
            if response.varbinds.next().is_some() {
                SnmpProbeOutcome::Answered(port)
            } else {
                SnmpProbeOutcome::Failed(
                    AttemptOutcome::NotThisService,
                    "answered without any varbinds".to_string(),
                )
            }
        }
        Ok(Err(e)) => SnmpProbeOutcome::Failed(AttemptOutcome::from(&e), e.to_string()),
        // The overwhelmingly common case on a swept subnet: nothing there speaks SNMP. Reported
        // only for a credential the user pinned to this address.
        Err(_) => SnmpProbeOutcome::Failed(
            AttemptOutcome::TimedOut,
            "no answer to sysDescr".to_string(),
        ),
    }
}

/// Try an SNMP GET on a specific port using the default "public" community string
pub async fn try_snmp_with_public_on_port(ip: IpAddr, port: u16) -> Result<Option<u16>, Error> {
    let sys_descr_oid =
        Oid::from(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).map_err(|e| anyhow!("Invalid Oid: {:?}", e))?;

    let target = format!("{}:{}", ip, port);
    let community = b"public";

    let session_result = timeout(
        Duration::from_millis(2000),
        AsyncSession::new_v2c(&target, community, 0),
    )
    .await;

    match session_result {
        Ok(Ok(mut session)) => {
            match timeout(Duration::from_millis(2000), session.get(&sys_descr_oid)).await {
                Ok(Ok(mut response)) => {
                    if response.varbinds.next().is_some() {
                        Ok(Some(port))
                    } else {
                        Ok(None)
                    }
                }
                Ok(Err(_)) => Ok(None),
                Err(_) => Ok(None),
            }
        }
        Ok(Err(_)) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// A credential the daemon cannot read at all, and what it is reported as.
///
/// GH #668. The reporter's SNMP credential held `{"mode":"FilePath","path":"public"}` — a
/// community string typed into a file-path field — and every host in the scan reported
/// `outcome=Unreachable`, which is a verdict about the network. Nothing about the network was
/// wrong: `os error 2` is a missing file, the probe never opened a socket, and the fix is one edit
/// in Scanopy rather than anything on the switch.
///
/// No network is involved in either test: `create_session` resolves the credential's fields before
/// it binds anything, so an unresolvable one fails before the address is ever contacted. That is
/// also the reason this is worth classifying — the failure carries no information about the host.
#[cfg(test)]
mod unreadable_credential_tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::{ResolvableSecret, SnmpV3Params};
    use crate::server::credentials::r#impl::types::{SnmpV3AuthProtocol, SnmpV3PrivProtocol};

    fn missing_file() -> ResolvableSecret {
        ResolvableSecret::FilePath {
            path: "/nonexistent/scanopy-test/community".into(),
        }
    }

    fn ip() -> IpAddr {
        "192.0.2.1".parse().unwrap()
    }

    #[tokio::test]
    async fn a_community_that_cannot_be_read_is_a_configuration_fault_not_an_unreachable_device() {
        let credential = SnmpQueryCredential {
            version: SnmpVersion::V2c,
            community: missing_file(),
            v3: None,
        };

        let outcome = try_snmp_with_credential_on_port(ip(), &credential, 161).await;

        let SnmpProbeOutcome::Failed(outcome, _) = outcome else {
            panic!("a credential that cannot be read cannot have answered");
        };
        assert_eq!(
            outcome,
            AttemptOutcome::Malformed,
            "reporting this as Unreachable sends an operator to check that a device is online, \
             when what is wrong is the credential we never managed to read"
        );
    }

    /// The v3 path reaches the same verdict by a different route. A v3 credential *does*
    /// authenticate during engine discovery, so a `create_session` failure is normally the
    /// device's answer and is reported as `Rejected` — but not when the failure happened before
    /// the handshake, resolving a password file that is not there. "Check the username and
    /// password" is the wrong instruction for a password nobody could read.
    #[tokio::test]
    async fn a_v3_password_file_that_cannot_be_read_is_not_reported_as_a_refused_credential() {
        let credential = SnmpQueryCredential {
            version: SnmpVersion::V3,
            community: ResolvableSecret::Value {
                value: String::new(),
            },
            v3: Some(SnmpV3Params {
                security_name: "monitor".to_string(),
                auth_protocol: SnmpV3AuthProtocol::Sha1,
                auth_password: missing_file(),
                priv_protocol: SnmpV3PrivProtocol::Aes128,
                priv_password: missing_file(),
                context_name: None,
            }),
        };

        let outcome = try_snmp_with_credential_on_port(ip(), &credential, 161).await;

        let SnmpProbeOutcome::Failed(outcome, _) = outcome else {
            panic!("a credential that cannot be read cannot have answered");
        };
        assert_eq!(outcome, AttemptOutcome::Malformed);
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

    mod fetch_endpoint {
        use super::*;
        use rustls::pki_types::pem::PemObject;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
        use tokio::net::TcpListener;

        // Throwaway self-signed P-256 cert for 127.0.0.1 listeners; the client skips verification.
        const CERT_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIBfzCCASWgAwIBAgIUK9JlYRip00xB8Gndjgk3zb5Wqg4wCgYIKoZIzj0EAwIw
FDESMBAGA1UEAwwJbG9jYWxob3N0MCAXDTI2MDkyOTAxNDc0M1oYDzIxMjYwOTA1
MDE0NzQzWjAUMRIwEAYDVQQDDAlsb2NhbGhvc3QwWTATBgcqhkjOPQIBBggqhkjO
PQMBBwNCAASWhzFdL+lxOqhRlT5B8+kiBS86zxvIpav+CuhxIhqTNk7mB309FRVS
0CYSlEft7THfykUb67efpyxaxpI6rJhMo1MwUTAdBgNVHQ4EFgQULwbUF5TojfIH
kvkwF3qiqwZnUhUwHwYDVR0jBBgwFoAULwbUF5TojfIHkvkwF3qiqwZnUhUwDwYD
VR0TAQH/BAUwAwEB/zAKBggqhkjOPQQDAgNIADBFAiEA3kWdhykfMKVIDt9A0G4k
KPcx8C7FMfezckPzvmzq8JMCIFsjUXIAVie/pimi/PGXS1Xey/VfACNxcIS+sIkp
5g2P
-----END CERTIFICATE-----";
        const KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQglR2BoNuUDBSzNHFA
uuMtBnV5LaUBKtyWZYXPE4wRHyqhRANCAASWhzFdL+lxOqhRlT5B8+kiBS86zxvI
pav+CuhxIhqTNk7mB309FRVS0CYSlEft7THfykUb67efpyxaxpI6rJhM
-----END PRIVATE KEY-----";

        const TLS_HANDSHAKE_RECORD: u8 = 0x16;

        fn tls_acceptor() -> tokio_rustls::TlsAcceptor {
            let config = rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from_pem_slice(CERT_PEM.as_bytes()).unwrap()],
                PrivateKeyDer::from_pem_slice(KEY_PEM.as_bytes()).unwrap(),
            )
            .unwrap();
            tokio_rustls::TlsAcceptor::from(Arc::new(config))
        }

        async fn respond<S: AsyncRead + AsyncWrite + Unpin>(mut stream: S, status_line: &str) {
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf).await {
                    Ok(0) | Err(_) => return,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let response =
                format!("HTTP/1.1 {status_line}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.shutdown().await;
        }

        /// Serves 127.0.0.1 on an ephemeral port. A connection opening with a TLS handshake gets
        /// `tls_status`; a plain-HTTP one gets `400 Bad Request`, as a TLS listener answers it.
        async fn serve(tls_status: Option<&'static str>) -> u16 {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let acceptor = tls_acceptor();
            tokio::spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    let acceptor = acceptor.clone();
                    tokio::spawn(async move {
                        let mut first = [0u8; 1];
                        let is_tls = matches!(stream.peek(&mut first).await, Ok(1))
                            && first[0] == TLS_HANDSHAKE_RECORD;
                        match (is_tls, tls_status) {
                            (true, Some(status)) => {
                                if let Ok(tls) = acceptor.accept(stream).await {
                                    respond(tls, status).await;
                                }
                            }
                            (true, None) => {}
                            (false, _) => respond(stream, "400 Bad Request").await,
                        }
                    });
                }
            });
            port
        }

        async fn fetch_http_first(port: u16) -> Option<EndpointResponse> {
            let client = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap();
            let endpoint = Endpoint::for_pattern(PortType::new_tcp(port), "/");
            let urls = vec![
                format!("http://127.0.0.1:{port}/"),
                format!("https://127.0.0.1:{port}/"),
            ];
            fetch_endpoint(&client, endpoint, urls).await
        }

        #[tokio::test]
        async fn a_400_is_kept_when_the_other_scheme_gets_no_response() {
            let port = serve(None).await;

            let response = fetch_http_first(port).await.expect("the 400 is a response");

            assert_eq!(response.status, 400);
        }

        /// A TLS port the TCP peek did not flag as HTTPS is tried over plain HTTP first, and the
        /// TLS listener refuses that with 400. The HTTPS attempt is what reaches the application.
        #[tokio::test]
        async fn a_400_over_http_falls_through_to_https() {
            let port = serve(Some("200 OK")).await;

            let response = fetch_http_first(port).await.expect("HTTPS answered");

            assert_eq!(response.status, 200);
        }
    }
}
