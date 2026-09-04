use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::types::base::BillingPlan;
use crate::server::organizations::demo_status::DemoPopulateStatus;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::services::traits::EventBusService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::types::metadata::HasId;
use crate::server::tags::entity_tags::EntityTagService;
use crate::server::{
    organizations::r#impl::base::Organization,
    shared::{services::traits::CrudService, storage::generic::GenericPostgresStorage},
};
use anyhow::Error;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use strum::IntoDiscriminant;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct OrganizationService {
    storage: Arc<GenericPostgresStorage<Organization>>,
    event_bus: Arc<EventBus>,
    /// In-memory status of each org's background demo-populate task, polled by
    /// the frontend after the `202`. See [`super::demo_status`].
    demo_status: RwLock<HashMap<Uuid, DemoPopulateStatus>>,
}

impl EventBusService<Organization> for OrganizationService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &Organization) -> Option<Uuid> {
        None
    }
    fn get_organization_id(&self, entity: &Organization) -> Option<Uuid> {
        Some(entity.id)
    }
}

#[async_trait]
impl CrudService<Organization> for OrganizationService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Organization>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl OrganizationService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Organization>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            demo_status: RwLock::new(HashMap::new()),
        }
    }

    /// Claim the single-flight demo-populate slot for `org_id`. Returns the
    /// initial `Running` status (to hand straight back in the `202` body) when
    /// the slot was free, or `None` if a populate is already `Running` for this
    /// org — the caller should then respond `409`.
    pub async fn try_begin_demo(&self, org_id: Uuid) -> Option<DemoPopulateStatus> {
        let mut map = self.demo_status.write().await;
        if matches!(map.get(&org_id), Some(DemoPopulateStatus::Running { .. })) {
            return None;
        }
        let status = DemoPopulateStatus::Running {
            started_at: Utc::now(),
        };
        map.insert(org_id, status.clone());
        Some(status)
    }

    /// Record the terminal status of an org's demo-populate task.
    pub async fn set_demo_status(&self, org_id: Uuid, status: DemoPopulateStatus) {
        self.demo_status.write().await.insert(org_id, status);
    }

    /// Current demo-populate status for `org_id`, if a task has ever run.
    pub async fn get_demo_status(&self, org_id: &Uuid) -> Option<DemoPopulateStatus> {
        self.demo_status.read().await.get(org_id).cloned()
    }

    /// Move every self-hosted org onto `target`, the plan this deployment runs
    /// on (`plans::self_hosted_plan`). The caller gates this on a self-hosted
    /// deployment (no Stripe secret).
    ///
    /// This is what keeps an org provisioned under an older, capped plan from
    /// staying capped: on boot it is moved onto the current unrestricted plan.
    /// Idempotent — plans equal to `target` are skipped, so re-running on every
    /// boot does nothing once reconciled. The plan write goes through the
    /// `PlanReconciled` billing event → this service's own
    /// `Subscriber<BillingOperation>` impl (the sole writer of
    /// `organizations.plan`); we never write the row here. Best-effort: a per-org
    /// publish failure is logged, not fatal. Returns the number of orgs moved.
    pub async fn reconcile_self_hosted_plans(&self, target: BillingPlan) -> Result<u64, Error> {
        let orgs = self.get_all(StorableFilter::<Organization>::new()).await?;

        let mut upgraded = 0u64;
        for org in orgs {
            // Skip only orgs already on exactly `target`. Plan equality is
            // config-based (see `BillingPlan`'s `PartialEq`), so two tiers with
            // identical caps compare equal — hence the discriminant check as
            // well, which keeps a differently-named tier from passing for the
            // target and keeping its own feature matrix. A `None` plan never
            // matches: it would otherwise leave the org on the Free-plan
            // fallback the runtime uses for plan-less rows.
            if org
                .base
                .plan
                .is_some_and(|p| p.discriminant() == target.discriminant() && p == target)
            {
                continue;
            }
            let current = org.base.plan.unwrap_or_default();

            if let Err(e) = self
                .event_bus()
                .publish(Event::new(
                    OrgScope {
                        organization_id: org.id,
                    },
                    BillingOperation::PlanReconciled {
                        from: current,
                        to: target,
                    },
                    AuthenticatedEntity::System,
                ))
                .await
            {
                tracing::warn!(
                    organization_id = %org.id,
                    error = %e,
                    "Failed to publish plan reconciliation for org",
                );
                continue;
            }
            upgraded += 1;
        }

        if upgraded > 0 {
            tracing::info!(
                count = upgraded,
                plan = %target.id(),
                "Reconciled self-hosted org plan(s) to the self-hosted plan",
            );
        }

        Ok(upgraded)
    }
}
