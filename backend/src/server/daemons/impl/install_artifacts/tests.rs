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
    connected.base.last_seen = Some(chrono::Utc::now());

    let target = |d: &Daemon, command_type| {
        parse_unix_install(&build_install_artifacts(
            "https://app.scanopy.net",
            d,
            None,
            &[],
            command_type,
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
        connected.base.last_seen = Some(chrono::Utc::now());
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
    );
    assert!(dp.linux.contains("--server-url https://app.scanopy.net"));
    assert!(dp.linux.contains("--daemon-api-key"));

    let sp = build_install_artifacts(
        "https://app.scanopy.net",
        &daemon(DaemonMode::ServerPoll, "https://edge.corp:60073"),
        None,
        &[],
        InstallCommandType::Install,
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
