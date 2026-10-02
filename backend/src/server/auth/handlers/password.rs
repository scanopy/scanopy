//! Password update, email-change request, and forgot/reset flows.
use super::*;
use crate::server::openapi::tags as api_tags;
use crate::server::shared::types::api::ApiJson;
use validator::Validate;

#[utoipa::path(
    post,
    path = "/update",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    responses(
        (status = 200, description = "Password updated", body = ApiResponse<User>),
        (status = 401, description = "Not authenticated", body = ApiErrorResponse),
        (status = 403, description = "Blocked in demo mode", body = ApiErrorResponse),
    )
)]
pub(crate) async fn update_password_auth(
    State(state): State<Arc<AppState>>,
    session: Session,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    auth: Authorized<IsUser>,
    ApiJson(request): ApiJson<UpdatePasswordRequest>,
) -> ApiResult<Json<ApiResponse<User>>> {
    // Enforce the password policy on change (length + complexity), matching
    // registration — the service takes raw fields, so validate here.
    request
        .validate()
        .map_err(|e| ApiError::bad_request(&format!("Validation failed: {}", e)))?;

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

    // Keep this session valid after the epoch bump that update_password applied
    // to log out the user's other sessions.
    session
        .insert("session_epoch", user.base.session_epoch)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;

    Ok(Json(ApiResponse::success(user)))
}

#[utoipa::path(
    post,
    path = "/request-email-change",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    request_body = RequestEmailChangeRequest,
    responses(
        (status = 200, description = "Verification email sent to new address", body = EmptyApiResponse),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
        (status = 401, description = "Not authenticated", body = ApiErrorResponse),
    )
)]
pub(crate) async fn request_email_change(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    auth: Authorized<IsUser>,
    ApiJson(request): ApiJson<RequestEmailChangeRequest>,
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
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Password reset email sent", body = EmptyApiResponse),
    )
)]
pub(crate) async fn forgot_password(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    ApiJson(request): ApiJson<ForgotPasswordRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_agent = user_agent.map(|u| u.to_string());

    state
        .services
        .auth_service
        .initiate_password_reset(&request.email, ip, user_agent)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/reset-password",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password reset successful", body = ApiResponse<User>),
        (status = 400, description = "Invalid or expired token", body = ApiErrorResponse),
    )
)]
pub(crate) async fn reset_password(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    session: Session,
    ApiJson(request): ApiJson<ResetPasswordRequest>,
) -> ApiResult<Json<ApiResponse<User>>> {
    // Enforce the password policy on reset (length + complexity), matching
    // registration — the service takes raw fields, so validate here.
    request
        .validate()
        .map_err(|e| ApiError::bad_request(&format!("Validation failed: {}", e)))?;

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

    // Store the post-reset epoch so this session survives the epoch bump that
    // complete_password_reset applied to invalidate all other sessions.
    session
        .insert("session_epoch", user.base.session_epoch)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;
    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;

    Ok(Json(ApiResponse::success(user)))
}
