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
    license::{instances::InstanceLimits, online::InstanceUsage},
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::types::{BillingOperation, OnboardingOperationDiscriminants},
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
    pub sites: LimitNotificationLevel,
    pub seats: LimitNotificationLevel,
    /// The **highest** announced daemon-sunset ceiling (last unsupported version) this org has
    /// already been emailed about (e.g. "0.17.4"); every cutover at or below it counts as
    /// communicated. Because ceilings are totally ordered, a higher one always supersedes a lower
    /// one, so this single value ratchets monotonically even when several announced cutovers
    /// affect one org at once — the boot-time sweep emails each org at most once per ceiling and
    /// never oscillates. `None` until the first sunset email is sent.
    #[serde(default)]
    pub sunset_notified_last_unsupported: Option<Version>,
    /// The `license_paid_through` an air-gapped organization has already been
    /// warned about. The warning goes out once per licence period; a renewal
    /// moves the date, so the next period's warning sends. `None` until the
    /// first one.
    #[serde(default)]
    pub airgap_expiry_notified_through: Option<DateTime<Utc>>,
    /// The `license_checkin_at` already reported as the last check-in before a
    /// silence. Reported once per silence; the next check-in moves
    /// `license_checkin_at` off it, which re-arms the report and marks the key
    /// as resumed. `None` until the first silence.
    #[serde(default)]
    pub license_silence_reported_for: Option<DateTime<Utc>>,
    /// Whether the org's online key is reported as running on more server
    /// instances than it covers. Set when a check-in first finds it over the
    /// limit and cleared once it is back within, so the report goes out once
    /// per overrun.
    #[serde(default)]
    pub license_over_instance_limit: bool,
}

#[derive(
    Debug, Clone, Serialize, Validate, Deserialize, Default, PartialEq, Eq, Hash, ToSchema,
)]
pub struct OrganizationBase {
    /// Stripe customer ID - internal, not exposed to API
    #[serde(skip)]
    pub stripe_customer_id: Option<String>,
    /// Human-facing name for this organization.
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    /// The plan this organization is on.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only, required)]
    pub plan: BillingPlan,
    /// Current billing state of that plan.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only, required)]
    pub plan_status: Option<PlanStatus>,
    /// Progress through first-run setup.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only, required)]
    pub onboarding: Vec<OnboardingOperationDiscriminants>,
    /// Whether a payment method is on file.
    #[serde(default)]
    #[schema(read_only)]
    pub has_payment_method: bool,
    /// Whether the subscription is billed by sent invoice, against a purchase
    /// order. Such an org has no card, so this is the other half of "can this
    /// org pay?" — see [`Organization::can_pay`].
    #[serde(default)]
    #[schema(read_only)]
    pub bills_by_invoice: bool,
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
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
    pub next_renewal_at: Option<DateTime<Utc>>,
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
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
    pub license_paid_through: Option<DateTime<Utc>>,
    /// Last time a self-hosted server fetched an entitlement with this org's
    /// online license key.
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
    pub license_checkin_at: Option<DateTime<Utc>>,
    /// Version embedded in online license keys - internal, not exposed to API.
    /// Rotating the key increments it, retiring every earlier key.
    #[serde(skip)]
    pub license_key_version: i64,
    /// `iat` embedded in this org's online license key. Held so re-minting
    /// returns a byte-identical key rather than a new string each time; set on
    /// first issue and moved on rotation. Not key material.
    #[serde(skip)]
    pub license_key_issued_at: Option<DateTime<Utc>>,
    /// Which license key this org currently has issued - internal, not exposed
    /// to API. `None` reads as online. Switching retires the previous key.
    #[serde(skip)]
    pub license_key_type: Option<crate::server::license::types::LicenseKeyType>,
    /// Highest server version a check-in with this org's online key has
    /// reported - internal, not exposed to API. Several servers can share a
    /// key, so it only ever rises. `None` until a server new enough to send
    /// its version checks in.
    #[serde(skip)]
    pub license_server_version: Option<Version>,
    /// The date this org's air-gapped key stays current until, or `None` when
    /// it holds an online key or that date has passed.
    ///
    /// Computed on read from `license_key_type` and `license_paid_through`,
    /// never stored. It carries the one fact the UI needs, that the org cannot
    /// change plan yet, without exposing `license_key_type`, which stays
    /// internal. See [`Organization::air_gapped_key_current_until`].
    #[serde(default)]
    #[schema(read_only)]
    pub air_gapped_key_current_until: Option<DateTime<Utc>>,
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

    /// Whether the org has a way to pay its next invoice: a card on file, or a
    /// subscription billed by sent invoice. Payment prompts and the paths that
    /// check Stripe for a card both read this, so neither asks an
    /// invoice-billed customer for a card.
    pub fn can_pay(&self) -> bool {
        self.base.has_payment_method || self.base.bills_by_invoice
    }

    /// Whether the org's subscription has ended without a paid plan being
    /// chosen since. The org keeps the plan it lapsed from; the billing
    /// middleware, the discovery scheduler and daemon work handout all read
    /// this so a lapsed org is read-only in one consistent way.
    pub fn is_lapsed(&self) -> bool {
        self.base.plan.is_stripe_managed()
            && self.base.plan_status
                == Some(crate::server::billing::types::base::PlanStatus::Cancelled)
    }

    /// Whether the org is inside its trial at `now`. A trial with a cancellation scheduled is
    /// still a trial (Stripe keeps it `trialing`; we mirror `cancel_at` as `PendingCancellation`),
    /// so a plan switch keeps it running and still owes a card for a cloud plan.
    pub fn is_in_trial(&self, now: DateTime<Utc>) -> bool {
        use crate::server::billing::types::base::PlanStatus;
        matches!(
            self.base.plan_status,
            Some(PlanStatus::Trialing) | Some(PlanStatus::PendingCancellation)
        ) && self.base.trial_end_date.is_some_and(|end| end > now)
    }

    /// Whether the org gets no new discovery: it is lapsed, or it is a Scanopy Cloud org on a
    /// self-hosted license plan, whose app is closed beyond Settings and whose scans belong on
    /// its own server. Schedules and daemons stay configured, so landing on a live cloud plan
    /// resumes them. The discovery scheduler and daemon work handout both read this.
    pub fn discovery_paused(&self, deployment_type: crate::server::config::DeploymentType) -> bool {
        self.is_lapsed()
            || (!deployment_type.is_self_hosted() && self.base.plan.license_plan().is_some())
    }

    /// The date an air-gapped key stays current until, when the org holds one.
    ///
    /// An air-gapped key validates offline and carries its own expiry, so
    /// while this is `Some` the organization is committed to the plan it
    /// bought: it cannot switch back to an online key, and it cannot move to
    /// another license plan ([`Self::air_gapped_plan_change_blocked_until`]).
    /// Both refusals read this, so the two rules cannot drift apart.
    pub fn air_gapped_key_current_until(&self) -> Option<DateTime<Utc>> {
        let paid_through = self.base.license_paid_through?;
        (self.base.license_key_type == Some(crate::server::license::types::LicenseKeyType::Offline)
            && Utc::now() < paid_through)
            .then_some(paid_through)
    }

    /// The date a move to `target` waits for, when the org's air-gapped key forbids it. The key
    /// holds the org to its license tier, so another license plan waits for the key to run out. A
    /// cloud plan does not: the key keeps working to the expiry it carries, and the cloud plan
    /// bills from the switch with no credit for the license time (`change_plan`).
    pub fn air_gapped_plan_change_blocked_until(
        &self,
        target: crate::server::billing::types::base::BillingPlan,
    ) -> Option<DateTime<Utc>> {
        self.air_gapped_key_current_until()
            .filter(|_| target.license_plan().is_some())
    }

    /// Whether a move to `target` leaves a current air-gapped key running on the customer's
    /// server: a move to a cloud plan. The key keeps the license term it was paid for, so the
    /// move credits none of it.
    pub fn keeps_air_gapped_term_on_move(
        &self,
        target: crate::server::billing::types::base::BillingPlan,
    ) -> bool {
        self.air_gapped_key_current_until().is_some() && target.license_plan().is_none()
    }

    /// Record a successful check-in at `at` from a server reporting
    /// `server_version`, and return the events it produces, decided from the
    /// state before the write:
    ///
    /// - `LicenseCheckInsResumed` when the previous check-in is the one a
    ///   silence was reported for.
    /// - `LicenseActivated` on the first check-in of the current key. Rotating
    ///   or switching moves `license_key_issued_at`, and a retired key is
    ///   refused before it gets here, so an earlier check-in belongs to a key
    ///   that no longer works.
    /// - `LicenseServerUpgraded` when the version beats every one reported
    ///   before. Skipped on a first activation, which already carries it.
    pub fn record_license_check_in(
        &mut self,
        at: DateTime<Utc>,
        server_version: Option<Version>,
    ) -> Vec<BillingOperation> {
        let previous = self.base.license_checkin_at;
        let mut events = Vec::new();

        if let Some(previous) = previous
            && self.base.notifications.license_silence_reported_for == Some(previous)
        {
            events.push(BillingOperation::LicenseCheckInsResumed {
                silent_days: (at - previous).num_days(),
            });
        }

        let activated = match (previous, self.base.license_key_issued_at) {
            (None, _) => true,
            (Some(previous), Some(issued_at)) => previous < issued_at,
            (Some(_), None) => false,
        };
        if activated {
            events.push(BillingOperation::LicenseActivated {
                plan: Some(self.base.plan),
                server_version: server_version.clone(),
            });
        }

        if let Some(version) = server_version
            && self
                .base
                .license_server_version
                .as_ref()
                .is_none_or(|highest| version > *highest)
        {
            let from = self.base.license_server_version.replace(version.clone());
            if from.is_some() || !activated {
                events.push(BillingOperation::LicenseServerUpgraded { from, to: version });
            }
        }

        self.base.license_checkin_at = Some(at);
        events
    }

    /// The `LicenseCheckInsStopped` report for an online key that has gone
    /// [`LICENSE_SILENCE_THRESHOLD_DAYS`] without checking in, marking it
    /// reported so the daily sweep sends it once per silence. `None` when the
    /// key is still checking in, was already reported, or is not expected to
    /// check in at all: an air-gapped key, a non-license plan, a lapsed
    /// subscription, or a license past its expiry, where the server stopping
    /// is the license ending rather than an uninstall.
    pub fn report_license_silence(&mut self, now: DateTime<Utc>) -> Option<BillingOperation> {
        use crate::server::license::mint::{GRACE_PERIOD_DAYS, PAID_THROUGH_BUFFER_DAYS};
        use crate::server::license::types::LicenseKeyType;

        let last_check_in_at = self.base.license_checkin_at?;
        let paid_through = self.base.license_paid_through?;
        let licensed = self.base.plan.license_plan().is_some();
        let online = self.base.license_key_type.unwrap_or_default() == LicenseKeyType::Online;
        let expires_at =
            paid_through + chrono::Duration::days(PAID_THROUGH_BUFFER_DAYS + GRACE_PERIOD_DAYS);
        let silent =
            now - last_check_in_at >= chrono::Duration::days(LICENSE_SILENCE_THRESHOLD_DAYS);
        let reported =
            self.base.notifications.license_silence_reported_for == Some(last_check_in_at);

        if !licensed || !online || self.is_lapsed() || now >= expires_at || !silent || reported {
            return None;
        }

        self.base.notifications.license_silence_reported_for = Some(last_check_in_at);
        Some(BillingOperation::LicenseCheckInsStopped {
            last_check_in_at,
            server_version: self.base.license_server_version.clone(),
        })
    }
}

impl Organization {
    /// How many production and non-production servers the org's online key
    /// covers, from its plan.
    pub fn license_instance_limits(&self) -> InstanceLimits {
        self.base.plan.license_instance_limits()
    }

    /// Record the instance counts a check-in found, returning
    /// `LicenseInstanceLimitExceeded` when it is the first check-in of an
    /// overrun. Back within the limits re-arms the report.
    pub fn record_license_instance_usage(
        &mut self,
        usage: InstanceUsage,
    ) -> Option<BillingOperation> {
        let was_over = self.base.notifications.license_over_instance_limit;
        let over = usage.is_over();
        self.base.notifications.license_over_instance_limit = over;
        if !over || was_over {
            return None;
        }
        // Report the class that is over; over implies it has a limit.
        let (active, limit) = match usage.limit {
            Some(limit) if usage.active > limit => (usage.active, limit),
            _ => (usage.non_production_active, usage.non_production_limit?),
        };
        Some(BillingOperation::LicenseInstanceLimitExceeded { active, limit })
    }
}

/// Days an online key goes without checking in before it counts as stopped.
/// Servers check in every 6 hours, so this is about 28 missed check-ins: long
/// enough to rule out a maintenance window, and the same span as the
/// entitlement grace period.
pub const LICENSE_SILENCE_THRESHOLD_DAYS: i64 = 7;

impl Display for Organization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {:?}", self.base.name, self.id)
    }
}

impl crate::server::shared::events::log::EntityLog for Organization {}

impl ChangeTriggersTopologyStaleness<Organization> for Organization {
    fn triggers_staleness(&self, _other: Option<Organization>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::license::types::LicenseKeyType;

    fn org(key_type: Option<LicenseKeyType>, paid_through: Option<DateTime<Utc>>) -> Organization {
        let mut org = Organization::default();
        org.base.license_key_type = key_type;
        org.base.license_paid_through = paid_through;
        org
    }

    /// The middleware, the discovery scheduler and daemon work handout all
    /// read this. A plan with no Stripe lifecycle (Free, Community, Demo)
    /// never lapses whatever its status column says, and a live subscription
    /// in any other state is not lapsed.
    #[test]
    fn only_a_stripe_managed_plan_with_an_ended_subscription_is_lapsed() {
        use crate::server::billing::plans::{get_free_plan, get_self_hosted_standard_plan};
        use crate::server::billing::types::base::PlanStatus;

        let with = |plan: BillingPlan, status: Option<PlanStatus>| {
            let mut org = Organization::default();
            org.base.plan = plan;
            org.base.plan_status = status;
            org
        };

        assert!(with(get_self_hosted_standard_plan(), Some(PlanStatus::Cancelled)).is_lapsed());
        assert!(!with(get_self_hosted_standard_plan(), Some(PlanStatus::Active)).is_lapsed());
        assert!(!with(get_self_hosted_standard_plan(), Some(PlanStatus::PastDue)).is_lapsed());
        assert!(!with(get_self_hosted_standard_plan(), None).is_lapsed());
        assert!(!with(get_free_plan(), Some(PlanStatus::Cancelled)).is_lapsed());
        assert!(!Organization::default().is_lapsed());
    }

    /// `change_plan`'s card requirement reads this. A cancellation scheduled mid-trial leaves the
    /// trial running; one scheduled on a paid subscription after its trial does not.
    #[test]
    fn a_trial_with_a_scheduled_cancellation_is_still_a_trial() {
        use crate::server::billing::types::base::PlanStatus;

        let now = Utc::now();
        let with = |status: Option<PlanStatus>, trial_end: Option<DateTime<Utc>>| {
            let mut org = Organization::default();
            org.base.plan_status = status;
            org.base.trial_end_date = trial_end;
            org
        };
        let future = Some(now + chrono::Duration::days(5));
        let past = Some(now - chrono::Duration::days(5));

        assert!(with(Some(PlanStatus::Trialing), future).is_in_trial(now));
        assert!(with(Some(PlanStatus::PendingCancellation), future).is_in_trial(now));
        assert!(!with(Some(PlanStatus::PendingCancellation), past).is_in_trial(now));
        assert!(!with(Some(PlanStatus::Active), past).is_in_trial(now));
        assert!(!with(Some(PlanStatus::Cancelled), future).is_in_trial(now));
        assert!(!with(None, None).is_in_trial(now));
    }

    /// Two refusals read this: switching back to an online key, and changing
    /// plan. Both are about a key the customer is already running, so only an
    /// air-gapped key with time left on it counts.
    #[test]
    fn only_an_unexpired_air_gapped_key_is_current() {
        let future = Utc::now() + chrono::Duration::days(30);
        let past = Utc::now() - chrono::Duration::days(1);

        assert_eq!(
            org(Some(LicenseKeyType::Offline), Some(future)).air_gapped_key_current_until(),
            Some(future),
        );

        // The period they paid for is over, so both rules lift on their own.
        assert_eq!(
            org(Some(LicenseKeyType::Offline), Some(past)).air_gapped_key_current_until(),
            None
        );

        // An online key is retired by a version bump the moment anything
        // changes, so it never blocks either transition.
        assert_eq!(
            org(Some(LicenseKeyType::Online), Some(future)).air_gapped_key_current_until(),
            None
        );
        // `None` reads as online, per the column's documented default.
        assert_eq!(org(None, Some(future)).air_gapped_key_current_until(), None);

        // No paid-through date means nothing to be current until.
        assert_eq!(
            org(Some(LicenseKeyType::Offline), None).air_gapped_key_current_until(),
            None
        );
    }

    /// A current air-gapped key holds the org to its license tier, but a move to
    /// a cloud plan goes through and keeps the license term on the customer's
    /// server rather than crediting it.
    #[test]
    fn a_current_air_gapped_key_blocks_license_moves_but_not_cloud_ones() {
        use crate::server::billing::plans::{get_purchasable_plans, get_self_hosted_plus_plan};

        let until = Utc::now() + chrono::Duration::days(30);
        let mut holder = org(Some(LicenseKeyType::Offline), Some(until));
        holder.base.plan = get_self_hosted_plus_plan();

        for target in get_purchasable_plans() {
            let license_target = target.license_plan().is_some();
            assert_eq!(
                holder.air_gapped_plan_change_blocked_until(target),
                license_target.then_some(until),
                "{target:?}"
            );
            assert_eq!(
                holder.keeps_air_gapped_term_on_move(target),
                !license_target,
                "{target:?}"
            );
        }

        // An online key blocks nothing and keeps nothing.
        let mut online = org(Some(LicenseKeyType::Online), Some(until));
        online.base.plan = get_self_hosted_plus_plan();
        for target in get_purchasable_plans() {
            assert_eq!(online.air_gapped_plan_change_blocked_until(target), None);
            assert!(!online.keeps_air_gapped_term_on_move(target));
        }
    }

    /// A lapsed org and a cloud org on a license plan get no new discovery; a
    /// self-hosted server's org on its license plan scans as usual.
    #[test]
    fn discovery_pauses_for_lapsed_orgs_and_cloud_orgs_on_a_license_plan() {
        use crate::server::billing::plans::{get_purchasable_plans, get_self_hosted_standard_plan};
        use crate::server::billing::types::base::PlanStatus;
        use crate::server::config::DeploymentType;

        let on = |plan: BillingPlan, status: Option<PlanStatus>| {
            let mut org = Organization::default();
            org.base.plan = plan;
            org.base.plan_status = status;
            org
        };

        for plan in get_purchasable_plans() {
            let active = on(plan, Some(PlanStatus::Active));
            assert_eq!(
                active.discovery_paused(DeploymentType::Cloud),
                plan.license_plan().is_some(),
                "{plan:?}"
            );
        }
        let licensed = on(get_self_hosted_standard_plan(), None);
        assert!(!licensed.discovery_paused(DeploymentType::Commercial));
        assert!(!licensed.discovery_paused(DeploymentType::Community));

        let lapsed = on(get_self_hosted_standard_plan(), Some(PlanStatus::Cancelled));
        assert!(lapsed.discovery_paused(DeploymentType::Cloud));
        assert!(!Organization::default().discovery_paused(DeploymentType::Cloud));
    }

    fn at(day: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_800_000_000, 0).unwrap() + chrono::Duration::days(day)
    }

    fn names(events: &[BillingOperation]) -> Vec<String> {
        events.iter().map(|e| e.to_string()).collect()
    }

    /// An org issued a key on day 0 that has never checked in.
    fn issued() -> Organization {
        let mut org = Organization::default();
        org.base.license_key_issued_at = Some(at(0));
        org
    }

    #[test]
    fn a_key_activates_on_its_first_check_in_and_again_after_rotation() {
        let mut org = issued();
        org.base.plan = crate::server::billing::plans::get_commercial_self_hosted_plan();
        let v = Version::new(0, 17, 20);

        let first = org.record_license_check_in(at(1), Some(v.clone()));
        assert_eq!(
            first,
            vec![BillingOperation::LicenseActivated {
                plan: Some(org.base.plan),
                server_version: Some(Version::new(0, 17, 20))
            }],
            "the activation carries the version, so no separate upgrade"
        );
        assert_eq!(org.base.license_checkin_at, Some(at(1)));
        assert_eq!(org.base.license_server_version, Some(v.clone()));

        assert!(
            org.record_license_check_in(at(2), Some(v.clone()))
                .is_empty()
        );

        // Rotation moves the stamp past the last check-in.
        org.base.license_key_issued_at = Some(at(3));
        assert_eq!(
            names(&org.record_license_check_in(at(4), Some(v))),
            vec!["license_activated"]
        );
    }

    #[test]
    fn only_a_new_highest_version_counts_as_an_upgrade() {
        let mut org = issued();
        org.record_license_check_in(at(1), Some(Version::new(0, 17, 20)));

        assert_eq!(
            org.record_license_check_in(at(2), Some(Version::new(0, 17, 21))),
            vec![BillingOperation::LicenseServerUpgraded {
                from: Some(Version::new(0, 17, 20)),
                to: Version::new(0, 17, 21),
            }]
        );

        // A second server on the same key still runs the older release.
        assert!(
            org.record_license_check_in(at(3), Some(Version::new(0, 17, 20)))
                .is_empty()
        );
        assert!(
            org.record_license_check_in(at(4), Some(Version::new(0, 17, 21)))
                .is_empty()
        );
        assert!(org.record_license_check_in(at(5), None).is_empty());
        assert_eq!(
            org.base.license_server_version,
            Some(Version::new(0, 17, 21))
        );
    }

    #[test]
    fn a_server_activated_before_the_version_header_reports_its_first_version_as_an_upgrade() {
        let mut org = issued();
        org.record_license_check_in(at(1), None);

        assert_eq!(
            org.record_license_check_in(at(2), Some(Version::new(0, 17, 21))),
            vec![BillingOperation::LicenseServerUpgraded {
                from: None,
                to: Version::new(0, 17, 21),
            }]
        );
    }

    /// A self-hosted org whose online key last checked in on day 1, paid
    /// well past every day these tests look at.
    fn checking_in() -> Organization {
        use crate::server::billing::plans::get_self_hosted_standard_plan;
        let mut org = issued();
        org.base.plan = get_self_hosted_standard_plan();
        org.base.plan_status = Some(PlanStatus::Active);
        org.base.license_paid_through = Some(at(365));
        org.record_license_check_in(at(1), Some(Version::new(0, 17, 20)));
        org
    }

    #[test]
    fn a_silent_key_is_reported_once_and_resumes_on_its_next_check_in() {
        let mut org = checking_in();

        assert_eq!(
            org.report_license_silence(at(7)),
            None,
            "six days is not yet silent"
        );

        assert_eq!(
            org.report_license_silence(at(8)),
            Some(BillingOperation::LicenseCheckInsStopped {
                last_check_in_at: at(1),
                server_version: Some(Version::new(0, 17, 20)),
            })
        );
        assert_eq!(org.report_license_silence(at(9)), None, "already reported");

        assert_eq!(
            org.record_license_check_in(at(11), None),
            vec![BillingOperation::LicenseCheckInsResumed { silent_days: 10 }]
        );

        // The next silence is a new one.
        assert!(org.report_license_silence(at(18)).is_some());
    }

    #[test]
    fn a_key_not_expected_to_check_in_is_never_reported_silent() {
        use crate::server::billing::plans::get_free_plan;
        let silent_day = at(30);

        let mut air_gapped = checking_in();
        air_gapped.base.license_key_type = Some(LicenseKeyType::Offline);
        assert_eq!(air_gapped.report_license_silence(silent_day), None);

        let mut cancelled = checking_in();
        cancelled.base.plan_status = Some(PlanStatus::Cancelled);
        assert_eq!(cancelled.report_license_silence(silent_day), None);

        // The license ran out: the server stopping is the license ending.
        let mut expired = checking_in();
        expired.base.license_paid_through = Some(at(10));
        assert_eq!(expired.report_license_silence(silent_day), None);

        let mut not_licensed = checking_in();
        not_licensed.base.plan = get_free_plan();
        assert_eq!(not_licensed.report_license_silence(silent_day), None);

        let mut never_checked_in = issued();
        never_checked_in.base.plan = checking_in().base.plan;
        never_checked_in.base.license_paid_through = Some(at(365));
        assert_eq!(never_checked_in.report_license_silence(silent_day), None);
    }

    /// Every check-in reports the count, so the overrun has to be reported on
    /// the first one only, and again only after the key was back within.
    #[test]
    fn an_instance_overrun_is_reported_once_until_back_within_the_limit() {
        let over = InstanceUsage {
            active: 2,
            limit: Some(1),
            non_production_active: 0,
            non_production_limit: Some(2),
        };
        let within = InstanceUsage { active: 1, ..over };
        let mut org = Organization::default();

        assert_eq!(org.record_license_instance_usage(within), None);
        assert_eq!(
            org.record_license_instance_usage(over),
            Some(BillingOperation::LicenseInstanceLimitExceeded {
                active: 2,
                limit: 1
            })
        );
        assert_eq!(org.record_license_instance_usage(over), None);
        assert_eq!(org.record_license_instance_usage(within), None);
        assert!(org.record_license_instance_usage(over).is_some());
    }

    /// A third non-production server is over on a paid self-hosted tier and
    /// the report names that class; a custom plan has no cap, so moving to
    /// one clears the overrun at the next check-in.
    #[test]
    fn non_production_servers_past_the_limit_are_an_overrun_on_paid_tiers() {
        use crate::server::billing::plans::{
            get_commercial_self_hosted_plan, get_self_hosted_plus_plan,
        };

        let usage_on = |org: &Organization| {
            let limits = org.license_instance_limits();
            InstanceUsage {
                active: 1,
                limit: limits.production,
                non_production_active: 3,
                non_production_limit: limits.non_production,
            }
        };

        let mut org = Organization::default();
        org.base.plan = get_self_hosted_plus_plan();
        assert_eq!(
            org.record_license_instance_usage(usage_on(&org)),
            Some(BillingOperation::LicenseInstanceLimitExceeded {
                active: 3,
                limit: 2
            })
        );

        org.base.plan = get_commercial_self_hosted_plan();
        assert_eq!(org.record_license_instance_usage(usage_on(&org)), None);
        assert!(!org.base.notifications.license_over_instance_limit);
    }
}
