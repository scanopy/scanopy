//! How each billing event changes the organization row.
//!
//! Updates flag columns (`last_paused_at`, `trial_extended_used`,
//! `last_downgrade_at`, `last_downgrade_from_plan`) on the variants that drive
//! Phase 5 eligibility gates and the downgrade banner; mirrors
//! `BillingOperation::implied_status()` onto `organizations.plan_status` so
//! every billing event keeps the canonical status column in sync; and writes
//! `organizations.plan` + `trial_end_date` from the variants that establish
//! or change the current plan (`CheckoutCompleted`, `TrialStarted`,
//! `PlanChanged`, `TrialExtended`).
//!
//! The code that decides a billing event applies it to the org row it holds
//! locked, writes the row, and only then publishes the event. See
//! `OrganizationService::commit_billing`.

use anyhow::Error;
use chrono::{DateTime, Utc};
use strum::IntoDiscriminant;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    billing::types::base::PlanStatus,
    organizations::{
        r#impl::base::Organization,
        service::{LockedOrganization, OrganizationService},
    },
    shared::{
        events::{
            traits::Event,
            types::{BillingOperation, OnboardingOperation, TrialExtensionSource, UpdateReason},
        },
        services::traits::EventBusService,
    },
};

impl OrganizationService {
    /// Apply billing events to the locked organization, write it, then publish them.
    ///
    /// The caller decides `events` from `locked.org`, so reading the prior state,
    /// choosing the transition and writing it are one step under the org lock: a
    /// second webhook for the same org waits, then reads the committed row. Each
    /// event changes the row through [`BillingOperation::apply`], in order, so a
    /// later event sees what an earlier one wrote. `milestone` is recorded on the
    /// same write and published, ahead of the billing events, only when this call
    /// added it. Saved as `System` with an `Updated` event when anything changed.
    pub async fn commit_billing(
        &self,
        mut locked: LockedOrganization<'_>,
        milestone: Option<Event<OnboardingOperation>>,
        events: Vec<Event<BillingOperation>>,
    ) -> Result<(), Error> {
        let milestone = milestone.filter(|event| {
            let step = event.operation.discriminant();
            let new = locked.org.not_onboarded(&step);
            if new {
                locked.org.base.onboarding.push(step);
            }
            new
        });
        let mut changed = milestone.is_some();
        for event in &events {
            changed |= event.operation.apply(&mut locked.org, event.timestamp);
        }
        if changed {
            locked
                .save(
                    AuthenticatedEntity::System,
                    UpdateReason::OrganizationBillingSynced,
                )
                .await?;
        } else {
            locked.release().await?;
        }

        if let Some(milestone) = milestone {
            self.event_bus().publish(milestone).await;
        }
        for event in events {
            self.event_bus().publish(event).await;
        }
        Ok(())
    }

    /// Lock the organization and [`commit_billing`](Self::commit_billing) `events`, for a caller
    /// whose events don't depend on the row's current state. `false`, publishing nothing, when
    /// the organization doesn't exist.
    pub async fn apply_billing(
        &self,
        organization_id: Uuid,
        events: Vec<Event<BillingOperation>>,
    ) -> Result<bool, Error> {
        let Some(locked) = self.lock_for_update(organization_id).await? else {
            return Ok(false);
        };
        self.commit_billing(locked, None, events).await?;
        Ok(true)
    }
}

impl BillingOperation {
    /// Apply this event to `organization`. `at` is when the event happened,
    /// for the columns that record it. Returns whether anything changed.
    pub fn apply(&self, organization: &mut Organization, at: DateTime<Utc>) -> bool {
        let implied = self.implied_status();
        let mut landed_on_free = false;
        let mut changed = false;
        match self {
            BillingOperation::Paused { .. } => {
                organization.base.last_paused_at = Some(at);
                changed = true;
            }
            BillingOperation::CheckoutCompleted {
                plan,
                next_renewal_at,
                ..
            } => {
                if organization.base.plan != *plan {
                    organization.base.plan = *plan;
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
                if organization.base.plan != *plan {
                    organization.base.plan = *plan;
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
            BillingOperation::TrialExtended {
                new_trial_end,
                source,
                ..
            } => {
                // The flag is the org's one self-serve extension. Staff
                // extending the trial, or a plan switch lengthening it, leaves
                // it available.
                if *source == TrialExtensionSource::User && !organization.base.trial_extended_used {
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
                if organization.base.plan.license_plan().is_some()
                    && organization.base.license_paid_through != Some(*new_trial_end)
                {
                    organization.base.license_paid_through = Some(*new_trial_end);
                    changed = true;
                }
            }
            BillingOperation::TrialEnded {
                next_renewal_at, ..
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
                license_key_type: _,
            } => {
                if organization.base.plan != *to {
                    organization.base.plan = *to;
                    changed = true;
                }
                if *is_downgrade {
                    organization.base.last_downgrade_at = Some(at);
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
                    && organization.base.plan_status == Some(PlanStatus::Trialing)
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
                // Never shortens a period already paid for or granted. An
                // org that defaulted on an earlier invoice cannot reach
                // here: issuing one needs payment terms, and the write-off
                // at cancellation takes those away until the debt settles.
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
            BillingOperation::LicenseResumed {
                plan,
                resumed_through,
                next_renewal_at,
                ..
            } => {
                // Settling the written-off invoice puts the org back on
                // its plan with the service it was still owed. Set the
                // date rather than only extending it: `PaymentSucceeded`
                // has already been applied to this organization and
                // restored the invoice's own term end, which is the past
                // for anyone settling after it expired.
                if organization.base.plan != *plan {
                    organization.base.plan = *plan;
                    changed = true;
                }
                if organization.base.license_paid_through != Some(*resumed_through) {
                    organization.base.license_paid_through = Some(*resumed_through);
                    changed = true;
                }
                if next_renewal_at.is_some()
                    && organization.base.next_renewal_at != *next_renewal_at
                {
                    organization.base.next_renewal_at = *next_renewal_at;
                    changed = true;
                }
                // The replacement subscription bills by sent invoice, so
                // nothing should ask this org for a card.
                if !organization.base.bills_by_invoice {
                    organization.base.bills_by_invoice = true;
                    changed = true;
                }
            }
            BillingOperation::LicenseReconciled { to, .. } => {
                // Move the org's stored plan to the license-resolved tier,
                // in either direction. Idempotent: the reconcile pass only
                // emits this when the plans differ, and the guard here makes
                // a re-emission a no-op regardless. No renewal/downgrade
                // bookkeeping — self-hosted plans carry no Stripe subscription.
                if organization.base.plan != *to {
                    organization.base.plan = *to;
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
            BillingOperation::SubscriptionCancelled { .. } => {
                // A cloud plan, trialing or not, lands on Free: the org keeps
                // using the app within Free's limits, with no status because
                // there is no subscription, and `last_downgrade_*` records
                // where it came from. A license plan stays as it is, and the
                // implied_status mirror below sets plan_status = Cancelled,
                // which makes the org read-only until it chooses a paid plan;
                // its `license_paid_through` is left alone so the key runs out
                // on the schedule it was issued with.
                if organization.base.plan.license_plan().is_none()
                    && !organization.base.plan.is_free()
                {
                    organization.base.last_downgrade_from_plan = Some(organization.base.plan);
                    organization.base.last_downgrade_at = Some(at);
                    organization.base.plan = crate::server::billing::plans::get_free_plan();
                    landed_on_free = true;
                }
                //
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
                // Resetting it on cancel left it stale-false after a
                // resubscribe-without-trial.
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
                ..
            } => {
                organization.base.last_discount_at = Some(at);
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
            BillingOperation::StripeCustomerCreated { customer_id, .. } => {
                if organization.base.stripe_customer_id.as_deref() != Some(customer_id.as_str()) {
                    organization.base.stripe_customer_id = Some(customer_id.clone());
                    changed = true;
                }
            }
            _ => {}
        }

        // Mirror the canonical PlanStatus implied by every billing
        // operation onto `plan_status`. Single source of truth via
        // `BillingOperation::implied_status()`; downstream consumers
        // (auth gates, BillingTab pills, PostHog properties) read the typed
        // enum, set in one place.
        // A cancellation that landed on Free leaves no subscription to have a
        // status.
        let implied = if landed_on_free { None } else { implied };
        if landed_on_free && organization.base.plan_status.is_some() {
            organization.base.plan_status = None;
            changed = true;
        }
        if let Some(status) = implied {
            let new_status = Some(status);
            if organization.base.plan_status != new_status {
                organization.base.plan_status = new_status;
                changed = true;
            }
        }

        changed
    }
}
