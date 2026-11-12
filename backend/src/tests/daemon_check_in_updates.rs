//! A daemon check-in that changes its version or health reaches the live-update stream, so an open
//! upgrade modal and Home see the upgrade without a reload. One that changes neither stays quiet.

use semver::{Prerelease, Version};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::TryRecvError;

use crate::daemon::runtime::state::DaemonStatus;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::daemons::r#impl::api::DaemonHealth;
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::daemons::r#impl::version::DaemonVersionPolicy;
use crate::server::live_updates::types::LiveDaemonUpdate;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::factory::StorageFactory;
use crate::server::shared::storage::traits::Storage;

use super::{daemon, host, organization, site, test_services, user};

/// A version the policy calls outdated: a pre-release of the one it recommends.
fn outdated_version() -> Version {
    let mut version = DaemonVersionPolicy::default().recommended;
    version.pre = Prerelease::new("rc.1").unwrap();
    version
}

fn health(daemon: &Daemon) -> DaemonHealth {
    let policy = DaemonVersionPolicy::default();
    DaemonHealth::of(&daemon.base, &policy.evaluate(daemon.base.version.as_ref()))
}

/// A daemon that has checked in before, on `version`.
async fn stored_daemon(
    storage: &StorageFactory,
    services: &ServiceFactory,
    version: Version,
) -> Daemon {
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let site_row = services
        .site_service
        .create(site(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let daemon_host = storage.hosts.create(&host(&site_row.id)).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();
    let mut row = daemon(&site_row.id, &daemon_host.id);
    row.base.user_id = owner.id;
    row.base.version = Some(version);
    storage.daemons.create(&row).await.unwrap();
    row
}

fn status_of(daemon: &Daemon, version: Version) -> DaemonStatus {
    DaemonStatus {
        url: None,
        name: daemon.base.name.clone(),
        mode: daemon.base.mode,
        version: Some(version),
        capabilities: Default::default(),
        interfaced_subnets: None,
        interfaces_fingerprint: None,
        ready_for_work: true,
        os: None,
    }
}

async fn reload(services: &ServiceFactory, daemon: &Daemon) -> Daemon {
    services
        .daemon_service
        .get_by_id(&daemon.id)
        .await
        .unwrap()
        .unwrap()
}

fn assert_announced(updates: &mut Receiver<LiveDaemonUpdate>, daemon: &Daemon) {
    assert_eq!(
        updates.try_recv().unwrap(),
        LiveDaemonUpdate {
            id: daemon.id,
            site_id: daemon.base.site_id,
        }
    );
}

#[tokio::test]
async fn an_upgraded_daemon_starting_up_is_announced_and_no_longer_outdated() {
    let (storage, services, _db) = test_services().await;
    let stored = stored_daemon(&storage, &services, outdated_version()).await;
    assert_eq!(health(&stored), DaemonHealth::Outdated);
    let mut updates = services.daemon_service.subscribe_live_daemon_updates();

    let recommended = DaemonVersionPolicy::default().recommended;
    services
        .daemon_service
        .process_startup(
            stored.id,
            recommended.clone(),
            None,
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let after = reload(&services, &stored).await;
    assert_eq!(after.base.version, Some(recommended));
    assert_ne!(health(&after), DaemonHealth::Outdated);
    assert_announced(&mut updates, &stored);
}

#[tokio::test]
async fn an_upgraded_daemon_reporting_status_is_announced_and_no_longer_outdated() {
    let (storage, services, _db) = test_services().await;
    let stored = stored_daemon(&storage, &services, outdated_version()).await;
    assert_eq!(health(&stored), DaemonHealth::Outdated);
    let mut updates = services.daemon_service.subscribe_live_daemon_updates();

    let recommended = DaemonVersionPolicy::default().recommended;
    services
        .daemon_service
        .process_status(
            stored.id,
            status_of(&stored, recommended.clone()),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let after = reload(&services, &stored).await;
    assert_eq!(after.base.version, Some(recommended));
    assert_ne!(health(&after), DaemonHealth::Outdated);
    assert_announced(&mut updates, &stored);
}

#[tokio::test]
async fn a_check_in_that_changes_nothing_is_not_announced() {
    let (storage, services, _db) = test_services().await;
    let recommended = DaemonVersionPolicy::default().recommended;
    let stored = stored_daemon(&storage, &services, recommended.clone()).await;
    let mut updates = services.daemon_service.subscribe_live_daemon_updates();

    services
        .daemon_service
        .process_status(
            stored.id,
            status_of(&stored, recommended.clone()),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    services
        .daemon_service
        .process_startup(stored.id, recommended, None, AuthenticatedEntity::System)
        .await
        .unwrap();

    assert_eq!(updates.try_recv(), Err(TryRecvError::Empty));
}
