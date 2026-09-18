//! Payment-method & portal sessions, subscription status, plan changes, and invoice-payment failure handling.
use super::*;

impl BillingService {
    /// Create a SetupIntent for the org's Stripe customer, returning its
    /// `client_secret` for the frontend Payment Element. Ensures the customer
    /// exists first (created at card-collection time, per the signup flow).
    /// `usage = off_session` so the saved card can later be charged when a
    /// trial converts or a paid subscription renews. Redirect-based payment
    /// methods are disabled so `confirmSetup({ redirect: 'if_required' })`
    /// never needs a return URL.
    pub async fn create_setup_intent(
        &self,
        organization_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let customer_id = self
            .get_or_create_customer(organization_id, authentication)
            .await?;

        let mut automatic_payment_methods = CreateSetupIntentAutomaticPaymentMethods::new(true);
        automatic_payment_methods.allow_redirects =
            Some(CreateSetupIntentAutomaticPaymentMethodsAllowRedirects::Never);

        let setup_intent = CreateSetupIntent::new()
            .customer(customer_id.to_string())
            .automatic_payment_methods(automatic_payment_methods)
            .usage(CreateSetupIntentUsage::OffSession)
            .metadata([("organization_id".to_string(), organization_id.to_string())])
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        tracing::info!(
            organization_id = %organization_id,
            setup_intent_id = %setup_intent.id,
            "SetupIntent created for in-app card collection"
        );

        setup_intent
            .client_secret
            .ok_or_else(|| anyhow!("SetupIntent did not return a client_secret"))
    }

    /// Finalize a client-confirmed SetupIntent: verify it succeeded for this
    /// org's customer and set the collected card as the customer's default
    /// invoice payment method. Does NOT emit `PaymentMethodAdded` — that event
    /// (which drives the `has_payment_method` mirror, the email, and analytics)
    /// is owned solely by the `payment_method.attached` webhook, so it fires
    /// exactly once. Callers that need an immediate, race-free "card on file?"
    /// answer (the charge-vs-Checkout branch) read Stripe via
    /// `customer_has_payment_method` rather than the event-sourced mirror.
    pub async fn finalize_payment_method(
        &self,
        organization_id: Uuid,
        setup_intent_id: String,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let setup_intent = RetrieveSetupIntent::new(setup_intent_id.clone())
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        if setup_intent.status != SetupIntentStatus::Succeeded {
            return Err(anyhow!(
                "SetupIntent {} is not in a succeeded state (status: {:?})",
                setup_intent_id,
                setup_intent.status
            ));
        }

        let payment_method_id = setup_intent
            .payment_method
            .as_ref()
            .map(|pm| pm.id().to_string())
            .ok_or_else(|| anyhow!("SetupIntent {} has no payment method", setup_intent_id))?;

        // Tenant isolation: the SetupIntent must belong to this org's customer.
        let customer_id = self
            .get_or_create_customer(organization_id, authentication)
            .await?;
        let setup_intent_customer = setup_intent.customer.as_ref().map(|c| c.id().to_string());
        if setup_intent_customer.as_deref() != Some(customer_id.as_str()) {
            return Err(anyhow!(
                "SetupIntent {} does not belong to organization {}",
                setup_intent_id,
                organization_id
            ));
        }

        // Set as default payment method for future invoices (matches the
        // webhook path in webhooks.rs::handle_payment_method_attached).
        let mut invoice_settings = UpdateCustomerInvoiceSettings::new();
        invoice_settings.default_payment_method = Some(payment_method_id);
        UpdateCustomer::new(customer_id)
            .invoice_settings(invoice_settings)
            .send(&self.stripe)
            .await?;

        // Saving a card is how an org billed by invoice switches back to card
        // payments: the next invoice charges it.
        let organization = self.get_organization(organization_id).await?;
        if let Ok(sub) = self.find_current_subscription(&organization).await
            && sub.collection_method == stripe_shared::SubscriptionCollectionMethod::SendInvoice
        {
            UpdateSubscription::new(&sub.id)
                .collection_method(stripe_shared::SubscriptionCollectionMethod::ChargeAutomatically)
                .send(&self.stripe)
                .await?;
            tracing::info!(
                organization_id = %organization_id,
                subscription_id = %sub.id,
                "Subscription switched from invoice billing to card"
            );
        }

        // No PaymentMethodAdded emission here — the `payment_method.attached`
        // webhook is the sole emitter (one event → one mirror flip, one email,
        // one analytics capture). Synchronous "card on file?" callers read
        // Stripe via `customer_has_payment_method`.
        Ok(())
    }

    pub async fn create_portal_session(
        &self,
        organization_id: Uuid,
        return_url: String,
    ) -> Result<String, Error> {
        // Get customer ID
        let organization = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow!("Organization not found"))?;

        let customer_id = organization
            .base
            .stripe_customer_id
            .ok_or_else(|| anyhow!("No Stripe customer ID"))?;

        let session = CreateBillingPortalSession::new(CustomerId::from(customer_id.clone()))
            .return_url(return_url)
            .send(&self.stripe)
            .await?;

        tracing::info!(
            organization_id = %organization_id,
            customer_id = %customer_id,
            "Created billing portal session"
        );

        Ok(session.url)
    }

    /// Whether an org still has an active paid Stripe subscription that will
    /// keep billing them. Used to block destructive actions (e.g. org delete)
    /// that would leave Stripe charging an unreachable customer.
    ///
    /// Returns `false` for:
    /// - Free and other non-Stripe plans (no Stripe subscription)
    /// - Pending-cancellation, paused, or cancelled status
    /// - Orgs with no subscription history at all
    pub async fn has_active_paid_subscription(&self, organization_id: Uuid) -> Result<bool, Error> {
        let Some(org) = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
        else {
            return Ok(false);
        };
        let plan = org
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        if plan.is_free() || !plan.is_stripe_managed() {
            return Ok(false);
        }
        Ok(matches!(
            org.base.plan_status,
            Some(PlanStatus::Active) | Some(PlanStatus::Trialing) | Some(PlanStatus::PastDue)
        ))
    }

    /// Authoritative check (via Stripe, not the `has_payment_method` mirror) of
    /// whether the org's Stripe customer has at least one payment method on
    /// file. The mirror is event-sourced and lags the in-app SetupIntent flow
    /// by an event-bus tick, so the synchronous charge-vs-Checkout branch in
    /// `create_checkout_session` reads Stripe directly here — otherwise a user
    /// who just added a card could be bounced to hosted Checkout to re-enter it.
    /// Returns `false` when the org has no Stripe customer yet. Tenant-isolated:
    /// the customer id comes from the org row, never request input.
    pub async fn customer_has_payment_method(&self, organization_id: Uuid) -> Result<bool, Error> {
        let Some(org) = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
        else {
            return Ok(false);
        };
        let Some(customer_id) = org.base.stripe_customer_id.clone() else {
            return Ok(false);
        };
        let payment_methods = ListPaymentMethodsCustomer::new(CustomerId::from(customer_id))
            .send(&self.stripe)
            .await?;
        Ok(!payment_methods.data.is_empty())
    }

    /// Schedule a downgrade to Free at the end of the billing cycle.
    ///
    /// Sets `cancel_at = MaxPeriodEnd` on the active subscription (Stripe
    /// computes the period-end and writes it to `cancel_at`). Stripe keeps the
    /// subscription active until the period ends, then fires `customer.subscription.deleted`
    /// which triggers auto-Free creation via `handle_subscription_deleted`.
    pub async fn schedule_downgrade(
        &self,
        organization_id: Uuid,
        _authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;
        let customer_id = organization
            .base
            .stripe_customer_id
            .ok_or_else(|| anyhow!("No Stripe customer ID"))?;

        let subs = ListSubscription::new()
            .customer(CustomerId::from(customer_id))
            .send(&self.stripe)
            .await?;

        if let Some(sub) = subs.data.iter().find(|s| {
            matches!(
                s.status,
                SubscriptionStatus::Active | SubscriptionStatus::Trialing
            )
        }) {
            let is_trialing = sub.status == SubscriptionStatus::Trialing;

            UpdateSubscription::new(&sub.id)
                .cancel_at(UpdateSubscriptionCancelAt::MaxPeriodEnd)
                .send(&self.stripe)
                .await?;

            tracing::info!(
                organization_id = %organization_id,
                subscription_id = %sub.id,
                is_trialing,
                "Scheduled downgrade to Free at period end"
            );

            if is_trialing {
                Ok("Your plan will change to Free when your trial ends.".to_string())
            } else {
                Ok("Your plan will change to Free at the end of your billing cycle.".to_string())
            }
        } else {
            Err(anyhow!("No active subscription found"))
        }
    }

    /// Preview what would change when switching to a different plan
    pub async fn preview_plan_change(
        &self,
        organization_id: Uuid,
        target_plan: BillingPlan,
    ) -> Result<ChangePlanPreview, Error> {
        let org_filter = StorableFilter::<Network>::new_from_org_id(&organization_id);
        let networks = self.network_service.get_all(org_filter.clone()).await?;
        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();

        // count_for_networks/count_for_org narrow to live rows, so snapshot
        // closed-copies don't inflate the billable host/seat counts against
        // plan limits.
        let host_count = self.host_service.count_for_networks(&network_ids).await?;
        let seat_count = self.user_service.count_for_org(&organization_id).await?;

        let target_config = target_plan.config();

        let excess_hosts = target_config
            .included_hosts
            .map(|limit| host_count.saturating_sub(limit))
            .unwrap_or(0);

        let excess_networks = target_config
            .included_networks
            .map(|limit| (networks.len() as u64).saturating_sub(limit))
            .unwrap_or(0);

        let excess_seats = target_config
            .included_seats
            .map(|limit| seat_count.saturating_sub(limit))
            .unwrap_or(0);

        Ok(ChangePlanPreview {
            excess_hosts,
            excess_networks,
            excess_seats,
        })
    }

    /// Change the organization's billing plan
    ///
    /// Updates the Stripe subscription to the target plan's price.
    /// The webhook handles setting the plan in our database.
    pub async fn change_plan(
        &self,
        organization_id: Uuid,
        target_plan: BillingPlan,
        _authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow!("Organization not found"))?;

        let customer_id = organization
            .base
            .stripe_customer_id
            .clone()
            .ok_or_else(|| anyhow!("No Stripe customer ID"))?;

        let base_price = self
            .get_price_from_lookup_key(target_plan.stripe_base_price_lookup_key())
            .await?
            .ok_or_else(|| anyhow!("Could not find price for target plan"))?;

        let org_subscriptions = ListSubscription::new()
            .customer(CustomerId::from(customer_id))
            .send(&self.stripe)
            .await?;

        if let Some(sub) = org_subscriptions.data.iter().find(|s| {
            matches!(
                s.status,
                SubscriptionStatus::Active | SubscriptionStatus::Trialing
            )
        }) {
            // Find the base price item to replace
            let base_item = sub
                .items
                .data
                .first()
                .ok_or_else(|| anyhow!("No subscription items found"))?;

            let proration = if sub.status == SubscriptionStatus::Trialing {
                UpdateSubscriptionProrationBehavior::None
            } else {
                UpdateSubscriptionProrationBehavior::AlwaysInvoice
            };

            let mut items = vec![UpdateSubscriptionItems {
                id: Some(base_item.id.to_string()),
                price: Some(base_price.id.to_string()),
                quantity: Some(1),
                ..Default::default()
            }];
            // A plan with no add-on prices (the self-hosted tiers, Starter)
            // also drops any seat/network add-on items. They are priced for
            // the old plan, and Stripe rejects a subscription whose items
            // bill on different intervals, such as a monthly add-on beside a
            // yearly self-hosted base.
            let target_config = target_plan.config();
            if target_config.seat_cents.is_none() && target_config.network_cents.is_none() {
                items.extend(
                    sub.items
                        .data
                        .iter()
                        .skip(1)
                        .map(|item| UpdateSubscriptionItems {
                            id: Some(item.id.to_string()),
                            deleted: Some(true),
                            ..Default::default()
                        }),
                );
            }

            // TEMP diag: proves Stripe was asked to change, and names the price
            // and the subscription whose metadata the webhook reads back.
            tracing::info!(
                subscription_id = %sub.id,
                subscription_status = ?sub.status,
                lookup_key = %target_plan.stripe_base_price_lookup_key(),
                price_id = %base_price.id,
                target_plan = %target_plan.name(),
                "TEMP diag: updating Stripe subscription for plan change"
            );

            UpdateSubscription::new(&sub.id)
                .items(items)
                .metadata([
                    ("plan".to_string(), serde_json::to_string(&target_plan)?),
                    ("organization_id".to_string(), organization_id.to_string()),
                ])
                .proration_behavior(proration)
                // Clear any pending cancellation. We standardize on `cancel_at`
                // everywhere we SET or READ scheduled-cancellation state, but
                // async-stripe-billing's UpdateSubscription has no way to send
                // `cancel_at: null` (Option::is_none is skip-serialized), so the
                // documented canonical clear `cancel_at_period_end(false)` is
                // the only SDK-supported path. Stripe interprets it as "clear
                // all scheduled cancellation state" regardless of how cancel_at
                // was originally set.
                .cancel_at_period_end(false)
                .send(&self.stripe)
                .await?;

            let is_trialing = sub.status == SubscriptionStatus::Trialing;

            tracing::info!(
                organization_id = %organization_id,
                target_plan = %target_plan.name(),
                is_trialing,
                "Plan changed via subscription update"
            );

            if is_trialing {
                Ok(format!(
                    "Plan changed to {}. Your trial continues.",
                    target_plan.name()
                ))
            } else {
                Ok(format!("Plan changed to {}", target_plan.name()))
            }
        } else {
            Err(anyhow!("No active subscription found to modify"))
        }
    }

    pub(crate) async fn get_org_from_invoice(
        &self,
        invoice: &stripe_billing::Invoice,
    ) -> Result<Option<Organization>, Error> {
        let Some(customer) = invoice.customer.as_ref() else {
            return Ok(None);
        };
        let customer_id = customer.id().to_string();
        let filter = StorableFilter::<Organization>::new_with_stripe_customer_id(&customer_id);
        self.organization_service
            .get_unique(filter)
            .await?
            .at_most_one()
    }

    pub(crate) async fn handle_invoice_payment_failed(
        &self,
        invoice: stripe_billing::Invoice,
    ) -> Result<(), Error> {
        let Some(organization) = self.get_org_from_invoice(&invoice).await? else {
            tracing::debug!("No org found for invoice.payment_failed — ignoring");
            return Ok(());
        };

        // Skip for Free plan orgs — legacy $0 subscriptions may still generate invoices
        if organization.base.plan.as_ref().is_none_or(|p| p.is_free()) {
            tracing::info!(organization_id = %organization.id, "Skipping payment_failed — Free plan (legacy subscription)");
            return Ok(());
        }

        // Skip only for an org with no way to pay at all, which is the trial
        // auto-cancel flow this guard exists for. `has_payment_method` alone
        // means "a card is attached in Stripe", so it is permanently false for
        // an invoice-billed org and would drop every genuine failure for a
        // customer paying against a purchase order.
        if !organization.can_pay() {
            tracing::info!(organization_id = %organization.id, "Skipping payment_failed — no way to pay (trial auto-cancel)");
            return Ok(());
        }

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentFailed {
                    invoice_id: invoice
                        .id
                        .as_ref()
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                    amount_cents: invoice.amount_due,
                    plan: organization.base.plan.unwrap_or_else(get_free_plan),
                    attempt_count: invoice.attempt_count as u32,
                },
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    pub(crate) async fn handle_invoice_payment_action_required(
        &self,
        invoice: stripe_billing::Invoice,
    ) -> Result<(), Error> {
        let Some(organization) = self.get_org_from_invoice(&invoice).await? else {
            tracing::debug!("No org found for invoice.payment_action_required — ignoring");
            return Ok(());
        };

        // Skip for Free plan orgs — legacy $0 subscriptions may still generate invoices
        if organization.base.plan.as_ref().is_none_or(|p| p.is_free()) {
            tracing::info!(organization_id = %organization.id, "Skipping payment_action_required — Free plan (legacy subscription)");
            return Ok(());
        }

        // As in `handle_invoice_payment_failed`: an invoice-billed org has no
        // card by design, so the bare mirror would silence it here too.
        if !organization.can_pay() {
            tracing::info!(organization_id = %organization.id, "Skipping payment_action_required — no way to pay (trial auto-cancel)");
            return Ok(());
        }

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::PaymentActionRequired {
                    invoice_id: invoice
                        .id
                        .as_ref()
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                    hosted_invoice_url: invoice.hosted_invoice_url.clone(),
                },
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(())
    }

    /// Find the active / trialing / paused subscription for an org. Used by
    /// pause/resume/extend-trial/cancel — all operate on the org's current
    /// Stripe subscription regardless of state.
    pub(crate) async fn find_current_subscription(
        &self,
        organization: &Organization,
    ) -> Result<Subscription, Error> {
        let customer_id = organization
            .base
            .stripe_customer_id
            .clone()
            .ok_or_else(|| anyhow!("No Stripe customer ID"))?;

        let subs = ListSubscription::new()
            .customer(CustomerId::from(customer_id))
            .send(&self.stripe)
            .await?;

        subs.data
            .into_iter()
            .find(|s| {
                matches!(
                    s.status,
                    SubscriptionStatus::Active
                        | SubscriptionStatus::Trialing
                        | SubscriptionStatus::Paused
                        | SubscriptionStatus::PastDue
                )
            })
            .ok_or_else(|| anyhow!("No active subscription found"))
    }
}
