//! Starting, cancelling, and turning schedules on or off for several discoveries at once. Each
//! checks every discovery before acting on any, then acts one at a time, so sessions queue on a
//! shared daemon in a defined order.

use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::auth::middleware::permissions::{Authorized, Can, Write};
use crate::server::config::AppState;
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::discovery::handlers::{session_starting_error, wake_daemon};
use crate::server::discovery::r#impl::base::Discovery;
use crate::server::discovery::service::{order_for_cancel, set_enabled_refusal, start_refusal};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiJson, ApiResponse, ApiResult, EmptyApiResponse,
};
use crate::server::sites::r#impl::Site;

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(bulk_start_sessions))
        .routes(routes!(bulk_cancel_sessions))
        .routes(routes!(bulk_set_enabled))
}

/// Every requested discovery, refusing an empty list, a missing id, or one on a site the caller
/// can't reach.
async fn load_all(
    state: &Arc<AppState>,
    ids: &[Uuid],
    site_ids: &[Uuid],
) -> Result<Vec<Discovery>, ApiError> {
    if ids.is_empty() {
        return Err(ApiError::bulk_empty());
    }
    let discoveries = state
        .services
        .discovery_service
        .get_all(StorableFilter::<Discovery>::new_from_entity_ids(ids))
        .await?;
    if let Some(missing) = ids
        .iter()
        .find(|id| !discoveries.iter().any(|d| d.id == **id))
    {
        return Err(ApiError::entity_not_found::<Discovery>(missing));
    }
    if let Some(outside) = discoveries
        .iter()
        .find(|d| !site_ids.contains(&d.base.site_id))
    {
        return Err(ApiError::entity_access_denied::<Site>(outside.base.site_id));
    }
    Ok(discoveries)
}

/// Start discovery sessions
///
/// Starts a session for each discovery, as `POST /start-session` does for one. Sessions start
/// in turn: on a daemon running several, the first is dispatched and the rest queue behind it.
/// The request is refused, and nothing starts, if any discovery is a past run, a rescan, or
/// already running.
#[utoipa::path(
    post,
    operation_id = "bulk_start_discovery_sessions",
    path = "/bulk-start-session",
    tag = Discovery::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "IDs of the discoveries to start"),
    responses(
        (status = 200, description = "Sessions started, in the order given", body = ApiResponse<Vec<DiscoveryUpdatePayload>>),
        (status = 400, description = "A discovery can't be started; the message names it", body = ApiErrorResponse),
        (status = 404, description = "A discovery was not found", body = ApiErrorResponse),
        (status = 409, description = "A discovery is already running", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_start_sessions(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Discovery, Write>>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<Vec<DiscoveryUpdatePayload>>>> {
    let discoveries = load_all(&state, &ids, &auth.site_ids()).await?;
    let service = &state.services.discovery_service;
    for discovery in &discoveries {
        let running = service
            .has_active_session_for_discovery(&discovery.id)
            .await;
        if let Some(refusal) = start_refusal(discovery, running) {
            return Err(refusal.into_api_error(&discovery.base.name));
        }
    }

    let mut daemon_ids: Vec<Uuid> = discoveries.iter().map(|d| d.base.daemon_id).collect();
    daemon_ids.sort();
    daemon_ids.dedup();
    for daemon_id in daemon_ids {
        wake_daemon(&state, daemon_id).await?;
    }

    // One at a time: concurrent starts on an idle daemon can each see an empty queue and all
    // become Pending, and the extras are then reaped as stalled.
    let entity = auth.into_entity();
    let mut sessions = Vec::with_capacity(discoveries.len());
    for discovery in discoveries {
        sessions.push(service.start_session(discovery, entity.clone()).await?);
    }
    Ok(Json(ApiResponse::success(sessions)))
}

/// Cancel discovery sessions
///
/// Cancels the running session of each discovery, given discovery IDs. Sessions still queued
/// are cancelled before running ones, so cancelling a running session never starts a queued one
/// that is also being cancelled. The request is refused, and nothing is cancelled, if any
/// discovery isn't running or its session is just starting on its daemon.
#[utoipa::path(
    post,
    operation_id = "bulk_cancel_discovery_sessions",
    path = "/bulk-cancel",
    tag = Discovery::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "IDs of the discoveries whose sessions to cancel"),
    responses(
        (status = 200, description = "Sessions cancelled", body = EmptyApiResponse),
        (status = 400, description = "A discovery isn't running; the message names it", body = ApiErrorResponse),
        (status = 404, description = "A discovery was not found", body = ApiErrorResponse),
        (status = 409, description = "A session is starting on its daemon", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_cancel_sessions(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Discovery, Write>>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let discoveries = load_all(&state, &ids, &auth.site_ids()).await?;
    let service = &state.services.discovery_service;
    let mut sessions = Vec::with_capacity(discoveries.len());
    for discovery in &discoveries {
        let session = service
            .session_for_discovery(&discovery.id)
            .await
            .ok_or_else(|| {
                ApiError::bad_request(&format!("\"{}\" isn't running.", discovery.base.name))
            })?;
        if session.phase == DiscoveryPhase::Starting {
            return Err(session_starting_error());
        }
        sessions.push(session);
    }

    order_for_cancel(&mut sessions, |session| session.phase);
    let entity = auth.into_entity();
    for session in sessions {
        service
            .cancel_session(session.session_id, entity.clone())
            .await?;
    }
    Ok(Json(ApiResponse::success(())))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkSetEnabledRequest {
    /// IDs of the scheduled discoveries.
    pub ids: Vec<Uuid>,
    /// Whether their schedules run.
    pub enabled: bool,
}

/// Turn discovery schedules on or off
///
/// Turns the schedule of each scheduled discovery on or off, and changes nothing else about
/// them. The request is refused, and nothing changes, if any discovery has no schedule or is
/// running.
#[utoipa::path(
    post,
    operation_id = "bulk_set_discoveries_enabled",
    path = "/bulk-set-enabled",
    tag = Discovery::ENTITY_NAME_PLURAL,
    request_body = BulkSetEnabledRequest,
    responses(
        (status = 200, description = "Schedules updated", body = ApiResponse<Vec<Discovery>>),
        (status = 400, description = "A discovery has no schedule; the message names it", body = ApiErrorResponse),
        (status = 404, description = "A discovery was not found", body = ApiErrorResponse),
        (status = 409, description = "A discovery is running", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_set_enabled(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Discovery, Write>>,
    ApiJson(request): ApiJson<BulkSetEnabledRequest>,
) -> ApiResult<Json<ApiResponse<Vec<Discovery>>>> {
    let discoveries = load_all(&state, &request.ids, &auth.site_ids()).await?;
    let service = &state.services.discovery_service;
    for discovery in &discoveries {
        let running = service
            .has_active_session_for_discovery(&discovery.id)
            .await;
        if let Some(refusal) = set_enabled_refusal(discovery, running) {
            return Err(refusal);
        }
    }

    let entity = auth.into_entity();
    let mut updated = Vec::with_capacity(discoveries.len());
    for discovery in discoveries {
        updated.push(
            service
                .set_enabled(discovery, request.enabled, entity.clone())
                .await?,
        );
    }
    Ok(Json(ApiResponse::success(updated)))
}
