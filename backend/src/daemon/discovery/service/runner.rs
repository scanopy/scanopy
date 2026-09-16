use crate::daemon::discovery::integration::container::ContainerRuntime;
use crate::daemon::discovery::service::base::DiscoveryRunner;
use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{DaemonUtils, merge_host_and_docker_subnets};
use crate::server::credentials::r#impl::mapping::{CredentialMapping, CredentialQueryPayload};
use crate::server::daemons::r#impl::api::DaemonDiscoveryRequest;
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::hosts::r#impl::base::Host;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::subnets::r#impl::base::Subnet;
use anyhow::{Error, Result};
use futures::future::join_all;
use std::net::{IpAddr, Ipv4Addr};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

// Phase 1 (0-5%): Self-report + localhost integrations.
// Phase 2 (5-100%): Network scan with per-host integration probe + execute.

impl DiscoveryRunner {
    pub async fn discover(
        &mut self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        let is_first_run = !self.service.config_store.has_self_reported().await;
        let gateway_ips = self
            .service
            .utils
            .get_own_routing_table_gateway_ips()
            .await?;
        let ops = DiscoveryOps::new(&self.service, DiscoveryType::from(&*self));

        // Local Docker/Podman socket integrations are no longer injected here. They arrive in the
        // server-sent `credential_mappings` as DockerSocket/PodmanSocket overrides at 127.0.0.1,
        // driven by the per-daemon `integration_targets` (init-command targeting) — explicit
        // opt-in, no per-integration daemon flags. The localhost-integration phase probes them;
        // if the socket isn't actually present the probe fails gracefully and is skipped. The
        // container integration resolves the concrete local socket path (rootful/rootless Podman,
        // Docker socket) when it sees these payloads.

        // Always try SNMP "public" community on all hosts.
        // Injected as a broadcast default — user-configured credentials (IP overrides) take priority.
        self.credential_mappings.push(CredentialMapping {
            default_credential: Some(CredentialQueryPayload::Snmp(
                crate::server::credentials::r#impl::mapping::SnmpQueryCredential::public_default(),
            )),
            // Synthesized here, not stored anywhere, so there is no record for a warning about it
            // to point at.
            default_credential_id: None,
            ip_overrides: vec![],
        });

        // Create subnets before session init (like other runners)
        let created_subnets = match self.create_initial_subnets(&ops, &cancel).await {
            Ok(subnets) => subnets,
            Err(e) => {
                let daemon_id = self.service.config_store.get_id().await?;
                if let Err(init_err) = ops
                    .initialize_session(&request, daemon_id, gateway_ips)
                    .await
                {
                    tracing::error!(
                        "Failed to initialize session for error reporting: {}",
                        init_err
                    );
                    return Err(e);
                }
                ops.finish_session(Err(e), cancel).await?;
                return Ok(());
            }
        };

        // Start session
        ops.start_session(&request, gateway_ips).await?;

        // Run the orchestrated phases
        let discovery_result = self
            .run_unified_phases(&ops, &created_subnets, is_first_run, &cancel)
            .await;

        ops.finish_session(discovery_result, cancel).await?;
        Ok(())
    }

    /// Pre-session subnet setup. Merges daemon interface subnets with Docker network
    /// subnets (host CIDR wins on overlap). Runs before session initialization so
    /// subnets exist for self-report and localhost integration phases.
    async fn create_initial_subnets(
        &self,
        ops: &DiscoveryOps,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error> {
        let network_id = self
            .service
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let utils = &self.service.utils;

        let interface_filter = self.service.config_store.get_interfaces().await?;
        let (_, subnets, _) = utils
            .get_own_interfaces(network_id, &interface_filter)
            .await?;

        // Get docker subnets for merging
        let (docker_proxy, docker_proxy_ssl_info, _ssl_temp_handles, _, _) =
            crate::daemon::discovery::integration::docker::resolve_docker_proxy(
                &self.credential_mappings,
                &self.service.config_store,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::debug!(error = %e, "Failed to resolve Docker proxy for subnet discovery");
                (Ok(None), Ok(None), Vec::new(), None, None)
            });

        let docker_subnets = if let Ok(docker_client) = self
            .service
            .utils
            .new_docker_client(docker_proxy, docker_proxy_ssl_info)
            .await
        {
            self.service
                .utils
                .get_subnets_from_docker_networks(
                    network_id,
                    &docker_client,
                    ContainerRuntime::Docker,
                    Uuid::nil(),
                )
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Merge host and Docker subnets — host subnets always win on CIDR overlap
        let merged = merge_host_and_docker_subnets(subnets, docker_subnets);

        // Filter out container-runtime bridge subnets (Docker/Podman) — those
        // are handled by the container integration phase.
        let subnets_to_create: Vec<Subnet> = merged
            .into_iter()
            .filter(|s| !s.is_container_bridge_subnet())
            .collect();

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

        Ok(created_subnets)
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
        if let Err(e) = self
            .run_localhost_integrations(ops, created_subnets, cancel)
            .await
        {
            tracing::error!(error = %e, "Localhost integration phase failed, continuing");
        }

        ops.report_progress(100).await?;

        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Phase 2: Network Scan (5-100%)
        session.set_progress_range(5, 100);

        let network_hosts = self.run_network_phase(cancel).await?;

        // Completion banner
        self.log_completion_banner(&network_hosts, start.elapsed());

        Ok(())
    }

    /// Run integrations for localhost credentials (e.g., Docker on daemon host).
    /// Uses the same dispatch as network scanning — localhost credentials aren't special,
    /// they just target a known host_id instead of ARP-discovered hosts.
    async fn run_localhost_integrations(
        &self,
        ops: &DiscoveryOps,
        created_subnets: &[Subnet],
        cancel: &CancellationToken,
    ) -> Result<(), Error> {
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
            return Ok(());
        }

        tracing::info!(
            mappings = localhost_mappings.len(),
            "Running localhost integrations"
        );

        // Probe with 127.0.0.1 — credentials are keyed to localhost, not the daemon's real IP.
        // The daemon's real IP is used for subnet/interface matching below.
        let localhost_ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let probe_results = dispatch::probe_integrations(
            localhost_ip,
            &localhost_mappings,
            &[],  // No port scan for localhost — integrations do their own probing
            true, // skip the probe-gate: daemon-host integrations (e.g. a proxy) always self-probe here
            cancel,
            &self.service.utils,
            ops.config_store.get_accept_invalid_scan_certs().await?,
        )
        .await?;

        // Deliver before the early return below. These were built correctly and then dropped,
        // so a wrong Docker or Podman socket credential on the daemon host reported nothing at
        // all — and the early return is precisely the path a failing credential takes.
        ops.record_credential_issues(&probe_results.credential_issues)
            .await;

        if probe_results.client_responses.is_empty() {
            tracing::debug!("No localhost integration probes succeeded");
            return Ok(());
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
            return Ok(());
        };

        let ip_address = IPAddress::new(crate::server::ip_addresses::r#impl::base::IPAddressBase {
            network_id: subnet.base.network_id,
            host_id: Uuid::nil(),
            name: None,
            subnet_id: subnet.id,
            ip_address: host_ip,
            mac_address: None,
            position: 0,
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
        };

        let mut host_data = match ops
            .build_host_from_scan(params, None, self.host_naming_fallback)
            .await?
        {
            Some(hd) => hd,
            None => {
                tracing::warn!("Localhost service matching returned no host");
                return Ok(());
            }
        };

        host_data.host.id = self.host_id;

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
            scanning_subnet: None,
            ip_address_id: Some(ip_address.id),
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
            cancel,
        )
        .await?;

        Ok(())
    }

    /// Network phase: run ARP + deep scan to discover hosts and services
    async fn run_network_phase(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<(IpAddr, Host, super::network::DiscoveredHostData)>, Error> {
        // Network discovery owns subnet resolution — unified just coordinates
        let network_discovery = super::network::NetworkScan::new(
            self.subnet_ids.clone(),
            self.host_naming_fallback,
            self.scan_settings.clone(),
            self.credential_mappings.clone(),
            self.target_ips.clone(),
            self.extra_ports.clone(),
        );

        let ops = super::ops::DiscoveryOps::new(&self.service, DiscoveryType::from(self));
        let utils = &self.service.utils;

        let resolved = network_discovery
            .resolve_scan_subnets(&ops, utils, cancel)
            .await?;

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
                resolved.network_subnets,
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
