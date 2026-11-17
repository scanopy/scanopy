//! Each Setup item type's condition, in one exhaustive match over facts gathered once.
//!
//! [`SetupItemDiscriminants::items`] is pure: [`SetupContext`] is a plain struct
//! [`super::service::SetupService::gather`] fills through each entity's service, so every arm is
//! tested from a hand-built context.

mod scans;
#[cfg(test)]
mod tests;

pub use scans::is_setup_issue_warning;
use scans::{failed_scan_items, run_warning_items};

use std::collections::{BTreeSet, HashSet};

use semver::Version;
use uuid::Uuid;

use super::types::{
    FailedScan, SetupItem, SetupItemDiscriminants, SetupItemState, SetupItemTier, SetupListEntry,
};
use crate::server::organizations::r#impl::intent::SignupIntent;
use crate::{
    daemon::discovery::types::{
        base::DiscoveryTerminalReason,
        warnings::{DiscoveryWarning, DiscoveryWarningCode},
    },
    server::{
        auth::access::Resource, billing::types::base::BillingPlan,
        credentials::r#impl::types::CredentialIntegration, daemons::r#impl::api::DaemonHealth,
    },
};

/// A daemon as the Setup list reads it.
#[derive(Debug, Clone)]
pub struct SetupDaemon {
    pub id: Uuid,
    pub name: String,
    pub site_id: Uuid,
    pub health: DaemonHealth,
    pub version: Option<Version>,
}

#[derive(Debug, Clone)]
pub struct SetupSite {
    pub id: Uuid,
    pub name: String,
}

/// A credential no scan will send to a daemon.
#[derive(Debug, Clone)]
pub struct TargetlessCredential {
    pub id: Uuid,
    pub name: String,
    pub integration: CredentialIntegration,
}

/// An integration the latest scans found on hosts with no credential of it, still uncovered now.
#[derive(Debug, Clone)]
pub struct UncoveredIntegration {
    pub integration: CredentialIntegration,
    pub host_ids: Vec<Uuid>,
    pub site_ids: Vec<Uuid>,
}

/// One of a daemon's latest completed runs, holding only its setup-issue warnings.
#[derive(Debug, Clone)]
pub struct DaemonRunWarnings {
    pub daemon_id: Uuid,
    pub discovery_id: Uuid,
    /// The scan the run belongs to, whose settings answer some warnings.
    pub scan_id: Option<Uuid>,
    pub warnings: Vec<DiscoveryWarning>,
}

/// A scan whose latest run didn't finish.
#[derive(Debug, Clone)]
pub struct FailedScanRun {
    pub daemon_id: Uuid,
    pub scan_id: Uuid,
    pub scan_name: String,
    /// The latest run, which didn't finish.
    pub run_id: Uuid,
    pub reason: DiscoveryTerminalReason,
    /// The run before it didn't finish either.
    pub previous_run_failed: bool,
    /// An enabled schedule runs the scan again.
    pub scheduled: bool,
    /// The scan's next run is a full scan: this run was one, and didn't count.
    pub next_run_full: bool,
}

/// Rows whose count completes a Setup item.
#[derive(Debug, Clone, Default)]
pub struct SetupCounts {
    pub dependencies: u64,
    pub user_api_keys: u64,
    pub shares: u64,
    pub members: u64,
    pub pending_invites: u64,
}

/// The facts every item type's condition reads.
#[derive(Debug, Clone)]
pub struct SetupContext {
    /// The Setup types the org has done (`organizations.setup`).
    pub recorded: HashSet<SetupItemDiscriminants>,
    pub daemons: Vec<SetupDaemon>,
    pub sites: Vec<SetupSite>,
    pub targetless_credentials: Vec<TargetlessCredential>,
    pub uncovered_integrations: Vec<UncoveredIntegration>,
    /// Each daemon's latest run of each discovery type, newest first.
    pub run_warnings: Vec<DaemonRunWarnings>,
    /// Each daemon's scans whose latest run didn't finish, newest first.
    pub failed_scans: Vec<FailedScanRun>,
    /// The first daemon to connect, which runs the first scan.
    pub first_scan_daemon_id: Option<Uuid>,
    /// Unconfirmed hosts per site, on the sites the user reads hosts on. Sites with none are absent.
    pub unconfirmed_hosts: Vec<(Uuid, u64)>,
    /// Unconfirmed subnets per site, on the sites the user reads subnets on.
    pub unconfirmed_subnets: Vec<(Uuid, u64)>,
    pub counts: SetupCounts,
    /// Some entity in the org holds a tag.
    pub has_tag_assignment: bool,
    /// Some service holds a tag of the org's application group.
    pub has_application_tag_assignment: bool,
    pub has_manual_interface_credential: bool,
    pub plan: BillingPlan,
    /// Whether this is Scanopy Cloud. The two questions are asked only there.
    pub cloud: bool,
    /// What the signup wants Scanopy to do, which moves the Setup steps it names up the list.
    /// Empty when never asked or skipped.
    pub intents: Vec<SignupIntent>,
    /// The resources the user may write. `None` in the sweep, which acts for the whole org and
    /// passes every write check.
    pub can_write: Option<BTreeSet<Resource>>,
}

impl SetupContext {
    fn has(&self, item_type: SetupItemDiscriminants) -> bool {
        self.recorded.contains(&item_type)
    }

    fn can_write(&self, resource: Resource) -> bool {
        self.can_write
            .as_ref()
            .is_none_or(|writable| writable.contains(&resource))
    }

    /// Where a type sorts in the list: Issues and the first steps (daemon, first scan, topology)
    /// keep their place, then the Setup steps the org's intents name, then the rest. Ties keep
    /// declaration order. Locks are unaffected, so a step moved up can still read Locked.
    pub fn list_rank(&self, item_type: SetupItemDiscriminants) -> u8 {
        use SetupItemDiscriminants as T;
        match item_type {
            _ if item_type.tier() == SetupItemTier::Issue => 0,
            T::InstallDaemon | T::RunFirstScan | T::OpenTopology => 1,
            _ if self
                .intents
                .iter()
                .any(|intent| intent.setup_items().contains(&item_type)) =>
            {
                2
            }
            _ => 3,
        }
    }

    /// Whether the dependency tutorial is done, recorded or shown done by the org's rows.
    fn dependency_tutorial_done(&self) -> bool {
        use SetupItemDiscriminants as T;
        self.has(T::DependencyTutorial) || T::DependencyTutorial.live_done(self)
    }
}

impl SetupItemDiscriminants {
    /// This type's entries for the org. Issues return one Open entry per affected entity. A
    /// Setup type returns its one entry, or none when it doesn't apply to this org and user (a
    /// plan feature, Scanopy Cloud, a write permission). A Setup entry is Done once recorded or
    /// once [`Self::live_done`] holds, Locked while its prerequisite isn't recorded, and Open
    /// otherwise.
    pub fn items(self, ctx: &SetupContext) -> Vec<SetupListEntry> {
        use SetupItemDiscriminants as T;
        let features = ctx.plan.features();
        let issue = |item: SetupItem| SetupListEntry::new(item, SetupItemState::Open);
        let step = |applies: bool, locked: bool, item: SetupItem| {
            if !applies {
                return Vec::new();
            }
            let state = if ctx.has(self) || self.live_done(ctx) {
                SetupItemState::Done
            } else if locked {
                SetupItemState::Locked
            } else {
                SetupItemState::Open
            };
            vec![SetupListEntry::new(item, state)]
        };
        match self {
            T::DaemonHealth => ctx
                .daemons
                .iter()
                .filter(|d| d.health != DaemonHealth::Healthy)
                .map(|d| {
                    issue(SetupItem::DaemonHealth {
                        daemon_id: d.id,
                        name: d.name.clone(),
                        site_id: d.site_id,
                        health: d.health,
                    })
                })
                .collect(),
            T::UncoveredIntegration => ctx
                .uncovered_integrations
                .iter()
                .map(|u| {
                    issue(SetupItem::UncoveredIntegration {
                        integration: u.integration,
                        host_ids: u.host_ids.clone(),
                        site_ids: u.site_ids.clone(),
                    })
                })
                .collect(),
            T::TargetlessCredential => ctx
                .targetless_credentials
                .iter()
                .map(|c| {
                    issue(SetupItem::TargetlessCredential {
                        credential_id: c.id,
                        name: c.name.clone(),
                        integration: c.integration,
                    })
                })
                .collect(),
            // Before the first connection, InstallDaemon already asks for a daemon.
            T::SiteWithoutDaemon if !ctx.has(T::InstallDaemon) => Vec::new(),
            T::SiteWithoutDaemon => ctx
                .sites
                .iter()
                .filter(|s| !ctx.daemons.iter().any(|d| d.site_id == s.id))
                .map(|s| {
                    issue(SetupItem::SiteWithoutDaemon {
                        site_id: s.id,
                        name: s.name.clone(),
                    })
                })
                .collect(),
            T::RunWarnings => run_warning_items(ctx).into_iter().map(issue).collect(),
            T::FailedScans => failed_scan_items(ctx).into_iter().map(issue).collect(),
            T::UnconfirmedHosts if ctx.can_write(Resource::Host) => ctx
                .unconfirmed_hosts
                .iter()
                .filter(|(_, count)| *count > 0)
                .map(|&(site_id, count)| issue(SetupItem::UnconfirmedHosts { site_id, count }))
                .collect(),
            T::UnconfirmedSubnets if ctx.can_write(Resource::Subnet) => ctx
                .unconfirmed_subnets
                .iter()
                .filter(|(_, count)| *count > 0)
                .map(|&(site_id, count)| issue(SetupItem::UnconfirmedSubnets { site_id, count }))
                .collect(),
            T::UnconfirmedHosts | T::UnconfirmedSubnets => Vec::new(),
            // A daemon created and never connected is an AwaitingConnection Issue, which waits
            // in this step's place.
            T::InstallDaemon => step(
                true,
                ctx.daemons
                    .iter()
                    .any(|d| d.health == DaemonHealth::AwaitingConnection),
                SetupItem::InstallDaemon,
            ),
            T::RunFirstScan => step(
                true,
                !ctx.has(T::InstallDaemon),
                SetupItem::RunFirstScan {
                    daemon_id: ctx.first_scan_daemon_id,
                },
            ),
            // The topology views are a plan feature.
            T::OpenTopology => step(
                features.includes_topology(),
                !ctx.has(T::RunFirstScan),
                SetupItem::OpenTopology,
            ),
            // Cloud only. Questions are never locked: they can be answered from the start.
            T::AnswerReferralSource => step(ctx.cloud, false, SetupItem::AnswerReferralSource),
            T::AnswerProfile => step(ctx.cloud, false, SetupItem::AnswerProfile),
            T::ExploreViews => step(
                features.includes_topology(),
                !ctx.has(T::OpenTopology),
                SetupItem::ExploreViews,
            ),
            T::ApplicationTags => step(
                features.tagging && ctx.can_write(Resource::Tag),
                !ctx.has(T::OpenTopology),
                SetupItem::ApplicationTags,
            ),
            // The tutorial and the step it unlocks read the same; only the click differs. They are
            // one row and one count: the tutorial until it is done, then the step it unlocks.
            // Dependencies are drawn on the topology, so both need a plan with its views.
            T::DependencyTutorial => step(
                features.includes_topology()
                    && ctx.can_write(Resource::Dependency)
                    && !ctx.dependency_tutorial_done(),
                !ctx.has(T::OpenTopology),
                SetupItem::DependencyTutorial,
            ),
            T::Dependencies => step(
                features.includes_topology()
                    && ctx.can_write(Resource::Dependency)
                    && ctx.dependency_tutorial_done(),
                false,
                SetupItem::Dependencies,
            ),
            // Left out until the first scan, so the list leads with daemon, scan, topology. A
            // credential added before then (the Create Daemon wizard offers one) shows it done.
            T::InterfaceCredential => step(
                ctx.can_write(Resource::Credential)
                    && (ctx.has(T::RunFirstScan) || ctx.has(self) || self.live_done(ctx)),
                false,
                SetupItem::InterfaceCredential,
            ),
            T::Tags => step(
                features.tagging && ctx.can_write(Resource::Tag),
                !ctx.has(T::RunFirstScan),
                SetupItem::Tags,
            ),
            T::InviteTeam => step(
                ctx.plan.can_invite_users() && ctx.can_write(Resource::Member),
                false,
                SetupItem::InviteTeam,
            ),
            T::MultiSite => step(
                plan_allows_several_sites(&ctx.plan) && ctx.can_write(Resource::NewSite),
                false,
                SetupItem::MultiSite,
            ),
            T::ApiKeys => step(
                features.api_access,
                !ctx.has(T::RunFirstScan),
                SetupItem::ApiKeys,
            ),
            T::Share => step(
                features.sharing && ctx.can_write(Resource::Share),
                !ctx.has(T::OpenTopology),
                SetupItem::Share,
            ),
        }
    }

    /// Whether the org's rows show this Setup type done, which records it. `false` for Issues
    /// and for the types a request records directly (the first connection, the first scan, the
    /// topology views, the answers).
    pub fn live_done(self, ctx: &SetupContext) -> bool {
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
            | Self::OpenTopology
            | Self::AnswerReferralSource
            | Self::AnswerProfile
            | Self::ExploreViews => false,
            Self::Tags => ctx.has_tag_assignment,
            Self::ApplicationTags => ctx.has_application_tag_assignment,
            // A dependency made without the tutorial means it isn't needed.
            Self::DependencyTutorial | Self::Dependencies => ctx.counts.dependencies > 0,
            Self::InterfaceCredential => ctx.has_manual_interface_credential,
            Self::InviteTeam => ctx.counts.members > 1 || ctx.counts.pending_invites > 0,
            Self::MultiSite => ctx.sites.len() > 1,
            Self::ApiKeys => ctx.counts.user_api_keys > 0,
            Self::Share => ctx.counts.shares > 0,
        }
    }
}

/// Whether the plan holds more than one site, included or bought.
fn plan_allows_several_sites(plan: &BillingPlan) -> bool {
    plan.site_limit().is_none_or(|included| included > 1) || plan.config().site_cents.is_some()
}
