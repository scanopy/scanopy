//! Email chck, registration, onboarding setup, and pending-setup application.
use super::*;
use crate::server::auth::disposable_email::is_disposable_email;
use crate::server::auth::email_domain::{DomainCheck, check_email_domain};
use crate::server::auth::r#impl::api::CheckEmailResponse;
use crate::server::auth::r#impl::base::SignupAnswers;
use crate::server::openapi::tags as api_tags;
use crate::server::organizations::r#impl::intent::{
    OTHER_ANSWER_MAX_CHARS, OTHER_ANSWER_MIN_CHARS, other_answer,
};
use crate::server::shared::types::api::ApiJson;

/// Check whether an email is available
#[utoipa::path(
    post,
    path = "/check-email",
    tags = [api_tags::INTERNAL],
    request_body = CheckEmailRequest,
    responses(
        (status = 200, description = "Whether the address is available", body = ApiResponse<CheckEmailResponse>),
        (status = 403, description = "Password login is disabled", body = ApiErrorResponse),
    )
)]
pub(crate) async fn check_email(
    State(state): State<Arc<AppState>>,
    ApiJson(request): ApiJson<CheckEmailRequest>,
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

/// Register user
#[utoipa::path(
    post,
    path = "/register",
    tags = [api_tags::INTERNAL],
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
    ApiJson(request): ApiJson<RegisterRequest>,
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

    ensure_registration_allowed(&state, &session).await?;

    let deployment_type = get_deployment_type(&state.config);

    // Honeypot: hidden field filled = likely bot (cloud only)
    if deployment_type == DeploymentType::Cloud
        && request.website.as_ref().is_some_and(|w| !w.is_empty())
    {
        tracing::warn!(
            ip = %ip,
            email_domain = %request.email.domain(),
            honeypot_value = %request.website.as_ref().unwrap(),
            "Bot registration blocked by honeypot"
        );
        let fake_user = User::new(UserBase {
            email: request.email.clone(),
            has_password: true,
            terms_accepted_at: if request.terms_accepted {
                Some(crate::server::shared::clock::now())
            } else {
                None
            },
            ..UserBase::default()
        });
        return Ok(Json(ApiResponse::success(fake_user)));
    }

    if deployment_type.requires_terms_acceptance() && !request.terms_accepted {
        return Err(ApiError::bad_request(
            "Please accept terms and conditions to proceed",
        ));
    }

    if deployment_type == DeploymentType::Cloud && is_disposable_email(&request.email) {
        return Err(ApiError::conflict(
            "Email address uses a disposable domain. Please register with a non-disposable email address.",
        ));
    }

    // DNS deliverability gate (cloud only): reject domains that definitively
    // cannot receive mail (no MX/A/AAAA). Transient DNS failures fail open.
    if deployment_type == DeploymentType::Cloud
        && check_email_domain(request.email.domain()).await == DomainCheck::Undeliverable
    {
        return Err(ApiError::coded(
            StatusCode::CONFLICT,
            ErrorCode::ValidationEmailDomainUndeliverable,
        ));
    }

    // SES address validation (when configured): rejects low-validity, disposable and
    // random-looking addresses. Any SES failure lets the signup through.
    if let Some(validator) = &state.services.email_validator
        && let Some(code) = validator.check(&request.email).await
    {
        return Err(ApiError::validation(code));
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
    let (org_id, permissions, site_assignments, invited_as) =
        match process_pending_invite(&state, &session).await {
            Ok(Some(invite)) => (
                Some(invite.organization_id),
                Some(invite.permissions),
                invite.site_assignments,
                invite.send_to,
            ),
            Ok(_) => (None, None, vec![], None),
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
        apply_onboarding_answers(&session, &mut pending_setup).await;
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
                site_assignments,
                invited_as,
            },
            deployment_type,
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

    // A new org's pending setup was applied in `provision_user`.
    if matches!(provision_org, ProvisionOrg::New(_)) {
        clear_pending_setup(&session).await;
    }

    // An invitee joining a license-plan org on a link-only invite starts gated.
    let user = with_org_session_gates(&state, user).await?;
    Ok(Json(ApiResponse::success(user)))
}

/// Refuse sign-up when registration is disabled, unless the session holds a pending invite
/// that is still valid. Fails closed: no invite, an expired or deleted one, or an error
/// reading either is a refusal.
pub(crate) async fn ensure_registration_allowed(
    state: &AppState,
    session: &Session,
) -> Result<(), ApiError> {
    if !state.config.disable_registration {
        return Ok(());
    }

    let refused = || ApiError::coded(StatusCode::FORBIDDEN, ErrorCode::AuthRegistrationDisabled);

    let invite_id = session
        .get::<Uuid>("pending_invite_id")
        .await
        .ok()
        .flatten()
        .ok_or_else(refused)?;

    state
        .services
        .invite_service
        .get_valid_invite(invite_id)
        .await
        .map(|_| ())
        .map_err(|_| refused())
}

/// Store pre-registration setup in session
///
/// The organization name and first site, kept until registration completes.
#[utoipa::path(
    post,
    path = "/setup",
    tags = [api_tags::INTERNAL],
    request_body = SetupRequest,
    responses(
        (status = 200, description = "Setup data stored", body = ApiResponse<SetupResponse>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    )
)]
pub(crate) async fn setup(
    State(state): State<Arc<AppState>>,
    session: Session,
    ApiJson(request): ApiJson<SetupRequest>,
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

    let site = validate_setup_site(request.site.as_ref(), get_deployment_type(&state.config))?;
    let site_id = site.as_ref().map(|n| n.site_id);

    // Store setup data in session. `use_case` is read fresh from session at
    // register time (not snapshotted here) so that subsequent calls to
    // `/onboarding-step` can update it without `/setup` being re-called.
    let pending_setup = PendingSetup {
        org_name: request.organization_name.trim().to_string(),
        site,
        use_case: UseCase::Other,
        answers: SignupAnswers::default(),
    };

    session
        .insert("pending_setup", pending_setup)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save setup data: {}", e)))?;

    Ok(Json(ApiResponse::success(SetupResponse { site_id })))
}

/// Validate the requested first site and assign its id.
///
/// The site may be omitted only on cloud, where a self-hosted license buyer
/// gets one if the org later moves to a cloud plan. A self-hosted instance has
/// no billing events to create it later, so it always needs one.
fn validate_setup_site(
    site: Option<&SiteSetup>,
    deployment_type: DeploymentType,
) -> Result<Option<PendingSiteSetup>, ApiError> {
    let Some(site) = site else {
        if deployment_type == DeploymentType::Cloud {
            return Ok(None);
        }
        return Err(ApiError::bad_request("Site name cannot be empty"));
    };

    let name = site.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("Site name cannot be empty"));
    }
    if name.len() > 100 {
        return Err(ApiError::bad_request(
            "Site name must be 100 characters or less",
        ));
    }

    Ok(Some(PendingSiteSetup {
        name: name.to_string(),
        site_id: Uuid::new_v4(),
    }))
}

/// Clear all pending setup data from session
pub async fn clear_pending_setup(session: &Session) {
    let _ = session.remove::<PendingSetup>("pending_setup").await;
    let _ = session.remove::<String>("onboarding_step").await;
    let _ = session.remove::<String>("onboarding_use_case").await;
    let _ = session
        .remove::<SignupAnswers>(ONBOARDING_ANSWERS_KEY)
        .await;
}

/// Session key for the answers `/onboarding-step` saves beside the use case.
const ONBOARDING_ANSWERS_KEY: &str = "onboarding_answers";

/// Fill the pending setup with the answers saved by `/onboarding-step`, read at register time so
/// an answer changed after `/setup` still counts. Free text is kept only where "Something else"
/// was picked. Both the password and the OIDC register paths call this.
pub async fn apply_onboarding_answers(session: &Session, pending_setup: &mut PendingSetup) {
    if let Some(use_case) = session
        .get::<UseCase>("onboarding_use_case")
        .await
        .ok()
        .flatten()
    {
        pending_setup.use_case = use_case;
    }
    let answers: SignupAnswers = session
        .get(ONBOARDING_ANSWERS_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    pending_setup.answers = answers.for_use_case(pending_setup.use_case);
}

/// The answers in an `/onboarding-step` request, with free text trimmed. A text answer outside
/// 3 to 200 characters is refused.
fn answers_from_request(request: &OnboardingStepRequest) -> Result<SignupAnswers, ApiError> {
    let text = |value: &Option<String>| -> Result<Option<String>, ApiError> {
        value
            .as_deref()
            .map(|text| {
                other_answer(text).ok_or_else(|| {
                    ApiError::bad_request(&format!(
                        "An answer to \"Something else\" must be {OTHER_ANSWER_MIN_CHARS} to \
                         {OTHER_ANSWER_MAX_CHARS} characters"
                    ))
                })
            })
            .transpose()
    };
    Ok(SignupAnswers {
        use_case_other: text(&request.use_case_other)?,
        // Each intent once, in the order picked.
        intents: request.intents.as_ref().map(|picked| {
            let mut intents = Vec::with_capacity(picked.len());
            for intent in picked {
                if !intents.contains(intent) {
                    intents.push(*intent);
                }
            }
            intents
        }),
        intent_other: text(&request.intent_other)?,
    })
}

/// Store onboarding step in session
#[utoipa::path(
    post,
    path = "/onboarding-step",
    tags = [api_tags::INTERNAL],
    request_body = OnboardingStepRequest,
    responses(
        (status = 200, description = "Step saved", body = EmptyApiResponse),
    )
)]
pub(crate) async fn onboarding_step(
    session: Session,
    ApiJson(request): ApiJson<OnboardingStepRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Checked before anything is saved, so a refused request changes nothing.
    let update = answers_from_request(&request)?;

    let mut answers: SignupAnswers = session
        .get(ONBOARDING_ANSWERS_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    answers.merge(update);
    session
        .insert(ONBOARDING_ANSWERS_KEY, answers)
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to save onboarding answers: {}", e))
        })?;

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
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Onboarding state", body = ApiResponse<OnboardingStateResponse>),
    )
)]
pub(crate) async fn onboarding_state(
    session: Session,
) -> ApiResult<Json<ApiResponse<OnboardingStateResponse>>> {
    let step: Option<String> = session.get("onboarding_step").await.ok().flatten();
    let use_case: Option<UseCase> = session.get("onboarding_use_case").await.ok().flatten();
    let answers: SignupAnswers = session
        .get(ONBOARDING_ANSWERS_KEY)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let (org_name, site, site_id) = if let Some(pending_setup) = session
        .get::<PendingSetup>("pending_setup")
        .await
        .ok()
        .flatten()
    {
        let site_id = pending_setup.site.as_ref().map(|n| n.site_id);
        let site = pending_setup.site.map(|n| OnboardingSiteState {
            id: Some(n.site_id),
            name: n.name,
        });
        (Some(pending_setup.org_name), site, site_id)
    } else {
        (None, None, None)
    };

    Ok(Json(ApiResponse::success(OnboardingStateResponse {
        step,
        use_case,
        use_case_other: answers.use_case_other,
        intents: answers.intents,
        intent_other: answers.intent_other,
        org_name,
        site,
        site_id,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::organizations::r#impl::intent::SignupIntent;

    #[test]
    fn only_cloud_signups_may_skip_the_site() {
        assert!(
            validate_setup_site(None, DeploymentType::Cloud)
                .unwrap()
                .is_none()
        );
        assert!(validate_setup_site(None, DeploymentType::Community).is_err());

        let blank = SiteSetup {
            name: "  ".to_string(),
        };
        assert!(validate_setup_site(Some(&blank), DeploymentType::Cloud).is_err());

        let named = SiteSetup {
            name: " Lab ".to_string(),
        };
        let site = validate_setup_site(Some(&named), DeploymentType::Community)
            .unwrap()
            .unwrap();
        assert_eq!(site.name, "Lab");
    }

    fn step(
        use_case_other: Option<&str>,
        intents: Option<Vec<SignupIntent>>,
        intent_other: Option<&str>,
    ) -> OnboardingStepRequest {
        OnboardingStepRequest {
            step: "intents".to_string(),
            use_case: None,
            use_case_other: use_case_other.map(String::from),
            intents,
            intent_other: intent_other.map(String::from),
        }
    }

    /// A step request carrying a "Something else" answer out of bounds is refused whole, so
    /// nothing it carries is saved. Answers within bounds are stored trimmed, and each intent once.
    #[test]
    fn onboarding_answers_are_bounded_trimmed_and_deduplicated() {
        assert!(answers_from_request(&step(Some("ab"), None, None)).is_err());
        assert!(answers_from_request(&step(None, None, Some(&"x".repeat(201)))).is_err());

        let answers = answers_from_request(&step(
            Some("  a school project "),
            Some(vec![
                SignupIntent::Inventory,
                SignupIntent::Other,
                SignupIntent::Inventory,
            ]),
            Some(" client audits "),
        ))
        .expect("within bounds");
        assert_eq!(answers.use_case_other.as_deref(), Some("a school project"));
        assert_eq!(
            answers.intents,
            Some(vec![SignupIntent::Inventory, SignupIntent::Other])
        );
        assert_eq!(answers.intent_other.as_deref(), Some("client audits"));

        // A skip is an empty list, kept apart from never answering.
        let skipped = answers_from_request(&step(None, Some(vec![]), None)).unwrap();
        assert_eq!(skipped.intents, Some(vec![]));
        assert_eq!(
            answers_from_request(&step(None, None, None))
                .unwrap()
                .intents,
            None
        );
    }

    /// Text typed for "Something else" and then abandoned for another option is not stored.
    #[test]
    fn free_text_is_kept_only_where_something_else_was_picked() {
        let typed = SignupAnswers {
            use_case_other: Some("a school project".to_string()),
            intents: Some(vec![SignupIntent::Inventory]),
            intent_other: Some("client audits".to_string()),
        };

        let stored = typed.clone().for_use_case(UseCase::InternalIt);
        assert_eq!(stored.use_case_other, None);
        assert_eq!(stored.intent_other, None);
        assert_eq!(stored.intents, Some(vec![SignupIntent::Inventory]));

        let mut with_other = typed;
        with_other.intents = Some(vec![SignupIntent::Other]);
        let stored = with_other.for_use_case(UseCase::Other);
        assert_eq!(stored.use_case_other.as_deref(), Some("a school project"));
        assert_eq!(stored.intent_other.as_deref(), Some("client audits"));
    }

    /// A later step request only overwrites the answers it carries.
    #[test]
    fn a_step_request_keeps_answers_it_does_not_carry() {
        let mut saved = SignupAnswers {
            use_case_other: Some("a school project".to_string()),
            intents: None,
            intent_other: None,
        };
        saved.merge(SignupAnswers {
            intents: Some(vec![SignupIntent::Workloads]),
            ..Default::default()
        });
        assert_eq!(saved.use_case_other.as_deref(), Some("a school project"));
        assert_eq!(saved.intents, Some(vec![SignupIntent::Workloads]));
    }
}
