use anyhow::{Context, Error, Result};
use async_fs;
use clap::{Args, Parser, Subcommand, arg, command};
use directories_next::ProjectDirs;
use figment::{
    Figment,
    providers::{Env, Format, Json, Serialized},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use std::net::IpAddr;

use crate::server::credentials::r#impl::mapping::IntegrationTarget;
use crate::server::daemons::r#impl::{api::LegacyCapabilities, base::DaemonMode};

/// Parse the `SCANOPY_CREDENTIAL_IDS` / `--credential-id` compact token grammar into per-daemon
/// [`IntegrationTarget`]s. Every token references a stored credential by id; the suffix is the
/// target IP(s) — the daemon host is just its loopback address, like any other IP target:
/// - `<uuid>` → `Network` (broadcast default)
/// - `<uuid>@127.0.0.1` (a sole loopback) → `DaemonHost` (the daemon's own host, e.g. a local
///   Docker/Podman socket credential)
/// - `<uuid>@<ip>[+<ip>...]` → `Hosts` (specific host IP overrides)
///
/// Sockets are ordinary credentials now — create one (UI/API) and reference it with its
/// loopback address (`<uuid>@127.0.0.1`).
pub fn parse_integration_target_tokens(
    tokens: &[String],
) -> anyhow::Result<Vec<IntegrationTarget>> {
    tokens
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(parse_integration_target_token)
        .collect()
}

fn parse_integration_target_token(token: &str) -> anyhow::Result<IntegrationTarget> {
    match token.split_once('@') {
        // `<uuid>@<ip>[+<ip>...]` → IP targeting. A sole-loopback target is the daemon's own
        // host (DaemonHost scope); anything else is specific Hosts overrides.
        Some((uuid_part, ip_list)) => {
            let ips = ip_list
                .split('+')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.parse::<IpAddr>().map_err(|_| {
                        anyhow::anyhow!("Invalid IP '{}' in credential token '{}'", s, token)
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            if ips.is_empty() {
                anyhow::bail!(
                    "Credential token '{}' has '@' but no IPs (use '<uuid>@<ip>[+<ip>]')",
                    token
                );
            }
            let credential_id = parse_credential_id(uuid_part, token)?;
            if ips.len() == 1 && ips[0].is_loopback() {
                Ok(IntegrationTarget::DaemonHost { credential_id })
            } else {
                Ok(IntegrationTarget::Hosts { credential_id, ips })
            }
        }
        // `<uuid>` → network-level default.
        None => Ok(IntegrationTarget::Network {
            credential_id: parse_credential_id(token, token)?,
        }),
    }
}

fn parse_credential_id(value: &str, token: &str) -> anyhow::Result<Uuid> {
    Uuid::parse_str(value.trim()).map_err(|_| {
        anyhow::anyhow!(
            "Invalid credential token '{}' (expected '<uuid>' or '<uuid>@<ip>[+<ip>]')",
            token
        )
    })
}

/// Top-level daemon CLI.
///
/// With no subcommand, `scanopy-daemon <flags>` runs the daemon using the flattened
/// [`DaemonArgs`] exactly as before. `scanopy-daemon install [flags]` /
/// `scanopy-daemon uninstall [flags]` dispatch to the installer.
/// `args_conflicts_with_subcommands` keeps the bare-run and subcommand paths from mixing, so
/// no existing invocation regresses.
#[derive(Parser)]
#[command(name = "scanopy-daemon")]
#[command(about = "Scanopy network discovery and test execution daemon")]
#[command(version)]
#[command(args_conflicts_with_subcommands = true)]
pub struct DaemonCli {
    #[command(subcommand)]
    pub command: Option<DaemonCommand>,

    #[command(flatten)]
    pub run_args: DaemonArgs,

    /// Internal marker: `daemon install` bakes this into the Windows service binPath so the
    /// daemon knows the SCM launched it and should run under the service control dispatcher.
    /// Not a config value; hidden from `--help`. Ignored on non-Windows platforms.
    #[arg(long, hide = true)]
    pub service: bool,
}

/// Install/uninstall/list subcommands. Install and uninstall reuse the daemon's own connection
/// flags so there is a single source of truth for configuration (see [`DaemonArgs`]).
// `Install` flattens the full `DaemonArgs` and so dwarfs `Uninstall`; boxing the variant (the usual
// large_enum_variant remedy) is incompatible with clap's Subcommand derive, and this enum is only
// ever constructed once at CLI parse time, so the size gap is harmless.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub enum DaemonCommand {
    /// Install the daemon as a background service: place the binary, write `config.json`, and
    /// register a system service (systemd / launchd / Windows SCM). Accepts the same
    /// connection/identity flags as the daemon itself.
    Install(InstallArgs),
    /// Stop and remove the daemon's system service and delete its `config.json`.
    Uninstall(UninstallArgs),
    /// List the Scanopy daemons installed on this host, with the selector to use for each.
    List,
}

/// Flags for `scanopy-daemon install`: the shared daemon flags plus install-only options.
#[derive(Args)]
pub struct InstallArgs {
    #[command(flatten)]
    pub args: DaemonArgs,

    /// Only place the binary and write config; do not register or start the system service.
    #[arg(long)]
    pub no_service: bool,

    /// Directory to install the daemon binary into (defaults to the platform location,
    /// e.g. `/usr/local/bin` on Unix).
    #[arg(long)]
    pub bin_dir: Option<PathBuf>,
}

/// Flags for `scanopy-daemon uninstall`.
#[derive(Args)]
pub struct UninstallArgs {
    /// Which installed daemon to remove: its name, its slot, or its service id (see
    /// `scanopy-daemon list`). Optional when the host has only one daemon installed.
    #[arg(long)]
    pub name: Option<String>,

    /// Remove every daemon installed on this host.
    #[arg(long)]
    pub all: bool,

    /// Also delete the installed binary from disk (by default the binary is left in place).
    #[arg(long)]
    pub purge: bool,
}

/// Connection and identity flags shared by the bare-daemon run and the `install` subcommand.
/// This is the single input surface for daemon configuration — the installer feeds these same
/// flags to [`AppConfig::load`], so there is no second config path.
///
/// It doubles as the **API wire type** for install configuration: the server accepts one of these
/// on `POST /daemons/provision` and emits it into the install command and the MSI filename hash
/// (see [`DaemonArgs::install_config_pairs`]). Rather than mirroring these fields into a separate
/// request struct — which would be a third copy of a list already duplicated into
/// `ui/src/lib/features/daemons/config.ts` — the struct is reused directly.
///
/// Every field that the *server* controls (identity, mode, connection target) or that is a secret
/// is `#[serde(skip)]`, so it can never be set from the wire while remaining a normal CLI flag.
/// The deserializable set is therefore exactly the advanced daemon settings the UI exposes; the
/// server fills the rest in from the daemon record before emitting. `serde` and `clap` attributes
/// are independent, so this does not affect CLI parsing.
#[derive(Args, Serialize, Deserialize, Default, Clone, utoipa::ToSchema)]
pub struct DaemonArgs {
    /// Complete Server URL
    // Server-controlled: comes from the server's own public URL at provision time.
    #[serde(skip)]
    #[arg(long)]
    pub server_url: Option<String>,

    /// Network ID to join
    // Server-controlled: the tenancy boundary, taken from the provisioned record.
    #[serde(skip)]
    #[arg(long)]
    pub network_id: Option<String>,

    /// Port the daemon listens on.
    // Server-controlled: derived from the daemon record's `url` for ServerPoll daemons.
    #[serde(skip)]
    #[arg(short, long)]
    pub daemon_port: Option<u16>,

    /// Name for this daemon
    // Server-controlled: the daemon learns its name via the handshake.
    #[serde(skip)]
    #[arg(long)]
    pub name: Option<String>,

    /// Logging verbosity
    #[arg(long)]
    pub log_level: Option<String>,

    /// Seconds between heartbeat updates to the server
    #[arg(long)]
    pub heartbeat_interval: Option<u64>,

    /// IP address to bind daemon to
    #[arg(long)]
    pub bind_address: Option<String>,

    /// API key
    // Secret: minted server-side, never accepted from a client.
    #[serde(skip)]
    #[arg(long)]
    pub daemon_api_key: Option<String>,

    /// Optional local proxy for Docker API on the daemon host. Supports non-SSL and SSL; SSL requires additional config vars
    // Configured via the credentials/discovery UI, not daemon install config.
    #[serde(skip)]
    #[arg(long)]
    pub docker_proxy: Option<String>,

    /// Path to SSL certificate if using a docker proxy with SSL
    #[serde(skip)]
    #[arg(long)]
    pub docker_proxy_ssl_cert: Option<String>,

    /// Path to SSL private key if using a docker proxy with SSL
    // Secret.
    #[serde(skip)]
    #[arg(long)]
    pub docker_proxy_ssl_key: Option<String>,

    /// Path to SSL chain if using a docker proxy with SSL
    #[serde(skip)]
    #[arg(long)]
    pub docker_proxy_ssl_chain: Option<String>,

    /// DaemonPoll: Daemon connects to server; works behind NAT/firewall without opening ports. ServerPoll: Server connects to daemon, for deployments where daemon cannot make outbound connections - requires providing Daemon URL
    // Server-controlled: immutable post-provision, taken from the daemon record.
    #[serde(skip)]
    #[arg(long)]
    pub mode: Option<DaemonMode>,

    /// Allow self-signed certs for daemon -> server connections
    #[arg(long)]
    pub allow_self_signed_certs: Option<bool>,

    /// Base URL where server can reach daemon
    // Server-controlled: captured on the daemon record at provision time.
    #[serde(skip)]
    #[arg(long)]
    pub daemon_url: Option<String>,

    /// User ID of the person who installed this daemon. Used for deprecation notifications.
    // Server-controlled: the provisioning member.
    #[serde(skip)]
    #[arg(long)]
    pub user_id: Option<Uuid>,

    /// Accept invalid TLS certificates when scanning endpoints. Enabled by default since scanners probe arbitrary internal services.
    #[arg(long)]
    pub accept_invalid_scan_certs: Option<bool>,

    /// Integration target tokens (repeatable). Each is `<uuid>` (credential, no specific IP),
    /// `<uuid>@<ip>[+<ip>...]` (credential pinned to IP(s)), or `docker-socket` / `podman-socket`
    /// (credential-less local socket on the daemon host).
    // Server-controlled: seeded from `seed_credential_refs` on the provision request.
    #[serde(skip)]
    #[arg(long = "credential-id")]
    pub credential_ids: Option<Vec<String>>,

    /// Path to log file. Defaults to platform-specific path. Set to "none" to disable file logging.
    #[arg(long)]
    pub log_file: Option<String>,

    /// Enable faster ARP scanning on Windows by using broadcast ARP via Npcap instead of native SendARP, which doesn't support broadcast. **Requires Npcap installation**. Ignored on Linux/macOS.
    // Deprecated: scan settings are now per-discovery via ScanSettings, and not UI-exposed.
    #[serde(skip)]
    #[arg(long)]
    pub use_npcap_arp: Option<bool>,

    /// Number of ARP retry rounds for non-responding hosts (default: 2, meaning 3 total attempts)
    #[serde(skip)]
    #[arg(long)]
    pub arp_retries: Option<u32>,

    /// Maximum ARP packets per second (default: 50, go more conservative for networks with enterprise switches)
    #[serde(skip)]
    #[arg(long)]
    pub arp_rate_pps: Option<u32>,

    /// Maximum port scan probes per second (default: 500, controls rate of TCP/UDP connection attempts to avoid overwhelming target hosts)
    #[serde(skip)]
    #[arg(long)]
    pub scan_rate_pps: Option<u32>,

    /// Number of ports scanned concurrently per host. Higher values scan faster but may overwhelm some hosts
    #[serde(skip)]
    #[arg(long)]
    pub port_scan_batch_size: Option<usize>,

    /// Restrict daemon to specific network interface(s). Comma-separated for multiple (e.g., eth0,eth1). Leave empty for all interfaces
    #[arg(long, value_delimiter = ',')]
    pub interfaces: Option<Vec<String>>,

    /// Directory that holds this daemon's `config.json`. Overrides the default per-user location.
    /// The `install` command bakes this into the generated system service (pointing at a system
    /// directory) so the service reads config from the exact path the installer wrote — a system
    /// service runs under a different profile than the installer, so relying on the per-user
    /// `$HOME`/`%APPDATA%` path would leave the service unable to find its config.
    // Locator baked into system services by `install`, not a user-facing config field.
    #[serde(skip)]
    #[arg(long)]
    pub config_dir: Option<PathBuf>,

    /// Which daemon already installed on this host the `install` command targets: its name, its
    /// slot, its service id, or its daemon id (see `scanopy-daemon list`). Only needed on a host
    /// running several daemons; otherwise the installer resolves the target on its own.
    // Install-time selector, not a config value — `AppConfig::load` never merges it. Server-set on
    // the reconfigure command so it targets exactly the daemon it was generated for.
    #[serde(skip)]
    #[arg(long)]
    pub instance: Option<String>,
}

/// One emittable install-config value, paired with the key it takes in each install artifact.
/// A `None` key means the field is deliberately absent from that artifact.
pub struct InstallConfigPair {
    /// Long flag for the CLI `install` command, e.g. `--log-level`.
    pub cli_flag: Option<&'static str>,
    /// Query key for the MSI pre-fill filename. Must match the property map in
    /// `backend/wix/parse-filename.js`.
    pub msi_key: Option<&'static str>,
    /// Environment variable for the docker-compose install, e.g. `SCANOPY_LOG_LEVEL`. These are
    /// the names Figment picks up via `Env::prefixed("SCANOPY_")` below.
    pub env_var: Option<&'static str>,
    /// Rendered value.
    pub value: String,
}

impl DaemonArgs {
    /// The set values of this config, rendered once for every install artifact.
    ///
    /// Having a single table is what keeps the CLI command, the MSI filename hash, the
    /// docker-compose env block, and `backend/wix/parse-filename.js` from drifting — the
    /// artifacts key the same field differently (`--log-level` vs `loglevel` vs
    /// `SCANOPY_LOG_LEVEL`), and previously only the JS and the frontend knew their own names.
    ///
    /// `None` fields are skipped entirely, which keeps commands terse and keeps the base64 MSI
    /// filename clear of the ~255-character filename limit.
    pub fn install_config_pairs(&self) -> Vec<InstallConfigPair> {
        fn push(
            pairs: &mut Vec<InstallConfigPair>,
            cli_flag: Option<&'static str>,
            msi_key: Option<&'static str>,
            env_var: Option<&'static str>,
            value: Option<String>,
        ) {
            if let Some(value) = value {
                pairs.push(InstallConfigPair {
                    cli_flag,
                    msi_key,
                    env_var,
                    value,
                });
            }
        }

        // Exhaustive destructure (no `..`) so that adding a field to `DaemonArgs` fails to
        // compile until it is either emitted here or explicitly marked as not emitted.
        let Self {
            server_url,
            network_id: _, // Identity travels via the 1:1 api key binding, not a flag
            daemon_port,
            name,
            log_level,
            heartbeat_interval,
            bind_address,
            daemon_api_key,
            docker_proxy: _, // Configured via the credentials/discovery UI
            docker_proxy_ssl_cert: _,
            docker_proxy_ssl_key: _,
            docker_proxy_ssl_chain: _,
            mode,
            allow_self_signed_certs,
            daemon_url: _, // Captured on the daemon record at provision time
            user_id: _,    // Server-set from the provisioning member
            accept_invalid_scan_certs,
            credential_ids: _, // Seeded server-side from `seed_credential_refs`
            log_file,
            use_npcap_arp: _, // Deprecated: scan settings are per-discovery via ScanSettings
            arp_retries: _,
            arp_rate_pps: _,
            scan_rate_pps: _,
            port_scan_batch_size: _,
            interfaces,
            config_dir: _, // Install-time locator, baked into the service definition
            instance,
        } = self;

        let mut pairs = Vec::new();

        // The daemon infers ServerPoll from the *absence* of a server url, so neither the CLI
        // command nor the compose env needs a mode; the MSI has no such inference and pre-fills
        // MODE explicitly.
        push(
            &mut pairs,
            None,
            Some("mode"),
            None,
            render_mode(mode.as_ref()),
        );
        // The daemon learns its name via the handshake, so the CLI command and compose env both
        // omit it; the MSI needs it up front to label the install it creates.
        push(&mut pairs, None, Some("name"), None, name.clone());
        // Which already-installed daemon to act on, for the rare host running several. Nothing to
        // select on a fresh MSI install, and a docker daemon is one container per compose file.
        push(&mut pairs, Some("--instance"), None, None, instance.clone());
        push(
            &mut pairs,
            Some("--server-url"),
            Some("url"),
            Some("SCANOPY_SERVER_URL"),
            server_url.clone(),
        );
        // A live credential must never sit in a filename. A compose file is a local artifact the
        // operator already holds, so it does carry the key.
        push(
            &mut pairs,
            Some("--daemon-api-key"),
            None,
            Some("SCANOPY_DAEMON_API_KEY"),
            daemon_api_key.clone(),
        );
        push(
            &mut pairs,
            Some("--daemon-port"),
            Some("port"),
            Some("SCANOPY_DAEMON_PORT"),
            daemon_port.map(|v| v.to_string()),
        );
        push(
            &mut pairs,
            Some("--bind-address"),
            Some("addr"),
            Some("SCANOPY_BIND_ADDRESS"),
            bind_address.clone(),
        );
        push(
            &mut pairs,
            Some("--log-level"),
            Some("loglevel"),
            Some("SCANOPY_LOG_LEVEL"),
            log_level.clone(),
        );
        push(
            &mut pairs,
            Some("--log-file"),
            Some("logfile"),
            Some("SCANOPY_LOG_FILE"),
            log_file.clone(),
        );
        push(
            &mut pairs,
            Some("--heartbeat-interval"),
            Some("heartbeat"),
            Some("SCANOPY_HEARTBEAT_INTERVAL"),
            heartbeat_interval.map(|v| v.to_string()),
        );
        push(
            &mut pairs,
            Some("--interfaces"),
            Some("interfaces"),
            Some("SCANOPY_INTERFACES"),
            interfaces
                .as_ref()
                .filter(|v| !v.is_empty())
                .map(|v| v.join(",")),
        );
        push(
            &mut pairs,
            Some("--allow-self-signed-certs"),
            Some("allowselfsigned"),
            Some("SCANOPY_ALLOW_SELF_SIGNED_CERTS"),
            allow_self_signed_certs.map(|v| v.to_string()),
        );
        push(
            &mut pairs,
            Some("--accept-invalid-scan-certs"),
            Some("acceptinvalidscan"),
            Some("SCANOPY_ACCEPT_INVALID_SCAN_CERTS"),
            accept_invalid_scan_certs.map(|v| v.to_string()),
        );

        pairs
    }
}

/// Debug is written in terms of [`DaemonArgs::install_config_pairs`] rather than derived, so it
/// cannot leak secrets: the api key is redacted explicitly, and the fields that never reach an
/// install artifact at all (the docker proxy TLS material among them) are simply not enumerated.
impl std::fmt::Debug for DaemonArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct("DaemonArgs");
        for pair in self.install_config_pairs() {
            let key = pair.cli_flag.or(pair.msi_key).unwrap_or("?");
            if pair.cli_flag == Some("--daemon-api-key") {
                out.field(key, &"<redacted>");
            } else {
                out.field(key, &pair.value);
            }
        }
        out.finish_non_exhaustive()
    }
}

/// Render a mode using clap's own `ValueEnum` naming, so the emitted value is by construction
/// one the daemon's `--mode` parser accepts.
fn render_mode(mode: Option<&DaemonMode>) -> Option<String> {
    use clap::ValueEnum;
    mode.and_then(|m| m.to_possible_value())
        .map(|v| v.get_name().to_string())
}

/// The name a daemon carries when nothing else names it. Also the name the server provisions
/// the integrated daemon under, so a self-host install's record and the daemon's own default
/// agree without either side having to remember the literal.
pub const DEFAULT_DAEMON_NAME: &str = "scanopy-daemon";

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
    /// Path to log file. None = platform default, "none" = disabled.
    #[serde(default)]
    pub log_file: Option<String>,
    pub heartbeat_interval: u64,
    pub bind_address: String,

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
    /// Per-daemon integration targeting parsed from the init command (credentialed cred↔IP and
    /// credential-less local sockets). Sent in the registration request and written to this
    /// daemon's Discovery. Local sockets are explicit opt-in here — there are no
    /// per-integration boolean enable flags.
    #[serde(default)]
    pub integration_targets: Vec<IntegrationTarget>,
    /// Daemon capabilities (docker socket availability, interfaced subnets)
    /// Updated after SelfReport discovery completes
    #[serde(default)]
    pub capabilities: LegacyCapabilities,
    /// Set to true after the first self-report completes
    #[serde(default)]
    pub has_self_reported: bool,
    /// Resolved on-disk path of this config, computed once by [`AppConfig::load`]. Runtime-only
    /// (never serialized): the daemon and installer use it instead of re-deriving the path, so the
    /// `--config-dir` override is honored consistently by both the write and read sides.
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
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
            name: DEFAULT_DAEMON_NAME.to_string(),
            log_level: "info".to_string(),
            log_file: None,
            heartbeat_interval: 30,
            // Start with no id — a provisioned daemon learns its server-assigned id from the
            // register / first-contact handshake, and sends a nil X-Daemon-ID until then. A
            // self-generated random id here would NOT match the provisioned record and the
            // server's 1:1 anti-reuse check would reject it as a daemon mismatch. (Legacy
            // daemons that self-registered keep their persisted id in config.json, unaffected.)
            id: Uuid::nil(),
            last_heartbeat: None,
            host_id: None,
            daemon_api_key: None,
            user_id: None,
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
            capabilities: LegacyCapabilities::default(),
            integration_targets: Vec::new(),
            has_self_reported: false,
            config_path: None,
        }
    }
}

impl AppConfig {
    /// Resolve the `config.json` path.
    ///
    /// When `config_dir` is `Some` (e.g. the `--config-dir` baked into a system service), the
    /// config lives at `<config_dir>/config.json` — an explicit, profile-independent location.
    /// Otherwise it falls back to the per-user [`ProjectDirs`] location, namespaced by daemon name
    /// (default name / `None` uses the legacy un-namespaced path for backward compat).
    pub fn get_config_path_for_name(
        name: Option<&str>,
        config_dir: Option<&Path>,
    ) -> Result<(bool, PathBuf)> {
        let config_path = if let Some(dir) = config_dir {
            dir.join("config.json")
        } else {
            let proj_dirs = ProjectDirs::from("com", "scanopy", "daemon")
                .ok_or_else(|| anyhow::anyhow!("Unable to determine config directory"))?;
            match name {
                // Use namespaced path for custom daemon names
                Some(n) if n != DEFAULT_DAEMON_NAME => {
                    proj_dirs.config_dir().join(n).join("config.json")
                }
                // Legacy path for default name or None
                _ => proj_dirs.config_dir().join("config.json"),
            }
        };

        Ok((config_path.exists(), config_path))
    }

    /// Get config path using default (legacy) per-user location.
    pub fn get_config_path() -> Result<(bool, PathBuf)> {
        Self::get_config_path_for_name(None, None)
    }

    /// Platform default **system** config directory for service installs — a fixed location the
    /// service can read regardless of the user profile it runs under (unlike the per-user
    /// [`ProjectDirs`] path). Mirrors [`AppConfig::default_log_path`]'s per-OS scheme; the installer
    /// namespaces this by daemon name.
    pub fn default_system_config_dir() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            PathBuf::from("/etc/scanopy/daemon")
        }

        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/Library/Application Support/Scanopy/daemon")
        }

        #[cfg(target_os = "windows")]
        {
            let program_data =
                std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
            PathBuf::from(program_data).join("Scanopy").join("daemon")
        }

        // FreeBSD / OpenBSD and other unixes: ports/pkg convention.
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            PathBuf::from("/usr/local/etc/scanopy/daemon")
        }
    }

    pub fn load(cli_args: DaemonArgs) -> anyhow::Result<Self> {
        // Determine config path from the daemon name and any explicit --config-dir override.
        let (config_exists, config_path) = AppConfig::get_config_path_for_name(
            cli_args.name.as_deref(),
            cli_args.config_dir.as_deref(),
        )?;

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

        // SCANOPY_CREDENTIAL_IDS carries the compact integration-target token grammar (see
        // `parse_integration_target_tokens`). Collect the raw comma-separated tokens here;
        // they're parsed into `integration_targets` after extraction (below), since the typed
        // enum doesn't round-trip cleanly through Figment env merging. CLI tokens take priority.
        let env_target_tokens: Option<Vec<String>> =
            std::env::var("SCANOPY_CREDENTIAL_IDS").ok().map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            });

        // Add environment variables (interfaces and credential_ids handled above to support
        // comma-separated values)
        figment = figment
            .merge(Env::prefixed("NETVISOR_").ignore(&["INTERFACES", "CREDENTIAL_IDS"]))
            .merge(Env::prefixed("SCANOPY_").ignore(&["INTERFACES", "CREDENTIAL_IDS"]));

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
        if let Some(log_file) = cli_args.log_file {
            figment = figment.merge(("log_file", log_file));
        }
        if let Some(heartbeat_interval) = cli_args.heartbeat_interval {
            figment = figment.merge(("heartbeat_interval", heartbeat_interval));
        }
        if let Some(bind_address) = cli_args.bind_address {
            figment = figment.merge(("bind_address", bind_address));
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
        // Whether the mode was explicitly chosen (CLI or env). If not, it is
        // inferred from server_url presence after extraction below, so the
        // two-flag install works: `--server-url … --api-key …` => DaemonPoll,
        // `--api-key …` (no server url) => ServerPoll.
        let mode_explicitly_set = cli_args.mode.is_some()
            || std::env::var("SCANOPY_MODE").is_ok()
            || std::env::var("NETVISOR_MODE").is_ok();
        if let Some(mode) = cli_args.mode {
            figment = figment.merge(("mode", mode));
        }
        if let Some(allow_self_signed_certs) = cli_args.allow_self_signed_certs {
            figment = figment.merge(("allow_self_signed_certs", allow_self_signed_certs));
        }
        if let Some(user_id) = cli_args.user_id {
            figment = figment.merge(("user_id", user_id));
        }
        if let Some(accept_invalid_scan_certs) = cli_args.accept_invalid_scan_certs {
            figment = figment.merge(("accept_invalid_scan_certs", accept_invalid_scan_certs));
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
        let mut config: AppConfig = figment
            .extract()
            .map_err(|e| Error::msg(format!("Configuration error: {}", e)))?;

        // Infer mode from server_url when it was not explicitly set: DaemonPoll
        // dials the server (needs a server_url), ServerPoll is dialed by the
        // server (no server_url). An explicit --mode / SCANOPY_MODE still wins.
        if !mode_explicitly_set {
            config.mode = if config.server_url.is_some() {
                DaemonMode::DaemonPoll
            } else {
                DaemonMode::ServerPoll
            };
        }

        // Parse integration-target tokens last so CLI > env > config-file precedence holds:
        // CLI tokens win if provided, else env tokens; if neither, keep whatever the config file
        // already deserialized into `integration_targets`.
        if let Some(tokens) = cli_args.credential_ids.or(env_target_tokens) {
            config.integration_targets = parse_integration_target_tokens(&tokens)?;
        }

        // Record the resolved path so the runtime read/write side uses it verbatim instead of
        // re-deriving (which would re-read `$HOME`/`--config-dir` and could diverge).
        config.config_path = Some(config_path);

        Ok(config)
    }

    /// Returns the resolved log file path based on config.
    /// Returns None if file logging is disabled (log_file = "none").
    pub fn resolve_log_path(&self) -> Option<PathBuf> {
        match self.log_file.as_deref() {
            Some("none") | Some("false") | Some("off") => None,
            Some(explicit_path) => Some(PathBuf::from(explicit_path)),
            None => Some(Self::default_log_path(&self.name)),
        }
    }

    /// Platform-specific default log file path.
    /// Always namespaced under a `scanopy/` subdirectory with `{name}.log`.
    pub fn default_log_path(name: &str) -> PathBuf {
        let filename = format!("{}.log", name);

        #[cfg(target_os = "linux")]
        {
            PathBuf::from("/var/log/scanopy").join(&filename)
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(home) = std::env::var_os("HOME") {
                PathBuf::from(home)
                    .join("Library/Logs/scanopy")
                    .join(&filename)
            } else {
                PathBuf::from("/tmp/scanopy").join(&filename)
            }
        }

        #[cfg(target_os = "windows")]
        {
            let program_data =
                std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
            PathBuf::from(program_data).join("scanopy").join(&filename)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            PathBuf::from("/tmp/scanopy").join(&filename)
        }
    }

    /// Platform default **system** log file for service installs — a fixed, root-writable path
    /// (unlike [`default_log_path`], which is `$HOME`-relative on macOS/BSD). The installer bakes
    /// this into the service via `--log-file` so logs land somewhere deterministic regardless of
    /// the service's runtime profile.
    pub fn default_system_log_path(name: &str) -> PathBuf {
        let filename = format!("{}.log", name);

        #[cfg(target_os = "windows")]
        {
            let program_data =
                std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
            PathBuf::from(program_data).join("scanopy").join(&filename)
        }

        #[cfg(not(target_os = "windows"))]
        {
            PathBuf::from("/var/log/scanopy").join(&filename)
        }
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
            tracing::debug!("No existing runtime config found, will create new on first save");
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

    /// Write the current config to disk, creating the parent directory if needed.
    ///
    /// Used by the `install` subcommand to persist `config.json` before starting the service.
    /// Reuses the atomic temp-write+rename in [`ConfigStore::save`] rather than hand-rolling a
    /// file write.
    pub async fn persist(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            async_fs::create_dir_all(parent)
                .await
                .context("Failed to create config directory")?;
        }
        let config = self.config.read().await.clone();
        self.save(&config).await
    }

    pub async fn get_id(&self) -> Result<Uuid> {
        let config = self.config.read().await;
        Ok(config.id)
    }

    pub async fn get_name(&self) -> Result<String> {
        let config = self.config.read().await;
        Ok(config.name.clone())
    }

    pub async fn set_name(&self, name: String) -> Result<()> {
        let mut config = self.config.write().await;
        config.name = name;
        self.save(&config.clone()).await
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

    /// Deprecated: scan settings are now per-discovery via ScanSettings.
    /// Kept for backwards compatibility with old daemons.
    pub async fn get_use_npcap_arp(&self) -> Result<bool> {
        let config = self.config.read().await;
        Ok(config.use_npcap_arp)
    }

    /// Deprecated: scan settings are now per-discovery via ScanSettings.
    pub async fn get_arp_retries(&self) -> Result<u32> {
        let config = self.config.read().await;
        Ok(config.arp_retries)
    }

    /// Deprecated: scan settings are now per-discovery via ScanSettings.
    pub async fn get_arp_rate_pps(&self) -> Result<u32> {
        let config = self.config.read().await;
        Ok(config.arp_rate_pps)
    }

    /// Deprecated: scan settings are now per-discovery via ScanSettings.
    pub async fn get_scan_rate_pps(&self) -> Result<u32> {
        let config = self.config.read().await;
        Ok(config.scan_rate_pps)
    }

    /// Deprecated: scan settings are now per-discovery via ScanSettings.
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

    pub async fn get_integration_targets(&self) -> Result<Vec<IntegrationTarget>> {
        let config = self.config.read().await;
        Ok(config.integration_targets.clone())
    }

    pub async fn get_capabilities(&self) -> Result<LegacyCapabilities> {
        let config = self.config.read().await;
        Ok(config.capabilities.clone())
    }

    pub async fn set_capabilities(&self, capabilities: LegacyCapabilities) -> Result<()> {
        let mut config = self.config.write().await;
        config.capabilities = capabilities;
        self.save(&config.clone()).await
    }

    pub async fn has_self_reported(&self) -> bool {
        self.config.read().await.has_self_reported
    }

    pub async fn set_has_self_reported(&self) -> Result<()> {
        let mut config = self.config.write().await;
        config.has_self_reported = true;
        self.save(&config.clone()).await
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serial_test::serial;

    use crate::daemon::shared::config::{DaemonCli, parse_integration_target_tokens};
    use crate::server::credentials::r#impl::mapping::IntegrationTarget;
    use crate::{daemon::shared::config::AppConfig, tests::DAEMON_CONFIG_FIXTURE};
    use clap::{CommandFactory, Parser};
    use std::collections::HashMap;
    use uuid::Uuid;

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

            panic!("Failed to load config fixture");
        }
    }

    /// The `install` subcommand doubles as reconfigure: it is re-run against an already-installed
    /// daemon with only the settings that changed, and layers them over that daemon's existing
    /// `config.json` before writing it back. The reconfigure command deliberately carries no
    /// credential, so anything it does *not* carry has to survive the round trip — a load that
    /// rebuilt from defaults would write back a config with no api key and no cached identity,
    /// leaving the daemon unable to authenticate.
    #[test]
    #[serial]
    fn a_reconfigure_load_keeps_the_settings_it_does_not_carry() {
        use crate::daemon::shared::config::DaemonArgs;

        let dir = tempfile::tempdir().unwrap();
        let daemon_id = Uuid::new_v4();
        std::fs::write(
            dir.path().join("config.json"),
            serde_json::json!({
                "name": "edge-01",
                "id": daemon_id,
                "daemon_api_key": "the-key-it-already-has",
                "server_url": "https://old.example",
                "daemon_port": 60073,
                "log_level": "info",
                "heartbeat_interval": 30,
                "bind_address": "0.0.0.0",
                "server_target": null,
                "server_port": null,
            })
            .to_string(),
        )
        .unwrap();

        let config = AppConfig::load(DaemonArgs {
            config_dir: Some(dir.path().to_path_buf()),
            server_url: Some("https://new.example".to_string()),
            ..Default::default()
        })
        .expect("config loads from the directory it will be written back to");

        assert_eq!(
            config.daemon_api_key.as_deref(),
            Some("the-key-it-already-has")
        );
        assert_eq!(config.id, daemon_id);
        assert_eq!(config.name, "edge-01");
        // …while what the command did carry still wins.
        assert_eq!(config.server_url.as_deref(), Some("https://new.example"));
    }

    #[derive(Debug)]
    struct FieldInfo {
        cli_flag: String,
        env_var: Option<String>,
        help_text: String,
    }

    const EXCLUDED_FIELDS: [&str; 19] = [
        "daemon_api_key",
        "network_id",
        "server_url",
        // Locator baked into system services by `install`, not a user-facing config field
        "config_dir",
        // Selects which already-installed daemon an install command acts on; resolved by the
        // installer, never persisted, and meaningless to a fresh MSI install
        "instance",
        // Internal marker baked into the Windows service binPath so the daemon runs under the
        // SCM dispatcher; not a user-facing config field
        "service",
        // Automatically set by install command, not user-configurable
        "user_id",
        "credential_ids",
        // Legacy fields not exposed in UI
        "server_target",
        "server_port",
        // Configured via credentials/discovery UI, not daemon config
        "docker_proxy",
        "docker_proxy_ssl_cert",
        "docker_proxy_ssl_key",
        "docker_proxy_ssl_chain",
        // Deprecated: scan settings are now per-discovery via ScanSettings
        "use_npcap_arp",
        "arp_retries",
        "arp_rate_pps",
        "scan_rate_pps",
        "port_scan_batch_size",
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

    /// Every `--flag` the Windows MSI passes to `scanopy-daemon install` must be a real
    /// `DaemonArgs` long flag. This catches drift across the Rust↔WiX-XML boundary (a
    /// compile-time check isn't possible there) — e.g. renaming a flag in Rust without
    /// updating `backend/wix/main.wxs`, or a typo'd flag in the installer.
    #[test]
    fn msi_install_flags_are_valid_cli_flags() {
        let wxs = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/wix/main.wxs"))
            .expect("read backend/wix/main.wxs");

        let cmd = DaemonCli::command();
        let valid: std::collections::HashSet<String> = cmd
            .get_arguments()
            .filter_map(|a| a.get_long().map(|s| s.to_string()))
            .collect();

        // Collect every `--flag` token in the file. The .wxs only uses `--` for daemon
        // CLI flags (HTML comment `<!--`/`-->` markers decode to an empty token and are
        // skipped), so any token that isn't a known flag is real drift.
        let bytes = wxs.as_bytes();
        let mut msi_flags = std::collections::HashSet::new();
        let mut i = 0;
        while i + 2 < bytes.len() {
            if bytes[i] == b'-' && bytes[i + 1] == b'-' {
                let start = i + 2;
                let mut j = start;
                while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'-') {
                    j += 1;
                }
                if j > start {
                    msi_flags.insert(wxs[start..j].to_string());
                }
                i = j.max(i + 2);
            } else {
                i += 1;
            }
        }

        let unknown: Vec<&String> = msi_flags.iter().filter(|f| !valid.contains(*f)).collect();
        assert!(
            unknown.is_empty(),
            "backend/wix/main.wxs references daemon flags that don't exist in DaemonArgs: {:?}",
            unknown
        );

        // COMPLETENESS: the MSI must expose every config field the UI does (the UI's fieldDefs
        // == DaemonArgs minus EXCLUDED_FIELDS, enforced by config_fields_are_in_sync), except
        // the ones a Windows service install genuinely can't/shouldn't carry: network + user
        // come from the 1:1 key, the reachable url is captured at provision, credential refs are
        // seeded at provision, and config-dir is baked by the installer. Without this direction,
        // a field could be dropped from the MSI silently (the validity check above only rejects
        // *extra* flags, not missing ones).
        const MSI_EXCLUDED_FLAGS: &[&str] = &[
            "network-id",
            "user-id",
            "daemon-url",
            "credential-id",
            "config-dir",
        ];
        let expected: std::collections::HashSet<String> = cmd
            .get_arguments()
            .filter(|a| {
                let id = a.get_id().to_string();
                id != "help" && id != "version" && !EXCLUDED_FIELDS.contains(&id.as_str())
            })
            .filter_map(|a| a.get_long().map(|s| s.to_string()))
            .filter(|f| !MSI_EXCLUDED_FLAGS.contains(&f.as_str()))
            .collect();
        let missing: Vec<&String> = expected
            .iter()
            .filter(|f| !msi_flags.contains(*f))
            .collect();
        assert!(
            missing.is_empty(),
            "backend/wix/main.wxs is missing config flags the UI exposes: {:?}. \
             Add them as MSI properties + install-command flags, or list them in \
             MSI_EXCLUDED_FLAGS if a service install intentionally can't carry them.",
            missing
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
        s.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim_end_matches('.')
            .to_string()
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
        let config = AppConfig::load(cli.run_args).expect("Failed to load config");

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

    /// Verifies that SCANOPY_CREDENTIAL_IDS accepts a comma-separated list of integration-target
    /// tokens and populates `integration_targets`. Figment does not auto-split comma-separated env
    /// values, so the daemon splits + parses them itself (regression test for #612, extended for
    /// the compact token grammar in #637).
    #[test]
    #[serial]
    fn test_scanopy_credential_ids_env_var() {
        let id1 = Uuid::parse_str("d0f384e1-3ac0-47ab-b9b9-3fc806353b76").unwrap();
        let id2 = Uuid::parse_str("36045aa7-c38c-45f6-928d-eccfbfabdede").unwrap();

        // Save and clear any existing value
        let original = std::env::var("SCANOPY_CREDENTIAL_IDS").ok();
        // SAFETY: This test runs serially and restores the original value after
        unsafe {
            std::env::set_var(
                "SCANOPY_CREDENTIAL_IDS",
                format!("{id1}@127.0.0.1,{id1},{id2}@10.0.0.5+10.0.0.6"),
            )
        };

        // Load config with empty CLI args (env var should take effect)
        let cli = DaemonCli::parse_from::<[&str; 0], &str>([]);
        let config = AppConfig::load(cli.run_args).expect("Failed to load config");

        // Restore original value
        // SAFETY: This test runs serially
        unsafe {
            if let Some(val) = original {
                std::env::set_var("SCANOPY_CREDENTIAL_IDS", val);
            } else {
                std::env::remove_var("SCANOPY_CREDENTIAL_IDS");
            }
        }

        assert_eq!(
            config.integration_targets,
            vec![
                IntegrationTarget::DaemonHost { credential_id: id1 },
                IntegrationTarget::Network { credential_id: id1 },
                IntegrationTarget::Hosts {
                    credential_id: id2,
                    ips: vec!["10.0.0.5".parse().unwrap(), "10.0.0.6".parse().unwrap()],
                },
            ],
            "SCANOPY_CREDENTIAL_IDS env var should populate integration_targets field"
        );
    }

    #[test]
    fn test_parse_integration_target_tokens_grammar() {
        let id = Uuid::new_v4();
        let tokens = vec![
            id.to_string(),
            format!("{id}@127.0.0.1"),
            format!("{id}@::1"),
            format!("{id}@10.0.0.5"),
            format!("{id}@127.0.0.1+10.0.0.5"),
        ];
        let parsed = parse_integration_target_tokens(&tokens).expect("tokens parse");
        assert_eq!(
            parsed,
            vec![
                // bare uuid → network default
                IntegrationTarget::Network { credential_id: id },
                // sole loopback (v4 or v6) → the daemon host
                IntegrationTarget::DaemonHost { credential_id: id },
                IntegrationTarget::DaemonHost { credential_id: id },
                // a remote IP → specific host
                IntegrationTarget::Hosts {
                    credential_id: id,
                    ips: vec!["10.0.0.5".parse().unwrap()],
                },
                // loopback mixed with a remote IP is not a sole-loopback → Hosts
                IntegrationTarget::Hosts {
                    credential_id: id,
                    ips: vec!["127.0.0.1".parse().unwrap(), "10.0.0.5".parse().unwrap()],
                },
            ]
        );

        // Invalid tokens are rejected.
        assert!(parse_integration_target_tokens(&["not-a-uuid".to_string()]).is_err());
        assert!(
            parse_integration_target_tokens(&[format!("{id}@not-an-ip")]).is_err(),
            "invalid IP should error"
        );
        assert!(
            parse_integration_target_tokens(&[format!("{id}@")]).is_err(),
            "trailing @ with no IPs should error"
        );
    }

    /// The server renders these tokens into a docker-compose `SCANOPY_CREDENTIAL_IDS`, and the
    /// daemon parses them back with the function above. Round-tripping every variant through
    /// both is what keeps the writer and the reader of that grammar in agreement.
    #[test]
    fn integration_target_tokens_round_trip_through_display() {
        let id = Uuid::new_v4();
        let targets = vec![
            IntegrationTarget::Network { credential_id: id },
            IntegrationTarget::DaemonHost { credential_id: id },
            IntegrationTarget::Hosts {
                credential_id: id,
                ips: vec!["10.0.0.5".parse().unwrap()],
            },
            IntegrationTarget::Hosts {
                credential_id: id,
                ips: vec!["127.0.0.1".parse().unwrap(), "10.0.0.5".parse().unwrap()],
            },
        ];

        let tokens: Vec<String> = targets.iter().map(|t| t.to_string()).collect();
        assert_eq!(
            parse_integration_target_tokens(&tokens).expect("rendered tokens parse"),
            targets
        );
    }
}
