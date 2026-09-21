//! Generic integration dispatch — probe and execute integrations for any host.
//!
//! Used by both network scanning (deep_scan_host) and localhost phase.
//! Given credential mappings + a target IP, probes each integration, then
//! executes successful ones against HostData.

use std::any::Any;
use std::collections::HashMap;
use std::net::IpAddr;

use anyhow::Error;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::daemon::discovery::credentials::{ApplicableCredential, resolve_credentials_for_ip};
use crate::daemon::discovery::service::ops::{DiscoveryOps, HostData};
use crate::daemon::discovery::service::warnings::{
    AttemptOutcome, CredentialIssue, CredentialIssueReason, issue_for_attempt,
};
use crate::daemon::utils::base::PlatformDaemonUtils;
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::discovery::r#impl::types::HostNamingFallback;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::subnets::r#impl::base::Subnet;

use super::{
    DiscoveryIntegration, IntegrationContext, IntegrationRegistry, InterfaceSource, ProbeContext,
    ProbeSuccess, execute_with_progress_reporting,
};

/// Run one credential against one integration and say what happened.
///
/// The single path from a *probe failure* to an operator-visible outcome. That is narrower than it
/// first appears, and the narrowness is what made three paths go quiet: this can only speak for
/// attempts that produce an error. A branch that skips, a collection that half-succeeds, and a
/// caller that drops the result all bypass it without touching it. [`Disposition`] is what covers
/// those.
///
/// Returns `Ok` on success, or the issue worth reporting — `None` inside the `Err` for the
/// failures that are noise rather than news.
async fn attempt_credential(
    integration: &dyn DiscoveryIntegration,
    ctx: &ProbeContext<'_>,
    discriminant: CredentialQueryPayloadDiscriminants,
    user_assigned: bool,
    credential_id: Option<Uuid>,
) -> Result<ProbeSuccess, Option<CredentialIssue>> {
    let failure = match integration.probe(ctx).await {
        Ok(success) => return Ok(success),
        Err(failure) => failure,
    };
    let outcome = failure.outcome();

    // Whether this is a finding is `issue_for_attempt`'s call, not ours — a network default
    // failing is routine (it is broadcast at every address in the subnet) and a cancelled attempt
    // is not news at all. The log level follows the same verdict, so an operator reading the log
    // and an operator reading the scan warnings see the same set of problems.
    let issue = issue_for_attempt(
        discriminant,
        ctx.ip,
        outcome,
        failure.message().to_string(),
        user_assigned,
        credential_id,
    );

    if issue.is_none() {
        tracing::debug!(
            ip = %ctx.ip,
            integration = ?discriminant,
            ?outcome,
            error = failure.message(),
            "Integration probe failed, trying next credential"
        );
    } else {
        tracing::warn!(
            ip = %ctx.ip,
            integration = ?discriminant,
            ?outcome,
            error = failure.message(),
            "Configured credential did not work"
        );
    }
    Err(issue)
}

/// What became of one credential mapping at one address.
///
/// # Why this exists
///
/// The reporting mechanism this replaces guarded the *error* channel: a failure could not be
/// built without a classification, and only `DiscoveryOps` could deliver one. That is a real
/// guarantee and it covered none of the ways a credential actually went quiet — a `continue` with
/// nothing to classify, a partial success returning `Ok`, an issue built and never read. Silence
/// was the default, and a branch got it by saying nothing.
///
/// So the default is inverted. An entry is opened the moment a mapping resolves to an
/// integration, before any branch can skip it, and every path has to write its outcome. A future
/// branch that says nothing leaves [`Self::Unresolved`], which is loud rather than absent — see
/// [`resolve_ledger`]. That does not make the mistake impossible; Rust will not require a field be
/// set before a `continue`. It changes which way the mistake falls: we find out instead of a
/// customer.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Disposition {
    /// Nothing has claimed this. Never correct at the end of a dispatch.
    Unresolved,
    /// The probe succeeded. What happens next is `execute_integrations`' to account for.
    Probed,
    /// Deliberately not reported. The reason is carried so "why was this silent?" is answerable
    /// from the record rather than by re-deriving it from the control flow.
    Suppressed(&'static str),
    Failed(AttemptOutcome, String),
}

/// One credential mapping's slot in the ledger.
struct DispositionEntry {
    discriminant: CredentialQueryPayloadDiscriminants,
    /// Whether the user pinned this to a host, which decides if a failure is a finding.
    user_assigned: bool,
    /// The stored credential this mapping is for, so a ledger-reported failure can name it. The
    /// mapping is built per credential, so one id covers the whole slot.
    credential_id: Option<Uuid>,
    disposition: Disposition,
}

/// Turn the ledger into the issues worth reporting, and complain about anything unaccounted for.
///
/// The `Unresolved` check is the whole point of the type: it fires in tests and in the SNMP
/// simulator long before a customer scan, which is where the three gaps this replaced should have
/// been caught and were not.
fn resolve_ledger(ledger: Vec<DispositionEntry>, ip: IpAddr) -> Vec<CredentialIssue> {
    let mut issues = Vec::new();
    for entry in ledger {
        match entry.disposition {
            Disposition::Unresolved => {
                debug_assert!(
                    false,
                    "credential dispatch left {:?} for {} unaccounted for — a branch was added \
                     without recording what it did",
                    entry.discriminant, ip
                );
                tracing::error!(
                    ip = %ip,
                    integration = ?entry.discriminant,
                    "Credential dispatch finished without recording an outcome; this is a bug"
                );
            }
            Disposition::Probed => {}
            Disposition::Suppressed(reason) => tracing::debug!(
                ip = %ip,
                integration = ?entry.discriminant,
                reason,
                "Credential produced nothing, deliberately not reported"
            ),
            Disposition::Failed(outcome, message) => {
                issues.extend(issue_for_attempt(
                    entry.discriminant,
                    ip,
                    outcome,
                    message,
                    entry.user_assigned,
                    entry.credential_id,
                ));
            }
        }
    }
    issues
}

/// Results from probing all integrations for a single host IP.
pub struct IntegrationProbeResults {
    pub client_responses: HashMap<ClientProbe, Vec<PortType>>,
    pub probe_handles: HashMap<CredentialQueryPayloadDiscriminants, Box<dyn Any + Send + Sync>>,
    /// The credential that successfully probed per integration — `cred_id` is
    /// `Some` for user-configured (host-assigned) credentials and `None` for
    /// network-default fallbacks. Execute reads from this to run against the
    /// credential that actually worked; only `Some` entries participate in
    /// credential_assignments (defaults are network-wide, not host-scoped).
    pub working_credential_ids:
        HashMap<CredentialQueryPayloadDiscriminants, (Option<Uuid>, CredentialQueryPayload)>,
    /// Ports discovered by integration probes (added to open_ports).
    pub additional_ports: Vec<PortType>,
    /// IP-targeted credentials that produced nothing at this address, for the caller to
    /// surface. Only credentials the user deliberately assigned to a host appear here — a
    /// network default failing is routine, since it is tried at every address in the subnet.
    pub credential_issues: Vec<CredentialIssue>,
}

/// Probe all integrations for a host IP against credential mappings.
///
/// For each credential mapping, resolves the credential for this IP,
/// checks probe gate ports, then tries probe until one succeeds.
/// Returns aggregated probe results for subsequent service matching and execution.
/// `skip_gate` bypasses `probe_gate_ports` — used for the daemon's own host
/// (localhost) phase, which does no port scan and lets integrations self-probe.
/// The network-scan phase passes `false` so the gate keeps the broad scan cheap.
pub async fn probe_integrations(
    ip: IpAddr,
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
    open_ports: &[PortType],
    skip_gate: bool,
    cancel: &CancellationToken,
    utils: &PlatformDaemonUtils,
    accept_invalid_certs: bool,
) -> Result<IntegrationProbeResults, Error> {
    let mut results = IntegrationProbeResults {
        client_responses: HashMap::new(),
        probe_handles: HashMap::new(),
        working_credential_ids: HashMap::new(),
        additional_ports: Vec::new(),
        credential_issues: Vec::new(),
    };

    // Combine caller's open ports with probe-discovered ports for gate checks
    let mut all_open_ports: Vec<PortType> = open_ports.to_vec();

    // First pass (synchronous, cheap): resolve each mapping to a probe task, applying
    // the discriminant / integration / credentials / gate checks. Gate checks use the
    // port-scan `open_ports`; probe-discovered ports don't feed later gates (negligible
    // in practice — probes surface their own service's ports — and it lets the probes
    // run concurrently below).
    struct ProbeTask<'a> {
        /// Index into `ledger`. Carried through the concurrent probe so each task's outcome lands
        /// back in the slot opened for it.
        entry: usize,
        discriminant: CredentialQueryPayloadDiscriminants,
        integration: Box<dyn DiscoveryIntegration>,
        credentials: Vec<ApplicableCredential<'a>>,
    }
    let mut tasks: Vec<ProbeTask> = Vec::new();
    let mut ledger: Vec<DispositionEntry> = Vec::new();
    for mapping in credential_mappings {
        let Some(discriminant) = mapping
            .default_credential
            .as_ref()
            .map(|c| c.into())
            .or_else(|| mapping.ip_overrides.first().map(|o| (&o.credential).into()))
        else {
            // An empty mapping carries no credential, so there is nothing to account for. This
            // is the one skip above the ledger, deliberately: opening a slot would mean
            // reporting on a credential that does not exist.
            continue;
        };

        // Opened before any branch below can skip, which is what makes the rest of this loop
        // unable to go quiet by omission.
        let entry = ledger.len();
        // Same rule `resolve_credentials_for_ip` applies: an override counts only at the address
        // it names, and a nil id means a broadcast default rather than something a user pinned
        // here. Without the address filter a mapping targeting some *other* host would make this
        // one look user-assigned and start reporting network defaults at every address in a /24.
        let user_assigned = mapping
            .ip_overrides
            .iter()
            .any(|o| o.ip == ip && o.credential_id != Uuid::nil());
        // The credential this mapping stands for at this address: the override pinned here if
        // there is one, otherwise the broadcast default. A mapping is built per credential, so
        // these are the same row in every case where both are present.
        let credential_id = mapping
            .ip_overrides
            .iter()
            .find(|o| o.ip == ip && o.credential_id != Uuid::nil())
            .map(|o| o.credential_id)
            .or(mapping.default_credential_id);
        ledger.push(DispositionEntry {
            discriminant,
            user_assigned,
            credential_id,
            disposition: Disposition::Unresolved,
        });

        let Some(integration) = IntegrationRegistry::get(discriminant) else {
            tracing::warn!(integration = ?discriminant, "Skipping unrecognized credential type from newer server");
            // A credential type this daemon cannot run. The server blocks configuring one, so
            // reaching here means a mixed-version fleet — reported, because the operator's
            // credential silently does nothing on this daemon until it is upgraded.
            ledger[entry].disposition = Disposition::Failed(
                AttemptOutcome::Malformed,
                "this daemon is too old to use this credential type; upgrade it".to_string(),
            );
            continue;
        };
        let credentials = resolve_credentials_for_ip(mapping, ip);
        if credentials.is_empty() {
            ledger[entry].disposition =
                Disposition::Suppressed("no credential in this mapping applies to this address");
            continue;
        }
        if !skip_gate {
            let gate_ports = integration.probe_gate_ports(credentials[0].credential);
            if !gate_ports.is_empty() && !gate_ports.iter().all(|gp| all_open_ports.contains(gp)) {
                // Silent until now, and the single likeliest reason a working credential
                // appears to do nothing: the port on the credential does not match the port
                // the service actually listens on, so no connection is ever attempted.
                //
                // Reported only for a credential the user pinned here: a broadcast default whose
                // port is closed is routine on a sweep, and it now carries an id too, so presence
                // of an id no longer distinguishes the two.
                if let Some(applicable) = credentials.iter().find(|c| c.user_assigned) {
                    results.credential_issues.push(CredentialIssue {
                        integration: discriminant,
                        ip,
                        reason: CredentialIssueReason::GateClosed {
                            ports: gate_ports.clone(),
                        },
                        credential_id: applicable.credential_id,
                    });
                    ledger[entry].disposition =
                        Disposition::Suppressed("gate closed; reported as GateClosed");
                } else {
                    ledger[entry].disposition = Disposition::Suppressed(
                        "gate closed on a network default, which is routine on a sweep",
                    );
                }
                continue;
            }
        }
        tasks.push(ProbeTask {
            entry,
            discriminant,
            integration,
            credentials,
        });
    }

    if cancel.is_cancelled() {
        return Err(Error::msg("Discovery was cancelled"));
    }

    // Probe all mappings concurrently. Each task tries its credentials in order and
    // returns the first success (or None). This collapses the previously-serial
    // per-credential probe latency (e.g. v1+v2c+v3 SNMP + the public default, each with
    // multi-second UDP timeouts on non-responders) into roughly one probe's wall-clock.
    let outcomes = futures::future::join_all(tasks.into_iter().map(|task| {
        let ProbeTask {
            entry,
            discriminant,
            integration,
            credentials,
        } = task;
        async move {
            // Failures of IP-targeted credentials, reported only if nothing here wins.
            // Reporting them eagerly would flag the benign try-many case: SnmpV1, V2c and V3
            // assigned to one host are all attempted and the first success wins, so the ones
            // tried before it "failed" without anything being wrong.
            let mut targeted_failures: Vec<CredentialIssue> = Vec::new();
            for applicable in &credentials {
                if cancel.is_cancelled() {
                    return (
                        entry,
                        None,
                        Vec::new(),
                        Disposition::Suppressed("the scan was cancelled"),
                    );
                }
                let cred_id = applicable.credential_id;
                match attempt_credential(
                    integration.as_ref(),
                    &ProbeContext {
                        ip,
                        credential: applicable.credential,
                        credential_id: cred_id,
                        cancel,
                        utils,
                        accept_invalid_certs,
                    },
                    discriminant,
                    applicable.user_assigned,
                    cred_id,
                )
                .await
                {
                    Ok(success) => {
                        return (
                            entry,
                            Some((
                                discriminant,
                                cred_id,
                                applicable.credential.clone(),
                                success,
                            )),
                            Vec::new(),
                            Disposition::Probed,
                        );
                    }
                    Err(issue) => targeted_failures.extend(issue),
                }
            }
            // Nothing in this mapping worked. `attempt_credential` already applied the reporting
            // policy, so the ledger records that this was accounted for rather than repeating it.
            let disposition = if targeted_failures.is_empty() {
                Disposition::Suppressed("every credential here is a network default; routine")
            } else {
                Disposition::Suppressed("reported by attempt_credential")
            };
            (entry, None, targeted_failures, disposition)
        }
    }))
    .await;

    if cancel.is_cancelled() {
        return Err(Error::msg("Discovery was cancelled"));
    }

    // Merge in original mapping order so winner-selection is unchanged from the serial
    // version: for a given integration the last successful mapping's credential wins
    // (overwrite), and probe-discovered ports are unioned.
    let mut winners = Vec::new();
    for (entry, winner, failures, disposition) in outcomes {
        ledger[entry].disposition = disposition;
        results.credential_issues.extend(failures);
        winners.push(winner);
    }

    // Every mapping now has to have said what became of it. Anything still `Unresolved` is a
    // branch added without recording an outcome, which is the class of bug this replaced.
    results.credential_issues.extend(resolve_ledger(ledger, ip));

    for (discriminant, cred_id, credential, success) in winners.into_iter().flatten() {
        let ProbeSuccess {
            client_probe,
            ports,
            handle,
        } = success;
        tracing::info!(ip = %ip, integration = ?discriminant, ports = ?ports, "Integration probe succeeded");
        for port in &ports {
            if !all_open_ports.contains(port) {
                all_open_ports.push(*port);
                results.additional_ports.push(*port);
            }
        }
        results.client_responses.insert(client_probe, ports);
        if let Some(handle) = handle {
            results.probe_handles.insert(discriminant, handle);
        }
        // `cred_id` is Some for user-configured creds and None for network-default
        // fallbacks; execute needs the payload either way, so we insert unconditionally.
        results
            .working_credential_ids
            .insert(discriminant, (cred_id, credential));
    }

    Ok(results)
}

/// Parameters for integration execution dispatch.
pub struct ExecuteParams<'a> {
    pub ip: IpAddr,
    pub cancel: &'a CancellationToken,
    pub ops: &'a DiscoveryOps,
    pub utils: &'a PlatformDaemonUtils,
    pub open_ports: &'a [PortType],
    pub endpoint_responses: &'a [crate::server::services::r#impl::endpoints::EndpointResponse],
    pub host_id: Uuid,
    pub host_naming_fallback: HostNamingFallback,
    pub known_subnets: &'a [Subnet],
    pub scanning_subnet: Option<&'a Subnet>,
    pub ip_address_id: Option<Uuid>,
}

/// Execute integrations whose probe succeeded and whose associated service was matched.
///
/// Derive the integration discriminant a mapping resolves to (from its default
/// credential, else its first ip-override).
fn mapping_discriminant(
    mapping: &CredentialMapping<CredentialQueryPayload>,
) -> Option<CredentialQueryPayloadDiscriminants> {
    mapping
        .default_credential
        .as_ref()
        .map(|c| c.into())
        .or_else(|| mapping.ip_overrides.first().map(|o| (&o.credential).into()))
}

/// Collapse credential mappings to the distinct `(integration, winning credential id)`
/// collections `execute_integrations` should run, preserving first-seen order and
/// dropping mappings with no probe winner. Deduping by the winning credential (not the
/// mapping) means N mappings that share one integration + winner run once, while a
/// distinct winning credential still runs.
fn dedup_execution_keys(
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
    working_credential_ids: &HashMap<
        CredentialQueryPayloadDiscriminants,
        (Option<Uuid>, CredentialQueryPayload),
    >,
) -> Vec<(CredentialQueryPayloadDiscriminants, Option<Uuid>)> {
    let mut seen = std::collections::HashSet::new();
    let mut keys = Vec::new();
    for mapping in credential_mappings {
        let Some(discriminant) = mapping_discriminant(mapping) else {
            continue;
        };
        let Some((cred_id, _)) = working_credential_ids.get(&discriminant) else {
            continue;
        };
        let key = (discriminant, *cred_id);
        if seen.insert(key) {
            keys.push(key);
        }
    }
    keys
}

/// Enriches host_data with integration-discovered services, ports, ip_addresses.
/// Also populates credential_assignments for successful integrations.
pub async fn execute_integrations(
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
    probe_results: &IntegrationProbeResults,
    host_data: &mut HostData,
    params: &ExecuteParams<'_>,
) -> Result<(), Error> {
    // Multiple credential mappings can resolve to the same integration + winning
    // credential (e.g. SnmpV1/V2c/V3 credentials plus the injected public default all
    // collapse to the single Snmp discriminant, which has one probe winner). Running
    // execute() once per mapping re-does the full collection against the same host
    // with the same credential — pure repetition. dedup_execution_keys() collapses
    // the mappings to the distinct (integration, winning credential) collections that
    // actually need to run; a genuinely different winning credential still runs.
    for (discriminant, _cred_id) in
        dedup_execution_keys(credential_mappings, &probe_results.working_credential_ids)
    {
        let Some(integration) = IntegrationRegistry::get(discriminant) else {
            continue;
        };

        // Use the credential that actually succeeded during probe. If no probe
        // winner was recorded for this integration, there's nothing to execute.
        let Some((cred_id, credential)) = probe_results.working_credential_ids.get(&discriminant)
        else {
            continue;
        };

        // Check if integration's associated service was matched
        let cred_type_discriminant: crate::server::credentials::r#impl::types::CredentialTypeDiscriminants = discriminant.into();
        let associated_service = cred_type_discriminant
            .to_credential_type()
            .associated_service();
        let service_matched = host_data
            .services
            .iter()
            .any(|s| s.base.service_definition.id() == associated_service.id());

        if !service_matched {
            // The credential authenticated and the collection never ran, which reads to an
            // operator exactly like the integration being ignored — the symptom a customer spent
            // days on, watching a controller they had authenticated to produce nothing.
            //
            // A successful probe feeds `ClientProbe` into service matching, so this failing is
            // anomalous rather than routine, which is why it is worth a line rather than a
            // per-host flood.
            tracing::warn!(
                ip = %params.ip,
                integration = ?discriminant,
                service = associated_service.name(),
                "Credential worked but its service was not matched; skipping collection"
            );
            params
                .ops
                .record_attempt_failure(
                    credential.into(),
                    params.ip,
                    AttemptOutcome::CollectionFailed,
                    format!(
                        "authenticated, but the {} service was not identified on this host, so \
                         nothing was collected",
                        associated_service.name()
                    ),
                    true,
                    *cred_id,
                )
                .await;
            continue;
        }

        let accept_invalid_certs = params
            .ops
            .config_store
            .get_accept_invalid_scan_certs()
            .await
            .unwrap_or(false);

        let matched_services_snapshot = host_data.services.clone();

        let probe_handle_ref = probe_results
            .probe_handles
            .get(&discriminant)
            .map(|h| h.as_ref() as &(dyn std::any::Any + Send + Sync));

        let ctx = IntegrationContext {
            ip: params.ip,
            credential,
            credential_id: *cred_id,
            interface_source: InterfaceSource {
                credential: discriminant,
                scope: integration.interface_view_scope(),
            },
            cancel: params.cancel,
            ops: params.ops,
            utils: params.utils,
            probe_handle: probe_handle_ref,
            matched_services: &matched_services_snapshot,
            open_ports: params.open_ports,
            endpoint_responses: params.endpoint_responses,
            host_id: params.host_id,
            host_naming_fallback: params.host_naming_fallback,
            known_subnets: params.known_subnets,
            accept_invalid_certs,
            scanning_subnet: params.scanning_subnet,
        };

        if let Err(e) =
            execute_with_progress_reporting(integration.as_ref(), &ctx, host_data, || async {
                let _ = params.ops.heartbeat().await;
            })
            .await
        {
            // A failed integration execute means a matched service (e.g. a Docker/Podman
            // daemon) produced no child services — the user-visible "unclaimed open ports,
            // no services" symptom. Surface it at warn so the underlying error (often a
            // bollard/serde response mismatch) is diagnosable rather than swallowed.
            tracing::warn!(
                ip = %params.ip,
                integration = ?discriminant,
                outcome = ?e.outcome(),
                error = e.message(),
                "Integration execute failed"
            );

            // …and tell the operator, which the log alone never did. The credential worked and
            // the collection after it did not, so this host's data is missing rather than stale
            // — a materially different thing from every other credential issue, and previously
            // knowable only by reading the daemon log.
            //
            // `user_assigned: true` unconditionally: unlike a probe, execute only runs after a
            // credential has already succeeded against this host, so there is no broadcast-noise
            // case to suppress here.
            params
                .ops
                .record_attempt_failure(
                    credential.into(),
                    params.ip,
                    e.outcome(),
                    e.message().to_string(),
                    true,
                    *cred_id,
                )
                .await;
        }
    }

    host_data
        .host
        .base
        .credential_assignments
        .extend(credential_assignments_from_probes(
            &probe_results.working_credential_ids,
            params.ip_address_id,
        ));

    Ok(())
}

/// Turn the credentials that probed successfully into host credential assignments.
///
/// This is how a discovery-scoped credential earns its keep: the server drops a
/// discovery's one-shot `integration_targets` once a scan completes, and what
/// survives is exactly the assignments produced here (written to the
/// `host_credentials` junction by `discover_host`). A Docker/Podman socket
/// credential probed over the daemon's own loopback address earns an assignment
/// on the daemon host and so keeps scanning containers on every later scan.
///
/// Two kinds are deliberately excluded:
/// - **SNMP**, which records its own assignments in `SnmpIntegration::execute`.
/// - **Network defaults** (`None` id), which are network-wide by definition and
///   must not be pinned to whichever host happened to answer them.
///
/// Runs on probe success alone — a matched-service skip or a failed `execute()`
/// does not suppress it, because the credential is proven either way.
pub(crate) fn credential_assignments_from_probes(
    working_credential_ids: &HashMap<
        CredentialQueryPayloadDiscriminants,
        (Option<Uuid>, CredentialQueryPayload),
    >,
    ip_address_id: Option<Uuid>,
) -> Vec<CredentialAssignment> {
    working_credential_ids
        .iter()
        .filter(|(discriminant, _)| **discriminant != CredentialQueryPayloadDiscriminants::Snmp)
        .filter_map(|(_, (cred_id, _credential))| {
            Some(CredentialAssignment {
                credential_id: (*cred_id)?,
                ip_address_ids: ip_address_id.map(|id| vec![id]),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::ContainerSocketQueryCredential;

    fn snmp_mapping() -> CredentialMapping<CredentialQueryPayload> {
        CredentialMapping {
            default_credential: Some(CredentialQueryPayload::default()), // Snmp
            ..Default::default()
        }
    }

    fn docker_socket_mapping() -> CredentialMapping<CredentialQueryPayload> {
        CredentialMapping {
            default_credential: Some(CredentialQueryPayload::DockerSocket(
                ContainerSocketQueryCredential { socket_path: None },
            )),
            ..Default::default()
        }
    }

    fn winners(
        entries: Vec<(
            CredentialQueryPayloadDiscriminants,
            Option<Uuid>,
            CredentialQueryPayload,
        )>,
    ) -> HashMap<CredentialQueryPayloadDiscriminants, (Option<Uuid>, CredentialQueryPayload)> {
        entries
            .into_iter()
            .map(|(d, id, payload)| (d, (id, payload)))
            .collect()
    }

    #[test]
    fn dedup_collapses_duplicate_snmp_mappings_to_one() {
        // SnmpV1/V2c/V3 + injected public default all resolve to the single Snmp
        // discriminant with one probe winner: three mappings, one collection.
        let mappings = vec![snmp_mapping(), snmp_mapping(), snmp_mapping()];
        let cred_id = Some(Uuid::new_v4());
        let w = winners(vec![(
            CredentialQueryPayloadDiscriminants::Snmp,
            cred_id,
            CredentialQueryPayload::default(),
        )]);

        let keys = dedup_execution_keys(&mappings, &w);
        assert_eq!(
            keys,
            vec![(CredentialQueryPayloadDiscriminants::Snmp, cred_id)]
        );
    }

    #[test]
    fn dedup_drops_mappings_without_probe_winner() {
        // No probe winner for the mapping's integration => nothing to execute.
        let mappings = vec![snmp_mapping()];
        let w = winners(vec![]);
        assert!(dedup_execution_keys(&mappings, &w).is_empty());
    }

    #[test]
    fn dedup_preserves_distinct_integrations_in_order() {
        // Different integrations each keep their own collection; first-seen order.
        let mappings = vec![snmp_mapping(), docker_socket_mapping(), snmp_mapping()];
        let snmp_id = Some(Uuid::new_v4());
        let docker_id = Some(Uuid::new_v4());
        let w = winners(vec![
            (
                CredentialQueryPayloadDiscriminants::Snmp,
                snmp_id,
                CredentialQueryPayload::default(),
            ),
            (
                CredentialQueryPayloadDiscriminants::DockerSocket,
                docker_id,
                CredentialQueryPayload::DockerSocket(ContainerSocketQueryCredential {
                    socket_path: None,
                }),
            ),
        ]);

        let keys = dedup_execution_keys(&mappings, &w);
        assert_eq!(
            keys,
            vec![
                (CredentialQueryPayloadDiscriminants::Snmp, snmp_id),
                (CredentialQueryPayloadDiscriminants::DockerSocket, docker_id),
            ]
        );
    }

    fn docker_socket_payload() -> CredentialQueryPayload {
        CredentialQueryPayload::DockerSocket(ContainerSocketQueryCredential { socket_path: None })
    }

    /// A local Docker/Podman socket credential arrives as a `DaemonHost` integration target,
    /// which the server drops from the discovery once the scan completes. What has to carry it
    /// into every later scan is the assignment produced here, on the daemon host's own loopback
    /// address — without it, container discovery would silently stop after the first scan.
    #[test]
    fn a_working_socket_credential_earns_an_assignment_on_the_address_it_probed() {
        let cred_id = Uuid::new_v4();
        let loopback_ip_id = Uuid::new_v4();
        let w = winners(vec![(
            CredentialQueryPayloadDiscriminants::DockerSocket,
            Some(cred_id),
            docker_socket_payload(),
        )]);

        let assignments = credential_assignments_from_probes(&w, Some(loopback_ip_id));

        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].credential_id, cred_id);
        assert_eq!(assignments[0].ip_address_ids, Some(vec![loopback_ip_id]));
    }

    /// Two kinds must never be promoted here: SNMP records its own assignments inside
    /// `SnmpIntegration::execute`, and a network default (`None` id) is network-wide by
    /// definition — pinning it to whichever host answered would turn a broadcast credential
    /// into a host-scoped one.
    #[test]
    fn snmp_and_network_defaults_are_not_promoted() {
        let w = winners(vec![
            (
                CredentialQueryPayloadDiscriminants::Snmp,
                Some(Uuid::new_v4()),
                CredentialQueryPayload::default(),
            ),
            (
                CredentialQueryPayloadDiscriminants::DockerSocket,
                None,
                docker_socket_payload(),
            ),
        ]);

        assert!(
            credential_assignments_from_probes(&w, Some(Uuid::new_v4())).is_empty(),
            "neither an SNMP winner nor an unidentified network default earns a host assignment"
        );
    }
}

#[cfg(test)]
mod ledger_tests {
    use super::*;

    fn ip() -> IpAddr {
        "10.0.0.1".parse().unwrap()
    }

    fn entry(user_assigned: bool, disposition: Disposition) -> DispositionEntry {
        DispositionEntry {
            discriminant: CredentialQueryPayloadDiscriminants::Snmp,
            user_assigned,
            credential_id: None,
            disposition,
        }
    }

    /// The reason the ledger exists. Three paths went quiet by saying nothing, and each was found
    /// by reading the code rather than by anything failing — this is what should have caught them.
    ///
    /// `debug_assert!` fires in test builds, so reaching the assertion is the pass condition and
    /// this has to check it the long way round.
    #[test]
    #[should_panic(expected = "unaccounted for")]
    fn a_branch_that_records_nothing_is_loud() {
        resolve_ledger(vec![entry(true, Disposition::Unresolved)], ip());
    }

    /// A credential the user pinned to this host, which failed, is the case the whole mechanism
    /// exists for.
    #[test]
    fn a_recorded_failure_becomes_an_issue() {
        let issues = resolve_ledger(
            vec![entry(
                true,
                Disposition::Failed(AttemptOutcome::Rejected, "refused".to_string()),
            )],
            ip(),
        );

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].ip, ip());
        assert!(matches!(
            issues[0].reason,
            CredentialIssueReason::Attempted {
                outcome: AttemptOutcome::Rejected,
                ..
            }
        ));
    }

    /// The same failure on a network default stays quiet. It is broadcast at every address in the
    /// subnet, so reporting it would put a line per unresponsive host into the notification —
    /// the policy lives in `issue_for_attempt` and the ledger defers to it rather than
    /// second-guessing it.
    #[test]
    fn a_recorded_failure_on_a_network_default_stays_quiet() {
        let issues = resolve_ledger(
            vec![entry(
                false,
                Disposition::Failed(AttemptOutcome::Rejected, "refused".to_string()),
            )],
            ip(),
        );
        assert!(issues.is_empty());
    }

    /// Silence has to be a choice with a reason attached, not the absence of code. These are the
    /// states a branch writes when it means "nothing to say here", and none of them reports.
    #[test]
    fn deliberate_silences_are_recorded_and_not_reported() {
        let issues = resolve_ledger(
            vec![
                entry(true, Disposition::Probed),
                entry(true, Disposition::Suppressed("the scan was cancelled")),
                entry(
                    true,
                    Disposition::Suppressed(
                        "no credential in this mapping applies to this address",
                    ),
                ),
            ],
            ip(),
        );
        assert!(issues.is_empty());
    }

    /// Several mappings at one address each get their own slot, so one failing does not mask
    /// another and one succeeding does not swallow a failure beside it.
    #[test]
    fn entries_are_independent() {
        let issues = resolve_ledger(
            vec![
                entry(true, Disposition::Probed),
                entry(
                    true,
                    Disposition::Failed(AttemptOutcome::TlsFailed, "bad cert".to_string()),
                ),
                entry(
                    true,
                    Disposition::Suppressed("gate closed; reported as GateClosed"),
                ),
            ],
            ip(),
        );
        assert_eq!(issues.len(), 1, "{issues:?}");
    }
}
