use super::arp::{self, ArpScanResult};
use super::dcp;
use super::icmp;
use super::mdns;
use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::discovery::service::warnings::{CredentialIssue, CredentialIssueReason};
use crate::daemon::discovery::types::base::DiscoveryCriticalError;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::daemon::utils::app_probe::{ProbeContext, scan_app_probes};
use crate::daemon::utils::base::{DaemonUtils, PlatformDaemonUtils, filtered_own_nics};
use crate::daemon::utils::scanner::{
    ScanConcurrencyController, can_arp_scan, probe_snmp_ports, scan_endpoints, scan_tcp_ports,
};
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, IpOverride,
};
use crate::server::discovery::r#impl::scan_settings::defaults;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase, InterfaceDataComplete};
use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::{Service, ServiceMatchBaselineParams};
use crate::server::shared::attribution::{AttributeSource, Attributed};
use crate::server::shared::types::entities::EntitySource;
use crate::server::{
    daemons::r#impl::base::DaemonMode,
    hosts::r#impl::{
        api::{DiscoveryHostRequest, HostResponse},
        attributes::HostHostnameValue,
        base::{Host, HostBase},
        name::host_name_from_parts,
    },
    subnets::r#impl::base::Subnet,
};
use anyhow::Error;
use cidr::IpCidr;
use futures::StreamExt;
use pnet::datalink;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{net::IpAddr, sync::Arc};
use tokio::sync::mpsc as tokio_mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::{
    CONNECT_ONLY_PORTS, DeepScanParams, DiscoveredHostData, DispatchedAddresses, FULL_SCAN_COST_CS,
    LATE_ARRIVAL_GRACE_PERIOD, LIGHT_SCAN_COST_CS, LivenessEvidence, MAX_PROGRESS_REPORT_INTERVAL,
    NetworkScan, PROGRESS_ARP_PHASE, PROGRESS_DEEP_SCAN_PHASE, PROGRESS_GRACE_PHASE,
    RESPONSIVENESS_COST_CS, enumerated_host_has_evidence, integration_cost_for_ip, is_host_address,
    liveness_probe_ports,
};

impl NetworkScan {
    pub async fn scan_and_process_hosts(
        &self,
        subnets: Vec<Subnet>,
        network_subnets: Vec<Subnet>,
        target_ips: Option<HashSet<IpAddr>>,
        cancel: CancellationToken,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
    ) -> Result<Vec<(IpAddr, Host, DiscoveredHostData)>, Error> {
        let session = ops.get_session().await?;

        let interface_filter = ops.config_store.get_interfaces().await?;
        let (own_addresses, _, subnet_cidr_to_mac) = utils
            .get_own_interfaces(session.info.network_id, &interface_filter)
            .await?;

        // Filter out loopback subnets — they are not scannable
        let subnets: Vec<Subnet> = subnets
            .into_iter()
            .filter(|s| !s.base.subnet_type.is_loopback())
            .collect();

        // A targeted rescan narrows enumeration to specific addresses within the
        // subnets above. `None` means sweep them entirely.
        let is_targeted = |ip: &IpAddr| target_ips.as_ref().is_none_or(|t| t.contains(ip));

        // Credentials resolve per-IP during the scan of that IP, and the enumeration above is
        // the only source of addresses — `target_ips` narrows it and never adds to it. So a
        // credential assigned to a host outside these subnets is skipped in complete silence,
        // which is exactly how a correctly configured integration can produce no traffic and no
        // error. Say so before the scan starts, while the reason is still knowable.
        let unreachable = unreachable_credential_targets(&self.credential_mappings, &subnets);
        if !unreachable.is_empty()
            && let Ok(session_state) = ops.get_session().await
            && let Ok(mut issues) = session_state.credential_issues.lock()
        {
            issues.extend(unreachable);
        }

        // Get scan settings from discovery request, falling back to defaults
        let use_npcap = self.scan_settings.use_npcap_arp;
        let arp_retries = self
            .scan_settings
            .arp_retries
            .unwrap_or(defaults::arp_retries());
        let arp_rate_pps = self
            .scan_settings
            .arp_rate_pps
            .unwrap_or(defaults::arp_rate_pps());
        let scan_rate_pps = self
            .scan_settings
            .scan_rate_pps
            .unwrap_or(defaults::scan_rate_pps());
        let port_scan_batch_size = self
            .scan_settings
            .port_scan_batch_size
            .unwrap_or(defaults::port_scan_batch_size())
            .clamp(16, 1000);

        // Check ARP capability once before partitioning
        let arp_available = can_arp_scan(use_npcap);

        // Scan scope, kept for the post-scan credential-reachability check: a credential
        // targeting an address outside these was never contacted, and saying so is the point.
        let scanned_subnets = subnets.clone();

        // Partition subnets (not IPs) into interfaced vs non-interfaced.
        // IPs are generated per-subnet at point of use to avoid allocating a
        // single Vec with every IP across all subnets (which OOMs on bogus CIDRs).
        let (interfaced_subnets, non_interfaced_subnets): (Vec<_>, Vec<_>) = if arp_available {
            subnets.into_iter().partition(|s: &Subnet| {
                subnet_cidr_to_mac
                    .get(&s.base.cidr)
                    .and_then(|m| *m)
                    .is_some()
            })
        } else {
            (Vec::new(), subnets)
        };

        let count_ips =
            |subnets: &[Subnet]| -> u64 { count_scan_ips(subnets, target_ips.as_ref()) };
        let interfaced_ip_count = count_ips(&interfaced_subnets);
        let non_interfaced_ip_count = count_ips(&non_interfaced_subnets);
        let total_ips = interfaced_ip_count + non_interfaced_ip_count;

        // Calculate estimated ARP duration for progress reporting
        let arp_target_count = interfaced_ip_count;
        let total_rounds = 1 + arp_retries as u64;
        let send_time_per_round_secs = arp_target_count / arp_rate_pps.max(1) as u64;
        let estimated_arp_duration = Duration::from_secs(
            total_rounds * (send_time_per_round_secs + arp::ROUND_WAIT.as_secs())
                + arp::POST_SCAN_RECEIVE.as_secs(),
        );
        let pipeline_start = Instant::now();

        tracing::info!(
            total_ips,
            interfaced_ips = interfaced_ip_count,
            non_interfaced_ips = non_interfaced_ip_count,
            estimated_arp_secs = estimated_arp_duration.as_secs(),
            arp_method = if cfg!(target_family = "windows") && !use_npcap {
                "SendARP"
            } else {
                "Broadcast"
            },
            "Starting continuous discovery pipeline"
        );

        ops.report_progress(0).await?;

        let arp_subnet_count = interfaced_subnets.len();

        // Use the port batch size from the coordinated calculation
        let effective_batch_size = port_scan_batch_size;

        // Calculate deep scan concurrency based on FDs available after ARP
        let mut deep_scan_concurrency =
            utils.get_optimal_deep_scan_concurrency(effective_batch_size, arp_subnet_count)?;

        // Create shared concurrency controller for graceful degradation
        let scan_controller = ScanConcurrencyController::new(effective_batch_size);

        let gateway_ips = utils.get_own_routing_table_gateway_ips().await?;

        // Create async channel for discovered hosts
        // Buffer size allows ARP to run ahead while deep scanning catches up
        let (host_tx, mut host_rx) = tokio_mpsc::channel::<(IpAddr, Subnet, LivenessEvidence)>(256);

        // Start ARP scanning for interfaced subnets — build target IPs per-subnet.
        // ARP needs all targets upfront (multi-round retries), so a Vec is required.
        // Cap per-subnet based on the configurable arp_scan_cutoff prefix.
        let arp_cutoff = self
            .scan_settings
            .arp_scan_cutoff
            .unwrap_or(defaults::arp_scan_cutoff());
        let max_arp_targets: usize = 1usize << (32 - arp_cutoff as u32);

        // Work-based discovery progress signal (B): `discovery_packets_sent` is incremented per
        // ARP *and* ICMP request across every subnet/round; `discovery_packets_total` is the upper
        // bound of requests this scan will send (targets × rounds). Progress/ETA derive from real
        // send throughput so a wrong rate estimate can't pin the bar or lie about the ETA. Both
        // sweeps feed the same pair because they share the one 0-30% progress band.
        let discovery_packets_sent = Arc::new(AtomicU64::new(0));
        let mut discovery_packets_total: u64 = 0;
        let arp_total_rounds = 1 + arp_retries as u64;

        // ---------------------------------------------------------------
        // mDNS / DNS-SD browse
        // ---------------------------------------------------------------
        // Runs to completion before the sweeps rather than alongside them: it is one multicast
        // burst per broadcast domain taking a few seconds against a scan measured in minutes, and
        // every host scanned afterwards can then be matched against a complete picture instead of
        // racing one being assembled.
        //
        // Link-local by construction, so this only ever sees the daemon's own broadcast domains.
        // A routed subnet gets nothing from it however live its hosts are — stated here because
        // an empty result on such a subnet is correct behaviour, not a fault.
        let mdns_hosts = Arc::new(
            self.browse_mdns(&own_addresses, &scanned_subnets, target_ips.as_ref())
                .await,
        );

        // ---------------------------------------------------------------
        // PROFINET DCP Identify sweep — see network::dcp
        // ---------------------------------------------------------------
        // Unlike every sweep above and below, DCP never targets an IP: it multicasts once per
        // interface and an unknown number of devices reply, some with no IP at all. It has
        // nothing to feed into the IP-keyed `host_tx` channel the rest of this pipeline uses, and
        // a device it finds may never reach `deep_scan_host`. So it runs as a background task
        // collecting raw results here, submitted directly via `ops.create_host` — the same
        // function every other discovery source uses — once joined near the end of this function.
        // Spawned now rather than awaited inline so its listen window (a few seconds per
        // interface) overlaps the rest of the pipeline instead of adding to it.
        //
        // Runs on a dedicated `std::thread::spawn`, fired synchronously right here, mirroring
        // `arp::scan_subnet`'s shape rather than going through `tokio::task::spawn_blocking`.
        // `datalink::channel()` opens a raw BPF/AF_PACKET fd, and on macOS opening one at fd >=
        // 1024 used to abort the process (see vendor/pnet_datalink/README.md; fixed there). ARP
        // avoids ever being anywhere near that by opening its channel on its own OS thread,
        // started before deep-scan's fd-heavy work has ramped up — not queued behind whatever
        // else the Tokio blocking pool happens to be running. DCP gets the same guarantee here:
        // the thread below starts immediately, so its `datalink::channel()` call runs at a
        // predictably low fd rather than at a scheduling-dependent one. Only the *result*, not
        // the channel open, crosses back into async code — via a oneshot that the thread fills
        // once its whole sweep (every interface) is done.
        let dcp_capable_interfaces: Vec<_> = filtered_own_nics(&interface_filter)
            .into_iter()
            .filter(dcp::is_dcp_capable)
            .collect();
        let (dcp_tx, dcp_sweep) = tokio::sync::oneshot::channel();
        std::thread::spawn(move || {
            let mut found = Vec::new();
            for interface in dcp_capable_interfaces {
                let name = interface.name.clone();
                match dcp::scan_interface(&interface) {
                    Ok(responses) => found.extend(responses),
                    Err(e) => {
                        tracing::warn!(interface = %name, error = %e, "DCP sweep failed on interface")
                    }
                }
            }
            let _ = dcp_tx.send(found);
        });

        // ---------------------------------------------------------------
        // ICMP echo sweep — the second liveness signal
        // ---------------------------------------------------------------
        // Runs concurrently with ARP over the whole scan scope. It exists because an address on
        // an interfaced subnet that does not answer ARP is otherwise never queued at all: the ARP
        // forwarder emits one message per reply and nothing else (GH #678). It is purely
        // additive — it never filters what the other paths would have found, because plenty of
        // hosts answer ARP while dropping ICMP (Windows blocks echo by default).
        let icmp_available = icmp::is_available();
        let icmp_responders = if icmp_available {
            // One flat target list: ICMP is layer 4 and routed by the kernel, so a single sweep
            // covers interfaced and non-interfaced subnets alike. Capped by the same cutoff ARP
            // uses — this list is materialised, and an uncapped /8 would exhaust memory the way
            // the streaming enumeration below is careful not to.
            let mut targets: Vec<std::net::Ipv4Addr> = Vec::new();
            for subnet in &scanned_subnets {
                for addr in subnet.base.cidr.iter().map(|a| a.address()) {
                    if !is_targeted(&addr) {
                        continue;
                    }
                    // The network and broadcast addresses are skipped here and nowhere else: ARP
                    // enumerates them harmlessly because nothing owns them and nothing replies,
                    // while an echo request to the same address is answered by every responder on
                    // the segment at once. See `is_host_address`.
                    if !is_host_address(&subnet.base.cidr, addr) {
                        continue;
                    }
                    if let IpAddr::V4(ipv4) = addr {
                        targets.push(ipv4);
                        if targets.len() >= max_arp_targets {
                            break;
                        }
                    }
                }
                if targets.len() >= max_arp_targets {
                    tracing::warn!(
                        cutoff = format!("/{}", arp_cutoff),
                        max_ips = max_arp_targets,
                        "ICMP target list truncated to /{} cutoff",
                        arp_cutoff
                    );
                    break;
                }
            }
            discovery_packets_total += targets.len() as u64 * arp_total_rounds;
            spawn_icmp_sweep(targets, arp_retries, scan_rate_pps, &discovery_packets_sent)
        } else {
            // Resolves immediately with nothing, so every consumer below takes the same path
            // whether or not ICMP is available.
            IcmpSweep::unavailable()
        };

        // Joined by the ICMP release task below, which must not run until every ARP forwarder has
        // finished — otherwise it could claim an address ARP was about to report, and the host
        // would lose the MAC only ARP carries.
        let mut arp_forwarders: Vec<std::thread::JoinHandle<()>> = Vec::new();

        if !interfaced_subnets.is_empty() {
            let mut subnet_to_ips: HashMap<IpCidr, (Subnet, Vec<std::net::Ipv4Addr>)> =
                HashMap::new();
            for subnet in &interfaced_subnets {
                let entry = subnet_to_ips
                    .entry(*subnet.base.cidr)
                    .or_insert_with(|| (subnet.clone(), Vec::new()));
                for addr in subnet.base.cidr.iter().map(|a| a.address()) {
                    if !is_targeted(&addr) {
                        continue;
                    }
                    if let IpAddr::V4(ipv4) = addr {
                        entry.1.push(ipv4);
                        if entry.1.len() >= max_arp_targets {
                            tracing::warn!(
                                cidr = %subnet.base.cidr,
                                cutoff = format!("/{}", arp_cutoff),
                                max_ips = max_arp_targets,
                                "ARP target list truncated to /{} cutoff",
                                arp_cutoff
                            );
                            break;
                        }
                    }
                }
            }

            let actual_arp_targets: usize = subnet_to_ips.values().map(|(_, ips)| ips.len()).sum();
            tracing::info!(
                subnets = subnet_to_ips.len(),
                total_ips = actual_arp_targets,
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
                discovery_packets_total += target_count as u64 * arp_total_rounds;

                match arp::scan_subnet(
                    &interface,
                    source_ipv4,
                    source_mac,
                    target_ips,
                    use_npcap,
                    arp_retries,
                    arp_rate_pps,
                    discovery_packets_sent.clone(),
                ) {
                    Ok(arp_rx) => {
                        // Spawn a task to forward ARP results to the async channel
                        // Use spawn_blocking since std::sync::mpsc::recv_timeout is blocking
                        let host_tx = host_tx.clone();
                        let subnet = subnet.clone();

                        // Use a background thread for the blocking recv, forward via channel.
                        // Hard timeout prevents infinite hangs if the ARP receiver thread
                        // gets stuck (e.g., on bridge ip_addresses with continuous traffic).
                        // The handle is kept so the ICMP release below can wait for every
                        // forwarder to finish — that is what makes "ARP had its chance at this
                        // address" a fact rather than a guess.
                        arp_forwarders.push(std::thread::spawn(move || {
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
                                        // Use blocking_send since we're in a std thread.
                                        // ARP evidence dispatches immediately and outranks
                                        // everything else — it is the only signal carrying a MAC.
                                        if host_tx
                                            .blocking_send((
                                                IpAddr::V4(ip),
                                                subnet.clone(),
                                                LivenessEvidence::Arp(mac),
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
                        }));
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

        // Release the addresses a non-ARP signal answered for.
        //
        // This is the case those signals exist for: on a subnet the daemon has an interface on, an
        // address that does not answer ARP was previously never queued at all — the forwarder
        // above emits one message per reply and nothing else — so a VM behind a hypervisor bridge
        // was invisible with no setting able to reach it (GH #678).
        //
        // Both signals land here rather than only ICMP, because each reaches addresses the other
        // does not. Our ARP is raw layer-2 injection, so it fails on exactly the paths #678
        // describes while ICMP and mDNS both go through the kernel; and a host can drop echo
        // requests while still advertising its services (macOS stealth mode does precisely that).
        // The browse also has no target list to truncate, so past `arp_scan_cutoff` on a very
        // large subnet it is the only signal still reaching anything.
        //
        // Deliberately last: joining the forwarders first means every ARP reply is already in the
        // channel ahead of these messages, so the receiver claims those addresses first and an
        // address several signals found keeps its MAC. Holding a `host_tx` clone also keeps the
        // channel open until this finishes, so the pipeline cannot decide discovery is over while
        // these are still to come.
        if !interfaced_subnets.is_empty() {
            let host_tx = host_tx.clone();
            let mut icmp = icmp_responders.clone();
            let release_subnets = interfaced_subnets.clone();
            let browsed = mdns_hosts.clone();
            tokio::spawn(async move {
                let responders = icmp.responders().await;

                // mDNS first: where both answered, its evidence is the more specific statement
                // about the host. The two are interchangeable to everything downstream — neither
                // yields a MAC and both skip the responsiveness check — so this only decides
                // which one the logs name.
                let released_addresses: Vec<(IpAddr, LivenessEvidence)> = browsed
                    .keys()
                    .map(|ip| (*ip, LivenessEvidence::Mdns))
                    .chain(responders.iter().map(|ip| (*ip, LivenessEvidence::Icmp)))
                    .collect();

                if released_addresses.is_empty() {
                    return;
                }
                if tokio::task::spawn_blocking(move || {
                    for forwarder in arp_forwarders {
                        let _ = forwarder.join();
                    }
                })
                .await
                .is_err()
                {
                    tracing::warn!(
                        "ARP forwarders could not be joined; not releasing non-ARP responders, \
                         which risks scanning an address twice"
                    );
                    return;
                }

                let mut released = 0u64;
                for (ip, evidence) in released_addresses {
                    // Only addresses the interfaced path swept. The enumerated stream already
                    // dispatched every address in its own range, tagged with these same results,
                    // so anything else here is out of scope by construction.
                    let Some(subnet) = release_subnets.iter().find(|s| s.base.cidr.contains(&ip))
                    else {
                        continue;
                    };
                    if host_tx.send((ip, subnet.clone(), evidence)).await.is_err() {
                        return; // Receiver dropped
                    }
                    released += 1;
                }
                tracing::info!(
                    released,
                    "Queued non-ARP responders for deep scan (ARP-claimed and duplicate \
                     addresses are skipped by the receiver)"
                );
            });
        }

        // Captured before the subnets move into the streaming task below. Used to retire the
        // responsiveness-check budget for addresses the ping sweep answered for, which is seeded
        // per non-interfaced address but only spent by addresses that actually run the check.
        let non_interfaced_subnet_ids: HashSet<Uuid> =
            non_interfaced_subnets.iter().map(|s| s.id).collect();

        // Send all non-interfaced IPs directly to deep scanner (no discovery phase).
        // Key insight: ARP filters to responsive hosts before expensive port scanning.
        // For non-interfaced subnets where ARP isn't possible, just deep scan all IPs
        // directly - we're going to port scan them anyway.
        if !non_interfaced_subnets.is_empty() {
            tracing::info!(
                count = non_interfaced_ip_count,
                "Queuing non-interfaced IPs for deep scan (no ARP available)"
            );

            // Stream IPs directly from CIDR iterators — zero allocation.
            // Each IP is generated on-the-fly and sent through the channel.
            let host_tx = host_tx.clone();
            let stream_targets = target_ips.clone();
            let mut icmp = icmp_responders.clone();
            let browsed = mdns_hosts.clone();
            tokio::spawn(async move {
                // Wait for the ping sweep before streaming, so each address can be tagged with
                // the evidence that exists for it. An address that answered ICMP is already
                // proven alive and skips the TCP responsiveness check; every other address is
                // `Enumerated` and still has to earn its way past it, exactly as before.
                //
                // Only the responder *set* is held in memory — the enumeration itself still
                // streams, so a bogus CIDR cannot exhaust memory here.
                let responders = icmp.responders().await;
                for subnet in non_interfaced_subnets {
                    for addr in subnet.base.cidr.iter().map(|a| a.address()) {
                        if stream_targets.as_ref().is_some_and(|t| !t.contains(&addr)) {
                            continue;
                        }
                        // Both non-ARP signals are consulted here, not just the ping sweep. When
                        // ARP is unavailable outright every subnet takes this path, the daemon's
                        // own segment included — and that is exactly where a browse has something
                        // to say.
                        let evidence = if browsed.contains_key(&addr) {
                            LivenessEvidence::Mdns
                        } else if responders.contains(&addr) {
                            LivenessEvidence::Icmp
                        } else {
                            LivenessEvidence::Enumerated
                        };
                        if host_tx
                            .send((addr, subnet.clone(), evidence))
                            .await
                            .is_err()
                        {
                            return; // Receiver dropped
                        }
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

        // Server-configurable hard ceiling for this run (default = the historical 6h).
        let max_discovery_duration = Duration::from_secs(
            self.scan_settings
                .max_discovery_duration
                .unwrap_or(defaults::max_discovery_duration()) as u64,
        );

        // Batch-level progress tracking for smoother UX
        // TCP port scanning is the bulk of deep scan work
        let is_full_scan = self.scan_settings.is_full_scan;
        let scan_port_count = if is_full_scan {
            65535_usize
        } else {
            self.light_scan_ports.len()
        };
        let batches_per_host = scan_port_count.div_ceil(effective_batch_size);
        let scan_cost_cs = if is_full_scan {
            FULL_SCAN_COST_CS
        } else {
            LIGHT_SCAN_COST_CS
        };
        let total_cost = Arc::new(AtomicUsize::new(0));
        let completed_cost = Arc::new(AtomicUsize::new(0));
        // Seed total_cost with the responsiveness-check work for every non-interfaced IP
        // (checked whether or not it responds), so progress/ETA account for draining that
        // range instead of pinning at ~95% while it's still being scanned.
        total_cost.fetch_add(
            non_interfaced_ip_count as usize * RESPONSIVENESS_COST_CS,
            Ordering::Relaxed,
        );

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
        let mut pending_hosts: Vec<(IpAddr, Subnet, LivenessEvidence)> = Vec::new();

        // Tracks which addresses have already been handed to the deep scanner, so an address
        // both sweeps answered for is scanned once. The channel itself has no dedup — every
        // message it carries spawns a scan — and before ICMP there was exactly one producer per
        // address, so nothing needed one.
        let mut dispatched = DispatchedAddresses::default();

        // Use interval instead of sleep - interval persists across select iterations
        // whereas sleep creates a new future each time and gets dropped when other branches fire
        let mut progress_ticker = tokio::time::interval(Duration::from_secs(1));

        // Helper to calculate phase-weighted progress
        // Note: counters passed by value to avoid borrowing issues in closure
        let calculate_progress = |channel_closed: bool,
                                  has_pending_scans: bool,
                                  grace_elapsed: Duration,
                                  total_cost_val: usize,
                                  completed_cost_val: usize,
                                  hosts_discovered_val: usize,
                                  hosts_scanned_val: usize,
                                  discovery_packets_sent_val: u64,
                                  discovery_packets_total_val: u64|
         -> u8 {
            if !channel_closed {
                // ARP + deep scan run concurrently, so blend both so the bar keeps moving
                // through the ARP tail instead of pinning at 30% then jumping when the
                // channel closes.
                //
                // ARP fraction (0-30%): prefer real packets-sent throughput (self-correcting
                // when the send rate differs from the estimate); fall back to the time
                // estimate before any packet is sent, or on paths that can't report sends
                // (Windows SendARP).
                let arp_frac = if discovery_packets_total_val > 0 && discovery_packets_sent_val > 0
                {
                    (discovery_packets_sent_val as f64 / discovery_packets_total_val as f64)
                        .min(1.0)
                } else if estimated_arp_duration.as_secs() > 0 {
                    (pipeline_start.elapsed().as_secs_f64() / estimated_arp_duration.as_secs_f64())
                        .min(1.0)
                } else {
                    1.0
                };
                // Deep-scan fraction of the work discovered so far. Its denominator grows
                // as ARP keeps finding hosts, so a couple of early completions can spike it
                // (e.g. 2 of an eventual 34 hosts done = 100%). Weighting the deep-scan
                // contribution by `arp_frac` below discounts that until discovery is
                // actually complete, so the bar can't jump ahead then stall.
                let deep_frac = if total_cost_val > 0 {
                    (completed_cost_val as f64 / total_cost_val as f64).min(1.0)
                } else if hosts_discovered_val > 0 {
                    (hosts_scanned_val as f64 / hosts_discovered_val as f64).min(1.0)
                } else {
                    0.0
                };
                // blended = arp_frac * (30 + deep_frac*65). At arp_frac→1 this converges to
                // the post-channel-close formula (30 + deep_frac*65), a seamless handoff.
                let blended = arp_frac
                    * (PROGRESS_ARP_PHASE as f64 + deep_frac * PROGRESS_DEEP_SCAN_PHASE as f64);
                // Cap below the grace band — the channel must close before we reach 95%.
                (blended as u8).min(PROGRESS_ARP_PHASE + PROGRESS_DEEP_SCAN_PHASE - 1)
            } else if total_cost_val > 0
                && (completed_cost_val < total_cost_val || has_pending_scans)
            {
                // Deep scan phase (30-95%): Based on batch completion ratio for smooth progress
                let scan_progress = completed_cost_val as f64 / total_cost_val as f64;
                PROGRESS_ARP_PHASE + (scan_progress * PROGRESS_DEEP_SCAN_PHASE as f64) as u8
            } else if has_pending_scans && total_cost_val == 0 && hosts_discovered_val > 0 {
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
                        Some((ip, subnet, evidence)) => {
                            // One dispatch per address, whichever producer got here first.
                            // ARP arrives during the sweep and claims as it goes; the ping
                            // sweep's own responders are released on channel close below, by
                            // which point every address ARP found is already claimed and keeps
                            // the MAC only ARP carries.
                            if !dispatched.claim(ip) {
                                continue;
                            }

                            // The responsiveness-check budget is seeded up front for every
                            // non-interfaced address, and normally retired inside
                            // deep_scan_host. An address a non-ARP signal already proved alive
                            // skips that check entirely, so retire its share here instead —
                            // otherwise completed_cost can never converge on total_cost and the
                            // scan stalls short of 100%. Keyed on the signal rather than on
                            // `is_confirmed_live`, since an ARP-answered address was never
                            // seeded into this budget in the first place.
                            if matches!(
                                evidence,
                                LivenessEvidence::Icmp | LivenessEvidence::Mdns
                            ) && non_interfaced_subnet_ids.contains(&subnet.id)
                            {
                                completed_cost.fetch_add(RESPONSIVENESS_COST_CS, Ordering::Relaxed);
                            }

                            // Only count hosts something has already answered for.
                            // Addresses with no signal yet are counted after the responsiveness
                            // check passes in deep_scan_host().
                            if evidence.is_confirmed_live() {
                                hosts_discovered.fetch_add(1, Ordering::Relaxed);
                            }
                            *last_activity.lock().unwrap() = Instant::now();

                            // Early-report a minimal host so the UI shows it immediately.
                            // Only for addresses a sweep answered for — they're confirmed live.
                            // Everything else must pass the TCP responsiveness check in
                            // deep_scan_host() first.
                            let mac = evidence.mac();
                            if evidence.is_confirmed_live()
                                && let std::collections::hash_map::Entry::Vacant(e) = early_reported_hosts.entry(ip)
                            {
                                let early_subnet = subnet.clone();
                                let early_cancel = cancel.clone();
                                let early_entity_buffer = ops.entity_buffer.clone();
                                let early_config_store = ops.config_store.clone();
                                let early_api_client = ops.api_client.clone();
                                let early_handle = tokio::spawn(async move {
                                    // Unnamed: the address is an identifier, and the display ladder
                                    // titles the host by it without a copy in `name`.
                                    let host = Host::new(HostBase {
                                        network_id: early_subnet.base.network_id,
                                        source: EntitySource::Discovery,
                                        ..Default::default()
                                    });
                                    let host_id = host.id;
                                    let ip_address = IPAddress::new(IPAddressBase {
                                        network_id: early_subnet.base.network_id,
                                        host_id: Uuid::nil(),
                                        name: None,
                                        subnet_id: early_subnet.id,
                                        ip_address: ip,
                                        // We broadcast for this address and something answered claiming it.
                mac_address: mac
                    .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::ArpReply)),
                                        position: 0,
                                    });
                                    let request = DiscoveryHostRequest {
                                        host,
                                        ip_addresses: vec![ip_address],
                                        ports: vec![],
                                        services: vec![],
                                        interfaces: vec![],
                                        subnets: vec![],
                                        // Early host stub carries no ifTable; nothing to prune against.
                                        interfaces_complete: true,
                                        // ...and no neighbour data either, so there is nothing
                                        // for it to overwrite.
                                        interface_data_complete: InterfaceDataComplete::default(),
                                        // This daemon is this build; it submits the current
                                        // shape by construction.
                                        superseded_wire_shape: false,
                                    };
                                    early_entity_buffer.push_host(request.clone()).await;
                                    let mode = early_config_store.get_mode().await?;
                                    match mode {
                                        DaemonMode::DaemonPoll => {
                                            let _response: HostResponse = early_api_client
                                                .post("/api/v1/hosts/discovery", &request, "Failed to create early host")
                                                .await?;
                                            Ok(host_id)
                                        }
                                        DaemonMode::ServerPoll => {
                                            let _actual = early_entity_buffer
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
                                let completed_cost = completed_cost.clone();
                                let total_cost = total_cost.clone();
                                let hosts_discovered = hosts_discovered.clone();
                                let scan_controller = scan_controller.clone();

                                // Only count batches for addresses a sweep already answered for.
                                // Everything else has batches counted AFTER the responsiveness check.
                                if evidence.is_confirmed_live() {
                                    let integration_cost = self.compute_integration_cost_for_ip(ip);
                                    total_cost.fetch_add(scan_cost_cs + integration_cost, Ordering::Relaxed);
                                }
                                let probe_raw_socket_ports = self.scan_settings.probe_raw_socket_ports;
                                let light_scan_ports = self.light_scan_ports.clone();
                                let network_subnets_ref = network_subnets.clone();

                                let mdns_hosts_ref = mdns_hosts.clone();
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
                                            evidence,
                                            cancel,
                                            scan_rate_pps,
                                            port_scan_batch_size: effective_batch_size,
                                            gateway_ips: &gateway_ips,
                                            completed_cost: Some(&completed_cost),
                                            total_cost: Some(&total_cost),
                                            hosts_discovered: Some(&hosts_discovered),
                                            batches_per_host,
                                            scan_cost_cs,
                                            scan_controller,
                                            probe_raw_socket_ports,
                                            early_host_id,
                                            is_full_scan,
                                            light_scan_ports: &light_scan_ports,

                                            mdns_hosts: mdns_hosts_ref,
                                            credential_mappings: &self.credential_mappings,
                                            known_subnets: network_subnets_ref,
                                        }, ops, utils)
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
                                // Only count batches for addresses a sweep already answered for.
                                // Everything else has batches counted AFTER the responsiveness check.
                                if evidence.is_confirmed_live() {
                                    let integration_cost = self.compute_integration_cost_for_ip(ip);
                                    total_cost.fetch_add(scan_cost_cs + integration_cost, Ordering::Relaxed);
                                }
                                pending_hosts.push((ip, subnet, evidence));
                            }
                        }
                        None => {
                            channel_closed = true;

                            tracing::info!(
                                hosts_discovered = hosts_discovered.load(Ordering::Relaxed),
                                pending_scans = pending_scans.len(),
                                pending_hosts = pending_hosts.len(),
                                total_cost = total_cost.load(Ordering::Relaxed),
                                completed_cost = completed_cost.load(Ordering::Relaxed),
                                elapsed_secs = pipeline_start.elapsed().as_secs(),
                                "Host discovery channel closed, transitioning to deep scan phase"
                            );

                            // ARP complete - recalculate concurrency without ARP FD reservation
                            // Those FDs (2 per subnet) are now available for deep scanning
                            if let Ok(new_concurrency) = utils.get_optimal_deep_scan_concurrency(
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
                    if let Some((ip, subnet, evidence)) = pending_hosts.pop() {
                        let cancel = cancel.clone();
                        let gateway_ips = gateway_ips.clone();
                        let hosts_scanned = hosts_scanned.clone();
                        let last_activity = last_activity.clone();
                        let completed_cost = completed_cost.clone();
                        let total_cost = total_cost.clone();
                        let hosts_discovered = hosts_discovered.clone();
                        let scan_controller = scan_controller.clone();
                        let probe_raw_socket_ports = self.scan_settings.probe_raw_socket_ports;
                        let light_scan_ports = self.light_scan_ports.clone();
                        let network_subnets_ref = network_subnets.clone();

                        let mdns_hosts_ref = mdns_hosts.clone();
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
                                    evidence,
                                    cancel,
                                    scan_rate_pps,
                                    port_scan_batch_size: effective_batch_size,
                                    gateway_ips: &gateway_ips,
                                    completed_cost: Some(&completed_cost),
                                    total_cost: Some(&total_cost),
                                    hosts_discovered: Some(&hosts_discovered),
                                    batches_per_host,
                                    scan_cost_cs,
                                    scan_controller,
                                    probe_raw_socket_ports,
                                    early_host_id,
                                    is_full_scan,
                                    light_scan_ports: &light_scan_ports,

                                    mdns_hosts: mdns_hosts_ref,
                                    credential_mappings: &self.credential_mappings,
                                    known_subnets: network_subnets_ref,
                                }, ops, utils)
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
                    let total_cost_val = total_cost.load(Ordering::Relaxed);
                    let completed_cost_val = completed_cost.load(Ordering::Relaxed);
                    let hosts_discovered_val = hosts_discovered.load(Ordering::Relaxed);
                    let hosts_scanned_val = hosts_scanned.load(Ordering::Relaxed);
                    let discovery_packets_sent_val = discovery_packets_sent.load(Ordering::Relaxed);

                    // Calculate progress. Clamp to be monotonic: the blended ARP+deep-scan
                    // value can dip when ARP discovers more hosts (growing the deep-scan
                    // denominator), and a progress bar must never go backwards.
                    let progress = calculate_progress(
                        channel_closed,
                        has_pending,
                        grace_elapsed,
                        total_cost_val,
                        completed_cost_val,
                        hosts_discovered_val,
                        hosts_scanned_val,
                        discovery_packets_sent_val,
                        discovery_packets_total,
                    )
                    .max(last_progress_report);

                    // Remaining ARP send/retry work — dominates a sparse subnet's tail
                    // (rounds 2-3 re-probe every dead IP). Derived from real send rate so
                    // the ETA stops claiming "<1 min" while ARP still has minutes to go.
                    let arp_remaining_secs = if !channel_closed
                        && discovery_packets_total > 0
                        && discovery_packets_sent_val > 0
                    {
                        let arp_elapsed = pipeline_start.elapsed().as_secs_f64();
                        let rate = discovery_packets_sent_val as f64 / arp_elapsed.max(0.001);
                        let remaining = discovery_packets_total.saturating_sub(discovery_packets_sent_val);
                        (remaining as f64 / rate.max(0.001)) as u32
                    } else {
                        0
                    };

                    // Update estimation atomics on the session
                    if let Ok(session) = ops.get_session().await {
                        session.hosts_discovered.store(hosts_discovered_val as u32, Ordering::Relaxed);

                        if channel_closed && hosts_scanned_val > 0 {
                            let started = deep_scan_started_at.get_or_insert(Instant::now());
                            let deep_scan_elapsed = started.elapsed();
                            // Host-based estimate: real per-host completion time for the
                            // responsive hosts (TCP + endpoints + SNMP + host creation).
                            let time_per_host = deep_scan_elapsed.as_secs_f64() / hosts_scanned_val as f64;
                            let remaining_hosts = hosts_discovered_val.saturating_sub(hosts_scanned_val);
                            let host_based = (remaining_hosts as f64 * time_per_host) as u32;
                            // Cost-based estimate also captures pending non-interfaced
                            // responsiveness work — dead IPs never increment
                            // hosts_discovered, so host-based alone collapses to "<1 min"
                            // while a large non-interfaced range is still draining. Take
                            // the larger so the ETA reflects whichever work dominates.
                            let cost_based = if completed_cost_val > 0 {
                                let time_per_cost_unit =
                                    deep_scan_elapsed.as_secs_f64() / completed_cost_val as f64;
                                let remaining_cost = total_cost_val.saturating_sub(completed_cost_val);
                                (remaining_cost as f64 * time_per_cost_unit) as u32
                            } else {
                                0
                            };
                            let remaining_secs = host_based.max(cost_based)
                                + LATE_ARRIVAL_GRACE_PERIOD.as_secs() as u32;
                            session.estimated_remaining_secs.store(remaining_secs, Ordering::Relaxed);
                        } else {
                            // ARP phase still active. The ETA must cover BOTH the remaining
                            // ARP send/retry work and any concurrent deep-scan cost, taking
                            // whichever dominates — otherwise the near-done deep-scan cost
                            // alone reports "<1 min" while the ARP tail still runs.
                            let deep_remaining = if completed_cost_val > 0 {
                                let started = deep_scan_started_at.get_or_insert(Instant::now());
                                let deep_scan_elapsed = started.elapsed();
                                let time_per_cost_unit = deep_scan_elapsed.as_secs_f64() / completed_cost_val as f64;
                                let remaining_cost = total_cost_val.saturating_sub(completed_cost_val);
                                (remaining_cost as f64 * time_per_cost_unit * 1.2) as u32
                            } else {
                                0
                            };
                            let remaining_secs = deep_remaining.max(arp_remaining_secs)
                                + LATE_ARRIVAL_GRACE_PERIOD.as_secs() as u32;
                            session.estimated_remaining_secs.store(remaining_secs, Ordering::Relaxed);
                        }
                    }

                    // Report progress if it changed OR if enough time has passed (heartbeat)
                    let time_since_last_report = last_progress_time.elapsed();
                    if progress != last_progress_report || time_since_last_report >= MAX_PROGRESS_REPORT_INTERVAL {
                        last_progress_report = progress;
                        last_progress_time = Instant::now();
                        let _ = ops.report_progress(progress.min(99)).await;
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
            if pipeline_start.elapsed() >= max_discovery_duration {
                let discovered = hosts_discovered.load(Ordering::Relaxed);
                let scanned = hosts_scanned.load(Ordering::Relaxed);
                // Hosts that were discovered but never deep-scanned (queued work + the
                // discovered/scanned gap), so the user sees what was left behind.
                let not_scanned = pending_scans
                    .len()
                    .saturating_add(pending_hosts.len())
                    .max(discovered.saturating_sub(scanned));
                tracing::error!(
                    elapsed_secs = pipeline_start.elapsed().as_secs(),
                    hosts_discovered = discovered,
                    hosts_scanned = scanned,
                    pending_scans = pending_scans.len(),
                    pending_hosts = pending_hosts.len(),
                    channel_closed,
                    "Discovery hit global timeout, forcing completion"
                );
                if not_scanned > 0 {
                    // Reuse the estimate already computed this tick to say how much
                    // work was left, without any new estimation plumbing.
                    let remaining = ops
                        .get_session()
                        .await
                        .ok()
                        .map(|s| s.estimated_remaining_secs.load(Ordering::Relaxed))
                        .filter(|&s| s != u32::MAX && s > 0);
                    let hours =
                        u32::try_from(max_discovery_duration.as_secs() / 3600).unwrap_or(u32::MAX);
                    let hosts_not_scanned = u32::try_from(not_scanned).unwrap_or(u32::MAX);
                    // Two codes rather than one with an optional figure: an operator reading
                    // "~40 min remaining" is being told how much to raise the limit by, and a
                    // warning that silently omits that when no estimate exists reads as though
                    // there were none left.
                    let warning = match remaining {
                        Some(secs) => DiscoveryWarning::ScanTimeLimitWithEstimate {
                            hours,
                            hosts_not_scanned,
                            minutes_remaining: secs.div_ceil(60),
                        },
                        None => DiscoveryWarning::ScanTimeLimit {
                            hours,
                            hosts_not_scanned,
                        },
                    };
                    if let Ok(session) = ops.get_session().await
                        && let Ok(mut warnings) = session.warnings.lock()
                    {
                        warnings.push(warning);
                    }
                }
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

        ops.report_progress(100).await?;

        // A credential pinned to an in-scope address that nothing answered at is skipped just as
        // silently as one pinned outside the scanned subnets — the deep scan only ever runs for
        // hosts that responded. Only knowable once the run is over, hence here rather than
        // alongside the pre-scan check.
        let answered: HashSet<IpAddr> = results.iter().map(|(ip, _, _)| *ip).collect();
        let unanswered = unanswered_credential_targets(
            &self.credential_mappings,
            &scanned_subnets,
            target_ips.as_ref(),
            &answered,
        );
        if !unanswered.is_empty()
            && let Ok(session_state) = ops.get_session().await
            && let Ok(mut issues) = session_state.credential_issues.lock()
        {
            issues.extend(unanswered);
        }

        // Addresses that completed a handshake and then answered nothing. Raised per subnet, at the
        // end, because the middlebox that causes it causes it for the whole range at once.
        let declined = self.declined_warnings();
        if !declined.is_empty()
            && let Ok(session_state) = ops.get_session().await
            && let Ok(mut warnings) = session_state.warnings.lock()
        {
            warnings.extend(declined);
        }

        // Collect and submit whatever the DCP sweep found. By now its listen window (a few
        // seconds per interface) has almost always already elapsed against the rest of this
        // pipeline, so this await is ordinarily immediate.
        match dcp_sweep.await {
            Ok(found) => {
                let dcp_count = found.len();
                for response in found {
                    self.submit_dcp_host(session.info.network_id, response, ops, &cancel)
                        .await;
                }
                if dcp_count > 0 {
                    tracing::info!(dcp_count, "PROFINET DCP sweep found devices");
                }
            }
            Err(e) => tracing::warn!(error = %e, "DCP sweep thread failed to report back"),
        }

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

    /// Submit one PROFINET DCP-identified device as a standalone host: a fresh UUID, no IP
    /// addresses (even if the device reported one — DCP's whole marginal value is the no-IP
    /// case; a configured device is already reachable via ARP/ICMP's normal flow), and one
    /// interface carrying the MAC that answered. `create_with_children` matches this against any
    /// host this network already holds carrying that MAC, or mints one if it does not, gated on
    /// `AttributeSource::ProfinetDcp` being a `Native`-tier source — this call never needs to
    /// weaken that gate itself.
    ///
    /// `interfaces_complete: false`: this submission is the sole contributor to itself and never
    /// claims to have seen the device's whole ifTable, so the server must not prune any
    /// SNMP-discovered interfaces already on file for a host this MAC resolves onto.
    async fn submit_dcp_host(
        &self,
        network_id: Uuid,
        response: dcp::DcpIdentifyResponse,
        ops: &DiscoveryOps,
        cancel: &CancellationToken,
    ) {
        let mut host = Host::new(HostBase {
            network_id,
            source: EntitySource::Discovery,
            ..Default::default()
        });
        if let Some(name) = response.name_of_station {
            host.base
                .apply_name(host_name_from_parts(name, AttributeSource::ProfinetDcp));
        }

        let interface = Interface::new(InterfaceBase {
            host_id: Uuid::nil(), // Server assigns.
            network_id,
            // No real value exists — DCP Identify carries no per-port description, and `if_descr`
            // stays `Option` for exactly this producer. Not fabricated text.
            if_descr: None,
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(response.mac),
                AttributeSource::ProfinetDcp,
            )),
            ..Default::default()
        });

        if let Err(e) = ops
            .create_host(
                host,
                vec![],
                vec![],
                vec![],
                vec![interface],
                vec![],
                false,
                InterfaceDataComplete::none(),
                cancel,
            )
            .await
        {
            tracing::warn!(
                mac = %response.mac,
                error = %e,
                "Failed to submit PROFINET DCP-discovered host"
            );
        }
    }

    async fn deep_scan_host(
        &self,
        params: DeepScanParams<'_>,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
    ) -> Result<Option<(IpAddr, Host, DiscoveredHostData)>, Error> {
        let DeepScanParams {
            ip,
            subnet,
            evidence,
            cancel,
            scan_rate_pps,
            port_scan_batch_size,
            gateway_ips,
            completed_cost,
            total_cost,
            hosts_discovered,
            batches_per_host,
            scan_cost_cs,
            scan_controller,
            probe_raw_socket_ports,
            early_host_id,
            is_full_scan,
            light_scan_ports,
            credential_mappings,
            known_subnets,

            mdns_hosts,
        } = params;

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        // Use fixed batch size, limited by scan controller if FD exhaustion has occurred
        let effective_batch_size = port_scan_batch_size.min(scan_controller.batch_size());

        // For addresses no sweep has answered for, check responsiveness first. This avoids full
        // 65k port scans on addresses that aren't online.
        //
        // Keyed on the evidence rather than on the MAC being absent: an ICMP echo reply proves
        // the address is alive without yielding a MAC, and putting it through this check would
        // drop it again for answering no TCP port — undoing the entire point of the ping sweep.
        let mut responsiveness_ports: HashSet<u16> = HashSet::new();
        if !evidence.is_confirmed_live() {
            // Every port the deep scan would look at, so this check can only skip an address
            // nothing we were going to probe answers on. See `liveness_probe_ports`.
            let discovery_ports: Vec<u16> = liveness_probe_ports(light_scan_ports);

            tracing::debug!(
                ip = %ip,
                ports = discovery_ports.len(),
                "Checking responsiveness (no liveness signal yet)"
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

            // The responsiveness check itself is accounted work (seeded into total_cost
            // up front for every non-interfaced IP); mark it complete now, on both the
            // responsive and unresponsive paths.
            if let Some(counter) = completed_cost {
                counter.fetch_add(RESPONSIVENESS_COST_CS, Ordering::Relaxed);
            }

            if responsive_ports.is_empty() {
                tracing::debug!(ip = %ip, "Host unresponsive, skipping deep scan");
                return Ok(None);
            }

            // Host is responsive - NOW we count it in hosts_discovered and total_cost
            // This ensures only responsive hosts contribute to progress calculation
            if let Some(discovered) = hosts_discovered {
                discovered.fetch_add(1, Ordering::Relaxed);
            }
            if let Some(total) = total_cost {
                // Integration cost, counted once per distinct integration for this IP
                // (see integration_cost_for_ip) so it matches the completed-cost accrual.
                let integration_cost_cs = integration_cost_for_ip(credential_mappings, ip);
                total.fetch_add(scan_cost_cs + integration_cost_cs, Ordering::Relaxed);
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

            // Update cost-based progress: each batch contributes a fraction of scan_cost_cs
            if let Some(counter) = completed_cost {
                let cost_per_batch = if batches_per_host > 0 {
                    scan_cost_cs / batches_per_host
                } else {
                    0
                };
                counter.fetch_add(cost_per_batch, Ordering::Relaxed);
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

        // SNMP port probing. Credentialed SNMP is SnmpIntegration.probe()'s below; this still
        // tries the default `public` community, which is the only way an SNMP device is found on a
        // network with no credential configured.
        let snmp_ports = probe_snmp_ports(ip, cancel.clone(), &[]).await?;
        open_ports.extend(snmp_ports);

        // Non-credentialed application probes (DNS, NTP, DHCP, BACnet, Modbus, OPC UA,
        // EtherNet/IP). TCP probes gate on `open_ports` from the connect scan above, so this has
        // to run after it; the identities come back here and are applied further down, after the
        // credentialed integrations have had their say.
        let probe_ctx = ProbeContext {
            ip,
            subnet_cidr: *subnet.base.cidr,
            is_gateway: gateway_ips.contains(&ip),
            cancel: cancel.clone(),
            scan_controller: scan_controller.clone(),
        };
        let app_probe_results =
            scan_app_probes(&probe_ctx, &open_ports, effective_batch_size, scan_rate_pps).await;
        for result in &app_probe_results {
            if !open_ports.contains(&result.port) {
                open_ports.push(result.port);
            }
        }

        // Read once here rather than at the endpoint scan below: integration probes make their
        // own TLS calls and need the same policy.
        let accept_invalid_certs = ops.config_store.get_accept_invalid_scan_certs().await?;

        // Integration probes — each checks connectivity and returns a ClientProbe for service matching
        use crate::daemon::discovery::integration::dispatch;
        let mut probe_results = dispatch::probe_integrations(
            ip,
            credential_mappings,
            &open_ports,
            false, // network scan: keep the probe-gate (cheap broad scan)
            &cancel,
            utils,
            accept_invalid_certs,
        )
        .await?;
        open_ports.extend(probe_results.additional_ports.iter());
        // Hand any IP-targeted credential that produced nothing to the session, which renders
        // them as one line at the end. `probe_integrations` has no session handle of its own.
        ops.record_credential_issues(&probe_results.credential_issues)
            .await;
        // Mark this host's integration cost as completed once its probes resolve. Uses
        // the SAME per-distinct-integration cost that total_cost accrued for the host,
        // so completed_cost converges to total_cost (the scan ETA/progress stay accurate
        // even when several SNMP credentials cover the host).
        if let Some(counter) = completed_cost {
            counter.fetch_add(
                integration_cost_for_ip(credential_mappings, ip),
                Ordering::Relaxed,
            );
        }
        // Fold the application probes into the same evidence map the credentialed integrations
        // fill, so `Pattern::ClientResponse` needs no second channel.
        for result in &app_probe_results {
            if let Some(client_probe) = result.client_probe {
                probe_results
                    .client_responses
                    .entry(client_probe)
                    .or_default()
                    .push(result.port);
            }
        }
        let client_responses = &probe_results.client_responses;

        // Endpoint scanning
        let mut ports_to_check = open_ports.clone();
        let endpoint_only_ports = Service::endpoint_only_ports();
        ports_to_check.extend(endpoint_only_ports);
        ports_to_check.sort_by_key(|p| (p.number(), p.protocol()));
        ports_to_check.dedup();

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

        // An address nothing answered for has to show evidence beyond the bare connect that got it
        // this far. Everything is in hand by now: the probes have spoken, the endpoints have
        // replied, the credentials have authenticated or not.
        //
        // This is the check the FortiGate SIP session-helper report turned on. Before it, a
        // middlebox completing a handshake on behalf of an empty address was enough to record a
        // host there, on every routed VLAN it fronted.
        if !evidence.is_confirmed_live() {
            let mut validated_ports: HashSet<PortType> =
                app_probe_results.iter().map(|result| result.port).collect();
            validated_ports.extend(client_responses.values().flatten().copied());
            validated_ports.extend(
                endpoint_responses
                    .iter()
                    .map(|response| response.endpoint.port_type),
            );

            if !enumerated_host_has_evidence(
                &open_ports,
                &validated_ports,
                self.scan_settings.trust_port_only_detections,
            ) {
                tracing::debug!(
                    ip = %ip,
                    open_ports = ?open_ports.iter().map(|p| p.number()).collect::<Vec<_>>(),
                    "Nothing answered on this enumerated address; recording no host"
                );
                // Recorded rather than warned here: the same reason applies to every address in
                // the range, and the run raises one warning per subnet at the end.
                self.note_declined_address(&subnet.base.cidr, &open_ports);
                return Ok(None);
            }
        }

        open_ports.sort_by_key(|p| (p.number(), p.protocol()));
        open_ports.dedup();

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        tracing::info!(
            ip = %ip,
            open_ports = open_ports.len(),
            endpoints = endpoint_responses.len(),
            "Deep scan complete"
        );

        // DNS hostname lookup (SNMP sysName fallback now handled by SnmpIntegration.execute())
        //
        // That lookup is a unicast PTR query, so it answers only for addresses some DNS server
        // holds a record for. mDNS fills the gap it cannot reach: `.local` is not a unicast zone,
        // and the device answers for itself. Used as a fallback rather than a replacement — where
        // an operator maintains reverse DNS, that is the more deliberate name of the two.
        let dns_sd = mdns_hosts.get(&ip).cloned();
        let hostname = match self.get_hostname_for_ip(ip).await? {
            Some(ptr) => Some(Attributed::new(
                HostHostnameValue(ptr),
                AttributeSource::ReverseDns,
            )),
            None => dns_sd
                .as_ref()
                .and_then(|host| host.hostname.clone())
                .map(|srv| Attributed::new(HostHostnameValue(srv), AttributeSource::DnsSdHostname)),
        };
        // MAC enrichment from SNMP ipAddrTable now handled by SnmpIntegration.execute()
        let ip_address = IPAddress::new(IPAddressBase {
            network_id: subnet.base.network_id,
            host_id: Uuid::nil(), // Placeholder - server will set correct host_id
            name: None,
            subnet_id: subnet.id,
            ip_address: ip,
            // Only ARP yields one; an ICMP-discovered host records no MAC, exactly as a
            // TCP-responsive one on a non-interfaced subnet always has. We broadcast for this
            // address and something answered claiming it, which is the strongest evidence a MAC
            // gets — and what §6's minting rule will ask for.
            mac_address: evidence
                .mac()
                .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::ArpReply)),
            position: 0,
        });

        // Filter raw socket ports from service matching when probe_raw_socket_ports is off,
        // matching the same filtering applied in scan_endpoints() for endpoint probing.
        if !probe_raw_socket_ports {
            open_ports.retain(|p| !p.is_raw_socket());
        }

        // The second layer, and the one the gate above cannot cover. A *real* host on a poisoned
        // subnet passes that gate on its own evidence — and the middlebox is still answering every
        // other port on its behalf, so a connect-only definition would attach its service to a host
        // that has nothing of the sort. Veeam on 9392 and Denodo on 9090 are the remaining cases.
        //
        // Keyed on `reached_on_link` rather than `is_confirmed_live`, which is the distinction the
        // lab caught: 10.77.0.50 is a real host that answers ICMP, so it was confirmed live, and the
        // middlebox answered 9392 on its behalf anyway. It came back carrying a Veeam server it does
        // not run. ICMP proves the host is there; only ARP or mDNS proves we are on its segment and
        // reaching its ports rather than the appliance in front of them.
        if !evidence.reached_on_link() && !self.scan_settings.trust_port_only_detections {
            let before = open_ports.len();
            open_ports.retain(|port| !CONNECT_ONLY_PORTS.contains(port));
            if open_ports.len() != before {
                tracing::debug!(
                    ip = %ip,
                    "Withheld connect-only ports from service matching on an enumerated address"
                );
            }
        }

        if let Ok(Some(mut host_data)) = ops
            .build_host_from_scan(
                ServiceMatchBaselineParams {
                    subnet,
                    ip_address: &ip_address,
                    all_ports: &open_ports,
                    endpoint_responses: &endpoint_responses,
                    virtualization_metadata: &None,
                    virtualization_service_id: None,
                    client_responses,
                    // Directly scanned, not reported by a controller.
                    managed_device: &None,
                    // Present only when the browse reached this address's broadcast domain —
                    // mDNS is link-local, so a routed subnet yields `None` however live the
                    // host is.
                    dns_sd: &dns_sd,
                },
                hostname,
                self.host_naming_fallback,
            )
            .await
        {
            // Reuse the early-reported host ID so the server updates the existing record
            host_data.host.id = early_host_id;

            // Execute integrations whose probe succeeded and service matched
            let execute_params = dispatch::ExecuteParams {
                ip,
                cancel: &cancel,
                ops,
                utils,
                open_ports: &open_ports,
                endpoint_responses: &endpoint_responses,
                host_id: early_host_id,
                host_naming_fallback: self.host_naming_fallback,
                known_subnets: &known_subnets,
                scanning_subnet: Some(subnet),
                ip_address_id: Some(ip_address.id),
            };
            dispatch::execute_integrations(
                credential_mappings,
                &probe_results,
                &mut host_data,
                &execute_params,
            )
            .await
            .ok();

            // Applied last, so a credentialed read of the same field keeps its value — the rule
            // `ControllerIdentity::enrich` already documents. A probe sees only what a discovery
            // packet carries; SNMP and the controllers read the device's own inventory.
            for result in &app_probe_results {
                if let Some(identity) = &result.identity {
                    identity.enrich(&mut host_data);
                }
            }

            ops.record_equal_reach_integrations(ip, &host_data).await;

            // Extract final state from host_data
            let interfaces_complete = host_data.interfaces_complete;
            let interface_data_complete = host_data.interface_data_complete;
            let host = host_data.host;
            let ip_addresses = host_data.ip_addresses;
            let ports = host_data.ports;
            let services = host_data.services;
            let interfaces = host_data.interfaces;
            let subnets = host_data.subnets;

            let services_count = services.len();
            let if_entries_count = interfaces.len();
            let docker_services = services
                .iter()
                .filter(|s| s.base.virtualization_metadata.is_some())
                .count();
            if docker_services > 0 {
                tracing::info!(
                    ip = %ip,
                    total_services = services_count,
                    docker_container_services = docker_services,
                    ip_addresses = ip_addresses.len(),
                    "Creating host with container services from integration"
                );
            }

            match ops
                .create_host(
                    host,
                    ip_addresses,
                    ports,
                    services,
                    interfaces,
                    subnets,
                    interfaces_complete,
                    interface_data_complete,
                    &cancel,
                )
                .await
            {
                Ok(host_response) => {
                    tracing::info!(
                        ip = %ip,
                        services = services_count,
                        interfaces = if_entries_count,
                        "Host created"
                    );
                    let host_data = DiscoveredHostData {
                        docker_service_id: host_response
                            .services
                            .iter()
                            .find(|s| s.base.service_definition.id() == "Docker")
                            .map(|s| s.id),
                        ip_addresses: host_response.ip_addresses.clone(),
                    };
                    return Ok(Some((ip, host_response.to_host(), host_data)));
                }
                Err(e) => {
                    // Include the server error so create rejections are diagnosable
                    // from the daemon log alone (create_host does not retry on
                    // ApiErrorResponse, so the reason otherwise lives only server-side).
                    tracing::warn!(ip = %ip, error = %e, "Host creation failed");
                }
            }
        } else {
            tracing::debug!(ip = %ip, "Host processing returned None");
        }

        Ok(None)
    }
}

/// Number of addresses a scan will actually probe across `subnets`.
///
/// Without a target filter this is the full address space of each CIDR, derived
/// from the prefix so nothing is materialized. With one — a rescan, which
/// substitutes its /32 for the real subnet containing it — the prefix says
/// nothing about the work: counting it would seed the progress budget with a
/// whole subnet's worth of scanning for a single address, pinning the bar near
/// zero and reporting an ETA in minutes for a job that takes seconds.
pub(crate) fn count_scan_ips(subnets: &[Subnet], target_ips: Option<&HashSet<IpAddr>>) -> u64 {
    match target_ips {
        Some(targets) => subnets
            .iter()
            .map(|s| targets.iter().filter(|ip| s.base.cidr.contains(ip)).count() as u64)
            .sum(),
        None => subnets
            .iter()
            .map(|s| 1u64 << (32 - s.base.cidr.network_length() as u64))
            .sum(),
    }
}

/// Credentials pinned to an address that this scan will never visit.
///
/// Only `ip_overrides` are considered: a default credential is network-wide and has no target
/// to be unreachable. Loopback overrides are excluded because they are how a daemon-host
/// credential (a Docker or Podman socket) addresses the daemon's own machine — that address is
/// deliberately absent from the scannable subnets and reporting it would be a false alarm on
/// every scan.
///
/// Deliberately independent of `target_ips`. A targeted rescan narrows to one host on purpose,
/// and flagging every other host's credentials as "not tried" would make the warning noise on
/// the most common operation there is.
pub(crate) fn unreachable_credential_targets(
    mappings: &[CredentialMapping<CredentialQueryPayload>],
    subnets: &[Subnet],
) -> Vec<CredentialIssue> {
    let overrides: Vec<_> = mappings
        .iter()
        .flat_map(|m| m.ip_overrides.iter())
        .collect();
    let in_scope = |ip: &IpAddr| subnets.iter().any(|s| s.base.cidr.contains(ip));

    overrides
        .iter()
        .filter(|o| !o.ip.is_loopback())
        .filter(|o| !in_scope(&o.ip))
        .filter(|o| !sibling_address_qualifies(&overrides, o, &in_scope))
        .map(|o| CredentialIssue {
            integration: (&o.credential).into(),
            ip: o.ip,
            reason: CredentialIssueReason::TargetNotScanned,
            credential_id: (o.credential_id != Uuid::nil()).then_some(o.credential_id),
        })
        .collect()
}

/// Whether another address of the same host, under the same credential, already satisfies the test
/// this override failed.
///
/// A host assignment means "use this credential on this device", and it fans out to one override
/// per address the host holds. A multi-homed device — a wireless AP on a guest range and a
/// management LAN, say — then produces an override the scan cannot use alongside one it can, and
/// reporting the first reads as a misconfigured credential when the device was reached and the
/// credential worked. The advice that comes with it ("add the subnet, or move the credential to a
/// host inside it") is then wrong twice over: the credential is already on the right host.
///
/// Keyed on the host *and* the credential, so two different credentials on one device are still
/// judged separately. An override with no `host_id` — an integration target, the legacy mapping,
/// or any mapping from a server that predates the field — has no siblings and keeps the
/// per-address rule unchanged, which is also what makes a genuinely mis-targeted credential on a
/// single-homed host still report.
fn sibling_address_qualifies(
    overrides: &[&IpOverride<CredentialQueryPayload>],
    subject: &IpOverride<CredentialQueryPayload>,
    qualifies: &impl Fn(&IpAddr) -> bool,
) -> bool {
    let Some(host_id) = subject.host_id else {
        return false;
    };
    overrides.iter().any(|other| {
        other.host_id == Some(host_id)
            && other.credential_id == subject.credential_id
            && other.ip != subject.ip
            && qualifies(&other.ip)
    })
}

impl NetworkScan {
    /// Browse the daemon's own broadcast domains for mDNS/DNS-SD announcements.
    ///
    /// Results are narrowed to the scan's scope for the same reason
    /// [`unreachable_credential_targets`] exists: multicast reaches whatever shares the link,
    /// including addresses on subnets this discovery was never asked to cover, and a rescan
    /// deliberately narrows to specific addresses. Recording the rest would invent hosts the
    /// operator did not ask for.
    async fn browse_mdns(
        &self,
        own_addresses: &[IPAddress],
        scanned_subnets: &[Subnet],
        target_ips: Option<&HashSet<IpAddr>>,
    ) -> HashMap<IpAddr, mdns::DnsSdHost> {
        let interface_addresses: Vec<std::net::Ipv4Addr> = own_addresses
            .iter()
            .filter_map(|address| match address.base.ip_address {
                IpAddr::V4(v4) if !v4.is_loopback() => Some(v4),
                _ => None,
            })
            .collect();

        if interface_addresses.is_empty() {
            return HashMap::new();
        }

        let mut hosts = mdns::browse(&interface_addresses).await;
        hosts.retain(|ip, _| {
            scanned_subnets.iter().any(|s| s.base.cidr.contains(ip))
                && target_ips.is_none_or(|t| t.contains(ip))
        });
        hosts
    }
}

/// A handle to the ICMP sweep, which runs concurrently with ARP.
///
/// The set of responders is published once, when the sweep finishes. Two consumers wait on it —
/// the non-interfaced enumeration, which needs to tag each address it streams, and the pipeline
/// loop, which releases interfaced-subnet responders ARP never claimed. `watch` carries the value
/// to both without either racing the other or the producer.
#[derive(Clone)]
pub(super) struct IcmpSweep {
    rx: tokio::sync::watch::Receiver<Option<Arc<HashSet<IpAddr>>>>,
}

impl IcmpSweep {
    /// A sweep that never ran. Consumers take the same path as they would for one that found
    /// nothing, so there is no "is ICMP on?" branch anywhere downstream.
    fn unavailable() -> Self {
        let (_tx, rx) = tokio::sync::watch::channel(Some(Arc::new(HashSet::new())));
        Self { rx }
    }

    /// The addresses that answered, once the sweep has finished.
    async fn responders(&mut self) -> Arc<HashSet<IpAddr>> {
        match self.rx.wait_for(|v| v.is_some()).await {
            Ok(value) => value.clone().unwrap_or_default(),
            // The producer task died. Degrading to "nothing answered" keeps the scan running with
            // exactly the pre-ICMP behaviour, which is the right failure mode for a signal that is
            // only ever additive.
            Err(_) => Arc::new(HashSet::new()),
        }
    }
}

/// Start the ICMP sweep and drain its (blocking) receiver on a worker thread.
fn spawn_icmp_sweep(
    targets: Vec<std::net::Ipv4Addr>,
    retries: u32,
    rate_pps: u32,
    packets_sent: &Arc<AtomicU64>,
) -> IcmpSweep {
    let (tx, rx) = tokio::sync::watch::channel(None);
    let packets_sent = packets_sent.clone();

    tokio::task::spawn_blocking(move || {
        let mut responders: HashSet<IpAddr> = HashSet::new();
        match icmp::sweep(targets, retries, rate_pps, packets_sent) {
            Ok(results) => {
                // The sender closes the channel when both sweep threads finish, so this drains
                // for exactly as long as the sweep runs.
                for result in results {
                    responders.insert(IpAddr::V4(result.ip));
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "ICMP sweep failed to start; continuing without it");
            }
        }
        tracing::info!(responders = responders.len(), "ICMP echo sweep finished");
        let _ = tx.send(Some(Arc::new(responders)));
    });

    IcmpSweep { rx }
}

/// Credentials pinned to an in-scope address that no host answered at.
///
/// The complement of [`unreachable_credential_targets`]: that one catches an address the scan
/// never looked at, this one an address it looked at and found nobody home. Both end with the
/// credential untried and nothing said about it, but they have different fixes, so they stay
/// separate reasons.
///
/// `answered` is the set of addresses that produced a scanned host. Addresses narrowed out by a
/// targeted rescan are excluded for the same reason as in the pre-scan check: skipping them is
/// the point of a rescan, not a fault.
pub(crate) fn unanswered_credential_targets(
    mappings: &[CredentialMapping<CredentialQueryPayload>],
    subnets: &[Subnet],
    target_ips: Option<&HashSet<IpAddr>>,
    answered: &HashSet<IpAddr>,
) -> Vec<CredentialIssue> {
    let overrides: Vec<_> = mappings
        .iter()
        .flat_map(|m| m.ip_overrides.iter())
        .collect();

    overrides
        .iter()
        .filter(|o| !o.ip.is_loopback())
        .filter(|o| subnets.iter().any(|s| s.base.cidr.contains(&o.ip)))
        .filter(|o| target_ips.is_none_or(|t| t.contains(&o.ip)))
        .filter(|o| !answered.contains(&o.ip))
        // The same rule as the pre-scan check, against the addresses that answered rather than
        // the ones in scope: a device that answered on one of its addresses was reached, and its
        // silent second address is not an untried credential.
        .filter(|o| !sibling_address_qualifies(&overrides, o, &|ip| answered.contains(ip)))
        .map(|o| CredentialIssue {
            integration: (&o.credential).into(),
            ip: o.ip,
            reason: CredentialIssueReason::TargetNotResponding,
            credential_id: (o.credential_id != Uuid::nil()).then_some(o.credential_id),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::subnets::r#impl::base::SubnetBase;
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
    use crate::server::subnets::r#impl::types::SubnetType;
    use std::str::FromStr;

    fn subnet(cidr: &str) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr::IpCidr::from_str(cidr).unwrap()),
                AttributeSource::DaemonSelfReport,
            ),
            network_id: uuid::Uuid::new_v4(),
            name: cidr.to_string(),
            description: None,
            subnet_type: SubnetType::Lan,
            virtualization_service_id: None,
            source: crate::server::shared::types::entities::EntitySource::System,
            tags: Vec::new(),
        })
    }

    fn ip(s: &str) -> IpAddr {
        IpAddr::from_str(s).unwrap()
    }

    #[test]
    fn unfiltered_count_is_the_whole_address_space() {
        let subnets = [subnet("10.0.5.0/24"), subnet("10.0.6.0/30")];
        assert_eq!(count_scan_ips(&subnets, None), 256 + 4);
    }

    #[test]
    fn a_rescan_costs_one_address_not_the_subnet_it_sits_in() {
        // The substituted subnet is the real /24, so an unfiltered count would
        // budget 256 addresses of work for a single-host rescan.
        let subnets = [subnet("10.0.5.0/24")];
        let targets = HashSet::from([ip("10.0.5.7")]);
        assert_eq!(count_scan_ips(&subnets, Some(&targets)), 1);
    }

    #[test]
    fn targets_outside_the_scanned_subnets_are_not_counted() {
        let subnets = [subnet("10.0.5.0/24")];
        let targets = HashSet::from([ip("10.0.5.7"), ip("192.168.1.9")]);
        assert_eq!(count_scan_ips(&subnets, Some(&targets)), 1);
    }

    #[test]
    fn multiple_targets_in_one_subnet_each_count() {
        // A host with several addresses on the same NIC mints one /32 per
        // address; all substitute to the same parent subnet.
        let subnets = [subnet("10.0.5.0/24")];
        let targets = HashSet::from([ip("10.0.5.7"), ip("10.0.5.8")]);
        assert_eq!(count_scan_ips(&subnets, Some(&targets)), 2);
    }

    fn mapping_targeting(addr: &str) -> CredentialMapping<CredentialQueryPayload> {
        use crate::server::credentials::r#impl::mapping::IpOverride;
        CredentialMapping {
            ip_overrides: vec![IpOverride {
                ip: ip(addr),
                credential: CredentialQueryPayload::default(), // Snmp
                credential_id: Uuid::new_v4(),
                host_id: None,
            }],
            ..Default::default()
        }
    }

    /// The reported failure mode: a credential assigned to a controller on a network the
    /// discovery does not cover is never tried, and said nothing about it.
    #[test]
    fn a_target_outside_every_scanned_subnet_is_reported() {
        let subnets = [subnet("10.0.5.0/24")];
        let mappings = [mapping_targeting("192.168.1.9")];

        let issues = unreachable_credential_targets(&mappings, &subnets);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].ip, ip("192.168.1.9"));
        assert_eq!(issues[0].reason, CredentialIssueReason::TargetNotScanned);
    }

    #[test]
    fn a_target_inside_a_scanned_subnet_is_not_reported() {
        let subnets = [subnet("10.0.5.0/24")];
        let mappings = [mapping_targeting("10.0.5.7")];
        assert!(unreachable_credential_targets(&mappings, &subnets).is_empty());
    }

    /// A Docker/Podman socket credential addresses the daemon's own machine over loopback,
    /// which is deliberately not a scannable subnet. Reporting it would fire on every scan.
    #[test]
    fn a_loopback_target_is_never_reported() {
        let subnets = [subnet("10.0.5.0/24")];
        let mappings = [mapping_targeting("127.0.0.1")];
        assert!(unreachable_credential_targets(&mappings, &subnets).is_empty());
    }

    /// The case a real scan exposed: the address sits inside a scanned subnet, so the pre-scan
    /// check passes it, but nothing answers there and the deep scan never runs — leaving the
    /// credential untried and, before this, entirely silent.
    #[test]
    fn an_in_scope_address_nobody_answered_at_is_reported() {
        let subnets = [subnet("192.168.4.0/22")];
        let mappings = [mapping_targeting("192.168.4.141")];
        let answered = HashSet::from([ip("192.168.4.196")]);

        assert!(
            unreachable_credential_targets(&mappings, &subnets).is_empty(),
            "the address is inside a scanned subnet, so it is not 'not scanned'"
        );

        let issues = unanswered_credential_targets(&mappings, &subnets, None, &answered);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].ip, ip("192.168.4.141"));
        assert_eq!(issues[0].reason, CredentialIssueReason::TargetNotResponding);
    }

    #[test]
    fn an_address_that_answered_is_not_reported() {
        let subnets = [subnet("192.168.4.0/22")];
        let mappings = [mapping_targeting("192.168.4.196")];
        let answered = HashSet::from([ip("192.168.4.196")]);
        assert!(unanswered_credential_targets(&mappings, &subnets, None, &answered).is_empty());
    }

    /// A rescan narrows to specific addresses on purpose. Reporting every other pinned
    /// credential as unanswered would make the warning noise on the most common operation.
    #[test]
    fn a_rescan_does_not_report_the_hosts_it_deliberately_skipped() {
        let subnets = [subnet("192.168.4.0/22")];
        let mappings = [mapping_targeting("192.168.4.141")];
        let targets = HashSet::from([ip("192.168.4.196")]);
        let answered = HashSet::from([ip("192.168.4.196")]);
        assert!(
            unanswered_credential_targets(&mappings, &subnets, Some(&targets), &answered)
                .is_empty()
        );
    }

    /// A network default has no target, so it can never be unreachable — only pinned
    /// `ip_overrides` can.
    #[test]
    fn a_network_default_is_not_reported() {
        let subnets = [subnet("10.0.5.0/24")];
        let mappings = [CredentialMapping {
            default_credential: Some(CredentialQueryPayload::default()),
            ..Default::default()
        }];
        assert!(unreachable_credential_targets(&mappings, &subnets).is_empty());
    }

    /// One credential assigned to a multi-homed host, expanded to one override per address.
    fn multi_homed(
        cred: Uuid,
        host: Uuid,
        addrs: &[&str],
    ) -> CredentialMapping<CredentialQueryPayload> {
        use crate::server::credentials::r#impl::mapping::IpOverride;
        CredentialMapping {
            ip_overrides: addrs
                .iter()
                .map(|a| IpOverride {
                    ip: ip(a),
                    credential: CredentialQueryPayload::default(), // Snmp
                    credential_id: cred,
                    host_id: Some(host),
                })
                .collect(),
            ..Default::default()
        }
    }

    /// The shape a live scan produced: a wireless AP on a guest range and a management LAN, with
    /// one credential assigned to the device. The scan covers the LAN, reaches the AP and uses the
    /// credential successfully — and the guest address, which no scanned subnet holds, was then
    /// reported as a credential that was never contacted. The advice was wrong twice over: the
    /// credential is already on the right host, and it already worked.
    #[test]
    fn a_reached_hosts_out_of_scope_address_is_not_reported() {
        let subnets = [subnet("192.168.4.0/22")];
        let mappings = [multi_homed(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &["172.30.10.1", "192.168.7.235"],
        )];

        assert!(
            unreachable_credential_targets(&mappings, &subnets).is_empty(),
            "the device was reached at its LAN address; its guest address is not an untried credential"
        );
    }

    /// The other side of the same rule, and the thing it must not swallow: a credential pinned to
    /// a host the scan genuinely cannot reach has no sibling to vouch for it and still reports.
    #[test]
    fn a_host_with_no_in_scope_address_is_still_reported() {
        let subnets = [subnet("192.168.4.0/22")];
        let mappings = [multi_homed(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &["172.30.10.1", "172.30.10.2"],
        )];

        assert_eq!(unreachable_credential_targets(&mappings, &subnets).len(), 2);
    }

    /// Two addresses that merely share a scan are not siblings. Without the host id — an
    /// integration target, the legacy mapping, or any mapping from a server that predates the
    /// field — the per-address rule stands exactly as it did.
    #[test]
    fn addresses_of_different_hosts_do_not_vouch_for_each_other() {
        let subnets = [subnet("192.168.4.0/22")];
        let unrelated = [
            mapping_targeting("172.30.10.1"),
            mapping_targeting("192.168.7.235"),
        ];
        assert_eq!(
            unreachable_credential_targets(&unrelated, &subnets).len(),
            1,
            "no host id means no siblings, so the out-of-scope address reports as it always did"
        );

        // Same two addresses, same host, but two different credentials: each is judged on its own.
        let host = Uuid::new_v4();
        let two_creds = [
            multi_homed(Uuid::new_v4(), host, &["172.30.10.1"]),
            multi_homed(Uuid::new_v4(), host, &["192.168.7.235"]),
        ];
        assert_eq!(
            unreachable_credential_targets(&two_creds, &subnets).len(),
            1,
            "a second credential reaching the host says nothing about whether the first was tried"
        );
    }

    /// The post-scan half, against the addresses that answered rather than the ones in scope: a
    /// device that answered on one address was reached, and its silent second address is not an
    /// untried credential.
    #[test]
    fn a_hosts_silent_address_is_not_reported_when_another_answered() {
        let subnets = [subnet("192.168.4.0/22")];
        let host = Uuid::new_v4();
        let mappings = [multi_homed(
            Uuid::new_v4(),
            host,
            &["192.168.4.10", "192.168.4.11"],
        )];

        let answered = HashSet::from([ip("192.168.4.10")]);
        assert!(unanswered_credential_targets(&mappings, &subnets, None, &answered).is_empty());

        // Nothing answered anywhere on the host, so both addresses are still reported.
        let silent = HashSet::new();
        assert_eq!(
            unanswered_credential_targets(&mappings, &subnets, None, &silent).len(),
            2
        );
    }
}
