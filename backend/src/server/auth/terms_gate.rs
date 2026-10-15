//! The terms re-acceptance gate.
//!
//! A user whose `terms_accepted_at` is before [`TERMS_ACCEPTANCE_EFFECTIVE`],
//! or who has none on record, must accept the terms again before the API
//! serves their session (`middleware::terms::terms_gate_middleware`). To gate
//! everyone again after the terms change, move it to the new terms'
//! publication time.
//!
//! The gate applies only where acceptance is collected: Scanopy Cloud
//! ([`DeploymentType::requires_terms_acceptance`]). Registration and OIDC
//! sign-up ask for it only there, so a self-hosted server has nothing to gate
//! on, and this is a constant rather than server config. Demo orgs are exempt:
//! their one shared account would accept on behalf of every visitor.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

use crate::server::{billing::types::base::BillingPlan, config::DeploymentType};

/// When the terms every user must have accepted were published: 10:00 EDT on 2026-10-14. An
/// acceptance before this moment was of the earlier terms, even on the same day.
pub const TERMS_ACCEPTANCE_EFFECTIVE: DateTime<Utc> = {
    let Some(date) = NaiveDate::from_ymd_opt(2026, 10, 14) else {
        panic!("invalid terms effective date");
    };
    let Some(time) = NaiveTime::from_hms_opt(14, 0, 0) else {
        panic!("invalid terms effective time");
    };
    NaiveDateTime::new(date, time).and_utc()
};

/// Whether the gate applies at all. Off: nobody is asked to re-accept, and the modal never opens.
/// Set to `true` to enforce [`TERMS_ACCEPTANCE_EFFECTIVE`] again.
pub const TERMS_GATE_ACTIVE: bool = false;

fn acceptance_required_since(
    deployment_type: DeploymentType,
    plan: Option<&BillingPlan>,
    accepted_at: Option<DateTime<Utc>>,
    effective: DateTime<Utc>,
) -> bool {
    deployment_type.requires_terms_acceptance()
        && !plan.is_some_and(BillingPlan::is_demo)
        && !accepted_at.is_some_and(|at| at >= effective)
}

/// Whether a user in an org on `plan` must accept the current terms before
/// using the app.
pub fn acceptance_required(
    deployment_type: DeploymentType,
    plan: Option<&BillingPlan>,
    accepted_at: Option<DateTime<Utc>>,
) -> bool {
    TERMS_GATE_ACTIVE
        && acceptance_required_since(
            deployment_type,
            plan,
            accepted_at,
            TERMS_ACCEPTANCE_EFFECTIVE,
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::plans::get_free_plan;
    use chrono::Duration;

    const CLOUD: DeploymentType = DeploymentType::Cloud;

    #[test]
    fn missing_or_earlier_acceptance_is_gated_and_current_acceptance_is_not() {
        let plan = get_free_plan();
        let effective = TERMS_ACCEPTANCE_EFFECTIVE;
        let required = |at| acceptance_required_since(CLOUD, Some(&plan), at, effective);

        assert!(required(None));
        assert!(required(Some(effective - Duration::seconds(1))));
        assert!(!required(Some(effective)));
        assert!(!required(Some(effective + Duration::days(3))));
    }

    #[test]
    fn an_acceptance_earlier_on_the_publication_day_is_gated() {
        let plan = get_free_plan();
        let same_day_before = TERMS_ACCEPTANCE_EFFECTIVE
            .date_naive()
            .and_time(NaiveTime::MIN);
        assert!(acceptance_required_since(
            CLOUD,
            Some(&plan),
            Some(same_day_before.and_utc()),
            TERMS_ACCEPTANCE_EFFECTIVE
        ));
    }

    #[test]
    fn moving_the_effective_date_later_gates_a_user_who_accepted_the_earlier_terms() {
        let plan = get_free_plan();
        let accepted = Some(TERMS_ACCEPTANCE_EFFECTIVE);

        assert!(!acceptance_required(CLOUD, Some(&plan), accepted));
        let later = TERMS_ACCEPTANCE_EFFECTIVE + Duration::days(90);
        assert!(acceptance_required_since(
            CLOUD,
            Some(&plan),
            accepted,
            later
        ));
    }

    #[test]
    fn self_hosted_servers_and_demo_orgs_are_never_gated() {
        let demo = BillingPlan::Demo(crate::server::billing::types::base::PlanConfig::default());
        for self_hosted in [DeploymentType::Commercial, DeploymentType::Community] {
            assert!(!acceptance_required(
                self_hosted,
                Some(&get_free_plan()),
                None
            ));
        }
        assert!(!acceptance_required(CLOUD, Some(&demo), None));
    }
}
