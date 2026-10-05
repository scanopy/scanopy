use crate::{
    daemon::{
        discovery::handlers as discovery_handlers,
        runtime::{
            service::LOG_TARGET,
            state::{CreatedEntitiesPayload, DaemonStatus, DiscoveryPollResponse},
            types::{DaemonAppState, InitializeDaemonRequest},
        },
        shared::auth::server_auth_middleware,
    },
    server::{
        daemons::r#impl::{
            api::{FirstContactRequest, LegacyCapabilities},
            base::DaemonMode,
        },
        shared::{
            legacy::site_wire_request_middleware,
            types::api::{ApiJson, ApiResponse, ApiResult},
        },
    },
};
use axum::{
    Json, Router,
    extract::State,
    middleware,
    routing::{get, post},
};
use std::sync::Arc;

/// Create daemon HTTP router.
/// The `state` parameter is required for applying authentication middleware
/// to ServerPoll mode endpoints.
pub fn create_router(state: Arc<DaemonAppState>, mode: DaemonMode) -> Router<Arc<DaemonAppState>> {
    // Public routes (no auth required). Served in both modes: `/api/health` backs the
    // container healthchecks (docker-compose.yml, Dockerfile.daemon) and the daemon's own
    // service detection, and `/api/initialize` is how a server with
    // SCANOPY_INTEGRATED_DAEMON_URL bootstraps the DaemonPoll daemon it ships alongside.
    let public_routes = Router::new()
        .route("/api/health", get(get_health))
        .route("/api/initialize", post(initialize));

    // The rest of the surface exists solely so the server can drive a daemon it polls, and
    // the server only ever dials daemons in ServerPoll mode. A DaemonPoll daemon reaches out
    // for its work instead, so it serves none of them.
    if mode == DaemonMode::DaemonPoll {
        return public_routes.layer(middleware::from_fn(site_wire_request_middleware));
    }

    // Authenticated routes (ServerPoll mode - server must provide valid API key)
    // Discovery initiate/cancel require auth to prevent unauthorized scans
    let authenticated_routes = Router::new()
        .route("/api/status", get(get_status))
        .route("/api/first-contact", post(handle_first_contact))
        .route("/api/poll", get(get_discovery_poll))
        .route(
            "/api/discovery/entities-created",
            post(receive_created_entities),
        )
        .route(
            "/api/discovery/initiate",
            post(discovery_handlers::handle_discovery_request),
        )
        .route(
            "/api/discovery/cancel",
            post(discovery_handlers::handle_cancel_request),
        )
        .route_layer(middleware::from_fn_with_state(
            state,
            server_auth_middleware,
        ));

    // A server that predates the site rename sends `network_id`.
    public_routes
        .merge(authenticated_routes)
        .layer(middleware::from_fn(site_wire_request_middleware))
}

async fn get_health() -> ApiResult<Json<ApiResponse<String>>> {
    tracing::info!("Received healthcheck request");

    Ok(Json(ApiResponse::success(
        "Scanopy Daemon Running".to_string(),
    )))
}

async fn initialize(
    State(state): State<Arc<DaemonAppState>>,
    ApiJson(request): ApiJson<InitializeDaemonRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    // Check if daemon is already initialized (once-only guard)
    // Prevents re-initialization attacks - if both site_id and api_key are set,
    // return success without modifying the configuration
    let existing_site_id = state.config.get_site_id().await.ok().flatten();
    let existing_api_key = state.config.get_api_key().await.ok().flatten();

    if existing_site_id.is_some() && existing_api_key.is_some() {
        tracing::warn!(
            site_id = %request.site_id,
            "Received initialization request but daemon is already initialized - ignoring"
        );
        return Ok(Json(ApiResponse::success(
            "Daemon already initialized".to_string(),
        )));
    }

    tracing::info!(
        site_id = %request.site_id,
        api_key = %request.api_key,
        "Received initialization signal",
    );

    state
        .services
        .runtime_service
        .initialize_services(request.site_id, request.api_key)
        .await?;

    Ok(Json(ApiResponse::success(
        "Daemon initialized successfully".to_string(),
    )))
}

/// Get daemon status (for ServerPoll mode).
/// Returns lightweight status: url, name, mode, version.
async fn get_status(
    State(state): State<Arc<DaemonAppState>>,
) -> ApiResult<Json<ApiResponse<DaemonStatus>>> {
    let status = state.services.daemon_state.get_status().await;
    Ok(Json(ApiResponse::success(status)))
}

/// Handle first contact from server (for ServerPoll mode).
/// Server calls this on first poll to assign the daemon its server-side ID.
/// Returns daemon status (same as GET /api/status) to avoid extra round-trip.
async fn handle_first_contact(
    State(state): State<Arc<DaemonAppState>>,
    ApiJson(request): ApiJson<FirstContactRequest>,
) -> ApiResult<Json<ApiResponse<DaemonStatus>>> {
    let current_id = state.config.get_id().await.unwrap_or_default();

    tracing::info!(
        target: LOG_TARGET,
        current_id = %current_id,
        assigned_id = %request.daemon_id,
        "Received first contact from server"
    );

    // Store the server-assigned daemon ID
    if let Err(e) = state.config.set_id(request.daemon_id).await {
        tracing::error!(
            target: LOG_TARGET,
            error = %e,
            "Failed to store assigned daemon ID"
        );
    } else {
        tracing::info!(
            target: LOG_TARGET,
            daemon_id = %request.daemon_id,
            "Stored server-assigned daemon ID"
        );
    }

    // Cache the server-provisioned identity (additive; older servers omit these).
    if let Some(site_id) = request.site_id
        && let Err(e) = state.config.set_site_id(site_id).await
    {
        tracing::error!(
            target: LOG_TARGET,
            error = %e,
            "Failed to store assigned site ID"
        );
    }
    if let Some(name) = request.name.clone()
        && let Err(e) = state.config.set_name(name).await
    {
        tracing::error!(
            target: LOG_TARGET,
            error = %e,
            "Failed to store assigned daemon name"
        );
    }

    // Log server capabilities
    tracing::info!(
        target: LOG_TARGET,
        "  Server version:  {}",
        request.server_capabilities.server_version
    );
    tracing::info!(
        target: LOG_TARGET,
        "  Min daemon ver:  {}",
        request.server_capabilities.minimum_daemon_version
    );

    // Log deprecation warnings
    request.server_capabilities.log_warnings();

    let capabilities = LegacyCapabilities {
        interfaced_subnet_ids: vec![],
    };

    state.config.set_capabilities(capabilities).await?;

    // Return current status (saves an extra round-trip)
    let status = state.services.daemon_state.get_status().await;
    Ok(Json(ApiResponse::success(status)))
}

/// Get discovery poll data (for ServerPoll mode).
/// Returns current progress and any pending buffered entities.
///
/// Note: Entities remain in the buffer until the server confirms them
/// via POST /api/discovery/entities-created. This prevents the race condition
/// where entities are removed before confirmation can be received.
async fn get_discovery_poll(
    State(state): State<Arc<DaemonAppState>>,
) -> ApiResult<Json<ApiResponse<DiscoveryPollResponse>>> {
    let progress = state.services.daemon_state.get_progress().await;
    let entities = state.services.daemon_state.get_pending_entities().await;

    // Clear terminal payload after serving it so it's not resent on the next poll.
    // The server only needs to receive the terminal state once.
    if let Some(ref p) = progress
        && p.phase.is_terminal()
    {
        state.services.daemon_state.clear_terminal_payload().await;
    }

    Ok(Json(ApiResponse::success(DiscoveryPollResponse {
        progress,
        entities,
    })))
}

/// Receive created entity confirmations from server (for ServerPoll mode).
/// Server sends back actual entities (with deduped IDs) after processing polled entities.
///
/// This completes the ServerPoll entity lifecycle:
/// 1. Discovery pushes entity to buffer (Pending state)
/// 2. Server polls → get_pending() returns pending entities
/// 3. Server processes entities → sends confirmation here
/// 4. This handler marks entities as Created → await_*() can now find them
///
/// Note: Created entries are NOT cleared immediately. They remain in the buffer
/// so that await_*() calls can find them. Since get_pending() only returns
/// Pending entries, the server won't re-process them. Cleanup happens when
/// the discovery service clears the buffer at session boundaries.
async fn receive_created_entities(
    State(state): State<Arc<DaemonAppState>>,
    ApiJson(payload): ApiJson<CreatedEntitiesPayload>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let buffer = state.services.daemon_state.entity_buffer();

    let subnet_count = payload.subnets.len();
    let host_count = payload.hosts.len();

    // Mark subnets as created with actual server data
    for (pending_id, actual_subnet) in payload.subnets {
        if let Some((old_id, new_id)) = buffer.mark_subnet_created(pending_id, actual_subnet).await
        {
            tracing::debug!(
                old_id = %old_id,
                new_id = %new_id,
                "Subnet ID changed after server deduplication"
            );
        }
    }

    // Mark hosts as created with actual server data
    for (pending_id, actual_host) in payload.hosts {
        buffer.mark_host_created(pending_id, actual_host).await;
    }

    // Note: We intentionally do NOT call clear_created() here.
    // The await_*() methods poll for Created entries, and if we clear immediately,
    // there's a race condition where the entry is removed before the polling loop
    // can observe it. Entries remain as Created (won't be re-sent by get_pending())
    // and can be cleaned up at session boundaries.

    tracing::info!(
        subnets = subnet_count,
        hosts = host_count,
        "Received and processed created entities confirmation from server"
    );

    Ok(Json(ApiResponse::success(
        "Created entities acknowledged".to_string(),
    )))
}
