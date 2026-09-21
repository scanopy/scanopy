use crate::bail_validation;
use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};
use crate::daemon::runtime::service::LOG_TARGET;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::credentials::r#impl::mapping::{IntegrationTarget, Target};
use crate::server::credentials::service::CredentialService;
use crate::server::daemons::r#impl::api::{DaemonDiscoveryRequest, DiscoveryUpdatePayload};
use crate::server::daemons::service::DaemonService;
use crate::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
use crate::server::networks::service::NetworkService;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants};
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event, OrgScope};
use crate::server::shared::events::types::{
    EntityOperation, OnboardingOperation, OnboardingOperationDiscriminants,
};
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
    collections::{HashMap, HashSet},
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
    /// Network IDs with an in-flight network snapshot. While a network is in
    /// this set, new sessions on it start in `AwaitingSnapshot` and are not
    /// dispatched until `release_network_for_snapshot` clears the entry.
    /// In-memory only — crash drops any in-flight manual-snapshot intent,
    /// which is acceptable since callers retry.
    running_snapshots: RwLock<HashSet<Uuid>>,
    /// Daemon IDs seen submitting a wire shape a current daemon no longer produces, since their
    /// last completed session.
    ///
    /// Entity submission and session completion are separate requests, and the raw shape is only
    /// visible on the first — so the observation is latched here and drained into the terminal
    /// payload's warnings. Keyed by daemon rather than session because a submission carries no
    /// session id (see `DiscoveryHostRequest`, which has none).
    ///
    /// In-memory only, and deliberately: it describes one scan, and a restart that loses it costs
    /// a warning the next scan raises again.
    superseded_wire_daemons: RwLock<HashSet<Uuid>>,
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

        // Scoped so the borrows on `entity`/`current` end before the update below.
        let (schedule_changed, enabled_changed, entity_is_scheduled) = {
            let new_schedule = entity.base.run_type.schedule();
            let current_schedule = current.base.run_type.schedule();

            // If it's a scheduled discovery and schedule or timezone has changed,
            // it needs rescheduling; likewise on an enabled-state transition.
            let schedule_changed = match (&new_schedule, &current_schedule) {
                (Some(new), Some(cur)) => {
                    cur.cron_schedule != new.cron_schedule || cur.timezone != new.timezone
                }
                _ => false,
            };
            let enabled_changed = match (&new_schedule, &current_schedule) {
                (Some(new), Some(cur)) => cur.enabled != new.enabled,
                _ => false,
            };

            (schedule_changed, enabled_changed, new_schedule.is_some())
        };

        let needs_reschedule = schedule_changed || enabled_changed;

        let updated = if needs_reschedule && entity_is_scheduled {
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
                if updated.base.run_type.is_scheduled_enabled() {
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

        if let Some(scope) = EntityScope::from_ids(
            updated.id(),
            updated.clone().into(),
            self.get_network_id(&updated),
            self.get_organization_id(&updated),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Updated, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            suppress_logs,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(updated)
    }
}

mod cleanup;
mod discovery_crud;
mod dispatch;
mod lifecycle;
mod scheduling;
mod sessions;
mod state;
