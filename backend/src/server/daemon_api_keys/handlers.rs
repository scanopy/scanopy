use crate::server::shared::types::api::ApiJson;
use crate::server::{
    auth::middleware::permissions::{Authorized, Can, Delete, Write},
    config::AppState,
    daemon_api_keys::r#impl::{api::DaemonApiKeyResponse, base::DaemonApiKey},
    daemons::r#impl::base::Daemon,
    shared::{
        api_key_common::{ApiKeyService, ApiKeyType, generate_api_key_for_storage},
        handlers::{
            ordering::OrderField,
            query::{
                FilterQueryExtractor, OrderDirection, PaginationParams, deserialize_bool_values,
                on_requested_sites,
            },
            traits::{
                BulkDeleteResponse, CrudHandlers, bulk_delete_handler, delete_handler,
                update_handler,
            },
        },
        services::traits::CrudService,
        storage::{
            filter::StorableFilter,
            traits::{Entity, Storable},
        },
        types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse},
        validation::validate_site_access,
    },
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
// Daemon API Key Ordering
// ============================================================================

/// Fields the daemon API key list can be ordered and grouped by.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum DaemonApiKeyOrderField {
    #[default]
    CreatedAt,
    Name,
    UpdatedAt,
    SiteId,
    IsEnabled,
    /// Keys never used sort last in both directions.
    LastUsed,
    /// Keys that never expire sort last in both directions.
    ExpiresAt,
}

impl OrderField for DaemonApiKeyOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "api_keys.created_at",
            Self::Name => "api_keys.name",
            Self::UpdatedAt => "api_keys.updated_at",
            Self::SiteId => "api_keys.site_id",
            Self::IsEnabled => "api_keys.is_enabled",
            Self::LastUsed => "api_keys.last_used",
            Self::ExpiresAt => "api_keys.expires_at",
        }
    }

    fn sorts_by_site_name(&self) -> bool {
        matches!(self, Self::SiteId)
    }

    fn nulls_last(&self) -> bool {
        matches!(self, Self::LastUsed | Self::ExpiresAt)
    }
}

// ============================================================================
// Daemon API Key Filter Query
// ============================================================================

/// Query parameters for filtering and ordering daemon API keys.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct DaemonApiKeyFilterQuery {
    /// Filter by site ID. Repeat the parameter to pass several.
    #[serde(alias = "site_id")]
    pub site_ids: Option<Vec<Uuid>>,
    /// Filter by specific entity IDs (for selective loading)
    pub ids: Option<Vec<Uuid>>,
    /// `true` returns only keys no daemon is bound to, the ones managed from the API keys page;
    /// `false` only keys bound to a daemon. Omit for both.
    pub unbound: Option<bool>,
    /// Only enabled keys (`true`), disabled ones (`false`), or both (repeat it). Sent empty,
    /// matches none.
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
    pub group_by: Option<DaemonApiKeyOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<DaemonApiKeyOrderField>,
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

impl FilterQueryExtractor for DaemonApiKeyFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        let filter = on_requested_sites(filter, self.site_ids.as_deref(), user_site_ids);
        let filter = match &self.ids {
            Some(ids) if !ids.is_empty() => filter.entity_ids(ids),
            _ => filter,
        };
        let filter = match self.unbound {
            Some(unbound) => filter.column_is_null("daemon_id", unbound),
            None => filter,
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

mod generated {
    use super::*;
    crate::crud_list_page_handler!(DaemonApiKey);
    crate::crud_get_by_id_handler!(DaemonApiKey);
    crate::crud_export_csv_handler!(DaemonApiKey);
    crate::crud_get_field_values_handler!(DaemonApiKey);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_daemon_api_key))
        .routes(routes!(generated::get_by_id, delete_daemon_api_key))
        .routes(routes!(update_daemon_api_key))
        .routes(routes!(rotate_key_handler))
        .routes(routes!(bulk_delete_daemon_api_keys))
        .routes(routes!(generated::export_csv))
        .routes(routes!(generated::get_field_values))
}

/// Create daemon API key
#[utoipa::path(
    post,
    path = "",
    tag = DaemonApiKey::ENTITY_NAME_PLURAL,
    request_body = DaemonApiKey,
    responses(
        (status = 200, description = "Daemon API key created", body = ApiResponse<DaemonApiKeyResponse>),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 403, description = "Insufficient permissions (member+ required)", body = ApiErrorResponse),
        (status = 500, description = "Internal server error", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_daemon_api_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<DaemonApiKey, Write>>,
    ApiJson(mut api_key): ApiJson<DaemonApiKey>,
) -> ApiResult<Json<ApiResponse<DaemonApiKeyResponse>>> {
    let site_ids = auth.site_ids();
    let _ = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id();

    validate_site_access(Some(api_key.base.site_id), &site_ids, "create")?;

    let (plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::Daemon);

    let service = DaemonApiKey::get_service(&state);
    api_key.base.key = hashed;
    let entity = auth.into_entity();
    let api_key = service.create(api_key, entity).await.map_err(|e| {
        tracing::error!(
            error = %e,
            user_id = ?user_id,
            "Failed to create daemon API key"
        );
        ApiError::internal_error(&e.to_string())
    })?;

    Ok(Json(ApiResponse::success(DaemonApiKeyResponse {
        key: plaintext,
        api_key,
    })))
}

/// Update daemon API key
#[utoipa::path(
    put,
    path = "/{id}",
    tag = DaemonApiKey::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Daemon API key ID")),
    request_body = DaemonApiKey,
    responses(
        (status = 200, description = "Daemon API key updated", body = ApiResponse<DaemonApiKey>),
        (status = 404, description = "Daemon API key not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn update_daemon_api_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<DaemonApiKey, Write>>,
    Path(id): Path<Uuid>,
    ApiJson(mut request): ApiJson<DaemonApiKey>,
) -> ApiResult<Json<ApiResponse<DaemonApiKey>>> {
    let site_ids = auth.site_ids();

    // Fetch existing to preserve immutable fields
    let existing = DaemonApiKey::get_service(&state)
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<DaemonApiKey>(id))?;

    // Validate user has access to this key's site
    validate_site_access(Some(existing.base.site_id), &site_ids, "update")?;

    // Preserve the key hash - don't allow it to be changed via update
    request.preserve_immutable_fields(&existing);

    // Evict the auth resolution cache so a disable / expiry change on this key
    // takes effect immediately rather than waiting out the TTL. The key hash is
    // immutable, so `existing.base.key` is the cache key both before and after.
    let service = state.services.daemon_api_key_service.clone();
    let hashed_key = existing.base.key.clone();

    // Delegate to generic handler
    let result =
        update_handler::<DaemonApiKey>(State(state), auth, Path(id), ApiJson(request)).await;
    service.invalidate_resolution(&hashed_key).await;
    result
}

/// Rotate daemon API key
#[utoipa::path(
    post,
    operation_id = "rotate_daemon_api_key",
    path = "/{id}/rotate",
    tag = DaemonApiKey::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Daemon API key ID")),
    responses(
        (status = 200, description = "Daemon API key rotated, returns new key", body = ApiResponse<String>),
        (status = 404, description = "Daemon API key not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn rotate_key_handler(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<DaemonApiKey, Write>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    Path(api_key_id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let user_agent = user_agent.map(|u| u.to_string());
    let user_id = auth.user_id();

    let service = DaemonApiKey::get_service(&state);

    // Capture the pre-rotation hash so we can evict it: after rotation the old
    // plaintext must stop authenticating immediately, not linger in the cache
    // until the TTL. The new hash is uncached (populated on its next auth).
    let existing = service
        .get_by_id(&api_key_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<DaemonApiKey>(api_key_id))?;
    validate_site_access(Some(existing.base.site_id), &auth.site_ids(), "rotate")?;
    let old_hashed_key = existing.base.key;

    let key = service
        .rotate_key(api_key_id, ip, user_agent, auth.into_entity())
        .await
        .map_err(|e| {
            tracing::error!(
                api_key_id = %api_key_id,
                user_id = ?user_id,
                error = %e,
                "Failed to rotate daemon API key"
            );
            ApiError::internal_error(&e.to_string())
        })?;

    service.invalidate_resolution(&old_hashed_key).await;

    Ok(Json(ApiResponse::success(key)))
}

/// Delete daemon API key
///
/// Returns 409 Conflict if the key is currently assigned to a daemon.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = DaemonApiKey::ENTITY_NAME_PLURAL,
    operation_id = "delete_daemon_api_key",
    summary = "Delete daemon API key",
    params(("id" = Uuid, Path, description = "daemon_api_key ID")),
    responses(
        (status = 200, description = "daemon_api_key deleted", body = EmptyApiResponse),
        (status = 404, description = "daemon_api_key not found", body = ApiErrorResponse),
        (status = 409, description = "API key is in use by a daemon", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn delete_daemon_api_key(
    state: State<Arc<AppState>>,
    auth: Authorized<Can<DaemonApiKey, Delete>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let filter = StorableFilter::<Daemon>::new_from_uuid_column("api_key_id", &id);
    let daemons = state
        .services
        .daemon_service
        .get_all(filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    if !daemons.is_empty() {
        let names: Vec<&str> = daemons.iter().map(|d| d.base.name.as_str()).collect();
        return Err(ApiError::conflict(&format!(
            "Cannot delete API key: currently in use by daemon(s): {}",
            names.join(", ")
        )));
    }

    // Evict the deleted key's resolution so it can't authenticate from cache.
    let service = state.services.daemon_api_key_service.clone();
    let hashed_key = service
        .get_by_id(&id)
        .await
        .ok()
        .flatten()
        .map(|k| k.base.key);

    let result = delete_handler::<DaemonApiKey>(state, auth, Path(id)).await;
    if let Some(hashed_key) = hashed_key {
        service.invalidate_resolution(&hashed_key).await;
    }
    result
}

/// Bulk delete daemon API keys
///
/// Returns 409 Conflict if any key is currently assigned to a daemon.
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = DaemonApiKey::ENTITY_NAME_PLURAL,
    operation_id = "bulk_delete_daemon_api_keys",
    summary = "Bulk delete daemon API keys",
    request_body(content = Vec<Uuid>, description = "Array of Daemon API Key IDs to delete"),
    responses(
        (status = 200, description = "daemon_api_keys deleted", body = ApiResponse<BulkDeleteResponse>),
        (status = 409, description = "One or more API keys are in use by daemons", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_delete_daemon_api_keys(
    state: State<Arc<AppState>>,
    auth: Authorized<Can<DaemonApiKey, Delete>>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    let filter = StorableFilter::<Daemon>::new_from_uuids_column("api_key_id", &ids);
    let daemons = state
        .services
        .daemon_service
        .get_all(filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    if !daemons.is_empty() {
        let details: Vec<String> = daemons
            .iter()
            .map(|d| {
                let key_id = d
                    .base
                    .api_key_id
                    .map(|id| id.to_string())
                    .unwrap_or_default();
                format!("key {} (daemon: {})", key_id, d.base.name)
            })
            .collect();
        return Err(ApiError::conflict(&format!(
            "Cannot delete API keys: currently in use by daemon(s): {}",
            details.join(", ")
        )));
    }

    bulk_delete_handler::<DaemonApiKey>(state, auth, ApiJson(ids)).await
}
