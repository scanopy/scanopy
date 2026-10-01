//! Typed wrapper for the `subscription.metadata` HashMap Stripe exposes on
//! `Subscription`. Both the billing endpoints (which write keys when calling
//! `UpdateSubscription::metadata`) and the webhook handler (which reads keys
//! to recover Scanopy-only context like the cancel reason) go through this
//! struct so the contract stays in one place. Stringly-typed `metadata.get`
//! calls everywhere else are an anti-pattern; the typed instance is the
//! source of truth. `StripeOrgMetadata` plays the same role for the
//! org-only metadata on customers, SetupIntents and Checkout sessions.
//!
//! Stripe's update API preserves keys that aren't sent, so a partial
//! instance can be written without disturbing identification fields like
//! `organization_id` or `plan` that an earlier write put on the
//! subscription.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::billing::types::base::{BillingPlan, CancelReason, SaveOffer};

const KEY_ORGANIZATION_ID: &str = "organization_id";
const KEY_PLAN: &str = "plan";
const KEY_PAUSE_DURATION_DAYS: &str = "scanopy_pause_duration_days";
const KEY_PAUSED_AT: &str = "scanopy_paused_at";
const KEY_TRIAL_EXTENDED_DAYS: &str = "scanopy_trial_extended_days";
const KEY_CANCEL_REASON: &str = "scanopy_cancel_reason";
const KEY_CANCEL_SAVE_OFFER_SHOWN: &str = "scanopy_cancel_save_offer_shown";
const KEY_CANCEL_SAVE_OFFER_REDEEMED: &str = "scanopy_cancel_save_offer_redeemed";

/// All Scanopy-owned and Scanopy-shared keys that can ride on a Stripe
/// Subscription's `metadata` map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StripeSubscriptionMetadata {
    pub organization_id: Option<Uuid>,
    pub plan: Option<BillingPlan>,
    pub scanopy_pause_duration_days: Option<u32>,
    /// UTC unix timestamp (seconds) of when the pause was initiated. Used by
    /// the resume path to compute the actual elapsed paused days so the next
    /// renewal can be shifted forward by exactly that amount (early resume
    /// shouldn't push the renewal by the full requested duration).
    pub scanopy_paused_at: Option<i64>,
    pub scanopy_trial_extended_days: Option<u32>,
    pub scanopy_cancel_reason: Option<CancelReason>,
    pub scanopy_cancel_save_offer_shown: Option<Vec<SaveOffer>>,
    pub scanopy_cancel_save_offer_redeemed: Option<SaveOffer>,
}

impl StripeSubscriptionMetadata {
    /// The identification keys every Scanopy-created subscription carries.
    /// The webhooks read both to map the subscription back to its org and
    /// plan.
    pub fn identity(organization_id: Uuid, plan: BillingPlan) -> Self {
        Self {
            organization_id: Some(organization_id),
            plan: Some(plan),
            ..Default::default()
        }
    }

    /// Build the HashMap to pass to Stripe. Only `Some` fields land in the
    /// map. Stripe preserves keys that aren't sent, so a partial instance
    /// can extend an existing subscription's metadata without overwriting
    /// identification fields written by an earlier call.
    pub fn to_stripe(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        if let Some(org_id) = self.organization_id {
            out.insert(KEY_ORGANIZATION_ID.into(), org_id.to_string());
        }
        if let Some(plan) = &self.plan
            && let Ok(plan_json) = serde_json::to_string(plan)
        {
            out.insert(KEY_PLAN.into(), plan_json);
        }
        if let Some(days) = self.scanopy_pause_duration_days {
            out.insert(KEY_PAUSE_DURATION_DAYS.into(), days.to_string());
        }
        if let Some(ts) = self.scanopy_paused_at {
            out.insert(KEY_PAUSED_AT.into(), ts.to_string());
        }
        if let Some(days) = self.scanopy_trial_extended_days {
            out.insert(KEY_TRIAL_EXTENDED_DAYS.into(), days.to_string());
        }
        if let Some(reason) = self.scanopy_cancel_reason {
            out.insert(KEY_CANCEL_REASON.into(), reason.to_string());
        }
        if let Some(shown) = &self.scanopy_cancel_save_offer_shown
            && let Ok(shown_json) = serde_json::to_string(shown)
        {
            out.insert(KEY_CANCEL_SAVE_OFFER_SHOWN.into(), shown_json);
        }
        if let Some(redeemed) = self.scanopy_cancel_save_offer_redeemed {
            out.insert(KEY_CANCEL_SAVE_OFFER_REDEEMED.into(), redeemed.to_string());
        }
        out
    }

    /// Parse a Stripe metadata map into the typed view. Missing or
    /// unparseable fields land as `None`; this never panics. The webhook
    /// handler degrades gracefully on partial data — e.g., a cancel
    /// initiated via the Stripe Portal has no `scanopy_cancel_*` keys, so
    /// those fields are `None` and the handler substitutes its own
    /// defaults.
    pub fn from_stripe(m: &HashMap<String, String>) -> Self {
        Self {
            organization_id: m.get(KEY_ORGANIZATION_ID).and_then(|s| s.parse().ok()),
            plan: m.get(KEY_PLAN).and_then(|s| serde_json::from_str(s).ok()),
            scanopy_pause_duration_days: m
                .get(KEY_PAUSE_DURATION_DAYS)
                .and_then(|s| s.parse().ok()),
            scanopy_paused_at: m.get(KEY_PAUSED_AT).and_then(|s| s.parse().ok()),
            scanopy_trial_extended_days: m
                .get(KEY_TRIAL_EXTENDED_DAYS)
                .and_then(|s| s.parse().ok()),
            scanopy_cancel_reason: m.get(KEY_CANCEL_REASON).and_then(|s| parse_simple_enum(s)),
            scanopy_cancel_save_offer_shown: m
                .get(KEY_CANCEL_SAVE_OFFER_SHOWN)
                .and_then(|s| serde_json::from_str(s).ok()),
            scanopy_cancel_save_offer_redeemed: m
                .get(KEY_CANCEL_SAVE_OFFER_REDEEMED)
                .and_then(|s| parse_simple_enum(s)),
        }
    }

    /// True if any Scanopy-owned key is present. Lets the webhook detect
    /// transitions even when identification fields aren't written by this
    /// instance.
    pub fn contains_scanopy_keys(&self) -> bool {
        self.scanopy_pause_duration_days.is_some()
            || self.scanopy_paused_at.is_some()
            || self.scanopy_trial_extended_days.is_some()
            || self.scanopy_cancel_reason.is_some()
            || self.scanopy_cancel_save_offer_shown.is_some()
            || self.scanopy_cancel_save_offer_redeemed.is_some()
    }
}

/// Identity metadata for Stripe objects that only need to name their org:
/// customers, SetupIntents and Checkout sessions. `get_or_create_customer`
/// searches customers by this key, so the writer and the search query share
/// `KEY_ORGANIZATION_ID`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StripeOrgMetadata {
    pub organization_id: Option<Uuid>,
}

impl StripeOrgMetadata {
    pub fn new(organization_id: Uuid) -> Self {
        Self {
            organization_id: Some(organization_id),
        }
    }

    /// Build the HashMap to pass to Stripe. Only `Some` fields land in the map.
    pub fn to_stripe(&self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        if let Some(org_id) = self.organization_id {
            out.insert(KEY_ORGANIZATION_ID.into(), org_id.to_string());
        }
        out
    }

    /// Parse a Stripe metadata map. A missing or malformed id lands as `None`.
    pub fn from_stripe(m: &HashMap<String, String>) -> Self {
        Self {
            organization_id: m.get(KEY_ORGANIZATION_ID).and_then(|s| s.parse().ok()),
        }
    }

    /// Stripe Search Query Language clause matching objects whose metadata
    /// names `organization_id`.
    pub fn search_query(organization_id: Uuid) -> String {
        format!("metadata['{KEY_ORGANIZATION_ID}']:'{organization_id}'")
    }
}

/// Parse a snake_case stringly-encoded enum variant by round-tripping it
/// through `serde_json::from_value` against the wrapped string. We pick
/// this over `FromStr` because both `CancelReason` and `SaveOffer` already
/// derive `Deserialize` with `rename_all = "snake_case"`, and this avoids
/// requiring a `FromStr` impl on every enum the metadata might carry.
fn parse_simple_enum<T: serde::de::DeserializeOwned>(s: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::plans::get_free_plan;

    fn sample_full() -> StripeSubscriptionMetadata {
        StripeSubscriptionMetadata {
            organization_id: Some(Uuid::new_v4()),
            plan: Some(get_free_plan()),
            scanopy_pause_duration_days: Some(60),
            scanopy_paused_at: Some(1_700_000_000),
            scanopy_trial_extended_days: Some(7),
            scanopy_cancel_reason: Some(CancelReason::TooExpensive),
            scanopy_cancel_save_offer_shown: Some(vec![SaveOffer::Pause, SaveOffer::Discount]),
            scanopy_cancel_save_offer_redeemed: Some(SaveOffer::Pause),
        }
    }

    #[test]
    fn fully_populated_round_trips() {
        let original = sample_full();
        let serialized = original.to_stripe();
        let parsed = StripeSubscriptionMetadata::from_stripe(&serialized);
        assert_eq!(parsed, original);
    }

    #[test]
    fn empty_round_trips() {
        let original = StripeSubscriptionMetadata::default();
        let serialized = original.to_stripe();
        assert!(serialized.is_empty());
        let parsed = StripeSubscriptionMetadata::from_stripe(&serialized);
        assert_eq!(parsed, original);
    }

    #[test]
    fn partial_scanopy_only_round_trips() {
        let original = StripeSubscriptionMetadata {
            scanopy_cancel_reason: Some(CancelReason::Other),
            scanopy_cancel_save_offer_shown: Some(vec![]),
            ..Default::default()
        };
        let serialized = original.to_stripe();
        let parsed = StripeSubscriptionMetadata::from_stripe(&serialized);
        assert_eq!(parsed, original);
    }

    #[test]
    fn malformed_values_degrade_to_none() {
        let mut raw = HashMap::new();
        raw.insert(KEY_ORGANIZATION_ID.into(), "not-a-uuid".into());
        raw.insert(KEY_PLAN.into(), "{not json".into());
        raw.insert(KEY_PAUSE_DURATION_DAYS.into(), "thirty".into());
        raw.insert(KEY_CANCEL_REASON.into(), "bogus_reason".into());
        raw.insert(
            KEY_CANCEL_SAVE_OFFER_SHOWN.into(),
            "[\"definitely_not_an_offer\"]".into(),
        );

        let parsed = StripeSubscriptionMetadata::from_stripe(&raw);
        assert!(parsed.organization_id.is_none());
        assert!(parsed.plan.is_none());
        assert!(parsed.scanopy_pause_duration_days.is_none());
        assert!(parsed.scanopy_cancel_reason.is_none());
        assert!(parsed.scanopy_cancel_save_offer_shown.is_none());
    }

    #[test]
    fn contains_scanopy_keys_reflects_scanopy_fields_only() {
        let identification_only = StripeSubscriptionMetadata {
            organization_id: Some(Uuid::new_v4()),
            plan: Some(get_free_plan()),
            ..Default::default()
        };
        assert!(!identification_only.contains_scanopy_keys());

        let with_reason = StripeSubscriptionMetadata {
            scanopy_cancel_reason: Some(CancelReason::Other),
            ..Default::default()
        };
        assert!(with_reason.contains_scanopy_keys());
    }

    #[test]
    fn org_metadata_round_trips() {
        let original = StripeOrgMetadata::new(Uuid::new_v4());
        let parsed = StripeOrgMetadata::from_stripe(&original.to_stripe());
        assert_eq!(parsed, original);

        let empty = StripeOrgMetadata::default();
        assert!(empty.to_stripe().is_empty());
        assert_eq!(StripeOrgMetadata::from_stripe(&HashMap::new()), empty);
    }

    #[test]
    fn org_search_query_targets_the_written_key_and_value() {
        let org_id = Uuid::new_v4();
        let written = StripeOrgMetadata::new(org_id).to_stripe();
        let query = StripeOrgMetadata::search_query(org_id);
        for (key, value) in &written {
            assert!(
                query.contains(&format!("metadata['{key}']:'{value}'")),
                "search query {query} does not match written metadata {key}={value}"
            );
        }
        assert_eq!(written.len(), 1);
    }
}
