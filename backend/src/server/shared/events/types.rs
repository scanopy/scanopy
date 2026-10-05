use crate::server::{
    auth::r#impl::{base::PendingSiteSetup, oidc::OidcProviderMetadata},
    billing::types::base::{
        BillingInvoice, BillingPlan, BillingReason, CancelReason, LimitSource, LimitType, SaveOffer,
    },
    credentials::r#impl::types::serialize_secret_value,
    discovery::r#impl::types::DiscoveryType,
    license::types::LicenseKeyType,
    organizations::r#impl::base::UseCase,
    shared::api_key_common::ApiKeyType,
};
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use secrecy::{ExposeSecret, SecretString};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAndToken {
    pub email: EmailAddress,
    /// Anyone holding the token can verify the address or reset the password, and events are
    /// logged as JSON, so it serializes as the redacted sentinel. The email subscriber reads it
    /// in memory with `expose_secret`.
    #[serde(serialize_with = "serialize_secret_value")]
    pub token: SecretString,
}

/// `SecretString` has no `PartialEq`, so compare the exposed tokens explicitly.
impl PartialEq for EmailAndToken {
    fn eq(&self, other: &Self) -> bool {
        self.email == other.email && self.token.expose_secret() == other.token.expose_secret()
    }
}

impl Eq for EmailAndToken {}

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
        included_sites: Option<u64>,
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
    /// A trial converted to a paid subscription. A trial that ends without
    /// converting cancels the subscription, so it arrives as
    /// `SubscriptionCancelled { was_trialing: true }` instead.
    TrialEnded {
        plan: BillingPlan,
        /// New `sub.items.data[0].current_period_end` after the trial→paid
        /// snap.
        next_renewal_at: Option<DateTime<Utc>>,
    },
    PlanChanged {
        from: BillingPlan,
        to: BillingPlan,
        is_downgrade: bool,
        /// `sub.items.data[0].current_period_end` after the change.
        next_renewal_at: Option<DateTime<Utc>>,
        /// The license key type the org holds, captured at the publish site.
        /// A change between self-hosted plans reaches an online key on its
        /// own, and an air-gapped key has to be copied again, so the email
        /// differs. `None` for an org that never issued a key, and on events
        /// recorded before this field existed.
        license_key_type: Option<LicenseKeyType>,
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
        /// The license key type the org holds, captured at the publish site.
        /// An online key dies at the server's next check-in and an air-gapped
        /// key runs to its embedded expiry, so the email differs. `None` for
        /// an org that never issued a key, and on events recorded before this
        /// field existed.
        license_key_type: Option<LicenseKeyType>,
        /// The subscription ended for non-payment and we wrote off the unpaid
        /// invoice, so the licence stopped with it rather than running to the
        /// end of a period the customer paid for.
        ///
        /// Carried rather than derived because the write-off's claw-back
        /// reaches the org row through a Stripe round trip, arriving after
        /// this event. A subscriber that read `license_paid_through` would see
        /// the date as it stood before the write-off and promise the customer
        /// time we had just taken away. False on rows written before the
        /// write-off existed, all of which were ordinary endings.
        #[serde(default)]
        defaulted: bool,
    },
    PaymentSucceeded {
        invoice: BillingInvoice,
        /// The org's `license_paid_through` as it stood before this payment
        /// advanced it, captured at the publish site.
        ///
        /// The air-gapped renewal email needs it to name the expiry baked
        /// into the key already installed on the customer's server. Reading
        /// the org row instead races the organization subscriber, which
        /// advances that column off this same event, and subscriber dispatch
        /// order is unspecified. `None` when the org had no paid-through.
        previous_license_paid_through: Option<DateTime<Utc>>,
    },
    /// Stripe finalized an invoice sent to the customer for payment by its due
    /// date (`invoice.finalized`, `send_invoice` only). The org subscriber
    /// licenses the self-hosted servers until the due date plus grace, so a
    /// buyer paying by invoice can deploy before the money arrives.
    InvoiceIssued {
        invoice: BillingInvoice,
    },
    /// A sent invoice will never be paid: voided, or marked uncollectible.
    /// The org subscriber takes back the license period it granted.
    InvoiceVoided {
        invoice: BillingInvoice,
    },
    /// An organization that lapsed for non-payment settled the invoice we had
    /// written off, and is back on its plan with a replacement subscription.
    ///
    /// Carries the resumed date rather than leaving it to `PaymentSucceeded`,
    /// which fires first on the same organization and restores the invoice's
    /// own term end. That is the wrong date for anyone settling late: the
    /// service they are owed starts when they pay, not when the invoice said.
    LicenseResumed {
        plan: BillingPlan,
        resumed_through: DateTime<Utc>,
        next_renewal_at: Option<DateTime<Utc>>,
    },
    /// Stripe could not finalize a draft licence invoice
    /// (`invoice.finalization_failed`), so it was never issued or emailed and
    /// no licence period was granted. Carries Stripe's reason, usually a
    /// rejected tax ID, for the email that asks the owner to check the
    /// billing details. Implies no status: the org keeps whatever it had.
    InvoiceFinalizationFailed {
        invoice: BillingInvoice,
        reason: String,
    },
    /// A sent invoice passed its due date unpaid (`invoice.overdue`). Stripe
    /// makes no charge attempt on one, so `PaymentFailed` never fires and this
    /// is the only signal that an invoice buyer has stopped paying. The
    /// license keeps its grace period; the status is what changes.
    InvoiceOverdue {
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
        /// The plan being cancelled. Subscribers segment on it: a self-hosted
        /// plan's cancellation is about a license key, not cloud access.
        /// `None` on events recorded before this field existed.
        plan: Option<BillingPlan>,
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
    /// A self-hosted org's plan was reconciled to the tier its license entitles,
    /// in either direction. Emitted at startup and each time an online key's
    /// entitlement is swapped in, never by Stripe. The org subscriber
    /// writes the new plan; email is deliberately not sent (the email subscriber
    /// allowlists discriminants and excludes this one) so a silent instance-level
    /// upgrade doesn't spam org owners. Transient/event-only — never persisted to
    /// a `BillingOperation` DB column, so it is intentionally absent from the
    /// `DbEnumContributor` baseline (matches `StripeCustomerCreated` etc.).
    LicenseReconciled {
        from: BillingPlan,
        to: BillingPlan,
    },
    /// The org's first license key was issued, on the owner's first visit to
    /// the license tab. Fires once per org: rotations and type switches are
    /// tracked by the frontend. The first key is always online, since an org
    /// reaches air-gapped only by switching from it. Telemetry-only, never
    /// persisted.
    LicenseKeyIssued,
    /// The first successful entitlement check-in for the org's current online
    /// key: a customer server is running it. Fires once per key version, so a
    /// rotated key activates again when its replacement checks in.
    /// `server_version` is `None` for servers older than the version header.
    LicenseActivated {
        server_version: Option<String>,
    },
    /// A check-in reported a server version higher than any reported before
    /// for this org. Several servers can share one key, so only a new highest
    /// version counts; a mixed-version fleet never repeats the event.
    LicenseServerUpgraded {
        from: Option<String>,
        to: String,
    },
    /// An online key that used to check in has been silent for
    /// [`LICENSE_SILENCE_THRESHOLD_DAYS`](crate::server::organizations::r#impl::base::LICENSE_SILENCE_THRESHOLD_DAYS):
    /// the server is down, offline, or Scanopy was uninstalled. Sent once per
    /// silence by the daily sweep.
    LicenseCheckInsStopped {
        last_check_in_at: DateTime<Utc>,
        server_version: Option<String>,
    },
    /// A key reported by `LicenseCheckInsStopped` checked in again.
    LicenseCheckInsResumed {
        silent_days: i64,
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
            Self::PlanChanged { to, .. } | Self::LicenseReconciled { to, .. } => Some(to),
            Self::CancellationInitiated { plan, .. } => plan.as_ref(),
            _ => None,
        }
    }

    /// A receipt or "your plan is active" notice for a customer paying
    /// nothing: a cloud renewal invoice that charged nothing, or a
    /// non-trial cloud checkout or trial conversion onto a plan priced at
    /// zero. The email, PostHog and Brevo subscribers skip these; every other
    /// subscriber still sees them.
    pub fn is_zero_dollar_notice(&self) -> bool {
        match self {
            Self::PaymentSucceeded { invoice, .. } => {
                invoice.billing_reason == BillingReason::SubscriptionCycle
                    && invoice.license_paid_through().is_none()
                    && invoice.charged_nothing()
            }
            Self::CheckoutCompleted {
                plan, is_trialing, ..
            } => plan.license_plan().is_none() && !is_trialing && plan.is_priced_at_zero(),
            Self::TrialEnded { plan, .. } => plan.is_priced_at_zero(),
            _ => false,
        }
    }

    /// Plan the org *lands on* after this event, for the PostHog person/group
    /// `plan_type`. A `SubscriptionCancelled` leaves the org on the plan it carries (lapsed, read-only), so this is
    /// `plan()` for every variant; it stays a separate accessor so the
    /// analytics call site names the intent.
    pub fn resulting_plan_name(&self) -> Option<&'static str> {
        use crate::server::shared::types::metadata::TypeMetadataProvider;
        self.plan().map(|p| p.name())
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
            | Self::LicenseResumed { .. }
            | Self::Reactivated {
                trialing: false, ..
            } => Some(PlanStatus::Active),

            // A full cancellation (trialing or not) leaves the org on the
            // plan it carries, lapsed: read-only until it chooses a paid plan.
            // The org subscriber's matching arm clears the subscription
            // mirrors (renewal date, invoice billing, discount) off the same
            // event; the status is what makes it read-only.
            Self::SubscriptionCancelled { .. } => Some(PlanStatus::Cancelled),

            Self::Reactivated { trialing: true, .. }
            | Self::TrialStarted { .. }
            | Self::TrialExtended { .. } => Some(PlanStatus::Trialing),
            Self::TrialEnded { .. } => Some(PlanStatus::Active),

            Self::PaymentFailed { .. }
            | Self::PaymentActionRequired { .. }
            | Self::InvoiceOverdue { .. } => Some(PlanStatus::PastDue),

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
            // `LicenseReconciled` swaps the org's plan (Community →
            // CommercialSelfHosted) but implies no status transition — both are
            // billing-exempt self-hosted plans. Like `PlanChanged`, the plan
            // write is owned by the org subscriber's arm, not `plan_status`.
            // The license key and check-in variants describe a customer
            // server, not the subscription.
            Self::CheckoutStarted { .. }
            | Self::PlanChanged { .. }
            | Self::LicenseReconciled { .. }
            | Self::LicenseKeyIssued
            | Self::LicenseActivated { .. }
            | Self::LicenseServerUpgraded { .. }
            | Self::LicenseCheckInsStopped { .. }
            | Self::LicenseCheckInsResumed { .. }
            | Self::TrialWillEnd { .. }
            | Self::FeatureLimitHit { .. }
            | Self::PaymentSucceeded { .. }
            | Self::InvoiceIssued { .. }
            | Self::InvoiceVoided { .. }
            | Self::InvoiceFinalizationFailed { .. }
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
        /// The first site requested at signup. `None` for a self-hosted
        /// license buyer and on events recorded before this field existed.
        #[serde(default)]
        site: Option<PendingSiteSetup>,
    },
    OnboardingModalCompleted,
    PlanSelected {
        plan: BillingPlan,
    },
    DaemonPromptDismissed,
    DaemonPromptAccepted,
    FirstDaemonRegistered {
        daemon_name: String,
        site_name: String,
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
    SecondSiteCreated {
        site_id: Uuid,
        site_name: String,
        total_sites: u32,
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
        site_id: Uuid,
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
    TopologyShareViewed {
        share_id: Uuid,
        has_password: bool,
    },
    TopologyEmbedViewed {
        share_id: Uuid,
        has_password: bool,
    },
    /// An email left for a user with an account. `utm_campaign` and
    /// `utm_medium` are the values the email's links carry, so a send joins
    /// the landing pageview a click produces.
    EmailSent {
        utm_campaign: String,
        utm_medium: String,
        user_id: Uuid,
    },
}

impl AnalyticsOperation {
    /// The PostHog person an event belongs to. An email goes to one user, so
    /// its send lands on the same person as their click; a share view has no
    /// viewer identity and is attributed to the org.
    pub fn distinct_id(&self, organization_id: Uuid) -> String {
        match self {
            Self::EmailSent { user_id, .. } => user_id.to_string(),
            Self::TopologyShareViewed { .. } | Self::TopologyEmbedViewed { .. } => {
                format!("org:{organization_id}")
            }
        }
    }
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
    fn license_reconciled_round_trip() {
        use crate::server::billing::plans::{get_commercial_self_hosted_plan, get_community_plan};
        round_trip(BillingOperation::LicenseReconciled {
            from: get_community_plan(),
            to: get_commercial_self_hosted_plan(),
        });
    }

    #[test]
    fn license_reconciled_plan_reports_target() {
        use crate::server::billing::plans::{get_commercial_self_hosted_plan, get_community_plan};
        use crate::server::shared::types::metadata::TypeMetadataProvider;
        let op = BillingOperation::LicenseReconciled {
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
            license_key_type: Some(LicenseKeyType::Offline),
            defaulted: false,
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
            license_key_type: None,
            defaulted: false,
        });
    }

    #[test]
    fn checkout_completed_round_trip_paid() {
        round_trip(BillingOperation::CheckoutCompleted {
            plan: get_free_plan(),
            included_sites: Some(3),
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
            included_sites: Some(3),
            included_seats: Some(5),
            mrr_amount_cents: 4900,
            is_trialing: true,
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        });
    }

    #[test]
    fn cancellation_initiated_round_trip_with_stripe_details() {
        round_trip(BillingOperation::CancellationInitiated {
            plan: Some(get_free_plan()),
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
            plan: None,
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
    fn an_ended_subscription_lapses_on_the_plan_it_carries() {
        use crate::server::billing::plans::get_enterprise_plan;
        use crate::server::billing::types::base::PlanStatus;

        // Cancelling a paid plan leaves the org on that plan, lapsed: the
        // analytics plan label stays, and the implied status is what makes
        // the org read-only.
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
            license_key_type: None,
            defaulted: false,
        };
        assert_eq!(cancelled.resulting_plan_name(), Some("Enterprise"));
        assert_eq!(cancelled.implied_status(), Some(PlanStatus::Cancelled));

        // A converted trial keeps the paid plan it carries, live.
        let trial_won = BillingOperation::TrialEnded {
            plan: get_enterprise_plan(),
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        };
        assert_eq!(trial_won.resulting_plan_name(), Some("Enterprise"));
        assert_eq!(trial_won.implied_status(), Some(PlanStatus::Active));

        // Non-downgrade events return the plan they carry.
        let checkout = BillingOperation::CheckoutCompleted {
            plan: get_enterprise_plan(),
            included_sites: None,
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
            license_key_type: Some(LicenseKeyType::Online),
        });
    }

    /// Ledger rows written before the segmentation fields existed carry no
    /// such keys, and still have to read back.
    #[test]
    fn events_recorded_before_the_segmentation_fields_still_deserialize() {
        for op in [
            BillingOperation::PlanChanged {
                from: get_free_plan(),
                to: get_free_plan(),
                is_downgrade: false,
                next_renewal_at: None,
                license_key_type: None,
            },
            BillingOperation::CancellationInitiated {
                plan: None,
                reason_code: None,
                stripe_feedback: None,
                stripe_reason: None,
                comment: None,
                save_offer_shown: vec![],
                save_offer_redeemed: None,
                planned_period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            },
        ] {
            let mut json = serde_json::to_value(&op).expect("serialize");
            strip_key(&mut json, "license_key_type");
            strip_key(&mut json, "plan");
            let back: BillingOperation = serde_json::from_value(json).expect("deserialize");
            assert_eq!(op, back);
        }
    }

    fn strip_key(value: &mut serde_json::Value, key: &str) {
        if let serde_json::Value::Object(map) = value {
            map.remove(key);
            for child in map.values_mut() {
                strip_key(child, key);
            }
        }
    }

    #[test]
    fn trial_ended_round_trip() {
        round_trip(BillingOperation::TrialEnded {
            plan: get_free_plan(),
            next_renewal_at: DateTime::<Utc>::from_timestamp(1_800_000_000, 0),
        });
    }

    /// Events are logged as JSON via `Display`. A reset or verification token in that line
    /// lets anyone who reads the logs take over the account, so it must render redacted.
    #[test]
    fn logged_auth_events_do_not_contain_the_token() {
        use crate::server::auth::middleware::auth::AuthenticatedEntity;
        use crate::server::shared::events::traits::{AuthScope, Event};

        let token = "reset-token-3f9a1c";
        let event = Event::new(
            AuthScope {
                user_id: None,
                organization_id: None,
                ip_address: "127.0.0.1".parse().unwrap(),
                user_agent: None,
            },
            AuthOperation::PasswordResetRequested {
                email_and_token: EmailAndToken {
                    email: "owner@example.test".parse().unwrap(),
                    token: token.to_string().into(),
                },
            },
            AuthenticatedEntity::Anonymous,
        );

        let line = event.to_string();
        assert!(!line.contains(token), "{line}");
        assert!(line.contains("owner@example.test"), "{line}");
        assert!(!format!("{event:?}").contains(token));
    }
}
