//! Checkout sessions, free-plan activation, trial creation, addon pricing, and customer teardown.
use super::*;

impl BillingService {
    /// Create checkout session for upgrading
    pub async fn create_checkout_session(
        &self,
        organization_id: Uuid,
        plan: BillingPlan,
        success_url: String,
        cancel_url: String,
        authentication: AuthenticatedEntity,
    ) -> Result<CheckoutSession, Error> {
        // Clone authentication for event publishing later
        let auth_for_event = authentication.clone();

        let is_returning_customer = if let Some(organization) = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
        {
            let has_non_free_plan = organization
                .base
                .plan
                .as_ref()
                .is_some_and(|p| !p.is_free());
            let has_trialed = organization.base.trial_end_date.is_some();
            has_non_free_plan || has_trialed
        } else {
            return Err(anyhow!(
                "Could not find an organization with id {}",
                organization_id
            ));
        };

        // Get or create Stripe customer
        let customer_id = self
            .get_or_create_customer(organization_id, authentication)
            .await?;

        let base_price = self
            .get_price_from_lookup_key(plan.stripe_base_price_lookup_key())
            .await?
            .ok_or_else(|| anyhow!("Could not find base price for selected plan"))?;

        // Only apply trial if plan has trial days AND customer is new (not returning)
        let trial_days = if is_returning_customer || plan.config().trial_days == 0 {
            None
        } else {
            Some(plan.config().trial_days)
        };

        // Allow trial or $0 plans without requiring credit card
        let payment_method_collection = if trial_days.is_some() || plan.config().base_cents == 0 {
            CreateCheckoutSessionPaymentMethodCollection::IfRequired
        } else {
            CreateCheckoutSessionPaymentMethodCollection::Always
        };

        let create_checkout_session = CreateCheckoutSession::new()
            .customer(customer_id)
            .success_url(success_url)
            .cancel_url(cancel_url)
            .mode(CheckoutSessionMode::Subscription)
            .payment_method_collection(payment_method_collection)
            .billing_address_collection(CheckoutSessionBillingAddressCollection::Auto)
            .customer_update(CreateCheckoutSessionCustomerUpdate {
                name: Some(CreateCheckoutSessionCustomerUpdateName::Auto),
                address: if plan.is_commercial() {
                    Some(CreateCheckoutSessionCustomerUpdateAddress::Auto)
                } else {
                    None
                },
                shipping: None,
            })
            .tax_id_collection(CreateCheckoutSessionTaxIdCollection::new(
                plan.is_commercial(),
            ))
            .line_items(vec![CreateCheckoutSessionLineItems {
                price: Some(base_price.id.to_string()),
                quantity: Some(1),
                adjustable_quantity: None,
                price_data: None,
                tax_rates: None,
                dynamic_tax_rates: None,
            }])
            .metadata([("organization_id".to_string(), organization_id.to_string())])
            .subscription_data(CreateCheckoutSessionSubscriptionData {
                trial_period_days: trial_days,
                metadata: Some(
                    [
                        ("organization_id".to_string(), organization_id.to_string()),
                        ("plan".to_string(), serde_json::to_string(&plan)?),
                    ]
                    .into(),
                ),
                ..Default::default()
            });

        let session = create_checkout_session
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        tracing::info!(
            organization_id = %organization_id,
            plan = %plan.name(),
            session_id = %session.id,
            "Checkout session created successfully"
        );

        // Publish checkout_started event for email automation
        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutStarted {
                    plan,
                    has_trial: plan.config().trial_days > 0,
                },
                auth_for_event,
            ))
            .await?;

        Ok(session)
    }

    /// Activate the Free plan directly without Stripe.
    /// Plan/status is now derived from the subscriptions ledger via the
    /// CheckoutCompleted billing event below.
    pub async fn activate_free_plan(
        &self,
        organization_id: Uuid,
        plan: BillingPlan,
        authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;

        // Publish PlanSelected onboarding event
        if organization.not_onboarded(&OnboardingOperationDiscriminants::PlanSelected) {
            self.event_bus
                .publish(Event::new(
                    OrgScope { organization_id },
                    OnboardingOperation::PlanSelected { plan },
                    authentication.clone(),
                ))
                .await?;
        }

        // Publish CheckoutCompleted billing event
        let plan_config = plan.config();
        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutCompleted {
                    plan,
                    included_networks: plan_config.included_networks,
                    included_seats: plan_config.included_seats,
                    mrr_amount_cents: 0,
                    is_trialing: false,
                    // Free direct-activation has no Stripe sub.
                    next_renewal_at: None,
                },
                authentication,
            ))
            .await?;

        tracing::info!(
            organization_id = %organization_id,
            plan = %plan.name(),
            "Free plan activated directly (no Stripe)"
        );

        Ok("Free plan activated!".to_string())
    }

    /// Create a trial subscription directly via the Stripe API, skipping Checkout
    pub async fn create_trial_subscription(
        &self,
        organization_id: Uuid,
        plan: BillingPlan,
        authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        // Guard: prevent trial reuse — org can only trial once.
        // `trial_end_date` is set when a trial starts and never cleared.
        let has_ever_trialed = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
            .and_then(|o| o.base.trial_end_date)
            .is_some();
        if has_ever_trialed {
            return Err(anyhow!(
                "Organization {} has already used a trial",
                organization_id
            ));
        }

        let auth_for_event = authentication.clone();

        let customer_id = self
            .get_or_create_customer(organization_id, authentication)
            .await?;

        let base_price = self
            .get_price_from_lookup_key(plan.stripe_base_price_lookup_key())
            .await?
            .ok_or_else(|| anyhow!("Could not find base price for selected plan"))?;

        let subscription = CreateSubscription::new(customer_id)
            .items(vec![CreateSubscriptionItems {
                price: Some(base_price.id.to_string()),
                quantity: Some(1),
                ..Default::default()
            }])
            .trial_period_days(plan.config().trial_days)
            .trial_settings(CreateSubscriptionTrialSettings::new(
                CreateSubscriptionTrialSettingsEndBehavior::new(
                    CreateSubscriptionTrialSettingsEndBehaviorMissingPaymentMethod::Cancel,
                ),
            ))
            .metadata([
                ("organization_id".to_string(), organization_id.to_string()),
                ("plan".to_string(), serde_json::to_string(&plan)?),
            ])
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        tracing::info!(
            organization_id = %organization_id,
            plan = %plan.name(),
            subscription_id = %subscription.id,
            trial_days = plan.config().trial_days,
            "Trial subscription created directly (skipped checkout)"
        );

        // Publish checkout_started event for email automation
        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutStarted {
                    plan,
                    has_trial: true,
                },
                auth_for_event,
            ))
            .await?;

        Ok(format!("Your {} trial has started!", plan.name()))
    }

    /// Subscribe to a paid plan with the card already on file, no trial and no
    /// Stripe-hosted page. This is what a returning customer gets after the
    /// in-app dialog collects their card, so buying a plan stays in the app for
    /// cloud and self-hosted alike.
    ///
    /// Stripe charges the customer's default payment method for the first
    /// invoice. A card that needs 3D Secure leaves that invoice requiring
    /// action; the existing `invoice.payment_action_required` webhook carries
    /// the hosted URL where the customer finishes authenticating.
    pub async fn create_paid_subscription(
        &self,
        organization_id: Uuid,
        plan: BillingPlan,
        authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let auth_for_event = authentication.clone();

        let customer_id = self
            .get_or_create_customer(organization_id, authentication)
            .await?;

        let base_price = self
            .get_price_from_lookup_key(plan.stripe_base_price_lookup_key())
            .await?
            .ok_or_else(|| anyhow!("Could not find base price for selected plan"))?;

        let subscription = CreateSubscription::new(customer_id)
            .items(vec![CreateSubscriptionItems {
                price: Some(base_price.id.to_string()),
                quantity: Some(1),
                ..Default::default()
            }])
            .metadata([
                ("organization_id".to_string(), organization_id.to_string()),
                ("plan".to_string(), serde_json::to_string(&plan)?),
            ])
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        tracing::info!(
            organization_id = %organization_id,
            plan = %plan.name(),
            subscription_id = %subscription.id,
            subscription_status = %subscription.status,
            "Paid subscription created in-app with the card on file"
        );

        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutStarted {
                    plan,
                    has_trial: false,
                },
                auth_for_event,
            ))
            .await?;

        Ok(format!("You're on the {} plan.", plan.name()))
    }

    pub async fn update_addon_prices(
        &self,
        organization: Organization,
        network_count: u64,
        seat_count: u64,
    ) -> Result<(), Error> {
        tracing::info!(
            organization_id = %organization.id,
            network_count = %network_count,
            seat_count = %seat_count,
            "Updating addon prices"
        );

        let plan = organization
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        if plan.is_free() {
            // Free plan has no addons to update.
            return Ok(());
        }
        let customer_id = organization
            .base
            .stripe_customer_id
            .clone()
            .ok_or_else(|| {
                anyhow!(
                    "Organization {} doesn't have a Stripe customer ID",
                    organization.base.name
                )
            })?;

        let extra_networks = if let Some(included_networks) = plan.config().included_networks {
            network_count.saturating_sub(included_networks)
        } else {
            0
        };

        let extra_seats = if let Some(included_seats) = plan.config().included_seats {
            seat_count.saturating_sub(included_seats)
        } else {
            0
        };

        // Query all subscriptions and filter for Active or Trialing status
        // This ensures trial subscriptions are included when syncing addon quantities
        let org_subscriptions = ListSubscription::new()
            .customer(customer_id)
            .send(&self.stripe)
            .await?;

        let subscription = org_subscriptions
            .data
            .iter()
            .find(|s| {
                matches!(
                    s.status,
                    SubscriptionStatus::Active | SubscriptionStatus::Trialing
                )
            })
            .ok_or_else(|| anyhow!("No active or trialing subscription found"))?;

        // Build items array - need to update quantities on existing items
        let mut items_to_update = vec![];

        // Track what we found
        let mut found_seat_item = false;
        let mut found_network_item = false;

        // Find existing subscription items by price lookup key
        for item in &subscription.items.data {
            let price_id = &item.price.id;

            // Check if this is a seat addon item
            if let Some(seat_lookup) = plan.stripe_seat_addon_price_lookup_key()
                && let Some(seat_price) = self.get_price_from_lookup_key(seat_lookup).await?
                && price_id == &seat_price.id
            {
                found_seat_item = true;
                items_to_update.push(UpdateSubscriptionItems {
                    id: Some(item.id.to_string()),
                    price: Some(price_id.to_string()),
                    quantity: Some(extra_seats),
                    deleted: if extra_seats == 0 { Some(true) } else { None },
                    ..Default::default()
                });
                continue;
            }

            // Check if this is a network addon item
            if let Some(network_lookup) = plan.stripe_network_addon_price_lookup_key()
                && let Some(network_price) = self.get_price_from_lookup_key(network_lookup).await?
                && price_id == &network_price.id
            {
                found_network_item = true;
                items_to_update.push(UpdateSubscriptionItems {
                    id: Some(item.id.to_string()),
                    price: Some(price_id.to_string()),
                    quantity: Some(extra_networks),
                    deleted: if extra_networks == 0 {
                        Some(true)
                    } else {
                        None
                    },
                    ..Default::default()
                });
                continue;
            }
        }

        // Add new seat item if needed
        if !found_seat_item
            && extra_seats > 0
            && let Some(seat_lookup) = plan.stripe_seat_addon_price_lookup_key()
            && let Some(seat_price) = self.get_price_from_lookup_key(seat_lookup).await?
        {
            items_to_update.push(UpdateSubscriptionItems {
                price: Some(seat_price.id.to_string()),
                quantity: Some(extra_seats),
                ..Default::default()
            });
        }

        // Add new network item if needed
        if !found_network_item
            && extra_networks > 0
            && let Some(network_lookup) = plan.stripe_network_addon_price_lookup_key()
            && let Some(network_price) = self.get_price_from_lookup_key(network_lookup).await?
        {
            items_to_update.push(UpdateSubscriptionItems {
                price: Some(network_price.id.to_string()),
                quantity: Some(extra_networks),
                ..Default::default()
            });
        }

        // Update the subscription if there are changes
        if !items_to_update.is_empty() {
            UpdateSubscription::new(&subscription.id)
                .items(items_to_update)
                .proration_behavior(UpdateSubscriptionProrationBehavior::CreateProrations)
                .send(&self.stripe)
                .await?;

            tracing::info!(
                organization_id = %organization.id,
                subscription_id = %subscription.id,
                extra_seats = ?extra_seats,
                extra_networks = ?extra_networks,
                "Updated subscription addon quantities"
            );
        }

        Ok(())
    }

    /// Get existing customer or create new one. On create, publishes
    /// `StripeCustomerCreated` so the org-service subscriber mirrors the
    /// customer id onto `organizations.stripe_customer_id`.
    pub(crate) async fn get_or_create_customer(
        &self,
        organization_id: Uuid,
        _authentication: AuthenticatedEntity,
    ) -> Result<CustomerId, Error> {
        // Check if org already has stripe_customer_id
        let organization = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow!("Organization {} doesn't exist.", organization_id))?;

        if let Some(customer_id) = organization.base.stripe_customer_id.clone() {
            return Ok(CustomerId::from(customer_id.to_owned()));
        }

        let organization_owners = self
            .user_service
            .get_organization_owners(&organization_id)
            .await?;

        let first_owner = organization_owners
            .first()
            .ok_or_else(|| anyhow!("Organization {} doesn't have an owner.", organization_id))?;

        // Create new customer
        let create_customer = CreateCustomer::new()
            .metadata([("organization_id".to_string(), organization_id.to_string())])
            .email(first_owner.base.email.clone());

        let customer = create_customer.send(&self.stripe).await?;

        tracing::info!(
            organization_id = %organization_id,
            customer_id = %customer.id,
            customer_email = %first_owner.base.email,
            "Created new Stripe customer"
        );

        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::StripeCustomerCreated {
                    customer_id: customer.id.to_string(),
                },
                AuthenticatedEntity::System,
            ))
            .await?;

        Ok(customer.id)
    }

    /// Permanently delete the Stripe customer. Stripe auto-cancels active
    /// subscriptions and retains invoices/charges as a deleted-customer
    /// tombstone for accounting.
    pub async fn delete_stripe_customer(&self, customer_id: &str) -> Result<(), Error> {
        DeleteCustomer::new(CustomerId::from(customer_id.to_owned()))
            .send(&self.stripe)
            .await?;
        Ok(())
    }
}
