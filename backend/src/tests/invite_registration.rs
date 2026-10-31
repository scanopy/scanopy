//! Sign-up on a server with registration disabled: a valid pending invite lets a new user
//! through on both the password and the OIDC path, and anything short of one is refused.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum_client_ip::ClientIp;
use axum_extra::extract::Host;
use email_address::EmailAddress;
use testcontainers::{ContainerAsync, GenericImage};
use tower_sessions::Session;
use tower_sessions_memory_store::MemoryStore;
use uuid::Uuid;

use super::{organization, setup_test_db, user};
use crate::server::auth::handlers::oidc::oidc_authorize;
use crate::server::auth::handlers::registration::register;
use crate::server::auth::r#impl::api::{OidcAuthorizeParams, RegisterRequest};
use crate::server::auth::r#impl::oidc::OidcProviderConfig;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::config::{AppState, ServerConfig};
use crate::server::invites::handlers::process_pending_invite;
use crate::server::invites::r#impl::base::Invite;
use crate::server::shared::events::types::DeletionReason;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::types::api::{ApiError, ApiJson};
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::users::r#impl::base::User;
use crate::server::users::r#impl::permissions::UserOrgPermissions;

const IP: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
const OIDC_SLUG: &str = "idp";

/// App state on a fresh database with one OIDC provider configured. The issuer is
/// unreachable, so an authorize request that gets past the gates fails at discovery.
async fn test_state(disable_registration: bool) -> (Arc<AppState>, ContainerAsync<GenericImage>) {
    let (pool, database_url, container) = setup_test_db().await;
    pool.close().await;
    let config = ServerConfig {
        database_url,
        disable_registration,
        oidc_providers: Some(vec![OidcProviderConfig {
            name: "IdP".to_string(),
            slug: OIDC_SLUG.to_string(),
            logo: None,
            issuer_url: "http://127.0.0.1:1".to_string(),
            client_id: "client".to_string(),
            client_secret: "secret".to_string(),
            trusted_audiences: vec![],
        }]),
        ..ServerConfig::default()
    };
    (AppState::new(config).await.unwrap(), container)
}

fn new_session() -> Session {
    Session::new(None, Arc::new(MemoryStore::default()), None)
}

/// An org with an owner, and an invite from that owner valid for `expiration_hours`.
async fn org_with_invite(state: &AppState, expiration_hours: i64) -> Invite {
    let org = state
        .services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let mut owner = user(&org.id);
    owner.base.permissions = UserOrgPermissions::Owner;
    owner.base.email = EmailAddress::new_unchecked(format!("owner-{}@example.com", org.id));
    let owner = state
        .services
        .user_service
        .create(owner, AuthenticatedEntity::System)
        .await
        .unwrap();
    let invite = Invite::with_expiration(
        org.id,
        "http://localhost".to_string(),
        owner.id,
        expiration_hours,
        UserOrgPermissions::Member,
        vec![],
        None,
    );
    state
        .services
        .access_service
        .create_invite(invite, AuthenticatedEntity::System)
        .await
        .unwrap()
}

/// What `GET /invites/{id}/accept` leaves in the session of a visitor who is not signed in.
async fn hold_invite(session: &Session, invite: &Invite) {
    session
        .insert("pending_org_invite", invite.base.organization_id)
        .await
        .unwrap();
    session
        .insert("pending_invite_id", invite.id)
        .await
        .unwrap();
    session
        .insert("pending_invite_permissions", invite.base.permissions)
        .await
        .unwrap();
}

fn request(email: &str) -> RegisterRequest {
    RegisterRequest {
        email: EmailAddress::new_unchecked(email),
        password: "Correct-Horse-Battery-9".to_string(),
        terms_accepted: true,
        website: None,
    }
}

async fn sign_up(state: &Arc<AppState>, session: Session, email: &str) -> Result<User, ApiError> {
    register(
        State(state.clone()),
        Host("localhost".to_string()),
        ClientIp(IP),
        None,
        session,
        ApiJson(request(email)),
    )
    .await
    .map(|json| json.0.data().unwrap().clone())
}

async fn authorize_register(state: &Arc<AppState>, session: Session) -> Result<(), ApiError> {
    oidc_authorize(
        State(state.clone()),
        Host("localhost".to_string()),
        Path(OIDC_SLUG.to_string()),
        session,
        Query(OidcAuthorizeParams {
            flow: Some("register".to_string()),
            return_url: Some("http://localhost/onboarding".to_string()),
            terms_accepted: Some(true),
        }),
    )
    .await
    .map(|_| ())
}

fn is_registration_disabled(err: &ApiError) -> bool {
    err.status == StatusCode::FORBIDDEN
        && err.error_code.as_ref().map(ErrorCode::code)
            == Some(ErrorCode::AuthRegistrationDisabled.code())
}

async fn user_exists(state: &AppState, email: &str) -> bool {
    !state
        .services
        .user_service
        .get_all(StorableFilter::<User>::new_from_email(
            &EmailAddress::new_unchecked(email),
        ))
        .await
        .unwrap()
        .is_empty()
}

#[tokio::test]
async fn an_invited_password_sign_up_joins_the_inviting_org() {
    let (state, _container) = test_state(true).await;
    let invite = org_with_invite(&state, 24).await;
    let session = new_session();
    hold_invite(&session, &invite).await;

    let user = sign_up(&state, session, "invitee@example.com")
        .await
        .unwrap();
    assert_eq!(user.base.organization_id, invite.base.organization_id);
    assert_eq!(user.base.permissions, UserOrgPermissions::Member);
}

#[tokio::test]
async fn a_password_sign_up_without_a_valid_invite_is_refused() {
    let (state, _container) = test_state(true).await;

    let refused = sign_up(&state, new_session(), "nobody@example.com")
        .await
        .unwrap_err();
    assert!(is_registration_disabled(&refused), "{refused:?}");
    assert!(!user_exists(&state, "nobody@example.com").await);

    let expired = org_with_invite(&state, -1).await;
    let session = new_session();
    hold_invite(&session, &expired).await;
    let refused = sign_up(&state, session, "expired@example.com")
        .await
        .unwrap_err();
    assert!(is_registration_disabled(&refused), "{refused:?}");
    assert!(!user_exists(&state, "expired@example.com").await);

    let deleted = org_with_invite(&state, 24).await;
    state
        .services
        .invite_service
        .delete(
            &deleted.id,
            AuthenticatedEntity::System,
            DeletionReason::InviteConsumed,
        )
        .await
        .unwrap();
    let session = new_session();
    hold_invite(&session, &deleted).await;
    let refused = sign_up(&state, session, "deleted@example.com")
        .await
        .unwrap_err();
    assert!(is_registration_disabled(&refused), "{refused:?}");
    assert!(!user_exists(&state, "deleted@example.com").await);
}

#[tokio::test]
async fn oidc_register_needs_a_valid_invite_when_registration_is_disabled() {
    let (state, _container) = test_state(true).await;

    let refused = authorize_register(&state, new_session()).await.unwrap_err();
    assert!(is_registration_disabled(&refused), "{refused:?}");

    let invite = org_with_invite(&state, 24).await;
    let session = new_session();
    hold_invite(&session, &invite).await;
    // Past the gate, the request goes on to the provider, which is unreachable here.
    let outcome = authorize_register(&state, session).await;
    assert!(
        !outcome.as_ref().is_err_and(is_registration_disabled),
        "{outcome:?}"
    );
}

#[tokio::test]
async fn oidc_register_is_open_when_registration_is_enabled() {
    let (state, _container) = test_state(false).await;

    let outcome = authorize_register(&state, new_session()).await;
    assert!(
        !outcome.as_ref().is_err_and(is_registration_disabled),
        "{outcome:?}"
    );
}

/// #604: a failed invite use was logged and ignored, so the account was still created in the
/// inviting org.
#[tokio::test]
async fn a_deleted_invite_cannot_be_used() {
    let (state, _container) = test_state(false).await;
    let invite = org_with_invite(&state, 24).await;
    state
        .services
        .invite_service
        .delete(
            &invite.id,
            AuthenticatedEntity::System,
            DeletionReason::InviteConsumed,
        )
        .await
        .unwrap();
    let session = new_session();
    hold_invite(&session, &invite).await;

    assert!(process_pending_invite(&state, &session).await.is_err());
}

#[tokio::test]
async fn an_uninvited_sign_up_with_registration_enabled_creates_a_new_org() {
    let (state, _container) = test_state(false).await;
    let session = new_session();
    session
        .insert(
            "pending_setup",
            crate::server::auth::r#impl::base::PendingSetup {
                org_name: "Lab".to_string(),
                site: Some(crate::server::auth::r#impl::base::PendingSiteSetup {
                    name: "Home".to_string(),
                    site_id: Uuid::new_v4(),
                }),
                use_case: crate::server::organizations::r#impl::base::UseCase::Other,
            },
        )
        .await
        .unwrap();

    let user = sign_up(&state, session, "founder@example.com")
        .await
        .unwrap();
    assert_eq!(user.base.permissions, UserOrgPermissions::Owner);
}
