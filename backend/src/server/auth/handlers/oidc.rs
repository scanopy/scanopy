//! OIDC providers, authorize/callback, link/login/register flows, and unlink.
use super::*;
use crate::server::openapi::tags as api_tags;

pub(crate) async fn list_oidc_providers(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<ApiResponse<Vec<OidcProviderMetadata>>>> {
    let oidc_service = state
        .services
        .oidc_service
        .as_ref()
        .ok_or_else(|| ApiError::internal_error("OIDC not configured"))?;

    Ok(Json(ApiResponse::success(oidc_service.list_providers())))
}

pub(crate) async fn oidc_authorize(
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
    let (auth_url, pending_auth) = provider.authorize_url(flow).await.map_err(|e| {
        ApiError::internal_error(&format!(
            "Failed to generate auth URL for '{}': {}.",
            slug, e
        ))
    })?;

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

pub(crate) async fn oidc_callback(
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
            if let Ok(Some(org)) = state
                .services
                .organization_service
                .get_by_id(&user.base.organization_id)
                .await
            {
                let plan = org
                    .base
                    .plan
                    .unwrap_or_else(crate::server::billing::plans::get_free_plan);
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

            // Save user_id + session epoch to session
            if let Err(e) = session
                .insert("session_epoch", user.base.session_epoch)
                .await
                .and(session.insert("user_id", user.id).await)
            {
                tracing::error!("Failed to save session: {}", e);
                return Err(Redirect::to(&format!(
                    "{}?error={}",
                    return_url,
                    urlencoding::encode(&format!("Failed to create session: {}", e))
                )));
            }

            Ok(Redirect::to(&format!(
                "{}?login_method=oidc:{}",
                return_url, slug
            )))
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
    let (org_id, permissions, site_ids) = match process_pending_invite(&state, &session).await {
        Ok(Some((org_id, permissions, site_ids))) => (Some(org_id), Some(permissions), site_ids),
        Ok(_) => (None, None, vec![]),
        Err(e) => {
            return Err(Redirect::to(&format!(
                "{}?error={}",
                return_url,
                urlencoding::encode(&format!("Failed to process invite: {}", e))
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
        return Err(Redirect::to(&format!(
            "{}?error={}&error_code=org_setup_info_missing",
            return_url,
            urlencoding::encode("Organization setup information missing")
        )));
    };

    let billing_enabled = state.config.stripe_secret.is_some();

    // Register user
    match oidc_service
        .register(
            pending_auth,
            LoginRegisterParams {
                provision_org: provision_org.clone(),
                permissions,
                ip,
                user_agent,
                site_ids,
            },
            OidcRegisterParams {
                terms_accepted_at,
                billing_enabled,
                provider_slug: slug,
                code,
                deployment_type: get_deployment_type(&state.config),
                marketing_opt_in,
            },
        )
        .await
    {
        Ok(result) => {
            let user = match result {
                OidcRegisterResult::NewUser(user) => user,
                OidcRegisterResult::ExistingUser(user) => user,
                OidcRegisterResult::EmailAlreadyExists => {
                    return Err(Redirect::to(&format!(
                        "{}?error={}&error_code=user_email_in_use",
                        return_url,
                        urlencoding::encode(
                            "An account with this email already exists. Please sign in instead."
                        )
                    )));
                }
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

            // Save user_id + session epoch to session
            if let Err(e) = session
                .insert("session_epoch", user.base.session_epoch)
                .await
                .and(session.insert("user_id", user.id).await)
            {
                tracing::error!("Failed to save session: {}", e);
                return Err(Redirect::to(&format!(
                    "{}?error={}",
                    return_url,
                    urlencoding::encode(&format!("Failed to create session: {}", e))
                )));
            }

            // If this is a new org, create site/topology
            if let ProvisionOrg::New(setup) = provision_org
                && let Err(e) = apply_pending_setup(&state, &user, setup).await
            {
                tracing::error!("Failed to apply pending setup: {:?}", e);
                // Don't fail registration, just log the error — the user can
                // complete onboarding manually.
            }

            // Clear pending setup data from session (no-op if invite path took
            // ProvisionOrg::Existing; the keys simply weren't set).
            clear_pending_setup(&session).await;

            Ok(Redirect::to(&format!(
                "{}?login_method=oidc:{}",
                return_url, slug
            )))
        }
        Err(e) => {
            tracing::error!("Failed to register via OIDC: {}", e);
            let error_msg = format!("Failed to register: {}", e);
            Err(Redirect::to(&format!(
                "{}?error={}",
                return_url,
                urlencoding::encode(&error_msg),
            )))
        }
    }
}

#[utoipa::path(
    post,
    path = "/oidc/{slug}/unlink",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    params(("slug" = String, Path, description = "OIDC provider slug")),
    responses(
        (status = 200, description = "OIDC account unlinked", body = ApiResponse<User>),
        (status = 401, description = "Not authenticated", body = ApiErrorResponse),
        (status = 403, description = "Blocked in demo mode", body = ApiErrorResponse),
        (status = 404, description = "Provider not found", body = ApiErrorResponse),
    )
)]
pub(crate) async fn unlink_oidc_account(
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
