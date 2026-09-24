//! Windows ICMP echo via the native iphlpapi API.
//!
//! Mirrors [`super::super::arp::sendarp`], which already reaches into this same Windows module for
//! `SendARP`. Unlike the raw-socket path this needs **no administrator rights**, which makes
//! Windows the one platform where ICMP is cheaper to obtain than ARP.

#[cfg(target_family = "windows")]
use std::net::Ipv4Addr;
#[cfg(target_family = "windows")]
use std::sync::Arc;
#[cfg(target_family = "windows")]
use std::sync::atomic::AtomicU64;

#[cfg(target_family = "windows")]
use anyhow::Result;

#[cfg(target_family = "windows")]
use super::types::IcmpScanResult;

/// Concurrent echoes in flight, matching the SendARP path's chunking.
#[cfg(target_family = "windows")]
pub(crate) const ECHO_CONCURRENCY: usize = 50;

/// Per-request timeout in milliseconds. A LAN round trip is single-digit milliseconds; this is
/// generous enough for a slow device without stalling a sweep on a dead address.
#[cfg(target_family = "windows")]
pub(crate) const ECHO_TIMEOUT_MS: u32 = 1000;

/// `IP_SUCCESS`. `IcmpSendEcho` reports unreachables as *replies* carrying a failure status, so a
/// non-zero return is not on its own evidence that anything is alive.
#[cfg(target_family = "windows")]
const IP_SUCCESS: u32 = 0;

/// Payload sent with each echo, kept identical to the raw path's for symmetry.
#[cfg(target_family = "windows")]
const PAYLOAD_TOKEN: &[u8; 8] = b"scanopy\0";

/// Whether ICMP echo is usable. On Windows the API is always present and needs no privileges;
/// this only confirms a handle can be opened.
#[cfg(target_family = "windows")]
pub fn is_available() -> bool {
    use windows::Win32::NetworkManagement::IpHelper::{IcmpCloseHandle, IcmpCreateFile};

    // SAFETY: both are well-defined Windows APIs taking no pointers of ours; the handle is closed
    // immediately and never escapes.
    unsafe {
        match IcmpCreateFile() {
            Ok(handle) => {
                let _ = IcmpCloseHandle(handle);
                true
            }
            Err(e) => {
                tracing::warn!(error = %e, "ICMP echo unavailable: IcmpCreateFile failed");
                false
            }
        }
    }
}

/// Sweep `targets` with ICMP echo requests, streaming responders as they answer.
///
/// Same contract as the raw-socket path: `retries` extra rounds go only to addresses that stayed
/// silent, and `packets_sent` counts every attempt so the caller's progress paces against the
/// real send schedule.
#[cfg(target_family = "windows")]
pub fn sweep(
    targets: Vec<Ipv4Addr>,
    retries: u32,
    rate_pps: u32,
    packets_sent: Arc<AtomicU64>,
) -> Result<std::sync::mpsc::Receiver<IcmpScanResult>> {
    use std::collections::HashSet;
    use std::sync::Mutex;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    let (tx, rx) = std::sync::mpsc::channel();
    let send_delay = Duration::from_micros(1_000_000 / rate_pps.max(1) as u64);
    let total_rounds = 1 + retries;

    std::thread::spawn(move || {
        let found: Mutex<HashSet<Ipv4Addr>> = Mutex::new(HashSet::new());

        for _round in 1..=total_rounds {
            let round_targets: Vec<Ipv4Addr> = {
                let found = found.lock().unwrap();
                targets
                    .iter()
                    .filter(|ip| !found.contains(ip))
                    .copied()
                    .collect()
            };
            if round_targets.is_empty() {
                break;
            }

            for chunk in round_targets.chunks(ECHO_CONCURRENCY) {
                std::thread::scope(|s| {
                    for &ip in chunk {
                        let tx = tx.clone();
                        let found = &found;
                        let packets_sent = packets_sent.clone();
                        s.spawn(move || {
                            packets_sent.fetch_add(1, Ordering::Relaxed);
                            if !echo_single(ip) {
                                return;
                            }
                            let mut found = found.lock().unwrap();
                            if found.insert(ip) {
                                tracing::debug!(ip = %ip, "ICMP: Host answered echo");
                                let _ = tx.send(IcmpScanResult { ip });
                            }
                        });
                    }
                });
                // Rate-limit per chunk rather than per packet: the chunk already blocks for up to
                // ECHO_TIMEOUT_MS, so sleeping per packet would compound into a far slower sweep
                // than the configured rate implies.
                std::thread::sleep(send_delay * chunk.len() as u32);
            }
        }
    });

    Ok(rx)
}

/// Reply buffer, aligned for the struct the API writes into it.
///
/// `ICMP_ECHO_REPLY` contains a pointer, so it wants pointer alignment; a bare `[u8; N]` is only
/// byte-aligned, and both the API's write and our read back would be misaligned. The size is what
/// the API documents: the struct, plus the request data echoed back, plus eight bytes for an
/// error message.
#[cfg(target_family = "windows")]
#[repr(C, align(8))]
struct ReplyBuffer(
    [u8; std::mem::size_of::<windows::Win32::NetworkManagement::IpHelper::ICMP_ECHO_REPLY>()
        + PAYLOAD_TOKEN.len()
        + 8],
);

/// One echo request. `true` only when the target itself answered successfully — a router's
/// "destination unreachable" comes back as a *reply* carrying a failure status, so a non-zero
/// return is not on its own evidence that anything is alive at the address we asked about.
#[cfg(target_family = "windows")]
fn echo_single(target_ip: Ipv4Addr) -> bool {
    use windows::Win32::NetworkManagement::IpHelper::{
        ICMP_ECHO_REPLY, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
    };

    // SAFETY: IcmpCreateFile/IcmpSendEcho/IcmpCloseHandle are well-defined Windows APIs. The
    // request and reply buffers are stack-allocated, live for the whole call, and are sized and
    // aligned as the API documents. The handle is closed on every path. The reply is read back
    // with `read_unaligned` so the read stays sound even if the layout assumption above is ever
    // wrong.
    unsafe {
        let Ok(handle) = IcmpCreateFile() else {
            return false;
        };

        let dest_ip = u32::from_ne_bytes(target_ip.octets());
        let request = *PAYLOAD_TOKEN;
        let mut reply_buffer =
            ReplyBuffer([0u8; std::mem::size_of::<ICMP_ECHO_REPLY>() + PAYLOAD_TOKEN.len() + 8]);

        let replies = IcmpSendEcho(
            handle,
            dest_ip,
            request.as_ptr() as *const _,
            request.len() as u16,
            None,
            reply_buffer.0.as_mut_ptr() as *mut _,
            reply_buffer.0.len() as u32,
            ECHO_TIMEOUT_MS,
        );

        let _ = IcmpCloseHandle(handle);

        if replies == 0 {
            return false;
        }
        let reply: ICMP_ECHO_REPLY =
            std::ptr::read_unaligned(reply_buffer.0.as_ptr() as *const ICMP_ECHO_REPLY);
        reply.Status == IP_SUCCESS
    }
}

// Stubs for non-Windows platforms, which use the raw-socket path instead.
#[cfg(not(target_family = "windows"))]
pub fn is_available() -> bool {
    false
}

#[cfg(not(target_family = "windows"))]
pub fn sweep(
    _targets: Vec<std::net::Ipv4Addr>,
    _retries: u32,
    _rate_pps: u32,
    _packets_sent: std::sync::Arc<std::sync::atomic::AtomicU64>,
) -> anyhow::Result<std::sync::mpsc::Receiver<super::types::IcmpScanResult>> {
    Err(anyhow::anyhow!("IcmpSendEcho is only available on Windows"))
}
