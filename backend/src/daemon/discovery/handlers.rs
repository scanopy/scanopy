use crate::daemon::discovery::manager::{CancelTarget, InitiateOutcome};
use crate::daemon::runtime::types::DaemonAppState;
use crate::server::{
    daemons::r#impl::api::{DaemonDiscoveryRequest, DaemonDiscoveryResponse},
    shared::types::api::{ApiError, ApiResponse, ApiResult},
};
use axum::{Router, extract::State, response::Json, routing::post};
use std::sync::Arc;
use uuid::Uuid;

pub fn create_router() -> Router<Arc<DaemonAppState>> {
    Router::new()
        .route("/initiate", post(handle_discovery_request))
        .route("/cancel", post(handle_cancel_request))
}

pub async fn handle_discovery_request(
    State(state): State<Arc<DaemonAppState>>,
    Json(request): Json<DaemonDiscoveryRequest>,
) -> ApiResult<Json<ApiResponse<DaemonDiscoveryResponse>>> {
    let session_id = request.session_id;
    tracing::info!(
        "Received {} discovery request, session ID {}",
        request.discovery_type,
        request.session_id
    );

    let manager = &state.services.discovery_manager;

    match manager.try_initiate_session(request).await {
        InitiateOutcome::Started => Ok(Json(ApiResponse::success(DaemonDiscoveryResponse {
            session_id,
        }))),
        InitiateOutcome::Busy { .. } => {
            Err(ApiError::conflict("Discovery session already in progress"))
        }
    }
}

pub async fn handle_cancel_request(
    State(state): State<Arc<DaemonAppState>>,
    Json(session_id): Json<Uuid>,
) -> ApiResult<Json<ApiResponse<Uuid>>> {
    tracing::info!(
        "Received discovery cancellation request for session {}",
        session_id
    );

    let manager = state.services.discovery_manager.clone();

    // Ask the manager once, for this session only. A cancel for a session that has since ended
    // is "nothing to cancel", not a cancel for whatever started next. Just signal cancellation,
    // don't wait: the spawned task winds down and clears its own slot.
    if manager.cancel(CancelTarget::Session(session_id)).await {
        Ok(Json(ApiResponse::success(session_id)))
    } else {
        Err(ApiError::conflict(
            "Discovery session not currently running",
        ))
    }
}
