//! ICMP echo sweep over a raw transport socket (Linux, macOS, BSD).
//!
//! Structurally a sibling of [`super::super::arp::broadcast`] — a sender thread and a receiver
//! thread joined by a `std::sync::mpsc` channel, with the same safety valves — because
//! `pnet_transport` is blocking and cannot be awaited. The differences from ARP are all
//! consequences of operating a layer up:
//!
//! - **No interface, no source MAC.** A `Layer4` socket hands the packet to the kernel's routing
//!   table, so one channel covers every subnet in scope. This is precisely why a `ping` from
//!   inside a container reaches a VM behind a hypervisor bridge when layer-2 ARP injection does
//!   not (GH #678).
//! - **The socket sees every ICMP packet on the host**, not just ours — other processes' pings,
//!   and their replies. Replies are matched on type, our per-sweep identifier, *and* a payload
//!   token before the source address is believed.

#[cfg(not(target_family = "windows"))]
use std::collections::HashSet;
#[cfg(not(target_family = "windows"))]
use std::net::{IpAddr, Ipv4Addr};
#[cfg(not(target_family = "windows"))]
use std::sync::Arc;
#[cfg(not(target_family = "windows"))]
use std::sync::atomic::AtomicU64;
#[cfg(not(target_family = "windows"))]
use std::time::Duration;

#[cfg(not(target_family = "windows"))]
use anyhow::Result;

#[cfg(not(target_family = "windows"))]
use super::types::IcmpScanResult;

/// Wait after each round before retrying the addresses that stayed silent.
///
/// Far shorter than ARP's three seconds: an echo reply on a LAN comes back in single-digit
/// milliseconds, and unlike ARP there is no switch-level policing to pace around.
#[cfg(not(target_family = "windows"))]
const ROUND_WAIT: Duration = Duration::from_secs(1);

/// Extra receive time after the final round, for stragglers.
#[cfg(not(target_family = "windows"))]
const POST_SCAN_RECEIVE: Duration = Duration::from_secs(2);

/// Hard max lifetime for the receiver thread (defense-in-depth), mirroring the ARP receiver.
#[cfg(not(target_family = "windows"))]
const MAX_RECEIVER_LIFETIME: Duration = Duration::from_secs(300);

/// Payload stamped into every echo request and echoed back verbatim by a responder.
///
/// The identifier alone is 16 bits and another tool on the host may well pick the same one; the
/// token makes a foreign reply that collides with our identifier fail the second check rather
/// than enter the results as a phantom host.
#[cfg(not(target_family = "windows"))]
pub(super) const PAYLOAD_TOKEN: &[u8; 8] = b"scanopy\0";

/// Bytes on the wire for one echo request: 8-byte ICMP header plus the token.
#[cfg(not(target_family = "windows"))]
pub(super) const ECHO_PACKET_LEN: usize = 8 + PAYLOAD_TOKEN.len();

/// Whether a raw ICMP socket can be opened.
///
/// Mirrors [`super::super::arp::broadcast::is_available`]: attempt the real thing once rather
/// than inferring from capabilities we cannot see from inside a container.
#[cfg(not(target_family = "windows"))]
pub fn is_available() -> bool {
    use pnet::packet::ip::IpNextHeaderProtocols;
    use pnet::transport::TransportChannelType::Layer4;
    use pnet::transport::TransportProtocol::Ipv4;
    use pnet::transport::transport_channel;

    match transport_channel(64, Layer4(Ipv4(IpNextHeaderProtocols::Icmp))) {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "ICMP echo unavailable: could not open a raw ICMP socket. Hosts that answer ping \
                 but not ARP will not be discovered. On Linux this needs NET_RAW (the same \
                 capability ARP scanning already requires); on macOS and BSD it needs root."
            );
            false
        }
    }
}

/// Build one echo request into `buf`, which must be [`ECHO_PACKET_LEN`] bytes.
#[cfg(not(target_family = "windows"))]
pub(super) fn build_echo_request(buf: &mut [u8], identifier: u16, sequence: u16) {
    use pnet::packet::Packet;
    use pnet::packet::icmp::echo_request::MutableEchoRequestPacket;
    use pnet::packet::icmp::{IcmpPacket, IcmpTypes, checksum};

    let mut packet = MutableEchoRequestPacket::new(buf).expect("echo request buffer is sized");
    packet.set_icmp_type(IcmpTypes::EchoRequest);
    packet.set_identifier(identifier);
    packet.set_sequence_number(sequence);
    packet.set_payload(PAYLOAD_TOKEN);

    // The checksum covers the whole ICMP message and must be computed last, over a packet whose
    // own checksum field still reads zero — which it does, since the buffer starts zeroed and
    // nothing above writes it.
    let sum = checksum(&IcmpPacket::new(packet.packet()).expect("just-built packet parses"));
    packet.set_checksum(sum);
}

/// Whether `payload` is an echo reply to *our* sweep.
///
/// Split out from the receive loop so the matching rule — the part that keeps another process's
/// pings out of the results — can be exercised without a socket.
#[cfg(not(target_family = "windows"))]
pub(super) fn is_our_echo_reply(payload: &[u8], identifier: u16) -> bool {
    use pnet::packet::Packet;
    use pnet::packet::icmp::echo_reply::EchoReplyPacket;
    use pnet::packet::icmp::{IcmpPacket, IcmpTypes};

    let Some(icmp) = IcmpPacket::new(payload) else {
        return false;
    };
    if icmp.get_icmp_type() != IcmpTypes::EchoReply {
        return false;
    }
    let Some(reply) = EchoReplyPacket::new(payload) else {
        return false;
    };
    reply.get_identifier() == identifier && reply.payload().starts_with(PAYLOAD_TOKEN)
}

/// Sweep `targets` with ICMP echo requests, streaming responders as they answer.
///
/// `retries` extra rounds are sent only to addresses that have stayed silent, matching the ARP
/// sweep's targeted-retry behaviour. `packets_sent` is incremented per attempt, sent or failed,
/// so the caller's progress bar paces against the real send schedule.
#[cfg(not(target_family = "windows"))]
pub fn sweep(
    targets: Vec<Ipv4Addr>,
    retries: u32,
    rate_pps: u32,
    packets_sent: Arc<AtomicU64>,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<std::sync::mpsc::Receiver<IcmpScanResult>> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    use pnet::packet::Packet;
    use pnet::packet::ip::IpNextHeaderProtocols;
    use pnet::transport::TransportChannelType::Layer4;
    use pnet::transport::TransportProtocol::Ipv4;
    use pnet::transport::{icmp_packet_iter, transport_channel};

    let (result_tx, result_rx) = mpsc::channel();

    // A 16-bit identifier scoped to this sweep. The process id keeps concurrent daemons apart and
    // the counter keeps successive sweeps within one daemon apart, so a late reply to the
    // previous sweep cannot be credited to this one.
    let identifier = next_identifier();

    let target_set: HashSet<Ipv4Addr> = targets.iter().copied().collect();
    let target_count = target_set.len();

    // 4096 is comfortably above the 8-byte header plus token we send and the largest echo reply a
    // responder will send back.
    let (mut tx, mut rx) = transport_channel(4096, Layer4(Ipv4(IpNextHeaderProtocols::Icmp)))?;

    let send_delay = Duration::from_micros(1_000_000 / rate_pps.max(1) as u64);
    let total_rounds = 1 + retries;

    tracing::debug!(
        targets = target_count,
        total_rounds,
        rate_pps,
        identifier,
        "Starting ICMP echo sweep"
    );

    let found: Arc<Mutex<HashSet<Ipv4Addr>>> = Arc::new(Mutex::new(HashSet::new()));
    let sending_done = Arc::new(AtomicBool::new(false));

    let found_recv = found.clone();
    let sending_done_recv = sending_done.clone();
    let targets_recv = target_set.clone();

    let receiver_handle = thread::spawn(move || {
        let start = Instant::now();
        let mut iter = icmp_packet_iter(&mut rx);
        let mut deadline: Option<Instant> = None;

        loop {
            // A packet that isn't ours must still fall through to the exit checks below rather
            // than short-circuit the iteration. The ARP receiver documents why: on a busy segment
            // the raw socket delivers unrelated traffic continuously, so a loop that only
            // re-checks its stop condition on an idle tick never gets one and runs forever.
            if let Ok(Some((packet, addr))) = iter.next_with_timeout(Duration::from_millis(100))
                && let IpAddr::V4(source) = addr
                && targets_recv.contains(&source)
                && is_our_echo_reply(packet.packet(), identifier)
                && found_recv.lock().unwrap().insert(source)
            {
                tracing::debug!(ip = %source, "ICMP: Host answered echo");
                let _ = result_tx.send(IcmpScanResult { ip: source });
            }

            // Once sending finishes, run a bounded tail for stragglers rather than exiting on the
            // first idle tick — a slow device's reply can arrive well after the last request.
            if deadline.is_none() && sending_done_recv.load(Ordering::Relaxed) {
                deadline = Some(Instant::now() + POST_SCAN_RECEIVE);
            }
            if deadline.is_some_and(|d| Instant::now() >= d) {
                break;
            }
            if start.elapsed() >= MAX_RECEIVER_LIFETIME {
                tracing::warn!(
                    elapsed_secs = start.elapsed().as_secs(),
                    "ICMP receiver hit max lifetime, forcing exit"
                );
                break;
            }
        }

        let found = found_recv.lock().unwrap().len();
        tracing::debug!(
            elapsed_secs = start.elapsed().as_secs(),
            hosts_found = found,
            hosts_silent = target_count - found,
            "ICMP echo sweep completed"
        );
    });

    thread::spawn(move || {
        // As in the ARP sender: cancelling stops sending, and the loop still falls through to
        // `sending_done` and the join that close the result channel.
        for round in 1..=total_rounds {
            if cancel.is_cancelled() {
                tracing::debug!(round, "ICMP sweep cancelled; no further echoes sent");
                break;
            }
            let round_targets: Vec<Ipv4Addr> = {
                let found = found.lock().unwrap();
                target_set
                    .iter()
                    .filter(|ip| !found.contains(ip))
                    .copied()
                    .collect()
            };

            if round_targets.is_empty() {
                tracing::debug!(
                    round,
                    "All ICMP targets answered, skipping remaining rounds"
                );
                break;
            }

            let mut buf = [0u8; ECHO_PACKET_LEN];
            for (index, target) in round_targets.iter().enumerate() {
                if cancel.is_cancelled() {
                    break;
                }
                // Rebuild per target: the sequence number varies, and with it the checksum.
                buf.fill(0);
                build_echo_request(&mut buf, identifier, (round as usize * 1024 + index) as u16);
                let packet = pnet::packet::icmp::echo_request::EchoRequestPacket::new(&buf)
                    .expect("just-built packet parses");

                if let Err(e) = tx.send_to(packet, IpAddr::V4(*target)) {
                    tracing::trace!(target = %target, error = %e, "Failed to send ICMP echo");
                }
                // Count every attempt, sent or failed — this paces the progress bar against the
                // send schedule, which is what the ETA is derived from.
                packets_sent.fetch_add(1, Ordering::Relaxed);
                thread::sleep(send_delay);
            }

            if !cancel.is_cancelled() {
                thread::sleep(ROUND_WAIT);
            }
        }

        sending_done.store(true, Ordering::Relaxed);
        let _ = receiver_handle.join();
    });

    Ok(result_rx)
}

/// A per-sweep ICMP identifier.
#[cfg(not(target_family = "windows"))]
fn next_identifier() -> u16 {
    use std::sync::atomic::{AtomicU16, Ordering};
    static SWEEP: AtomicU16 = AtomicU16::new(0);
    let sweep = SWEEP.fetch_add(1, Ordering::Relaxed);
    (std::process::id() as u16).wrapping_add(sweep)
}

// Stubs for Windows, which uses the iphlpapi path instead.
#[cfg(target_family = "windows")]
pub fn is_available() -> bool {
    false
}

#[cfg(target_family = "windows")]
pub fn sweep(
    _targets: Vec<std::net::Ipv4Addr>,
    _retries: u32,
    _rate_pps: u32,
    _packets_sent: std::sync::Arc<std::sync::atomic::AtomicU64>,
    _cancel: tokio_util::sync::CancellationToken,
) -> anyhow::Result<std::sync::mpsc::Receiver<super::types::IcmpScanResult>> {
    Err(anyhow::anyhow!(
        "Raw ICMP sockets are not used on Windows; see the iphlpapi path"
    ))
}

#[cfg(all(test, not(target_family = "windows")))]
mod tests {
    use super::*;

    fn reply_bytes(identifier: u16, token: &[u8]) -> Vec<u8> {
        // An echo reply is byte-identical to a request but for the type field, so build a request
        // and flip it. Keeps the fixture honest about the wire format instead of hand-rolling
        // bytes that only this test agrees with.
        let mut buf = vec![0u8; 8 + token.len()];
        build_echo_request(&mut buf, identifier, 1);
        buf[0] = 0; // IcmpTypes::EchoReply
        buf[8..].copy_from_slice(token);
        buf
    }

    /// A reply to our own sweep is accepted. Round-trips the builder against the matcher, so a
    /// change to either that breaks the pairing fails here.
    #[test]
    fn our_own_echo_reply_is_recognised() {
        let identifier = 0x4242;
        assert!(is_our_echo_reply(
            &reply_bytes(identifier, PAYLOAD_TOKEN),
            identifier
        ));
    }

    /// The reason the sweep can share a host with anything else that pings: a raw ICMP socket
    /// receives every process's replies, and crediting one to a target would invent a live host.
    #[test]
    fn a_reply_to_someone_else_s_ping_is_rejected() {
        assert!(
            !is_our_echo_reply(&reply_bytes(0x1111, PAYLOAD_TOKEN), 0x4242),
            "a foreign identifier must not match"
        );
        assert!(
            !is_our_echo_reply(&reply_bytes(0x4242, b"OTHER\0\0\0"), 0x4242),
            "a colliding identifier must still fail on the payload token"
        );
    }

    /// An echo *request* — which the socket also sees, including our own outgoing ones — is not a
    /// reply and must not count as one.
    #[test]
    fn an_echo_request_is_not_a_reply() {
        let mut buf = vec![0u8; ECHO_PACKET_LEN];
        build_echo_request(&mut buf, 0x4242, 1);
        assert!(!is_our_echo_reply(&buf, 0x4242));
    }

    /// Truncated or empty payloads reach the matcher whenever the socket hands up a runt packet.
    #[test]
    fn a_malformed_packet_is_rejected_rather_than_panicking() {
        assert!(!is_our_echo_reply(&[], 0x4242));
        assert!(!is_our_echo_reply(&[0, 0, 0], 0x4242));
    }
}
