//! Authentication middleware for daemon HTTP server (ServerPoll mode).
//!
//! In ServerPoll mode, the server polls the daemon rather than the daemon pushing
//! to the server. The server authenticates by sending a Bearer token that matches
//! the daemon's configured API key.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    daemon::runtime::types::DaemonAppState,
    server::shared::{api_key_common::hash_api_key, types::api::ApiError},
};

/// Extract Bearer token from Authorization header
fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Middleware that validates Bearer token authentication for ServerPoll mode endpoints.
///
/// This middleware:
/// 1. Extracts the Bearer token from the Authorization header
/// 2. Hashes the token using SHA-256 (same as server-side key hashing)
/// 3. Compares to the hash of the daemon's configured API key
/// 4. Rejects the request if validation fails
///
/// Note: This middleware should only be applied to routes that require authentication
/// (the ServerPoll endpoints: /api/status, /api/poll). Health checks and other
/// public endpoints should not use this middleware.
pub async fn server_auth_middleware(
    State(state): State<Arc<DaemonAppState>>,
    request: Request,
    next: Next,
) -> Response {
    // Extract Bearer token from header
    let token = match extract_bearer_token(request.headers()) {
        Some(t) => t,
        None => {
            tracing::debug!("Missing or invalid Authorization header");
            return ApiError::unauthorized("Missing or invalid Authorization header".to_string())
                .into_response();
        }
    };

    // Get the daemon's configured API key
    let expected_key = match state.config.get_api_key().await {
        Ok(Some(key)) => key,
        Ok(None) => {
            tracing::warn!("Daemon not configured with API key, rejecting request");
            return ApiError::unauthorized("Daemon not configured".to_string()).into_response();
        }
        Err(e) => {
            tracing::error!("Failed to get API key from config: {}", e);
            return ApiError::internal_error("Configuration error").into_response();
        }
    };

    // Hash both tokens and compare
    let incoming_hash = hash_api_key(token);
    let expected_hash = hash_api_key(&expected_key);

    if incoming_hash != expected_hash {
        tracing::warn!(
            "Rejected server poll: API key mismatch — the server presented a key that doesn't \
             match this daemon's configured key. Re-provision/re-install this daemon so its key \
             matches the server."
        );
        return ApiError::unauthorized("Invalid API key".to_string()).into_response();
    }

    // Authentication successful, proceed to handler
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();

        // No header
        assert_eq!(extract_bearer_token(&headers), None);

        // Valid Bearer token
        headers.insert("Authorization", "Bearer test-token-123".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), Some("test-token-123"));

        // Wrong scheme
        headers.insert("Authorization", "Basic abc123".parse().unwrap());
        assert_eq!(extract_bearer_token(&headers), None);
    }
}
