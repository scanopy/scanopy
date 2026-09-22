use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use mac_address::MacAddress;
use pnet::datalink::{self, Channel, NetworkInterface};
use pnet::packet::Packet;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, ArpPacket, MutableArpPacket};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::util::MacAddr;
use tokio_util::sync::CancellationToken;

use super::types::ArpScanResult;

/// Wait time after each round before retrying non-responders
pub const ROUND_WAIT: Duration = Duration::from_secs(3);
/// Extra receive time after final round
pub const POST_SCAN_RECEIVE: Duration = Duration::from_secs(5);
/// Hard max lifetime for the receiver thread (defense-in-depth)
const MAX_RECEIVER_LIFETIME: Duration = Duration::from_secs(300);

/// Check if broadcast ARP is available (requires raw socket capability)
pub fn is_available() -> bool {
    let interfaces = datalink::interfaces();

    tracing::debug!(
        interface_count = interfaces.len(),
        "Checking ARP availability: enumerating interfaces"
    );

    // Log why each interface is rejected
    // Note: on Windows, pnet hardcodes flags=0 so is_up() always returns false.
    // We skip the is_up() check there and rely on datalink::channel() as the real test.
    for iface in &interfaces {
        if iface.is_loopback() {
            tracing::trace!(name = %iface.name, "Skipping loopback interface");
        } else if !cfg!(target_family = "windows") && !iface.is_up() {
            tracing::trace!(name = %iface.name, "Skipping interface: not up");
        } else if iface.mac.is_none() {
            tracing::trace!(name = %iface.name, "Skipping interface: no MAC address");
        }
    }

    let suitable = interfaces.into_iter().find(|iface| {
        #[cfg(target_family = "windows")]
        let up_check = true; // pnet flags are broken on Windows
        #[cfg(not(target_family = "windows"))]
        let up_check = iface.is_up();

        up_check && !iface.is_loopback() && iface.mac.is_some()
    });

    let Some(interface) = suitable else {
        tracing::warn!(
            "ARP scanning unavailable: no suitable network interface found. \
             Ensure container has a non-loopback interface with a MAC address."
        );
        return false;
    };

    tracing::debug!(
        interface = %interface.name,
        mac = ?interface.mac,
        "Found suitable interface for ARP, testing raw socket capability"
    );

    let config = pnet::datalink::Config {
        read_timeout: Some(Duration::from_millis(100)),
        ..Default::default()
    };

    match datalink::channel(&interface, config) {
        Ok(_) => {
            tracing::debug!(
                interface = %interface.name,
                "ARP scanning available: raw socket channel created successfully"
            );
            true
        }
        Err(e) => {
            tracing::warn!(
                interface = %interface.name,
                error = %e,
                "ARP scanning unavailable: failed to create raw socket channel. \
                 Ensure container has NET_RAW and NET_ADMIN capabilities. \
                 Error typically indicates missing privileges for raw packet access."
            );
            false
        }
    }
}

/// Scan subnet using broadcast ARP with targeted retries.
/// Returns a channel receiver that provides results as they arrive.
///
/// # Arguments
/// * `interface` - Network interface to scan on
/// * `source_ip` - Source IP for ARP requests
/// * `source_mac` - Source MAC for ARP requests
/// * `targets` - List of IPs to scan
/// * `retries` - Number of retry rounds for non-responding hosts (0 = single attempt)
/// * `rate_pps` - Maximum packets per second (rate limiting for switch compatibility)
/// * `packets_sent` - Shared counter incremented per ARP request sent, so callers can
///   drive work-based progress/ETA off real send throughput rather than a rate estimate.
/// * `cancel` - Stops the send rounds; the sender still marks sending done and joins.
#[allow(clippy::too_many_arguments)]
pub fn scan_subnet(
    interface: &NetworkInterface,
    source_ip: Ipv4Addr,
    source_mac: MacAddress,
    targets: Vec<Ipv4Addr>,
    retries: u32,
    rate_pps: u32,
    packets_sent: Arc<AtomicU64>,
    cancel: CancellationToken,
) -> Result<std::sync::mpsc::Receiver<ArpScanResult>> {
    use std::sync::mpsc;

    let interface = interface.clone();
    let target_set: HashSet<Ipv4Addr> = targets.iter().copied().collect();

    let (tx, rx) = mpsc::channel();

    // Spawn background thread for the entire ARP scan
    std::thread::spawn(move || {
        if let Err(e) = scan_subnet_background(
            &interface,
            source_ip,
            source_mac,
            target_set,
            retries,
            rate_pps,
            packets_sent,
            tx,
            cancel,
        ) {
            tracing::warn!(error = %e, "ARP scan background thread failed");
        }
    });

    Ok(rx)
}

#[allow(clippy::too_many_arguments)]
fn scan_subnet_background(
    interface: &NetworkInterface,
    source_ip: Ipv4Addr,
    source_mac: MacAddress,
    targets: HashSet<Ipv4Addr>,
    retries: u32,
    rate_pps: u32,
    packets_sent: Arc<AtomicU64>,
    result_tx: std::sync::mpsc::Sender<ArpScanResult>,
    cancel: CancellationToken,
) -> Result<()> {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    // Calculate send delay from rate limit (pps -> microseconds between packets)
    // rate_pps of 50 = 20ms between packets, rate_pps of 1000 = 1ms
    let send_delay = Duration::from_micros(1_000_000 / rate_pps.max(1) as u64);

    // Use a larger buffer to avoid dropping packets
    let config = pnet::datalink::Config {
        read_timeout: Some(Duration::from_millis(50)),
        read_buffer_size: 65536,
        write_buffer_size: 65536,
        ..Default::default()
    };

    let (mut tx, mut rx) = match datalink::channel(interface, config)? {
        Channel::Ethernet(tx, rx) => (tx, rx),
        _ => return Err(anyhow!("Unsupported channel type")),
    };

    let source_mac_pnet = MacAddr::new(
        source_mac.bytes()[0],
        source_mac.bytes()[1],
        source_mac.bytes()[2],
        source_mac.bytes()[3],
        source_mac.bytes()[4],
        source_mac.bytes()[5],
    );

    let total_rounds = 1 + retries;
    let target_count = targets.len();

    tracing::debug!(
        interface = %interface.name,
        source_ip = %source_ip,
        targets = target_count,
        total_rounds,
        rate_pps,
        send_delay_ms = send_delay.as_millis(),
        "Starting ARP scan with concurrent send/receive"
    );

    // Shared state between sender and receiver threads
    let found_ips = Arc::new(Mutex::new(HashSet::<Ipv4Addr>::new()));
    let sending_done = Arc::new(AtomicBool::new(false));
    let current_round = Arc::new(AtomicU32::new(1));

    // Stats for logging
    let total_packets_received = Arc::new(AtomicU32::new(0));
    let total_arp_replies = Arc::new(AtomicU32::new(0));

    let targets_clone = targets.clone();
    let found_ips_recv = found_ips.clone();
    let sending_done_recv = sending_done.clone();
    let total_packets_received_clone = total_packets_received.clone();
    let total_arp_replies_clone = total_arp_replies.clone();
    let current_round_recv = current_round.clone();

    // Receiver thread - runs continuously while sending is in progress
    let receiver_handle = thread::spawn(move || {
        let our_mac = source_mac_pnet;
        let start = Instant::now();

        loop {
            match rx.next() {
                Ok(packet) => {
                    total_packets_received_clone.fetch_add(1, Ordering::Relaxed);

                    if let Some(ethernet) = EthernetPacket::new(packet) {
                        // Skip our own outgoing packets
                        if ethernet.get_source() == our_mac {
                            continue;
                        }

                        if ethernet.get_ethertype() == EtherTypes::Arp
                            && let Some(arp) = ArpPacket::new(ethernet.payload())
                            && arp.get_operation() == ArpOperations::Reply
                        {
                            total_arp_replies_clone.fetch_add(1, Ordering::Relaxed);
                            let sender_ip = arp.get_sender_proto_addr();

                            if targets_clone.contains(&sender_ip) {
                                let mut found = found_ips_recv.lock().unwrap();
                                if !found.contains(&sender_ip) {
                                    found.insert(sender_ip);
                                    let mac = MacAddress::new(arp.get_sender_hw_addr().octets());
                                    let round = current_round_recv.load(Ordering::Relaxed);

                                    tracing::debug!(
                                        round,
                                        ip = %sender_ip,
                                        mac = %mac,
                                        "ARP: Host discovered"
                                    );

                                    let _ = result_tx.send(ArpScanResult { ip: sender_ip, mac });
                                }
                            }
                        }
                    }

                    // Check stop condition after processing - on bridge ip_addresses,
                    // the Err/timeout path may never fire because LAN traffic
                    // arrives continuously via the raw socket
                    if sending_done_recv.load(Ordering::Relaxed) {
                        break;
                    }

                    // Defense-in-depth: hard max lifetime prevents infinite hangs
                    if start.elapsed() >= MAX_RECEIVER_LIFETIME {
                        tracing::warn!(
                            elapsed_secs = start.elapsed().as_secs(),
                            "ARP receiver hit max lifetime, forcing exit"
                        );
                        break;
                    }
                }
                Err(_) => {
                    // Timeout - check if we should stop
                    if sending_done_recv.load(Ordering::Relaxed) {
                        // Sender is done, do final receive period
                        break;
                    }

                    // Defense-in-depth: hard max lifetime
                    if start.elapsed() >= MAX_RECEIVER_LIFETIME {
                        tracing::warn!(
                            elapsed_secs = start.elapsed().as_secs(),
                            "ARP receiver hit max lifetime during timeout, forcing exit"
                        );
                        break;
                    }
                }
            }
        }

        // Final receive period to catch stragglers
        tracing::debug!(
            post_scan_secs = POST_SCAN_RECEIVE.as_secs(),
            "Final receive period"
        );
        let final_deadline = Instant::now() + POST_SCAN_RECEIVE;

        while Instant::now() < final_deadline {
            match rx.next() {
                Ok(packet) => {
                    total_packets_received_clone.fetch_add(1, Ordering::Relaxed);

                    if let Some(ethernet) = EthernetPacket::new(packet) {
                        if ethernet.get_source() == our_mac {
                            continue;
                        }

                        if ethernet.get_ethertype() == EtherTypes::Arp
                            && let Some(arp) = ArpPacket::new(ethernet.payload())
                            && arp.get_operation() == ArpOperations::Reply
                        {
                            total_arp_replies_clone.fetch_add(1, Ordering::Relaxed);
                            let sender_ip = arp.get_sender_proto_addr();

                            if targets_clone.contains(&sender_ip) {
                                let mut found = found_ips_recv.lock().unwrap();
                                if !found.contains(&sender_ip) {
                                    found.insert(sender_ip);
                                    let mac = MacAddress::new(arp.get_sender_hw_addr().octets());

                                    tracing::debug!(
                                        ip = %sender_ip,
                                        mac = %mac,
                                        "ARP: Late host discovered"
                                    );

                                    let _ = result_tx.send(ArpScanResult { ip: sender_ip, mac });
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(1));
                }
            }
        }

        let packets = total_packets_received_clone.load(Ordering::Relaxed);
        let replies = total_arp_replies_clone.load(Ordering::Relaxed);
        let found = found_ips_recv.lock().unwrap().len();

        tracing::debug!(
            elapsed_secs = start.elapsed().as_secs(),
            total_packets_received = packets,
            total_arp_replies = replies,
            hosts_found = found,
            hosts_missed = target_count - found,
            "ARP scan completed"
        );
    });

    // Sender thread - sends packets with rate limiting
    thread::spawn(move || {
        let start = Instant::now();

        // Process each round. A cancelled discovery stops sending, and only sending: the loop
        // still falls through to `sending_done` and the receiver join below, which are what close
        // the result channel. Skipping them would leave the receiver holding it until its
        // lifetime cap, and the pipeline waiting on it.
        for round in 1..=total_rounds {
            if cancel.is_cancelled() {
                tracing::debug!(round, "ARP scan cancelled; no further requests sent");
                break;
            }
            current_round.store(round, Ordering::Relaxed);

            // Determine which IPs to scan this round (exclude already found)
            let round_targets: Vec<Ipv4Addr> = {
                let found = found_ips.lock().unwrap();
                targets
                    .iter()
                    .filter(|ip| !found.contains(ip))
                    .copied()
                    .collect()
            };

            if round_targets.is_empty() {
                tracing::debug!(
                    round,
                    total_rounds,
                    "All targets found, skipping remaining rounds"
                );
                break;
            }

            let found_before = found_ips.lock().unwrap().len();
            tracing::debug!(
                round,
                total_rounds,
                targets_this_round = round_targets.len(),
                found_so_far = found_before,
                "Starting ARP round"
            );

            // Send ARP requests for this round
            let mut sent_ok = 0u64;
            let mut sent_err = 0u64;

            for target_ip in &round_targets {
                if cancel.is_cancelled() {
                    break;
                }
                let packet = build_arp_request(source_mac_pnet, source_ip, *target_ip);
                match tx.send_to(&packet, None) {
                    Some(Ok(())) => sent_ok += 1,
                    Some(Err(e)) => {
                        sent_err += 1;
                        if sent_err <= 3 {
                            tracing::warn!(target = %target_ip, error = %e, "Failed to send ARP request");
                        }
                    }
                    None => sent_err += 1,
                }
                // Count every attempt (ok or err) — this tracks progress through the
                // send schedule, which is what the ETA/progress bar is pacing against.
                packets_sent.fetch_add(1, Ordering::Relaxed);
                thread::sleep(send_delay);
            }

            tracing::debug!(round, sent_ok, sent_err, "ARP round send complete");
            if cancel.is_cancelled() {
                continue;
            }

            // Wait for responses before next round (targeted retry needs to know who responded)
            thread::sleep(ROUND_WAIT);

            let found_after = found_ips.lock().unwrap().len();
            let found_this_round = found_after - found_before;
            tracing::debug!(
                round,
                found_this_round,
                total_found = found_after,
                remaining = target_count - found_after,
                "ARP round complete"
            );
        }

        // Signal receiver that sending is done
        sending_done.store(true, Ordering::Relaxed);

        tracing::debug!(
            elapsed_secs = start.elapsed().as_secs(),
            "ARP sending complete, waiting for receiver"
        );

        // Wait for receiver to finish
        let _ = receiver_handle.join();
    });

    Ok(())
}

fn build_arp_request(source_mac: MacAddr, source_ip: Ipv4Addr, target_ip: Ipv4Addr) -> Vec<u8> {
    let mut ethernet_buffer = vec![0u8; 42]; // 14 (eth) + 28 (arp)
    let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer).unwrap();

    ethernet_packet.set_destination(MacAddr::broadcast());
    ethernet_packet.set_source(source_mac);
    ethernet_packet.set_ethertype(EtherTypes::Arp);

    let mut arp_buffer = vec![0u8; 28];
    let mut arp_packet = MutableArpPacket::new(&mut arp_buffer).unwrap();

    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Request);
    arp_packet.set_sender_hw_addr(source_mac);
    arp_packet.set_sender_proto_addr(source_ip);
    arp_packet.set_target_hw_addr(MacAddr::zero());
    arp_packet.set_target_proto_addr(target_ip);

    ethernet_packet.set_payload(arp_packet.packet());
    ethernet_buffer
}

#[cfg(test)]
fn parse_arp_reply(packet: &[u8]) -> Option<(Ipv4Addr, MacAddress)> {
    let ethernet = EthernetPacket::new(packet)?;
    if ethernet.get_ethertype() != EtherTypes::Arp {
        return None;
    }

    let arp = ArpPacket::new(ethernet.payload())?;
    if arp.get_operation() != ArpOperations::Reply {
        return None;
    }

    let sender_ip = arp.get_sender_proto_addr();
    let sender_mac = MacAddress::new(arp.get_sender_hw_addr().octets());

    Some((sender_ip, sender_mac))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_arp_request_creates_valid_packet() {
        let source_mac = MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55);
        let source_ip = Ipv4Addr::new(192, 168, 1, 100);
        let target_ip = Ipv4Addr::new(192, 168, 1, 1);

        let packet = build_arp_request(source_mac, source_ip, target_ip);

        // Verify packet size
        assert_eq!(packet.len(), 42);

        // Parse and verify ethernet header
        let eth = EthernetPacket::new(&packet).unwrap();
        assert_eq!(eth.get_destination(), MacAddr::broadcast());
        assert_eq!(eth.get_source(), source_mac);
        assert_eq!(eth.get_ethertype(), EtherTypes::Arp);

        // Parse and verify ARP packet
        let arp = ArpPacket::new(eth.payload()).unwrap();
        assert_eq!(arp.get_hardware_type(), ArpHardwareTypes::Ethernet);
        assert_eq!(arp.get_protocol_type(), EtherTypes::Ipv4);
        assert_eq!(arp.get_operation(), ArpOperations::Request);
        assert_eq!(arp.get_sender_hw_addr(), source_mac);
        assert_eq!(arp.get_sender_proto_addr(), source_ip);
        assert_eq!(arp.get_target_proto_addr(), target_ip);
    }

    #[test]
    fn test_parse_arp_reply_extracts_sender_info() {
        // Build a mock ARP reply packet
        let mut packet = vec![0u8; 42];

        // Ethernet header
        let mut eth = MutableEthernetPacket::new(&mut packet).unwrap();
        eth.set_destination(MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55));
        eth.set_source(MacAddr::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF));
        eth.set_ethertype(EtherTypes::Arp);

        // ARP payload
        let mut arp_buffer = vec![0u8; 28];
        {
            let mut arp = MutableArpPacket::new(&mut arp_buffer).unwrap();
            arp.set_hardware_type(ArpHardwareTypes::Ethernet);
            arp.set_protocol_type(EtherTypes::Ipv4);
            arp.set_hw_addr_len(6);
            arp.set_proto_addr_len(4);
            arp.set_operation(ArpOperations::Reply);
            arp.set_sender_hw_addr(MacAddr::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF));
            arp.set_sender_proto_addr(Ipv4Addr::new(192, 168, 1, 1));
            arp.set_target_hw_addr(MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55));
            arp.set_target_proto_addr(Ipv4Addr::new(192, 168, 1, 100));
        }
        eth.set_payload(&arp_buffer);

        let result = parse_arp_reply(&packet);
        assert!(result.is_some());

        let (ip, mac) = result.unwrap();
        assert_eq!(ip, Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(mac.bytes(), [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    }

    #[test]
    fn test_parse_arp_reply_rejects_non_arp() {
        // Build a non-ARP ethernet packet
        let mut packet = vec![0u8; 42];
        let mut eth = MutableEthernetPacket::new(&mut packet).unwrap();
        eth.set_ethertype(EtherTypes::Ipv4); // Not ARP

        let result = parse_arp_reply(&packet);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_arp_reply_rejects_arp_request() {
        // Build an ARP request (not reply)
        let mut packet = vec![0u8; 42];
        let mut eth = MutableEthernetPacket::new(&mut packet).unwrap();
        eth.set_ethertype(EtherTypes::Arp);

        let mut arp_buffer = vec![0u8; 28];
        {
            let mut arp = MutableArpPacket::new(&mut arp_buffer).unwrap();
            arp.set_hardware_type(ArpHardwareTypes::Ethernet);
            arp.set_protocol_type(EtherTypes::Ipv4);
            arp.set_hw_addr_len(6);
            arp.set_proto_addr_len(4);
            arp.set_operation(ArpOperations::Request); // Request, not Reply
        }
        eth.set_payload(&arp_buffer);

        let result = parse_arp_reply(&packet);
        assert!(result.is_none());
    }
}
