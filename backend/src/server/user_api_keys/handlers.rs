use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::{OnboardingOperation, OnboardingOperationDiscriminants};
use crate::server::shared::extractors::Query;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::validation::validate_network_ids_access;
use crate::server::{
    auth::middleware::{
        features::{ApiKeyFeature, RequireFeature},
        permissions::{And, Authorized, IsUser, Member, RequireVerified, Viewer},
    },
    config::AppState,
    shared::{
        api_key_common::{ApiKeyService, ApiKeyType, generate_api_key_for_storage},
        handlers::{
            query::{FilterQueryExtractor, NoFilterQuery},
            traits::{BulkDeleteResponse, bulk_delete_handler, delete_handler, get_by_id_handler},
        },
        services::traits::{CrudService, EventBusService},
        types::api::{
            ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
            PaginatedApiResponse,
        },
    },
    user_api_keys::{
        r#impl::{api::UserApiKeyResponse, base::UserApiKey},
        service::UserApiKeyService,
    },
};
use axum::{
    Json,
    extract::{Path, State},
};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for CSV export
mod generated {
    use super::*;
    crate::crud_export_csv_handler!(UserApiKey);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all, create_user_api_key))
        .routes(routes!(get_by_id, delete))
        .routes(routes!(update_user_api_key))
        .routes(routes!(rotate_key_handler))
        .routes(routes!(bulk_delete))
        .routes(routes!(generated::export_csv))
}

/// Get all user API keys for the current user
#[utoipa::path(
    get,
    path = "",
    tag = UserApiKey::ENTITY_NAME_PLURAL,
    operation_id = "get_all_user_api_keys",
    params(NoFilterQuery),
    responses(
        (status = 200, description = "List of user API keys", body = PaginatedApiResponse<UserApiKey>),
        (status = 401, description = "Not authenticated", body = ApiErrorResponse),
        (status = 500, description = "Internal server error", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn get_all(
    State(state): State<Arc<AppState>>,
    _feature: RequireFeature<ApiKeyFeature>,
    auth: Authorized<IsUser>,
    query: Query<NoFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<UserApiKey>>> {
    let user_id = auth.require_user_id()?;
    let service = &state.services.user_api_key_service;

    let all_keys = service.get_for_user(&user_id).await.map_err(|e| {
        tracing::error!(
            user_id = %user_id,
            error = %e,
            "Failed to fetch user API keys"
        );
        ApiError::internal_error(&e.to_string())
    })?;

    // Apply pagination in-memory
    let pagination = query.pagination();
    let total_count = all_keys.len() as u64;
    let offset = pagination.effective_offset() as usize;
    let limit = pagination.effective_limit();

    let keys: Vec<UserApiKey> = match limit {
        Some(l) => all_keys.into_iter().skip(offset).take(l as usize).collect(),
        None => all_keys.into_iter().skip(offset).collect(),
    };

    let limit = limit.unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        keys,
        total_count,
        limit,
        offset,
    )))
}

/// Create a new user API key
#[utoipa::path(
    post,
    path = "",
    tag = UserApiKey::ENTITY_NAME_PLURAL,
    operation_id = "create_user_api_key",
    responses(
        (status = 200, description = "API key created", body = ApiResponse<UserApiKeyResponse>),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 403, description = "Invalid permissions or network access", body = ApiErrorResponse),
        (status = 500, description = "Internal server error", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn create_user_api_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<And<IsUser, Member>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    Json(mut api_key): Json<UserApiKey>,
) -> ApiResult<Json<ApiResponse<UserApiKeyResponse>>> {
    let user_id = auth.require_user_id()?;
    let organization_id = auth.require_organization_id()?;
    let user_permissions = auth.require_permissions()?;
    let user_network_ids = auth.network_ids();
    let entity = auth.entity.clone();

    tracing::debug!(
        api_key_name = %api_key.base.name,
        permissions = %api_key.base.permissions,
        user_id = %user_id,
        "User API key create request received"
    );

    // Validate permissions don't exceed user's permissions
    UserApiKeyService::validate_permissions(api_key.base.permissions, user_permissions)
        .map_err(|_| ApiError::permission_denied())?;

    // Validate network access is a subset of user's access
    validate_network_ids_access(&api_key.base.network_ids, &user_network_ids)?;

    // Set user_id and organization_id from authenticated user
    api_key.base.user_id = user_id;
    api_key.base.organization_id = organization_id;

    let (plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::User);
    api_key.base.key = hashed;

    let network_ids = api_key.base.network_ids.clone();

    let service = &state.services.user_api_key_service;
    let api_key = service
        .create_with_networks(api_key, network_ids, entity.clone())
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                user_id = %user_id,
                "Failed to create user API key"
            );
            ApiError::internal_error(&e.to_string())
        })?;

    // Emit FirstUserApiKeyCreated telemetry event if this is the first user API key
    let organization = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?;

    if let Some(organization) = organization
        && organization.not_onboarded(&OnboardingOperationDiscriminants::FirstUserApiKeyCreated)
    {
        service
            .event_bus()
            .publish(Event::new(
                OrgScope { organization_id },
                OnboardingOperation::FirstUserApiKeyCreated,
                entity,
            ))
            .await?;
    }

    Ok(Json(ApiResponse::success(UserApiKeyResponse {
        key: plaintext,
        api_key,
    })))
}

/// Update a user API key
#[utoipa::path(
    put,
    path = "/{id}",
    tag = UserApiKey::ENTITY_NAME_PLURAL,
    operation_id = "update_user_api_key",
    params(("id" = Uuid, Path, description = "API key ID")),
    responses(
        (status = 200, description = "API key updated", body = ApiResponse<UserApiKey>),
        (status = 403, description = "Not authorized to update this key", body = ApiErrorResponse),
        (status = 404, description = "API key not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn update_user_api_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<And<IsUser, Member>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    Path(id): Path<Uuid>,
    Json(mut request): Json<UserApiKey>,
) -> ApiResult<Json<ApiResponse<UserApiKey>>> {
    let user_id = auth.require_user_id()?;
    let user_permissions = auth.require_permissions()?;
    let user_network_ids = auth.network_ids();

    let service = &state.services.user_api_key_service;

    // Fetch existing to verify ownership
    let existing = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<UserApiKey>(id))?;

    // Verify the user owns this key
    if existing.base.user_id != user_id {
        return Err(ApiError::entity_access_denied::<UserApiKey>(id));
    }

    // Validate permissions don't exceed user's permissions
    UserApiKeyService::validate_permissions(request.base.permissions, user_permissions)
        .map_err(|_| ApiError::permission_denied())?;

    // Validate network access is a subset of user's access
    validate_network_ids_access(&request.base.network_ids, &user_network_ids)?;

    // Preserve immutable fields
    request.preserve_immutable_fields(&existing);

    // Update network access
    let network_ids = request.base.network_ids.clone();
    service.update_network_access(&id, &network_ids).await?;

    // Update the entity
    let updated = service
        .update(&mut request, auth.into_entity())
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Return with hydrated network_ids
    let mut result = updated;
    result.base.network_ids = network_ids;

    Ok(Json(ApiResponse::success(result)))
}

/// Rotate a user API key
#[utoipa::path(
    post,
    path = "/{id}/rotate",
    tag = UserApiKey::ENTITY_NAME_PLURAL,
    operation_id = "rotate_user_api_key",
    params(("id" = Uuid, Path, description = "API key ID")),
    responses(
        (status = 200, description = "API key rotated, returns new key", body = ApiResponse<String>),
        (status = 403, description = "Not authorized to rotate this key", body = ApiErrorResponse),
        (status = 404, description = "API key not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn rotate_key_handler(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<And<IsUser, Member>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    Path(api_key_id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let user_id = auth.require_user_id()?;
    let user_agent = user_agent.map(|u| u.to_string());

    let service = &state.services.user_api_key_service;
    let key = service
        .rotate_key(api_key_id, ip, user_agent, auth.into_entity())
        .await
        .map_err(|e| {
            tracing::error!(
                api_key_id = %api_key_id,
                user_id = %user_id,
                error = %e,
                "Failed to rotate user API key"
            );
            ApiError::internal_error(&e.to_string())
        })?;

    Ok(Json(ApiResponse::success(key)))
}

/// Get a user API key by ID
#[utoipa::path(
    get,
    path = "/{id}",
    tag = UserApiKey::ENTITY_NAME_PLURAL,
    operation_id = "get_user_api_key_by_id",
    params(("id" = Uuid, Path, description = "API key ID")),
    responses(
        (status = 200, description = "API key found", body = ApiResponse<UserApiKey>),
        (status = 404, description = "API key not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn get_by_id(
    state: State<Arc<AppState>>,
    auth: Authorized<And<IsUser, Viewer>>,
    _feature: RequireFeature<ApiKeyFeature>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<UserApiKey>>> {
    let user_id = auth.user_id().unwrap_or(Uuid::nil());
    let result =
        get_by_id_handler::<UserApiKey>(state, auth.into_permission::<Viewer>(), Path(id)).await?;

    if result
        .data()
        .map(|k| k.base.user_id != user_id)
        .unwrap_or(true)
    {
        return Err(ApiError::entity_not_found::<UserApiKey>(id));
    }
    Ok(result)
}

/// Delete a user API key
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = UserApiKey::ENTITY_NAME_PLURAL,
    operation_id = "delete_user_api_key",
    params(("id" = Uuid, Path, description = "API key ID")),
    responses(
        (status = 200, description = "API key deleted", body = EmptyApiResponse),
        (status = 404, description = "API key not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn delete(
    state: State<Arc<AppState>>,
    auth: Authorized<RequireVerified<And<IsUser, Member>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_id = auth.user_id().unwrap_or(Uuid::nil());

    // Verify ownership before deleting
    let key = state.services.user_api_key_service.get_by_id(&id).await?;
    if key
        .as_ref()
        .map(|k| k.base.user_id != user_id)
        .unwrap_or(true)
    {
        return Err(ApiError::entity_not_found::<UserApiKey>(id));
    }

    delete_handler::<UserApiKey>(state, auth.into_permission::<Member>(), Path(id)).await
}

/// Bulk delete user API keys
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = UserApiKey::ENTITY_NAME_PLURAL,
    operation_id = "bulk_delete_user_api_keys",
    request_body(content = Vec<Uuid>, description = "Array of User API Key IDs to delete"),
    responses(
        (status = 200, description = "API keys deleted", body = ApiResponse<BulkDeleteResponse>),
    ),
    security(("session" = []))
)]
pub async fn bulk_delete(
    state: State<Arc<AppState>>,
    auth: Authorized<RequireVerified<And<IsUser, Member>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    Json(ids): Json<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    let user_id = auth.user_id().unwrap_or(Uuid::nil());
    let service = &state.services.user_api_key_service;

    // Filter to only keys owned by this user
    let mut owned_ids = Vec::new();
    for id in &ids {
        if let Ok(Some(key)) = service.get_by_id(id).await
            && key.base.user_id == user_id
        {
            owned_ids.push(*id);
        }
    }

    let result =
        bulk_delete_handler::<UserApiKey>(state, auth.into_permission::<Member>(), Json(owned_ids))
            .await?;

    // Combine counts
    Ok(Json(ApiResponse::success(BulkDeleteResponse {
        deleted_count: result.data().map(|r| r.deleted_count).unwrap_or(0),
        requested_count: ids.len(),
    })))
}
