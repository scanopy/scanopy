//! Email check, registration, onboarding setup, and pending-setup application.
use super::*;
use crate::server::auth::email_domain::{DomainCheck, check_email_domain};
use crate::server::auth::r#impl::api::CheckEmailResponse;
use crate::server::openapi::tags as api_tags;

#[utoipa::path(
    post,
    path = "/check-email",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    request_body = CheckEmailRequest,
    responses(
        (status = 200, description = "Whether the address is available", body = ApiResponse<CheckEmailResponse>),
        (status = 403, description = "Password login is disabled", body = ApiErrorResponse),
    )
)]
pub(crate) async fn check_email(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CheckEmailRequest>,
) -> ApiResult<Json<ApiResponse<CheckEmailResponse>>> {
    // A genuine refusal, unlike a taken address: this deployment does not do
    // password registration at all, so there is no question to answer.
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

    // Answered, not failed. `register` still returns `UserEmailInUse` for a real
    // attempt; this endpoint only reports what the caller asked.
    Ok(Json(ApiResponse::success(CheckEmailResponse {
        available: existing.is_empty(),
    })))
}

#[utoipa::path(
    post,
    path = "/register",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "User registered successfully", body = ApiResponse<User>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
        (status = 403, description = "Registration disabled", body = ApiErrorResponse),
        (status = 409, description = "Email already exists", body = ApiErrorResponse),
    )
)]
pub(crate) async fn register(
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

    // Honeypot: hidden field filled = likely bot (cloud only)
    if get_deployment_type(&state.config) == DeploymentType::Cloud
        && request.website.as_ref().is_some_and(|w| !w.is_empty())
    {
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

    if !mailchecker::is_valid(request.email.as_str())
        && get_deployment_type(&state.config) == DeploymentType::Cloud
    {
        return Err(ApiError::conflict(
            "Email address uses a disposable domain. Please register with a non-disposable email address.",
        ));
    }

    // DNS deliverability gate (cloud only): reject domains that definitively
    // cannot receive mail (no MX/A/AAAA). Transient DNS failures fail open.
    if get_deployment_type(&state.config) == DeploymentType::Cloud
        && check_email_domain(request.email.domain()).await == DomainCheck::Undeliverable
    {
        return Err(ApiError::coded(
            StatusCode::CONFLICT,
            ErrorCode::ValidationEmailDomainUndeliverable,
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

    let provision_org = if let Some(org_id) = org_id {
        ProvisionOrg::Existing(org_id)
    } else if let Ok(Some(mut pending_setup)) = session.get::<PendingSetup>("pending_setup").await {
        // Read the latest use_case from session (set via /onboarding-step) so
        // a frontend update after /setup is reflected at register time.
        if let Some(use_case) = session
            .get::<UseCase>("onboarding_use_case")
            .await
            .ok()
            .flatten()
        {
            pending_setup.use_case = use_case;
        }
        ProvisionOrg::New(pending_setup)
    } else {
        return Err(ApiError::internal_error(
            "Organization setup information missing",
        ));
    };

    let user = state
        .services
        .auth_service
        .register(
            request,
            LoginRegisterParams {
                provision_org: provision_org.clone(),
                permissions,
                ip,
                user_agent,
                network_ids,
            },
            billing_enabled,
        )
        .await?;

    session
        .insert("session_epoch", user.base.session_epoch)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;
    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;

    // If this is a new org, create network/topology/daemon
    if let ProvisionOrg::New(setup) = provision_org {
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
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    request_body = SetupRequest,
    responses(
        (status = 200, description = "Setup data stored", body = ApiResponse<SetupResponse>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    )
)]
pub(crate) async fn setup(
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

    let network_id = Uuid::new_v4();
    let network = PendingNetworkSetup {
        name: name.to_string(),
        network_id,
    };

    // Store setup data in session. `use_case` is read fresh from session at
    // register time (not snapshotted here) so that subsequent calls to
    // `/onboarding-step` can update it without `/setup` being re-called.
    let pending_setup = PendingSetup {
        org_name: request.organization_name.trim().to_string(),
        network,
        use_case: UseCase::Other,
    };

    session
        .insert("pending_setup", pending_setup)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save setup data: {}", e)))?;

    Ok(Json(ApiResponse::success(SetupResponse { network_id })))
}

/// Clear all pending setup data from session
pub async fn clear_pending_setup(session: &Session) {
    let _ = session.remove::<PendingSetup>("pending_setup").await;
    let _ = session.remove::<String>("onboarding_step").await;
    let _ = session.remove::<String>("onboarding_use_case").await;
}

/// Store onboarding step in session
#[utoipa::path(
    post,
    path = "/onboarding-step",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    request_body = OnboardingStepRequest,
    responses(
        (status = 200, description = "Step saved", body = EmptyApiResponse),
    )
)]
pub(crate) async fn onboarding_step(
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

    Ok(Json(ApiResponse::success(())))
}

/// Get current onboarding state from session
#[utoipa::path(
    get,
    path = "/onboarding-state",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    responses(
        (status = 200, description = "Onboarding state", body = ApiResponse<OnboardingStateResponse>),
    )
)]
pub(crate) async fn onboarding_state(
    session: Session,
) -> ApiResult<Json<ApiResponse<OnboardingStateResponse>>> {
    let step: Option<String> = session.get("onboarding_step").await.ok().flatten();
    let use_case: Option<UseCase> = session.get("onboarding_use_case").await.ok().flatten();

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
pub(crate) async fn apply_pending_setup(
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

    // Create default topology (live view, snapshot_id = None)
    let topology = Topology::new(TopologyBase::new(network.id));
    state
        .services
        .topology_service
        .create(topology, auth_entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to create topology: {}", e)))?;

    // Handle integrated daemon if configured.
    //
    // The integrated daemon is provisioned, not handed a shared network key: since 0.17.5 a
    // daemon that reaches /register without resolving to an existing record is rejected
    // (`daemon_not_provisioned`), so a shared key would leave it permanently unregistered.
    // Provisioning creates its record + a 1:1 key up front; the daemon then learns its identity
    // from that key on first contact. Legacy (< 0.17.5) daemons still self-register with their
    // existing shared keys — those are untouched here.
    if let Some(integrated_daemon_url) = &state.config.integrated_daemon_url {
        let network_id = setup.network.network_id;

        let (_daemon, plaintext) = state
            .services
            .daemon_service
            .provision(
                &ProvisionDaemonRequest {
                    name: Some(DEFAULT_DAEMON_NAME.to_string()),
                    network_id: Some(network_id),
                    // The integrated daemon dials the server, so it needs no reachable url.
                    mode: DaemonMode::DaemonPoll,
                    url: None,
                    seed_credential_refs: Vec::new(),
                    daemon_id: None,
                },
                None,
                auth_entity.clone(),
            )
            .await?;

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
        .publish(Event::new(
            OrgScope { organization_id },
            OnboardingOperation::OnboardingModalCompleted,
            auth_entity,
        ))
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to publish telemetry: {}", e)))?;

    Ok(())
}
