//! Middleware for the daemon HTTP server.

use axum::{extract::Request, middleware::Next, response::Response};

/// Middleware that captures server-to-daemon requests and daemon responses as fixtures.
/// Enabled by `--features generate-fixtures`.
/// All requests to the daemon are assumed to come from the server.
#[cfg(feature = "generate-fixtures")]
pub async fn capture_fixtures_middleware(request: Request, next: Next) -> Response {
    use crate::server::auth::middleware::fixture_capture::should_capture;
    use axum::body::{Body, to_bytes};
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use std::sync::Mutex;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct CapturedExchange {
        method: String,
        path: String,
        request_body: serde_json::Value,
        response_status: u16,
        response_body: serde_json::Value,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct FixtureManifest {
        version: String,
        exchanges: Vec<CapturedExchange>,
    }

    static CAPTURED: Mutex<Vec<CapturedExchange>> = Mutex::new(Vec::new());

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/integration/compat/fixtures")
    }

    fn manifest_path(version: &str) -> PathBuf {
        // Use mode-specific filenames to avoid race conditions when multiple daemon
        // containers (daemon_poll and server_poll) write to the same shared volume.
        // These are merged into server_to_daemon.json during fixture generation.
        let mode = std::env::var("SCANOPY_MODE").unwrap_or_else(|_| "daemon_poll".to_string());
        let filename = format!("server_to_{}.json", mode);
        fixtures_dir().join(format!("v{}", version)).join(filename)
    }

    fn save_fixtures(version: &str) {
        let exchanges = {
            let guard = CAPTURED.lock().unwrap();
            guard.clone()
        };

        if exchanges.is_empty() {
            return;
        }

        let manifest = FixtureManifest {
            version: version.to_string(),
            exchanges,
        };

        let path = manifest_path(version);
        if let Some(parent) = path.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            tracing::warn!("Failed to create fixtures directory: {}", e);
            return;
        }

        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("Failed to write fixture manifest: {}", e);
            } else {
                tracing::info!("Updated fixture manifest: {:?}", path);
            }
        }
    }

    let method = request.method().as_str().to_string();
    let path = request.uri().path().to_string();

    // Skip health checks
    if !should_capture(&request) || path.ends_with("/health") {
        return next.run(request).await;
    }

    // Extract request body
    let (parts, body) = request.into_parts();
    let request_bytes = match to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Failed to read request body for fixture capture: {}", e);
            let request = Request::from_parts(parts, Body::empty());
            return next.run(request).await;
        }
    };

    let request_json = serde_json::from_slice::<serde_json::Value>(&request_bytes)
        .unwrap_or(serde_json::json!({}));

    let version = env!("CARGO_PKG_VERSION").to_string();

    // Reconstruct request and execute
    let request = Request::from_parts(parts, Body::from(request_bytes));
    let response = next.run(request).await;

    // Extract response body
    let status = response.status().as_u16();
    let (response_parts, response_body) = response.into_parts();
    let response_bytes = match to_bytes(response_body, usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Failed to read response body for fixture capture: {}", e);
            return Response::from_parts(response_parts, Body::empty());
        }
    };

    let response_json = serde_json::from_slice::<serde_json::Value>(&response_bytes)
        .unwrap_or(serde_json::json!({}));

    let exchange = CapturedExchange {
        method: method.clone(),
        path: path.clone(),
        request_body: request_json,
        response_status: status,
        response_body: response_json,
    };

    // Add to captured exchanges and save
    {
        let mut guard = CAPTURED.lock().unwrap();
        // Avoid duplicates (same method + path)
        if !guard
            .iter()
            .any(|e| e.method == exchange.method && e.path == exchange.path)
        {
            guard.push(exchange);
        }
    }
    save_fixtures(&version);

    // Reconstruct response
    Response::from_parts(response_parts, Body::from(response_bytes))
}

/// No-op middleware when feature is disabled
#[cfg(not(feature = "generate-fixtures"))]
pub async fn capture_fixtures_middleware(request: Request, next: Next) -> Response {
    next.run(request).await
}
