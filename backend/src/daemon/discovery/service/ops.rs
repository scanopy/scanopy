//! Shared discovery operations used by both the pipeline and integrations.
//!
//! `DiscoveryOps` provides entity creation, service matching, and progress reporting
//! without requiring `DiscoveryRunner` or its associated traits.

use std::{collections::HashSet, net::IpAddr, sync::Arc, time::Duration};

use anyhow::{Error, anyhow};
use backon::{ExponentialBuilder, Retryable};
use chrono::Utc;
use mac_address::MacAddress;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    daemon::{
        discovery::{
            buffer::EntityBuffer,
            service::warnings,
            types::base::{
                DiscoveryCriticalError, DiscoveryPhase, DiscoverySessionInfo,
                DiscoverySessionUpdate,
            },
            types::warnings::DiscoveryWarning,
        },
        shared::{api_client::DaemonApiClient, config::ConfigStore},
    },
    server::{
        credentials::r#impl::{
            mapping::CredentialQueryPayloadDiscriminants, types::CredentialAssignment,
        },
        daemons::r#impl::{
            api::{DaemonDiscoveryRequest, DiscoveryUpdatePayload},
            base::DaemonMode,
        },
        discovery::r#impl::types::{DiscoveryType, HostNamingFallback},
        hosts::r#impl::{
            api::{DiscoveryHostRequest, HostResponse},
            attributes::{
                HostChassisIdValue, HostFirmwareRevisionValue, HostHostnameAttributed,
                HostHostnameValue, HostManagementUrlValue, HostManufacturerValue, HostModelValue,
                HostSerialNumberValue, HostSoftwareRevisionValue, HostSysContactValue,
                HostSysDescrValue, HostSysLocationValue, HostSysNameValue, HostSysObjectIdValue,
            },
            base::{Host, HostBase},
            name::{HostName, HostNameSources},
            virtualization::HostVirtualization,
        },
        interfaces::{
            r#impl::base::{Interface, InterfaceDataComplete},
            service::match_existing_interface,
        },
        ip_addresses::r#impl::base::{IPAddress, MacEvidence, MacEvidenceValue},
        ports::r#impl::base::{Port, PortType},
        services::{
            definitions::{ServiceDefinitionRegistry, gateway::Gateway},
            r#impl::{
                base::{
                    DiscoverySessionServiceMatchParams, Service, ServiceMatchBaselineParams,
                    ServiceMatchServiceParams,
                },
                definitions::{ServiceDefinition, ServiceDefinitionExt},
                patterns::MatchConfidence,
            },
        },
        shared::{
            attribution::{AttributeSource, Attributed},
            types::api::ApiErrorResponse,
            types::entities::EntitySource,
            types::metadata::HasId,
        },
        subnets::r#impl::base::Subnet,
    },
};

use super::base::DaemonDiscoveryService;
use crate::daemon::discovery::integration::{InterfaceSource, InterfaceViewScope};

/// Default number of retries for entity creation during discovery.
const ENTITY_CREATION_MAX_RETRIES: usize = 5;

/// Timeout for waiting for server confirmation in ServerPoll mode.
const SERVER_POLL_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(120);

/// Minimum spacing between the *start* of consecutive DaemonPoll host-create
/// requests. Deep scans complete in near-simultaneous bursts; without this the
/// daemon fires many host-creates at once and they pile up on the server's
/// per-network `HostDedup` advisory lock, spiking the endpoint. ~40 req/s steady
/// ceiling — matched to the server's serialized create throughput, so this
/// flattens the arrival burst without costing real throughput.
const MIN_HOST_SUBMIT_INTERVAL: Duration = Duration::from_millis(25);

/// Reserve the next submission slot on `gate`, advancing it by `interval`, and
/// return the instant this caller may start. Concurrent callers each get a
/// distinct slot spaced `interval` apart. Reservation is O(1) under the lock —
/// a slow/retrying request that already reserved its slot never blocks later
/// callers from reserving theirs (they stagger the *start*, not the completion).
async fn reserve_submit_slot(
    gate: &tokio::sync::Mutex<tokio::time::Instant>,
    interval: Duration,
) -> tokio::time::Instant {
    let mut next = gate.lock().await;
    let at = (*next).max(tokio::time::Instant::now());
    *next = at + interval;
    at
}

/// Mutable host state passed to integration execute() methods.
/// Integrations enrich the host via builder methods.
///
/// `Clone` is load-bearing: `execute_with_progress_reporting` hands each integration a scratch
/// clone and merges it back only on success, so a timeout that drops the future cannot leave a
/// half-written host (GH #650). Deriving it rather than hand-listing the fields to copy means a
/// new field is covered without anyone remembering to add it.
#[derive(Clone)]
pub struct HostData {
    pub host: Host,
    pub services: Vec<Service>,
    pub ports: Vec<Port>,
    pub ip_addresses: Vec<IPAddress>,
    pub interfaces: Vec<Interface>,
    pub subnets: Vec<Subnet>,
    /// Whether `interfaces` is a complete, authoritative view of the host's ifTable. Set to false
    /// by the SNMP integration when the ifTable walk was cut short (timeout/error), so the server
    /// skips the stale-interface prune and cannot tear down L2 topology on a partial scan (#649).
    /// Defaults to true — integrations that don't walk the ifTable report an authoritative (usually
    /// empty) interface set, and the empty set is guarded server-side regardless.
    pub interfaces_complete: bool,
    /// Which groups of per-interface data (LLDP, CDP, FDB, VLAN membership) this scan read in
    /// full. A group read only partially must not overwrite what the server already holds — an
    /// empty result from a cut-short walk is indistinguishable from a device reporting nothing.
    pub interface_data_complete: InterfaceDataComplete,
    /// Every integration that has contributed interfaces to this host in this scan, newest last.
    ///
    /// Kept so a contributor can revise its own set without disturbing anyone else's, and so the
    /// merged completeness can be derived from who actually survived rather than from whoever
    /// wrote last. Not sent to the server — `interfaces`, `interfaces_complete` and
    /// `interface_data_complete` are the merged result.
    contributions: Vec<InterfaceContribution>,
    /// The first two full-ifTable integrations to collect this host, in the order they
    /// contributed. `HostData` has no route to the session's warning buffers, so the detection is
    /// held here and the runner records it where the host is submitted.
    equal_reach_integrations: Option<(
        CredentialQueryPayloadDiscriminants,
        CredentialQueryPayloadDiscriminants,
    )>,
}

/// One integration's interface set, as offered.
#[derive(Clone)]
struct InterfaceContribution {
    source: InterfaceSource,
    interfaces: Vec<Interface>,
    interfaces_complete: bool,
    data_complete: InterfaceDataComplete,
}

impl HostData {
    pub fn new(
        host: Host,
        services: Vec<Service>,
        ports: Vec<Port>,
        ip_addresses: Vec<IPAddress>,
        interfaces: Vec<Interface>,
        subnets: Vec<Subnet>,
    ) -> Self {
        Self {
            host,
            services,
            ports,
            ip_addresses,
            interfaces,
            subnets,
            interfaces_complete: true,
            interface_data_complete: InterfaceDataComplete::default(),
            contributions: Vec::new(),
            equal_reach_integrations: None,
        }
    }

    // --- Field builders ---
    //
    // Each takes the source that read the value, and the applier decides whether it lands. These
    // used to gate on `is_none()`, so within a single scan whichever integration ran first owned
    // the field and a better reading arriving later was dropped — precedence was the order SNMP,
    // the controllers and the app probes happen to run in, which nothing stated and nothing
    // enforced.
    pub fn with_sys_descr(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.sys_descr,
            Attributed::new(HostSysDescrValue(v), source),
        );
        self
    }

    pub fn with_sys_name(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.sys_name,
            Attributed::new(HostSysNameValue(v), source),
        );
        self
    }

    pub fn with_sys_object_id(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.sys_object_id,
            Attributed::new(HostSysObjectIdValue(v), source),
        );
        self
    }

    pub fn with_sys_location(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.sys_location,
            Attributed::new(HostSysLocationValue(v), source),
        );
        self
    }

    pub fn with_sys_contact(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.sys_contact,
            Attributed::new(HostSysContactValue(v), source),
        );
        self
    }

    pub fn with_chassis_id(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.chassis_id,
            Attributed::new(HostChassisIdValue(v), source),
        );
        self
    }

    pub fn with_manufacturer(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.manufacturer,
            Attributed::new(HostManufacturerValue(v), source),
        );
        self
    }

    pub fn with_model(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.model,
            Attributed::new(HostModelValue(v), source),
        );
        self
    }

    pub fn with_serial_number(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.serial_number,
            Attributed::new(HostSerialNumberValue(v), source),
        );
        self
    }

    pub fn with_firmware_revision(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.firmware_revision,
            Attributed::new(HostFirmwareRevisionValue(v), source),
        );
        self
    }

    pub fn with_software_revision(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.software_revision,
            Attributed::new(HostSoftwareRevisionValue(v), source),
        );
        self
    }

    pub fn with_management_url(&mut self, v: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.management_url,
            Attributed::new(HostManagementUrlValue(v), source),
        );
        self
    }

    pub fn with_virtualization(&mut self, v: HostVirtualization) -> &mut Self {
        if self.host.base.virtualization_metadata.is_none() {
            self.host.base.virtualization_metadata = Some(v);
        }
        self
    }

    /// Set MAC on the interface matching the given IP address.
    /// Used by SNMP to enrich MAC from ipAddrTable when ARP didn't provide one.
    pub fn with_mac_for_ip(
        &mut self,
        ip: IpAddr,
        mac: MacAddress,
        source: AttributeSource,
    ) -> &mut Self {
        if let Some(ip_address) = self
            .ip_addresses
            .iter_mut()
            .find(|i| i.base.ip_address == ip)
        {
            Attributed::apply(
                &mut ip_address.base.mac_address,
                MacEvidence::new(MacEvidenceValue(mac), source),
            );
        }
        self
    }

    // --- Append methods: multiple integrations can contribute ---

    pub fn add_service(&mut self, s: Service) -> &mut Self {
        self.services.push(s);
        self
    }

    pub fn add_port(&mut self, p: Port) -> &mut Self {
        self.ports.push(p);
        self
    }

    pub fn add_ip_address(&mut self, i: IPAddress) -> &mut Self {
        self.ip_addresses.push(i);
        self
    }

    /// Offer an interface set on behalf of one integration, merged with what others already gave.
    ///
    /// Several integrations can collect one host in a single scan against this one `HostData`.
    /// This used to be `replace_interfaces`, a plain setter, so the last writer won and the only
    /// brake was a hand-written `if !host_data.interfaces.is_empty()` in the two controller
    /// integrations — order-dependent, untested, and reading "richer" as "non-empty".
    ///
    /// **Nothing offered here is ever discarded.** Picking a winner is not safe even between two
    /// `FullIfTable` collectors: a device can answer SNMP with a thin or empty ifTable while
    /// answering gNMI with the real one, and with network-wide credentials which of them is asked
    /// first is arbitrary. So sets are unioned, and scope only decides which row wins where two
    /// contributors describe the *same* interface.
    ///
    /// Rules:
    /// - A contributor replaces its own previous offer. SNMP persists a bare interface set at a
    ///   checkpoint and swaps in the enriched one later; that is a revision, not a second opinion.
    /// - Interfaces are matched the way the server matches them — `if_name`, then `if_index`,
    ///   then a unique MAC — so the daemon cannot decide two rows are one interface where the
    ///   server would not, or the reverse.
    /// - Where two contributors describe one interface, the wider scope wins the whole row; on
    ///   equal scope the earlier contributor keeps it. Arbitrary, but bounded to one row.
    /// - Anything only a narrower contributor has is added.
    ///
    /// Completeness is then derived, never inherited: see `merge_contributions`.
    pub fn contribute_interfaces(
        &mut self,
        source: InterfaceSource,
        interfaces: Vec<Interface>,
        interfaces_complete: bool,
        data_complete: InterfaceDataComplete,
    ) -> &mut Self {
        let offer = InterfaceContribution {
            source,
            interfaces,
            interfaces_complete,
            data_complete,
        };

        match self
            .contributions
            .iter_mut()
            .find(|c| c.source.credential == source.credential)
        {
            Some(existing) => *existing = offer,
            None => {
                // Two integrations of equal reach on one host, such as SNMP and gNMI on one
                // switch, is a supported configuration. Two full-ifTable readers are surfaced to
                // the operator as an informational discovery warning.
                if let Some(peer) = self.contributions.iter().find(|c| {
                    c.source.scope == source.scope
                        && source.scope != InterfaceViewScope::NoInterfaces
                }) {
                    tracing::debug!(
                        host = %self.host.id,
                        first = ?peer.source.credential,
                        second = ?source.credential,
                        scope = ?source.scope,
                        "Two integrations of equal interface reach collected one host; their \
                         interface sets are merged"
                    );
                    if source.scope == InterfaceViewScope::FullIfTable
                        && self.equal_reach_integrations.is_none()
                    {
                        self.equal_reach_integrations =
                            Some((peer.source.credential, source.credential));
                    }
                }
                self.contributions.push(offer);
            }
        }

        self.merge_contributions();
        self
    }

    /// Fold every contribution into the interface set and the two completeness verdicts.
    ///
    /// The verdicts are the half that keeps merging safe, and both are deliberately pessimistic:
    ///
    /// - `interfaces_complete` — the flag that authorises the server to delete interfaces it holds
    ///   and no longer sees — is true only if a `FullIfTable` contributor said its own walk
    ///   finished *and* nobody else added a row beyond it. A row from outside the ifTable view is
    ///   proof the ifTable view was not the whole host, and pruning against it would delete the
    ///   very rows that proved it.
    /// - Each `InterfaceDataComplete` group is authoritative only if every contributor holding a
    ///   surviving row read that group in full. A contributor that never looks at CDP must not
    ///   lend its silence the authority to clear a CDP column somebody else collected.
    ///
    /// In the ordinary case — a controller's physical ports being a subset of an SNMP ifTable —
    /// nothing is added and both verdicts come out exactly as the ifTable contributor stated them.
    fn merge_contributions(&mut self) {
        let mut order: Vec<&InterfaceContribution> = self.contributions.iter().collect();
        // Widest view first, ties in arrival order, so the first writer of any row is the one
        // whose description of it stands.
        order.sort_by(|a, b| b.source.scope.cmp(&a.source.scope));

        let mut merged: Vec<Interface> = Vec::new();
        // Which contribution supplied each surviving row, positionally.
        let mut owners: Vec<InterfaceSource> = Vec::new();

        for contribution in &order {
            // Scoped to one contribution: two of *its* rows must not collapse onto one merged row
            // (the server's own #614 guard), but every merged row is a candidate again for the
            // next contributor.
            let mut claimed: HashSet<Uuid> = HashSet::new();
            for entry in &contribution.interfaces {
                if let Some(id) = match_existing_interface(entry, &merged, &claimed) {
                    // A wider view already described this interface, or an equal one got here
                    // first. Either way the row that stands is the one already in.
                    claimed.insert(id);
                    continue;
                }
                claimed.insert(entry.id);
                merged.push(entry.clone());
                owners.push(contribution.source);
            }
        }

        // Authority to delete belongs to an ifTable view, and only while it is the whole story: a
        // surviving row from a narrower contributor is proof that it was not.
        self.interfaces_complete = match order
            .iter()
            .find(|c| c.source.scope == InterfaceViewScope::FullIfTable)
        {
            Some(c) => {
                c.interfaces_complete
                    && !owners
                        .iter()
                        .any(|o| o.scope != InterfaceViewScope::FullIfTable)
            }
            None => false,
        };

        let surviving: HashSet<CredentialQueryPayloadDiscriminants> =
            owners.iter().map(|o| o.credential).collect();
        self.interface_data_complete = order
            .iter()
            .filter(|c| surviving.contains(&c.source.credential))
            .fold(InterfaceDataComplete::default(), |acc, c| {
                InterfaceDataComplete {
                    lldp: acc.lldp && c.data_complete.lldp,
                    cdp: acc.cdp && c.data_complete.cdp,
                    fdb: acc.fdb && c.data_complete.fdb,
                    vlan_membership: acc.vlan_membership && c.data_complete.vlan_membership,
                }
            });

        // Which source won each port, for whoever is chasing a missing edge: a link only the
        // losing contributor saw on a shared port is not drawn.
        if self.contributions.len() > 1 {
            let supplied: Vec<String> = order
                .iter()
                .map(|c| {
                    let rows: Vec<&Interface> = merged
                        .iter()
                        .zip(&owners)
                        .filter(|(_, owner)| owner.credential == c.source.credential)
                        .map(|(row, _)| row)
                        .collect();
                    let names: Vec<&str> = rows
                        .iter()
                        .filter_map(|row| row.base.if_name.as_deref())
                        .collect();
                    if names.len() == rows.len() {
                        format!("{:?}: [{}]", c.source.credential, names.join(", "))
                    } else {
                        format!("{:?}: {} interface(s)", c.source.credential, rows.len())
                    }
                })
                .collect();
            tracing::debug!(
                host = %self.host.id,
                supplied = %supplied.join("; "),
                "Merged interface contributions; each source lists the rows it supplied"
            );
        }

        self.interfaces = merged;
    }

    /// The first two full-ifTable integrations to collect this host, in contribution order, if
    /// two did.
    pub fn equal_reach_integrations(
        &self,
    ) -> Option<(
        CredentialQueryPayloadDiscriminants,
        CredentialQueryPayloadDiscriminants,
    )> {
        self.equal_reach_integrations
    }

    /// Every integration that has offered interfaces for this host, widest view first.
    ///
    /// The dispatch layer reads this to warn when two collectors of equal reach are pointed at one
    /// host — a configuration to tell the operator about, not a conflict to resolve here.
    pub fn interface_contributors(&self) -> Vec<InterfaceSource> {
        let mut sources: Vec<InterfaceSource> =
            self.contributions.iter().map(|c| c.source).collect();
        sources.sort_by(|a, b| b.scope.cmp(&a.scope));
        sources
    }

    /// Offer a subnet, ignoring one already present.
    ///
    /// Keyed on `Subnet`'s own equality (CIDR + network), which is the same natural key the
    /// server deduplicates on, so the two cannot drift apart. This was a bare push, and
    /// `container::execute` runs several times against one host — once per Docker/Podman
    /// socket/proxy credential type, and again for the sweep phase after the daemon-host phase —
    /// so the same bridge network was offered five or six times per scan (GH #650).
    pub fn add_subnet(&mut self, s: Subnet) -> &mut Self {
        if !self.subnets.contains(&s) {
            self.subnets.push(s);
        }
        self
    }

    pub fn add_credential_assignment(&mut self, ca: CredentialAssignment) -> &mut Self {
        self.host.base.credential_assignments.push(ca);
        self
    }

    /// Record a hostname an integration learned for the host being scanned, with its source.
    ///
    /// Ranked like every other attribute, so a controller's DHCP hostname fills in where the scan's
    /// own lookup found nothing and never displaces a stronger reading. A hostname is an identifier,
    /// not a name, so this leaves `name` alone (see the placement rule in `hosts::impl::name`).
    pub fn with_hostname(&mut self, hostname: String, source: AttributeSource) -> &mut Self {
        Attributed::apply(
            &mut self.host.base.hostname,
            Attributed::new(HostHostnameValue(hostname), source),
        );
        self
    }

    /// Apply a name an integration learned for the host being scanned.
    ///
    /// Integrations reach this through [`ControllerIdentity::enrich`]; it is here rather than
    /// inline in each integration so the precedence question is answered in one place.
    ///
    /// [`ControllerIdentity::enrich`]: crate::daemon::discovery::integration::controller::ControllerIdentity::enrich
    pub fn apply_name(&mut self, candidate: HostName) -> &mut Self {
        self.host.base.apply_name(candidate);
        self
    }
}

/// Shared discovery operations for both the pipeline and integrations.
///
/// Provides entity creation, service matching, and progress reporting
/// without requiring `DiscoveryRunner`.
pub struct DiscoveryOps {
    pub config_store: Arc<ConfigStore>,
    pub api_client: Arc<DaemonApiClient>,
    pub entity_buffer: Arc<EntityBuffer>,
    pub discovery_type: DiscoveryType,
    session: Arc<tokio::sync::RwLock<Option<super::base::DiscoverySession>>>,
    /// Stores the terminal state (Complete/Failed/Cancelled) for ServerPoll mode.
    /// In ServerPoll mode, the server polls for progress updates. If the session ends
    /// between polls, we need to retain the terminal state so the server can receive it.
    /// This is cleared when a new session starts.
    pub terminal_payload: Arc<RwLock<Option<DiscoveryUpdatePayload>>>,
    /// Shared with `DaemonDiscoveryService`: staggers DaemonPoll host-create
    /// request starts. See `create_host`.
    host_submit_gate: Arc<tokio::sync::Mutex<tokio::time::Instant>>,
}

impl DiscoveryOps {
    pub fn new(service: &DaemonDiscoveryService, discovery_type: DiscoveryType) -> Self {
        Self {
            config_store: service.config_store.clone(),
            api_client: service.api_client.clone(),
            entity_buffer: service.entity_buffer.clone(),
            discovery_type,
            session: service.current_session.clone(),
            terminal_payload: service.terminal_payload.clone(),
            host_submit_gate: service.host_submit_gate.clone(),
        }
    }

    pub async fn daemon_id(&self) -> Result<Uuid, Error> {
        self.config_store.get_id().await
    }

    pub async fn network_id(&self) -> Result<Uuid, Error> {
        self.config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow!("Network ID not set"))
    }

    pub async fn get_session(&self) -> Result<super::base::DiscoverySession, Error> {
        self.session
            .read()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("No active discovery session"))
    }

    // --- Session lifecycle methods ---

    /// Initialize a discovery session: set up session info, gateway IPs, clear terminal payload,
    /// create DiscoverySession.
    pub async fn initialize_session(
        &self,
        request: &DaemonDiscoveryRequest,
        daemon_id: Uuid,
        gateway_ips: Vec<IpAddr>,
    ) -> Result<(), Error> {
        tracing::debug!(
            "Setting session info for {} discovery session {}",
            request.discovery_type,
            request.session_id
        );
        let network_id = self
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow!("Network ID not set, aborting discovery session"))?;

        let session_info = DiscoverySessionInfo {
            session_id: request.session_id,
            network_id,
            daemon_id,
            started_at: Some(Utc::now()),
            discovery_type: request.discovery_type.clone(),
            discovery_id: request.discovery_id,
        };

        let session = super::base::DiscoverySession::new(session_info, gateway_ips);

        // Clear terminal payload from previous session (if any) when starting new session
        let mut terminal_payload = self.terminal_payload.write().await;
        *terminal_payload = None;
        drop(terminal_payload);

        let mut current_session = self.session.write().await;
        *current_session = Some(session);

        Ok(())
    }

    /// Start a discovery session: initialize session, report "Started" update.
    pub async fn start_session(
        &self,
        request: &DaemonDiscoveryRequest,
        gateway_ips: Vec<IpAddr>,
    ) -> Result<(), Error> {
        let daemon_id = self.config_store.get_id().await?;

        tracing::info!(
            "Starting {} discovery session {}",
            request.discovery_type,
            request.session_id
        );

        self.initialize_session(request, daemon_id, gateway_ips)
            .await?;

        self.report_discovery_update(DiscoverySessionUpdate {
            phase: DiscoveryPhase::Started,
            progress: 0,
            error: None,
            warnings: Vec::new(),
            finished_at: None,
        })
        .await?;

        let session = self.get_session().await?;

        tracing::info!(
            session_id = %session.info.session_id,
            discovery_type = ?self.discovery_type,
            "Discovery session started"
        );

        Ok(())
    }

    /// Finish a discovery session: report terminal state, store terminal payload for ServerPoll,
    /// clear entity buffer and session.
    pub async fn finish_session(
        &self,
        discovery_result: Result<(), Error>,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        let session = self.get_session().await?;
        let session_id = session.info.session_id;

        let final_progress = session
            .last_progress
            .load(std::sync::atomic::Ordering::Relaxed);

        // Non-fatal findings accumulated during the run (e.g. hit the time limit), plus one coded
        // warning per occurrence for each kind that fires per host — see
        // `crate::daemon::discovery::service::warnings` for why those are recorded typed and coded
        // here rather than pushed as sentences when they happen.
        let mut warnings = session
            .warnings
            .lock()
            .map(|w| w.clone())
            .unwrap_or_default();

        if let Ok(records) = session.incomplete_interface_walks.lock() {
            warnings.extend(warnings::warn_incomplete_interface_walks(&records));
        }
        if let Ok(records) = session.incomplete_snmp_walks.lock() {
            warnings.extend(warnings::warn_incomplete_snmp_walks(&records));
        }
        // Directly after the shortfalls, because on a device that produced both the two warnings
        // are halves of one story: why the read stopped, and what the device said was waiting.
        if let Ok(records) = session.contradicted_claims.lock() {
            warnings.extend(warnings::warn_contradicted_claims(&records));
        }
        if let Ok(records) = session.unresolved_lldp_ports.lock() {
            warnings.extend(warnings::warn_unresolved_lldp_ports(&records));
        }
        if let Ok(records) = session.malformed_neighbours.lock() {
            warnings.extend(warnings::warn_malformed_neighbours(&records));
        }
        if let Ok(records) = session.snmp_collected_nothing.lock() {
            warnings.extend(warnings::warn_snmp_collected_nothing(&records));
        }
        if let Ok(records) = session.vlan_recording_failures.lock() {
            warnings.extend(warnings::warn_vlan_recording_failures(&records));
        }
        if let Ok(records) = session.equal_reach_integrations.lock() {
            warnings.extend(warnings::warn_equal_reach_integrations(&records));
        }
        if let Ok(issues) = session.credential_issues.lock() {
            warnings.extend(warnings::warn_credential_issues(&issues));
        }

        truncate_warnings(&mut warnings);

        // Build the terminal update based on result
        let terminal_update = match &discovery_result {
            Ok(_) => {
                tracing::info!(
                    session_id = %session_id,
                    progress = 100,
                    warnings = warnings.len(),
                    "Discovery session completed successfully"
                );
                DiscoverySessionUpdate {
                    phase: DiscoveryPhase::Complete,
                    progress: 100,
                    error: None,
                    warnings,
                    finished_at: Some(Utc::now()),
                }
            }
            Err(_) if cancel.is_cancelled() => {
                tracing::warn!(
                    session_id = %session_id,
                    progress = %final_progress,
                    "Discovery session cancelled"
                );
                DiscoverySessionUpdate {
                    phase: DiscoveryPhase::Cancelled,
                    progress: final_progress,
                    error: None,
                    warnings,
                    finished_at: Some(Utc::now()),
                }
            }
            Err(e) => {
                tracing::error!(
                    session_id = %session_id,
                    progress = %final_progress,
                    error = %e,
                    "Discovery session failed"
                );

                let error = DiscoveryCriticalError::from_error_string(e.to_string())
                    .map(|e| e.to_string())
                    .unwrap_or(format!("Critical error: {}", e));

                cancel.cancel();
                DiscoverySessionUpdate {
                    phase: DiscoveryPhase::Failed,
                    progress: final_progress,
                    error: Some(error),
                    warnings,
                    finished_at: Some(Utc::now()),
                }
            }
        };

        // Snapshot canonical IDs of entities scanned this session BEFORE
        // clear_all wipes the buffer. Rides the terminal payload over the
        // wire as `DiscoveryUpdatePayload.scanned`; per-entity FK-update
        // subscribers consume it server-side via the in-memory
        // `EntityOperation::Created` event for the historical Discovery row.
        let scanned = self.entity_buffer.get_scanned_ids().await;

        // Report the terminal update (in DaemonPoll mode, this POSTs to server)
        self.report_discovery_update_with_scanned(terminal_update.clone(), Some(scanned.clone()))
            .await?;

        // Store terminal payload for ServerPoll mode - the server polls for progress
        // and needs to receive the terminal state even after current_session is cleared.
        // This payload persists until a new session starts.
        let mut terminal_payload = DiscoveryUpdatePayload::from_state_and_update(
            self.discovery_type.clone(),
            session.info.clone(),
            terminal_update,
        );
        terminal_payload.scanned = Some(scanned);
        let mut stored_terminal = self.terminal_payload.write().await;
        *stored_terminal = Some(terminal_payload);
        drop(stored_terminal);

        // Clear entity buffer - all await_*() calls have completed by now
        // (either successfully found Created entries or timed out)
        self.entity_buffer.clear_all().await;

        let mut current_session = self.session.write().await;
        if let Some(session) = current_session.as_ref()
            && session.info.session_id == session_id
        {
            *current_session = None;
        }

        if cancel.is_cancelled() {
            return Ok(());
        }

        Ok(())
    }

    /// Report a discovery update to the server. Non-critical: logs errors but doesn't fail.
    async fn report_discovery_update(&self, update: DiscoverySessionUpdate) -> Result<(), Error> {
        self.report_discovery_update_with_scanned(update, None)
            .await
    }

    /// Report a discovery update to the server, optionally including the
    /// terminal `ScannedEntityIds` payload for FK-update subscribers.
    /// Non-critical: logs errors but doesn't fail.
    async fn report_discovery_update_with_scanned(
        &self,
        update: DiscoverySessionUpdate,
        scanned: Option<crate::server::daemons::r#impl::api::ScannedEntityIds>,
    ) -> Result<(), Error> {
        use std::sync::atomic::Ordering;

        let session = self.get_session().await?;

        // Map progress for scanning updates through the session's progress range
        let update = if update.phase == DiscoveryPhase::Scanning {
            let start = session.progress_range_start.load(Ordering::Relaxed);
            let end = session.progress_range_end.load(Ordering::Relaxed);
            DiscoverySessionUpdate {
                progress: map_progress(update.progress, start, end),
                ..update
            }
        } else {
            update
        };

        let mut payload = DiscoveryUpdatePayload::from_state_and_update(
            self.discovery_type.clone(),
            session.info.clone(),
            update,
        );
        payload.scanned = scanned;

        // Populate estimation fields from session atomics
        let hosts = session.hosts_discovered.load(Ordering::Relaxed);
        if hosts > 0 {
            payload.hosts_discovered = Some(hosts);
        }
        let estimate = session.estimated_remaining_secs.load(Ordering::Relaxed);
        if estimate != u32::MAX {
            payload.estimated_remaining_secs = Some(estimate);
        }

        let path = format!("/api/v1/discovery/{}/update", session.info.session_id);

        // Progress updates are non-critical - log errors but don't fail discovery
        if let Err(e) = self
            .api_client
            .post_no_data(&path, &payload, "Failed to report discovery update")
            .await
        {
            tracing::warn!(
                session_id = %session.info.session_id,
                error = %e,
                "Failed to report discovery update"
            );
        } else {
            tracing::trace!(
                "Discovery update reported for session {}",
                session.info.session_id
            );
        }

        Ok(())
    }

    /// Record a credential attempt that did not succeed, if it is worth telling the operator.
    ///
    /// The one route an integration has to the operator, and the reason the session's buffers are
    /// not `pub`. Whether an attempt is a *finding* is not an integration's call to make — it
    /// turns on whether the user pinned this credential to this host or it is a network default
    /// tried at every address in the subnet, which the integration cannot see. The judgement
    /// lives in [`warnings::issue_for_attempt`] so both callers share it.
    pub async fn record_attempt_failure(
        &self,
        integration: CredentialQueryPayloadDiscriminants,
        ip: std::net::IpAddr,
        outcome: warnings::AttemptOutcome,
        message: String,
        user_assigned: bool,
        credential_id: Option<uuid::Uuid>,
    ) {
        let Some(issue) = warnings::issue_for_attempt(
            integration,
            ip,
            outcome,
            message,
            user_assigned,
            credential_id,
        ) else {
            return;
        };
        if let Ok(session) = self.get_session().await
            && let Ok(mut issues) = session.credential_issues.lock()
        {
            issues.push(issue);
        }
    }

    /// Record the SNMP data groups this host came up short on.
    pub async fn record_snmp_shortfalls(&self, records: Vec<warnings::IncompleteSnmpWalk>) {
        if records.is_empty() {
            return;
        }
        if let Ok(session) = self.get_session().await
            && let Ok(mut buffer) = session.incomplete_snmp_walks.lock()
        {
            buffer.extend(records);
        }
    }

    /// Record the device's own figures that this host's collection contradicted.
    ///
    /// Kept apart from [`Self::record_snmp_shortfalls`] because the two answer different
    /// questions and a device can warrant both: one says why the read stopped, the other says
    /// what the device claimed was there to read.
    pub async fn record_contradicted_claims(&self, records: Vec<warnings::ContradictedClaim>) {
        if records.is_empty() {
            return;
        }
        if let Ok(session) = self.get_session().await
            && let Ok(mut buffer) = session.contradicted_claims.lock()
        {
            buffer.extend(records);
        }
    }

    /// Hand a batch of already-classified credential issues to the session.
    ///
    /// `probe_integrations` has no session handle, so it returns its issues for the caller to
    /// deliver — and a caller that forgets makes them disappear after being correctly built. That
    /// is exactly what the localhost phase did, so a wrong Docker socket credential on the daemon
    /// host reported nothing at all. One method both callers share, rather than the delivery being
    /// re-implemented (or not) per phase.
    pub async fn record_credential_issues(&self, issues: &[warnings::CredentialIssue]) {
        if issues.is_empty() {
            return;
        }
        if let Ok(session) = self.get_session().await
            && let Ok(mut buffer) = session.credential_issues.lock()
        {
            buffer.extend(issues.iter().cloned());
        }
    }

    /// Record LLDP neighbours whose local port could not be matched to an interface.
    pub async fn record_unresolved_lldp_ports(&self, record: warnings::UnresolvedLldpPorts) {
        if record.unresolved == 0 && record.dropped == 0 {
            return;
        }
        if let Ok(session) = self.get_session().await
            && let Ok(mut buffer) = session.unresolved_lldp_ports.lock()
        {
            buffer.push(record);
        }
    }

    /// Record a device that answered the credential and returned nothing from any table.
    pub async fn record_snmp_collected_nothing(&self, record: warnings::SnmpCollectedNothing) {
        if let Ok(session) = self.get_session().await
            && let Ok(mut buffer) = session.snmp_collected_nothing.lock()
        {
            buffer.push(record);
        }
    }

    /// Record neighbour records discarded for missing the identifier L2 resolution needs.
    pub async fn record_malformed_neighbours(&self, record: warnings::MalformedNeighbours) {
        if record.discarded == 0 {
            return;
        }
        if let Ok(session) = self.get_session().await
            && let Ok(mut buffer) = session.malformed_neighbours.lock()
        {
            buffer.push(record);
        }
    }

    /// Record that two full-ifTable integrations collected this host, if they did.
    ///
    /// Called where the host is submitted, after every integration has run, because `HostData`
    /// holds the detection and has no route to the session itself.
    pub async fn record_equal_reach_integrations(&self, ip: IpAddr, host_data: &HostData) {
        let Some((first, second)) = host_data.equal_reach_integrations() else {
            return;
        };
        if let Ok(session) = self.get_session().await
            && let Ok(mut buffer) = session.equal_reach_integrations.lock()
        {
            buffer.push(warnings::EqualReachIntegrations { ip, first, second });
        }
    }

    /// Record a device whose VLAN table was read and could not be saved.
    pub async fn record_vlan_recording_failure(&self, ip: std::net::IpAddr) {
        if let Ok(session) = self.get_session().await
            && let Ok(mut buffer) = session.vlan_recording_failures.lock()
        {
            buffer.push(warnings::VlanRecordingFailed { ip });
        }
    }

    /// Record an ifTable walk that fell short, and in which of the two ways.
    pub async fn record_interface_shortfall(&self, record: warnings::IncompleteInterfaceWalk) {
        if let Ok(session) = self.get_session().await
            && let Ok(mut buffer) = session.incomplete_interface_walks.lock()
        {
            buffer.push(record);
        }
    }

    /// Report scanning progress. Mode-aware: DaemonPoll POSTs to server,
    /// ServerPoll updates session atomics only.
    pub async fn report_progress(&self, percent: u8) -> Result<(), Error> {
        use crate::daemon::discovery::types::base::DiscoverySessionUpdate;
        use std::sync::atomic::Ordering;

        let session = self.get_session().await?;
        let start = session.progress_range_start.load(Ordering::Relaxed);
        let end = session.progress_range_end.load(Ordering::Relaxed);
        let percent = map_progress(percent, start, end);

        let last_report_time = &session.last_progress_report_time;
        let last_progress = &session.last_progress;

        let prev_percent = last_progress.load(Ordering::Relaxed);
        let progress_changed = percent > prev_percent || percent == 100;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last_time = last_report_time.load(Ordering::Relaxed);

        let heartbeat_interval_secs = 30;
        let heartbeat_due = now >= last_time + heartbeat_interval_secs;

        if !progress_changed && !heartbeat_due && percent < 100 {
            return Ok(());
        }

        if percent < 100 && !heartbeat_due && now < last_time + 10 {
            return Ok(());
        }

        if last_report_time
            .compare_exchange(last_time, now, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            return Ok(());
        }

        last_progress.store(percent, Ordering::Relaxed);

        // Mode-aware: only POST in DaemonPoll mode
        let mode = self.config_store.get_mode().await?;
        if mode == DaemonMode::DaemonPoll {
            let update = DiscoverySessionUpdate::scanning(percent);
            let mut payload = DiscoveryUpdatePayload::from_state_and_update(
                self.discovery_type.clone(),
                session.info.clone(),
                update,
            );

            let hosts = session.hosts_discovered.load(Ordering::Relaxed);
            if hosts > 0 {
                payload.hosts_discovered = Some(hosts);
            }
            let estimate = session.estimated_remaining_secs.load(Ordering::Relaxed);
            if estimate != u32::MAX {
                payload.estimated_remaining_secs = Some(estimate);
            }

            let path = format!("/api/v1/discovery/{}/update", session.info.session_id);
            if let Err(e) = self
                .api_client
                .post_no_data(&path, &payload, "Failed to report discovery update")
                .await
            {
                tracing::warn!(
                    session_id = %session.info.session_id,
                    error = %e,
                    "Failed to report discovery update"
                );
            }
        }

        Ok(())
    }

    /// Create a host with its children.
    /// DaemonPoll: POSTs to server. ServerPoll: buffers for server to poll.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_host(
        &self,
        host: Host,
        ip_addresses: Vec<IPAddress>,
        ports: Vec<Port>,
        services: Vec<Service>,
        interfaces: Vec<Interface>,
        subnets: Vec<Subnet>,
        interfaces_complete: bool,
        interface_data_complete: InterfaceDataComplete,
        cancel: &CancellationToken,
    ) -> Result<HostResponse, Error> {
        let mode = self.config_store.get_mode().await?;
        let pending_id = host.id;

        let request = DiscoveryHostRequest {
            host,
            ip_addresses,
            ports,
            services,
            interfaces,
            subnets,
            interfaces_complete,
            interface_data_complete,
            // This daemon is this build; it submits the current shape by construction.
            superseded_wire_shape: false,
        };

        self.entity_buffer.push_host(request.clone()).await;

        match mode {
            DaemonMode::DaemonPoll => {
                // Stagger request starts so a burst of near-simultaneous deep-scan
                // completions doesn't hammer the host-create endpoint (where they
                // serialize on the server's per-network `HostDedup` lock).
                let scheduled =
                    reserve_submit_slot(&self.host_submit_gate, MIN_HOST_SUBMIT_INTERVAL).await;
                tokio::time::sleep_until(scheduled).await;

                let api_client = &self.api_client;
                let response: HostResponse = (|| async {
                    api_client
                        .post("/api/v1/hosts/discovery", &request, "Failed to create host")
                        .await
                })
                .retry(
                    ExponentialBuilder::default()
                        .with_min_delay(Duration::from_millis(500))
                        .with_max_delay(Duration::from_secs(30))
                        .with_max_times(ENTITY_CREATION_MAX_RETRIES),
                )
                .when(|e| e.downcast_ref::<ApiErrorResponse>().is_none())
                .notify(|e, dur| tracing::warn!("Retrying host creation after {:?}: {}", dur, e))
                .await?;

                self.entity_buffer
                    .mark_host_created(pending_id, response.clone())
                    .await;

                Ok(response)
            }
            DaemonMode::ServerPoll => {
                let actual_host = self
                    .entity_buffer
                    .await_host(&pending_id, SERVER_POLL_CONFIRMATION_TIMEOUT, cancel)
                    .await
                    .ok_or_else(|| {
                        if cancel.is_cancelled() {
                            anyhow!("Discovery cancelled while waiting for host creation")
                        } else {
                            anyhow!("Timeout waiting for host creation confirmation from server")
                        }
                    })?;

                Ok(HostResponse::from_host_with_children(
                    actual_host,
                    request.ip_addresses,
                    request.ports,
                    request.services,
                    request.interfaces,
                ))
            }
        }
    }

    /// Create a subnet.
    /// DaemonPoll: POSTs to server. ServerPoll: buffers for server to poll.
    pub async fn create_subnet(
        &self,
        subnet: &Subnet,
        cancel: &CancellationToken,
    ) -> Result<Subnet, Error> {
        let mode = self.config_store.get_mode().await?;
        let pending_id = subnet.id;

        self.entity_buffer.push_subnet(subnet.clone()).await;

        match mode {
            DaemonMode::DaemonPoll => {
                let api_client = &self.api_client;
                let actual: Subnet = (|| async {
                    api_client
                        .post("/api/v1/subnets", subnet, "Failed to create subnet")
                        .await
                })
                .retry(
                    ExponentialBuilder::default()
                        .with_min_delay(Duration::from_millis(500))
                        .with_max_delay(Duration::from_secs(30))
                        .with_max_times(ENTITY_CREATION_MAX_RETRIES),
                )
                .notify(|e, dur| tracing::warn!("Retrying subnet creation after {:?}: {}", dur, e))
                .await?;

                self.entity_buffer
                    .mark_subnet_created(pending_id, actual.clone())
                    .await;

                Ok(actual)
            }
            DaemonMode::ServerPoll => self
                .entity_buffer
                .await_subnet(&pending_id, SERVER_POLL_CONFIRMATION_TIMEOUT, cancel)
                .await
                .ok_or_else(|| {
                    if cancel.is_cancelled() {
                        anyhow!("Discovery cancelled while waiting for subnet creation")
                    } else {
                        anyhow!("Timeout waiting for subnet creation confirmation from server")
                    }
                }),
        }
    }

    /// Upsert VLANs discovered via SNMP. Returns mapping of vlan_number → entity UUID.
    /// Posts to the server's VLAN discovery endpoint.
    pub async fn upsert_vlans(
        &self,
        vlans: &[crate::daemon::discovery::integration::snmp::types::VlanInfo],
        network_id: Uuid,
    ) -> Result<std::collections::HashMap<u16, Uuid>, Error> {
        use crate::server::vlans::handlers::{
            VlanDiscoveryItem, VlanDiscoveryRequest, VlanDiscoveryResponse,
        };

        let request = VlanDiscoveryRequest {
            network_id,
            vlans: vlans
                .iter()
                .map(|v| VlanDiscoveryItem {
                    vlan_number: v.vlan_id,
                    name: v.name.clone(),
                })
                .collect(),
        };

        let api_client = &self.api_client;
        // `api_client.post` (→ `execute`) already strips the `ApiResponse` envelope and
        // returns the inner payload, so the type here must be the bare
        // `VlanDiscoveryResponse`, not `ApiResponse<VlanDiscoveryResponse>`. Wrapping it
        // made the daemon try to parse a second envelope out of `{"vlans":[...]}`, failing
        // with `missing field 'success'` and losing all VLAN resolution (GH #649).
        let response: VlanDiscoveryResponse = (|| async {
            api_client
                .post(
                    "/api/v1/vlans/discovery",
                    &request,
                    "Failed to upsert VLANs",
                )
                .await
        })
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(Duration::from_millis(500))
                .with_max_delay(Duration::from_secs(10))
                .with_max_times(3),
        )
        .notify(|e, dur| tracing::warn!("Retrying VLAN upsert after {:?}: {}", dur, e))
        .await?;

        let mut mapping = std::collections::HashMap::new();
        for item in response.vlans {
            mapping.insert(item.vlan_number, item.id);
        }
        // Record canonical VLAN IDs so the terminal payload's
        // `ScannedEntityIds.vlan_ids` includes them. Per-entity FK-update
        // subscribers consume this server-side.
        self.entity_buffer
            .push_scanned_vlans(mapping.values().copied())
            .await;
        Ok(mapping)
    }

    /// Run service matching against all registered service definitions.
    /// Returns matched services and ports. Pure logic — no side effects.
    pub fn match_services(
        &self,
        host: &Host,
        baseline_params: &ServiceMatchBaselineParams,
        gateway_ips: &[IpAddr],
        daemon_id: &Uuid,
        network_id: &Uuid,
    ) -> Result<(Vec<Service>, Vec<Port>), Error> {
        use crate::server::services::definitions::{
            docker_container::DockerContainer, open_ports::OpenPorts,
            podman_container::PodmanContainer,
        };

        let ServiceMatchBaselineParams { all_ports, .. } = baseline_params;

        let mut services = Vec::new();
        let mut host_ports = Vec::new();
        let mut unbound_ports = all_ports.to_vec();
        let mut container_matched = false;

        let mut sorted_service_definitions: Vec<Box<dyn ServiceDefinition>> =
            ServiceDefinitionRegistry::all_service_definitions()
                .into_iter()
                .collect();

        sorted_service_definitions.sort_by_key(|s| {
            if !ServiceDefinitionExt::is_generic(s) {
                0
            } else if s.id() == OpenPorts.id() {
                3
            } else if s.id() == DockerContainer.id()
                || s.id() == PodmanContainer.id()
                || s.id() == Gateway.id()
            {
                2
            } else {
                1
            }
        });

        for service_definition in sorted_service_definitions {
            let service_params = ServiceMatchServiceParams {
                service_definition,
                matched_services: &services,
                unbound_ports: &unbound_ports,
            };

            let params: DiscoverySessionServiceMatchParams<'_> =
                DiscoverySessionServiceMatchParams {
                    service_params,
                    baseline_params,
                    daemon_id,
                    discovery_type: &self.discovery_type,
                    network_id,
                    gateway_ips,
                    host_id: &host.id,
                };

            if let Some((service, mut ports, _endpoint)) = Service::from_discovery(params)
                && !container_matched
            {
                // A container (Docker or Podman) was matched as a service this round.
                if service
                    .base
                    .virtualization_metadata
                    .as_ref()
                    .and_then(|v| v.container_id())
                    .is_some()
                {
                    container_matched = true
                }

                let bound_port_types: Vec<PortType> =
                    ports.iter().map(|p| p.base.port_type).collect();

                host_ports.append(&mut ports);
                unbound_ports.retain(|p| !bound_port_types.contains(p));
                services.push(service);
            }
        }

        services.sort_by_key(|a| {
            -(match &a.base.source {
                EntitySource::DiscoveryWithMatch { details, .. } => {
                    (details.confidence as i32)
                        + if a.base.service_definition.has_logo() {
                            1
                        } else {
                            0
                        }
                }
                _ => MatchConfidence::NotApplicable as i32,
            })
        });

        host_ports.extend(unbound_ports.into_iter().map(Port::new_hostless));

        Ok((services, host_ports))
    }

    /// Build a HostData from scan results: creates host entity, runs service matching, names host.
    pub async fn build_host_from_scan(
        &self,
        params: ServiceMatchBaselineParams<'_>,
        hostname: Option<HostHostnameAttributed>,
        host_naming_fallback: HostNamingFallback,
    ) -> Result<Option<HostData>, Error> {
        let ServiceMatchBaselineParams { ip_address, .. } = params;

        let daemon_id = self.daemon_id().await?;
        let network_id = self.network_id().await?;
        let session = self.get_session().await?;
        let gateway_ips = session.gateway_ips.clone();

        let mut host = Host::new(HostBase {
            name: HostName::unnamed(),
            hostname,
            tags: Vec::new(),
            network_id,
            description: None,
            source: EntitySource::Discovery,
            virtualization_metadata: None,
            virtualization_service_id: None,
            hidden: false,
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            firmware_revision: None,
            software_revision: None,
            credential_assignments: vec![],
        });

        let ip_addresses = vec![ip_address.clone()];

        let (services, ports) =
            self.match_services(&host, &params, &gateway_ips, &daemon_id, &network_id)?;

        // Determine host name
        let best_service_name = services
            .iter()
            .find(|s| !ServiceDefinitionExt::is_generic(&s.base.service_definition))
            .map(|s| s.base.service_definition.name().to_string());

        // Only names go in `name`. The hostname and the address are identifiers with their own
        // columns, and the display ladder shows them without a copy (see the placement rule in
        // `hosts::impl::name`). The one name a scan can derive is a guess from the best
        // non-generic service, stored only when the user chose that fallback. It ranks below the
        // identifiers, and an integration that knows a human-assigned name replaces it later,
        // during `execute()`.
        if let (HostNamingFallback::BestService, Some(service)) =
            (host_naming_fallback, best_service_name)
        {
            host.base.apply_name(HostName::from_service(service));
        }

        // A DNS-SD instance name is what the owner typed during setup — "Living Room TV" rather
        // than "chromecast-a1b2c3" — so it outranks everything the scan can derive and applies
        // after them. `apply_name` compares rungs, so this is a no-op against a name a person
        // typed into Scanopy or one an integration read back out of a controller.
        if let Some(dns_sd) = params.dns_sd
            && let Some(instance_name) = &dns_sd.instance_name
        {
            host.base
                .apply_name(HostName::from_dns_sd(instance_name.clone()));
        }

        tracing::info!(
            ip = %ip_address.base.ip_address,
            host_name = %host.base.name,
            service_count = %services.len(),
            port_count = %ports.len(),
            "Processed host",
        );

        Ok(Some(HostData::new(
            host,
            services,
            ports,
            ip_addresses,
            vec![],
            vec![],
        )))
    }
}

fn map_progress(raw: u8, start: u8, end: u8) -> u8 {
    if start == 0 && end == 100 {
        return raw;
    }
    start + (raw as f64 * (end - start) as f64 / 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::shared::attribution;
    use tokio::sync::Mutex;
    use tokio::time::Instant;

    fn empty_host_data() -> HostData {
        use crate::server::hosts::r#impl::base::{Host, HostBase};
        HostData::new(
            Host::new(HostBase::default()),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        )
    }

    fn port(name: &str, if_index: i32) -> Interface {
        use crate::server::interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, InterfaceBase};
        Interface::new(InterfaceBase {
            host_id: Uuid::new_v4(),
            network_id: Uuid::new_v4(),
            if_index: Some(if_index),
            if_descr: Some(name.to_string()),
            if_name: Some(name.to_string()),
            if_alias: None,
            if_type: Some(6),
            speed_bps: None,
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(IfOperStatus::Up),
            mac_address: None,
            ip_address_id: None,
            ip_configured: false,
            neighbor_candidates: Vec::new(),
            fdb_macs: None,
            native_vlan_id: None,
            vlan_ids: None,
        })
    }

    fn source(
        credential: CredentialQueryPayloadDiscriminants,
        scope: InterfaceViewScope,
    ) -> InterfaceSource {
        InterfaceSource { credential, scope }
    }

    const SNMP: CredentialQueryPayloadDiscriminants = CredentialQueryPayloadDiscriminants::Snmp;
    const GNMI: CredentialQueryPayloadDiscriminants = CredentialQueryPayloadDiscriminants::Gnmi;
    const UNIFI: CredentialQueryPayloadDiscriminants =
        CredentialQueryPayloadDiscriminants::UnifiController;

    /// The positive control for the attribute path: what an integration read reaches the host.
    ///
    /// Worth pinning on its own because the failure it guards against is different in kind from
    /// the one below — a scan that stops delivering a model at all is not the same bug as one
    /// that delivers the wrong source's model.
    #[test]
    fn a_probes_model_reaches_the_host() {
        let mut data = empty_host_data();
        data.with_model(
            "WS-C2960X-48FPD-L".to_string(),
            AttributeSource::Probe(ClientProbe::Snmp),
        );

        assert_eq!(
            attribution::text_of(&data.host.base.model).as_deref(),
            Some("WS-C2960X-48FPD-L")
        );
    }

    /// The behaviour this item was taken for, at the layer it used to be decided by accident.
    ///
    /// This test was written first against the old `is_none()` gate, where it asserted the
    /// opposite: whichever integration ran first owned the value, so precedence was the order
    /// SNMP, the controllers and the app probes happen to run in. A controller's "Cisco Switch"
    /// beat SNMP's `WS-C2960X-48FPD-L` purely because controllers used to run first. Now the
    /// better reading wins whenever it arrives.
    #[test]
    fn a_better_reading_displaces_the_one_that_arrived_first() {
        let mut data = empty_host_data();
        data.with_model(
            "Cisco Switch".to_string(),
            AttributeSource::Probe(ClientProbe::UnifiController),
        );
        data.with_model(
            "WS-C2960X-48FPD-L".to_string(),
            AttributeSource::Probe(ClientProbe::Snmp),
        );

        assert_eq!(
            attribution::text_of(&data.host.base.model).as_deref(),
            Some("WS-C2960X-48FPD-L")
        );
    }

    /// The converse, and what makes the first test's ordering a decision rather than a race: a
    /// weaker source arriving later cannot undo it.
    #[test]
    fn a_weaker_reading_arriving_later_is_ignored() {
        let mut data = empty_host_data();
        data.with_model(
            "WS-C2960X-48FPD-L".to_string(),
            AttributeSource::Probe(ClientProbe::Snmp),
        );
        data.with_model(
            "Cisco Switch".to_string(),
            AttributeSource::Probe(ClientProbe::UnifiController),
        );

        assert_eq!(
            attribution::text_of(&data.host.base.model).as_deref(),
            Some("WS-C2960X-48FPD-L")
        );
    }

    /// SNMP persists a bare interface set at its checkpoint and swaps in the enriched one once the
    /// neighbour and FDB queries return. That is one contributor revising itself, so the second
    /// offer replaces the first rather than being merged into a doubled list.
    #[test]
    fn a_contributor_revising_its_own_set_replaces_it() {
        let mut host_data = empty_host_data();
        let full = source(SNMP, InterfaceViewScope::FullIfTable);

        host_data.contribute_interfaces(
            full,
            vec![port("Gi0/1", 1), port("Gi0/2", 2)],
            true,
            InterfaceDataComplete::none(),
        );
        host_data.contribute_interfaces(
            full,
            vec![port("Gi0/1", 1), port("Gi0/2", 2), port("Vlan1", 100)],
            true,
            InterfaceDataComplete::default(),
        );

        assert_eq!(host_data.interfaces.len(), 3);
        assert!(host_data.interfaces_complete);
        assert!(host_data.interface_data_complete.lldp);
        assert_eq!(
            host_data.equal_reach_integrations(),
            None,
            "one contributor revising itself is not a second integration"
        );
    }

    /// A controller's ports being a subset of the ifTable is the ordinary case, and it must cost
    /// nothing: the ifTable rows stand, no row is duplicated, and both completeness verdicts come
    /// out exactly as the ifTable contributor stated them — including its authority to prune.
    #[test]
    fn a_subset_of_physical_ports_merges_without_weakening_the_if_table() {
        let mut host_data = empty_host_data();

        host_data.contribute_interfaces(
            source(SNMP, InterfaceViewScope::FullIfTable),
            vec![port("Gi0/1", 1), port("Gi0/2", 2), port("Vlan1", 100)],
            true,
            InterfaceDataComplete::default(),
        );
        host_data.contribute_interfaces(
            source(UNIFI, InterfaceViewScope::PhysicalPortsOnly),
            vec![port("Gi0/1", 1), port("Gi0/2", 2)],
            false,
            InterfaceDataComplete {
                lldp: true,
                cdp: false,
                fdb: true,
                vlan_membership: false,
            },
        );

        assert_eq!(host_data.interfaces.len(), 3);
        assert!(host_data.interfaces_complete);
        assert!(host_data.interface_data_complete.cdp);
        assert_eq!(
            host_data.equal_reach_integrations(),
            None,
            "a controller beside an ifTable reader is the ordinary case, not one to report"
        );
    }

    /// SNMP and gNMI on one switch: both read the whole ifTable, so a port both describe keeps
    /// the first contributor's row and neighbours. The host carries which two, in that order, so
    /// the runner can say so in the scan record.
    #[test]
    fn two_full_if_table_contributors_are_recorded_in_the_order_they_answered() {
        let mut host_data = empty_host_data();

        host_data.contribute_interfaces(
            source(GNMI, InterfaceViewScope::FullIfTable),
            vec![port("eth1", 1), port("eth2", 2)],
            true,
            InterfaceDataComplete::default(),
        );
        host_data.contribute_interfaces(
            source(SNMP, InterfaceViewScope::FullIfTable),
            vec![port("eth1", 1), port("eth3", 3)],
            true,
            InterfaceDataComplete::default(),
        );

        assert_eq!(host_data.interfaces.len(), 3);
        assert_eq!(host_data.equal_reach_integrations(), Some((GNMI, SNMP)));
    }

    /// A port only the narrower view has is added — and its presence is itself the proof that the
    /// ifTable was not the whole host, so the set stops being authority to delete.
    #[test]
    fn a_port_only_the_narrower_view_has_is_added_and_costs_the_prune() {
        let mut host_data = empty_host_data();

        host_data.contribute_interfaces(
            source(SNMP, InterfaceViewScope::FullIfTable),
            vec![port("Gi0/1", 1)],
            true,
            InterfaceDataComplete::default(),
        );
        host_data.contribute_interfaces(
            source(UNIFI, InterfaceViewScope::PhysicalPortsOnly),
            vec![port("Gi0/1", 1), port("Gi0/9", 9)],
            false,
            InterfaceDataComplete {
                lldp: true,
                cdp: false,
                fdb: true,
                vlan_membership: false,
            },
        );

        assert_eq!(host_data.interfaces.len(), 2);
        assert!(!host_data.interfaces_complete);
        // The added row came from a contributor that never reads CDP, so nothing here may clear a
        // CDP column the SNMP walk populated.
        assert!(!host_data.interface_data_complete.cdp);
    }

    /// The case this replaced a hand-written guard for: two contributors of equal reach where one
    /// is thin. A device can answer SNMP with an almost-empty ifTable and gNMI with the real one,
    /// and network-wide credentials make the order arbitrary — so neither order may lose rows.
    #[test]
    fn a_thin_view_never_erases_a_rich_one_in_either_order() {
        let rich = vec![port("eth1", 1), port("eth2", 2), port("eth3", 3)];
        let thin = vec![port("eth1", 1)];

        let mut rich_first = empty_host_data();
        rich_first.contribute_interfaces(
            source(SNMP, InterfaceViewScope::FullIfTable),
            rich.clone(),
            true,
            InterfaceDataComplete::default(),
        );
        rich_first.contribute_interfaces(
            source(UNIFI, InterfaceViewScope::FullIfTable),
            thin.clone(),
            true,
            InterfaceDataComplete::default(),
        );

        let mut thin_first = empty_host_data();
        thin_first.contribute_interfaces(
            source(UNIFI, InterfaceViewScope::FullIfTable),
            thin,
            true,
            InterfaceDataComplete::default(),
        );
        thin_first.contribute_interfaces(
            source(SNMP, InterfaceViewScope::FullIfTable),
            rich,
            true,
            InterfaceDataComplete::default(),
        );

        assert_eq!(rich_first.interfaces.len(), 3);
        assert_eq!(thin_first.interfaces.len(), 3);
    }

    /// GH #650. `container::execute` runs several times against one host — once per Docker and
    /// Podman socket/proxy credential type, and again for the network sweep after the
    /// daemon-host phase — and each run offers the same bridge networks. Offering them was a
    /// bare push, so one scan submitted the same CIDR five or six times.
    #[test]
    fn the_same_bridge_network_is_only_offered_once() {
        use crate::server::hosts::r#impl::base::{Host, HostBase};
        use crate::server::shared::storage::traits::Storable;
        use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
        use crate::server::subnets::r#impl::types::SubnetType;

        let mut host_data = HostData::new(
            Host::new(HostBase::default()),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        );

        let base = SubnetBase {
            subnet_type: SubnetType::DockerBridge,
            ..Default::default()
        };
        // Distinct rows, same CIDR and network — exactly what a second execute() produces, since
        // every run mints fresh UUIDs for the subnets it reports.
        host_data.add_subnet(Subnet::new(base.clone()));
        host_data.add_subnet(Subnet::new(base));

        assert_eq!(host_data.subnets.len(), 1);
    }

    /// A burst of back-to-back reservations must be handed distinct start slots
    /// spaced at least `interval` apart — this is the property that staggers the
    /// host-create requests instead of firing them all at once.
    #[tokio::test]
    async fn reserve_submit_slot_spaces_a_burst_by_interval() {
        let interval = Duration::from_millis(25);
        let gate = Mutex::new(Instant::now());

        // Reserve in a tight loop (the real clock barely moves), the worst case
        // that would otherwise produce one instantaneous burst of requests.
        let mut slots = Vec::new();
        for _ in 0..10 {
            slots.push(reserve_submit_slot(&gate, interval).await);
        }

        // Consecutive reserved slots are at least `interval` apart, and the
        // whole burst is spread across ~(N-1)*interval rather than fired at once.
        for pair in slots.windows(2) {
            assert!(
                pair[1] - pair[0] >= interval,
                "consecutive slots must be spaced >= interval apart",
            );
        }
        assert!(*slots.last().unwrap() - slots[0] >= interval * (slots.len() as u32 - 1));
    }

    /// A caller that arrives when the gate has already drained (no recent
    /// traffic) starts immediately — the stagger never accrues idle delay.
    #[tokio::test]
    async fn reserve_submit_slot_no_delay_when_gate_idle() {
        // Seed the gate in the past to simulate "last submission was a while ago".
        let gate = Mutex::new(Instant::now() - Duration::from_secs(1));

        let before = Instant::now();
        let slot = reserve_submit_slot(&gate, Duration::from_millis(25)).await;

        // The reserved slot is "now", not the stale past value — no artificial
        // wait is imposed on the first request after an idle period.
        assert!(slot >= before);
    }
}

/// How many coded warnings one run's scan record holds.
///
/// One warning per occurrence is what makes them countable and gives each address its own
/// diagnostic, but it also turns an O(codes) payload into an O(codes x devices) one — an
/// LLDP-heavy network can produce thousands, all of which land in a single JSONB column. Well
/// above the ten the UI lists per code, so nothing an operator reads is affected.
const MAX_WARNINGS: usize = 500;

/// Cap the warning list, saying how much was left out.
///
/// The alternative is a payload that grows without bound or a list that simply stops, and a list
/// that stops reads as though that was all of them — the rule the whole warnings module is built
/// on. `WarningsTruncated` is emitted in place of the tail so the count survives.
fn truncate_warnings(warnings: &mut Vec<DiscoveryWarning>) {
    if warnings.len() <= MAX_WARNINGS {
        return;
    }
    let elided = warnings.len() - MAX_WARNINGS;
    warnings.truncate(MAX_WARNINGS);
    warnings.push(DiscoveryWarning::WarningsTruncated {
        elided: u32::try_from(elided).unwrap_or(u32::MAX),
    });
}
