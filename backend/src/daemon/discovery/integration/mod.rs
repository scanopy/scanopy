//! Discovery integration trait system.
//!
//! All discovery integrations follow the same flow:
//! 1. `probe()` — check if the integration's service responds with the given credential
//! 2. Service matching — probe result feeds into `Pattern::ClientResponse` matching
//! 3. `execute()` — scan/query the service, enrich HostData or create entities
//!
//! The pipeline dispatches integrations generically based on credential mappings
//! and service matches — no integration-specific code in the orchestrator.

pub mod container;
pub mod controller;
pub mod dispatch;
pub mod docker;
pub mod failure;
pub mod flex;
pub mod gnmi;
pub mod hypervisor;
pub mod instant_on;
pub mod libvirt;
pub mod podman;
pub mod proxmox;
pub mod snmp;
pub mod ssh;
pub mod unifi;
pub mod wake_on_lan;

use std::any::Any;
use std::net::IpAddr;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    daemon::discovery::service::warnings::AttemptOutcome,
    daemon::shared::config::ConfigStore,
    daemon::utils::base::PlatformDaemonUtils,
    server::{
        credentials::r#impl::mapping::{
            CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
        },
        credentials::r#impl::types::CredentialIntegration,
        discovery::r#impl::{
            collection::{Producer, report_undeclared_collection, undeclared_collection},
            types::HostNamingFallback,
        },
        ports::r#impl::base::PortType,
        services::r#impl::{base::Service, endpoints::EndpointResponse, patterns::ClientProbe},
        shared::attribution::AttributeMethod,
        shared::trusted_ca::TrustedCaBundle,
        subnets::r#impl::base::Subnet,
        subnets::r#impl::inference::{overlaps, placeable_subnet},
    },
};

use super::service::ops::{DiscoveryOps, HostData};
pub use crate::server::credentials::r#impl::types::InterfaceViewScope;
pub use failure::{IntegrationFailure, ProbeFailure};

// ============================================================================
// Collection completeness
// ============================================================================

/// What an integration is saying about the collection it just finished.
///
/// This is the *return* of `execute`, not a field on `HostData`, and that placement is the whole
/// mechanism. Completeness is only knowable when the collection ends, so a field — however
/// carefully constructed — has to start at some value and be corrected later, and the value it
/// starts at is a default nobody states. `HostData::new` defaults to "complete" at thirteen
/// construction sites, and server-side that default is *destructive*: a group marked complete is
/// authoritative in both directions, so `preserve_uncollected_data` clears the LLDP/CDP/FDB/VLAN
/// columns it was meant to protect.
///
/// As a return type there is no `()` to fall back on. An integration cannot reach the end of
/// `execute` without naming one of these, at the point where it actually knows.
pub enum Completeness {
    /// Everything this integration set out to collect, it collected.
    Complete,
    /// It did not. Becomes the operator's warning; the collection that did land is still merged,
    /// because a coherent subset is worth more than nothing as long as it is labelled.
    Partial(CollectionShortfall),
}

/// Who contributed an interface set, and how much of the device they can see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterfaceSource {
    pub credential: CredentialQueryPayloadDiscriminants,
    pub scope: InterfaceViewScope,
}

/// What an integration could not finish, phrased so the renderer can say it out loud.
pub struct CollectionShortfall {
    /// Plural noun for what was being collected — "containers", "devices". Reads as
    /// "read 250 of 300 containers".
    pub what: &'static str,
    pub collected: usize,
    pub expected: usize,
}

/// A commit point for an integration that deliberately persists mid-flight.
///
/// Most integrations never touch this. Their `execute` writes into a scratch buffer the caller
/// owns, so a timeout that drops the future drops everything they wrote with it — atomic by
/// construction, with nothing to declare and nothing to get wrong.
///
/// SNMP is the exception and needs to be: it persists a bare interface set as soon as the ifTable
/// walk finishes, so that a hang in the slower neighbour queries afterwards cannot leave a host
/// with zero interfaces. Committing is an explicit, named, greppable act rather than a property of
/// where in the function a mutation happened to land.
pub struct Checkpoint<'a> {
    committed: &'a std::sync::Mutex<Option<HostData>>,
}

impl<'a> Checkpoint<'a> {
    fn new(committed: &'a std::sync::Mutex<Option<HostData>>) -> Self {
        Self { committed }
    }

    /// Commit the collection so far. This is the only enrichment that survives if `execute` is
    /// later aborted by its timeout. Replaces any previous commit.
    pub fn commit(&self, host_data: &HostData) {
        if let Ok(mut slot) = self.committed.lock() {
            *slot = Some(host_data.clone());
        }
    }
}

/// Union the three subnet sources by id, preserving order: site-wide first, then the subnet
/// being swept, then anything this host's own collection turned up.
///
/// Shared by every integration where one credential reports on devices it did not scan. The
/// site's whole address space matters rather than the scan's scope: a controller reports every
/// device it manages, and on a segmented network almost none of them sit in the subnet a rescan is
/// sweeping — scoping to the sweep dropped all of them.
pub fn merge_subnets(
    known: &[Subnet],
    scanning: Option<&Subnet>,
    from_host: &[Subnet],
) -> Vec<Subnet> {
    let mut subnets = known.to_vec();
    for subnet in scanning.into_iter().chain(from_host) {
        if !subnets.iter().any(|s| s.id == subnet.id) {
            subnets.push(subnet.clone());
        }
    }
    subnets
}

/// The subnets a reported range is checked against in [`place_reported_range`]: the site's live
/// subnets and the one being swept, minus the organizational `0.0.0.0/0` rows and container
/// bridges, which overlap real ranges without describing the same segment, and minus inferred
/// ranges, which a reading corrects server-side.
pub fn held_subnets(known: &[Subnet], scanning: Option<&Subnet>) -> Vec<Subnet> {
    known
        .iter()
        .chain(scanning)
        .filter(|s| {
            !s.is_organizational_subnet()
                && !s.is_container_bridge_subnet()
                && s.base.cidr.source().method() != AttributeMethod::Inferred
        })
        .cloned()
        .collect()
}

/// Where an address that came with a reported range belongs.
#[derive(Debug)]
pub enum ReportedRange<'a> {
    /// No held subnet overlaps the reported range, so it is a segment this site does not hold
    /// yet: a router interface on another VLAN, a network a controller defines.
    New,
    /// The address goes on this held subnet. Either the reported range is that subnet, or the
    /// range overlaps held ones and this is the most specific held subnet containing the address.
    Held(&'a Subnet),
    /// The reported range overlaps a held subnet, and no held subnet contains the address.
    Unplaced,
}

/// Decide whether a range a device or controller reports becomes a subnet: an SNMP device's own
/// address and netmask, or a UniFi network's gateway address and prefix.
///
/// A device's netmask describes how that device is configured, not how the segment is built. A
/// host on a `/22` with a `/24` mask still answers ARP across the whole `/22`, and creating its
/// `/24` splits the segment: every address in that `/24` then lands on a subnet nothing else
/// agrees exists. So a reported range becomes a subnet only when it overlaps nothing the site
/// already holds. Otherwise the address goes on the held subnet that contains it.
///
/// `held` comes from [`held_subnets`].
pub fn place_reported_range<'a>(
    reported: &cidr::IpCidr,
    address: IpAddr,
    held: &'a [Subnet],
) -> ReportedRange<'a> {
    if let Some(same) = held.iter().find(|s| *s.base.cidr == *reported) {
        return ReportedRange::Held(same);
    }
    if !held.iter().any(|s| overlaps(&s.base.cidr, reported)) {
        return ReportedRange::New;
    }
    match placeable_subnet(held, address) {
        Some(subnet) => ReportedRange::Held(subnet),
        None => ReportedRange::Unplaced,
    }
}

// ============================================================================
// Trait
// ============================================================================

#[async_trait]
pub trait DiscoveryIntegration: Send + Sync {
    /// Which credential type this integration handles.
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants;

    /// Estimated execution time per invocation, in seconds.
    /// Used for cost-based progress estimation.
    fn estimated_seconds(&self) -> u32;

    /// Maximum execution time before the caller cancels.
    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// Maximum time `probe()` may take before the attempt is treated as timed out.
    ///
    /// Separate from [`Self::timeout`], which sizes a full collection: SNMP's is fifteen minutes,
    /// far too long to wait on a probe that has stopped answering. The default covers the slowest
    /// probe that does real work (gNMI's connect plus a capabilities RPC); integrations whose probe
    /// retries internally for longer override it.
    fn probe_timeout(&self) -> Duration {
        Duration::from_secs(120)
    }

    /// TCP ports that must be detected open before `probe()` is attempted: the ports the
    /// credential reaches the host through ([`CredentialReach`](crate::server::credentials::r#impl::mapping::CredentialReach)).
    /// Empty when it reaches the host without a port, or only over UDP, which SNMP probes itself.
    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        credential.reach().gate_ports()
    }

    /// Probe the target host: check if this integration's service responds
    /// with the given credential.
    ///
    /// Success: `ClientProbe` feeds into service matching, `handle` is passed to `execute()`.
    /// Failure: credential rejected or service not responding, with diagnostic message.
    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure>;

    /// Execute the integration's scanning/discovery logic.
    ///
    /// `host_data` is a **scratch buffer owned by the caller**, seeded from the real host. Enrich
    /// it via the builder methods, or create separate entities via `ctx.ops` (e.g. UniFi devices).
    /// It is merged into the real host only when this returns `Ok`, so an integration that is
    /// dropped mid-flight by the timeout cannot leave a half-written host behind — there is no
    /// path from here to the caller's `HostData` at all.
    ///
    /// That is the fix for GH #650: the container integration wrote its bridge subnets early and
    /// its container services last, so a 300s timeout persisted every subnet with none of the
    /// containers that gave them meaning, and the run still read as a success.
    ///
    /// To deliberately persist before finishing, call [`Checkpoint::commit`].
    ///
    /// Only called when `probe()` succeeded AND the associated service was matched.
    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure>;
}

// ============================================================================
// Client-library error classification
// ============================================================================

/// One impl per client library, so each integration classifies its own errors once rather than
/// every call site guessing. The foreign→domain direction keeps the vocabulary ours: a library
/// that adds an error variant fails to compile here rather than silently landing in a catch-all.
impl From<&bollard::errors::Error> for AttemptOutcome {
    fn from(error: &bollard::errors::Error) -> Self {
        use bollard::errors::Error as E;
        match error {
            // The daemon answered and refused us. On a TLS-protected socket that is the
            // certificate; on a plain one it is the socket's permissions. Either way the
            // operator's fix is the credential, not the network.
            E::DockerResponseServerError { status_code, .. }
                if *status_code == 401 || *status_code == 403 =>
            {
                Self::Rejected
            }
            // Anything else it answered means the service exists and is talking to us.
            E::DockerResponseServerError { .. }
            | E::JsonDataError { .. }
            | E::JsonSerdeError { .. }
            | E::APIVersionParseError { .. }
            | E::DockerStreamError { .. } => Self::NotThisService,
            E::CertPathError { .. }
            | E::CertMultipleKeys { .. }
            | E::CertParseError { .. }
            | E::NoNativeCertsError { .. }
            | E::LoadNativeCertsErrors { .. } => Self::TlsFailed,
            E::RequestTimeoutError => Self::TimedOut,
            // A malformed URL or missing socket path is our configuration, not their host.
            E::URLParseError { .. }
            | E::InvalidURIError { .. }
            | E::InvalidURIPartsError { .. }
            | E::UnsupportedURISchemeError { .. }
            | E::SocketNotFoundError(_)
            | E::NoHomePathError => Self::Malformed,
            _ => Self::Unreachable,
        }
    }
}

impl From<&reqwest::Error> for AttemptOutcome {
    fn from(error: &reqwest::Error) -> Self {
        if error.is_timeout() {
            return Self::TimedOut;
        }
        // reqwest folds TLS failures into `is_connect`, so the message is the only way to tell a
        // refused connection from a certificate the client would not accept — and they send an
        // operator to completely different places.
        if error.is_connect() {
            let text = error.to_string().to_lowercase();
            if text.contains("certificate") || text.contains("tls") || text.contains("ssl") {
                return Self::TlsFailed;
            }
            return Self::Unreachable;
        }
        if error.is_status() {
            return match error.status().map(|s| s.as_u16()) {
                Some(401) | Some(403) => Self::Rejected,
                _ => Self::NotThisService,
            };
        }
        if error.is_decode() {
            return Self::NotThisService;
        }
        Self::Unreachable
    }
}

impl From<&snmp2::Error> for AttemptOutcome {
    fn from(error: &snmp2::Error) -> Self {
        use snmp2::Error as E;
        match error {
            // v3 said no: the USM user, auth password or privacy password is wrong.
            E::AuthFailure(_) | E::Crypto(_) => Self::Rejected,
            // A v2c agent that does not know the community simply does not answer, so a community
            // mismatch here means we read a datagram meant for a different session — not that the
            // community is wrong.
            E::CommunityMismatch | E::RequestIdMismatch | E::AuthUpdated => Self::TimedOut,
            // We are talking to something, and it is not answering the way SNMP does.
            E::AsnParse
            | E::AsnInvalidLen
            | E::AsnWrongType
            | E::AsnUnsupportedType
            | E::AsnEof
            | E::AsnIntOverflow
            | E::UnsupportedVersion
            | E::ValueOutOfRange
            | E::BufferOverflow
            | E::Mib(_) => Self::NotThisService,
            E::Send | E::Receive => Self::Unreachable,
        }
    }
}

// ============================================================================
// Probe types
// ============================================================================

pub struct ProbeContext<'a> {
    pub ip: IpAddr,
    pub credential: &'a CredentialQueryPayload,
    pub credential_id: Option<Uuid>,
    pub cancel: &'a CancellationToken,
    pub utils: &'a PlatformDaemonUtils,
    /// Whether to accept self-signed / otherwise invalid TLS certificates, from the daemon's
    /// `accept_invalid_scan_certs` config. Mirrors [`IntegrationContext::accept_invalid_certs`]:
    /// integrations that authenticate over HTTPS make their *first* call here, so the probe needs
    /// the same policy the execute phase gets. Appliance controllers (UniFi and friends) ship
    /// self-signed certs by default, so without this the probe fails before execute is reached.
    pub accept_invalid_certs: bool,
    /// Extra CA roots from the daemon's `trusted_ca_bundle`, for controllers signed by a
    /// private CA. Mirrors [`IntegrationContext::trusted_ca`].
    pub trusted_ca: Option<&'a TrustedCaBundle>,
    /// The daemon's config, for state an integration keeps between scans: SSH host-key pins.
    pub config_store: &'a ConfigStore,
}

/// Successful probe — service responds with this credential.
pub struct ProbeSuccess {
    /// What was detected. Feeds into `client_responses` for `Pattern::ClientResponse` matching.
    /// `None` when the probe proves something about the host but there is no service to identify:
    /// Wake-on-LAN, whose probe succeeding means the host answered the scan.
    pub client_probe: Option<ClientProbe>,
    /// Ports the probe was detected on.
    pub ports: Vec<PortType>,
    /// Opaque keep-alive state passed to `execute()`.
    /// E.g., connected Docker client, working SNMP credential + port.
    pub handle: Option<Box<dyn Any + Send + Sync>>,
}

// ============================================================================
// Execution context
// ============================================================================

pub struct IntegrationContext<'a> {
    pub ip: IpAddr,
    pub credential: &'a CredentialQueryPayload,
    pub credential_id: Option<Uuid>,
    /// Who this integration is, for the purposes of contributing interfaces. Built by dispatch
    /// from `CredentialIntegration::interface_view_scope`, so a call site cannot disagree with the
    /// declaration.
    pub interface_source: InterfaceSource,
    /// The integration running, for checking what it submits against what it declares it
    /// reports. Built by dispatch from the integration's own `credential_type`.
    pub integration: CredentialIntegration,
    pub cancel: &'a CancellationToken,
    pub ops: &'a DiscoveryOps,
    pub utils: &'a PlatformDaemonUtils,
    /// Opaque state from `probe()`. Integration downcasts to its expected type.
    pub probe_handle: Option<&'a (dyn Any + Send + Sync)>,
    pub matched_services: &'a [Service],
    pub open_ports: &'a [PortType],
    pub endpoint_responses: &'a [EndpointResponse],
    pub host_id: Uuid,
    pub host_naming_fallback: HostNamingFallback,
    /// Subnets an integration may place a discovered address in — the site's whole address
    /// space during the network phase, and the just-created ones during the daemon-host phase.
    ///
    /// Deliberately *not* the scan's subnet list. An integration learns about addresses the
    /// sweep never visits: a UniFi controller reports every switch it manages, most of them on
    /// subnets a rescan of the controller does not touch. Host identity is IP-based, so a device
    /// that cannot be placed in a subnet is dropped rather than deduplicated — which made a
    /// controller rescan silently enrich nothing.
    pub known_subnets: &'a [Subnet],
    /// The subnets this discovery sweeps, which is what its network scan reaches: an address
    /// outside every one of them gets no ports or services from the scan, whatever an
    /// integration records for it. The daemon-host phase runs before the sweep and passes the
    /// subnets it created from the daemon's own interfaces.
    pub scanned_subnets: &'a [Subnet],
    pub accept_invalid_certs: bool,
    pub trusted_ca: Option<&'a TrustedCaBundle>,
    /// The subnet currently being scanned (needed by SNMP for remote subnet discovery).
    pub scanning_subnet: Option<&'a Subnet>,
    /// Whether this integration's progress is the session's progress: true in the daemon-host
    /// phase, which runs the integrations as its whole work. In a deep scan the scan's own cost
    /// accounting moves the bar, and an integration's percentage of its own work says nothing
    /// about the session's. See [`IntegrationContext::report_progress`].
    pub owns_progress: bool,
}

impl IntegrationContext<'_> {
    /// Report progress within this integration's work, 0-100. Forwarded to the session only
    /// where the integration owns the phase's progress ([`Self::owns_progress`]). Inside a deep
    /// scan it is dropped: the session reports overall progress that never falls, so an
    /// integration's 90 there became 90% for the whole scan and held the bar until it ended.
    pub async fn report_progress(&self, percent: u8) {
        if self.owns_progress {
            let _ = self.ops.report_progress(percent).await;
        }
    }
}

// ============================================================================
// Registry
// ============================================================================

/// Maps credential types to their discovery integration.
/// Exhaustive match — every credential type has an integration.
pub struct IntegrationRegistry;

impl IntegrationRegistry {
    /// Resolve a credential type to its integration. Returns `None` for the
    /// forward-compat `Unknown` variant — a newer server may send a credential type
    /// this daemon doesn't recognize (deserialized via `#[serde(other)]`); callers
    /// skip it rather than failing the whole discovery request.
    pub fn get(d: CredentialQueryPayloadDiscriminants) -> Option<Box<dyn DiscoveryIntegration>> {
        Some(match d {
            CredentialQueryPayloadDiscriminants::Snmp => Box::new(snmp::SnmpIntegration),
            CredentialQueryPayloadDiscriminants::Gnmi => Box::new(gnmi::GnmiIntegration),
            CredentialQueryPayloadDiscriminants::DockerProxy => Box::new(docker::DockerIntegration),
            CredentialQueryPayloadDiscriminants::DockerSocket => {
                Box::new(docker::DockerSocketIntegration)
            }
            CredentialQueryPayloadDiscriminants::PodmanProxy => Box::new(podman::PodmanIntegration),
            CredentialQueryPayloadDiscriminants::PodmanSocket => {
                Box::new(podman::PodmanSocketIntegration)
            }
            CredentialQueryPayloadDiscriminants::UnifiController => {
                Box::new(unifi::UnifiIntegration)
            }
            CredentialQueryPayloadDiscriminants::InstantOn => {
                Box::new(instant_on::InstantOnIntegration)
            }
            CredentialQueryPayloadDiscriminants::Ssh => Box::new(ssh::SshIntegration),
            CredentialQueryPayloadDiscriminants::WakeOnLan => {
                Box::new(wake_on_lan::WakeOnLanIntegration)
            }
            CredentialQueryPayloadDiscriminants::Proxmox => Box::new(proxmox::ProxmoxIntegration),
            CredentialQueryPayloadDiscriminants::Libvirt => Box::new(libvirt::LibvirtIntegration),
            CredentialQueryPayloadDiscriminants::Unknown => return None,
        })
    }
}

// ============================================================================
// Progress reporting wrapper
// ============================================================================

/// Wraps `execute()` with periodic progress re-reporting to prevent the server's
/// 5-minute stall detector from killing the session.
///
/// The integration calls [`IntegrationContext::report_progress`] (0-100 within its own work).
/// In the daemon-host phase that maps into the phase's range; inside a deep scan it is dropped,
/// because no range is set per integration there.
///
/// The `progress_fn` re-reports the current progress as a heartbeat every 30 seconds
/// if the integration hasn't reported recently.
/// Merges into `host_data` only what the integration actually finished, or committed on purpose.
///
/// The integration never sees `host_data` itself. It works against a scratch clone this function
/// owns; on success the scratch replaces the original, and on a timeout or failure the scratch is
/// dropped and the caller's host is exactly as it was. Before GH #650 the integration held
/// `&mut` on the real thing, so `tokio::time::timeout` dropping the future mid-execute left
/// whatever had already landed — for containers, every bridge subnet and not one container.
///
/// The categories of data the surviving collection added are checked against what the integration
/// declares collecting ([`CredentialIntegration::collects`]).
pub async fn execute_with_progress_reporting<F, Fut>(
    integration: &dyn DiscoveryIntegration,
    ctx: &IntegrationContext<'_>,
    host_data: &mut HostData,
    progress_fn: F,
) -> Result<(), IntegrationFailure>
where
    F: Fn() -> Fut + Send,
    Fut: std::future::Future<Output = ()> + Send,
{
    let timeout_duration = integration.timeout();
    let committed = std::sync::Mutex::new(None);
    let mut scratch = host_data.clone();
    let collected_before = host_data.collected_data();

    let result = {
        let checkpoint = Checkpoint::new(&committed);
        tokio::time::timeout(timeout_duration, async {
            let execute_fut = integration.execute(ctx, &mut scratch, &checkpoint);
            tokio::pin!(execute_fut);
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            interval.tick().await; // consume immediate first tick
            loop {
                tokio::select! {
                    result = &mut execute_fut => return result,
                    _ = interval.tick() => {
                        progress_fn().await;
                    }
                }
            }
        })
        .await
    };

    let outcome = match result {
        Ok(Ok(completeness)) => ExecuteOutcome::Finished(completeness),
        Ok(Err(failure)) => ExecuteOutcome::Failed(failure),
        Err(_) => ExecuteOutcome::TimedOut(timeout_duration),
    };
    let committed = committed.lock().ok().and_then(|mut slot| slot.take());

    let surviving = surviving_collection(outcome, scratch, committed);

    if let Some(collection) = surviving.host_data {
        let producer = Producer::Integration(ctx.integration);
        report_undeclared_collection(
            producer,
            undeclared_collection(producer, &collected_before, &collection.collected_data()),
        );
        *host_data = collection;
    }
    if let Some(shortfall) = &surviving.shortfall {
        report_shortfall(ctx, shortfall).await;
    }
    surviving.result
}

/// How `execute` ended, before any decision about what to keep.
enum ExecuteOutcome {
    Finished(Completeness),
    Failed(IntegrationFailure),
    TimedOut(Duration),
}

/// What survives an integration's execute, and what to tell the operator.
struct SurvivingCollection {
    /// Replaces the caller's host, or `None` to leave it exactly as it was.
    host_data: Option<HostData>,
    shortfall: Option<CollectionShortfall>,
    result: Result<(), IntegrationFailure>,
}

/// Decide what an integration's execute leaves behind.
///
/// Pure, and separated from the wrapper that calls it for the same reason [`issue_for_attempt`]
/// is separated from dispatch: this is the policy worth testing, and testing it in place would
/// mean standing up a whole `DiscoveryOps` and a bollard client.
///
/// [`issue_for_attempt`]: crate::daemon::discovery::service::warnings::issue_for_attempt
fn surviving_collection(
    outcome: ExecuteOutcome,
    scratch: HostData,
    committed: Option<HostData>,
) -> SurvivingCollection {
    match outcome {
        // Ran to the end: everything it collected is coherent, so all of it lands. A declared
        // shortfall still lands — a subset of whole containers beats nothing — but it is labelled.
        ExecuteOutcome::Finished(completeness) => SurvivingCollection {
            host_data: Some(scratch),
            shortfall: match completeness {
                Completeness::Complete => None,
                Completeness::Partial(shortfall) => Some(shortfall),
            },
            result: Ok(()),
        },
        // Did not. The scratch goes in the bin, and only an explicit checkpoint survives — which
        // is how SNMP keeps the interface set it walked before a later query hung. No checkpoint
        // means the caller's host is untouched, which is the whole point of GH #650: a container
        // scan that ran out of time must not leave bridge subnets it found no containers in.
        ExecuteOutcome::Failed(failure) => SurvivingCollection {
            host_data: committed,
            shortfall: None,
            result: Err(failure),
        },
        ExecuteOutcome::TimedOut(after) => SurvivingCollection {
            host_data: committed,
            shortfall: None,
            result: Err(IntegrationFailure::collection_timed_out(format!(
                "timed out after {after:?} with nothing recorded"
            ))),
        },
    }
}

/// Turn a declared shortfall into the operator's warning.
///
/// Derived from the same value the integration returned, so the machine-readable completeness and
/// the human-readable warning cannot disagree — they used to be populated independently by hand,
/// and an integration could mark data incomplete while telling the operator nothing, or the
/// reverse.
///
/// Says **recorded**, not "read": a shortfall means the collection stopped at a coherent boundary
/// and what it got was kept. The hard-cap message is the one that reports nothing landing. Both
/// share [`AttemptOutcome::CollectionTimedOut`]'s advice, so the advice stays silent on what was
/// persisted and each message states its own outcome — otherwise a partial scan reads
/// "this host's data was not recorded (read 27 of 32 containers)", which contradicts itself.
async fn report_shortfall(ctx: &IntegrationContext<'_>, shortfall: &CollectionShortfall) {
    ctx.ops
        .record_attempt_failure(
            ctx.credential.into(),
            ctx.ip,
            AttemptOutcome::CollectionTimedOut,
            format!(
                "recorded {} of {} {} before the time limit; the rest were not read",
                shortfall.collected, shortfall.expected, shortfall.what
            ),
            true,
            ctx.credential_id,
        )
        .await;
}

/// GH #650. What an integration leaves behind when it does not finish.
///
/// These drive [`surviving_collection`] directly rather than a live integration: the policy is
/// the part worth locking, and reaching it through `execute_with_progress_reporting` would mean
/// standing up a `DiscoveryOps`, a `PlatformDaemonUtils` and a bollard client to assert something
/// none of them participate in.
#[cfg(test)]
mod partial_result_tests {
    use super::*;
    use crate::server::hosts::r#impl::base::{Host, HostBase};
    use crate::server::shared::storage::traits::Storable;
    use crate::server::subnets::r#impl::base::SubnetTypeAttributed;
    use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
    use crate::server::subnets::r#impl::types::SubnetType;

    fn bare_host() -> HostData {
        HostData::new(
            Host::new(HostBase::default()),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        )
    }

    fn bridge_subnet() -> Subnet {
        Subnet::new(SubnetBase {
            subnet_type: SubnetTypeAttributed::unattributed(SubnetType::DockerBridge),
            ..Default::default()
        })
    }

    /// The reported bug, as a test. `container::execute` collected bridge subnets long before the
    /// containers inside them, so a scan that ran past its cap persisted 353 subnets and zero
    /// container services — and the run reported success.
    #[test]
    fn a_container_scan_that_runs_out_of_time_leaves_no_bridge_subnets_behind() {
        let mut scratch = bare_host();
        scratch.add_subnet(bridge_subnet());

        let surviving = surviving_collection(
            ExecuteOutcome::TimedOut(Duration::from_secs(300)),
            scratch,
            None,
        );

        assert!(
            surviving.host_data.is_none(),
            "a host must not end up holding bridge subnets it found no containers in"
        );
        assert!(surviving.result.is_err());
    }

    /// SNMP's guarantee, pinned. It commits its interface set as soon as the ifTable walk lands so
    /// a later hang cannot strand the host with zero interfaces — a blanket discard would
    /// reintroduce exactly the failure that mechanism exists to prevent.
    #[test]
    fn an_integration_that_checkpoints_deliberately_keeps_what_it_committed() {
        let mut committed = bare_host();
        committed.add_subnet(bridge_subnet());

        // The scratch went further than the checkpoint before it hung.
        let mut scratch = committed.clone();
        scratch.add_subnet(Subnet::new(SubnetBase {
            subnet_type: SubnetTypeAttributed::unattributed(SubnetType::PodmanBridge),
            ..Default::default()
        }));

        let surviving = surviving_collection(
            ExecuteOutcome::TimedOut(Duration::from_secs(300)),
            scratch,
            Some(committed),
        );

        let kept = surviving
            .host_data
            .expect("an explicit checkpoint must survive a timeout");
        assert_eq!(
            kept.subnets.len(),
            1,
            "only what was committed survives, not what the scratch had reached"
        );
    }

    #[test]
    fn a_scan_that_finishes_keeps_everything_it_found() {
        let mut scratch = bare_host();
        scratch.add_subnet(bridge_subnet());

        let surviving = surviving_collection(
            ExecuteOutcome::Finished(Completeness::Complete),
            scratch,
            None,
        );

        assert_eq!(
            surviving
                .host_data
                .expect("a completed scan must be merged")
                .subnets
                .len(),
            1,
            "the discard has to be conditional, or a working scan collects nothing"
        );
        assert!(surviving.shortfall.is_none());
        assert!(surviving.result.is_ok());
    }

    /// The `Ok(Err(_))` arm, which the timeout test never reaches: an integration that returns a
    /// failure has written just as much scratch as one that hung, and it goes the same way.
    #[test]
    fn an_integration_that_errors_is_discarded_like_one_that_hangs() {
        let mut scratch = bare_host();
        scratch.add_subnet(bridge_subnet());

        let surviving = surviving_collection(
            ExecuteOutcome::Failed(IntegrationFailure::collection_failed("boom")),
            scratch,
            None,
        );

        assert!(surviving.host_data.is_none());
    }

    /// A scan stopped by its soft deadline is a coherent subset — every container it reached is
    /// whole — so it is kept. What must not happen is keeping it silently.
    #[test]
    fn a_collection_that_stopped_early_is_kept_and_reported() {
        let mut scratch = bare_host();
        scratch.add_subnet(bridge_subnet());

        let surviving = surviving_collection(
            ExecuteOutcome::Finished(Completeness::Partial(CollectionShortfall {
                what: "containers",
                collected: 250,
                expected: 300,
            })),
            scratch,
            None,
        );

        assert!(
            surviving.host_data.is_some(),
            "a coherent subset is worth more than nothing"
        );
        let shortfall = surviving
            .shortfall
            .expect("a partial collection the operator is never told about is the original bug");
        assert_eq!((shortfall.collected, shortfall.expected), (250, 300));
    }

    /// The seam into the warning path: running out of time must not classify as the address
    /// never answering, which is what sent operators to check a service that had just answered.
    #[test]
    fn running_out_of_time_reads_as_a_collection_timeout_not_an_unreachable_host() {
        let surviving = surviving_collection(
            ExecuteOutcome::TimedOut(Duration::from_secs(300)),
            bare_host(),
            None,
        );

        let failure = surviving.result.expect_err("a timeout is a failure");
        assert_eq!(failure.outcome(), AttemptOutcome::CollectionTimedOut);
    }
}

#[cfg(test)]
mod classification_tests {
    use super::*;

    /// The customer-facing point of the whole mechanism: a wrong password and an unreachable
    /// host must not read the same. Docker's daemon answers 401/403 when it refuses us, and
    /// that used to arrive as "probe failed after 3 attempts" — indistinguishable from nothing
    /// listening on the port.
    #[test]
    fn a_refused_docker_socket_is_a_credential_problem_not_a_network_one() {
        let refused = bollard::errors::Error::DockerResponseServerError {
            status_code: 401,
            message: "unauthorized".to_string(),
        };
        assert_eq!(AttemptOutcome::from(&refused), AttemptOutcome::Rejected);

        let other = bollard::errors::Error::DockerResponseServerError {
            status_code: 500,
            message: "boom".to_string(),
        };
        assert_eq!(AttemptOutcome::from(&other), AttemptOutcome::NotThisService);
    }

    /// A certificate the client will not accept is fixed by a trust setting, not by re-typing a
    /// password — so it cannot share a line with a rejection.
    #[test]
    fn a_certificate_problem_is_reported_as_tls_not_as_a_rejection() {
        let error = bollard::errors::Error::CertMultipleKeys {
            count: 2,
            path: std::path::PathBuf::from("/tmp/key.pem"),
        };
        assert_eq!(AttemptOutcome::from(&error), AttemptOutcome::TlsFailed);
    }

    /// SNMPv3 authenticates during engine discovery, so a bad password is a genuine refusal.
    /// This is the case that told Motala their switch was unreachable when the password was
    /// simply wrong.
    #[test]
    fn snmp_auth_failure_is_a_rejection() {
        let error = snmp2::Error::AuthFailure(snmp2::v3::AuthErrorKind::NotAuthenticated);
        assert_eq!(AttemptOutcome::from(&error), AttemptOutcome::Rejected);
    }

    /// Reading someone else's datagram says nothing about the credential. Classifying it as a
    /// rejection would blame the operator's configuration for a transport race.
    #[test]
    fn a_desynced_session_is_not_a_rejection() {
        assert_eq!(
            AttemptOutcome::from(&snmp2::Error::RequestIdMismatch),
            AttemptOutcome::TimedOut
        );
        assert_eq!(
            AttemptOutcome::from(&snmp2::Error::Receive),
            AttemptOutcome::Unreachable
        );
    }

    /// A malformed response means something is listening and it is not SNMP — the operator's
    /// fix is the port, not the community.
    #[test]
    fn a_non_snmp_answer_points_at_the_port() {
        assert_eq!(
            AttemptOutcome::from(&snmp2::Error::AsnParse),
            AttemptOutcome::NotThisService
        );
    }

    /// `ProbeFailure` has no public fields and no `Default`, so this is the only way to build
    /// one. The constructors are the enforcement — a new integration cannot add a failure path
    /// without picking an outcome.
    #[test]
    fn constructors_carry_their_outcome() {
        assert_eq!(
            ProbeFailure::cancelled().outcome(),
            AttemptOutcome::Cancelled
        );
        assert_eq!(
            ProbeFailure::malformed("bad").outcome(),
            AttemptOutcome::Malformed
        );
        let with_context = ProbeFailure::rejected("refused").with_context("after 3 attempts");
        assert_eq!(with_context.outcome(), AttemptOutcome::Rejected);
        assert_eq!(with_context.message(), "after 3 attempts: refused");
    }

    /// An `anyhow` error from an integration degrades to the generic collection failure rather
    /// than forcing every `?` to be rewritten — but the outer timeout is more specific and says
    /// so, since the integration was still working when we stopped waiting.
    #[test]
    fn an_integration_failure_defaults_to_collection_failed() {
        let from_anyhow: IntegrationFailure = anyhow::Error::msg("something broke").into();
        assert_eq!(from_anyhow.outcome(), AttemptOutcome::CollectionFailed);

        assert_eq!(
            IntegrationFailure::with_outcome(AttemptOutcome::TimedOut, "slow").outcome(),
            AttemptOutcome::TimedOut
        );
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;
    use crate::daemon::discovery::service::base::DaemonDiscoveryService;
    use crate::daemon::utils::base::create_system_utils;
    use crate::server::credentials::r#impl::mapping::{
        CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
    };
    use crate::server::credentials::r#impl::types::wake_on_lan::WakeOnLanQueryCredential;
    use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback};
    use std::sync::atomic::Ordering;

    /// The progress an integration reports at 90% of its own work, from a context that owns the
    /// session's progress or not, in the given phase range. Returns the session's progress after.
    async fn progress_after_integration_reports_90(owns_progress: bool, range: (u8, u8)) -> u8 {
        let service = DaemonDiscoveryService::running_for_test(Uuid::new_v4()).await;
        let ops = DiscoveryOps::new(&service, DiscoveryType::default());
        let session = ops.get_session().await.unwrap();
        session.set_progress_range(range.0, range.1);

        let credential = CredentialQueryPayload::WakeOnLan(WakeOnLanQueryCredential {
            port: 9,
            wait_seconds: 0,
            broadcast_address: None,
            secure_on_password: None,
        });
        let utils = create_system_utils();
        let cancel = CancellationToken::new();
        let ctx = IntegrationContext {
            ip: IpAddr::from([10, 0, 0, 5]),
            credential: &credential,
            credential_id: None,
            interface_source: InterfaceSource {
                credential: CredentialQueryPayloadDiscriminants::WakeOnLan,
                scope: CredentialIntegration::WakeOnLan.interface_view_scope(),
            },
            integration: CredentialIntegration::WakeOnLan,
            cancel: &cancel,
            ops: &ops,
            utils: &utils,
            probe_handle: None,
            matched_services: &[],
            open_ports: &[],
            endpoint_responses: &[],
            host_id: Uuid::new_v4(),
            host_naming_fallback: HostNamingFallback::default(),
            known_subnets: &[],
            scanned_subnets: &[],
            accept_invalid_certs: false,
            trusted_ca: None,
            scanning_subnet: None,
            owns_progress,
        };

        ctx.report_progress(90).await;
        session.last_progress.load(Ordering::Relaxed)
    }

    /// Inside a deep scan the network phase spans 5-100%, and an integration's 90 used to map to
    /// 90% overall. The session's progress never falls, so the bar sat there until the scan ended.
    #[tokio::test]
    async fn an_integration_inside_a_deep_scan_leaves_the_sessions_progress_alone() {
        assert_eq!(
            progress_after_integration_reports_90(false, (5, 100)).await,
            0
        );
    }

    /// The daemon-host phase is its integrations' work, so their progress moves its share.
    #[tokio::test]
    async fn an_integration_in_the_daemon_host_phase_moves_the_phases_share() {
        assert_eq!(progress_after_integration_reports_90(true, (0, 5)).await, 4);
    }
}
