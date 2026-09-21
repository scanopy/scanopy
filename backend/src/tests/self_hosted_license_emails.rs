//! Which emails an organization on a self-hosted plan receives from the cloud
//! app, and who may reach the license key endpoints.
//!
//! The same `SelfHostedStandard` / `SelfHostedPlus` plan sits on two kinds of
//! org: the cloud org that bought the license (locked out of the app, nothing
//! scanned there) and the org on the customer's own server (`plan_for_license`),
//! where daemon and limit emails are the ones that matter. Several tests here
//! run both deployments side by side for that reason.

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::{self, Next};
use axum::routing::{get, post};
use chrono::Utc;
use email_address::EmailAddress;
use uuid::Uuid;

use super::self_hosted_licensing::{create_org, reload, test_state_with_email_dir};
use super::{daemon, network, user};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::{get_enterprise_plan, get_self_hosted_standard_plan};
use crate::server::billing::types::base::{BillingPlan, PlanConfig};
use crate::server::config::{AppState, DeploymentType};
use crate::server::email::EmailService;
use crate::server::email::logging::LoggingEmailProvider;
use crate::server::license::handlers::{
    create_license_key, get_current_license_key, rotate_license_key,
};
use crate::server::license::types::LicenseKeyType;
use crate::server::shared::events::traits::{Event, OrgScope, Subscriber};
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::services::traits::CrudService;
use crate::server::users::r#impl::permissions::UserOrgPermissions;

async fn create_owner(state: &AppState, organization_id: Uuid) {
    let mut owner = user(&organization_id);
    owner.base.email = EmailAddress::new_unchecked("owner@example.test");
    owner.base.permissions = UserOrgPermissions::Owner;
    owner.base.email_verified = true;
    state
        .services
        .user_service
        .create(owner, AuthenticatedEntity::System)
        .await
        .unwrap();
}

/// Everything the logging transport wrote, joined. Tests match on the
/// `utm_campaign` slug in the rendered links, never on copy.
fn sent(dir: &Path) -> String {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

fn campaign(slug: &str) -> String {
    format!("utm_campaign={slug}")
}

/// An email service over the state's own services, writing to `dir`, for a
/// chosen deployment. The state's own service reads as a self-hosted
/// deployment because the test config has no Stripe secret.
fn email_service_for(
    state: &AppState,
    dir: &Path,
    deployment_type: DeploymentType,
) -> EmailService {
    let services = &state.services;
    EmailService::new(
        Box::new(LoggingEmailProvider::new(Some(dir.to_path_buf()))),
        services.user_service.clone(),
        services.organization_service.clone(),
        services.host_service.clone(),
        services.network_service.clone(),
        services.service_service.clone(),
        services.daemon_service.clone(),
        "https://app.example.test".to_string(),
        deployment_type,
    )
}

async fn handle_billing(state: &AppState, organization_id: Uuid, operation: BillingOperation) {
    state
        .services
        .email_service
        .clone()
        .unwrap()
        .handle(vec![Event::new(
            OrgScope { organization_id },
            operation,
            AuthenticatedEntity::System,
        )])
        .await
        .unwrap();
}

#[tokio::test]
async fn a_self_hosted_trial_ending_gets_the_license_email_and_no_usage_recap() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(Some(dir.path().to_path_buf())).await;
    let org = create_org(&state, get_self_hosted_standard_plan(), None).await;
    create_owner(&state, org.id).await;

    handle_billing(
        &state,
        org.id,
        BillingOperation::TrialWillEnd {
            plan: get_self_hosted_standard_plan(),
            has_payment_method: false,
        },
    )
    .await;

    let sent = sent(dir.path());
    assert!(
        sent.contains(&campaign("self_hosted_trial_ending_no_payment")),
        "{sent}"
    );
    assert!(!sent.contains(&campaign("trial_ending_")), "{sent}");
}

#[tokio::test]
async fn a_cloud_trial_ending_still_gets_the_usage_recap() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(Some(dir.path().to_path_buf())).await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    create_owner(&state, org.id).await;

    handle_billing(
        &state,
        org.id,
        BillingOperation::TrialWillEnd {
            plan: get_enterprise_plan(),
            has_payment_method: true,
        },
    )
    .await;

    let sent = sent(dir.path());
    assert!(
        sent.contains(&campaign("trial_ending_has_payment")),
        "{sent}"
    );
    assert!(!sent.contains(&campaign("self_hosted_")), "{sent}");
}

#[tokio::test]
async fn a_cancelled_license_is_recognised_after_the_org_row_has_moved_to_free() {
    let dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(Some(dir.path().to_path_buf())).await;
    // The organization subscriber rewrites the row to Free off this same
    // event, and dispatch order is unspecified. Start from the row as it
    // stands once that subscriber has already run.
    let org = create_org(&state, crate::server::billing::plans::get_free_plan(), None).await;
    create_owner(&state, org.id).await;

    handle_billing(
        &state,
        org.id,
        BillingOperation::SubscriptionCancelled {
            plan: get_self_hosted_standard_plan(),
            reason_code: None,
            stripe_feedback: None,
            stripe_reason: None,
            internal_reason: None,
            comment: None,
            period_end: Utc::now(),
            was_trialing: true,
            mrr_amount_cents: 0,
            tenure_days: 14,
            license_key_type: Some(LicenseKeyType::Online),
        },
    )
    .await;

    let sent = sent(dir.path());
    assert!(
        sent.contains(&campaign("self_hosted_license_ended")),
        "{sent}"
    );
    assert!(
        !sent.contains(&campaign("subscription_cancelled")),
        "{sent}"
    );
}

/// A self-hosted plan with two included networks, so two network rows are at
/// the limit. (`check_plan_limits` ignores limits of one or less.)
fn self_hosted_plan_with_two_networks() -> BillingPlan {
    BillingPlan::SelfHostedStandard(PlanConfig {
        included_networks: Some(2),
        ..get_self_hosted_standard_plan().config()
    })
}

#[tokio::test]
async fn plan_limit_emails_skip_the_cloud_license_org_and_reach_the_customers_own_server() {
    let cloud_dir = tempfile::tempdir().unwrap();
    let own_server_dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(None).await;
    let org = create_org(&state, self_hosted_plan_with_two_networks(), None).await;
    create_owner(&state, org.id).await;
    for name in ["Office", "Warehouse"] {
        let mut network = network(&org.id);
        network.base.name = name.to_string();
        state
            .services
            .network_service
            .create(network, AuthenticatedEntity::System)
            .await
            .unwrap();
    }
    let notifications_before = reload(&state, org.id).await.base.notifications;

    // On the cloud app the limit describes the customer's own server, so
    // nothing is sent and the ratchet is left for a later move back to cloud.
    email_service_for(&state, cloud_dir.path(), DeploymentType::Cloud)
        .check_plan_limits(org.id, false)
        .await
        .unwrap();
    assert_eq!(sent(cloud_dir.path()), "");
    assert_eq!(
        reload(&state, org.id).await.base.notifications,
        notifications_before
    );

    // The customer's own server holds the very same plan, and there the limit
    // is real.
    email_service_for(&state, own_server_dir.path(), DeploymentType::Commercial)
        .check_plan_limits(org.id, false)
        .await
        .unwrap();
    assert!(
        sent(own_server_dir.path()).contains(&campaign("plan_limit_")),
        "{}",
        sent(own_server_dir.path())
    );
}

#[tokio::test]
async fn daemon_alerts_skip_the_cloud_license_org_and_reach_the_customers_own_server() {
    let cloud_dir = tempfile::tempdir().unwrap();
    let own_server_dir = tempfile::tempdir().unwrap();
    let (state, _container) = test_state_with_email_dir(None).await;
    let org = create_org(&state, get_self_hosted_standard_plan(), None).await;
    create_owner(&state, org.id).await;
    let network = state
        .services
        .network_service
        .create(network(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let daemon = daemon(&network.id, &Uuid::new_v4());
    let daemons = &state.services.daemon_service;

    daemons
        .send_unreachable_notification(
            &daemon,
            &email_service_for(&state, cloud_dir.path(), DeploymentType::Cloud),
        )
        .await
        .unwrap();
    assert_eq!(sent(cloud_dir.path()), "");

    daemons
        .send_unreachable_notification(
            &daemon,
            &email_service_for(&state, own_server_dir.path(), DeploymentType::Commercial),
        )
        .await
        .unwrap();
    assert!(
        sent(own_server_dir.path()).contains(&campaign("daemon_unreachable")),
        "{}",
        sent(own_server_dir.path())
    );
}

/// The three owner-only key routes, served with `entity` already
/// authenticated (the extractor reads it from request extensions).
async fn serve_key_routes(state: Arc<AppState>, entity: AuthenticatedEntity) -> String {
    let app = Router::new()
        .route("/keys", post(create_license_key))
        .route("/keys/current", get(get_current_license_key))
        .route("/keys/rotate", post(rotate_license_key))
        .layer(middleware::from_fn(
            move |mut request: Request<Body>, next: Next| {
                let entity = entity.clone();
                async move {
                    request.extensions_mut().insert(entity);
                    next.run(request).await
                }
            },
        ))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

/// Status and body for each of the three key routes.
async fn call_key_routes(base: &str) -> Vec<(&'static str, u16, String)> {
    let client = reqwest::Client::new();
    let requests = [
        (
            "POST /keys",
            client
                .post(format!("{base}/keys"))
                .json(&serde_json::json!({ "key_type": LicenseKeyType::Online })),
        ),
        (
            "GET /keys/current",
            client.get(format!("{base}/keys/current")),
        ),
        (
            "POST /keys/rotate",
            client.post(format!("{base}/keys/rotate")),
        ),
    ];
    let mut responses = Vec::new();
    for (route, request) in requests {
        let response = request.send().await.unwrap();
        responses.push((
            route,
            response.status().as_u16(),
            response.text().await.unwrap(),
        ));
    }
    responses
}

fn user_entity(organization_id: Uuid, permissions: UserOrgPermissions) -> AuthenticatedEntity {
    AuthenticatedEntity::User {
        user_id: Uuid::new_v4(),
        organization_id,
        permissions,
        network_ids: vec![],
        email: EmailAddress::new_unchecked("user@example.test"),
        email_verified: true,
    }
}

#[tokio::test]
async fn license_key_routes_refuse_everyone_below_owner() {
    let (state, _container) = test_state_with_email_dir(None).await;
    let org = create_org(&state, get_self_hosted_standard_plan(), None).await;

    let base = serve_key_routes(
        state.clone(),
        user_entity(org.id, UserOrgPermissions::Admin),
    )
    .await;
    for (route, status, body) in call_key_routes(&base).await {
        assert_eq!(status, 403, "{route}: {body}");
    }
    assert_eq!(reload(&state, org.id).await.base.license_key_version, 0);
}

#[tokio::test]
async fn license_key_routes_refuse_a_demo_organization_owner() {
    let (state, _container) = test_state_with_email_dir(None).await;
    let org = create_org(&state, BillingPlan::Demo(PlanConfig::default()), None).await;

    // The demo-mode middleware lets owners and every GET through, so without
    // the handler's own refusal `rotate` would bump the key version.
    let base = serve_key_routes(
        state.clone(),
        user_entity(org.id, UserOrgPermissions::Owner),
    )
    .await;
    for (route, status, body) in call_key_routes(&base).await {
        assert_eq!(status, 403, "{route}: {body}");
        assert!(body.contains("auth_demo_mode"), "{route}: {body}");
    }
    assert_eq!(reload(&state, org.id).await.base.license_key_version, 0);
}
