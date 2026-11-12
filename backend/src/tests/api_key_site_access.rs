//! What a user API key can reach after creating a site, through the real router and a real key.

use std::net::SocketAddr;
use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use super::cross_tenant_probe::serve_as;
use super::self_hosted_licensing::{cloud_test_state, create_org};
use super::{access_on, host, site, subnet, user};
use crate::server::auth::access::Assignment;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::config::AppState;
use crate::server::credentials::r#impl::base::{Credential, CredentialBase};
use crate::server::shared::api_key_common::{ApiKeyType, generate_api_key_for_storage};
use crate::server::shared::handlers::factory::create_router;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Storable;
use crate::server::site_assignments::r#impl::base::Principal;
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
    set_user_access(state, owner.id, UserOrgPermissions::Owner, &[]).await;
    user_key_of(
        state,
        owner.id,
        organization_id,
        permissions,
        Some(site_ids),
    )
    .await
}

/// Give a user their role, on `site_ids` when the role is limited to sites.
pub(super) async fn set_user_access(
    state: &AppState,
    user_id: Uuid,
    permissions: UserOrgPermissions,
    site_ids: &[Uuid],
) {
    state
        .services
        .access_service
        .set_access(
            Principal::User(user_id),
            permissions,
            &Assignment::for_person(permissions, site_ids),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
}

/// An enabled key for `user_id`, holding `site_ids`, or All sites when `None`. Returns the plaintext.
pub(super) async fn user_key_of(
    state: &AppState,
    user_id: Uuid,
    organization_id: Uuid,
    permissions: UserOrgPermissions,
    site_ids: Option<Vec<Uuid>>,
) -> (Uuid, String) {
    let (plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::User);
    let key = UserApiKey::new(UserApiKeyBase {
        key: hashed,
        name: "automation".to_string(),
        user_id,
        organization_id,
        permissions,
        is_enabled: true,
        ..UserApiKeyBase::default()
    });
    let created = state
        .services
        .access_service
        .create_api_key(key, site_ids, AuthenticatedEntity::System)
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
async fn a_viewer_creates_rotates_and_deletes_their_own_key_and_no_one_elses() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let own_site = state
        .services
        .site_service
        .create(site(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let mut viewer = user(&org.id);
    viewer.base.email = "viewer@example.com".parse().unwrap();
    let viewer = state
        .services
        .user_service
        .create(viewer, AuthenticatedEntity::System)
        .await
        .unwrap();
    let addr = serve_as(
        &state,
        AuthenticatedEntity::User {
            user_id: viewer.id,
            organization_id: org.id,
            permissions: UserOrgPermissions::Viewer,
            access: access_on(UserOrgPermissions::Viewer, &[own_site.id]),
            email: viewer.base.email.clone(),
            email_verified: true,
            terms_accepted_at: None,
        },
    )
    .await;
    let client = reqwest::Client::new();
    let url = |path: &str| format!("http://{addr}/api/v1/user-api-keys{path}");

    let body = serde_json::to_value(UserApiKey::new(UserApiKeyBase {
        name: "read-only".to_string(),
        permissions: UserOrgPermissions::Viewer,
        is_enabled: true,
        site_ids: Some(vec![own_site.id]),
        ..UserApiKeyBase::default()
    }))
    .unwrap();
    let response = client.post(url("")).json(&body).send().await.unwrap();
    let status = response.status().as_u16();
    let created: Value = response.json().await.unwrap();
    assert_eq!(status, 200, "{created}");
    let key_id = created["data"]["api_key"]["id"].as_str().unwrap();

    let path = format!("/{key_id}/rotate");
    let response = client.post(url(&path)).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);

    let (foreign, _) = user_key(&state, org.id, UserOrgPermissions::Viewer, vec![]).await;
    let path = format!("/{foreign}/rotate");
    let response = client.post(url(&path)).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 403);

    let path = format!("/{key_id}");
    let response = client.delete(url(&path)).send().await.unwrap();
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn a_key_assigns_a_credential_to_its_own_sites_only() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let create_site = || {
        state
            .services
            .site_service
            .create(site(&org.id), AuthenticatedEntity::System)
    };
    let (own, other) = (create_site().await.unwrap(), create_site().await.unwrap());
    let (_, key) = user_key(&state, org.id, UserOrgPermissions::Admin, vec![own.id]).await;
    let addr = serve(&state).await;

    let assigned_to = |site_id: Uuid| {
        serde_json::to_value(Credential::new(CredentialBase {
            name: format!("assigned to {site_id}"),
            organization_id: org.id,
            assigned_site_ids: vec![site_id],
            ..CredentialBase::default()
        }))
        .unwrap()
    };
    let path = "/api/v1/credentials";
    let body = assigned_to(own.id);
    let (status, body) = send(addr, reqwest::Method::POST, path, &key, Some(&body)).await;
    assert_eq!(status, 200, "{body}");
    let body = assigned_to(other.id);
    let (status, body) = send(addr, reqwest::Method::POST, path, &key, Some(&body)).await;
    assert_eq!(status, 403, "{body}");
}

/// The next `data:` event on an open stream, skipping keep-alives.
async fn next_event(stream: &mut reqwest::Response) -> String {
    let wait = std::time::Duration::from_secs(5);
    loop {
        let chunk = tokio::time::timeout(wait, stream.chunk())
            .await
            .expect("an event arrives")
            .unwrap()
            .expect("the stream stays open");
        let text = String::from_utf8_lossy(&chunk).into_owned();
        if text.contains("data:") {
            return text;
        }
    }
}

#[tokio::test]
async fn an_open_stream_stops_delivering_a_site_the_key_lost() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let create_site = || {
        state
            .services
            .site_service
            .create(site(&org.id), AuthenticatedEntity::System)
    };
    let (kept, lost) = (create_site().await.unwrap(), create_site().await.unwrap());
    let (key_id, key) = user_key(
        &state,
        org.id,
        UserOrgPermissions::Viewer,
        vec![kept.id, lost.id],
    )
    .await;
    let addr = serve(&state).await;

    let mut stream = reqwest::Client::new()
        .get(format!("http://{addr}/api/v1/live-updates/stream"))
        .bearer_auth(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status().as_u16(), 200);

    let topology = &state.services.topology_service;
    topology.broadcast_live_update(lost.id);
    let event = next_event(&mut stream).await;
    assert!(event.contains(&lost.id.to_string()), "{event}");

    // Seeding a site also broadcasts, so a `lost` event can pass its check just before the change.
    // `marker` only becomes visible with the change: nothing after its first event predates it.
    let marker = create_site().await.unwrap();
    let role = UserOrgPermissions::Viewer;
    state
        .services
        .access_service
        .set_access(
            Principal::ApiKey(key_id),
            role,
            &Assignment::for_api_key(role, Some(&[kept.id, marker.id])),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    topology.broadcast_live_update(marker.id);
    while !next_event(&mut stream)
        .await
        .contains(&marker.id.to_string())
    {}

    topology.broadcast_live_update(lost.id);
    topology.broadcast_live_update(marker.id);
    loop {
        let event = next_event(&mut stream).await;
        assert!(!event.contains(&lost.id.to_string()), "{event}");
        if event.contains(&marker.id.to_string()) {
            break;
        }
    }
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
        .unwrap()
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

    // The creator's assignment is Admin on the new site, so the key can change it too.
    let path = format!("/api/v1/sites/{created}");
    let body = serde_json::to_value(site(&org.id)).unwrap();
    let (status, body) = send(addr, reqwest::Method::PUT, &path, &key, Some(&body)).await;
    assert_eq!(status, 200, "{body}");
}

/// The key cap, through the real router and a real key: what its user loses, the key loses, on the
/// next request.
/// The first screen a person with no sites sees reads lists, so those stay empty, not refused.
#[tokio::test]
async fn a_key_on_no_site_reads_empty_lists_and_changes_nothing() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let elsewhere = state
        .services
        .site_service
        .create(site(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let (_, key) = user_key(&state, org.id, UserOrgPermissions::Member, vec![]).await;
    let addr = serve(&state).await;

    for path in ["/api/v1/sites", "/api/v1/hosts", "/api/v1/subnets"] {
        let (status, body) = send(addr, reqwest::Method::GET, path, &key, None).await;
        assert_eq!(status, 200, "{path}: {body}");
        assert_eq!(body["data"], serde_json::json!([]), "{path}");
    }

    let path = format!("/api/v1/sites/{}", elsewhere.id);
    let (status, _) = send(addr, reqwest::Method::GET, &path, &key, None).await;
    assert_ne!(status, 200, "a site read by id");

    let host = serde_json::to_value(host(&elsewhere.id)).unwrap();
    let (status, _) = send(
        addr,
        reqwest::Method::POST,
        "/api/v1/hosts",
        &key,
        Some(&host),
    )
    .await;
    assert_eq!(status, 403, "a write stays refused");
}

#[tokio::test]
async fn a_key_drops_with_its_owner() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let create_site = || {
        state
            .services
            .site_service
            .create(site(&org.id), AuthenticatedEntity::System)
    };
    let (kept, lost) = (create_site().await.unwrap(), create_site().await.unwrap());
    let mut person = user(&org.id);
    person.base.email = "keyholder@example.com".parse().unwrap();
    person.base.permissions = UserOrgPermissions::Admin;
    let person = state
        .services
        .user_service
        .create(person, AuthenticatedEntity::System)
        .await
        .unwrap();
    let admin = UserOrgPermissions::Admin;
    set_user_access(&state, person.id, admin, &[]).await;
    let (_, key) = user_key_of(
        &state,
        person.id,
        org.id,
        admin,
        Some(vec![kept.id, lost.id]),
    )
    .await;
    let addr = serve(&state).await;

    let listed = |body: &Value| -> Vec<Uuid> {
        let rows = body["data"].as_array().cloned().unwrap_or_default();
        let mut ids: Vec<Uuid> = rows
            .iter()
            .filter_map(|row| serde_json::from_value(row["id"].clone()).ok())
            .collect();
        ids.sort();
        ids
    };
    let sites = "/api/v1/sites";
    let (_, body) = send(addr, reqwest::Method::GET, sites, &key, None).await;
    let mut both = vec![kept.id, lost.id];
    both.sort();
    assert_eq!(listed(&body), both);
    let new_site = serde_json::to_value(site(&org.id)).unwrap();
    let credentials = "/api/v1/credentials";
    let credential = |name: &str| {
        serde_json::to_value(Credential::new(CredentialBase {
            name: name.to_string(),
            organization_id: org.id,
            ..CredentialBase::default()
        }))
        .unwrap()
    };
    let body = credential("before");
    let (status, body) = send(addr, reqwest::Method::POST, credentials, &key, Some(&body)).await;
    assert_eq!(
        status, 200,
        "the key writes credentials as an Admin: {body}"
    );

    // The owner is demoted and loses a site. The key's stored role and sites are untouched.
    let member = UserOrgPermissions::Member;
    set_user_access(&state, person.id, member, &[kept.id]).await;

    let (_, body) = send(addr, reqwest::Method::GET, sites, &key, None).await;
    assert_eq!(listed(&body), vec![kept.id], "the lost site is gone");
    let body = credential("after");
    let (status, _) = send(addr, reqwest::Method::POST, credentials, &key, Some(&body)).await;
    assert_eq!(status, 403, "credential writes drop with the role");
    let (status, _) = send(addr, reqwest::Method::POST, sites, &key, Some(&new_site)).await;
    assert_eq!(status, 403, "so does creating a site");
    let path = format!("/api/v1/sites/{}", kept.id);
    let body = serde_json::to_value(site(&org.id)).unwrap();
    let (status, _) = send(addr, reqwest::Method::PUT, &path, &key, Some(&body)).await;
    assert_eq!(status, 403, "a Member's site role can't edit a site");
    let new_subnet = serde_json::to_value(subnet(&kept.id)).unwrap();
    let path = "/api/v1/subnets";
    let (status, body) = send(addr, reqwest::Method::POST, path, &key, Some(&new_subnet)).await;
    assert_eq!(status, 200, "what the owner kept, the key keeps: {body}");
}

/// A limited Owner key stays limited, and keeps billing.
#[tokio::test]
async fn an_owner_key_on_some_sites_stays_limited_and_keeps_owner_resources() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let create_site = || {
        state
            .services
            .site_service
            .create(site(&org.id), AuthenticatedEntity::System)
    };
    let (held, other) = (create_site().await.unwrap(), create_site().await.unwrap());
    let (_, key) = user_key(&state, org.id, UserOrgPermissions::Owner, vec![held.id]).await;
    let addr = serve(&state).await;

    let (_, body) = send(addr, reqwest::Method::GET, "/api/v1/sites", &key, None).await;
    let listed = body["data"].as_array().unwrap();
    assert_eq!(listed.len(), 1, "{body}");
    assert_eq!(listed[0]["id"], serde_json::json!(held.id));

    let path = format!("/api/v1/sites/{}", other.id);
    let (status, _) = send(addr, reqwest::Method::GET, &path, &key, None).await;
    assert_eq!(status, 403);

    let path = "/api/billing/save-offer-coupon";
    let (status, body) = send(addr, reqwest::Method::GET, path, &key, None).await;
    assert_ne!(status, 403, "the key keeps billing: {body}");
}

/// Escalation through the routes: an Admin can't invite an Admin, an Owner can, and a user can make
/// a key at their own level and not above it.
#[tokio::test]
async fn invites_and_keys_stop_at_what_the_caller_could_grant() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let held = state
        .services
        .site_service
        .create(site(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let caller = |role: UserOrgPermissions, email: &str| {
        let session = user(&org.id);
        let state = state.clone();
        let email = email.to_string();
        async move {
            let mut session = session;
            session.base.email = email.parse().unwrap();
            session.base.permissions = role;
            let stored = state
                .services
                .user_service
                .create(session, AuthenticatedEntity::System)
                .await
                .unwrap();
            set_user_access(&state, stored.id, role, &[]).await;
            serve_as(
                &state,
                AuthenticatedEntity::User {
                    user_id: stored.id,
                    organization_id: org.id,
                    permissions: role,
                    access: state
                        .services
                        .access_service
                        .user_access(&stored)
                        .await
                        .unwrap(),
                    email: stored.base.email.clone(),
                    email_verified: true,
                    terms_accepted_at: None,
                },
            )
            .await
        }
    };
    let admin = caller(UserOrgPermissions::Admin, "admin@example.com").await;
    let owner = caller(UserOrgPermissions::Owner, "owner@example.com").await;

    let client = reqwest::Client::new();
    let invite = |addr: std::net::SocketAddr, role: UserOrgPermissions| {
        client
            .post(format!("http://{addr}/api/v1/invites"))
            .json(&serde_json::json!({"permissions": role, "site_ids": []}))
            .send()
    };
    let status = |response: reqwest::Response| response.status().as_u16();
    use UserOrgPermissions::{Admin, Member, Owner};
    assert_eq!(status(invite(admin, Admin).await.unwrap()), 403);
    assert_eq!(status(invite(admin, Owner).await.unwrap()), 403);
    assert_eq!(status(invite(admin, Member).await.unwrap()), 200);
    assert_eq!(status(invite(owner, Owner).await.unwrap()), 200);

    let key = |role: UserOrgPermissions| {
        let body = serde_json::to_value(UserApiKey::new(UserApiKeyBase {
            name: "own level".to_string(),
            permissions: role,
            is_enabled: true,
            site_ids: Some(vec![held.id]),
            ..UserApiKeyBase::default()
        }))
        .unwrap();
        client
            .post(format!("http://{admin}/api/v1/user-api-keys"))
            .json(&body)
            .send()
    };
    assert_eq!(status(key(Admin).await.unwrap()), 200);
    assert_eq!(status(key(Owner).await.unwrap()), 403);
}
