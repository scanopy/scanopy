use crate::server::{
    auth::r#impl::oidc::OidcProviderMetadata,
    billing::types::base::{
        BillingInvoice, BillingPlan, CancelReason, LimitSource, LimitType, SaveOffer,
    },
    discovery::r#impl::types::DiscoveryType,
    organizations::r#impl::base::UseCase,
    shared::api_key_common::ApiKeyType,
};
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use stripe_billing::{CancellationDetailsFeedback, CancellationDetailsReason};
use strum::EnumIter;
use strum_macros::EnumDiscriminants;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize)]
pub enum EventLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Semantic color for an event's log label. The logging subscriber maps this
/// to an ANSI background-color badge with white text. Only the
/// create/update/delete distinction is meaningful (green/blue/red); everything
/// else — telemetry, auth, billing, lifecycle — is `Neutral`, so operation
/// types opt in via `Operation::log_color` rather than mapping every variant.
#[derive(Debug, Clone, Copy)]
pub enum LabelColor {
    Green,
    Blue,
    Red,
    Neutral,
}

impl LabelColor {
    /// ANSI SGR params (bright background `;` bright-white foreground), without
    /// the escape framing.
    pub fn ansi_code(self) -> &'static str {
        match self {
            LabelColor::Green => "42;97",
            LabelColor::Blue => "44;97",
            LabelColor::Red => "41;97",
            LabelColor::Neutral => "40;97",
        }
    }
}

/// Authentication method for user-flow auth events. API-key auth lives on
/// dedicated variants (`RotateKey`, `ApiKeyAuthFailed`) — not here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum AuthMethod {
    Password,
    Oidc(OidcProviderMetadata),
}

/// Struct used for operations where an email + token is used: email verification, password reset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailAndToken {
    pub email: EmailAddress,
    pub token: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, EnumDiscriminants,
)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(
    Hash,
    EnumIter,
    strum::Display,
    strum::AsRefStr,
    Serialize,
    Deserialize,
))]
pub enum AuthOperation {
    // User Auth
    Register {
        method: AuthMethod,
        marketing_opt_in: bool,
        // If None, user was invited or OIDC and email verification is not required
        email_and_token: Option<EmailAndToken>,
    },
    LoginSuccess {
        method: AuthMethod,
        via_register_flow: bool,
    },
    LoginFailed {
        method: AuthMethod,
        attempted_email: EmailAddress,
    },
    PasswordResetRequested {
        email_and_token: EmailAndToken,
    },
    PasswordResetCompleted,
    PasswordChanged {
        had_password: bool,
        email: EmailAddress,
        timestamp: DateTime<Utc>,
    },
    EmailVerified,
    OidcLinked {
        email: EmailAddress,
        provider: OidcProviderMetadata,
    },
    OidcUnlinked {
        email: EmailAddress,
        provider: OidcProviderMetadata,
    },
    EmailVerificationRequested {
        email_and_token: EmailAndToken,
    },
    EmailChangeRequested {
        email_and_token: EmailAndToken,
    },
    EmailChanged {
        old_email: EmailAddress,
        new_email: EmailAddress,
    },
    LoggedOut,

    // Api Key Auth
    RotateKey {
        api_key_id: Uuid,
        key_type: ApiKeyType,
    },
    ApiKeyAuthFailed {
        key_type: ApiKeyType,
        reason: String,
        key_prefix: String,
    },
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, strum::Display, EnumDiscriminants,
)]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(
    Hash,
    EnumIter,
    strum::Display,
    strum::AsRefStr,
    Serialize,
    Deserialize,
))]
pub enum EntityOperation {
    Get,
    GetAll,
    Created,
    Updated,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum::Display, EnumDiscriminants)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(
    Hash,
    EnumIter,
    strum::Display,
    strum::AsRefStr,
    Serialize,
    Deserialize,
))]
pub enum BillingOperation {
    CheckoutStarted {
        plan: BillingPlan,
        has_trial: bool,
    },
    CheckoutCompleted {
        plan: BillingPlan,
        included_networks: Option<u64>,
        included_seats: Option<u64>,
        mrr_amount_cents: i64,
        is_trialing: bool,
        /// Stripe `sub.items.data[0].current_period_end` at checkout — None
        /// for Free direct-activation (no Stripe sub).
        next_renewal_at: Option<DateTime<Utc>>,
    },
    TrialStarted {
        plan: BillingPlan,
        trial_end: DateTime<Utc>,
        trial_days: u32,
    },
    TrialWillEnd {
        plan: BillingPlan,
        has_payment_method: bool,
    },
    TrialEnded {
        plan: BillingPlan,
        converted: bool,
        /// New `sub.items.data[0].current_period_end` after the trial→paid
        /// snap. None when `converted: false` (sub is gone).
        next_renewal_at: Option<DateTime<Utc>>,
    },
    PlanChanged {
        from: BillingPlan,
        to: BillingPlan,
        is_downgrade: bool,
        /// `sub.items.data[0].current_period_end` after the change.
        next_renewal_at: Option<DateTime<Utc>>,
    },
    SubscriptionCancelled {
        plan: BillingPlan,
        reason_code: Option<CancelReason>,
        stripe_feedback: Option<CancellationDetailsFeedback>,
        stripe_reason: Option<CancellationDetailsReason>,
        internal_reason: Option<String>,
        comment: Option<String>,
        period_end: DateTime<Utc>,
        was_trialing: bool,
        mrr_amount_cents: i64,
        tenure_days: u32,
    },
    PaymentSucceeded {
        invoice: BillingInvoice,
    },
    PaymentFailed {
        invoice_id: String,
        amount_cents: i64,
        plan: BillingPlan,
        attempt_count: u32,
    },
    PaymentActionRequired {
        invoice_id: String,
        /// Stripe-hosted authorization URL (3DS/SCA). Set in the cloud
        /// invoice payload; the email CTA links here directly so the user
        /// completes authorization on Stripe's page instead of navigating
        /// our settings modal.
        hosted_invoice_url: Option<String>,
    },
    PaymentRecovered {
        invoice_id: String,
        amount_cents: i64,
        plan: BillingPlan,
        attempt_count: u32,
        /// `sub.items.data[0].current_period_end` after the recovery.
        next_renewal_at: Option<DateTime<Utc>>,
    },
    FeatureLimitHit {
        limit_type: LimitType,
        current_count: u64,
        limit: u64,
        plan: BillingPlan,
        source: LimitSource,
    },
    Paused {
        plan: BillingPlan,
        duration_days: u32,
        resumes_at: DateTime<Utc>,
    },
    Resumed {
        was_early: bool,
    },
    TrialExtended {
        days_added: u32,
        new_trial_end: DateTime<Utc>,
    },
    CancellationInitiated {
        reason_code: Option<CancelReason>,
        stripe_feedback: Option<CancellationDetailsFeedback>,
        stripe_reason: Option<CancellationDetailsReason>,
        comment: Option<String>,
        save_offer_shown: Vec<SaveOffer>,
        save_offer_redeemed: Option<SaveOffer>,
        planned_period_end: DateTime<Utc>,
    },
    /// User-provided cancellation reason/comment, captured on a follow-up
    /// Stripe webhook (Portal-with-reason flow) ~hundreds of ms after the
    /// initial `CancellationInitiated`. Separate event because Stripe persists
    /// the two pieces of state at different times and either may fire alone
    /// — no-reason Portal cancels never produce this event.
    CancellationFeedbackProvided {
        stripe_feedback: Option<CancellationDetailsFeedback>,
        stripe_reason: Option<CancellationDetailsReason>,
        comment: Option<String>,
    },
    /// User cleared a pending cancellation (via in-app reactivate). Stripe's
    /// `cancel_at` flips from `Some(period_end)` back to `None`; we emit this
    /// so the org subscriber's `implied_status` mirror restores `plan_status`
    /// and analytics subscribers can attribute the un-churn. `trialing` carries
    /// the live Stripe subscription status so a sub reactivated mid-trial
    /// returns to `trialing` rather than being mislabelled `active`.
    Reactivated {
        trialing: bool,
        /// `sub.items.data[0].current_period_end` after the cancel was cleared.
        next_renewal_at: Option<DateTime<Utc>>,
    },
    /// Save-offer discount applied — the org subscriber persists the
    /// percent + expiry so the eligibility gate (once per org) can read
    /// them, the BillingTab chip can render the live percent, and the
    /// cancel modal can drop the Discount panel on a subsequent visit.
    DiscountApplied {
        percent_off: i64,
        expires_at: DateTime<Utc>,
    },
    PaymentMethodAdded,
    PaymentMethodRemoved,
    /// Stripe customer was created for this org; the subscriber records the
    /// customer id so downstream operations can address it. Fires from
    /// `get_or_create_customer` the first time we mint a customer for the
    /// org. Telemetry-only with respect to plan_status.
    StripeCustomerCreated {
        customer_id: String,
    },
    /// A self-hosted org's plan was reconciled to the plan the deployment runs
    /// on (`plans::self_hosted_plan`). Emitted by the startup reconciliation
    /// pass, not by Stripe — it exists so an org left on some other plan (an
    /// old capped tier, say) is moved onto the current one without operator
    /// action. The org subscriber writes the new plan; email is deliberately not
    /// sent (the email subscriber allowlists discriminants and excludes this
    /// one) so a silent instance-level change doesn't spam org owners.
    /// Transient/event-only — never persisted to a `BillingOperation` DB column,
    /// so it is intentionally absent from the `DbEnumContributor` baseline
    /// (matches `StripeCustomerCreated` etc.).
    PlanReconciled {
        from: BillingPlan,
        to: BillingPlan,
    },
}

impl BillingOperation {
    /// Plan carried by the event, where the variant has one. Used by analytics
    /// subscribers (PostHog person properties, Brevo CRM sync) that need the
    /// plan name without a per-call-site exhaustive match.
    pub fn plan(&self) -> Option<&BillingPlan> {
        match self {
            Self::CheckoutStarted { plan, .. }
            | Self::CheckoutCompleted { plan, .. }
            | Self::TrialStarted { plan, .. }
            | Self::TrialWillEnd { plan, .. }
            | Self::TrialEnded { plan, .. }
            | Self::SubscriptionCancelled { plan, .. }
            | Self::FeatureLimitHit { plan, .. }
            | Self::Paused { plan, .. }
            | Self::PaymentFailed { plan, .. }
            | Self::PaymentRecovered { plan, .. } => Some(plan),
            Self::PlanChanged { to, .. } | Self::PlanReconciled { to, .. } => Some(to),
            _ => None,
        }
    }

    /// Plan the org *lands on* after this event. For the downgrade-to-Free
    /// outcomes — `SubscriptionCancelled` and an unconverted `TrialEnded` —
    /// `plan()` carries the outgoing paid plan, but the org is moved to Free
    /// (the rewrite lives in the org subscriber's matching arm). Use this for
    /// the PostHog person/group `plan_type` so a churned org isn't mislabelled
    /// with its old paid plan; the literal event payload (the PostHog
    /// `metadata` blob) still serializes the carried plan unchanged.
    pub fn resulting_plan_name(&self) -> Option<&'static str> {
        use crate::server::billing::plans::get_free_plan;
        use crate::server::shared::types::metadata::TypeMetadataProvider;
        match self {
            Self::SubscriptionCancelled { .. }
            | Self::TrialEnded {
                converted: false, ..
            } => Some(get_free_plan().name()),
            _ => self.plan().map(|p| p.name()),
        }
    }

    /// Canonical mapping from a billing event to the `PlanStatus` it implies
    /// — or `None` for telemetry-only variants that don't affect status.
    /// Single source of truth used by Brevo's plan_status sync and PostHog
    /// person properties.
    pub fn implied_status(&self) -> Option<crate::server::billing::types::base::PlanStatus> {
        use crate::server::billing::types::base::PlanStatus;
        match self {
            Self::CheckoutCompleted { .. }
            | Self::PaymentRecovered { .. }
            | Self::Resumed { .. }
            | Self::Reactivated { trialing: false, .. }
            // A full cancellation / unconverted trial downgrades the org to the
            // Free plan, which is an *active* plan. The plan rewrite to Free
            // lives in the org subscriber's matching arm (status alone can't
            // express it); the status these events imply is Active.
            | Self::SubscriptionCancelled { .. }
            | Self::TrialEnded { converted: false, .. } => Some(PlanStatus::Active),

            Self::Reactivated { trialing: true, .. }
            | Self::TrialStarted { .. }
            | Self::TrialExtended { .. } => Some(PlanStatus::Trialing),
            Self::TrialEnded {
                converted: true, ..
            } => Some(PlanStatus::Active),

            Self::PaymentFailed { .. } | Self::PaymentActionRequired { .. } => {
                Some(PlanStatus::PastDue)
            }

            Self::Paused { .. } => Some(PlanStatus::Paused),

            Self::CancellationInitiated { .. } => Some(PlanStatus::PendingCancellation),

            // Telemetry-only — no state implication.
            //
            // - `PlanChanged` describes a plan transition, not a status
            //   transition. At its only emission site (tier switch on an
            //   active sub) `plan_status` was `Active` and stays `Active`;
            //   the lifecycle event that triggered the switch (or didn't
            //   trigger one — for paid→paid tier switches there's no
            //   accompanying status change) owns the status. The chained
            //   PlanChanged-for-Brevo-sync at the cancel site that used to
            //   make this return `Active` is gone — see
            //   `process_subscription_deleted_side_effects`; Brevo now
            //   handles the Free plan_type write off `SubscriptionCancelled`
            //   directly.
            // - `PaymentSucceeded` fires on every invoice.paid webhook
            //   including the $0 trial-setup invoice Stripe creates
            //   alongside `customer.subscription.created`. Treating it as
            //   `Active` would race the `TrialStarted` write and clobber
            //   `plan_status='trialing'`. Subscription lifecycle is owned by
            //   `CheckoutCompleted` / `TrialStarted` / `TrialEnded` /
            //   `Paused` / `Resumed` / `Cancelled`, and dunning recovery by
            //   `PaymentRecovered` — which fires inside `handle_invoice_paid`
            //   BEFORE `PaymentSucceeded` for the was-past-due case, so we
            //   lose nothing.
            // `PlanReconciled` swaps the org's plan (an old capped tier →
            // CommercialSelfHosted) but implies no status transition — both are
            // billing-exempt self-hosted plans. Like `PlanChanged`, the plan
            // write is owned by the org subscriber's arm, not `plan_status`.
            Self::CheckoutStarted { .. }
            | Self::PlanChanged { .. }
            | Self::PlanReconciled { .. }
            | Self::TrialWillEnd { .. }
            | Self::FeatureLimitHit { .. }
            | Self::PaymentSucceeded { .. }
            | Self::DiscountApplied { .. }
            | Self::CancellationFeedbackProvided { .. }
            | Self::StripeCustomerCreated { .. }
            | Self::PaymentMethodAdded
            | Self::PaymentMethodRemoved => None,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    strum::Display,
    utoipa::ToSchema,
    EnumDiscriminants,
)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(
    Hash,
    EnumIter,
    strum::Display,
    strum::AsRefStr,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
    strum::IntoStaticStr,
    strum::VariantNames,
))]
pub enum OnboardingOperation {
    OrgCreated {
        org_name: String,
        plan: BillingPlan,
        use_case: UseCase,
    },
    OnboardingModalCompleted,
    PlanSelected {
        plan: BillingPlan,
    },
    DaemonPromptDismissed,
    DaemonPromptAccepted,
    FirstDaemonRegistered {
        daemon_name: String,
        network_name: String,
    },
    /// Emitted when a user views their live topology after discovery has produced
    /// at least one host. (Originally tied to the topology-rebuild lifecycle, which
    /// was removed in Phase 2 snapshots; the variant name is retained to keep legacy
    /// persisted values valid, but it now means "first topology viewed".)
    FirstTopologyRebuild,
    FirstDiscoveryCompleted {
        discovery_type: DiscoveryType,
    },
    FirstHostDiscovered,
    SecondNetworkCreated {
        network_id: Uuid,
        network_name: String,
        total_networks: u32,
    },
    FirstTagCreated,
    #[serde(alias = "FirstGroupCreated")]
    FirstDependencyCreated,
    FirstUserApiKeyCreated,
    FirstSnmpCredentialCreated,
    FirstApplicationTagCreated,
    FirstCredentialCreated,
    FirstSnapshotCreated {
        snapshot_id: Uuid,
        network_id: Uuid,
    },
    InviteSent,
    InviteAccepted,
    ProfileCompleted {
        job_title: Option<String>,
        company_size: Option<String>,
    },
    ReferralSourceCompleted {
        referral_source: crate::server::organizations::handlers::ReferralSource,
        referral_source_other: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum::Display, EnumDiscriminants)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(
    Hash,
    EnumIter,
    strum::Display,
    strum::AsRefStr,
    Serialize,
    Deserialize,
))]
pub enum AnalyticsOperation {
    TopologyShareViewed { share_id: Uuid, has_password: bool },
    TopologyEmbedViewed { share_id: Uuid, has_password: bool },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::plans::get_free_plan;

    fn round_trip(op: BillingOperation) {
        let json = serde_json::to_string(&op).expect("serialize");
        let back: BillingOperation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(op, back, "round-trip mismatch for {json}");
    }

    #[test]
    fn plan_reconciled_round_trip() {
        use crate::server::billing::plans::{get_commercial_self_hosted_plan, get_community_plan};
        round_trip(BillingOperation::PlanReconciled {
            from: get_community_plan(),
            to: get_commercial_self_hosted_plan(),
        });
    }

    #[test]
    fn plan_reconciled_plan_reports_target() {
        use crate::server::billing::plans::{get_commercial_self_hosted_plan, get_community_plan};
        use crate::server::shared::types::metadata::TypeMetadataProvider;
        let op = BillingOperation::PlanReconciled {
            from: get_community_plan(),
            to: get_commercial_self_hosted_plan(),
        };
        // PostHog labels `plan_type` off `resulting_plan_name()` → must be the
        // upgraded target, not null, so analytics don't clobber the plan.
        assert_eq!(op.plan(), Some(&get_commercial_self_hosted_plan()));
        assert_eq!(
            op.resulting_plan_name(),
            Some(get_commercial_self_hosted_plan().name())
        );
        // Plan swap, not a status transition.
        assert_eq!(op.implied_status(), None);
    }

    #[test]
    fn subscription_cancelled_round_trip_with_all_optionals_some() {
        round_trip(BillingOperation::SubscriptionCancelled {
            plan: get_free_plan(),
            reason_code: Some(crate::server::billing::types::base::CancelReason::TooExpensive),
            stripe_feedback: Some(CancellationDetailsFeedback::TooExpensive),
            stripe_reason: Some(CancellationDetailsReason::PaymentFailed),
            internal_reason: Some("admin".to_string()),
            comment: Some("not for me".to_string()),
            period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            was_trialing: true,
            mrr_amount_cents: 9900,
            tenure_days: 42,
        });
    }

    #[test]
    fn subscription_cancelled_round_trip_with_all_optionals_none() {
        round_trip(BillingOperation::SubscriptionCancelled {
            plan: get_free_plan(),
            reason_code: None,
            stripe_feedback: None,
            stripe_reason: None,
            internal_reason: None,
            comment: None,
            period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            was_trialing: false,
            mrr_amount_cents: 0,
            tenure_days: 0,
        });
    }

    #[test]
    fn checkout_completed_round_trip_paid() {
        round_trip(BillingOperation::CheckoutCompleted {
            plan: get_free_plan(),
            included_networks: Some(3),
            included_seats: Some(5),
            mrr_amount_cents: 4900,
            is_trialing: false,
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        });
    }

    #[test]
    fn checkout_completed_round_trip_trialing() {
        round_trip(BillingOperation::CheckoutCompleted {
            plan: get_free_plan(),
            included_networks: Some(3),
            included_seats: Some(5),
            mrr_amount_cents: 4900,
            is_trialing: true,
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        });
    }

    #[test]
    fn cancellation_initiated_round_trip_with_stripe_details() {
        round_trip(BillingOperation::CancellationInitiated {
            reason_code: None,
            stripe_feedback: Some(CancellationDetailsFeedback::TooExpensive),
            stripe_reason: Some(CancellationDetailsReason::CancellationRequested),
            comment: Some("not for me".to_string()),
            save_offer_shown: vec![],
            save_offer_redeemed: None,
            planned_period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
        });
    }

    #[test]
    fn cancellation_initiated_round_trip_all_none() {
        round_trip(BillingOperation::CancellationInitiated {
            reason_code: None,
            stripe_feedback: None,
            stripe_reason: None,
            comment: None,
            save_offer_shown: vec![],
            save_offer_redeemed: None,
            planned_period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
        });
    }

    #[test]
    fn cancellation_feedback_provided_round_trip() {
        round_trip(BillingOperation::CancellationFeedbackProvided {
            stripe_feedback: Some(CancellationDetailsFeedback::TooExpensive),
            stripe_reason: Some(CancellationDetailsReason::CancellationRequested),
            comment: Some("test 6/18".to_string()),
        });
    }

    #[test]
    fn payment_failed_round_trip() {
        round_trip(BillingOperation::PaymentFailed {
            invoice_id: "in_123".to_string(),
            amount_cents: 9900,
            plan: get_free_plan(),
            attempt_count: 3,
        });
    }

    #[test]
    fn payment_recovered_round_trip() {
        round_trip(BillingOperation::PaymentRecovered {
            invoice_id: "in_456".to_string(),
            amount_cents: 9900,
            plan: get_free_plan(),
            attempt_count: 2,
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        });
    }

    #[test]
    fn stripe_customer_created_round_trip() {
        round_trip(BillingOperation::StripeCustomerCreated {
            customer_id: "cus_abc123".to_string(),
        });
    }

    #[test]
    fn resulting_plan_name_maps_downgrades_to_free() {
        use crate::server::billing::plans::get_enterprise_plan;
        use crate::server::shared::types::metadata::TypeMetadataProvider;

        // Cancelling a paid plan lands the org on Free, even though the event
        // still carries the outgoing (paid) plan.
        let cancelled = BillingOperation::SubscriptionCancelled {
            plan: get_enterprise_plan(),
            reason_code: None,
            stripe_feedback: None,
            stripe_reason: None,
            internal_reason: None,
            comment: None,
            period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            was_trialing: false,
            mrr_amount_cents: 0,
            tenure_days: 10,
        };
        assert_eq!(cancelled.plan().map(|p| p.name()), Some("Enterprise"));
        assert_eq!(cancelled.resulting_plan_name(), Some("Free"));

        // An unconverted trial also lands on Free.
        let trial_lost = BillingOperation::TrialEnded {
            plan: get_enterprise_plan(),
            converted: false,
            next_renewal_at: None,
        };
        assert_eq!(trial_lost.resulting_plan_name(), Some("Free"));

        // A converted trial keeps the paid plan it carries.
        let trial_won = BillingOperation::TrialEnded {
            plan: get_enterprise_plan(),
            converted: true,
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        };
        assert_eq!(trial_won.resulting_plan_name(), Some("Enterprise"));

        // Non-downgrade events return the plan they carry.
        let checkout = BillingOperation::CheckoutCompleted {
            plan: get_enterprise_plan(),
            included_networks: None,
            included_seats: None,
            mrr_amount_cents: 4900,
            is_trialing: false,
            next_renewal_at: None,
        };
        assert_eq!(checkout.resulting_plan_name(), Some("Enterprise"));

        // Events with no plan return None.
        assert_eq!(
            BillingOperation::PaymentMethodAdded.resulting_plan_name(),
            None
        );
    }

    #[test]
    fn reactivated_round_trip() {
        round_trip(BillingOperation::Reactivated {
            trialing: false,
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        });
    }

    #[test]
    fn plan_changed_round_trip() {
        round_trip(BillingOperation::PlanChanged {
            from: get_free_plan(),
            to: get_free_plan(),
            is_downgrade: false,
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        });
    }

    #[test]
    fn trial_ended_round_trip() {
        round_trip(BillingOperation::TrialEnded {
            plan: get_free_plan(),
            converted: true,
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        });
    }
}
