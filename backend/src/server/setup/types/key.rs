//! An item's identity, state and list entry.

use super::*;

/// One item's identity across sweeps and page loads.
#[derive(
    Serialize,
    Deserialize,
    ToSchema,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    EnumDiscriminants,
)]
#[strum_discriminants(derive(strum::VariantNames))]
#[serde(tag = "type")]
pub enum SetupItemKey {
    /// An item about the whole org (InstallDaemon, Tags, ...).
    #[schema(title = "Org")]
    Org { item_type: SetupItemDiscriminants },
    /// An item about one entity: a daemon's health, a credential, a site, an integration's hosts.
    #[schema(title = "Entity")]
    Entity {
        item_type: SetupItemDiscriminants,
        /// The daemon, credential or site the item concerns. For an uncovered integration, its
        /// first host.
        entity_id: Uuid,
    },
    /// One warning code from a daemon's latest runs.
    #[schema(title = "RunWarning")]
    RunWarning {
        /// The daemon whose latest runs raised the warning.
        daemon_id: Uuid,
        code: DiscoveryWarningCode,
    },
    /// A daemon's scans whose latest runs ended the same way short of finishing.
    #[schema(title = "FailedScans")]
    FailedScans {
        /// The daemon that ran the scans.
        daemon_id: Uuid,
        reason: DiscoveryTerminalReason,
    },
}

impl SetupItemKey {
    /// The type of the item this key identifies.
    pub fn item_type(&self) -> SetupItemDiscriminants {
        match self {
            Self::Org { item_type } | Self::Entity { item_type, .. } => *item_type,
            Self::RunWarning { .. } => SetupItemDiscriminants::RunWarnings,
            Self::FailedScans { .. } => SetupItemDiscriminants::FailedScans,
        }
    }
}

/// Where a Setup item stands. Issues are always Open.
#[derive(Serialize, Deserialize, ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SetupItemState {
    /// Still to do.
    Open,
    /// Waiting on an earlier step. Shown disabled.
    Locked,
    /// Done once, and done for good.
    Done,
}

/// One Setup list row as the dashboard serves it: the item, with its identity, its tier and its
/// state.
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct SetupListEntry {
    /// The item's identity: the same item on the next load has the same key.
    pub key: SetupItemKey,
    /// The group of the list the item sits in.
    pub tier: SetupItemTier,
    pub state: SetupItemState,
    /// Whether the trial sweep reports the item once it stays open a day: an Issue, or a first
    /// step still Open.
    pub alerts_when_stuck: bool,
    /// The sites the item is about, which the list can be narrowed to. Empty for an item about
    /// the whole org, which shows whatever sites are picked.
    pub site_ids: Vec<Uuid>,
    pub item: SetupItem,
}

impl SetupListEntry {
    pub fn new(item: SetupItem, state: SetupItemState) -> Self {
        let item_type = SetupItemDiscriminants::from(&item);
        Self {
            key: item.key(),
            tier: item_type.tier(),
            state,
            alerts_when_stuck: item_type.alerts_when_stuck() && state == SetupItemState::Open,
            site_ids: item.site_ids(),
            item,
        }
    }
}

/// An Issue or first step the trial sweep saw open, and since when.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct OpenSetupItem {
    pub key: SetupItemKey,
    pub open_since: DateTime<Utc>,
}
