use crate::bail_validation;
use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::daemon::runtime::service::LOG_TARGET;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::config::DeploymentType;
use crate::server::credentials::r#impl::mapping::{IntegrationTarget, Target};
use crate::server::credentials::service::CredentialService;
use crate::server::daemons::r#impl::api::{
    DaemonDiscoveryRequest, DiscoveryUpdatePayload, ScannedEntityIds,
};
use crate::server::daemons::service::DaemonService;
use crate::server::digest::service::DiscoveryDigestService;
use crate::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
use crate::server::discovery_warnings::service::DiscoveryWarningService;
use crate::server::hosts::service::HostService;
use crate::server::organizations::service::OrganizationService;
use crate::server::setup::types::{SetupItemDiscriminants, SetupOperation};
use crate::server::shared::entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants};
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::shared::events::types::{EntityOperation, UpdateReason};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::traits::{Entity, PaginatedResult, Storable, Storage, Unique};
use crate::server::shared::types::api::ApiError;
use crate::server::sites::service::SiteService;
use crate::server::subnets::r#impl::base::Subnet;
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
///
/// **Lock order.** Any code holding more than one of the session locks takes them in this order:
/// `running_snapshots` → `sessions` → `session_last_updated` → `daemon_sessions` →
/// `daemon_pull_cancellations` → `discovery_sessions`. Taking a later one and then waiting on an
/// earlier one deadlocks against a writer doing the opposite, and a deadlock here is not bounded
/// by anything: every session read, including the UI's poll, dispatch and the stall sweep itself,
/// queues behind it until the server restarts.
pub struct DiscoveryService {
    self_ref: Weak<Self>,
    discovery_storage: Arc<GenericPostgresStorage<Discovery>>,
    sessions: RwLock<HashMap<Uuid, DiscoveryUpdatePayload>>, // session_id -> session state mapping
    daemon_sessions: RwLock<HashMap<Uuid, Vec<Uuid>>>,       // daemon_id -> session_id mapping
    discovery_sessions: RwLock<HashMap<Uuid, Uuid>>, // discovery_id -> session_id mapping (enforces one active session per discovery)
    /// daemon_id → the cancel requested for the session it is running. Sent on every DaemonPoll
    /// work request until the session ends, and read by the stall sweep for both modes to end a
    /// session its daemon has not stopped by `state::CANCEL_DEADLINE`.
    daemon_pull_cancellations: RwLock<HashMap<Uuid, state::PendingCancel>>,
    /// Site IDs with an in-flight site snapshot. While a site is in
    /// this set, new sessions on it start in `AwaitingSnapshot` and are not
    /// dispatched until `release_site_for_snapshot` clears the entry.
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
    /// Per daemon, the subnets its host requests stored during the running scan. Drained into
    /// the terminal update's scanned set; see `note_touched_subnets`.
    touched_subnets: RwLock<HashMap<Uuid, HashSet<Uuid>>>,
    /// Per daemon, the credentials its latest dispatch withheld because the daemon is too old to
    /// parse them. Latched at dispatch, which builds the request but cannot write the session
    /// (`update_session` replaces it wholesale), and drained into the terminal update's warnings.
    withheld_credentials: RwLock<HashMap<Uuid, Vec<DiscoveryWarning>>>,
    session_last_updated: RwLock<HashMap<Uuid, chrono::DateTime<Utc>>>,
    update_tx: broadcast::Sender<DiscoveryUpdatePayload>,
    scheduler: Option<Arc<JobScheduler>>,
    job_ids: RwLock<HashMap<Uuid, Uuid>>, // discovery_id -> scheduler job_id mapping
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    credential_service: Arc<CredentialService>,
    site_service: Arc<SiteService>,
    organization_service: Arc<OrganizationService>,
    /// Each historical run's warnings, stored as rows beside it.
    warning_service: Arc<DiscoveryWarningService>,
    /// Whether this is Scanopy Cloud, where a license-plan org gets no scheduled runs.
    deployment_type: DeploymentType,
    // Lazy dependencies (set after construction to break circular dependencies)
    daemon_service: std::sync::OnceLock<Arc<DaemonService>>,
    /// Stamps the discovery FKs on what a finished session scanned.
    host_service: std::sync::OnceLock<Arc<HostService>>,
    /// Computes a finished session's digest.
    digest_service: std::sync::OnceLock<Arc<DiscoveryDigestService>>,
}

impl EventBusService<Discovery> for DiscoveryService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &Discovery) -> Option<Uuid> {
        Some(entity.base.site_id)
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

    // Every read attaches a historical run's warnings, which the run's own row does not hold.

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Discovery>, anyhow::Error> {
        let Some(mut discovery) = self.discovery_storage.get_by_id(id).await? else {
            return Ok(None);
        };
        self.hydrate_tags(&mut discovery).await?;
        self.attach_warnings(std::slice::from_mut(&mut discovery))
            .await?;
        Ok(Some(discovery))
    }

    async fn get_all(
        &self,
        filter: StorableFilter<Discovery>,
    ) -> Result<Vec<Discovery>, anyhow::Error> {
        let mut all = self.discovery_storage.get_all(filter).await?;
        self.bulk_hydrate_tags(&mut all, None).await?;
        self.attach_warnings(&mut all).await?;
        Ok(all)
    }

    async fn get_paginated(
        &self,
        filter: StorableFilter<Discovery>,
    ) -> Result<PaginatedResult<Discovery>, anyhow::Error> {
        self.get_paginated_ordered(filter, "created_at ASC").await
    }

    async fn get_paginated_ordered(
        &self,
        filter: StorableFilter<Discovery>,
        order_by: &str,
    ) -> Result<PaginatedResult<Discovery>, anyhow::Error> {
        let mut paginated = self
            .discovery_storage
            .get_paginated(filter, order_by)
            .await?;
        self.bulk_hydrate_tags(&mut paginated.items, None).await?;
        self.attach_warnings(&mut paginated.items).await?;
        Ok(paginated)
    }

    async fn get_unique(
        &self,
        filter: StorableFilter<Discovery>,
    ) -> Result<Unique<Discovery>, anyhow::Error> {
        Ok(match self.discovery_storage.get_unique(filter).await? {
            Unique::One(mut discovery) => {
                self.hydrate_tags(&mut discovery).await?;
                self.attach_warnings(std::slice::from_mut(&mut discovery))
                    .await?;
                Unique::One(discovery)
            }
            other => other,
        })
    }

    async fn update(
        &self,
        entity: &mut Discovery,
        authentication: AuthenticatedEntity,
        mut reason: UpdateReason,
    ) -> Result<Discovery, anyhow::Error> {
        Self::validate_timezone(&entity.base.run_type)?;

        let current = self
            .get_by_id(&entity.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not find discovery {}", entity))?;

        // Preserve server-managed fields from current DB state
        // (the API client may send stale values for these read-only fields)
        entity.scan_count = current.scan_count;
        // Storage returns this entity as given, so the computed field is set here.
        entity.next_run_is_full_scan = entity.full_scan_is_due();

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
            // Remove old schedule first
            self.remove_scheduled_job(&entity.id).await;

            // Update in DB
            let mut updated = self.discovery_storage.update(entity).await?;

            // Re-add cron job (schedule_discovery guards on !enabled, so disabling skips re-add)
            if let Some(arc_self) = self.self_ref.upgrade()
                && let Err(e) = Self::schedule_discovery(&arc_self, &updated).await
            {
                // Only disable if we were trying to enable/reschedule (not if already disabling).
                // The event then names the failure, so its log says why it is disabled.
                if updated.base.run_type.is_scheduled_enabled() {
                    updated.disable();
                    updated = self.discovery_storage.update(&mut updated).await?;
                    reason = UpdateReason::DiscoveryScheduleFailed {
                        error: e.to_string(),
                    };
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

        self.publish_updated(&current, &updated, authentication, reason)
            .await;

        Ok(updated)
    }
}

mod bulk;
mod cancel;
mod cleanup;
mod discovery_crud;
mod dispatch;
mod history;
mod lifecycle;
mod scheduling;
mod sessions;
mod state;
mod warnings;
pub use bulk::{StartRefusal, order_for_cancel, set_enabled_refusal, start_refusal};
