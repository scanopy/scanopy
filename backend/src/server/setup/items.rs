//! Each Setup item type's condition, in one exhaustive match over facts gathered once.
//!
//! [`SetupItemDiscriminants::items`] is pure: [`SetupContext`] is a plain struct
//! [`super::service::SetupService::gather`] fills through each entity's service, so every arm is
//! tested from a hand-built context.

use std::collections::{BTreeSet, HashSet};

use semver::Version;
use uuid::Uuid;

use super::types::{SetupItem, SetupItemDiscriminants, SetupItemState, SetupListEntry};
use crate::{
    daemon::discovery::types::warnings::{DiscoveryWarning, DiscoveryWarningCode},
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
            // The Create Daemon wizard offers credentials, so this needs no step first.
            T::InterfaceCredential => step(
                ctx.can_write(Resource::Credential),
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

/// One item per daemon and code, from the newest run that raised it. A daemon deleted since has
/// nothing to fix. A credential withheld from a daemon too old for it clears once the daemon is
/// upgraded past the gap, before the next run says so.
fn run_warning_items(ctx: &SetupContext) -> Vec<SetupItem> {
    let mut items: Vec<SetupItem> = Vec::new();
    for run in &ctx.run_warnings {
        let Some(daemon) = ctx.daemons.iter().find(|d| d.id == run.daemon_id) else {
            continue;
        };
        for warning in &run.warnings {
            if let DiscoveryWarning::CredentialNeedsNewerDaemon {
                last_unsupported_daemon_version,
                ..
            } = warning
                && daemon
                    .version
                    .as_ref()
                    .is_some_and(|v| v > last_unsupported_daemon_version)
            {
                continue;
            }
            let code = warning.code();
            let existing = items.iter_mut().find_map(|item| match item {
                SetupItem::RunWarnings {
                    daemon_id,
                    code: c,
                    warnings,
                    ..
                } if *daemon_id == daemon.id && *c == code => Some(warnings),
                _ => None,
            });
            match existing {
                Some(warnings) => warnings.push(warning.clone()),
                None => items.push(SetupItem::RunWarnings {
                    daemon_id: daemon.id,
                    daemon_name: daemon.name.clone(),
                    site_id: daemon.site_id,
                    discovery_id: run.discovery_id,
                    scan_id: run.scan_id,
                    code,
                    warnings: vec![warning.clone()],
                }),
            }
        }
    }
    items
}

/// Whether a stored run warning still belongs on the Setup list: its code is a setup issue, and a
/// credential warning still names its credential (one deleted since has nothing to fix).
pub fn is_setup_issue_warning(warning: &DiscoveryWarning) -> bool {
    let code: DiscoveryWarningCode = warning.code();
    code.is_setup_issue() && (!code.concerns_credential() || warning.credential_id().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::setup::types::SetupItemTier;
    use crate::server::{
        billing::plans::{get_enterprise_plan, get_free_plan},
        credentials::{
            r#impl::capability::CapabilityGap, r#impl::mapping::CredentialQueryPayloadDiscriminants,
        },
    };
    use SetupItemDiscriminants as T;
    use strum::IntoEnumIterator;

    fn daemon(site_id: Uuid, health: DaemonHealth) -> SetupDaemon {
        SetupDaemon {
            id: Uuid::new_v4(),
            name: "edge-01".to_string(),
            site_id,
            health,
            version: Some(Version::new(0, 18, 0)),
        }
    }

    fn site(name: &str) -> SetupSite {
        SetupSite {
            id: Uuid::new_v4(),
            name: name.to_string(),
        }
    }

    /// An org on a plan with every feature, whose user may write everything, with nothing
    /// recorded and nothing found.
    fn ctx() -> SetupContext {
        SetupContext {
            recorded: HashSet::new(),
            daemons: Vec::new(),
            sites: vec![site("Head office")],
            targetless_credentials: Vec::new(),
            uncovered_integrations: Vec::new(),
            run_warnings: Vec::new(),
            first_scan_daemon_id: None,
            unconfirmed_hosts: Vec::new(),
            unconfirmed_subnets: Vec::new(),
            counts: SetupCounts {
                members: 1,
                ..Default::default()
            },
            has_tag_assignment: false,
            has_application_tag_assignment: false,
            has_manual_interface_credential: false,
            plan: get_enterprise_plan(),
            cloud: true,
            can_write: Some(Resource::iter().collect()),
        }
    }

    fn recorded(mut ctx: SetupContext, types: &[T]) -> SetupContext {
        ctx.recorded.extend(types.iter().copied());
        ctx
    }

    fn without_write(mut ctx: SetupContext, resource: Resource) -> SetupContext {
        if let Some(writable) = ctx.can_write.as_mut() {
            writable.remove(&resource);
        }
        ctx
    }

    fn count(item_type: T, ctx: &SetupContext) -> usize {
        item_type.items(ctx).len()
    }

    fn items(item_type: T, ctx: &SetupContext) -> Vec<SetupItem> {
        item_type.items(ctx).into_iter().map(|e| e.item).collect()
    }

    /// The state of a Setup type's one entry, `None` when it doesn't apply.
    fn state(item_type: T, ctx: &SetupContext) -> Option<SetupItemState> {
        match item_type.items(ctx).as_slice() {
            [] => None,
            [entry] => Some(entry.state),
            entries => panic!("{item_type:?} listed {} entries", entries.len()),
        }
    }

    fn needs_newer_daemon(last_unsupported: Version) -> DiscoveryWarning {
        DiscoveryWarning::CredentialNeedsNewerDaemon {
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            credential_id: Some(Uuid::new_v4()),
            gap: CapabilityGap::ParsesCredentialType,
            last_unsupported_daemon_version: last_unsupported,
            daemon_version: None,
        }
    }

    fn time_limit() -> DiscoveryWarning {
        DiscoveryWarning::ScanTimeLimit {
            hours: 4,
            hosts_not_scanned: 12,
        }
    }

    #[test]
    fn every_unhealthy_daemon_is_an_issue() {
        let mut ctx = ctx();
        let site_id = ctx.sites[0].id;
        let outdated = daemon(site_id, DaemonHealth::Outdated);
        ctx.daemons = vec![daemon(site_id, DaemonHealth::Healthy), outdated.clone()];
        let items = items(T::DaemonHealth, &ctx);
        assert!(matches!(
            items.as_slice(),
            [SetupItem::DaemonHealth { daemon_id, health: DaemonHealth::Outdated, .. }]
                if *daemon_id == outdated.id
        ));
    }

    #[test]
    fn found_integrations_and_targetless_credentials_are_issues_while_listed() {
        let mut ctx = ctx();
        assert_eq!(count(T::UncoveredIntegration, &ctx), 0);
        assert_eq!(count(T::TargetlessCredential, &ctx), 0);
        ctx.uncovered_integrations = vec![UncoveredIntegration {
            integration: CredentialIntegration::Snmp,
            host_ids: vec![Uuid::new_v4()],
            site_ids: vec![ctx.sites[0].id],
        }];
        ctx.targetless_credentials = vec![TargetlessCredential {
            id: Uuid::new_v4(),
            name: "Core switch".to_string(),
            integration: CredentialIntegration::Snmp,
        }];
        assert_eq!(count(T::UncoveredIntegration, &ctx), 1);
        assert_eq!(count(T::TargetlessCredential, &ctx), 1);
    }

    /// Only once the org has connected a daemon, and then on every site with none, including after
    /// the last daemon is deleted.
    #[test]
    fn a_site_without_a_daemon_is_an_issue_after_the_first_connection() {
        let mut ctx = ctx();
        let branch = site("Branch");
        ctx.sites.push(branch.clone());
        ctx.daemons = vec![daemon(ctx.sites[0].id, DaemonHealth::Healthy)];
        assert_eq!(count(T::SiteWithoutDaemon, &ctx), 0);

        let mut ctx = recorded(ctx, &[T::InstallDaemon]);
        assert!(matches!(
            items(T::SiteWithoutDaemon, &ctx).as_slice(),
            [SetupItem::SiteWithoutDaemon { site_id, .. }] if *site_id == branch.id
        ));

        ctx.daemons.clear();
        assert_eq!(count(T::SiteWithoutDaemon, &ctx), 2);
    }

    /// One item per daemon and code, carrying every warning of that code and the newest run that
    /// raised it. A deleted daemon's runs raise nothing.
    #[test]
    fn run_warnings_group_by_daemon_and_code() {
        let mut ctx = ctx();
        let d = daemon(ctx.sites[0].id, DaemonHealth::Healthy);
        ctx.daemons = vec![d.clone()];
        let (newer, older) = (Uuid::new_v4(), Uuid::new_v4());
        ctx.run_warnings = vec![
            DaemonRunWarnings {
                daemon_id: d.id,
                discovery_id: newer,
                scan_id: None,
                warnings: vec![time_limit()],
            },
            DaemonRunWarnings {
                daemon_id: d.id,
                discovery_id: older,
                scan_id: None,
                warnings: vec![time_limit()],
            },
            DaemonRunWarnings {
                daemon_id: Uuid::new_v4(),
                discovery_id: Uuid::new_v4(),
                scan_id: None,
                warnings: vec![time_limit()],
            },
        ];
        let items = items(T::RunWarnings, &ctx);
        assert!(matches!(
            items.as_slice(),
            [SetupItem::RunWarnings { discovery_id, code: DiscoveryWarningCode::ScanTimeLimit, warnings, .. }]
                if *discovery_id == newer && warnings.len() == 2
        ));
    }

    /// A credential withheld from a daemon too old for it clears once the daemon passes the gap.
    #[test]
    fn a_withheld_credential_clears_once_the_daemon_is_newer_than_the_gap() {
        let mut ctx = ctx();
        let d = daemon(ctx.sites[0].id, DaemonHealth::Healthy);
        ctx.daemons = vec![d.clone()];
        ctx.run_warnings = vec![DaemonRunWarnings {
            daemon_id: d.id,
            discovery_id: Uuid::new_v4(),
            scan_id: None,
            warnings: vec![needs_newer_daemon(Version::new(0, 18, 0))],
        }];
        assert_eq!(count(T::RunWarnings, &ctx), 1);

        ctx.daemons[0].version = Some(Version::new(0, 18, 1));
        assert_eq!(count(T::RunWarnings, &ctx), 0);
    }

    /// A daemon created and never connected is an AwaitingConnection Issue, and InstallDaemon
    /// waits behind it.
    #[test]
    fn a_never_connected_daemon_locks_install_daemon() {
        let mut ctx = ctx();
        assert_eq!(state(T::InstallDaemon, &ctx), Some(SetupItemState::Open));

        ctx.daemons = vec![daemon(ctx.sites[0].id, DaemonHealth::AwaitingConnection)];
        assert_eq!(state(T::InstallDaemon, &ctx), Some(SetupItemState::Locked));
        assert!(matches!(
            items(T::DaemonHealth, &ctx).as_slice(),
            [SetupItem::DaemonHealth {
                health: DaemonHealth::AwaitingConnection,
                ..
            }]
        ));

        ctx.daemons.clear();
        let ctx = recorded(ctx, &[T::InstallDaemon]);
        assert_eq!(state(T::InstallDaemon, &ctx), Some(SetupItemState::Done));
    }

    #[test]
    fn run_first_scan_opens_at_the_first_connection_and_names_the_daemon() {
        let mut ctx = ctx();
        assert_eq!(state(T::RunFirstScan, &ctx), Some(SetupItemState::Locked));

        let daemon_id = Uuid::new_v4();
        ctx.first_scan_daemon_id = Some(daemon_id);
        let ctx = recorded(ctx, &[T::InstallDaemon]);
        assert_eq!(state(T::RunFirstScan, &ctx), Some(SetupItemState::Open));
        assert!(matches!(
            items(T::RunFirstScan, &ctx).as_slice(),
            [SetupItem::RunFirstScan { daemon_id: Some(d) }] if *d == daemon_id
        ));
        assert_eq!(
            state(T::RunFirstScan, &recorded(ctx, &[T::RunFirstScan])),
            Some(SetupItemState::Done)
        );
    }

    /// Each Setup type's prerequisite, and the context where its rows show it done. A type the
    /// rows can't show is done only once recorded.
    fn setup_cases() -> Vec<(T, Option<T>, Option<SetupContext>)> {
        let base = ctx();
        let with = |change: &dyn Fn(&mut SetupContext)| {
            let mut ctx = base.clone();
            change(&mut ctx);
            Some(ctx)
        };
        vec![
            (T::InstallDaemon, None, None),
            (T::RunFirstScan, Some(T::InstallDaemon), None),
            (T::OpenTopology, Some(T::RunFirstScan), None),
            (T::AnswerReferralSource, None, None),
            (T::AnswerProfile, None, None),
            (T::ExploreViews, Some(T::OpenTopology), None),
            (
                T::ApplicationTags,
                Some(T::OpenTopology),
                with(&|c| c.has_application_tag_assignment = true),
            ),
            (
                T::InterfaceCredential,
                None,
                with(&|c| c.has_manual_interface_credential = true),
            ),
            (
                T::Tags,
                Some(T::RunFirstScan),
                with(&|c| c.has_tag_assignment = true),
            ),
            (T::InviteTeam, None, with(&|c| c.counts.pending_invites = 1)),
            (T::MultiSite, None, with(&|c| c.sites.push(site("Branch")))),
            (
                T::ApiKeys,
                Some(T::RunFirstScan),
                with(&|c| c.counts.user_api_keys = 1),
            ),
            (
                T::Share,
                Some(T::OpenTopology),
                with(&|c| c.counts.shares = 1),
            ),
        ]
    }

    /// Every Setup type is Locked until its prerequisite is recorded, then Open, then Done once
    /// recorded or once the rows show it done, whatever its prerequisite. The dependency pair,
    /// one row between them, has its own test.
    #[test]
    fn a_setup_item_is_locked_then_open_then_done() {
        let cases = setup_cases();
        assert_eq!(
            cases.iter().map(|(t, ..)| *t).collect::<Vec<_>>(),
            T::iter()
                .filter(|t| t.tier() == SetupItemTier::Setup)
                .filter(|t| !matches!(t, T::DependencyTutorial | T::Dependencies))
                .collect::<Vec<_>>(),
            "every Setup type has a case"
        );
        for (item_type, prerequisite, live_done) in cases {
            let ready = recorded(ctx(), &prerequisite.into_iter().collect::<Vec<_>>());
            if prerequisite.is_some() {
                assert_eq!(
                    state(item_type, &ctx()),
                    Some(SetupItemState::Locked),
                    "{item_type:?}"
                );
            }
            assert_eq!(
                state(item_type, &ready),
                Some(SetupItemState::Open),
                "{item_type:?}"
            );
            assert_eq!(
                state(item_type, &recorded(ctx(), &[item_type])),
                Some(SetupItemState::Done),
                "{item_type:?} recorded"
            );
            assert!(!item_type.live_done(&ctx()), "{item_type:?}");
            if let Some(live_done) = live_done {
                assert!(item_type.live_done(&live_done), "{item_type:?}");
                assert_eq!(
                    state(item_type, &live_done),
                    Some(SetupItemState::Done),
                    "{item_type:?} done in the rows"
                );
            }
        }
    }

    /// The dependency tutorial and the step it unlocks are one row: the tutorial, Locked then
    /// Open, until it is done, then the step. A dependency made first completes both.
    #[test]
    fn the_dependency_pair_is_one_row() {
        let pair = |ctx: &SetupContext| -> Vec<(T, SetupItemState)> {
            [T::DependencyTutorial, T::Dependencies]
                .into_iter()
                .flat_map(|t| t.items(ctx).into_iter().map(move |e| (t, e.state)))
                .collect()
        };
        let mut with_dependency = recorded(ctx(), &[T::OpenTopology]);
        with_dependency.counts.dependencies = 1;
        for (context, expected) in [
            (ctx(), (T::DependencyTutorial, SetupItemState::Locked)),
            (
                recorded(ctx(), &[T::OpenTopology]),
                (T::DependencyTutorial, SetupItemState::Open),
            ),
            (
                recorded(ctx(), &[T::OpenTopology, T::DependencyTutorial]),
                (T::Dependencies, SetupItemState::Open),
            ),
            (with_dependency, (T::Dependencies, SetupItemState::Done)),
        ] {
            assert_eq!(pair(&context), vec![expected]);
        }
    }

    /// A Setup item that needs a write is left out for a user without it. The sweep, with no
    /// user, passes every write check.
    #[test]
    fn a_setup_item_needing_a_write_is_left_out_without_it() {
        for (item_type, resource) in [
            (T::ApplicationTags, Resource::Tag),
            (T::Tags, Resource::Tag),
            (T::DependencyTutorial, Resource::Dependency),
            (T::InterfaceCredential, Resource::Credential),
            (T::InviteTeam, Resource::Member),
            (T::MultiSite, Resource::NewSite),
            (T::Share, Resource::Share),
        ] {
            assert_eq!(
                count(item_type, &without_write(ctx(), resource)),
                0,
                "{item_type:?}"
            );
            let mut sweep = ctx();
            sweep.can_write = None;
            assert_eq!(count(item_type, &sweep), 1, "{item_type:?} in the sweep");
        }
    }

    /// Free holds one seat and one site and lacks tags, the API, sharing and the topology views.
    /// Dependencies are mapped on the topology, so their steps are left out with it.
    /// The questions are asked on Cloud only.
    #[test]
    fn a_setup_item_outside_the_plan_or_deployment_is_left_out() {
        let mut free = ctx();
        free.plan = get_free_plan();
        for item_type in [
            T::InviteTeam,
            T::MultiSite,
            T::ApiKeys,
            T::Share,
            T::Tags,
            T::ApplicationTags,
            T::OpenTopology,
            T::ExploreViews,
            T::DependencyTutorial,
        ] {
            assert_eq!(count(item_type, &free), 0, "{item_type:?}");
        }
        // The step the tutorial unlocks stays out too, once the tutorial is done.
        free.recorded.insert(T::DependencyTutorial);
        assert_eq!(count(T::Dependencies, &free), 0);
        let mut self_hosted = ctx();
        self_hosted.cloud = false;
        assert_eq!(count(T::AnswerReferralSource, &self_hosted), 0);
        assert_eq!(count(T::AnswerProfile, &self_hosted), 0);
    }

    /// Counts only what the user may review, one Issue per kind of row per site that has some.
    #[test]
    fn unconfirmed_rows_are_issues_per_site_the_user_can_write() {
        let mut ctx = ctx();
        assert_eq!(count(T::UnconfirmedHosts, &ctx), 0);
        let (hq, branch) = (ctx.sites[0].id, Uuid::new_v4());
        ctx.unconfirmed_hosts = vec![(hq, 3), (branch, 2)];
        ctx.unconfirmed_subnets = vec![(hq, 1)];
        let hosts: Vec<(Uuid, u64)> = items(T::UnconfirmedHosts, &ctx)
            .into_iter()
            .map(|item| match item {
                SetupItem::UnconfirmedHosts { site_id, count } => (site_id, count),
                other => panic!("unexpected item {other:?}"),
            })
            .collect();
        assert_eq!(hosts, [(hq, 3), (branch, 2)]);
        let read_only = without_write(ctx, Resource::Subnet);
        assert_eq!(count(T::UnconfirmedSubnets, &read_only), 0);
        assert_eq!(count(T::UnconfirmedHosts, &read_only), 2);
    }

    /// The sweep reports Issues and Open first steps only: a first step Locked or Done, and every
    /// later step, is never reported.
    #[test]
    fn only_issues_and_open_first_steps_alert_when_stuck() {
        let mut ctx = ctx();
        ctx.unconfirmed_hosts = vec![(ctx.sites[0].id, 2)];
        let alerting = |ctx: &SetupContext| -> Vec<T> {
            T::iter()
                .flat_map(|t| t.items(ctx))
                .filter(|e| e.alerts_when_stuck)
                .map(|e| T::from(&e.item))
                .collect()
        };
        assert_eq!(alerting(&ctx), [T::UnconfirmedHosts, T::InstallDaemon]);
        let mut ctx = recorded(ctx, &[T::InstallDaemon]);
        ctx.daemons = vec![daemon(ctx.sites[0].id, DaemonHealth::Healthy)];
        assert_eq!(alerting(&ctx), [T::UnconfirmedHosts, T::RunFirstScan]);
    }

    /// Types are declared Issues first, then Setup, which is the order the list is shown in.
    #[test]
    fn declaration_order_groups_types_by_tier() {
        let tiers: Vec<_> = T::iter().map(|t| t.tier()).collect();
        assert!(tiers.is_sorted(), "{tiers:?}");
    }

    #[test]
    fn a_deleted_credentials_warning_is_not_a_setup_issue() {
        let DiscoveryWarning::CredentialNeedsNewerDaemon {
            integration,
            gap,
            last_unsupported_daemon_version,
            ..
        } = needs_newer_daemon(Version::new(0, 18, 0))
        else {
            unreachable!()
        };
        let orphaned = DiscoveryWarning::CredentialNeedsNewerDaemon {
            integration,
            credential_id: None,
            gap,
            last_unsupported_daemon_version,
            daemon_version: None,
        };
        assert!(is_setup_issue_warning(&needs_newer_daemon(Version::new(
            0, 18, 0
        ))));
        assert!(!is_setup_issue_warning(&orphaned));
        assert!(is_setup_issue_warning(&time_limit()));
    }
}
