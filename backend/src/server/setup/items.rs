//! Each Setup item type's condition, in one exhaustive match over facts gathered once.
//!
//! [`SetupItemDiscriminants::items`] is pure: [`SetupContext`] is a plain struct
//! [`super::service::SetupService::gather`] fills through each entity's service, so every arm is
//! tested from a hand-built context.

use std::collections::{BTreeSet, HashSet};

use semver::Version;
use uuid::Uuid;

use super::types::{SetupItem, SetupItemDiscriminants};
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

/// Rows the Suggestions count.
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
    /// The types the org finished or dismissed (`organizations.setup`).
    pub recorded: HashSet<SetupItemDiscriminants>,
    pub daemons: Vec<SetupDaemon>,
    pub sites: Vec<SetupSite>,
    pub targetless_credentials: Vec<TargetlessCredential>,
    pub uncovered_integrations: Vec<UncoveredIntegration>,
    /// Each daemon's latest run of each discovery type, newest first.
    pub run_warnings: Vec<DaemonRunWarnings>,
    /// A connected daemon to run the first scan.
    pub first_scan_daemon_id: Option<Uuid>,
    pub unconfirmed_hosts: u64,
    pub unconfirmed_subnets: u64,
    pub counts: SetupCounts,
    pub has_manual_tag: bool,
    pub has_manual_application_tag: bool,
    pub has_manual_interface_credential: bool,
    pub plan: BillingPlan,
    /// Whether this is Scanopy Cloud. The two questions are asked only there.
    pub cloud: bool,
    /// The resources the user may write. `None` in the sweep, which has no user and lists no
    /// Suggestions.
    pub can_write: Option<BTreeSet<Resource>>,
}

impl SetupContext {
    fn has(&self, item_type: SetupItemDiscriminants) -> bool {
        self.recorded.contains(&item_type)
    }

    fn can_write(&self, resource: Resource) -> bool {
        self.can_write
            .as_ref()
            .is_some_and(|writable| writable.contains(&resource))
    }
}

impl SetupItemDiscriminants {
    /// This type's items for the org. Issues return one item per affected entity, Next steps and
    /// Suggestions zero or one. Nothing once the type is recorded as finished or dismissed.
    pub fn items(self, ctx: &SetupContext) -> Vec<SetupItem> {
        use SetupItemDiscriminants as T;
        if ctx.has(self) {
            return Vec::new();
        }
        let features = ctx.plan.features();
        let one = |condition: bool, item: SetupItem| if condition { vec![item] } else { vec![] };
        match self {
            T::DaemonHealth => ctx
                .daemons
                .iter()
                .filter(|d| d.health != DaemonHealth::Healthy)
                .map(|d| SetupItem::DaemonHealth {
                    daemon_id: d.id,
                    name: d.name.clone(),
                    site_id: d.site_id,
                    health: d.health,
                })
                .collect(),
            T::UncoveredIntegration => ctx
                .uncovered_integrations
                .iter()
                .map(|u| SetupItem::UncoveredIntegration {
                    integration: u.integration,
                    host_ids: u.host_ids.clone(),
                    site_ids: u.site_ids.clone(),
                })
                .collect(),
            T::TargetlessCredential => ctx
                .targetless_credentials
                .iter()
                .map(|c| SetupItem::TargetlessCredential {
                    credential_id: c.id,
                    name: c.name.clone(),
                    integration: c.integration,
                })
                .collect(),
            // Before the first connection, InstallDaemon already asks for a daemon.
            T::SiteWithoutDaemon if !ctx.has(T::InstallDaemon) => Vec::new(),
            T::SiteWithoutDaemon => ctx
                .sites
                .iter()
                .filter(|s| !ctx.daemons.iter().any(|d| d.site_id == s.id))
                .map(|s| SetupItem::SiteWithoutDaemon {
                    site_id: s.id,
                    name: s.name.clone(),
                })
                .collect(),
            T::RunWarnings => run_warning_items(ctx),
            // A daemon that never connected is an AwaitingConnection Issue instead.
            T::InstallDaemon => one(
                !ctx.daemons
                    .iter()
                    .any(|d| d.health == DaemonHealth::AwaitingConnection),
                SetupItem::InstallDaemon,
            ),
            T::RunFirstScan => match ctx.first_scan_daemon_id {
                Some(daemon_id) if ctx.has(T::InstallDaemon) => {
                    vec![SetupItem::RunFirstScan { daemon_id }]
                }
                _ => Vec::new(),
            },
            // The topology views are a plan feature, so the step waits for a plan with them.
            T::OpenTopology => one(
                ctx.has(T::RunFirstScan) && features.includes_topology(),
                SetupItem::OpenTopology,
            ),
            // Cloud only, one question at a time: the referral source first, then the profile.
            T::AnswerReferralSource => one(
                ctx.cloud && ctx.has(T::InstallDaemon),
                SetupItem::AnswerReferralSource,
            ),
            T::AnswerProfile => one(
                ctx.cloud && ctx.has(T::RunFirstScan) && ctx.has(T::AnswerReferralSource),
                SetupItem::AnswerProfile,
            ),
            T::ReviewUnconfirmed => {
                let hosts = if ctx.can_write(Resource::Host) {
                    ctx.unconfirmed_hosts
                } else {
                    0
                };
                let subnets = if ctx.can_write(Resource::Subnet) {
                    ctx.unconfirmed_subnets
                } else {
                    0
                };
                one(
                    hosts + subnets > 0,
                    SetupItem::ReviewUnconfirmed { hosts, subnets },
                )
            }
            T::ExplorePerspectives => one(ctx.has(T::OpenTopology), SetupItem::ExplorePerspectives),
            T::ApplicationTags => one(
                ctx.has(T::OpenTopology)
                    && features.tagging
                    && !ctx.has_manual_application_tag
                    && ctx.can_write(Resource::Tag),
                SetupItem::ApplicationTags,
            ),
            T::Dependencies => one(
                ctx.has(T::OpenTopology)
                    && ctx.counts.dependencies == 0
                    && ctx.can_write(Resource::Dependency),
                SetupItem::Dependencies,
            ),
            T::InterfaceCredential => one(
                ctx.has(T::RunFirstScan)
                    && !ctx.has_manual_interface_credential
                    && ctx.can_write(Resource::Credential),
                SetupItem::InterfaceCredential,
            ),
            T::Tags => one(
                ctx.has(T::RunFirstScan)
                    && features.tagging
                    && !ctx.has_manual_tag
                    && ctx.can_write(Resource::Tag),
                SetupItem::Tags,
            ),
            T::InviteTeam => one(
                ctx.plan.can_invite_users()
                    && ctx.counts.members == 1
                    && ctx.counts.pending_invites == 0
                    && ctx.can_write(Resource::Member),
                SetupItem::InviteTeam,
            ),
            T::MultiSite => one(
                plan_allows_several_sites(&ctx.plan)
                    && ctx.sites.len() == 1
                    && ctx.can_write(Resource::NewSite),
                SetupItem::MultiSite,
            ),
            T::ApiKeys => one(
                features.api_access && ctx.has(T::RunFirstScan) && ctx.counts.user_api_keys == 0,
                SetupItem::ApiKeys,
            ),
            T::Share => one(
                features.sharing
                    && ctx.has(T::OpenTopology)
                    && ctx.counts.shares == 0
                    && ctx.can_write(Resource::Share),
                SetupItem::Share,
            ),
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
            unconfirmed_hosts: 0,
            unconfirmed_subnets: 0,
            counts: SetupCounts {
                members: 1,
                ..Default::default()
            },
            has_manual_tag: false,
            has_manual_application_tag: false,
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
        let items = T::DaemonHealth.items(&ctx);
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
            T::SiteWithoutDaemon.items(&ctx).as_slice(),
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
        let items = T::RunWarnings.items(&ctx);
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
    /// steps aside for it.
    #[test]
    fn a_never_connected_daemon_replaces_install_daemon() {
        let mut ctx = ctx();
        assert_eq!(count(T::InstallDaemon, &ctx), 1);

        ctx.daemons = vec![daemon(ctx.sites[0].id, DaemonHealth::AwaitingConnection)];
        assert_eq!(count(T::InstallDaemon, &ctx), 0);
        assert!(matches!(
            T::DaemonHealth.items(&ctx).as_slice(),
            [SetupItem::DaemonHealth {
                health: DaemonHealth::AwaitingConnection,
                ..
            }]
        ));

        ctx.daemons.clear();
        let ctx = recorded(ctx, &[T::InstallDaemon]);
        assert_eq!(count(T::InstallDaemon, &ctx), 0);
    }

    #[test]
    fn run_first_scan_follows_the_first_connection_and_names_the_daemon() {
        let mut ctx = ctx();
        let daemon_id = Uuid::new_v4();
        ctx.first_scan_daemon_id = Some(daemon_id);
        assert_eq!(count(T::RunFirstScan, &ctx), 0);

        let ctx = recorded(ctx, &[T::InstallDaemon]);
        assert!(matches!(
            T::RunFirstScan.items(&ctx).as_slice(),
            [SetupItem::RunFirstScan { daemon_id: d }] if *d == daemon_id
        ));
        assert_eq!(
            count(T::RunFirstScan, &recorded(ctx, &[T::RunFirstScan])),
            0
        );
    }

    #[test]
    fn open_topology_follows_the_first_scan_on_a_plan_with_topology() {
        assert_eq!(count(T::OpenTopology, &ctx()), 0);
        let ctx = recorded(ctx(), &[T::RunFirstScan]);
        assert_eq!(count(T::OpenTopology, &ctx), 1);

        let mut free = ctx.clone();
        free.plan = get_free_plan();
        assert_eq!(count(T::OpenTopology, &free), 0);
    }

    /// The referral question follows the first daemon, and the profile question follows both the
    /// first scan and the referral answer, so one shows at a time.
    #[test]
    fn the_questions_follow_the_daemon_the_first_scan_and_each_other() {
        assert_eq!(count(T::AnswerReferralSource, &ctx()), 0);
        assert_eq!(count(T::AnswerProfile, &ctx()), 0);
        let after_daemon = recorded(ctx(), &[T::InstallDaemon]);
        assert_eq!(count(T::AnswerReferralSource, &after_daemon), 1);
        assert_eq!(count(T::AnswerProfile, &after_daemon), 0);
        let after_scan = recorded(after_daemon, &[T::RunFirstScan]);
        assert_eq!(count(T::AnswerProfile, &after_scan), 0);
        let answered = recorded(after_scan, &[T::AnswerReferralSource]);
        assert_eq!(count(T::AnswerProfile, &answered), 1);
    }

    #[test]
    fn a_self_hosted_server_asks_neither_question() {
        let mut self_hosted = recorded(
            ctx(),
            &[T::InstallDaemon, T::RunFirstScan, T::AnswerReferralSource],
        );
        self_hosted.cloud = false;
        assert_eq!(count(T::AnswerProfile, &self_hosted), 0);
        self_hosted.recorded.remove(&T::AnswerReferralSource);
        assert_eq!(count(T::AnswerReferralSource, &self_hosted), 0);
    }

    /// Counts only what the user may review.
    #[test]
    fn unconfirmed_rows_count_only_what_the_user_can_write() {
        let mut ctx = ctx();
        assert_eq!(count(T::ReviewUnconfirmed, &ctx), 0);
        ctx.unconfirmed_hosts = 3;
        ctx.unconfirmed_subnets = 1;
        let ctx = without_write(ctx, Resource::Subnet);
        assert!(matches!(
            T::ReviewUnconfirmed.items(&ctx).as_slice(),
            [SetupItem::ReviewUnconfirmed {
                hosts: 3,
                subnets: 0
            }]
        ));
        assert_eq!(
            count(T::ReviewUnconfirmed, &without_write(ctx, Resource::Host)),
            0
        );
    }

    /// Each Suggestion appears once its prerequisite is recorded, the user may act on it and the
    /// org hasn't already done it, and is gone once dismissed.
    #[test]
    fn suggestions_need_their_step_their_permission_and_no_prior_use() {
        let after_topology = recorded(ctx(), &[T::InstallDaemon, T::RunFirstScan, T::OpenTopology]);
        // (type, the resource it needs, a context where the org has already done it)
        let mut done_tags = after_topology.clone();
        done_tags.has_manual_tag = true;
        let mut done_application_tags = after_topology.clone();
        done_application_tags.has_manual_application_tag = true;
        let mut done_dependencies = after_topology.clone();
        done_dependencies.counts.dependencies = 1;
        let mut done_interface = after_topology.clone();
        done_interface.has_manual_interface_credential = true;
        let mut done_invite = after_topology.clone();
        done_invite.counts.pending_invites = 1;
        let mut done_sites = after_topology.clone();
        done_sites.sites.push(site("Branch"));
        let mut done_api_keys = after_topology.clone();
        done_api_keys.counts.user_api_keys = 1;
        let mut done_shares = after_topology.clone();
        done_shares.counts.shares = 1;
        let cases = [
            (
                T::ApplicationTags,
                Some(Resource::Tag),
                done_application_tags,
            ),
            (
                T::Dependencies,
                Some(Resource::Dependency),
                done_dependencies,
            ),
            (
                T::InterfaceCredential,
                Some(Resource::Credential),
                done_interface,
            ),
            (T::Tags, Some(Resource::Tag), done_tags),
            (T::InviteTeam, Some(Resource::Member), done_invite),
            (T::MultiSite, Some(Resource::NewSite), done_sites),
            (T::ApiKeys, None, done_api_keys),
            (T::Share, Some(Resource::Share), done_shares),
        ];
        for (item_type, resource, done) in cases {
            assert_eq!(count(item_type, &after_topology), 1, "{item_type:?}");
            // Inviting and adding sites need no setup step first.
            if !matches!(item_type, T::InviteTeam | T::MultiSite) {
                assert_eq!(count(item_type, &ctx()), 0, "{item_type:?} before setup");
            }
            assert_eq!(count(item_type, &done), 0, "{item_type:?} already done");
            if let Some(resource) = resource {
                let read_only = without_write(after_topology.clone(), resource);
                assert_eq!(
                    count(item_type, &read_only),
                    0,
                    "{item_type:?} without write"
                );
            }
            let dismissed = recorded(after_topology.clone(), &[item_type]);
            assert_eq!(count(item_type, &dismissed), 0, "{item_type:?} dismissed");
        }
        assert_eq!(count(T::ExplorePerspectives, &after_topology), 1);
        assert_eq!(count(T::ExplorePerspectives, &ctx()), 0);
    }

    /// Free holds one seat and one site and lacks tags, the API and sharing, so it suggests none of
    /// them.
    #[test]
    fn suggestions_for_features_outside_the_plan_stay_hidden() {
        let mut free = recorded(ctx(), &[T::InstallDaemon, T::RunFirstScan, T::OpenTopology]);
        free.plan = get_free_plan();
        for item_type in [
            T::InviteTeam,
            T::MultiSite,
            T::ApiKeys,
            T::Share,
            T::Tags,
            T::ApplicationTags,
        ] {
            assert_eq!(count(item_type, &free), 0, "{item_type:?}");
        }
    }

    /// Types are declared Issues first, then Next steps, then Suggestions, which is the order the
    /// list is shown in.
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
