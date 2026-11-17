//! The plan a signup's intents call for, and why.
//!
//! Each [`SignupIntent`] names the features it needs. The recommended plan is the lowest plan on
//! a ladder that has every one of them, so the recommendation follows the plan flags in
//! `plans.rs` with no second table to keep in step.
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::{
    billing::types::{
        base::{BillingPlan, BillingPlanDiscriminants},
        features::{CLOUD_LADDER, SELF_HOSTED_LADDER},
    },
    organizations::r#impl::intent::SignupIntent,
    shared::types::metadata::HasId,
};

/// One picked intent the recommended plan serves, and the features it needs that the plan below
/// lacks. On the lowest plan, every feature the intent needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RecommendationReason {
    pub intent: SignupIntent,
    /// Feature ids, as in `features.json`.
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlanRecommendation {
    /// The recommended plan's type, as in `BillingPlan.type`.
    pub plan: String,
    /// The intents that call for this plan rather than the one below it.
    pub reasons: Vec<RecommendationReason>,
}

/// The recommendation on each hosting tab of the plan picker.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlanRecommendations {
    pub cloud: Option<PlanRecommendation>,
    pub self_hosted: Option<PlanRecommendation>,
}

impl PlanRecommendations {
    /// `None` on both tabs when the org was never asked, skipped, or picked only "Something
    /// else".
    pub fn for_intents(intents: Option<&[SignupIntent]>) -> Self {
        let intents = intents.unwrap_or_default();
        Self {
            cloud: recommend(intents, &CLOUD_LADDER),
            self_hosted: recommend(intents, &SELF_HOSTED_LADDER),
        }
    }
}

/// The lowest plan on `ladder` with every feature `intents` need, with the reasons. `None` when
/// the intents need no feature.
pub fn recommend(
    intents: &[SignupIntent],
    ladder: &[BillingPlanDiscriminants],
) -> Option<PlanRecommendation> {
    let index_on_ladder = |disc: BillingPlanDiscriminants| ladder.iter().position(|d| *d == disc);

    // A feature no plan on the ladder has cannot be recommended for, so it is left out.
    let index = intents
        .iter()
        .flat_map(|intent| intent.features())
        .filter_map(|feature| feature.minimum_plan_on(ladder).and_then(index_on_ladder))
        .max()?;

    let plan = BillingPlan::default_for_discriminant(ladder[index])?;
    let below = index
        .checked_sub(1)
        .and_then(|i| BillingPlan::default_for_discriminant(ladder[i]));

    let reasons = intents
        .iter()
        .filter_map(|intent| {
            let features: Vec<String> = intent
                .features()
                .iter()
                .map(|feature| feature.id())
                .filter(|id| plan.has_feature(id))
                .filter(|id| below.is_none_or(|below| !below.has_feature(id)))
                .map(String::from)
                .collect();
            (!features.is_empty()).then_some(RecommendationReason {
                intent: *intent,
                features,
            })
        })
        .collect();

    Some(PlanRecommendation {
        plan: plan.id().to_string(),
        reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    fn plan(id: &str) -> BillingPlan {
        CLOUD_LADDER
            .iter()
            .chain(SELF_HOSTED_LADDER.iter())
            .filter_map(|d| BillingPlan::default_for_discriminant(*d))
            .find(|p| p.id() == id)
            .unwrap()
    }

    /// Every pair of intents, including a single intent picked twice.
    fn picks() -> Vec<Vec<SignupIntent>> {
        let all: Vec<SignupIntent> = SignupIntent::iter().collect();
        let mut picks = Vec::new();
        for a in &all {
            for b in &all {
                picks.push(vec![*a, *b]);
            }
        }
        picks
    }

    /// The recommended plan has every feature the picked intents need, and the plan below it
    /// lacks at least one, on both ladders and for every pair of intents.
    #[test]
    fn the_recommendation_is_the_lowest_plan_with_every_needed_feature() {
        for ladder in [&CLOUD_LADDER[..], &SELF_HOSTED_LADDER[..]] {
            for intents in picks() {
                let needed: Vec<&str> = intents
                    .iter()
                    .flat_map(|i| i.features())
                    .map(|f| f.id())
                    .collect();
                let Some(rec) = recommend(&intents, ladder) else {
                    assert!(needed.is_empty(), "{intents:?} needs {needed:?}");
                    continue;
                };
                let recommended = plan(&rec.plan);
                assert!(
                    needed.iter().all(|id| recommended.has_feature(id)),
                    "{intents:?} on {}",
                    rec.plan
                );
                let index = ladder
                    .iter()
                    .position(|d| {
                        BillingPlan::default_for_discriminant(*d).unwrap().id() == rec.plan
                    })
                    .unwrap();
                if let Some(below) = index.checked_sub(1) {
                    let below = BillingPlan::default_for_discriminant(ladder[below]).unwrap();
                    assert!(
                        needed.iter().any(|id| !below.has_feature(id)),
                        "{intents:?}: {} would do",
                        below.id()
                    );
                }
            }
        }
    }

    /// A reason names only features the plan below lacks, so it says why this plan and not the
    /// one below. The intents that drove the choice always have a reason.
    #[test]
    fn each_reason_names_what_the_plan_below_lacks() {
        for intents in picks() {
            let Some(rec) = recommend(&intents, &CLOUD_LADDER) else {
                continue;
            };
            assert!(!rec.reasons.is_empty(), "{intents:?}");
            let index = CLOUD_LADDER
                .iter()
                .position(|d| BillingPlan::default_for_discriminant(*d).unwrap().id() == rec.plan)
                .unwrap();
            if let Some(below) = index.checked_sub(1) {
                let below = BillingPlan::default_for_discriminant(CLOUD_LADDER[below]).unwrap();
                for reason in &rec.reasons {
                    assert!(reason.features.iter().all(|id| !below.has_feature(id)));
                }
            }
        }
    }

    #[test]
    fn no_answer_skip_or_only_something_else_recommends_nothing() {
        assert_eq!(
            PlanRecommendations::for_intents(None),
            PlanRecommendations::default()
        );
        assert_eq!(
            PlanRecommendations::for_intents(Some(&[])),
            PlanRecommendations::default()
        );
        assert_eq!(
            PlanRecommendations::for_intents(Some(&[SignupIntent::Other])),
            PlanRecommendations::default()
        );
    }
}
