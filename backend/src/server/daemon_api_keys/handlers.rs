use crate::server::shared::types::api::ApiJson;
use crate::server::{
    auth::middleware::permissions::{Authorized, Member},
    config::AppState,
    daemon_api_keys::r#impl::{api::DaemonApiKeyResponse, base::DaemonApiKey},
    daemons::r#impl::base::Daemon,
    shared::{
        api_key_common::{ApiKeyService, ApiKeyType, generate_api_key_for_storage},
        handlers::traits::{
            BulkDeleteResponse, CrudHandlers, bulk_delete_handler, delete_handler, update_handler,
        },
        services::traits::CrudService,
        storage::{filter::StorableFilter, traits::Entity},
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
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

mod generated {
    use super::*;
    crate::crud_get_all_handler!(DaemonApiKey);
    crate::crud_get_by_id_handler!(DaemonApiKey);
    crate::crud_export_csv_handler!(DaemonApiKey);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_daemon_api_key))
        .routes(routes!(generated::get_by_id, delete_daemon_api_key))
        .routes(routes!(update_daemon_api_key))
        .routes(routes!(rotate_key_handler))
        .routes(routes!(bulk_delete_daemon_api_keys))
        .routes(routes!(generated::export_csv))
}

/// Create Daemon API Key
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
    auth: Authorized<Member>,
    ApiJson(mut api_key): ApiJson<DaemonApiKey>,
) -> ApiResult<Json<ApiResponse<DaemonApiKeyResponse>>> {
    let site_ids = auth.site_ids();
    let _ = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id();

    tracing::debug!(
        api_key_name = %api_key.base.name,
        site_id = %api_key.base.site_id,
        user_id = ?user_id,
        "Daemon API key create request received"
    );

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

/// Update a Daemon API Key
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
    auth: Authorized<Member>,
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

/// Rotate a Daemon API Key
#[utoipa::path(
    post,
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
    auth: Authorized<Member>,
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
    let old_hashed_key = service
        .get_by_id(&api_key_id)
        .await
        .ok()
        .flatten()
        .map(|k| k.base.key);

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

    if let Some(old_hashed_key) = old_hashed_key {
        service.invalidate_resolution(&old_hashed_key).await;
    }

    Ok(Json(ApiResponse::success(key)))
}

/// Delete a Daemon API Key
///
/// Returns 409 Conflict if the key is currently assigned to a daemon.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = DaemonApiKey::ENTITY_NAME_PLURAL,
    operation_id = "delete_daemon_api_key",
    summary = "Delete daemon_api_key",
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
    auth: Authorized<Member>,
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

/// Bulk delete Daemon API Keys
///
/// Returns 409 Conflict if any key is currently assigned to a daemon.
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = DaemonApiKey::ENTITY_NAME_PLURAL,
    operation_id = "bulk_delete_daemon_api_keys",
    summary = "Bulk delete daemon_api_keys",
    request_body(content = Vec<Uuid>, description = "Array of Daemon API Key IDs to delete"),
    responses(
        (status = 200, description = "daemon_api_keys deleted", body = ApiResponse<BulkDeleteResponse>),
        (status = 409, description = "One or more API keys are in use by daemons", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_delete_daemon_api_keys(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
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
