//! A user API key on All sites, through the real router: who can grant it, what it reaches, what
//! it stores, and creating a site with it.

use std::net::SocketAddr;
use std::sync::Arc;

use reqwest::Method;
use serde_json::{Value, json};
use uuid::Uuid;

use super::api_key_site_access::{send, serve, set_user_access, user_key_of};
use super::cross_tenant_probe::serve_as;
use super::self_hosted_licensing::{cloud_test_state, create_org};
use super::{host, site, subnet, user};
use crate::server::auth::access::{Assignment, AssignmentScope};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::config::AppState;
use crate::server::shared::services::traits::CrudService;
use crate::server::site_assignments::r#impl::base::Principal;
use crate::server::sites::r#impl::Site;
use crate::server::users::r#impl::{base::User, permissions::UserOrgPermissions};

const KEYS: &str = "/api/v1/user-api-keys";

async fn stored_site(state: &AppState, organization_id: Uuid) -> Site {
    state
        .services
        .site_service
        .create(site(&organization_id), AuthenticatedEntity::System)
        .await
        .unwrap()
}

/// A user holding `role` on `site_ids` (every site for an Admin), and the API served as their
/// session.
async fn signed_in(
    state: &Arc<AppState>,
    organization_id: Uuid,
    role: UserOrgPermissions,
    site_ids: &[Uuid],
) -> (User, SocketAddr) {
    let mut person = user(&organization_id);
    person.base.email = format!("{}@example.com", Uuid::new_v4()).parse().unwrap();
    person.base.permissions = role;
    let person = state
        .services
        .user_service
        .create(person, AuthenticatedEntity::System)
        .await
        .unwrap();
    set_user_access(state, person.id, role, site_ids).await;
    let person = state
        .services
        .user_service
        .get_by_id(&person.id)
        .await
        .unwrap()
        .unwrap();
    let access = state
        .services
        .access_service
        .user_access(&person)
        .await
        .unwrap();
    let addr = serve_as(
        state,
        AuthenticatedEntity::User {
            user_id: person.id,
            organization_id,
            permissions: role,
            access,
            email: person.base.email.clone(),
            email_verified: true,
            terms_accepted_at: None,
        },
    )
    .await;
    (person, addr)
}

/// Send as the signed-in session `addr` serves.
async fn as_session(
    addr: SocketAddr,
    method: Method,
    path: &str,
    body: Option<&Value>,
) -> (u16, Value) {
    let mut request = reqwest::Client::new().request(method, format!("http://{addr}{path}"));
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.send().await.unwrap();
    let status = response.status().as_u16();
    (status, response.json().await.unwrap_or(Value::Null))
}

fn new_key(role: UserOrgPermissions, site_ids: Value) -> Value {
    json!({
        "name": "automation",
        "permissions": role,
        "is_enabled": true,
        "expires_at": null,
        "tags": [],
        "site_ids": site_ids,
    })
}

/// The ids in a list response, sorted.
fn listed(body: &Value) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = body["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|row| serde_json::from_value(row["id"].clone()).ok())
        .collect();
    ids.sort();
    ids
}

fn sorted(mut ids: Vec<Uuid>) -> Vec<Uuid> {
    ids.sort();
    ids
}

async fn rows_of(state: &AppState, key_id: Uuid) -> Vec<Assignment> {
    state
        .services
        .site_assignment_service
        .get_for(Principal::ApiKey(key_id))
        .await
        .unwrap()
}

#[tokio::test]
async fn an_all_sites_key_reaches_every_site_and_creates_sites_in_its_own_organization() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let other_org = create_org(&state, get_enterprise_plan(), None).await;
    let (a, b) = (
        stored_site(&state, org.id).await,
        stored_site(&state, org.id).await,
    );
    let admin = UserOrgPermissions::Admin;
    let (_, session) = signed_in(&state, org.id, admin, &[]).await;

    let (status, created) = as_session(
        session,
        Method::POST,
        KEYS,
        Some(&new_key(admin, Value::Null)),
    )
    .await;
    assert_eq!(status, 200, "{created}");
    assert_eq!(created["data"]["api_key"]["site_ids"], Value::Null);
    let key_id: Uuid = serde_json::from_value(created["data"]["api_key"]["id"].clone()).unwrap();
    let key = created["data"]["key"].as_str().unwrap().to_string();
    let rows = rows_of(&state, key_id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].scope, AssignmentScope::AllSites);

    let (_, read) = as_session(session, Method::GET, &format!("{KEYS}/{key_id}"), None).await;
    assert_eq!(read["data"]["site_ids"], Value::Null, "{read}");
    let filtered = format!("{KEYS}?site_ids={}", a.id);
    let (_, body) = as_session(session, Method::GET, &filtered, None).await;
    assert_eq!(
        listed(&body),
        vec![key_id],
        "a site filter matches an All sites key"
    );

    let addr = serve(&state).await;
    let (_, body) = send(addr, Method::GET, "/api/v1/sites", &key, None).await;
    assert_eq!(listed(&body), sorted(vec![a.id, b.id]));
    for site_id in [a.id, b.id] {
        let body = serde_json::to_value(host(&site_id)).unwrap();
        let (status, body) = send(addr, Method::POST, "/api/v1/hosts", &key, Some(&body)).await;
        assert_eq!(status, 200, "{body}");
    }

    let later = stored_site(&state, org.id).await;
    let path = format!("/api/v1/sites/{}", later.id);
    let (status, body) = send(addr, Method::GET, &path, &key, None).await;
    assert_eq!(status, 200, "a site made after the key: {body}");
    let body = serde_json::to_value(subnet(&later.id)).unwrap();
    let (status, body) = send(addr, Method::POST, "/api/v1/subnets", &key, Some(&body)).await;
    assert_eq!(status, 200, "{body}");

    for body in [
        json!({"name": "Branch office"}),
        json!({"name": "Warehouse", "organization_id": other_org.id}),
    ] {
        let (status, created) = send(addr, Method::POST, "/api/v1/sites", &key, Some(&body)).await;
        assert_eq!(status, 200, "{created}");
        assert_eq!(created["data"]["organization_id"], json!(org.id));
        let path = format!("/api/v1/sites/{}", created["data"]["id"].as_str().unwrap());
        let (status, body) = send(addr, Method::GET, &path, &key, None).await;
        assert_eq!(status, 200, "the key reaches the site it made: {body}");
    }
    assert_eq!(
        rows_of(&state, key_id).await.len(),
        1,
        "creating sites adds no rows to an All sites key"
    );
}

#[tokio::test]
async fn a_caller_on_listed_sites_cannot_grant_all_sites() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let (held, other) = (
        stored_site(&state, org.id).await,
        stored_site(&state, org.id).await,
    );
    let member = UserOrgPermissions::Member;
    // The caller holds every site there is, as a list.
    let (_, session) = signed_in(&state, org.id, member, &[held.id, other.id]).await;
    let (_, narrower) = signed_in(&state, org.id, member, &[held.id]).await;

    let (foreign_status, foreign) = as_session(
        narrower,
        Method::POST,
        KEYS,
        Some(&new_key(member, json!([other.id]))),
    )
    .await;
    assert_eq!(foreign_status, 403, "{foreign}");

    for addr in [session, narrower] {
        let (status, body) = as_session(
            addr,
            Method::POST,
            KEYS,
            Some(&new_key(member, Value::Null)),
        )
        .await;
        assert_eq!(status, foreign_status, "{body}");
        assert_eq!(body["code"], foreign["code"], "{body}");
        assert_eq!(
            body["params"]["entity"], foreign["params"]["entity"],
            "{body}"
        );
    }

    let (status, created) = as_session(
        session,
        Method::POST,
        KEYS,
        Some(&new_key(member, json!([held.id]))),
    )
    .await;
    assert_eq!(status, 200, "{created}");
    let mut key = created["data"]["api_key"].clone();
    key["site_ids"] = Value::Null;
    let path = format!("{KEYS}/{}", key["id"].as_str().unwrap());
    let (status, body) = as_session(session, Method::PUT, &path, Some(&key)).await;
    assert_eq!(status, foreign_status, "nor on update: {body}");
    assert_eq!(body["code"], foreign["code"], "{body}");
}

#[tokio::test]
async fn an_empty_site_list_is_refused_and_a_key_with_no_rows_reaches_nothing() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let held = stored_site(&state, org.id).await;
    let admin = UserOrgPermissions::Admin;
    let (person, session) = signed_in(&state, org.id, admin, &[]).await;

    let (status, body) = as_session(
        session,
        Method::POST,
        KEYS,
        Some(&new_key(admin, json!([]))),
    )
    .await;
    assert_eq!(status, 400, "{body}");
    let mut omitted = new_key(admin, Value::Null);
    omitted.as_object_mut().unwrap().remove("site_ids");
    let (status, body) = as_session(session, Method::POST, KEYS, Some(&omitted)).await;
    assert_eq!(status, 400, "an omitted list is not All sites: {body}");

    let (status, created) = as_session(
        session,
        Method::POST,
        KEYS,
        Some(&new_key(admin, json!([held.id]))),
    )
    .await;
    assert_eq!(status, 200, "{created}");
    let mut key = created["data"]["api_key"].clone();
    key["site_ids"] = json!([]);
    let path = format!("{KEYS}/{}", key["id"].as_str().unwrap());
    let (status, body) = as_session(session, Method::PUT, &path, Some(&key)).await;
    assert_eq!(status, 400, "{body}");

    // A key from before site assignments: stored with no rows.
    let (legacy_id, legacy) = user_key_of(&state, person.id, org.id, admin, Some(vec![])).await;
    assert!(rows_of(&state, legacy_id).await.is_empty());
    let (_, read) = as_session(session, Method::GET, &format!("{KEYS}/{legacy_id}"), None).await;
    assert_eq!(read["data"]["site_ids"], json!([]), "{read}");
    let addr = serve(&state).await;
    let (status, body) = send(addr, Method::GET, "/api/v1/sites", &legacy, None).await;
    assert_eq!(status, 200, "{body}");
    assert!(listed(&body).is_empty(), "{body}");
}

#[tokio::test]
async fn switching_a_key_between_listed_sites_and_all_sites_keeps_only_the_current_rows() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let (a, b) = (
        stored_site(&state, org.id).await,
        stored_site(&state, org.id).await,
    );
    let admin = UserOrgPermissions::Admin;
    let (_, session) = signed_in(&state, org.id, admin, &[]).await;
    let (status, created) = as_session(
        session,
        Method::POST,
        KEYS,
        Some(&new_key(admin, json!([a.id, b.id]))),
    )
    .await;
    assert_eq!(status, 200, "{created}");
    let mut key = created["data"]["api_key"].clone();
    let key_id: Uuid = serde_json::from_value(key["id"].clone()).unwrap();
    let path = format!("{KEYS}/{key_id}");

    let scopes = |rows: Vec<Assignment>| -> Vec<AssignmentScope> {
        let mut scopes: Vec<AssignmentScope> = rows.into_iter().map(|row| row.scope).collect();
        scopes.sort_by_key(|scope| format!("{scope:?}"));
        scopes
    };
    let mut both = vec![AssignmentScope::Site(a.id), AssignmentScope::Site(b.id)];
    both.sort_by_key(|scope| format!("{scope:?}"));
    assert_eq!(scopes(rows_of(&state, key_id).await), both);

    key["site_ids"] = Value::Null;
    let (status, body) = as_session(session, Method::PUT, &path, Some(&key)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["data"]["site_ids"], Value::Null);
    assert_eq!(
        scopes(rows_of(&state, key_id).await),
        vec![AssignmentScope::AllSites]
    );

    key["site_ids"] = json!([a.id]);
    let (status, body) = as_session(session, Method::PUT, &path, Some(&key)).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        scopes(rows_of(&state, key_id).await),
        vec![AssignmentScope::Site(a.id)]
    );
    let (_, read) = as_session(session, Method::GET, &path, None).await;
    assert_eq!(read["data"]["site_ids"], json!([a.id]), "{read}");
}

#[tokio::test]
async fn an_all_sites_key_drops_to_its_owners_sites() {
    let (state, _container) = cloud_test_state().await;
    let org = create_org(&state, get_enterprise_plan(), None).await;
    let (kept, lost) = (
        stored_site(&state, org.id).await,
        stored_site(&state, org.id).await,
    );
    let admin = UserOrgPermissions::Admin;
    let (person, _) = signed_in(&state, org.id, admin, &[]).await;
    let (_, key) = user_key_of(&state, person.id, org.id, admin, None).await;
    let addr = serve(&state).await;

    let (_, body) = send(addr, Method::GET, "/api/v1/sites", &key, None).await;
    assert_eq!(listed(&body), sorted(vec![kept.id, lost.id]));

    let member = UserOrgPermissions::Member;
    set_user_access(&state, person.id, member, &[kept.id]).await;
    let later = stored_site(&state, org.id).await;

    let (_, body) = send(addr, Method::GET, "/api/v1/sites", &key, None).await;
    assert_eq!(listed(&body), vec![kept.id], "only what the owner kept");
    let path = format!("/api/v1/sites/{}", later.id);
    let (status, _) = send(addr, Method::GET, &path, &key, None).await;
    assert_eq!(status, 403, "nor a site made after the demotion");
}
