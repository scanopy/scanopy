use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use anyhow::Error;

use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{DaemonUtils, PlatformDaemonUtils};
use crate::server::subnets::r#impl::base::Subnet;

use super::NetworkScan;

/// Outcome of network-phase target resolution.
///
/// `target_ips` is `Some` only for a rescan, which names its addresses directly
/// rather than sweeping subnets. The subnets returned alongside are the real
/// ones containing those addresses, so everything downstream — the
/// interfaced/non-interfaced partition, the ARP source lookups, IP attribution —
/// works exactly as it does for a normal scan.
pub struct ResolvedScanTargets {
    pub subnets: Vec<Subnet>,
    pub target_ips: Option<HashSet<IpAddr>>,
    /// Every subnet the server holds for this site, whether or not this run sweeps it.
    ///
    /// Distinct from `subnets`, and the two must not be conflated. `subnets` is scan *scope*:
    /// what gets swept, and what "this credential targets an address we never contacted" is
    /// judged against. This is the site's *address space*, which an integration needs to
    /// place a device it learned about from somewhere other than the sweep — a UniFi
    /// controller reports switches on subnets a rescan of the controller never touches, and
    /// host identity is IP-based, so a device that cannot be placed in a subnet cannot be
    /// deduplicated and is dropped.
    pub site_subnets: Vec<Subnet>,
}

impl NetworkScan {
    /// Network-phase target resolution: either the subnets to sweep, or the
    /// specific addresses a rescan is verifying.
    /// The site's subnets, as sent with the run, or asked for when the server was too old to
    /// send them.
    ///
    /// Only a scan that names subnets by id needs these: a sweep reads the daemon's own
    /// interfaces, and creates what it finds. Asking is a DaemonPoll-only fallback, since a
    /// ServerPoll daemon has no server URL to ask with; there, an empty list is as far as this
    /// run gets, and the error says so rather than reporting a connection it never tried.
    async fn site_subnets(&self, ops: &DiscoveryOps) -> Result<Vec<Subnet>, Error> {
        if !self.known_subnets.is_empty() {
            return Ok(self.known_subnets.clone());
        }
        let Some(api_client) = &ops.api_client else {
            return Err(anyhow::anyhow!(
                "The server sent no subnets with this scan, and a ServerPoll daemon cannot ask \
                 for them. Upgrade the server, or scan the daemon's own subnets instead of \
                 naming them."
            ));
        };
        api_client
            .get("/api/v1/subnets", "Failed to get subnets")
            .await
    }

    pub async fn resolve_scan_subnets(
        &self,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
    ) -> Result<ResolvedScanTargets, Error> {
        let site_id = ops
            .config_store
            .get_site_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Site ID not set"))?;

        // A rescan names its targets, so resolve each to the subnet that holds it.
        if let Some(target_ips) = &self.target_ips {
            return self
                .resolve_rescan_targets(target_ips, ops, utils, site_id)
                .await;
        }

        // A scan that names subnets cannot run without them. A sweep only uses them to place
        // what integrations report, so it carries on with what it has.
        let site_subnets = match self.site_subnets(ops).await {
            Ok(subnets) => subnets,
            Err(e) if self.subnet_ids.is_some() => return Err(e),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Scanning without the site's subnet list; integrations will place what \
                     they find by the subnets this scan creates"
                );
                Vec::new()
            }
        };

        // Target specific subnets if provided in discovery type
        let subnets = if let Some(subnet_ids) = &self.subnet_ids {
            site_subnets
                .iter()
                .filter(|s| subnet_ids.contains(&s.id))
                .cloned()
                .collect()

        // Otherwise the subnets the session classified and created at its start, as the server
        // confirmed them. Size filtering for non-interfaced subnets is done later in
        // scan_and_process_hosts() where subnet_cidr_to_mac is available.
        } else {
            self.created_subnets.clone()
        };

        Ok(ResolvedScanTargets {
            subnets,
            target_ips: None,
            site_subnets,
        })
    }

    /// Map each rescan target to the site subnet it is scanned from.
    ///
    /// Targets that can't be resolved are dropped with a warning rather than
    /// failing the session, so one address with no subnet doesn't cost a host
    /// its rescan when the rest are scannable. The session fails only when
    /// *nothing* resolves. A resolved address that doesn't answer is a scan
    /// result (not found), not a failure.
    async fn resolve_rescan_targets(
        &self,
        target_ips: &HashSet<IpAddr>,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
        site_id: uuid::Uuid,
    ) -> Result<ResolvedScanTargets, Error> {
        let interface_filter = ops.config_store.get_interfaces().await?;
        let (_, _, subnet_cidr_to_mac) =
            utils.get_own_interfaces(site_id, &interface_filter).await?;

        let all_subnets = self.site_subnets(ops).await?;

        let resolution = resolve_rescan_subnets(target_ips, &subnet_cidr_to_mac, &all_subnets);

        for (ip, subnet, reach) in &resolution.attributions {
            tracing::info!(
                target = %ip,
                subnet = %subnet,
                reach = %reach,
                "Resolved rescan target to its containing subnet"
            );
        }
        for (ip, skip) in &resolution.skipped {
            tracing::warn!(target = %ip, reason = %skip, "Skipping rescan target");
        }

        if resolution.subnets.is_empty() {
            let reasons = resolution
                .skipped
                .iter()
                .map(|(ip, skip)| format!("{ip} ({skip})"))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(anyhow::anyhow!(
                "Cannot rescan this host: none of its addresses are scannable by this daemon — {reasons}"
            ));
        }

        Ok(ResolvedScanTargets {
            subnets: resolution.subnets,
            target_ips: Some(resolution.resolved),
            site_subnets: all_subnets,
        })
    }
}

/// Why a rescan target was left out of the scan.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RescanSkip {
    /// A loopback address is reachable without ARP and has no interfaced subnet
    /// to be scanned from — and loopback subnets are dropped from every scan
    /// anyway (see `scan_and_process_hosts`), so resolving one buys nothing.
    Loopback,
    /// Only a container bridge subnet holds the address. Sweeps leave those to
    /// the container scan, and so does a rescan.
    ContainerBridge,
    /// No subnet the server holds for this site contains the address, so a host
    /// found there could not be attributed to one. The server sends only
    /// addresses that have a subnet, so reaching here means the subnet was
    /// deleted after the rescan started.
    NoSubnet,
}

impl std::fmt::Display for RescanSkip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loopback => write!(f, "loopback addresses are reached locally, not scanned"),
            Self::ContainerBridge => write!(
                f,
                "container bridge addresses are scanned by the container scan"
            ),
            Self::NoSubnet => write!(f, "no subnet on this site contains it"),
        }
    }
}

/// The scannable half of a rescan's target set, plus what was dropped and why.
pub(crate) struct RescanResolution {
    /// The subnets the surviving targets will be scanned from.
    pub subnets: Vec<Subnet>,
    /// The targets that resolved — narrower than the requested set, so progress
    /// budgeting doesn't count addresses that will never be scanned.
    pub resolved: HashSet<IpAddr>,
    /// `(target, containing CIDR, how it will be reached)` per resolved target.
    pub attributions: Vec<(IpAddr, cidr::IpCidr, RescanReach)>,
    pub skipped: Vec<(IpAddr, RescanSkip)>,
}

/// How a resolved rescan target will actually be reached.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RescanReach {
    /// The daemon has a MAC on the containing subnet, so the address is ARP'd —
    /// the high-fidelity path, which sees a live host even when every port is
    /// firewalled.
    Arp,
    /// The containing interface has no MAC (a point-to-point tunnel, say), so
    /// there is nothing to ARP. The address is still routable, so it takes the
    /// TCP-responsiveness path the scanner already applies to non-interfaced
    /// subnets. Lower fidelity: a live host answering on no port reads as
    /// unresponsive.
    TcpOnly,
    /// The daemon has no interface on the containing subnet and reaches it
    /// through a router (another VLAN, a remote site), probed by TCP the way a
    /// sweep probes a named subnet it has no interface on. Same fidelity as
    /// `TcpOnly`.
    Routed,
}

impl std::fmt::Display for RescanReach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arp => write!(f, "arp"),
            Self::TcpOnly => write!(f, "tcp-only (no MAC on the containing interface)"),
            Self::Routed => write!(f, "tcp-only (routed, no interface on the subnet)"),
        }
    }
}

/// Resolve rescan targets to the site subnets they'll be scanned from.
///
/// Pure: the caller supplies the daemon's live interfaces and the server's
/// subnet records. Unresolvable targets are reported in `skipped` instead of
/// aborting, so a mixed host (a real address plus a loopback) still rescans.
///
/// Each target takes the first of these that holds it:
/// 1. an interfaced subnet with a MAC, so ARP is used wherever it exists;
/// 2. a MAC-less interfaced subnet (a tunnel), probed by TCP;
/// 3. any other site subnet, reached through the daemon's routes and probed by
///    TCP. Container bridge subnets are left to the container scan here, as in
///    a sweep.
///
/// Within each, the narrowest subnet wins. `scan_and_process_hosts` partitions
/// by the same "has a MAC on this CIDR" test, so the reach reported here is the
/// path the scan takes.
pub(crate) fn resolve_rescan_subnets(
    target_ips: &HashSet<IpAddr>,
    subnet_cidr_to_mac: &HashMap<cidr::IpCidr, Option<mac_address::MacAddress>>,
    all_subnets: &[Subnet],
) -> RescanResolution {
    let mut resolution = RescanResolution {
        subnets: Vec::new(),
        resolved: HashSet::new(),
        attributions: Vec::new(),
        skipped: Vec::new(),
    };

    // Iterate in a stable order so logs and errors don't reshuffle per run.
    let mut ordered: Vec<IpAddr> = target_ips.iter().copied().collect();
    ordered.sort();

    for ip in ordered {
        // Loopback is the one address with no scannable form: it needs no ARP,
        // and loopback subnets are dropped from every scan downstream, so
        // resolving one could only ever produce an empty sweep.
        if ip.is_loopback() {
            resolution.skipped.push((ip, RescanSkip::Loopback));
            continue;
        }

        // The narrowest site subnet holding the address among those `eligible` admits.
        let narrowest = |eligible: &dyn Fn(&Subnet) -> bool| {
            let cidr = longest_prefix_containing(
                all_subnets
                    .iter()
                    .filter(|s| eligible(s))
                    .map(|s| *s.base.cidr),
                ip,
            )?;
            all_subnets
                .iter()
                .find(|s| *s.base.cidr == cidr && eligible(s))
        };
        let interfaced = |with_mac: bool| {
            narrowest(&|s: &Subnet| {
                subnet_cidr_to_mac
                    .get(&*s.base.cidr)
                    .is_some_and(|mac| mac.is_some() == with_mac)
            })
        };

        let found = interfaced(true)
            .map(|s| (s, RescanReach::Arp))
            .or_else(|| interfaced(false).map(|s| (s, RescanReach::TcpOnly)))
            .or_else(|| {
                narrowest(&|s: &Subnet| !s.is_container_bridge_subnet())
                    .map(|s| (s, RescanReach::Routed))
            });

        let Some((parent, reach)) = found else {
            let skip = if all_subnets.iter().any(|s| s.base.cidr.contains(&ip)) {
                RescanSkip::ContainerBridge
            } else {
                RescanSkip::NoSubnet
            };
            resolution.skipped.push((ip, skip));
            continue;
        };

        resolution.resolved.insert(ip);
        resolution.attributions.push((ip, *parent.base.cidr, reach));
        if !resolution.subnets.iter().any(|s| s.id == parent.id) {
            resolution.subnets.push(parent.clone());
        }
    }

    resolution
}

/// The most specific of `cidrs` that contains `ip`.
///
/// Longest prefix wins, matching the server's dangling-subnet repair
/// (`resolve_dangling_subnet_id`): where a daemon has overlapping interfaces,
/// the narrower one is the interface that will actually ARP the address, and
/// of overlapping routed subnets the narrower one is where the host lives.
pub(crate) fn longest_prefix_containing(
    cidrs: impl Iterator<Item = cidr::IpCidr>,
    ip: IpAddr,
) -> Option<cidr::IpCidr> {
    cidrs
        .filter(|cidr| cidr.contains(&ip))
        .max_by_key(|cidr| cidr.network_length())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue, SubnetTypeAttributed};
    use crate::server::subnets::r#impl::types::SubnetType;
    use std::str::FromStr;

    fn cidr(s: &str) -> cidr::IpCidr {
        cidr::IpCidr::from_str(s).unwrap()
    }

    fn ip(s: &str) -> IpAddr {
        IpAddr::from_str(s).unwrap()
    }

    #[test]
    fn picks_the_narrowest_containing_interface() {
        // A daemon interfaced on both a /16 and a /24 covering the target must
        // scan from the /24 — that is the interface on the target's link.
        let candidates = [
            cidr("10.0.0.0/16"),
            cidr("10.0.5.0/24"),
            cidr("10.0.9.0/24"),
        ];
        assert_eq!(
            longest_prefix_containing(candidates.into_iter(), ip("10.0.5.7")),
            Some(cidr("10.0.5.0/24"))
        );
    }

    #[test]
    fn no_containing_interface_is_not_a_match() {
        let candidates = [cidr("10.0.5.0/24"), cidr("192.168.1.0/24")];
        assert_eq!(
            longest_prefix_containing(candidates.into_iter(), ip("172.16.0.4")),
            None
        );
    }

    #[test]
    fn a_host_route_can_be_the_match() {
        let candidates = [cidr("10.0.5.0/24"), cidr("10.0.5.7/32")];
        assert_eq!(
            longest_prefix_containing(candidates.into_iter(), ip("10.0.5.7")),
            Some(cidr("10.0.5.7/32"))
        );
    }

    fn a_mac() -> Option<mac_address::MacAddress> {
        Some(mac_address::MacAddress::new([0, 1, 2, 3, 4, 5]))
    }

    fn subnet(cidr_str: &str) -> Subnet {
        Subnet {
            id: uuid::Uuid::new_v4(),
            base: crate::server::subnets::r#impl::base::SubnetBase {
                cidr: SubnetCidr::new(
                    SubnetCidrValue(cidr(cidr_str)),
                    AttributeSource::DaemonSelfReport,
                ),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn targets(ips: &[&str]) -> HashSet<IpAddr> {
        ips.iter().map(|s| ip(s)).collect()
    }

    /// The reported bug: a daemon host carries 127.0.0.1 alongside its real address, and the
    /// loopback — which has no MAC and so no interfaced subnet — used to fail the whole rescan.
    /// The real address must still be scanned.
    #[test]
    fn a_loopback_target_is_skipped_without_costing_the_rescan() {
        let interfaces =
            HashMap::from([(cidr("10.0.5.0/24"), a_mac()), (cidr("127.0.0.0/8"), None)]);
        let records = vec![subnet("10.0.5.0/24"), subnet("127.0.0.0/8")];

        let resolution =
            resolve_rescan_subnets(&targets(&["127.0.0.1", "10.0.5.7"]), &interfaces, &records);

        assert_eq!(resolution.resolved, targets(&["10.0.5.7"]));
        assert_eq!(
            resolution
                .subnets
                .iter()
                .map(|s| *s.base.cidr)
                .collect::<Vec<_>>(),
            vec![cidr("10.0.5.0/24")]
        );
        assert_eq!(
            resolution.skipped,
            vec![(ip("127.0.0.1"), RescanSkip::Loopback)]
        );
    }

    /// Nothing scannable is left, so the caller has an empty subnet set to fail on — a clear
    /// refusal rather than a scan that quietly covers no address.
    #[test]
    fn a_loopback_only_target_set_resolves_to_nothing() {
        let interfaces = HashMap::from([(cidr("127.0.0.0/8"), None)]);

        let resolution = resolve_rescan_subnets(
            &targets(&["127.0.0.1"]),
            &interfaces,
            &[subnet("127.0.0.0/8")],
        );

        assert!(resolution.subnets.is_empty());
        assert!(resolution.resolved.is_empty());
    }

    /// A MAC-less interface still carries a route, so its addresses are rescanned over the
    /// TCP path rather than refused. Without this a VPN/tunnel-only daemon — where every
    /// interface is point-to-point and has no MAC — could not rescan anything at all.
    #[test]
    fn a_tunnel_address_falls_back_to_the_tcp_path() {
        let interfaces = HashMap::from([(cidr("10.0.0.0/24"), None)]);

        let resolution = resolve_rescan_subnets(
            &targets(&["10.0.0.2"]),
            &interfaces,
            &[subnet("10.0.0.0/24")],
        );

        assert_eq!(resolution.resolved, targets(&["10.0.0.2"]));
        assert!(resolution.skipped.is_empty());
        assert_eq!(
            resolution.attributions,
            vec![(ip("10.0.0.2"), cidr("10.0.0.0/24"), RescanReach::TcpOnly)]
        );
    }

    /// ARP is not given up wherever it is available: an interface with a MAC wins over a
    /// MAC-less one covering the same address, so the fallback never silently downgrades a
    /// target that could have been ARP'd.
    #[test]
    fn an_arp_capable_interface_wins_over_a_mac_less_one() {
        let interfaces =
            HashMap::from([(cidr("10.0.0.0/24"), a_mac()), (cidr("10.0.0.0/16"), None)]);

        let resolution = resolve_rescan_subnets(
            &targets(&["10.0.0.2"]),
            &interfaces,
            &[subnet("10.0.0.0/24")],
        );

        assert_eq!(
            resolution.attributions,
            vec![(ip("10.0.0.2"), cidr("10.0.0.0/24"), RescanReach::Arp)]
        );
    }

    fn bridge(cidr_str: &str) -> Subnet {
        let mut s = subnet(cidr_str);
        s.base.subnet_type = SubnetTypeAttributed::unattributed(SubnetType::DockerBridge);
        s
    }

    /// A host on another VLAN, reached through a router: the daemon has no interface there,
    /// and the site's subnet holding it is scanned by TCP, as a named-subnet sweep would.
    #[test]
    fn a_routed_target_resolves_to_its_site_subnet_over_tcp() {
        let interfaces = HashMap::from([(cidr("192.168.1.0/24"), a_mac())]);
        let records = vec![subnet("192.168.1.0/24"), subnet("10.20.0.0/24")];

        let resolution = resolve_rescan_subnets(&targets(&["10.20.0.5"]), &interfaces, &records);

        assert_eq!(resolution.resolved, targets(&["10.20.0.5"]));
        assert!(resolution.skipped.is_empty());
        assert_eq!(
            resolution.attributions,
            vec![(ip("10.20.0.5"), cidr("10.20.0.0/24"), RescanReach::Routed)]
        );
    }

    /// An address no site subnet holds can't be attributed to one, so it is skipped rather
    /// than scanned as a /32 whose host the server would drop.
    #[test]
    fn a_target_in_no_subnet_is_skipped() {
        let interfaces = HashMap::from([(cidr("192.168.1.0/24"), a_mac())]);

        let resolution = resolve_rescan_subnets(
            &targets(&["172.16.0.4"]),
            &interfaces,
            &[subnet("192.168.1.0/24")],
        );

        assert!(resolution.subnets.is_empty());
        assert_eq!(
            resolution.skipped,
            vec![(ip("172.16.0.4"), RescanSkip::NoSubnet)]
        );
    }

    /// Container bridge subnets are scanned by the container scan, so a rescan doesn't route
    /// TCP probes into one.
    #[test]
    fn a_target_on_a_container_bridge_is_left_to_the_container_scan() {
        let resolution = resolve_rescan_subnets(
            &targets(&["172.17.0.3"]),
            &HashMap::new(),
            &[bridge("172.17.0.0/16")],
        );

        assert!(resolution.subnets.is_empty());
        assert_eq!(
            resolution.skipped,
            vec![(ip("172.17.0.3"), RescanSkip::ContainerBridge)]
        );
    }

    /// One host with an ARP'd address, a routed one, one in no subnet and a loopback: each
    /// resolves on its own, and the unresolvable ones don't cost the others.
    #[test]
    fn a_mixed_host_resolves_each_address_on_its_own() {
        let interfaces = HashMap::from([
            (cidr("192.168.1.0/24"), a_mac()),
            (cidr("127.0.0.0/8"), None),
        ]);
        let records = vec![
            subnet("192.168.1.0/24"),
            subnet("10.20.0.0/16"),
            subnet("10.20.5.0/24"),
            subnet("127.0.0.0/8"),
        ];

        let resolution = resolve_rescan_subnets(
            &targets(&["192.168.1.9", "10.20.5.7", "172.16.0.4", "127.0.0.1"]),
            &interfaces,
            &records,
        );

        assert_eq!(resolution.resolved, targets(&["192.168.1.9", "10.20.5.7"]));
        // The narrower of two routed subnets holding the address wins.
        assert_eq!(
            resolution.attributions,
            vec![
                (ip("10.20.5.7"), cidr("10.20.5.0/24"), RescanReach::Routed),
                (ip("192.168.1.9"), cidr("192.168.1.0/24"), RescanReach::Arp),
            ]
        );
        assert_eq!(
            resolution.skipped,
            vec![
                (ip("127.0.0.1"), RescanSkip::Loopback),
                (ip("172.16.0.4"), RescanSkip::NoSubnet),
            ]
        );
    }

    /// An interface on the address's link whose subnet the server has no record of no longer
    /// drops the target: a broader site subnet holding it is scanned by TCP instead.
    #[test]
    fn an_interface_without_a_record_falls_back_to_a_routed_subnet() {
        let interfaces = HashMap::from([(cidr("10.0.5.0/24"), a_mac())]);

        let resolution = resolve_rescan_subnets(
            &targets(&["10.0.5.7"]),
            &interfaces,
            &[subnet("10.0.0.0/16")],
        );

        assert_eq!(
            resolution.attributions,
            vec![(ip("10.0.5.7"), cidr("10.0.0.0/16"), RescanReach::Routed)]
        );
    }

    /// Several targets on one link share the subnet they're scanned from.
    #[test]
    fn targets_on_one_link_resolve_to_a_single_subnet() {
        let interfaces = HashMap::from([(cidr("10.0.5.0/24"), a_mac())]);

        let resolution = resolve_rescan_subnets(
            &targets(&["10.0.5.7", "10.0.5.8"]),
            &interfaces,
            &[subnet("10.0.5.0/24")],
        );

        assert_eq!(resolution.subnets.len(), 1);
        assert_eq!(resolution.resolved.len(), 2);
    }
}
