//! Billing subscribers: Site/User Created/Deleted entity events, and `OrgCreated`.
//!
//! Drives billing-side bookkeeping when tenant resources change: seat counts,
//! Stripe metered usage, plan-limit enforcement. Starts a new cloud org's trial.

use anyhow::Error;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::server::{
    billing::service::BillingService,
    shared::{
        entities::EntityDiscriminants,
        events::{
            registry::SubscriberRegistration,
            traits::{
                EntityEventFilter, Event, EventFilter, EventScope, ScopeOrganization, Subscriber,
            },
            types::{
                AuthOperation, AuthOperationDiscriminants, EntityOperation,
                EntityOperationDiscriminants,
            },
        },
        services::traits::CrudService,
        storage::filter::StorableFilter,
    },
    sites::r#impl::Site,
    users::r#impl::base::User,
};

#[async_trait]
impl Subscriber<EntityOperation> for BillingService {
    fn filter(&self) -> EntityEventFilter {
        let create_or_delete = Some(vec![
            EntityOperationDiscriminants::Created,
            EntityOperationDiscriminants::Deleted,
        ]);
        EntityEventFilter::by_entity(HashMap::from([
            (EntityDiscriminants::Site, create_or_delete.clone()),
            (EntityDiscriminants::User, create_or_delete),
        ]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        for event in events {
            // Resolve the org_id from the event scope (org-scoped entity) or
            // from the site (site-scoped entity).
            let org_id = match event.scope.organization() {
                Some(ScopeOrganization::Org(org_id)) => org_id,
                Some(ScopeOrganization::Site(site_id)) => {
                    match self.site_service.get_by_id(&site_id).await? {
                        Some(site) => site.base.organization_id,
                        None => continue,
                    }
                }
                None => continue,
            };

            let Some(org) = self.organization_service.get_by_id(&org_id).await? else {
                continue;
            };

            let site_filter = StorableFilter::<Site>::new_from_org_id(&org_id);
            let user_filter = StorableFilter::<User>::new_from_org_id(&org_id);

            let site_count = self.site_service.get_all(site_filter).await?.len();
            let seat_count = self.user_service.get_all(user_filter).await?.len();

            let plan = org.base.plan;
            if plan.config().seat_cents.is_none() && plan.config().site_cents.is_none() {
                continue;
            }
            // A lapsed org has no live subscription to carry add-on
            // quantities; the one entity change it can still make (deleting
            // the org) would otherwise fail here looking for one.
            if org.is_lapsed() {
                continue;
            }

            self.update_addon_prices(org, site_count as u64, seat_count as u64)
                .await?;
        }

        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<BillingService, EntityOperation>());

/// Starts a self-hosted license buyer's trial at signup, on the license plan the org was created
/// with, so the key is ready without a plan pick. A cloud signup starts on Free and starts its own
/// trial from the plan picker. Only cloud runs a `BillingService`. A failure leaves the org on its
/// license plan with no subscription, which the app shows with the plan picker.
#[async_trait]
impl Subscriber<AuthOperation> for BillingService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::ops(vec![AuthOperationDiscriminants::OrgCreated])
    }

    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        for event in events {
            // Signup names no site only for a license buyer.
            let AuthOperation::OrgCreated { site: None, .. } = &event.operation else {
                continue;
            };
            let Some(organization_id) = event.scope.organization_id else {
                continue;
            };
            if let Err(e) = self
                .start_default_trial(organization_id, event.authentication.clone())
                .await
            {
                tracing::warn!(
                    %organization_id,
                    error = %e,
                    "Could not start the license trial at signup; the org stays on the plan picker"
                );
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<BillingService, AuthOperation>());
