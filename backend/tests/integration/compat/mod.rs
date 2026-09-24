//! API compatibility tests.
//!
//! These tests verify that the server and daemon can handle requests from
//! different versions, ensuring backwards compatibility.
//!
//! ## Fixture Generation
//!
//! Fixtures are automatically captured during integration tests when running
//! with `--features generate-fixtures`:
//!
//! - `daemon_to_server.json`: Requests the daemon makes to the server
//! - `server_to_daemon.json`: Requests the server makes to the daemon
//! - `openapi.json`: OpenAPI spec for schema validation
//!
//! ## Replay Testing
//!
//! Replay tests load fixtures and make actual HTTP requests to verify
//! compatibility. IDs in paths and bodies are substituted with test values.
//! Response bodies are validated against the captured OpenAPI schema.

mod replay;
mod schema;
mod types;

pub use replay::*;

use crate::infra::{
    DAEMONPOLL_CONTAINER, SERVERPOLL_CONTAINER, SERVERPOLL_DAEMON_URL, TestClient,
    clear_discovery_data, pause_daemons, setup_authenticated_user, unpause_daemon,
};
use scanopy::server::daemon_api_keys::r#impl::api::DaemonApiKeyResponse;
use scanopy::server::daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase};
use scanopy::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use scanopy::server::shared::storage::traits::Storable;
use uuid::Uuid;

const SERVER_URL: &str = "http://localhost:60072";

/// Create a daemon API key for use in compat tests.
async fn create_compat_test_api_key(network_id: Uuid) -> Result<String, String> {
    let client = TestClient::new();

    // Re-authenticate to get a session
    setup_authenticated_user(&client).await?;

    let api_key = DaemonApiKey::new(DaemonApiKeyBase {
        key: String::new(),
        name: "Compat Test API Key".to_string(),
        last_used: None,
        expires_at: None,
        network_id,
        is_enabled: true,
        tags: Vec::new(),
        daemon_id: None,
        plaintext: None,
    });

    let response: DaemonApiKeyResponse = client.post("/api/v1/auth/daemon", &api_key).await?;
    Ok(response.key)
}

/// Cancel any active discovery sessions on the server.
/// This should be called before daemon compat tests to ensure clean state.
pub async fn cancel_server_discovery_sessions(client: &TestClient) -> Result<(), String> {
    // Get active sessions from server
    // Note: TestClient.get() already unwraps ApiResponse, so we get Vec<T> directly
    let sessions: Vec<DiscoveryUpdatePayload> =
        client.get("/api/v1/discovery/active-sessions").await?;

    if sessions.is_empty() {
        return Ok(());
    }

    println!(
        "  Cancelling {} active discovery session(s) on server",
        sessions.len()
    );

    for session in sessions {
        let cancel_url = format!("/api/v1/discovery/{}/cancel", session.session_id);
        // Ignore errors - session may have completed between listing and cancelling
        let _: Result<(), _> = client.post_empty(&cancel_url).await;
    }

    Ok(())
}

/// Run all compatibility tests against running server and daemon.
///
/// The `serverpoll_daemon_api_key` is the API key that was used to initialize
/// the ServerPoll daemon during the discovery phase. This key is needed to
/// authenticate requests during daemon compat tests.
pub async fn run_compat_tests(
    daemon_id: Uuid,
    network_id: Uuid,
    organization_id: Uuid,
    user_id: Uuid,
    serverpoll_daemon_api_key: &str,
) -> Result<(), String> {
    // Freeze the daemon containers BEFORE clearing/replaying. The DaemonPoll
    // daemon scans the shared test network on its own ~30s cadence; if a real
    // discovery interleaves with the replay (clear → POST /subnets → POST
    // /hosts/discovery) it deletes/replaces the fixture subnet an ip references,
    // causing an intermittent ip→subnet FK violation. Pause first so no new
    // discovery data arrives between the clear and the replay. ServerPoll is
    // unpaused again before daemon-compat (which replays fixtures to it).
    pause_daemons();

    // Clear discovery data from previous test phases to give fixtures a clean slate
    // This prevents FK constraint violations when fixtures reference specific IDs
    clear_discovery_data()?;

    // Create a daemon API key for server compat test replay requests
    let api_key = create_compat_test_api_key(network_id).await?;
    println!("  Created daemon API key for compat tests");

    let ctx = ReplayContext {
        daemon_id,
        network_id,
        user_id,
        organization_id,
        api_key: api_key.clone(),
    };

    println!("\n=== Server Compatibility (old daemon → current server) ===");
    run_server_compat_tests(SERVER_URL, &ctx, clear_discovery_data).await?;

    println!("\n=== Daemon Compatibility (old server → current daemon) ===");
    // Daemon-compat replays fixtures to the ServerPoll daemon, so it must be live
    // again. DaemonPoll stays paused (unused here); cleanup's `down -v` releases it.
    unpause_daemon(SERVERPOLL_CONTAINER);

    // Cancel any discovery sessions from Phase 1 before running daemon compat tests
    // First cancel on server (updates server state and notifies daemon via event)
    let client = TestClient::new();
    setup_authenticated_user(&client).await?;
    cancel_server_discovery_sessions(&client).await?;
    // The server cancels by session id; wait for the daemon to wind that session down
    wait_for_daemon_idle(SERVERPOLL_DAEMON_URL, serverpoll_daemon_api_key).await?;

    // Use the API key from when the ServerPoll daemon was provisioned during discovery
    let daemon_ctx = ReplayContext {
        daemon_id,
        network_id,
        user_id,
        organization_id,
        api_key: serverpoll_daemon_api_key.to_string(),
    };

    run_daemon_compat_tests(SERVERPOLL_DAEMON_URL, &daemon_ctx).await?;

    // Resume the DaemonPoll daemon now that compat is done — later phases (e.g.
    // fixture generation) `docker exec` into it, which fails while it is paused.
    unpause_daemon(DAEMONPOLL_CONTAINER);

    println!("\n✅ All compatibility tests passed!");
    Ok(())
}
