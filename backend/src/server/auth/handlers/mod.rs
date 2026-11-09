use crate::server::{
    auth::{
        r#impl::{
            api::{
                CheckEmailRequest, ForgotPasswordRequest, LoginRequest, OidcAuthorizeParams,
                OidcCallbackParams, OnboardingSiteState, OnboardingStateResponse,
                OnboardingStepRequest, RegisterRequest, RequestEmailChangeRequest,
                ResendVerificationRequest, ResetPasswordRequest, SetupRequest, SetupResponse,
                SiteSetup, UpdatePasswordRequest, VerifyEmailRequest,
            },
            base::{LoginRegisterParams, PendingSetup, PendingSiteSetup, ProvisionOrg},
            oidc::{OidcFlow, OidcPendingAuth, OidcProviderMetadata, OidcRegisterParams},
        },
        middleware::permissions::{Authorized, IsUser},
        oidc::{OidcRegisterResult, OidcService},
    },
    config::{AppState, DeploymentType, get_deployment_type},
    invites::handlers::process_pending_invite,
    organizations::r#impl::base::UseCase,
    shared::{
        services::traits::CrudService,
        storage::{filter::StorableFilter, traits::Storable},
        types::{
            api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse},
            error_codes::ErrorCode,
        },
    },
    users::r#impl::base::{User, UserBase},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Json, Redirect},
    routing::get,
};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, extract::Host, headers::UserAgent};
use chrono::{DateTime, Utc};
use std::{net::IpAddr, sync::Arc};
use tower_sessions::Session;
use url::Url;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

/// Hosts where demo org login is allowed
const DEMO_HOSTS: &[&str] = &[
    "demo.scanopy.net",
    "localhost",
    "scanopy-staging.maya.cloud",
];

/// The dedicated demo-only domain where registration is blocked
/// and only demo accounts can log in
const DEMO_ONLY_HOST: &str = "demo.scanopy.net";

fn is_demo_host(host: &str) -> bool {
    let hostname = host.split(':').next().unwrap_or(host);
    DEMO_HOSTS.contains(&hostname)
}

fn is_demo_only_host(host: &str) -> bool {
    let hostname = host.split(':').next().unwrap_or(host);
    hostname == DEMO_ONLY_HOST
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(check_email))
        .routes(routes!(register))
        .routes(routes!(login))
        .routes(routes!(logout))
        .routes(routes!(get_current_user))
        .routes(routes!(accept_terms))
        // Note: /keys routes are handled separately via OpenApiRouter in factory.rs
        .routes(routes!(update_password_auth))
        .routes(routes!(setup))
        .routes(routes!(onboarding_step))
        .routes(routes!(onboarding_state))
        .route("/oidc/providers", get(list_oidc_providers))
        .route("/oidc/{slug}/authorize", get(oidc_authorize))
        .route("/oidc/{slug}/callback", get(oidc_callback))
        .routes(routes!(unlink_oidc_account))
        .routes(routes!(request_email_change))
        .routes(routes!(forgot_password))
        .routes(routes!(reset_password))
        .routes(routes!(verify_email))
        .routes(routes!(resend_verification))
}

pub(crate) mod oidc;
mod password;
pub(crate) mod registration;
mod session;
mod verification;
use oidc::*;
use password::*;
use registration::*;
use session::*;
use verification::*;
