//! Stripe webhook ingestion and subscription/payment-method event handlers.
use super::*;
use stripe_shared::SubscriptionCollectionMethod;

impl BillingService {
    /// Handle webhook events
    pub async fn handle_webhook(&self, payload: &str, signature: &str) -> Result<(), Error> {
        let event = Webhook::construct_event(payload, signature, &self.webhook_secret)?;

        tracing::debug!(
            event_type = ?event.type_,
            event_id = %event.id,
            "Received Stripe webhook"
        );

        match event.type_ {
            EventType::CustomerSubscriptionCreated | EventType::CustomerSubscriptionUpdated => {
                let sub = match event.data.object {
                    EventObject::CustomerSubscriptionCreated(sub) => Some(sub),
                    EventObject::CustomerSubscriptionUpdated(sub) => Some(sub),
                    _ => None,
                };

                if let Some(sub) = sub {
                    self.handle_subscription_update(sub).await?;
                }
            }
            EventType::CustomerSubscriptionTrialWillEnd => {
                if let EventObject::CustomerSubscriptionTrialWillEnd(sub) = event.data.object {
                    self.handle_trial_will_end(sub).await?;
                }
            }
            EventType::CustomerSubscriptionPaused | EventType::CustomerSubscriptionDeleted => {
                let sub = match event.data.object {
                    EventObject::CustomerSubscriptionDeleted(sub) => Some(sub),
                    EventObject::CustomerSubscriptionPaused(sub) => Some(sub),
                    _ => None,
                };
                if let Some(sub) = sub {
                    self.handle_subscription_deleted(sub).await?;
                }
            }
            // CheckoutSessionCompleted intentionally unhandled. Stripe fires it
            // alongside payment_method.attached for every Checkout-collected
            // card; both used to flow through here and resulted in two
            // PaymentMethodAdded events (and two "Payment method added" emails)
            // per single user action. payment_method.attached is the canonical
            // signal and is handled below.
            EventType::PaymentMethodAttached => {
                if let EventObject::PaymentMethodAttached(pm) = event.data.object
                    && let Some(customer) = pm.customer.as_ref()
                {
                    self.handle_payment_method_attached(
                        customer.id().to_string(),
                        pm.id.to_string(),
                    )
                    .await?;
                }
            }
            EventType::PaymentMethodDetached => {
                // The PaymentMethod.customer field is null after detachment —
                // extract the previous customer ID from the raw event payload.
                if let EventObject::PaymentMethodDetached(_) = event.data.object {
                    let raw: serde_json::Value = serde_json::from_str(payload)?;
                    if let Some(customer_id) = raw
                        .get("data")
                        .and_then(|d| d.get("previous_attributes"))
                        .and_then(|pa| pa.get("customer"))
                        .and_then(|c| c.as_str())
                    {
                        self.handle_payment_method_detached(customer_id.to_string())
                            .await?;
                    }
                }
            }
            EventType::InvoicePaymentFailed => {
                if let EventObject::InvoicePaymentFailed(invoice) = event.data.object {
                    self.handle_invoice_payment_failed(invoice).await?;
                }
            }
            EventType::InvoicePaymentActionRequired => {
                if let EventObject::InvoicePaymentActionRequired(invoice) = event.data.object {
                    self.handle_invoice_payment_action_required(invoice).await?;
                }
            }
            EventType::InvoicePaid => {
                if let EventObject::InvoicePaid(invoice) = event.data.object {
                    self.handle_invoice_paid(invoice).await?;
                }
            }
            EventType::InvoiceCreated => {
                if let EventObject::InvoiceCreated(invoice) = event.data.object {
                    self.handle_invoice_created(invoice).await?;
                }
            }
            EventType::InvoiceOverdue => {
                if let EventObject::InvoiceOverdue(invoice) = event.data.object {
                    self.handle_invoice_overdue(invoice).await?;
                }
            }
            EventType::InvoiceFinalized => {
                if let EventObject::InvoiceFinalized(invoice) = event.data.object {
                    self.handle_invoice_finalized(invoice).await?;
                }
            }
            EventType::InvoiceVoided | EventType::InvoiceMarkedUncollectible => {
                let invoice = match event.data.object {
                    EventObject::InvoiceVoided(invoice) => Some(invoice),
                    EventObject::InvoiceMarkedUncollectible(invoice) => Some(invoice),
                    _ => None,
                };
                if let Some(invoice) = invoice {
                    self.handle_invoice_voided(invoice).await?;
                }
            }
            _ => {
                tracing::debug!(
                    event_type = ?event.type_,
                    "Unhandled webhook event type"
                );
            }
        }

        Ok(())
    }

    async fn handle_subscription_update(&self, sub: Subscription) -> Result<(), Error> {
        tracing::debug!(
            subscription_id = %sub.id,
            subscription_status = ?sub.status,
            metadata = ?sub.metadata,
            "Processing subscription update"
        );

        let org_id = sub
            .metadata
            .get("organization_id")
            .ok_or_else(|| anyhow!("No organization_id in subscription metadata"))?;

        let plan_str = sub
            .metadata
            .get("plan")
            .ok_or_else(|| anyhow!("No plan in subscription metadata"))?;

        let plan: BillingPlan = serde_json::from_str(plan_str)?;

        tracing::info!(
            organization_id = %org_id,
            plan = %plan.name(),
            subscription_status = ?sub.status,
            subscription_id = %sub.id,
            "Subscription updated"
        );

        let org_id = Uuid::parse_str(org_id)?;

        let organization = match self.organization_service.get_by_id(&org_id).await? {
            Some(org) => org,
            None => {
                // Organization was deleted - acknowledge webhook to stop retries
                tracing::warn!(
                    stripe_customer_id = %sub.customer.id(),
                    "Received subscription update for deleted organization, ignoring"
                );
                return Ok(());
            }
        };

        let owners = self
            .user_service
            .get_organization_owners(&organization.id)
            .await?;

        // Snapshot pre-webhook state from the org row so we can detect
        // transitions (None plan, was-trialing, etc.) before applying
        // webhook updates.
        let prior_plan = organization
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        let prior_status = organization.base.plan_status;
        let prior_was_free = prior_plan.is_free();
        let prior_was_trialing = prior_status == Some(PlanStatus::Trialing);

        // Typed view of `sub.metadata`. Phase 5 Scanopy-only context
        // (cancel reason, save-offer state, etc.) rides here from the
        // endpoint and is read back in the detection arms below.
        let meta = StripeSubscriptionMetadata::from_stripe(&sub.metadata);

        // Pending cancellation — user keeps current plan until period ends.
        // `sub.cancel_at` is the universal signal for "scheduled
        // cancellation": Stripe sets it on every scheduled-cancel path
        // (Portal cancel, our cancel_subscription endpoint via
        // .cancel_at(MaxPeriodEnd), dashboard "cancel at end of period").
        // `cancel_at_period_end` is asymmetric — only set when WE pass it
        // explicitly via the API — so we don't gate on it.
        //
        // Stripe Portal-with-reason fires TWO update webhooks ~hundreds of ms
        // apart: the first carries only Stripe's internal `reason`; the second
        // carries the user-provided `feedback` + `comment`. We model these as
        // two distinct events:
        //   - `CancellationInitiated`: the cancel-scheduled signal (always
        //     fires on the false→true plan_status transition).
        //   - `CancellationFeedbackProvided`: the user-input signal (fires
        //     when cancellation_details carries feedback or comment).
        // Modeling them separately avoids the dedup-via-flag dance and matches
        // Stripe's actual two-webhook reality.
        if let Some(period_end_ts) = sub.cancel_at {
            let (stripe_feedback, comment, stripe_reason) =
                extract_cancellation_details(sub.cancellation_details.as_ref());
            let has_user_feedback = stripe_feedback.is_some() || comment.is_some();

            if prior_status != Some(PlanStatus::PendingCancellation) {
                // First cancel-scheduled webhook — emit CancellationInitiated.
                if let Some(owner) = owners.first() {
                    let authentication: AuthenticatedEntity = owner.clone().into();
                    let planned_period_end =
                        chrono::DateTime::<Utc>::from_timestamp(period_end_ts, 0)
                            .unwrap_or_else(Utc::now);
                    self.event_bus
                        .publish(Event::new(
                            OrgScope {
                                organization_id: organization.id,
                            },
                            BillingOperation::CancellationInitiated {
                                plan: organization.base.plan,
                                reason_code: meta.scanopy_cancel_reason,
                                stripe_feedback,
                                stripe_reason,
                                comment: comment.clone(),
                                save_offer_shown: meta
                                    .scanopy_cancel_save_offer_shown
                                    .clone()
                                    .unwrap_or_default(),
                                save_offer_redeemed: meta.scanopy_cancel_save_offer_redeemed,
                                planned_period_end,
                            },
                            authentication.clone(),
                        ))
                        .await?;
                    // In-app modal may set feedback in the same Stripe call,
                    // so the first webhook can already carry user input.
                    // Portal-with-reason's first webhook never does.
                    if has_user_feedback {
                        self.event_bus
                            .publish(Event::new(
                                OrgScope {
                                    organization_id: organization.id,
                                },
                                BillingOperation::CancellationFeedbackProvided {
                                    stripe_feedback,
                                    stripe_reason,
                                    comment,
                                },
                                authentication,
                            ))
                            .await?;
                    }
                }
            } else if has_user_feedback {
                // Follow-up webhook with user input — emit only the feedback
                // event. The cancellation itself was already announced.
                if let Some(owner) = owners.first() {
                    let authentication: AuthenticatedEntity = owner.clone().into();
                    self.event_bus
                        .publish(Event::new(
                            OrgScope {
                                organization_id: organization.id,
                            },
                            BillingOperation::CancellationFeedbackProvided {
                                stripe_feedback,
                                stripe_reason,
                                comment,
                            },
                            authentication,
                        ))
                        .await?;
                }
            }
            // Otherwise: already initiated, no user feedback on this webhook
            // — nothing to emit.
            return Ok(());
        }

        // First time signing up for a plan
        if let Some(owner) = owners.first()
            && (prior_plan.is_free() || prior_status.is_none())
            && organization.not_onboarded(&OnboardingOperationDiscriminants::PlanSelected)
        {
            let authentication: AuthenticatedEntity = owner.clone().into();
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: organization.id,
                    },
                    OnboardingOperation::PlanSelected { plan },
                    authentication,
                ))
                .await?;
        }

        // Publish billing lifecycle events for email automation
        if let Some(owner) = owners.first() {
            let authentication: AuthenticatedEntity = owner.clone().into();
            let is_trialing = sub.status == SubscriptionStatus::Trialing;

            // Checkout completed: first subscription creation, upgrade from
            // Free, or a lapsed org choosing a paid plan again. The last case
            // needs this arm even when the plan name is unchanged, because
            // nothing else implies Active for it.
            if prior_status.is_none()
                || prior_was_free
                || prior_status == Some(PlanStatus::Cancelled)
            {
                let plan_config = plan.config();
                self.event_bus
                    .publish(Event::new(
                        OrgScope {
                            organization_id: organization.id,
                        },
                        BillingOperation::CheckoutCompleted {
                            plan,
                            included_networks: plan_config.included_networks,
                            included_seats: plan_config.included_seats,
                            mrr_amount_cents: mrr_from_subscription(&sub),
                            is_trialing,
                            next_renewal_at: next_renewal_from_subscription(&sub),
                        },
                        authentication.clone(),
                    ))
                    .await?;

                // Trial started (if subscription is in trialing state)
                if is_trialing {
                    let trial_days = plan.config().trial_days;
                    let trial_end_dt = sub
                        .trial_end
                        .and_then(|t| chrono::DateTime::<Utc>::from_timestamp(t, 0))
                        .unwrap_or_else(Utc::now);
                    self.event_bus
                        .publish(Event::new(
                            OrgScope {
                                organization_id: organization.id,
                            },
                            BillingOperation::TrialStarted {
                                plan,
                                trial_end: trial_end_dt,
                                trial_days,
                            },
                            authentication.clone(),
                        ))
                        .await?;
                }
            }

            // Trial ended (transition from trialing to active)
            if prior_was_trialing && sub.status == SubscriptionStatus::Active {
                self.event_bus
                    .publish(Event::new(
                        OrgScope {
                            organization_id: organization.id,
                        },
                        BillingOperation::TrialEnded {
                            plan,
                            converted: true,
                            next_renewal_at: next_renewal_from_subscription(&sub),
                        },
                        authentication,
                    ))
                    .await?;
            }
        }

        // Detect plan changes — emit PlanChanged so the ledger reflects the
        // new plan as derived state. Skip if plan hasn't changed.

        // Cancel duplicate subscriptions — when Stripe Checkout creates a new subscription
        // for an existing customer, the old subscription still exists. Clean it up.
        if let Some(customer_id) = &organization.base.stripe_customer_id {
            let all_subs = ListSubscription::new()
                .customer(CustomerId::from(customer_id.clone()))
                .send(&self.stripe)
                .await?;

            let old_subs: Vec<_> = all_subs
                .data
                .iter()
                .filter(|s| {
                    s.id != sub.id
                        && matches!(
                            s.status,
                            SubscriptionStatus::Active | SubscriptionStatus::Trialing
                        )
                })
                .collect();

            for old_sub in old_subs {
                CancelSubscription::new(&old_sub.id)
                    .send(&self.stripe)
                    .await?;
                tracing::info!(
                    old_subscription = %old_sub.id,
                    new_subscription = %sub.id,
                    "Cancelled duplicate subscription during upgrade"
                );
            }
        }

        // Publish PlanChanged event if plan type actually changed (covers upgrades, downgrades, tier switches).
        // Only emit if the prior state had a real subscription history (not the
        // synthetic Free default returned when no events exist).
        if prior_status.is_some()
            && prior_plan.name() != plan.name()
            && let Some(owner) = owners.first()
        {
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::PlanChanged {
                        from: prior_plan,
                        to: plan,
                        is_downgrade: plan.is_free(),
                        next_renewal_at: next_renewal_from_subscription(&sub),
                        license_key_type: organization.base.license_key_type,
                    },
                    owner.clone().into(),
                ))
                .await?;
        }

        // Phase 5 transition arms — each one fires on the false→true edge
        // and is gated by the prior org state so a subsequent unrelated
        // `customer.subscription.updated` event doesn't re-emit. See
        // `StripeSubscriptionMetadata::from_stripe` for the shape of `meta`.

        // Paused arm — endpoint stashed `scanopy_pause_duration_days` and
        // set Stripe's `pause_collection`. Webhook reads both.
        if prior_status != Some(PlanStatus::Paused)
            && let Some(pause_collection) = sub.pause_collection.as_ref()
            && let Some(owner) = owners.first()
        {
            let resumes_at = pause_collection
                .resumes_at
                .and_then(|t| chrono::DateTime::<Utc>::from_timestamp(t, 0))
                .unwrap_or_else(Utc::now);
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::Paused {
                        plan,
                        duration_days: meta.scanopy_pause_duration_days.unwrap_or(0),
                        resumes_at,
                    },
                    owner.clone().into(),
                ))
                .await?;
        }

        // Resumed arm — pause_collection cleared. `was_early` degrades to
        // `false` from the webhook because async-stripe-webhook doesn't
        // surface Stripe's `previous_attributes` field through the current
        // SDK plumbing, so we can't distinguish a user-clicked resume from
        // a scheduled auto-resume. The signal remains useful: "this org
        // resumed."
        //
        // The Stripe sub's `current_period_end` is unchanged by the
        // pause→resume cycle (pause_collection doesn't move the cycle).
        // For auto-resume (Stripe clearing pause_collection at resumes_at
        // without the manual resume_subscription endpoint running), we
        // apply the same prorated balance credit here. The credit call
        // uses a sub-id-and-paused-at idempotency key so when the manual
        // path's API call ALSO triggers this webhook arm, we don't
        // double-credit.
        if prior_status == Some(PlanStatus::Paused)
            && sub.pause_collection.is_none()
            && let Some(owner) = owners.first()
        {
            if let Err(e) = self.apply_pause_credit_if_due(&sub, &organization).await {
                tracing::error!(
                    organization_id = %org_id,
                    subscription_id = %sub.id,
                    error = %e,
                    "Webhook resume: pause-credit apply failed",
                );
            }

            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::Resumed { was_early: false },
                    owner.clone().into(),
                ))
                .await?;
        }

        // TrialExtended arm — endpoint stashed `scanopy_trial_extended_days`
        // in metadata; subscriber flips `trial_extended_used` to true on
        // emission so subsequent webhooks skip.
        if !organization.base.trial_extended_used
            && let Some(days_added) = meta.scanopy_trial_extended_days
            && let Some(owner) = owners.first()
        {
            let new_trial_end = sub
                .trial_end
                .and_then(|t| chrono::DateTime::<Utc>::from_timestamp(t, 0))
                .unwrap_or_else(Utc::now);
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::TrialExtended {
                        days_added,
                        new_trial_end,
                    },
                    owner.clone().into(),
                ))
                .await?;
        }

        // Reactivated arm — pending cancellation cleared. Idempotency via
        // `prior_status == pending_cancellation`; the subscriber then restores
        // `plan_status` via `implied_status`. Carry the live trial state so a sub
        // reactivated mid-trial returns to `trialing`, not `active`.
        if prior_status == Some(PlanStatus::PendingCancellation)
            && sub.cancel_at.is_none()
            && let Some(owner) = owners.first()
        {
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::Reactivated {
                        trialing: sub.status == SubscriptionStatus::Trialing,
                        next_renewal_at: next_renewal_from_subscription(&sub),
                    },
                    owner.clone().into(),
                ))
                .await?;
        }

        // An unpaid sent invoice is the one way a licence buyer stops paying,
        // and Stripe reports it here: "if the subscription's collection_method
        // is set to send_invoice, it becomes past_due when its invoice remains
        // unpaid by the due date". No charge is attempted on one, so
        // `invoice.payment_failed` never fires, and `invoice.overdue` only
        // arrives when a Billing Automation is configured to send it.
        if sent_invoice_went_past_due(prior_status, sub.status, sub.collection_method)
            && let Some(invoice) = self.open_license_invoice(&organization).await?
        {
            self.report_invoice_overdue(&organization, invoice).await?;
        }

        tracing::info!(
            "Updated organization {} subscription status to {}",
            org_id,
            sub.status
        );
        Ok(())
    }

    /// Handle trial_will_end webhook (3 days before trial expiry)
    async fn handle_trial_will_end(&self, sub: Subscription) -> Result<(), Error> {
        // Skip email if subscription is already marked for cancellation (e.g., user switched to Free)
        if sub.cancel_at.is_some() {
            tracing::info!(
                "Trial ending soon but subscription is pending cancellation, skipping email"
            );
            return Ok(());
        }

        // Recover identification fields via the typed metadata view (the
        // stringly-typed `metadata.get` form is the documented anti-pattern;
        // `StripeSubscriptionMetadata` is the source of truth).
        let meta = StripeSubscriptionMetadata::from_stripe(&sub.metadata);
        let org_id = meta
            .organization_id
            .ok_or_else(|| anyhow!("No organization_id in subscription metadata"))?;
        let plan = meta
            .plan
            .ok_or_else(|| anyhow!("No plan in subscription metadata"))?;

        let Some(organization) = self.organization_service.get_by_id(&org_id).await? else {
            tracing::warn!(
                organization_id = %org_id,
                event = "trial_will_end",
                "Stripe webhook for deleted organization — skipping"
            );
            return Ok(());
        };

        tracing::info!(
            organization_id = %org_id,
            has_payment_method = organization.base.has_payment_method,
            "Trial ending soon"
        );

        // Publish TrialWillEnd event for email automation
        let owners = self
            .user_service
            .get_organization_owners(&organization.id)
            .await?;

        if let Some(owner) = owners.first() {
            self.event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::TrialWillEnd {
                        plan,
                        // Either way to pay keeps the subscription alive, so a
                        // buyer paying by invoice is not asked for a card.
                        has_payment_method: organization.can_pay(),
                    },
                    owner.clone().into(),
                ))
                .await?;
        }

        Ok(())
    }

    async fn handle_payment_method_attached(
        &self,
        customer_id: String,
        payment_method_id: String,
    ) -> Result<(), Error> {
        let filter = StorableFilter::<Organization>::new_with_stripe_customer_id(&customer_id);
        let Some(organization) = self
            .organization_service
            .get_unique(filter)
            .await?
            .at_most_one()?
        else {
            tracing::debug!(
                stripe_customer_id = %customer_id,
                "No organization found for payment_method.attached — ignoring"
            );
            return Ok(());
        };

        // Set as default payment method for future invoices so Stripe can
        // charge it when the trial ends or the next billing cycle occurs
        let mut invoice_settings = UpdateCustomerInvoiceSettings::new();
        invoice_settings.default_payment_method = Some(payment_method_id);
        UpdateCustomer::new(CustomerId::from(customer_id))
            .invoice_settings(invoice_settings)
            .send(&self.stripe)
            .await?;

        // The PaymentMethodAdded event below is logged by the logging
        // subscriber; the org subscriber flips `has_payment_method`.
        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentMethodAdded,
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    async fn handle_payment_method_detached(&self, customer_id: String) -> Result<(), Error> {
        let filter = StorableFilter::<Organization>::new_with_stripe_customer_id(&customer_id);
        let Some(organization) = self
            .organization_service
            .get_unique(filter)
            .await?
            .at_most_one()?
        else {
            tracing::debug!(
                stripe_customer_id = %customer_id,
                "No organization found for payment_method.detached — ignoring"
            );
            return Ok(());
        };

        // Check if the customer still has any payment methods remaining
        let remaining = ListPaymentMethodsCustomer::new(CustomerId::from(customer_id.clone()))
            .send(&self.stripe)
            .await?;

        if !remaining.data.is_empty() {
            tracing::info!(
                organization_id = %organization.id,
                remaining_count = remaining.data.len(),
                "Payment method detached but customer still has others — not emitting PaymentMethodRemoved"
            );
            return Ok(());
        }

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentMethodRemoved,
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    async fn handle_subscription_deleted(&self, sub: Subscription) -> Result<(), Error> {
        let org_id = sub
            .metadata
            .get("organization_id")
            .ok_or_else(|| anyhow!("No organization_id in subscription metadata"))?;
        let org_id = Uuid::parse_str(org_id)?;

        // Guard: this handler is bound to both `customer.subscription.paused`
        // and `customer.subscription.deleted`. A paused sub is not deleted —
        // our /pause endpoint already emitted `BillingOperation::Paused`,
        // so the webhook is a no-op here.
        if sub.pause_collection.is_some() {
            tracing::info!(
                organization_id = %org_id,
                subscription_id = %sub.id,
                "Subscription is paused, not deleted — not a lapse"
            );
            return Ok(());
        }

        // --- Snapshot prior subscription state, then publish the cancellation
        // event. The lapse (plan kept, plan_status = Cancelled) is owned by the
        // `SubscriptionCancelled` arm of the org billing subscriber (single
        // writer); this handler does not touch the org row. ---

        let Some(organization) = self.organization_service.get_by_id(&org_id).await? else {
            tracing::warn!(
                organization_id = %org_id,
                subscription_id = %sub.id,
                event = "subscription_deleted",
                "Stripe webhook for deleted organization — skipping"
            );
            return Ok(());
        };

        let cancelled_plan = organization
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        let was_trialing = organization.base.plan_status == Some(PlanStatus::Trialing);
        let customer_id = organization.base.stripe_customer_id.clone();
        // Whether a save-offer discount is currently applied — drives the
        // customer-discount removal below so the downgrade doesn't leave a
        // coupon that re-applies to a future subscription.
        let had_active_discount = organization.base.discount_save_offer_active_until.is_some();
        let (stripe_feedback, cancel_comment, stripe_reason) =
            extract_cancellation_details(sub.cancellation_details.as_ref());
        // `internal_reason` has no producer today (it would carry a free-form
        // reason for a system-initiated cancellation). It used to read the
        // untyped `cancel_reason` metadata key, which nothing in the codebase
        // ever writes — so the read was dead. Leave it `None` until a typed
        // producer exists.
        let internal_reason: Option<String> = None;
        let mrr_amount_cents = mrr_from_subscription(&sub);
        let tenure_days = (Utc::now() - organization.created_at).num_days().max(0) as u32;
        let license_key_type = organization.base.license_key_type;

        let free_plan = get_free_plan();

        // --- Async phase: side effects that don't need to block the webhook response ---

        let sub_id = sub.id.to_string();
        let user_service = Arc::clone(&self.user_service);
        let event_bus = Arc::clone(&self.event_bus);
        let stripe = self.stripe.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::process_subscription_deleted_side_effects(
                org_id,
                sub_id,
                customer_id,
                was_trialing,
                had_active_discount,
                free_plan,
                Some(cancelled_plan),
                stripe_feedback,
                stripe_reason,
                internal_reason,
                cancel_comment,
                sub.ended_at
                    .or(sub.canceled_at)
                    .or(sub.cancel_at)
                    .unwrap_or_else(|| Utc::now().timestamp()),
                mrr_amount_cents,
                tenure_days,
                license_key_type,
                user_service,
                event_bus,
                stripe,
            )
            .await
            {
                tracing::error!(
                    organization_id = %org_id,
                    error = %e,
                    "Failed to process subscription deletion side effects"
                );
            }
        });

        Ok(())
    }

    /// Async side effects after subscription deletion: guard 2 (revert if needed),
    /// plan restriction enforcement, event publishing, and emails. Invite
    /// revocation runs separately via `InviteService::Subscriber<BillingOperation>`
    /// triggered by the `SubscriptionCancelled` event published below.
    #[allow(clippy::too_many_arguments)]
    async fn process_subscription_deleted_side_effects(
        org_id: Uuid,
        sub_id: String,
        customer_id: Option<String>,
        was_trialing: bool,
        had_active_discount: bool,
        free_plan: BillingPlan,
        cancelled_plan: Option<BillingPlan>,
        stripe_feedback: Option<CancellationDetailsFeedback>,
        stripe_reason: Option<stripe_billing::CancellationDetailsReason>,
        internal_reason: Option<String>,
        cancel_comment: Option<String>,
        period_end_ts: i64,
        mrr_amount_cents: i64,
        tenure_days: u32,
        license_key_type: Option<LicenseKeyType>,
        user_service: Arc<UserService>,
        event_bus: Arc<EventBus>,
        stripe: stripe::Client,
    ) -> Result<(), Error> {
        // Guard 2: If org has another active subscription, revert the downgrade
        if let Some(customer_id) = &customer_id {
            let all_subs = ListSubscription::new()
                .customer(CustomerId::from(customer_id.clone()))
                .send(&stripe)
                .await?;
            if all_subs.data.iter().any(|s| {
                s.id.as_str() != sub_id
                    && matches!(
                        s.status,
                        SubscriptionStatus::Active | SubscriptionStatus::Trialing
                    )
            }) {
                // Another active subscription exists, so the cancel was an
                // upgrade-side-effect. Suppress SubscriptionCancelled here;
                // the subscriber consequently never runs for this deletion,
                // so has_payment_method stays at its pre-deletion value
                // (true, given another active sub). No defensive revert
                // needed — the subscriber is the sole writer for this field.
                tracing::info!(
                    organization_id = %org_id,
                    "Org has another active subscription — preserved previous plan derivation"
                );
                return Ok(());
            }
        }

        // The lapse is now committed (Guard 2 passed). If a
        // save-offer discount was applied, remove it from the Stripe customer
        // so it can't carry over to a future subscription. The org's discount
        // mirror fields are cleared by the `SubscriptionCancelled` subscriber
        // arm; `last_discount_at` is preserved there so the user stays
        // ineligible for a second discount. Best-effort: a missing discount is
        // not worth failing the webhook over.
        if had_active_discount
            && let Some(customer_id) = &customer_id
            && let Err(e) = DeleteDiscountCustomer::new(CustomerId::from(customer_id.clone()))
                .send(&stripe)
                .await
        {
            tracing::warn!(
                organization_id = %org_id,
                customer_id = %customer_id,
                error = ?e,
                "Failed to remove customer discount on downgrade (may already be absent)"
            );
        }

        // Publish events and send emails. Invites get revoked downstream
        // by `InviteService::Subscriber<BillingOperation>` reacting to the
        // `SubscriptionCancelled` event we publish below.
        let owners = user_service.get_organization_owners(&org_id).await?;

        if let Some(owner) = owners.first() {
            let authentication: AuthenticatedEntity = owner.clone().into();

            let period_end =
                chrono::DateTime::<Utc>::from_timestamp(period_end_ts, 0).unwrap_or_else(Utc::now);
            event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    BillingOperation::SubscriptionCancelled {
                        plan: cancelled_plan.unwrap_or(free_plan),
                        reason_code: None,
                        stripe_feedback,
                        stripe_reason,
                        internal_reason: internal_reason.clone(),
                        comment: cancel_comment.clone(),
                        period_end,
                        was_trialing,
                        mrr_amount_cents,
                        tenure_days,
                        license_key_type,
                    },
                    authentication.clone(),
                ))
                .await?;
        }

        Ok(())
    }
}

/// Whether this subscription update is the moment a sent invoice's buyer fell
/// behind: past due now, not before, on a subscription Stripe bills by
/// invoice rather than charging.
fn sent_invoice_went_past_due(
    prior_status: Option<PlanStatus>,
    status: SubscriptionStatus,
    collection_method: SubscriptionCollectionMethod,
) -> bool {
    status == SubscriptionStatus::PastDue
        && prior_status != Some(PlanStatus::PastDue)
        && collection_method == SubscriptionCollectionMethod::SendInvoice
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stripe attempts no charge on a sent invoice, so this transition is the
    /// only notice that one went unpaid. A card subscription reaches past due
    /// through the charge-attempt events instead, and must not double up.
    #[test]
    fn only_a_sent_invoice_newly_past_due_is_reported() {
        assert!(sent_invoice_went_past_due(
            Some(PlanStatus::Active),
            SubscriptionStatus::PastDue,
            SubscriptionCollectionMethod::SendInvoice
        ));
        // Already reported: the org is sitting in past due.
        assert!(!sent_invoice_went_past_due(
            Some(PlanStatus::PastDue),
            SubscriptionStatus::PastDue,
            SubscriptionCollectionMethod::SendInvoice
        ));
        // A card subscription: invoice.payment_failed owns this.
        assert!(!sent_invoice_went_past_due(
            Some(PlanStatus::Active),
            SubscriptionStatus::PastDue,
            SubscriptionCollectionMethod::ChargeAutomatically
        ));
        // Any other status change on an invoice-billed subscription.
        assert!(!sent_invoice_went_past_due(
            Some(PlanStatus::Trialing),
            SubscriptionStatus::Active,
            SubscriptionCollectionMethod::SendInvoice
        ));
    }
}
