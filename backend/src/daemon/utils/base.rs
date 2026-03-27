use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::{DiscoveryMetadata, EntitySource};
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
use crate::server::subnets::r#impl::types::SubnetType;
use anyhow::Error;
use anyhow::anyhow;
use async_trait::async_trait;
use bollard::query_parameters::ListNetworksOptions;
use bollard::{API_DEFAULT_VERSION, Docker};
use cidr::IpCidr;
use local_ip_address::local_ip;
use mac_address::MacAddress;
use net_route::Handle;
use pnet::ipnetwork::IpNetwork;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

pub const SCAN_TIMEOUT: Duration = Duration::from_millis(800);

/// Cross-platform system utilities trait
#[async_trait]
pub trait DaemonUtils {
    fn new() -> Self;

    /// Get MAC address for an IP from ARP table
    async fn get_mac_address_for_ip(&self, ip: IpAddr) -> Result<Option<MacAddress>, Error>;

    fn get_fd_limit() -> Result<usize, Error>;

    fn get_own_ip_address(&self) -> Result<IpAddr, Error> {
        match local_ip() {
            Ok(ip) => {
                tracing::debug!(ip = %ip, "Detected local IP address");
                Ok(ip)
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to detect local IP address. This may occur in MACVLAN containers \
                     or environments without a default route."
                );
                Err(anyhow!("Failed to get local IP address: {}", e))
            }
        }
    }

    fn get_own_mac_address(&self) -> Result<Option<MacAddress>, Error> {
        mac_address::get_mac_address().map_err(|e| anyhow!("Failed to get own MAC address: {}", e))
    }

    fn get_own_hostname(&self) -> Option<String> {
        hostname::get()
            .ok()
            .map(|os_str| os_str.to_string_lossy().into_owned())
    }

    async fn get_own_interfaces(
        &self,
        discovery_type: DiscoveryType,
        daemon_id: Uuid,
        network_id: Uuid,
        interface_filter: &[String],
    ) -> Result<
        (
            Vec<Interface>,
            Vec<Subnet>,
            HashMap<IpCidr, Option<MacAddress>>,
        ),
        Error,
    > {
        let all_interfaces = pnet::datalink::interfaces();

        // Apply interface filter if specified
        let interfaces: Vec<_> = if interface_filter.is_empty() {
            all_interfaces
        } else {
            let filtered: Vec<_> = all_interfaces
                .into_iter()
                .filter(|iface| interface_filter.iter().any(|f| f == &iface.name))
                .collect();

            if filtered.is_empty() {
                tracing::warn!(
                    filter = ?interface_filter,
                    "No interfaces matched the filter. Check --interface argument."
                );
            } else {
                tracing::debug!(
                    filter = ?interface_filter,
                    matched = filtered.len(),
                    "Filtered interfaces by --interfaces argument"
                );
            }

            filtered
        };

        tracing::debug!(
            interface_count = interfaces.len(),
            "Enumerating network interfaces"
        );

        for interface in &interfaces {
            tracing::debug!(
                name = %interface.name,
                index = interface.index,
                is_up = interface.is_up(),
                is_loopback = interface.is_loopback(),
                mac = ?interface.mac,
                ips = ?interface.ips,
                flags = interface.flags,
                "Found interface"
            );
        }

        // First pass: collect all interface data and potential subnets
        let mut potential_subnets: Vec<(String, IpNetwork)> = Vec::new();
        let mut interface_data: Vec<(String, IpAddr, Option<MacAddress>)> = Vec::new();

        for interface in interfaces.into_iter() {
            let name = interface.name.clone();
            let mac_address = match interface.mac {
                Some(mac) if !mac.octets().iter().all(|o| *o == 0) => {
                    Some(MacAddress::new(mac.octets()))
                }
                _ => None,
            };

            for ip in interface.ips.iter() {
                interface_data.push((name.clone(), ip.ip(), mac_address));
                potential_subnets.push((name.clone(), *ip));
            }
        }

        // Second pass: create unique subnets from valid networks
        let mut subnet_map: HashMap<IpCidr, Subnet> = HashMap::new();

        for (interface_name, ip_network) in potential_subnets {
            if let Some(subnet) = Subnet::from_discovery(
                interface_name,
                &ip_network,
                daemon_id,
                &discovery_type,
                network_id,
            ) {
                subnet_map.entry(subnet.base.cidr).or_insert(subnet);
            }
        }

        // Third pass: assign all interfaces to appropriate subnets
        let mut interfaces = Vec::new();
        let mut cidr_to_mac = HashMap::new();

        for (interface_name, ip_addr, mac_address) in interface_data {
            // Find which subnet this IP belongs to
            if let Some(subnet) = subnet_map
                .values()
                .filter(|s| s.base.cidr.contains(&ip_addr))
                .max_by_key(|s| s.base.cidr.network_length())
            {
                cidr_to_mac
                    .entry(subnet.base.cidr)
                    .and_modify(|existing: &mut Option<MacAddress>| {
                        // Prefer a valid MAC over None
                        if existing.is_none() && mac_address.is_some() {
                            *existing = mac_address;
                        }
                    })
                    .or_insert(mac_address);

                interfaces.push(Interface::new(InterfaceBase {
                    network_id: subnet.base.network_id,
                    host_id: Uuid::nil(), // Placeholder - server will set correct host_id
                    name: Some(interface_name),
                    subnet_id: subnet.id,
                    ip_address: ip_addr,
                    mac_address,
                    position: interfaces.len() as i32,
                }));
            }
        }

        let subnets: Vec<Subnet> = subnet_map.into_values().collect();

        Ok((interfaces, subnets, cidr_to_mac))
    }

    async fn new_docker_client(
        &self,
        docker_proxy: Result<Option<String>, Error>,
        docker_proxy_ssl_info: Result<Option<(String, String, String)>, Error>,
    ) -> Result<Docker, Error> {
        use tokio::time::timeout;

        const DOCKER_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

        tracing::debug!("Creating Docker client connection");
        let start = std::time::Instant::now();

        let client = if let Ok(Some(docker_proxy)) = docker_proxy {
            tracing::debug!(proxy = %docker_proxy, "Using Docker proxy");
            if docker_proxy.contains("https://")
                && let Ok(Some((cert, key, chain))) = docker_proxy_ssl_info
            {
                let cert_path = PathBuf::from(cert);
                let key_path = PathBuf::from(key);
                let chain_path = PathBuf::from(chain);

                Docker::connect_with_ssl(
                    &docker_proxy,
                    &key_path,
                    &cert_path,
                    &chain_path,
                    15,
                    API_DEFAULT_VERSION,
                )
                .map_err(|e| anyhow::anyhow!("Failed to connect to Docker: {}", e))?
            } else {
                Docker::connect_with_http(&docker_proxy, 4, API_DEFAULT_VERSION)
                    .map_err(|e| anyhow::anyhow!("Failed to connect to Docker: {}", e))?
            }
        } else {
            tracing::debug!("Using Docker local defaults");
            Docker::connect_with_local_defaults()
                .map_err(|e| anyhow::anyhow!("Failed to connect to Docker: {}", e))?
        };

        // Ping Docker with retry and exponential backoff
        const MAX_PING_ATTEMPTS: u32 = 3;
        let mut last_error = None;
        for attempt in 1..=MAX_PING_ATTEMPTS {
            match timeout(DOCKER_CONNECT_TIMEOUT, client.ping()).await {
                Ok(Ok(_)) => {
                    tracing::debug!(
                        elapsed_ms = start.elapsed().as_millis(),
                        attempt,
                        "Docker client connected successfully"
                    );
                    return Ok(client);
                }
                Ok(Err(e)) => {
                    last_error = Some(format!("Docker ping failed: {}", e));
                    if attempt < MAX_PING_ATTEMPTS {
                        let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                        tracing::warn!(
                            attempt,
                            backoff_ms = backoff.as_millis(),
                            error = %e,
                            "Docker ping failed, retrying"
                        );
                        tokio::time::sleep(backoff).await;
                    }
                }
                Err(_) => {
                    last_error = Some(format!(
                        "Docker connection timed out after {:?}",
                        DOCKER_CONNECT_TIMEOUT
                    ));
                    if attempt < MAX_PING_ATTEMPTS {
                        let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                        tracing::warn!(
                            attempt,
                            backoff_ms = backoff.as_millis(),
                            "Docker ping timed out, retrying"
                        );
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }
        tracing::warn!(
            elapsed_ms = start.elapsed().as_millis(),
            "Docker ping failed after {} attempts",
            MAX_PING_ATTEMPTS
        );
        Err(anyhow::anyhow!(
            last_error.unwrap_or_else(|| "Docker connection failed".to_string())
        ))
    }

    async fn get_subnets_from_docker_networks(
        &self,
        daemon_id: Uuid,
        network_id: Uuid,
        client: &Docker,
        discovery_type: DiscoveryType,
    ) -> Result<Vec<Subnet>, Error> {
        let subnets: Vec<Subnet> = client
            .list_networks(None::<ListNetworksOptions>)
            .await?
            .into_iter()
            .filter_map(|n| {
                let driver = n.driver.as_deref().unwrap_or("bridge");

                // Include Docker networks that can be scanned
                // Skip: host (no separate CIDR), none (no networking), null (invalid)
                let subnet_type = match driver {
                    "bridge" | "overlay" => SubnetType::DockerBridge,
                    "macvlan" => SubnetType::MacVlan,
                    "ipvlan" => SubnetType::IpVlan,
                    _ => {
                        tracing::trace!(
                            network_name = ?n.name,
                            driver = driver,
                            "Skipping unsupported Docker network driver"
                        );
                        return None;
                    }
                };

                let network_name = n.name.clone().unwrap_or("Unknown Network".to_string());
                n.ipam.clone().map(|ipam| (network_name, ipam, subnet_type))
            })
            .filter_map(|(network_name, ipam, subnet_type)| {
                ipam.config
                    .map(|config| (network_name, config, subnet_type))
            })
            .flat_map(|(network_name, configs, subnet_type)| {
                configs
                    .iter()
                    .filter_map(|c| {
                        if let Some(cidr) = &c.subnet {
                            return Some(Subnet::new(SubnetBase {
                                cidr: IpCidr::from_str(cidr).ok()?,
                                description: None,
                                tags: Vec::new(),
                                network_id,
                                name: network_name.clone(),
                                subnet_type,
                                source: EntitySource::Discovery {
                                    metadata: vec![DiscoveryMetadata::new(
                                        discovery_type.clone(),
                                        daemon_id,
                                    )],
                                },
                            }));
                        }
                        None
                    })
                    .collect::<Vec<Subnet>>()
            })
            .collect();

        Ok(subnets)
    }

    async fn get_own_routing_table_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        let routing_handle = Handle::new()?;
        let routes = routing_handle.list().await?;

        Ok(routes
            .into_iter()
            .filter_map(|r| match r.gateway {
                Some(gateway) if gateway != r.destination => Some(gateway),
                _ => None,
            })
            .collect())
    }

    /// Get optimal concurrency for ARP scanning (OS-specific due to BPF limits on macOS)
    fn get_optimal_arp_concurrency(&self) -> Result<usize, Error>;

    /// Get optimal concurrency for deep port scanning.
    ///
    /// # Arguments
    /// * `port_batch_size` - Number of ports scanned concurrently per host in deep scan
    /// * `arp_subnet_count` - Number of ARP datalink channels currently open (2 FDs each)
    fn get_optimal_deep_scan_concurrency(
        &self,
        port_batch_size: usize,
        arp_subnet_count: usize,
    ) -> Result<usize, Error>;
}

/// Merge host (physical) and Docker subnets, giving host subnets precedence.
/// - Host subnets are always kept
/// - Docker subnets with CIDRs matching host subnets are dropped (host wins)
///
/// Callers that don't want DockerBridge subnets (e.g., self-report) should
/// filter them separately after calling this function.
pub fn merge_host_and_docker_subnets(
    host_subnets: Vec<Subnet>,
    docker_subnets: Vec<Subnet>,
) -> Vec<Subnet> {
    let host_cidrs: HashSet<IpCidr> = host_subnets.iter().map(|s| s.base.cidr).collect();

    let filtered_docker: Vec<Subnet> = docker_subnets
        .into_iter()
        .filter(|s| {
            let dominated_by_host = host_cidrs.contains(&s.base.cidr);
            if dominated_by_host {
                tracing::debug!(
                    cidr = %s.base.cidr,
                    "Filtering out Docker subnet (host takes precedence)"
                );
            }
            !dominated_by_host
        })
        .collect();

    [host_subnets, filtered_docker].concat()
}

#[cfg(target_os = "linux")]
use crate::daemon::utils::linux::LinuxDaemonUtils;
#[cfg(target_os = "linux")]
pub type PlatformDaemonUtils = LinuxDaemonUtils;

#[cfg(target_os = "macos")]
use crate::daemon::utils::macos::MacOsDaemonUtils;
#[cfg(target_os = "macos")]
pub type PlatformDaemonUtils = MacOsDaemonUtils;

#[cfg(target_family = "windows")]
use crate::daemon::utils::windows::WindowsDaemonUtils;
#[cfg(target_family = "windows")]
pub type PlatformDaemonUtils = WindowsDaemonUtils;

pub fn create_system_utils() -> PlatformDaemonUtils {
    PlatformDaemonUtils::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::shared::types::entities::EntitySource;
    use std::str::FromStr;

    fn make_subnet(cidr: &str, subnet_type: SubnetType) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: IpCidr::from_str(cidr).unwrap(),
            network_id: Uuid::nil(),
            name: String::new(),
            description: None,
            subnet_type,
            source: EntitySource::Manual,
            tags: Vec::new(),
        })
    }

    #[test]
    fn macvlan_overlap_keeps_physical_only() {
        let host = vec![make_subnet("192.168.1.0/24", SubnetType::Lan)];
        let docker = vec![make_subnet("192.168.1.0/24", SubnetType::MacVlan)];

        let result = merge_host_and_docker_subnets(host, docker);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].base.subnet_type, SubnetType::Lan);
    }

    #[test]
    fn docker_bridge_kept_when_no_overlap() {
        let host = vec![];
        let docker = vec![make_subnet("172.17.0.0/16", SubnetType::DockerBridge)];

        let result = merge_host_and_docker_subnets(host, docker);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].base.subnet_type, SubnetType::DockerBridge);
    }

    #[test]
    fn no_overlap_keeps_both() {
        let host = vec![make_subnet("192.168.1.0/24", SubnetType::Lan)];
        let docker = vec![make_subnet("10.0.0.0/8", SubnetType::IpVlan)];

        let result = merge_host_and_docker_subnets(host, docker);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn mixed_filters_only_overlapping_docker() {
        let host = vec![make_subnet("192.168.1.0/24", SubnetType::Lan)];
        let docker = vec![
            make_subnet("192.168.1.0/24", SubnetType::MacVlan),
            make_subnet("172.17.0.0/16", SubnetType::DockerBridge),
            make_subnet("10.0.0.0/8", SubnetType::IpVlan),
        ];

        let result = merge_host_and_docker_subnets(host, docker);
        // MacVlan dropped (overlaps host), Bridge + IpVlan kept (no overlap)
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].base.subnet_type, SubnetType::Lan);
        assert_eq!(result[1].base.subnet_type, SubnetType::DockerBridge);
        assert_eq!(result[2].base.subnet_type, SubnetType::IpVlan);
    }
}
