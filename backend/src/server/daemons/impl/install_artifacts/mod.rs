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

use super::base::{Daemon, DaemonMode};
use crate::daemon::shared::config::{DOCKER_CONFIG_DIR, DaemonArgs};
use crate::server::credentials::r#impl::mapping::IntegrationTarget;

/// The `install.sh` one-liner that fetches + runs the Unix installer bootstrap.
const UNIX_INSTALL_SCRIPT: &str = "bash -c \"$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)\"";
/// Windows daemon exe (matches the UI's hardcoded release URL).
const WINDOWS_EXE_URL: &str =
    "https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.exe";
/// The signed Windows MSI release asset. A static GitHub asset URL — the UI hardcodes it as a
/// const rather than the server sending it per-provision (it's the same for every tenant). Kept
/// here for reuse by any server-side MSI tooling; only the per-daemon [`encode_msi_filename`]
/// name is tenant-specific and travels in the provision response.
pub const WINDOWS_MSI_URL: &str =
    "https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.msi";

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
/// `--server-url`), ServerPoll is dialed by the server (so it does not). Name and network never
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

/// Container image the compose file runs.
const DOCKER_IMAGE: &str = "ghcr.io/scanopy/scanopy/daemon:latest";

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
/// artifacts, so they cannot drift. Notably no network id, user id, name or mode: those are
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
fn docker_compose(env_lines: &[String], daemon: &Daemon) -> String {
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

    let mut lines = vec![
        "services:".to_string(),
        "  daemon:".to_string(),
        format!("    image: {DOCKER_IMAGE}"),
        "    container_name: scanopy-daemon".to_string(),
        "    network_mode: host".to_string(),
        "    privileged: true".to_string(),
        "    restart: unless-stopped".to_string(),
        "    environment:".to_string(),
    ];
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
                "{UNIX_INSTALL_SCRIPT} && sudo scanopy-daemon install {}",
                install_flags(&args, quote_posix)
            ),
            format!(
                "Invoke-WebRequest -Uri \"{WINDOWS_EXE_URL}\" -OutFile \"scanopy-daemon-windows-amd64.exe\"; .\\scanopy-daemon-windows-amd64.exe install {}",
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
        .then(|| docker_compose(&docker_env, daemon));

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

#[cfg(test)]
mod tests;
