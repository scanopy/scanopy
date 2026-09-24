//! Replay tests for API compatibility.
//!
//! These tests load captured fixtures and replay them against a running
//! server or daemon to verify backwards compatibility.
//!
//! ## Integration Test Usage
//!
//! The `run_server_compat_tests` and `run_daemon_compat_tests` functions are
//! called from the integration test suite to verify compatibility with older
//! daemon/server versions using the already-running containers.

use super::schema::validate_response;
use super::types::{
    CapturedExchange, FixtureManifest, get_fixture_versions, load_manifest, load_openapi_spec,
};
use regex::Regex;
use scanopy::server::auth::middleware::fixture_capture::FIXTURE_REPLAY_HEADER;
use uuid::Uuid;

/// Context for replaying requests with substituted IDs.
pub struct ReplayContext {
    pub daemon_id: Uuid,
    pub network_id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub api_key: String,
}

impl ReplayContext {
    /// Substitute IDs in a path.
    /// Replaces any UUID in the path with the daemon_id.
    pub fn substitute_path(&self, path: &str) -> String {
        let uuid_regex = Regex::new(
            r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        )
        .unwrap();

        uuid_regex
            .replace_all(path, self.daemon_id.to_string().as_str())
            .to_string()
    }

    /// Substitute IDs in a request body.
    /// Replaces known ID fields with test context values.
    pub fn substitute_body(&self, body: &serde_json::Value) -> serde_json::Value {
        let mut body = body.clone();

        if let Some(obj) = body.as_object_mut() {
            // Replace known ID fields
            if obj.contains_key("daemon_id") {
                obj.insert(
                    "daemon_id".to_string(),
                    serde_json::json!(self.daemon_id.to_string()),
                );
            }
            if obj.contains_key("network_id") {
                obj.insert(
                    "network_id".to_string(),
                    serde_json::json!(self.network_id.to_string()),
                );
            }
            if obj.contains_key("user_id") {
                obj.insert(
                    "user_id".to_string(),
                    serde_json::json!(self.user_id.to_string()),
                );
            }
            if obj.contains_key("organization_id") {
                obj.insert(
                    "organization_id".to_string(),
                    serde_json::json!(self.organization_id.to_string()),
                );
            }

            // Recursively process nested objects and arrays
            for (_, value) in obj.iter_mut() {
                if value.is_object() {
                    *value = self.substitute_body(value);
                } else if value.is_array() {
                    *value = self.substitute_array(value);
                }
            }
        }

        body
    }

    /// Recursively substitute IDs in array elements.
    fn substitute_array(&self, arr: &serde_json::Value) -> serde_json::Value {
        if let Some(items) = arr.as_array() {
            serde_json::Value::Array(
                items
                    .iter()
                    .map(|item| {
                        if item.is_object() {
                            self.substitute_body(item)
                        } else if item.is_array() {
                            self.substitute_array(item)
                        } else {
                            item.clone()
                        }
                    })
                    .collect(),
            )
        } else {
            arr.clone()
        }
    }
}

/// Result of replaying an exchange.
pub struct ReplayResult {
    pub exchange: CapturedExchange,
    pub actual_status: u16,
    pub actual_body: serde_json::Value,
    pub status_ok: bool,
    pub schema_validation: Option<Result<(), String>>,
}

impl ReplayResult {
    /// Check if the replay was fully successful (2xx status and valid schema).
    pub fn is_success(&self) -> bool {
        self.status_ok && self.schema_validation.as_ref().is_none_or(|r| r.is_ok())
    }

    /// Format result for display.
    fn format_result(&self) -> String {
        if self.is_success() {
            format!(
                "  ✓ {} {} -> {} (schema: valid)",
                self.exchange.method, self.exchange.path, self.actual_status
            )
        } else if self.status_ok {
            let schema_err = self
                .schema_validation
                .as_ref()
                .and_then(|r| r.as_ref().err())
                .map(|s| s.as_str())
                .unwrap_or("unknown");
            format!(
                "  ✗ {} {} -> {} (schema validation failed)\n    {}",
                self.exchange.method, self.exchange.path, self.actual_status, schema_err
            )
        } else {
            format!(
                "  ✗ {} {} -> {} (expected 2xx)\n    Response: {}",
                self.exchange.method,
                self.exchange.path,
                self.actual_status,
                serde_json::to_string_pretty(&self.actual_body).unwrap_or_default()
            )
        }
    }
}

/// Replay a single exchange against a server/daemon.
pub async fn replay_exchange(
    client: &reqwest::Client,
    base_url: &str,
    exchange: &CapturedExchange,
    ctx: &ReplayContext,
    openapi: Option<&serde_json::Value>,
) -> Result<ReplayResult, String> {
    let path = ctx.substitute_path(&exchange.path);
    let url = format!("{}{}", base_url, path);
    let body = ctx.substitute_body(&exchange.request_body);

    let mut req = match exchange.method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url).json(&body),
        "PUT" => client.put(&url).json(&body),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url).json(&body),
        _ => client.get(&url),
    };

    // Add daemon headers for server requests
    req = req
        .header(FIXTURE_REPLAY_HEADER, "true")
        .header("X-Daemon-ID", ctx.daemon_id.to_string())
        .header("Authorization", format!("Bearer {}", &ctx.api_key));

    let response = req.send().await.map_err(|e| e.to_string())?;
    let actual_status = response.status().as_u16();
    let actual_body = response
        .json::<serde_json::Value>()
        .await
        .unwrap_or(serde_json::json!({}));

    // Check status is 2xx
    let status_ok = (200..300).contains(&actual_status);

    // Validate response against OpenAPI schema if available
    let schema_validation = openapi.map(|spec| {
        validate_response(
            spec,
            &exchange.path, // Use original path for schema lookup
            &exchange.method,
            actual_status,
            &actual_body,
        )
    });

    Ok(ReplayResult {
        exchange: exchange.clone(),
        actual_status,
        actual_body,
        status_ok,
        schema_validation,
    })
}

/// Paths that should be skipped during replay.
/// Note: Intentionally empty - discovery data is now cleared before compat tests
/// run, giving fixtures a clean slate without FK constraint issues.
const SKIP_PATH_PREFIXES: &[&str] = &[];

/// Path suffixes to skip during replay.
/// Registration and startup are version-gated — old daemons are rejected.
const SKIP_PATH_SUFFIXES: &[&str] = &["/register", "/startup"];

/// Paths that should be skipped during daemon replay tests.
/// Note: Intentionally empty - we cancel discovery sessions between fixtures.
const SKIP_DAEMON_PATH_PREFIXES: &[&str] = &[];

/// Check if an exchange path should be skipped.
fn should_skip_path(path: &str, is_daemon_test: bool) -> bool {
    let skip_general = SKIP_PATH_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || SKIP_PATH_SUFFIXES
            .iter()
            .any(|suffix| path.ends_with(suffix));

    let skip_daemon = is_daemon_test
        && SKIP_DAEMON_PATH_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix));

    skip_general || skip_daemon
}

/// Replay all exchanges from a manifest.
///
/// Only replays exchanges that originally returned 2xx status codes.
/// Exchanges that originally failed (4xx, 5xx) are skipped since they
/// represent expected failure cases in the original flow (e.g., startup
/// before registration returns 404).
///
/// Some paths with complex entity dependencies are also skipped.
pub async fn replay_manifest(
    client: &reqwest::Client,
    base_url: &str,
    manifest: &FixtureManifest,
    ctx: &ReplayContext,
    openapi: Option<&serde_json::Value>,
    is_daemon_test: bool,
) -> Vec<Result<ReplayResult, String>> {
    let mut results = Vec::new();

    for exchange in &manifest.exchanges {
        // Skip exchanges that originally failed - these are expected failure cases
        // in the original flow (e.g., startup before registration returns 404)
        if !(200..300).contains(&exchange.response_status) {
            continue;
        }

        // Skip paths with complex entity dependencies
        if should_skip_path(&exchange.path, is_daemon_test) {
            continue;
        }

        let result = replay_exchange(client, base_url, exchange, ctx, openapi).await;
        results.push(result);
    }

    results
}

/// Run server compatibility tests - replays old daemon requests against current server.
/// Returns Ok(()) if all fixtures replay successfully, Err with details otherwise.
pub async fn run_server_compat_tests(
    server_url: &str,
    ctx: &ReplayContext,
    clear_data_fn: impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    let versions = get_fixture_versions("daemon_to_server.json");
    if versions.is_empty() {
        println!("  No daemon_to_server fixtures found, skipping");
        return Ok(());
    }

    let client = reqwest::Client::new();

    for version in versions {
        // Clear discovery data before each fixture version to prevent CIDR/ID collisions
        clear_data_fn()?;

        let Some(manifest) = load_manifest(&version, "daemon_to_server.json") else {
            continue;
        };

        let openapi = load_openapi_spec(&version);
        if openapi.is_none() {
            println!(
                "  Warning: No OpenAPI spec for v{}, skipping schema validation",
                version
            );
        }

        println!("  Testing server compatibility with daemon v{}", version);

        let results =
            replay_manifest(&client, server_url, &manifest, ctx, openapi.as_ref(), false).await;

        for result in results {
            match result {
                Ok(r) if r.is_success() => {
                    println!("{}", r.format_result());
                }
                Ok(r) => {
                    return Err(r.format_result());
                }
                Err(e) => {
                    return Err(format!("  ✗ Request failed: {}", e));
                }
            }
        }
    }

    Ok(())
}

fn daemon_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))
}

/// Wait until the daemon reports no running session (`ready_for_work` on `GET /api/status`).
pub async fn wait_for_daemon_idle(daemon_url: &str, api_key: &str) -> Result<(), String> {
    let client = daemon_client()?;
    // 30 seconds (120 * 250ms): a cancelled scan winds down cooperatively.
    for i in 0..120 {
        let status: serde_json::Value = client
            .get(format!("{}/api/status", daemon_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header(FIXTURE_REPLAY_HEADER, "true")
            .send()
            .await
            .map_err(|e| format!("Failed to read daemon status: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse daemon status: {}", e))?;

        if status["data"]["ready_for_work"].as_bool() == Some(true) {
            if i > 0 {
                println!("    Daemon idle after {} ms", i * 250);
            }
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    Err("Daemon still running a discovery session after 30 seconds".to_string())
}

/// Cancel `session_id` on the daemon and wait for the daemon to go idle. The daemon cancels by
/// session id, so a 409 means that session is not running.
pub async fn stop_daemon_session(
    daemon_url: &str,
    api_key: &str,
    session_id: Uuid,
) -> Result<(), String> {
    let response = daemon_client()?
        .post(format!("{}/api/discovery/cancel", daemon_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header(FIXTURE_REPLAY_HEADER, "true")
        .json(&session_id)
        .send()
        .await
        .map_err(|e| format!("Failed to cancel daemon discovery: {}", e))?;

    let status = response.status();
    if !status.is_success() && status != reqwest::StatusCode::CONFLICT {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Failed to cancel daemon discovery {}: {} - {}",
            session_id, status, body
        ));
    }
    wait_for_daemon_idle(daemon_url, api_key).await
}

/// The session a manifest's `/api/discovery/initiate` starts. The replay sends the fixture's
/// session id unchanged, so this is the id the daemon runs it under.
fn initiated_session_id(manifest: &FixtureManifest) -> Option<Uuid> {
    manifest
        .exchanges
        .iter()
        .find(|e| e.path == "/api/discovery/initiate" && (200..300).contains(&e.response_status))
        .and_then(|e| e.request_body["session_id"].as_str())
        .and_then(|id| id.parse().ok())
}

/// Run daemon compatibility tests - replays old server requests against current daemon.
/// Returns Ok(()) if all fixtures replay successfully, Err with details otherwise.
///
/// Note: This requires a ServerPoll daemon (exposes HTTP API) to be running.
/// The test environment should include a daemon-serverpoll container.
pub async fn run_daemon_compat_tests(daemon_url: &str, ctx: &ReplayContext) -> Result<(), String> {
    let versions = get_fixture_versions("server_to_daemon.json");
    if versions.is_empty() {
        println!("  No server_to_daemon fixtures found, skipping");
        return Ok(());
    }

    let client = daemon_client()?;

    // Each version's /api/discovery/initiate needs an idle daemon
    wait_for_daemon_idle(daemon_url, &ctx.api_key).await?;

    for version in versions {
        let Some(manifest) = load_manifest(&version, "server_to_daemon.json") else {
            continue;
        };

        let openapi = load_openapi_spec(&version);
        if openapi.is_none() {
            println!(
                "  Warning: No OpenAPI spec for v{}, skipping schema validation",
                version
            );
        }

        println!("  Testing daemon compatibility with server v{}", version);

        let results =
            replay_manifest(&client, daemon_url, &manifest, ctx, openapi.as_ref(), true).await;

        for result in results {
            match result {
                Ok(r) if r.is_success() => {
                    println!("{}", r.format_result());
                }
                Ok(r) => {
                    return Err(r.format_result());
                }
                Err(e) => {
                    return Err(format!("  ✗ Request failed: {}", e));
                }
            }
        }

        // A Unified initiate runs a real scan; stop it before the next version initiates
        if let Some(session_id) = initiated_session_id(&manifest) {
            stop_daemon_session(daemon_url, &ctx.api_key, session_id).await?;
        }
    }

    Ok(())
}
