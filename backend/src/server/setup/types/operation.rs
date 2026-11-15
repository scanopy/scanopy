//! What happened to an item on the Setup list, and the fields its events carry.

use super::*;

/// The answers a question item collected. Empty when the user skipped the question.
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
#[serde(tag = "type")]
pub enum SetupAnswers {
    Profile {
        job_title: Option<String>,
        company_size: Option<String>,
    },
    ReferralSource {
        referral_source: ReferralSource,
        referral_source_other: Option<String>,
    },
}

/// What happened to an item on an org's Setup list.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, strum::Display, EnumDiscriminants)]
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
pub enum SetupOperation {
    /// A recorded item finished. Stored in `setup`.
    Completed {
        item_type: SetupItemDiscriminants,
        subject_id: Option<Uuid>,
    },
    /// A question item was answered (or skipped, with empty answers). Stored in `setup`.
    Answered {
        item_type: SetupItemDiscriminants,
        answers: SetupAnswers,
    },
    /// The sweep saw an open item clear.
    Resolved {
        key: SetupItemKey,
        tier: SetupItemTier,
        hours_open: u32,
    },
    /// The sweep saw an item stay open 24 h on a trialing org.
    Stuck {
        key: SetupItemKey,
        tier: SetupItemTier,
        plan: BillingPlan,
        trial_end: DateTime<Utc>,
        hours_open: u32,
    },
}

impl SetupOperation {
    /// The item type this operation stores in `organizations.setup`. `None` for the sweep's
    /// events, which store nothing there.
    pub fn recorded_item_type(&self) -> Option<SetupItemDiscriminants> {
        match self {
            Self::Completed { item_type, .. } | Self::Answered { item_type, .. } => {
                Some(*item_type)
            }
            Self::Resolved { .. } | Self::Stuck { .. } => None,
        }
    }

    /// The fields consumers outside the server receive. A sweep event spreads its key into
    /// single-level fields, so a PostHog insight breaks down on `metadata.item_type` with no
    /// nested lookup.
    pub fn metadata(&self) -> SetupOperationMetadata<'_> {
        let (key, tier, plan, trial_end, hours_open) = match self {
            Self::Completed { .. } | Self::Answered { .. } => {
                return SetupOperationMetadata::Recorded(self);
            }
            Self::Resolved {
                key,
                tier,
                hours_open,
            } => (key, tier, None, None, hours_open),
            Self::Stuck {
                key,
                tier,
                plan,
                trial_end,
                hours_open,
            } => (key, tier, Some(plan), Some(*trial_end), hours_open),
        };
        let (entity_id, daemon_id, warning_code, reason) = match key {
            SetupItemKey::Org { .. } => (None, None, None, None),
            SetupItemKey::Entity { entity_id, .. } => (Some(*entity_id), None, None, None),
            SetupItemKey::RunWarning { daemon_id, code } => {
                (None, Some(*daemon_id), Some(*code), None)
            }
            SetupItemKey::FailedScans { daemon_id, reason } => {
                (None, Some(*daemon_id), None, Some(*reason))
            }
        };
        SetupOperationMetadata::Swept(SweptItemMetadata {
            operation: self.into(),
            item_type: key.item_type(),
            entity_id,
            daemon_id,
            warning_code,
            reason,
            tier: *tier,
            plan,
            trial_end,
            hours_open: *hours_open,
        })
    }
}

/// See [`SetupOperation::metadata`].
#[derive(Serialize)]
#[serde(untagged)]
pub enum SetupOperationMetadata<'a> {
    Recorded(&'a SetupOperation),
    Swept(SweptItemMetadata<'a>),
}

/// A Resolved or Stuck event with its key's fields at the top level.
#[derive(Serialize)]
pub struct SweptItemMetadata<'a> {
    #[serde(rename = "type")]
    operation: SetupOperationDiscriminants,
    item_type: SetupItemDiscriminants,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    daemon_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning_code: Option<DiscoveryWarningCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<DiscoveryTerminalReason>,
    tier: SetupItemTier,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<&'a BillingPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trial_end: Option<DateTime<Utc>>,
    hours_open: u32,
}
