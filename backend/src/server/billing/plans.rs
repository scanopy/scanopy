use super::types::base::{BillingPlan, BillingRate, PlanConfig};

pub const YEARLY_DISCOUNT: f32 = 0.2;

/// Returns the canonical list of billing plans for Scanopy.
/// This is the single source of truth for plan definitions.
fn get_default_plans() -> Vec<BillingPlan> {
    vec![
        BillingPlan::Starter(PlanConfig {
            base_cents: 1499,
            rate: BillingRate::Month,
            trial_days: 0,
            seat_cents: None,
            network_cents: None,
            host_cents: None,
            included_seats: Some(1),
            included_networks: Some(1),
            included_hosts: None,
        }),
        BillingPlan::Pro(PlanConfig {
            base_cents: 4999,
            rate: BillingRate::Month,
            trial_days: 14,
            seat_cents: None,
            network_cents: Some(1000),
            host_cents: None,
            included_seats: Some(1),
            included_networks: Some(3),
            included_hosts: None,
        }),
        BillingPlan::Business(PlanConfig {
            base_cents: 9999,
            rate: BillingRate::Month,
            trial_days: 14,
            seat_cents: Some(1000),
            network_cents: Some(700),
            host_cents: None,
            included_seats: Some(5),
            included_networks: Some(15),
            included_hosts: None,
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
    })
}

pub fn get_website_fixture_plans() -> Vec<BillingPlan> {
    // Free is already included via get_purchasable_plans(), so exclude it from
    // non_saas_plans to avoid duplicate entries in the generated fixture.
    let non_saas_plans = [
        get_enterprise_plan(),
        get_community_plan(),
        get_commercial_self_hosted_plan(),
    ];

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

    all_plans
}
