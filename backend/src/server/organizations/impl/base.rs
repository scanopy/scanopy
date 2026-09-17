use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Display;
use strum::{Display, IntoStaticStr};
use strum_macros::EnumIter;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    billing::types::base::{BillingPlan, PlanStatus},
    shared::{
        entities::ChangeTriggersTopologyStaleness, events::types::OnboardingOperationDiscriminants,
    },
};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Default,
    EnumIter,
    ToSchema,
    IntoStaticStr,
    Display,
)]
#[serde(rename_all = "lowercase")]
pub enum UseCase {
    Homelab,
    #[serde(rename = "internal_it", alias = "company")]
    InternalIt,
    Msp,
    #[default]
    Other,
}

/// Deserialize UseCase from an Option<String>, mapping null to UseCase::Other.
fn deserialize_use_case_from_option<'de, D>(deserializer: D) -> Result<UseCase, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt.as_deref() {
        Some("homelab") => Ok(UseCase::Homelab),
        Some("internal_it") | Some("company") => Ok(UseCase::InternalIt),
        Some("msp") => Ok(UseCase::Msp),
        Some("other") => Ok(UseCase::Other),
        _ => Ok(UseCase::Other),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub enum LimitNotificationLevel {
    #[default]
    None,
    Approaching,
    Reached,
}

/// Per-organization notification bookkeeping stored in the `notifications`
/// JSONB column. Started as plan-limit ratchets; now also carries the daemon
/// sunset ratchet, hence the generalized name. Internal (never on the API).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct OrgNotifications {
    pub hosts: LimitNotificationLevel,
    pub networks: LimitNotificationLevel,
    pub seats: LimitNotificationLevel,
    /// The **highest** announced daemon-sunset floor this org has already been
    /// emailed about (e.g. "0.17.5"); every cutover at or below it counts as
    /// communicated. Because floors are totally ordered, a higher floor always
    /// supersedes a lower one, so this single value ratchets monotonically even
    /// when several announced cutovers affect one org at once — the boot-time
    /// sweep emails each org at most once per floor and never oscillates.
    /// `None` until the first sunset email is sent.
    #[serde(default)]
    pub sunset_notified_floor: Option<Version>,
}

#[derive(
    Debug, Clone, Serialize, Validate, Deserialize, Default, PartialEq, Eq, Hash, ToSchema,
)]
pub struct OrganizationBase {
    /// Stripe customer ID - internal, not exposed to API
    #[serde(default, skip_serializing)]
    pub stripe_customer_id: Option<String>,
    /// Human-facing name for this organization.
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    /// The plan this organization is on.
    #[serde(default)]
    #[schema(read_only, required)]
    pub plan: Option<BillingPlan>,
    /// Current billing state of that plan.
    #[serde(default)]
    #[schema(read_only, required)]
    pub plan_status: Option<PlanStatus>,
    /// Progress through first-run setup.
    #[schema(read_only, required)]
    pub onboarding: Vec<OnboardingOperationDiscriminants>,
    /// Whether the org has a way to pay: a payment method on file, or its
    /// subscription is billed by sent invoice.
    #[serde(default)]
    #[schema(read_only)]
    pub has_payment_method: bool,
    /// When the free trial ends, if one is running.
    #[serde(default)]
    #[schema(read_only)]
    pub trial_end_date: Option<DateTime<Utc>>,
    /// Most recent `Paused` billing event's timestamp; powers the 6-month
    /// rolling pause cooldown.
    #[serde(default)]
    #[schema(read_only)]
    pub last_paused_at: Option<DateTime<Utc>>,
    /// Whether the org has used its one-time trial-extend perk.
    #[serde(default)]
    #[schema(read_only)]
    pub trial_extended_used: bool,
    /// Most recent downgrade event timestamp (paid→cheaper, or paid→cancelled);
    /// powers the 14-day downgrade banner.
    #[serde(default)]
    #[schema(read_only)]
    pub last_downgrade_at: Option<DateTime<Utc>>,
    /// Plan downgraded from at `last_downgrade_at`; pairs with the timestamp
    /// so the banner can render "you downgraded from Pro".
    #[serde(default)]
    #[schema(read_only)]
    pub last_downgrade_from_plan: Option<BillingPlan>,
    /// Most recent save-offer-discount application. NULL = never. Drives the
    /// once-per-org eligibility check in `apply_discount_save_offer` and
    /// hides the Discount panel on the cancel modal for any return visit.
    #[serde(default)]
    #[schema(read_only)]
    pub last_discount_at: Option<DateTime<Utc>>,
    /// Percent off the currently-active save-offer discount applies. Read
    /// live by the BillingTab chip so a future coupon swap renders the new
    /// value without a code change.
    #[serde(default)]
    #[schema(read_only)]
    pub discount_save_offer_percent_off: Option<i64>,
    /// When the currently-active save-offer discount window expires. The
    /// BillingTab chip renders only while `> now()`; expiry needs no
    /// cleanup job.
    #[serde(default)]
    #[schema(read_only)]
    pub discount_save_offer_active_until: Option<DateTime<Utc>>,
    /// Stripe `subscription.items.data[0].current_period_end`, mirrored on
    /// every billing event that re-anchors the period (checkout, trial start
    /// / end, plan change, renewal, pause/resume, reactivate). Cleared by
    /// SubscriptionCancelled. Powers the "Next renewal on …" line in
    /// BillingPlanModal; the UI interprets the value based on plan_status
    /// (hide for paused/cancelled/past_due where the stored value can be
    /// stale or meaningless).
    #[serde(default)]
    #[schema(read_only)]
    pub next_renewal_at: Option<DateTime<Utc>>,
    /// Brevo company ID - internal, not exposed to API
    #[serde(default, skip_serializing)]
    pub brevo_company_id: Option<String>,
    /// Latest entitlement an online license key fetched from Scanopy Cloud.
    /// Instance-level: every org row holds the same value. It works as an
    /// offline key until it expires, so it is never read from or written to
    /// the API.
    #[serde(skip)]
    pub license_entitlement: Option<String>,
    /// When Scanopy Cloud last answered this instance's license check-in.
    #[serde(skip)]
    pub license_entitlement_at: Option<DateTime<Utc>>,
    /// Per-org notification bookkeeping (plan-limit ratchets + daemon sunset).
    #[serde(default, skip_serializing)]
    pub notifications: OrgNotifications,
    /// Use case selection (homelab, company, msp, other)
    #[serde(default, deserialize_with = "deserialize_use_case_from_option")]
    pub use_case: UseCase,
    /// When the org's self-hosted license is paid through: the trial end
    /// during a self-hosted trial, then the end of the last paid invoice's
    /// service period. While a sent invoice is unpaid, its due date plus a
    /// grace window. License keys and entitlements expire 7 days later.
    #[serde(default)]
    #[schema(read_only)]
    pub license_paid_through: Option<DateTime<Utc>>,
    /// Last time a self-hosted server fetched an entitlement with this org's
    /// online license key.
    #[serde(default)]
    #[schema(read_only)]
    pub license_checkin_at: Option<DateTime<Utc>>,
    /// Version embedded in online license keys - internal, not exposed to API.
    /// Rotating the key increments it, retiring every earlier key.
    #[serde(default, skip_serializing)]
    pub license_key_version: i64,
    /// `iat` embedded in this org's online license key. Held so re-minting
    /// returns a byte-identical key rather than a new string each time; set on
    /// first issue and moved on rotation. Not key material.
    #[serde(default, skip_serializing)]
    pub license_key_issued_at: Option<DateTime<Utc>>,
    /// Which license key this org currently has issued - internal, not exposed
    /// to API. `None` reads as online. Switching retires the previous key.
    #[serde(default, skip_serializing)]
    pub license_key_type: Option<crate::server::license::types::LicenseKeyType>,
}

#[derive(
    Debug, Clone, Validate, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema,
)]
pub struct Organization {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: OrganizationBase,
}

impl Organization {
    pub fn not_onboarded(&self, step: &OnboardingOperationDiscriminants) -> bool {
        !self.base.onboarding.contains(step)
    }

    pub fn has_onboarded(&self, step: &OnboardingOperationDiscriminants) -> bool {
        self.base.onboarding.contains(step)
    }
}

impl Display for Organization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<Organization> for Organization {
    fn triggers_staleness(&self, _other: Option<Organization>) -> bool {
        false
    }
}
