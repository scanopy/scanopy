use crate::server::{
    billing::types::features::Feature,
    shared::types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::Hash;
use stripe_product::price::CreatePriceRecurringInterval;
use strum::{Display, EnumDiscriminants, EnumIter, IntoDiscriminant, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

mod cancel_flow;
mod invoices;
mod plans;
mod status;
pub use cancel_flow::*;
pub use invoices::*;
pub use plans::*;
pub use status::*;

#[cfg(test)]
mod cancel_modal_tests {
    use super::*;

    fn save_offers_for(reason: CancelReason) -> Vec<String> {
        let metadata = reason.metadata();
        metadata["save_offers"]
            .as_array()
            .expect("save_offers should be an array")
            .iter()
            .map(|v| v.as_str().expect("offer should be a string").to_string())
            .collect()
    }

    #[test]
    fn cancel_reason_too_expensive_offers_pause_and_discount() {
        assert_eq!(
            save_offers_for(CancelReason::TooExpensive),
            vec!["pause", "discount"]
        );
    }

    #[test]
    fn cancel_reason_unused_offers_pause_only() {
        assert_eq!(save_offers_for(CancelReason::Unused), vec!["pause"]);
    }

    #[test]
    fn cancel_reasons_without_offers_return_empty_list() {
        for reason in [
            CancelReason::MissingFeatures,
            CancelReason::SwitchedService,
            CancelReason::CustomerService,
            CancelReason::LowQuality,
            CancelReason::TooComplex,
            CancelReason::Other,
        ] {
            assert!(
                save_offers_for(reason).is_empty(),
                "{reason:?} should have no save offers"
            );
        }
    }

    #[test]
    fn cancel_reason_id_is_snake_case() {
        assert_eq!(CancelReason::TooExpensive.id(), "too_expensive");
        assert_eq!(CancelReason::Other.id(), "other");
    }

    #[test]
    fn save_offer_id_is_snake_case() {
        assert_eq!(SaveOffer::Pause.id(), "pause");
        assert_eq!(SaveOffer::Discount.id(), "discount");
        assert_eq!(SaveOffer::Downgrade.id(), "downgrade");
    }

    #[test]
    fn plan_status_writes_canonical_spelling() {
        // Wire writes always use the British spelling so downstream
        // string comparisons (frontend `'cancelled'`, Brevo sync, etc)
        // stay consistent.
        assert_eq!(PlanStatus::Cancelled.to_string(), "cancelled");
        assert_eq!(
            serde_json::to_string(&PlanStatus::Cancelled).unwrap(),
            r#""cancelled""#
        );
    }

    #[test]
    fn plan_status_parses_either_spelling_from_storage() {
        // Existing DB rows may carry either spelling: pre-Phase-5 writers
        // echoed Stripe's American `"canceled"`; current writers use the
        // canonical `"cancelled"`. Both must round-trip back to the
        // typed variant on read.
        use std::str::FromStr;
        assert_eq!(PlanStatus::from_str("cancelled"), Ok(PlanStatus::Cancelled));
        assert_eq!(PlanStatus::from_str("canceled"), Ok(PlanStatus::Cancelled));

        // Serde path (JSONB, API request bodies) honors the alias too.
        let from_canonical: PlanStatus = serde_json::from_str(r#""cancelled""#).unwrap();
        let from_legacy: PlanStatus = serde_json::from_str(r#""canceled""#).unwrap();
        assert_eq!(from_canonical, PlanStatus::Cancelled);
        assert_eq!(from_legacy, PlanStatus::Cancelled);
    }
}

#[cfg(test)]
mod snapshot_retention_tests {
    use super::*;
    use crate::server::billing::types::base::PlanConfig;

    fn cfg() -> PlanConfig {
        PlanConfig::default()
    }

    #[test]
    fn no_override_returns_plan_fixture_value() {
        assert_eq!(BillingPlan::Free(cfg()).snapshot_retention_days(None), 0);
        assert_eq!(BillingPlan::Starter(cfg()).snapshot_retention_days(None), 7);
        assert_eq!(BillingPlan::Pro(cfg()).snapshot_retention_days(None), 30);
        assert_eq!(
            BillingPlan::Business(cfg()).snapshot_retention_days(None),
            90
        );
        assert_eq!(BillingPlan::Team(cfg()).snapshot_retention_days(None), 90);
        assert_eq!(
            BillingPlan::Community(cfg()).snapshot_retention_days(None),
            90
        );
        assert_eq!(
            BillingPlan::Enterprise(cfg()).snapshot_retention_days(None),
            90
        );
        assert_eq!(BillingPlan::Demo(cfg()).snapshot_retention_days(None), 90);
        assert_eq!(
            BillingPlan::CommercialSelfHosted(cfg()).snapshot_retention_days(None),
            90
        );
    }

    #[test]
    fn env_override_wins_for_every_plan_tier() {
        let override_value = Some(365);
        assert_eq!(
            BillingPlan::Free(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Starter(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Pro(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Business(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Community(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Enterprise(cfg()).snapshot_retention_days(override_value),
            365
        );
    }

    #[test]
    fn override_of_zero_disables_snapshots() {
        // Universal escape hatch: an operator can set the override to 0 to
        // disable snapshots on every plan (e.g. to drain a self-hosted box).
        assert_eq!(BillingPlan::Pro(cfg()).snapshot_retention_days(Some(0)), 0);
        assert_eq!(
            BillingPlan::Business(cfg()).snapshot_retention_days(Some(0)),
            0
        );
    }
}

#[cfg(test)]
mod can_invite_users_tests {
    use crate::server::billing::plans::*;
    use crate::server::shared::types::metadata::HasId;

    // A team plan (finite multi-seat, unlimited, or buy-more) can invite; a
    // solo single-seat plan cannot. The actual seat cap is enforced per-invite
    // in the invites handler, so a finite cap like Standard's must NOT gate the
    // capability off (regression: it did while the check keyed off seat_cents).
    #[test]
    fn team_plans_can_invite_solo_plans_cannot() {
        // Finite multi-seat, no overage purchase (Standard: 25 seats).
        assert!(get_self_hosted_standard_plan().can_invite_users());
        // Unlimited seats.
        assert!(get_self_hosted_plus_plan().can_invite_users());
        assert!(get_commercial_self_hosted_plan().can_invite_users());
        assert!(get_enterprise_plan().can_invite_users());
        // The self-hosted plan: uncapped seats, so inviting is always on.
        assert!(get_community_plan().can_invite_users());
        // Buy-more seats (Business).
        assert!(
            get_purchasable_plans()
                .into_iter()
                .find(|p| p.id() == "Business")
                .unwrap()
                .can_invite_users()
        );
        // Single-seat solo plan.
        assert!(!get_free_plan().can_invite_users());
    }
}
