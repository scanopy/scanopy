//! Email-verification gates: each refuses an unverified user, allows a verified one, and is
//! lifted on a server with no email transport. Also covers how a new account starts
//! (verified or not) and the verify, resend, reset and email-change paths.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::FromRequestParts;
use axum::http::Request;
use axum::middleware::{self, Next};
use chrono::{Duration, Utc};
use email_address::EmailAddress;
use testcontainers::{ContainerAsync, GenericImage};
use uuid::Uuid;

use super::{organization, setup_test_db, site, user};
use crate::server::auth::r#impl::api::RegisterRequest;
use crate::server::auth::r#impl::base::{LoginRegisterParams, ProvisionOrg};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, Member, RequireVerified};
use crate::server::config::{AppState, DeploymentType, ServerConfig};
use crate::server::daemons::r#impl::api::ProvisionDaemonRequest;
use crate::server::daemons::r#impl::base::{DaemonMode, DaemonOs};
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::types::api::ApiError;
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::sites::r#impl::Site;
use crate::server::users::r#impl::base::User;
use crate::server::users::r#impl::permissions::UserOrgPermissions;

const IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// App state on a fresh database. `email_log_dir` set means the server has an email
/// transport; `None` means it has none.
async fn test_state(
    email_log_dir: Option<PathBuf>,
) -> (Arc<AppState>, ContainerAsync<GenericImage>) {
    let (pool, database_url, container) = setup_test_db().await;
    pool.close().await;
    let config = ServerConfig {
        database_url,
        email_log_dir,
        ..ServerConfig::default()
    };
    (AppState::new(config).await.unwrap(), container)
}

/// State with an email transport writing into a temp dir the caller keeps alive.
async fn state_with_email() -> (
    Arc<AppState>,
    ContainerAsync<GenericImage>,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let (state, container) = test_state(Some(dir.path().to_path_buf())).await;
    (state, container, dir)
}

async fn create_org(state: &AppState) -> Organization {
    state
        .services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap()
}

async fn create_user(
    state: &AppState,
    organization_id: Uuid,
    email: &str,
    permissions: UserOrgPermissions,
    email_verified: bool,
) -> User {
    let mut u = user(&organization_id);
    u.base.email = EmailAddress::new_unchecked(email);
    u.base.permissions = permissions;
    u.base.email_verified = email_verified;
    state
        .services
        .user_service
        .create(u, AuthenticatedEntity::System)
        .await
        .unwrap()
}

async fn create_site(state: &AppState, organization_id: Uuid) -> Site {
    state
        .services
        .site_service
        .create(site(&organization_id), AuthenticatedEntity::System)
        .await
        .unwrap()
}

async fn reload_user(state: &AppState, id: Uuid) -> User {
    state
        .services
        .user_service
        .get_by_id(&id)
        .await
        .unwrap()
        .unwrap()
}

fn session_entity(user: &User, site_ids: Vec<Uuid>) -> AuthenticatedEntity {
    AuthenticatedEntity::User {
        user_id: user.id,
        organization_id: user.base.organization_id,
        permissions: user.base.permissions,
        site_ids,
        email: user.base.email.clone(),
        email_verified: user.base.email_verified,
        terms_accepted_at: user.base.terms_accepted_at,
    }
}

/// The wire code of a refusal, as the UI receives it.
fn wire_code(err: &ApiError) -> Option<&'static str> {
    err.error_code.as_ref().map(ErrorCode::code)
}

/// Run the `RequireVerified<Member>` extractor against a request carrying `entity`.
async fn require_verified(
    state: &Arc<AppState>,
    entity: AuthenticatedEntity,
) -> Result<(), ApiError> {
    let mut request = Request::new(Body::empty());
    request.extensions_mut().insert(entity);
    let (mut parts, _) = request.into_parts();
    Authorized::<RequireVerified<Member>>::from_request_parts(&mut parts, state)
        .await
        .map(|_| ())
        .map_err(|e| e.0)
}

// ============================================================================
// RequireVerified extractor
// ============================================================================

#[tokio::test]
async fn require_verified_refuses_unverified_and_allows_verified() {
    let (state, _container, _dir) = state_with_email().await;
    let org = create_org(&state).await;
    let verified = create_user(
        &state,
        org.id,
        "v@example.com",
        UserOrgPermissions::Member,
        true,
    )
    .await;
    let unverified = create_user(
        &state,
        org.id,
        "u@example.com",
        UserOrgPermissions::Member,
        false,
    )
    .await;

    assert!(
        require_verified(&state, session_entity(&verified, vec![]))
            .await
            .is_ok()
    );

    let refused = require_verified(&state, session_entity(&unverified, vec![]))
        .await
        .unwrap_err();
    assert_eq!(refused.status, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(
        wire_code(&refused),
        Some(ErrorCode::AuthEmailVerificationRequired.code())
    );
}

#[tokio::test]
async fn require_verified_is_lifted_without_an_email_transport() {
    let (state, _container) = test_state(None).await;
    let org = create_org(&state).await;
    let unverified = create_user(
        &state,
        org.id,
        "u@example.com",
        UserOrgPermissions::Member,
        false,
    )
    .await;

    assert!(
        require_verified(&state, session_entity(&unverified, vec![]))
            .await
            .is_ok()
    );
}

// ============================================================================
// Daemon limit for orgs with no verified owner
// ============================================================================

fn provision_request(site_id: Uuid, name: &str) -> ProvisionDaemonRequest {
    ProvisionDaemonRequest {
        name: Some(name.to_string()),
        site_id: Some(site_id),
        mode: DaemonMode::DaemonPoll,
        url: None,
        seed_credential_refs: vec![],
        os: None,
        daemon_id: None,
    }
}

/// Provision two daemons for an org whose only owner has the given verification state, and
/// return the second attempt's result.
async fn provision_second_daemon(state: &AppState, owner_verified: bool) -> Result<(), ApiError> {
    let org = create_org(state).await;
    let owner = create_user(
        state,
        org.id,
        "owner@example.com",
        UserOrgPermissions::Owner,
        owner_verified,
    )
    .await;
    let site = create_site(state, org.id).await;
    let daemons = &state.services.daemon_service;
    let auth = session_entity(&owner, vec![site.id]);

    daemons
        .provision(&provision_request(site.id, "first"), None, auth.clone())
        .await
        .expect("the first daemon is always allowed");
    // What the create wizard reads before its first step must match what provisioning does.
    let reached = daemons
        .unverified_daemon_limit_reached(org.id)
        .await
        .unwrap();
    let result = daemons
        .provision(&provision_request(site.id, "second"), None, auth)
        .await
        .map(|_| ());
    assert_eq!(
        reached,
        result.is_err(),
        "eligibility and provisioning disagree"
    );
    result
}

#[tokio::test]
async fn second_daemon_is_refused_while_no_owner_is_verified() {
    let (state, _container, _dir) = state_with_email().await;
    let refused = provision_second_daemon(&state, false).await.unwrap_err();
    assert_eq!(
        wire_code(&refused),
        Some(ErrorCode::AuthEmailVerificationRequired.code())
    );
}

#[tokio::test]
async fn second_daemon_is_allowed_once_an_owner_is_verified() {
    let (state, _container, _dir) = state_with_email().await;
    assert!(provision_second_daemon(&state, true).await.is_ok());
}

#[tokio::test]
async fn second_daemon_limit_is_lifted_without_an_email_transport() {
    let (state, _container) = test_state(None).await;
    assert!(provision_second_daemon(&state, false).await.is_ok());
}

// ============================================================================
// Whether a new account starts verified
// ============================================================================

/// Register a password user into an existing org, as an invite acceptance does.
async fn register_invited(
    state: &AppState,
    organization_id: Uuid,
    email: &str,
    invited_as: Option<&str>,
) -> User {
    state
        .services
        .auth_service
        .register(
            RegisterRequest {
                email: EmailAddress::new_unchecked(email),
                password: "Correct-Horse-Battery-9".to_string(),
                terms_accepted: true,
                website: None,
            },
            LoginRegisterParams {
                provision_org: ProvisionOrg::Existing(organization_id),
                permissions: Some(UserOrgPermissions::Member),
                ip: IP,
                user_agent: None,
                site_ids: vec![],
                invited_as: invited_as.map(EmailAddress::new_unchecked),
            },
            DeploymentType::Community,
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn invite_acceptance_verifies_only_the_address_the_invite_was_sent_to() {
    let (state, _container, _dir) = state_with_email().await;
    let org = create_org(&state).await;

    let matching = register_invited(
        &state,
        org.id,
        "Alice@Example.com",
        Some("alice@example.com"),
    )
    .await;
    assert!(matching.base.email_verified);

    let other = register_invited(
        &state,
        org.id,
        "mallory@example.com",
        Some("bob@example.com"),
    )
    .await;
    assert!(!other.base.email_verified);
    assert!(other.base.email_verification_token.is_some());

    let link_only = register_invited(&state, org.id, "carol@example.com", None).await;
    assert!(!link_only.base.email_verified);
    assert!(link_only.base.email_verification_token.is_some());
}

#[tokio::test]
async fn sign_up_starts_verified_without_an_email_transport() {
    let (state, _container) = test_state(None).await;
    let org = create_org(&state).await;
    let link_only = register_invited(&state, org.id, "carol@example.com", None).await;
    assert!(link_only.base.email_verified);
}

#[tokio::test]
async fn an_identity_provider_assertion_verifies_the_address() {
    let (state, _container, _dir) = state_with_email().await;
    let auth = &state.services.auth_service;
    let email = EmailAddress::new_unchecked("sso@example.com");

    assert!(auth.verified_at_sign_up(&email, true, None));
    assert!(!auth.verified_at_sign_up(&email, false, None));
}

// ============================================================================
// Verify, resend, password reset, email change
// ============================================================================

#[tokio::test]
async fn an_unknown_or_expired_verification_token_gets_a_coded_400() {
    let (state, _container, _dir) = state_with_email().await;
    let auth = &state.services.auth_service;
    let org = create_org(&state).await;

    let unknown = ApiError::from(
        auth.verify_email("no-such-token", IP, None)
            .await
            .unwrap_err(),
    );
    assert_eq!(
        wire_code(&unknown),
        Some(ErrorCode::AuthVerificationTokenInvalid.code())
    );

    let mut expired = create_user(
        &state,
        org.id,
        "late@example.com",
        UserOrgPermissions::Owner,
        false,
    )
    .await;
    expired.base.email_verification_token = Some("expired-token".to_string());
    expired.base.email_verification_expires = Some(Utc::now() - Duration::hours(1));
    state
        .services
        .user_service
        .update(&mut expired, AuthenticatedEntity::System)
        .await
        .unwrap();
    let err = auth
        .verify_email("expired-token", IP, None)
        .await
        .unwrap_err();
    let err = ApiError::from(err);
    assert_eq!(err.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(
        wire_code(&err),
        Some(ErrorCode::AuthVerificationTokenInvalid.code())
    );
}

#[tokio::test]
async fn resend_answers_the_same_whether_or_not_the_address_has_an_unverified_account() {
    let (state, _container, _dir) = state_with_email().await;
    let auth = &state.services.auth_service;
    let org = create_org(&state).await;
    let verified = create_user(
        &state,
        org.id,
        "done@example.com",
        UserOrgPermissions::Owner,
        true,
    )
    .await;

    auth.resend_verification_email(&EmailAddress::new_unchecked("nobody@example.com"), IP, None)
        .await
        .expect("an address with no account gets the ordinary answer");
    auth.resend_verification_email(&verified.base.email, IP, None)
        .await
        .expect("an already verified address gets the ordinary answer");
    assert!(
        reload_user(&state, verified.id)
            .await
            .base
            .email_verification_token
            .is_none(),
        "nothing is sent to a verified address"
    );
}

#[tokio::test]
async fn resend_is_limited_per_address() {
    let (state, _container, _dir) = state_with_email().await;
    let auth = &state.services.auth_service;
    let org = create_org(&state).await;
    let pending = create_user(
        &state,
        org.id,
        "wait@example.com",
        UserOrgPermissions::Owner,
        false,
    )
    .await;

    auth.resend_verification_email(&pending.base.email, IP, None)
        .await
        .unwrap();
    assert!(
        reload_user(&state, pending.id)
            .await
            .base
            .email_verification_token
            .is_some()
    );

    let again = auth
        .resend_verification_email(&pending.base.email, IP, None)
        .await
        .unwrap_err();
    assert!(matches!(
        ApiError::from(again).error_code,
        Some(ErrorCode::AuthVerificationEmailCooldown { .. })
    ));
}

#[tokio::test]
async fn completing_a_password_reset_verifies_the_address() {
    let (state, _container, _dir) = state_with_email().await;
    let org = create_org(&state).await;
    let mut pending = create_user(
        &state,
        org.id,
        "reset@example.com",
        UserOrgPermissions::Owner,
        false,
    )
    .await;
    pending.base.password_reset_token = Some("reset-token".to_string());
    pending.base.password_reset_expires = Some(Utc::now() + Duration::hours(1));
    state
        .services
        .user_service
        .update(&mut pending, AuthenticatedEntity::System)
        .await
        .unwrap();

    let reset = state
        .services
        .auth_service
        .complete_password_reset("reset-token", "Correct-Horse-Battery-9", IP, None)
        .await
        .unwrap();
    assert!(reset.base.email_verified);
    assert!(reload_user(&state, pending.id).await.base.email_verified);
}

#[tokio::test]
async fn email_change_requests_are_limited_per_new_address() {
    let (state, _container, _dir) = state_with_email().await;
    let auth = &state.services.auth_service;
    let org = create_org(&state).await;
    let password = "Correct-Horse-Battery-9";
    let registered = auth
        .register(
            RegisterRequest {
                email: EmailAddress::new_unchecked("old@example.com"),
                password: password.to_string(),
                terms_accepted: true,
                website: None,
            },
            LoginRegisterParams {
                provision_org: ProvisionOrg::Existing(org.id),
                permissions: Some(UserOrgPermissions::Owner),
                ip: IP,
                user_agent: None,
                site_ids: vec![],
                invited_as: None,
            },
            DeploymentType::Community,
        )
        .await
        .unwrap();
    let new_email = EmailAddress::new_unchecked("new@example.com");

    auth.request_email_change(
        registered.id,
        Some(password.to_string()),
        new_email.clone(),
        IP,
        None,
    )
    .await
    .unwrap();
    let again = auth
        .request_email_change(
            registered.id,
            Some(password.to_string()),
            new_email,
            IP,
            None,
        )
        .await
        .unwrap_err();
    assert!(matches!(
        ApiError::from(again).error_code,
        Some(ErrorCode::AuthVerificationEmailCooldown { .. })
    ));
}

// ============================================================================
// Emailing the install command
// ============================================================================

/// Serve the daemon routes with `entity` already authenticated, and return the address.
async fn serve_daemon_routes(state: &Arc<AppState>, entity: AuthenticatedEntity) -> SocketAddr {
    let (router, _) = crate::server::daemons::handlers::create_router().split_for_parts();
    let app = router
        .layer(middleware::from_fn(
            move |mut request: Request<Body>, next: Next| {
                let entity = entity.clone();
                async move {
                    request.extensions_mut().insert(entity);
                    next.run(request).await
                }
            },
        ))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

async fn post_email_command(addr: SocketAddr, body: serde_json::Value) -> (u16, String) {
    let response = reqwest::Client::new()
        .post(format!("http://{addr}/email-install-command"))
        .json(&body)
        .send()
        .await
        .unwrap();
    (response.status().as_u16(), response.text().await.unwrap())
}

/// The HTML of every email the logging transport has written so far, joined. The `.txt`
/// alternative is decoded from this HTML, so markup checks belong here.
fn sent_html(dir: &std::path::Path) -> String {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "html"))
        .map(|path| std::fs::read_to_string(path).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn emailing_the_install_command_sends_the_server_built_command() {
    let (state, _container, dir) = state_with_email().await;
    let org = create_org(&state).await;
    let owner = create_user(
        &state,
        org.id,
        "owner@example.com",
        UserOrgPermissions::Owner,
        true,
    )
    .await;
    let site = create_site(&state, org.id).await;
    let daemons = &state.services.daemon_service;
    let auth = session_entity(&owner, vec![site.id]);
    let (daemon, key) = daemons
        .provision(&provision_request(site.id, "first"), None, auth.clone())
        .await
        .unwrap();
    let (_, other_key) = daemons
        .provision(&provision_request(site.id, "second"), None, auth.clone())
        .await
        .unwrap();
    let addr = serve_daemon_routes(&state, auth).await;

    let request = |daemon_id: Uuid, api_key: &str, log_file: &str| {
        serde_json::json!({
            "daemon_id": daemon_id,
            "purpose": "install",
            "log_file": log_file,
            "os": DaemonOs::Linux,
            "docker": false,
            "api_key": api_key,
        })
    };

    // Another daemon's key is refused, so no other text can be substituted in.
    let (status, body) =
        post_email_command(addr, request(daemon.id, &other_key, "/tmp/d.log")).await;
    assert_eq!(status, 400, "{body}");

    let (status, body) =
        post_email_command(addr, request(daemon.id, &key, "/tmp/<b>daemon</b>.log")).await;
    assert_eq!(status, 200, "{body}");

    // The handler sends before it responds, so the email is on disk by now.
    let email = sent_html(dir.path());
    assert!(email.contains(&key), "the key is filled into the command");
    assert!(!email.contains(&other_key));
    assert!(!email.contains("&lt;API_KEY&gt;"));
    assert!(
        email.contains("&lt;b&gt;daemon&lt;/b&gt;"),
        "client-chosen values are escaped in the HTML body"
    );
}

#[tokio::test]
async fn emailing_the_install_command_requires_a_verified_email() {
    let (state, _container, _dir) = state_with_email().await;
    let org = create_org(&state).await;
    let owner = create_user(
        &state,
        org.id,
        "owner@example.com",
        UserOrgPermissions::Owner,
        false,
    )
    .await;
    let site = create_site(&state, org.id).await;
    let auth = session_entity(&owner, vec![site.id]);
    let (daemon, key) = state
        .services
        .daemon_service
        .provision(&provision_request(site.id, "first"), None, auth.clone())
        .await
        .unwrap();
    let addr = serve_daemon_routes(&state, auth).await;

    let (status, body) = post_email_command(
        addr,
        serde_json::json!({
            "daemon_id": daemon.id,
            "purpose": "install",
            "os": DaemonOs::Linux,
            "api_key": key,
        }),
    )
    .await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("auth_email_verification_required"), "{body}");
}
