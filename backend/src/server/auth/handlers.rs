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
            base::{LoginRegisterParams, PendingNetworkSetup, PendingSetup},
            oidc::{OidcFlow, OidcPendingAuth, OidcProviderMetadata, OidcRegisterParams},
        },
        middleware::{
            auth::AuthenticatedEntity,
            permissions::{Authorized, IsUser},
        },
        oidc::{OidcRegisterResult, OidcService},
    },
    config::{AppState, DeploymentType, get_deployment_type},
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        types::{CredentialType, SecretValue},
    },
    daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase},
    invites::handlers::process_pending_invite,
    networks::r#impl::{Network, NetworkBase},
    shared::api_key_common::{ApiKeyType, generate_api_key_for_storage},
    shared::{
        events::types::{OnboardingEvent, OnboardingOperation},
        services::traits::CrudService,
        storage::filter::StorableFilter,
        storage::traits::Storable,
        types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse},
        types::error_codes::ErrorCode,
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
use bad_email::is_email_unwanted;
use chrono::{DateTime, Utc};
use secrecy::SecretString;
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

#[utoipa::path(
    post,
    path = "/check-email",
    tags = ["auth", "internal"],
    request_body = CheckEmailRequest,
    responses(
        (status = 200, description = "Email is available", body = EmptyApiResponse),
        (status = 409, description = "Email already in use", body = ApiErrorResponse),
    )
)]
async fn check_email(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CheckEmailRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    if state.config.disable_password_login {
        return Err(ApiError::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::AuthPasswordLoginDisabled,
        ));
    }

    let existing = state
        .services
        .user_service
        .get_all(StorableFilter::<User>::new_from_email(&request.email))
        .await?;
    if !existing.is_empty() {
        return Err(ApiError::coded(
            StatusCode::CONFLICT,
            ErrorCode::UserEmailInUse {
                email: request.email.to_string(),
            },
        ));
    }
    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/register",
    tags = ["auth", "internal"],
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "User registered successfully", body = ApiResponse<User>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
        (status = 403, description = "Registration disabled", body = ApiErrorResponse),
        (status = 409, description = "Email already exists", body = ApiErrorResponse),
    )
)]
async fn register(
    State(state): State<Arc<AppState>>,
    Host(host): Host,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    session: Session,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<Json<ApiResponse<User>>> {
    // Block registration on dedicated demo domain
    if is_demo_only_host(&host) {
        return Err(ApiError::forbidden(
            "Account creation is disabled on the demo site",
        ));
    }

    if state.config.disable_password_login {
        return Err(ApiError::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::AuthPasswordLoginDisabled,
        ));
    }

    if state.config.disable_registration {
        return Err(ApiError::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::AuthRegistrationDisabled,
        ));
    }

    // Honeypot: hidden "website" field filled = likely bot
    if request.website.as_ref().is_some_and(|w| !w.is_empty()) {
        tracing::warn!(
            ip = %ip,
            email = %request.email,
            honeypot_value = %request.website.as_ref().unwrap(),
            "Bot registration blocked by honeypot"
        );
        let fake_user = User::new(UserBase {
            email: request.email.clone(),
            has_password: true,
            terms_accepted_at: if request.terms_accepted {
                Some(Utc::now())
            } else {
                None
            },
            ..UserBase::default()
        });
        return Ok(Json(ApiResponse::success(fake_user)));
    }

    let billing_enabled = state.config.stripe_secret.is_some();

    if billing_enabled && !request.terms_accepted {
        return Err(ApiError::bad_request(
            "Please accept terms and conditions to proceed",
        ));
    }

    if is_email_unwanted(request.email.as_str())
        && get_deployment_type(state.clone()) == DeploymentType::Cloud
    {
        return Err(ApiError::conflict(
            "Email address uses a disposable domain. Please register with a non-disposable email address.",
        ));
    }

    // Check if email is already in use
    let existing = state
        .services
        .user_service
        .get_all(StorableFilter::<User>::new_from_email(&request.email))
        .await?;
    if !existing.is_empty() {
        return Err(ApiError::coded(
            StatusCode::CONFLICT,
            ErrorCode::UserEmailInUse {
                email: request.email.to_string(),
            },
        ));
    }

    let user_agent = user_agent.map(|u| u.to_string());

    // Check for pending invite
    let (org_id, permissions, network_ids) = match process_pending_invite(&state, &session).await {
        Ok(Some((org_id, permissions, network_ids))) => {
            (Some(org_id), Some(permissions), network_ids)
        }
        Ok(_) => (None, None, vec![]),
        Err(e) => {
            return Err(ApiError::internal_error(&format!(
                "Failed to process invite: {}",
                e
            )));
        }
    };

    // Track if this is a new org (not an invite)
    let is_new_org = org_id.is_none();

    // Extract pending setup from session (only relevant for new orgs)
    let pending_setup = if is_new_org {
        extract_pending_setup(&session).await
    } else {
        None
    };

    let user = state
        .services
        .auth_service
        .register(
            request,
            LoginRegisterParams {
                org_id,
                permissions,
                ip,
                user_agent,
                network_ids,
            },
            pending_setup.clone(),
            billing_enabled,
        )
        .await?;

    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;

    // If this is a new org and setup was provided, create network/topology/daemon
    if is_new_org && let Some(setup) = pending_setup {
        // Apply setup: create network, seed data, topology
        apply_pending_setup(&state, &user, setup).await?;

        // Clear pending setup data from session
        clear_pending_setup(&session).await;
    }

    Ok(Json(ApiResponse::success(user)))
}

/// Store pre-registration setup data (org name, networks, seed preference) in session
#[utoipa::path(
    post,
    path = "/setup",
    tags = ["auth", "internal"],
    request_body = SetupRequest,
    responses(
        (status = 200, description = "Setup data stored", body = ApiResponse<SetupResponse>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    )
)]
async fn setup(
    session: Session,
    Json(request): Json<SetupRequest>,
) -> ApiResult<Json<ApiResponse<SetupResponse>>> {
    // Validate request
    if request.organization_name.trim().is_empty() {
        return Err(ApiError::field_empty("organization_name"));
    }
    if request.organization_name.len() > 100 {
        return Err(ApiError::bad_request(
            "Organization name must be 100 characters or less",
        ));
    }

    let name = request.network.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("Network name cannot be empty"));
    }
    if name.len() > 100 {
        return Err(ApiError::bad_request(
            "Network name must be 100 characters or less",
        ));
    }

    // Validate SNMP configuration
    if request.network.snmp_enabled {
        if request
            .network
            .snmp_community
            .as_ref()
            .is_none_or(|c| c.is_empty())
        {
            return Err(ApiError::bad_request(
                "SNMP community string is required when SNMP is enabled",
            ));
        }
        if let Some(ref community) = request.network.snmp_community
            && community.len() > 256
        {
            return Err(ApiError::bad_request(
                "SNMP community string must be 256 characters or less",
            ));
        }
    }

    let network_id = Uuid::new_v4();
    let network = PendingNetworkSetup {
        name: name.to_string(),
        network_id,
        snmp_enabled: request.network.snmp_enabled,
        snmp_version: request.network.snmp_version.clone(),
        snmp_community: request.network.snmp_community.clone(),
    };

    // Store setup data in session
    let pending_setup = PendingSetup {
        org_name: request.organization_name.trim().to_string(),
        network,
        use_case: None,              // Will be merged from onboarding step
        referral_source: None,       // Will be merged from onboarding step
        referral_source_other: None, // Will be merged from onboarding step
    };

    session
        .insert("pending_setup", pending_setup)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save setup data: {}", e)))?;

    Ok(Json(ApiResponse::success(SetupResponse { network_id })))
}

/// Extract pending setup data from session
/// Also merges in use_case from the onboarding step if present
pub async fn extract_pending_setup(session: &Session) -> Option<PendingSetup> {
    let mut setup: PendingSetup = session.get("pending_setup").await.ok().flatten()?;

    // Merge in use_case from onboarding step if not already set
    if setup.use_case.is_none()
        && let Ok(Some(use_case)) = session.get::<String>("onboarding_use_case").await
    {
        setup.use_case = Some(use_case);
    }

    if setup.referral_source.is_none()
        && let Ok(Some(referral_source)) = session.get::<String>("onboarding_referral_source").await
    {
        setup.referral_source = Some(referral_source);
    }
    if setup.referral_source_other.is_none()
        && let Ok(Some(referral_source_other)) = session
            .get::<String>("onboarding_referral_source_other")
            .await
    {
        setup.referral_source_other = Some(referral_source_other);
    }

    Some(setup)
}

/// Clear all pending setup data from session
pub async fn clear_pending_setup(session: &Session) {
    let _ = session.remove::<PendingSetup>("pending_setup").await;
    let _ = session.remove::<String>("onboarding_step").await;
    let _ = session.remove::<String>("onboarding_use_case").await;
    let _ = session.remove::<String>("onboarding_referral_source").await;
    let _ = session
        .remove::<String>("onboarding_referral_source_other")
        .await;
}

/// Store onboarding step in session
#[utoipa::path(
    post,
    path = "/onboarding-step",
    tags = ["auth", "internal"],
    request_body = OnboardingStepRequest,
    responses(
        (status = 200, description = "Step saved", body = EmptyApiResponse),
    )
)]
async fn onboarding_step(
    session: Session,
    Json(request): Json<OnboardingStepRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    session
        .insert("onboarding_step", request.step)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save onboarding step: {}", e)))?;

    // Also save use_case if provided
    if let Some(use_case) = request.use_case {
        session
            .insert("onboarding_use_case", use_case)
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!("Failed to save onboarding use_case: {}", e))
            })?;
    }

    if let Some(referral_source) = request.referral_source {
        session
            .insert("onboarding_referral_source", referral_source)
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!(
                    "Failed to save onboarding referral_source: {}",
                    e
                ))
            })?;
    }
    if let Some(referral_source_other) = request.referral_source_other {
        session
            .insert("onboarding_referral_source_other", referral_source_other)
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!(
                    "Failed to save onboarding referral_source_other: {}",
                    e
                ))
            })?;
    }

    Ok(Json(ApiResponse::success(())))
}

/// Get current onboarding state from session
#[utoipa::path(
    get,
    path = "/onboarding-state",
    tags = ["auth", "internal"],
    responses(
        (status = 200, description = "Onboarding state", body = ApiResponse<OnboardingStateResponse>),
    )
)]
async fn onboarding_state(
    session: Session,
) -> ApiResult<Json<ApiResponse<OnboardingStateResponse>>> {
    let step: Option<String> = session.get("onboarding_step").await.ok().flatten();
    let use_case: Option<String> = session.get("onboarding_use_case").await.ok().flatten();

    let (org_name, network, network_id) = if let Some(pending_setup) = session
        .get::<PendingSetup>("pending_setup")
        .await
        .ok()
        .flatten()
    {
        let n = &pending_setup.network;
        let network = OnboardingNetworkState {
            id: Some(n.network_id),
            name: n.name.clone(),
            snmp_enabled: n.snmp_enabled,
            snmp_version: n.snmp_version.clone(),
            snmp_community: n.snmp_community.clone(),
        };
        (
            Some(pending_setup.org_name),
            Some(network),
            Some(n.network_id),
        )
    } else {
        (None, None, None)
    };

    Ok(Json(ApiResponse::success(OnboardingStateResponse {
        step,
        use_case,
        org_name,
        network,
        network_id,
    })))
}

/// Apply pending setup after user registration: create network, topology, seed data
/// Org name, onboarding status, and billing plan are now set in provision_user
async fn apply_pending_setup(
    state: &Arc<AppState>,
    user: &User,
    setup: PendingSetup,
) -> Result<(), ApiError> {
    let organization_id = user.base.organization_id;
    let auth_entity: AuthenticatedEntity = user.clone().into();

    let pending_network = &setup.network;

    // Create the network with its pre-generated ID
    let mut network = Network::new(NetworkBase::new(organization_id));
    network.id = pending_network.network_id;
    network.base.name = pending_network.name.clone();

    let network = state
        .services
        .network_service
        .create(network, auth_entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to create network: {}", e)))?;

    state
        .services
        .network_service
        .create_organizational_subnets(network.id, auth_entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to seed data: {}", e)))?;

    // Create default topology
    let topology = Topology::new(TopologyBase::new("My Topology".to_string(), network.id));
    state
        .services
        .topology_service
        .create(topology, auth_entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to create topology: {}", e)))?;

    // Create SNMP credential if enabled
    if pending_network.snmp_enabled
        && let Some(ref community) = pending_network.snmp_community
    {
        let credential_name = format!("{} SNMP Credential", pending_network.name);
        let credential = Credential::new(CredentialBase {
            organization_id,
            name: credential_name,
            credential_type: CredentialType::SnmpV2c {
                community: SecretValue::Inline {
                    value: SecretString::new(community.clone().into()),
                },
            },
            target_ips: None,
            tags: Vec::new(),
        });

        let created_credential = state
            .services
            .credential_service
            .create(credential, auth_entity.clone())
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!("Failed to create credential: {}", e))
            })?;

        // Link credential to network via junction table
        state
            .services
            .credential_service
            .set_network_credentials(&network.id, &[created_credential.id])
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!("Failed to link credential to network: {}", e))
            })?;
    }

    // Handle integrated daemon if configured
    if let Some(integrated_daemon_url) = &state.config.integrated_daemon_url {
        let network_id = setup.network.network_id;
        let (plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::Daemon);

        state
            .services
            .daemon_api_key_service
            .create(
                DaemonApiKey::new(DaemonApiKeyBase {
                    key: hashed,
                    name: "Integrated Daemon API Key".to_string(),
                    last_used: None,
                    expires_at: None,
                    network_id,
                    is_enabled: true,
                    tags: Vec::new(),
                    plaintext: None,
                }),
                AuthenticatedEntity::System,
            )
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!("Failed to create integrated daemon key: {}", e))
            })?;

        state
            .services
            .daemon_service
            .initialize_local_daemon(integrated_daemon_url.clone(), network_id, plaintext)
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!("Failed to initialize local daemon: {}", e))
            })?;
    }

    // Publish telemetry event
    state
        .services
        .event_bus
        .publish_onboarding(OnboardingEvent {
            id: Uuid::new_v4(),
            organization_id,
            operation: OnboardingOperation::OnboardingModalCompleted,
            timestamp: Utc::now(),
            metadata: serde_json::json!({
                "pre_registration_setup": true,
                "network_count": 1
            }),
            authentication: auth_entity,
        })
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to publish telemetry: {}", e)))?;

    Ok(())
}

#[utoipa::path(
    post,
    path = "/login",
    tags = ["auth", "internal"],
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = ApiResponse<User>),
        (status = 401, description = "Invalid credentials", body = ApiErrorResponse),
        (status = 403, description = "Login forbidden", body = ApiErrorResponse),
    )
)]
async fn login(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    Host(host): Host,
    user_agent: Option<TypedHeader<UserAgent>>,
    session: Session,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<ApiResponse<User>>> {
    if state.config.disable_password_login {
        return Err(ApiError::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::AuthPasswordLoginDisabled,
        ));
    }

    let user_agent = user_agent.map(|u| u.to_string());

    let user = state
        .services
        .auth_service
        .login(request, ip, user_agent)
        .await?;

    // Check if user is trying to log into demo account on non-demo and visa versa
    if let Some(organization) = state
        .services
        .organization_service
        .get_by_id(&user.base.organization_id)
        .await?
        && let Some(plan) = organization.base.plan
    {
        if plan.is_demo() && !is_demo_host(&host) {
            return Err(ApiError::forbidden(
                "You can't log in to the demo account on this instance.",
            ));
        } else if !plan.is_demo() && is_demo_only_host(&host) {
            return Err(ApiError::forbidden(
                "You can only log in to the demo account on this instance.",
            ));
        }

    // Couldn't get organization for some reason and user is on demo site - block login
    } else if is_demo_only_host(&host) {
        return Err(ApiError::forbidden(
            "You can only log in to the demo account on this instance.",
        ));
    }

    // Cycle session ID to prevent session fixation attacks
    session
        .cycle_id()
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to cycle session: {}", e)))?;

    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;

    Ok(Json(ApiResponse::success(user)))
}

#[utoipa::path(
    post,
    path = "/logout",
    tags = ["auth", "internal"],
    responses(
        (status = 200, description = "Logout successful", body = EmptyApiResponse),
    )
)]
async fn logout(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    session: Session,
) -> ApiResult<Json<ApiResponse<()>>> {
    if let Ok(Some(user_id)) = session.get::<Uuid>("user_id").await {
        let user_agent = user_agent.map(|u| u.to_string());

        state
            .services
            .auth_service
            .logout(user_id, ip, user_agent)
            .await?;
    }

    session
        .delete()
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to delete session: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/me",
    tags = ["auth", "internal"],
    responses(
        (status = 200, description = "Current user", body = ApiResponse<User>),
        (status = 401, description = "Not authenticated", body = ApiErrorResponse),
    )
)]
async fn get_current_user(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> ApiResult<Json<ApiResponse<User>>> {
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to read session: {}", e)))?
        .ok_or_else(ApiError::not_authenticated)?;

    let user = state
        .services
        .user_service
        .get_by_id(&user_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<User>(user_id))?;

    Ok(Json(ApiResponse::success(user)))
}

#[utoipa::path(
    post,
    path = "/update",
    tags = ["auth", "internal"],
    responses(
        (status = 200, description = "Password updated", body = ApiResponse<User>),
        (status = 401, description = "Not authenticated", body = ApiErrorResponse),
        (status = 403, description = "Blocked in demo mode", body = ApiErrorResponse),
    )
)]
async fn update_password_auth(
    State(state): State<Arc<AppState>>,
    session: Session,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    auth: Authorized<IsUser>,
    Json(request): Json<UpdatePasswordRequest>,
) -> ApiResult<Json<ApiResponse<User>>> {
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to read session: {}", e)))?
        .ok_or_else(ApiError::not_authenticated)?;

    let user_agent = user_agent.map(|u| u.to_string());

    let user = state
        .services
        .auth_service
        .update_password(
            user_id,
            request.current_password,
            request.new_password,
            ip,
            user_agent,
            auth.into_entity(),
        )
        .await?;

    Ok(Json(ApiResponse::success(user)))
}

#[utoipa::path(
    post,
    path = "/request-email-change",
    tags = ["auth", "internal"],
    request_body = RequestEmailChangeRequest,
    responses(
        (status = 200, description = "Verification email sent to new address", body = EmptyApiResponse),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
        (status = 401, description = "Not authenticated", body = ApiErrorResponse),
    )
)]
async fn request_email_change(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    auth: Authorized<IsUser>,
    Json(request): Json<RequestEmailChangeRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_id = auth.require_user_id()?;
    let user_agent = user_agent.map(|u| u.to_string());

    state
        .services
        .auth_service
        .request_email_change(
            user_id,
            request.current_password,
            request.new_email,
            ip,
            user_agent,
        )
        .await?;

    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/forgot-password",
    tags = ["auth", "internal"],
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Password reset email sent", body = EmptyApiResponse),
    )
)]
async fn forgot_password(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    Json(request): Json<ForgotPasswordRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_agent = user_agent.map(|u| u.to_string());

    state
        .services
        .auth_service
        .initiate_password_reset(
            &request.email,
            state.config.public_url.clone(),
            ip,
            user_agent,
        )
        .await?;

    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/reset-password",
    tags = ["auth", "internal"],
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successful", body = ApiResponse<User>),
        (status = 400, description = "Invalid or expired token", body = ApiErrorResponse),
    )
)]
async fn reset_password(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    session: Session,
    Json(request): Json<ResetPasswordRequest>,
) -> ApiResult<Json<ApiResponse<User>>> {
    let user_agent = user_agent.map(|u| u.to_string());

    let user = state
        .services
        .auth_service
        .complete_password_reset(&request.token, &request.password, ip, user_agent)
        .await?;

    // Cycle session ID to prevent session fixation attacks
    session
        .cycle_id()
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to cycle session: {}", e)))?;

    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;

    Ok(Json(ApiResponse::success(user)))
}

#[utoipa::path(
    post,
    path = "/verify-email",
    tags = ["auth", "internal"],
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified successfully", body = ApiResponse<User>),
        (status = 400, description = "Invalid or expired token", body = ApiErrorResponse),
    )
)]
async fn verify_email(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    session: Session,
    Json(request): Json<VerifyEmailRequest>,
) -> ApiResult<Json<ApiResponse<User>>> {
    let user_agent = user_agent.map(|u| u.to_string());

    let user = state
        .services
        .auth_service
        .verify_email(&request.token, ip, user_agent)
        .await?;

    // Cycle session ID to prevent session fixation attacks
    session
        .cycle_id()
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to cycle session: {}", e)))?;

    // Auto-login user after successful verification
    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;

    Ok(Json(ApiResponse::success(user)))
}

#[utoipa::path(
    post,
    path = "/resend-verification",
    tags = ["auth", "internal"],
    request_body = ResendVerificationRequest,
    responses(
        (status = 200, description = "Verification email sent", body = EmptyApiResponse),
        (status = 400, description = "Invalid request or already verified", body = ApiErrorResponse),
        (status = 429, description = "Rate limited", body = ApiErrorResponse),
    )
)]
async fn resend_verification(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ResendVerificationRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    state
        .services
        .auth_service
        .resend_verification_email(&request.email)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

async fn list_oidc_providers(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ApiResponse<Vec<OidcProviderMetadata>>>> {
    let oidc_service = state
        .services
        .oidc_service
        .as_ref()
        .ok_or_else(|| ApiError::internal_error("OIDC not configured"))?;

    Ok(Json(ApiResponse::success(oidc_service.list_providers())))
}

async fn oidc_authorize(
    State(state): State<Arc<AppState>>,
    Host(host): Host,
    Path(slug): Path<String>,
    session: Session,
    Query(params): Query<OidcAuthorizeParams>,
) -> ApiResult<Redirect> {
    let billing_enabled = state.config.stripe_secret.is_some();

    let oidc_service = state
        .services
        .oidc_service
        .as_ref()
        .ok_or_else(|| ApiError::internal_error("OIDC not configured"))?;

    // Verify provider exists
    let provider = oidc_service
        .get_provider(&slug)
        .ok_or_else(|| ApiError::not_found(format!("OIDC provider '{}' not found", slug)))?;

    // Parse and validate flow parameter
    let flow = match params.flow.as_deref() {
        Some("login") => OidcFlow::Login,
        Some("register") => {
            // Block registration on dedicated demo domain
            if is_demo_only_host(&host) {
                return Err(ApiError::forbidden(
                    "Account creation is disabled on the demo site",
                ));
            }

            if state.config.disable_registration {
                return Err(ApiError::coded(
                    StatusCode::FORBIDDEN,
                    ErrorCode::AuthRegistrationDisabled,
                ));
            }

            let terms_accepted = params.terms_accepted.unwrap_or(false);

            if billing_enabled && !terms_accepted {
                return Err(ApiError::bad_request(
                    "Please accept terms and conditions to proceed",
                ));
            }

            OidcFlow::Register
        }
        Some("link") => OidcFlow::Link,
        Some(other) => {
            return Err(ApiError::bad_request(&format!(
                "Invalid flow '{}'. Must be 'login', 'register', or 'link'",
                other
            )));
        }
        None => {
            return Err(ApiError::bad_request(
                "flow parameter is required (login, register, or link)",
            ));
        }
    };

    // Validate return_url is present
    let return_url = params
        .return_url
        .ok_or_else(|| ApiError::bad_request("return_url parameter is required"))?;

    // Generate authorization URL using provider
    let (auth_url, pending_auth) = provider
        .authorize_url(flow)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to generate auth URL for '{}': {}. Check that the issuer URL is reachable from the server.", slug, e)))?;

    // Store OIDC flow state in session
    session
        .insert("oidc_pending_auth", pending_auth)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save pending auth: {}", e)))?;

    session
        .insert("oidc_provider_slug", slug)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save provider slug: {}", e)))?;

    session
        .insert("oidc_return_url", return_url)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save return URL: {}", e)))?;

    // Store registration flags if present

    if let Some(terms_accepted) = params.terms_accepted {
        session
            .insert("oidc_terms_accepted", terms_accepted)
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!("Failed to save terms_accepted_at: {}", e))
            })?;
    }

    if let Some(marketing_opt_in) = params.marketing_opt_in {
        session
            .insert("oidc_marketing_opt_in", marketing_opt_in)
            .await
            .map_err(|e| {
                ApiError::internal_error(&format!("Failed to save marketing_opt_in: {}", e))
            })?;
    }

    Ok(Redirect::to(&auth_url))
}

async fn oidc_callback(
    State(state): State<Arc<AppState>>,
    Host(host): Host,
    Path(slug): Path<String>,
    session: Session,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    Query(params): Query<OidcCallbackParams>,
) -> Result<Redirect, Redirect> {
    let user_agent = user_agent.map(|u| u.to_string());

    // Verify OIDC is configured
    let oidc_service = match state.services.oidc_service.as_ref() {
        Some(service) => service,
        None => {
            return Err(Redirect::to(&format!(
                "/error?message={}",
                urlencoding::encode("OIDC is not configured on this server")
            )));
        }
    };

    // Verify provider exists
    if oidc_service.get_provider(&slug).is_none() {
        return Err(Redirect::to(&format!(
            "/error?message={}",
            urlencoding::encode(&format!("OIDC provider '{}' not found", slug))
        )));
    }

    // Extract and validate session data
    let return_url: String = session
        .get("oidc_return_url")
        .await
        .ok()
        .flatten()
        .ok_or_else(|| {
            Redirect::to(&format!(
                "/error?message={}",
                urlencoding::encode("Session error: No return URL found")
            ))
        })?;

    let pending_auth: OidcPendingAuth = session
        .get("oidc_pending_auth")
        .await
        .ok()
        .flatten()
        .ok_or_else(|| {
            Redirect::to(&format!(
                "{}?error={}",
                return_url,
                urlencoding::encode("No pending authentication found. Please try again.")
            ))
        })?;

    let session_slug: String = session
        .get("oidc_provider_slug")
        .await
        .ok()
        .flatten()
        .ok_or_else(|| {
            Redirect::to(&format!(
                "{}?error={}",
                return_url,
                urlencoding::encode("Session error: No provider slug found")
            ))
        })?;

    // Verify provider slug matches
    if session_slug != slug {
        return Err(Redirect::to(&format!(
            "{}?error={}",
            return_url,
            urlencoding::encode("Provider mismatch in callback")
        )));
    }

    // Verify CSRF token
    if pending_auth.csrf_token != params.state {
        return Err(Redirect::to(&format!(
            "{}?error={}",
            return_url,
            urlencoding::encode("Invalid security token. Please try again.")
        )));
    }

    // Extract registration flags before consuming OIDC state (needed for Register flow)
    let terms_accepted: bool = session
        .get("oidc_terms_accepted")
        .await
        .ok()
        .flatten()
        .unwrap_or(false);
    let marketing_opt_in: bool = session
        .get("oidc_marketing_opt_in")
        .await
        .ok()
        .flatten()
        .unwrap_or(false);

    // Consume OIDC state immediately to prevent replay attacks
    let _ = session.remove::<OidcPendingAuth>("oidc_pending_auth").await;
    let _ = session.remove::<String>("oidc_provider_slug").await;
    let _ = session.remove::<String>("oidc_return_url").await;
    let _ = session.remove::<bool>("oidc_terms_accepted").await;
    let _ = session.remove::<bool>("oidc_marketing_opt_in").await;

    // Parse return URL for error handling
    let return_url_parsed = Url::parse(&return_url).map_err(|_| {
        Redirect::to(&format!(
            "/error?message={}",
            urlencoding::encode("Invalid return URL")
        ))
    })?;

    // Handle different flows
    match pending_auth.flow {
        OidcFlow::Link => {
            handle_link_flow(HandleLinkFlowParams {
                oidc_service,
                slug: &slug,
                code: &params.code,
                pending_auth,
                ip,
                user_agent,
                session,
                return_url: return_url_parsed,
                host,
            })
            .await
        }
        OidcFlow::Login => {
            handle_login_flow(
                state.clone(),
                HandleLinkFlowParams {
                    oidc_service,
                    slug: &slug,
                    code: &params.code,
                    pending_auth,
                    ip,
                    user_agent,
                    session,
                    return_url: return_url_parsed,
                    host,
                },
            )
            .await
        }
        OidcFlow::Register => {
            let terms_accepted_at = if terms_accepted {
                Some(Utc::now())
            } else {
                None
            };

            handle_register_flow(
                state.clone(),
                terms_accepted_at,
                marketing_opt_in,
                HandleLinkFlowParams {
                    oidc_service,
                    slug: &slug,
                    code: &params.code,
                    pending_auth,
                    ip,
                    user_agent,
                    session,
                    return_url: return_url_parsed,
                    host,
                },
            )
            .await
        }
    }
}

struct HandleLinkFlowParams<'a> {
    oidc_service: &'a OidcService,
    slug: &'a str,
    code: &'a str,
    pending_auth: OidcPendingAuth,
    ip: IpAddr,
    user_agent: Option<String>,
    session: Session,
    return_url: Url,
    host: String,
}

async fn handle_link_flow(params: HandleLinkFlowParams<'_>) -> Result<Redirect, Redirect> {
    let HandleLinkFlowParams {
        oidc_service,
        slug,
        code,
        pending_auth,
        ip,
        user_agent,
        session,
        mut return_url,
        host: _,
    } = params;

    // Add modal query param to return URL so settings modal opens
    return_url
        .query_pairs_mut()
        .append_pair("modal", "settings");

    // Verify user is logged in
    let user_id: Uuid = session.get("user_id").await.ok().flatten().ok_or_else(|| {
        let mut url = return_url.clone();
        url.query_pairs_mut()
            .append_pair("error", "You must be logged in to link an OIDC account.");
        Redirect::to(url.as_str())
    })?;

    // Link OIDC account to user
    match oidc_service
        .link_to_user(slug, &user_id, code, pending_auth, ip, user_agent)
        .await
    {
        Ok(_) => Ok(Redirect::to(return_url.as_str())),
        Err(e) => {
            tracing::error!("Failed to link OIDC: {}", e);

            return_url
                .query_pairs_mut()
                .append_pair("error", &format!("Failed to link OIDC account: {}", e));
            Err(Redirect::to(return_url.as_str()))
        }
    }
}

async fn handle_login_flow(
    state: Arc<AppState>,
    params: HandleLinkFlowParams<'_>,
) -> Result<Redirect, Redirect> {
    let HandleLinkFlowParams {
        oidc_service,
        slug,
        code,
        pending_auth,
        ip,
        user_agent,
        session,
        return_url,
        host,
    } = params;

    // Login user
    match oidc_service
        .login(slug, code, pending_auth, ip, user_agent)
        .await
    {
        Ok(user) => {
            // Validate host matches user's org plan (same as regular login)
            if let Ok(Some(organization)) = state
                .services
                .organization_service
                .get_by_id(&user.base.organization_id)
                .await
                && let Some(plan) = organization.base.plan
            {
                if plan.is_demo() && !is_demo_host(&host) {
                    return Err(Redirect::to(&format!(
                        "{}?error={}",
                        return_url,
                        urlencoding::encode(
                            "You can't log in to the demo account on this instance."
                        )
                    )));
                } else if !plan.is_demo() && is_demo_only_host(&host) {
                    return Err(Redirect::to(&format!(
                        "{}?error={}",
                        return_url,
                        urlencoding::encode(
                            "You can only log in to the demo account on this instance."
                        )
                    )));
                }
            } else if is_demo_only_host(&host) {
                // Couldn't get organization - block login on demo site
                return Err(Redirect::to(&format!(
                    "{}?error={}",
                    return_url,
                    urlencoding::encode(
                        "You can only log in to the demo account on this instance."
                    )
                )));
            }

            // Cycle session ID to prevent session fixation attacks
            if let Err(e) = session.cycle_id().await {
                tracing::error!("Failed to cycle session ID: {}", e);
                return Err(Redirect::to(&format!(
                    "{}?error={}",
                    return_url,
                    urlencoding::encode(&format!("Failed to create session: {}", e))
                )));
            }

            // Save user_id to session
            if let Err(e) = session.insert("user_id", user.id).await {
                tracing::error!("Failed to save session: {}", e);
                return Err(Redirect::to(&format!(
                    "{}?error={}",
                    return_url,
                    urlencoding::encode(&format!("Failed to create session: {}", e))
                )));
            }

            Ok(Redirect::to(return_url.as_str()))
        }
        Err(e) => {
            tracing::error!("Failed to login via OIDC: {}", e);
            Err(Redirect::to(&format!(
                "{}?error={}",
                return_url,
                urlencoding::encode(&format!("Failed to login: {}", e))
            )))
        }
    }
}

async fn handle_register_flow(
    state: Arc<AppState>,
    terms_accepted_at: Option<DateTime<Utc>>,
    marketing_opt_in: bool,
    params: HandleLinkFlowParams<'_>,
) -> Result<Redirect, Redirect> {
    let HandleLinkFlowParams {
        oidc_service,
        slug,
        code,
        pending_auth,
        ip,
        user_agent,
        session,
        return_url,
        host: _,
    } = params;

    // Process pending invite if present
    let (org_id, permissions, network_ids) = match process_pending_invite(&state, &session).await {
        Ok(Some((org_id, permissions, network_ids))) => {
            (Some(org_id), Some(permissions), network_ids)
        }
        Ok(_) => (None, None, vec![]),
        Err(e) => {
            return Err(Redirect::to(&format!(
                "{}?error={}",
                return_url,
                urlencoding::encode(&format!("Failed to process invite: {}", e))
            )));
        }
    };

    // Track if this is a new org (not an invite)
    let is_new_org = org_id.is_none();

    // Extract pending setup from session (only relevant for new orgs)
    let pending_setup = if is_new_org {
        extract_pending_setup(&session).await
    } else {
        None
    };

    let billing_enabled = state.config.stripe_secret.is_some();

    // Register user
    match oidc_service
        .register(
            pending_auth,
            LoginRegisterParams {
                org_id,
                permissions,
                ip,
                user_agent,
                network_ids,
            },
            OidcRegisterParams {
                terms_accepted_at,
                billing_enabled,
                provider_slug: slug,
                code,
                deployment_type: get_deployment_type(state.clone()),
                marketing_opt_in,
            },
            pending_setup.clone(),
        )
        .await
    {
        Ok(result) => {
            let (user, is_new_user) = match result {
                OidcRegisterResult::NewUser(user) => (user, true),
                OidcRegisterResult::ExistingUser(user) => (user, false),
            };

            // Cycle session ID to prevent session fixation attacks
            if let Err(e) = session.cycle_id().await {
                tracing::error!("Failed to cycle session ID: {}", e);
                return Err(Redirect::to(&format!(
                    "{}?error={}",
                    return_url,
                    urlencoding::encode(&format!("Failed to create session: {}", e))
                )));
            }

            // Save user_id to session
            if let Err(e) = session.insert("user_id", user.id).await {
                tracing::error!("Failed to save session: {}", e);
                return Err(Redirect::to(&format!(
                    "{}?error={}",
                    return_url,
                    urlencoding::encode(&format!("Failed to create session: {}", e))
                )));
            }

            // Only apply pending setup for new users in new orgs
            if is_new_user
                && is_new_org
                && let Some(setup) = pending_setup
                && let Err(e) = apply_pending_setup(&state, &user, setup).await
            {
                tracing::error!("Failed to apply pending setup: {:?}", e);
                // Don't fail registration, just log the error
                // The user can complete onboarding manually
            }

            // Clear pending setup data from session
            clear_pending_setup(&session).await;

            Ok(Redirect::to(return_url.as_str()))
        }
        Err(e) => {
            tracing::error!("Failed to register via OIDC: {}", e);
            let error_msg = format!("Failed to register: {}", e);
            let err_str = e.to_string();
            let error_code = if err_str.contains("already exists") {
                "&error_code=user_email_in_use"
            } else {
                ""
            };
            Err(Redirect::to(&format!(
                "{}?error={}{}",
                return_url,
                urlencoding::encode(&error_msg),
                error_code
            )))
        }
    }
}

#[utoipa::path(
    post,
    path = "/oidc/{slug}/unlink",
    tags = ["auth", "internal"],
    params(("slug" = String, Path, description = "OIDC provider slug")),
    responses(
        (status = 200, description = "OIDC account unlinked", body = ApiResponse<User>),
        (status = 401, description = "Not authenticated", body = ApiErrorResponse),
        (status = 403, description = "Blocked in demo mode", body = ApiErrorResponse),
        (status = 404, description = "Provider not found", body = ApiErrorResponse),
    )
)]
async fn unlink_oidc_account(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    session: Session,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
) -> ApiResult<Json<ApiResponse<User>>> {
    let user_agent = user_agent.map(|u| u.to_string());

    let oidc_service = state
        .services
        .oidc_service
        .as_ref()
        .ok_or_else(|| ApiError::internal_error("OIDC not configured"))?;

    // Verify provider exists
    if oidc_service.get_provider(&slug).is_none() {
        return Err(ApiError::not_found(format!(
            "OIDC provider '{}' not found",
            slug
        )));
    }

    // Get user_id from session
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to read session: {}", e)))?
        .ok_or_else(ApiError::not_authenticated)?;

    // Unlink OIDC account
    let updated_user = oidc_service
        .unlink_from_user(&slug, &user_id, ip, user_agent)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to unlink OIDC: {}", e)))?;

    Ok(Json(ApiResponse::success(updated_user)))
}
