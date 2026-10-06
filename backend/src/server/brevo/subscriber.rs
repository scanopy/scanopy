//! Brevo subscriber for billing, onboarding, auth, discovery, and entity events.
//!
//! Syncs CRM state: company attributes (plan, status, lifecycle markers,
//! engagement counters) and contact attributes (email, role, marketing opt-in,
//! signup metadata). Routes auth `Register` to contact creation + DOI flow;
//! routes org/billing/discovery/entity events to company updates.
//!
//! Every handler processes its whole batch and reports failures as `NonRetryable`: a re-run
//! would repeat the syncs that already succeeded, and company creation and the DOI register
//! email are not idempotent.

use crate::{
    daemon::discovery::types::base::{DiscoveryPhase, DiscoveryPhaseDiscriminants},
    server::{
        auth::middleware::auth::AuthenticatedEntity,
        brevo::service::BrevoService,
        shared::{
            entities::EntityDiscriminants,
            events::{
                registry::SubscriberRegistration,
                traits::{
                    EntityEventFilter, Event, EventFilter, EventScope, NonRetryable,
                    ScopeOrganization, Subscriber,
                },
                types::{
                    AuthOperation, AuthOperationDiscriminants, BillingOperation, EntityOperation,
                    EntityOperationDiscriminants, OnboardingOperation,
                },
            },
        },
    },
};
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[async_trait]
impl Subscriber<BillingOperation> for BrevoService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        let mut failures = Vec::new();
        for event in &events {
            if event.operation.is_zero_dollar_notice() {
                tracing::debug!(
                    operation = %event.operation,
                    organization_id = %event.scope.organization_id,
                    "Skipping Brevo billing event for a customer paying nothing"
                );
                continue;
            }
            if let Err(e) = self.handle_billing_event(event).await {
                failures.push(anyhow!("billing {}: {e:#}", event.operation));
            }
        }
        NonRetryable::from_failures(failures).map_or(Ok(()), |e| Err(e.into()))
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<BrevoService, BillingOperation>());

#[async_trait]
impl Subscriber<OnboardingOperation> for BrevoService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        let mut failures = Vec::new();
        for event in &events {
            if let Err(e) = self.handle_onboarding_event(event).await {
                failures.push(anyhow!("onboarding {}: {e:#}", event.operation));
            }
        }
        NonRetryable::from_failures(failures).map_or(Ok(()), |e| Err(e.into()))
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<
    BrevoService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<AuthOperation> for BrevoService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::ops(vec![
            AuthOperationDiscriminants::LoginSuccess,
            AuthOperationDiscriminants::Register,
        ])
    }

    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        let mut failures = Vec::new();
        for event in &events {
            match &event.operation {
                AuthOperation::LoginSuccess { .. } => {
                    if let AuthenticatedEntity::User { email, user_id, .. } = &event.authentication
                        && let Err(e) = self
                            .update_contact_last_login(email.to_string(), *user_id)
                            .await
                    {
                        failures.push(anyhow!("login sync for user {user_id}: {e:#}"));
                    }
                }
                AuthOperation::Register { .. } => {
                    if let Err(e) = self.handle_register(event).await {
                        failures.push(anyhow!("register: {e:#}"));
                    }
                }
                _ => {}
            }
        }
        NonRetryable::from_failures(failures).map_or(Ok(()), |e| Err(e.into()))
    }
}
inventory::submit!(SubscriberRegistration::new::<BrevoService, AuthOperation>());

#[async_trait]
impl Subscriber<DiscoveryPhase> for BrevoService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::ops(vec![DiscoveryPhaseDiscriminants::Scanning])
    }

    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), Error> {
        let mut failures = Vec::new();
        for event in &events {
            if event.operation == DiscoveryPhase::Scanning
                && let Some(org_id) = self.get_org_id_from_site(&event.scope.site_id).await
                && let Err(e) = self.update_company_last_discovery(org_id).await
            {
                failures.push(anyhow!("discovery sync for org {org_id}: {e:#}"));
            }
        }
        NonRetryable::from_failures(failures).map_or(Ok(()), |e| Err(e.into()))
    }
}
inventory::submit!(SubscriberRegistration::new::<BrevoService, DiscoveryPhase>());

#[async_trait]
impl Subscriber<EntityOperation> for BrevoService {
    fn filter(&self) -> EntityEventFilter {
        let create_or_delete = Some(vec![
            EntityOperationDiscriminants::Created,
            EntityOperationDiscriminants::Deleted,
        ]);
        EntityEventFilter::by_entity(HashMap::from([
            (EntityDiscriminants::Site, create_or_delete.clone()),
            (EntityDiscriminants::Host, create_or_delete.clone()),
            (EntityDiscriminants::User, create_or_delete),
        ]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        // Aggregate org IDs whose entity counts changed; sync once per org.
        let mut org_ids_for_metrics: HashSet<Uuid> = HashSet::new();
        for event in &events {
            let org_id = match event.scope.organization() {
                Some(ScopeOrganization::Org(org_id)) => Some(org_id),
                Some(ScopeOrganization::Site(site_id)) => self.get_org_id_from_site(&site_id).await,
                None => None,
            };
            org_ids_for_metrics.extend(org_id);
        }

        let mut failures = Vec::new();
        for org_id in org_ids_for_metrics {
            if let Err(e) = self.sync_org_entity_metrics(org_id).await {
                failures.push(anyhow!("entity metrics sync for org {org_id}: {e:#}"));
            }
        }
        NonRetryable::from_failures(failures).map_or(Ok(()), |e| Err(e.into()))
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<BrevoService, EntityOperation>());
