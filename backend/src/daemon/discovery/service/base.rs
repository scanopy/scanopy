use std::{
    net::IpAddr,
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering},
    },
    time::Duration,
};

use crate::server::shared::types::api::ApiErrorResponse;
use crate::{
    daemon::{
        discovery::{
            buffer::EntityBuffer, manager::DaemonDiscoverySessionManager,
            types::base::DiscoveryCriticalError,
        },
        shared::api_client::DaemonApiClient,
    },
    server::{
        discovery::r#impl::types::{DiscoveryType, HostNamingFallback},
        services::{
            definitions::{docker_container::DockerContainer, open_ports::OpenPorts},
            r#impl::{
                base::{
                    DiscoverySessionServiceMatchParams, ServiceMatchBaselineParams,
                    ServiceMatchServiceParams,
                },
                patterns::MatchConfidence,
                virtualization::{DockerVirtualization, ServiceVirtualization},
            },
        },
        shared::types::entities::{DiscoveryMetadata, EntitySource},
    },
};
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use chrono::Utc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use uuid::Uuid;

use crate::{
    daemon::{
        discovery::types::base::{DiscoveryPhase, DiscoverySessionInfo, DiscoverySessionUpdate},
        shared::config::ConfigStore,
        utils::base::{PlatformDaemonUtils, create_system_utils},
    },
    server::{
        daemons::r#impl::{
            api::{DaemonDiscoveryRequest, DiscoveryUpdatePayload},
            base::DaemonMode,
        },
        hosts::r#impl::{
            api::{DiscoveryHostRequest, HostResponse},
            base::{Host, HostBase},
        },
        if_entries::r#impl::base::IfEntry,
        interfaces::r#impl::base::Interface,
        ports::r#impl::base::{Port, PortType},
        services::{
            definitions::{ServiceDefinitionRegistry, gateway::Gateway},
            r#impl::{
                base::Service,
                definitions::{ServiceDefinition, ServiceDefinitionExt},
            },
        },
        shared::types::metadata::HasId,
        subnets::r#impl::base::Subnet,
    },
};

pub struct DiscoveryRunner<T> {
    pub service: Arc<DaemonDiscoveryService>,
    pub manager: Arc<DaemonDiscoverySessionManager>,
    pub domain: T,
}

impl<T> DiscoveryRunner<T> {
    pub fn new(
        service: Arc<DaemonDiscoveryService>,
        manager: Arc<DaemonDiscoverySessionManager>,
        domain: T,
    ) -> Self {
        Self {
            service,
            manager,
            domain,
        }
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
        }
    }

    pub fn set_progress_range(&self, start: u8, end: u8) {
        self.progress_range_start.store(start, Ordering::Relaxed);
        self.progress_range_end.store(end, Ordering::Relaxed);
    }
}

fn map_progress(raw: u8, start: u8, end: u8) -> u8 {
    if start == 0 && end == 100 {
        return raw;
    }
    start + (raw as f64 * (end - start) as f64 / 100.0) as u8
}

impl<T> AsRef<DaemonDiscoveryService> for DiscoveryRunner<T> {
    fn as_ref(&self) -> &DaemonDiscoveryService {
        &self.service
    }
}

pub struct DaemonDiscoveryService {
    pub config_store: Arc<ConfigStore>,
    pub api_client: Arc<DaemonApiClient>,
    pub utils: PlatformDaemonUtils,
    pub current_session: Arc<RwLock<Option<DiscoverySession>>>,
    pub entity_buffer: Arc<EntityBuffer>,
    /// Stores the terminal state (Complete/Failed/Cancelled) for ServerPoll mode.
    /// In ServerPoll mode, the server polls for progress updates. If the session ends
    /// between polls, we need to retain the terminal state so the server can receive it.
    /// This is cleared when a new session starts.
    pub terminal_payload: Arc<RwLock<Option<DiscoveryUpdatePayload>>>,
}

impl DaemonDiscoveryService {
    pub fn new(config_store: Arc<ConfigStore>, entity_buffer: Arc<EntityBuffer>) -> Self {
        Self {
            api_client: Arc::new(DaemonApiClient::new(config_store.clone())),
            config_store,
            utils: create_system_utils(),
            current_session: Arc::new(RwLock::new(None)),
            entity_buffer,
            terminal_payload: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_session(&self) -> Result<DiscoverySession, Error> {
        self.current_session
            .read()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("No active discovery session"))
    }
}

#[async_trait]
pub trait RunsDiscovery: AsRef<DaemonDiscoveryService> + Send + Sync {
    fn discovery_type(&self) -> DiscoveryType;

    async fn discover(
        &self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error>;

    /// Report scanning progress with automatic time-based throttling.
    /// Reports if progress has changed OR at least 30 seconds have passed (heartbeat).
    /// Percent should be 0-100.
    async fn report_scanning_progress(&self, percent: u8) -> Result<(), Error> {
        let session = self.as_ref().get_session().await?;
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

        // Heartbeat interval - report even if progress hasn't changed
        let heartbeat_interval_secs = 30;
        let heartbeat_due = now >= last_time + heartbeat_interval_secs;

        // Skip if: progress hasn't changed AND heartbeat not due AND not 100%
        if !progress_changed && !heartbeat_due && percent < 100 {
            return Ok(());
        }

        // Throttle rapid updates (min 10 seconds between reports, unless 100%)
        if percent < 100 && !heartbeat_due && now < last_time + 10 {
            return Ok(());
        }

        // Try to claim this report slot
        if last_report_time
            .compare_exchange(last_time, now, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            return Ok(());
        }

        // Update last progress
        last_progress.store(percent, Ordering::Relaxed);

        self.report_discovery_update(DiscoverySessionUpdate::scanning(percent))
            .await
    }

    async fn report_discovery_update(&self, update: DiscoverySessionUpdate) -> Result<(), Error> {
        let session = self.as_ref().get_session().await?;
        let discovery_type = self.discovery_type();

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
            discovery_type,
            session.info.clone(),
            update,
        );

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
            .as_ref()
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
}

#[async_trait]
pub trait DiscoversNetworkedEntities:
    AsRef<DaemonDiscoveryService> + Send + Sync + RunsDiscovery
{
    async fn get_gateway_ips(&self) -> Result<Vec<IpAddr>, Error>;

    async fn discover_create_subnets(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error>;

    async fn initialize_discovery_session(
        &self,
        request: DaemonDiscoveryRequest,
        daemon_id: Uuid,
    ) -> Result<(), Error> {
        tracing::debug!(
            "Setting session info for {} discovery session {}",
            request.discovery_type,
            request.session_id
        );
        let gateway_ips = self.get_gateway_ips().await?;
        let network_id = self
            .as_ref()
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
        };

        let session = DiscoverySession::new(session_info, gateway_ips);

        // Clear terminal payload from previous session (if any) when starting new session
        let mut terminal_payload = self.as_ref().terminal_payload.write().await;
        *terminal_payload = None;
        drop(terminal_payload);

        let mut current_session = self.as_ref().current_session.write().await;
        *current_session = Some(session);

        Ok(())
    }

    async fn start_discovery(&self, request: DaemonDiscoveryRequest) -> Result<(), Error> {
        let daemon_id = self.as_ref().config_store.get_id().await?;

        tracing::info!(
            "Starting {} discovery session {}",
            request.discovery_type,
            request.session_id
        );

        self.initialize_discovery_session(request, daemon_id)
            .await?;

        self.report_discovery_update(DiscoverySessionUpdate {
            phase: DiscoveryPhase::Started,
            progress: 0,
            error: None,
            finished_at: None,
        })
        .await?;

        let session = self.as_ref().get_session().await?;

        tracing::info!(
            session_id = %session.info.session_id,
            discovery_type = ?self.discovery_type(),
            "Discovery session started"
        );

        Ok(())
    }

    async fn finish_discovery(
        &self,
        discovery_result: Result<(), Error>,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        let session = self.as_ref().get_session().await?;
        let session_id = session.info.session_id;

        let final_progress = session
            .last_progress
            .load(std::sync::atomic::Ordering::Relaxed);

        // Build the terminal update based on result
        let terminal_update = match &discovery_result {
            Ok(_) => {
                tracing::info!(
                    session_id = %session_id,
                    progress = 100,
                    "Discovery session completed successfully"
                );
                DiscoverySessionUpdate {
                    phase: DiscoveryPhase::Complete,
                    progress: 100,
                    error: None,
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
                    finished_at: Some(Utc::now()),
                }
            }
        };

        // Report the terminal update (in DaemonPoll mode, this POSTs to server)
        self.report_discovery_update(terminal_update.clone())
            .await?;

        // Store terminal payload for ServerPoll mode - the server polls for progress
        // and needs to receive the terminal state even after current_session is cleared.
        // This payload persists until a new session starts.
        let terminal_payload = DiscoveryUpdatePayload::from_state_and_update(
            self.discovery_type(),
            session.info.clone(),
            terminal_update,
        );
        let mut stored_terminal = self.as_ref().terminal_payload.write().await;
        *stored_terminal = Some(terminal_payload);
        drop(stored_terminal);

        // Clear entity buffer - all await_*() calls have completed by now
        // (either successfully found Created entries or timed out)
        self.as_ref().entity_buffer.clear_all().await;

        let mut current_session = self.as_ref().current_session.write().await;
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

    async fn process_host<'a>(
        &self,
        params: ServiceMatchBaselineParams<'a>,
        hostname: Option<String>,
        host_naming_fallback: HostNamingFallback,
    ) -> Result<Option<(Host, Vec<Interface>, Vec<Port>, Vec<Service>)>, Error> {
        let ServiceMatchBaselineParams::<'a> { interface, .. } = params;

        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let session = self.as_ref().get_session().await?;
        let gateway_ips = session.gateway_ips.clone();
        let discovery_type = self.discovery_type();

        // Create host - children (interfaces, ports, services) are passed separately
        let mut host = Host::new(HostBase {
            name: "Unknown Device".to_string(),
            hostname: hostname.clone(),
            tags: Vec::new(),
            network_id,
            description: None,
            source: EntitySource::Discovery {
                metadata: vec![DiscoveryMetadata::new(discovery_type.clone(), daemon_id)],
            },
            virtualization: None,
            hidden: false,
            // SNMP fields - populated by snmp.rs when SNMP is enabled
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
        });

        // Store interfaces separately to pass to server
        let interfaces = vec![interface.clone()];

        let (services, ports) = self.discover_services(
            &host,
            &params,
            &gateway_ips,
            &daemon_id,
            &network_id,
            &discovery_type,
        )?;

        // Determine host's name
        let best_service_name = services
            .iter()
            .find(|s| !ServiceDefinitionExt::is_generic(&s.base.service_definition))
            .map(|s| s.base.service_definition.name().to_string());

        if let Some(hostname) = hostname {
            host.base.name = hostname;
        } else if host_naming_fallback == HostNamingFallback::BestService
            && let Some(best_service_name) = best_service_name
        {
            host.base.name = best_service_name
        } else if host_naming_fallback == HostNamingFallback::Ip {
            host.base.name = interface.base.ip_address.to_string()
        } else if let Some(best_service_name) = best_service_name {
            host.base.name = best_service_name
        } else {
            host.base.name = interface.base.ip_address.to_string()
        }

        tracing::info!(
            ip = %interface.base.ip_address,
            host_name = %host.base.name,
            service_count = %services.len(),
            port_count = %ports.len(),
            "Processed host",
        );
        Ok(Some((host, interfaces, ports, services)))
    }

    fn discover_services(
        &self,
        host: &Host,
        baseline_params: &ServiceMatchBaselineParams,
        gateway_ips: &[IpAddr],
        daemon_id: &Uuid,
        network_id: &Uuid,
        discovery_type: &DiscoveryType,
    ) -> Result<(Vec<Service>, Vec<Port>), Error> {
        let ServiceMatchBaselineParams { all_ports, .. } = baseline_params;

        let mut services = Vec::new();
        let mut host_ports = Vec::new();

        // Track which ports are bound vs open for services to bind to
        let mut unbound_ports = all_ports.to_vec();

        let mut container_matched = false;

        let mut sorted_service_definitions: Vec<Box<dyn ServiceDefinition>> =
            ServiceDefinitionRegistry::all_service_definitions()
                .into_iter()
                .collect();

        sorted_service_definitions.sort_by_key(|s| {
            if !ServiceDefinitionExt::is_generic(s) {
                0 // Highest priority - non-generic services
            } else if s.id() == OpenPorts.id() {
                // Catch-all for open ports, should be dead last
                3
            } else if s.id() == DockerContainer.id() || s.id() == Gateway.id() {
                // Docker Containers and Gateways need to go second to last last
                // Other generic services should be able to get matched first
                2
            } else {
                // Generic services that aren't Docker Container or Gateway
                1
            }
        });

        // Add services from detected ports
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
                    discovery_type,
                    network_id,
                    gateway_ips,
                    host_id: &host.id,
                };

            if let Some((service, mut ports, _endpoint)) = Service::from_discovery(params)
                && !container_matched
            {
                // If a container was matched w the provided virtualization, no others can be matched
                if let Some(ServiceVirtualization::Docker(DockerVirtualization {
                    container_id: Some(_),
                    ..
                })) = &service.base.virtualization
                {
                    container_matched = true
                }

                // Add any bound ports to host ports array, remove from open ports
                let bound_port_types: Vec<PortType> =
                    ports.iter().map(|p| p.base.port_type).collect();

                host_ports.append(&mut ports);

                // Add new service
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

        // Add unbound ports as hostless ports
        host_ports.extend(unbound_ports.into_iter().map(Port::new_hostless));

        Ok((services, host_ports))
    }
}

/// Default number of retries for entity creation during discovery.
/// This handles transient failures during server switchovers (blue-green deployments).
const ENTITY_CREATION_MAX_RETRIES: usize = 5;

/// Timeout for waiting for server confirmation in ServerPoll mode.
/// Must be > 2x poll interval (30s) to handle worst-case timing.
const SERVER_POLL_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(120);

#[async_trait]
pub trait CreatesDiscoveredEntities:
    AsRef<DaemonDiscoveryService> + Send + Sync + RunsDiscovery
{
    /// Create a host with its children (interfaces, ports, services).
    /// Pass an empty `if_entries` vec if SNMP data is not available (e.g., Docker discovery).
    /// In DaemonPoll mode: Immediately sends to server and returns the response.
    /// In ServerPoll mode: Buffers the host for server to poll, waits for confirmation.
    async fn create_host(
        &self,
        host: Host,
        interfaces: Vec<Interface>,
        ports: Vec<Port>,
        services: Vec<Service>,
        if_entries: Vec<IfEntry>,
        cancel: &CancellationToken,
    ) -> Result<HostResponse, Error> {
        let service = self.as_ref();
        let mode = service.config_store.get_mode().await?;
        let pending_id = host.id;

        let request = DiscoveryHostRequest {
            host,
            interfaces,
            ports,
            services,
            if_entries,
        };

        // Always buffer first (for both modes)
        service.entity_buffer.push_host(request.clone()).await;

        match mode {
            DaemonMode::DaemonPoll => {
                // Immediately send to server, get response (with retry on transient failures)
                let api_client = &service.api_client;
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
                .when(|e| {
                    // Don't retry structured API errors (e.g. billing limit reached)
                    e.downcast_ref::<ApiErrorResponse>().is_none()
                })
                .notify(|e, dur| tracing::warn!("Retrying host creation after {:?}: {}", dur, e))
                .await?;

                // Mark as created in buffer with actual server data
                service
                    .entity_buffer
                    .mark_host_created(pending_id, response.clone())
                    .await;

                Ok(response)
            }
            DaemonMode::ServerPoll => {
                // Wait for server to poll and confirm creation
                let actual_host = service
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

                // Build a HostResponse from the confirmed host
                // Note: In ServerPoll mode, we don't have hydrated children back
                // but the Host ID is what matters for subsequent operations
                Ok(HostResponse::from_host_with_children(
                    actual_host,
                    request.interfaces,
                    request.ports,
                    request.services,
                    request.if_entries,
                ))
            }
        }
    }

    /// Create a subnet.
    ///
    /// In DaemonPoll mode: Immediately sends to server and returns the response.
    /// In ServerPoll mode: Buffers the subnet for server to poll, waits for confirmation.
    async fn create_subnet(
        &self,
        subnet: &Subnet,
        cancel: &CancellationToken,
    ) -> Result<Subnet, Error> {
        let service = self.as_ref();
        let mode = service.config_store.get_mode().await?;
        let pending_id = subnet.id;

        // Always buffer first (for both modes)
        service.entity_buffer.push_subnet(subnet.clone()).await;

        match mode {
            DaemonMode::DaemonPoll => {
                // Immediately send to server, get response (with retry on transient failures)
                let api_client = &service.api_client;
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

                // Mark as created in buffer with actual server data
                service
                    .entity_buffer
                    .mark_subnet_created(pending_id, actual.clone())
                    .await;

                Ok(actual)
            }
            DaemonMode::ServerPoll => {
                // Wait for server to poll and confirm creation
                service
                    .entity_buffer
                    .await_subnet(&pending_id, SERVER_POLL_CONFIRMATION_TIMEOUT, cancel)
                    .await
                    .ok_or_else(|| {
                        if cancel.is_cancelled() {
                            anyhow!("Discovery cancelled while waiting for subnet creation")
                        } else {
                            anyhow!("Timeout waiting for subnet creation confirmation from server")
                        }
                    })
            }
        }
    }
}
