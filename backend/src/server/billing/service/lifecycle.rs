//! Pause/resume, trial extension, cancellation/reactivation, save offers, and invoice-paid handling.
use super::*;

impl BillingService {
    /// Pause subscription billing for the requested duration. Eligibility:
    /// rolling 6-month cooldown anchored on `last_paused_at`.
    ///
    /// Pattern A: this endpoint calls Stripe (with the duration stashed in
    /// `metadata.scanopy_pause_duration_days`) and returns. The resulting
    /// `customer.subscription.updated` webhook detects the transition and
    /// emits `BillingOperation::Paused`. The org subscriber then mirrors
    /// `plan_status` to `paused` via `implied_status`.
    pub async fn pause_subscription(
        &self,
        organization_id: Uuid,
        duration: PauseDuration,
        _authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;

        if let Some(last) = organization.base.last_paused_at {
            let cooldown_end = last + chrono::Duration::days(180);
            if cooldown_end > Utc::now() {
                return Err(anyhow!(
                    "You last paused on {}. You can pause again on {}.",
                    last.format("%B %-d, %Y"),
                    cooldown_end.format("%B %-d, %Y")
                ));
            }
        }

        // Eligibility gates on our typed `plan_status` (the DB source of truth,
        // updated from Stripe webhooks) rather than the live Stripe
        // `sub.status`. The cancel modal hides pause while `plan_status ===
        // 'trialing'`, so the UI never brings a trialing user here; this
        // server-side gate enforces the same for direct API hits. Paused /
        // past_due / pending_cancellation / cancelled are also rejected.
        // An air-gapped key keeps validating on the customer's own server for
        // as long as it has left to run, so a pause here would freeze the
        // billing and none of the access. Nothing can call that key back.
        if organization.base.license_key_type == Some(LicenseKeyType::Offline) {
            return Err(crate::server::shared::types::api::ValidationError::new(
                "Pausing is not available while you hold an air-gapped license key, because the \
                 key on your server keeps working until it expires.",
            )
            .into());
        }

        // The cancel modal offers pause to no self-hosted plan. This holds a
        // direct API call to the same rule: the paused and resumed emails
        // describe the cloud app locking and unlocking, which says nothing
        // true about a license. The air-gapped refusal above stays separate
        // because an org that moved back to a cloud plan keeps its key type.
        if organization
            .base
            .plan
            .is_some_and(|plan| plan.license_plan().is_some())
        {
            return Err(crate::server::shared::types::api::ValidationError::new(
                "Pausing is not available on self-hosted plans.",
            )
            .into());
        }

        if organization.base.plan_status != Some(PlanStatus::Active) {
            return Err(anyhow!(
                "Subscription must be active to pause; current status: {}",
                organization
                    .base
                    .plan_status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "(none)".to_string())
            ));
        }

        let sub = self.find_current_subscription(&organization).await?;

        let now = Utc::now();
        let resumes_at = now + chrono::Duration::days(duration.days() as i64);

        // No yearly span-renewal guard. If the pause spans the renewal,
        // Stripe generates the next yearly draft mid-pause and finalizes
        // it when pause_collection clears at resume — partially offset by
        // the pause credit on balance, with the remainder charged to the
        // card. The UI shows an InlineInfo at pause time explaining the
        // net charge so the customer isn't surprised.

        // Dev / test-clock gotcha: `resumes_at` is wall-clock time, but Stripe
        // evaluates pause_collection against the subscription's *test clock*
        // when one is attached. If the clock has been advanced past
        // `resumes_at` (e.g., a clock advanced to 2027 while wall-clock is
        // 2026), Stripe silently accepts the request, lands the metadata, but
        // never sets pause_collection — the request behaves as if the pause
        // had already auto-resumed. Symptoms: `pause_collection_set=false`
        // on the SDK response and the follow-up webhook, the Paused arm
        // doesn't fire, `plan_status` stays `active`. To unstick, rewind /
        // reset the test clock so its current time is before `resumes_at`,
        // or shorten `duration` to land beyond the clock.

        let meta = StripeSubscriptionMetadata {
            scanopy_pause_duration_days: Some(duration.days()),
            scanopy_paused_at: Some(now.timestamp()),
            ..Default::default()
        };

        let updated = UpdateSubscription::new(&sub.id)
            .pause_collection(UpdateSubscriptionPauseCollection {
                behavior: UpdateSubscriptionPauseCollectionBehavior::KeepAsDraft,
                resumes_at: Some(resumes_at.timestamp()),
            })
            .metadata(meta.to_stripe())
            .send(&self.stripe)
            .await
            .map_err(|e| {
                tracing::error!(
                    organization_id = %organization_id,
                    subscription_id = %sub.id,
                    subscription_status = %sub.status,
                    duration_days = duration.days(),
                    error = ?e,
                    "Stripe rejected pause_collection"
                );
                anyhow!("Stripe rejected the pause request: {e}")
            })?;

        tracing::info!(
            organization_id = %organization_id,
            subscription_id = %updated.id,
            subscription_status = %updated.status,
            pause_collection_set = updated.pause_collection.is_some(),
            pause_collection_resumes_at = ?updated.pause_collection.as_ref().and_then(|p| p.resumes_at),
            "Stripe accepted pause_collection"
        );

        Ok(format!(
            "Subscription paused until {}.",
            resumes_at.format("%B %-d, %Y")
        ))
    }

    /// Resume a paused subscription by clearing `pause_collection`.
    ///
    /// Stripe's dedicated `ResumeSubscription` endpoint only handles
    /// `status='paused'` (trial without payment method). Our pauses set
    /// `pause_collection` while `status` stays `active`, so the resume
    /// path is `UpdateSubscription` with `pause_collection=` (empty form
    /// value — Stripe's documented "clear this field" convention).
    ///
    /// The SDK builder can't express that: its field is
    /// `Option<UpdateSubscriptionPauseCollection>` with
    /// `skip_serializing_if = "Option::is_none"`, so `None` is omitted
    /// rather than nulled. We send the form value directly via a custom
    /// `StripeRequest` impl, reusing the existing `stripe::Client`.
    ///
    /// Pause-credit ownership: this endpoint ONLY clears pause_collection
    /// and reports the predicted credit amount for UI display. The actual
    /// `customer_balance_transactions` POST happens in the webhook arm,
    /// triggered by Stripe's `customer.subscription.updated` for the
    /// pause_collection clear we just made. Same arm handles auto-resume.
    /// Single writer = no idempotency dance needed.
    pub async fn resume_subscription(
        &self,
        organization_id: Uuid,
        _authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;
        let sub = self.find_current_subscription(&organization).await?;

        if sub.pause_collection.is_none() {
            return Err(anyhow!("Subscription is not paused; nothing to resume."));
        }

        ClearPauseCollection::new(sub.id.clone())
            .customize()
            .send(&self.stripe)
            .await
            .map_err(|e| {
                tracing::error!(
                    organization_id = %organization_id,
                    subscription_id = %sub.id,
                    subscription_status = %sub.status,
                    error = ?e,
                    "Stripe rejected resume"
                );
                anyhow!("Stripe rejected the resume request: {e}")
            })?;

        Ok("Subscription resumed.".to_string())
    }

    /// Apply the prorated pause credit by posting a `customer_balance_transactions`
    /// with the computed amount. Called from the webhook Resumed arm — the
    /// only writer for this. Returns `Ok(Some(cents))` on apply, `Ok(None)`
    /// when there's nothing to credit (metadata missing, item missing, or
    /// computed amount is zero/negative).
    pub(crate) async fn apply_pause_credit_if_due(
        &self,
        sub: &Subscription,
        organization: &Organization,
    ) -> Result<Option<i64>, Error> {
        let Some(PauseCredit {
            credit_cents,
            actual_paused_secs,
        }) = compute_pause_credit(sub, organization)
        else {
            return Ok(None);
        };

        let Some(customer_id) = organization.base.stripe_customer_id.clone() else {
            tracing::warn!(
                subscription_id = %sub.id,
                "Organization has no stripe_customer_id; can't apply pause credit"
            );
            return Ok(None);
        };

        // Use the same clamped duration the credit math used so the
        // description and the amount reconcile (e.g. a 35-day elapsed
        // clamped to a 30-day requested duration labels the credit "30
        // days" — matching the dollars).
        let days_label = actual_paused_secs / 86_400;

        CreateCustomerCustomerBalanceTransaction::new(
            stripe_shared::CustomerId::from(customer_id),
            -credit_cents,
            stripe_types::Currency::USD,
        )
        .description(format!("Pause credit ({} days)", days_label))
        .send(&self.stripe)
        .await
        .map_err(|e| anyhow!("Stripe rejected pause-credit balance transaction: {e}"))?;

        tracing::info!(
            subscription_id = %sub.id,
            credit_cents,
            "Applied pause credit to customer balance"
        );

        Ok(Some(credit_cents))
    }

    /// Self-serve trial extend (+7 days, once per org lifetime).
    ///
    /// Pattern A: the endpoint stashes `scanopy_trial_extended_days` in
    /// metadata and calls Stripe. The webhook recognizes the key as a
    /// Scanopy-driven extend and emits `BillingOperation::TrialExtended`;
    /// the subscriber flips `trial_extended_used` to true.
    pub async fn extend_trial(
        &self,
        organization_id: Uuid,
        _authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;

        if organization.base.trial_extended_used {
            return Err(anyhow!("Trial has already been extended."));
        }
        if organization.base.plan_status != Some(PlanStatus::Trialing) {
            return Err(anyhow!("Trial extend is only available during a trial."));
        }
        let current_trial_end = organization
            .base
            .trial_end_date
            .ok_or_else(|| anyhow!("Organization has no trial end date"))?;

        let new_trial_end = current_trial_end + chrono::Duration::days(7);
        let sub = self.find_current_subscription(&organization).await?;

        let meta = StripeSubscriptionMetadata {
            scanopy_trial_extended_days: Some(7),
            ..Default::default()
        };

        UpdateSubscription::new(&sub.id)
            .trial_end(UpdateSubscriptionTrialEnd::Timestamp(
                new_trial_end.timestamp(),
            ))
            .metadata(meta.to_stripe())
            .send(&self.stripe)
            .await?;

        Ok(format!(
            "Trial extended to {}.",
            new_trial_end.format("%B %-d, %Y")
        ))
    }

    /// End the trial now and charge the card on file.
    ///
    /// Offered when the customer wants something a trial cannot give them (an
    /// air-gapped license key, which outlives any revocation and so needs a
    /// paid subscription). Stripe invoices the first cycle immediately and
    /// charges the customer's default payment method.
    ///
    /// Pattern A, like every other lifecycle action: this calls Stripe and
    /// returns. The resulting `customer.subscription.updated` (trialing →
    /// active) emits `TrialEnded { converted: true }` and `invoice.paid` emits
    /// `PaymentSucceeded`, which is what advances `license_paid_through` past
    /// the trial end. No metadata marker is needed — unlike a trial extension,
    /// the status transition is signal enough.
    pub async fn end_trial(
        &self,
        organization_id: Uuid,
        _authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;

        if organization.base.plan_status != Some(PlanStatus::Trialing) {
            return Err(anyhow!("This organization is not in a trial."));
        }
        // Read Stripe rather than the `has_payment_method` mirror: the card is
        // usually added seconds earlier, and the mirror lags by a webhook. An
        // org billed by invoice has no card and needs none.
        if !organization.base.bills_by_invoice
            && !self.customer_has_payment_method(organization_id).await?
        {
            return Err(anyhow!("Add a payment method before ending your trial."));
        }

        let sub = self.find_current_subscription(&organization).await?;

        UpdateSubscription::new(&sub.id)
            .trial_end(UpdateSubscriptionTrialEnd::Now)
            .send(&self.stripe)
            .await?;

        tracing::info!(
            organization_id = %organization_id,
            subscription_id = %sub.id,
            "Trial ended early at the customer's request"
        );

        Ok("Your trial has ended and your subscription is active.".to_string())
    }

    /// In-app subscription cancellation. Sets Stripe `cancel_at` (via the
    /// `MaxPeriodEnd` sentinel — Stripe computes the period-end timestamp),
    /// stashes the canonical Scanopy reason + save-offer context in
    /// subscription metadata, and returns the period end so the modal can
    /// render the retention disclosure inline.
    ///
    /// Pattern A: the webhook handler detects `sub.cancel_at.is_some()` and
    /// emits `BillingOperation::CancellationInitiated` with the
    /// metadata-derived payload. The subscriber then mirrors `plan_status`
    /// to `pending_cancellation`. We standardize on `cancel_at` because it's
    /// the universal scheduled-cancel signal across all Stripe paths (in-app,
    /// Customer Portal, dashboard); `cancel_at_period_end` is only set when
    /// we explicitly request it via the API, so we don't gate on it.
    pub async fn cancel_subscription(
        &self,
        organization_id: Uuid,
        request: CancelSubscriptionRequest,
        _authentication: AuthenticatedEntity,
    ) -> Result<CancelSubscriptionResponse, Error> {
        let organization = self.get_organization(organization_id).await?;
        let sub = self.find_current_subscription(&organization).await?;

        // Eligibility: only Active or Trialing subs may schedule a
        // period-end cancellation. Past-due, paused, or already-pending
        // subs reach this endpoint only by manual API call or a UI bug;
        // refuse cleanly.
        if !matches!(
            sub.status,
            SubscriptionStatus::Active | SubscriptionStatus::Trialing
        ) {
            return Err(anyhow!(
                "Subscription must be active or trialing to cancel; current status: {}",
                sub.status
            ));
        }
        if sub.cancel_at_period_end {
            return Err(anyhow!("Subscription is already pending cancellation."));
        }

        let stripe_feedback: Option<UpdateSubscriptionCancellationDetailsFeedback> =
            map_cancel_reason_to_stripe(request.reason_code);

        let mut cancellation_details = UpdateSubscriptionCancellationDetails::new();
        cancellation_details.feedback = stripe_feedback;
        cancellation_details.comment = request.comment;

        let meta = StripeSubscriptionMetadata {
            scanopy_cancel_reason: Some(request.reason_code),
            scanopy_cancel_save_offer_shown: Some(request.save_offer_shown),
            scanopy_cancel_save_offer_redeemed: request.save_offer_redeemed,
            ..Default::default()
        };

        let updated = UpdateSubscription::new(&sub.id)
            .cancel_at(UpdateSubscriptionCancelAt::MaxPeriodEnd)
            .cancellation_details(cancellation_details)
            .metadata(meta.to_stripe())
            .send(&self.stripe)
            .await
            .map_err(|e| {
                tracing::error!(
                    organization_id = %organization_id,
                    subscription_id = %sub.id,
                    subscription_status = %sub.status,
                    reason_code = ?request.reason_code,
                    error = ?e,
                    "Stripe rejected cancel_at_period_end"
                );
                anyhow!("Stripe rejected the cancel request: {e}")
            })?;

        tracing::info!(
            organization_id = %organization_id,
            subscription_id = %updated.id,
            cancel_at_period_end = updated.cancel_at_period_end,
            cancel_at = ?updated.cancel_at,
            "Stripe accepted cancel_at_period_end"
        );

        // Period end: read back the cancel_at Stripe just set. Fall back to
        // the first item's current_period_end in the unlikely event the API
        // response omits cancel_at.
        let period_end_ts = updated
            .cancel_at
            .or_else(|| updated.items.data.first().map(|i| i.current_period_end))
            .ok_or_else(|| anyhow!("Stripe did not return a period end"))?;
        let period_end = chrono::DateTime::<Utc>::from_timestamp(period_end_ts, 0)
            .ok_or_else(|| anyhow!("Invalid period_end timestamp from Stripe"))?;

        Ok(CancelSubscriptionResponse { period_end })
    }

    /// Clear a pending cancellation. Pattern A: endpoint calls Stripe to
    /// clear the scheduled cancellation (see in-line comment about the SDK
    /// constraint that forces us to use `cancel_at_period_end(false)` for
    /// the clear-only path); the webhook detects `sub.cancel_at` flipping
    /// back to `None` and emits `BillingOperation::Reactivated`. The
    /// subscriber mirrors `plan_status` back to `active`.
    pub async fn reactivate_subscription(
        &self,
        organization_id: Uuid,
        _authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;
        let sub = self.find_current_subscription(&organization).await?;

        // Eligibility: there must be a pending cancellation to clear.
        if sub.cancel_at.is_none() {
            return Err(anyhow!(
                "Subscription is not pending cancellation; nothing to reactivate."
            ));
        }

        // Clear the scheduled cancellation. We standardize on `cancel_at`
        // everywhere we SET or READ scheduled-cancellation state, but
        // async-stripe-billing's UpdateSubscription has no way to send
        // `cancel_at: null` (Option::is_none is skip-serialized), so the
        // documented canonical clear `cancel_at_period_end(false)` is the
        // only SDK-supported path. Stripe interprets it as "clear all
        // scheduled cancellation state" regardless of how cancel_at was
        // originally set.
        let updated = UpdateSubscription::new(&sub.id)
            .cancel_at_period_end(false)
            .send(&self.stripe)
            .await
            .map_err(|e| {
                tracing::error!(
                    organization_id = %organization_id,
                    subscription_id = %sub.id,
                    subscription_status = %sub.status,
                    error = ?e,
                    "Stripe rejected reactivate (cancel_at_period_end=false)"
                );
                anyhow!("Stripe rejected the reactivate request: {e}")
            })?;

        tracing::info!(
            organization_id = %organization_id,
            subscription_id = %updated.id,
            cancel_at_period_end = updated.cancel_at_period_end,
            "Stripe accepted reactivate"
        );

        Ok("Subscription reactivated.".to_string())
    }

    /// Read live coupon terms (percent_off + duration_in_months) for the
    /// configured save-offer coupon. The cancel modal calls this to render
    /// the Discount panel body dynamically from Stripe. Returns `Ok(None)`
    /// when the env var is unset — the cancel modal hides the panel in
    /// that case.
    pub async fn get_save_offer_coupon(
        &self,
        organization_id: Uuid,
    ) -> Result<Option<SaveOfferCoupon>, Error> {
        let Ok(coupon_id) = std::env::var("STRIPE_SAVE_OFFER_COUPON_ID") else {
            return Ok(None);
        };

        let organization = self.get_organization(organization_id).await?;
        let Some(plan) = organization.base.plan else {
            // No plan selected yet — nothing to discount.
            return Ok(None);
        };
        if !plan.is_stripe_managed() {
            // No Stripe sub to attach a coupon to (Free / Community / Demo /
            // CommercialSelfHosted).
            return Ok(None);
        }
        if plan.license_plan().is_some() {
            // Self-hosted licences are sold at their published annual price;
            // discounting one to retain a customer is not an offer we make.
            return Ok(None);
        }
        let billing_rate = plan.config().rate;
        let sub = self.find_current_subscription(&organization).await?;

        // Stripe moved `current_period_end` from the top-level Subscription
        // to per-item in newer API versions. Our subs are single-item
        // (one base plan), so the first item carries the canonical
        // next-invoice timestamp.
        let Some(next_renewal_ts) = sub.items.data.first().map(|i| i.current_period_end) else {
            tracing::warn!(
                organization_id = %organization_id,
                subscription_id = %sub.id,
                "Subscription has no items; cannot compute renewal date",
            );
            return Ok(None);
        };
        let Some(next_renewal_at) = DateTime::<Utc>::from_timestamp(next_renewal_ts, 0) else {
            return Ok(None);
        };

        let coupon = RetrieveCoupon::new(coupon_id.clone())
            .send(&self.stripe)
            .await
            .map_err(|e| {
                tracing::error!(
                    coupon_id = %coupon_id,
                    error = ?e,
                    "Failed to retrieve save-offer coupon"
                );
                anyhow!("Failed to read save-offer coupon: {e}")
            })?;
        let percent_off = coupon.percent_off.unwrap_or(0.0).round() as i64;
        let duration_in_months = coupon.duration_in_months.unwrap_or(12);

        // Per Stripe's repeating-coupon docs, a `duration_in_months=N` coupon
        // applies to every invoice generated in the N months after the coupon
        // is first applied. For a yearly sub the discount, when it catches an
        // invoice, applies to the full yearly amount — but if the next renewal
        // lands after the coupon's N-month window, no invoice falls inside
        // and the offer is functionally a no-op. Hide it in that case so we
        // don't promise the user something we can't deliver.
        // See: https://docs.stripe.com/billing/subscriptions/coupons
        let coupon_window_end =
            Utc::now() + chrono::Months::new(u32::try_from(duration_in_months).unwrap_or(12));
        if next_renewal_at > coupon_window_end {
            tracing::debug!(
                organization_id = %organization_id,
                next_renewal_at = %next_renewal_at,
                coupon_window_end = %coupon_window_end,
                "Save-offer coupon would not catch next renewal; hiding panel",
            );
            return Ok(None);
        }

        Ok(Some(SaveOfferCoupon {
            percent_off,
            duration_in_months,
            next_renewal_at,
            billing_rate,
        }))
    }

    /// Apply the discount save offer. Reads coupon ID from
    /// `STRIPE_SAVE_OFFER_COUPON_ID` env var. The cancel modal hides the
    /// discount panel when the env var is unset; this guard is
    /// defense-in-depth.
    pub async fn apply_discount_save_offer(
        &self,
        organization_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let coupon_id = std::env::var("STRIPE_SAVE_OFFER_COUPON_ID")
            .map_err(|_| anyhow!("Discount save offer is not configured"))?;

        let organization = self.get_organization(organization_id).await?;

        // Eligibility: once-per-org. If the org has ever redeemed the
        // save-offer discount, refuse — the cancel modal also hides the
        // panel client-side, so this is defense in depth.
        if organization.base.last_discount_at.is_some() {
            return Err(anyhow!("You've already used your one-time discount."));
        }

        // Self-hosted licences are never discounted; `get_save_offer_coupon`
        // returns nothing for them, so the panel is already hidden.
        if organization
            .base
            .plan
            .is_some_and(|plan| plan.license_plan().is_some())
        {
            return Err(crate::server::shared::types::api::ValidationError::new(
                "The retention discount is not available on self-hosted plans.",
            )
            .into());
        }

        // Eligibility gates on our typed `plan_status` (the DB source of truth,
        // updated from Stripe webhooks) rather than the live Stripe
        // `sub.status`. Active and Trialing are both allowed: a trialing user
        // can lock in the discount for their first invoice at trial-end (the
        // cancel modal suppresses pause — but not discount — while trialing).
        // Paused / past_due / pending_cancellation / cancelled are rejected.
        if !matches!(
            organization.base.plan_status,
            Some(PlanStatus::Active) | Some(PlanStatus::Trialing)
        ) {
            return Err(anyhow!(
                "Subscription must be active to apply the discount; current status: {}",
                organization
                    .base
                    .plan_status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "(none)".to_string())
            ));
        }

        let sub = self.find_current_subscription(&organization).await?;

        let updated = UpdateSubscription::new(&sub.id)
            .discounts(vec![DiscountsDataParam {
                coupon: Some(coupon_id.clone()),
                discount: None,
                promotion_code: None,
            }])
            .send(&self.stripe)
            .await
            .map_err(|e| {
                tracing::error!(
                    organization_id = %organization_id,
                    subscription_id = %sub.id,
                    subscription_status = %sub.status,
                    coupon_id = %coupon_id,
                    error = ?e,
                    "Stripe rejected discount apply"
                );
                anyhow!("Stripe rejected the discount: {e}")
            })?;

        tracing::info!(
            organization_id = %organization_id,
            subscription_id = %updated.id,
            discounts_count = updated.discounts.len(),
            first_discount_id = ?updated.discounts.first().map(|d| d.id()),
            "Stripe accepted discount apply"
        );

        // Look up the coupon to read percent_off + duration_in_months so the
        // DiscountApplied event carries the live values. This lets the
        // BillingTab chip render the actual percent (not a hard-coded one)
        // and the expires_at gate stays accurate for any future coupon.
        let coupon = RetrieveCoupon::new(coupon_id.clone())
            .send(&self.stripe)
            .await
            .map_err(|e| {
                tracing::error!(
                    organization_id = %organization_id,
                    coupon_id = %coupon_id,
                    error = ?e,
                    "Failed to retrieve coupon details after apply"
                );
                anyhow!("Failed to read coupon details: {e}")
            })?;
        let percent_off = coupon.percent_off.unwrap_or(0.0).round() as i64;
        let duration_in_months = coupon.duration_in_months.unwrap_or(12);
        let expires_at = Utc::now() + chrono::Duration::days(duration_in_months * 30);

        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::DiscountApplied {
                    percent_off,
                    expires_at,
                },
                authentication,
            ))
            .await?;

        Ok("Discount applied to your subscription.".to_string())
    }

    pub(crate) async fn handle_invoice_paid(
        &self,
        invoice: stripe_billing::Invoice,
    ) -> Result<(), Error> {
        let Some(organization) = self.get_org_from_invoice(&invoice).await? else {
            tracing::debug!("No org found for invoice.paid — ignoring");
            return Ok(());
        };

        let was_past_due = organization.base.plan_status == Some(PlanStatus::PastDue);

        if was_past_due {
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: organization.id,
                    },
                    BillingOperation::PaymentRecovered {
                        invoice_id: invoice
                            .id
                            .as_ref()
                            .map(|i| i.to_string())
                            .unwrap_or_default(),
                        amount_cents: invoice.amount_paid,
                        plan: organization.base.plan.unwrap_or_else(get_free_plan),
                        attempt_count: invoice.attempt_count as u32,
                        // The sub object isn't in scope here; Stripe fires
                        // customer.subscription.updated right after this
                        // webhook for renewals, and the handler-side emits
                        // (CheckoutCompleted / PlanChanged / Reactivated /
                        // TrialEnded) carry next_renewal_at. For a pure
                        // renewal without a status/plan change, the org
                        // value lags by ~one webhook tick; acceptable for
                        // the UI use case (BillingPlanModal is glanced
                        // at occasionally, not real-time).
                        next_renewal_at: None,
                    },
                    AuthenticatedEntity::System,
                ))
                .await?;
        }

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentSucceeded {
                    invoice: BillingInvoice::from(&invoice),
                    // Captured here, before any subscriber runs. The
                    // organization subscriber advances this column off this
                    // same event, so a subscriber that read the row could not
                    // tell the old value from the new one.
                    previous_license_paid_through: organization.base.license_paid_through,
                },
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }
}
