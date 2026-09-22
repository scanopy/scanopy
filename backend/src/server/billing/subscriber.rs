//! Billing subscriber for Network/User Created/Deleted entity events.
//!
//! Drives billing-side bookkeeping when tenant resources change: seat counts,
//! Stripe metered usage, plan-limit enforcement.

use anyhow::Error;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::server::{
    billing::service::BillingService,
    networks::r#impl::Network,
    shared::{
        entities::EntityDiscriminants,
        events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, Subscriber},
            types::{EntityOperation, EntityOperationDiscriminants},
        },
        services::traits::CrudService,
        storage::filter::StorableFilter,
    },
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
            (EntityDiscriminants::Network, create_or_delete.clone()),
            (EntityDiscriminants::User, create_or_delete),
        ]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        for event in events {
            // Resolve the org_id from the event scope (org-scoped entity) or
            // from the network (network-scoped entity).
            let org_id = if let Some(org_id) = event.scope.organization_id() {
                org_id
            } else if let Some(network_id) = event.scope.network_id() {
                match self.network_service.get_by_id(&network_id).await? {
                    Some(network) => network.base.organization_id,
                    None => continue,
                }
            } else {
                continue;
            };

            let Some(org) = self.organization_service.get_by_id(&org_id).await? else {
                continue;
            };

            let network_filter = StorableFilter::<Network>::new_from_org_id(&org_id);
            let user_filter = StorableFilter::<User>::new_from_org_id(&org_id);

            let network_count = self.network_service.get_all(network_filter).await?.len();
            let seat_count = self.user_service.get_all(user_filter).await?.len();

            let plan = org
                .base
                .plan
                .unwrap_or_else(crate::server::billing::plans::get_free_plan);
            if plan.config().seat_cents.is_none() && plan.config().network_cents.is_none() {
                continue;
            }
            // A lapsed org has no live subscription to carry add-on
            // quantities; the one entity change it can still make (deleting
            // the org) would otherwise fail here looking for one.
            if org.is_lapsed() {
                continue;
            }

            self.update_addon_prices(org, network_count as u64, seat_count as u64)
                .await?;
        }

        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<BillingService, EntityOperation>());
