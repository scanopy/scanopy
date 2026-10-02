//! Email verification and resend handlers.
use super::*;
use crate::server::openapi::tags as api_tags;
use crate::server::shared::types::api::ApiJson;

#[utoipa::path(
    post,
    path = "/verify-email",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified successfully", body = ApiResponse<User>),
        (status = 400, description = "Invalid or expired token", body = ApiErrorResponse),
    )
)]
pub(crate) async fn verify_email(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    session: Session,
    ApiJson(request): ApiJson<VerifyEmailRequest>,
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
        .insert("session_epoch", user.base.session_epoch)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;
    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;

    Ok(Json(ApiResponse::success(user)))
}

#[utoipa::path(
    post,
    path = "/resend-verification",
    tags = [api_tags::AUTH, api_tags::INTERNAL],
    request_body = ResendVerificationRequest,
    responses(
        (status = 200, description = "Verification email sent", body = EmptyApiResponse),
        (status = 400, description = "Invalid request or already verified", body = ApiErrorResponse),
        (status = 429, description = "Rate limited", body = ApiErrorResponse),
    )
)]

pub(crate) async fn resend_verification(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    ApiJson(request): ApiJson<ResendVerificationRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_agent = user_agent.map(|u| u.to_string());

    state
        .services
        .auth_service
        .resend_verification_email(&request.email, ip, user_agent)
        .await?;

    Ok(Json(ApiResponse::success(())))
}
