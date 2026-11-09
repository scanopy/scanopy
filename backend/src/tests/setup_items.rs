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
        types::{SetupItem, SetupItemDiscriminants, SetupItemTier},
    },
    shared::{
        services::{factory::ServiceFactory, traits::CrudService},
        storage::{factory::StorageFactory, traits::Storage},
        types::entities::EntitySource,
    },
    tags::r#impl::base::{Tag, TagBase},
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
    // Every feature, so a Suggestion is held back only by the condition under test.
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

/// Seeded starter tags and default credentials are `System`, so they leave the Tags and
/// InterfaceCredential suggestions up. A tag a person adds takes Tags down.
#[tokio::test]
async fn seeded_tags_and_credentials_leave_the_suggestions_up() {
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
    let seeded_tag = |source| Tag {
        id: Uuid::new_v4(),
        base: TagBase {
            name: format!("{source:?} tag"),
            organization_id: f.org.id,
            source,
            ..Default::default()
        },
        ..Default::default()
    };
    storage
        .tags
        .create(&seeded_tag(EntitySource::System))
        .await
        .unwrap();
    storage
        .credentials
        .create(&Credential::new(CredentialBase {
            organization_id: f.org.id,
            source: EntitySource::System,
            ..Default::default()
        }))
        .await
        .unwrap();

    let shown = types(&list(&services, &f).await);
    assert!(shown.contains(&SetupItemDiscriminants::Tags), "{shown:?}");
    assert!(
        shown.contains(&SetupItemDiscriminants::InterfaceCredential),
        "{shown:?}"
    );

    storage
        .tags
        .create(&seeded_tag(EntitySource::Manual))
        .await
        .unwrap();
    assert!(!types(&list(&services, &f).await).contains(&SetupItemDiscriminants::Tags));
}

/// The demo is a product tour of a finished setup: nothing to fix and no step left.
#[tokio::test]
async fn the_demo_org_has_no_issues_and_no_next_steps() {
    let (storage, services, _container) = test_services().await;
    let mut org = organization();
    org.base.setup = <SetupItemDiscriminants as strum::IntoEnumIterator>::iter()
        .filter(|t| t.tier() == SetupItemTier::NextStep)
        .collect();
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

    let ctx = services
        .setup_service
        .gather(&org, Some(&owner_on(&org, &site_ids)))
        .await
        .unwrap();
    let blocking: Vec<SetupItem> = SetupService::list(&ctx)
        .into_iter()
        .filter(|item| SetupItemDiscriminants::from(item).tier() != SetupItemTier::Suggestion)
        .collect();
    assert!(blocking.is_empty(), "{blocking:#?}");
}
