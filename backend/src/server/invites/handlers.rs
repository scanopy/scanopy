use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::features::{InviteUsersFeature, RequireFeature};
use crate::server::auth::middleware::permissions::{Admin, Authorized, RequireVerified};
use crate::server::config::AppState;
use crate::server::invites::r#impl::base::Invite;
use crate::server::organizations::r#impl::api::CreateInviteRequest;
use crate::server::shared::events::types::{
    BillingEvent, BillingOperation, OnboardingEvent, OnboardingOperation,
};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::ApiResponse;
use crate::server::shared::types::api::ApiResult;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, EmptyApiResponse};
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::users::r#impl::base::User;
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use anyhow::Error;
use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Redirect;
use axum::routing::get;
use chrono::Utc;
use std::sync::Arc;
use tower_sessions::Session;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_invites, create_invite))
        .routes(routes!(get_invite, revoke_invite))
        // Accept invite link - redirect endpoint, not suitable for OpenAPI
        .route("/{id}/accept", get(accept_invite_link))
}

/// Create invite
#[utoipa::path(
    post,
    path = "",
    tag = Invite::ENTITY_NAME_PLURAL,
    request_body = CreateInviteRequest,
    responses(
        (status = 200, description = "Invite created", body = ApiResponse<Invite>),
        (status = 403, description = "Cannot create invite with higher permissions", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_invite(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<Admin>>,
    RequireFeature { plan, .. }: RequireFeature<InviteUsersFeature>,
    Json(request): Json<CreateInviteRequest>,
) -> ApiResult<Json<ApiResponse<Invite>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

    // Get user permissions and email from entity
    let (permissions, from_email) = match &auth.entity {
        AuthenticatedEntity::User {
            permissions, email, ..
        } => (*permissions, Some(email.clone())),
        AuthenticatedEntity::ApiKey { permissions, .. } => (*permissions, None),
        _ => return Err(ApiError::forbidden("User or API key required")),
    };

    if request.permissions > permissions {
        return Err(ApiError::forbidden(
            "Cannot create invite with higher permissions than your own",
        ));
    }

    for network_id in &request.network_ids {
        if !network_ids.contains(network_id) {
            return Err(ApiError::forbidden(
                "Cannot grant access to networks you don't have access to",
            ));
        }
    }
    // Seat limit check
    if let Some(max_seats) = plan.config().included_seats
        && plan.config().seat_cents.is_none()
    {
        let user_filter = StorableFilter::<User>::new_from_org_id(&organization_id);

        let current_members = state
            .services
            .user_service
            .get_all(user_filter)
            .await
            .unwrap_or_default()
            .len();

        let pending_invites = state
            .services
            .invite_service
            .get_org_invites(&organization_id)
            .await
            .unwrap_or_default()
            .len();

        let total_seats_used = current_members + pending_invites;

        if total_seats_used >= max_seats as usize {
            let _ = state
                .services
                .event_bus
                .publish_billing(BillingEvent::new(
                    Uuid::new_v4(),
                    organization_id,
                    BillingOperation::FeatureLimitHit,
                    Utc::now(),
                    auth.entity.clone(),
                    serde_json::json!({
                        "limit_type": "seats",
                        "current_count": total_seats_used,
                        "limit": max_seats,
                        "plan_type": plan.name(),
                    }),
                ))
                .await;

            return Err(ApiError::forbidden(&format!(
                "Seat limit reached ({}/{}). Upgrade your plan for more seats, or delete any unused pending invites.",
                total_seats_used, max_seats
            )));
        }
    }

    // Check if invited user already has an account
    if let Some(ref send_to) = request.send_to {
        let all_users = state
            .services
            .user_service
            .get_all(StorableFilter::<User>::new_from_email(send_to))
            .await
            .unwrap_or_default();

        if !all_users.is_empty() {
            return Err(ApiError::bad_request(
                "A user with this email already has an account. They must delete their account before joining a new organization.",
            ));
        }
    }

    let send_to = request.send_to.clone();
    let expiration_hours = request.expiration_hours.unwrap_or(168); // Default 7 days
    let entity = auth.entity.clone();

    let invite = Invite::with_expiration(
        organization_id,
        state.config.public_url.clone(),
        user_id,
        expiration_hours,
        request.permissions,
        request.network_ids,
        send_to.clone(),
    );

    let invite = state
        .services
        .invite_service
        .create(invite, entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Emit InviteSent telemetry event if this is the first invite
    let organization = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?;

    if let Some(organization) = organization
        && organization.not_onboarded(&OnboardingOperation::InviteSent)
    {
        state
            .services
            .invite_service
            .event_bus()
            .publish_onboarding(OnboardingEvent {
                id: Uuid::new_v4(),
                organization_id,
                operation: OnboardingOperation::InviteSent,
                timestamp: Utc::now(),
                metadata: serde_json::json!({}),
                authentication: entity,
            })
            .await?;
    }

    if let Some(send_to) = send_to
        && let Some(email_service) = &state.services.email_service
        && let Some(from_addr) = from_email
    {
        let url = format!(
            "{}/api/v1/invites/{}/accept",
            invite.base.url.trim_end_matches('/'),
            invite.id
        );
        email_service.send_invite(send_to, from_addr, url).await?;
    }

    Ok(Json(ApiResponse::success(invite)))
}

/// Get an invite by ID
#[utoipa::path(
    get,
    path = "/{id}",
    tag = Invite::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Invite ID")),
    responses(
        (status = 200, description = "Invite details", body = ApiResponse<Invite>),
        (status = 400, description = "Invalid or expired invite", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_invite(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<Invite>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let invite = state
        .services
        .invite_service
        .get_valid_invite(id)
        .await
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    // Validate invite belongs to the user's organization
    if invite.base.organization_id != organization_id {
        return Err(ApiError::entity_access_denied::<Invite>(id));
    }

    Ok(Json(ApiResponse::success(invite)))
}

/// List all invites
#[utoipa::path(
    get,
    path = "",
    tag = Invite::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "List of active invites", body = ApiResponse<Vec<Invite>>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_invites(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
) -> ApiResult<Json<ApiResponse<Vec<Invite>>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

    // Get user permissions from entity
    let permissions = match &auth.entity {
        AuthenticatedEntity::User { permissions, .. }
        | AuthenticatedEntity::ApiKey { permissions, .. } => *permissions,
        _ => return Err(ApiError::forbidden("User or API key required")),
    };

    // Show user invites that they created or created for users with permissions lower than them
    let invites = state
        .services
        .invite_service
        .list_active_invites(&organization_id)
        .await
        .into_iter()
        .filter(|i| {
            i.base.created_by == user_id
                || i.base.permissions < permissions
                || permissions == UserOrgPermissions::Owner
        })
        .collect();

    Ok(Json(ApiResponse::success(invites)))
}

/// Revoke an invite
#[utoipa::path(
    delete,
    path = "/{id}/revoke",
    tag = Invite::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Invite ID")),
    responses(
        (status = 200, description = "Invite revoked", body = EmptyApiResponse),
        (status = 400, description = "Invalid invite", body = ApiErrorResponse),
        (status = 403, description = "Cannot revoke this invite", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn revoke_invite(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<Admin>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

    // Get user permissions from entity
    let permissions = match &auth.entity {
        AuthenticatedEntity::User { permissions, .. }
        | AuthenticatedEntity::ApiKey { permissions, .. } => *permissions,
        _ => return Err(ApiError::forbidden("User or API key required")),
    };

    // Get the invite to verify ownership
    let invite = state
        .services
        .invite_service
        .get_valid_invite(id)
        .await
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    if invite.base.organization_id != organization_id {
        return Err(ApiError::entity_access_denied::<Invite>(id));
    }

    // Verify user can revoke this invite
    if !(user_id == invite.base.created_by
        || invite.base.permissions < permissions
        || permissions == UserOrgPermissions::Owner)
    {
        return Err(ApiError::permission_denied());
    }

    state
        .services
        .invite_service
        .delete(&id, auth.into_entity())
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

/// Accept an invite link - redirects to registration/login with pending org invite in session
async fn accept_invite_link(
    State(state): State<Arc<AppState>>,
    session: Session,
    Path(id): Path<Uuid>,
) -> Result<Redirect, Redirect> {
    // Validate the invite and get organization_id
    let invite = match state.services.invite_service.get_valid_invite(id).await {
        Ok(invite) => invite,
        Err(e) => {
            tracing::warn!("Invalid invite token: {}", e);
            return Err(Redirect::to(&format!(
                "/?error={}",
                urlencoding::encode("Invalid or expired invite link")
            )));
        }
    };

    let org_name = state
        .services
        .organization_service
        .get_by_id(&invite.base.organization_id)
        .await
        .unwrap_or_default()
        .map(|o| o.base.name)
        .unwrap_or("Unknown Organization".to_string());
    let inviting_user_email = state
        .services
        .user_service
        .get_by_id(&invite.base.created_by)
        .await
        .unwrap_or_default()
        .map(|u| u.base.email.to_string())
        .unwrap_or("Unknown User".to_string());

    // Store the pending invite in the session
    if let Err(e) = session
        .insert("pending_org_invite", invite.base.organization_id)
        .await
    {
        tracing::error!("Failed to save pending invite to session: {}", e);
        return Err(Redirect::to(&format!(
            "/?error={}",
            urlencoding::encode("Failed to process invite. Please try again.")
        )));
    }

    if let Err(e) = session.insert("pending_invite_id", id).await {
        tracing::error!("Failed to save invite token to session: {}", e);
        return Err(Redirect::to(&format!(
            "/?error={}",
            urlencoding::encode("Failed to process invite. Please try again.")
        )));
    };

    if let Err(e) = session
        .insert("pending_invite_permissions", invite.base.permissions)
        .await
    {
        tracing::error!("Failed to save invite permissions to session: {}", e);
        return Err(Redirect::to(&format!(
            "/?error={}",
            urlencoding::encode("Failed to process invite. Please try again.")
        )));
    };

    if let Err(e) = session
        .insert("pending_network_ids", invite.base.network_ids.clone())
        .await
    {
        tracing::error!("Failed to save invite network_ids to session: {}", e);
        return Err(Redirect::to(&format!(
            "/?error={}",
            urlencoding::encode("Failed to process invite. Please try again.")
        )));
    };

    // Check if user is already logged in
    if let Ok(Some(user_id)) = session.get::<uuid::Uuid>("user_id").await {
        // Check if user is already in an organization - they must leave first
        if let Ok(Some(_user)) = state.services.user_service.get_by_id(&user_id).await {
            // User already belongs to an org - revoke invite and redirect to error
            if let Err(e) = state
                .services
                .invite_service
                .delete(&id, AuthenticatedEntity::System)
                .await
            {
                tracing::warn!("Failed to revoke invite {}: {}", id, e);
            }
            return Err(Redirect::to(&format!(
                "/error?type=already_in_org&message={}",
                urlencoding::encode(
                    "You are already a member of an organization. Please leave your current organization before accepting this invite."
                )
            )));
        }

        // User is logged in but not in an org - add them to the organization immediately
        if let Some((org_id, permissions, network_ids)) = process_pending_invite(&state, &session)
            .await
            .map_err(|e| {
                tracing::error!("Failed to process invite for logged-in user: {}", e);
                Redirect::to(&format!("/?error={}", urlencoding::encode(&e.to_string())))
            })?
        {
            let mut user = state
                .services
                .user_service
                .get_by_id(&user_id)
                .await
                .map_err(|_| {
                    Redirect::to(&format!(
                        "/?error={}",
                        urlencoding::encode(&format!(
                            "Failed get user to update organization {}",
                            user_id
                        ))
                    ))
                })?
                .ok_or_else(|| {
                    Redirect::to(&format!(
                        "/?error={}",
                        urlencoding::encode(&format!(
                            "Failed to update organization for user {}",
                            user_id
                        ))
                    ))
                })?;

            user.base.organization_id = org_id;
            user.base.permissions = permissions;
            user.base.network_ids = network_ids;
            // Update user's organization
            state
                .services
                .user_service
                .update(&mut user, AuthenticatedEntity::System)
                .await
                .map_err(|_| {
                    Redirect::to(&format!(
                        "/?error={}",
                        urlencoding::encode(&format!(
                            "Failed update user organization {}",
                            user_id
                        ))
                    ))
                })?;
        }

        // Redirect to home
        return Ok(Redirect::to("/"));
    }

    // User is not logged in - redirect to onboarding/registration with invite params
    Ok(Redirect::to(&format!(
        "/onboarding?org_name={}&invited_by={}",
        org_name, inviting_user_email
    )))
}

pub async fn process_pending_invite(
    state: &Arc<AppState>,
    session: &Session,
) -> Result<Option<(Uuid, UserOrgPermissions, Vec<Uuid>)>, Error> {
    // Check for pending invite in session
    let pending_org_id = match session.get::<Uuid>("pending_org_invite").await {
        Ok(Some(org_id)) => org_id,
        _ => return Ok(None), // No pending invite
    };

    let invite_id = match session.get::<Uuid>("pending_invite_id").await {
        Ok(Some(token)) => token,
        _ => return Ok(None), // No token stored
    };

    let permissions = match session
        .get::<UserOrgPermissions>("pending_invite_permissions")
        .await
    {
        Ok(Some(permissions)) => permissions,
        _ => return Ok(None), // No permissions stored
    };

    let network_ids = match session.get::<Vec<Uuid>>("pending_network_ids").await {
        Ok(Some(network_ids)) => network_ids,
        _ => return Ok(None), // No network ids
    };

    // Mark invite as used. Failure is fatal: prevents TOCTOU where an invite gets
    // consumed/revoked between authorization (e.g. has_valid_pending_invite in the
    // register handler) and this consumption point. If we swallowed this error, a
    // deleted invite would still result in user creation + org assignment.
    state
        .services
        .invite_service
        .use_invite(invite_id)
        .await
        .map_err(|e| {
            tracing::warn!(invite_id = %invite_id, error = %e, "invite consumption failed (TOCTOU race or revoked)");
            Error::msg(format!("invite {} consumption failed: {}", invite_id, e))
        })?;

    // Emit InviteAccepted telemetry event if this is the first accepted invite
    let organization = state
        .services
        .organization_service
        .get_by_id(&pending_org_id)
        .await
        .ok()
        .flatten();

    if let Some(organization) = organization
        && organization.not_onboarded(&OnboardingOperation::InviteAccepted)
        && let Err(e) = state
            .services
            .invite_service
            .event_bus()
            .publish_onboarding(OnboardingEvent {
                id: Uuid::new_v4(),
                organization_id: pending_org_id,
                operation: OnboardingOperation::InviteAccepted,
                timestamp: Utc::now(),
                metadata: serde_json::json!({}),
                authentication: AuthenticatedEntity::System,
            })
            .await
    {
        tracing::warn!("Failed to emit InviteAccepted telemetry event: {}", e);
    }

    // Clear session data
    let _ = session.remove::<Uuid>("pending_org_invite").await;
    let _ = session.remove::<String>("pending_invite_id").await;
    let _ = session.remove::<String>("pending_invite_permissions").await;
    let _ = session.remove::<String>("pending_network_ids").await;

    Ok(Some((pending_org_id, permissions, network_ids)))
}
