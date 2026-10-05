//! Typed payload enums for the cancellation flow: cancel reasons, save offers, and limit types.
use super::*;

// ===========================================================================
// Component enums for typed BillingOperation payloads
// ===========================================================================

/// Cancellation reason captured in `SubscriptionCancelled` /
/// `CancellationInitiated` events. Mirrors the values surfaced in the
/// in-app cancel flow (Phase 5).
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
    EnumIter,
    IntoStaticStr,
    VariantNames,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CancelReason {
    TooExpensive,
    MissingFeatures,
    SwitchedService,
    Unused,
    CustomerService,
    LowQuality,
    TooComplex,
    Other,
}

impl HasId for CancelReason {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for CancelReason {
    fn color(&self) -> Color {
        // Visual differentiation isn't load-bearing here; the modal renders
        // these as a list, not a chart. Use a single neutral palette so the
        // fixture has stable values.
        Color::Gray
    }

    fn icon(&self) -> Icon {
        match self {
            Self::TooExpensive => Icon::DollarSign,
            Self::MissingFeatures => Icon::Layers,
            Self::SwitchedService => Icon::ArrowRightLeft,
            Self::Unused => Icon::CircleSlash,
            Self::CustomerService => Icon::Headset,
            Self::LowQuality => Icon::Frown,
            Self::TooComplex => Icon::Puzzle,
            Self::Other => Icon::MessageCircle,
        }
    }
}

impl TypeMetadataProvider for CancelReason {
    fn name(&self) -> &'static str {
        match self {
            Self::TooExpensive => "Too expensive",
            Self::MissingFeatures => "Missing features",
            Self::SwitchedService => "Switched to another service",
            Self::Unused => "Not using it enough",
            Self::CustomerService => "Customer service",
            Self::LowQuality => "Low quality",
            Self::TooComplex => "Too complex",
            Self::Other => "Other",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        // Reason → save-offer mapping. The frontend reads this from
        // `cancel-reasons.json` to drive step 2 of the cancel modal.
        // `Discount` is included unconditionally; the UI filters it out
        // when `discount_save_offer_available` is false on the org payload.
        let save_offers: Vec<&'static str> = match self {
            Self::TooExpensive => vec![SaveOffer::Pause.id(), SaveOffer::Discount.id()],
            Self::Unused => vec![SaveOffer::Pause.id()],
            _ => vec![],
        };
        serde_json::json!({ "save_offers": save_offers })
    }
}

/// Save-offer choices presented during in-app cancellation (Phase 5).
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
    EnumIter,
    IntoStaticStr,
    VariantNames,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SaveOffer {
    Pause,
    Discount,
    Downgrade,
}

impl HasId for SaveOffer {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SaveOffer {
    fn color(&self) -> Color {
        match self {
            Self::Pause => Color::Amber,
            Self::Discount => Color::Green,
            Self::Downgrade => Color::Blue,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Pause => Icon::Pause,
            Self::Discount => Icon::BadgePercent,
            Self::Downgrade => Icon::TrendingDown,
        }
    }
}

impl TypeMetadataProvider for SaveOffer {
    fn name(&self) -> &'static str {
        match self {
            Self::Pause => "Pause subscription",
            Self::Discount => "Apply a discount",
            Self::Downgrade => "Switch to a smaller plan",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Pause => {
                "Take a break for 30, 60, or 90 days. We'll keep your data and resume billing when you return."
            }
            Self::Discount => "Stay subscribed at a lower rate for the next few months.",
            Self::Downgrade => "Move to a plan with fewer features but a lower price.",
        }
    }
}

/// Dimension hit when a `FeatureLimitHit` event fires.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Display, EnumIter, VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum LimitType {
    Sites,
    Hosts,
    Seats,
    Snapshots,
}

/// Origin of the request that triggered the limit hit. `Api` covers
/// user-initiated requests; `Discovery` covers automated discovery flows.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Display, EnumIter, VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum LimitSource {
    Api,
    Discovery,
}
