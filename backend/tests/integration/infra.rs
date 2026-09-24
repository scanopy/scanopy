//! Test infrastructure: ContainerManager, TestClient, helpers, and database utilities.

use email_address::EmailAddress;
use reqwest::StatusCode;
use scanopy::server::auth::r#impl::api::{
    LoginRequest, NetworkSetup, RegisterRequest, SetupRequest, SetupResponse,
};
use scanopy::server::daemons::r#impl::api::ProvisionDaemonResponse;
use scanopy::server::daemons::r#impl::base::Daemon;
use scanopy::server::networks::r#impl::Network;
use scanopy::server::organizations::r#impl::base::Organization;
use scanopy::server::shared::storage::generic::GenericPostgresStorage;
use scanopy::server::shared::storage::traits::{Storable, Storage};
use scanopy::server::shared::types::api::ApiResponse;
use scanopy::server::users::r#impl::base::User;
use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::fmt::Display;
use std::process::{Child, Command};
use uuid::Uuid;

/// Database URL for test database (exposed on port 5435 by docker-compose.test.yml)
const TEST_DATABASE_URL: &str = "postgres://postgres:password@localhost:5435/scanopy";

pub const BASE_URL: &str = "http://localhost:60072";
pub const TEST_PASSWORD: &str = "TestPassword123!";
/// ServerPoll daemon URL for test client (localhost - accessible from host)
pub const SERVERPOLL_DAEMON_URL: &str = "http://localhost:60074";
/// ServerPoll daemon URL for server (Docker internal network)
const SERVERPOLL_DAEMON_URL_INTERNAL: &str = "http://daemon-serverpoll:60074";

/// DaemonPoll daemon container name (compose service `daemon`).
/// Matches the compose default `<project>-<service>-<index>` naming, same as
/// `scanopy-postgres-dev-1` used by `exec_sql`.
pub const DAEMONPOLL_CONTAINER: &str = "scanopy-daemon-1";
/// ServerPoll daemon container name (compose service `daemon-serverpoll`).
pub const SERVERPOLL_CONTAINER: &str = "scanopy-daemon-serverpoll-1";

// =============================================================================
// Container Management
// =============================================================================

pub struct ContainerManager {
    container_process: Option<Child>,
}

impl ContainerManager {
    pub fn new() -> Self {
        Self {
            container_process: None,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        println!("Starting containers with docker compose...");

        // First, bring down any existing containers and remove volumes
        // This ensures a clean database for each test run
        // Clean up old containers but preserve volumes (cargo-cache, rust-target, etc.)
        // This makes subsequent test runs much faster since dependencies are already compiled
        println!("  Cleaning up old containers...");
        let _ = Command::new("docker")
            .args([
                "compose",
                "-f",
                "docker-compose.test.yml",
                "down",
                "--remove-orphans",
            ])
            .current_dir("..")
            .output();

        // Reset the two places daemon identity is persisted — the database and the daemons'
        // own config files — together. They are a matched pair: a daemon record without the
        // matching local api key (or the reverse) leaves the daemon unable to register, and
        // the daemon's once-only `/api/initialize` guard means a stale config silently
        // refuses the new credentials. The named volume is dropped by name rather than with
        // `down -v` so the cargo/target caches, which make repeat runs bearable, survive.
        println!("  Resetting database and persisted daemon configs...");
        let _ = Command::new("docker")
            .args(["volume", "rm", "scanopy_postgres_data"])
            .output();
        for dir in ["data/daemon_config", "data/daemon_serverpoll_config"] {
            let _ = std::fs::remove_dir_all(format!("../{}", dir));
        }

        let status = Command::new("docker")
            .args([
                "compose",
                "-f",
                "docker-compose.test.yml",
                "up",
                "--build",
                "--force-recreate",
                "--wait",
            ])
            // The containers always build with `generate-fixtures`; only a run that asked for
            // fixtures lets them record, so a plain test run leaves the fixtures untouched.
            .env(
                "CAPTURE_COMPAT_FIXTURES",
                cfg!(feature = "generate-fixtures").to_string(),
            )
            .current_dir("..")
            .status()
            .map_err(|e| format!("Failed to start containers: {}", e))?;

        if !status.success() {
            return Err("Failed to start containers".to_string());
        }

        println!("✅ Server and daemon are healthy!");
        Ok(())
    }

    pub fn cleanup(&mut self) {
        println!("\nCleaning up containers...");

        if let Some(mut process) = self.container_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        let _ = Command::new("make")
            .arg("dev-down")
            .current_dir("..")
            .output();

        let _ = Command::new("docker")
            .args([
                "compose",
                "-f",
                "docker-compose.test.yml",
                "down",
                "-v",
                "--rmi",
                "local",
                "--remove-orphans",
            ])
            .current_dir("..")
            .output();

        println!("✅ All containers cleaned up successfully");
    }
}

impl Drop for ContainerManager {
    fn drop(&mut self) {
        if std::thread::panicking() && std::env::var("CI").is_ok() {
            println!("\n⚠️  Test failed in CI - leaving containers running for log collection");
            return;
        }
        self.cleanup();
    }
}

// =============================================================================
// Test Client
// =============================================================================

pub struct TestClient {
    pub client: reqwest::Client,
}

impl TestClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .cookie_store(true)
                .build()
                .unwrap(),
        }
    }

    pub async fn register(&self, email: &EmailAddress, password: &str) -> Result<User, String> {
        let register_request = RegisterRequest {
            email: email.clone(),
            password: password.to_string(),
            terms_accepted: false,
            marketing_opt_in: false,
            website: None,
        };

        let response = self
            .client
            .post(format!("{}/api/auth/register", BASE_URL))
            .json(&register_request)
            .send()
            .await
            .map_err(|e| format!("Registration request failed: {}", e))?;

        self.parse_response(response, "register user").await
    }

    pub async fn login(&self, email: &EmailAddress, password: &str) -> Result<User, String> {
        let login_request = LoginRequest {
            email: email.clone(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/api/auth/login", BASE_URL))
            .json(&login_request)
            .send()
            .await
            .map_err(|e| format!("Login request failed: {}", e))?;

        self.parse_response(response, "login").await
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let response = self
            .client
            .get(format!("{}{}", BASE_URL, path))
            .send()
            .await
            .map_err(|e| format!("GET {} failed: {}", path, e))?;

        self.parse_response(response, &format!("GET {}", path))
            .await
    }

    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let response = self
            .client
            .post(format!("{}{}", BASE_URL, path))
            .json(body)
            .send()
            .await
            .map_err(|e| format!("POST {} failed: {}", path, e))?;

        self.parse_response(response, &format!("POST {}", path))
            .await
    }

    pub async fn put<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, String> {
        let response = self
            .client
            .put(format!("{}{}", BASE_URL, path))
            .json(body)
            .send()
            .await
            .map_err(|e| format!("PUT {} failed: {}", path, e))?;

        self.parse_response(response, &format!("PUT {}", path))
            .await
    }

    /// POST request with no body
    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let response = self
            .client
            .post(format!("{}{}", BASE_URL, path))
            .send()
            .await
            .map_err(|e| format!("POST {} failed: {}", path, e))?;

        self.parse_response(response, &format!("POST {}", path))
            .await
    }

    /// DELETE request that doesn't expect response data (just success: true)
    pub async fn delete_no_content(&self, path: &str) -> Result<(), String> {
        let response = self
            .client
            .delete(format!("{}{}", BASE_URL, path))
            .send()
            .await
            .map_err(|e| format!("DELETE {} failed: {}", path, e))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "DELETE {} failed with status {}: {}",
                path, status, body
            ));
        }

        Ok(())
    }

    pub async fn post_expect_status<B: Serialize>(
        &self,
        path: &str,
        body: &B,
        expected_status: StatusCode,
    ) -> Result<String, String> {
        let response = self
            .client
            .post(format!("{}{}", BASE_URL, path))
            .json(body)
            .send()
            .await
            .map_err(|e| format!("POST {} failed: {}", path, e))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status == expected_status {
            Ok(body)
        } else {
            Err(format!(
                "Expected status {}, got {}: {}",
                expected_status, status, body
            ))
        }
    }

    pub async fn get_expect_status(
        &self,
        path: &str,
        expected_status: StatusCode,
    ) -> Result<String, String> {
        let response = self
            .client
            .get(format!("{}{}", BASE_URL, path))
            .send()
            .await
            .map_err(|e| format!("GET {} failed: {}", path, e))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status == expected_status {
            Ok(body)
        } else {
            Err(format!(
                "Expected status {}, got {}: {}",
                expected_status, status, body
            ))
        }
    }

    pub async fn setup(&self, request: &SetupRequest) -> Result<SetupResponse, String> {
        let response = self
            .client
            .post(format!("{}/api/auth/setup", BASE_URL))
            .json(request)
            .send()
            .await
            .map_err(|e| format!("POST /auth/setup failed: {}", e))?;

        self.parse_response(response, "POST /auth/setup").await
    }

    async fn parse_response<T: DeserializeOwned>(
        &self,
        response: reqwest::Response,
        operation: &str,
    ) -> Result<T, String> {
        let status = response.status();

        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read body".to_string());
            return Err(format!(
                "{} failed with status {}: {}",
                operation, status, body
            ));
        }

        let api_response = response
            .json::<ApiResponse<T>>()
            .await
            .map_err(|e| format!("Failed to parse {} response: {}", operation, e))?;

        if !api_response.is_success() {
            let error = api_response.error().unwrap_or("Unknown error");
            return Err(format!("{} returned error: {}", operation, error));
        }

        api_response
            .into_data()
            .ok_or_else(|| format!("No data in {} response", operation))
    }
}

// =============================================================================
// Test Context
// =============================================================================

pub struct TestContext {
    pub client: TestClient,
    pub network_id: Uuid,
    pub organization_id: Uuid,
    pub db_pool: PgPool,
}

impl TestContext {
    /// Insert an entity directly into the database, bypassing API authorization.
    /// Useful for creating test fixtures that the current user shouldn't have access to.
    pub async fn insert_entity<T: Storable + Display>(&self, entity: &T) -> Result<T, String> {
        let storage: GenericPostgresStorage<T> = GenericPostgresStorage::new(self.db_pool.clone());
        storage
            .create(entity)
            .await
            .map_err(|e| format!("Failed to insert entity: {}", e))
    }

    /// Delete an entity directly from the database by ID.
    pub async fn delete_entity<T: Storable + Display>(&self, id: &Uuid) -> Result<(), String> {
        let storage: GenericPostgresStorage<T> = GenericPostgresStorage::new(self.db_pool.clone());
        storage
            .delete(id)
            .await
            .map_err(|e| format!("Failed to delete entity: {}", e))
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a database connection pool for direct database access in tests
pub async fn create_test_db_pool() -> Result<PgPool, String> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(TEST_DATABASE_URL)
        .await
        .map_err(|e| format!("Failed to connect to test database: {}", e))
}

pub async fn retry<T, F, Fut>(
    description: &str,
    max_retries: u32,
    delay_secs: u64,
    operation: F,
) -> Result<T, String>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let mut last_error = String::new();

    for attempt in 1..=max_retries {
        match operation().await {
            Ok(result) => {
                println!("✅ {}", description);
                return Ok(result);
            }
            Err(e) => {
                if attempt < max_retries {
                    println!(
                        "⏳ Attempt {}/{}: {} - {}",
                        attempt, max_retries, description, e
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
                }
                last_error = e;
            }
        }
    }

    Err(format!("{}: {}", description, last_error))
}

pub async fn setup_authenticated_user(client: &TestClient) -> Result<User, String> {
    println!("\n=== Authenticating Test User ===");

    let test_email: EmailAddress = EmailAddress::new_unchecked("user@gmail.com");

    let setup_request = SetupRequest {
        organization_name: "My Organization".to_string(),
        network: NetworkSetup {
            name: "My Network".to_string(),
        },
    };

    match client.setup(&setup_request).await {
        Ok(response) => {
            println!("✅ Setup completed, network_id: {:?}", response.network_id);
        }
        Err(e) => {
            println!("⚠️  Setup failed (may already be registered): {}", e);
        }
    }

    match client.register(&test_email, TEST_PASSWORD).await {
        Ok(user) => {
            println!("✅ Registered new user: {}", user.base.email);
            Ok(user)
        }
        Err(e) if e.contains("already taken") || e.contains("already in use") => {
            println!("User already exists, logging in...");
            client.login(&test_email, TEST_PASSWORD).await
        }
        Err(e) => Err(e),
    }
}

pub async fn wait_for_organization(client: &TestClient) -> Result<Organization, String> {
    retry("wait for organization to be created", 15, 2, || async {
        let organization: Option<Organization> = client.get("/api/v1/organizations").await?;
        organization.ok_or_else(|| "No organization found yet".to_string())
    })
    .await
}

pub async fn wait_for_network(client: &TestClient) -> Result<Network, String> {
    retry("wait for network to be created", 15, 2, || async {
        let networks: Vec<Network> = client.get("/api/v1/networks").await?;
        networks
            .first()
            .cloned()
            .ok_or_else(|| "No networks found yet".to_string())
    })
    .await
}

pub async fn wait_for_daemon(client: &TestClient) -> Result<Daemon, String> {
    retry("wait for daemon registration", 15, 2, || async {
        let daemons: Vec<Daemon> = client.get("/api/v1/daemons").await?;

        if daemons.is_empty() {
            return Err("No daemons registered yet".to_string());
        }

        // Find the DaemonPoll daemon. Its record is created up front by the integrated-daemon
        // provisioning that runs during setup, so the record's mere existence proves nothing —
        // `last_seen` is what proves the daemon actually reached the server and registered.
        let daemon = daemons
            .into_iter()
            .find(|d| d.base.name == "scanopy-daemon")
            .ok_or_else(|| "DaemonPoll daemon not found".to_string())?;

        if daemon.base.last_seen.is_none() {
            return Err(
                "DaemonPoll daemon provisioned but has not contacted the server yet".to_string(),
            );
        }

        Ok(daemon)
    })
    .await
}

/// Wait until a provisioned ServerPoll daemon has been polled by the server and
/// has reported its version. Provisioning intentionally leaves the version `None`
/// (an unconnected daemon's version is genuinely unknown), so it is only populated
/// after the server's first status poll — and unified discovery cannot be triggered
/// until then. Mirrors `wait_for_daemon`, but keys on the specific daemon and its
/// version rather than the DaemonPoll daemon's `last_seen`.
pub async fn wait_for_serverpoll_daemon_version(
    client: &TestClient,
    daemon_id: Uuid,
) -> Result<Daemon, String> {
    retry(
        "wait for ServerPoll daemon to report its version",
        30,
        2,
        || async {
            let daemons: Vec<Daemon> = client.get("/api/v1/daemons").await?;
            let daemon = daemons
                .into_iter()
                .find(|d| d.id == daemon_id)
                .ok_or_else(|| "ServerPoll daemon record not found".to_string())?;

            if daemon.base.version.is_none() {
                return Err(
                    "ServerPoll daemon provisioned but has not reported its version yet"
                        .to_string(),
                );
            }

            Ok(daemon)
        },
    )
    .await
}

/// The Home Assistant fixture container, published on the host by docker-compose.test.yml.
const HOME_ASSISTANT_URL: &str = "http://localhost:8123";
/// Its container name, so a failure here can say where to look.
const HOME_ASSISTANT_CONTAINER: &str = "homeassistant-discovery";

/// Wait until the Home Assistant fixture serves the page its service definition matches on.
///
/// `docker compose up --wait` returns once a container is *running*, and the image's own
/// healthcheck says nothing about the HTTP listener, so a scan can run while Home Assistant is
/// still doing its first-boot permissions pass. It answers ARP and ICMP throughout, so the host
/// is discovered and only its port stays silent — which is exactly how the v0.17.16 release run
/// failed, with `endpoints_scanned=105 responses=0` against that address and an assertion that
/// read like a discovery bug.
///
/// Polls for the condition `Pattern::Endpoint(tcp 8123, "/", "home assistant")` requires, so a
/// pass here means the scan has something to match rather than merely something to connect to.
pub async fn wait_for_home_assistant() -> Result<(), String> {
    println!("\n=== Waiting for Home Assistant ===");

    retry(
        "wait for Home Assistant to serve its page",
        60,
        5,
        || async {
            let response = reqwest::get(HOME_ASSISTANT_URL)
                .await
                .map_err(|e| format!("no answer from {}: {}", HOME_ASSISTANT_URL, e))?;

            let status = response.status();
            if !(status.is_success() || status.is_redirection()) {
                return Err(format!("answered {}", status));
            }

            let body = response.text().await.map_err(|e| {
                format!("answered {} but the body could not be read: {}", status, e)
            })?;

            if !body.to_lowercase().contains("home assistant") {
                return Err(format!(
                    "answered {} with {} bytes that do not mention Home Assistant",
                    status,
                    body.len()
                ));
            }

            Ok(())
        },
    )
    .await
    .map_err(|e| format!("{}. Check `docker logs {}`", e, HOME_ASSISTANT_CONTAINER))
}

/// Provision and initialize the ServerPoll daemon.
///
/// This follows the proper ServerPoll provisioning flow:
/// 1. Call POST /api/v1/daemons/provision on the server to create daemon record + get API key
/// 2. Call POST /api/initialize on the daemon to configure it with the API key
///
/// Returns the provisioned daemon response (includes daemon record and API key).
pub async fn provision_serverpoll_daemon(
    client: &TestClient,
    network_id: Uuid,
) -> Result<ProvisionDaemonResponse, String> {
    println!("\n=== Provisioning ServerPoll Daemon ===");

    // Step 1: Provision daemon on server (creates record + generates API key)
    // Use internal URL so server can reach daemon via Docker network
    let provision_response: ProvisionDaemonResponse = client
        .post(
            "/api/v1/daemons/provision",
            &serde_json::json!({
                "name": "scanopy-daemon-serverpoll",
                "network_id": network_id,
                // Must be explicit: provisioning covers both modes and defaults to DaemonPoll,
                // so omitting this provisions a DaemonPoll record that the server's ServerPoll
                // poller never picks up — the daemon then sits idle and discovery stalls.
                "mode": "server_poll",
                "url": SERVERPOLL_DAEMON_URL_INTERNAL
            }),
        )
        .await?;

    println!(
        "✅ Daemon provisioned on server (id: {})",
        provision_response.daemon.id
    );

    // Step 2: Initialize daemon with network_id and API key
    println!("  Initializing ServerPoll daemon with credentials...");

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let response = http_client
        .post(format!("{}/api/initialize", SERVERPOLL_DAEMON_URL))
        .json(&serde_json::json!({
            "network_id": network_id,
            "api_key": provision_response.daemon_api_key
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to initialize daemon: {}", e))?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Daemon initialization failed: {}", body));
    }

    println!("✅ ServerPoll daemon initialized");
    Ok(provision_response)
}

// =============================================================================
// Database Helpers
// =============================================================================

pub fn exec_sql(sql: &str) -> Result<String, String> {
    let output = Command::new("docker")
        .args([
            "exec",
            "scanopy-postgres-dev-1",
            "psql",
            "-U",
            "postgres",
            "-d",
            "scanopy",
            "-c",
            sql,
        ])
        .output()
        .map_err(|e| format!("Failed to execute SQL: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "SQL failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn set_plan_status(status: Option<&str>) -> Result<(), String> {
    let sql = match status {
        Some(s) => format!("UPDATE organizations SET plan_status = '{}';", s),
        None => "UPDATE organizations SET plan_status = NULL;".to_string(),
    };
    exec_sql(&sql)?;
    Ok(())
}

pub fn set_billable_plan() -> Result<(), String> {
    let plan_json = r#"{"type": "Starter", "rate": "Month", "base_cents": 0, "trial_days": 0}"#;
    let sql = format!("UPDATE organizations SET plan = '{}';", plan_json);
    exec_sql(&sql)?;
    Ok(())
}

pub fn reset_plan_to_default() -> Result<(), String> {
    let plan_json = r#"{"type": "Community", "rate": "Month", "base_cents": 0, "trial_days": 0}"#;
    let sql = format!("UPDATE organizations SET plan = '{}';", plan_json);
    exec_sql(&sql)?;
    Ok(())
}

/// Clear all discovery data to provide a clean slate between test runs.
///
/// This is useful for:
/// - Running discovery with multiple daemon modes without data pollution
/// - Preparing for compat tests that need to replay fixtures with specific IDs
///
/// Deletion order respects FK constraints:
/// bindings → services → ports → interfaces → hosts → subnets
pub fn clear_discovery_data() -> Result<(), String> {
    println!("  Clearing discovery data...");

    // Delete in FK order
    exec_sql("DELETE FROM bindings")?;
    exec_sql("DELETE FROM services")?;
    exec_sql("DELETE FROM ports")?;
    exec_sql("DELETE FROM interfaces")?;
    exec_sql("DELETE FROM hosts")?;
    exec_sql("DELETE FROM subnets")?;

    println!("  ✅ Discovery data cleared");
    Ok(())
}

/// Freeze the daemon containers so they cannot run real discoveries that would
/// race the compat-test replay (the DaemonPoll daemon scans the shared test
/// network on its own ~30s cadence). Uses `docker pause` (SIGSTOP) rather than
/// stop, so no state is lost and the daemons resume exactly where they were.
///
/// Tolerant of already-paused state — a benign pause failure must not fail the
/// suite.
pub fn pause_daemons() {
    println!("  Pausing daemon containers for compat replay...");
    let _ = Command::new("docker")
        .args(["pause", DAEMONPOLL_CONTAINER, SERVERPOLL_CONTAINER])
        .output();
}

/// Resume a previously paused daemon container (see [`pause_daemons`]).
/// Tolerant of already-running state.
pub fn unpause_daemon(container: &str) {
    println!("  Unpausing daemon container {}...", container);
    let _ = Command::new("docker").args(["unpause", container]).output();
}
