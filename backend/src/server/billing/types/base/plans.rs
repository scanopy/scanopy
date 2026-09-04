//! Billing plans: tiers, config, rate, the feature matrix, and plan metadata.
use super::*;

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Display,
    IntoStaticStr,
    EnumIter,
    EnumDiscriminants,
    VariantNames,
    Eq,
    ToSchema,
)]
#[strum_discriminants(derive(IntoStaticStr, Serialize))]
#[serde(tag = "type")]
pub enum BillingPlan {
    #[schema(title = "Community")]
    Community(PlanConfig),
    #[schema(title = "Free")]
    Free(PlanConfig),
    #[schema(title = "Starter")]
    Starter(PlanConfig),
    #[schema(title = "Pro")]
    Pro(PlanConfig),
    #[schema(title = "Team")]
    Team(PlanConfig),
    #[schema(title = "Business")]
    Business(PlanConfig),
    #[schema(title = "Enterprise")]
    Enterprise(PlanConfig),
    #[schema(title = "Demo")]
    Demo(PlanConfig),
    #[schema(title = "CommercialSelfHosted")]
    CommercialSelfHosted(PlanConfig),
    #[schema(title = "SelfHostedStandard")]
    SelfHostedStandard(PlanConfig),
    #[schema(title = "SelfHostedPlus")]
    SelfHostedPlus(PlanConfig),
}

impl PartialOrd for BillingPlanDiscriminants {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        fn cloud_tier(d: &BillingPlanDiscriminants) -> Option<u8> {
            match d {
                BillingPlanDiscriminants::Free => Some(0),
                BillingPlanDiscriminants::Starter => Some(1),
                BillingPlanDiscriminants::Pro => Some(2),
                BillingPlanDiscriminants::Team => Some(3),
                BillingPlanDiscriminants::Business => Some(4),
                BillingPlanDiscriminants::Enterprise => Some(5),
                _ => None,
            }
        }
        // Self-hosted commercial tiers form a second ordered ladder, separate
        // from and incomparable with the cloud ladder.
        fn self_hosted_tier(d: &BillingPlanDiscriminants) -> Option<u8> {
            match d {
                BillingPlanDiscriminants::Community => Some(0),
                BillingPlanDiscriminants::SelfHostedStandard => Some(1),
                BillingPlanDiscriminants::SelfHostedPlus => Some(2),
                _ => None,
            }
        }
        if let (Some(a), Some(b)) = (cloud_tier(self), cloud_tier(other)) {
            return Some(a.cmp(&b));
        }
        if let (Some(a), Some(b)) = (self_hosted_tier(self), self_hosted_tier(other)) {
            return Some(a.cmp(&b));
        }
        if self == other {
            return Some(std::cmp::Ordering::Equal);
        }
        None
    }
}

impl PartialEq for BillingPlan {
    fn eq(&self, other: &Self) -> bool {
        self.config() == other.config()
    }
}

impl Hash for BillingPlan {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.config().hash(state);
    }
}

impl Default for BillingPlan {
    /// The self-hosted plan. It backstops `Option<BillingPlan>::unwrap_or_default()`
    /// for rows with no plan set, which on a self-hosted deployment is what every
    /// organization runs on anyway (see `plans::self_hosted_plan`).
    fn default() -> Self {
        use crate::server::billing::plans::self_hosted_plan;

        self_hosted_plan()
    }
}

impl BillingPlan {
    pub fn to_yearly(&self, discount: f32) -> Self {
        let mut yearly_config = self.config();
        yearly_config.rate = BillingRate::Year;

        // Round discounted monthly base to nearest dollar then subtract 1 cent
        // so yearly prices end in .99 (e.g. $14.99/mo → $11.99/mo billed yearly).
        let monthly_base = Self::round_to_99(yearly_config.base_cents as f32 * (1.0 - discount));
        yearly_config.base_cents = monthly_base * 12;
        yearly_config.seat_cents = yearly_config.seat_cents.map(|c| {
            let monthly = Self::round_to_dollar(c as f32 * (1.0 - discount));
            monthly * 12
        });
        yearly_config.network_cents = yearly_config.network_cents.map(|c| {
            let monthly = Self::round_to_dollar(c as f32 * (1.0 - discount));
            monthly * 12
        });
        yearly_config.host_cents = yearly_config.host_cents.map(|c| {
            let monthly = Self::round_to_dollar(c as f32 * (1.0 - discount));
            monthly * 12
        });

        let mut yearly_plan = *self;
        yearly_plan.set_config(yearly_config);
        yearly_plan
    }
    fn round_to_dollar(cents: f32) -> i64 {
        ((cents / 100.0).round() * 100.0) as i64
    }

    /// Round to nearest dollar, then subtract 1 cent so the price ends in .99.
    fn round_to_99(cents: f32) -> i64 {
        Self::round_to_dollar(cents) - 1
    }

    pub fn billing_period(&self) -> &str {
        self.config().rate.billing_period()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Default, Hash, ToSchema)]
pub struct PlanConfig {
    /// Fixed charge per billing period, in cents.
    pub base_cents: i64,
    /// Billing interval this configuration is priced for.
    pub rate: BillingRate,
    /// Length of the free trial, in days. Zero when the plan has no trial.
    pub trial_days: u32,

    // None = can't pay for more
    /// Charge per seat beyond `included_seats`, in cents.
    pub seat_cents: Option<i64>,
    /// Charge per network beyond `included_networks`, in cents.
    pub network_cents: Option<i64>,
    /// Charge per host beyond `included_hosts`, in cents.
    pub host_cents: Option<i64>,

    // None = unlimited
    /// Seats included before per-seat charges apply.
    pub included_seats: Option<u64>,
    /// Networks included before per-network charges apply.
    pub included_networks: Option<u64>,
    /// Hosts included before per-host charges apply.
    pub included_hosts: Option<u64>,
    /// Organizations allowed on one self-hosted server instance. `None` =
    /// unlimited. Only enforced for self-hosted deployments (see
    /// `provision_user`); cloud stays multi-tenant regardless. Defaulted so
    /// existing stored plan JSON deserializes unchanged.
    #[serde(default)]
    pub included_orgs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Display, Copy, PartialEq, Eq, Default, Hash)]
pub enum Hosting {
    SelfHosted,
    /// Hosting-agnostic: available managed or self-hosted (Enterprise).
    Any,
    #[default]
    Cloud,
}

/// How a plan is acquired. Surfaced in plan metadata so the marketing site and
/// app choose the right call to action. `Contact` flips to `Stripe` for the
/// self-hosted paid tiers once the license-automation flow ships.
#[derive(Debug, Clone, Serialize, Deserialize, Display, Copy, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PurchaseFlow {
    /// Self-serve Stripe checkout.
    Stripe,
    /// Contact sales / founder-issued license.
    Contact,
    /// No purchase (free / open source).
    #[default]
    None,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Display, Default, Copy, PartialEq, Eq, Hash, ToSchema,
)]
pub enum BillingRate {
    #[default]
    Month,
    Year,
}

impl BillingRate {
    pub fn stripe_recurring_interval(&self) -> CreatePriceRecurringInterval {
        match self {
            BillingRate::Month => CreatePriceRecurringInterval::Month,
            BillingRate::Year => CreatePriceRecurringInterval::Year,
        }
    }

    pub fn billing_period(&self) -> &'static str {
        match self {
            BillingRate::Month => "Monthly",
            BillingRate::Year => "Yearly",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingPlanFeatures {
    pub share_views: bool,
    pub remove_created_with: bool,
    pub audit_logs: bool,
    pub webhooks: bool,
    pub api_access: bool,
    pub onboarding_call: bool,
    pub custom_sso: bool,
    pub saml: bool,
    /// Runs fully offline (no phone-home). Deployable air-gapped.
    pub air_gapped_deployment: bool,
    pub managed_deployment: bool,
    pub whitelabeling: bool,
    pub live_chat_support: bool,
    pub embeds: bool,
    pub email_support: bool,
    pub priority_support: bool,
    // Core features
    pub network_mapping: bool,
    pub png_export: bool,
    pub svg_export: bool,
    pub mermaid_export: bool,
    pub confluence_export: bool,
    pub pdf_export: bool,
    pub html_export: bool,
    pub scheduled_discovery: bool,
    pub discovery_integrations: bool,
    pub csv_export: bool,
    /// How many days of snapshots the plan retains before the daily sweep
    /// deletes them. `0` means snapshots are unavailable on this plan. The
    /// env-var override (`SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE`) takes
    /// precedence at runtime — see `BillingPlan::snapshot_retention_days`.
    pub snapshot_retention_days: u32,
}

impl BillingPlan {
    pub fn config(&self) -> PlanConfig {
        match self {
            BillingPlan::Community(plan_config) => *plan_config,
            BillingPlan::Free(plan_config) => *plan_config,
            BillingPlan::Starter(plan_config) => *plan_config,
            BillingPlan::Pro(plan_config) => *plan_config,
            BillingPlan::Team(plan_config) => *plan_config,
            BillingPlan::Business(plan_config) => *plan_config,
            BillingPlan::Enterprise(plan_config) => *plan_config,
            BillingPlan::Demo(plan_config) => *plan_config,
            BillingPlan::CommercialSelfHosted(plan_config) => *plan_config,
            BillingPlan::SelfHostedStandard(plan_config) => *plan_config,
            BillingPlan::SelfHostedPlus(plan_config) => *plan_config,
        }
    }

    pub fn set_config(&mut self, config: PlanConfig) {
        match self {
            BillingPlan::Community(plan_config) => *plan_config = config,
            BillingPlan::Free(plan_config) => *plan_config = config,
            BillingPlan::Starter(plan_config) => *plan_config = config,
            BillingPlan::Pro(plan_config) => *plan_config = config,
            BillingPlan::Team(plan_config) => *plan_config = config,
            BillingPlan::Business(plan_config) => *plan_config = config,
            BillingPlan::Enterprise(plan_config) => *plan_config = config,
            BillingPlan::Demo(plan_config) => *plan_config = config,
            BillingPlan::CommercialSelfHosted(plan_config) => *plan_config = config,
            BillingPlan::SelfHostedStandard(plan_config) => *plan_config = config,
            BillingPlan::SelfHostedPlus(plan_config) => *plan_config = config,
        }
    }

    pub fn is_commercial(&self) -> bool {
        matches!(
            self,
            BillingPlan::Pro(_)
                | BillingPlan::Team(_)
                | BillingPlan::Business(_)
                | BillingPlan::Enterprise(_)
                | BillingPlan::CommercialSelfHosted(_)
                | BillingPlan::SelfHostedStandard(_)
                | BillingPlan::SelfHostedPlus(_)
                | BillingPlan::Demo(_)
        )
    }

    pub fn is_free(&self) -> bool {
        matches!(self, BillingPlan::Free(_))
    }

    pub fn is_demo(&self) -> bool {
        matches!(self, BillingPlan::Demo(_))
    }

    /// Plans where the customer hosts Scanopy themselves and Stripe is not in
    /// the loop. Use this to skip checks that only make sense for cloud plans.
    pub fn is_self_hosted(&self) -> bool {
        matches!(
            self,
            BillingPlan::Community(_)
                | BillingPlan::CommercialSelfHosted(_)
                | BillingPlan::SelfHostedStandard(_)
                | BillingPlan::SelfHostedPlus(_)
        )
    }

    /// Plans whose lifecycle is driven by a Stripe subscription. Drives the
    /// frontend hard-gate (no Stripe = no required modal), the post-Stripe
    /// poll predicate, the "needs a card" banner, and retention-offer
    /// surfaces (Discount panel, Discount chip).
    ///
    /// Free is intentionally NOT Stripe-managed: `activate_free_plan` bypasses
    /// Stripe entirely (no customer, no subscription created). Older Free orgs
    /// may carry a leftover Stripe customer + cancelled subscription from a
    /// previous model, but those subs are not active and don't drive plan
    /// state — `is_free()` is the right check for "this plan doesn't pay."
    ///
    /// Exhaustive on purpose — adding a new variant must classify it
    /// explicitly.
    pub fn is_stripe_managed(&self) -> bool {
        match self {
            BillingPlan::Starter(_)
            | BillingPlan::Pro(_)
            | BillingPlan::Team(_)
            | BillingPlan::Business(_)
            | BillingPlan::Enterprise(_) => true,
            BillingPlan::Free(_)
            | BillingPlan::Community(_)
            | BillingPlan::Demo(_)
            | BillingPlan::CommercialSelfHosted(_)
            | BillingPlan::SelfHostedStandard(_)
            | BillingPlan::SelfHostedPlus(_) => false,
        }
    }

    pub fn is_enterprise(&self) -> bool {
        matches!(self, BillingPlan::Enterprise(_))
    }

    pub fn host_limit(&self) -> Option<u64> {
        self.config().included_hosts
    }

    pub fn network_limit(&self) -> Option<u64> {
        self.config().included_networks
    }

    pub fn seat_limit(&self) -> Option<u64> {
        self.config().included_seats
    }

    /// Snapshot retention window in days for this plan. `0` means snapshots
    /// are unavailable. `env_override` (`SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE`)
    /// is a universal escape hatch — when set it wins over the fixture value
    /// for every plan tier. Self-hosted operators use it to extend retention
    /// without forking the plan fixture.
    pub fn snapshot_retention_days(&self, env_override: Option<u32>) -> u32 {
        env_override.unwrap_or_else(|| self.features().snapshot_retention_days)
    }

    /// Whether the plan supports having more than one member at all (a coarse
    /// capability gate for the invite UI/endpoint). The actual seat cap is
    /// enforced per-invite in the invites handler, so this only distinguishes
    /// solo plans from team plans:
    /// - unlimited seats (`None`) → yes
    /// - a finite cap above one seat (e.g. Standard's 25) → yes
    /// - can buy more seats (`seat_cents`) → yes
    /// - a single-seat plan (Free/Community/Starter/Pro) → no
    pub fn can_invite_users(&self) -> bool {
        match self.config().included_seats {
            None => true,
            Some(included_seats) => included_seats > 1 || self.config().seat_cents.is_some(),
        }
    }

    pub fn hosting(&self) -> Hosting {
        match self {
            BillingPlan::Community(_) => Hosting::SelfHosted,
            BillingPlan::CommercialSelfHosted(_) => Hosting::SelfHosted,
            BillingPlan::SelfHostedStandard(_) => Hosting::SelfHosted,
            BillingPlan::SelfHostedPlus(_) => Hosting::SelfHosted,
            BillingPlan::Enterprise(_) => Hosting::Any,
            _ => Hosting::Cloud, // Free, Starter, Pro, Team, Business, Demo
        }
    }

    /// How a prospect acquires this plan, so the website/app can pick a CTA
    /// from an explicit signal rather than inferring from hosting + commercial.
    pub fn purchase_flow(&self) -> PurchaseFlow {
        match self {
            BillingPlan::Starter(_)
            | BillingPlan::Pro(_)
            | BillingPlan::Team(_)
            | BillingPlan::Business(_) => PurchaseFlow::Stripe,
            BillingPlan::Enterprise(_)
            | BillingPlan::CommercialSelfHosted(_)
            | BillingPlan::SelfHostedStandard(_)
            | BillingPlan::SelfHostedPlus(_) => PurchaseFlow::Contact,
            BillingPlan::Community(_) | BillingPlan::Free(_) | BillingPlan::Demo(_) => {
                PurchaseFlow::None
            }
        }
    }

    /// Returns the next-lower-tier plan within this plan's ladder.
    /// Two independent ladders: the cloud tiers (Free → … → Enterprise) and
    /// the self-hosted commercial tiers (Community → Standard → Plus). Returns
    /// None for the bottom of a ladder and for plans in no ladder (Team, Demo).
    pub fn previous_tier(&self) -> Option<BillingPlanDiscriminants> {
        let ladders: [&[BillingPlanDiscriminants]; 2] = [
            &[
                BillingPlanDiscriminants::Free,
                BillingPlanDiscriminants::Starter,
                BillingPlanDiscriminants::Pro,
                BillingPlanDiscriminants::Business,
                BillingPlanDiscriminants::Enterprise,
            ],
            &[
                BillingPlanDiscriminants::Community,
                BillingPlanDiscriminants::SelfHostedStandard,
                BillingPlanDiscriminants::SelfHostedPlus,
            ],
        ];

        let my_disc = self.discriminant();
        for ladder in ladders {
            if let Some(idx) = ladder.iter().position(|d| *d == my_disc) {
                if idx == 0 {
                    return None;
                }
                return Some(ladder[idx - 1]);
            }
        }
        None
    }

    /// Returns feature IDs added by this plan over its previous tier.
    /// For Free: returns all enabled features (it's the baseline).
    /// For self-hosted plans with no previous tier: returns all enabled non-universal features.
    /// For cloud plans: returns features new vs the previous tier.
    pub fn incremental_features(&self) -> Vec<&'static str> {
        let enabled = self.enabled_feature_ids();

        match self.previous_tier() {
            Some(prev_disc) => {
                let prev_plan = Self::default_for_discriminant(prev_disc);
                match prev_plan {
                    Some(plan) => {
                        let prev_features = plan.enabled_feature_ids();
                        enabled.difference(&prev_features).copied().collect()
                    }
                    None => enabled.into_iter().collect(),
                }
            }
            None if self.is_free() => {
                // Free plan: show all enabled features (it's the baseline)
                enabled.into_iter().collect()
            }
            None => {
                // Self-hosted/other plans: show features beyond universal (Free) baseline
                let universal = Self::universal_feature_ids();
                enabled.difference(&universal).copied().collect()
            }
        }
    }

    /// Returns set of feature IDs where the feature is enabled on this plan.
    pub fn enabled_feature_ids(&self) -> HashSet<&'static str> {
        let features = self.features();
        let json = serde_json::to_value(&features).unwrap();
        let obj = json.as_object().unwrap();
        obj.iter()
            .filter(|(_, v)| v.as_bool().unwrap_or(false))
            .map(|(k, _)| {
                // Leak the key string so we get &'static str
                // This is fine since these are a small fixed set called infrequently
                let s: &'static str = Box::leak(k.clone().into_boxed_str());
                s
            })
            .collect()
    }

    /// Features that are universal across all plans (present on Free).
    fn universal_feature_ids() -> HashSet<&'static str> {
        use crate::server::billing::plans::get_free_plan;
        get_free_plan().enabled_feature_ids()
    }

    /// Whether the feature identified by `feature_id` is enabled on this plan.
    /// Boolean features → true/false directly; numeric features (e.g.
    /// `snapshot_retention_days`) are "enabled" when the value is > 0.
    pub fn has_feature(&self, feature_id: &str) -> bool {
        let features = self.features();
        let json = serde_json::to_value(&features).unwrap();
        let Some(v) = json.get(feature_id) else {
            return false;
        };
        if let Some(b) = v.as_bool() {
            return b;
        }
        v.as_u64().map(|n| n > 0).unwrap_or(false)
    }

    /// Build a default plan instance for a given discriminant (monthly, default config).
    pub fn default_for_discriminant(disc: BillingPlanDiscriminants) -> Option<BillingPlan> {
        use crate::server::billing::plans::*;

        match disc {
            BillingPlanDiscriminants::Free => Some(get_free_plan()),
            BillingPlanDiscriminants::Community => Some(get_community_plan()),
            BillingPlanDiscriminants::Enterprise => Some(get_enterprise_plan()),
            BillingPlanDiscriminants::CommercialSelfHosted => {
                Some(get_commercial_self_hosted_plan())
            }
            BillingPlanDiscriminants::SelfHostedStandard => Some(get_self_hosted_standard_plan()),
            BillingPlanDiscriminants::SelfHostedPlus => Some(get_self_hosted_plus_plan()),
            // For purchasable plans, find them from the default list
            _ => get_purchasable_plans()
                .into_iter()
                .find(|p| p.discriminant() == disc),
        }
    }

    pub fn custom_price(&self) -> Option<&str> {
        match self {
            BillingPlan::Enterprise(_) => Some("Custom"),
            BillingPlan::Community(_) | BillingPlan::Free(_) => Some("Free"),
            BillingPlan::CommercialSelfHosted(_) => Some("Custom"),
            _ => None,
        }
    }

    pub fn stripe_product_id(&self) -> String {
        self.to_string().to_lowercase()
    }

    pub fn stripe_base_price_lookup_key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.stripe_product_id(),
            self.config().base_cents,
            self.config().rate
        )
    }

    pub fn stripe_seat_addon_price_lookup_key(&self) -> Option<String> {
        self.config().seat_cents.map(|c| {
            format!(
                "{}_seats_{}_{}",
                self.stripe_product_id(),
                c,
                self.config().rate
            )
        })
    }

    pub fn stripe_network_addon_price_lookup_key(&self) -> Option<String> {
        self.config().network_cents.map(|c| {
            format!(
                "{}_networks_{}_{}",
                self.stripe_product_id(),
                c,
                self.config().rate
            )
        })
    }

    pub fn features(&self) -> BillingPlanFeatures {
        match self {
            // The self-hosted edition: every product capability is on. The
            // remaining `false`s are services Scanopy LLC sells (onboarding
            // call, managed deployment, support channels), not features of the
            // software, so a self-hosted instance can't grant them to itself.
            BillingPlan::Community { .. } => BillingPlanFeatures {
                saml: true,
                air_gapped_deployment: true,
                share_views: true,
                onboarding_call: false,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                api_access: true,
                custom_sso: true,
                managed_deployment: false,
                whitelabeling: true,
                live_chat_support: false,
                embeds: true,
                email_support: false,
                priority_support: false,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::Free { .. } => BillingPlanFeatures {
                saml: false,
                air_gapped_deployment: false,
                share_views: false,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: false,
                custom_sso: false,
                api_access: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: false,
                email_support: false,
                priority_support: false,
                network_mapping: true,
                png_export: true,
                svg_export: false,
                mermaid_export: false,
                confluence_export: false,
                pdf_export: false,
                html_export: false,
                scheduled_discovery: false,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 0,
            },
            BillingPlan::Starter { .. } => BillingPlanFeatures {
                saml: false,
                air_gapped_deployment: false,
                share_views: true,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: true,
                custom_sso: false,
                api_access: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: false,
                email_support: true,
                priority_support: false,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: false,
                confluence_export: false,
                pdf_export: false,
                html_export: false,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 7,
            },
            BillingPlan::Pro { .. } => BillingPlanFeatures {
                saml: false,
                air_gapped_deployment: false,
                share_views: true,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: true,
                api_access: true,
                custom_sso: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: false,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: false,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 30,
            },
            BillingPlan::Team { .. } => BillingPlanFeatures {
                saml: false,
                air_gapped_deployment: false,
                share_views: true,
                onboarding_call: true,
                webhooks: false,
                audit_logs: false,
                remove_created_with: true,
                custom_sso: false,
                api_access: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::Business { .. } => BillingPlanFeatures {
                saml: false,
                air_gapped_deployment: false,
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                custom_sso: false,
                api_access: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::Enterprise { .. } => BillingPlanFeatures {
                saml: true,
                air_gapped_deployment: true,
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                custom_sso: true,
                api_access: true,
                managed_deployment: true,
                whitelabeling: true,
                live_chat_support: true,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::Demo { .. } => BillingPlanFeatures {
                saml: true,
                air_gapped_deployment: true,
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                custom_sso: true,
                api_access: true,
                managed_deployment: true,
                whitelabeling: true,
                live_chat_support: true,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::CommercialSelfHosted { .. } => BillingPlanFeatures {
                saml: true,
                air_gapped_deployment: true,
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                api_access: true,
                custom_sso: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::SelfHostedStandard { .. } => BillingPlanFeatures {
                saml: true,
                air_gapped_deployment: false,
                share_views: true,
                onboarding_call: false,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                api_access: true,
                custom_sso: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: false,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::SelfHostedPlus { .. } => BillingPlanFeatures {
                saml: true,
                air_gapped_deployment: true,
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                api_access: true,
                custom_sso: true,
                managed_deployment: false,
                whitelabeling: true,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<Feature>> for BillingPlanFeatures {
    fn into(self) -> Vec<Feature> {
        let mut features = vec![];

        let BillingPlanFeatures {
            share_views,
            onboarding_call,
            webhooks,
            audit_logs,
            remove_created_with,
            custom_sso,
            saml,
            air_gapped_deployment,
            managed_deployment,
            whitelabeling,
            api_access,
            live_chat_support,
            embeds,
            email_support,
            priority_support,
            network_mapping,
            png_export,
            svg_export,
            mermaid_export,
            confluence_export,
            pdf_export,
            html_export,
            scheduled_discovery,
            discovery_integrations,
            csv_export,
            snapshot_retention_days,
        } = self;

        if share_views {
            features.push(Feature::ShareViews)
        }

        if custom_sso {
            features.push(Feature::CustomSso)
        }

        if saml {
            features.push(Feature::Saml)
        }

        if air_gapped_deployment {
            features.push(Feature::AirGappedDeployment)
        }

        if api_access {
            features.push(Feature::ApiAccess)
        }

        if managed_deployment {
            features.push(Feature::ManagedDeployment)
        }

        if embeds {
            features.push(Feature::Embeds)
        }

        if whitelabeling {
            features.push(Feature::Whitelabeling)
        }

        if live_chat_support {
            features.push(Feature::LiveChatSupport)
        }

        if priority_support {
            features.push(Feature::PrioritySupport)
        }

        if email_support {
            features.push(Feature::EmailSupport)
        }

        if onboarding_call {
            features.push(Feature::OnboardingCall)
        }

        if webhooks {
            features.push(Feature::Webhooks);
        }

        if audit_logs {
            features.push(Feature::AuditLogs)
        }

        if remove_created_with {
            features.push(Feature::RemoveCreatedWith)
        }

        if network_mapping {
            features.push(Feature::NetworkMapping)
        }

        if png_export {
            features.push(Feature::PngExport)
        }

        if svg_export {
            features.push(Feature::SvgExport)
        }

        if mermaid_export {
            features.push(Feature::MermaidExport)
        }

        if confluence_export {
            features.push(Feature::ConfluenceExport)
        }

        if pdf_export {
            features.push(Feature::PdfExport)
        }

        if html_export {
            features.push(Feature::HtmlExport)
        }

        if scheduled_discovery {
            features.push(Feature::ScheduledDiscovery)
        }

        if discovery_integrations {
            features.push(Feature::DiscoveryIntegrations)
        }

        if csv_export {
            features.push(Feature::CsvExport)
        }

        if snapshot_retention_days > 0 {
            features.push(Feature::SnapshotRetentionDays)
        }

        features
    }
}

impl HasId for BillingPlan {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for BillingPlan {
    fn icon(&self) -> Icon {
        match self {
            BillingPlan::Community { .. } => Icon::Heart,
            BillingPlan::Free { .. } => Icon::Gift,
            BillingPlan::Starter { .. } => Icon::ThumbsUp,
            BillingPlan::Pro { .. } => Icon::Zap,
            BillingPlan::Team { .. } => Icon::Users,
            BillingPlan::Business { .. } => Icon::Briefcase,
            BillingPlan::Enterprise { .. } => Icon::Building,
            BillingPlan::Demo { .. } => Icon::TestTube,
            BillingPlan::CommercialSelfHosted { .. } => Icon::ServerCog,
            BillingPlan::SelfHostedStandard { .. } => Icon::Server,
            BillingPlan::SelfHostedPlus { .. } => Icon::ServerCog,
        }
    }

    fn color(&self) -> Color {
        match self {
            BillingPlan::Community { .. } => Color::Pink,
            BillingPlan::Free { .. } => Color::Green,
            BillingPlan::Starter { .. } => Color::Blue,
            BillingPlan::Pro { .. } => Color::Yellow,
            BillingPlan::Team { .. } => Color::Orange,
            BillingPlan::Business { .. } => Color::Indigo,
            BillingPlan::Enterprise { .. } => Color::Teal,
            BillingPlan::Demo { .. } => Color::Purple,
            BillingPlan::CommercialSelfHosted { .. } => Color::Gray,
            BillingPlan::SelfHostedStandard { .. } => Color::Blue,
            BillingPlan::SelfHostedPlus { .. } => Color::Indigo,
        }
    }
}

impl TypeMetadataProvider for BillingPlan {
    fn name(&self) -> &'static str {
        match self {
            BillingPlan::Community { .. } => "Community Edition",
            BillingPlan::Free { .. } => "Free",
            BillingPlan::Starter { .. } => "Starter",
            BillingPlan::Pro { .. } => "Pro",
            BillingPlan::Team { .. } => "Team",
            BillingPlan::Business { .. } => "Business",
            BillingPlan::Enterprise { .. } => "Enterprise",
            BillingPlan::Demo { .. } => "Demo",
            BillingPlan::CommercialSelfHosted { .. } => "Commercial Edition",
            BillingPlan::SelfHostedStandard { .. } => "Self-Hosted Standard",
            BillingPlan::SelfHostedPlus { .. } => "Self-Hosted Plus",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            BillingPlan::Community { .. } => {
                "Community edition for self-hosting and open-source enthusiasts"
            }
            BillingPlan::Free { .. } => "For hobbyists exploring a small network",
            BillingPlan::Starter { .. } => "For homelabbers automating documentation",
            BillingPlan::Pro { .. } => "For IT pros managing multiple networks",
            BillingPlan::Team { .. } => {
                "Collaborate on infrastructure documentation with your team"
            }
            BillingPlan::Business { .. } => "For MSPs managing client infrastructure",
            BillingPlan::Enterprise { .. } => "For organizations needing custom deployment",
            BillingPlan::Demo { .. } => "Demo mode",
            BillingPlan::CommercialSelfHosted { .. } => {
                "Commercial edition for businesses with on-premise deployments"
            }
            BillingPlan::SelfHostedStandard { .. } => {
                "Commercial self-hosted license for a single organization"
            }
            BillingPlan::SelfHostedPlus { .. } => {
                "Multi-organization self-hosted license with offline keys and SAML"
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let config = self.config();
        let previous_tier = self
            .previous_tier()
            .and_then(BillingPlan::default_for_discriminant)
            .map(|p| p.id());

        serde_json::json!({
            // Pricing information
            "base_cents": config.base_cents,
            "rate": config.rate,
            "trial_days": config.trial_days,
            "seat_cents": config.seat_cents,
            "network_cents": config.network_cents,
            "host_cents": config.host_cents,
            "included_seats": config.included_seats,
            "included_networks": config.included_networks,
            "included_hosts": config.included_hosts,
            "included_orgs": config.included_orgs,
            // Feature flags and metadata
            "features": self.features(),
            "is_commercial": self.is_commercial(),
            "is_stripe_managed": self.is_stripe_managed(),
            "is_free": self.is_free(),
            "is_demo": self.is_demo(),
            "is_enterprise": self.is_enterprise(),
            "hosting": self.hosting(),
            "custom_price": self.custom_price(),
            "purchase_flow": self.purchase_flow(),
            // Tier relationship
            "incremental_features": self.incremental_features(),
            "previous_tier": previous_tier
        })
    }
}
