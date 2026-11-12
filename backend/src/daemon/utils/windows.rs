#[cfg(target_family = "windows")]
use crate::daemon::utils::base::DaemonUtils;

#[cfg(target_family = "windows")]
use anyhow::{Error, Result, anyhow};
#[cfg(target_family = "windows")]
use async_trait::async_trait;
#[cfg(target_family = "windows")]
use mac_address::MacAddress;
#[cfg(target_family = "windows")]
use std::net::{IpAddr, Ipv4Addr};

#[cfg(target_family = "windows")]
pub struct WindowsDaemonUtils;

#[cfg(target_family = "windows")]
#[async_trait]
impl DaemonUtils for WindowsDaemonUtils {
    fn new() -> Self {
        Self {}
    }

    fn get_fd_limit() -> Result<usize, anyhow::Error> {
        // Windows doesn't have a direct equivalent to Unix file descriptors
        // The _getmaxstdio() function returns the maximum number of FILE streams (default 512)
        // However, socket handles use a different mechanism

        // For sockets specifically, Windows has a per-process limit of ~65000 handles
        // But practically, we should use a conservative value similar to Unix defaults

        // Return a reasonable default that works well on Windows
        // This is roughly equivalent to what a typical Windows system can handle
        Ok(2048)
    }

    fn get_optimal_arp_concurrency(&self) -> Result<usize, Error> {
        // Windows uses WinPcap/Npcap for raw packet capture
        // More permissive than macOS but still keep reasonable limits
        let fd_limit = Self::get_fd_limit()?;
        let reserved = 203;
        let available = fd_limit.saturating_sub(reserved);

        let concurrency = std::cmp::min(30, available / 10);
        let concurrency = std::cmp::max(1, concurrency);

        tracing::debug!(
            fd_limit = fd_limit,
            available = available,
            concurrency = concurrency,
            "Calculated ARP concurrency"
        );

        Ok(concurrency)
    }

    fn deep_scan_concurrency_at(
        &self,
        fd_limit: usize,
        port_batch_size: usize,
        arp_subnet_count: usize,
    ) -> usize {
        // 200 reserved handles: Windows has no standard-stream descriptors to count.
        crate::daemon::utils::base::deep_scan_concurrency(
            fd_limit,
            port_batch_size,
            arp_subnet_count,
            200,
            false,
        )
    }

    async fn get_mac_address_for_ip(&self, ip: IpAddr) -> Result<Option<MacAddress>> {
        use windows::Win32::NetworkManagement::IpHelper::{GetIpNetTable, MIB_IPNETTABLE};

        let ipv4_addr = match ip {
            IpAddr::V4(addr) => addr,
            IpAddr::V6(_) => return Ok(None), // IPv6 ARP not supported in this implementation
        };

        // First call to get required buffer size
        let mut size: u32 = 0;
        let _result = unsafe { GetIpNetTable(None, &mut size, true) };

        if size == 0 {
            return Ok(None);
        }

        // Allocate buffer and get the actual table
        let mut buffer = vec![0u8; size as usize];
        let table_ptr = buffer.as_mut_ptr() as *mut MIB_IPNETTABLE;

        let result = unsafe { GetIpNetTable(Some(table_ptr), &mut size, true) };

        if result != 0 {
            return Err(anyhow!("GetIpNetTable failed with error code: {}", result));
        }

        // Parse the table
        let table = unsafe { &*table_ptr };
        let entries = unsafe {
            std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize)
        };

        // Find matching IP
        for entry in entries {
            let entry_ip = Ipv4Addr::from(u32::from_be(entry.dwAddr));
            if entry_ip == ipv4_addr {
                // Extract MAC address bytes (only use first 6 bytes)
                let mac_bytes = [
                    entry.bPhysAddr[0],
                    entry.bPhysAddr[1],
                    entry.bPhysAddr[2],
                    entry.bPhysAddr[3],
                    entry.bPhysAddr[4],
                    entry.bPhysAddr[5],
                ];

                return Ok(Some(MacAddress::new(mac_bytes)));
            }
        }

        Ok(None)
    }
}
