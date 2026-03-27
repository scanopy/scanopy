use crate::{
    daemon::discovery::service::base::{
        CreatesDiscoveredEntities, DiscoversNetworkedEntities, DiscoveryRunner, RunsDiscovery,
    },
    server::{
        bindings::r#impl::base::Binding,
        daemons::r#impl::api::DaemonDiscoveryRequest,
        discovery::r#impl::types::DiscoveryType,
        interfaces::r#impl::base::{ALL_INTERFACES_IP, Interface},
        ports::r#impl::base::{Port, PortType},
        services::{
            definitions::scanopy_daemon::ScanopyDaemon,
            r#impl::{base::ServiceBase, definitions::ServiceDefinition, patterns::MatchDetails},
        },
        shared::{
            storage::traits::Storable,
            types::entities::{DiscoveryMetadata, EntitySource},
        },
        subnets::r#impl::base::Subnet,
    },
};
use crate::{
    daemon::utils::base::{DaemonUtils, merge_host_and_docker_subnets},
    server::{
        hosts::r#impl::base::{Host, HostBase},
        services::r#impl::base::Service,
    },
};
use anyhow::{Error, Result};
use async_trait::async_trait;
use futures::future::join_all;
use std::net::{IpAddr, Ipv4Addr};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Default)]
pub struct SelfReportDiscovery {
    host_id: Uuid,
}

impl SelfReportDiscovery {
    pub fn new(host_id: Uuid) -> Self {
        Self { host_id }
    }
}

impl CreatesDiscoveredEntities for DiscoveryRunner<SelfReportDiscovery> {}

#[async_trait]
impl DiscoversNetworkedEntities for DiscoveryRunner<SelfReportDiscovery> {
    async fn get_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        // SelfReport doesn't need gateway IPs for service matching
        Ok(Vec::new())
    }

    async fn discover_create_subnets(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error> {
        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let utils = &self.as_ref().utils;

        let interface_start = std::time::Instant::now();
        let interface_filter = self.as_ref().config_store.get_interfaces().await?;
        let (_, subnets, _) = utils
            .get_own_interfaces(
                self.discovery_type(),
                daemon_id,
                network_id,
                &interface_filter,
            )
            .await?;
        tracing::debug!(
            elapsed_ms = interface_start.elapsed().as_millis(),
            subnet_count = subnets.len(),
            "Network subnets gathered for self-report"
        );

        // Get docker subnets so we can merge them with host subnets correctly
        // Host subnets take precedence — Docker subnets with matching CIDRs are dropped
        let docker_proxy = self.as_ref().config_store.get_docker_proxy().await;
        let docker_proxy_ssl_info = self.as_ref().config_store.get_docker_proxy_ssl_info().await;

        let docker_client = self
            .as_ref()
            .utils
            .new_docker_client(docker_proxy, docker_proxy_ssl_info)
            .await;

        let docker_subnets = if let Ok(docker_client) = docker_client {
            tracing::debug!("Docker client available, fetching Docker networks");
            match self
                .as_ref()
                .utils
                .get_subnets_from_docker_networks(
                    daemon_id,
                    network_id,
                    &docker_client,
                    self.discovery_type(),
                )
                .await
            {
                Ok(docker_subnets) => {
                    tracing::debug!(
                        docker_subnet_count = docker_subnets.len(),
                        cidrs = ?docker_subnets.iter().map(|s| s.base.cidr.to_string()).collect::<Vec<_>>(),
                        "Docker subnets detected"
                    );
                    docker_subnets
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to get Docker networks - proceeding without Docker subnet filtering"
                    );
                    Vec::new()
                }
            }
        } else {
            tracing::debug!("Docker socket not available - skipping Docker subnet detection");
            Vec::new()
        };

        // Merge host and Docker subnets — host subnets always win on CIDR overlap
        let merged = merge_host_and_docker_subnets(subnets, docker_subnets);

        // Filter out DockerBridge subnets — those are handled by Docker discovery
        let subnets_to_create: Vec<Subnet> = merged
            .into_iter()
            .filter(|s| !s.is_docker_bridge_subnet())
            .collect();

        tracing::info!(
            subnet_count = subnets_to_create.len(),
            cidrs = ?subnets_to_create.iter().map(|s| s.base.cidr.to_string()).collect::<Vec<_>>(),
            "Creating subnets from discovered interfaces"
        );

        // Create subnets individually, collecting successes and logging failures
        // This prevents one subnet failure from blocking all interfaces
        let subnet_futures = subnets_to_create.iter().map(|subnet| async move {
            let cidr = subnet.base.cidr;
            match self.create_subnet(subnet, cancel).await {
                Ok(created) => {
                    tracing::debug!(
                        cidr = %cidr,
                        subnet_id = %created.id,
                        "Subnet created successfully"
                    );
                    Some(created)
                }
                Err(e) => {
                    tracing::warn!(
                        cidr = %cidr,
                        error = %e,
                        "Failed to create subnet - interfaces in this CIDR will be skipped"
                    );
                    None
                }
            }
        });
        let created_subnets: Vec<Subnet> = join_all(subnet_futures)
            .await
            .into_iter()
            .flatten()
            .collect();

        tracing::debug!(
            created_count = created_subnets.len(),
            requested_count = subnets_to_create.len(),
            "Subnet creation complete"
        );

        Ok(created_subnets)
    }
}

#[async_trait]
impl RunsDiscovery for DiscoveryRunner<SelfReportDiscovery> {
    fn discovery_type(&self) -> DiscoveryType {
        DiscoveryType::SelfReport {
            host_id: self.domain.host_id,
        }
    }

    async fn discover(
        &self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        // Create subnets first (before session initialization, like Network discovery)
        let created_subnets = match self.discover_create_subnets(&cancel).await {
            Ok(subnets) => subnets,
            Err(e) => {
                // Pre-start failure: initialize a minimal session so we can report the error
                let daemon_id = self.as_ref().config_store.get_id().await?;
                if let Err(init_err) = self.initialize_discovery_session(request, daemon_id).await {
                    tracing::error!(
                        "Failed to initialize session for error reporting: {}",
                        init_err
                    );
                    return Err(e);
                }
                self.finish_discovery(Err(e), cancel).await?;
                return Ok(());
            }
        };

        // Initialize session and report Started phase
        self.start_discovery(request).await?;

        // Run the actual discovery work, capturing any errors
        let discovery_result = self
            .run_self_report_discovery(&created_subnets, cancel.clone())
            .await;

        // Report completion/failure and clear session
        self.finish_discovery(discovery_result, cancel).await?;

        Ok(())
    }
}

impl DiscoveryRunner<SelfReportDiscovery> {
    /// Core self-report discovery logic, separated for proper error handling with finish_discovery
    async fn run_self_report_discovery(
        &self,
        created_subnets: &[Subnet],
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        // Check cancellation early
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let utils = &self.as_ref().utils;
        let host_id = self.domain.host_id;

        let binding_address = self.as_ref().config_store.get_bind_address().await?;
        let binding_ip = IpAddr::V4(binding_address.parse::<Ipv4Addr>()?);

        // Re-fetch interfaces (subnets were already created in discover_create_subnets)
        let interface_filter = self.as_ref().config_store.get_interfaces().await?;
        let (interfaces, _, _) = utils
            .get_own_interfaces(
                self.discovery_type(),
                daemon_id,
                network_id,
                &interface_filter,
            )
            .await?;
        tracing::debug!(
            interface_count = interfaces.len(),
            "Network interfaces gathered for host creation"
        );

        // Capabilities are now updated via process_status on every poll — no separate POST needed.

        // Check cancellation
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Filter interfaces to only those with matching created subnets
        // Update subnet_id references since created subnets may differ from discovered
        let original_interface_count = interfaces.len();
        let interfaces: Vec<Interface> = interfaces
            .into_iter()
            .filter_map(|mut i| {
                if let Some(subnet) = created_subnets
                    .iter()
                    .find(|s| s.base.cidr.contains(&i.base.ip_address))
                {
                    i.base.subnet_id = subnet.id;
                    return Some(i);
                }
                tracing::warn!(
                    interface_name = ?i.base.name,
                    ip_address = %i.base.ip_address,
                    "Dropping interface - no matching subnet was created (subnet creation may have failed)"
                );
                None
            })
            .collect();

        if interfaces.len() < original_interface_count {
            tracing::warn!(
                original_count = original_interface_count,
                kept_count = interfaces.len(),
                dropped_count = original_interface_count - interfaces.len(),
                "Some interfaces were dropped due to missing subnets"
            );
        } else {
            tracing::debug!(
                interface_count = interfaces.len(),
                "All interfaces have matching subnets"
            );
        }

        let daemon_bound_subnet_ids: Vec<Uuid> = if binding_address == ALL_INTERFACES_IP.to_string()
        {
            created_subnets.iter().map(|s| s.id).collect()
        } else {
            created_subnets
                .iter()
                .filter(|s| s.base.cidr.contains(&binding_ip))
                .map(|s| s.id)
                .collect()
        };

        let own_port = Port::new_hostless(PortType::new_tcp(
            self.as_ref().config_store.get_port().await?,
        ));
        let own_port_id = own_port.id;
        let local_ip = utils.get_own_ip_address()?;
        let hostname = utils.get_own_hostname();

        // Create host base - children (interfaces, ports, services) are passed separately
        let host_base = HostBase {
            name: hostname.clone().unwrap_or(format!("{}", local_ip)),
            hostname,
            network_id,
            description: Some("Scanopy daemon".to_string()),
            tags: Vec::new(),
            source: EntitySource::Discovery {
                metadata: vec![DiscoveryMetadata::new(self.discovery_type(), daemon_id)],
            },
            hidden: false,
            virtualization: None,
            // SNMP fields - not applicable to self-report
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
            credential_assignments: vec![],
        };

        // Ports to create with the host
        let ports = vec![own_port];

        let mut host = Host::new(host_base);
        host.id = host_id;

        let mut services = Vec::new();
        let daemon_service_definition = ScanopyDaemon;

        let daemon_service_bound_interfaces: Vec<&Interface> = interfaces
            .iter()
            .filter(|i| daemon_bound_subnet_ids.contains(&i.base.subnet_id))
            .collect();

        let daemon_service = Service::new(ServiceBase {
            name: ServiceDefinition::name(&daemon_service_definition).to_string(),
            service_definition: Box::new(daemon_service_definition),
            tags: Vec::new(),
            network_id,
            bindings: daemon_service_bound_interfaces
                .iter()
                .map(|i| Binding::new_port_serviceless(own_port_id, Some(i.id)))
                .collect(),
            host_id: host.id,
            virtualization: None,
            source: EntitySource::DiscoveryWithMatch {
                metadata: vec![DiscoveryMetadata::new(self.discovery_type(), daemon_id)],
                details: MatchDetails::new_certain("Scanopy Daemon self-report"),
            },
            position: 0,
        });

        services.push(daemon_service);

        tracing::debug!(
            "Collected information about own host with local IP: {}, Hostname: {:?}",
            local_ip,
            host.base.hostname
        );

        // Check cancellation before creating host
        if cancel.is_cancelled() {
            return Err(anyhow::anyhow!("Discovery cancelled"));
        }

        // Pass interfaces and ports separately - server will create them with the correct host_id
        tracing::debug!("Creating host with interfaces, ports, and services");
        self.create_host(host, interfaces.clone(), ports, services, vec![], &cancel)
            .await?;

        Ok(())
    }
}
