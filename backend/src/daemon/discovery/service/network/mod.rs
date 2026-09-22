pub mod arp;
pub mod dcp;
mod dns;
pub mod icmp;
pub mod mdns;
mod scan;
mod subnets;

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use cidr::IpCidr;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use mac_address::MacAddress;
use uuid::Uuid;

use crate::daemon::discovery::integration::IntegrationRegistry;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::daemon::utils::scanner::ScanConcurrencyController;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::HostNamingFallback;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::Service;
use crate::server::subnets::r#impl::base::Subnet;

/// Per-host data discovered during deep_scan_host().
/// Used by subsequent discovery phases (e.g., Docker container scanning) to link
/// containers to the correct virtualizing service and provide host ip_addresses.
#[derive(Debug, Clone, Default)]
pub struct DiscoveredHostData {
    pub docker_service_id: Option<Uuid>,
    pub ip_addresses: Vec<IPAddress>,
}

/// Grace period to wait for late ARP arrivals after the last deep scan completes
const LATE_ARRIVAL_GRACE_PERIOD: Duration = Duration::from_secs(30);

// The hard maximum duration for a single discovery run is now server-configurable
// via ScanSettings::max_discovery_duration (default 21600s = 6h); see scan.rs.

/// Maximum interval between progress reports (heartbeat even if progress unchanged)
const MAX_PROGRESS_REPORT_INTERVAL: Duration = Duration::from_secs(30);

// Progress phase weights (must sum to 100)
const PROGRESS_ARP_PHASE: u8 = 30; // 0-30%: ARP discovery
const PROGRESS_DEEP_SCAN_PHASE: u8 = 65; // 30-95%: Deep scanning
const PROGRESS_GRACE_PHASE: u8 = 5; // 95-100%: Grace period

/// Cost of a full port scan per host in centiseconds
const FULL_SCAN_COST_CS: usize = 9000; // ~90 seconds
/// Cost of a light scan per host in centiseconds
const LIGHT_SCAN_COST_CS: usize = 800; // ~8 seconds

/// Cost (centiseconds) of a non-interfaced IP's TCP responsiveness check (an ~2s
/// multi-port connect scan a dead IP still has to be probed with). Counted in the cost
/// model — seeded into total_cost up front and accrued into completed_cost as each
/// check finishes — so progress/ETA reflect draining a large non-interfaced range
/// instead of pinning at ~95%/"<1 min" while those IPs are still being checked.
const RESPONSIVENESS_COST_CS: usize = 200; // ~2 seconds

#[derive(Default)]
pub struct NetworkScan {
    subnet_ids: Option<Vec<Uuid>>,
    host_naming_fallback: HostNamingFallback,
    scan_settings: ScanSettings,
    /// All credential mappings for integration dispatch.
    credential_mappings: Vec<
        crate::server::credentials::r#impl::mapping::CredentialMapping<
            crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
        >,
    >,
    /// Specific addresses to scan (a rescan). `None` sweeps the subnets.
    target_ips: Option<HashSet<std::net::IpAddr>>,
    /// Precomputed TCP port set: discovery ports, credential-required ports, and
    /// for a rescan the ports already known on the target.
    light_scan_ports: HashSet<u16>,
    /// Addresses declined for lack of evidence, by subnet.
    ///
    /// Aggregated rather than warned per address: a middlebox fronting a /24 declines 254 of them
    /// for one reason, and 254 copies of a sentence is not 254 findings. Emitted as one warning per
    /// subnet when the run finishes.
    declined: std::sync::Arc<std::sync::Mutex<HashMap<String, DeclinedAddresses>>>,
    /// Bounds reverse DNS and counts the lookups that went unanswered.
    reverse_dns: Arc<dns::ReverseDns>,
}

/// What one subnet's declined addresses had in common.
#[derive(Default)]
pub(super) struct DeclinedAddresses {
    /// How many addresses in the subnet answered a connect and nothing else.
    pub(super) count: u32,
    /// How often each port completed a handshake. A middlebox answers the same set at every
    /// address, so the frequency is what distinguishes its ports from a real host's.
    pub(super) ports: std::collections::BTreeMap<u16, u32>,
}

impl NetworkScan {
    pub fn new(
        subnet_ids: Option<Vec<Uuid>>,
        host_naming_fallback: HostNamingFallback,
        scan_settings: ScanSettings,
        credential_mappings: Vec<
            crate::server::credentials::r#impl::mapping::CredentialMapping<
                crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
            >,
        >,
        target_ips: Option<HashSet<std::net::IpAddr>>,
        extra_ports: Vec<u16>,
    ) -> Self {
        // Build light scan port set: discovery ports + credential-required ports
        let mut light_scan_ports: HashSet<u16> = Service::all_discovery_ports()
            .iter()
            .filter(|p| p.is_tcp())
            .map(|p| p.number())
            .collect();

        // Add ports from all credential types generically
        for mapping in &credential_mappings {
            if let Some(default) = &mapping.default_credential {
                light_scan_ports.extend(default.required_scan_ports());
            }
            for override_entry in &mapping.ip_overrides {
                light_scan_ports.extend(override_entry.credential.required_scan_ports());
            }
        }

        // A rescan verifies the ports already recorded on its target, so fold
        // them in the same way credentials widen the set. Cost and batching are
        // derived from this set's size, so they stay correct for free.
        light_scan_ports.extend(extra_ports);

        Self {
            subnet_ids,
            host_naming_fallback,
            scan_settings,
            credential_mappings,
            target_ips,
            light_scan_ports,
            declined: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            reverse_dns: Arc::new(dns::ReverseDns::default()),
        }
    }

    /// Record an address declined for want of evidence, for the end-of-run warning.
    pub(super) fn note_declined_address(&self, subnet: &IpCidr, open_ports: &[PortType]) {
        let Ok(mut declined) = self.declined.lock() else {
            return;
        };
        let entry = declined.entry(subnet.to_string()).or_default();
        entry.count = entry.count.saturating_add(1);
        for port in open_ports {
            *entry.ports.entry(port.number()).or_default() += 1;
        }
    }

    /// One warning per subnet that declined anything, most-declined first.
    pub(super) fn declined_warnings(&self) -> Vec<DiscoveryWarning> {
        match self.declined.lock() {
            Ok(declined) => declined_warnings(&declined),
            Err(_) => Vec::new(),
        }
    }

    /// Compute the total integration cost (centiseconds) for a specific IP.
    /// Thin delegator to [`integration_cost_for_ip`].
    fn compute_integration_cost_for_ip(&self, ip: IpAddr) -> usize {
        integration_cost_for_ip(&self.credential_mappings, ip)
    }
}

/// Total integration cost (centiseconds) the daemon attributes to `ip`, counting each
/// integration **once** regardless of how many credentials of that type cover the IP.
///
/// SNMP now runs a single collection per host (see `execute_integrations`' dedup), so
/// N SNMP credentials (v1/v2c/v3 + the injected "public" default) must cost the same as
/// one. This is used symmetrically for both the total-cost estimate and the completed-
/// cost accrual (`scan.rs`) so `completed_cost` converges to `total_cost` and the scan
/// ETA/progress stay accurate instead of over-counting per-credential.
pub(super) fn integration_cost_for_ip(
    credential_mappings: &[crate::server::credentials::r#impl::mapping::CredentialMapping<
        crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
    >],
    ip: IpAddr,
) -> usize {
    let mut seen: HashSet<CredentialQueryPayloadDiscriminants> = HashSet::new();
    credential_mappings
        .iter()
        .filter_map(|m| {
            let discriminant: CredentialQueryPayloadDiscriminants = m
                .default_credential
                .as_ref()
                .map(|c| c.into())
                .or_else(|| m.ip_overrides.first().map(|o| (&o.credential).into()))?;
            let has_cred =
                m.ip_overrides.iter().any(|o| o.ip == ip) || m.default_credential.is_some();
            // Count each integration discriminant at most once per IP.
            if !has_cred || !seen.insert(discriminant) {
                return None;
            }
            let integration = IntegrationRegistry::get(discriminant)?;
            Some(integration.estimated_seconds() as usize * 100)
        })
        .sum()
}

/// How an address earned its way into the deep scan.
///
/// Replaces the bare `Option<MacAddress>` the host channel used to carry. That type could express
/// "ARP answered, here is the MAC" and "nothing has answered yet", but not "alive, and we have no
/// MAC" — which is exactly what an ICMP echo reply establishes. Conflating the last two would
/// hand an ICMP-confirmed address to the TCP responsiveness check and let it be dropped for
/// answering no port, which is the whole failure ICMP was added to fix (GH #678).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LivenessEvidence {
    /// An ARP reply. The only signal that yields a MAC, which is why it takes precedence over
    /// every other when more than one answers for the same address.
    Arp(MacAddress),
    /// An ICMP echo reply. Proves the address is alive; says nothing else about it.
    Icmp,
    /// An mDNS/DNS-SD announcement. Like ICMP it proves the address is alive without yielding a
    /// MAC, and it reaches two populations ICMP does not:
    ///
    /// - A host that drops echo requests but still advertises its services — macOS with stealth
    ///   mode enabled is the common one, since that setting stops ping replies and leaves Bonjour
    ///   untouched.
    /// - Addresses past `arp_scan_cutoff` on a very large interfaced subnet. Both sweeps work from
    ///   a materialised target list truncated at that prefix; a multicast browse is one packet to
    ///   the group and reaches every responder on the link however large the subnet is.
    Mdns,
    /// No signal yet — the address is simply within a subnet being swept. Must still pass the TCP
    /// responsiveness check before it is treated as a host.
    Enumerated,
}

impl LivenessEvidence {
    /// The MAC, when the evidence was the kind that carries one.
    pub(super) fn mac(&self) -> Option<MacAddress> {
        match self {
            Self::Arp(mac) => Some(*mac),
            Self::Icmp | Self::Mdns | Self::Enumerated => None,
        }
    }

    /// Whether something answered at this address.
    ///
    /// Gates the responsiveness check, the early-host report, and the cost accounting — all three
    /// of which previously keyed off `mac.is_some()` and so silently meant "ARP answered".
    pub(super) fn is_confirmed_live(&self) -> bool {
        match self {
            Self::Arp(_) | Self::Icmp | Self::Mdns => true,
            Self::Enumerated => false,
        }
    }

    /// Whether the daemon shares a broadcast domain with the address.
    ///
    /// A different question from [`Self::is_confirmed_live`], and the middlebox lab showed what
    /// conflating them costs. ARP and mDNS are link-local, so an answer means the daemon is on the
    /// segment and a TCP connect reaches the device itself. **ICMP is routed.** An echo reply proves
    /// something is alive at that address and says nothing about what answers its TCP ports — on a
    /// subnet fronted by a session helper that is the appliance, for live and empty addresses alike.
    ///
    /// So "a host is here" and "this port is really open on it" need different evidence, and only
    /// the second one governs whether a connect-only definition may name a service.
    pub(super) fn reached_on_link(&self) -> bool {
        match self {
            Self::Arp(_) | Self::Mdns => true,
            Self::Icmp | Self::Enumerated => false,
        }
    }
}

/// Whether `addr` can be a host address within `cidr`.
///
/// Exists because ICMP treats a subnet's network and broadcast addresses very differently from the
/// way ARP does. Nothing owns the network address, so nothing ARP-replies for it and the ARP sweep
/// was free to enumerate it. An echo request to the same address is answered by *many* hosts at
/// once — a live /22 returns a burst of duplicate replies — which both invents a host at an address
/// nothing occupies and makes one packet provoke a reply from the whole segment. That is the
/// opposite of what the rest of this scanner is built for: `arp_rate_pps` defaults to 50 precisely
/// to stay friendly to switch ARP-policing.
///
/// A /31 or /32 has no network or broadcast address to exclude — RFC 3021 point-to-point links use
/// both addresses as hosts, and a single-host rescan targets a /32 — so those pass everything.
pub(super) fn is_host_address(cidr: &IpCidr, addr: IpAddr) -> bool {
    if cidr.network_length() >= 31 {
        return true;
    }
    addr != cidr.first_address() && addr != cidr.last_address()
}

/// Ports some [`AppProbe`](crate::daemon::utils::app_probe::AppProbe) can validate.
///
/// Computed once: the probe registry is the service-definition registry, which is static, and
/// walking ~270 definitions per scanned address would be paid for nothing.
static PROBE_COVERED_PORTS: std::sync::LazyLock<
    HashSet<crate::server::ports::r#impl::base::PortType>,
> = std::sync::LazyLock::new(|| {
    crate::daemon::utils::app_probe::all_app_probes()
        .iter()
        .map(|probe| probe.port())
        .collect()
});

/// One warning per subnet that declined an address, most-declined first.
///
/// Free function rather than a method so the aggregation is testable without standing up a whole
/// [`NetworkScan`], which is what the shape of this needs checking for: the grouping, the counts and
/// the port ordering.
pub(super) fn declined_warnings(
    declined: &HashMap<String, DeclinedAddresses>,
) -> Vec<DiscoveryWarning> {
    let mut warnings: Vec<(u32, DiscoveryWarning)> = declined
        .iter()
        .map(|(subnet, addresses)| {
            // Most-answered port first: a middlebox answers the same set at every address, so
            // frequency is what tells its ports apart from one real host's.
            let mut ports: Vec<(u16, u32)> =
                addresses.ports.iter().map(|(p, n)| (*p, *n)).collect();
            ports.sort_by_key(|(port, seen)| (std::cmp::Reverse(*seen), *port));
            (
                addresses.count,
                DiscoveryWarning::ConnectionsWithoutProtocolResponse {
                    cidr: subnet.clone(),
                    declined: addresses.count,
                    ports: ports.into_iter().map(|(port, _)| port).collect(),
                },
            )
        })
        .collect();
    // Most-declined subnet first, then by name so the order is stable across runs.
    warnings.sort_by(|(a_count, a), (b_count, b)| {
        b_count.cmp(a_count).then_with(|| match (a, b) {
            (
                DiscoveryWarning::ConnectionsWithoutProtocolResponse { cidr: a, .. },
                DiscoveryWarning::ConnectionsWithoutProtocolResponse { cidr: b, .. },
            ) => a.cmp(b),
            _ => std::cmp::Ordering::Equal,
        })
    });
    warnings.into_iter().map(|(_, warning)| warning).collect()
}

/// TCP ports claimed only by a definition that matches on the connect alone.
///
/// Derived from the definitions rather than listed, so it shrinks by itself as exceptions are
/// retired and cannot drift from the guard that declares them. Six ports at the time of writing:
/// jetdirect's 9100, veeam's 9392, denodo's four.
pub(super) static CONNECT_ONLY_PORTS: std::sync::LazyLock<
    HashSet<crate::server::ports::r#impl::base::PortType>,
> = std::sync::LazyLock::new(|| {
    crate::server::services::definitions::ServiceDefinitionRegistry::connect_only_definitions()
        .into_iter()
        .map(|(_, port)| port)
        .collect()
});

/// Whether an address nothing answered for has any evidence of a host beyond a bare TCP connect.
///
/// This is the check the FortiGate report turned on. A firewall session helper completes the
/// handshake for every address on a routed VLAN, and an address with no ARP reply, no ICMP echo and
/// no mDNS advert was promoted to a host on the strength of that alone — one phantom "SIP Server"
/// per VLAN, none of them a device.
///
/// **Anything validated counts**, and by default nothing else does. A probe that spoke the protocol
/// and got a protocol answer, an HTTP endpoint that replied, a credential that authenticated — a
/// middlebox completing a handshake produces none of these.
///
/// `trust_port_only_detections` governs the one case that is genuinely ambiguous: **an open port
/// nothing could have validated.** If no probe covers the port, its silence means only that there
/// was nothing to ask, so the bare connect is the best evidence available.
///
/// - **Off (the default).** That is not enough. An address on a routed subnet is recorded only when
///   something answered. This is fail-closed, and the cost is stated rather than hidden: a host
///   whose every open port is one Scanopy cannot interrogate — a bespoke TCP service, and nothing
///   else open — is not recorded on a routed subnet. That is a real class of host.
/// - **On.** The uncovered open port counts, which is the behaviour before this setting existed.
///
/// The narrow framing "connect-only definitions" is not the rule, deliberately. 1720 showed why: a
/// full scan sweeps every port, and a middlebox answering one that no definition claims manufactures
/// a host just as readily as one that a connect-only definition claims. The rule has to be about
/// what was *validated*, not about a list of ports.
///
/// Confirmed-live addresses never reach this: ARP, ICMP and mDNS are evidence in their own right,
/// and [`LivenessEvidence::is_confirmed_live`] gates the call.
pub(super) fn enumerated_host_has_evidence(
    open_ports: &[crate::server::ports::r#impl::base::PortType],
    validated_ports: &HashSet<crate::server::ports::r#impl::base::PortType>,
    trust_port_only_detections: bool,
) -> bool {
    if !validated_ports.is_empty() {
        return true;
    }
    trust_port_only_detections
        && open_ports
            .iter()
            .any(|port| !PROBE_COVERED_PORTS.contains(port))
}

/// TCP ports the liveness check probes before committing to a deep scan of an address that
/// produced no ARP reply.
///
/// This must be a superset of every port the deep scan would go on to look at, or the check
/// rejects hosts the very next step would have identified. It previously read
/// [`Service::all_discovery_ports`] directly, which by construction excludes any port that only
/// appears in a `Pattern::Endpoint` or `Pattern::Header` — see `Pattern::ports`, which returns
/// nothing for those so the port-scan phase doesn't connect to a port the endpoint probe is about
/// to open anyway. Correct for the port-scan phase, wrong here: it left out 443, 8080, 3000, 5000,
/// 8443, 9000 and ~48 others, so an HTTPS-only host — or a Home Assistant on 8123 — was declared
/// unresponsive and dropped (GH #678). A full 65k scan didn't help either, since it runs behind
/// this check.
///
/// So the set is assembled from what the scan itself would probe:
/// - `light_scan_ports`, which already carries the discovery ports, any credential-required ports,
///   and on a rescan the target host's own recorded ports (see [`NetworkScan::new`]) — the last of
///   which is why a rescan of a known host no longer disagrees with what that host is known to run.
/// - [`Service::endpoint_only_ports`], the ports the deep scan folds back in for endpoint probing.
pub(super) fn liveness_probe_ports(light_scan_ports: &HashSet<u16>) -> Vec<u16> {
    let mut ports: HashSet<u16> = light_scan_ports.clone();
    ports.extend(
        Service::endpoint_only_ports()
            .iter()
            .filter(|p| p.is_tcp())
            .map(|p| p.number()),
    );
    ports.into_iter().collect()
}

/// Which addresses have already been handed to the deep scanner.
///
/// The host channel has **no dedup of its own** — every message it carries spawns a deep scan, and
/// `early_reported_hosts` dedups only the early *stub*. Before ICMP there was exactly one producer
/// per address, so nothing needed one. With a ping sweep running alongside ARP, an address both
/// answered for would otherwise be scanned twice and produce two hosts.
///
/// Precedence falls out of *when* each source is consulted rather than from any ranking here. ARP
/// replies stream in and claim their addresses as they arrive, keeping the MAC only they carry;
/// the ping sweep's responders are released at the end of the discovery phase, by which point
/// every address ARP was going to find is already claimed. See
/// [`LivenessEvidence`] for why the distinction has to survive into the deep scan at all.
#[derive(Debug, Default)]
pub(super) struct DispatchedAddresses(HashSet<IpAddr>);

impl DispatchedAddresses {
    /// Claim `ip` for dispatch. `false` means something already claimed it and the caller must
    /// drop this one on the floor.
    pub(super) fn claim(&mut self, ip: IpAddr) -> bool {
        self.0.insert(ip)
    }
}

pub(super) struct DeepScanParams<'a> {
    ip: IpAddr,
    subnet: &'a Subnet,
    evidence: LivenessEvidence,
    cancel: tokio_util::sync::CancellationToken,
    scan_rate_pps: u32,
    port_scan_batch_size: usize,
    gateway_ips: &'a [IpAddr],
    completed_cost: Option<&'a Arc<AtomicUsize>>,
    total_cost: Option<&'a Arc<AtomicUsize>>,
    hosts_discovered: Option<&'a Arc<AtomicUsize>>,
    batches_per_host: usize,
    scan_cost_cs: usize,
    scan_controller: Arc<ScanConcurrencyController>,
    probe_raw_socket_ports: bool,
    early_host_id: Uuid,
    is_full_scan: bool,
    light_scan_ports: &'a HashSet<u16>,
    /// What the mDNS browse collected, keyed by address. Empty on any subnet the daemon has no
    /// interface on, because multicast does not cross a router.
    mdns_hosts: Arc<std::collections::HashMap<IpAddr, mdns::DnsSdHost>>,
    credential_mappings: &'a [crate::server::credentials::r#impl::mapping::CredentialMapping<
        crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
    >],
    known_subnets: Vec<Subnet>,
}

#[cfg(test)]
mod tests {
    use super::{
        DispatchedAddresses, IpCidr, LivenessEvidence, MacAddress, integration_cost_for_ip,
        is_host_address, liveness_probe_ports,
    };
    use crate::server::credentials::r#impl::mapping::{
        ContainerSocketQueryCredential, CredentialMapping, CredentialQueryPayload,
    };
    use crate::server::services::r#impl::base::Service;
    use std::collections::HashSet;
    use std::net::{IpAddr, Ipv4Addr};

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn mac() -> MacAddress {
        MacAddress::new([0, 1, 2, 3, 4, 5])
    }

    /// The live failure this came from: the ping sweep enumerated `192.168.4.0`, the network
    /// address of a `/22`, and the segment answered it as a broadcast — several hosts replying to
    /// one request, and a host record created at an address nothing occupies.
    ///
    /// The `/31` and `/32` cases are the reason this is a function rather than a blanket
    /// "skip first and last": a point-to-point link uses both of its addresses as hosts, and a
    /// single-host rescan targets a `/32`, so excluding those would make the sweep skip the only
    /// address it was asked about.
    #[test]
    fn only_addresses_a_host_can_occupy_are_swept() {
        let lan: IpCidr = "192.168.4.0/22".parse().unwrap();
        assert!(!is_host_address(&lan, ip("192.168.4.0")), "network address");
        assert!(
            !is_host_address(&lan, ip("192.168.7.255")),
            "broadcast address"
        );
        assert!(is_host_address(&lan, ip("192.168.4.29")));

        let point_to_point: IpCidr = "10.0.0.0/31".parse().unwrap();
        assert!(is_host_address(&point_to_point, ip("10.0.0.0")));
        assert!(is_host_address(&point_to_point, ip("10.0.0.1")));

        let single: IpCidr = "10.0.0.7/32".parse().unwrap();
        assert!(is_host_address(&single, ip("10.0.0.7")));
    }

    /// The failure this guards against: the host channel spawns a deep scan per message and has
    /// no dedup of its own, so an address the ping sweep and ARP both answered for would be
    /// scanned twice and land as two hosts.
    #[test]
    fn an_address_two_signals_answered_for_is_dispatched_once() {
        let mut dispatched = DispatchedAddresses::default();
        let addr = ip("10.0.5.7");

        assert!(dispatched.claim(addr), "the first signal dispatches");
        assert!(!dispatched.claim(addr), "the second must not");
    }

    /// The release path in miniature: the ping sweep offers every address it found, including
    /// ones ARP already reported. Only the addresses ARP never reached may go on to be scanned —
    /// re-dispatching the rest would scan them a second time, and without the MAC ARP carried.
    #[test]
    fn releasing_responders_skips_the_ones_arp_already_claimed() {
        let mut dispatched = DispatchedAddresses::default();
        let claimed_by_arp = ip("10.0.5.7");
        let icmp_only = ip("10.0.5.8");
        dispatched.claim(claimed_by_arp);

        let released: Vec<IpAddr> = [claimed_by_arp, icmp_only]
            .into_iter()
            .filter(|ip| dispatched.claim(*ip))
            .collect();

        assert_eq!(released, vec![icmp_only]);
    }

    /// Two properties that must hold across every variant, rather than a restatement of each
    /// arm: a signal that yields a MAC has necessarily proven the address alive, and the one
    /// state that still owes the responsiveness check is the one nothing has answered for.
    ///
    /// Getting this backwards is the bug ICMP was added to fix, in mirror image — an
    /// ICMP-confirmed address sent through the TCP check is dropped again for answering no port.
    #[test]
    fn only_unanswered_addresses_owe_the_responsiveness_check() {
        for evidence in [
            LivenessEvidence::Arp(mac()),
            LivenessEvidence::Icmp,
            LivenessEvidence::Mdns,
            LivenessEvidence::Enumerated,
        ] {
            if evidence.mac().is_some() {
                assert!(
                    evidence.is_confirmed_live(),
                    "{evidence:?} yields a MAC, so something answered at that address"
                );
            }
            assert_eq!(
                evidence.is_confirmed_live(),
                evidence != LivenessEvidence::Enumerated,
                "{evidence:?} must owe the responsiveness check only if nothing answered"
            );
        }
    }

    /// The invariant the liveness check violated (GH #678): it probed
    /// `Service::all_discovery_ports()`, which excludes every port reachable only through a
    /// `Pattern::Endpoint`/`Pattern::Header`. The deep scan folds those back in, so the check was
    /// rejecting addresses the next step would have identified — an HTTPS-only host, or a Home
    /// Assistant on 8123, never got scanned.
    ///
    /// Asserted as a set relationship rather than against named ports so it tracks the service
    /// definitions instead of restating them: adding, moving or retiring a definition can't break
    /// it, but reintroducing the omission can.
    #[test]
    fn the_liveness_check_probes_every_port_the_scan_would() {
        // Two addresses no service definition claims, standing in for the ports a credential or a
        // rescan target contributes — those reach the check only via `light_scan_ports`.
        let light: HashSet<u16> = HashSet::from([45001, 45002]);
        let probed: HashSet<u16> = liveness_probe_ports(&light).into_iter().collect();

        for port in &light {
            assert!(
                probed.contains(port),
                "port {port} is in the scan's port set but not the liveness check's"
            );
        }
        for port in Service::endpoint_only_ports().iter().filter(|p| p.is_tcp()) {
            assert!(
                probed.contains(&port.number()),
                "endpoint-only port {} is probed by the deep scan but not the liveness check",
                port.number()
            );
        }
    }

    fn snmp_mapping() -> CredentialMapping<CredentialQueryPayload> {
        CredentialMapping {
            default_credential: Some(CredentialQueryPayload::default()), // Snmp
            ..Default::default()
        }
    }

    fn docker_mapping() -> CredentialMapping<CredentialQueryPayload> {
        CredentialMapping {
            default_credential: Some(CredentialQueryPayload::DockerSocket(
                ContainerSocketQueryCredential { socket_path: None },
            )),
            ..Default::default()
        }
    }

    // The estimate must count each integration ONCE per host regardless of how many
    // credentials of that type are configured — SNMP now runs one collection per host,
    // so v1+v2c+v3 (+ the injected public default) must cost the same as a single SNMP
    // credential. This is the ETA-inflation regression guard.
    #[test]
    fn snmp_counted_once_regardless_of_credential_count() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let one = integration_cost_for_ip(&[snmp_mapping()], ip);
        let four = integration_cost_for_ip(
            &[
                snmp_mapping(),
                snmp_mapping(),
                snmp_mapping(),
                snmp_mapping(),
            ],
            ip,
        );
        assert!(
            one > 0,
            "SNMP integration should have a non-zero estimated cost"
        );
        assert_eq!(
            one, four,
            "N SNMP credentials must cost the same as one (deduped by integration)"
        );
    }

    #[test]
    fn distinct_integrations_each_count_once() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let snmp = integration_cost_for_ip(&[snmp_mapping()], ip);
        let docker = integration_cost_for_ip(&[docker_mapping()], ip);
        let both = integration_cost_for_ip(&[snmp_mapping(), docker_mapping()], ip);
        assert_eq!(
            both,
            snmp + docker,
            "distinct integrations should each be counted once and summed"
        );
    }
}

#[cfg(test)]
mod enumerated_evidence {
    use super::enumerated_host_has_evidence;
    use crate::server::ports::r#impl::base::PortType;
    use std::collections::HashSet;

    /// The default. Every assertion below that does not say otherwise is about this.
    const STRICT: bool = false;
    /// The opt-in, for operators who would rather have the host than the certainty.
    const TRUSTING: bool = true;

    fn ports(numbers: &[u16]) -> Vec<PortType> {
        numbers.iter().map(|n| PortType::new_tcp(*n)).collect()
    }

    fn validated(numbers: &[u16]) -> HashSet<PortType> {
        numbers.iter().map(|n| PortType::new_tcp(*n)).collect()
    }

    /// The reported bug. A FortiGate session helper completes the handshake on 5060 for an address
    /// with no device behind it; the SIP probe gets no `SIP/2.0` back, so nothing validated. Before
    /// this check that address became a host with a "SIP Server" on it, on every routed VLAN.
    #[test]
    fn a_probe_backed_port_that_did_not_answer_is_not_a_host() {
        assert!(!enumerated_host_has_evidence(
            &ports(&[5060]),
            &validated(&[]),
            STRICT
        ));
    }

    /// The same address with a real SIP server on it: the probe answered, so it is a host.
    #[test]
    fn a_probe_that_answered_is_a_host() {
        assert!(enumerated_host_has_evidence(
            &ports(&[5060]),
            &validated(&[5060]),
            STRICT
        ));
    }

    /// The setting governs exactly one case: a port nothing could have interrogated. Off, its
    /// silence is not evidence; on, the bare connect stands as the best available.
    #[test]
    fn the_setting_governs_only_the_unprobeable_port() {
        // 9100 is JetDirect, declared `ProbeUnsafe` — writing to it prints. And a port no
        // definition claims at all, which is what a full scan turns up.
        for open in [ports(&[9100]), ports(&[43219])] {
            assert!(
                !enumerated_host_has_evidence(&open, &validated(&[]), STRICT),
                "{open:?}"
            );
            assert!(
                enumerated_host_has_evidence(&open, &validated(&[]), TRUSTING),
                "{open:?}"
            );
        }
    }

    /// The opt-in does not resurrect the original bug: a probe-backed port that stayed silent is
    /// still not evidence, however trusting the setting.
    #[test]
    fn the_setting_does_not_make_a_silent_probe_into_evidence() {
        assert!(!enumerated_host_has_evidence(
            &ports(&[5060]),
            &validated(&[]),
            TRUSTING
        ));
    }

    /// A real host on a poisoned VLAN: the middlebox answers 5060 for it too, but its own SSH
    /// answered, so the host is kept — with the setting off, which is the case that matters.
    #[test]
    fn one_validated_port_carries_an_address_whose_other_ports_did_not_answer() {
        assert!(enumerated_host_has_evidence(
            &ports(&[22, 5060]),
            &validated(&[22]),
            STRICT
        ));
    }

    /// Several probe-backed ports, none of which answered. A middlebox intercepting more than one
    /// helper port produces exactly this.
    #[test]
    fn several_silent_probe_backed_ports_are_still_not_a_host() {
        assert!(!enumerated_host_has_evidence(
            &ports(&[21, 554, 5060]),
            &validated(&[]),
            STRICT
        ));
    }

    /// Nothing open at all cannot be a host either, under either setting, and the responsiveness
    /// check upstream would have dropped it already.
    #[test]
    fn no_open_ports_is_not_a_host() {
        assert!(!enumerated_host_has_evidence(&[], &validated(&[]), STRICT));
        assert!(!enumerated_host_has_evidence(
            &[],
            &validated(&[]),
            TRUSTING
        ));
    }
}

#[cfg(test)]
mod link_local_evidence {
    use super::LivenessEvidence;
    use mac_address::MacAddress;

    /// The case the middlebox lab produced. 10.77.0.50 is a real host behind an appliance that
    /// answers TCP for the whole range; it replies to ICMP, so it is unambiguously live — and the
    /// scan still came back with a Veeam server on it, because the appliance answered 9392 on its
    /// behalf. Being alive is not evidence that its own ports were reached.
    #[test]
    fn an_icmp_reply_does_not_mean_we_reached_the_host_itself() {
        assert!(LivenessEvidence::Icmp.is_confirmed_live());
        assert!(!LivenessEvidence::Icmp.reached_on_link());
    }

    /// ARP and mDNS do not cross a router, so an answer places the daemon on the segment and a TCP
    /// connect goes to the device rather than to whatever fronts it. A printer's JetDirect on an
    /// on-link subnet must keep matching.
    #[test]
    fn arp_and_mdns_place_us_on_the_segment() {
        let evidence = LivenessEvidence::Arp(MacAddress::new([0, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]));
        assert!(evidence.reached_on_link());
        assert!(LivenessEvidence::Mdns.reached_on_link());
    }

    /// An enumerated address is neither, which is where this began.
    #[test]
    fn an_enumerated_address_is_neither_live_nor_on_link() {
        assert!(!LivenessEvidence::Enumerated.is_confirmed_live());
        assert!(!LivenessEvidence::Enumerated.reached_on_link());
    }
}

#[cfg(test)]
mod declined_warning {
    use super::{DeclinedAddresses, declined_warnings};
    use crate::daemon::discovery::types::warnings::DiscoveryWarning;
    use std::collections::{BTreeMap, HashMap};

    fn declined(count: u32, ports: &[(u16, u32)]) -> DeclinedAddresses {
        DeclinedAddresses {
            count,
            ports: ports.iter().copied().collect::<BTreeMap<_, _>>(),
        }
    }

    /// The reported shape: a FortiGate fronting a /24 answers the same helper ports at every
    /// address. One warning for the subnet, not 254 for the addresses.
    #[test]
    fn a_whole_range_of_declined_addresses_is_one_warning() {
        let mut map = HashMap::new();
        map.insert(
            "10.77.0.0/24".to_owned(),
            declined(254, &[(5060, 254), (21, 254), (554, 254)]),
        );

        let warnings = declined_warnings(&map);
        assert_eq!(warnings.len(), 1);
        let DiscoveryWarning::ConnectionsWithoutProtocolResponse { cidr, declined, .. } =
            &warnings[0]
        else {
            panic!("the only warning this raises");
        };
        assert_eq!(cidr, "10.77.0.0/24");
        assert_eq!(*declined, 254);
    }

    /// Ports the middlebox answers everywhere come first. The ones a single real host happened to
    /// have open are the tail, and reading the sentence top-down should describe the middlebox.
    #[test]
    fn the_ports_the_middlebox_answers_lead() {
        let mut map = HashMap::new();
        map.insert(
            "10.77.0.0/24".to_owned(),
            declined(254, &[(8080, 1), (5060, 254), (9999, 2), (21, 254)]),
        );

        let DiscoveryWarning::ConnectionsWithoutProtocolResponse { ports, .. } =
            &declined_warnings(&map)[0]
        else {
            panic!("the only warning this raises");
        };
        // 21 and 5060 answered at every address; between equal counts, the lower port first.
        assert_eq!(ports, &[21, 5060, 9999, 8080]);
    }

    /// Several routed subnets behind one appliance. The worst-affected leads, and the order is
    /// stable so a rerun does not reshuffle the list.
    #[test]
    fn subnets_are_ordered_by_how_many_addresses_each_declined() {
        let mut map = HashMap::new();
        map.insert("10.77.1.0/24".to_owned(), declined(12, &[(5060, 12)]));
        map.insert("10.77.2.0/24".to_owned(), declined(254, &[(5060, 254)]));
        map.insert("10.77.3.0/24".to_owned(), declined(12, &[(5060, 12)]));

        let subnets: Vec<String> = declined_warnings(&map)
            .into_iter()
            .map(|w| match w {
                DiscoveryWarning::ConnectionsWithoutProtocolResponse { cidr, .. } => cidr,
                _ => unreachable!("only this warning is produced here"),
            })
            .collect();
        assert_eq!(subnets, ["10.77.2.0/24", "10.77.1.0/24", "10.77.3.0/24"]);
    }

    /// A clean run raises nothing, which is what keeps this out of the way of networks it has
    /// nothing to say about.
    #[test]
    fn a_run_that_declined_nothing_warns_about_nothing() {
        assert!(declined_warnings(&HashMap::new()).is_empty());
    }
}
