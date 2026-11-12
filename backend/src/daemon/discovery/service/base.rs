use std::{
    net::IpAddr,
    sync::{
        Arc,
        atomic::{AtomicI64, AtomicU8, AtomicU32, AtomicU64},
    },
};

use crate::daemon::discovery::service::warnings::{
    ContradictedClaim, CredentialIssue, EqualReachIntegrations, IncompleteInterfaceWalk,
    IncompleteSnmpWalk, MalformedNeighbours, SnmpCollectedNothing, UnreachedHypervisorGuest,
    UnresolvedLldpPorts, VlanRecordingFailed,
};
use crate::daemon::discovery::types::base::DiscoverySessionInfo;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::daemon::{
    discovery::{buffer::EntityBuffer, manager::DaemonDiscoverySessionManager},
    shared::{api_client::DaemonApiClient, config::ConfigStore},
    utils::base::{
        PlatformDaemonUtils, create_system_utils, filtered_own_nics, interfaces_fingerprint,
    },
};
use crate::server::credentials::r#impl::mapping::{CredentialMapping, CredentialQueryPayload};
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback};
use crate::server::subnets::r#impl::base::Subnet;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;

pub struct DiscoveryRunner {
    pub service: Arc<DaemonDiscoveryService>,
    pub manager: Arc<DaemonDiscoverySessionManager>,
    /// The type this run was dispatched as, echoed back verbatim on every
    /// progress update. Kept whole rather than reconstructed, so the server's
    /// stored session keeps the type it started with.
    pub discovery_type: DiscoveryType,
    pub host_id: Uuid,
    pub subnet_ids: Option<Vec<Uuid>>,
    /// Specific addresses to scan (a rescan). `None` sweeps the subnets.
    pub target_ips: Option<std::collections::HashSet<std::net::IpAddr>>,
    /// TCP ports already known on the rescan target, scanned in addition to the
    /// standard discovery set.
    pub extra_ports: Vec<u16>,
    pub host_naming_fallback: HostNamingFallback,
    pub scan_settings: ScanSettings,
    pub credential_mappings: Vec<CredentialMapping<CredentialQueryPayload>>,
    /// The site's subnets as the server sent them with this run. Empty from a server too old
    /// to send them, which only a DaemonPoll daemon can be talking to: it falls back to asking.
    pub known_subnets: Vec<Subnet>,
}

impl DiscoveryRunner {
    /// Build a runner from the dispatched discovery type.
    ///
    /// Returns `None` for the frozen legacy types, which the manager stubs out
    /// rather than running.
    pub fn new(
        service: Arc<DaemonDiscoveryService>,
        manager: Arc<DaemonDiscoverySessionManager>,
        discovery_type: DiscoveryType,
        credential_mappings: Vec<CredentialMapping<CredentialQueryPayload>>,
        known_subnets: Vec<Subnet>,
    ) -> Option<Self> {
        let (host_id, subnet_ids, target_ips, extra_ports, host_naming_fallback, scan_settings) =
            match &discovery_type {
                DiscoveryType::Unified {
                    host_id,
                    subnet_ids,
                    host_naming_fallback,
                    scan_settings,
                } => (
                    *host_id,
                    subnet_ids.clone(),
                    None,
                    Vec::new(),
                    *host_naming_fallback,
                    scan_settings.clone(),
                ),
                DiscoveryType::Rescan {
                    host_id,
                    ips,
                    ports,
                    settings,
                    ..
                } => (
                    *host_id,
                    None,
                    Some(ips.iter().copied().collect()),
                    ports
                        .iter()
                        .filter(|p| p.is_tcp())
                        .map(|p| p.number())
                        .collect(),
                    HostNamingFallback::default(),
                    ScanSettings::from(settings),
                ),
                // Classifies the interfaced subnets and stops: nothing is scanned.
                DiscoveryType::InterfaceRefresh { host_id } => (
                    *host_id,
                    None,
                    None,
                    Vec::new(),
                    HostNamingFallback::default(),
                    ScanSettings::default(),
                ),
                DiscoveryType::SelfReport { .. }
                | DiscoveryType::Network { .. }
                | DiscoveryType::Docker { .. } => return None,
            };

        Some(Self {
            service,
            manager,
            discovery_type,
            host_id,
            subnet_ids,
            target_ips,
            extra_ports,
            host_naming_fallback,
            scan_settings,
            credential_mappings,
            known_subnets,
        })
    }
}

impl From<&DiscoveryRunner> for DiscoveryType {
    fn from(runner: &DiscoveryRunner) -> Self {
        runner.discovery_type.clone()
    }
}

#[derive(Clone)]
pub struct DiscoverySession {
    pub info: DiscoverySessionInfo,
    pub gateway_ips: Vec<IpAddr>,
    pub last_progress: Arc<AtomicU8>,
    pub last_progress_report_time: Arc<AtomicU64>,
    pub hosts_discovered: Arc<AtomicU32>,
    pub estimated_remaining_secs: Arc<AtomicU32>,
    pub progress_range_start: Arc<AtomicU8>,
    pub progress_range_end: Arc<AtomicU8>,
    /// Session updates in a row the server did not accept. The server reaps a session it hears
    /// nothing from for 5 minutes, so a count that keeps climbing in the log predicts that reap.
    pub consecutive_report_failures: Arc<AtomicU32>,
    /// When the network phase began. Its `max_discovery_duration` counts from here, so the
    /// session watchdog does too once it is set.
    pub network_phase_started: Arc<std::sync::OnceLock<std::time::Instant>>,
    /// When the scan last advanced, in Unix milliseconds: a progress rise, a host created, a deep
    /// scan or an integration probe or execute finished. The heartbeat does not stamp it, so it
    /// tells a scan that is moving from a task that is alive and stuck. The watchdog ends a
    /// session that has gone too long without one.
    pub last_activity_ms: Arc<AtomicI64>,
    /// Addresses being deep-scanned now, named in the error when the session stops advancing.
    pub in_flight_deep_scans: Arc<std::sync::Mutex<std::collections::BTreeSet<IpAddr>>>,
    /// Non-fatal warnings accumulated during the run (e.g. the discovery hit its
    /// time limit and left hosts un-scanned). Surfaced in the terminal session
    /// update so the user sees them without the run being marked as failed.
    ///
    /// Push here only for things that happen at most once per run. Anything that
    /// fires per host belongs in one of the typed accumulators below, which are
    /// rendered to a single line at finalize — a `Vec<String>` written per host
    /// multiplies by the host count and drowns the notification.
    ///
    /// Private, so the only way in is [`DiscoverySession::push_warning`], which writes the
    /// warning's log line as it records it.
    warnings: Arc<std::sync::Mutex<Vec<DiscoveryWarning>>>,
    // The three typed accumulators are `pub(super)` — visible throughout `service`, and to
    // nothing outside it. That is deliberate and load-bearing rather than tidiness.
    //
    // An integration observing a problem and an operator being told about it are two different
    // jobs, and only the first belongs to the integration: it alone knows what its error means,
    // and it has no idea whether the credential was one the user pinned to this host or a
    // site default broadcast at every address in the subnet — which is the difference between
    // a finding and hundreds of lines of noise. When these were `pub`, the SNMP integration
    // reached in and pushed directly, so that judgement was simply skipped for its warnings.
    //
    // Integrations now go through `DiscoveryOps`, which applies the policy in one place. The
    // scan pipeline under `service::` still writes directly, because it *is* the layer that
    // knows about scan scope.
    /// Per-host SNMP walks that could not be read in full. See
    /// [`crate::daemon::discovery::service::warnings`].
    pub(super) incomplete_snmp_walks: Arc<std::sync::Mutex<Vec<IncompleteSnmpWalk>>>,
    /// Devices whose own published figures disagree with what the collection read. Not a
    /// shortfall — a shortfall says why we stopped, this says what the device said was there.
    pub(super) contradicted_claims: Arc<std::sync::Mutex<Vec<ContradictedClaim>>>,
    /// Per-host ifTable walks that could not be read in full. Separate from the above because
    /// a truncated interface set and a truncated attribute column mean different things.
    pub(super) incomplete_interface_walks: Arc<std::sync::Mutex<Vec<IncompleteInterfaceWalk>>>,
    /// LLDP neighbours whose local port could not be matched to an interface. Not a shortfall in
    /// a walk — the data arrived, and could not be placed.
    pub(super) unresolved_lldp_ports: Arc<std::sync::Mutex<Vec<UnresolvedLldpPorts>>>,
    /// Neighbour records served without the identifier L2 resolution matches on. Also not a
    /// shortfall in a walk — the rows arrived and are unusable, which no rescan changes.
    pub(super) malformed_neighbours: Arc<std::sync::Mutex<Vec<MalformedNeighbours>>>,
    /// Devices that answered the credential and then returned nothing from any table. Not one
    /// group falling short of the others — there are no others.
    pub(super) snmp_collected_nothing: Arc<std::sync::Mutex<Vec<SnmpCollectedNothing>>>,
    /// Devices whose VLAN table was read and could not be saved.
    pub(super) vlan_recording_failures: Arc<std::sync::Mutex<Vec<VlanRecordingFailed>>>,
    /// Hosts whose interfaces two full-ifTable integrations both collected.
    pub(super) equal_reach_integrations: Arc<std::sync::Mutex<Vec<EqualReachIntegrations>>>,
    /// Running hypervisor guests at addresses no subnet this scan covers.
    pub(super) unreached_guests: Arc<std::sync::Mutex<Vec<UnreachedHypervisorGuest>>>,
    /// IP-targeted credentials that produced nothing, and why.
    pub(super) credential_issues: Arc<std::sync::Mutex<Vec<CredentialIssue>>>,
    /// What each stored credential did, keyed by credential. Not warnings: successes are recorded
    /// too, so the run shows what every credential produced.
    pub(super) credential_results: Arc<
        std::sync::Mutex<
            std::collections::BTreeMap<
                uuid::Uuid,
                crate::server::credentials::r#impl::run_results::CredentialRunOutcome,
            >,
        >,
    >,
}

impl DiscoverySession {
    pub fn new(info: DiscoverySessionInfo, gateway_ips: Vec<IpAddr>) -> Self {
        Self {
            info,
            gateway_ips,
            last_progress: Arc::new(AtomicU8::new(0)),
            last_progress_report_time: Arc::new(AtomicU64::new(0)),
            hosts_discovered: Arc::new(AtomicU32::new(0)),
            estimated_remaining_secs: Arc::new(AtomicU32::new(u32::MAX)),
            progress_range_start: Arc::new(AtomicU8::new(0)),
            progress_range_end: Arc::new(AtomicU8::new(100)),
            consecutive_report_failures: Arc::new(AtomicU32::new(0)),
            network_phase_started: Arc::new(std::sync::OnceLock::new()),
            last_activity_ms: Arc::new(AtomicI64::new(
                crate::server::shared::clock::now().timestamp_millis(),
            )),
            in_flight_deep_scans: Arc::new(std::sync::Mutex::new(Default::default())),
            warnings: Arc::new(std::sync::Mutex::new(Vec::new())),
            incomplete_snmp_walks: Arc::new(std::sync::Mutex::new(Vec::new())),
            contradicted_claims: Arc::new(std::sync::Mutex::new(Vec::new())),
            incomplete_interface_walks: Arc::new(std::sync::Mutex::new(Vec::new())),
            unresolved_lldp_ports: Arc::new(std::sync::Mutex::new(Vec::new())),
            malformed_neighbours: Arc::new(std::sync::Mutex::new(Vec::new())),
            snmp_collected_nothing: Arc::new(std::sync::Mutex::new(Vec::new())),
            vlan_recording_failures: Arc::new(std::sync::Mutex::new(Vec::new())),
            equal_reach_integrations: Arc::new(std::sync::Mutex::new(Vec::new())),
            unreached_guests: Arc::new(std::sync::Mutex::new(Vec::new())),
            credential_issues: Arc::new(std::sync::Mutex::new(Vec::new())),
            credential_results: Arc::new(std::sync::Mutex::new(Default::default())),
        }
    }

    /// Record a warning on the run and write its log line.
    pub fn push_warning(&self, warning: DiscoveryWarning) {
        self.push_warnings([warning]);
    }

    /// Record several warnings on the run, writing each one's log line.
    pub fn push_warnings(&self, new: impl IntoIterator<Item = DiscoveryWarning>) {
        let new: Vec<DiscoveryWarning> = new.into_iter().collect();
        for warning in &new {
            warning.log();
        }
        if let Ok(mut warnings) = self.warnings.lock() {
            warnings.extend(new);
        }
    }

    /// The warnings recorded on the run so far.
    pub fn pushed_warnings(&self) -> Vec<DiscoveryWarning> {
        self.warnings
            .lock()
            .map(|warnings| warnings.clone())
            .unwrap_or_default()
    }

    pub fn set_progress_range(&self, start: u8, end: u8) {
        use std::sync::atomic::Ordering;
        self.progress_range_start.store(start, Ordering::Relaxed);
        self.progress_range_end.store(end, Ordering::Relaxed);
    }

    /// Record that the scan advanced just now.
    pub fn stamp_activity(&self) {
        self.last_activity_ms.store(
            crate::server::shared::clock::now().timestamp_millis(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// When the scan last advanced.
    pub fn last_activity_at(&self) -> chrono::DateTime<chrono::Utc> {
        let ms = self
            .last_activity_ms
            .load(std::sync::atomic::Ordering::Relaxed);
        chrono::DateTime::from_timestamp_millis(ms).unwrap_or_default()
    }

    /// The addresses being deep-scanned now, in address order.
    pub fn in_flight_deep_scans(&self) -> Vec<IpAddr> {
        self.in_flight_deep_scans
            .lock()
            .map(|ips| ips.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Track `ip` as being deep-scanned until the returned guard drops, which also counts as the
    /// scan advancing.
    pub fn track_deep_scan(&self, ip: IpAddr) -> DeepScanInFlight {
        if let Ok(mut ips) = self.in_flight_deep_scans.lock() {
            ips.insert(ip);
        }
        DeepScanInFlight {
            ip,
            session: self.clone(),
        }
    }
}

/// A deep scan in progress. Dropping it, whether the scan finished or its future was dropped,
/// removes the address from the session's in-flight set and stamps activity.
pub struct DeepScanInFlight {
    ip: IpAddr,
    session: DiscoverySession,
}

impl Drop for DeepScanInFlight {
    fn drop(&mut self) {
        if let Ok(mut ips) = self.session.in_flight_deep_scans.lock() {
            ips.remove(&self.ip);
        }
        self.session.stamp_activity();
    }
}

pub struct DaemonDiscoveryService {
    pub config_store: Arc<ConfigStore>,
    /// `None` on a ServerPoll daemon, which the server collects from.
    pub api_client: Option<Arc<DaemonApiClient>>,
    pub utils: PlatformDaemonUtils,
    pub current_session: Arc<RwLock<Option<DiscoverySession>>>,
    pub entity_buffer: Arc<EntityBuffer>,
    /// Stores the terminal state (Complete/Failed/Cancelled) for ServerPoll mode.
    /// In ServerPoll mode, the server polls for progress updates. If the session ends
    /// between polls, we need to retain the terminal state so the server can receive it.
    /// This is cleared when a new session starts.
    pub terminal_payload: Arc<RwLock<Option<DiscoveryUpdatePayload>>>,
    /// Shared gate that staggers the start of DaemonPoll host-create requests so
    /// a burst of near-simultaneous deep-scan completions doesn't hammer the
    /// server's host-create endpoint (where they'd otherwise queue on the
    /// per-site `HostDedup` advisory lock). Holds the earliest instant the
    /// next host submission may start; each submission reserves and advances it.
    pub host_submit_gate: Arc<tokio::sync::Mutex<tokio::time::Instant>>,
    /// The last session's interfaced subnets until a status report takes them.
    interfaced_report: std::sync::Mutex<Option<InterfacedSubnetsReport>>,
}

impl DaemonDiscoveryService {
    pub fn new(
        config_store: Arc<ConfigStore>,
        api_client: Option<Arc<DaemonApiClient>>,
        entity_buffer: Arc<EntityBuffer>,
    ) -> Self {
        Self {
            api_client,
            config_store,
            utils: create_system_utils(),
            current_session: Arc::new(RwLock::new(None)),
            entity_buffer,
            terminal_payload: Arc::new(RwLock::new(None)),
            host_submit_gate: Arc::new(tokio::sync::Mutex::new(tokio::time::Instant::now())),
            interfaced_report: std::sync::Mutex::new(None),
        }
    }

    pub async fn get_session(&self) -> Result<DiscoverySession, anyhow::Error> {
        self.current_session
            .read()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No active discovery session"))
    }

    /// Hold a session's classified interfaced subnets for the next status report.
    pub fn set_interfaced_report(&self, subnets: Vec<Subnet>, fingerprint: String) {
        if let Ok(mut report) = self.interfaced_report.lock() {
            *report = Some(InterfacedSubnetsReport {
                subnets,
                fingerprint,
            });
        }
    }

    /// What a status report says about this daemon's interfaces: the interfaced subnets a session
    /// classified since the last report with the fingerprint they were classified from, or no
    /// subnets and the fingerprint of the NICs as they are now. `None` for the fingerprint only
    /// when the interface filter cannot be read.
    pub async fn status_interfaces(&self) -> (Option<Vec<Subnet>>, Option<String>) {
        if let Some(report) = self
            .interfaced_report
            .lock()
            .ok()
            .and_then(|mut r| r.take())
        {
            return (Some(report.subnets), Some(report.fingerprint));
        }
        let fingerprint = self
            .config_store
            .get_interfaces()
            .await
            .ok()
            .map(|filter| interfaces_fingerprint(&filtered_own_nics(&filter)));
        (None, fingerprint)
    }
}

#[cfg(test)]
impl DaemonDiscoveryService {
    /// A ServerPoll service, which reaches no server, already running session `session_id`.
    pub async fn running_for_test(session_id: Uuid) -> Arc<Self> {
        use crate::daemon::shared::config::AppConfig;
        let config = Arc::new(ConfigStore::new(
            std::env::temp_dir().join(format!("scanopy-test-{session_id}.json")),
            AppConfig::default(),
        ));
        let service = Arc::new(Self::new(config, None, Arc::new(EntityBuffer::new())));
        let info = DiscoverySessionInfo {
            session_id,
            site_id: Uuid::new_v4(),
            daemon_id: Uuid::new_v4(),
            started_at: Some(crate::server::shared::clock::now()),
            discovery_type: DiscoveryType::default(),
            discovery_id: Uuid::new_v4(),
        };
        *service.current_session.write().await = Some(DiscoverySession::new(info, Vec::new()));
        service
    }
}

/// The interfaced subnets a session classified, waiting for the next status report.
pub struct InterfacedSubnetsReport {
    pub subnets: Vec<Subnet>,
    /// [`interfaces_fingerprint`] of the NICs they were classified from.
    pub fingerprint: String,
}
