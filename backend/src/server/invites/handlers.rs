use crate::server::auth::access::{Action, Resource};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::features::{InviteUsersFeature, RequireFeature};
use crate::server::auth::middleware::permissions::{
    Authorized, Can, Delete, Read, RequireVerified, Write,
};
use crate::server::billing::types::base::{LimitSource, LimitType};
use crate::server::config::AppState;
use crate::server::invites::r#impl::base::Invite;
use crate::server::organizations::r#impl::api::CreateInviteRequest;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::{
    BillingOperation, OnboardingOperation, OnboardingOperationDiscriminants,
};
use crate::server::shared::events::types::{DeletionReason, UpdateReason};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::ApiResponse;
use crate::server::shared::types::api::ApiResult;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, EmptyApiResponse};
use crate::server::shared::validation::validate_site_ids_access;
use crate::server::users::r#impl::base::User;
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use anyhow::Error;
use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::response::Redirect;
use axum::routing::get;
use email_address::EmailAddress;
use std::sync::Arc;
use tower_sessions::Session;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_invites, create_invite))
        .routes(routes!(get_invite, revoke_invite))
        .routes(routes!(bulk_revoke_invites))
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
        (status = 400, description = "Recipient named but the caller has no address to send from", body = ApiErrorResponse),
        (status = 403, description = "Cannot create invite with higher permissions", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_invite(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<Can<Invite, Write>>>,
    RequireFeature { plan, .. }: RequireFeature<InviteUsersFeature>,
    ApiJson(request): ApiJson<CreateInviteRequest>,
) -> ApiResult<Json<ApiResponse<Invite>>> {
    // The sites the caller can hand out: an invite is org-keyed, the sites in it aren't.
    let site_ids = auth.entity.site_ids_for(Resource::Site, Action::Read);
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

    // A link-only invite needs no sender, but an API key has no address to send *from* — and the
    // send below is guarded on `from_email`, so naming a recipient would create the invite, return
    // 200, and never deliver anything. Refuse before the invite exists.
    if request.send_to.is_some() && from_email.is_none() {
        return Err(ApiError::bad_request(
            "Sending an invite email requires a user session. Omit send_to to create a link-only invite.",
        ));
    }

    if request.permissions > permissions {
        return Err(ApiError::forbidden(
            "Cannot create invite with higher permissions than your own",
        ));
    }

    validate_site_ids_access(&request.site_ids, &site_ids)?;
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
            state
                .services
                .event_bus
                .publish(Event::new(
                    OrgScope { organization_id },
                    BillingOperation::FeatureLimitHit {
                        limit_type: LimitType::Seats,
                        current_count: total_seats_used as u64,
                        limit: max_seats,
                        plan,
                        source: LimitSource::Api,
                    },
                    auth.entity.clone(),
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
        request.site_ids,
        send_to.clone(),
    );

    let invite = state
        .services
        .invite_service
        .create(invite, entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Emit InviteSent telemetry event if this is the first invite
    if state
        .services
        .organization_service
        .record_milestone(
            organization_id,
            OnboardingOperationDiscriminants::InviteSent,
        )
        .await?
    {
        state
            .services
            .invite_service
            .event_bus()
            .publish(Event::new(
                OrgScope { organization_id },
                OnboardingOperation::InviteSent,
                entity,
            ))
            .await;
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

/// Get invite by ID
#[utoipa::path(
    get,
    operation_id = "get_invite_by_id",
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
    auth: Authorized<Can<Invite, Read>>,
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

/// List invites
#[utoipa::path(
    get,
    operation_id = "list_invites",
    path = "",
    tag = Invite::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "List of active invites", body = ApiResponse<Vec<Invite>>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_invites(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Invite, Read>>,
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
        .filter(|i| i.is_listed_for(permissions, user_id))
        .collect();

    Ok(Json(ApiResponse::success(invites)))
}

/// Revoke invite
///
/// Deletes the invite, so its link stops working.
#[utoipa::path(
    delete,
    operation_id = "delete_invite",
    path = "/{id}",
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
    auth: Authorized<RequireVerified<Can<Invite, Delete>>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

    let permissions = caller_permissions(&auth.entity)?;

    let invite = revocable_invite(&state, id, organization_id, user_id, permissions).await?;

    state
        .services
        .invite_service
        .delete(
            &invite.id,
            auth.into_entity(),
            DeletionReason::EntityRequested,
        )
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

/// Revoke invites
///
/// Revokes each invite, as `DELETE /{id}` does for one. The request is refused, and
/// nothing is revoked, if any invite is invalid or one the caller can't revoke.
#[utoipa::path(
    post,
    operation_id = "bulk_delete_invites",
    path = "/bulk-delete",
    tag = Invite::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "IDs of the invites to revoke"),
    responses(
        (status = 200, description = "Invites revoked", body = EmptyApiResponse),
        (status = 400, description = "An invite is invalid", body = ApiErrorResponse),
        (status = 403, description = "An invite can't be revoked by the caller", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_revoke_invites(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<Can<Invite, Delete>>>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<()>>> {
    if ids.is_empty() {
        return Err(ApiError::bulk_empty());
    }
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;
    let permissions = caller_permissions(&auth.entity)?;

    for id in &ids {
        revocable_invite(&state, *id, organization_id, user_id, permissions).await?;
    }

    state
        .services
        .invite_service
        .delete_many(&ids, auth.into_entity(), DeletionReason::EntityRequested)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

/// The role of the user or API key making the request.
fn caller_permissions(entity: &AuthenticatedEntity) -> Result<UserOrgPermissions, ApiError> {
    match entity {
        AuthenticatedEntity::User { permissions, .. }
        | AuthenticatedEntity::ApiKey { permissions, .. } => Ok(*permissions),
        _ => Err(ApiError::forbidden("User or API key required")),
    }
}

/// The invite, if it is valid, in the caller's organization, and one they may revoke.
async fn revocable_invite(
    state: &Arc<AppState>,
    id: Uuid,
    organization_id: Uuid,
    caller_id: Uuid,
    caller_permissions: UserOrgPermissions,
) -> Result<Invite, ApiError> {
    let invite = state
        .services
        .invite_service
        .get_valid_invite(id)
        .await
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    if invite.base.organization_id != organization_id {
        return Err(ApiError::entity_access_denied::<Invite>(id));
    }
    if !may_revoke(
        caller_id,
        caller_permissions,
        invite.base.created_by,
        invite.base.permissions,
    ) {
        return Err(ApiError::permission_denied());
    }
    Ok(invite)
}

/// Whether the caller may revoke this invite: one they created, or one for a role below theirs,
/// or any as an Owner. The users list offers Revoke on the same terms.
fn may_revoke(
    caller_id: Uuid,
    caller_permissions: UserOrgPermissions,
    created_by: Uuid,
    invite_permissions: UserOrgPermissions,
) -> bool {
    caller_id == created_by
        || invite_permissions < caller_permissions
        || caller_permissions == UserOrgPermissions::Owner
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
        .insert("pending_site_ids", invite.base.site_ids.clone())
        .await
    {
        tracing::error!("Failed to save invite site_ids to session: {}", e);
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
                .delete(
                    &id,
                    AuthenticatedEntity::System,
                    DeletionReason::InviteConsumed,
                )
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
        if let Some(AcceptedInvite {
            organization_id: org_id,
            permissions,
            site_ids,
            ..
        }) = process_pending_invite(&state, &session)
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
            user.base.site_ids = site_ids;
            // Update user's organization
            state
                .services
                .user_service
                .update(
                    &mut user,
                    AuthenticatedEntity::System,
                    UpdateReason::UserJoinedOrganization,
                )
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

/// The membership a pending invite grants, read from the session when it is consumed.
pub struct AcceptedInvite {
    pub organization_id: Uuid,
    pub permissions: UserOrgPermissions,
    pub site_ids: Vec<Uuid>,
    /// The address the invite was emailed to. `None` for a link-only invite, or when the
    /// invite record could not be read back. Registering with this address proves it.
    pub send_to: Option<EmailAddress>,
}

pub async fn process_pending_invite(
    state: &Arc<AppState>,
    session: &Session,
) -> Result<Option<AcceptedInvite>, Error> {
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

    // Sessions opened before the site rename hold the ids under `pending_network_ids`.
    let site_ids = match session.get::<Vec<Uuid>>("pending_site_ids").await {
        Ok(Some(site_ids)) => site_ids,
        _ => match session.get::<Vec<Uuid>>("pending_network_ids").await {
            Ok(Some(site_ids)) => site_ids,
            _ => return Ok(None), // No site ids
        },
    };

    // Mark invite as used. A deleted or expired invite grants nothing, so its failure is
    // the caller's failure: no account or membership is created from it.
    let send_to = state
        .services
        .invite_service
        .use_invite(invite_id)
        .await?
        .base
        .send_to;

    // Emit InviteAccepted telemetry event if this is the first accepted invite
    if let Ok(true) = state
        .services
        .organization_service
        .record_milestone(
            pending_org_id,
            OnboardingOperationDiscriminants::InviteAccepted,
        )
        .await
    {
        state
            .services
            .invite_service
            .event_bus()
            .publish(Event::new(
                OrgScope {
                    organization_id: pending_org_id,
                },
                OnboardingOperation::InviteAccepted,
                AuthenticatedEntity::System,
            ))
            .await;
    }

    // Clear session data
    let _ = session.remove::<Uuid>("pending_org_invite").await;
    let _ = session.remove::<String>("pending_invite_id").await;
    let _ = session.remove::<String>("pending_invite_permissions").await;
    let _ = session.remove::<String>("pending_site_ids").await;
    let _ = session.remove::<String>("pending_network_ids").await;

    Ok(Some(AcceptedInvite {
        organization_id: pending_org_id,
        permissions,
        site_ids,
        send_to,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use UserOrgPermissions::{Admin, Member, Owner};

    #[test]
    fn revoking_follows_who_created_the_invite_and_the_role_it_grants() {
        let (me, someone) = (Uuid::new_v4(), Uuid::new_v4());
        assert!(
            may_revoke(me, Admin, me, Owner),
            "the creator can always revoke"
        );
        assert!(!may_revoke(me, Admin, someone, Admin));
        assert!(may_revoke(me, Admin, someone, Member));
        assert!(may_revoke(me, Owner, someone, Owner));
    }
}
