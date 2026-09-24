//! ICMP echo sweeping, with platform-specific implementations.
//!
//! ## Why this exists
//!
//! Before this, an address that produced no ARP reply on a subnet the daemon has an interface on
//! was never queued for scanning at all — the ARP forwarder emits one message per reply and
//! nothing else — so a host behind a hypervisor bridge that does not answer layer-2 injection was
//! invisible, with no setting able to reach it (GH #678). ICMP is the missing second signal, and
//! it is the one the reporter demonstrated: `ping` from inside the daemon's container reached the
//! host, because a layer-4 socket hands the packet to the kernel's routing table instead of
//! injecting a frame onto a chosen interface.
//!
//! ## ICMP is additive, never a gate
//!
//! Neither signal subsumes the other, so neither may filter the other:
//!
//! - **ARP is the only sweep that yields a MAC**, which `Pattern::MacVendor` and the stored
//!   `IPAddress` both depend on.
//! - **Windows blocks inbound echo by default** while answering ARP perfectly well, so gating
//!   anything on a ping would lose most of a Windows estate.
//!
//! An address is live if *any* signal answers; addresses that answer nothing still fall through to
//! the TCP responsiveness check exactly as before.
//!
//! ## Platform behaviour
//!
//! | Platform      | Method                     | Privilege                              |
//! |---------------|----------------------------|----------------------------------------|
//! | Linux         | raw socket (pnet transport)| `CAP_NET_RAW` — as ARP already needs   |
//! | macOS / BSD   | raw socket (pnet transport)| root — as ARP already needs            |
//! | Windows       | `IcmpSendEcho` (iphlpapi)  | none                                   |
//!
//! When unavailable the sweep is skipped and discovery behaves exactly as it did before.

pub mod iphlpapi;
pub mod raw;
pub mod types;

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use anyhow::Result;

pub use types::IcmpScanResult;

/// Sweep `targets` with ICMP echo requests using the platform-appropriate mechanism.
///
/// Returns a receiver that streams addresses as they answer. Unlike [`super::arp::scan_subnet`]
/// this takes no interface or source MAC: ICMP is routed by the kernel, so a single sweep covers
/// every subnet in scope regardless of which ones the daemon has an interface on.
///
/// * `retries` — extra rounds, sent only to addresses that stayed silent.
/// * `rate_pps` — send rate ceiling.
/// * `packets_sent` — incremented per attempt so callers can pace progress on real throughput.
pub fn sweep(
    targets: Vec<Ipv4Addr>,
    retries: u32,
    rate_pps: u32,
    packets_sent: Arc<AtomicU64>,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<std::sync::mpsc::Receiver<IcmpScanResult>> {
    #[cfg(target_family = "windows")]
    {
        // The iphlpapi sweep is a series of bounded OS requests and takes no cancellation.
        let _ = &cancel;
        iphlpapi::sweep(targets, retries, rate_pps, packets_sent)
    }

    #[cfg(not(target_family = "windows"))]
    {
        raw::sweep(targets, retries, rate_pps, packets_sent, cancel)
    }
}

/// Whether ICMP echo sweeping is available on this platform and process.
///
/// Mirrors [`crate::daemon::utils::scanner::can_arp_scan`]: attempt the real thing once and log
/// the outcome, rather than inferring from capabilities that cannot be observed from inside a
/// container.
pub fn is_available() -> bool {
    #[cfg(target_family = "windows")]
    let available = iphlpapi::is_available();

    #[cfg(not(target_family = "windows"))]
    let available = raw::is_available();

    if available {
        tracing::info!("ICMP echo capability confirmed. Hosts that answer ping are discoverable.");
    }
    // The unavailable branch is logged by the platform implementation, which knows what to tell
    // the operator to change.

    available
}
