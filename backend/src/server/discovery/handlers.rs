use crate::server::{
    auth::middleware::{
        auth::AuthenticatedEntity,
        permissions::{Authorized, IsDaemon, Member, Viewer},
    },
    config::AppState,
    daemons::r#impl::{api::DiscoveryUpdatePayload, version::supports_unified_discovery},
    discovery::r#impl::{
        base::Discovery,
        types::{DiscoveryType, RunType},
    },
    networks::r#impl::Network,
    shared::{
        handlers::traits::{create_handler, update_handler},
        services::traits::CrudService,
        storage::traits::Entity,
        types::{
            api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse},
            error_codes::ErrorCode,
        },
    },
};
use axum::http::StatusCode;
use axum::{
    extract::{Path, State},
    response::{
        Json, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use futures::Stream;
use std::{convert::Infallible, sync::Arc};
use tokio::sync::broadcast;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    crate::crud_get_all_handler!(Discovery);
    crate::crud_get_by_id_handler!(Discovery);
    crate::crud_delete_handler!(Discovery);
    crate::crud_bulk_delete_handler!(Discovery);
    crate::crud_export_csv_handler!(Discovery);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_discovery))
        .routes(routes!(
            generated::get_by_id,
            update_discovery,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
        .routes(routes!(start_session))
        .routes(routes!(get_active_sessions))
        .routes(routes!(cancel_discovery))
        // Internal daemon endpoints
        .routes(routes!(receive_discovery_update))
        // SSE endpoint (internal - not well-supported by OpenAPI)
        .route("/stream", get(discovery_stream))
}

/// Create new Discovery
#[utoipa::path(
    post,
    path = "",
    tag = Discovery::ENTITY_NAME_PLURAL,
    request_body = Discovery,
    responses(
        (status = 200, description = "Discovery created successfully", body = ApiResponse<Discovery>),
        (status = 400, description = "Invalid subnet network", body = ApiErrorResponse),
        (status = 400, description = "Can't create historical discovery", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_discovery(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(discovery): Json<Discovery>,
) -> ApiResult<Json<ApiResponse<Discovery>>> {
    if let RunType::Historical { .. } = discovery.base.run_type {
        return Err(ApiError::discovery_historical_read_only());
    }

    // Reject legacy discovery types — only Unified can be created
    if discovery.base.discovery_type.is_legacy() {
        return Err(ApiError::bad_request(&format!(
            "{} discoveries are frozen. Create a Unified discovery instead.",
            discovery.base.discovery_type,
        )));
    }

    // For Unified: check daemon version supports it
    if let DiscoveryType::Unified { .. } = &discovery.base.discovery_type {
        let daemon = state
            .services
            .daemon_service
            .get_by_id(&discovery.base.daemon_id)
            .await?
            .ok_or_else(|| ApiError::not_found("Daemon not found".to_string()))?;

        if !supports_unified_discovery(daemon.base.version.as_ref()) {
            return Err(ApiError::bad_request(
                "Daemon does not support unified discovery. Upgrade to version 0.15.0 or later.",
            ));
        }
    }

    // Check scheduled discovery restriction
    if matches!(discovery.base.run_type, RunType::Scheduled { .. })
        && let Some(org_id) = auth.organization_id()
        && let Some(org) = state
            .services
            .organization_service
            .get_by_id(&org_id)
            .await?
        && let Some(plan) = &org.base.plan
        && !plan.features().scheduled_discovery
    {
        return Err(ApiError::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::BillingFeatureNotAvailable {
                feature: "Scheduled Discovery".into(),
            },
        ));
    }

    // Validate subnet network membership for Network and Unified types
    let subnet_ids_to_check = match &discovery.base.discovery_type {
        DiscoveryType::Network { subnet_ids, .. } | DiscoveryType::Unified { subnet_ids, .. } => {
            subnet_ids.clone()
        }
        _ => None,
    };
    if let Some(ids) = subnet_ids_to_check {
        for subnet_id in &ids {
            if let Some(subnet) = state.services.subnet_service.get_by_id(subnet_id).await?
                && subnet.base.network_id != discovery.base.network_id
            {
                return Err(ApiError::discovery_subnet_network_mismatch(
                    &subnet.base.name,
                ));
            }
        }
    }

    // Delegate to generic handler (handles validation, auth checks, creation)
    create_handler::<Discovery>(State(state), auth, Json(discovery)).await
}

/// Update Discovery
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Discovery::ENTITY_NAME_PLURAL,
    params(("id" = uuid::Uuid, Path, description = "Discovery ID")),
    request_body = Discovery,
    responses(
        (status = 200, description = "Discovery updated successfully", body = ApiResponse<Discovery>),
        (status = 400, description = "Invalid subnet network", body = ApiErrorResponse),
        (status = 400, description = "Can't update historical discovery", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn update_discovery(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    id: Path<Uuid>,
    discovery: Json<Discovery>,
) -> ApiResult<Json<ApiResponse<Discovery>>> {
    if let RunType::Historical { .. } = discovery.base.run_type {
        return Err(ApiError::discovery_historical_read_only());
    }

    // Reject changing a legacy discovery's type
    if let Some(existing) = state.services.discovery_service.get_by_id(&id).await?
        && existing.base.discovery_type.is_legacy()
        && std::mem::discriminant(&existing.base.discovery_type)
            != std::mem::discriminant(&discovery.base.discovery_type)
    {
        return Err(ApiError::bad_request(
            "Cannot change the type of a legacy discovery. Create a new Unified discovery instead.",
        ));
    }

    // Check scheduled discovery restriction
    if matches!(discovery.base.run_type, RunType::Scheduled { .. })
        && let Some(org_id) = auth.organization_id()
        && let Some(org) = state
            .services
            .organization_service
            .get_by_id(&org_id)
            .await?
        && let Some(plan) = &org.base.plan
        && !plan.features().scheduled_discovery
    {
        return Err(ApiError::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::BillingFeatureNotAvailable {
                feature: "Scheduled Discovery".into(),
            },
        ));
    }

    update_handler::<Discovery>(state, auth, id, discovery).await
}

/// Receive discovery progress update from daemon
///
/// Internal endpoint for daemons to report discovery progress.
#[utoipa::path(
    post,
    path = "/{session_id}/update",
    tags = [Discovery::ENTITY_NAME_PLURAL, "internal"],
    params(("session_id" = Uuid, Path, description = "Discovery session ID")),
    request_body = DiscoveryUpdatePayload,
    responses(
        (status = 200, description = "Update received", body = EmptyApiResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn receive_discovery_update(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(_session_id): Path<Uuid>,
    Json(update): Json<DiscoveryUpdatePayload>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // IsDaemon guarantees exactly one network_id and a daemon_id
    let daemon_network_id = auth.network_ids()[0];
    let daemon_id = auth
        .daemon_id()
        .ok_or_else(|| anyhow::anyhow!("Could not get daemon ID from authentication"))?;

    // Validate daemon can only send updates for their own network
    if update.network_id != daemon_network_id {
        return Err(ApiError::daemon_network_mismatch());
    }

    // Validate daemon can only send updates as themselves
    if update.daemon_id != daemon_id {
        return Err(ApiError::daemon_identity_mismatch());
    }

    // Delegate to processor for shared progress update logic
    // This ensures both DaemonPoll and ServerPoll modes use the same logic
    state
        .services
        .daemon_service
        .process_discovery_progress(update)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

/// Start a Discovery Session
#[utoipa::path(
    post,
    path = "/start-session",
    tag = Discovery::ENTITY_NAME_PLURAL,
    request_body = Uuid,
    responses(
        (status = 200, description = "Discovery session started", body = ApiResponse<DiscoveryUpdatePayload>),
        (status = 404, description = "Discovery not found", body = ApiErrorResponse),
        (status = 409, description = "A session is already running for this discovery", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn start_session(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(discovery_id): Json<Uuid>,
) -> ApiResult<Json<ApiResponse<DiscoveryUpdatePayload>>> {
    let network_ids = auth.network_ids();
    let entity = auth.into_entity();

    let discovery = state
        .services
        .discovery_service
        .get_by_id(&discovery_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Discovery>(discovery_id))?;

    // Validate user has access to this discovery's network
    if !network_ids.contains(&discovery.base.network_id) {
        return Err(ApiError::entity_access_denied::<Network>(
            discovery.base.network_id,
        ));
    }

    // Auto-wake daemon if on standby
    if let Some(mut daemon) = state
        .services
        .daemon_service
        .get_by_id(&discovery.base.daemon_id)
        .await?
        && daemon.base.standby
    {
        daemon.base.standby = false;
        state
            .services
            .daemon_service
            .update(&mut daemon, AuthenticatedEntity::System)
            .await?;
        tracing::info!(
            daemon_id = %daemon.id,
            "Cleared daemon standby (discovery session started)"
        );
    }

    let update = state
        .services
        .discovery_service
        .start_session(discovery, entity)
        .await?;

    Ok(Json(ApiResponse::success(update)))
}

async fn discovery_stream(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.services.discovery_service.subscribe();
    let allowed_networks = auth.network_ids();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(update) => {
                    // Only emit if user has access to this discovery's network
                    if allowed_networks.contains(&update.network_id) {
                        let json = serde_json::to_string(&update).unwrap_or_default();
                        yield Ok(Event::default().data(json));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("SSE client lagged by {} messages", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Get active Discovery Sessions
#[utoipa::path(
    get,
    path = "/active-sessions",
    tag = Discovery::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "List of active discovery sessions", body = ApiResponse<Vec<DiscoveryUpdatePayload>>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_active_sessions(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> ApiResult<Json<ApiResponse<Vec<DiscoveryUpdatePayload>>>> {
    let network_ids = auth.network_ids();
    let sessions = state
        .services
        .discovery_service
        .get_all_sessions(&network_ids)
        .await;

    Ok(Json(ApiResponse::success(sessions)))
}

/// Cancel a Discovery Session
#[utoipa::path(
    post,
    path = "/{session_id}/cancel",
    tag = Discovery::ENTITY_NAME_PLURAL,
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Discovery session cancelled", body = EmptyApiResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn cancel_discovery(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Get session and validate user has access to this session's network
    let session = state
        .services
        .discovery_service
        .get_session(&session_id)
        .await
        .ok_or_else(|| ApiError::discovery_session_not_found(session_id))?;

    if !auth.network_ids().contains(&session.network_id) {
        return Err(ApiError::entity_access_denied::<Network>(
            session.network_id,
        ));
    }

    state
        .services
        .discovery_service
        .cancel_session(session_id, auth.into_entity())
        .await?;

    tracing::info!("Discovery session was {} cancelled", session_id);
    Ok(Json(ApiResponse::success(())))
}
