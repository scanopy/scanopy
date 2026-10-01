use axum::{
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use std::sync::Arc;

use crate::server::{
    auth::middleware::permissions::{Authorized, IsExternalService, Prometheus},
    config::AppState,
    shared::types::api::ApiError,
};

/// Get Prometheus metrics.
///
/// Requires external service authentication with X-Service-Name: prometheus header.
/// IP restrictions can be configured via SCANOPY_EXTERNAL_SERVICE_PROMETHEUS_ALLOWED_IPS.
pub async fn get_metrics(
    _auth: Authorized<IsExternalService<Prometheus>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Check if metrics are enabled (token configured)
    if state.config.metrics_token.is_none() {
        return ApiError::not_found("Metrics not enabled".to_string()).into_response();
    }

    let metrics = state.services.metrics_service.handle.render();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        metrics,
    )
        .into_response()
}
