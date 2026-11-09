use crate::daemon::discovery::integration::container::ContainerRuntime;
use crate::daemon::discovery::service::base::DiscoveryRunner;
use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{
    DaemonUtils, filtered_own_nics, interfaces_fingerprint, own_addresses_from_nics,
    session_subnets,
};
use crate::server::daemons::r#impl::api::DaemonDiscoveryRequest;
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::hosts::r#impl::base::Host;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::ports::r#impl::coverage::PortCoverage;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::inference::placeable_subnet;
use anyhow::{Error, Result};
use futures::future::join_all;
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

// Phase 1 (0-5%): Self-report + localhost integrations.
// Phase 2 (5-100%): Network scan with per-host integration probe + execute.

/// How often a running session resends its progress when nothing else has reported.
const HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

/// Run `work`, resending the session's progress every `HEARTBEAT_INTERVAL` for as long as it runs.
///
/// The server fails a session it hears nothing about for five minutes, and several stretches of a
/// healthy session report nothing for longer: self-report, integration probes, subnet creation,
/// subnet resolution, mDNS. The heartbeat loop is a branch of the `select!`, not an arm body, so a
/// slow heartbeat request runs alongside the work instead of pausing it.
async fn with_heartbeat<T>(ops: &DiscoveryOps, work: impl std::future::Future<Output = T>) -> T {
    let heartbeat = async {
        let mut ticker = tokio::time::interval(HEARTBEAT_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let _ = ops.heartbeat().await;
        }
    };

    tokio::select! {
        result = work => result,
        never = heartbeat => never,
    }
}

impl DiscoveryRunner {
    /// Every line logged while the session runs carries its id, including those from the scan
    /// pipeline and integrations, which do not know which session they serve.
    #[tracing::instrument(name = "discovery", skip_all, fields(session_id = %request.session_id))]
    pub async fn discover(
        &mut self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        let is_first_run = !self.service.config_store.has_self_reported().await;
        // Gateways only enrich what the scan finds. Failing to read them must not fail the
        // session before it exists, where nothing could report the failure.
        let gateway_ips = match self.service.utils.get_own_routing_table_gateway_ips().await {
            Ok(ips) => ips,
            Err(e) => {
                tracing::warn!(
                    session_id = %request.session_id,
                    error = %e,
                    "Could not read the routing table's gateways; scanning without them"
                );
                Vec::new()
            }
        };
        let ops = DiscoveryOps::new(&self.service, DiscoveryType::from(&*self));

        // Local Docker/Podman socket integrations are no longer injected here. They arrive in the
        // server-sent `credential_mappings` as DockerSocket/PodmanSocket overrides at 127.0.0.1,
        // driven by the per-daemon `integration_targets` (init-command targeting) — explicit
        // opt-in, no per-integration daemon flags. The localhost-integration phase probes them;
        // if the socket isn't actually present the probe fails gracefully and is skipped. The
        // container integration resolves the concrete local socket path (rootful/rootless Podman,
        // Docker socket) when it sees these payloads.

        // SNMP `public` arrives the same way: it is a stored default credential, assigned to the
        // site, so the daemon tries only what the server sends.

        // The session starts first, so the server hears `Started` before any subnet work. That
        // work can take minutes (in ServerPoll each subnet waits up to two minutes for the server
        // to confirm it) and used to run before the server knew the session existed.
        if let Err(e) = ops.start_session(&request, gateway_ips).await {
            // Without a session there is nothing to report the failure through.
            if ops.get_session().await.is_err() {
                return Err(e);
            }
            ops.finish_session(Err(e), cancel).await?;
            return Ok(());
        }

        // A credential whose files or sockets were declared for another OS cannot work on this
        // daemon. Drop it before any phase can try it, and say so once per credential.
        let os_mismatches = crate::daemon::discovery::credentials::take_os_mismatches(
            &mut self.credential_mappings,
            crate::server::daemons::r#impl::base::DaemonOs::current(),
        );
        if !os_mismatches.is_empty()
            && let Ok(session) = ops.get_session().await
            && let Ok(mut warnings) = session.warnings.lock()
        {
            warnings.extend(os_mismatches);
        }

        let discovery_result = with_heartbeat(&ops, async {
            let created_subnets = self.create_initial_subnets(&ops, &cancel).await?;
            // An interface refresh exists only to classify and report the interfaced subnets.
            if matches!(self.discovery_type, DiscoveryType::InterfaceRefresh { .. }) {
                return Ok(());
            }
            self.run_unified_phases(&ops, &created_subnets, is_first_run, &cancel)
                .await
        })
        .await;

        ops.finish_session(discovery_result, cancel).await?;
        Ok(())
    }

    /// Subnet setup at the start of a session. Merges daemon interface subnets with Docker
    /// network subnets (host CIDR wins on overlap). Runs before the self-report and localhost
    /// integration phases, which take the subnets it returns. Reports a desktop container VM from the
    /// same Docker connection.
    async fn create_initial_subnets(
        &self,
        ops: &DiscoveryOps,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error> {
        let site_id = self
            .service
            .config_store
            .get_site_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Site ID not set"))?;

        let interface_filter = self.service.config_store.get_interfaces().await?;
        let nics = filtered_own_nics(&interface_filter);
        let fingerprint = interfaces_fingerprint(&nics);
        let (_, host_subnets, _) = own_addresses_from_nics(nics, site_id);

        // The runtimes' bridges are handled by the container integration phase, so the session
        // neither creates nor scans them.
        let (runtime_subnets, docker_client) = self.runtime_subnets(site_id).await;
        report_desktop_vm(ops, docker_client.as_ref()).await;
        let subnets_to_create = session_subnets(host_subnets, runtime_subnets);

        tracing::info!(
            subnet_count = subnets_to_create.len(),
            cidrs = ?subnets_to_create.iter().map(|s| s.base.cidr.to_string()).collect::<Vec<_>>(),
            "Creating subnets for unified discovery"
        );

        let subnet_futures = subnets_to_create.iter().map(|subnet| async move {
            let cidr = *subnet.base.cidr;
            match ops.create_subnet(subnet, cancel).await {
                Ok(created) => {
                    tracing::debug!(cidr = %cidr, subnet_id = %created.id, "Subnet created");
                    Some(created)
                }
                Err(e) => {
                    tracing::warn!(cidr = %cidr, error = %e, "Failed to create subnet");
                    None
                }
            }
        });
        let created_subnets: Vec<Subnet> = join_all(subnet_futures)
            .await
            .into_iter()
            .flatten()
            .collect();

        // The daemon's interfaced subnets, reported once on its next status.
        self.service
            .set_interfaced_report(subnets_to_create, fingerprint);

        Ok(created_subnets)
    }

    /// The networks this host's container runtimes report, read with the session's localhost
    /// Docker and Podman credentials (socket or proxy). Empty with none: the session then has
    /// nothing that types a bridge, and scans each one as an ordinary subnet. Also returns the
    /// first Docker client it reached, for the desktop container VM check.
    async fn runtime_subnets(&self, site_id: Uuid) -> (Vec<Subnet>, Option<bollard::Docker>) {
        use crate::daemon::discovery::integration::container::{
            build_container_proxy_url, resolve_container_ssl,
        };
        use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;

        let utils = &self.service.utils;
        let mut subnets = Vec::new();
        let mut docker = None;
        let overrides = self
            .credential_mappings
            .iter()
            .flat_map(|m| m.ip_overrides.iter())
            .filter(|o| o.is_localhost());
        for o in overrides {
            // Temp files holding a proxy's TLS material, kept until its networks are listed.
            let mut _ssl_temp_handles = Vec::new();
            let (runtime, client) = match &o.credential {
                CredentialQueryPayload::DockerSocket(c)
                | CredentialQueryPayload::PodmanSocket(c) => {
                    let runtime = if matches!(o.credential, CredentialQueryPayload::DockerSocket(_))
                    {
                        ContainerRuntime::Docker
                    } else {
                        ContainerRuntime::Podman
                    };
                    // The same path resolution the socket integrations probe with.
                    let mut path = c
                        .socket_path
                        .as_ref()
                        .filter(|s| !s.is_blank())
                        .map(|s| s.as_str().to_string());
                    if runtime == ContainerRuntime::Podman && path.is_none() {
                        path = crate::daemon::discovery::integration::podman::resolve_podman_socket_path();
                    }
                    (
                        runtime,
                        utils.new_container_socket_client(runtime, path).await,
                    )
                }
                CredentialQueryPayload::DockerProxy(c) | CredentialQueryPayload::PodmanProxy(c) => {
                    let runtime = if matches!(o.credential, CredentialQueryPayload::DockerProxy(_))
                    {
                        ContainerRuntime::Docker
                    } else {
                        ContainerRuntime::Podman
                    };
                    let client = match resolve_container_ssl(c, runtime.label()) {
                        Ok((ssl, handles)) => {
                            _ssl_temp_handles = handles;
                            utils
                                .new_docker_client(
                                    Ok(Some(build_container_proxy_url(o.ip, c))),
                                    Ok(ssl),
                                )
                                .await
                        }
                        Err(e) => Err(e),
                    };
                    (runtime, client)
                }
                _ => continue,
            };
            match client {
                Ok(client) => {
                    subnets.extend(self.list_runtime_networks(site_id, &client, runtime).await);
                    if runtime == ContainerRuntime::Docker && docker.is_none() {
                        docker = Some(client);
                    }
                }
                Err(e) => tracing::warn!(
                    runtime = runtime.label(),
                    error = %e,
                    "Could not reach the container runtime to type its networks; its bridges \
                     are scanned as ordinary subnets"
                ),
            }
        }
        (subnets, docker)
    }

    async fn list_runtime_networks(
        &self,
        site_id: Uuid,
        client: &bollard::Docker,
        runtime: ContainerRuntime,
    ) -> Vec<Subnet> {
        self.service
            .utils
            .get_subnets_from_docker_networks(site_id, client, runtime, Uuid::nil())
            .await
            .unwrap_or_else(|e| {
                // Said here because nothing downstream will.
                tracing::warn!(
                    runtime = runtime.label(),
                    error = %e,
                    "Failed to list container networks; their bridges are scanned as ordinary \
                     subnets"
                );
                Vec::new()
            })
    }

    /// Run all unified discovery phases.
    ///
    /// Phase 1 (0-5%): Self-report + localhost integrations.
    /// Phase 2 (5-100%): Network scan with per-host integration probe + execute.
    async fn run_unified_phases(
        &self,
        ops: &DiscoveryOps,
        created_subnets: &[Subnet],
        is_first_run: bool,
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
        let start = std::time::Instant::now();
        let session = ops.get_session().await?;

        // Phase 1: Daemon Host (0-5%)
        session.set_progress_range(0, 5);

        if is_first_run {
            tracing::info!("Running self-report phase (first run)");

            if let Err(e) = self
                .run_self_report_phase(ops, created_subnets, cancel)
                .await
            {
                tracing::error!(error = %e, "Self-report phase failed, continuing with network phase");
            } else if let Err(e) = self.service.config_store.set_has_self_reported().await {
                tracing::warn!(error = %e, "Failed to persist self-report flag");
            }
        } else if let Err(e) = self
            .run_daemon_host_interfaces_phase(ops, created_subnets, cancel)
            .await
        {
            tracing::error!(error = %e, "Daemon-host interface phase failed, continuing");
        }

        // Run localhost integrations (generic — any integration with localhost credential)
        let bridge_override_ips = match self
            .run_localhost_integrations(ops, created_subnets, cancel)
            .await
        {
            Ok(ips) => ips,
            Err(e) => {
                tracing::error!(error = %e, "Localhost integration phase failed, continuing");
                HashSet::new()
            }
        };

        // Wake hosts that sleep between scans, so the sweep below can find them. After the
        // daemon-host phase because the subnets it creates are where a target's directed
        // broadcast is computed from.
        let wake_subnets: Vec<Subnet> = self
            .known_subnets
            .iter()
            .chain(created_subnets)
            .cloned()
            .collect();
        crate::daemon::discovery::wake_on_lan::wake(
            ops,
            &self.credential_mappings,
            &wake_subnets,
            cancel,
        )
        .await;

        ops.report_progress(100).await?;

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Phase 2: Network Scan (5-100%)
        session.set_progress_range(5, 100);

        let network_hosts = self
            .run_network_phase(&bridge_override_ips, created_subnets, cancel)
            .await?;

        // Completion banner
        self.log_completion_banner(&network_hosts, start.elapsed());

        Ok(())
    }

    /// Run integrations for localhost credentials (e.g., Docker on daemon host).
    /// Uses the same dispatch as network scanning — localhost credentials aren't special,
    /// they just target a known host_id instead of ARP-discovered hosts.
    ///
    /// Returns the override addresses on this host's container bridges, which the bridge step
    /// after the container scan owns (see `bridge_containers`).
    async fn run_localhost_integrations(
        &self,
        ops: &DiscoveryOps,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<HashSet<IpAddr>, Error> {
        use crate::daemon::discovery::integration::dispatch;

        // Build localhost-only credential mappings
        let localhost_mappings: Vec<_> = self
            .credential_mappings
            .iter()
            .filter(|m| m.ip_overrides.iter().any(|o| o.is_localhost()))
            .cloned()
            .collect();

        if localhost_mappings.is_empty() {
            tracing::debug!(
                "No localhost credential mappings found, skipping localhost integrations"
            );
            return Ok(HashSet::new());
        }

        tracing::info!(
            mappings = localhost_mappings.len(),
            "Running localhost integrations"
        );

        // Probe with 127.0.0.1 — credentials are keyed to localhost, not the daemon's real IP.
        // The daemon's real IP is used for subnet/interface matching below.
        let localhost_ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let trusted_ca = ops.config_store.get_trusted_ca().await;
        let probe_results = dispatch::probe_integrations(
            localhost_ip,
            &localhost_mappings,
            &[],  // No port scan for localhost — integrations do their own probing
            true, // skip the probe-gate: daemon-host integrations (e.g. a proxy) always self-probe here
            cancel,
            &self.service.utils,
            ops.config_store.get_accept_invalid_scan_certs().await?,
            trusted_ca.as_deref(),
            &ops.config_store,
        )
        .await?;
        ops.stamp_activity().await;

        // Deliver before the early return below. These were built correctly and then dropped,
        // so a wrong Docker or Podman socket credential on the daemon host reported nothing at
        // all — and the early return is precisely the path a failing credential takes.
        ops.record_credential_issues(&probe_results.credential_issues)
            .await;

        if probe_results.client_responses.is_empty() {
            tracing::debug!("No localhost integration probes succeeded");
            return Ok(HashSet::new());
        }

        tracing::info!(
            probes_succeeded = probe_results.client_responses.len(),
            "Localhost integration probes complete"
        );

        // Attribute the discovered runtime service to 127.0.0.1 — the credential is keyed to
        // localhost and the daemon host already carries a seeded loopback interface. Using the
        // daemon's real IP here would land in the loopback subnet (its real subnet isn't
        // discovered in a socket-only scan), adding a spurious non-loopback interface.
        let host_ip = localhost_ip;

        // Build HostData via service matching using probe results
        let subnet = created_subnets
            .iter()
            .find(|s| s.base.cidr.contains(&host_ip))
            .or_else(|| created_subnets.first());

        let Some(subnet) = subnet else {
            tracing::warn!("No subnet found for localhost integrations, skipping");
            return Ok(HashSet::new());
        };

        let ip_address = IPAddress::new(crate::server::ip_addresses::r#impl::base::IPAddressBase {
            site_id: subnet.base.site_id,
            host_id: Uuid::nil(),
            name: None,
            subnet_id: subnet.id,
            ip_address: host_ip,
            mac_address: None,
            position: 0,
            // The daemon's own loopback, the address its own-host credentials are keyed to.
            ip_address_source:
                crate::server::shared::attribution::AttributeSource::DaemonSelfReport,
        });

        let params = crate::server::services::r#impl::base::ServiceMatchBaselineParams {
            subnet,
            ip_address: &ip_address,
            all_ports: &probe_results.additional_ports.to_vec(),
            endpoint_responses: &vec![],
            virtualization_metadata: &None,
            virtualization_service_id: None,
            client_responses: &probe_results.client_responses,
            // The daemon's own host, probed locally.
            managed_device: &None,
            // Probed locally over loopback; multicast browsing does not apply.
            dns_sd: &None,
            certificates: &[],
            fingerprints: &[],
        };

        // Every port here is one a daemon-host integration's probe answered on, and records that
        // probe. The fallback is the daemon reading its own host.
        let mut host_data = match ops
            .build_host_from_scan(
                params,
                None,
                self.host_naming_fallback,
                crate::server::shared::attribution::AttributeSource::DaemonSelfReport,
            )
            .await?
        {
            Some(hd) => hd,
            None => {
                tracing::warn!("Localhost service matching returned no host");
                return Ok(HashSet::new());
            }
        };

        host_data.host.id = self.host_id;
        host_data.check_credential_free_collection();

        // A bridge container's `0.0.0.0` publish is bound at its runtime host's addresses, and
        // this pass collects the host over loopback. Add the daemon's own addresses that sit in a
        // subnet this scan created, so the publish is bound where it answers rather than at
        // 127.0.0.1 alone, and takes its port from Unclaimed Open Ports there (GH #784). An
        // address outside every created subnet stays out, for the misplacement noted above.
        let interface_filter = self.service.config_store.get_interfaces().await?;
        let (own_addresses, _, _) = self
            .service
            .utils
            .get_own_interfaces(subnet.base.site_id, &interface_filter)
            .await?;
        host_data
            .ip_addresses
            .extend(own_addresses_in_subnets(own_addresses, created_subnets));

        // Execute integrations — use localhost_ip since credentials are keyed to 127.0.0.1
        let execute_params = dispatch::ExecuteParams {
            ip: localhost_ip,
            cancel,
            ops,
            utils: &self.service.utils,
            open_ports: &probe_results.additional_ports,
            endpoint_responses: &[],
            host_id: self.host_id,
            host_naming_fallback: self.host_naming_fallback,
            // The daemon-host phase runs before the network sweep, so the subnets it just
            // created are all that is known at this point.
            known_subnets: created_subnets,
            scanned_subnets: created_subnets,
            scanning_subnet: None,
            ip_address_id: Some(ip_address.id),
            // The daemon-host phase is these integrations' work, so they move its bar.
            owns_progress: true,
        };

        dispatch::execute_integrations(
            &localhost_mappings,
            &probe_results,
            &mut host_data,
            &execute_params,
        )
        .await?;

        ops.record_equal_reach_integrations(localhost_ip, &host_data)
            .await;

        // The container scan above is the only record of which containers are running and where
        // on the bridges they sit, so credentials at them are tried here, before the host is sent.
        let bridge_override_ips = self
            .run_bridge_container_integrations(ops, &mut host_data, created_subnets, cancel)
            .await;

        // Persist results
        tracing::info!(
            services = host_data.services.len(),
            ip_addresses = host_data.ip_addresses.len(),
            "Persisting localhost integration results"
        );

        ops.create_host(
            host_data.host,
            host_data.ip_addresses,
            host_data.ports,
            host_data.services,
            host_data.interfaces,
            host_data.subnets,
            host_data.interfaces_complete,
            host_data.interface_data_complete,
            PortCoverage::default(),
            cancel,
        )
        .await?;

        Ok(bridge_override_ips)
    }

    /// Network phase: run ARP + deep scan to discover hosts and services.
    ///
    /// `bridge_override_ips` were handled by the bridge step, so their overrides are not handed to
    /// the scan, which would report each one as a target it did not scan.
    async fn run_network_phase(
        &self,
        bridge_override_ips: &HashSet<IpAddr>,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<Vec<(IpAddr, Host, super::network::DiscoveredHostData)>, Error> {
        let mut credential_mappings = self.credential_mappings.clone();
        for mapping in &mut credential_mappings {
            mapping
                .ip_overrides
                .retain(|o| !bridge_override_ips.contains(&o.ip));
        }

        // Network discovery owns subnet resolution — unified just coordinates
        let network_discovery = super::network::NetworkScan::new(
            self.subnet_ids.clone(),
            self.host_naming_fallback,
            self.scan_settings.clone(),
            credential_mappings,
            self.target_ips.clone(),
            self.extra_ports.clone(),
            self.known_subnets.clone(),
            created_subnets.to_vec(),
        );

        let ops = super::ops::DiscoveryOps::new(&self.service, DiscoveryType::from(self));
        let utils = &self.service.utils;

        let resolved = network_discovery.resolve_scan_subnets(&ops, utils).await?;

        tracing::info!(
            cidrs = ?resolved.subnets.iter().map(|s| s.base.cidr.to_string()).collect::<Vec<_>>(),
            targets = ?resolved.target_ips.as_ref().map(|t| t.len()),
            "Running network scan phase"
        );

        // scan_and_process_hosts uses the active session
        // (set by our start_discovery call above)
        let network_result = network_discovery
            .scan_and_process_hosts(
                resolved.subnets,
                resolved.site_subnets,
                resolved.target_ips,
                cancel.clone(),
                &ops,
                utils,
            )
            .await;

        match &network_result {
            Ok(hosts) => {
                tracing::info!(
                    hosts_discovered = hosts.len(),
                    "Network scan phase complete"
                );
            }
            Err(_) if cancel.is_cancelled() => {
                return Err(anyhow::anyhow!("Discovery cancelled"));
            }
            Err(e) => {
                tracing::error!(error = %e, "Network scan phase failed");
            }
        }

        network_result
    }

    /// Log a summary banner at the end of discovery, matching the start banner format.
    fn log_completion_banner(
        &self,
        network_hosts: &[(IpAddr, Host, super::network::DiscoveredHostData)],
        duration: std::time::Duration,
    ) {
        let hosts_discovered = network_hosts.len();
        let scan_type = if self.scan_settings.is_full_scan {
            "full"
        } else {
            "light"
        };

        // Banner
        tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        tracing::info!("  Discovery Complete");
        tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        tracing::info!("  {:<20}{}", "Hosts Discovered:", hosts_discovered);
        tracing::info!("  {:<20}{}s", "Duration:", duration.as_secs());
        tracing::info!("  {:<20}{}", "Scan Type:", scan_type);

        if !self.credential_mappings.is_empty() {
            let hosts_for_summary: Vec<_> = network_hosts
                .iter()
                .map(|(ip, host, _)| (*ip, host.clone()))
                .collect();
            let by_type = crate::daemon::discovery::credentials::summarize_credential_assignments(
                &hosts_for_summary,
                &self.credential_mappings,
            );

            tracing::info!("  ───────────────────────────────────────────────────────────");
            tracing::info!("  Credential Mappings:");
            for (type_label, details) in &by_type {
                tracing::info!(
                    "    {:<20}{} hosts",
                    format!("{}:", type_label),
                    details.len()
                );
                for detail in details {
                    tracing::info!("    {}  {}", type_label, detail);
                }
            }
        }

        tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }
}

/// The daemon's own addresses that sit in one of `subnets`, each filed under the most specific
/// one. Loopback stays out: the localhost pass already carries 127.0.0.1.
fn own_addresses_in_subnets(own: Vec<IPAddress>, subnets: &[Subnet]) -> Vec<IPAddress> {
    own.into_iter()
        .filter(|a| !a.base.ip_address.is_loopback())
        .filter_map(|mut a| {
            a.base.subnet_id = placeable_subnet(subnets, a.base.ip_address)?.id;
            Some(a)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::shared::types::entities::EntitySource;
    use crate::server::subnets::r#impl::base::{SubnetBase, SubnetCidr, SubnetCidrValue};
    use crate::server::subnets::r#impl::base::{SubnetName, SubnetTypeAttributed};
    use crate::server::subnets::r#impl::types::SubnetType;

    fn subnet(cidr: &str, subnet_type: SubnetType) -> Subnet {
        Subnet::new(SubnetBase {
            name: SubnetName::unattributed(cidr.to_string()),
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr.parse().unwrap()),
                AttributeSource::DaemonSelfReport,
            ),
            subnet_type: SubnetTypeAttributed::unattributed(subnet_type),
            source: EntitySource::Discovery,
            ..Default::default()
        })
    }

    fn own(ip: &str) -> IPAddress {
        IPAddress::new(IPAddressBase {
            ip_address: ip.parse().unwrap(),
            subnet_id: Uuid::new_v4(),
            ..Default::default()
        })
    }

    /// The Mac of GH #784: a LAN, a VPN tunnel and loopback were scanned. The LAN and VPN
    /// addresses join the socket-collected host under the subnets the scan created; loopback is
    /// already there, and an address no created subnet holds would be misplaced.
    #[test]
    fn own_addresses_join_only_under_a_created_subnet() {
        let lan = subnet("192.168.4.0/22", SubnetType::Lan);
        let vpn = subnet("10.0.0.0/24", SubnetType::VpnTunnel);
        let subnets = vec![
            subnet("127.0.0.0/8", SubnetType::Loopback),
            lan.clone(),
            vpn.clone(),
        ];

        let added = own_addresses_in_subnets(
            vec![
                own("127.0.0.1"),
                own("192.168.4.58"),
                own("10.0.0.3"),
                own("172.30.1.2"),
            ],
            &subnets,
        );

        let placed: Vec<(String, Uuid)> = added
            .iter()
            .map(|a| (a.base.ip_address.to_string(), a.base.subnet_id))
            .collect();
        assert_eq!(
            placed,
            vec![
                ("192.168.4.58".to_string(), lan.id),
                ("10.0.0.3".to_string(), vpn.id)
            ]
        );
    }
}

/// Warn when this daemon runs inside Docker Desktop, OrbStack or Colima, whose VM network keeps
/// ARP, mDNS and LLDP off the LAN. `docker` is the engine the daemon reached, if any: Colima is
/// named only by the engine's `info`.
async fn report_desktop_vm(ops: &DiscoveryOps, docker: Option<&bollard::Docker>) {
    use crate::daemon::discovery::types::warnings::DiscoveryWarning;
    use crate::daemon::utils::host_os::{desktop_vm, kernel_release};

    let Some(kernel) = kernel_release() else {
        return;
    };
    let engine_info = match docker {
        Some(client) => client.info().await.ok(),
        None => None,
    };
    let Some(vm) = desktop_vm(&kernel, engine_info.as_ref()) else {
        return;
    };
    tracing::warn!(
        ?vm,
        "The daemon runs inside a desktop container VM, so ARP, mDNS and LLDP do not reach the LAN"
    );
    if let Ok(session) = ops.get_session().await
        && let Ok(mut warnings) = session.warnings.lock()
    {
        warnings.push(DiscoveryWarning::DaemonInDesktopVm { vm });
    }
}
