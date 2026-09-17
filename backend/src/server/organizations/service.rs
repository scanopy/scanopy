use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::types::base::BillingPlan;
use crate::server::license::types::LicenseKeyType;
use crate::server::organizations::demo_status::DemoPopulateStatus;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::services::traits::EventBusService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::lock::{DEFAULT_LOCK_TIMEOUT, LockKey, SessionLockGuard};
use crate::server::shared::storage::traits::Storage;
use crate::server::shared::types::metadata::HasId;
use crate::server::tags::entity_tags::EntityTagService;
use crate::server::{
    organizations::r#impl::base::Organization,
    shared::{services::traits::CrudService, storage::generic::GenericPostgresStorage},
};
use anyhow::Error;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
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

/// Drop sub-second precision, so a value survives a database round trip
/// unchanged.
fn whole_second(at: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::from_timestamp(at.timestamp(), 0).unwrap_or(at)
}

impl OrganizationService {
    /// Hold the organization row's advisory lock. Every read-modify-write of
    /// an org that can race another (the billing event mirror, license key
    /// regeneration, license check-ins) takes it, because each writes the
    /// whole row back and would otherwise revert the other's change.
    pub(crate) async fn lock_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<SessionLockGuard, Error> {
        Ok(self
            .storage
            .session_lock(LockKey::Organization(organization_id), DEFAULT_LOCK_TIMEOUT)
            .await?)
    }

    /// Record that a self-hosted server fetched an entitlement. Publishes no
    /// entity event: check-ins are frequent and change nothing a user edits.
    pub async fn record_license_check_in(
        &self,
        organization_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), Error> {
        let lock = self.lock_organization(organization_id).await?;
        if let Some(mut organization) = self.get_by_id(&organization_id).await? {
            organization.base.license_checkin_at = Some(at);
            self.storage.update(&mut organization).await?;
        }
        lock.release().await?;
        Ok(())
    }

    /// The `iat` this org's online license key is signed with, assigned on
    /// first use. Holding it makes minting deterministic: every copy of the
    /// key returns the same string until the key is regenerated.
    pub async fn license_key_issued_at(
        &self,
        organization_id: Uuid,
    ) -> Result<DateTime<Utc>, Error> {
        let lock = self.lock_organization(organization_id).await?;
        let mut organization = self
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Organization {organization_id} not found"))?;

        let issued_at = match organization.base.license_key_issued_at {
            Some(issued_at) => issued_at,
            None => {
                // Whole seconds: the claim carries seconds, and Postgres keeps
                // microseconds, so a truncating round trip would otherwise hand
                // back a different stamp than the one just written.
                let issued_at = whole_second(Utc::now());
                organization.base.license_key_issued_at = Some(issued_at);
                self.storage.update(&mut organization).await?;
                tracing::info!(
                    organization_id = %organization_id,
                    key_version = organization.base.license_key_version,
                    "Issued online license key"
                );
                issued_at
            }
        };

        lock.release().await?;
        Ok(issued_at)
    }

    /// Make `key_type` the org's issued key, retiring the previous one when
    /// the type actually changes. A request for the type already issued is a
    /// re-read and changes nothing, so copying a key twice returns the same
    /// string.
    ///
    /// Retiring works by bumping the key version, which the entitlement
    /// endpoint checks. That kills an online key at once. An air-gapped key
    /// carries no version and never checks in, so a copy already installed on
    /// a server keeps working until its embedded expiry; the UI says so.
    pub async fn switch_license_key_type(
        &self,
        organization_id: Uuid,
        key_type: LicenseKeyType,
        authentication: AuthenticatedEntity,
    ) -> Result<Organization, Error> {
        let lock = self.lock_organization(organization_id).await?;
        let mut organization = self
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Organization {organization_id} not found"))?;

        if organization.base.license_key_type.unwrap_or_default() == key_type {
            lock.release().await?;
            return Ok(organization);
        }

        organization.base.license_key_type = Some(key_type);
        organization.base.license_key_version = organization
            .base
            .license_key_version
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("License key version overflow"))?;
        organization.base.license_key_issued_at = Some(whole_second(Utc::now()));
        let updated = self.update(&mut organization, authentication).await?;
        tracing::info!(
            organization_id = %organization_id,
            key_type = %key_type,
            key_version = updated.base.license_key_version,
            "Switched license key type"
        );

        lock.release().await?;
        Ok(updated)
    }

    /// Increment the org's license key version, retiring every online key
    /// issued so far.
    pub async fn regenerate_license_key(
        &self,
        organization_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<Organization, Error> {
        let lock = self.lock_organization(organization_id).await?;
        let mut organization = self
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Organization {organization_id} not found"))?;
        organization.base.license_key_version = organization
            .base
            .license_key_version
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("License key version overflow"))?;
        // Move the stamp too. The version alone already changes the key; this
        // keeps the stamp honest about when the current key was issued.
        organization.base.license_key_issued_at = Some(whole_second(Utc::now()));
        let updated = self.update(&mut organization, authentication).await?;
        tracing::info!(
            organization_id = %organization_id,
            key_version = updated.base.license_key_version,
            "Regenerated online license key"
        );
        lock.release().await?;
        Ok(updated)
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

    /// Reconcile self-hosted org plans to the `target` plan the license key
    /// entitles (resolved by the caller via `plan_for_license`). Every org whose
    /// current plan differs from `target` is moved onto it. The caller gates this
    /// on a self-hosted deployment (no Stripe secret) with `LicenseStatus::Valid`.
    ///
    /// Grandfathered customers hold claim-absent keys, which resolve to the same
    /// `CommercialSelfHosted` plan their orgs already carry, so this is a no-op
    /// for them. Idempotent — plans equal to `target` are skipped, so re-running
    /// on every boot does nothing once reconciled. The plan write goes through the
    /// `LicenseReconciled` billing event → this service's own
    /// `Subscriber<BillingOperation>` impl (the sole writer of
    /// `organizations.plan`); we never write the row here. Best-effort: a per-org
    /// publish failure is logged, not fatal. Returns the number of orgs moved.
    pub async fn reconcile_self_hosted_license_plans(
        &self,
        target: BillingPlan,
    ) -> Result<u64, Error> {
        let orgs = self.get_all(StorableFilter::<Organization>::new()).await?;

        let mut upgraded = 0u64;
        for org in orgs {
            // `None` resolves to the build default. Plan equality is config-based
            // (see `BillingPlan`'s `PartialEq`), so an org already on `target`
            // is skipped regardless of how its plan was set.
            let current = org.base.plan.unwrap_or_default();
            if current == target {
                continue;
            }

            if let Err(e) = self
                .event_bus()
                .publish(Event::new(
                    OrgScope {
                        organization_id: org.id,
                    },
                    BillingOperation::LicenseReconciled {
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
                    "Failed to publish license reconciliation for org",
                );
                continue;
            }
            upgraded += 1;
        }

        if upgraded > 0 {
            tracing::info!(
                count = upgraded,
                plan = %target.id(),
                "Reconciled self-hosted org plan(s) to license entitlement",
            );
        }

        Ok(upgraded)
    }

    /// Write the online license key's latest entitlement (`None` after the
    /// cloud rejected the key) and the check-in time to every org. The license
    /// is instance-level, so every row holds the same value. Written through
    /// storage, not `update`: this is server bookkeeping, and an `Updated`
    /// entity event per org on every check-in would carry nothing.
    pub async fn store_license_entitlement(
        &self,
        entitlement: Option<String>,
        checked_at: DateTime<Utc>,
    ) -> Result<(), Error> {
        for mut org in self.get_all(StorableFilter::<Organization>::new()).await? {
            org.base.license_entitlement = entitlement.clone();
            org.base.license_entitlement_at = Some(checked_at);
            self.storage.update(&mut org).await?;
        }
        Ok(())
    }

    /// The persisted entitlement with the latest check-in time across orgs,
    /// and that time.
    pub async fn load_license_entitlement(
        &self,
    ) -> Result<Option<(String, Option<DateTime<Utc>>)>, Error> {
        let orgs = self.get_all(StorableFilter::<Organization>::new()).await?;
        Ok(orgs
            .into_iter()
            .filter_map(|org| {
                Some((
                    org.base.license_entitlement?,
                    org.base.license_entitlement_at,
                ))
            })
            .max_by_key(|(_, checked_at)| *checked_at))
    }
}
