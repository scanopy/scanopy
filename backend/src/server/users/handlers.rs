use crate::server::auth::middleware::permissions::{
    Admin, Authorized, IsUser, Member, RequireVerified,
};
use crate::server::openapi::tags as api_tags;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::query::{FilterQueryExtractor, NoFilterQuery};
use crate::server::shared::handlers::traits::{BulkDeleteResponse, CrudHandlers, delete_handler};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
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
        .routes(routes!(get_user_by_id, update_user, delete_user))
        .routes(routes!(admin_update_user))
        .routes(routes!(bulk_delete_users))
        .routes(routes!(generated::export_csv))
}

/// Get user by ID
#[utoipa::path(
    get,
    path = "/{id}",
    tag = User::ENTITY_NAME_PLURAL,
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

    // Hydrate network_ids from junction table
    state
        .services
        .user_service
        .hydrate_network_ids(&mut user)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(user)))
}

/// List all users
#[utoipa::path(
    get,
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
        .filter(|u| {
            permissions == UserOrgPermissions::Owner
                || u.base.permissions < permissions
                || u.id == user_id
        })
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

    // Hydrate network_ids from junction table
    state
        .services
        .user_service
        .hydrate_network_ids_batch(&mut users)
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

/// Delete a user
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = User::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User deleted", body = EmptyApiResponse),
        (status = 404, description = "User not found", body = ApiErrorResponse),
        (status = 403, description = "Cannot delete user with higher permissions", body = ApiErrorResponse),
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

    if permissions < user_to_be_deleted.base.permissions {
        return Err(ApiError::unauthorized(
            "You can only delete users with lower permissions than you".to_string(),
        ));
    }

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
    tags = [User::ENTITY_NAME_PLURAL],
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
    Json(mut request): Json<User>,
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

    if request.base.organization_id != existing.base.organization_id {
        return Err(ApiError::permission_denied());
    }

    if request.base.permissions != existing.base.permissions {
        return Err(ApiError::permission_denied());
    }

    // Preserve fields that shouldn't be changed via this endpoint
    request.base.email = existing.base.email.clone();
    request.base.password_hash = existing.base.password_hash.clone();
    request.base.oidc_provider = existing.base.oidc_provider.clone();
    request.base.oidc_subject = existing.base.oidc_subject.clone();
    request.base.oidc_linked_at = existing.base.oidc_linked_at;

    if !request.base.display_settings.has_valid_time_zone() {
        return Err(ApiError::bad_request(
            "Invalid time zone. Use an IANA time zone like 'America/New_York'.",
        ));
    }

    let updated = service
        .update(&mut request, auth.into_entity())
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(updated)))
}

/// Admin update user (for changing permissions)
#[utoipa::path(
    put,
    path = "/{id}/admin",
    tags = [User::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
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
    Json(mut request): Json<User>,
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

    // Preserve fields that shouldn't be changed via this endpoint
    request.base.email = existing.base.email.clone();
    request.base.password_hash = existing.base.password_hash.clone();
    request.base.oidc_provider = existing.base.oidc_provider.clone();
    request.base.oidc_subject = existing.base.oidc_subject.clone();
    request.base.oidc_linked_at = existing.base.oidc_linked_at;
    request.base.email_settings = existing.base.email_settings.clone();
    request.base.display_settings = existing.base.display_settings.clone();

    // Capture network_ids before update (they're stored in junction table, not user record)
    let network_ids = request.base.network_ids.clone();

    let updated = service
        .update(&mut request, auth.into_entity())
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Persist network_ids to the junction table
    state
        .services
        .user_service
        .set_network_ids(&id, &network_ids)
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
        (status = 403, description = "Cannot delete users with higher permissions", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_delete_users(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<Admin>>,
    Json(ids): Json<Vec<Uuid>>,
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

    if users.iter().any(|u| u.base.permissions > permissions) {
        return Err(ApiError::unauthorized(
            "You can only delete users with lower permissions than you".to_string(),
        ));
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
        axum::extract::Json(ids),
    )
    .await
}
