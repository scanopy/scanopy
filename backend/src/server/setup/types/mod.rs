//! The Setup list's items and their metadata. Each item's identity and list entry live in
//! [`key`], its events in [`operation`], and the Issues list's grouping and query in [`issues`].

mod issues;
mod key;
mod operation;

pub use issues::*;
pub use key::*;
pub use operation::*;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};
use strum_macros::EnumDiscriminants;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    daemon::discovery::types::{
        base::DiscoveryTerminalReason,
        warnings::{DiscoveryWarning, DiscoveryWarningCode},
    },
    server::{
        auth::{
            access::{Action, Resource},
            middleware::auth::AuthenticatedEntity,
        },
        billing::types::base::BillingPlan,
        credentials::r#impl::types::CredentialIntegration,
        daemons::r#impl::api::DaemonHealth,
        organizations::handlers::ReferralSource,
        shared::types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
        shared::{
            concepts::Concept,
            entities::EntityDiscriminants,
            handlers::query::{OrderDirection, PaginationParams},
        },
    },
};

/// One thing on the org's Setup list: a problem to fix (Issue), or a step of setting up Scanopy
/// (Setup).
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
// No `serialize_all`: the static string is the variant name, which is also the serde value, so the
// fixture id, the i18n key and `type` on the wire are one string.
#[serde(tag = "type")]
pub enum SetupItem {
    // Issues
    /// A daemon whose health isn't Healthy.
    #[schema(title = "DaemonHealth")]
    DaemonHealth {
        /// The daemon.
        daemon_id: Uuid,
        /// The daemon's name.
        name: String,
        /// The site the daemon scans.
        site_id: Uuid,
        /// The daemon's status, the most severe that applies.
        health: DaemonHealth,
    },
    /// An integration the latest scans found on hosts with no credential of it.
    #[schema(title = "UncoveredIntegration")]
    UncoveredIntegration {
        /// The integration a credential would add.
        integration: CredentialIntegration,
        /// The hosts running its service, which the create flow pre-assigns when the
        /// integration's credentials can only be pinned to hosts.
        host_ids: Vec<Uuid>,
        /// The sites those hosts are on, pre-assigned instead when its credentials can apply to
        /// a whole site.
        site_ids: Vec<Uuid>,
    },
    /// A credential with no site or host assignment that no discovery targets, so no daemon
    /// receives it.
    #[schema(title = "TargetlessCredential")]
    TargetlessCredential {
        /// The credential.
        credential_id: Uuid,
        /// The credential's name.
        name: String,
        /// The integration it authenticates.
        integration: CredentialIntegration,
    },
    /// A site with no daemon, once the org has connected one.
    #[schema(title = "SiteWithoutDaemon")]
    SiteWithoutDaemon {
        /// The site.
        site_id: Uuid,
        /// The site's name.
        name: String,
    },
    /// One warning code from a daemon's latest runs.
    #[schema(title = "RunWarnings")]
    RunWarnings {
        /// The daemon that ran the scans.
        daemon_id: Uuid,
        /// The daemon's name.
        daemon_name: String,
        /// The site the daemon scans.
        site_id: Uuid,
        /// The newest run that raised the code.
        discovery_id: Uuid,
        /// The scan that run belongs to, whose settings answer some codes. `None` for a run
        /// recorded before runs carried it.
        scan_id: Option<Uuid>,
        /// The warning code.
        code: DiscoveryWarningCode,
        /// Every warning of the code from the daemon's latest runs.
        warnings: Vec<DiscoveryWarning>,
    },
    /// A daemon's scans whose latest run ended the same way short of finishing.
    #[schema(title = "FailedScans")]
    FailedScans {
        /// The daemon that ran the scans.
        daemon_id: Uuid,
        /// The daemon's name.
        daemon_name: String,
        /// The site the daemon scans.
        site_id: Uuid,
        /// Why the runs ended.
        reason: DiscoveryTerminalReason,
        /// The scans, newest run first.
        scans: Vec<FailedScan>,
    },
    /// Hosts known only from other devices' reports, which the user may review.
    #[schema(title = "UnconfirmedHosts")]
    UnconfirmedHosts {
        /// The site the hosts are on.
        site_id: Uuid,
        /// The unconfirmed hosts on the site.
        count: u64,
    },
    /// Subnets known only from other devices' reports, which the user may review.
    #[schema(title = "UnconfirmedSubnets")]
    UnconfirmedSubnets {
        /// The site the subnets are on.
        site_id: Uuid,
        /// The unconfirmed subnets on the site.
        count: u64,
    },
    // Setup, first steps first. The aliases read the onboarding milestones they replace, which
    // `organizations.setup` was copied from.
    /// Installing a daemon.
    #[schema(title = "InstallDaemon")]
    #[strum_discriminants(serde(alias = "FirstDaemonRegistered"))]
    InstallDaemon,
    /// The first scan.
    #[schema(title = "RunFirstScan")]
    #[strum_discriminants(serde(alias = "FirstDiscoveryCompleted"))]
    RunFirstScan {
        /// The first daemon to connect, which runs the first scan and whose progress the item
        /// shows. `None` while no daemon has connected, or once that daemon is deleted.
        daemon_id: Option<Uuid>,
    },
    /// Opening the topology.
    #[schema(title = "OpenTopology")]
    #[strum_discriminants(serde(alias = "FirstTopologyRebuild"))]
    OpenTopology,
    /// Asks how the org heard of Scanopy.
    #[schema(title = "AnswerReferralSource")]
    #[strum_discriminants(serde(alias = "ReferralSourceCompleted"))]
    AnswerReferralSource,
    /// Asks the user's role and company size.
    #[schema(title = "AnswerProfile")]
    #[strum_discriminants(serde(alias = "ProfileCompleted"))]
    AnswerProfile,
    /// The topology's other views.
    #[schema(title = "ExploreViews")]
    ExploreViews,
    /// Application tags, which group services on the Application view.
    #[schema(title = "ApplicationTags")]
    ApplicationTags,
    /// The dependency tutorial, which opens until it is finished or a dependency exists. It reads
    /// as the Dependencies item it unlocks, so the user sees one step whose click changes.
    #[schema(title = "DependencyTutorial")]
    DependencyTutorial,
    /// Dependencies between services.
    #[schema(title = "Dependencies")]
    Dependencies,
    /// A credential whose integration reports interfaces.
    #[schema(title = "InterfaceCredential")]
    InterfaceCredential,
    /// Tags on hosts.
    #[schema(title = "Tags")]
    Tags,
    /// Inviting members, on a plan with more than one seat.
    #[schema(title = "InviteTeam")]
    InviteTeam,
    /// A second site, on a plan with more than one.
    #[schema(title = "MultiSite")]
    MultiSite,
    /// A user API key, on a plan with API access.
    #[schema(title = "ApiKeys")]
    ApiKeys,
    /// A share link, on a plan with sharing.
    #[schema(title = "Share")]
    Share,
}

/// A scan on a [`SetupItem::FailedScans`] row, and the run of it that didn't finish.
#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq, Eq)]
pub struct FailedScan {
    /// The scan.
    pub scan_id: Uuid,
    /// The scan's name.
    pub name: String,
    /// The scan's latest run, which didn't finish.
    pub run_id: Uuid,
    /// The error the run ended with, if it has one.
    pub error: Option<String>,
    /// The scan's next run is a full scan again, because the full scan that didn't finish
    /// doesn't count.
    pub next_run_full: bool,
}

/// Which group of the Setup list an item sits in, in display order.
#[derive(
    Serialize,
    Deserialize,
    ToSchema,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    EnumIter,
    IntoStaticStr,
)]
pub enum SetupItemTier {
    Issue,
    Setup,
}

impl SetupItem {
    /// This item's identity across sweeps. An integration's hosts are keyed by its first host.
    pub fn key(&self) -> SetupItemKey {
        let item_type = SetupItemDiscriminants::from(self);
        let entity = |entity_id: Uuid| SetupItemKey::Entity {
            item_type,
            entity_id,
        };
        match self {
            Self::DaemonHealth { daemon_id, .. } => entity(*daemon_id),
            Self::UncoveredIntegration { host_ids, .. } => match host_ids.first() {
                Some(host_id) => entity(*host_id),
                None => SetupItemKey::Org { item_type },
            },
            Self::TargetlessCredential { credential_id, .. } => entity(*credential_id),
            Self::SiteWithoutDaemon { site_id, .. }
            | Self::UnconfirmedHosts { site_id, .. }
            | Self::UnconfirmedSubnets { site_id, .. } => entity(*site_id),
            Self::RunWarnings {
                daemon_id, code, ..
            } => SetupItemKey::RunWarning {
                daemon_id: *daemon_id,
                code: *code,
            },
            Self::FailedScans {
                daemon_id, reason, ..
            } => SetupItemKey::FailedScans {
                daemon_id: *daemon_id,
                reason: *reason,
            },
            Self::InstallDaemon
            | Self::RunFirstScan { .. }
            | Self::OpenTopology
            | Self::AnswerReferralSource
            | Self::AnswerProfile
            | Self::ExploreViews
            | Self::ApplicationTags
            | Self::DependencyTutorial
            | Self::Dependencies
            | Self::InterfaceCredential
            | Self::Tags
            | Self::InviteTeam
            | Self::MultiSite
            | Self::ApiKeys
            | Self::Share => SetupItemKey::Org { item_type },
        }
    }

    /// What this Issue's fix is about, which the Issues list groups by: a run warning's code's
    /// category, else its type's. `None` for a Setup step.
    pub fn issue_category(&self) -> Option<IssueCategory> {
        match self {
            Self::RunWarnings { code, .. } => code.issue_category(),
            _ => SetupItemDiscriminants::from(self).issue_category(),
        }
    }

    /// The sites this item is about. Empty for one about the whole org: a credential with no
    /// target, and every Setup item.
    pub fn site_ids(&self) -> Vec<Uuid> {
        match self {
            Self::DaemonHealth { site_id, .. }
            | Self::SiteWithoutDaemon { site_id, .. }
            | Self::RunWarnings { site_id, .. }
            | Self::FailedScans { site_id, .. }
            | Self::UnconfirmedHosts { site_id, .. }
            | Self::UnconfirmedSubnets { site_id, .. } => vec![*site_id],
            Self::UncoveredIntegration { site_ids, .. } => site_ids.clone(),
            Self::TargetlessCredential { .. }
            | Self::InstallDaemon
            | Self::RunFirstScan { .. }
            | Self::OpenTopology
            | Self::AnswerReferralSource
            | Self::AnswerProfile
            | Self::ExploreViews
            | Self::ApplicationTags
            | Self::DependencyTutorial
            | Self::Dependencies
            | Self::InterfaceCredential
            | Self::Tags
            | Self::InviteTeam
            | Self::MultiSite
            | Self::ApiKeys
            | Self::Share => Vec::new(),
        }
    }

    /// Whether `entity` may read what this item is about. An Issue names a daemon, a site, a run
    /// or a credential, so it shows only to a reader of that resource on that item's site. Setup
    /// items are about the org, and every member sees them.
    pub fn readable_by(&self, entity: &AuthenticatedEntity) -> bool {
        let on_site = |resource, site_id: &Uuid| {
            entity
                .site_ids_for(resource, Action::Read)
                .contains(site_id)
        };
        match self {
            Self::DaemonHealth { site_id, .. } => on_site(Resource::Daemon, site_id),
            Self::SiteWithoutDaemon { site_id, .. } => on_site(Resource::Site, site_id),
            Self::RunWarnings { site_id, code, .. } => {
                on_site(Resource::Discovery, site_id)
                    && (!code.concerns_credential()
                        || entity.can(Resource::Credential, Action::Read))
            }
            Self::FailedScans { site_id, .. } => on_site(Resource::Discovery, site_id),
            Self::TargetlessCredential { .. } => entity.can(Resource::Credential, Action::Read),
            Self::UncoveredIntegration { site_ids, .. } => {
                entity.can(Resource::Credential, Action::Read)
                    && site_ids
                        .iter()
                        .any(|site_id| on_site(Resource::Host, site_id))
            }
            Self::UnconfirmedHosts { site_id, .. } => on_site(Resource::Host, site_id),
            Self::UnconfirmedSubnets { site_id, .. } => on_site(Resource::Subnet, site_id),
            Self::InstallDaemon
            | Self::RunFirstScan { .. }
            | Self::OpenTopology
            | Self::AnswerReferralSource
            | Self::AnswerProfile
            | Self::ExploreViews
            | Self::ApplicationTags
            | Self::DependencyTutorial
            | Self::Dependencies
            | Self::InterfaceCredential
            | Self::Tags
            | Self::InviteTeam
            | Self::MultiSite
            | Self::ApiKeys
            | Self::Share => true,
        }
    }
}

impl SetupItemDiscriminants {
    pub fn tier(self) -> SetupItemTier {
        match self {
            Self::DaemonHealth
            | Self::UncoveredIntegration
            | Self::TargetlessCredential
            | Self::SiteWithoutDaemon
            | Self::RunWarnings
            | Self::FailedScans
            | Self::UnconfirmedHosts
            | Self::UnconfirmedSubnets => SetupItemTier::Issue,
            Self::InstallDaemon
            | Self::RunFirstScan
            | Self::OpenTopology
            | Self::AnswerReferralSource
            | Self::AnswerProfile
            | Self::ExploreViews
            | Self::ApplicationTags
            | Self::DependencyTutorial
            | Self::Dependencies
            | Self::InterfaceCredential
            | Self::Tags
            | Self::InviteTeam
            | Self::MultiSite
            | Self::ApiKeys
            | Self::Share => SetupItemTier::Setup,
        }
    }

    /// What an Issue type's fix is about. `None` for a Setup step. RunWarnings' own value is a
    /// fallback: each of its rows takes its code's category ([`SetupItem::issue_category`]).
    pub fn issue_category(self) -> Option<IssueCategory> {
        Some(match self {
            Self::DaemonHealth | Self::SiteWithoutDaemon => IssueCategory::Daemons,
            Self::UncoveredIntegration | Self::TargetlessCredential => IssueCategory::Credentials,
            Self::RunWarnings | Self::FailedScans => IssueCategory::Scans,
            Self::UnconfirmedHosts | Self::UnconfirmedSubnets => IssueCategory::Review,
            Self::InstallDaemon
            | Self::RunFirstScan
            | Self::OpenTopology
            | Self::AnswerReferralSource
            | Self::AnswerProfile
            | Self::ExploreViews
            | Self::ApplicationTags
            | Self::DependencyTutorial
            | Self::Dependencies
            | Self::InterfaceCredential
            | Self::Tags
            | Self::InviteTeam
            | Self::MultiSite
            | Self::ApiKeys
            | Self::Share => return None,
        })
    }

    /// Whether finishing this item is stored in `organizations.setup`: every Setup item is, the
    /// first time it is done, so it stays done. Issues never are, so they clear only when their
    /// condition clears.
    pub fn recorded(self) -> bool {
        self.tier() == SetupItemTier::Setup
    }

    /// Whether the Setup sweep reports an Open item of this type opening and clearing, and on a
    /// trial, left open a day: every Issue, and the first steps without which Scanopy maps
    /// nothing.
    pub fn alerts_when_stuck(self) -> bool {
        match self {
            Self::DaemonHealth
            | Self::UncoveredIntegration
            | Self::TargetlessCredential
            | Self::SiteWithoutDaemon
            | Self::RunWarnings
            | Self::FailedScans
            | Self::UnconfirmedHosts
            | Self::UnconfirmedSubnets
            | Self::InstallDaemon
            | Self::RunFirstScan
            | Self::OpenTopology => true,
            Self::AnswerReferralSource
            | Self::AnswerProfile
            | Self::ExploreViews
            | Self::ApplicationTags
            | Self::DependencyTutorial
            | Self::Dependencies
            | Self::InterfaceCredential
            | Self::Tags
            | Self::InviteTeam
            | Self::MultiSite
            | Self::ApiKeys
            | Self::Share => false,
        }
    }

    /// Whether the app records this item through `POST /organizations/setup`: an item finished in
    /// the browser, with no request of its own the server could record it on, or a question
    /// dismissed unanswered. Every other item is recorded where it happens, or from its live
    /// condition.
    pub fn recorded_by_client(self) -> bool {
        match self {
            Self::DependencyTutorial | Self::AnswerReferralSource | Self::AnswerProfile => true,
            Self::DaemonHealth
            | Self::UncoveredIntegration
            | Self::TargetlessCredential
            | Self::SiteWithoutDaemon
            | Self::RunWarnings
            | Self::FailedScans
            | Self::UnconfirmedHosts
            | Self::UnconfirmedSubnets
            | Self::InstallDaemon
            | Self::RunFirstScan
            | Self::OpenTopology
            | Self::ExploreViews
            | Self::ApplicationTags
            | Self::Dependencies
            | Self::InterfaceCredential
            | Self::Tags
            | Self::InviteTeam
            | Self::MultiSite
            | Self::ApiKeys
            | Self::Share => false,
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

impl HasId for SetupItemDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl SetupItemDiscriminants {
    /// The entity or concept this item is about, whose icon and color it takes. `None` for an
    /// item about neither.
    fn subject(&self) -> Option<&'static dyn EntityMetadataProvider> {
        Some(match self {
            Self::DaemonHealth | Self::InstallDaemon => &EntityDiscriminants::Daemon,
            Self::UncoveredIntegration | Self::TargetlessCredential => {
                &EntityDiscriminants::Credential
            }
            Self::SiteWithoutDaemon | Self::MultiSite => &EntityDiscriminants::Site,
            Self::RunWarnings | Self::FailedScans | Self::RunFirstScan => {
                &EntityDiscriminants::Discovery
            }
            Self::OpenTopology | Self::ExploreViews => &EntityDiscriminants::Topology,
            Self::AnswerProfile => &EntityDiscriminants::User,
            Self::UnconfirmedHosts => &EntityDiscriminants::Host,
            Self::UnconfirmedSubnets => &EntityDiscriminants::Subnet,
            Self::ApplicationTags => &Concept::Application,
            Self::DependencyTutorial | Self::Dependencies => &EntityDiscriminants::Dependency,
            // Ports, neighbours and VLANs are what the credential adds.
            Self::InterfaceCredential => &Concept::L2,
            Self::Tags => &EntityDiscriminants::Tag,
            Self::InviteTeam => &EntityDiscriminants::Invite,
            Self::ApiKeys => &EntityDiscriminants::UserApiKey,
            Self::Share => &EntityDiscriminants::Share,
            // A question about the person.
            Self::AnswerReferralSource => return None,
        })
    }
}

impl EntityMetadataProvider for SetupItemDiscriminants {
    fn color(&self) -> Color {
        self.subject().map_or(Color::Gray, |s| s.color())
    }

    fn icon(&self) -> Icon {
        self.subject()
            .map_or(Icon::MessageCircleQuestion, |s| s.icon())
    }
}

impl TypeMetadataProvider for SetupItemDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            Self::DaemonHealth => "Daemon needs attention",
            Self::UncoveredIntegration => "Integration found with no credential",
            Self::TargetlessCredential => "Credential not used by any scan",
            Self::SiteWithoutDaemon => "Site has no daemon",
            Self::RunWarnings => "Scan left data missing",
            Self::FailedScans => "Scan didn't finish",
            Self::UnconfirmedHosts => "Review unconfirmed hosts",
            Self::UnconfirmedSubnets => "Review unconfirmed subnets",
            Self::InstallDaemon => "Install a daemon",
            Self::RunFirstScan => "Run your first scan",
            Self::OpenTopology => "Open your network map",
            Self::AnswerReferralSource => "How did you hear about Scanopy?",
            Self::AnswerProfile => "Tell us about your role",
            Self::ExploreViews => "Switch topology views",
            Self::ApplicationTags => "Assign an application tag",
            Self::DependencyTutorial | Self::Dependencies => "Map a dependency",
            Self::InterfaceCredential => "Add a switch or router credential",
            Self::Tags => "Assign your first tag",
            Self::InviteTeam => "Invite your team",
            Self::MultiSite => "Add another site",
            Self::ApiKeys => "Create an API key",
            Self::Share => "Share a network map",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::DaemonHealth => "The daemon can't scan until its status is fixed.",
            Self::UncoveredIntegration => {
                "Scans found this integration on your hosts. Add a credential to read what it reports."
            }
            Self::TargetlessCredential => {
                "No site, host or discovery uses this credential, so no daemon receives it. Assign it a target."
            }
            Self::SiteWithoutDaemon => "Nothing on this site is scanned until it has a daemon.",
            Self::RunWarnings => "The daemon's latest scan couldn't read part of your network.",
            Self::FailedScans => {
                "Physical links weren't drawn, because a scan draws them only when it finishes."
            }
            Self::UnconfirmedHosts => {
                "Other devices reported these hosts, and no scan has reached them yet. Confirm, merge or dismiss them."
            }
            Self::UnconfirmedSubnets => {
                "Other devices reported these subnets, and no scan has reached them yet. Confirm, merge or dismiss them."
            }
            Self::InstallDaemon => {
                "The daemon scans your network and reports what it finds. Install it on a host on the network you want to map."
            }
            Self::RunFirstScan => "Your daemon is connected. Its first scan maps your network.",
            Self::OpenTopology => "See the hosts, subnets and links your scan found.",
            // The question is the whole row.
            Self::AnswerReferralSource | Self::AnswerProfile => "",
            Self::ExploreViews => "Switch the map to its layer 2, workloads or applications view.",
            Self::ApplicationTags => {
                "Tag a host or service with an application to see what makes it up as one group on the map."
            }
            Self::DependencyTutorial | Self::Dependencies => {
                "Record which services depend on which, and see the path requests take."
            }
            Self::InterfaceCredential => {
                "An SNMP or controller credential adds ports, neighbours and VLANs to your map."
            }
            Self::Tags => {
                "Tag a host, service or other record. Tags group records by role, owner or environment, and filter every list."
            }
            Self::InviteTeam => {
                "Your plan includes more seats. Invite the people who run the network with you."
            }
            Self::MultiSite => "Your plan includes more sites. Add one for each location you scan.",
            Self::ApiKeys => "Read and update your inventory from scripts and other tools.",
            Self::Share => "Share a read-only link to a network map, or embed it in a wiki.",
        }
    }

    /// An Issue type's category, or the tier for a Setup step. A RunWarnings row groups by its
    /// code's `issue_category`, so this one is only the type's fallback.
    fn category(&self) -> &'static str {
        match self.issue_category() {
            Some(category) => category.id(),
            None => self.tier().id(),
        }
    }
}

impl HasId for SetupItemTier {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SetupItemTier {
    fn color(&self) -> Color {
        match self {
            Self::Issue => Color::Red,
            Self::Setup => Color::Blue,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Issue => Icon::CircleAlert,
            Self::Setup => Icon::Rocket,
        }
    }
}

impl TypeMetadataProvider for SetupItemTier {
    fn name(&self) -> &'static str {
        match self {
            Self::Issue => "Issues",
            Self::Setup => "Setup",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Issue => {
                "Problems that leave part of your network unscanned. Each clears once fixed."
            }
            Self::Setup => "The steps of setting up Scanopy. Each stays done once done.",
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
