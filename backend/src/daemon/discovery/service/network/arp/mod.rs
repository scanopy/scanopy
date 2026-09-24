//! ARP scanning module with platform-specific implementations.
//!
//! ## Platform Behavior
//!
//! | Platform | Default Method       | Optional Method      | Fallback   |
//! |----------|---------------------|---------------------|------------|
//! | Linux    | Broadcast ARP (pnet) | -                   | Port scan  |
//! | macOS    | Broadcast ARP (pnet) | -                   | Port scan  |
//! | Windows  | SendARP (iphlpapi)   | Broadcast (Npcap)   | Port scan  |

pub mod broadcast;
pub mod sendarp;
pub mod types;

use std::net::Ipv4Addr;

use anyhow::Result;
use mac_address::MacAddress;
use pnet::datalink::NetworkInterface;

pub use broadcast::{POST_SCAN_RECEIVE, ROUND_WAIT};
pub use types::ArpScanResult;

/// Scan a subnet using the platform-appropriate ARP method.
///
/// Returns a channel receiver that streams results as hosts respond.
/// Uses targeted retries for non-responding hosts.
///
/// # Arguments
/// * `interface` - Network interface to use for scanning
/// * `source_ip` - Source IP address for ARP requests
/// * `source_mac` - Source MAC address for ARP requests
/// * `targets` - List of target IPs to scan
/// * `use_npcap` - (Windows only) Use Npcap broadcast ARP instead of SendARP
/// * `retries` - Number of retry rounds for non-responding hosts (0 = single attempt)
/// * `rate_pps` - Maximum packets per second (rate limiting for switch compatibility)
///
/// # Returns
/// Channel receiver that yields responsive hosts as they're discovered
#[allow(clippy::too_many_arguments)]
pub fn scan_subnet(
    interface: &NetworkInterface,
    source_ip: Ipv4Addr,
    source_mac: MacAddress,
    targets: Vec<Ipv4Addr>,
    use_npcap: bool,
    retries: u32,
    rate_pps: u32,
    packets_sent: std::sync::Arc<std::sync::atomic::AtomicU64>,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<std::sync::mpsc::Receiver<ArpScanResult>> {
    #[cfg(target_family = "windows")]
    {
        if use_npcap {
            match broadcast::scan_subnet(
                interface,
                source_ip,
                source_mac,
                targets.clone(),
                retries,
                rate_pps,
                packets_sent.clone(),
                cancel.clone(),
            ) {
                Ok(rx) => {
                    tracing::debug!("Npcap broadcast ARP scan started");
                    return Ok(rx);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Npcap broadcast ARP failed, falling back to SendARP"
                    );
                    // Fall through to SendARP below
                }
            }
        }
        // SendARP path can't report per-packet progress; the caller falls back to its
        // time-based ARP estimate when the counter stays at zero. It takes no cancellation
        // either: each SendARP call is a bounded OS request.
        let _ = &packets_sent;
        let _ = &cancel;
        return sendarp::scan_subnet_streaming(targets);
    }

    #[cfg(not(target_family = "windows"))]
    {
        let _ = use_npcap; // unused on non-Windows
        broadcast::scan_subnet(
            interface,
            source_ip,
            source_mac,
            targets,
            retries,
            rate_pps,
            packets_sent,
            cancel,
        )
    }
}

/// Check if ARP scanning is available on this platform.
///
/// # Arguments
/// * `use_npcap` - (Windows only) Check for Npcap availability instead of SendARP
///
/// # Returns
/// `true` if the selected ARP method is available
pub fn is_available(use_npcap: bool) -> bool {
    #[cfg(target_family = "windows")]
    {
        if use_npcap {
            let available = broadcast::is_available();
            tracing::debug!(
                available = available,
                method = "Npcap broadcast",
                "Checking ARP availability"
            );
            available
        } else {
            // SendARP is always available on Windows
            tracing::debug!(
                available = true,
                method = "SendARP",
                "Checking ARP availability"
            );
            true
        }
    }

    #[cfg(not(target_family = "windows"))]
    {
        let _ = use_npcap;
        let available = broadcast::is_available();
        tracing::debug!(
            available = available,
            method = "broadcast",
            "Checking ARP availability"
        );
        available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available_returns_bool() {
        // Just verify it doesn't panic and returns a boolean
        let _result = is_available(false);
        let _result_npcap = is_available(true);
    }
}
