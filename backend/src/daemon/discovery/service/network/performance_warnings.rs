//! The scan-performance warnings a network scan raises about its own targets.
//!
//! Each is a pure function over the subnets and settings the scan already holds, so the condition
//! and the warning it produces are testable without interfaces, sockets or a clock.

use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use uuid::Uuid;

use super::is_host_address;
use super::scan::{count_scan_ips, scan_addresses};
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::subnets::r#impl::base::Subnet;

/// Scan time on one subnet without an interface past which the scan says so.
pub(super) const SLOW_SUBNET_THRESHOLD: Duration = Duration::from_secs(15 * 60);

/// The ARP targets for one interfaced subnet, at most `max` of them, and the warning when the
/// subnet holds more.
pub(super) fn arp_targets(
    subnet: &Subnet,
    target_ips: Option<&HashSet<IpAddr>>,
    max: usize,
    cutoff: u8,
) -> (Vec<Ipv4Addr>, Option<DiscoveryWarning>) {
    let mut addresses =
        scan_addresses(&subnet.base.cidr, target_ips).filter_map(|addr| match addr {
            IpAddr::V4(ipv4) => Some(ipv4),
            IpAddr::V6(_) => None,
        });
    let targets: Vec<Ipv4Addr> = addresses.by_ref().take(max).collect();
    // Reaching the cutoff is not truncation: a subnet exactly the size the cutoff allows is
    // swept whole. Only an address left over means part of it was skipped.
    let warning = addresses
        .next()
        .is_some()
        .then(|| DiscoveryWarning::ArpSweepTruncated {
            subnet_id: Some(subnet.id),
            cidr: subnet.base.cidr.to_string(),
            addresses_swept: targets.len() as u64,
            subnet_addresses: count_scan_ips(std::slice::from_ref(subnet), target_ips),
            cutoff,
        });
    (targets, warning)
}

/// The ping sweep's one flat target list across `subnets`, at most `max` addresses, and the
/// warning naming the subnets it cut short or never reached.
///
/// Network and broadcast addresses are left out: an echo request to either is answered by every
/// responder on the segment at once. See [`is_host_address`].
pub(super) fn icmp_targets(
    subnets: &[Subnet],
    target_ips: Option<&HashSet<IpAddr>>,
    max: usize,
    cutoff: u8,
) -> (Vec<Ipv4Addr>, Option<DiscoveryWarning>) {
    let mut targets: Vec<Ipv4Addr> = Vec::new();
    let mut cidrs_left_out: Vec<String> = Vec::new();

    for subnet in subnets {
        let mut addresses = scan_addresses(&subnet.base.cidr, target_ips)
            .filter(|addr| is_host_address(&subnet.base.cidr, *addr))
            .filter_map(|addr| match addr {
                IpAddr::V4(ipv4) => Some(ipv4),
                IpAddr::V6(_) => None,
            });
        let room = max - targets.len();
        targets.extend(addresses.by_ref().take(room));
        if addresses.next().is_some() {
            cidrs_left_out.push(subnet.base.cidr.to_string());
        }
    }

    let warning = (!cidrs_left_out.is_empty()).then(|| DiscoveryWarning::IcmpSweepTruncated {
        addresses_pinged: targets.len() as u64,
        cutoff,
        cidrs_left_out,
    });
    (targets, warning)
}

/// One warning per container bridge network among a network scan's targets.
///
/// Docker and Podman bridges only. A MacVlan or IpVlan network puts its containers on the real
/// LAN, where a network scan is how they are meant to be found.
pub(super) fn container_subnet_warnings(subnets: &[Subnet]) -> Vec<DiscoveryWarning> {
    subnets
        .iter()
        .filter(|subnet| subnet.is_container_bridge_subnet())
        .map(|subnet| DiscoveryWarning::ContainerSubnetScanned {
            subnet_id: Some(subnet.id),
            cidr: subnet.base.cidr.to_string(),
        })
        .collect()
}

/// Scan time spent per subnet the daemon has no interface on.
///
/// The deep-scan loop interleaves addresses from every subnet, so the time one subnet took is not
/// the span between its first and last address. Each address's own scan time is summed instead,
/// and divided by how many addresses were scanned at once to give the share of the run's
/// wall-clock time the subnet used.
#[derive(Default)]
pub(super) struct SubnetProbeTimes {
    by_subnet: HashMap<Uuid, SubnetProbeTime>,
}

struct SubnetProbeTime {
    cidr: String,
    addresses: u64,
    scan_time: Duration,
}

impl SubnetProbeTimes {
    /// Record one address in `subnet` whose scan took `elapsed`.
    pub(super) fn record(&mut self, subnet: &Subnet, elapsed: Duration) {
        let entry = self
            .by_subnet
            .entry(subnet.id)
            .or_insert_with(|| SubnetProbeTime {
                cidr: subnet.base.cidr.to_string(),
                addresses: 0,
                scan_time: Duration::ZERO,
            });
        entry.addresses += 1;
        entry.scan_time += elapsed;
    }

    /// One warning per subnet that used at least [`SLOW_SUBNET_THRESHOLD`] of the run, slowest
    /// first. `concurrency` is how many addresses the run scanned at once.
    pub(super) fn warnings(&self, concurrency: usize) -> Vec<DiscoveryWarning> {
        let concurrency = u32::try_from(concurrency.max(1)).unwrap_or(u32::MAX);
        let mut slow: Vec<(u32, &Uuid, &SubnetProbeTime)> = self
            .by_subnet
            .iter()
            .filter_map(|(id, time)| {
                let share = time.scan_time / concurrency;
                (share >= SLOW_SUBNET_THRESHOLD).then(|| {
                    let minutes = u32::try_from(share.as_secs() / 60).unwrap_or(u32::MAX);
                    (minutes, id, time)
                })
            })
            .collect();
        slow.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cidr.cmp(&b.2.cidr)));
        slow.into_iter()
            .map(
                |(minutes, id, time)| DiscoveryWarning::SubnetProbedWithoutInterface {
                    subnet_id: Some(*id),
                    cidr: time.cidr.clone(),
                    addresses: time.addresses,
                    minutes,
                },
            )
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::shared::types::entities::EntitySource;
    use crate::server::subnets::r#impl::base::{
        SubnetBase, SubnetCidr, SubnetCidrValue, SubnetName, SubnetTypeAttributed,
    };
    use crate::server::subnets::r#impl::types::SubnetType;
    use std::str::FromStr;

    fn subnet_of(cidr: &str, subnet_type: SubnetType) -> Subnet {
        Subnet::new(SubnetBase {
            vlan_ids: Vec::new(),
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr::IpCidr::from_str(cidr).unwrap()),
                AttributeSource::DaemonSelfReport,
            ),
            site_id: Uuid::new_v4(),
            name: SubnetName::unattributed(cidr.to_string()),
            description: None,
            subnet_type: SubnetTypeAttributed::unattributed(subnet_type),
            virtualization_service_id: None,
            source: EntitySource::System,
            tags: Vec::new(),
        })
    }

    fn subnet(cidr: &str) -> Subnet {
        subnet_of(cidr, SubnetType::Lan)
    }

    /// A /24 cutoff holds 256 addresses.
    const SLASH_24: usize = 256;

    #[test]
    fn a_subnet_larger_than_the_cutoff_is_swept_up_to_it_and_says_so() {
        let large = subnet("10.0.4.0/23");
        let (targets, warning) = arp_targets(&large, None, SLASH_24, 24);

        assert_eq!(targets.len(), 256);
        assert_eq!(
            warning,
            Some(DiscoveryWarning::ArpSweepTruncated {
                subnet_id: Some(large.id),
                cidr: "10.0.4.0/23".to_string(),
                addresses_swept: 256,
                subnet_addresses: 512,
                cutoff: 24,
            })
        );
    }

    /// The old check warned as soon as the list reached the cutoff, which a subnet of exactly
    /// that size does with nothing left out.
    #[test]
    fn a_subnet_the_size_of_the_cutoff_is_swept_whole_without_a_warning() {
        let (targets, warning) = arp_targets(&subnet("10.0.4.0/24"), None, SLASH_24, 24);

        assert_eq!(targets.len(), 256);
        assert_eq!(warning, None);
    }

    #[test]
    fn a_rescan_of_named_addresses_in_a_large_subnet_is_not_truncated() {
        let targets_in: HashSet<IpAddr> = ["10.0.5.9", "10.0.4.7"]
            .iter()
            .map(|s| IpAddr::from_str(s).unwrap())
            .collect();
        let (targets, warning) =
            arp_targets(&subnet("10.0.4.0/23"), Some(&targets_in), SLASH_24, 24);

        assert_eq!(targets.len(), 2);
        assert_eq!(warning, None);
    }

    /// Each /24 holds 254 host addresses, so the list fills partway through the second subnet
    /// and never reaches the third.
    #[test]
    fn a_ping_sweep_past_the_cutoff_names_the_subnets_it_left_out() {
        let subnets = [
            subnet("10.0.1.0/24"),
            subnet("10.0.2.0/24"),
            subnet("10.0.3.0/24"),
        ];
        let (targets, warning) = icmp_targets(&subnets, None, SLASH_24, 24);

        assert_eq!(targets.len(), 256);
        assert_eq!(
            warning,
            Some(DiscoveryWarning::IcmpSweepTruncated {
                addresses_pinged: 256,
                cutoff: 24,
                cidrs_left_out: vec!["10.0.2.0/24".to_string(), "10.0.3.0/24".to_string()],
            })
        );
    }

    #[test]
    fn a_ping_sweep_inside_the_cutoff_covers_every_subnet_without_a_warning() {
        let subnets = [subnet("10.0.1.0/25"), subnet("10.0.2.0/25")];
        let (targets, warning) = icmp_targets(&subnets, None, SLASH_24, 24);

        assert_eq!(targets.len(), 126 + 126);
        assert_eq!(warning, None);
    }

    #[test]
    fn container_bridges_among_the_targets_each_get_a_warning() {
        let docker = subnet_of("172.17.0.0/16", SubnetType::DockerBridge);
        let podman = subnet_of("10.88.0.0/16", SubnetType::PodmanBridge);
        let subnets = [
            subnet("10.0.1.0/24"),
            docker.clone(),
            subnet_of("10.0.9.0/24", SubnetType::MacVlan),
            podman.clone(),
        ];

        assert_eq!(
            container_subnet_warnings(&subnets),
            vec![
                DiscoveryWarning::ContainerSubnetScanned {
                    subnet_id: Some(docker.id),
                    cidr: "172.17.0.0/16".to_string(),
                },
                DiscoveryWarning::ContainerSubnetScanned {
                    subnet_id: Some(podman.id),
                    cidr: "10.88.0.0/16".to_string(),
                },
            ]
        );
    }

    #[test]
    fn a_scan_of_lan_subnets_raises_no_container_warning() {
        let subnets = [subnet("10.0.1.0/24"), subnet("10.0.2.0/24")];
        assert!(container_subnet_warnings(&subnets).is_empty());
    }

    /// 2,000 addresses at 3s each is 100 minutes of scanning. Four at a time, that is 25 minutes
    /// of the run.
    #[test]
    fn a_subnet_that_used_more_than_the_threshold_is_reported_with_its_share_of_the_run() {
        let slow = subnet("10.9.0.0/21");
        let quick = subnet("10.0.1.0/24");
        let mut times = SubnetProbeTimes::default();
        for _ in 0..2000 {
            times.record(&slow, Duration::from_secs(3));
        }
        for _ in 0..254 {
            times.record(&quick, Duration::from_secs(3));
        }

        assert_eq!(
            times.warnings(4),
            vec![DiscoveryWarning::SubnetProbedWithoutInterface {
                subnet_id: Some(slow.id),
                cidr: "10.9.0.0/21".to_string(),
                addresses: 2000,
                minutes: 25,
            }]
        );
    }

    /// The same 100 minutes of scanning spread over ten addresses at once is 10 minutes of the
    /// run, under the threshold.
    #[test]
    fn a_subnet_under_the_threshold_raises_nothing() {
        let subnet = subnet("10.9.0.0/21");
        let mut times = SubnetProbeTimes::default();
        for _ in 0..2000 {
            times.record(&subnet, Duration::from_secs(3));
        }

        assert!(times.warnings(10).is_empty());
        assert!(SubnetProbeTimes::default().warnings(1).is_empty());
    }
}
