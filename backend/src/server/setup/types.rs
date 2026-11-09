use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};
use strum_macros::EnumDiscriminants;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    daemon::discovery::types::warnings::{DiscoveryWarning, DiscoveryWarningCode},
    server::{
        billing::types::base::BillingPlan, credentials::r#impl::types::CredentialIntegration,
        daemons::r#impl::api::DaemonHealth, organizations::handlers::ReferralSource,
    },
};

/// One thing on the org's Setup list: a problem to fix (Issue), the next step of first-run setup
/// (Next step), or a feature to try (Suggestion).
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, EnumDiscriminants)]
#[strum_discriminants(derive(
    Serialize,
    Deserialize,
    ToSchema,
    IntoStaticStr,
    EnumIter,
    Hash,
    Ord,
    PartialOrd
))]
#[strum_discriminants(strum(serialize_all = "snake_case"))]
#[serde(tag = "type")]
pub enum SetupItem {
    // Issues
    DaemonHealth {
        daemon_id: Uuid,
        name: String,
        health: DaemonHealth,
    },
    UncoveredIntegration {
        integration: CredentialIntegration,
        host_ids: Vec<Uuid>,
        site_ids: Vec<Uuid>,
    },
    TargetlessCredential {
        credential_id: Uuid,
        name: String,
        integration: CredentialIntegration,
    },
    SiteWithoutDaemon {
        site_id: Uuid,
        name: String,
    },
    RunWarnings {
        daemon_id: Uuid,
        daemon_name: String,
        discovery_id: Uuid,
        code: DiscoveryWarningCode,
        warnings: Vec<DiscoveryWarning>,
    },
    // Next steps. The aliases read the onboarding milestones they replace, which
    // `organizations.setup` was copied from.
    #[strum_discriminants(serde(alias = "FirstDaemonRegistered"))]
    InstallDaemon,
    #[strum_discriminants(serde(alias = "FirstDiscoveryCompleted"))]
    RunFirstScan {
        daemon_id: Uuid,
    },
    #[strum_discriminants(serde(alias = "FirstTopologyRebuild"))]
    OpenTopology,
    // Suggestions
    #[strum_discriminants(serde(alias = "ReferralSourceCompleted"))]
    AnswerReferralSource,
    #[strum_discriminants(serde(alias = "ProfileCompleted"))]
    AnswerProfile,
    ReviewUnconfirmed {
        hosts: u64,
        subnets: u64,
    },
    ExplorePerspectives,
    ApplicationTags,
    Dependencies,
    InterfaceCredential,
    Tags,
    InviteTeam,
    MultiSite,
    ApiKeys,
    Share,
}

/// Which group of the Setup list an item sits in, in display order.
#[derive(
    Serialize, Deserialize, ToSchema, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum SetupItemTier {
    Issue,
    NextStep,
    Suggestion,
}

/// One item's identity across sweeps.
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(tag = "type")]
pub enum SetupItemKey {
    /// An item about the whole org (InstallDaemon, Tags, ...).
    Org { item_type: SetupItemDiscriminants },
    /// An item about one entity: a daemon's health, a credential, a site, an integration's hosts.
    Entity {
        item_type: SetupItemDiscriminants,
        entity_id: Uuid,
    },
    /// One warning code from a daemon's latest runs.
    RunWarning {
        daemon_id: Uuid,
        code: DiscoveryWarningCode,
    },
}

impl SetupItemDiscriminants {
    pub fn tier(self) -> SetupItemTier {
        match self {
            Self::DaemonHealth
            | Self::UncoveredIntegration
            | Self::TargetlessCredential
            | Self::SiteWithoutDaemon
            | Self::RunWarnings => SetupItemTier::Issue,
            Self::InstallDaemon | Self::RunFirstScan | Self::OpenTopology => {
                SetupItemTier::NextStep
            }
            Self::AnswerReferralSource
            | Self::AnswerProfile
            | Self::ReviewUnconfirmed
            | Self::ExplorePerspectives
            | Self::ApplicationTags
            | Self::Dependencies
            | Self::InterfaceCredential
            | Self::Tags
            | Self::InviteTeam
            | Self::MultiSite
            | Self::ApiKeys
            | Self::Share => SetupItemTier::Suggestion,
        }
    }

    /// Whether finishing this item is stored in `organizations.setup`, because live state can't
    /// show it: InstallDaemon (first connection), RunFirstScan, OpenTopology, AnswerProfile,
    /// AnswerReferralSource. Every Suggestion is stored when dismissed. Issues never are, so they
    /// clear only when their condition clears.
    pub fn recorded(self) -> bool {
        match self {
            Self::DaemonHealth
            | Self::UncoveredIntegration
            | Self::TargetlessCredential
            | Self::SiteWithoutDaemon
            | Self::RunWarnings => false,
            Self::InstallDaemon
            | Self::RunFirstScan
            | Self::OpenTopology
            | Self::AnswerReferralSource
            | Self::AnswerProfile
            | Self::ReviewUnconfirmed
            | Self::ExplorePerspectives
            | Self::ApplicationTags
            | Self::Dependencies
            | Self::InterfaceCredential
            | Self::Tags
            | Self::InviteTeam
            | Self::MultiSite
            | Self::ApiKeys
            | Self::Share => true,
        }
    }

    /// The types `values` names, read from stored JSON. A value no type or alias matches (an
    /// onboarding milestone with no setup item) is dropped, and so is a repeat, so the next write
    /// stores the cleaned list.
    pub fn parse_recorded(values: Vec<serde_json::Value>) -> Vec<Self> {
        let mut recorded: Vec<Self> = Vec::new();
        for item_type in values
            .into_iter()
            .filter_map(|v| serde_json::from_value::<Self>(v).ok())
        {
            if !recorded.contains(&item_type) {
                recorded.push(item_type);
            }
        }
        recorded
    }
}

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
    /// A Suggestion was dismissed. Stored in `setup`.
    Dismissed { item_type: SetupItemDiscriminants },
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
            Self::Completed { item_type, .. }
            | Self::Answered { item_type, .. }
            | Self::Dismissed { item_type } => Some(*item_type),
            Self::Resolved { .. } | Self::Stuck { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `organizations.setup` was copied from `onboarding` as it was. Each milestone with a setup
    /// item reads as that item, and the rest drop.
    #[test]
    fn onboarding_milestones_read_as_their_setup_items() {
        let stored = vec![
            json!("OrgCreated"),
            json!("OnboardingModalCompleted"),
            json!("FirstDaemonRegistered"),
            json!("FirstDiscoveryCompleted"),
            json!("FirstTopologyRebuild"),
            json!("ProfileCompleted"),
            json!("ReferralSourceCompleted"),
            json!("FirstTagCreated"),
            json!("DaemonPromptDismissed"),
            json!("InviteSent"),
        ];
        assert_eq!(
            SetupItemDiscriminants::parse_recorded(stored),
            vec![
                SetupItemDiscriminants::InstallDaemon,
                SetupItemDiscriminants::RunFirstScan,
                SetupItemDiscriminants::OpenTopology,
                SetupItemDiscriminants::AnswerProfile,
                SetupItemDiscriminants::AnswerReferralSource,
            ]
        );
    }

    /// An org that recorded a step under its old name and again under its new one holds it once.
    #[test]
    fn an_item_recorded_under_both_names_is_kept_once() {
        let stored = vec![json!("FirstDaemonRegistered"), json!("InstallDaemon")];
        assert_eq!(
            SetupItemDiscriminants::parse_recorded(stored),
            vec![SetupItemDiscriminants::InstallDaemon]
        );
    }

    /// Every item type survives a write and a read of the record.
    #[test]
    fn every_item_type_reads_back_as_written() {
        use strum::IntoEnumIterator;
        let all: Vec<SetupItemDiscriminants> = SetupItemDiscriminants::iter().collect();
        let stored = serde_json::to_value(&all).unwrap();
        let values: Vec<serde_json::Value> = serde_json::from_value(stored).unwrap();
        assert_eq!(SetupItemDiscriminants::parse_recorded(values), all);
    }
}
