use anyhow::{Context, Error, Result};
use async_fs;
use clap::{Parser, arg, command};
use directories_next::ProjectDirs;
use figment::{
    Figment,
    providers::{Env, Format, Json, Serialized},
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::server::daemons::r#impl::{api::DaemonCapabilities, base::DaemonMode};

#[derive(Parser)]
#[command(name = "scanopy-daemon")]
#[command(about = "Scanopy network discovery and test execution daemon")]
pub struct DaemonCli {
    /// Complete Server URL
    #[arg(long)]
    server_url: Option<String>,

    /// Network ID to join
    #[arg(long)]
    network_id: Option<String>,

    /// Port the daemon listens on. Combined with daemon URL for server to connect.
    #[arg(short, long)]
    daemon_port: Option<u16>,

    /// Name for this daemon
    #[arg(long)]
    name: Option<String>,

    /// Logging verbosity
    #[arg(long)]
    log_level: Option<String>,

    /// Seconds between heartbeat updates to the server
    #[arg(long)]
    heartbeat_interval: Option<u64>,

    /// IP address to bind daemon to
    #[arg(long)]
    bind_address: Option<String>,

    /// Maximum parallel host scans
    #[arg(long)]
    concurrent_scans: Option<usize>,

    /// API key
    #[arg(long)]
    daemon_api_key: Option<String>,

    /// Optional local proxy for Docker API on the daemon host. Supports non-SSL and SSL; SSL requires additional config vars
    #[arg(long)]
    docker_proxy: Option<String>,

    /// Path to SSL certificate if using a docker proxy with SSL
    #[arg(long)]
    docker_proxy_ssl_cert: Option<String>,

    /// Path to SSL private key if using a docker proxy with SSL
    #[arg(long)]
    docker_proxy_ssl_key: Option<String>,

    /// Path to SSL chain if using a docker proxy with SSL
    #[arg(long)]
    docker_proxy_ssl_chain: Option<String>,

    /// DaemonPoll: Daemon connects to server; works behind NAT/firewall without opening ports. ServerPoll: Server connects to daemon, for deployments where daemon cannot make outbound connections - requires providing Daemon URL
    #[arg(long)]
    mode: Option<DaemonMode>,

    /// Allow self-signed certs for daemon -> server connections
    #[arg(long)]
    allow_self_signed_certs: Option<bool>,

    /// Base URL where server can reach daemon (without port). Port is specified separately.
    #[arg(long)]
    daemon_url: Option<String>,

    /// User ID of the person who installed this daemon. Used for deprecation notifications.
    #[arg(long)]
    user_id: Option<Uuid>,

    /// Enable faster ARP scanning on Windows by using broadcast ARP via Npcap instead of native SendARP, which doesn't support broadcast. **Requires Npcap installation**. Ignored on Linux/macOS.
    #[arg(long)]
    use_npcap_arp: Option<bool>,

    /// Number of ARP retry rounds for non-responding hosts (default: 2, meaning 3 total attempts)
    #[arg(long)]
    arp_retries: Option<u32>,

    /// Maximum ARP packets per second (default: 50, go more conservative for networks with enterprise switches)
    #[arg(long)]
    arp_rate_pps: Option<u32>,

    /// Maximum port scan probes per second (default: 500, controls rate of TCP/UDP connection attempts to avoid overwhelming target hosts)
    #[arg(long)]
    scan_rate_pps: Option<u32>,

    /// Number of ports scanned concurrently per host. Higher values scan faster but may overwhelm some hosts
    #[arg(long)]
    port_scan_batch_size: Option<usize>,

    /// Restrict daemon to specific network interface(s). Comma-separated for multiple (e.g., eth0,eth1). Leave empty for all interfaces
    #[arg(long, value_delimiter = ',')]
    interfaces: Option<Vec<String>>,
}

/// Unified configuration struct that handles both startup and runtime config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    // Server connection
    pub server_url: Option<String>,
    pub network_id: Option<Uuid>,

    // Legacy server connection
    pub server_target: Option<String>,
    pub server_port: Option<u16>,

    // Daemon settings
    pub daemon_port: u16,
    pub name: String,
    pub log_level: String,
    pub heartbeat_interval: u64,
    pub bind_address: String,
    pub concurrent_scans: usize,

    // Runtime state
    pub id: Uuid,
    #[serde(default)]
    pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub host_id: Option<Uuid>,
    #[serde(default, alias = "daemon_api_key")]
    pub daemon_api_key: Option<String>,
    /// User responsible for maintaining this daemon (from install command)
    #[serde(default)]
    pub user_id: Option<Uuid>,
    #[serde(default)]
    pub docker_proxy: Option<String>,
    #[serde(default)]
    pub mode: DaemonMode,
    #[serde(default)]
    allow_self_signed_certs: bool,
    daemon_url: Option<String>,
    #[serde(default)]
    docker_proxy_ssl_cert: Option<String>,
    #[serde(default)]
    docker_proxy_ssl_key: Option<String>,
    #[serde(default)]
    docker_proxy_ssl_chain: Option<String>,
    /// Accept invalid TLS certificates when scanning endpoints.
    /// Defaults to true since scanners probe arbitrary internal services.
    #[serde(default = "default_accept_invalid_scan_certs")]
    pub accept_invalid_scan_certs: bool,
    #[serde(default)]
    pub use_npcap_arp: bool,
    #[serde(default = "default_arp_retries")]
    pub arp_retries: u32,
    #[serde(default = "default_arp_rate_pps")]
    pub arp_rate_pps: u32,
    #[serde(default = "default_scan_rate_pps")]
    pub scan_rate_pps: u32,
    #[serde(default = "default_port_scan_batch_size")]
    pub port_scan_batch_size: usize,
    /// Network interfaces to restrict scanning to. Empty means all interfaces.
    #[serde(default)]
    pub interfaces: Vec<String>,
    /// Daemon capabilities (docker socket availability, interfaced subnets)
    /// Updated after SelfReport discovery completes
    #[serde(default)]
    pub capabilities: DaemonCapabilities,
}

fn default_accept_invalid_scan_certs() -> bool {
    true
}

fn default_arp_retries() -> u32 {
    2 // Default: 2 retries = 3 total attempts
}

fn default_arp_rate_pps() -> u32 {
    50 // Default: 50 pps, safe for most enterprise switches
}

fn default_scan_rate_pps() -> u32 {
    500 // Default: 500 pps (2ms between probes), safe for most devices
}

fn default_port_scan_batch_size() -> usize {
    200 // Default: 200 ports concurrently per host
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: None,
            network_id: None,
            daemon_port: 60073,
            bind_address: "0.0.0.0".to_string(),
            name: "scanopy-daemon".to_string(),
            log_level: "info".to_string(),
            heartbeat_interval: 30,
            id: Uuid::new_v4(),
            last_heartbeat: None,
            host_id: None,
            daemon_api_key: None,
            user_id: None,
            concurrent_scans: 15,
            docker_proxy: None,
            mode: DaemonMode::DaemonPoll,
            server_port: None,
            server_target: None,
            allow_self_signed_certs: false,
            daemon_url: None,
            docker_proxy_ssl_cert: None,
            docker_proxy_ssl_chain: None,
            docker_proxy_ssl_key: None,
            accept_invalid_scan_certs: default_accept_invalid_scan_certs(),
            use_npcap_arp: false,
            arp_retries: default_arp_retries(),
            arp_rate_pps: default_arp_rate_pps(),
            interfaces: Vec::new(),
            scan_rate_pps: default_scan_rate_pps(),
            port_scan_batch_size: default_port_scan_batch_size(),
            capabilities: DaemonCapabilities::default(),
        }
    }
}

impl AppConfig {
    /// Get config path, optionally namespaced by daemon name.
    /// If name is None or "scanopy-daemon" (default), uses legacy path for backward compat.
    pub fn get_config_path_for_name(name: Option<&str>) -> Result<(bool, PathBuf)> {
        let proj_dirs = ProjectDirs::from("com", "scanopy", "daemon")
            .ok_or_else(|| anyhow::anyhow!("Unable to determine config directory"))?;

        let config_path = match name {
            // Use namespaced path for custom daemon names
            Some(n) if n != "scanopy-daemon" => proj_dirs.config_dir().join(n).join("config.json"),
            // Legacy path for default name or None
            _ => proj_dirs.config_dir().join("config.json"),
        };

        Ok((config_path.exists(), config_path))
    }

    /// Get config path using default (legacy) location
    pub fn get_config_path() -> Result<(bool, PathBuf)> {
        Self::get_config_path_for_name(None)
    }

    pub fn load(cli_args: DaemonCli) -> anyhow::Result<Self> {
        // Determine config path based on daemon name
        let (config_exists, config_path) =
            AppConfig::get_config_path_for_name(cli_args.name.as_deref())?;

        // Standard configuration layering: Defaults → Config file → Env → CLI (highest priority)
        let mut figment = Figment::from(Serialized::defaults(AppConfig::default()));

        // Add config file if it exists
        if config_exists {
            figment = figment.merge(Json::file(&config_path));
        }

        // Handle SCANOPY_INTERFACES specially - Figment doesn't auto-split comma-separated values
        if let Ok(interfaces_str) = std::env::var("SCANOPY_INTERFACES") {
            let interfaces: Vec<String> = interfaces_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            figment = figment.merge(("interfaces", interfaces));
        }

        // Add environment variables (interfaces handled above to support comma-separated values)
        figment = figment
            .merge(Env::prefixed("NETVISOR_").ignore(&["INTERFACES"]))
            .merge(Env::prefixed("SCANOPY_").ignore(&["INTERFACES"]));

        for (key, _) in std::env::vars() {
            if key.starts_with("NETVISOR_") {
                tracing::warn!(
                    "Env vars prefixed with NETVISOR_ Will be deprecated in v0.13.0: {} - please migrate to SCANOPY_{}",
                    key,
                    key.trim_start_matches("NETVISOR_")
                );
                break; // Only warn once
            }
        }

        // Add CLI overrides (highest priority) - only if explicitly provided
        if let Some(server_url) = cli_args.server_url {
            figment = figment.merge(("server_url", server_url));
        }
        if let Some(network_id) = cli_args.network_id {
            figment = figment.merge(("network_id", network_id));
        }
        if let Some(port) = cli_args.daemon_port {
            figment = figment.merge(("daemon_port", port));
        }
        if let Some(name) = cli_args.name {
            figment = figment.merge(("name", name));
        }
        if let Some(log_level) = cli_args.log_level {
            figment = figment.merge(("log_level", log_level));
        }
        if let Some(heartbeat_interval) = cli_args.heartbeat_interval {
            figment = figment.merge(("heartbeat_interval", heartbeat_interval));
        }
        if let Some(bind_address) = cli_args.bind_address {
            figment = figment.merge(("bind_address", bind_address));
        }
        if let Some(concurrent_scans) = cli_args.concurrent_scans {
            figment = figment.merge(("concurrent_scans", concurrent_scans));
        }
        if let Some(api_key) = cli_args.daemon_api_key {
            figment = figment.merge(("daemon_api_key", api_key));
        }
        if let Some(docker_proxy) = cli_args.docker_proxy {
            figment = figment.merge(("docker_proxy", docker_proxy));
        }
        if let Some(docker_proxy_ssl_key) = cli_args.docker_proxy_ssl_key {
            figment = figment.merge(("docker_proxy_ssl_key", docker_proxy_ssl_key));
        }
        if let Some(docker_proxy_ssl_cert) = cli_args.docker_proxy_ssl_cert {
            figment = figment.merge(("docker_proxy_ssl_cert", docker_proxy_ssl_cert));
        }
        if let Some(docker_proxy_ssl_chain) = cli_args.docker_proxy_ssl_chain {
            figment = figment.merge(("docker_proxy_ssl_chain", docker_proxy_ssl_chain));
        }
        if let Some(mode) = cli_args.mode {
            figment = figment.merge(("mode", mode));
        }
        if let Some(allow_self_signed_certs) = cli_args.allow_self_signed_certs {
            figment = figment.merge(("allow_self_signed_certs", allow_self_signed_certs));
        }
        if let Some(user_id) = cli_args.user_id {
            figment = figment.merge(("user_id", user_id));
        }
        if let Some(use_npcap_arp) = cli_args.use_npcap_arp {
            figment = figment.merge(("use_npcap_arp", use_npcap_arp));
        }
        if let Some(arp_retries) = cli_args.arp_retries {
            figment = figment.merge(("arp_retries", arp_retries));
        }
        if let Some(arp_rate_pps) = cli_args.arp_rate_pps {
            figment = figment.merge(("arp_rate_pps", arp_rate_pps));
        }
        if let Some(scan_rate_pps) = cli_args.scan_rate_pps {
            figment = figment.merge(("scan_rate_pps", scan_rate_pps));
        }
        if let Some(port_scan_batch_size) = cli_args.port_scan_batch_size {
            figment = figment.merge(("port_scan_batch_size", port_scan_batch_size));
        }
        if let Some(interface) = cli_args.interfaces {
            figment = figment.merge(("interfaces", interface));
        }

        let config: AppConfig = figment
            .extract()
            .map_err(|e| Error::msg(format!("Configuration error: {}", e)))?;

        Ok(config)
    }
}

pub struct ConfigStore {
    path: PathBuf,
    config: Arc<RwLock<AppConfig>>,
}

impl ConfigStore {
    pub fn new(path: PathBuf, initial_config: AppConfig) -> Self {
        Self {
            path,
            config: Arc::new(RwLock::new(initial_config)),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            async_fs::create_dir_all(parent)
                .await
                .context("Failed to create config directory")?;
        }

        // Load existing config if it exists and merge with current config
        if self.path.exists() {
            self.load().await?;
        } else {
            tracing::info!("No existing runtime config found, will create new on first save");
        }

        Ok(())
    }

    async fn load(&self) -> Result<()> {
        let content = async_fs::read_to_string(&self.path)
            .await
            .context("Failed to read config file")?;

        let loaded_config: AppConfig =
            serde_json::from_str(&content).context("Failed to parse config file")?;

        // Merge loaded runtime state with current config
        let mut config = self.config.write().await;
        config.id = loaded_config.id;
        config.last_heartbeat = loaded_config.last_heartbeat;

        Ok(())
    }

    async fn save(&self, config: &AppConfig) -> Result<()> {
        let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

        // Atomic write: write to temp file then rename
        let temp_path = self.path.with_extension("tmp");

        async_fs::write(&temp_path, json)
            .await
            .context("Failed to write temp config file")?;

        async_fs::rename(&temp_path, &self.path)
            .await
            .context("Failed to move temp config to final location")?;

        Ok(())
    }

    pub async fn get_id(&self) -> Result<Uuid> {
        let config = self.config.read().await;
        Ok(config.id)
    }

    pub async fn get_name(&self) -> Result<String> {
        let config = self.config.read().await;
        Ok(config.name.clone())
    }

    pub async fn set_id(&self, id: Uuid) -> Result<()> {
        let mut config = self.config.write().await;
        config.id = id;
        self.save(&config.clone()).await
    }

    pub async fn get_allow_self_signed_certs(&self) -> Result<bool> {
        let config = self.config.read().await;
        Ok(config.allow_self_signed_certs)
    }

    pub async fn get_api_key(&self) -> Result<Option<String>> {
        let config = self.config.read().await;
        Ok(config.daemon_api_key.clone())
    }

    pub async fn set_api_key(&self, api_key: String) -> Result<()> {
        let mut config = self.config.write().await;
        config.daemon_api_key = Some(api_key);
        self.save(&config.clone()).await
    }

    pub async fn get_user_id(&self) -> Result<Option<Uuid>> {
        let config = self.config.read().await;
        Ok(config.user_id)
    }

    pub async fn set_user_id(&self, user_id: Uuid) -> Result<()> {
        let mut config = self.config.write().await;
        config.user_id = Some(user_id);
        self.save(&config.clone()).await
    }

    pub async fn get_port(&self) -> Result<u16> {
        let config = self.config.read().await;
        Ok(config.daemon_port)
    }

    pub async fn set_port(&self, port: u16) -> Result<()> {
        let mut config = self.config.write().await;
        config.daemon_port = port;
        self.save(&config.clone()).await
    }

    pub async fn get_bind_address(&self) -> Result<String> {
        let config = self.config.read().await;
        Ok(config.bind_address.clone())
    }

    pub async fn get_mode(&self) -> Result<DaemonMode> {
        let config = self.config.read().await;
        Ok(config.mode)
    }

    pub async fn set_network_id(&self, network_id: Uuid) -> Result<()> {
        let mut config = self.config.write().await;
        config.network_id = Some(network_id);
        self.save(&config.clone()).await
    }

    pub async fn get_network_id(&self) -> Result<Option<Uuid>> {
        let config = self.config.read().await;

        Ok(config.network_id)
    }

    pub async fn get_daemon_url(&self) -> Result<Option<String>> {
        let config = self.config.read().await;

        Ok(config.daemon_url.clone())
    }

    pub async fn get_server_url(&self) -> Result<String> {
        let config = self.config.read().await;

        if let Some(server_port) = config.server_port
            && let Some(server_target) = &config.server_target
        {
            Ok(format!("{}:{}", server_target, server_port))
        } else if let Some(server_url) = config.server_url.clone() {
            Ok(server_url)
        } else {
            Err(anyhow::anyhow!("Server URL is not configured"))
        }
    }

    pub async fn get_concurrent_scans(&self) -> Result<usize> {
        let config = self.config.read().await;
        Ok(config.concurrent_scans)
    }

    pub async fn get_docker_proxy(&self) -> Result<Option<String>> {
        let config = self.config.read().await;
        Ok(config.docker_proxy.clone())
    }

    pub async fn get_docker_proxy_ssl_info(&self) -> Result<Option<(String, String, String)>> {
        let config = self.config.read().await;

        if let (Some(ssl_cert), Some(ssl_key), Some(ssl_chain)) = (
            config.docker_proxy_ssl_cert.clone(),
            config.docker_proxy_ssl_key.clone(),
            config.docker_proxy_ssl_chain.clone(),
        ) {
            Ok(Some((ssl_cert, ssl_key, ssl_chain)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_heartbeat_interval(&self) -> Result<u64> {
        let config = self.config.read().await;
        Ok(config.heartbeat_interval)
    }

    pub async fn update_heartbeat(&self) -> Result<()> {
        let mut config = self.config.write().await;
        config.last_heartbeat = Some(chrono::Utc::now());
        self.save(&config.clone()).await
    }

    pub async fn get_config(&self) -> AppConfig {
        let config = self.config.read().await;
        config.clone()
    }

    pub async fn get_use_npcap_arp(&self) -> Result<bool> {
        let config = self.config.read().await;
        Ok(config.use_npcap_arp)
    }

    pub async fn get_arp_retries(&self) -> Result<u32> {
        let config = self.config.read().await;
        Ok(config.arp_retries)
    }

    pub async fn get_arp_rate_pps(&self) -> Result<u32> {
        let config = self.config.read().await;
        Ok(config.arp_rate_pps)
    }

    pub async fn get_scan_rate_pps(&self) -> Result<u32> {
        let config = self.config.read().await;
        Ok(config.scan_rate_pps)
    }

    pub async fn get_port_scan_batch_size(&self) -> Result<usize> {
        let config = self.config.read().await;
        Ok(config.port_scan_batch_size)
    }

    pub async fn get_accept_invalid_scan_certs(&self) -> Result<bool> {
        let config = self.config.read().await;
        Ok(config.accept_invalid_scan_certs)
    }

    pub async fn get_interfaces(&self) -> Result<Vec<String>> {
        let config = self.config.read().await;
        Ok(config.interfaces.clone())
    }

    pub async fn get_capabilities(&self) -> Result<DaemonCapabilities> {
        let config = self.config.read().await;
        Ok(config.capabilities.clone())
    }

    pub async fn set_capabilities(&self, capabilities: DaemonCapabilities) -> Result<()> {
        let mut config = self.config.write().await;
        config.capabilities = capabilities;
        self.save(&config.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serial_test::serial;

    use crate::daemon::shared::config::DaemonCli;
    use crate::{daemon::shared::config::AppConfig, tests::DAEMON_CONFIG_FIXTURE};
    use clap::{CommandFactory, Parser};
    use std::collections::HashMap;

    #[test]
    #[serial]
    fn test_daemon_config_backward_compatibility() {
        // Try to load config from fixture (from latest release)
        let config_path = Path::new(DAEMON_CONFIG_FIXTURE);

        if config_path.exists() {
            println!("Testing backward compatibility with fixture from latest release");
            let config_json =
                std::fs::read_to_string(config_path).expect("Failed to read daemon config fixture");

            let loaded: Result<AppConfig, _> = serde_json::from_str(&config_json);

            assert!(
                loaded.is_ok(),
                "Failed to load daemon config from latest release: {:?}",
                loaded.err()
            );

            let config = loaded.unwrap();

            // Verify required fields exist
            assert!(!config.name.is_empty(), "Config name is empty");
            assert!(config.daemon_port > 0, "Config port is invalid");
        } else {
            println!(
                "⚠️  No daemon config fixture found at {}",
                DAEMON_CONFIG_FIXTURE
            );
            println!("   Run release workflow to generate fixtures");

            assert!(false, "Failed to load config fixture");
        }
    }

    #[derive(Debug)]
    struct FieldInfo {
        cli_flag: String,
        env_var: Option<String>,
        help_text: String,
    }

    const EXCLUDED_FIELDS: [&str; 6] = [
        "daemon_api_key",
        "network_id",
        "server_url",
        // Automatically set by install command, not user-configurable
        "user_id",
        // Legacy fields not exposed in UI
        "server_target",
        "server_port",
    ];

    #[test]
    fn config_fields_are_in_sync() {
        let rust_fields = extract_rust_fields();
        let frontend_fields = extract_frontend_fields();

        let mut errors = Vec::new();

        // Check all Rust fields exist in frontend
        for (id, rust_info) in &rust_fields {
            // Check frontend
            match frontend_fields.get(id) {
                None => errors.push(format!("Field '{}' missing from frontend", id)),
                Some(fe_info) => {
                    if fe_info.cli_flag != rust_info.cli_flag {
                        errors.push(format!(
                            "Field '{}' CLI flag mismatch: rust='{}', frontend='{}'",
                            id, rust_info.cli_flag, fe_info.cli_flag
                        ));
                    }
                    if fe_info.env_var != rust_info.env_var {
                        errors.push(format!(
                            "Field '{}' env var mismatch: rust={:?}, frontend={:?}",
                            id, rust_info.env_var, fe_info.env_var
                        ));
                    }
                    // Normalize whitespace for description comparison
                    let rust_desc = normalize_text(&rust_info.help_text);
                    let fe_desc = normalize_text(&fe_info.help_text);
                    if rust_desc != fe_desc {
                        errors.push(format!(
                            "Field '{}' help text mismatch:\n  rust: '{}'\n  frontend: '{}'",
                            id, rust_desc, fe_desc
                        ));
                    }
                }
            }
        }

        // Check for fields in frontend/markdown that aren't in Rust
        for id in frontend_fields.keys() {
            if !rust_fields.contains_key(id) {
                errors.push(format!("Field '{}' in frontend but not in Rust", id));
            }
        }

        assert!(
            errors.is_empty(),
            "Config sync errors:\n{}",
            errors.join("\n")
        );
    }

    fn extract_rust_fields() -> HashMap<String, FieldInfo> {
        let cmd = DaemonCli::command();
        cmd.get_arguments()
            .filter(|a| {
                let id = a.get_id().to_string();
                id != "help" && id != "version" && !EXCLUDED_FIELDS.contains(&id.as_str())
            })
            .map(|a| {
                let id = a.get_id().to_string();

                // Derive env var from field ID using same conversion as Figment
                let env_var = format!("SCANOPY_{}", id.to_uppercase());

                let info = FieldInfo {
                    cli_flag: a.get_long().map(|l| format!("--{}", l)).unwrap_or_default(),
                    env_var: Some(env_var),
                    help_text: a.get_help().map(|h| h.to_string()).unwrap_or_default(),
                };
                (id, info)
            })
            .collect()
    }

    fn extract_frontend_fields() -> HashMap<String, FieldInfo> {
        let json = include_str!("../../tests/daemon-config-frontend-fields.json");
        let fields: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();

        fields
            .into_iter()
            .filter_map(|v| {
                let id = v.get("id")?.as_str()?.to_string(); // Already snake_case
                let info = FieldInfo {
                    cli_flag: v.get("cliFlag")?.as_str()?.to_string(),
                    env_var: v.get("envVar").and_then(|e| e.as_str()).map(String::from),
                    help_text: v.get("helpText")?.as_str()?.to_string(),
                };
                Some((id, info))
            })
            .collect()
    }

    fn normalize_text(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Regression test for https://github.com/scanopy/scanopy/issues/463
    /// Verifies that SCANOPY_INTERFACES env var correctly populates the interfaces config field.
    /// The bug was that CLI field name (interfaces) didn't match config struct field name (interface_filter),
    /// so Figment looked for SCANOPY_INTERFACE_FILTER instead of SCANOPY_INTERFACES.
    #[test]
    #[serial]
    fn test_scanopy_interfaces_env_var() {
        // Save and clear any existing value
        let original = std::env::var("SCANOPY_INTERFACES").ok();
        // SAFETY: This test runs serially and restores the original value after
        unsafe { std::env::set_var("SCANOPY_INTERFACES", "eth0,enp6s18") };

        // Load config with empty CLI args (env var should take effect)
        let cli = DaemonCli::parse_from::<[&str; 0], &str>([]);
        let config = AppConfig::load(cli).expect("Failed to load config");

        // Restore original value
        // SAFETY: This test runs serially
        unsafe {
            if let Some(val) = original {
                std::env::set_var("SCANOPY_INTERFACES", val);
            } else {
                std::env::remove_var("SCANOPY_INTERFACES");
            }
        }

        assert_eq!(
            config.interfaces,
            vec!["eth0", "enp6s18"],
            "SCANOPY_INTERFACES env var should populate interfaces field"
        );
    }
}
