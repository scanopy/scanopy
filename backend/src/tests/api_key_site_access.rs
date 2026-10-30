//! What a user API key can reach after creating a site, through the real router and a real key.

use std::net::SocketAddr;
use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use super::self_hosted_licensing::{cloud_test_state, create_org};
use super::{site, subnet, user};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::config::AppState;
use crate::server::shared::api_key_common::{ApiKeyType, generate_api_key_for_storage};
use crate::server::shared::handlers::factory::create_router;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Storable;
use crate::server::user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase};
use crate::server::users::r#impl::permissions::UserOrgPermissions;

/// Serve the whole API on a local port.
pub(super) async fn serve(state: &Arc<AppState>) -> SocketAddr {
    let (router, _) = create_router(state.clone());
    let app = router.with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap()
    });
    addr
}

/// An enabled key for a new user in `organization_id`, holding `site_ids`. Returns the plaintext.
pub(super) async fn user_key(
    state: &AppState,
    organization_id: Uuid,
    permissions: UserOrgPermissions,
    site_ids: Vec<Uuid>,
) -> (Uuid, String) {
    let owner = state
        .services
        .user_service
        .create(user(&organization_id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let (plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::User);
    let key = UserApiKey::new(UserApiKeyBase {
        key: hashed,
        name: "automation".to_string(),
        user_id: owner.id,
        organization_id,
        permissions,
        is_enabled: true,
        ..UserApiKeyBase::default()
    });
    let created = state
        .services
        .user_api_key_service
        .create_with_sites(key, site_ids, AuthenticatedEntity::System)
        .await
        .unwrap();
    (created.id, plaintext)
}

pub(super) async fn send(
    addr: SocketAddr,
    method: reqwest::Method,
    path: &str,
    key: &str,
    body: Option<&Value>,
) -> (u16, Value) {
    let mut request = reqwest::Client::new()
        .request(method, format!("http://{addr}{path}"))
        .bearer_auth(key);
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.send().await.unwrap();
    let status = response.status().as_u16();
    let text = response.text().await.unwrap();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

#[tokio::test]
async fn a_key_that_creates_a_site_can_list_it_and_populate_it_and_keeps_its_other_sites() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let existing = state
        .services
        .site_service
        .create(site(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let (key_id, key) =
        user_key(&state, org.id, UserOrgPermissions::Admin, vec![existing.id]).await;
    let addr = serve(&state).await;

    let new_site = serde_json::to_value(site(&org.id)).unwrap();
    let (status, body) = send(
        addr,
        reqwest::Method::POST,
        "/api/v1/sites",
        &key,
        Some(&new_site),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let created: Uuid = serde_json::from_value(body["data"]["id"].clone()).unwrap();

    let mut held = state
        .services
        .user_api_key_service
        .get_site_ids(&key_id)
        .await
        .unwrap();
    held.sort();
    let mut expected = vec![existing.id, created];
    expected.sort();
    assert_eq!(held, expected, "the key gains the site and keeps the other");

    let (status, body) = send(addr, reqwest::Method::GET, "/api/v1/sites", &key, None).await;
    assert_eq!(status, 200, "{body}");
    let listed: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["id"].as_str())
        .collect();
    assert!(listed.contains(&created.to_string().as_str()), "{body}");

    let new_subnet = serde_json::to_value(subnet(&created)).unwrap();
    let (status, body) = send(
        addr,
        reqwest::Method::POST,
        "/api/v1/subnets",
        &key,
        Some(&new_subnet),
    )
    .await;
    assert_eq!(status, 200, "{body}");
}
