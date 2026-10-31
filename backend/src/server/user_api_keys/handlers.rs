use crate::server::openapi::tags as api_tags;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::{
    OnboardingOperation, OnboardingOperationDiscriminants, UpdateReason,
};
use crate::server::shared::extractors::Query;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::validation::validate_site_ids_access;
use crate::server::{
    auth::access::{Action, Assignment, Resource},
    auth::middleware::{
        features::{ApiKeyFeature, RequireFeature},
        permissions::{
            And, Authorized, Can, Delete, IsUser, PermissionRequirement, Read, RequireVerified,
            Write,
        },
    },
    config::AppState,
    shared::{
        api_key_common::{ApiKeyService, ApiKeyType, generate_api_key_for_storage},
        handlers::{
            list::list_page,
            ordering::OrderField,
            query::{
                FilterQueryExtractor, OrderDirection, PaginationParams, deserialize_bool_values,
            },
            traits::{
                BulkDeleteResponse, ListScope, bulk_delete_handler, delete_handler,
                get_by_id_handler,
            },
        },
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, traits::Storable},
        types::api::{
            ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
            PaginatedApiResponse,
        },
    },
    site_assignments::r#impl::base::Principal,
    user_api_keys::r#impl::{api::UserApiKeyResponse, base::UserApiKey},
    users::r#impl::permissions::UserOrgPermissions,
};
use axum::{
    Json,
    extract::{Path, State},
};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// User API Key Ordering
// ============================================================================

/// Fields the user API key list can be ordered and grouped by.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum UserApiKeyOrderField {
    #[default]
    CreatedAt,
    Name,
    UpdatedAt,
    Permissions,
    IsEnabled,
    /// Keys never used sort last in both directions.
    LastUsed,
    /// Keys that never expire sort last in both directions.
    ExpiresAt,
}

impl OrderField for UserApiKeyOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "user_api_keys.created_at",
            Self::Name => "user_api_keys.name",
            Self::UpdatedAt => "user_api_keys.updated_at",
            Self::Permissions => "user_api_keys.permissions",
            Self::IsEnabled => "user_api_keys.is_enabled",
            Self::LastUsed => "user_api_keys.last_used",
            Self::ExpiresAt => "user_api_keys.expires_at",
        }
    }

    fn nulls_last(&self) -> bool {
        matches!(self, Self::LastUsed | Self::ExpiresAt)
    }
}

// ============================================================================
// User API Key Filter Query
// ============================================================================

/// Query parameters for filtering and ordering the signed-in user's API keys.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct UserApiKeyFilterQuery {
    /// Only keys limited to one of these roles. Repeat for several.
    pub permissions: Option<Vec<UserOrgPermissions>>,
    /// Only keys with access to one of these sites. Repeat for several.
    #[serde(alias = "site_id")]
    pub site_ids: Option<Vec<Uuid>>,
    /// Only enabled keys (`true`), disabled ones (`false`), or both (repeat it). Sent empty, matches
    /// none.
    #[serde(default, deserialize_with = "deserialize_bool_values")]
    pub is_enabled: Option<Vec<bool>>,
    /// Filter by tag IDs (returns keys that have ANY of the specified tags)
    pub tag_ids: Option<Vec<Uuid>>,
    /// Tag sets the key must hold one tag from each of. Each entry is a comma-separated list of
    /// tag ids. One entry per tag group the list is filtered by.
    pub tag_sets: Option<Vec<String>>,
    /// Free-text search. Case-insensitive substring match against the key's name.
    pub search: Option<String>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<UserApiKeyOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<UserApiKeyOrderField>,
    /// Group by the tag each row holds from this tag group. Takes the place of `group_by`.
    pub group_by_tag_group: Option<Uuid>,
    /// Sort by the position of the tag each row holds from this tag group. Takes the place of
    /// `order_by`.
    pub order_by_tag_group: Option<Uuid>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl FilterQueryExtractor for UserApiKeyFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        _user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        let filter = match &self.permissions {
            Some(roles) if !roles.is_empty() => filter.text_column_in(
                "permissions",
                &roles.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
            ),
            _ => filter,
        };
        match &self.is_enabled {
            Some(values) => filter.bool_column_in("is_enabled", values),
            None => filter,
        }
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

// Generated handlers for CSV export and filter options
mod generated {
    use super::*;
    crate::crud_export_csv_handler!(UserApiKey, internal);
    crate::crud_get_field_values_handler!(UserApiKey);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all, create_user_api_key))
        .routes(routes!(get_by_id, delete))
        .routes(routes!(update_user_api_key))
        .routes(routes!(rotate_key_handler))
        .routes(routes!(bulk_delete))
        .routes(routes!(generated::export_csv))
        .routes(routes!(generated::get_field_values))
}

/// List user API keys
///
/// The signed-in user's keys.
#[utoipa::path(
    get,
    path = "",
    tags = [UserApiKey::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    operation_id = "list_user_api_keys",
    params(UserApiKeyFilterQuery),
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
    auth: Authorized<And<IsUser, Can<UserApiKey, Read>>>,
    Query(query): Query<UserApiKeyFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<UserApiKey>>> {
    // Only the signed-in user's own keys: `UserApiKey::access_filter` scopes by user.
    let scope = ListScope::of(&auth)?;
    Ok(Json(list_page::<UserApiKey>(&state, &scope, &query).await?))
}

/// Refuses a key holding anything its user doesn't, at the organization or on any site.
async fn refuse_key_escalation<P: PermissionRequirement>(
    state: &AppState,
    auth: &Authorized<P>,
    organization_id: Uuid,
    key: &UserApiKey,
) -> Result<(), ApiError> {
    let role = key.base.permissions;
    let assignments = Assignment::for_api_key(role, &key.base.site_ids);
    let granted = state
        .services
        .access_service
        .resolve(organization_id, role, &assignments)
        .await?;
    if auth.require_access()?.may_grant_to_key(&granted) {
        Ok(())
    } else {
        Err(ApiError::permission_denied())
    }
}

/// Create user API key
#[utoipa::path(
    post,
    path = "",
    tags = [UserApiKey::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    operation_id = "create_user_api_key",
    request_body = UserApiKey,
    responses(
        (status = 200, description = "API key created", body = ApiResponse<UserApiKeyResponse>),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 403, description = "Invalid permissions or site access", body = ApiErrorResponse),
        (status = 500, description = "Internal server error", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn create_user_api_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<And<IsUser, Can<UserApiKey, Write>>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    ApiJson(mut api_key): ApiJson<UserApiKey>,
) -> ApiResult<Json<ApiResponse<UserApiKeyResponse>>> {
    let user_id = auth.require_user_id()?;
    let organization_id = auth.require_organization_id()?;
    // The sites the user can hand out: a key is org-keyed, the sites in it aren't.
    let user_site_ids = auth.entity.site_ids_for(Resource::Site, Action::Read);
    let entity = auth.entity.clone();

    validate_site_ids_access(&api_key.base.site_ids, &user_site_ids)?;
    refuse_key_escalation(&state, &auth, organization_id, &api_key).await?;

    // Set user_id and organization_id from authenticated user
    api_key.base.user_id = user_id;
    api_key.base.organization_id = organization_id;

    let (plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::User);
    api_key.base.key = hashed;

    let site_ids = api_key.base.site_ids.clone();

    let service = &state.services.user_api_key_service;
    let api_key = state
        .services
        .access_service
        .create_api_key(api_key, site_ids, entity.clone())
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
    if state
        .services
        .organization_service
        .record_milestone(
            organization_id,
            OnboardingOperationDiscriminants::FirstUserApiKeyCreated,
        )
        .await?
    {
        service
            .event_bus()
            .publish(Event::new(
                OrgScope { organization_id },
                OnboardingOperation::FirstUserApiKeyCreated,
                entity,
            ))
            .await;
    }

    Ok(Json(ApiResponse::success(UserApiKeyResponse {
        key: plaintext,
        api_key,
    })))
}

/// Update user API key
#[utoipa::path(
    put,
    path = "/{id}",
    tags = [UserApiKey::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    operation_id = "update_user_api_key",
    params(("id" = Uuid, Path, description = "API key ID")),
    request_body = UserApiKey,
    responses(
        (status = 200, description = "API key updated", body = ApiResponse<UserApiKey>),
        (status = 403, description = "Not authorized to update this key", body = ApiErrorResponse),
        (status = 404, description = "API key not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn update_user_api_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<And<IsUser, Can<UserApiKey, Write>>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    Path(id): Path<Uuid>,
    ApiJson(mut request): ApiJson<UserApiKey>,
) -> ApiResult<Json<ApiResponse<UserApiKey>>> {
    let user_id = auth.require_user_id()?;
    let organization_id = auth.require_organization_id()?;
    let user_site_ids = auth.entity.site_ids_for(Resource::Site, Action::Read);

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

    validate_site_ids_access(&request.base.site_ids, &user_site_ids)?;
    refuse_key_escalation(&state, &auth, organization_id, &request).await?;

    // Preserve immutable fields
    request.preserve_immutable_fields(&existing);

    let site_ids = request.base.site_ids.clone();
    let role = request.base.permissions;
    let entity = auth.into_entity();

    // Update the entity
    let updated = service
        .update(&mut request, entity.clone(), UpdateReason::EntityRequested)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    state
        .services
        .access_service
        .set_access(
            Principal::ApiKey(id),
            role,
            &Assignment::for_api_key(role, &site_ids),
            entity,
        )
        .await?;

    // Return with hydrated site_ids
    let mut result = updated;
    result.base.site_ids = site_ids;

    Ok(Json(ApiResponse::success(result)))
}

/// Rotate user API key
#[utoipa::path(
    post,
    path = "/{id}/rotate",
    tags = [UserApiKey::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
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
    auth: Authorized<RequireVerified<And<IsUser, Can<UserApiKey, Write>>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    Path(api_key_id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let user_id = auth.require_user_id()?;
    let user_agent = user_agent.map(|u| u.to_string());

    let service = &state.services.user_api_key_service;
    let existing = service
        .get_by_id(&api_key_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<UserApiKey>(api_key_id))?;
    if existing.base.user_id != user_id {
        return Err(ApiError::entity_access_denied::<UserApiKey>(api_key_id));
    }

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

/// Get user API key by ID
#[utoipa::path(
    get,
    path = "/{id}",
    tags = [UserApiKey::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
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
    auth: Authorized<And<IsUser, Can<UserApiKey, Read>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<UserApiKey>>> {
    let user_id = auth.user_id().unwrap_or(Uuid::nil());
    let result = get_by_id_handler::<UserApiKey>(state, auth, Path(id)).await?;

    if result
        .data()
        .map(|k| k.base.user_id != user_id)
        .unwrap_or(true)
    {
        return Err(ApiError::entity_not_found::<UserApiKey>(id));
    }
    Ok(result)
}

/// Delete user API key
#[utoipa::path(
    delete,
    path = "/{id}",
    tags = [UserApiKey::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
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
    auth: Authorized<RequireVerified<And<IsUser, Can<UserApiKey, Delete>>>>,
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

    delete_handler::<UserApiKey>(state, auth, Path(id)).await
}

/// Bulk delete user API keys
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tags = [UserApiKey::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    operation_id = "bulk_delete_user_api_keys",
    request_body(content = Vec<Uuid>, description = "Array of User API Key IDs to delete"),
    responses(
        (status = 200, description = "API keys deleted", body = ApiResponse<BulkDeleteResponse>),
    ),
    security(("session" = []))
)]
pub async fn bulk_delete(
    state: State<Arc<AppState>>,
    auth: Authorized<RequireVerified<And<IsUser, Can<UserApiKey, Delete>>>>,
    _feature: RequireFeature<ApiKeyFeature>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
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

    let result = bulk_delete_handler::<UserApiKey>(state, auth, ApiJson(owned_ids)).await?;

    // Combine counts
    Ok(Json(ApiResponse::success(BulkDeleteResponse {
        deleted_count: result.data().map(|r| r.deleted_count).unwrap_or(0),
        requested_count: ids.len(),
    })))
}
