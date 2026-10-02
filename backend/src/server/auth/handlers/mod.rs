use crate::daemon::shared::config::DEFAULT_DAEMON_NAME;
use crate::server::{
    auth::{
        r#impl::{
            api::{
                CheckEmailRequest, ForgotPasswordRequest, LoginRequest, OidcAuthorizeParams,
                OidcCallbackParams, OnboardingNetworkState, OnboardingStateResponse,
                OnboardingStepRequest, RegisterRequest, RequestEmailChangeRequest,
                ResendVerificationRequest, ResetPasswordRequest, SetupRequest, SetupResponse,
                UpdatePasswordRequest, VerifyEmailRequest,
            },
            base::{LoginRegisterParams, PendingNetworkSetup, PendingSetup, ProvisionOrg},
            oidc::{OidcFlow, OidcPendingAuth, OidcProviderMetadata, OidcRegisterParams},
        },
        middleware::{
            auth::AuthenticatedEntity,
            permissions::{Authorized, IsUser},
        },
        oidc::{OidcRegisterResult, OidcService},
    },
    config::{AppState, DeploymentType, get_deployment_type},
    daemons::r#impl::{
        api::ProvisionDaemonRequest,
        base::{DaemonMode, DaemonOs},
    },
    invites::handlers::process_pending_invite,
    networks::r#impl::{Network, NetworkBase},
    organizations::r#impl::base::UseCase,
    shared::{
        events::{
            traits::{Event, OrgScope},
            types::OnboardingOperation,
        },
        services::traits::CrudService,
        storage::{filter::StorableFilter, traits::Storable},
        types::{
            api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse},
            error_codes::ErrorCode,
        },
    },
    topology::types::base::{Topology, TopologyBase},
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

mod oidc;
mod password;
mod registration;
mod session;
mod verification;
use oidc::*;
use password::*;
use registration::*;
use session::*;
use verification::*;
#[cfg(test)]
mod tests {
    #[test]
    fn test_legitimate_regional_domains_not_blocked() {
        let legitimate = [
            "user@yahoo.co.jp",
            "user@yahoo.co.uk",
            "user@yahoo.ca",
            "user@hotmail.co.uk",
            "user@hotmail.it",
            "user@outlook.com",
            "user@gmail.com",
            "user@protonmail.com",
        ];
        for email in legitimate {
            assert!(
                mailchecker::is_valid(email),
                "{} should not be blocked as disposable",
                email
            );
        }
    }

    #[test]
    fn test_disposable_domains_blocked() {
        let disposable = [
            "user@mailinator.com",
            "user@guerrillamail.com",
            "user@yopmail.com",
            "user@sharklasers.com",
            "user@trashmail.com",
        ];
        for email in disposable {
            assert!(
                !mailchecker::is_valid(email),
                "{} should be blocked as disposable",
                email
            );
        }
    }
}
