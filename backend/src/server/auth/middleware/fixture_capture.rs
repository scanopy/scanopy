//! Fixture capture middleware for daemon API compatibility testing.
//!
//! When the `generate-fixtures` feature is enabled, this middleware captures
//! requests from daemons and responses from the server for backwards compatibility testing.
//!
//! Daemon requests are identified by the presence of the `X-Daemon-ID` header.
//!
//! To generate fixtures, run the server with:
//!   cargo run --bin server --features generate-fixtures

use axum::{extract::Request, middleware::Next, response::Response};

/// Marks a request the compat suite replays from an existing fixture. A replay is never a new
/// recording: recording one would file the current version's traffic under the old version it
/// replays, overwriting that version's genuine fixture.
pub const FIXTURE_REPLAY_HEADER: &str = "X-Scanopy-Fixture-Replay";

/// Whether this process should record `request` as a fixture. The test containers are always built
/// with `generate-fixtures`, so recording also needs `CAPTURE_COMPAT_FIXTURES=true`, which the
/// integration suite sets only when it runs with that feature itself (the release run).
#[cfg(feature = "generate-fixtures")]
pub fn should_capture(request: &Request) -> bool {
    std::env::var("CAPTURE_COMPAT_FIXTURES").is_ok_and(|v| v == "true")
        && !request.headers().contains_key(FIXTURE_REPLAY_HEADER)
}

/// Middleware that captures daemon requests and server responses as fixtures.
/// Enabled by `--features generate-fixtures`.
/// Daemon requests are identified by the presence of the X-Daemon-ID header.
#[cfg(feature = "generate-fixtures")]
pub async fn capture_fixtures_middleware(request: Request, next: Next) -> Response {
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
        fixtures_dir()
            .join(format!("v{}", version))
            .join("daemon_to_server.json")
    }

    fn extract_daemon_version(body: &serde_json::Value) -> Option<String> {
        body.get("version")
            .or_else(|| body.get("daemon_version"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn save_openapi_spec(version: &str) {
        let version_dir = fixtures_dir().join(format!("v{}", version));
        let spec_path = version_dir.join("openapi.json");

        // Only copy if it doesn't exist yet
        if spec_path.exists() {
            return;
        }

        // Try to copy from the UI static directory
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.join("ui/static/openapi.json"));

        if let Some(source) = source
            && source.exists()
        {
            if let Err(e) = std::fs::create_dir_all(&version_dir) {
                tracing::warn!("Failed to create fixtures directory: {}", e);
                return;
            }
            if let Err(e) = std::fs::copy(&source, &spec_path) {
                tracing::warn!("Failed to copy OpenAPI spec: {}", e);
            } else {
                tracing::info!("Saved OpenAPI spec: {:?}", spec_path);
            }
        }
    }

    fn save_fixtures(version: &str) {
        let exchanges = {
            let guard = CAPTURED.lock().unwrap();
            guard.clone()
        };

        if exchanges.is_empty() {
            return;
        }

        // Also save the OpenAPI spec for this version
        save_openapi_spec(version);

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

    // Only capture requests from daemons (identified by X-Daemon-ID header)
    if !should_capture(&request) || request.headers().get("X-Daemon-ID").is_none() {
        return next.run(request).await;
    }

    let method = request.method().as_str().to_string();
    let path = request.uri().path().to_string();

    // Skip health checks
    if path.ends_with("/health") {
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

    let version = extract_daemon_version(&request_json)
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

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
