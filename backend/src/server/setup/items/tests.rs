//! Each item type's condition, tested from a hand-built [`SetupContext`].

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
        failed_scans: Vec::new(),
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
        intents: Vec::new(),
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
/// one row between them, and the credential step, left out rather than locked, have their
/// own tests.
#[test]
fn a_setup_item_is_locked_then_open_then_done() {
    let cases = setup_cases();
    assert_eq!(
        cases.iter().map(|(t, ..)| *t).collect::<Vec<_>>(),
        T::iter()
            .filter(|t| t.tier() == SetupItemTier::Setup)
            .filter(|t| {
                !matches!(
                    t,
                    T::DependencyTutorial | T::Dependencies | T::InterfaceCredential
                )
            })
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

/// The credential step is left out until the first scan, so the list leads with daemon, scan
/// and topology. Then it is Open, and Done once recorded or once a manual credential exists,
/// which shows it done before the first scan too.
#[test]
fn the_credential_step_appears_after_the_first_scan() {
    assert_eq!(state(T::InterfaceCredential, &ctx()), None);

    let scanned = recorded(ctx(), &[T::RunFirstScan]);
    assert_eq!(
        state(T::InterfaceCredential, &scanned),
        Some(SetupItemState::Open)
    );
    assert_eq!(
        state(
            T::InterfaceCredential,
            &recorded(scanned, &[T::InterfaceCredential])
        ),
        Some(SetupItemState::Done)
    );

    let mut early = ctx();
    early.has_manual_interface_credential = true;
    assert_eq!(
        state(T::InterfaceCredential, &early),
        Some(SetupItemState::Done)
    );
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
        let scanned = recorded(ctx(), &[T::RunFirstScan]);
        assert_eq!(
            count(item_type, &without_write(scanned.clone(), resource)),
            0,
            "{item_type:?}"
        );
        let mut sweep = scanned;
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

fn failed_run(daemon: &SetupDaemon, reason: DiscoveryTerminalReason) -> FailedScanRun {
    FailedScanRun {
        daemon_id: daemon.id,
        scan_id: Uuid::new_v4(),
        scan_name: "Office LAN".to_string(),
        run_id: Uuid::new_v4(),
        reason,
        error: None,
        previous_run_failed: false,
        scheduled: false,
        next_run_full: false,
    }
}

/// Scans whose runs ended the same way on one daemon are one item, naming each scan. Another
/// reason is its own item, and a deleted daemon's scans raise none.
#[test]
fn failed_scans_group_by_daemon_and_reason() {
    let mut ctx = ctx();
    let d = daemon(ctx.sites[0].id, DaemonHealth::Healthy);
    ctx.daemons = vec![d.clone()];
    let gone = daemon(ctx.sites[0].id, DaemonHealth::Healthy);
    ctx.failed_scans = vec![
        failed_run(&d, DiscoveryTerminalReason::SessionSilent),
        failed_run(&d, DiscoveryTerminalReason::SessionSilent),
        failed_run(&d, DiscoveryTerminalReason::WatchdogTimeout),
        failed_run(&gone, DiscoveryTerminalReason::SessionSilent),
    ];
    let rows: Vec<(DiscoveryTerminalReason, usize)> = items(T::FailedScans, &ctx)
        .into_iter()
        .map(|item| match item {
            SetupItem::FailedScans { reason, scans, .. } => (reason, scans.len()),
            other => panic!("unexpected item {other:?}"),
        })
        .collect();
    assert_eq!(
        rows,
        [
            (DiscoveryTerminalReason::SessionSilent, 2),
            (DiscoveryTerminalReason::WatchdogTimeout, 1),
        ]
    );
}

/// A lost connection is the daemon's Issue while its health reports it. Once the daemon is
/// back, the scan that never finished is listed.
#[test]
fn a_lost_connection_is_one_row_owned_by_the_daemon_while_it_lasts() {
    for (reason, health) in itertools::iproduct!(
        [
            DiscoveryTerminalReason::DaemonUnreachable,
            DiscoveryTerminalReason::DaemonOffline,
            DiscoveryTerminalReason::StalledNoUpdates,
        ],
        [DaemonHealth::Unreachable, DaemonHealth::NotCheckingIn]
    ) {
        let mut ctx = ctx();
        let d = daemon(ctx.sites[0].id, health);
        ctx.daemons = vec![d.clone()];
        ctx.failed_scans = vec![failed_run(&d, reason)];
        assert_eq!(count(T::FailedScans, &ctx), 0, "{reason:?}");
        assert_eq!(count(T::DaemonHealth, &ctx), 1, "{reason:?}");

        ctx.daemons[0].health = DaemonHealth::Healthy;
        assert_eq!(
            count(T::FailedScans, &ctx),
            1,
            "{reason:?} after reconnecting"
        );
    }
    // A cause the connection doesn't explain is listed whatever the daemon's health.
    let mut ctx = ctx();
    let d = daemon(ctx.sites[0].id, DaemonHealth::Unreachable);
    ctx.daemons = vec![d.clone()];
    ctx.failed_scans = vec![failed_run(&d, DiscoveryTerminalReason::WatchdogTimeout)];
    assert_eq!(count(T::FailedScans, &ctx), 1);
}

/// A one-off the next run usually gets past waits for a second failure, unless no schedule
/// will run the scan again.
#[test]
fn a_one_off_failure_waits_for_a_repeat_on_a_scheduled_scan() {
    let mut ctx = ctx();
    let d = daemon(ctx.sites[0].id, DaemonHealth::Healthy);
    ctx.daemons = vec![d.clone()];
    let mut busy = failed_run(&d, DiscoveryTerminalReason::DaemonBusy);
    busy.scheduled = true;
    ctx.failed_scans = vec![busy];
    assert_eq!(count(T::FailedScans, &ctx), 0);

    ctx.failed_scans[0].previous_run_failed = true;
    assert_eq!(count(T::FailedScans, &ctx), 1);

    ctx.failed_scans[0].previous_run_failed = false;
    ctx.failed_scans[0].scheduled = false;
    assert_eq!(count(T::FailedScans, &ctx), 1);
}

fn listed(ctx: &SetupContext) -> Vec<T> {
    crate::server::setup::service::SetupService::list(ctx)
        .iter()
        .map(|entry| T::from(&entry.item))
        .collect()
}

/// An intent moves the steps it names to just after the first steps, and the list keeps
/// every entry and its lock. With no intents the list keeps declaration order.
#[test]
fn intents_move_their_steps_up_without_unlocking_them() {
    let base = recorded(ctx(), &[T::InstallDaemon, T::RunFirstScan]);
    let unordered = listed(&base);
    assert!(
        unordered
            .windows(2)
            .all(|pair| T::iter().position(|t| t == pair[0])
                <= T::iter().position(|t| t == pair[1])),
        "{unordered:?}"
    );

    for intent in SignupIntent::iter() {
        let mut with_intent = base.clone();
        with_intent.intents = vec![intent];
        let ordered = listed(&with_intent);

        let mut sorted_back = ordered.clone();
        sorted_back.sort_by_key(|t| T::iter().position(|d| d == *t));
        let mut expected = unordered.clone();
        expected.sort_by_key(|t| T::iter().position(|d| d == *t));
        assert_eq!(sorted_back, expected, "{intent:?} dropped or added an entry");

        let after_topology = ordered
            .iter()
            .position(|t| *t == T::OpenTopology)
            .map(|i| i + 1);
        let named: Vec<T> = ordered
            .iter()
            .copied()
            .filter(|t| intent.setup_items().contains(t))
            .collect();
        if let (Some(start), false) = (after_topology, named.is_empty()) {
            assert_eq!(ordered[start..start + named.len()], named[..], "{intent:?}");
        }
    }

    // The dependency tutorial moves up but stays locked until the topology is opened.
    let mut troubleshooting = base.clone();
    troubleshooting.intents = vec![SignupIntent::Troubleshooting];
    let entries = crate::server::setup::service::SetupService::list(&troubleshooting);
    let tutorial = entries
        .iter()
        .find(|e| T::from(&e.item) == T::DependencyTutorial)
        .expect("listed");
    assert_eq!(tutorial.state, SetupItemState::Locked);
}
