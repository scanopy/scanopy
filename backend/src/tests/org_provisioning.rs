//! What a new organization gets on Scanopy Cloud and on a self-hosted server:
//! its plan, the self-hosted org cap and how `/api/config` reports it, and
//! where daemon install commands download from.

use std::sync::Arc;

use axum::Extension;
use axum::extract::State;
use axum::response::IntoResponse;
use uuid::Uuid;

use super::self_hosted_licensing::{cloud_test_state, test_state_from};
use crate::server::{
    auth::{
        r#impl::base::{PendingSetup, PendingSiteSetup, ProvisionOrg, ProvisionUserParams},
        middleware::auth::AuthenticatedEntity,
    },
    billing::types::base::BillingPlan,
    config::{AppState, ServerConfig, get_deployment_type, get_public_config},
    daemons::r#impl::{base::Daemon, install_artifacts::DownloadSource},
    organizations::r#impl::base::UseCase,
    shared::{
        handlers::cache::AppCache,
        services::traits::CrudService,
        storage::filter::StorableFilter,
        types::{api::ApiError, error_codes::ErrorCode},
    },
    sites::r#impl::Site,
};

/// Sign up a new owner with a new org, as registration does.
async fn sign_up(state: &AppState, email: &str) -> anyhow::Result<Uuid> {
    let user = state
        .services
        .auth_service
        .provision_user(ProvisionUserParams {
            email: email.parse().unwrap(),
            password_hash: Some("hash".to_string()),
            oidc_subject: None,
            oidc_provider: None,
            provision_org: ProvisionOrg::New(PendingSetup {
                org_name: "Acme".to_string(),
                site: Some(PendingSiteSetup {
                    name: "Head Office".to_string(),
                    site_id: Uuid::new_v4(),
                }),
                use_case: UseCase::InternalIt,
            }),
            permissions: None,
            site_assignments: Vec::new(),
            terms_accepted_at: None,
            email_verified: true,
            deployment_type: get_deployment_type(&state.config),
        })
        .await?;
    Ok(user.base.organization_id)
}

async fn plan_of(state: &AppState, organization_id: Uuid) -> BillingPlan {
    state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await
        .unwrap()
        .unwrap()
        .base
        .plan
}

async fn org_limit_reached(state: &Arc<AppState>) -> bool {
    let response = get_public_config(State(state.clone()), Extension(Arc::new(AppCache::new())))
        .await
        .into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    json["data"]["org_limit_reached"].as_bool().unwrap()
}

#[tokio::test]
async fn a_self_hosted_server_gives_the_license_plan_and_caps_new_orgs() {
    let (state, _container) = test_state_from(ServerConfig::default()).await;
    assert!(!org_limit_reached(&state).await);

    let org = sign_up(&state, "first@example.com").await.unwrap();
    assert!(matches!(
        plan_of(&state, org).await,
        BillingPlan::Community(_)
    ));
    assert!(org_limit_reached(&state).await);

    let refused = sign_up(&state, "second@example.com")
        .await
        .expect_err("a second org is over the Community cap");
    assert!(matches!(
        refused
            .downcast_ref::<ApiError>()
            .and_then(|e| e.error_code.clone()),
        Some(ErrorCode::AuthOrgLimitReached)
    ));

    assert_eq!(
        DownloadSource::for_server(&state).await,
        DownloadSource::Github
    );
}

#[tokio::test]
async fn cloud_starts_new_orgs_on_free_and_never_caps_them() {
    let (state, _container) = cloud_test_state().await;

    let first = sign_up(&state, "first@example.com").await.unwrap();
    let second = sign_up(&state, "second@example.com").await.unwrap();
    assert!(plan_of(&state, first).await.is_free());
    assert!(plan_of(&state, second).await.is_free());
    assert!(!org_limit_reached(&state).await);

    assert_eq!(
        DownloadSource::for_server(&state).await,
        DownloadSource::Scarf
    );
}

/// On a server with an integrated daemon, a new org's site gets one provisioned daemon, and
/// seeding it again provisions no second one.
#[tokio::test]
async fn a_new_orgs_site_gets_the_integrated_daemon_once() {
    let (state, _container) = test_state_from(ServerConfig {
        // Nothing listens here: initializing the daemon fails and is logged, provisioning stands.
        integrated_daemon_url: Some("http://127.0.0.1:1".to_string()),
        ..ServerConfig::default()
    })
    .await;
    let org = sign_up(&state, "owner@example.com").await.unwrap();
    state.services.event_bus.flush().await;

    let site_ids: Vec<Uuid> = state
        .services
        .site_service
        .get_all(StorableFilter::<Site>::new_from_org_id(&org))
        .await
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    let daemons = || async {
        state
            .services
            .daemon_service
            .get_all(StorableFilter::<Daemon>::new_from_site_ids(&site_ids))
            .await
            .unwrap()
            .len()
    };
    assert_eq!(daemons().await, 1);

    state
        .services
        .org_setup_service
        .seed_integrated_daemon(site_ids[0], AuthenticatedEntity::System)
        .await
        .unwrap();
    assert_eq!(daemons().await, 1);
}
