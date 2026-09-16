//! Service construction, plan/price lookups, and Stripe product initialization.
use super::*;

/// Stripe accepts at most this many `marketing_features` per product and
/// rejects the whole create call past it. The self-hosted tiers enable more
/// features than this.
const MAX_MARKETING_FEATURES: usize = 15;

impl BillingService {
    /// The product with `id`, created from `create` when Stripe does not
    /// already have it.
    ///
    /// A retrieve can fail for reasons other than the product being absent — a
    /// transient error or a rate limit reads the same to the caller — and the
    /// create that follows then fails with `resource_already_exists`. Since
    /// this runs at startup, that turned a recoverable hiccup into a server
    /// that would not boot. So a failed create is followed by one more
    /// retrieve, and only a product that can be neither read nor created is an
    /// error.
    async fn get_or_create_product(
        &self,
        id: &str,
        create: CreateProduct,
    ) -> Result<Product, Error> {
        if let Ok(product) = RetrieveProduct::new(id).send(&self.stripe).await {
            tracing::debug!("Product {} already exists", product.id);
            return Ok(product);
        }

        match create.send(&self.stripe).await {
            Ok(product) => {
                tracing::debug!("Created product: {}", product.id);
                Ok(product)
            }
            Err(create_error) => RetrieveProduct::new(id)
                .send(&self.stripe)
                .await
                .inspect(|product| {
                    tracing::debug!(
                        error = %create_error,
                        "Product {} exists after all; keeping it",
                        product.id
                    );
                })
                .map_err(|retrieve_error| {
                    anyhow!(
                        "Product {id} could not be created ({create_error}) \
                         or read back ({retrieve_error})"
                    )
                }),
        }
    }

    pub fn new(params: BillingServiceParams) -> Self {
        let BillingServiceParams {
            stripe_secret,
            webhook_secret,
            organization_service,
            user_service,
            network_service,
            host_service,
            event_bus,
        } = params;

        Self {
            stripe: Client::new(stripe_secret),
            webhook_secret,
            organization_service,
            network_service,
            host_service,
            user_service,
            plans: OnceLock::new(),
            event_bus,
        }
    }

    pub fn get_plans(&self) -> Vec<BillingPlan> {
        self.plans.get().map(|v| v.to_vec()).unwrap_or_default()
    }

    pub async fn get_organization(&self, organization_id: Uuid) -> Result<Organization, Error> {
        self.organization_service
            .get_by_id(&organization_id)
            .await?
            .ok_or_else(|| anyhow!("Organization {} not found", organization_id))
    }

    pub async fn get_price_from_lookup_key(
        &self,
        lookup_key: String,
    ) -> Result<Option<Price>, Error> {
        let price = SearchPrice::new(format!("lookup_key: \"{}\"", lookup_key))
            .limit(1)
            .send(&self.stripe)
            .await?
            .data
            .first()
            .cloned();

        Ok(price)
    }

    pub async fn initialize_products(&self, plans: Vec<BillingPlan>) -> Result<(), Error> {
        let mut created_plans = Vec::new();

        tracing::info!(
            plan_count = plans.len(),
            "Initializing Stripe products and prices"
        );

        // Create seat and network products
        let seat_product = self
            .get_or_create_product(
                SEAT_PRODUCT_ID,
                CreateProduct::new(SEAT_PRODUCT_NAME)
                    .id(SEAT_PRODUCT_ID)
                    .description("Additional seats over what's included in the base plan"),
            )
            .await?;

        let network_product = self
            .get_or_create_product(
                NETWORK_PRODUCT_ID,
                CreateProduct::new(NETWORK_PRODUCT_NAME)
                    .id(NETWORK_PRODUCT_ID)
                    .description("Additional networks over what's included in the base plan"),
            )
            .await?;

        for plan in plans {
            // Skip free and contact-only plans — they don't need Stripe products
            if matches!(
                plan,
                BillingPlan::Community(_)
                    | BillingPlan::CommercialSelfHosted(_)
                    | BillingPlan::Enterprise(_)
                    | BillingPlan::Demo(_)
            ) {
                continue;
            }

            let product_id = plan.stripe_product_id();
            let features: Vec<Feature> = plan.features().into();

            // Stripe rejects a product carrying more than
            // MAX_MARKETING_FEATURES of them, and the self-hosted tiers enable
            // more than that. These are shop-window copy, so the overflow is
            // dropped rather than failing product creation (and with it server
            // startup).
            let features: Vec<Features> = features
                .iter()
                .take(MAX_MARKETING_FEATURES)
                .map(|f| Features::new(f.name()))
                .collect();

            let product = self
                .get_or_create_product(
                    &product_id,
                    CreateProduct::new(plan.name())
                        .id(product_id.clone())
                        .marketing_features(features)
                        .description(plan.description()),
                )
                .await?;

            // Create base price
            match self
                .get_price_from_lookup_key(plan.stripe_base_price_lookup_key())
                .await?
            {
                Some(p) => {
                    tracing::debug!("Price {} already exists", p.id);
                }
                None => {
                    // Create price
                    let create_base_price = CreatePrice::new(stripe_types::Currency::USD)
                        .lookup_key(plan.stripe_base_price_lookup_key())
                        .product(product.id.clone())
                        .unit_amount(plan.config().base_cents)
                        .recurring(CreatePriceRecurring {
                            interval: plan.config().rate.stripe_recurring_interval(),
                            interval_count: Some(1),
                            trial_period_days: None,
                            meter: None,
                            usage_type: Some(CreatePriceRecurringUsageType::Licensed),
                        });

                    let price = create_base_price.send(&self.stripe).await?;

                    tracing::debug!("Created price: {}", price.id);
                }
            };

            // Create seat prices
            if let (Some(seat_lookup_key), Some(seat_cents)) = (
                plan.stripe_seat_addon_price_lookup_key(),
                plan.config().seat_cents,
            ) {
                // Create seat addon price
                match self
                    .get_price_from_lookup_key(seat_lookup_key.clone())
                    .await?
                {
                    Some(p) => {
                        tracing::debug!("Price {} already exists", p.id);
                    }
                    None => {
                        // Create price
                        let create_seat_price = CreatePrice::new(stripe_types::Currency::USD)
                            .lookup_key(seat_lookup_key)
                            .product(seat_product.id.clone())
                            .unit_amount(seat_cents)
                            .recurring(CreatePriceRecurring {
                                interval: plan.config().rate.stripe_recurring_interval(),
                                interval_count: Some(1),
                                trial_period_days: None,
                                meter: None,
                                usage_type: Some(CreatePriceRecurringUsageType::Licensed),
                            });

                        let price = create_seat_price.send(&self.stripe).await?;

                        tracing::debug!("Created price: {}", price.id);
                    }
                };
            }

            // Create network prices
            if let (Some(network_lookup_key), Some(network_cents)) = (
                plan.stripe_network_addon_price_lookup_key(),
                plan.config().network_cents,
            ) {
                // Create network addon price
                match self
                    .get_price_from_lookup_key(network_lookup_key.clone())
                    .await?
                {
                    Some(p) => {
                        tracing::debug!("Price {} already exists", p.id);
                    }
                    None => {
                        // Create price
                        let create_network_price = CreatePrice::new(stripe_types::Currency::USD)
                            .lookup_key(network_lookup_key)
                            .product(network_product.id.clone())
                            .unit_amount(network_cents)
                            .recurring(CreatePriceRecurring {
                                interval: plan.config().rate.stripe_recurring_interval(),
                                interval_count: Some(1),
                                trial_period_days: None,
                                meter: None,
                                usage_type: Some(CreatePriceRecurringUsageType::Licensed),
                            });

                        let price = create_network_price.send(&self.stripe).await?;

                        tracing::debug!("Created price: {}", price.id);
                    }
                };
            }

            created_plans.push(plan)
        }

        created_plans.push(get_enterprise_plan());
        created_plans.push(get_enterprise_plan().to_yearly(YEARLY_DISCOUNT));

        let _ = self.plans.set(created_plans.clone());

        tracing::info!(
            initialized_plans = created_plans.len(),
            "Successfully initialized all Stripe products"
        );

        Ok(())
    }
}
