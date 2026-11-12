//! The hourly trial sweep: reports an Issue or first step left open 24 hours on a trialing org, so
//! someone can reach out while the trial runs. It also records the Setup items the org has done.

use std::collections::BTreeSet;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

use super::service::SetupService;
use super::types::{OpenSetupItem, SetupItemKey, SetupOperation};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    billing::types::base::{BillingPlan, PlanStatus},
    organizations::r#impl::base::{OrgNotifications, Organization},
    shared::{
        events::{
            traits::{Event, OrgScope},
            types::UpdateReason,
        },
        services::traits::{CrudService, EventBusService},
        storage::filter::StorableFilter,
    },
};

/// How long an item stays open before the sweep reports it.
const STUCK_AFTER: Duration = Duration::hours(24);

impl SetupService {
    /// Sweep every trialing org's Setup list at `now`. Returns the events published. One org's
    /// failure is logged and the sweep moves on.
    pub async fn sweep_trials(&self, now: DateTime<Utc>) -> Result<usize> {
        let trialing = self
            .organization_service
            .get_all(StorableFilter::<Organization>::new().plan_status(PlanStatus::Trialing))
            .await?;
        let mut published = 0;
        for org in trialing {
            let organization_id = org.id;
            match self.sweep_trial(org, now).await {
                Ok(count) => published += count,
                Err(e) => tracing::warn!(
                    organization_id = %organization_id,
                    error = %e,
                    "Setup trial sweep failed for an organization"
                ),
            }
        }
        Ok(published)
    }

    async fn sweep_trial(&self, org: Organization, now: DateTime<Utc>) -> Result<usize> {
        let Some(trial_end) = org.base.trial_end_date else {
            return Ok(0);
        };
        let mut ctx = self.gather(&org, None).await?;
        self.record_newly_done(&mut ctx, org.id, &AuthenticatedEntity::System)
            .await?;
        let listed: BTreeSet<SetupItemKey> = Self::list(&ctx)
            .into_iter()
            .filter(|entry| entry.alerts_when_stuck)
            .map(|entry| entry.key)
            .collect();

        let Some(mut locked) = self.organization_service.lock_for_update(org.id).await? else {
            return Ok(0);
        };
        let before = locked.org.base.notifications.clone();
        let plan = locked.org.base.plan;
        let operations = sweep(
            &mut locked.org.base.notifications,
            &listed,
            now,
            &plan,
            trial_end,
        );
        if locked.org.base.notifications == before {
            locked.release().await?;
        } else {
            locked
                .save(
                    AuthenticatedEntity::System,
                    UpdateReason::OrganizationNotificationRecorded,
                )
                .await?;
        }

        let count = operations.len();
        for operation in operations {
            self.organization_service
                .event_bus()
                .publish(Event::new(
                    OrgScope {
                        organization_id: org.id,
                    },
                    operation,
                    AuthenticatedEntity::System,
                ))
                .await;
        }
        Ok(count)
    }
}

/// Advance `notifications` to the keys `listed` now, and return what to report: Resolved for
/// each open key no longer listed, and Stuck for each key open `STUCK_AFTER` and never reported.
pub(crate) fn sweep(
    notifications: &mut OrgNotifications,
    listed: &BTreeSet<SetupItemKey>,
    now: DateTime<Utc>,
    plan: &BillingPlan,
    trial_end: DateTime<Utc>,
) -> Vec<SetupOperation> {
    let hours_open = |open_since: DateTime<Utc>| (now - open_since).num_hours().max(0) as u32;
    let mut operations = Vec::new();

    let (still_open, cleared): (Vec<OpenSetupItem>, Vec<OpenSetupItem>) =
        std::mem::take(&mut notifications.setup_items_open)
            .into_iter()
            .partition(|open| listed.contains(&open.key));
    for open in cleared {
        operations.push(SetupOperation::Resolved {
            tier: open.key.item_type().tier(),
            hours_open: hours_open(open.open_since),
            key: open.key,
        });
    }
    notifications.setup_items_open = still_open;

    for key in listed {
        if !notifications.setup_items_open.iter().any(|o| &o.key == key) {
            notifications.setup_items_open.push(OpenSetupItem {
                key: key.clone(),
                open_since: now,
            });
        }
    }

    for open in &notifications.setup_items_open {
        if now - open.open_since >= STUCK_AFTER
            && !notifications.setup_items_reported.contains(&open.key)
        {
            notifications.setup_items_reported.push(open.key.clone());
            operations.push(SetupOperation::Stuck {
                key: open.key.clone(),
                tier: open.key.item_type().tier(),
                plan: *plan,
                trial_end,
                hours_open: hours_open(open.open_since),
            });
        }
    }
    operations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{
        billing::plans::get_enterprise_plan,
        setup::types::{SetupItemDiscriminants, SetupOperationDiscriminants},
    };
    use strum::IntoDiscriminant;

    fn install_daemon() -> SetupItemKey {
        SetupItemKey::Org {
            item_type: SetupItemDiscriminants::InstallDaemon,
        }
    }

    /// Runs the sweep against one org's bookkeeping, starting at `start`.
    struct Org {
        notifications: OrgNotifications,
        start: DateTime<Utc>,
    }

    impl Org {
        fn new() -> Self {
            Self {
                notifications: OrgNotifications::default(),
                start: crate::server::shared::clock::now(),
            }
        }

        /// The operations the sweep `hours` after the start reports, with `listed` open.
        fn at(&mut self, hours: i64, listed: &[SetupItemKey]) -> Vec<SetupOperationDiscriminants> {
            let now = self.start + Duration::hours(hours);
            sweep(
                &mut self.notifications,
                &listed.iter().cloned().collect(),
                now,
                &get_enterprise_plan(),
                now + Duration::days(10),
            )
            .iter()
            .map(|op| op.discriminant())
            .collect()
        }
    }

    #[test]
    fn an_item_is_reported_stuck_at_24_hours_and_not_before() {
        let mut org = Org::new();
        let open = [install_daemon()];
        assert!(org.at(0, &open).is_empty());
        assert!(org.at(23, &open).is_empty());
        assert_eq!(org.at(24, &open), [SetupOperationDiscriminants::Stuck]);
    }

    #[test]
    fn a_stuck_item_is_reported_once() {
        let mut org = Org::new();
        let open = [install_daemon()];
        org.at(0, &open);
        org.at(24, &open);
        assert!(org.at(25, &open).is_empty());
        assert!(org.at(72, &open).is_empty());
    }

    #[test]
    fn an_item_that_clears_is_resolved_with_its_hours_open() {
        let mut org = Org::new();
        org.at(0, &[install_daemon()]);
        let now = org.start + Duration::hours(5);
        let operations = sweep(
            &mut org.notifications,
            &BTreeSet::new(),
            now,
            &get_enterprise_plan(),
            now,
        );
        assert!(matches!(
            operations.as_slice(),
            [SetupOperation::Resolved { hours_open: 5, key, .. }] if *key == install_daemon()
        ));
        assert!(org.notifications.setup_items_open.is_empty());
    }

    #[test]
    fn a_reopened_item_is_not_reported_stuck_again() {
        let mut org = Org::new();
        let open = [install_daemon()];
        org.at(0, &open);
        org.at(24, &open);
        assert_eq!(org.at(30, &[]), [SetupOperationDiscriminants::Resolved]);
        assert!(org.at(31, &open).is_empty());
        assert!(org.at(60, &open).is_empty());
    }

    /// Each key keeps its own clock: a second item opening later is reported a day after it
    /// opened, not with the first.
    #[test]
    fn each_item_is_timed_from_when_it_opened() {
        let mut org = Org::new();
        let daemon = SetupItemKey::Entity {
            item_type: SetupItemDiscriminants::DaemonHealth,
            entity_id: uuid::Uuid::new_v4(),
        };
        org.at(0, &[install_daemon()]);
        assert_eq!(
            org.at(24, &[install_daemon(), daemon.clone()]),
            [SetupOperationDiscriminants::Stuck]
        );
        assert_eq!(
            org.at(48, &[install_daemon(), daemon]),
            [SetupOperationDiscriminants::Stuck]
        );
    }

    /// A sweep's key lands as single-level metadata fields, so PostHog breaks down on them.
    #[test]
    fn a_stuck_event_sends_its_key_as_flat_fields() {
        let daemon_id = uuid::Uuid::new_v4();
        let operation = SetupOperation::Stuck {
            key: SetupItemKey::RunWarning {
                daemon_id,
                code:
                    crate::daemon::discovery::types::warnings::DiscoveryWarningCode::ScanTimeLimit,
            },
            tier: SetupItemDiscriminants::RunWarnings.tier(),
            plan: get_enterprise_plan(),
            trial_end: crate::server::shared::clock::now(),
            hours_open: 24,
        };
        let sent = serde_json::to_value(operation.metadata()).unwrap();
        assert_eq!(sent["type"], "Stuck");
        assert_eq!(sent["item_type"], "RunWarnings");
        assert_eq!(sent["daemon_id"], daemon_id.to_string());
        assert!(sent["warning_code"].is_string());
        assert_eq!(sent["tier"], "Issue");
        assert!(sent["plan"]["type"].is_string());
        assert_eq!(sent["hours_open"], 24);
    }
}
