//! Asking the server to poll unreachable ServerPoll daemons again, one at a time or several.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Json;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Can, Write};
use crate::server::config::AppState;
use crate::server::daemons::r#impl::base::{Daemon, DaemonMode};
use crate::server::discovery::r#impl::base::Discovery;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiJson, ApiResponse, ApiResult, EmptyApiResponse,
};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(retry_connection))
        .routes(routes!(bulk_retry_connection))
}

/// Why the caller can't retry this daemon's connection, if they can't.
fn retry_refusal(daemon: &Daemon, site_ids: &[Uuid]) -> Option<ApiError> {
    if !site_ids.contains(&daemon.base.site_id) {
        Some(ApiError::entity_access_denied::<Daemon>(daemon.id))
    } else if daemon.base.mode != DaemonMode::ServerPoll {
        Some(ApiError::bad_request(&format!(
            "Connection retry is only available for ServerPoll mode daemons, and \"{}\" isn't one.",
            daemon.base.name
        )))
    } else {
        None
    }
}

/// Retry connection to an unreachable Daemon
///
/// Resets the is_unreachable flag for a daemon that was marked unreachable
/// due to repeated polling failures. The poller will attempt to contact
/// the daemon again on the next cycle.
#[utoipa::path(
    post,
    path = "/{id}/retry-connection",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "retry_daemon_connection",
    summary = "Retry connection to unreachable daemon",
    params(("id" = Uuid, Path, description = "Daemon ID")),
    responses(
        (status = 200, description = "Connection retry initiated", body = EmptyApiResponse),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn retry_connection(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Discovery, Write>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let mut daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;
    if let Some(refusal) = retry_refusal(&daemon, &auth.site_ids()) {
        return Err(refusal);
    }

    state
        .services
        .daemon_service
        .retry_connection(&mut daemon, auth.into_entity())
        .await?;
    Ok(Json(ApiResponse::success(())))
}

/// Retry connection to unreachable Daemons
///
/// Resets the is_unreachable flag on each daemon, as `POST /{id}/retry-connection` does for one.
/// The poller contacts each again on its next cycle; one that is still down is marked
/// unreachable again, and its unreachable notification is sent again. The request is refused,
/// and nothing changes, if any daemon isn't a ServerPoll daemon.
#[utoipa::path(
    post,
    path = "/bulk-retry-connection",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "bulk_retry_daemon_connections",
    summary = "Retry connection to unreachable daemons",
    request_body(content = Vec<Uuid>, description = "IDs of the daemons to retry"),
    responses(
        (status = 200, description = "Connection retries initiated", body = EmptyApiResponse),
        (status = 400, description = "A daemon isn't a ServerPoll daemon; the message names it", body = ApiErrorResponse),
        (status = 404, description = "A daemon was not found", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_retry_connection(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Discovery, Write>>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<()>>> {
    if ids.is_empty() {
        return Err(ApiError::bulk_empty());
    }
    let daemons = state
        .services
        .daemon_service
        .get_all(StorableFilter::<Daemon>::new_from_entity_ids(&ids))
        .await?;
    if let Some(missing) = ids.iter().find(|id| !daemons.iter().any(|d| d.id == **id)) {
        return Err(ApiError::entity_not_found::<Daemon>(missing));
    }
    let site_ids = auth.site_ids();
    if let Some(refusal) = daemons.iter().find_map(|d| retry_refusal(d, &site_ids)) {
        return Err(refusal);
    }

    let entity = auth.into_entity();
    for mut daemon in daemons {
        state
            .services
            .daemon_service
            .retry_connection(&mut daemon, entity.clone())
            .await?;
    }
    Ok(Json(ApiResponse::success(())))
}
