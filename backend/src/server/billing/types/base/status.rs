//! Derived subscription status enum and its metadata.
use super::*;

/// Derived subscription status — our domain enum, never Stripe's raw status.
/// Stripe webhook events map to typed `BillingOperation` variants at reception
/// (in `billing/service.rs`); each variant deterministically implies a
/// `PlanStatus` for downstream feature gates via
/// `BillingOperation::implied_status`.
///
/// `FromStr` is derived (via strum) so the storage layer can round-trip a
/// snake_case `text` column back into the typed value; `ToSchema` exposes
/// the enum as a stricter string union in the generated OpenAPI schema so
/// the frontend's `org.plan_status === 'paused'` comparisons are
/// compile-checked against the canonical variant list.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Display,
    strum::EnumString,
    EnumIter,
    IntoStaticStr,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PlanStatus {
    Active,
    Trialing,
    PastDue,
    Paused,
    PendingCancellation,
    /// The subscription has ended (an unpaid trial ran out, or a scheduled
    /// cancellation reached its period end) and the org has not chosen a
    /// paid plan since. The org keeps its plan; the billing middleware makes
    /// the app read-only until it does.
    ///
    /// `canceled` (American) is the legacy spelling Stripe's
    /// `SubscriptionStatus` serializes with, and pre-Phase-5 writers
    /// echoed that value straight into `organizations.plan_status`. We
    /// canonicalize on `cancelled` (British, matching the variant's
    /// `serialize_all = "snake_case"` default) for new writes, but accept
    /// the American spelling on read for any rows still carrying it.
    #[serde(alias = "canceled")]
    #[strum(serialize = "cancelled", serialize = "canceled")]
    Cancelled,
}

impl HasId for PlanStatus {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for PlanStatus {
    fn color(&self) -> Color {
        // Mirrors the former `getPlanStatusColor` mapping in BillingTab.svelte
        // so the badge text colour is unchanged by the move to metadata.
        match self {
            Self::Active => Color::Green,
            Self::Trialing => Color::Blue,
            Self::PastDue => Color::Red,
            Self::PendingCancellation => Color::Amber,
            Self::Paused => Color::Orange,
            Self::Cancelled => Color::Yellow,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Active => Icon::CircleCheck,
            Self::Trialing => Icon::Clock,
            Self::PastDue => Icon::CircleAlert,
            Self::Paused => Icon::Pause,
            Self::PendingCancellation => Icon::TriangleAlert,
            Self::Cancelled => Icon::CircleX,
        }
    }
}

impl TypeMetadataProvider for PlanStatus {
    fn name(&self) -> &'static str {
        // `Cancelled` reads as "Lapsed": it covers a trial that ran out as
        // well as a cancellation that reached its period end, and the org
        // keeps its plan either way.
        match self {
            Self::Active => "Active",
            Self::Trialing => "Trialing",
            Self::PastDue => "Past due",
            Self::Paused => "Paused",
            Self::PendingCancellation => "Cancelling",
            Self::Cancelled => "Lapsed",
        }
    }
}
