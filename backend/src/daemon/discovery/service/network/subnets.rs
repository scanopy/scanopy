use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use anyhow::Error;
use futures::future::try_join_all;
use tokio_util::sync::CancellationToken;

use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{DaemonUtils, PlatformDaemonUtils};
use crate::server::daemons::r#impl::base::DaemonMode;
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
    /// Every subnet the server holds for this network, whether or not this run sweeps it.
    ///
    /// Distinct from `subnets`, and the two must not be conflated. `subnets` is scan *scope*:
    /// what gets swept, and what "this credential targets an address we never contacted" is
    /// judged against. This is the network's *address space*, which an integration needs to
    /// place a device it learned about from somewhere other than the sweep — a UniFi
    /// controller reports switches on subnets a rescan of the controller never touches, and
    /// host identity is IP-based, so a device that cannot be placed in a subnet cannot be
    /// deduplicated and is dropped.
    pub network_subnets: Vec<Subnet>,
}

impl NetworkScan {
    /// Network-phase target resolution: either the subnets to sweep, or the
    /// specific addresses a rescan is verifying.
    /// The network's subnets, as sent with the run, or asked for when the server was too old to
    /// send them.
    ///
    /// Only a scan that names subnets by id needs these: a sweep reads the daemon's own
    /// interfaces, and creates what it finds. Asking is a DaemonPoll-only fallback, since a
    /// ServerPoll daemon has no server URL to ask with; there, an empty list is as far as this
    /// run gets, and the error says so rather than reporting a connection it never tried.
    async fn network_subnets(&self, ops: &DiscoveryOps) -> Result<Vec<Subnet>, Error> {
        if !self.known_subnets.is_empty() {
            return Ok(self.known_subnets.clone());
        }
        if ops.config_store.get_mode().await? == DaemonMode::ServerPoll {
            return Err(anyhow::anyhow!(
                "The server sent no subnets with this scan, and a ServerPoll daemon cannot ask \
                 for them. Upgrade the server, or scan the daemon's own subnets instead of \
                 naming them."
            ));
        }
        ops.api_client
            .get("/api/v1/subnets", "Failed to get subnets")
            .await
    }

    pub async fn resolve_scan_subnets(
        &self,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
        cancel: &CancellationToken,
    ) -> Result<ResolvedScanTargets, Error> {
        let network_id = ops
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        // A rescan names its targets, so resolve each to the subnet that holds it.
        if let Some(target_ips) = &self.target_ips {
            return self
                .resolve_rescan_targets(target_ips, ops, utils, network_id)
                .await;
        }

        // A scan that names subnets cannot run without them. A sweep only uses them to place
        // what integrations report, so it carries on with what it has.
        let network_subnets = match self.network_subnets(ops).await {
            Ok(subnets) => subnets,
            Err(e) if self.subnet_ids.is_some() => return Err(e),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Scanning without the network's subnet list; integrations will place what \
                     they find by the subnets this scan creates"
                );
                Vec::new()
            }
        };

        // Target specific subnets if provided in discovery type
        let subnets = if let Some(subnet_ids) = &self.subnet_ids {
            network_subnets
                .iter()
                .filter(|s| subnet_ids.contains(&s.id))
                .cloned()
                .collect()

        // Target all interfaced subnets if not
        } else {
            let interface_filter = ops.config_store.get_interfaces().await?;
            let (_, subnets, _) = utils
                .get_own_interfaces(network_id, &interface_filter)
                .await?;

            // Filter out docker bridge subnets (handled in docker discovery).
            // Size filtering for non-interfaced subnets is done later in
            // scan_and_process_hosts() where subnet_cidr_to_mac is available.
            let subnets: Vec<Subnet> = subnets
                .into_iter()
                .filter(|s| {
                    if s.is_container_bridge_subnet() {
                        tracing::warn!("Skipping {} with CIDR {}, container bridge subnets are scanned in container discovery", s.base.name, s.base.cidr);
                        return false
                    }

                    true
                })
                .collect();
            let subnet_futures = subnets
                .iter()
                .map(|subnet| ops.create_subnet(subnet, cancel));
            try_join_all(subnet_futures).await?
        };

        Ok(ResolvedScanTargets {
            subnets,
            target_ips: None,
            network_subnets,
        })
    }

    /// Map each rescan target to the interfaced subnet containing it.
    ///
    /// Targets that can't be resolved are dropped with a warning rather than
    /// failing the session — one address this daemon can't reach must not cost a
    /// host its rescan when the rest are perfectly scannable. The session fails
    /// only when *nothing* resolves.
    async fn resolve_rescan_targets(
        &self,
        target_ips: &HashSet<IpAddr>,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
        network_id: uuid::Uuid,
    ) -> Result<ResolvedScanTargets, Error> {
        let interface_filter = ops.config_store.get_interfaces().await?;
        let (_, _, subnet_cidr_to_mac) = utils
            .get_own_interfaces(network_id, &interface_filter)
            .await?;

        let all_subnets = self.network_subnets(ops).await?;

        let resolution = resolve_rescan_subnets(target_ips, &subnet_cidr_to_mac, &all_subnets);

        for (ip, subnet, reach) in &resolution.attributions {
            tracing::info!(
                target = %ip,
                subnet = %subnet,
                reach = %reach,
                "Resolved rescan target to its containing interfaced subnet"
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
            network_subnets: all_subnets,
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
    /// No interface of this daemon sits on a subnet containing the address, so
    /// there is no route to it. Normally the server's precondition has already
    /// filtered these out; reaching here means its junction is stale.
    NoInterfacedSubnet,
    /// An interface covers the address but the server has no subnet record for
    /// that CIDR, so there is nothing to attribute discovered hosts to.
    NoSubnetRecord(cidr::IpCidr),
}

impl std::fmt::Display for RescanSkip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loopback => write!(f, "loopback addresses are reached locally, not scanned"),
            Self::NoInterfacedSubnet => {
                write!(f, "this daemon has no interface on a subnet containing it")
            }
            Self::NoSubnetRecord(cidr) => write!(f, "no subnet record exists for {cidr}"),
        }
    }
}

/// The scannable half of a rescan's target set, plus what was dropped and why.
pub(crate) struct RescanResolution {
    /// The interfaced subnets the surviving targets will be scanned from.
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
}

impl std::fmt::Display for RescanReach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arp => write!(f, "arp"),
            Self::TcpOnly => write!(f, "tcp-only (no MAC on the containing interface)"),
        }
    }
}

/// Resolve rescan targets to the interfaced subnets they'll be scanned from.
///
/// Pure: the caller supplies the daemon's live interfaces and the server's
/// subnet records. Unresolvable targets are reported in `skipped` instead of
/// aborting, so a mixed host (a real address plus a loopback) still rescans.
///
/// A containing interface with a MAC always wins over one without, so ARP is
/// used wherever it exists and the TCP fallback engages only for an address
/// that genuinely cannot be ARP'd.
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

        // Prefer an interface that can ARP. Falling back to a MAC-less one is
        // what lets a VPN/tunnel-only daemon rescan at all — every address it
        // routes to sits on a point-to-point interface with no MAC, and
        // refusing those made rescan impossible rather than merely lower
        // fidelity.
        let (containing, reach) = match longest_prefix_containing(
            subnet_cidr_to_mac
                .iter()
                .filter(|(_, mac)| mac.is_some())
                .map(|(cidr, _)| *cidr),
            ip,
        ) {
            Some(cidr) => (Some(cidr), RescanReach::Arp),
            None => (
                longest_prefix_containing(subnet_cidr_to_mac.keys().copied(), ip),
                RescanReach::TcpOnly,
            ),
        };

        let Some(containing_cidr) = containing else {
            resolution
                .skipped
                .push((ip, RescanSkip::NoInterfacedSubnet));
            continue;
        };

        let Some(parent) = all_subnets
            .iter()
            .find(|s| *s.base.cidr == containing_cidr)
            .cloned()
        else {
            resolution
                .skipped
                .push((ip, RescanSkip::NoSubnetRecord(containing_cidr)));
            continue;
        };

        resolution.resolved.insert(ip);
        resolution.attributions.push((ip, containing_cidr, reach));
        if !resolution.subnets.iter().any(|s| s.id == parent.id) {
            resolution.subnets.push(parent);
        }
    }

    resolution
}

/// The most specific of `cidrs` that contains `ip`.
///
/// Longest prefix wins, matching the server's dangling-subnet repair
/// (`resolve_dangling_subnet_id`): where a daemon has overlapping interfaces,
/// the narrower one is the interface that will actually ARP the address.
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
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
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
        // No match means the address can't be ARP'd, so the rescan skips it
        // rather than silently scanning it without ARP.
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

    /// A target no interface covers at all, or one whose subnet the server has no record of,
    /// is dropped on its own — a stale interfaced-subnet junction must not cost the others.
    #[test]
    fn one_unresolvable_target_does_not_drop_its_siblings() {
        let interfaces = HashMap::from([
            (cidr("10.0.5.0/24"), a_mac()),
            (cidr("192.168.1.0/24"), a_mac()),
        ]);
        // No server record for 192.168.1.0/24, and nothing at all covering 172.16.0.4.
        let records = vec![subnet("10.0.5.0/24")];

        let resolution = resolve_rescan_subnets(
            &targets(&["10.0.5.7", "172.16.0.4", "192.168.1.9"]),
            &interfaces,
            &records,
        );

        assert_eq!(resolution.resolved, targets(&["10.0.5.7"]));
        assert_eq!(
            resolution.skipped,
            vec![
                (ip("172.16.0.4"), RescanSkip::NoInterfacedSubnet),
                (
                    ip("192.168.1.9"),
                    RescanSkip::NoSubnetRecord(cidr("192.168.1.0/24"))
                ),
            ]
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
