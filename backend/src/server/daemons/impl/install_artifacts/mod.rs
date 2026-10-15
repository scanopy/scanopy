//! Server-side assembly of daemon install artifacts.
//!
//! The server is the single source of truth for the install command — it knows its own public
//! URL and the canonical flag format, so the UI/MCP/email don't each re-derive them. This is a
//! pure builder: it never mints or persists anything. The daemon's api key is emitted as the
//! [`API_KEY_PLACEHOLDER`] and the frontend substitutes the plaintext it holds from the
//! provision response (the plaintext exists only at mint time, and is never stored for
//! DaemonPoll). The MSI is a static release asset the UI links to directly; only its per-daemon
//! pre-fill filename ([`encode_msi_filename`], no secret) is built here.

use base64ct::{Base64UrlUnpadded, Encoding};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::base::{Daemon, DaemonMode, DaemonOs};
use super::version::{installed_by_install_engine, own_version, persists_docker_config};
use crate::daemon::shared::config::{DOCKER_CONFIG_DIR, DaemonArgs};
use crate::server::billing::types::base::BillingPlan;
use crate::server::config::{AppState, DeploymentType, get_deployment_type};
use crate::server::credentials::r#impl::mapping::IntegrationTarget;
use crate::server::license::service::self_hosted_plan;

/// The `install.sh` one-liner that fetches + runs the Unix installer bootstrap. On its own the
/// script downloads from GitHub; [`DownloadSource::Scarf`] appends `--gateway`.
const UNIX_INSTALL_SCRIPT: &str = "bash -c \"$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)\"";
/// Windows daemon exe through the Scarf gateway, which counts the download and redirects to the
/// GitHub release asset.
const WINDOWS_EXE_SCARF_URL: &str =
    "https://get.scanopy.net/latest/scanopy-daemon-windows-amd64.exe";
/// The same exe straight from GitHub, for when Scarf is down or blocked.
const WINDOWS_EXE_GITHUB_URL: &str =
    "https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.exe";
/// The signed Windows MSI release asset. A static GitHub asset URL — the UI hardcodes it as a
/// const rather than the server sending it per-provision (it's the same for every tenant). Kept
/// here for reuse by any server-side MSI tooling; only the per-daemon [`encode_msi_filename`]
/// name is tenant-specific and travels in the provision response.
pub const WINDOWS_MSI_URL: &str =
    "https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.msi";
/// The MSI through the Scarf gateway. Requested only to count a download; the bytes come from
/// [`WINDOWS_MSI_URL`].
pub const WINDOWS_MSI_SCARF_URL: &str =
    "https://get.scanopy.net/latest/scanopy-daemon-windows-amd64.msi";
/// The daemon image, pulled through the Scarf gateway in front of GHCR.
const DAEMON_IMAGE_SCARF_REPO: &str = "docker.scanopy.net/scanopy/scanopy/daemon";
/// The daemon image straight from GHCR.
const DAEMON_IMAGE_GHCR_REPO: &str = "ghcr.io/scanopy/scanopy/daemon";
/// Compose comment above a Scarf image line, for when Scarf is down or blocked.
const DOCKER_PULL_HELP: &str = "# Pull fails? https://scanopy.net/docs/setting-up-daemons/troubleshooting-setup/#docker-pull-or-download-fails";

/// Where generated commands download the daemon from. Scarf counts downloads for Scanopy Cloud
/// and commercial self-hosted servers; a Community Edition server downloads from GitHub and GHCR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadSource {
    Scarf,
    Github,
}

impl From<&BillingPlan> for DownloadSource {
    fn from(plan: &BillingPlan) -> Self {
        match plan {
            BillingPlan::Community(_) => Self::Github,
            BillingPlan::Free(_)
            | BillingPlan::Starter(_)
            | BillingPlan::Pro(_)
            | BillingPlan::Team(_)
            | BillingPlan::Business(_)
            | BillingPlan::Enterprise(_)
            | BillingPlan::Demo(_)
            | BillingPlan::CommercialSelfHosted(_)
            | BillingPlan::SelfHostedStandard(_)
            | BillingPlan::SelfHostedPlus(_) => Self::Scarf,
        }
    }
}

impl DownloadSource {
    /// The source for this server. Every Cloud org gets Scarf. A self-hosted server's orgs all
    /// carry the plan its license resolves to (Community without a valid license), so that plan
    /// decides.
    pub async fn for_server(state: &AppState) -> Self {
        if get_deployment_type(&state.config) == DeploymentType::Cloud {
            return Self::Scarf;
        }
        (&self_hosted_plan(state.services.license_service.as_deref()).await).into()
    }

    fn unix_install_script(self) -> String {
        match self {
            // `install.sh` lands in `$0`, so `--gateway` reaches the script as `$1`.
            Self::Scarf => format!("{UNIX_INSTALL_SCRIPT} install.sh --gateway"),
            Self::Github => UNIX_INSTALL_SCRIPT.to_string(),
        }
    }

    /// The compose `image:` line for `tag`, with the pull help above it on Scarf.
    fn daemon_image_lines(self, tag: &str) -> Vec<String> {
        match self {
            Self::Scarf => vec![
                DOCKER_PULL_HELP.to_string(),
                format!("image: {DAEMON_IMAGE_SCARF_REPO}:{tag}"),
            ],
            Self::Github => vec![format!("image: {DAEMON_IMAGE_GHCR_REPO}:{tag}")],
        }
    }

    /// PowerShell that downloads the Windows exe to `out_file`. Through Scarf it falls back to
    /// GitHub. Every request stops on error, so a caller running under
    /// `$ErrorActionPreference = 'Stop'` halts when no source answers.
    fn windows_exe_download(self, out_file: &str) -> String {
        match self {
            Self::Scarf => format!(
                "try {{ Invoke-WebRequest -Uri \"{WINDOWS_EXE_SCARF_URL}\" -OutFile \"{out_file}\" -ErrorAction Stop }} \
                 catch {{ Write-Host \"get.scanopy.net unreachable, downloading from GitHub\"; \
                 Invoke-WebRequest -Uri \"{WINDOWS_EXE_GITHUB_URL}\" -OutFile \"{out_file}\" -ErrorAction Stop }}"
            ),
            Self::Github => format!(
                "Invoke-WebRequest -Uri \"{WINDOWS_EXE_GITHUB_URL}\" -OutFile \"{out_file}\" -ErrorAction Stop"
            ),
        }
    }
}

/// The docker install method.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DockerInstall {
    /// A ready-to-run `docker-compose.yml` for a first install. `None` for a reconfigure — the
    /// operator keeps their own compose and swaps in `env`, rather than replacing the whole file.
    pub compose: Option<String>,
    /// The `SCANOPY_*` environment variables (`KEY=value`) this daemon is configured with. For a
    /// reconfigure these are exactly the vars that changed, so the UI can show them as a swap-in.
    pub env: Vec<String>,
}

/// The Windows MSI install method. The MSI itself is a static release asset the UI links to; only
/// the per-daemon pre-fill data is tenant-specific.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MsiInstall {
    /// Filename encoding this daemon's non-secret config. Save or rename the downloaded MSI to
    /// this name to pre-fill the installer — parse-filename.js decodes it. The api key is never
    /// encoded. Renaming a signed MSI doesn't affect its signature.
    pub filename: String,
    /// Config keys that did not fit in `filename` (a filename is capped at 255 characters). Empty
    /// for any ordinary config. The MSI falls back to its built-in defaults for these, so the UI
    /// should tell the user to set them in the installer — the other methods carry the full config.
    pub omitted_config_keys: Vec<String>,
}

/// Everything the UI needs to install (or reconfigure) a daemon, one field per install method so
/// each is a first-class peer with its own content — no method is a special case bolted onto a
/// list. The binary methods are ready-to-paste commands (any api key is the [`API_KEY_PLACEHOLDER`],
/// filled in client-side); docker and msi carry their own structured content.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstallArtifacts {
    /// Download for Linux.
    pub linux: String,
    /// Download for macOS.
    pub macos: String,
    /// Download for Windows.
    pub windows: String,
    /// Download for FreeBSD.
    pub freebsd: String,
    /// Container image reference.
    pub docker: DockerInstall,
    /// Windows installer package.
    pub msi: MsiInstall,
}

impl InstallArtifacts {
    /// The ready-to-run text for one install method: the OS's binary command, or the docker
    /// compose when `docker` is set. Docker is Linux-only, and a reconfigure has no compose, so
    /// those combinations have no command.
    pub fn command(&self, os: DaemonOs, docker: bool) -> Option<&str> {
        match (os, docker) {
            (DaemonOs::Linux, true) => self.docker.compose.as_deref(),
            (_, true) => None,
            (DaemonOs::Linux, false) => Some(&self.linux),
            (DaemonOs::MacOS, false) => Some(&self.macos),
            (DaemonOs::Windows, false) => Some(&self.windows),
            (DaemonOs::FreeBsd, false) => Some(&self.freebsd),
        }
    }
}

/// What the caller wants the command to do — the one axis that actually varies.
///
/// `install` brings a new daemon up: it carries the api-key placeholder, fetches the binary, and
/// spells out the connectivity + advanced config, and nothing else. `rekey` is the same command
/// for a daemon that already exists on a host (a legacy daemon getting its first bound key), so
/// once that daemon has connected it also names the install to act on. `reconfigure` adjusts an
/// already-installed daemon in place: no key, no fetch, just the server-held connectivity —
/// `scanopy-daemon install` layers it over the existing `config.json`.
///
/// `install` and `rekey` are separate because the record can't tell them apart: a daemon created
/// in the wizard has `last_seen` set as soon as it connects, and the wizard refetches its command
/// after that, which must not change shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstallCommandType {
    Install,
    Rekey,
    Reconfigure,
}

impl InstallCommandType {
    /// Whether the emitted command carries the daemon's api key.
    ///
    /// The plaintext is never known here — the builder emits the [`API_KEY_PLACEHOLDER`] and the
    /// frontend fills it from the key it minted.
    pub fn embeds_key(&self) -> bool {
        matches!(self, Self::Install | Self::Rekey)
    }
}

/// Stand-in for the daemon's api key in an emitted command. The builder never mints, so it
/// cannot know the plaintext; it emits this token and the frontend substitutes the key it holds
/// from the provision/associate response. Keeps command *format* server-authored while the one
/// secret is handled exactly once, at mint time.
pub const API_KEY_PLACEHOLDER: &str = "<API_KEY>";

/// Resolve the daemon config to emit for a given purpose, with every server-controlled field
/// taken from the daemon record.
///
/// The advanced fields (`log_level`, `interfaces`, …) are the only ones a client can set —
/// the rest are `#[serde(skip)]` on [`DaemonArgs`] and so never survive deserialization — but
/// the overwrite is unconditional rather than trusting that, since this is what guarantees the
/// emitted artifacts describe the daemon that was actually provisioned.
fn install_args(
    public_url: &str,
    daemon: &Daemon,
    install_config: Option<&DaemonArgs>,
    command_type: InstallCommandType,
) -> DaemonArgs {
    // Only an install or re-key seeds from the caller's advanced settings; a reconfigure keeps
    // whatever is already in the daemon's config.json.
    let mut args = match command_type {
        InstallCommandType::Install | InstallCommandType::Rekey => {
            install_config.cloned().unwrap_or_default()
        }
        InstallCommandType::Reconfigure => DaemonArgs::default(),
    };

    // Both types carry the server-held connectivity. Only DaemonPoll dials the server, so only
    // it gets a server url; its absence is also how the daemon infers ServerPoll, which is why
    // no command needs `--mode`. ServerPoll instead needs the port the server dials, taken from
    // the record. `url` is empty for DaemonPoll.
    args.server_url = match daemon.base.mode {
        DaemonMode::DaemonPoll if !public_url.is_empty() => Some(public_url.to_string()),
        _ => None,
    };
    args.daemon_port = (daemon.base.mode == DaemonMode::ServerPoll)
        .then(|| {
            url::Url::parse(&daemon.base.url)
                .ok()
                .and_then(|u| u.port_or_known_default())
        })
        .flatten();

    if command_type.embeds_key() {
        // name/mode only reach the MSI pre-fill (both have no CLI flag), so the command still
        // omits `--name`; the MSI needs them up front. The builder never mints, so the key flag
        // carries the placeholder and the frontend fills it in.
        args.name = Some(daemon.base.name.clone());
        args.mode = Some(daemon.base.mode);
        args.daemon_api_key = Some(API_KEY_PLACEHOLDER.to_string());
    }
    // Reconfigure carries no credential (it is displayed persistently, and omitting it leaves
    // the daemon's existing key in place) and no name/mode.

    // Which daemon on the target host the command acts on. A host can run several, and the
    // command carries no identity, so the installer would otherwise have to ask. The daemon id is
    // the right selector: non-secret, immutable, and cached in each install's config.json from the
    // handshake — so it resolves exactly, unlike a name the operator can change in the UI. It is
    // not an identity *assertion*: the server takes a provisioned daemon's identity from the 1:1
    // key binding and ignores what the client claims.
    //
    // Emitted on every reconfigure, and on a re-key of a daemon that has connected before. Never on
    // an install: a first install has nothing to select, and its command must read the same
    // before and after the daemon's first handshake.
    args.instance = match command_type {
        InstallCommandType::Install => None,
        InstallCommandType::Rekey => daemon.base.last_seen.map(|_| daemon.id.to_string()),
        InstallCommandType::Reconfigure => Some(daemon.id.to_string()),
    };

    args
}

/// Filesystem limit the encoded name has to live within.
const MAX_MSI_FILENAME_LEN: usize = 255;
const MSI_FILENAME_PREFIX: &str = "scanopy-daemon-";
const MSI_FILENAME_SUFFIX: &str = ".msi";

/// Percent-escape only what the query grammar actually needs — the `%` escape marker and the
/// `&`/`=` delimiters — plus non-ASCII, which the JScript decoder handles byte-wise. Everything
/// else survives verbatim: the whole query is base64url'd into the filename anyway, so escaping
/// spaces and backslashes bought nothing and cost 3 characters each against a tight budget.
fn escape_msi_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '%' => out.push_str("%25"),
            '&' => out.push_str("%26"),
            '=' => out.push_str("%3D"),
            c if c.is_ascii() => out.push(c),
            c => {
                let mut buf = [0u8; 4];
                for byte in c.encode_utf8(&mut buf).as_bytes() {
                    out.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    out
}

/// Longest query that still fits, given base64 expands 3 bytes to 4 characters.
fn max_msi_query_len() -> usize {
    let budget = MAX_MSI_FILENAME_LEN - MSI_FILENAME_PREFIX.len() - MSI_FILENAME_SUFFIX.len();
    budget / 4 * 3
}

/// Build the config query string the MSI filename encodes. Keys match the property map in
/// `backend/wix/parse-filename.js`. The api key is deliberately absent — a live credential must
/// never sit in a filename.
///
/// The whole config has to fit in a filename, which a pathological config (long url + long log
/// path + several long Windows interface names) can exceed. Pairs are taken in order until the
/// budget runs out, and [`DaemonArgs::install_config_pairs`] yields identity first, so what
/// survives is always the part that makes the installer usable. Anything dropped is returned so
/// the caller can tell the user, rather than the MSI quietly installing a differently-configured
/// daemon than the command on screen.
fn msi_config_query(args: &DaemonArgs) -> (String, Vec<String>) {
    let budget = max_msi_query_len();
    let mut query = String::new();
    let mut omitted = Vec::new();

    for pair in args.install_config_pairs() {
        let Some(key) = pair.msi_key else { continue };
        let encoded = format!("{key}={}", escape_msi_value(&pair.value));
        let separator = usize::from(!query.is_empty());
        if query.len() + separator + encoded.len() > budget {
            omitted.push(key.to_string());
            continue;
        }
        if separator == 1 {
            query.push('&');
        }
        query.push_str(&encoded);
    }

    (query, omitted)
}

/// Build the MSI download filename: the whole config query string as ONE base64url segment,
/// so the name stays short even as more config fields are added (vs one `~~field=hex~~` per
/// field, which would blow past the ~255-char filename limit). Decoded by parse-filename.js.
/// Also returns the config keys that did not fit (see [`msi_config_query`]).
pub fn encode_msi_filename(
    public_url: &str,
    daemon: &Daemon,
    install_config: Option<&DaemonArgs>,
) -> (String, Vec<String>) {
    // The MSI only ever performs a first install, so it always pre-fills the full config.
    let args = install_args(
        public_url,
        daemon,
        install_config,
        InstallCommandType::Install,
    );
    let (query, omitted) = msi_config_query(&args);
    let blob = Base64UrlUnpadded::encode_string(query.as_bytes());
    (
        format!("{MSI_FILENAME_PREFIX}{blob}{MSI_FILENAME_SUFFIX}"),
        omitted,
    )
}

/// Quote a value for a POSIX shell, leaving already-safe values bare so the common command
/// stays readable. Values like a Windows log path or an interface name can contain spaces.
fn quote_posix(value: &str) -> String {
    if is_shell_safe(value) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Quote a value for PowerShell. Same intent as [`quote_posix`], but a literal single quote is
/// escaped by doubling rather than by the POSIX close-escape-reopen dance.
fn quote_powershell(value: &str) -> String {
    if is_shell_safe(value) {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "''"))
}

fn is_shell_safe(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:,=@+".contains(c))
}

/// The `install` flags for a resolved config. DaemonPoll dials the server (so it carries
/// `--server-url`), ServerPoll is dialed by the server (so it does not). Name and site never
/// reach the CLI — the daemon learns its name via the handshake — so the command carries
/// neither; everything else set on `args` is emitted, including the `--instance` selector that
/// tells a multi-daemon host which install the command is for.
fn install_flags(args: &DaemonArgs, quote: fn(&str) -> String) -> String {
    args.install_config_pairs()
        .iter()
        .filter_map(|p| {
            p.cli_flag.map(|flag| {
                // The key value is left bare: an api key is always shell-safe, and the
                // placeholder must stay a clean find-and-replace target for the frontend —
                // quoting `<API_KEY>` would leave stray quotes around the substituted key.
                let value = if flag == "--daemon-api-key" {
                    p.value.clone()
                } else {
                    quote(&p.value)
                };
                format!("{flag} {value}")
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Quote a compose env value if YAML would otherwise mis-read it. Values sit in a
/// `- KEY=value` list item, where most characters are fine bare; leading/trailing whitespace
/// and a ` #` (which starts a comment) are the cases that need quoting.
fn quote_yaml(value: &str) -> String {
    let needs_quoting = value.trim() != value || value.contains(" #") || value.contains('"');
    if !needs_quoting {
        return value.to_string();
    }
    format!("\"{}\"", value.replace('\\', r"\\").replace('"', "\\\""))
}

/// The `SCANOPY_*` environment variables the docker daemon is configured with, as `KEY=value`
/// lines. This is the daemon's config expressed for compose.
///
/// The set comes from the same [`DaemonArgs::install_config_pairs`] table as the CLI and MSI
/// artifacts, so they cannot drift. Notably no site id, user id, name or mode: those are
/// `#[serde(skip)]` on [`DaemonArgs`] precisely because a client must not assert them, and the
/// binary install command dropped them for the same reason — identity comes from the 1:1
/// api-key binding and the handshake. A compose that asserted them could disagree with the
/// record its key is bound to.
///
/// For a reconfigure these are exactly the vars that changed (connectivity), so the UI can show
/// them as a swap-in for the operator's existing compose rather than a whole replacement file.
fn docker_env_lines(args: &DaemonArgs, seed_credential_refs: &[IntegrationTarget]) -> Vec<String> {
    let mut env: Vec<String> = args
        .install_config_pairs()
        .iter()
        .filter_map(|p| {
            p.env_var
                .map(|key| format!("{key}={}", quote_yaml(&p.value)))
        })
        .collect();

    if !seed_credential_refs.is_empty() {
        let tokens = seed_credential_refs
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(",");
        env.push(format!("SCANOPY_CREDENTIAL_IDS={}", quote_yaml(&tokens)));
    }

    env
}

/// Build the full `docker-compose.yml` for a first install, from the config env lines.
fn docker_compose(env_lines: &[String], daemon: &Daemon, source: DownloadSource) -> String {
    let mut env = env_lines.to_vec();

    // Docker-only: logs must land on the mounted volume to survive the container, so a log file
    // is always set even when the CLI install would leave the platform default in place.
    if !env.iter().any(|e| e.starts_with("SCANOPY_LOG_FILE=")) {
        env.push(format!(
            "SCANOPY_LOG_FILE=/var/log/scanopy/{}.log",
            daemon.base.name
        ));
    }

    let volumes = [
        format!("daemon-config:{DOCKER_CONFIG_DIR}"),
        "/var/run/docker.sock:/var/run/docker.sock:ro".to_string(),
        "/var/log/scanopy:/var/log/scanopy".to_string(),
    ];

    let mut lines = vec!["services:".to_string(), "  daemon:".to_string()];
    lines.extend(
        source
            .daemon_image_lines("latest")
            .iter()
            .map(|l| format!("    {l}")),
    );
    lines.extend([
        "    container_name: scanopy-daemon".to_string(),
        "    network_mode: host".to_string(),
        "    privileged: true".to_string(),
        "    restart: unless-stopped".to_string(),
        "    environment:".to_string(),
    ]);
    lines.extend(env.iter().map(|e| format!("      - {e}")));
    lines.push("    volumes:".to_string());
    lines.extend(volumes.iter().map(|v| format!("      - {v}")));
    lines.push(String::new());
    lines.push("volumes:".to_string());
    lines.push("  daemon-config:".to_string());

    lines.join("\n")
}

/// Assemble the install artifacts for a daemon, shaped by what they are for.
///
/// This is a pure function of its inputs — it never mints or persists anything. Any api key in
/// the emitted commands is the [`API_KEY_PLACEHOLDER`], filled in client-side.
pub fn build_install_artifacts(
    public_url: &str,
    daemon: &Daemon,
    install_config: Option<&DaemonArgs>,
    seed_credential_refs: &[IntegrationTarget],
    command_type: InstallCommandType,
    source: DownloadSource,
) -> InstallArtifacts {
    let public_url = public_url.trim_end_matches('/');
    let args = install_args(public_url, daemon, install_config, command_type);

    // A reconfigure runs against an already-installed daemon, so it must not re-fetch the
    // binary — it only re-asserts config. An install fetches: even re-keying a legacy daemon,
    // picking up the current binary alongside the new key is desirable.
    let (unix, windows) = match command_type {
        InstallCommandType::Reconfigure => (
            format!(
                "sudo scanopy-daemon install {}",
                install_flags(&args, quote_posix)
            ),
            // On Windows the binary lives in Program Files rather than on PATH.
            format!(
                "& \"$env:ProgramFiles\\Scanopy\\scanopy-daemon.exe\" install {}",
                install_flags(&args, quote_powershell)
            ),
        ),
        InstallCommandType::Install | InstallCommandType::Rekey => (
            // Unix binary platforms share the fetch-script + `install` shape.
            format!(
                "{} && sudo scanopy-daemon install {}",
                source.unix_install_script(),
                install_flags(&args, quote_posix)
            ),
            // `Stop` halts before `install` when no download source answers.
            format!(
                "$ErrorActionPreference = 'Stop'; {}; .\\scanopy-daemon-windows-amd64.exe install {}",
                source.windows_exe_download("scanopy-daemon-windows-amd64.exe"),
                install_flags(&args, quote_powershell)
            ),
        ),
    };

    // Docker's config as env lines. An *install* also gets a ready-to-run compose; a
    // *reconfigure* gets no compose — replacing a running container's whole compose would drop
    // the operator's own settings — only the changed env vars, which the UI shows as a swap-in.
    // The daemon reads its key + prior config from the persisted `daemon-config` volume either
    // way, just as the binary reconfigure relies on the on-disk config.json.
    let docker_env = docker_env_lines(&args, seed_credential_refs);
    let compose = (command_type != InstallCommandType::Reconfigure)
        .then(|| docker_compose(&docker_env, daemon, source));

    let (msi_filename, msi_omitted_config_keys) =
        encode_msi_filename(public_url, daemon, install_config);

    InstallArtifacts {
        linux: unix.clone(),
        macos: unix.clone(),
        freebsd: unix,
        windows,
        docker: DockerInstall {
            compose,
            env: docker_env,
        },
        msi: MsiInstall {
            filename: msi_filename,
            omitted_config_keys: msi_omitted_config_keys,
        },
    }
}

/// What one upgrade step does. The UI labels each step by its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum UpgradeStepType {
    /// Fetch the new binary into place.
    Download,
    /// Restart the daemon's service onto the new binary.
    Restart,
    /// Stop the daemon's service (Windows, daemons installed before the install engine).
    Stop,
    /// Start the daemon's service (Windows, daemons installed before the install engine).
    Start,
}

/// One command of a binary upgrade, in the order it runs.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpgradeStep {
    pub r#type: UpgradeStepType,
    /// The shell command to run.
    pub command: String,
}

impl UpgradeStep {
    fn new(r#type: UpgradeStepType, command: String) -> Self {
        Self { r#type, command }
    }
}

/// The docker upgrade method.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DockerUpgrade {
    /// Copies the daemon's config onto the `daemon-config` volume before the recreate. Set for
    /// images that keep config outside the volume, where the recreate would otherwise erase it.
    pub config_copy: Option<String>,
    /// Pulls and recreates on the `latest` tag.
    pub pull: String,
    /// The compose `image:` line pinned to this server's version.
    pub pinned_image: String,
}

/// Everything the UI needs to upgrade a daemon: the binary steps per OS, and the docker method.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpgradeArtifacts {
    /// Binary upgrade steps on Linux.
    pub linux: Vec<UpgradeStep>,
    /// Binary upgrade steps on macOS.
    pub macos: Vec<UpgradeStep>,
    /// Binary upgrade steps on Windows.
    pub windows: Vec<UpgradeStep>,
    /// Binary upgrade steps on FreeBSD.
    pub freebsd: Vec<UpgradeStep>,
    pub docker: DockerUpgrade,
    /// The restart step names the service after the daemon, as installs from before the install
    /// engine were named. Such a daemon may have been set up differently, so the UI points the
    /// operator to `scanopy-daemon list` for the real service.
    pub legacy_service_name: bool,
}

/// Program Files directory the Windows installer places the daemon binary in.
const WINDOWS_BIN_DIR: &str = "$env:ProgramFiles\\Scanopy";

/// Assemble the upgrade steps for a daemon.
///
/// A daemon installed by the install engine is restarted with `restart --instance <daemon id>`.
/// The restart runs the binary the download step just placed, so it has the subcommand whatever
/// version the daemon reports, and the installer resolves the id to the slot's service on its
/// own. The server never names the service. Older daemons get the service named after the
/// daemon, which is how they were registered before slots.
pub fn build_upgrade_artifacts(daemon: &Daemon, source: DownloadSource) -> UpgradeArtifacts {
    use crate::daemon::install::{launchd_label, service_id};
    use UpgradeStepType::{Download, Restart, Start, Stop};

    let version = daemon.base.version.as_ref();
    let legacy_service_name = !installed_by_install_engine(version);
    let unix_download = UpgradeStep::new(Download, source.unix_install_script());

    let (linux, macos, freebsd, windows) = if legacy_service_name {
        let name = &daemon.base.name;
        let unix_service = quote_posix(&service_id(name));
        let windows_service = quote_powershell(&service_id(name));
        let unix =
            |restart: String| vec![unix_download.clone(), UpgradeStep::new(Restart, restart)];
        (
            unix(format!("sudo systemctl restart {unix_service}")),
            unix(format!(
                "sudo launchctl kickstart -k {}",
                quote_posix(&format!("system/{}", launchd_label(name)))
            )),
            unix(format!("sudo service {unix_service} restart")),
            // The running exe is locked, so it is stopped before being overwritten in place.
            vec![
                UpgradeStep::new(Stop, format!("sc.exe stop {windows_service}")),
                UpgradeStep::new(
                    Download,
                    source.windows_exe_download(&format!("{WINDOWS_BIN_DIR}\\scanopy-daemon.exe")),
                ),
                UpgradeStep::new(Start, format!("sc.exe start {windows_service}")),
            ],
        )
    } else {
        let instance = daemon.id.to_string();
        let unix = vec![
            unix_download,
            UpgradeStep::new(
                Restart,
                format!("sudo scanopy-daemon restart --instance {instance}"),
            ),
        ];
        let windows = vec![
            // Windows refuses to overwrite a running exe but lets it be renamed, and every daemon
            // on the host runs the same one. Download beside it, move the running copy aside, and
            // move the new one in. The retired copy gets a unique name: an earlier one can still be
            // running (another daemon not yet restarted), and an in-use file can't be replaced.
            // `restart` and `install` clear retired copies once released. Errors stop the chain,
            // so a failed download never moves the working exe away.
            UpgradeStep::new(
                Download,
                format!(
                    "$ErrorActionPreference = 'Stop'; \
                     {}; \
                     Move-Item \"{WINDOWS_BIN_DIR}\\scanopy-daemon.exe\" \"{WINDOWS_BIN_DIR}\\scanopy-daemon.$(Get-Date -Format yyyyMMddHHmmss).old\"; \
                     Move-Item \"{WINDOWS_BIN_DIR}\\scanopy-daemon.new\" \"{WINDOWS_BIN_DIR}\\scanopy-daemon.exe\"",
                    source.windows_exe_download(&format!("{WINDOWS_BIN_DIR}\\scanopy-daemon.new"))
                ),
            ),
            UpgradeStep::new(
                Restart,
                format!(
                    "& \"{WINDOWS_BIN_DIR}\\scanopy-daemon.exe\" restart --instance {instance}"
                ),
            ),
        ];
        (unix.clone(), unix.clone(), unix, windows)
    };

    UpgradeArtifacts {
        linux,
        macos,
        windows,
        freebsd,
        docker: DockerUpgrade {
            // `-n` never overwrites a config already on the volume.
            config_copy: (!persists_docker_config(version)).then(|| {
                format!(
                    "docker compose exec daemon sh -c 'mkdir -p {DOCKER_CONFIG_DIR} && cp -rn /root/.config/daemon/. {DOCKER_CONFIG_DIR}/'"
                )
            }),
            pull: "docker compose pull\ndocker compose up -d".to_string(),
            // On Scarf the swap-in moves a daemon still on `ghcr.io` onto Scarf, so it carries the
            // help comment for a pull that fails there.
            pinned_image: source
                .daemon_image_lines(&format!("v{}", own_version()))
                .join("\n"),
        },
        legacy_service_name,
    }
}

#[cfg(test)]
mod tests;
