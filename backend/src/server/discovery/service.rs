use crate::bail_validation;
use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::daemon::runtime::service::LOG_TARGET;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::credentials::service::CredentialService;
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::daemons::service::DaemonService;
use crate::server::discovery::r#impl::base::Discovery;
use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
use crate::server::networks::service::NetworkService;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants};
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::types::{EntityEvent, EntityOperation};
use crate::server::shared::events::types::{OnboardingEvent, OnboardingOperation};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::traits::{Entity, Storable, Storage};
use crate::server::shared::types::api::ApiError;
use crate::server::tags::entity_tags::EntityTagService;
use anyhow::anyhow;
use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::Utc;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use tokio::sync::{RwLock, broadcast};
use tokio_cron_scheduler::{JobBuilder, JobScheduler};
use uuid::Uuid;

/// Server-side session management for discovery
pub struct DiscoveryService {
    self_ref: Weak<Self>,
    discovery_storage: Arc<GenericPostgresStorage<Discovery>>,
    sessions: RwLock<HashMap<Uuid, DiscoveryUpdatePayload>>, // session_id -> session state mapping
    daemon_sessions: RwLock<HashMap<Uuid, Vec<Uuid>>>,       // daemon_id -> session_id mapping
    discovery_sessions: RwLock<HashMap<Uuid, Uuid>>, // discovery_id -> session_id mapping (enforces one active session per discovery)
    daemon_pull_cancellations: RwLock<HashMap<Uuid, (bool, Uuid)>>, // daemon_id -> (boolean, session_id) mapping for pull mode cancellations of current session on daemon
    session_last_updated: RwLock<HashMap<Uuid, chrono::DateTime<Utc>>>,
    update_tx: broadcast::Sender<DiscoveryUpdatePayload>,
    scheduler: Option<Arc<JobScheduler>>,
    job_ids: RwLock<HashMap<Uuid, Uuid>>, // discovery_id -> scheduler job_id mapping
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    credential_service: Arc<CredentialService>,
    network_service: Arc<NetworkService>,
    organization_service: Arc<OrganizationService>,
    // Lazy dependency (set after construction to break circular dependency)
    daemon_service: std::sync::OnceLock<Arc<DaemonService>>,
}

impl EventBusService<Discovery> for DiscoveryService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Discovery) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Discovery) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Discovery> for DiscoveryService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Discovery>> {
        &self.discovery_storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn update(
        &self,
        entity: &mut Discovery,
        authentication: AuthenticatedEntity,
    ) -> Result<Discovery, anyhow::Error> {
        Self::validate_timezone(&entity.base.run_type)?;

        let current = self
            .get_by_id(&entity.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not find discovery {}", entity))?;

        // Preserve server-managed fields from current DB state
        // (the API client may send stale values for these read-only fields)
        entity.scan_count = current.scan_count;

        // If it's a scheduled discovery and schedule or timezone has changed, need to reschedule
        let schedule_changed = if let RunType::Scheduled {
            cron_schedule: new_cron,
            timezone: new_tz,
            ..
        } = &entity.base.run_type
            && let RunType::Scheduled {
                cron_schedule: current_cron,
                timezone: current_tz,
                ..
            } = &current.base.run_type
        {
            current_cron != new_cron || current_tz != new_tz
        } else {
            false
        };

        // Detect enabled state transitions (disabled→enabled or enabled→disabled)
        let enabled_changed = if let RunType::Scheduled {
            enabled: new_enabled,
            ..
        } = &entity.base.run_type
            && let RunType::Scheduled {
                enabled: current_enabled,
                ..
            } = &current.base.run_type
        {
            current_enabled != new_enabled
        } else {
            false
        };

        let needs_reschedule = schedule_changed || enabled_changed;

        let updated = if needs_reschedule
            && matches!(entity.base.run_type, RunType::Scheduled { .. })
        {
            tracing::debug!(
                discovery_id = %entity.id,
                "Rescheduling discovery (schedule_changed={}, enabled_changed={})",
                schedule_changed, enabled_changed
            );

            // Remove old schedule first
            self.remove_scheduled_job(&entity.id).await;

            // Update in DB
            let mut updated = self.discovery_storage.update(entity).await?;

            // Re-add cron job (schedule_discovery guards on !enabled, so disabling skips re-add)
            if let Some(arc_self) = self.self_ref.upgrade()
                && let Err(e) = Self::schedule_discovery(&arc_self, &updated).await
            {
                // Only disable if we were trying to enable/reschedule (not if already disabling)
                if matches!(
                    updated.base.run_type,
                    RunType::Scheduled { enabled: true, .. }
                ) {
                    updated.disable();
                    let disabled_discovery = self.discovery_storage.update(&mut updated).await?;

                    tracing::error!(
                        "Failed to reschedule discovery {}. Discovery updated but disabled. Error: {}",
                        disabled_discovery.id,
                        e
                    );
                }
            }

            updated
        } else {
            // For non-scheduled or no reschedule needed, just update
            self.discovery_storage.update(entity).await?
        };

        // Update tags in junction table
        if let Some(entity_tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
            && let Some(tags) = updated.get_tags()
        {
            entity_tag_service
                .set_tags(
                    updated.id(),
                    EntityDiscriminants::Discovery,
                    tags.clone(),
                    org_id,
                )
                .await?;
        }

        let trigger_stale = updated.triggers_staleness(Some(current));
        let suppress_logs = self.suppress_logs(None, None);

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: updated.id(),
                network_id: self.get_network_id(&updated),
                organization_id: self.get_organization_id(&updated),
                entity_type: updated.clone().into(),
                operation: EntityOperation::Updated,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale,
                    "suppress_logs": suppress_logs
                }),
                authentication,
            })
            .await?;

        Ok(updated)
    }
}

impl DiscoveryService {
    pub async fn new(
        discovery_storage: Arc<GenericPostgresStorage<Discovery>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        credential_service: Arc<CredentialService>,
        network_service: Arc<NetworkService>,
        organization_service: Arc<OrganizationService>,
    ) -> Result<Arc<Self>> {
        let (tx, _rx) = broadcast::channel(100); // Buffer 100 messages
        let scheduler = JobScheduler::new().await?;
        let scheduler = Some(Arc::new(scheduler));

        Ok(Arc::new_cyclic(|weak| Self {
            self_ref: weak.clone(),
            discovery_storage,
            sessions: RwLock::new(HashMap::new()),
            daemon_sessions: RwLock::new(HashMap::new()),
            discovery_sessions: RwLock::new(HashMap::new()),
            daemon_pull_cancellations: RwLock::new(HashMap::new()),
            session_last_updated: RwLock::new(HashMap::new()),
            update_tx: tx,
            scheduler,
            job_ids: RwLock::new(HashMap::new()),
            event_bus,
            entity_tag_service,
            credential_service,
            network_service,
            organization_service,
            daemon_service: std::sync::OnceLock::new(),
        }))
    }

    /// Set the daemon service dependency after construction.
    /// This breaks the circular dependency: DaemonService holds Arc<DiscoveryService>,
    /// and DiscoveryService holds OnceLock<Arc<DaemonService>>.
    pub fn set_daemon_service(
        &self,
        service: Arc<DaemonService>,
    ) -> Result<(), Arc<DaemonService>> {
        self.daemon_service.set(service)
    }

    /// Expose stream to handler
    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveryUpdatePayload> {
        self.update_tx.subscribe()
    }

    /// Get session state
    pub async fn get_session(&self, session_id: &Uuid) -> Option<DiscoveryUpdatePayload> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Get session state
    pub async fn get_all_sessions(&self, network_ids: &[Uuid]) -> Vec<DiscoveryUpdatePayload> {
        let all_sessions = self.sessions.read().await;
        all_sessions
            .values()
            .filter(|v| network_ids.contains(&v.network_id))
            .cloned()
            .collect()
    }

    pub async fn get_sessions_for_daemon(&self, daemon_id: &Uuid) -> Vec<DiscoveryUpdatePayload> {
        let daemon_session_ids = self.daemon_sessions.read().await;
        let session_ids = daemon_session_ids
            .get(daemon_id)
            .cloned()
            .unwrap_or_default();

        let all_sessions = self.sessions.read().await;

        // Preserve order from daemon_sessions Vec (not HashMap iteration order)
        // Only return Pending sessions - once dispatched, they transition to Starting
        session_ids
            .iter()
            .filter_map(|session_id| all_sessions.get(session_id).cloned())
            .filter(|session| session.phase == DiscoveryPhase::Pending)
            .collect()
    }

    /// Clear all sessions for a daemon from in-memory state.
    /// Used by tests to ensure clean state between phases.
    pub async fn clear_sessions_for_daemon(&self, daemon_id: &Uuid) {
        let mut sessions = self.sessions.write().await;
        let mut daemon_sessions = self.daemon_sessions.write().await;
        let mut session_last_updated = self.session_last_updated.write().await;
        let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;
        let mut discovery_sessions = self.discovery_sessions.write().await;

        if let Some(session_ids) = daemon_sessions.remove(daemon_id) {
            for session_id in &session_ids {
                sessions.remove(session_id);
                session_last_updated.remove(session_id);
                discovery_sessions.retain(|_, sid| sid != session_id);
            }
            tracing::debug!(
                daemon_id = %daemon_id,
                session_count = session_ids.len(),
                "Cleared all sessions for daemon"
            );
        }

        daemon_pull_cancellations.remove(daemon_id);
    }

    /// Check if daemon has an active (dispatched, non-terminal) discovery session.
    /// Both Queued and Pending are excluded — neither has been dispatched yet.
    pub async fn has_active_session_for_daemon(&self, daemon_id: &Uuid) -> bool {
        let daemon_session_ids = self.daemon_sessions.read().await;
        let session_ids = daemon_session_ids
            .get(daemon_id)
            .cloned()
            .unwrap_or_default();

        let all_sessions = self.sessions.read().await;

        session_ids.iter().any(|session_id| {
            all_sessions
                .get(session_id)
                .map(|s| {
                    !s.phase.is_terminal()
                        && s.phase != DiscoveryPhase::Queued
                        && s.phase != DiscoveryPhase::Pending
                })
                .unwrap_or(false)
        })
    }

    /// Transition a session from Pending to Starting phase.
    /// Called when the session is dispatched to the daemon.
    pub async fn transition_session_to_starting(&self, session_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id)
            && session.phase == DiscoveryPhase::Pending
        {
            session.phase = DiscoveryPhase::Starting;
            self.session_last_updated
                .write()
                .await
                .insert(session_id, Utc::now());
            tracing::debug!(
                session_id = %session_id,
                "Transitioned session to Starting phase"
            );
        }
    }

    pub async fn pull_cancellation_for_daemon(&self, daemon_id: &Uuid) -> (bool, Uuid) {
        let mut daemon_cancellation_ids = self.daemon_pull_cancellations.write().await;
        daemon_cancellation_ids
            .remove(daemon_id)
            .unwrap_or((false, Uuid::nil()))
    }

    /// Validate timezone string if present on a scheduled discovery
    fn validate_timezone(run_type: &RunType) -> Result<()> {
        if let RunType::Scheduled {
            timezone: Some(tz), ..
        } = run_type
            && tz.parse::<chrono_tz::Tz>().is_err()
        {
            bail_validation!(
                "Invalid timezone '{}'. Use an IANA timezone like 'America/New_York'.",
                tz
            );
        }
        Ok(())
    }

    /// Create a new scheduled discovery
    pub async fn create_discovery(
        self: &Arc<Self>,
        discovery: Discovery,
        authentication: AuthenticatedEntity,
    ) -> Result<Discovery> {
        Self::validate_timezone(&discovery.base.run_type)?;
        let mut created_discovery = if discovery.id == Uuid::nil() {
            self.discovery_storage
                .create(&Discovery::new(discovery.base))
                .await?
        } else {
            self.discovery_storage.create(&discovery).await?
        };

        // Save tags to junction table
        if let Some(entity_tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
        {
            entity_tag_service
                .set_tags(
                    created_discovery.id,
                    EntityDiscriminants::Discovery,
                    created_discovery.base.tags.clone(),
                    org_id,
                )
                .await?;
        }

        // If it's a scheduled discovery, add it to the scheduler
        if matches!(created_discovery.base.run_type, RunType::Scheduled { .. })
            && let Err(e) = Self::schedule_discovery(self, &created_discovery).await
        {
            // Disable and save to DB
            created_discovery.disable();
            let disabled_discovery = self
                .discovery_storage
                .update(&mut created_discovery)
                .await?;

            tracing::error!(
                "Failed to schedule discovery {}. Discovery created but disabled. Error: {}",
                disabled_discovery.id,
                e
            );

            return Ok(disabled_discovery);
        }

        let trigger_stale = created_discovery.triggers_staleness(None);

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: created_discovery.id(),
                network_id: self.get_network_id(&created_discovery),
                organization_id: self.get_organization_id(&created_discovery),
                entity_type: created_discovery.clone().into(),
                operation: EntityOperation::Created,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale
                }),

                authentication,
            })
            .await?;

        Ok(created_discovery)
    }

    /// Delete group
    pub async fn delete_discovery(
        self: &Arc<Self>,
        id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let discovery = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Discovery not found"))?;

        // If it's scheduled, remove from scheduler first (with timeout to prevent deadlock)
        if matches!(discovery.base.run_type, RunType::Scheduled { .. }) {
            self.remove_scheduled_job(id).await;
            tracing::debug!("Removed scheduled job for discovery {}", id);
        }

        // Remove tags from junction table
        if let Some(tag_service) = self.entity_tag_service() {
            tag_service
                .remove_all_for_entity(*id, EntityDiscriminants::Discovery)
                .await?;
        }

        self.discovery_storage.delete(id).await?;

        let trigger_stale = discovery.triggers_staleness(None);

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: discovery.id(),
                network_id: self.get_network_id(&discovery),
                organization_id: self.get_organization_id(&discovery),
                entity_type: discovery.into(),
                operation: EntityOperation::Deleted,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale
                }),

                authentication,
            })
            .await?;
        Ok(())
    }

    /// Initialize scheduler with all scheduled discoveries
    pub async fn start_scheduler(self: &Arc<Self>) -> Result<()> {
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))?;

        // Clear any stale job_id mappings from previous runs
        self.job_ids.write().await.clear();

        let filter = StorableFilter::<Discovery>::new_for_scheduled_discoveries();

        let discoveries = self.discovery_storage.get_all(filter).await?;
        let count = discoveries.len();

        let mut failed_count = 0;
        for mut discovery in discoveries {
            if let Err(e) = Self::schedule_discovery(self, &discovery).await {
                tracing::error!(
                    "Failed to schedule discovery {}: {}. Disabling.",
                    discovery.id,
                    e
                );

                // Disable and save
                discovery.disable();
                let _ = self.discovery_storage.update(&mut discovery).await;
                failed_count += 1;
            }
        }

        scheduler.start().await?;

        if failed_count == 0 {
            tracing::info!(target: LOG_TARGET, "Discovery scheduler started with {} jobs", count);
        } else {
            tracing::warn!(
                target: LOG_TARGET,
                "Discovery scheduler started with {}/{} jobs. {} failed and were disabled.",
                count - failed_count,
                count,
                failed_count
            );
        }

        Ok(())
    }

    /// Schedule a single discovery
    async fn schedule_discovery(
        service: &Arc<DiscoveryService>,
        discovery: &Discovery,
    ) -> Result<Uuid> {
        let _ = service
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))?;

        let RunType::Scheduled {
            cron_schedule,
            enabled,
            timezone,
            ..
        } = &discovery.base.run_type
        else {
            return Err(anyhow::anyhow!("Discovery is not scheduled"));
        };

        if !enabled {
            return Err(anyhow::anyhow!("Discovery is not enabled"));
        }

        let scheduler = service
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))?;

        let tz: chrono_tz::Tz = timezone
            .as_deref()
            .unwrap_or("UTC")
            .parse()
            .unwrap_or(chrono_tz::UTC);

        let discovery = discovery.clone();
        let discovery_id = discovery.id;
        let storage = service.discovery_storage.clone();

        // Clone self to use start_session
        let service_clone = Arc::clone(service);

        let job = JobBuilder::new()
            .with_timezone(tz)
            .with_cron_job_type()
            .with_schedule(cron_schedule.as_str())?
            .with_run_async(Box::new(move |_uuid, _lock| {
                let discovery = discovery.clone();
                let storage = storage.clone();
                let service = service_clone.clone();

                Box::pin(async move {
                    // Check if discovery is still enabled before running — cron jobs
                    // may linger after failed removals
                    let fresh = match storage.get_by_id(&discovery_id).await {
                        Ok(Some(fresh))
                            if matches!(
                                fresh.base.run_type,
                                RunType::Scheduled { enabled: true, .. }
                            ) =>
                        {
                            fresh
                        }
                        _ => {
                            tracing::debug!("Skipping disabled/deleted discovery {}", discovery_id);
                            return;
                        }
                    };

                    // Check if daemon is reachable before starting session
                    if let Some(daemon_service) = service.daemon_service.get() {
                        match daemon_service
                            .storage()
                            .get_by_id(&fresh.base.daemon_id)
                            .await
                        {
                            Ok(Some(daemon))
                                if daemon.base.is_unreachable || daemon.base.standby =>
                            {
                                tracing::debug!(
                                    discovery_id = %discovery_id,
                                    daemon_id = %fresh.base.daemon_id,
                                    is_unreachable = daemon.base.is_unreachable,
                                    standby = daemon.base.standby,
                                    "Skipping scheduled discovery — daemon is not available"
                                );
                                return;
                            }
                            Ok(None) => {
                                tracing::debug!(
                                    discovery_id = %discovery_id,
                                    daemon_id = %fresh.base.daemon_id,
                                    "Skipping scheduled discovery — daemon not found"
                                );
                                return;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    discovery_id = %discovery_id,
                                    error = %e,
                                    "Failed to check daemon status, proceeding with discovery"
                                );
                            }
                            _ => {} // daemon is reachable, proceed
                        }
                    }

                    tracing::info!("Running scheduled discovery {}", &discovery.id);

                    match service
                        .start_session(discovery.clone(), AuthenticatedEntity::System)
                        .await
                    {
                        Ok(_) => {
                            // Reload fresh discovery from DB to avoid overwriting fields
                            match storage.get_by_id(&discovery_id).await {
                                Ok(Some(mut fresh)) => {
                                    if let RunType::Scheduled {
                                        ref mut last_run, ..
                                    } = fresh.base.run_type
                                    {
                                        *last_run = Some(Utc::now());
                                        if let Err(e) = storage.update(&mut fresh).await {
                                            tracing::error!(
                                                "Failed to update schedule times: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                                _ => {
                                    tracing::warn!(
                                        "Failed to reload discovery {} for last_run update",
                                        discovery_id
                                    );
                                }
                            };
                        }
                        Err(e) => {
                            tracing::error!("Scheduled discovery {} failed: {:?}", discovery_id, e);
                        }
                    }
                })
            }))
            .build()?;

        let job_id = tokio::time::timeout(std::time::Duration::from_secs(5), scheduler.add(job))
            .await
            .map_err(|_| {
                anyhow!(
                    "Timed out adding scheduled job for discovery {}",
                    discovery_id
                )
            })?
            .map_err(|e| {
                anyhow!(
                    "Failed to add scheduled job for discovery {}: {}",
                    discovery_id,
                    e
                )
            })?;

        // Store the mapping so we can remove the job later when the schedule is updated
        service.job_ids.write().await.insert(discovery_id, job_id);

        tracing::debug!(
            "Scheduled discovery {} with job_id {} and cron: {}",
            discovery_id,
            job_id,
            cron_schedule
        );
        Ok(job_id)
    }

    /// Get pending_credential_ids for a session by reverse-looking up the discovery entity.
    pub async fn get_pending_credential_ids_for_session(&self, session_id: &Uuid) -> Vec<Uuid> {
        let discovery_id = self
            .discovery_sessions
            .read()
            .await
            .iter()
            .find(|(_, sid)| *sid == session_id)
            .map(|(did, _)| *did);

        if let Some(discovery_id) = discovery_id
            && let Ok(Some(discovery)) = self.discovery_storage.get_by_id(&discovery_id).await
        {
            return discovery.pending_credential_ids;
        }
        vec![]
    }

    /// Build a DaemonDiscoveryRequest with all credential mappings resolved.
    /// Called by both DaemonPoll and ServerPoll dispatch points.
    pub async fn build_daemon_request(
        &self,
        session: &DiscoveryUpdatePayload,
        network_id: Uuid,
        pending_credential_ids: &[Uuid],
    ) -> Result<crate::server::daemons::r#impl::api::DaemonDiscoveryRequest, anyhow::Error> {
        let credential_mappings = if matches!(session.discovery_type, DiscoveryType::Unified { .. })
        {
            self.credential_service
                .build_all_credential_mappings(network_id, pending_credential_ids)
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };

        Ok(
            crate::server::daemons::r#impl::api::DaemonDiscoveryRequest {
                session_id: session.session_id,
                discovery_type: session.discovery_type.clone(),
                credential_mappings,
            },
        )
    }

    /// Create a new discovery session
    pub async fn start_session(
        &self,
        mut discovery: Discovery,
        authentication: AuthenticatedEntity,
    ) -> Result<DiscoveryUpdatePayload, ApiError> {
        // Enforce one active session per discovery configuration
        if self
            .discovery_sessions
            .read()
            .await
            .contains_key(&discovery.id)
        {
            return Err(ApiError::conflict(
                "A session is already running for this discovery",
            ));
        }

        // Update last_run on the discovery (covers all code paths: handler, scheduler, registration)
        match &mut discovery.base.run_type {
            RunType::Scheduled { last_run, .. } => *last_run = Some(Utc::now()),
            RunType::AdHoc { last_run, .. } => *last_run = Some(Utc::now()),
            _ => {}
        }
        discovery.updated_at = Utc::now();
        if let Err(e) = self.discovery_storage.update(&mut discovery).await {
            tracing::error!(
                "Failed to update last_run for discovery {}: {}",
                discovery.id,
                e
            );
        }

        let session_id = Uuid::new_v4();

        // Hydrate SNMP credentials
        let discovery_type = if let DiscoveryType::Network {
            host_naming_fallback,
            subnet_ids,
            ..
        } = discovery.base.discovery_type
        {
            DiscoveryType::Network {
                subnet_ids,
                host_naming_fallback,
                snmp_credentials: self
                    .credential_service
                    .build_snmp_credentials_for_discovery(discovery.base.network_id)
                    .await
                    .map_err(|e| ApiError::internal_error(&e.to_string()))?,
            }
        } else {
            discovery.base.discovery_type
        };

        let mut session_payload = DiscoveryUpdatePayload::new(
            session_id,
            discovery.base.daemon_id,
            discovery.base.network_id,
            discovery_type,
        );

        // Compute whether this scan should be a full port scan and set on scan_settings
        if let DiscoveryType::Unified {
            ref mut scan_settings,
            ..
        } = session_payload.discovery_type
        {
            let full_scan_interval = scan_settings.full_scan_interval.unwrap_or(
                crate::server::discovery::r#impl::scan_settings::defaults::full_scan_interval(),
            );
            // 0 = never full scan (all light), 1 = every scan is full
            scan_settings.is_full_scan = discovery.force_full_scan
                || (full_scan_interval != 0
                    && (full_scan_interval == 1
                        || discovery.scan_count == 1
                        || (discovery.scan_count > 1
                            && discovery.scan_count.is_multiple_of(full_scan_interval))));
        }

        // Track discovery -> session mapping
        self.discovery_sessions
            .write()
            .await
            .insert(discovery.id, session_id);

        // Check if daemon has any sessions running
        let daemon_is_running_discovery = if let Some(daemon_sessions) = self
            .daemon_sessions
            .read()
            .await
            .get(&discovery.base.daemon_id)
        {
            !daemon_sessions.is_empty()
        } else {
            false
        };

        // Promote Queued → Pending if daemon has no other sessions
        if !daemon_is_running_discovery {
            session_payload.phase = DiscoveryPhase::Pending;
            self.session_last_updated
                .write()
                .await
                .insert(session_id, Utc::now());
        }

        // Add to session map
        self.sessions
            .write()
            .await
            .insert(session_id, session_payload.clone());

        // Add session to queue
        self.daemon_sessions
            .write()
            .await
            .entry(discovery.base.daemon_id)
            .or_default()
            .push(session_id);

        // Publish event if no other sessions are running for daemon
        // DaemonService subscribes to this event and sends the request to the daemon.
        if !daemon_is_running_discovery {
            self.event_bus()
                .publish_discovery(session_payload.into_discovery_event_with_auth(authentication))
                .await
                .map_err(|e| ApiError::internal_error(&e.to_string()))?;
        }

        let _ = self.update_tx.send(session_payload.clone());

        Ok(session_payload)
    }

    /// Update progress for a session
    /// If the session doesn't exist (e.g., server restarted during discovery),
    /// auto-creates it from the payload context to maintain resilience.
    pub async fn update_session(&self, update: DiscoveryUpdatePayload) -> Result<(), Error> {
        tracing::debug!("Updated session {:?}", update);

        let mut sessions = self.sessions.write().await;

        let mut last_updated = self.session_last_updated.write().await;
        // Check if we've seen this session before (used as tombstone for completed sessions)
        let already_seen = last_updated.contains_key(&update.session_id);
        // Track last update time
        last_updated.insert(update.session_id, Utc::now());

        // Auto-create session if it doesn't exist (handles server restarts during discovery)
        if let std::collections::hash_map::Entry::Vacant(e) = sessions.entry(update.session_id) {
            // If we already tracked this session but it's no longer in the sessions map,
            // it was already processed and removed. Skip redundant terminal updates from
            // old daemons that don't clear their terminal payload after serving it.
            if update.phase.is_terminal() && already_seen {
                tracing::debug!(
                    session_id = %update.session_id,
                    phase = %update.phase,
                    "Ignoring redundant terminal update (already processed)"
                );
                return Ok(());
            }

            tracing::info!(
                session_id = %update.session_id,
                daemon_id = %update.daemon_id,
                network_id = %update.network_id,
                "Auto-creating session from daemon update"
            );

            // Track in daemon_sessions map
            let mut daemon_sessions = self.daemon_sessions.write().await;
            daemon_sessions
                .entry(update.daemon_id)
                .or_default()
                .push(update.session_id);
            drop(daemon_sessions);

            // Insert the session
            e.insert(update.clone());
        }

        let session = sessions.get_mut(&update.session_id).unwrap();

        let daemon_id = session.daemon_id;
        let network_id = session.network_id;

        tracing::debug!(
            session_id = %update.session_id,
            phase = %update.phase,
            progress = %update.progress,
            "Updated session",
        );

        // Publish onboarding milestone BEFORE SSE update so it's
        // in the DB when the SSE-triggered org refetch arrives
        if update.phase == DiscoveryPhase::Complete
            && matches!(
                update.discovery_type,
                DiscoveryType::Network { .. } | DiscoveryType::Unified { .. }
            )
            && let Ok(Some(network)) = self.network_service.get_by_id(&network_id).await
            && let Ok(Some(org)) = self
                .organization_service
                .get_by_id(&network.base.organization_id)
                .await
            && org.not_onboarded(&OnboardingOperation::FirstDiscoveryCompleted)
        {
            let _ = self
                .event_bus
                .publish_onboarding(OnboardingEvent::new(
                    Uuid::new_v4(),
                    org.id,
                    OnboardingOperation::FirstDiscoveryCompleted,
                    Utc::now(),
                    AuthenticatedEntity::System,
                    serde_json::json!({
                        "discovery_type": update.discovery_type.to_string(),
                    }),
                ))
                .await;
        }

        let _ = self.update_tx.send(update.clone());

        *session = update.clone();

        if session.phase.is_terminal() {
            self.event_bus()
                .publish_discovery(session.into_discovery_event())
                .await?;

            // If user cancelled session, but it finished before we could send cancellation, remove key so it doesn't cancel upcoming sessions
            self.pull_cancellation_for_daemon(&session.daemon_id).await;

            // Create historical discovery record
            let network_name = match self.network_service.get_by_id(&session.network_id).await {
                Ok(Some(network)) => network.base.name,
                _ => "Unknown Network".to_string(),
            };

            let historical_discovery = Discovery {
                id: Uuid::new_v4(),
                created_at: session.started_at.unwrap_or(Utc::now()),
                updated_at: Utc::now(),
                base: crate::server::discovery::r#impl::base::DiscoveryBase {
                    daemon_id: session.daemon_id,
                    network_id: session.network_id,
                    name: if matches!(session.discovery_type, DiscoveryType::Unified { .. }) {
                        "Discovery".to_string()
                    } else {
                        format!("{} \u{2014} {}", session.discovery_type, network_name)
                    },
                    tags: Vec::new(),
                    discovery_type: session.discovery_type.clone(),
                    run_type: RunType::Historical {
                        results: Box::new(session.clone()),
                    },
                },
                scan_count: 0,
                force_full_scan: false,
                pending_credential_ids: vec![],
            };

            // Increment scan_count and clear ephemeral fields only on successful completion.
            // Failures/cancellations preserve these so the next retry uses the same config.
            if session.phase == DiscoveryPhase::Complete {
                // Reverse-lookup: find discovery_id from session_id
                let discovery_id = self
                    .discovery_sessions
                    .read()
                    .await
                    .iter()
                    .find(|(_, sid)| **sid == session.session_id)
                    .map(|(did, _)| *did);

                if let Some(discovery_id) = discovery_id
                    && let Ok(Some(mut parent_discovery)) =
                        self.discovery_storage.get_by_id(&discovery_id).await
                {
                    parent_discovery.scan_count += 1;
                    parent_discovery.force_full_scan = false;
                    parent_discovery.pending_credential_ids = vec![];
                    parent_discovery.updated_at = Utc::now();
                    if let Err(e) = self.discovery_storage.update(&mut parent_discovery).await {
                        tracing::error!(
                            "Failed to increment scan_count for discovery {}: {}",
                            discovery_id,
                            e
                        );
                    }
                }
            }

            // Save to database
            if let Err(e) = self.discovery_storage.create(&historical_discovery).await {
                tracing::error!(
                    "Failed to create historical discovery record for session {}: {}",
                    session.session_id,
                    e
                );
            } else {
                self.event_bus()
                    .publish_entity(EntityEvent {
                        id: Uuid::new_v4(),
                        entity_id: historical_discovery.id(),
                        network_id: self.get_network_id(&historical_discovery),
                        organization_id: self.get_organization_id(&historical_discovery),
                        entity_type: historical_discovery.into(),
                        operation: EntityOperation::Created,
                        timestamp: Utc::now(),
                        metadata: serde_json::json!({}),
                        authentication: AuthenticatedEntity::System,
                    })
                    .await?;
            }

            // Get next session info BEFORE trying to send request
            let next_session_info = if let Some(daemon_sessions) = self
                .daemon_sessions
                .write()
                .await
                .get_mut(&session.daemon_id)
            {
                daemon_sessions.retain(|s| *s != session.session_id);

                // Promote next Queued session to Pending and start its stall clock
                daemon_sessions
                    .first()
                    .and_then(|next_session_id| sessions.get_mut(next_session_id))
                    .map(|next_session| {
                        next_session.phase = DiscoveryPhase::Pending;
                        last_updated.insert(next_session.session_id, Utc::now());
                        (next_session.discovery_type.clone(), next_session.session_id)
                    })
            } else {
                None
            };

            // Remove the completed session
            sessions.remove(&update.session_id);

            // Remove from discovery_sessions map (find by session_id value)
            self.discovery_sessions
                .write()
                .await
                .retain(|_, sid| *sid != update.session_id);

            drop(sessions);
            drop(last_updated);

            // Publish event which will trigger notifying any daemons in ServerPoll to start session
            // If daemon is daemon_poll mode, it will request next session on its next poll
            if let Some((discovery_type, session_id)) = next_session_info {
                let mut started_payload =
                    DiscoveryUpdatePayload::new(session_id, daemon_id, network_id, discovery_type);
                started_payload.phase = DiscoveryPhase::Pending;

                self.event_bus()
                    .publish_discovery(started_payload.into_discovery_event())
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn cancel_session(
        &self,
        session_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        // Get the session
        let session = match self.get_session(&session_id).await {
            Some(session) => session,
            None => {
                return Err(anyhow!("Session '{}' not found", session_id));
            }
        };

        let network_id = session.network_id;
        let daemon_id = session.daemon_id;
        let phase = session.phase;

        let cancelled_update = DiscoveryUpdatePayload {
            session_id,
            network_id,
            daemon_id,
            phase: DiscoveryPhase::Cancelled,
            progress: 0,
            error: None,
            started_at: session.started_at,
            finished_at: Some(Utc::now()),
            discovery_type: session.discovery_type,
            hosts_discovered: None,
            estimated_remaining_secs: None,
        };

        // Handle based on current phase
        match phase {
            // Queued/Pending sessions: just remove from queue
            DiscoveryPhase::Queued | DiscoveryPhase::Pending => {
                let mut sessions = self.sessions.write().await;
                let mut daemon_sessions = self.daemon_sessions.write().await;

                let was_pending = phase == DiscoveryPhase::Pending;

                // Remove from sessions map
                sessions.remove(&session_id);

                // Remove from daemon queue
                if let Some(queue) = daemon_sessions.get_mut(&daemon_id) {
                    queue.retain(|id| *id != session_id);

                    // If we removed the Pending session, promote next Queued → Pending
                    if was_pending
                        && let Some(next_session) =
                            queue.first().and_then(|next_id| sessions.get_mut(next_id))
                    {
                        next_session.phase = DiscoveryPhase::Pending;
                        self.session_last_updated
                            .write()
                            .await
                            .insert(next_session.session_id, Utc::now());
                    }
                }

                // Remove from discovery_sessions map
                self.discovery_sessions
                    .write()
                    .await
                    .retain(|_, sid| *sid != session_id);

                drop(sessions);
                drop(daemon_sessions);

                // Broadcast cancellation update so frontend knows
                let _ = self.update_tx.send(cancelled_update);

                tracing::info!("Cancelled {} session {} from queue", phase, session_id);
                Ok(())
            }

            // Starting phase: wait briefly then retry
            DiscoveryPhase::Starting => Err(anyhow!(
                "Session is starting on daemon. Please try again in a moment."
            )),

            // Active phases: send cancellation to daemon
            // We do BOTH actions to support both daemon modes:
            // 1. Publish DiscoveryCancelled event - DaemonService subscriber handles ServerPoll mode
            // 2. Set cancellation flag - DaemonPoll mode checks on next poll via request_work
            DiscoveryPhase::Started | DiscoveryPhase::Scanning => {
                self.event_bus()
                    .publish_discovery(
                        cancelled_update.into_discovery_event_with_auth(authentication),
                    )
                    .await?;

                // Set cancellation flag for DaemonPoll mode (checked on next poll)
                self.daemon_pull_cancellations
                    .write()
                    .await
                    .insert(daemon_id, (true, session_id));

                tracing::info!(
                    daemon_id = %daemon_id,
                    session_id = %session_id,
                    "Discovery cancellation requested",
                );

                Ok(())
            }

            // Terminal phases: already done
            DiscoveryPhase::Complete | DiscoveryPhase::Failed | DiscoveryPhase::Cancelled => {
                tracing::info!(
                    "Session {} is already in terminal state: {}, nothing to cancel",
                    session_id,
                    phase
                );
                Ok(())
            }
        }
    }

    pub async fn cleanup_old_sessions(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut sessions = self.sessions.write().await;
        let mut daemon_sessions = self.daemon_sessions.write().await;
        let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;
        let mut discovery_sessions = self.discovery_sessions.write().await;

        let mut to_remove = Vec::new();
        for (session_id, session) in sessions.iter() {
            if let Some(finished_at) = session.finished_at
                && finished_at < cutoff
            {
                to_remove.push(*session_id);
            }
        }

        for session_id in to_remove {
            if let Some(session) = sessions.remove(&session_id) {
                daemon_pull_cancellations.remove(&session.daemon_id);

                if let Some(daemon_sessions) = daemon_sessions.get_mut(&session.daemon_id) {
                    daemon_sessions.retain(|s| *s != session.session_id);
                }

                discovery_sessions.retain(|_, sid| *sid != session_id);

                tracing::debug!("Cleaned up old discovery session {}", session_id);
            }
        }
    }

    /// Cleanup stalled sessions (called periodically from background task)
    pub async fn cleanup_stalled_sessions(&self) {
        let now = Utc::now();
        let stall_threshold = chrono::Duration::minutes(5);

        // First pass: identify stalled sessions (read locks only)
        let stalled_sessions: Vec<DiscoveryUpdatePayload> = {
            let sessions = self.sessions.read().await;
            let last_updated = self.session_last_updated.read().await;

            sessions
                .iter()
                .filter_map(|(session_id, session)| {
                    // Only check phases that are subject to stall cleanup
                    if !session.phase.can_be_cleaned_up() {
                        return None;
                    }

                    // Check last update time
                    let is_stalled = if let Some(last_update_time) = last_updated.get(session_id) {
                        now.signed_duration_since(*last_update_time) > stall_threshold
                    } else if let Some(started_at) = session.started_at {
                        now.signed_duration_since(started_at) > stall_threshold
                    } else {
                        // Session with no tracking timestamps at all —
                        // it was dispatched but never reported back. Treat as stalled.
                        tracing::warn!(
                            session_id = %session_id,
                            phase = ?session.phase,
                            "Session has no tracking timestamps, treating as stalled"
                        );
                        true
                    };

                    if is_stalled {
                        Some(session.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        if stalled_sessions.is_empty() {
            return;
        }

        // Second pass: request cancellation for stalled sessions (no locks held)
        // We do BOTH actions to support both daemon modes:
        // 1. Publish DiscoveryCancelled event - DaemonService subscriber handles ServerPoll mode
        // 2. Set cancellation flag - DaemonPoll mode checks on next poll via request_work
        for session in &stalled_sessions {
            let daemon_id = session.daemon_id;
            let session_id = session.session_id;

            tracing::warn!(
                session_id = %session_id,
                daemon_id = %daemon_id,
                "Requesting cancellation for stalled session"
            );

            let cancelled_update = DiscoveryUpdatePayload {
                session_id,
                network_id: session.network_id,
                daemon_id,
                phase: DiscoveryPhase::Cancelled,
                progress: session.progress,
                error: None,
                started_at: session.started_at,
                finished_at: Some(Utc::now()),
                discovery_type: session.discovery_type.clone(),
                hosts_discovered: None,
                estimated_remaining_secs: None,
            };

            if let Err(e) = self
                .event_bus()
                .publish_discovery(cancelled_update.into_discovery_event())
                .await
            {
                tracing::warn!(
                    daemon_id = %session.daemon_id,
                    session_id = %session.session_id,
                    error = %e,
                    "Failed to publish cancellation event for stalled session"
                );
            }

            // Set cancellation flag for DaemonPoll mode (checked on next poll)
            self.daemon_pull_cancellations
                .write()
                .await
                .insert(daemon_id, (true, session_id));

            tracing::info!(
                daemon_id = %daemon_id,
                session_id = %session_id,
                "Cancellation requested for stalled session"
            );
        }

        // Third pass: cleanup session state (write locks)
        let mut sessions = self.sessions.write().await;
        let mut last_updated = self.session_last_updated.write().await;
        let mut daemon_sessions = self.daemon_sessions.write().await;
        let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;
        let mut discovery_sessions = self.discovery_sessions.write().await;

        let mut stalled_count = 0;

        for session in stalled_sessions {
            if let Some(mut session) = sessions.remove(&session.session_id) {
                let daemon_id = session.daemon_id;
                let session_id = session.session_id;

                tracing::warn!(
                    session_id = %session_id,
                    daemon_id = %daemon_id,
                    phase = ?session.phase,
                    "Cleaning up stalled discovery session (no updates for 5+ minutes)"
                );

                // Update to failed state
                session.phase = DiscoveryPhase::Failed;
                session.error = Some(
                    "Session stalled - no updates received from daemon for more than 5 minutes"
                        .to_string(),
                );
                session.finished_at = Some(now);

                // Remove from daemon sessions queue and promote next Queued → Pending
                if let Some(queue) = daemon_sessions.get_mut(&daemon_id) {
                    queue.retain(|id| *id != session_id);

                    // Promote next Queued session to Pending
                    if let Some(next_session) =
                        queue.first().and_then(|next_id| sessions.get_mut(next_id))
                        && next_session.phase == DiscoveryPhase::Queued
                    {
                        next_session.phase = DiscoveryPhase::Pending;
                        last_updated.insert(next_session.session_id, Utc::now());
                    }
                }

                // Remove from discovery_sessions map
                discovery_sessions.retain(|_, sid| *sid != session_id);

                // Remove from last_updated tracking
                last_updated.remove(&session_id);

                // Broadcast the failed state update
                let _ = self.update_tx.send(session.clone());

                // Clean up any pending cancellation for this daemon/session
                if let Some((_, cancel_session_id)) = daemon_pull_cancellations.get(&daemon_id)
                    && *cancel_session_id == session_id
                {
                    daemon_pull_cancellations.remove(&daemon_id);
                    tracing::debug!(
                        "Removed stale cancellation flag for daemon {} session {}",
                        daemon_id,
                        session_id
                    );
                }

                // Create historical discovery record for the stalled session,
                // but only if the daemon still exists (it may have been deleted,
                // which would cause a FK violation on the discovery table).
                let daemon_exists = match self.daemon_service.get() {
                    Some(ds) => ds
                        .get_by_id(&session.daemon_id)
                        .await
                        .ok()
                        .flatten()
                        .is_some(),
                    None => false,
                };

                if daemon_exists {
                    let network_name =
                        match self.network_service.get_by_id(&session.network_id).await {
                            Ok(Some(network)) => network.base.name,
                            _ => "Unknown Network".to_string(),
                        };

                    let historical_discovery = Discovery {
                        id: Uuid::new_v4(),
                        created_at: session.started_at.unwrap_or(now),
                        updated_at: now,
                        base: crate::server::discovery::r#impl::base::DiscoveryBase {
                            daemon_id: session.daemon_id,
                            network_id: session.network_id,
                            tags: Vec::new(),
                            name: if matches!(session.discovery_type, DiscoveryType::Unified { .. })
                            {
                                "Discovery".to_string()
                            } else {
                                format!("{} \u{2014} {}", session.discovery_type, network_name)
                            },
                            discovery_type: session.discovery_type.clone(),
                            run_type: RunType::Historical {
                                results: Box::new(session),
                            },
                        },
                        scan_count: 0,
                        force_full_scan: false,
                        pending_credential_ids: vec![],
                    };

                    if let Err(e) = self.discovery_storage.create(&historical_discovery).await {
                        tracing::error!(
                            "Failed to create historical discovery record for stalled session {}: {}",
                            session_id,
                            e
                        );
                    }
                } else {
                    tracing::debug!(
                        session_id = %session_id,
                        daemon_id = %daemon_id,
                        "Skipping historical record for stalled session — daemon no longer exists"
                    );
                }

                stalled_count += 1;
            }
        }

        // Evict tombstones: last_updated entries for sessions that no longer exist
        // in the sessions map and are older than the stall threshold. These are left
        // behind after terminal processing to guard against redundant polls from old
        // daemons (see update_session). Safe to clean up once enough time has passed.
        last_updated.retain(|id, ts| {
            sessions.contains_key(id) || now.signed_duration_since(*ts) < stall_threshold
        });

        if stalled_count > 0 {
            tracing::info!("Cleaned up {} stalled discovery sessions", stalled_count);
        }
    }

    /// Remove a scheduled job using fire-and-forget to prevent deadlocks.
    /// The scheduler's `remove()` can hang indefinitely if the background task is blocked.
    /// We clean up the job_id mapping immediately and spawn the actual removal as a
    /// background task so it never blocks the critical path.
    async fn remove_scheduled_job(&self, discovery_id: &Uuid) {
        // Read the job_id first, then drop the read lock before acquiring write lock.
        // Holding a RwLock read guard while awaiting .write() deadlocks.
        let job_id = self.job_ids.read().await.get(discovery_id).copied();
        if let Some(scheduler) = &self.scheduler
            && let Some(job_id) = job_id
        {
            // Always clean up the mapping immediately
            self.job_ids.write().await.remove(discovery_id);

            // Fire-and-forget the actual scheduler removal — it may hang
            // but won't block the current task
            let scheduler = Arc::clone(scheduler);
            tokio::spawn(async move {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    scheduler.remove(&job_id),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => tracing::warn!(
                        job_id = %job_id,
                        error = ?e,
                        "Failed to remove scheduled job"
                    ),
                    Err(_) => tracing::warn!(
                        job_id = %job_id,
                        "Timed out removing scheduled job"
                    ),
                }
            });
        }
    }
}
