use super::types::base::{BillingPlan, BillingRate, PlanConfig};
use crate::server::license::types::{LicenseClaims, LicensePlan};

pub const YEARLY_DISCOUNT: f32 = 0.2;

// Self-hosted commercial tier pricing and caps. Founder-set; edit here.
// Prices are the full annual charge in cents (these tiers are annual-only).
pub const SELF_HOSTED_STANDARD_ANNUAL_CENTS: i64 = 400_000; // $4,000/yr
pub const SELF_HOSTED_STANDARD_NETWORKS: u64 = 50;
pub const SELF_HOSTED_STANDARD_SEATS: u64 = 25;
pub const SELF_HOSTED_STANDARD_ORGS: u64 = 1;

pub const SELF_HOSTED_PLUS_ANNUAL_CENTS: i64 = 600_000; // $6,000/yr
pub const SELF_HOSTED_PLUS_NETWORKS: u64 = 100;
pub const SELF_HOSTED_PLUS_SEATS: u64 = 50;
pub const SELF_HOSTED_PLUS_ORGS: u64 = 5;

/// Card-less trial length for both paid self-hosted tiers.
pub const SELF_HOSTED_TRIAL_DAYS: u32 = 14;

/// Returns the canonical list of purchasable cloud plans for Scanopy.
/// This is the single source of truth for plan definitions.
///
/// Starter is no longer sold: the variant stays for the organizations that
/// hold it, but it is absent here so it reaches neither Stripe product
/// initialization, checkout validation, nor the plan picker.
fn get_default_plans() -> Vec<BillingPlan> {
    vec![
        BillingPlan::Pro(PlanConfig {
            base_cents: 9999,
            rate: BillingRate::Month,
            trial_days: 14,
            seat_cents: None,
            network_cents: Some(2500),
            host_cents: None,
            included_seats: Some(1),
            included_networks: Some(3),
            included_hosts: None,
            included_orgs: None,
        }),
        BillingPlan::Business(PlanConfig {
            base_cents: 24999,
            rate: BillingRate::Month,
            trial_days: 14,
            seat_cents: Some(1500),
            network_cents: Some(1500),
            host_cents: None,
            included_seats: Some(5),
            included_networks: Some(15),
            included_hosts: None,
            included_orgs: None,
        }),
    ]
}

pub fn get_enterprise_plan() -> BillingPlan {
    BillingPlan::Enterprise(PlanConfig {
        base_cents: 0,
        rate: BillingRate::Month,
        trial_days: 0,
        seat_cents: None,
        network_cents: None,
        host_cents: None,
        included_seats: None,
        included_networks: None,
        included_hosts: None,
        included_orgs: None,
    })
}

pub fn get_free_plan() -> BillingPlan {
    BillingPlan::Free(PlanConfig {
        base_cents: 0,
        rate: BillingRate::Month,
        trial_days: 0,
        seat_cents: None,
        network_cents: None,
        host_cents: None,
        included_seats: Some(1),
        included_networks: Some(1),
        included_hosts: Some(25),
        included_orgs: None,
    })
}

pub fn get_community_plan() -> BillingPlan {
    BillingPlan::Community(PlanConfig {
        base_cents: 0,
        rate: BillingRate::Month,
        trial_days: 0,
        seat_cents: None,
        network_cents: None,
        host_cents: None,
        included_seats: Some(1),
        included_networks: Some(1),
        included_hosts: None,
        included_orgs: Some(1),
    })
}

pub fn get_commercial_self_hosted_plan() -> BillingPlan {
    BillingPlan::CommercialSelfHosted(PlanConfig {
        base_cents: 0,
        rate: BillingRate::Month,
        trial_days: 0,
        seat_cents: None,
        network_cents: None,
        host_cents: None,
        included_seats: None,
        included_networks: None,
        included_hosts: None,
        included_orgs: None,
    })
}

/// Self-Hosted Standard: single-org commercial license, annual-only, published
/// price. Phone-home license (not air-gapped). Hard caps, no Stripe overage.
pub fn get_self_hosted_standard_plan() -> BillingPlan {
    BillingPlan::SelfHostedStandard(PlanConfig {
        base_cents: SELF_HOSTED_STANDARD_ANNUAL_CENTS,
        rate: BillingRate::Year,
        trial_days: SELF_HOSTED_TRIAL_DAYS,
        seat_cents: None,
        network_cents: None,
        host_cents: None,
        included_seats: Some(SELF_HOSTED_STANDARD_SEATS),
        included_networks: Some(SELF_HOSTED_STANDARD_NETWORKS),
        included_hosts: None,
        included_orgs: Some(SELF_HOSTED_STANDARD_ORGS),
    })
}

/// Self-Hosted Plus: multi-org commercial license with offline keys and SAML,
/// annual-only, published price. Air-gapped. Hard caps, no Stripe overage.
pub fn get_self_hosted_plus_plan() -> BillingPlan {
    BillingPlan::SelfHostedPlus(PlanConfig {
        base_cents: SELF_HOSTED_PLUS_ANNUAL_CENTS,
        rate: BillingRate::Year,
        trial_days: SELF_HOSTED_TRIAL_DAYS,
        seat_cents: None,
        network_cents: None,
        host_cents: None,
        included_seats: Some(SELF_HOSTED_PLUS_SEATS),
        included_networks: Some(SELF_HOSTED_PLUS_NETWORKS),
        included_hosts: None,
        included_orgs: Some(SELF_HOSTED_PLUS_ORGS),
    })
}

/// Resolve the self-hosted commercial plan a license key entitles. Claim-absent
/// keys (legacy and custom/grandfathered) map to the unlimited
/// CommercialSelfHosted plan, so existing customer keys keep working unchanged.
pub fn plan_for_license(claims: &LicenseClaims) -> BillingPlan {
    match claims.plan {
        Some(LicensePlan::Standard) => get_self_hosted_standard_plan(),
        Some(LicensePlan::Plus) => get_self_hosted_plus_plan(),
        None => get_commercial_self_hosted_plan(),
    }
}

pub fn get_website_fixture_plans() -> Vec<BillingPlan> {
    // Enterprise and Community ship as monthly + yearly rows. The two paid
    // self-hosted tiers are annual-only and come from get_purchasable_plans.
    // CommercialSelfHosted is intentionally excluded — it is a
    // legacy/grandfather-only plan, no longer published; it remains available
    // via billing-plans-all.json.
    let non_saas_plans = [get_enterprise_plan(), get_community_plan()];

    let non_saas_yearly = non_saas_plans.iter().map(|p| p.to_yearly(YEARLY_DISCOUNT));

    let mut all_plans = get_purchasable_plans();
    all_plans.extend(non_saas_plans);
    all_plans.extend(non_saas_yearly);
    // Add Free yearly variant (monthly Free is already in get_purchasable_plans)
    all_plans.push(get_free_plan().to_yearly(YEARLY_DISCOUNT));

    all_plans
}

/// Returns both monthly and yearly versions of all plans, plus the Free plan.
/// Yearly plans get a 20% discount.
pub fn get_purchasable_plans() -> Vec<BillingPlan> {
    let monthly_plans = get_default_plans();
    let mut all_plans = monthly_plans.clone();

    // Add yearly versions with 20% discount
    for plan in monthly_plans {
        all_plans.push(plan.to_yearly(YEARLY_DISCOUNT));
    }

    // Free plan (no yearly variant needed)
    all_plans.push(get_free_plan());

    // Paid self-hosted tiers: annual-only, constructed directly as Year rows
    // (so they bypass the monthly → yearly discount above).
    all_plans.push(get_self_hosted_standard_plan());
    all_plans.push(get_self_hosted_plus_plan());

    all_plans
}
