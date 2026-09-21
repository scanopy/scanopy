use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use anyhow::Error;
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::daemon::discovery::types::warnings::DiscoveryWarning;

use super::NetworkScan;

/// How long one reverse lookup gets, including any wait for a free slot.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);

/// Most lookups a scan runs at once.
///
/// A lookup runs on the blocking pool, and the timeout above only stops *waiting* for it: the
/// thread stays in the resolver until the resolver gives up, which on a dead resolver is far
/// longer. Without a bound, one lookup per deep-scanned host piles up threads the pool shares with
/// the ICMP sweep's drain and the ARP join, both of which the pipeline waits on to finish.
const MAX_CONCURRENT_LOOKUPS: usize = 32;

/// Reverse DNS for one scan: bounded, and counting what went unanswered.
pub(super) struct ReverseDns {
    permits: Arc<Semaphore>,
    timeouts: AtomicU32,
}

impl Default for ReverseDns {
    fn default() -> Self {
        Self {
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_LOOKUPS)),
            timeouts: AtomicU32::new(0),
        }
    }
}

impl ReverseDns {
    /// The hostname `ip` resolves to, or `None` if it has none or no answer came in time.
    async fn lookup(&self, ip: IpAddr) -> Option<String> {
        let permits = self.permits.clone();
        let lookup = async move {
            let permit = permits.acquire_owned().await?;
            // The permit moves into the thread and is released when the lookup really ends, not
            // when the caller stops waiting for it.
            let hostname = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                dns_lookup::lookup_addr(&ip)
            })
            .await??;
            Ok::<String, Error>(hostname)
        };

        match timeout(LOOKUP_TIMEOUT, lookup).await {
            Ok(Ok(hostname)) => Some(hostname),
            // No PTR record, or the lookup failed outright: an answer, just not a name.
            Ok(Err(_)) => None,
            Err(_) => {
                self.timeouts.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    fn timeout_warning(&self) -> Option<DiscoveryWarning> {
        let count = self.timeouts.load(Ordering::Relaxed);
        (count > 0).then_some(DiscoveryWarning::ReverseDnsTimedOut { count })
    }
}

impl NetworkScan {
    pub(super) async fn get_hostname_for_ip(&self, ip: IpAddr) -> Result<Option<String>, Error> {
        Ok(self.reverse_dns.lookup(ip).await)
    }

    /// The end-of-run warning for reverse lookups that went unanswered, if any did.
    pub(super) fn reverse_dns_warning(&self) -> Option<DiscoveryWarning> {
        self.reverse_dns.timeout_warning()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_timed_out_lookup_holds_its_slot_until_its_thread_is_done() {
        // The caller gives up after the timeout, but the resolver thread is still running. Its
        // slot has to stay taken, or the bound on threads in the resolver means nothing.
        let permits = Arc::new(Semaphore::new(1));
        let (release, released) = std::sync::mpsc::channel::<()>();
        let held = permits.clone();
        let waiting = timeout(Duration::from_millis(20), async move {
            let permit = held.acquire_owned().await.unwrap();
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let _ = released.recv();
            })
            .await
        })
        .await;

        assert!(waiting.is_err(), "the caller stopped waiting");
        assert_eq!(permits.available_permits(), 0);

        release.send(()).unwrap();
        let _ = permits.acquire().await.unwrap();
    }
}
