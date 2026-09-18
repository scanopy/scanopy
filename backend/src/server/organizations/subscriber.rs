//! Organizations subscriber for OnboardingOperation and BillingOperation events.
//!
//! Onboarding: persists milestone discriminants onto `organizations.onboarding`
//! so the UI checklist renders without re-deriving from the event log.
//!
//! Billing: updates flag columns (`last_paused_at`, `trial_extended_used`,
//! `last_downgrade_at`, `last_downgrade_from_plan`) on the variants that drive
//! Phase 5 eligibility gates and the downgrade banner; mirrors
//! `BillingOperation::implied_status()` onto `organizations.plan_status` so
//! every billing event keeps the canonical status column in sync; and writes
//! `organizations.plan` + `trial_end_date` from the variants that establish
//! or change the current plan (`CheckoutCompleted`, `TrialStarted`,
//! `PlanChanged`, `TrialExtended`).

use anyhow::Error;
use async_trait::async_trait;

use strum::IntoDiscriminant;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    organizations::service::OrganizationService,
    shared::{
        events::{
            registry::SubscriberRegistration,
            traits::{Event, EventFilter, Subscriber},
            types::{BillingOperation, OnboardingOperation},
        },
        services::traits::CrudService,
    },
};

#[async_trait]
impl Subscriber<OnboardingOperation> for OrganizationService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        for event in events {
            if let Some(mut organization) = self.get_by_id(&event.scope.organization_id).await? {
                let onboarding_step = event.operation.discriminant();
                if organization.not_onboarded(&onboarding_step) {
                    organization.base.onboarding.push(onboarding_step);
                    self.update(&mut organization, AuthenticatedEntity::System)
                        .await?;
                }
            }
        }

        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    OrganizationService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<BillingOperation> for OrganizationService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        // Wildcard so every billing event participates in the plan_status
        // mirror below; per-variant flag updates still gate themselves.
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            let org_id = event.scope.organization_id;
            // This handler writes the whole row back, so hold the org lock
            // across the read and the write: otherwise a license key
            // regeneration landing in between would be reverted, reviving
            // the retired key. Dropping the guard (the `continue` below)
            // releases it.
            let lock = self.lock_organization(org_id).await?;
            let Some(mut organization) = self.get_by_id(&org_id).await? else {
                continue;
            };

            let implied = event.operation.implied_status();
            let mut changed = false;
            match &event.operation {
                BillingOperation::Paused { .. } => {
                    organization.base.last_paused_at = Some(event.timestamp);
                    changed = true;
                }
                BillingOperation::CheckoutCompleted {
                    plan,
                    next_renewal_at,
                    ..
                } => {
                    if organization.base.plan.as_ref() != Some(plan) {
                        organization.base.plan = Some(*plan);
                        changed = true;
                    }
                    if next_renewal_at.is_some()
                        && organization.base.next_renewal_at != *next_renewal_at
                    {
                        organization.base.next_renewal_at = *next_renewal_at;
                        changed = true;
                    }
                }
                BillingOperation::TrialStarted {
                    plan, trial_end, ..
                } => {
                    if organization.base.plan.as_ref() != Some(plan) {
                        organization.base.plan = Some(*plan);
                        changed = true;
                    }
                    if organization.base.trial_end_date != Some(*trial_end) {
                        organization.base.trial_end_date = Some(*trial_end);
                        changed = true;
                    }
                    // Stripe sets the trialing sub's current_period_end to
                    // trial_end; mirror it onto next_renewal_at so the UI
                    // shows "First invoice on <trial_end>".
                    if organization.base.next_renewal_at != Some(*trial_end) {
                        organization.base.next_renewal_at = Some(*trial_end);
                        changed = true;
                    }
                    // A self-hosted trial licenses its servers until trial end.
                    if plan.license_plan().is_some()
                        && organization.base.license_paid_through != Some(*trial_end)
                    {
                        organization.base.license_paid_through = Some(*trial_end);
                        changed = true;
                    }
                }
                BillingOperation::TrialExtended { new_trial_end, .. } => {
                    if !organization.base.trial_extended_used {
                        organization.base.trial_extended_used = true;
                        changed = true;
                    }
                    if organization.base.trial_end_date != Some(*new_trial_end) {
                        organization.base.trial_end_date = Some(*new_trial_end);
                        changed = true;
                    }
                    // Trial extension shifts the trialing sub's period_end too.
                    if organization.base.next_renewal_at != Some(*new_trial_end) {
                        organization.base.next_renewal_at = Some(*new_trial_end);
                        changed = true;
                    }
                    if organization
                        .base
                        .plan
                        .is_some_and(|plan| plan.license_plan().is_some())
                        && organization.base.license_paid_through != Some(*new_trial_end)
                    {
                        organization.base.license_paid_through = Some(*new_trial_end);
                        changed = true;
                    }
                }
                BillingOperation::TrialEnded {
                    converted: true,
                    next_renewal_at,
                    ..
                } => {
                    // Trial converted to paid; Stripe re-anchored
                    // current_period_end. Mirror it.
                    if next_renewal_at.is_some()
                        && organization.base.next_renewal_at != *next_renewal_at
                    {
                        organization.base.next_renewal_at = *next_renewal_at;
                        changed = true;
                    }
                }
                BillingOperation::PlanChanged {
                    from,
                    to,
                    is_downgrade,
                    next_renewal_at,
                } => {
                    if organization.base.plan.as_ref() != Some(to) {
                        organization.base.plan = Some(*to);
                        changed = true;
                    }
                    if *is_downgrade {
                        organization.base.last_downgrade_at = Some(event.timestamp);
                        organization.base.last_downgrade_from_plan = Some(*from);
                        changed = true;
                    }
                    if next_renewal_at.is_some()
                        && organization.base.next_renewal_at != *next_renewal_at
                    {
                        organization.base.next_renewal_at = *next_renewal_at;
                        changed = true;
                    }
                    // Switching onto a self-hosted plan mid-trial keeps the
                    // trial running and raises no invoice, so the trial end
                    // is the only paid-through date there will be until the
                    // first invoice is paid.
                    if to.license_plan().is_some()
                        && organization.base.plan_status
                            == Some(crate::server::billing::types::base::PlanStatus::Trialing)
                        && let Some(trial_end) = organization.base.trial_end_date
                        && organization.base.license_paid_through != Some(trial_end)
                    {
                        organization.base.license_paid_through = Some(trial_end);
                        changed = true;
                    }
                }
                BillingOperation::PaymentSucceeded { invoice, .. } => {
                    if let Some(paid_through) = invoice.license_paid_through()
                        && organization.base.license_paid_through != Some(paid_through)
                    {
                        organization.base.license_paid_through = Some(paid_through);
                        changed = true;
                    }
                }
                BillingOperation::InvoiceIssued { invoice } => {
                    // A buyer paying by invoice deploys as soon as it is issued.
                    // Never shortens a period already paid for or granted.
                    if let Some(provisional) = invoice.provisional_paid_through()
                        && organization
                            .base
                            .license_paid_through
                            .is_none_or(|current| current < provisional)
                    {
                        organization.base.license_paid_through = Some(provisional);
                        changed = true;
                    }
                    // Paying against a purchase order is how this org pays, so
                    // nothing should ask it for a card.
                    if !organization.base.bills_by_invoice {
                        organization.base.bills_by_invoice = true;
                        changed = true;
                    }
                }
                BillingOperation::InvoiceVoided { invoice } => {
                    // Take back only the grant this invoice made: a later
                    // payment or a newer invoice has since moved the date.
                    if let (Some(provisional), Some(unpaid_from)) = (
                        invoice.provisional_paid_through(),
                        invoice.license_unpaid_from(),
                    ) && organization.base.license_paid_through == Some(provisional)
                    {
                        organization.base.license_paid_through = Some(unpaid_from);
                        changed = true;
                    }
                }
                BillingOperation::LicenseReconciled { to, .. } => {
                    // Move the org's stored plan to the license-resolved tier,
                    // in either direction. Idempotent: the reconcile pass only
                    // emits this when the plans differ, and the guard here makes
                    // a re-emission a no-op regardless. No renewal/downgrade
                    // bookkeeping — self-hosted plans carry no Stripe subscription.
                    if organization.base.plan.as_ref() != Some(to) {
                        organization.base.plan = Some(*to);
                        changed = true;
                    }
                }
                BillingOperation::Reactivated {
                    next_renewal_at, ..
                } => {
                    if next_renewal_at.is_some()
                        && organization.base.next_renewal_at != *next_renewal_at
                    {
                        organization.base.next_renewal_at = *next_renewal_at;
                        changed = true;
                    }
                }
                BillingOperation::PaymentRecovered {
                    next_renewal_at, ..
                } => {
                    if next_renewal_at.is_some()
                        && organization.base.next_renewal_at != *next_renewal_at
                    {
                        organization.base.next_renewal_at = *next_renewal_at;
                        changed = true;
                    }
                }
                BillingOperation::SubscriptionCancelled { plan, .. }
                | BillingOperation::TrialEnded {
                    converted: false,
                    plan,
                    ..
                } => {
                    // A full cancellation / unconverted trial always downgrades
                    // the org to Free. The cancel-side-effects path used to chain
                    // a separate PlanChanged event for this; we now do the write
                    // here so the downgrade is owned by the source event. The
                    // implied_status mirror below sets plan_status = Active (Free
                    // is an active plan) in the same write — one owner, one write.
                    let free_plan = crate::server::billing::plans::get_free_plan();
                    organization.base.last_downgrade_at = Some(event.timestamp);
                    organization.base.last_downgrade_from_plan = Some(*plan);
                    if organization.base.plan.as_ref() != Some(&free_plan) {
                        organization.base.plan = Some(free_plan);
                    }
                    // The subscription that billed by invoice is gone, so the
                    // org has no standing way to pay by invoice either. Unlike
                    // a saved card (below), invoice billing is a property of
                    // the subscription, not of the customer.
                    if organization.base.bills_by_invoice {
                        organization.base.bills_by_invoice = false;
                    }
                    // NOTE: do NOT touch `has_payment_method` here. Cancelling a
                    // subscription does not detach the customer's saved cards;
                    // the flag's sole authoritative writers are
                    // `PaymentMethodAdded` / `PaymentMethodRemoved` (driven by
                    // the Stripe `payment_method.attached`/`detached` webhooks).
                    // Resetting it on cancel/downgrade left it stale-false after
                    // downgrade-to-Free or resubscribe-without-trial.
                    // Subscription is gone; clear the renewal mirror.
                    if organization.base.next_renewal_at.is_some() {
                        organization.base.next_renewal_at = None;
                    }
                    // Drop any active save-offer discount — the subscription it
                    // applied to is gone, so leaving these set would show a stale
                    // "discount active" chip and could re-apply on resubscribe.
                    // `last_discount_at` is deliberately preserved so the
                    // once-per-org eligibility gate still blocks a second
                    // discount. The Stripe-side discount is removed in
                    // `BillingService::handle_subscription_deleted`.
                    organization.base.discount_save_offer_active_until = None;
                    organization.base.discount_save_offer_percent_off = None;
                    changed = true;
                }
                BillingOperation::DiscountApplied {
                    percent_off,
                    expires_at,
                } => {
                    organization.base.last_discount_at = Some(event.timestamp);
                    organization.base.discount_save_offer_percent_off = Some(*percent_off);
                    organization.base.discount_save_offer_active_until = Some(*expires_at);
                    changed = true;
                }
                BillingOperation::PaymentMethodAdded => {
                    if !organization.base.has_payment_method {
                        organization.base.has_payment_method = true;
                        changed = true;
                    }
                    // Saving a card puts the subscription back on automatic
                    // charges (`finalize_payment_method`), so it no longer
                    // bills by invoice.
                    if organization.base.bills_by_invoice {
                        organization.base.bills_by_invoice = false;
                        changed = true;
                    }
                }
                BillingOperation::PaymentMethodRemoved => {
                    if organization.base.has_payment_method {
                        organization.base.has_payment_method = false;
                        changed = true;
                    }
                }
                BillingOperation::StripeCustomerCreated { customer_id } => {
                    if organization.base.stripe_customer_id.as_deref() != Some(customer_id.as_str())
                    {
                        organization.base.stripe_customer_id = Some(customer_id.clone());
                        changed = true;
                    }
                }
                _ => {}
            }

            // Mirror the canonical PlanStatus implied by every billing
            // operation onto `plan_status`. Single source of truth via
            // `BillingOperation::implied_status()`; downstream consumers
            // (auth gates, BillingTab pills, Brevo sync) read the typed
            // enum, set in one place.
            if let Some(status) = implied {
                let new_status = Some(status);
                if organization.base.plan_status != new_status {
                    organization.base.plan_status = new_status;
                    changed = true;
                }
            }

            if changed {
                self.update(&mut organization, AuthenticatedEntity::System)
                    .await?;
            }
            lock.release().await?;
        }

        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    OrganizationService,
    BillingOperation,
>());
