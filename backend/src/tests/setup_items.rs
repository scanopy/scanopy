//! `SetupService`: the Setup list read back from stored rows, for the conditions that live in
//! storage rather than in the pure arms (`setup/items.rs`).

use chrono::Duration;
use email_address::EmailAddress;
use semver::Version;
use uuid::Uuid;

use super::{daemon, host, organization, site, test_services, user};
use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    billing::plans::get_enterprise_plan,
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        capability::CapabilityGap,
        mapping::CredentialQueryPayloadDiscriminants,
    },
    daemons::r#impl::{api::DiscoveryUpdatePayload, base::Daemon},
    discovery::r#impl::types::DiscoveryType,
    organizations::{demo_data::DemoData, demo_seed::insert_demo_data, r#impl::base::Organization},
    setup::{
        service::SetupService,
        types::{SetupItem, SetupItemDiscriminants, SetupItemState, SetupItemTier},
    },
    shared::{
        entities::EntityDiscriminants,
        services::{factory::ServiceFactory, traits::CrudService},
        storage::{factory::StorageFactory, traits::Storage},
        types::entities::EntitySource,
    },
    tags::{
        groups::base::{TagGroup, TagGroupBase, TagGroupType},
        r#impl::base::{Tag, TagBase},
    },
    users::r#impl::permissions::UserOrgPermissions,
};

fn owner_on(org: &Organization, site_ids: &[Uuid]) -> AuthenticatedEntity {
    AuthenticatedEntity::User {
        user_id: Uuid::new_v4(),
        organization_id: org.id,
        permissions: UserOrgPermissions::Owner,
        access: crate::tests::access_on(UserOrgPermissions::Owner, site_ids),
        email: EmailAddress::new_unchecked("owner@example.test"),
        email_verified: true,
        terms_accepted_at: None,
    }
}

/// An org with `recorded` finished, one site, and a connected daemon on the current version.
struct Fixture {
    org: Organization,
    site_id: Uuid,
    host_id: Uuid,
    daemon: Daemon,
}

async fn fixture(
    storage: &StorageFactory,
    services: &ServiceFactory,
    recorded: &[SetupItemDiscriminants],
) -> Fixture {
    let mut org = organization();
    org.base.setup = recorded.to_vec();
    // Every feature, so a Setup item is held back only by the condition under test.
    org.base.plan = get_enterprise_plan();
    storage.organizations.create(&org).await.unwrap();
    let site_row = services
        .site_service
        .create(site(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let h = storage.hosts.create(&host(&site_row.id)).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();
    let mut d = daemon(&site_row.id, &h.id);
    d.base.user_id = owner.id;
    d.base.version = Some(Version::parse(env!("CARGO_PKG_VERSION")).unwrap());
    storage.daemons.create(&d).await.unwrap();
    Fixture {
        org,
        site_id: site_row.id,
        host_id: h.id,
        daemon: d,
    }
}

async fn list(services: &ServiceFactory, f: &Fixture) -> Vec<SetupItem> {
    let ctx = services
        .setup_service
        .gather(&f.org, Some(&owner_on(&f.org, &[f.site_id])))
        .await
        .unwrap();
    SetupService::list(&ctx)
        .into_iter()
        .map(|e| e.item)
        .collect()
}

fn types(items: &[SetupItem]) -> Vec<SetupItemDiscriminants> {
    items.iter().map(SetupItemDiscriminants::from).collect()
}

/// Finish a session of `discovery_type` on the fixture's daemon carrying `warnings`.
async fn finish_run(
    services: &ServiceFactory,
    f: &Fixture,
    discovery_type: DiscoveryType,
    warnings: Vec<DiscoveryWarning>,
) {
    let mut update =
        DiscoveryUpdatePayload::new(Uuid::new_v4(), f.daemon.id, f.site_id, discovery_type, None);
    update.phase = DiscoveryPhase::Complete;
    update.progress = 100;
    update.started_at = Some(crate::server::shared::clock::now() - Duration::minutes(1));
    update.finished_at = Some(crate::server::shared::clock::now());
    update.warnings = warnings;
    services.discovery_service.update_session(update).await;
}

fn self_report(f: &Fixture) -> DiscoveryType {
    DiscoveryType::SelfReport { host_id: f.host_id }
}

fn network() -> DiscoveryType {
    DiscoveryType::Network {
        subnet_ids: None,
        host_naming_fallback: Default::default(),
        snmp_credentials: Default::default(),
    }
}

fn time_limit() -> DiscoveryWarning {
    DiscoveryWarning::ScanTimeLimit {
        hours: 4,
        hosts_not_scanned: 12,
    }
}

fn run_warning_codes(items: &[SetupItem]) -> usize {
    items
        .iter()
        .filter(|i| matches!(i, SetupItem::RunWarnings { .. }))
        .count()
}

#[tokio::test]
async fn a_credential_with_no_target_is_an_issue_until_assigned_to_a_site() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(&storage, &services, &[]).await;
    let credential = storage
        .credentials
        .create(&Credential::new(CredentialBase {
            organization_id: f.org.id,
            name: "Core switch".to_string(),
            source: EntitySource::Manual,
            ..Default::default()
        }))
        .await
        .unwrap();

    assert!(list(&services, &f).await.iter().any(|item| matches!(
        item,
        SetupItem::TargetlessCredential { credential_id, .. } if *credential_id == credential.id
    )));

    services
        .credential_service
        .create_site_credentials(&[(f.site_id, credential.id)])
        .await
        .unwrap();
    assert!(
        !types(&list(&services, &f).await).contains(&SetupItemDiscriminants::TargetlessCredential)
    );
}

/// The latest run of each type holds the daemon's warnings: a clean newer run of the same type
/// clears them, and a run of another type leaves them.
#[tokio::test]
async fn a_warning_clears_when_the_next_run_of_its_type_comes_back_clean() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(&storage, &services, &[]).await;

    finish_run(&services, &f, self_report(&f), vec![time_limit()]).await;
    assert_eq!(run_warning_codes(&list(&services, &f).await), 1);

    finish_run(&services, &f, network(), Vec::new()).await;
    assert_eq!(run_warning_codes(&list(&services, &f).await), 1);

    finish_run(&services, &f, self_report(&f), Vec::new()).await;
    assert_eq!(run_warning_codes(&list(&services, &f).await), 0);
}

/// A credential withheld from an old daemon stops being an issue once the daemon is upgraded,
/// before another scan runs.
#[tokio::test]
async fn a_withheld_credential_clears_when_the_daemon_is_upgraded() {
    let (storage, services, _container) = test_services().await;
    let mut f = fixture(&storage, &services, &[]).await;
    let credential = storage
        .credentials
        .create(&Credential::new(CredentialBase {
            organization_id: f.org.id,
            source: EntitySource::Manual,
            ..Default::default()
        }))
        .await
        .unwrap();
    services
        .credential_service
        .create_site_credentials(&[(f.site_id, credential.id)])
        .await
        .unwrap();
    let current = f.daemon.base.version.clone().unwrap();
    let withheld = DiscoveryWarning::CredentialNeedsNewerDaemon {
        integration: CredentialQueryPayloadDiscriminants::Snmp,
        credential_id: Some(credential.id),
        gap: CapabilityGap::ParsesCredentialType,
        last_unsupported_daemon_version: current.clone(),
        daemon_version: Some(current.clone()),
    };
    finish_run(&services, &f, self_report(&f), vec![withheld]).await;
    assert_eq!(run_warning_codes(&list(&services, &f).await), 1);

    f.daemon.base.version = Some(Version::new(
        current.major,
        current.minor,
        current.patch + 1,
    ));
    storage.daemons.update(&mut f.daemon).await.unwrap();
    assert_eq!(run_warning_codes(&list(&services, &f).await), 0);
}

/// The state of the one entry of a Setup type in the list `owner` reads, recording what the
/// org's rows show done as the dashboard does.
async fn setup_state(
    services: &ServiceFactory,
    f: &Fixture,
    item_type: SetupItemDiscriminants,
) -> Option<SetupItemState> {
    let org = services
        .organization_service
        .get_by_id(&f.org.id)
        .await
        .unwrap()
        .unwrap();
    services
        .setup_service
        .for_user(&org, &owner_on(&org, &[f.site_id]))
        .await
        .unwrap()
        .into_iter()
        .find(|e| SetupItemDiscriminants::from(&e.item) == item_type)
        .map(|e| e.state)
}

/// A seeded tag nobody assigned leaves "Assign your first tag" open. Assigning one marks it done,
/// and removing the tag again leaves it done.
#[tokio::test]
async fn assigning_a_tag_completes_tags_for_good() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(
        &storage,
        &services,
        &[
            SetupItemDiscriminants::InstallDaemon,
            SetupItemDiscriminants::RunFirstScan,
        ],
    )
    .await;
    let tag = Tag {
        id: Uuid::new_v4(),
        base: TagBase {
            name: "Production".to_string(),
            organization_id: f.org.id,
            source: EntitySource::System,
            ..Default::default()
        },
        ..Default::default()
    };
    storage.tags.create(&tag).await.unwrap();
    assert_eq!(
        setup_state(&services, &f, SetupItemDiscriminants::Tags).await,
        Some(SetupItemState::Open)
    );

    services
        .entity_tag_service
        .add_tag(f.host_id, EntityDiscriminants::Host, tag.id, f.org.id)
        .await
        .unwrap();
    assert_eq!(
        setup_state(&services, &f, SetupItemDiscriminants::Tags).await,
        Some(SetupItemState::Done)
    );

    services
        .entity_tag_service
        .remove_tag(f.host_id, EntityDiscriminants::Host, tag.id)
        .await
        .unwrap();
    assert_eq!(
        setup_state(&services, &f, SetupItemDiscriminants::Tags).await,
        Some(SetupItemState::Done)
    );
}

/// An application tag on a host completes "Assign an application tag", as one on a service does:
/// the Application view groups by either.
#[tokio::test]
async fn an_application_tag_on_a_host_completes_application_tags() {
    let (storage, services, _container) = test_services().await;
    let f = fixture(
        &storage,
        &services,
        &[
            SetupItemDiscriminants::InstallDaemon,
            SetupItemDiscriminants::RunFirstScan,
            SetupItemDiscriminants::OpenTopology,
        ],
    )
    .await;
    let now = crate::server::shared::clock::now();
    let group = TagGroup {
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: TagGroupBase::new(f.org.id, "Application".into(), TagGroupType::Application),
    };
    storage.tag_groups.create(&group).await.unwrap();
    let app_tag = Tag {
        id: Uuid::new_v4(),
        base: TagBase {
            name: "Billing".to_string(),
            organization_id: f.org.id,
            tag_group_id: Some(group.id),
            ..Default::default()
        },
        ..Default::default()
    };
    storage.tags.create(&app_tag).await.unwrap();
    assert_eq!(
        setup_state(&services, &f, SetupItemDiscriminants::ApplicationTags).await,
        Some(SetupItemState::Open)
    );

    services
        .entity_tag_service
        .add_tag(f.host_id, EntityDiscriminants::Host, app_tag.id, f.org.id)
        .await
        .unwrap();
    assert_eq!(
        setup_state(&services, &f, SetupItemDiscriminants::ApplicationTags).await,
        Some(SetupItemState::Done)
    );
}

/// The demo is a product tour of a set-up network: nothing to fix. Its unconfirmed hosts and
/// subnet are there to show the review flow, so they are its only Issues.
#[tokio::test]
async fn the_demo_orgs_only_issues_are_its_unconfirmed_rows() {
    let (storage, services, _container) = test_services().await;
    let mut org = organization();
    org.base.setup = vec![
        SetupItemDiscriminants::InstallDaemon,
        SetupItemDiscriminants::RunFirstScan,
        SetupItemDiscriminants::OpenTopology,
    ];
    storage.organizations.create(&org).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();
    let demo_data = DemoData::generate(org.id, owner.id, org.base.use_case);
    let site_ids: Vec<Uuid> = demo_data.sites.iter().map(|s| s.id).collect();
    insert_demo_data(
        &services,
        demo_data,
        org.id,
        owner.id,
        AuthenticatedEntity::System,
    )
    .await
    .unwrap();

    let issues: Vec<SetupItem> = services
        .setup_service
        .for_user(&org, &owner_on(&org, &site_ids))
        .await
        .unwrap()
        .into_iter()
        .filter(|e| e.tier == SetupItemTier::Issue)
        .map(|e| e.item)
        .filter(|item| {
            !matches!(
                item,
                SetupItem::UnconfirmedHosts { .. } | SetupItem::UnconfirmedSubnets { .. }
            )
        })
        .collect();
    assert!(issues.is_empty(), "{issues:#?}");
}

/// The trial sweep reports a step left open a day on a trialing org, once, and leaves an org
/// that isn't trialing alone.
#[tokio::test]
async fn the_trial_sweep_reports_a_trialing_orgs_open_step_once() {
    use crate::server::billing::types::base::PlanStatus;
    use crate::server::setup::types::SetupItemKey;

    let (storage, services, _container) = test_services().await;
    let start = crate::server::shared::clock::now();
    let mut orgs = Vec::new();
    for status in [PlanStatus::Trialing, PlanStatus::Active] {
        let mut org = organization();
        org.base.plan = get_enterprise_plan();
        org.base.plan_status = Some(status);
        org.base.trial_end_date = Some(start + Duration::days(10));
        storage.organizations.create(&org).await.unwrap();
        orgs.push(org.id);
    }
    let notifications = |id: Uuid| {
        let organizations = services.organization_service.clone();
        async move {
            organizations
                .get_by_id(&id)
                .await
                .unwrap()
                .unwrap()
                .base
                .notifications
        }
    };
    let install_daemon = SetupItemKey::Org {
        item_type: SetupItemDiscriminants::InstallDaemon,
    };

    let setup = &services.setup_service;
    setup.sweep_trials(start).await.unwrap();
    setup
        .sweep_trials(start + Duration::hours(24))
        .await
        .unwrap();
    setup
        .sweep_trials(start + Duration::hours(25))
        .await
        .unwrap();

    let trialing = notifications(orgs[0]).await;
    assert_eq!(trialing.setup_items_reported, vec![install_daemon.clone()]);
    assert!(
        trialing
            .setup_items_open
            .iter()
            .any(|open| open.key == install_daemon && open.open_since == start)
    );
    let active = notifications(orgs[1]).await;
    assert!(active.setup_items_open.is_empty());
    assert!(active.setup_items_reported.is_empty());
}
