//! `OrganizationService::record_setup`: the caller that adds an item type is the one that
//! publishes its event, however many requests reach it together, and what reacts to that event.

use uuid::Uuid;

use super::self_hosted_license_emails::create_owner;
use super::self_hosted_licensing::{create_org, test_state_with_email_dir};
use super::{daemon, organization, site, test_services, user};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    billing::plans::get_free_plan,
    setup::types::{SetupItemDiscriminants, SetupOperation},
    shared::{services::traits::CrudService, storage::traits::Storage},
};

fn install_daemon(daemon_id: Uuid) -> SetupOperation {
    SetupOperation::Completed {
        item_type: SetupItemDiscriminants::InstallDaemon,
        subject_id: Some(daemon_id),
    }
}

#[tokio::test]
async fn concurrent_records_of_one_item_add_it_once() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();

    let organizations = &services.organization_service;
    let dismissed = || SetupOperation::Dismissed {
        item_type: SetupItemDiscriminants::Tags,
    };
    let (a, b) = tokio::join!(
        organizations.record_setup(org.id, dismissed(), AuthenticatedEntity::System),
        organizations.record_setup(org.id, dismissed(), AuthenticatedEntity::System),
    );
    let added = [a.unwrap(), b.unwrap()];
    assert_eq!(added.iter().filter(|added| **added).count(), 1);

    let stored = organizations.get_by_id(&org.id).await.unwrap().unwrap();
    assert_eq!(stored.base.setup, vec![SetupItemDiscriminants::Tags]);
}

/// The first daemon connecting sends the discovery guide, naming the daemon and its site, and a
/// second connection sends nothing.
#[tokio::test]
async fn the_first_daemon_connection_sends_the_discovery_guide_once() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(Some(dir.path().to_path_buf())).await;
    let org = create_org(&state, get_free_plan(), None).await;
    create_owner(&state, org.id).await;
    let mut site = site(&org.id);
    site.base.name = "Head office".to_string();
    let site = state
        .services
        .site_service
        .create(site, AuthenticatedEntity::System)
        .await
        .unwrap();
    let owner = state
        .services
        .user_service
        .get_organization_owners(&org.id)
        .await
        .unwrap()
        .remove(0);
    let mut daemon = daemon(&site.id, &Uuid::new_v4());
    daemon.base.name = "edge-01".to_string();
    daemon.base.user_id = owner.id;
    let daemon = state
        .services
        .daemon_service
        .storage()
        .create(&daemon)
        .await
        .unwrap();

    let organizations = &state.services.organization_service;
    for _ in 0..2 {
        organizations
            .record_setup(
                org.id,
                install_daemon(daemon.id),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap();
    }
    state.services.event_bus.flush().await;

    let guides: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        // The logging transport writes each email twice, as HTML and as text; count the HTML.
        .filter(|email| email.starts_with("<!--") && email.contains("utm_campaign=discovery_guide"))
        .collect();
    assert_eq!(guides.len(), 1, "{guides:?}");
    assert!(guides[0].contains("edge-01"), "{}", guides[0]);
    assert!(guides[0].contains("Head office"), "{}", guides[0]);
}

/// A daemon created for the org but never started (the integrated daemon a signup seeds is
/// provisioned this way) doesn't count as connected, and its first check-in does.
#[tokio::test]
async fn a_daemon_that_never_connected_is_not_connected() {
    let (storage, services, _container) = test_services().await;
    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let site = site(&org.id);
    storage.sites.create(&site).await.unwrap();
    let daemons = &services.daemon_service;

    assert!(!daemons.org_has_connected_daemon(org.id).await.unwrap());

    let owner = storage.users.create(&user(&org.id)).await.unwrap();
    let mut provisioned = daemon(&site.id, &Uuid::new_v4());
    provisioned.base.last_seen = None;
    provisioned.base.user_id = owner.id;
    let mut provisioned = storage.daemons.create(&provisioned).await.unwrap();
    assert!(!daemons.org_has_connected_daemon(org.id).await.unwrap());

    provisioned.base.last_seen = Some(crate::server::shared::clock::now());
    storage.daemons.update(&mut provisioned).await.unwrap();
    assert!(daemons.org_has_connected_daemon(org.id).await.unwrap());

    // Another org's connected daemon doesn't count.
    let other = organization();
    storage.organizations.create(&other).await.unwrap();
    assert!(!daemons.org_has_connected_daemon(other.id).await.unwrap());
}
