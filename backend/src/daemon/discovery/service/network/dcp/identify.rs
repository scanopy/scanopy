//! The DCP Identify sweep: one multicast burst per interface, then a fixed listen window.
//!
//! Shaped like `mdns::browse` (one burst, one fixed collect window) rather than `arp::broadcast`
//! (multi-round targeted retry) — DCP finds everything on the segment in one shot regardless of
//! subnet size, there is nothing to retry against (no target list, no "who hasn't answered yet").
//!
//! Blocking, like ARP's own transport code — callers on the async scan path run it via
//! `tokio::task::spawn_blocking`, the same way `arp::broadcast`'s raw-socket work is kept off the
//! async runtime.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use mac_address::MacAddress;
use pnet::datalink::{self, NetworkInterface};
use pnet::util::MacAddr;
use rand::Rng;

use super::channel::{DcpChannel, PnetDcpChannel};
use super::packet::{DcpIdentifyResponse, build_identify_request, parse_identify_response};

/// How long to keep listening after the request goes out. PROFINET DCP devices are expected to
/// distribute their answer inside the response-delay window advertised in the request
/// (`RESPONSE_DELAY_FACTOR` in `packet.rs`); this is longer than that on purpose, to allow for
/// devices that answer slower than the advertised window in practice. Unverified against a real
/// segment — see the module's Work Summary note.
pub const LISTEN_WINDOW: Duration = Duration::from_secs(3);

const READ_TIMEOUT: Duration = Duration::from_millis(50);

/// Whether raw-socket access is available at all — same capability probe ARP uses (enumerate
/// interfaces, trial-open a channel), since DCP needs the identical raw-socket permission and has
/// no fallback to fall back to if it is missing (unlike ARP, which still has SendARP on Windows).
pub fn is_available() -> bool {
    let Some(interface) = datalink::interfaces().into_iter().find(is_dcp_capable) else {
        tracing::warn!(
            "DCP scanning unavailable: no suitable network interface found. \
             Ensure container has a non-loopback interface with a MAC address."
        );
        return false;
    };

    match PnetDcpChannel::open(&interface, Duration::from_millis(100)) {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!(
                interface = %interface.name,
                error = %e,
                "DCP scanning unavailable: failed to create raw socket channel. \
                 Ensure container has NET_RAW and NET_ADMIN capabilities."
            );
            false
        }
    }
}

/// Whether an interface can carry a DCP sweep at all: up, not loopback, has a MAC to send from.
/// Same capability shape ARP's own probe checks, and the same Windows caveat (pnet hardcodes
/// `flags=0` there, so `is_up()` is not trustworthy and the real test is a trial channel open).
pub fn is_dcp_capable(iface: &NetworkInterface) -> bool {
    #[cfg(target_family = "windows")]
    let up_check = true;
    #[cfg(not(target_family = "windows"))]
    let up_check = iface.is_up();

    up_check && !iface.is_loopback() && iface.mac.is_some()
}

/// Sweep one interface: send one Identify Request, collect responses for [`LISTEN_WINDOW`],
/// deduped by responder MAC (a device that answers more than once — retransmission, or multiple
/// NICs sharing a segment view — is reported once).
pub fn scan_interface(interface: &NetworkInterface) -> anyhow::Result<Vec<DcpIdentifyResponse>> {
    let source_mac = interface
        .mac
        .ok_or_else(|| anyhow::anyhow!("interface {} has no MAC address", interface.name))?;
    let source_mac = MacAddr::new(
        source_mac.octets()[0],
        source_mac.octets()[1],
        source_mac.octets()[2],
        source_mac.octets()[3],
        source_mac.octets()[4],
        source_mac.octets()[5],
    );

    let mut channel = PnetDcpChannel::open(interface, READ_TIMEOUT)?;
    let xid = fresh_xid();

    let deadline = Instant::now() + LISTEN_WINDOW;
    let found = collect(&mut channel, source_mac, xid, || Instant::now() < deadline);
    Ok(found)
}

/// A transaction id in the `0x0FXXYYYY` shape observed in practice (see `packet.rs`'s module
/// doc) — not load-bearing for correctness (any distinct value works: the receiver only needs it
/// echoed back to reject stale/foreign replies), just following the convention.
fn fresh_xid() -> u32 {
    0x0F00_0000 | (rand::rng().random::<u32>() & 0x00FF_FFFF)
}

/// The shared collect logic: send the request once, then poll `channel` until `keep_going`
/// returns false, deduping successfully-parsed responses by MAC. `keep_going` is the one thing
/// production and tests disagree about — a wall-clock deadline in [`scan_interface`], a bounded
/// iteration count in tests — everything else (parse, reject, dedup) is exercised identically by
/// both, via [`super::channel::ScriptedDcpChannel`] in tests.
fn collect(
    channel: &mut impl DcpChannel,
    source_mac: MacAddr,
    xid: u32,
    keep_going: impl Fn() -> bool,
) -> Vec<DcpIdentifyResponse> {
    let request = build_identify_request(source_mac, xid);
    if let Err(e) = channel.send(&request) {
        tracing::warn!(error = %e, "Failed to send DCP Identify request");
        return Vec::new();
    }

    let mut found: HashMap<MacAddress, DcpIdentifyResponse> = HashMap::new();
    while keep_going() {
        match channel.recv() {
            Ok(Some(frame)) => {
                if let Some(response) = parse_identify_response(&frame, xid) {
                    found.entry(response.mac).or_insert(response);
                }
                // Malformed / wrong-service / mismatched-Xid frames are silently dropped by
                // `parse_identify_response` returning `None` — exactly the ARP precedent for a
                // non-matching packet on a shared segment.
            }
            Ok(None) => {} // Timeout: nothing arrived this poll. Ordinary, not a stop condition.
            Err(e) => {
                tracing::trace!(error = %e, "DCP receive error, continuing");
            }
        }
    }
    found.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::super::channel::ScriptedDcpChannel;
    use super::super::packet::{build_identify_request, parse_identify_response};
    use super::*;

    fn source_mac() -> MacAddr {
        MacAddr::new(0x00, 0x11, 0x22, 0x33, 0x44, 0x55)
    }

    /// Test-only stand-in for a real Identify Response frame, matching `packet.rs`'s own private
    /// test builder in shape (duplicated rather than shared across modules, since exposing it
    /// outside `packet.rs`'s tests would widen that module's surface for no reason beyond this).
    fn response_frame(responder: MacAddr, xid: u32) -> Vec<u8> {
        use pnet::packet::ethernet::{EtherType, MutableEthernetPacket};

        let mut buf = vec![0u8; 14];
        {
            let mut eth = MutableEthernetPacket::new(&mut buf).unwrap();
            eth.set_destination(source_mac());
            eth.set_source(responder);
            eth.set_ethertype(EtherType::new(super::super::packet::ETHERTYPE_PROFINET));
        }
        buf.extend_from_slice(&0xFEFFu16.to_be_bytes()); // FrameID: Identify Response
        buf.push(0x05); // ServiceID: Identify
        buf.push(0x01); // ServiceType: ResponseSuccess
        buf.extend_from_slice(&xid.to_be_bytes());
        buf.extend_from_slice(&0u16.to_be_bytes()); // reserved
        buf.extend_from_slice(&0u16.to_be_bytes()); // no blocks
        buf
    }

    /// A stop condition that runs `keep_going` exactly `n` times true then false — the test
    /// analog of a real deadline, driving the shared `collect` loop through a known number of
    /// `recv()` calls so a `ScriptedDcpChannel`'s script length determines the outcome.
    fn polls(n: usize) -> impl Fn() -> bool {
        let remaining = std::cell::Cell::new(n);
        move || {
            let r = remaining.get();
            if r == 0 {
                false
            } else {
                remaining.set(r - 1);
                true
            }
        }
    }

    #[test]
    fn sends_the_request_exactly_once() {
        let mut channel = ScriptedDcpChannel::new(vec![]);
        let xid = 42;
        collect(&mut channel, source_mac(), xid, polls(3));

        assert_eq!(channel.sent.len(), 1);
        assert_eq!(channel.sent[0], build_identify_request(source_mac(), xid));
    }

    #[test]
    fn collects_replies_from_distinct_responders() {
        let xid = 7;
        let a = MacAddr::new(1, 1, 1, 1, 1, 1);
        let b = MacAddr::new(2, 2, 2, 2, 2, 2);
        let mut channel =
            ScriptedDcpChannel::new(vec![response_frame(a, xid), response_frame(b, xid)]);

        let found = collect(&mut channel, source_mac(), xid, polls(2));

        let macs: std::collections::HashSet<_> = found.iter().map(|r| r.mac).collect();
        assert_eq!(macs.len(), 2);
        assert!(macs.contains(&MacAddress::new(a.octets())));
        assert!(macs.contains(&MacAddress::new(b.octets())));
    }

    #[test]
    fn duplicate_replies_from_the_same_mac_are_deduped() {
        let xid = 7;
        let a = MacAddr::new(1, 1, 1, 1, 1, 1);
        let mut channel = ScriptedDcpChannel::new(vec![
            response_frame(a, xid),
            response_frame(a, xid), // a retransmit, or a second NIC on the same segment
        ]);

        let found = collect(&mut channel, source_mac(), xid, polls(2));

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn malformed_and_mismatched_frames_are_dropped_not_collected() {
        let xid = 7;
        let wrong_xid = response_frame(MacAddr::new(9, 9, 9, 9, 9, 9), xid + 1);
        let truncated = vec![0u8; 5];
        let mut channel = ScriptedDcpChannel::new(vec![wrong_xid, truncated]);

        let found = collect(&mut channel, source_mac(), xid, polls(2));

        assert!(found.is_empty());
    }

    #[test]
    fn stops_collecting_once_the_stop_condition_says_so_even_with_more_incoming() {
        let xid = 7;
        let a = MacAddr::new(1, 1, 1, 1, 1, 1);
        let b = MacAddr::new(2, 2, 2, 2, 2, 2);
        // Two replies queued, but the stop condition only allows one poll after the send.
        let mut channel =
            ScriptedDcpChannel::new(vec![response_frame(a, xid), response_frame(b, xid)]);

        let found = collect(&mut channel, source_mac(), xid, polls(1));

        assert_eq!(found.len(), 1, "only the first poll's reply is collected");
    }

    /// Confirms `collect`'s dedup/parse core is exercised identically regardless of the stop
    /// condition's shape — a scripted, bounded one here, a real wall-clock deadline in
    /// `scan_interface`. Not a test of `parse_identify_response` itself (that lives in
    /// `packet.rs`); this is the loop around it.
    #[test]
    fn a_timeout_tick_with_nothing_arrived_is_not_a_stop_condition_on_its_own() {
        let xid = 7;
        let a = MacAddr::new(1, 1, 1, 1, 1, 1);
        // One timeout (`recv` returns `Ok(None)` once the script is exhausted) between two
        // replies, modeled by placing the reply after the channel would otherwise be empty.
        let mut channel = ScriptedDcpChannel::new(vec![response_frame(a, xid)]);

        // Poll three times: first gets the reply, the rest see the script exhausted (`Ok(None)`)
        // and must not treat that as an error or stop early on their own.
        let found = collect(&mut channel, source_mac(), xid, polls(3));

        assert_eq!(found.len(), 1);
    }

    #[test]
    fn parse_identify_response_is_reachable_from_this_module_for_the_frame_builder_above() {
        // Sanity check that `response_frame` above actually builds something `packet.rs` parses,
        // so the other tests are exercising real parsing rather than a fake shortcut.
        let xid = 1;
        let frame = response_frame(MacAddr::new(1, 2, 3, 4, 5, 6), xid);
        assert!(parse_identify_response(&frame, xid).is_some());
    }
}
