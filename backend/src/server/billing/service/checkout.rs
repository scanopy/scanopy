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
            let has_non_free_plan = !organization.base.plan.is_free();
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

        let mut create_checkout_session = CreateCheckoutSession::new()
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
                metadata: None,
            }])
            .metadata(StripeOrgMetadata::new(organization_id).to_stripe())
            .subscription_data(CreateCheckoutSessionSubscriptionData {
                trial_period_days: trial_days,
                metadata: Some(
                    StripeSubscriptionMetadata::identity(organization_id, plan).to_stripe(),
                ),
                ..Default::default()
            });
        if let Some(id) = self.payment_method_configuration_id() {
            create_checkout_session = create_checkout_session.payment_method_configuration(id);
        }

        let session = create_checkout_session
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        // Publish checkout_started event for email automation
        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutStarted {
                    plan,
                    has_trial: plan.config().trial_days > 0,
                    auto_started: false,
                    stripe_id: session.id.to_string(),
                    subscription_status: None,
                },
                auth_for_event,
            ))
            .await;

        Ok(session)
    }

    /// Create a trial subscription directly via the Stripe API, skipping Checkout. A cloud
    /// plan's trial needs a payment method on file first, so it converts when the trial ends;
    /// the picker collects the card before asking for the trial. A license plan's trial needs
    /// none.
    pub async fn create_trial_subscription(
        &self,
        organization_id: Uuid,
        plan: BillingPlan,
        authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        // Asked of Stripe rather than the `has_payment_method` mirror, which lags a card the
        // payment dialog has just saved.
        if plan.license_plan().is_none()
            && !self.customer_has_payment_method(organization_id).await?
        {
            return Err(crate::server::shared::types::api::ValidationError::new(
                "Add a payment method to start a trial of this plan.",
            )
            .into());
        }
        self.start_trial(organization_id, plan, authentication, false)
            .await
    }

    /// Start the trial of the license plan a self-hosted license buyer signed up on, so they
    /// get their key without picking a plan. Returns `Ok(None)` when the org is not on a
    /// license plan, or its subscription has already started. A repeat before the
    /// subscription lands reaches Stripe with the same per-org idempotency key and gets the
    /// first subscription back.
    pub async fn start_default_trial(
        &self,
        organization_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<Option<String>, Error> {
        let organization = self.get_organization(organization_id).await?;
        let plan = organization.base.plan;
        if plan.license_plan().is_none() || organization.base.plan_status.is_some() {
            return Ok(None);
        }
        self.start_trial(organization_id, plan, authentication, true)
            .await
            .map(Some)
    }

    async fn start_trial(
        &self,
        organization_id: Uuid,
        plan: BillingPlan,
        authentication: AuthenticatedEntity,
        auto_started: bool,
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

        let subscription = CreateSubscription::new()
            .customer(customer_id)
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
            .metadata(StripeSubscriptionMetadata::identity(organization_id, plan).to_stripe())
            // A trial is allowed once per org, so the key is the org: a retried
            // or concurrent request returns the first subscription instead of
            // opening a second trial before `trial_end_date` lands on the org.
            .customize()
            .request_strategy(RequestStrategy::Idempotent(
                IdempotencyKey::new(format!("trial-subscription-{organization_id}"))
                    .map_err(|e| anyhow!("Invalid idempotency key: {e:?}"))?,
            ))
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        // Publish checkout_started event for email automation
        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutStarted {
                    plan,
                    has_trial: true,
                    auto_started,
                    stripe_id: subscription.id.to_string(),
                    subscription_status: Some(subscription.status.clone()),
                },
                auth_for_event,
            ))
            .await;

        // Write the trial onto the org now, through the handler the
        // subscription webhook runs, so the app unlocks when this returns
        // rather than when the webhook arrives. The webhook then finds the org
        // already trialing and announces nothing twice. If this fails, the
        // webhook still lands the plan.
        if let Err(e) = self
            .handle_subscription_update(
                subscription,
                false,
                crate::server::shared::clock::now().timestamp(),
            )
            .await
        {
            tracing::warn!(
                organization_id = %organization_id,
                error = %e,
                "Could not record the new trial ahead of its webhook"
            );
        }

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

        let subscription = CreateSubscription::new()
            .customer(customer_id)
            .items(vec![CreateSubscriptionItems {
                price: Some(base_price.id.to_string()),
                quantity: Some(1),
                ..Default::default()
            }])
            .metadata(StripeSubscriptionMetadata::identity(organization_id, plan).to_stripe())
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        self.event_bus
            .publish(Event::new(
                OrgScope { organization_id },
                BillingOperation::CheckoutStarted {
                    plan,
                    has_trial: false,
                    auto_started: false,
                    stripe_id: subscription.id.to_string(),
                    subscription_status: Some(subscription.status),
                },
                auth_for_event,
            ))
            .await;

        Ok(format!("You're on the {} plan.", plan.name()))
    }

    pub async fn update_addon_prices(
        &self,
        organization: Organization,
        site_count: u64,
        seat_count: u64,
    ) -> Result<(), Error> {
        tracing::info!(
            organization_id = %organization.id,
            site_count = %site_count,
            seat_count = %seat_count,
            "Updating addon prices"
        );

        let plan = organization.base.plan;
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

        let extra_sites = if let Some(included_sites) = plan.config().included_sites {
            site_count.saturating_sub(included_sites)
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
        let mut found_site_item = false;

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

            // Check if this is a site addon item
            if let Some(site_lookup) = plan.stripe_site_addon_price_lookup_key()
                && let Some(site_price) = self.get_price_from_lookup_key(site_lookup).await?
                && price_id == &site_price.id
            {
                found_site_item = true;
                items_to_update.push(UpdateSubscriptionItems {
                    id: Some(item.id.to_string()),
                    price: Some(price_id.to_string()),
                    quantity: Some(extra_sites),
                    deleted: if extra_sites == 0 { Some(true) } else { None },
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

        // Add new site item if needed
        if !found_site_item
            && extra_sites > 0
            && let Some(site_lookup) = plan.stripe_site_addon_price_lookup_key()
            && let Some(site_price) = self.get_price_from_lookup_key(site_lookup).await?
        {
            items_to_update.push(UpdateSubscriptionItems {
                price: Some(site_price.id.to_string()),
                quantity: Some(extra_sites),
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
                extra_sites = ?extra_sites,
                "Updated subscription addon quantities"
            );
        }

        Ok(())
    }

    /// Get existing customer or create new one. On create or adopt, writes the
    /// customer id onto `organizations.stripe_customer_id` under the org lock
    /// before returning, then publishes `StripeCustomerCreated`. Webhooks that
    /// find the org by customer id can therefore find it as soon as Stripe
    /// sends them.
    ///
    /// The write runs after the customer exists in Stripe and can fail,
    /// leaving a customer the org row doesn't name. Before creating, this
    /// searches Stripe for a customer carrying the org's id in metadata and
    /// adopts it, which heals the mirror. Stripe search lags writes by up to a
    /// minute (longer during incidents), so two requests close together can
    /// both miss; the create's idempotency key makes those return one
    /// customer. The lock is taken only for the write, not across the Stripe
    /// calls.
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

        let (customer_id, adopted) = match self.find_org_customer(organization_id).await? {
            Some(customer_id) => (customer_id, true),
            None => {
                let organization_owners = self
                    .user_service
                    .get_organization_owners(&organization_id)
                    .await?;

                let first_owner = organization_owners.first().ok_or_else(|| {
                    anyhow!("Organization {} doesn't have an owner.", organization_id)
                })?;

                let idempotency_key =
                    IdempotencyKey::new(format!("customer-{organization_id}-{}", first_owner.id))
                        .map_err(|e| anyhow!("Invalid idempotency key: {e:?}"))?;

                let customer = CreateCustomer::new()
                    .metadata(StripeOrgMetadata::new(organization_id).to_stripe())
                    .email(first_owner.base.email.clone())
                    .customize()
                    .request_strategy(RequestStrategy::Idempotent(idempotency_key))
                    .send(&self.stripe)
                    .await?;
                (customer.id, false)
            }
        };

        self.organization_service
            .apply_billing(
                organization_id,
                vec![Event::new(
                    OrgScope { organization_id },
                    BillingOperation::StripeCustomerCreated {
                        customer_id: customer_id.to_string(),
                        adopted,
                    },
                    AuthenticatedEntity::System,
                )],
            )
            .await?;

        Ok(customer_id)
    }

    /// The oldest live Stripe customer whose metadata names this org, if any.
    /// Stripe search excludes deleted customers. More than one hit means
    /// duplicates were created before this lookup existed; the oldest is the
    /// one any earlier subscription most likely hangs off.
    async fn find_org_customer(&self, organization_id: Uuid) -> Result<Option<CustomerId>, Error> {
        let hits = SearchCustomer::new(StripeOrgMetadata::search_query(organization_id))
            .send(&self.stripe)
            .await?
            .data;

        let mut matching: Vec<_> = hits
            .into_iter()
            .filter(|c| {
                c.metadata
                    .as_ref()
                    .map(StripeOrgMetadata::from_stripe)
                    .and_then(|m| m.organization_id)
                    == Some(organization_id)
            })
            .collect();

        if matching.len() > 1 {
            tracing::warn!(
                organization_id = %organization_id,
                customer_ids = ?matching.iter().map(|c| c.id.to_string()).collect::<Vec<_>>(),
                "Multiple Stripe customers carry this org's id; adopting the oldest"
            );
        }

        matching.sort_by_key(|c| c.created);
        Ok(matching.into_iter().next().map(|c| c.id))
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
