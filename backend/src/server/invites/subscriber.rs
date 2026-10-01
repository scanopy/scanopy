//! InviteService subscriber for BillingOperation events.
//!
//! Revokes an org's outstanding invites when its subscription is cancelled.
//! Decoupled from BillingService so a billing-side state transition doesn't
//! need to hold a direct `Arc<InviteService>`.

use anyhow::Error;
use async_trait::async_trait;

use crate::server::{
    invites::service::InviteService,
    shared::events::{
        registry::SubscriberRegistration,
        traits::{Event, EventFilter, Subscriber},
        types::{BillingOperation, BillingOperationDiscriminants},
    },
};

#[async_trait]
impl Subscriber<BillingOperation> for InviteService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::ops(vec![BillingOperationDiscriminants::SubscriptionCancelled])
    }

    /// Revoking is an idempotent delete, so a failure is a plain `Err` and the bus retries the
    /// whole batch. Every event is still attempted first.
    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        let mut first_error = None;
        for event in events {
            if matches!(
                event.operation,
                BillingOperation::SubscriptionCancelled { .. }
            ) {
                let org_id = event.scope.organization_id;
                if let Err(e) = self.revoke_org_invites(&org_id).await {
                    first_error.get_or_insert(e.context(format!(
                        "revoke invites for org {org_id} on subscription cancellation"
                    )));
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}
inventory::submit!(SubscriberRegistration::new::<InviteService, BillingOperation>());
