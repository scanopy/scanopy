#[cfg(target_os = "macos")]
use crate::daemon::utils::base::DaemonUtils;

#[cfg(target_os = "macos")]
#[derive(Clone)]
pub struct MacOsDaemonUtils;

#[cfg(target_os = "macos")]
use anyhow::{Error, Result, anyhow};
#[cfg(target_os = "macos")]
use mac_address::MacAddress;
#[cfg(target_os = "macos")]
impl MacOsDaemonUtils {
    /// Parse MAC address from macOS format (handles missing leading zeros)
    fn parse_macos_mac_address(&self, mac_str: &str) -> Result<MacAddress, Error> {
        let parts: Vec<&str> = mac_str.split(':').collect();
        if parts.len() != 6 {
            return Err(anyhow!("Invalid MAC address format: {}", mac_str));
        }

        let mut mac_bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            // Handle macOS format where leading zeros are omitted (e.g., "0:22:7" instead of "00:22:07")
            mac_bytes[i] = u8::from_str_radix(part, 16)
                .map_err(|_| anyhow!("Invalid hex in MAC address: {}", part))?;
        }

        Ok(MacAddress::new(mac_bytes))
    }
}

#[cfg(target_os = "macos")]
use async_trait::async_trait;
#[cfg(target_os = "macos")]
use std::net::IpAddr;
#[cfg(target_os = "macos")]
#[async_trait]
impl DaemonUtils for MacOsDaemonUtils {
    fn new() -> Self {
        Self {}
    }

    fn get_fd_limit() -> Result<usize, Error> {
        use libc::{RLIMIT_NOFILE, getrlimit, rlimit};

        let mut rlim = rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };

        let result = unsafe { getrlimit(RLIMIT_NOFILE, &mut rlim as *mut rlimit) };

        if result == 0 {
            Ok(rlim.rlim_cur as usize)
        } else {
            Err(anyhow!("Failed to get file descriptor limit"))
        }
    }

    fn get_optimal_arp_concurrency(&self) -> Result<usize, Error> {
        // macOS has limited BPF devices and FD_SET has a fixed-size array (32 on some versions)
        // Opening too many concurrent datalink channels causes index out of bounds panic
        // Keep this conservative to avoid crashing
        Ok(10)
    }

    fn deep_scan_concurrency_at(
        &self,
        fd_limit: usize,
        port_batch_size: usize,
        arp_subnet_count: usize,
    ) -> usize {
        // 203 reserved: the three standard streams plus the shared 200. Conservative under
        // macOS's default soft limit of 256.
        crate::daemon::utils::base::deep_scan_concurrency(
            fd_limit,
            port_batch_size,
            arp_subnet_count,
            203,
            true,
        )
    }

    async fn get_mac_address_for_ip(&self, ip: IpAddr) -> Result<Option<MacAddress>, Error> {
        use tokio::process::Command;

        tracing::debug!("Attempting to get MAC address for IP: {}", ip);

        let output = Command::new("arp")
            .args(["-n", &ip.to_string()])
            .output()
            .await?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);

            // Parse macOS arp output: "? (192.168.1.1) at 0:22:7:4a:21:d5 on en0 ifscope [ethernet]"
            for line in output_str.lines() {
                tracing::debug!("Processing arp output line: {}", line);
                if line.contains(&ip.to_string()) {
                    tracing::debug!("Line contains IP: {}", ip);
                    // Look for "at MAC_ADDRESS" pattern
                    if let Some(at_pos) = line.find(" at ") {
                        let after_at = &line[at_pos + 4..];
                        if let Some(space_pos) = after_at.find(' ') {
                            let mac_str = &after_at[..space_pos];
                            tracing::debug!("Found MAC string candidate: {}", mac_str);
                            if mac_str.contains(':') && mac_str.matches(':').count() == 5 {
                                match self.parse_macos_mac_address(mac_str) {
                                    Ok(mac) => {
                                        tracing::debug!("Parsed MAC address: {}", mac);
                                        return Ok(Some(mac));
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to parse MAC address '{}': {:?}",
                                            mac_str,
                                            e
                                        );
                                        return Err(e);
                                    }
                                }
                            } else {
                                tracing::debug!(
                                    "MAC string does not have expected format: {}",
                                    mac_str
                                );
                            }
                        } else {
                            tracing::debug!("No space found after MAC string in line: {}", line);
                        }
                    } else {
                        tracing::debug!("No ' at ' found in line: {}", line);
                    }
                }
            }
            tracing::debug!("No matching MAC address found for IP: {}", ip);
        } else {
            tracing::warn!("arp command failed with status: {}", output.status);
        }

        Ok(None)
    }
}
