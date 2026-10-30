use crate::server::auth::middleware::permissions::{
    Admin, Authorized, IsUser, Member, RequireVerified,
};
use crate::server::openapi::tags as api_tags;
use crate::server::shared::events::types::UpdateReason;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::query::{FilterQueryExtractor, NoFilterQuery};
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, delete_handler, validate_entity_references,
};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, EmptyApiResponse, PaginatedApiResponse,
};
use crate::server::users::r#impl::base::User;
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use crate::server::{
    config::AppState,
    shared::{
        services::traits::CrudService,
        types::api::{ApiResponse, ApiResult},
    },
};
use anyhow::anyhow;
use axum::extract::{Path, State};
use axum::response::Json;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for CSV export
mod generated {
    use super::*;
    crate::crud_export_csv_handler!(User);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_users))
        .routes(routes!(crate::server::users::members::list_members))
        .routes(routes!(get_user_by_id, update_user, delete_user))
        .routes(routes!(admin_update_user))
        .routes(routes!(bulk_delete_users))
        .routes(routes!(generated::export_csv))
}

/// Get user by ID
#[utoipa::path(
    get,
    path = "/{id}",
    tags = [User::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User found", body = ApiResponse<User>),
        (status = 404, description = "User not found", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn get_user_by_id(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<User>>> {
    let auth_org_id = auth.require_organization_id()?;
    let service = User::get_service(&state);

    let mut user = service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<User>(id))?;

    // Validate user is in the same organization
    if user.base.organization_id != auth_org_id {
        return Err(ApiError::permission_denied());
    }

    // Hydrate site_ids from junction table
    state
        .services
        .user_service
        .hydrate_site_ids(&mut user)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(user)))
}

/// List users
#[utoipa::path(
    get,
    operation_id = "list_users",
    path = "",
    tag = User::ENTITY_NAME_PLURAL,
    params(NoFilterQuery),
    responses(
        (status = 200, description = "List of users", body = PaginatedApiResponse<User>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn get_all_users(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    query: Query<NoFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<User>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

    // Get user permissions from entity
    let permissions = match &auth.entity {
        crate::server::auth::middleware::auth::AuthenticatedEntity::User {
            permissions, ..
        }
        | crate::server::auth::middleware::auth::AuthenticatedEntity::ApiKey {
            permissions, ..
        } => *permissions,
        _ => return Err(ApiError::user_required()),
    };

    let org_filter = StorableFilter::<User>::new_from_org_id(&organization_id);

    let service = User::get_service(&state);
    // Fetch all users first (permission filtering happens in-memory)
    let all_users: Vec<User> = service
        .get_all(org_filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .into_iter()
        .filter(|u| u.is_listed_for(permissions, user_id))
        .collect();

    // Apply pagination in-memory after filtering
    let pagination = query.pagination();
    let total_count = all_users.len() as u64;
    let offset = pagination.effective_offset() as usize;
    let limit = pagination.effective_limit();

    let mut users: Vec<User> = match limit {
        Some(l) => all_users
            .into_iter()
            .skip(offset)
            .take(l as usize)
            .collect(),
        None => all_users.into_iter().skip(offset).collect(),
    };

    // Hydrate site_ids from junction table
    state
        .services
        .user_service
        .hydrate_site_ids_batch(&mut users)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    let limit = limit.unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        users,
        total_count,
        limit,
        offset,
    )))
}

/// Delete user
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = User::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User deleted", body = EmptyApiResponse),
        (status = 404, description = "User not found", body = ApiErrorResponse),
        (status = 403, description = "Cannot delete yourself, or a user whose role you could not grant", body = ApiErrorResponse),
        (status = 409, description = "Cannot delete the only owner", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn delete_user(
    state: State<Arc<AppState>>,
    auth: Authorized<RequireVerified<Admin>>,
    id: Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Get user permissions from entity
    let permissions = match &auth.entity {
        crate::server::auth::middleware::auth::AuthenticatedEntity::User {
            permissions, ..
        }
        | crate::server::auth::middleware::auth::AuthenticatedEntity::ApiKey {
            permissions, ..
        } => *permissions,
        _ => return Err(ApiError::user_required()),
    };

    let user_to_be_deleted = state
        .services
        .user_service
        .get_by_id(&id.0)
        .await?
        .ok_or_else(|| anyhow!("User {} does not exist", id.0))?;

    refuse_user_delete(
        auth.user_id(),
        permissions,
        user_to_be_deleted.id,
        user_to_be_deleted.base.permissions,
    )?;

    let count_owners = state
        .services
        .user_service
        .get_organization_owners(&organization_id)
        .await?
        .len();

    if user_to_be_deleted.base.permissions == UserOrgPermissions::Owner && count_owners == 1 {
        return Err(ApiError::conflict(
            "Can't delete the only owner in an organization.",
        ));
    }

    delete_handler::<User>(state, auth.into_permission::<Member>(), id).await
}

/// Update your own user record
#[utoipa::path(
    put,
    path = "/{id}",
    tags = [User::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "User ID")),
    request_body = User,
    responses(
        (status = 200, description = "User updated", body = ApiResponse<User>),
        (status = 403, description = "Cannot update another user's record", body = ApiErrorResponse),
        (status = 404, description = "User not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
    Path(id): Path<Uuid>,
    ApiJson(mut request): ApiJson<User>,
) -> ApiResult<Json<ApiResponse<User>>> {
    let auth_user_id = auth.require_user_id()?;
    if auth_user_id != id {
        return Err(ApiError::unauthorized(
            "You can only update your own user record".to_string(),
        ));
    }
    let service = User::get_service(&state);

    // Verify entity exists
    let existing = service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<User>(id))?;

    // The update writes the row named by the body's id, so pin it to the path
    // id the checks above ran against, and keep every server-owned field.
    request.set_id(existing.id);
    request.set_created_at(existing.created_at);
    request.preserve_immutable_fields(&existing);

    if request.base.organization_id != existing.base.organization_id {
        return Err(ApiError::permission_denied());
    }

    if request.base.permissions != existing.base.permissions {
        return Err(ApiError::permission_denied());
    }

    if !request.base.display_settings.has_valid_time_zone() {
        return Err(ApiError::bad_request(
            "Invalid time zone. Use an IANA time zone like 'America/New_York'.",
        ));
    }

    let updated = service
        .update(
            &mut request,
            auth.into_entity(),
            UpdateReason::EntityRequested,
        )
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(updated)))
}

/// Update user permissions
///
/// Sets the user's role in the organization.
#[utoipa::path(
    put,
    operation_id = "update_user_permissions",
    path = "/{id}/admin",
    tags = [User::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "User ID")),
    request_body = User,
    responses(
        (status = 200, description = "User updated", body = ApiResponse<User>),
        (status = 403, description = "Cannot update user with higher permissions", body = ApiErrorResponse),
        (status = 404, description = "User not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn admin_update_user(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<Admin>>,
    Path(id): Path<Uuid>,
    ApiJson(mut request): ApiJson<User>,
) -> ApiResult<Json<ApiResponse<User>>> {
    let admin_user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

    // Get admin permissions from entity
    let admin_permissions = match &auth.entity {
        crate::server::auth::middleware::auth::AuthenticatedEntity::User {
            permissions, ..
        }
        | crate::server::auth::middleware::auth::AuthenticatedEntity::ApiKey {
            permissions, ..
        } => *permissions,
        _ => return Err(ApiError::user_required()),
    };

    let service = User::get_service(&state);

    // Verify target user exists
    let existing = service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<User>(id))?;

    // Target must belong to the admin's own organization. This handler calls
    // service.update directly (bypassing the generic org-scoped handler), so
    // without this check an admin could modify a lower-privileged user in
    // another org by id.
    if existing.base.organization_id != auth.require_organization_id()? {
        return Err(ApiError::permission_denied());
    }

    // Cannot edit yourself through this endpoint
    if admin_user_id == id {
        return Err(ApiError::permission_denied());
    }

    // Can only edit users with lower permissions than yourself
    if existing.base.permissions >= admin_permissions {
        return Err(ApiError::permission_denied());
    }

    // Cannot promote user to same or higher level than yourself
    if admin_permissions != UserOrgPermissions::Owner
        && request.base.permissions >= admin_permissions
    {
        return Err(ApiError::permission_denied());
    }

    // Cannot change organization
    if request.base.organization_id != existing.base.organization_id {
        return Err(ApiError::permission_denied());
    }

    // Pin the write to the path id and keep every server-owned field. An
    // admin also can't change another user's own preferences.
    request.set_id(existing.id);
    request.set_created_at(existing.created_at);
    request.preserve_immutable_fields(&existing);
    request.base.email_settings = existing.base.email_settings.clone();
    request.base.display_settings = existing.base.display_settings.clone();

    validate_entity_references::<User>(
        &state,
        &request,
        Some(&existing),
        &auth.site_ids(),
        auth.require_organization_id()?,
    )
    .await?;

    // Capture site_ids before update (they're stored in junction table, not user record)
    let site_ids = request.base.site_ids.clone();

    let updated = service
        .update(
            &mut request,
            auth.into_entity(),
            UpdateReason::EntityRequested,
        )
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Persist site_ids to the junction table
    state
        .services
        .user_service
        .set_site_ids(&id, &site_ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(updated)))
}

/// Bulk delete users
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = User::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "Array of User IDs to delete"),
    responses(
        (status = 200, description = "Users deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 403, description = "Cannot delete yourself, or a user whose role you could not grant", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_delete_users(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<Admin>>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    use crate::server::shared::handlers::traits::bulk_delete_handler;

    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Get user permissions from entity
    let permissions = match &auth.entity {
        crate::server::auth::middleware::auth::AuthenticatedEntity::User {
            permissions, ..
        }
        | crate::server::auth::middleware::auth::AuthenticatedEntity::ApiKey {
            permissions, ..
        } => *permissions,
        _ => return Err(ApiError::user_required()),
    };

    let user_filter = StorableFilter::<User>::new_from_entity_ids(&ids);
    let users = state.services.user_service.get_all(user_filter).await?;

    for user in &users {
        refuse_user_delete(auth.user_id(), permissions, user.id, user.base.permissions)?;
    }

    let owners = state
        .services
        .user_service
        .get_organization_owners(&organization_id)
        .await?;

    if owners.iter().all(|o| users.contains(o)) {
        return Err(ApiError::unauthorized(
            "Can't delete all of an organization's owners".to_string(),
        ));
    }

    bulk_delete_handler::<User>(
        axum::extract::State(state),
        auth.into_permission::<Member>(),
        ApiJson(ids),
    )
    .await
}

/// Refuses deleting yourself, or a user whose role you could not have granted. The users list
/// offers Delete for exactly the users this lets through.
fn refuse_user_delete(
    caller_id: Option<Uuid>,
    caller_permissions: UserOrgPermissions,
    target_id: Uuid,
    target_permissions: UserOrgPermissions,
) -> Result<(), ApiError> {
    if caller_id == Some(target_id) {
        return Err(ApiError::forbidden("You can't delete your own user"));
    }
    if !caller_permissions.can_grant_user_permission(&target_permissions) {
        return Err(ApiError::unauthorized(
            "You can only delete users whose role you could grant".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_user_cannot_delete_themselves() {
        let me = Uuid::new_v4();
        assert!(
            refuse_user_delete(
                Some(me),
                UserOrgPermissions::Owner,
                me,
                UserOrgPermissions::Owner
            )
            .is_err()
        );
    }

    #[test]
    fn deleting_follows_who_you_could_grant() {
        let caller = Some(Uuid::new_v4());
        let refused = |caller_role, target_role| {
            refuse_user_delete(caller, caller_role, Uuid::new_v4(), target_role).is_err()
        };
        assert!(refused(
            UserOrgPermissions::Admin,
            UserOrgPermissions::Admin
        ));
        assert!(!refused(
            UserOrgPermissions::Admin,
            UserOrgPermissions::Member
        ));
        assert!(!refused(
            UserOrgPermissions::Owner,
            UserOrgPermissions::Admin
        ));
    }
}
