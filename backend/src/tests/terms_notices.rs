//! The terms-notice sweep against a real database: who is sent a terms
//! update, and that each user is sent it once.

use std::path::Path;

use chrono::Days;
use email_address::EmailAddress;
use governor::{DefaultDirectRateLimiter, RateLimiter};
use uuid::Uuid;

use super::self_hosted_license_emails::{campaign, email_service_for};
use super::self_hosted_licensing::{create_org, test_state_with_email_dir};
use super::user;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::{get_enterprise_plan, get_self_hosted_standard_plan};
use crate::server::config::{AppState, DeploymentType};
use crate::server::email::rate_limit;
use crate::server::email::transport::SES_MESSAGE_TAGS;
use crate::server::release_notes::terms::TermsCatalog;
use crate::server::release_notes::terms::tests::terms_file;
use crate::server::shared::services::traits::CrudService;
use crate::server::users::r#impl::base::User;

/// One update, written today and taking effect in 40 days.
fn catalog() -> TermsCatalog {
    let today = crate::server::shared::clock::now().date_naive();
    let effective = today + Days::new(40);
    TermsCatalog::from_files([terms_file(&effective.to_string(), &today.to_string())])
}

fn bulk() -> DefaultDirectRateLimiter {
    RateLimiter::direct(rate_limit::per_second(1000))
}

async fn create_user(state: &AppState, organization_id: Uuid, email: &str, verified: bool) -> User {
    let mut user = user(&organization_id);
    user.base.email = EmailAddress::new_unchecked(email);
    user.base.email_verified = verified;
    state
        .services
        .user_service
        .create(user, AuthenticatedEntity::System)
        .await
        .unwrap()
}

/// The terms notices the logging transport wrote, one per send.
fn notices(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "html"))
        .map(|path| std::fs::read_to_string(path).unwrap())
        .filter(|html| html.contains(&campaign("terms_update")))
        .collect()
}

/// A verified user is sent the update once, with the SES tags that record
/// who got which version; an unverified one is sent nothing; a second sweep
/// sends nothing more.
#[tokio::test]
async fn each_verified_user_is_sent_an_update_once() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(None).await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let verified = create_user(&state, org.id, "verified@example.test", true).await;
    create_user(&state, org.id, "unverified@example.test", false).await;

    let email = email_service_for(&state, dir.path(), DeploymentType::Cloud);
    let catalog = catalog();
    let today = crate::server::shared::clock::now().date_naive();
    for _ in 0..2 {
        email
            .send_terms_notices_from(&catalog, today, &bulk())
            .await
            .unwrap();
    }

    let sent = notices(dir.path());
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains("To: verified@example.test"));
    let tags = sent[0]
        .lines()
        .find(|line| line.starts_with(SES_MESSAGE_TAGS))
        .expect("a cloud send carries SES tags");
    assert!(tags.contains(&verified.id.to_string()));
    assert!(tags.contains("terms_version="));

    let reloaded = state
        .services
        .user_service
        .get_by_id(&verified.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        reloaded.base.terms_notice_version,
        Some(today + Days::new(40))
    );
}

/// Containers sweeping at the same moment send each user one notice.
#[tokio::test]
async fn concurrent_sweeps_send_once() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(None).await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    create_user(&state, org.id, "owner@example.test", true).await;

    let services: Vec<_> = (0..4)
        .map(|_| email_service_for(&state, dir.path(), DeploymentType::Cloud))
        .collect();
    let catalog = catalog();
    let today = crate::server::shared::clock::now().date_naive();
    let bulk = bulk();
    let results = futures::future::join_all(
        services
            .iter()
            .map(|service| service.send_terms_notices_from(&catalog, today, &bulk)),
    )
    .await;
    for result in results {
        result.unwrap();
    }

    assert_eq!(notices(dir.path()).len(), 1);
}

/// The org that bought a self-hosted licence is locked out of the cloud app
/// but still bound by the terms, so its users are sent the notice.
#[tokio::test]
async fn a_licensee_locked_out_of_the_cloud_app_is_sent_the_notice() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(None).await;
    let org = create_org(&state, get_self_hosted_standard_plan(), None).await;
    create_user(&state, org.id, "licensee@example.test", true).await;

    email_service_for(&state, dir.path(), DeploymentType::Cloud)
        .send_terms_notices_from(
            &catalog(),
            crate::server::shared::clock::now().date_naive(),
            &bulk(),
        )
        .await
        .unwrap();

    assert_eq!(notices(dir.path()).len(), 1);
}

/// A self-hosted server's users have no agreement with Scanopy LLC.
#[tokio::test]
async fn a_self_hosted_server_sends_no_notice() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(None).await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    create_user(&state, org.id, "admin@example.test", true).await;

    for deployment in [DeploymentType::Commercial, DeploymentType::Community] {
        email_service_for(&state, dir.path(), deployment)
            .send_terms_notices_from(
                &catalog(),
                crate::server::shared::clock::now().date_naive(),
                &bulk(),
            )
            .await
            .unwrap();
    }

    assert!(notices(dir.path()).is_empty());
}
