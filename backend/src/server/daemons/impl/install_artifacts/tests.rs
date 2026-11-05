use super::*;
use crate::daemon::shared::config::{DaemonCli, DaemonCommand};
use crate::server::daemons::r#impl::base::DaemonBase;
use crate::server::shared::storage::traits::Storable;

fn daemon(mode: DaemonMode, url: &str) -> Daemon {
    Daemon::new(DaemonBase {
        host_id: uuid::Uuid::new_v4(),
        site_id: uuid::Uuid::new_v4(),
        url: url.to_string(),
        last_seen: None,
        mode,
        name: "edge-01".to_string(),
        tags: Vec::new(),
        version: None,
        user_id: uuid::Uuid::new_v4(),
        api_key_id: None,
        is_unreachable: false,
        standby: false,
        standby_cleared_at: None,
        os: None,
    })
}

/// Split a POSIX command the way a shell would, honouring the single-quoting
/// [`quote_posix`] applies, so a round-trip test exercises the real emitted string.
fn shell_split(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut has_token = false;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_quotes = !in_quotes;
                has_token = true;
            }
            '\\' if in_quotes => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            c if c.is_whitespace() && !in_quotes => {
                if has_token {
                    tokens.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            c => {
                current.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        tokens.push(current);
    }
    tokens
}

/// Parse the `install` half of an emitted unix command back through the daemon's own clap
/// parser, so assertions are about what the daemon would actually receive.
fn parse_unix_install(artifacts: &InstallArtifacts) -> DaemonArgs {
    use clap::Parser;

    // An install prefixes the bootstrap with `&& sudo `; a reconfigure has no bootstrap.
    let install = artifacts
        .linux
        .split("&& sudo ")
        .nth(1)
        .unwrap_or(&artifacts.linux)
        .trim_start_matches("sudo ");
    let parsed = DaemonCli::parse_from(shell_split(install));
    let Some(DaemonCommand::Install(args)) = parsed.command else {
        panic!("expected an install subcommand, got {install}");
    };
    args.args
}

/// Re-keying an installed daemon must change only its credential. Server-side record edits
/// The builder never mints, so an install command carries the key *placeholder*, not a real
/// key — the frontend substitutes the plaintext it holds. It must never contain a literal
/// key value.
#[test]
fn install_command_carries_the_key_placeholder() {
    let args = parse_unix_install(&build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::DaemonPoll, ""),
        None,
        &[],
        InstallCommandType::Install,
        DownloadSource::Scarf,
    ));
    assert_eq!(args.daemon_api_key.as_deref(), Some(API_KEY_PLACEHOLDER));
}

/// The reconfigure command is displayed persistently, so it must never carry a credential —
/// not even the placeholder — leaving the daemon's existing key in place. It re-asserts the
/// ServerPoll port (which the server dials) and omits `--name`.
#[test]
fn reconfigure_command_reasserts_connectivity_without_a_credential() {
    let sp = parse_unix_install(&build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::ServerPoll, "https://edge.corp:60074"),
        None,
        &[],
        InstallCommandType::Reconfigure,
        DownloadSource::Scarf,
    ));
    assert_eq!(sp.daemon_api_key, None);
    assert_eq!(sp.name, None);
    assert_eq!(sp.daemon_port, Some(60074));

    let dp = parse_unix_install(&build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::DaemonPoll, ""),
        None,
        &[],
        InstallCommandType::Reconfigure,
        DownloadSource::Scarf,
    ));
    assert_eq!(dp.daemon_api_key, None);
    assert_eq!(dp.server_url.as_deref(), Some("https://app.scanopy.net"));
    assert_eq!(dp.daemon_port, None);
}

/// A host can run several daemons, and no command carries a name, so a command that acts on an
/// *existing* install has to say which one — by daemon id, the one selector that is neither
/// secret nor editable. A reconfigure, and a re-key of a daemon that has already connected, both
/// resolve to exactly one install. A re-key of a daemon that never connected has nothing to
/// select.
#[test]
fn commands_targeting_an_existing_install_carry_a_selector_for_it() {
    let fresh = daemon(DaemonMode::DaemonPoll, "");
    let mut connected = daemon(DaemonMode::DaemonPoll, "");
    connected.base.last_seen = Some(crate::server::shared::clock::now());

    let target = |d: &Daemon, command_type| {
        parse_unix_install(&build_install_artifacts(
            "https://app.scanopy.net",
            d,
            None,
            &[],
            command_type,
            DownloadSource::Scarf,
        ))
        .instance
    };

    assert_eq!(target(&fresh, InstallCommandType::Rekey), None);
    assert_eq!(
        target(&connected, InstallCommandType::Rekey),
        Some(connected.id.to_string())
    );
    assert_eq!(
        target(&fresh, InstallCommandType::Reconfigure),
        Some(fresh.id.to_string())
    );
}

/// Every flag and env var an install artifact may carry. Identity (site, user, name, mode)
/// and the `--instance` selector are deliberately absent: the server assigns identity at
/// provision, and a first install has no existing install to select.
fn allowed_install_settings(mode: DaemonMode) -> Vec<&'static str> {
    let mut allowed = vec!["--daemon-api-key", "SCANOPY_DAEMON_API_KEY"];
    match mode {
        DaemonMode::DaemonPoll => allowed.extend(["--server-url", "SCANOPY_SERVER_URL"]),
        DaemonMode::ServerPoll => allowed.extend(["--daemon-port", "SCANOPY_DAEMON_PORT"]),
    }
    // The Advanced panel's settings, present only when the user changed them.
    allowed.extend([
        "--log-level",
        "SCANOPY_LOG_LEVEL",
        "--log-file",
        "SCANOPY_LOG_FILE",
        "--heartbeat-interval",
        "SCANOPY_HEARTBEAT_INTERVAL",
        "--bind-address",
        "SCANOPY_BIND_ADDRESS",
        "--interfaces",
        "SCANOPY_INTERFACES",
        "--allow-self-signed-certs",
        "SCANOPY_ALLOW_SELF_SIGNED_CERTS",
        "--accept-invalid-scan-certs",
        "SCANOPY_ACCEPT_INVALID_SCAN_CERTS",
    ]);
    allowed
}

/// The `--flag` tokens and `SCANOPY_*` keys a command or compose file sets.
fn settings_in(text: &str) -> Vec<String> {
    // Only what follows the `install` subcommand configures the daemon; the bootstrap before it
    // passes its own arguments to install.sh.
    let text = text.rsplit(" install ").next().unwrap_or(text);
    text.split(|c: char| c.is_whitespace() || c == '"')
        .filter_map(|token| {
            if token.starts_with("--") {
                Some(token.to_string())
            } else {
                token
                    .find("SCANOPY_")
                    .and_then(|start| token[start..].split('=').next())
                    .map(str::to_string)
            }
        })
        .collect()
}

fn install_daemons() -> Vec<Daemon> {
    let mut daemons = Vec::new();
    for (mode, url) in [
        (DaemonMode::DaemonPoll, ""),
        (DaemonMode::ServerPoll, "https://edge.corp:60074"),
    ] {
        let fresh = daemon(mode, url);
        let mut connected = fresh.clone();
        connected.base.last_seen = Some(crate::server::shared::clock::now());
        daemons.extend([fresh, connected]);
    }
    daemons
}

fn changed_advanced_config() -> DaemonArgs {
    DaemonArgs {
        log_level: Some("debug".to_string()),
        log_file: Some("/var/log/d.log".to_string()),
        heartbeat_interval: Some(45),
        bind_address: Some("10.0.0.5".to_string()),
        interfaces: Some(vec!["eth0".to_string()]),
        allow_self_signed_certs: Some(true),
        accept_invalid_scan_certs: Some(false),
        ..Default::default()
    }
}

/// The create wizard's install command carries the key, the connectivity its mode needs, and
/// the Advanced settings the user changed. Nothing else, for any mode, install method, or
/// connection state.
#[test]
fn install_artifacts_carry_only_allowed_settings() {
    let advanced = changed_advanced_config();
    for d in install_daemons() {
        for config in [None, Some(&advanced)] {
            let a = build_install_artifacts(
                "https://app.scanopy.net",
                &d,
                config,
                &[],
                InstallCommandType::Install,
                DownloadSource::Scarf,
            );
            let allowed = allowed_install_settings(d.base.mode);
            for (method, text) in [
                ("linux", a.linux.clone()),
                ("macos", a.macos.clone()),
                ("windows", a.windows.clone()),
                ("freebsd", a.freebsd.clone()),
                ("docker env", a.docker.env.join("\n")),
                (
                    "docker compose",
                    a.docker.compose.clone().unwrap_or_default(),
                ),
            ] {
                for setting in settings_in(&text) {
                    assert!(
                        allowed.contains(&setting.as_str()),
                        "{method} install for a {:?} daemon carries {setting}: {text}",
                        d.base.mode
                    );
                }
            }
        }
    }
}

/// The wizard refetches the install command after the daemon's first handshake (window focus,
/// daemon list invalidation). The refetched command must be the one already on screen.
#[test]
fn install_command_is_unchanged_once_the_daemon_connects() {
    for mode_daemons in install_daemons().chunks(2) {
        let [fresh, connected] = mode_daemons else {
            unreachable!()
        };
        let build = |d: &Daemon| {
            build_install_artifacts(
                "https://app.scanopy.net",
                d,
                None,
                &[],
                InstallCommandType::Install,
                DownloadSource::Scarf,
            )
        };
        let (before, after) = (build(fresh), build(connected));
        assert_eq!(before.linux, after.linux);
        assert_eq!(before.windows, after.windows);
        assert_eq!(before.docker.compose, after.docker.compose);
    }
}

/// A reconfigure runs against an installed daemon, so it must not re-download the binary.
#[test]
fn reconfigure_command_does_not_refetch_the_binary() {
    let artifacts = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::ServerPoll, "https://edge.corp:60074"),
        None,
        &[],
        InstallCommandType::Reconfigure,
        DownloadSource::Scarf,
    );
    for (platform, command) in [
        ("linux", &artifacts.linux),
        ("macos", &artifacts.macos),
        ("windows", &artifacts.windows),
        ("freebsd", &artifacts.freebsd),
    ] {
        assert!(
            !command.contains("install.sh") && !command.contains("Invoke-WebRequest"),
            "{platform} command re-fetches the binary: {command}"
        );
    }
}

/// The docker artifacts must not assert identity. Those fields are `#[serde(skip)]` on
/// DaemonArgs precisely so a client cannot set them, and the binary command dropped them for
/// the same reason — identity comes from the 1:1 key binding. A compose that carried them
/// could disagree with the record its key is bound to.
#[test]
fn docker_install_yields_a_compose_carrying_config_but_never_identity() {
    let config = DaemonArgs {
        log_level: Some("debug".to_string()),
        heartbeat_interval: Some(45),
        ..Default::default()
    };
    let artifacts = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::DaemonPoll, ""),
        Some(&config),
        &[],
        InstallCommandType::Install,
        DownloadSource::Scarf,
    );
    let compose = artifacts
        .docker
        .compose
        .as_deref()
        .expect("an install yields a docker compose");

    assert!(compose.contains("SCANOPY_SERVER_URL=https://app.scanopy.net"));
    assert!(compose.contains(&format!("SCANOPY_DAEMON_API_KEY={API_KEY_PLACEHOLDER}")));
    assert!(compose.contains("SCANOPY_LOG_LEVEL=debug"));
    assert!(compose.contains("SCANOPY_HEARTBEAT_INTERVAL=45"));
    // Docker-only: logs must land on the mounted volume.
    assert!(compose.contains("SCANOPY_LOG_FILE=/var/log/scanopy/edge-01.log"));
    // The env lines are also exposed structurally.
    assert!(
        artifacts
            .docker
            .env
            .iter()
            .any(|e| e == "SCANOPY_LOG_LEVEL=debug")
    );

    for identity in [
        "SCANOPY_SITE_ID",
        "SCANOPY_USER_ID",
        "SCANOPY_NAME",
        "SCANOPY_MODE",
    ] {
        assert!(
            !compose.contains(identity),
            "compose asserts {identity}, which the client must not be able to set"
        );
    }
}

/// A reconfigure has no full compose — the operator keeps their own — only the changed env
/// vars to swap in. For a ServerPoll daemon that is the port the server dials; the key is
/// never among them (the persisted config volume holds it).
#[test]
fn docker_reconfigure_yields_env_vars_not_a_compose() {
    let artifacts = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::ServerPoll, "https://edge.corp:60074"),
        None,
        &[],
        InstallCommandType::Reconfigure,
        DownloadSource::Scarf,
    );

    assert!(
        artifacts.docker.compose.is_none(),
        "a reconfigure must not hand back a whole compose to replace"
    );
    assert!(
        artifacts
            .docker
            .env
            .contains(&"SCANOPY_DAEMON_PORT=60074".to_string())
    );
    assert!(
        !artifacts
            .docker
            .env
            .iter()
            .any(|e| e.starts_with("SCANOPY_DAEMON_API_KEY"))
    );
}

/// The command is only useful if the daemon can actually parse it. Emit a fully populated
/// config, then feed the emitted string back through the daemon's own clap parser and
/// compare values — this covers the flag names, the value rendering, and the shell quoting
/// in one go, and fails only on a real regression rather than a reworded command.
#[test]
fn emitted_command_parses_back_into_the_same_config() {
    use clap::Parser;

    let config = DaemonArgs {
        log_level: Some("debug".to_string()),
        // A path with a space is the case bare interpolation would break.
        log_file: Some("/var/log/my daemon/d.log".to_string()),
        heartbeat_interval: Some(45),
        bind_address: Some("10.0.0.5".to_string()),
        interfaces: Some(vec!["eth0".to_string(), "Ethernet 2".to_string()]),
        allow_self_signed_certs: Some(true),
        accept_invalid_scan_certs: Some(false),
        ..Default::default()
    };
    let artifacts = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::DaemonPoll, ""),
        Some(&config),
        &[],
        InstallCommandType::Install,
        DownloadSource::Scarf,
    );
    // Take the `scanopy-daemon install ...` half of the `bootstrap && install` one-liner.
    let install = artifacts.linux.split("&& sudo ").nth(1).unwrap();
    let parsed = DaemonCli::parse_from(shell_split(install));
    let Some(DaemonCommand::Install(install_args)) = parsed.command else {
        panic!("expected an install subcommand, got {install}");
    };
    let args = install_args.args;

    assert_eq!(args.log_level.as_deref(), Some("debug"));
    assert_eq!(args.log_file.as_deref(), Some("/var/log/my daemon/d.log"));
    assert_eq!(args.heartbeat_interval, Some(45));
    assert_eq!(args.bind_address.as_deref(), Some("10.0.0.5"));
    assert_eq!(
        args.interfaces,
        Some(vec!["eth0".to_string(), "Ethernet 2".to_string()])
    );
    assert_eq!(args.allow_self_signed_certs, Some(true));
    assert_eq!(args.accept_invalid_scan_certs, Some(false));
    assert_eq!(args.daemon_api_key.as_deref(), Some(API_KEY_PLACEHOLDER));
    assert_eq!(args.server_url.as_deref(), Some("https://app.scanopy.net"));
}

/// Server-controlled and secret fields are `#[serde(skip)]`, so a client cannot smuggle
/// them in through `install_config` — the emitted command uses the provisioned record's
/// values regardless of what the request body claimed.
#[test]
fn client_supplied_config_cannot_override_server_controlled_fields() {
    let body = r#"{
        "log_level": "trace",
        "daemon_api_key": "attacker-key",
        "server_url": "https://evil.example",
        "site_id": "00000000-0000-0000-0000-000000000001",
        "name": "impostor"
    }"#;
    let config: DaemonArgs = serde_json::from_str(body).unwrap();

    assert_eq!(config.log_level.as_deref(), Some("trace"));
    assert_eq!(config.daemon_api_key, None);
    assert_eq!(config.server_url, None);
    assert_eq!(config.site_id, None);
    assert_eq!(config.name, None);

    let artifacts = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::DaemonPoll, ""),
        Some(&config),
        &[],
        InstallCommandType::Install,
        DownloadSource::Scarf,
    );
    assert!(
        artifacts
            .linux
            .contains(&format!("--daemon-api-key {API_KEY_PLACEHOLDER}"))
    );
    assert!(artifacts.linux.contains("--log-level trace"));
    assert!(!artifacts.linux.contains("evil.example"));
    assert!(!artifacts.linux.contains("attacker-key"));
}

/// An ordinary advanced config rides in the MSI filename intact — nothing is dropped, and
/// the values survive the escape + base64 round trip.
#[test]
fn msi_filename_carries_advanced_config() {
    let config = DaemonArgs {
        log_level: Some("debug".to_string()),
        heartbeat_interval: Some(45),
        interfaces: Some(vec!["eth0".to_string(), "Ethernet 2".to_string()]),
        allow_self_signed_certs: Some(true),
        ..Default::default()
    };
    let (filename, omitted) = encode_msi_filename(
        "https://app.scanopy.net",
        &daemon(DaemonMode::DaemonPoll, ""),
        Some(&config),
    );

    assert!(omitted.is_empty(), "unexpectedly dropped {omitted:?}");
    assert!(filename.len() <= 255);

    let fields = decode_msi_filename(&filename);
    assert_eq!(fields.get("loglevel").map(String::as_str), Some("debug"));
    assert_eq!(fields.get("heartbeat").map(String::as_str), Some("45"));
    assert_eq!(
        fields.get("interfaces").map(String::as_str),
        Some("eth0,Ethernet 2")
    );
    assert_eq!(
        fields.get("allowselfsigned").map(String::as_str),
        Some("true")
    );
}

/// The whole config rides in a filename, so a pathological one cannot fit. It must stay
/// within the limit by dropping trailing fields — never by emitting an oversized name —
/// and identity (which is what makes the installer usable at all) must always survive.
/// Whatever is dropped is reported so the user can be told.
#[test]
fn oversized_msi_config_is_truncated_and_reported() {
    let config = DaemonArgs {
        log_level: Some("trace".to_string()),
        log_file: Some(
            r"C:\ProgramData\Scanopy\daemon\logs\scanopy-daemon-verbose.log".to_string(),
        ),
        heartbeat_interval: Some(300),
        bind_address: Some("255.255.255.255".to_string()),
        interfaces: Some(vec![
            "Ethernet Adapter Multiplexor Driver".to_string(),
            "Wi-Fi 6 AX201 160MHz".to_string(),
            "vEthernet (Default Switch)".to_string(),
        ]),
        allow_self_signed_certs: Some(true),
        accept_invalid_scan_certs: Some(true),
        ..Default::default()
    };
    let (filename, omitted) = encode_msi_filename(
        "https://scanopy.some-quite-long-customer-subdomain.example.com:60072",
        &daemon(DaemonMode::DaemonPoll, ""),
        Some(&config),
    );

    assert!(
        filename.len() <= 255,
        "MSI filename is {} chars, over the 255 limit",
        filename.len()
    );
    assert!(
        !omitted.is_empty(),
        "a config this large cannot fit, so something must be reported as dropped"
    );

    // Identity survives; the dropped keys are absent from the filename and named in the
    // report, so the two always agree.
    let fields = decode_msi_filename(&filename);
    assert_eq!(fields.get("mode").map(String::as_str), Some("daemon_poll"));
    assert_eq!(fields.get("name").map(String::as_str), Some("edge-01"));
    for key in &omitted {
        assert!(
            !fields.contains_key(key),
            "{key} was reported dropped but is present in the filename"
        );
    }
}

#[test]
fn daemon_poll_command_dials_the_server_serverpoll_does_not() {
    let dp = build_install_artifacts(
        "https://app.scanopy.net/",
        &daemon(DaemonMode::DaemonPoll, ""),
        None,
        &[],
        InstallCommandType::Install,
        DownloadSource::Scarf,
    );
    assert!(dp.linux.contains("--server-url https://app.scanopy.net"));
    assert!(dp.linux.contains("--daemon-api-key"));

    let sp = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::ServerPoll, "https://edge.corp:60073"),
        None,
        &[],
        InstallCommandType::Install,
        DownloadSource::Scarf,
    );
    assert!(!sp.linux.contains("--server-url"));
    assert!(sp.linux.contains("--daemon-api-key"));
}

// Decode the filename the way parse-filename.js does (strip prefix, base64url-decode,
// parse the query string) so the encode<->decode scheme is validated in Rust; the
// JScript CA is a faithful port of this. Percent-decoding here is ASCII-only (matching
// the JScript's manual decoder), sufficient for mode/name/url values.
fn decode_msi_filename(filename: &str) -> std::collections::HashMap<String, String> {
    let blob = filename
        .strip_prefix("scanopy-daemon-")
        .and_then(|s| s.strip_suffix(".msi"))
        .expect("scanopy-daemon-<blob>.msi");
    let query = String::from_utf8(Base64UrlUnpadded::decode_vec(blob).unwrap()).unwrap();
    query
        .split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_string(), urlencoding::decode(v).unwrap().into_owned()))
        .collect()
}

#[test]
fn msi_filename_is_one_base64_segment_that_round_trips() {
    // DaemonPoll pre-fills the server url it dials.
    let (name, _) = encode_msi_filename(
        "https://app.scanopy.net:60072",
        &daemon(DaemonMode::DaemonPoll, ""),
        None,
    );
    // One compact segment, no per-field `~~` markers.
    assert!(name.starts_with("scanopy-daemon-"));
    assert!(name.ends_with(".msi"));
    assert!(!name.contains("~~"));

    let fields = decode_msi_filename(&name);
    assert_eq!(fields.get("mode").map(String::as_str), Some("daemon_poll"));
    assert_eq!(fields.get("name").map(String::as_str), Some("edge-01"));
    // The url survives its :// and : intact through percent-encode + base64.
    assert_eq!(
        fields.get("url").map(String::as_str),
        Some("https://app.scanopy.net:60072")
    );

    // ServerPoll is dialed by the server → no server url encoded.
    let sp = decode_msi_filename(
        &encode_msi_filename(
            "https://app.scanopy.net",
            &daemon(DaemonMode::ServerPoll, "https://edge.corp"),
            None,
        )
        .0,
    );
    assert_eq!(sp.get("mode").map(String::as_str), Some("server_poll"));
    assert!(!sp.contains_key("url"));
}

#[test]
fn msi_filename_is_encoded_for_rename_prefill() {
    let a = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::ServerPoll, "https://edge.corp"),
        None,
        &[],
        InstallCommandType::Install,
        DownloadSource::Scarf,
    );
    // Filename carries the encoded values for a rename-to-prefill; the static MSI URL
    // is a UI-side const, not part of the per-tenant provision response.
    assert!(a.msi.filename.starts_with("scanopy-daemon-"));
    assert!(a.msi.filename.ends_with(".msi"));
}

#[test]
fn install_command_omits_name() {
    let a = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::DaemonPoll, ""),
        None,
        &[],
        InstallCommandType::Install,
        DownloadSource::Scarf,
    );
    assert!(!a.linux.contains("--name"));
}

/// Every `/root/.config…` path a file names, cut at the first character a path can't hold.
fn root_config_paths(text: &str) -> Vec<&str> {
    text.match_indices("/root/.config")
        .map(|(start, _)| {
            let rest = &text[start..];
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_')))
                .unwrap_or(rest.len());
            rest[..end].trim_end_matches('.')
        })
        .collect()
}

/// The daemon image only persists config if every compose mounts the volume where the image
/// keeps it. GH #760: the composes and the image disagreed for six releases and every container
/// recreate started the daemon blank, with nothing failing. Each shipped compose, the generated
/// one, and both Dockerfiles must name [`DOCKER_CONFIG_DIR`] and no other config path.
#[test]
fn docker_config_dir_matches_every_compose_and_dockerfile() {
    let generated = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::DaemonPoll, ""),
        None,
        &[],
        InstallCommandType::Install,
        DownloadSource::Scarf,
    )
    .docker
    .compose
    .expect("an install yields a docker compose");

    let composes = [
        (
            "docker-compose.yml",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../docker-compose.yml"
            )),
        ),
        (
            "docker-compose.commercial.yml",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../docker-compose.commercial.yml"
            )),
        ),
        (
            "docker-compose.portainer.yml",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../docker-compose.portainer.yml"
            )),
        ),
        (
            "docker-compose.test.yml",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../docker-compose.test.yml"
            )),
        ),
        ("generated compose", generated.as_str()),
    ];
    for (name, text) in composes {
        let paths = root_config_paths(text);
        assert!(!paths.is_empty(), "{name} mounts no daemon config volume");
        for path in paths {
            assert_eq!(
                path, DOCKER_CONFIG_DIR,
                "{name} mounts daemon config at {path}, but the daemon image keeps it at \
                 {DOCKER_CONFIG_DIR}"
            );
        }
    }

    // The daemon's default per-user config dir on Linux as root (`ProjectDirs` for
    // com.scanopy.daemon). Not derivable here: the test may run on macOS or Windows.
    const DEFAULT_CONFIG_DIR: &str = "/root/.config/daemon";
    let dockerfiles = [
        (
            "Dockerfile.daemon",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Dockerfile.daemon")),
        ),
        (
            "Dockerfile.dev",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Dockerfile.dev")),
        ),
    ];
    for (name, text) in dockerfiles {
        assert!(
            text.contains(&format!("ln -s {DOCKER_CONFIG_DIR} {DEFAULT_CONFIG_DIR}")),
            "{name} must symlink {DEFAULT_CONFIG_DIR} onto {DOCKER_CONFIG_DIR}"
        );
        for path in root_config_paths(text) {
            assert!(
                path == DOCKER_CONFIG_DIR || path == DEFAULT_CONFIG_DIR,
                "{name} names config path {path}, which is neither {DOCKER_CONFIG_DIR} nor the \
                 symlinked default {DEFAULT_CONFIG_DIR}"
            );
        }
    }
}

fn daemon_at(version: Option<&str>, name: &str) -> Daemon {
    let mut daemon = daemon(DaemonMode::DaemonPoll, "");
    daemon.base.name = name.to_string();
    daemon.base.version = version.map(|v| semver::Version::parse(v).unwrap());
    daemon
}

fn step(steps: &[UpgradeStep], r#type: UpgradeStepType) -> &str {
    &steps
        .iter()
        .find(|s| s.r#type == r#type)
        .unwrap_or_else(|| panic!("no {type:?} step in {steps:?}"))
        .command
}

/// The upgrade's restart step must restart the service the installer registered for that daemon.
/// The server emits only a daemon id; this resolves it through the installer's own selector
/// matching and service naming, for each shape of install a host can hold.
#[test]
fn upgrade_restart_resolves_to_the_installed_slots_service() {
    use crate::daemon::install::{Installed, restart_target};
    use clap::Parser;

    let install = |slot: &str| Installed {
        slot: slot.to_string(),
        config_path: format!("/etc/scanopy/daemon/{slot}/config.json").into(),
        name: Some(format!("{slot} daemon")),
        daemon_id: Some(uuid::Uuid::new_v4()),
        api_key: None,
        daemon_port: None,
    };
    // The default slot, an allocated numbered slot, and a pre-slot install named after its daemon.
    let installed = [
        install("scanopy-daemon"),
        install("scanopy-daemon-2"),
        install("edge"),
    ];

    for (entry, expected_service) in
        installed
            .iter()
            .zip(["scanopy-daemon", "scanopy-daemon-2", "scanopy-daemon-edge"])
    {
        let mut daemon = daemon_at(Some("0.17.5"), "renamed in the UI since");
        daemon.id = entry.daemon_id.unwrap();
        let artifacts = build_upgrade_artifacts(&daemon, DownloadSource::Scarf);
        assert!(!artifacts.legacy_service_name);

        for steps in [&artifacts.linux, &artifacts.macos, &artifacts.freebsd] {
            let restart = step(steps, UpgradeStepType::Restart);
            let command = restart.strip_prefix("sudo ").unwrap();
            let Some(DaemonCommand::Restart(args)) =
                DaemonCli::parse_from(shell_split(command)).command
            else {
                panic!("not a restart command: {restart}");
            };
            let target = restart_target(&installed, args.instance.as_deref()).unwrap();
            assert_eq!(target.service_id(), expected_service, "{restart}");
        }
    }
}

/// Daemons from before the install engine get their service named after the daemon, as it was
/// registered then. A name with a space must reach `systemctl` as one argument.
#[test]
fn legacy_upgrade_restart_passes_the_service_name_as_one_argument() {
    let artifacts = build_upgrade_artifacts(
        &daemon_at(Some("0.17.4"), "DC Daemon"),
        DownloadSource::Scarf,
    );
    assert!(artifacts.legacy_service_name);

    let restart = shell_split(step(&artifacts.linux, UpgradeStepType::Restart));
    assert_eq!(
        restart.last().unwrap(),
        &crate::daemon::install::service_id("DC Daemon")
    );
}

#[test]
fn upgrade_names_the_service_only_for_daemons_older_than_the_install_engine() {
    for (version, legacy) in [
        (None, true),
        (Some("0.17.4"), true),
        (Some("0.17.5"), false),
    ] {
        let daemon = daemon_at(version, "edge-01");
        let artifacts = build_upgrade_artifacts(&daemon, DownloadSource::Scarf);
        assert_eq!(artifacts.legacy_service_name, legacy, "{version:?}");
        let names_instance = |steps: &[UpgradeStep]| {
            step(steps, UpgradeStepType::Restart).contains(&daemon.id.to_string())
        };
        assert_eq!(names_instance(&artifacts.linux), !legacy, "{version:?}");
        assert_eq!(
            artifacts
                .windows
                .iter()
                .any(|s| s.r#type == UpgradeStepType::Restart
                    && s.command.contains(&daemon.id.to_string())),
            !legacy,
            "{version:?}"
        );
    }
}

/// Every download the server hands out for one plan's server: each OS's install command, the
/// docker compose, and the upgrade steps for a current and a pre-install-engine daemon.
fn generated_downloads(plan: &BillingPlan) -> Vec<String> {
    let source = DownloadSource::from(plan);
    let install = build_install_artifacts(
        "https://scanopy.example.com",
        &daemon(DaemonMode::DaemonPoll, ""),
        None,
        &[],
        InstallCommandType::Install,
        source,
    );
    let mut downloads = vec![
        install.linux,
        install.macos,
        install.freebsd,
        install.windows,
        install
            .docker
            .compose
            .expect("an install yields a docker compose"),
    ];
    for version in [Some("0.17.4"), Some("0.17.5")] {
        let upgrade = build_upgrade_artifacts(&daemon_at(version, "edge-01"), source);
        for steps in [
            upgrade.linux,
            upgrade.macos,
            upgrade.freebsd,
            upgrade.windows,
        ] {
            downloads.extend(
                steps
                    .into_iter()
                    .filter(|s| s.r#type == UpgradeStepType::Download)
                    .map(|s| s.command),
            );
        }
        downloads.push(upgrade.docker.pinned_image);
    }
    downloads
}

/// A Community Edition server downloads straight from GitHub and GHCR. Cloud and commercial
/// self-hosted servers download through Scarf: every download names a Scarf host, or passes
/// `--gateway` to install.sh, which then fetches from get.scanopy.net.
#[test]
fn only_community_servers_download_outside_scarf() {
    use crate::server::billing::types::base::PlanConfig;

    let cfg = PlanConfig::default();
    let plans = [
        BillingPlan::Community(cfg),
        BillingPlan::Free(cfg),
        BillingPlan::Starter(cfg),
        BillingPlan::Pro(cfg),
        BillingPlan::Team(cfg),
        BillingPlan::Business(cfg),
        BillingPlan::Enterprise(cfg),
        BillingPlan::Demo(cfg),
        BillingPlan::CommercialSelfHosted(cfg),
        BillingPlan::SelfHostedStandard(cfg),
        BillingPlan::SelfHostedPlus(cfg),
    ];
    let scarf = [
        "get.scanopy.net",
        "docker.scanopy.net",
        "install.sh --gateway",
    ];

    for plan in plans {
        let community = matches!(plan, BillingPlan::Community(_));
        for download in generated_downloads(&plan) {
            let through_scarf = scarf.iter().any(|host| download.contains(host));
            assert_eq!(through_scarf, !community, "{plan:?}: {download}");
            if community {
                assert!(
                    download.contains("github.com/scanopy/scanopy")
                        || download.contains("githubusercontent.com/scanopy/scanopy")
                        || download.contains("ghcr.io/scanopy/scanopy"),
                    "{plan:?}: {download}"
                );
            }
        }
    }
}
