//! `scanopy-daemon install` / `uninstall` / `list` — a single install engine shared across all
//! desktop operating systems.
//!
//! One config model, one engine. The **only** configuration input is the same connection/identity
//! flags the daemon already parses ([`DaemonArgs`](crate::daemon::shared::config::DaemonArgs)); the
//! installer feeds them straight to [`AppConfig::load`] and persists the result with
//! [`ConfigStore::persist`]. There is no blob and no second config store.
//!
//! Install performs three steps, in order:
//! 1. **Place the binary** — copy the running executable to the platform location.
//! 2. **Write `config.json`** — the full merged config, including secrets.
//! 3. **Register a system service** — systemd / launchd / Windows SCM.
//!
//! The registered service runs `<binary> --config-dir <dir> --log-file <file>` and reads everything
//! else from `config.json`, so credentials never land in a unit file, plist, or process arguments.
//!
//! ## Slots
//!
//! A host can run several daemons. Each install owns a **slot** — a config directory plus the
//! service id derived from it — allocated once and never renamed. The first install on a host takes
//! the default slot (`scanopy-daemon`, the paths every single-daemon host has always used);
//! additional installs take `scanopy-daemon-2`, `-3`, … A slot is deliberately *not* the daemon's
//! name: the name is assigned by the server and only known after the handshake (at which point the
//! daemon caches it, along with its server-assigned id, into the slot's `config.json`), and it can
//! be changed in the UI afterwards. Slots stay put; names are what we show and match on.
//!
//! Because the install command carries no identity, the installer resolves which slot a command
//! means: an explicit `--instance`, else the slot already holding that api key, else the only slot,
//! else it asks. Nothing here overwrites an existing daemon without a selector, a key match, or an
//! answer — a non-interactive shell allocates a new slot rather than guessing.
//!
//! Uninstall reverses install (service → config → binary) and is idempotent: removing a daemon that
//! was never installed succeeds with a "nothing to remove" message.

use anyhow::{Context, Result};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

use crate::daemon::shared::config::{
    AppConfig, ConfigStore, DaemonCommand, InstallArgs, UninstallArgs,
};
use crate::server::daemons::r#impl::base::DaemonMode;

#[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
mod bsd;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_family = "windows")]
mod windows;

mod prompt;
use prompt::{ambiguous_target_prompt, installed_summary};

#[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
use bsd as platform;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_family = "windows")]
use windows as platform;

/// The default slot; kept unnamespaced for backward compatibility (see
/// [`AppConfig::get_config_path_for_name`]).
const DEFAULT_NAME: &str = "scanopy-daemon";

#[cfg(target_family = "windows")]
const BINARY_FILE_NAME: &str = "scanopy-daemon.exe";
#[cfg(not(target_family = "windows"))]
const BINARY_FILE_NAME: &str = "scanopy-daemon";

/// Identity and paths for the system service. The service runs
/// `{bin_path} --config-dir {config_dir} --log-file {log_file}` and reads all other settings
/// (including secrets, and its own name) from `config.json`. The explicit `--config-dir`/`--log-file`
/// are what let the service (running under a different profile than the installer) find the config
/// and logs the installer wrote — without depending on the runtime `$HOME`/`%APPDATA%`.
pub struct ServiceSpec {
    /// Service/unit identifier, e.g. `scanopy-daemon` or `scanopy-daemon-2`.
    pub service_id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Installed binary path the service should execute.
    pub bin_path: std::path::PathBuf,
    /// The slot this service belongs to. Drives the launchd label and the service id.
    pub slot: String,
    /// System config directory to bake into the launch command (`--config-dir`).
    pub config_dir: std::path::PathBuf,
    /// System log file to bake into the launch command (`--log-file`).
    pub log_file: std::path::PathBuf,
}

/// One Scanopy daemon installed on this host, as discovered on disk.
#[derive(Debug, Clone)]
pub struct Installed {
    /// Config directory name — the slot. See the module docs.
    pub slot: String,
    /// The `config.json` this was discovered from.
    pub config_path: PathBuf,
    /// Server-assigned daemon name, cached into the config after the handshake. `None` until the
    /// daemon has successfully registered.
    pub name: Option<String>,
    /// Server-assigned daemon id, cached after the handshake. Nil/absent until then.
    pub daemon_id: Option<Uuid>,
    /// The api key this install holds, used to recognise a re-install of the same daemon.
    pub api_key: Option<String>,
    /// The port this install's daemon listens on. Every daemon runs an HTTP listener, so two on
    /// one host must not share a port.
    pub daemon_port: Option<u16>,
}

/// The fields of a daemon `config.json` the installer needs to identify an install. Deliberately
/// its own all-optional struct rather than [`AppConfig`]: a config written by an older version (or
/// half-written by an interrupted install) must still be discoverable, and deserializing the full
/// struct would fail on any missing required field.
#[derive(serde::Deserialize)]
struct InstalledConfig {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    id: Option<Uuid>,
    #[serde(default)]
    daemon_api_key: Option<String>,
    #[serde(default)]
    daemon_port: Option<u16>,
}

impl Installed {
    fn read(slot: String, config_path: PathBuf) -> Self {
        let parsed = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|c| serde_json::from_str::<InstalledConfig>(&c).ok());
        let (name, daemon_id, api_key, daemon_port) = match parsed {
            Some(c) => (
                c.name,
                c.id.filter(|id| !id.is_nil()),
                c.daemon_api_key,
                c.daemon_port,
            ),
            None => (None, None, None, None),
        };
        Self {
            slot,
            config_path,
            name,
            daemon_id,
            api_key,
            daemon_port,
        }
    }

    pub fn service_id(&self) -> String {
        service_id(&self.slot)
    }

    /// What to call this daemon in output: its server-assigned name once it has connected.
    ///
    /// Keyed on the cached id rather than the name, because an install that has not completed a
    /// handshake yet still carries the *default* name — reporting that would name a daemon the
    /// operator never chose, and read as though the wrong install were being touched.
    fn label(&self) -> String {
        match (&self.name, self.daemon_id) {
            (Some(name), Some(_)) => name.clone(),
            _ => format!("{} (not yet connected)", self.slot),
        }
    }

    /// Whether `selector` — a daemon name, slot, service id, or daemon id — names this install.
    fn matches(&self, selector: &str) -> bool {
        let selector = selector.trim();
        let eq = |candidate: &str| candidate.eq_ignore_ascii_case(selector);
        eq(&self.slot)
            || eq(&self.service_id())
            || self.name.as_deref().is_some_and(eq)
            || self
                .daemon_id
                .is_some_and(|id| eq(&id.to_string()) || eq(&id.simple().to_string()))
    }
}

/// Which slot an `install` command is for.
#[derive(Debug, PartialEq, Eq)]
enum InstallTarget {
    /// An install that already exists, by index into the discovered list.
    Existing(usize),
    /// A slot that does not exist yet.
    New(String),
}

/// The outcome of resolving an `install` command against what is already installed. `Ambiguous`
/// means the command could equally be a second daemon or a re-key of an existing one — only the
/// operator knows which, so the caller asks (or, with nowhere to ask, takes the non-destructive
/// route).
#[derive(Debug, PartialEq, Eq)]
enum Resolution {
    Resolved(InstallTarget),
    Ambiguous,
}

/// Decide which slot an install command targets, without touching the filesystem or the terminal.
///
/// Ordered so that the destructive outcome is never reached by inference: an explicit selector
/// wins, then an exact api-key match (the same daemon being re-installed or upgraded), then the
/// trivial case of a host with nothing installed. Anything left is genuinely ambiguous.
fn resolve_install_target(
    installed: &[Installed],
    selector: Option<&str>,
    name: Option<&str>,
    api_key: Option<&str>,
) -> Result<Resolution> {
    // A selector names the install to act on *if it is here*. It is not an assertion that it is:
    // the same command reinstalls a daemon onto a rebuilt host, where nothing matches yet and
    // refusing would leave the operator with a command they cannot run. So an unmatched selector
    // falls through to the ordinary resolution below (the caller reports it) rather than failing.
    if let Some(selector) = selector
        && let Some(index) = installed.iter().position(|i| i.matches(selector))
    {
        return Ok(Resolution::Resolved(InstallTarget::Existing(index)));
    }

    // Back-compat: installs predating slots took their `--name` as the slot. While a name like
    // that still identifies an install here, a command carrying it means *that* install — without
    // this, a provisioning script that has always passed `--name` would stack up a new slot every
    // time its key was rotated. `--name` is not a selector for anything else: a name matching no
    // slot (any first install, since the command no longer carries one) just falls through.
    if let Some(name) = name
        && let Some(index) = installed.iter().position(|i| i.slot == name)
    {
        return Ok(Resolution::Resolved(InstallTarget::Existing(index)));
    }

    if let Some(key) = api_key.filter(|k| !k.is_empty())
        && let Some(index) = installed
            .iter()
            .position(|i| i.api_key.as_deref() == Some(key))
    {
        return Ok(Resolution::Resolved(InstallTarget::Existing(index)));
    }

    if installed.is_empty() {
        return Ok(Resolution::Resolved(InstallTarget::New(
            DEFAULT_NAME.to_string(),
        )));
    }

    Ok(Resolution::Ambiguous)
}

/// The next unused slot: the default one if free, else `scanopy-daemon-2`, `-3`, …
fn next_free_slot(installed: &[Installed]) -> String {
    let taken = |slot: &str| installed.iter().any(|i| i.slot == slot);
    if !taken(DEFAULT_NAME) {
        return DEFAULT_NAME.to_string();
    }
    (2..)
        .map(|n| format!("{DEFAULT_NAME}-{n}"))
        .find(|slot| !taken(slot))
        .expect("an unused slot always exists")
}

/// The port another install on this host has already claimed, if any.
///
/// Every daemon runs an HTTP listener, and the default bind is `0.0.0.0`, so the first daemon on a
/// host claims its port on every address — a second one binding the same port exits with "Address
/// already in use" and its service crash-loops. The install itself succeeds, so the failure only
/// shows up as a daemon that never connects.
fn port_conflict(installed: &[Installed], slot: &str, port: u16) -> bool {
    installed
        .iter()
        .any(|i| i.slot != slot && i.daemon_port == Some(port))
}

/// The lowest port at or above the daemon default that no other install has claimed.
fn next_free_port(installed: &[Installed], slot: &str) -> u16 {
    let default = AppConfig::default().daemon_port;
    (default..)
        .find(|port| !port_conflict(installed, slot, *port))
        .expect("a free port always exists below u16::MAX")
}

/// A trailing summary of what is installed, for error messages. Empty when nothing is.
fn describe_installed(installed: &[Installed]) -> String {
    if installed.is_empty() {
        return " No Scanopy daemons are installed here.".to_string();
    }
    let mut out = String::from("\nInstalled on this host:");
    for entry in installed {
        out.push_str(&format!(
            "\n  {} (service {})",
            entry.label(),
            entry.service_id()
        ));
    }
    out.push_str("\nRun `scanopy-daemon list` for details.");
    out
}

/// Dispatch an `install`/`uninstall`/`list` subcommand.
pub async fn run_command(command: DaemonCommand) -> Result<()> {
    match command {
        DaemonCommand::Install(args) => run_install(args).await,
        DaemonCommand::Uninstall(args) => run_uninstall(args).await,
        DaemonCommand::List => run_list(),
    }
}

fn run_list() -> Result<()> {
    let installed = installed_daemons();
    if installed.is_empty() {
        println!("No Scanopy daemons are installed on this host.");
        return Ok(());
    }

    println!("Scanopy daemons installed on this host:\n");
    for entry in &installed {
        println!("  {}", entry.label());
        println!("    Service: {}", entry.service_id());
        println!("    Config:  {}", entry.config_path.display());
        if let Some(id) = entry.daemon_id {
            println!("    ID:      {id}");
        }
    }
    println!(
        "\nTarget one with `install --instance <name>` or `uninstall --name <name>`.\
         \nA name, service id, or daemon id all work."
    );
    Ok(())
}

async fn run_install(args: InstallArgs) -> Result<()> {
    require_elevation("install")?;

    let InstallArgs {
        args: mut daemon_args,
        no_service,
        bin_dir,
    } = args;

    // Which daemon on this host is this command for? Resolved before anything is read or written,
    // since every path below hangs off the answer.
    let selector = daemon_args.instance.take();
    let installed = installed_daemons();
    if let Some(selector) = selector.as_deref()
        && !installed.iter().any(|i| i.matches(selector))
    {
        println!(
            "No daemon matching '{selector}' is installed here yet — continuing as a fresh install.{}",
            describe_installed(&installed)
        );
    }
    let slot = match resolve_install_target(
        &installed,
        selector.as_deref(),
        daemon_args.name.as_deref(),
        daemon_args.daemon_api_key.as_deref(),
    )? {
        Resolution::Resolved(InstallTarget::Existing(index)) => installed[index].slot.clone(),
        Resolution::Resolved(InstallTarget::New(slot)) => slot,
        Resolution::Ambiguous => choose_ambiguous_target(&installed)?,
    };

    // Whether the operator pinned a port. A ServerPoll command always carries one (the server
    // dials that exact port), so it must never be second-guessed below.
    let port_was_requested = daemon_args.daemon_port.is_some();

    // Point the config load at the slot's own config.json, so re-running install against an
    // existing daemon (the reconfigure command) layers over what is already there instead of
    // rebuilding from defaults — which would blank its api key and cached identity. An explicit
    // --config-dir still wins.
    if daemon_args.config_dir.is_none() {
        daemon_args.config_dir = Some(if no_service {
            profile_config_dir(&slot)?
        } else {
            system_config_dir(&slot)
        });
    }
    let config_dir = daemon_args
        .config_dir
        .clone()
        .expect("config_dir was just set");

    // Same config layering the daemon itself uses — single source of truth.
    let mut config = AppConfig::load(daemon_args)?;

    // Keep this daemon off a port another one on this host already listens on. The install command
    // carries no port (identity and connectivity come from the key and the server url), so without
    // this a second daemon installs cleanly and then crash-loops on "Address already in use",
    // visible only as a daemon that never connects. A port the operator asked for is left alone —
    // for ServerPoll it is the port the server dials, so moving it would break reachability.
    if !port_was_requested && port_conflict(&installed, &slot, config.daemon_port) {
        config.daemon_port = next_free_port(&installed, &slot);
        println!(
            "Port {} is already used by another daemon on this host; this one will listen on {}.",
            AppConfig::default().daemon_port,
            config.daemon_port
        );
    }

    // Only DaemonPoll dials out and needs a server URL. ServerPoll is dialed by the
    // server (installed with just --daemon-api-key), so it must not require one. Mode is
    // already resolved (inferred from server_url presence) by AppConfig::load above.
    if config.mode == DaemonMode::DaemonPoll
        && config.server_url.is_none()
        && config.server_target.is_none()
    {
        anyhow::bail!(
            "Missing --server-url: a DaemonPoll daemon needs a server URL to connect to. \
             Re-run: scanopy-daemon install --server-url <url> --daemon-api-key <key>"
        );
    }

    if let Some(existing) = installed.iter().find(|i| i.slot == slot) {
        println!("Reconfiguring installed daemon '{}'.", existing.label());
    }

    // 1. Place the binary.
    let bin_dir = bin_dir.unwrap_or_else(platform::default_bin_dir);
    let bin_path = bin_dir.join(BINARY_FILE_NAME);
    place_binary(&bin_path)?;

    // 2. Write config.json. A registered service runs under a different profile than this
    //    installer, so its config must live at a fixed *system* path it can always resolve — not
    //    the per-user $HOME/%APPDATA% path. `--no-service` installs keep the per-user path since
    //    the user runs the daemon themselves.
    let config_path = config_dir.join("config.json");
    let store = ConfigStore::new(config_path.clone(), config.clone());
    store
        .persist()
        .await
        .context("Failed to write daemon config")?;
    println!("Wrote config to {}", config_path.display());

    // 3. Register the service. The launch command carries --config-dir/--log-file so the service
    //    reads exactly what we just wrote, independent of its runtime profile.
    let spec = ServiceSpec {
        service_id: service_id(&slot),
        display_name: display_name(&slot),
        bin_path: bin_path.clone(),
        slot: slot.clone(),
        config_dir,
        log_file: AppConfig::default_system_log_path(&slot),
    };

    if no_service {
        println!(
            "Skipping service registration (--no-service). Start the daemon manually with:\n  {} --config-dir {}",
            bin_path.display(),
            spec.config_dir.display()
        );
    } else {
        platform::register_service(&spec).context("Failed to register the system service")?;
        println!("Registered and started service '{}'.", spec.service_id);
        // Tell the operator where to find the config and logs — the first thing they need
        // when a freshly installed service isn't behaving.
        println!("  Config: {}", config_path.display());
        println!("  Logs:   {}", spec.log_file.display());
    }

    println!("Scanopy daemon installed successfully.");
    Ok(())
}

/// Ask which daemon an ambiguous install command is for: an additional daemon alongside the
/// existing install(s), or a replacement for one of them. With no terminal to ask on, take the
/// only non-destructive option — a new slot — and say so, since the alternative would silently
/// overwrite a working daemon's credentials.
fn choose_ambiguous_target(installed: &[Installed]) -> Result<String> {
    if !std::io::stdin().is_terminal() {
        let slot = next_free_slot(installed);
        println!(
            "{}\n\nThe API key in this command is for a different daemon, so it is installed as an \
             additional daemon in slot '{slot}'.\n\
             To replace an existing daemon instead, re-run the command with \
             `--instance <name>` (see `scanopy-daemon list`).",
            installed_summary(installed).trim_end()
        );
        return Ok(slot);
    }

    print!("{}", ambiguous_target_prompt(installed));
    std::io::stdout().flush().ok();

    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("Failed to read your choice")?;
    let answer = answer.trim();

    if answer.is_empty() || answer.eq_ignore_ascii_case("n") {
        return Ok(next_free_slot(installed));
    }
    let choice: usize = answer
        .parse()
        .ok()
        .filter(|c| (1..=installed.len()).contains(c))
        .with_context(|| {
            format!("'{answer}' is not one of the choices. Type n or a number from the list above.")
        })?;
    Ok(installed[choice - 1].slot.clone())
}

/// Print a uniform per-item disposition line for uninstall, e.g.
/// `Config at /etc/…/config.json: Deleted` or
/// `Binary at /usr/local/bin/scanopy-daemon: Kept (re-run with --purge to delete)`.
fn report_disposition(label: &str, deleted: bool) {
    if deleted {
        println!("{label}: Deleted");
    } else {
        println!("{label}: Kept (re-run with --purge to delete)");
    }
}

async fn run_uninstall(args: UninstallArgs) -> Result<()> {
    require_elevation("uninstall")?;

    let installed = installed_daemons();
    let targets = uninstall_targets(&installed, &args)?;

    // `removed_anything`: something was actually deregistered/deleted.
    // `found_anything`: an artifact was present (even if kept). These drive the
    // final summary so a plain uninstall that only finds a kept binary/log
    // doesn't claim "no install found".
    // A selector naming nothing discoverable still gets its paths swept (a half-removed install,
    // service registered but config already gone, is only reachable that way). But if that turns
    // up nothing either, the operator named a daemon that simply isn't here — say so, and say what
    // is, rather than letting the generic summary below suggest `--purge` at them.
    let unmatched_selector = args
        .name
        .as_deref()
        .filter(|s| !installed.iter().any(|i| i.matches(s)));

    let mut removed_anything = false;
    let mut found_anything = false;
    for slot in &targets {
        let (removed, found) = remove_slot(slot, args.purge)?;
        removed_anything |= removed;
        found_anything |= found;
    }

    if let Some(selector) = unmatched_selector
        && !found_anything
    {
        println!(
            "No daemon matching '{selector}' is installed on this host.{}",
            describe_installed(&installed)
        );
        return Ok(());
    }

    // The binary is shared by every daemon on the host, so it can only go once nothing is left to
    // run it. Kept by default in any case; deleted only under --purge.
    let bin_path = platform::default_bin_dir().join(BINARY_FILE_NAME);
    if bin_path.exists() {
        found_anything = true;
        let remaining = installed_daemons();
        if args.purge && remaining.is_empty() {
            let retired = bin_path.with_extension("old");
            match remove_file_when_released(&bin_path) {
                Ok(()) => {
                    removed_anything = true;
                    report_disposition(&format!("Binary at {}", bin_path.display()), true);
                }
                // Windows cannot delete the image a process is executing — and on Windows that
                // process is usually this uninstaller, run from the very path being purged. It
                // does allow the file to be *renamed*, so retire it instead of failing an
                // uninstall that has already removed the service and config. `place_binary` and
                // the sweep below clear a retired binary once nothing holds it.
                Err(_) => {
                    let _ = remove_file_when_released(&retired);
                    std::fs::rename(&bin_path, &retired).with_context(|| {
                        format!("Failed to delete binary {}", bin_path.display())
                    })?;
                    removed_anything = true;
                    println!(
                        "Binary at {}: Removed (retired to {}, which this running uninstaller \
                         cannot delete; the next install clears it)",
                        bin_path.display(),
                        retired.display()
                    );
                }
            }
            // A binary retired by an earlier install goes too, once nothing runs it.
            if retired.exists() && remove_file_when_released(&retired).is_err() {
                println!(
                    "Retired binary at {}: still in use — safe to delete once that process exits.",
                    retired.display()
                );
            }
        } else if args.purge {
            println!(
                "Binary at {}: Kept ({} other daemon(s) still installed)",
                bin_path.display(),
                remaining.len()
            );
        } else {
            report_disposition(&format!("Binary at {}", bin_path.display()), false);
        }
    }

    if removed_anything {
        println!("Scanopy daemon uninstalled.");
    } else if found_anything {
        println!("Nothing removed — re-run with --purge to delete the kept file(s).");
    } else {
        println!("Nothing to remove — no Scanopy daemon install found.");
    }
    Ok(())
}

/// Which slots an `uninstall` invocation should remove.
///
/// A selector that matches nothing discoverable still resolves to itself, so a half-removed
/// install (service registered, config already deleted) stays removable by name. With no selector
/// and nothing installed, the default slot is returned so the command reports on the standard
/// paths rather than claiming there was nothing to look at.
fn uninstall_targets(installed: &[Installed], args: &UninstallArgs) -> Result<Vec<String>> {
    if args.all {
        return Ok(installed.iter().map(|i| i.slot.clone()).collect());
    }

    if let Some(selector) = &args.name {
        return Ok(vec![
            installed
                .iter()
                .find(|i| i.matches(selector))
                .map(|i| i.slot.clone())
                .unwrap_or_else(|| selector.clone()),
        ]);
    }

    match installed.len() {
        0 => Ok(vec![DEFAULT_NAME.to_string()]),
        1 => Ok(vec![installed[0].slot.clone()]),
        _ => Ok(vec![choose_uninstall_target(installed)?]),
    }
}

/// Ask which daemon to remove when several are installed. Never guesses: with no terminal to ask
/// on, it lists them and stops.
fn choose_uninstall_target(installed: &[Installed]) -> Result<String> {
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "This host has {} Scanopy daemons installed — say which to remove with \
             `--name <name>`, or remove them all with `--all`.{}",
            installed.len(),
            describe_installed(installed)
        );
    }

    println!("This host has several Scanopy daemons installed:\n");
    for (index, entry) in installed.iter().enumerate() {
        println!(
            "  [{}] {} (service {})",
            index + 1,
            entry.label(),
            entry.service_id()
        );
    }
    print!("\nWhich do you want to remove? [1-{}] ", installed.len());
    std::io::stdout().flush().ok();

    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("Failed to read your choice")?;
    let answer = answer.trim();
    let choice: usize = answer
        .parse()
        .ok()
        .filter(|c| (1..=installed.len()).contains(c))
        .with_context(|| {
            format!(
                "'{answer}' is not one of the choices (1-{})",
                installed.len()
            )
        })?;
    Ok(installed[choice - 1].slot.clone())
}

/// Remove one slot's service, config and logs. Returns `(removed_anything, found_anything)`.
fn remove_slot(slot: &str, purge: bool) -> Result<(bool, bool)> {
    let mut removed_anything = false;
    let mut found_anything = false;

    // 1. Stop + deregister the service (tolerant of an already-absent service).
    //    Always removed in both modes.
    let config_dir = system_config_dir(slot);
    let spec = ServiceSpec {
        service_id: service_id(slot),
        display_name: display_name(slot),
        bin_path: platform::default_bin_dir().join(BINARY_FILE_NAME),
        slot: slot.to_string(),
        config_dir: config_dir.clone(),
        log_file: AppConfig::default_system_log_path(slot),
    };
    if platform::deregister_service(&spec).context("Failed to remove the system service")? {
        removed_anything = true;
        found_anything = true;
        println!("Service '{}': Removed", spec.service_id);
    } else {
        println!("Service '{}': Not found", spec.service_id);
    }

    // 2. Remove config.json from both the system location (service installs) and the per-user
    //    profile location (--no-service / manual installs). Always removed in both modes.
    let system_cfg = config_dir.join("config.json");
    let profile_cfg = AppConfig::get_config_path_for_name(Some(slot), None)?.1;
    for config_path in [system_cfg, profile_cfg] {
        if config_path.exists() {
            std::fs::remove_file(&config_path)
                .with_context(|| format!("Failed to delete config {}", config_path.display()))?;
            removed_anything = true;
            found_anything = true;
            report_disposition(&format!("Config at {}", config_path.display()), true);
            // Take the slot directory with it once it holds nothing else, so removing a daemon
            // doesn't leave its slot behind as an empty husk. `remove_dir` only succeeds on an
            // empty directory, which is exactly the condition wanted — anything the operator put
            // there keeps the directory.
            if let Some(slot_dir) = config_path.parent() {
                let _ = std::fs::remove_dir(slot_dir);
            }
        }
    }

    // 3. Log files. The service logs to a known, installer-baked path
    //    (`--log-file`, always `default_system_log_path` == `spec.log_file`);
    //    macOS also has the launchd stdout/stderr capture. Kept by default
    //    (useful for a post-mortem), deleted only under --purge.
    let log_files: Vec<std::path::PathBuf> = [
        spec.log_file.clone(),
        // macOS also captures the launchd stdout log; cfg'd on the element so the
        // list is single-entry on other platforms without an unused `mut`/`vec!`.
        #[cfg(target_os = "macos")]
        std::path::PathBuf::from("/var/log/scanopy").join(format!("{}.out.log", spec.service_id)),
    ]
    .into_iter()
    .collect();
    for log_path in log_files.iter().filter(|p| p.exists()) {
        found_anything = true;
        if purge {
            std::fs::remove_file(log_path)
                .with_context(|| format!("Failed to delete log {}", log_path.display()))?;
            removed_anything = true;
        }
        report_disposition(&format!("Log at {}", log_path.display()), purge);
    }

    Ok((removed_anything, found_anything))
}

/// Delete a file that a service may only just have released.
///
/// Deregistering a service asks the platform to stop it and returns before it necessarily has:
/// `sc stop` is asynchronous. Windows refuses to delete an image while a process still runs it, so
/// removing the binary immediately after can fail purely on timing — a race that would otherwise
/// fail the whole uninstall. Retry briefly rather than reporting a failure the user can fix by
/// running the same command again a second later. A file that is already gone is a success.
fn remove_file_when_released(path: &Path) -> std::io::Result<()> {
    const ATTEMPTS: u32 = 10;
    for attempt in 0..ATTEMPTS {
        match std::fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) if attempt + 1 < ATTEMPTS => {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("the loop returns on its final attempt")
}

/// Copy the running executable to `dest`, with an already-in-place fast path. Idempotent.
///
/// The binary is staged beside `dest` and renamed into place rather than copied over it. Every
/// daemon on a host shares one binary, so by the time a second one is installed — or an existing
/// one is upgraded from a freshly downloaded copy — `dest` is the image a running service is
/// executing. Writing to it directly fails there: `ETXTBSY` on Unix, a sharing violation on
/// Windows. Renaming swaps the directory entry instead, which both platforms allow: the running
/// daemon keeps the inode it started from and picks the new binary up when it next restarts.
fn place_binary(dest: &Path) -> Result<()> {
    let src = std::env::current_exe().context("Failed to locate the running executable")?;

    if let (Ok(s), Ok(d)) = (src.canonicalize(), dest.canonicalize())
        && s == d
    {
        println!("Binary already in place at {}.", dest.display());
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    // Stage in the destination directory, so the rename below is same-filesystem (and therefore
    // atomic) rather than a cross-device copy.
    let staged = dest.with_extension("new");
    std::fs::copy(&src, &staged)
        .with_context(|| format!("Failed to copy binary to {}", staged.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("Failed to set permissions on {}", staged.display()))?;
    }

    // Windows refuses to *replace* a file that is currently executing, but it will let it be
    // renamed out of the way first. The retired copy can only be deleted once nothing is running
    // it, so both the removal here and the one below are best-effort.
    #[cfg(target_family = "windows")]
    let retired = dest.with_extension("old");
    #[cfg(target_family = "windows")]
    if dest.exists() {
        let _ = std::fs::remove_file(&retired);
        std::fs::rename(dest, &retired).with_context(|| {
            format!(
                "Failed to move the running binary aside at {}",
                dest.display()
            )
        })?;
    }

    std::fs::rename(&staged, dest)
        .with_context(|| format!("Failed to move binary into place at {}", dest.display()))?;

    #[cfg(target_family = "windows")]
    let _ = std::fs::remove_file(&retired);

    println!("Installed binary to {}.", dest.display());
    Ok(())
}

/// Bail with the platform-specific elevated re-invocation hint when not running as root/admin.
fn require_elevation(action: &str) -> Result<()> {
    if platform::is_elevated() {
        return Ok(());
    }
    eprintln!("{}", platform::elevation_hint());
    anyhow::bail!("Insufficient privileges: re-run the {action} command with elevated permissions")
}

/// Service/unit identifier for a slot. The default slot keeps its bare id for backward
/// compatibility; every other slot is namespaced under the `scanopy-daemon-` prefix — which the
/// allocated `scanopy-daemon-2`, `-3`, … already carry, so they are their own service id.
fn service_id(slot: &str) -> String {
    if slot == DEFAULT_NAME || is_numbered(slot) {
        slot.to_string()
    } else {
        format!("scanopy-daemon-{slot}")
    }
}

/// Whether `slot` is one of the installer-allocated `scanopy-daemon-<n>` slots. A pre-slot install
/// named after its daemon (`scanopy-daemon-edge`) is *not* one, and keeps the doubled service id it
/// was registered under, so it stays removable.
fn is_numbered(slot: &str) -> bool {
    slot.strip_prefix(&format!("{DEFAULT_NAME}-"))
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
}

fn display_name(slot: &str) -> String {
    if slot == DEFAULT_NAME {
        "Scanopy Daemon".to_string()
    } else {
        let suffix = slot
            .strip_prefix(&format!("{DEFAULT_NAME}-"))
            .unwrap_or(slot);
        format!("Scanopy Daemon ({suffix})")
    }
}

/// The system config directory for a slot — a fixed, profile-independent location
/// (`{default_system_config_dir}/{slot}`) that a system service can always resolve. Baked into the
/// service via `--config-dir`, this replaces the old `$HOME`-pinning approach and works uniformly
/// across systemd/launchd/rc.d and the Windows LocalSystem service (which has no `$HOME` lever).
fn system_config_dir(slot: &str) -> std::path::PathBuf {
    AppConfig::default_system_config_dir().join(slot)
}

/// The per-user config directory for a slot, used by `--no-service` installs (the user runs the
/// daemon themselves, under their own profile).
fn profile_config_dir(slot: &str) -> Result<PathBuf> {
    let (_, path) = AppConfig::get_config_path_for_name(Some(slot), None)?;
    path.parent()
        .map(Path::to_path_buf)
        .context("Failed to determine the per-user config directory")
}

/// Every Scanopy daemon installed on this host, ordered with the default slot first.
///
/// Both bases are scanned: the system one that service installs use, and the per-user one that
/// `--no-service` and manual installs use. A slot present in both is reported once (the system
/// config wins, being the one a registered service actually reads).
fn installed_daemons() -> Vec<Installed> {
    let mut found: Vec<Installed> = Vec::new();
    let profile_base = AppConfig::get_config_path_for_name(None, None)
        .ok()
        .and_then(|(_, path)| path.parent().map(Path::to_path_buf));

    for base in [Some(AppConfig::default_system_config_dir()), profile_base]
        .into_iter()
        .flatten()
    {
        for entry in installed_in(&base) {
            if !found.iter().any(|f| f.slot == entry.slot) {
                found.push(entry);
            }
        }
    }

    found.sort_by_key(|entry| (entry.slot != DEFAULT_NAME, entry.slot.clone()));
    found
}

/// The installs discoverable under one config base: `<base>/<slot>/config.json`, plus the
/// un-namespaced `<base>/config.json` that the default slot has always used in a user profile.
fn installed_in(base: &Path) -> Vec<Installed> {
    let mut found = Vec::new();

    let default_config = base.join("config.json");
    if default_config.is_file() {
        found.push(Installed::read(DEFAULT_NAME.to_string(), default_config));
    }

    let Ok(entries) = std::fs::read_dir(base) else {
        return found;
    };
    for entry in entries.flatten() {
        let config_path = entry.path().join("config.json");
        if !config_path.is_file() {
            continue;
        }
        let Some(slot) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !found.iter().any(|f: &Installed| f.slot == slot) {
            found.push(Installed::read(slot, config_path));
        }
    }

    found
}

/// The current command line, re-quoted well enough to paste after `sudo`.
fn current_invocation() -> String {
    std::env::args()
        .map(|a| {
            if a.is_empty() || a.contains(char::is_whitespace) {
                format!("\"{a}\"")
            } else {
                a
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Run a command and fail if it exits non-zero. Shared by the platform service modules.
fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute `{cmd}`"))?;
    if !status.success() {
        anyhow::bail!("`{cmd} {}` failed ({status})", args.join(" "));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
