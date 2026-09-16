use axum::{
    Router,
    http::{HeaderValue, Method},
    middleware,
};
use clap::Parser;
use scanopy::{
    daemon::runtime::service::StartupOutcome,
    daemon::shared::api_client::ConnectionError,
    daemon::{
        install::run_command,
        runtime::types::DaemonAppState,
        shared::{
            config::{AppConfig, ConfigStore, DaemonArgs, DaemonCli},
            handlers::create_router,
            middleware::capture_fixtures_middleware,
        },
        utils::base::{DaemonUtils, PlatformDaemonUtils},
    },
    server::daemons::r#impl::base::DaemonMode,
};
use std::{sync::Arc, time::Duration};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    // Parse CLI. The `install`/`uninstall`/`list` subcommands short-circuit here (own logging to
    // stdout); with no subcommand we fall through to running the daemon.
    let cli = DaemonCli::parse();
    if let Some(command) = cli.command {
        return build_runtime()?.block_on(run_command(command));
    }

    // On Windows, when the Service Control Manager launched us (signalled by the hidden `--service`
    // marker that `daemon install` bakes into the service binPath), run under the service control
    // dispatcher: it reports RUNNING/STOPPED to the SCM and maps the STOP control to shutdown. We
    // gate on the marker rather than always probing the SCM, because StartServiceCtrlDispatcher
    // blocks for ~30s before failing when not launched by the SCM — which would stall every
    // foreground run. If the dispatcher unexpectedly reports we're not under the SCM, fall through.
    #[cfg(windows)]
    if cli.service {
        match win_service::try_dispatch() {
            win_service::Dispatch::RanAsService(result) => return result,
            win_service::Dispatch::NotUnderScm => {}
        }
    }

    // Foreground/console: run the daemon, shutting down on Ctrl+C.
    build_runtime()?.block_on(run_daemon(cli.run_args, async {
        let _ = tokio::signal::ctrl_c().await;
    }))
}

/// Raise the process open-file-descriptor soft limit toward its hard limit so discovery
/// deep-scan concurrency isn't clamped (macOS' 256 default soft limit leaves too few FDs
/// after reserves, forcing concurrency to 1). Best-effort: logs and continues on failure.
/// No-op on Windows (handle-based, no RLIMIT_NOFILE).
#[cfg(unix)]
fn raise_fd_limit() {
    // Generous cap. macOS clamps setrlimit at kern.maxfilesperproc and rejects
    // RLIM_INFINITY, so request min(hard, TARGET) rather than "unlimited".
    const TARGET: libc::rlim_t = 10_240;
    let mut lim = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim) } != 0 {
        tracing::warn!("Could not read RLIMIT_NOFILE; leaving FD limit unchanged");
        return;
    }
    let old_soft = lim.rlim_cur;
    let desired = TARGET.min(lim.rlim_max);
    if old_soft >= desired {
        tracing::debug!(
            soft = old_soft,
            hard = lim.rlim_max,
            "FD limit already sufficient"
        );
        return;
    }
    lim.rlim_cur = desired;
    if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &lim) } == 0 {
        tracing::info!(
            old_soft,
            new_soft = desired,
            hard = lim.rlim_max,
            "Raised open-file-descriptor limit for scan concurrency"
        );
    } else {
        tracing::warn!(
            attempted = desired,
            hard = lim.rlim_max,
            "Failed to raise FD soft limit; deep-scan concurrency may stay low"
        );
    }
}

#[cfg(not(unix))]
fn raise_fd_limit() {}

fn build_runtime() -> anyhow::Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(4 * 1024 * 1024) // 4MB stack for deep async scanning
        .enable_all()
        .build()?)
}

/// Run the daemon until `shutdown` resolves. The foreground path passes Ctrl+C; the Windows
/// service path passes the SCM stop signal.
async fn run_daemon<F: std::future::Future<Output = ()>>(
    run_args: DaemonArgs,
    shutdown: F,
) -> anyhow::Result<()> {
    // Load config from the daemon run flags
    let config = AppConfig::load(run_args)?;

    // Initialize tracing with stdout + optional file appender
    let log_path = config.resolve_log_path();
    let env_filter = tracing_subscriber::EnvFilter::new(format!(
        "scanopy={lvl},daemon={lvl},events={lvl},pnet_datalink={lvl}",
        lvl = config.log_level
    ));

    // _guard must be held for the lifetime of the program to ensure logs flush
    let _file_guard: Option<WorkerGuard>;

    if let Some(ref path) = log_path {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let log_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let log_filename = path
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("scanopy-daemon.log"));
        let file_appender = tracing_appender::rolling::never(log_dir, log_filename);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        _file_guard = Some(guard);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .fmt_fields(scanopy::server::logging::format::LabelFields)
                    .with_ansi(scanopy::server::logging::format::supports_ansi()),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .fmt_fields(scanopy::server::logging::format::LabelFields)
                    .with_writer(non_blocking)
                    .with_ansi(false),
            )
            .init();
    } else {
        _file_guard = None;
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .fmt_fields(scanopy::server::logging::format::LabelFields)
                    .with_ansi(scanopy::server::logging::format::supports_ansi()),
            )
            .init();
    }

    // Raise the open-file-descriptor soft limit before anything opens FDs, so discovery's
    // deep-scan concurrency isn't starved (macOS defaults the soft limit to 256, which
    // forces deep-scan concurrency down to 1 on large scans — every host scans serially).
    raise_fd_limit();

    // Use the path AppConfig::load already resolved (honors --config-dir); don't re-derive, which
    // would re-read $HOME and could point somewhere the config wasn't written.
    let path = config
        .config_path
        .clone()
        .ok_or_else(|| anyhow::anyhow!("config path was not resolved during load"))?;
    let path_str = path.to_str().unwrap_or("<invalid path>");

    // Initialize unified storage with full config
    let config_store = Arc::new(ConfigStore::new(path.clone(), config.clone()));
    let utils = PlatformDaemonUtils::new();

    let daemon_id = config_store.get_id().await?;
    let daemon_name = config_store.get_name().await?;
    // ServerPoll daemons have no server URL (the server dials them), so this must not
    // hard-fail — a `?` here would crash-loop a ServerPoll daemon under the service's
    // KeepAlive before the mode is even checked. It's only needed for the DaemonPoll
    // connect path (guaranteed present there by mode inference) and the banner.
    let server_addr = config_store.get_server_url().await.ok();
    let network_id = config_store.get_network_id().await?;
    let api_key = config_store.get_api_key().await?;
    let mode = config_store.get_mode().await?;
    let interval_secs = config_store.get_heartbeat_interval().await?;
    let interval = Duration::from_secs(interval_secs);
    // Startup banner
    tracing::info!("");
    tracing::info!("   _____                                   ");
    tracing::info!("  / ___/_________ _____  ____  ____  __  __");
    tracing::info!("  \\__ \\/ ___/ __ `/ __ \\/ __ \\/ __ \\/ / / /");
    tracing::info!(" ___/ / /__/ /_/ / / / / /_/ / /_/ / /_/ / ");
    tracing::info!("/____/\\___/\\__,_/_/ /_/\\____/ .___/\\__, /  ");
    tracing::info!("                           /_/    /____/   ");
    tracing::info!("");
    tracing::info!("Scanopy Daemon v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    // A daemon that has not completed its handshake has no identity yet — the stored id is
    // `Uuid::nil()` and the server assigns the real one at first contact. Printing the
    // all-zeros UUID reads as a real id, and operators have chased it as one.
    if daemon_id.is_nil() {
        tracing::info!("  Daemon ID:       pending (assigned by the server at first contact)");
    } else {
        tracing::info!("  Daemon ID:       {}", daemon_id);
    }
    tracing::info!("  Name:            {}", daemon_name);
    match &network_id {
        Some(nid) => tracing::info!("  Network ID:      {}", nid),
        None => {
            tracing::info!("  Network ID:      pending (assigned by the server at first contact)")
        }
    }
    tracing::info!("  Config file:     {}", path_str);
    match &log_path {
        Some(p) => tracing::info!("  Log file:        {}", p.display()),
        None => tracing::info!("  Log file:        disabled (stdout only)"),
    }
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let state = DaemonAppState::new(config_store.clone(), utils).await?;
    let runtime_service = state.services.runtime_service.clone();

    // Create HTTP server with config values
    let api_router = create_router(state.clone(), mode).with_state(state);

    // Restrict CORS to server URL origin (defense-in-depth against exposed daemon ports)
    let cors = {
        let server_origin = config_store
            .get_server_url()
            .await
            .ok()
            .and_then(|url| url::Url::parse(&url).ok())
            .map(|u| format!("{}://{}", u.scheme(), u.authority()))
            .and_then(|o| o.parse::<HeaderValue>().ok());

        if let Some(origin) = server_origin {
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(Any)
        } else {
            // Fallback: no CORS (same-origin only)
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
                .allow_headers(Any)
        }
    };

    let app = Router::new().merge(api_router).layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .layer(middleware::from_fn(capture_fixtures_middleware)),
    );

    let bind_addr = format!("{}:{}", config.bind_address, config.daemon_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    // Spawn server in background
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Configuration summary: how this daemon operates. Every line below is gated on the mode
    // it is actually true for — a DaemonPoll daemon dials out and is never dialled, so a bind
    // address or a URL for it describes nothing it does.
    tracing::info!("Configuration:");
    tracing::info!("  Mode:            {:?}", mode);
    match mode {
        DaemonMode::DaemonPoll => {
            if let Some(addr) = &server_addr {
                tracing::info!("  Connects to:     {}", addr);
            }
        }
        DaemonMode::ServerPoll => {
            tracing::info!("  Listens on:      {}", bind_addr);
            // Only the configured URL is a fact: it is captured on the daemon record at
            // provisioning time, which is the URL the server actually dials — the daemon never
            // self-reports one (`register_with_server` sends `url: None`). The fallback the
            // daemon can compute is its own IP and port over plain http, which a proxy, NAT or
            // TLS terminator makes wrong, so it is not printed as if the server used it. An
            // invented URL here is what sent one operator hunting an empty `daemons.url`
            // column that is empty by design (GH #611).
            if let Some(daemon_url) = config_store.get_daemon_url().await? {
                tracing::info!("  Daemon URL:      {}", daemon_url);
            }
        }
    }
    let ip_addresses = config_store.get_interfaces().await.unwrap_or_default();
    if ip_addresses.is_empty() {
        tracing::info!("  Interfaces:      all (no restriction)");
    } else {
        tracing::info!("  Interfaces:      {}", ip_addresses.join(", "));
    }

    // Deprecation warnings for config values that have moved to server-side settings
    if config.docker_proxy.is_some() {
        tracing::warn!(
            "Deprecated config: docker_proxy, docker_proxy_ssl_cert, docker_proxy_ssl_key, docker_proxy_ssl_chain"
        );
        tracing::warn!("  Docker proxy config will no longer be read from daemon in v0.16.0.");
        tracing::warn!("  Migrate by creating a DockerProxy credential in the Scanopy UI.");
        tracing::warn!("  See: https://scanopy.net/docs/guides/unified-discovery-migration/");
    }

    {
        use scanopy::server::discovery::r#impl::scan_settings::defaults;
        let has_deprecated_scan_settings = config.arp_retries != defaults::arp_retries()
            || config.arp_rate_pps != defaults::arp_rate_pps()
            || config.scan_rate_pps != defaults::scan_rate_pps()
            || config.port_scan_batch_size != defaults::port_scan_batch_size();
        if has_deprecated_scan_settings {
            tracing::warn!(
                "Deprecated config: arp_retries, arp_rate_pps, scan_rate_pps, port_scan_batch_size"
            );
            tracing::warn!(
                "  Scan settings are now configured per-discovery on the server and will no longer be read from daemon in v0.16.0."
            );
            tracing::warn!("  See: https://scanopy.net/docs/guides/unified-discovery-migration/");
        }
    }

    // Initialize services based on mode
    /// Log a connection error with prescriptive guidance if available.
    fn log_connection_error(e: &anyhow::Error) {
        tracing::warn!("{e}");
        // If the error chain contains a ConnectionError, log its guidance
        if let Some(conn_err) = e.downcast_ref::<ConnectionError>() {
            tracing::warn!("{}", conn_err.cause_and_fix());
        }
    }

    let startup_result: Result<(), ()> = match mode {
        DaemonMode::DaemonPoll => {
            if let Some(api_key) = api_key {
                // A server-provisioned daemon starts with just a 1:1 key and no
                // network id — the server derives its identity from the key and
                // returns it (cached on the register response). Use nil as a
                // placeholder network id; a legacy daemon still passes its own.
                let effective_network_id = network_id.unwrap_or_else(uuid::Uuid::nil);
                tracing::info!(
                    "Connecting to server at {}...",
                    server_addr.as_deref().unwrap_or("<server url>")
                );
                let mut result = runtime_service
                    .initialize_services(effective_network_id, api_key.clone())
                    .await?;

                if let StartupOutcome::ConnectionFailed(ref e) = result {
                    log_connection_error(e);
                    tracing::info!("Retrying connection...");

                    const RETRY_DELAYS: &[u64] = &[5, 10, 20, 40, 60];
                    for (i, &delay) in RETRY_DELAYS.iter().enumerate() {
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                        tracing::info!(
                            "Connection attempt {}/{}...",
                            i + 2,
                            RETRY_DELAYS.len() + 1
                        );
                        result = runtime_service
                            .initialize_services(effective_network_id, api_key.clone())
                            .await?;
                        match &result {
                            StartupOutcome::Ok => {
                                tracing::info!("Connected successfully");
                                break;
                            }
                            StartupOutcome::ConnectionFailed(e) => {
                                tracing::warn!("Still unreachable: {e}");
                                if let Some(conn_err) = e.downcast_ref::<ConnectionError>() {
                                    tracing::warn!("{}", conn_err.cause_and_fix());
                                }
                            }
                            StartupOutcome::AuthFailed(_) | StartupOutcome::VersionRejected(_) => {
                                break;
                            }
                        }
                    }
                }

                match result {
                    StartupOutcome::Ok => Ok(()),
                    StartupOutcome::ConnectionFailed(_) => Err(()),
                    StartupOutcome::AuthFailed(e) => {
                        // Terminal, server-reachable rejection (bad/regenerated key, or the
                        // daemon isn't provisioned). Surface the server's actual reason and let
                        // it speak for itself — it already carries any remedy.
                        tracing::error!("Registration rejected by the server: {e}");
                        Err(())
                    }
                    StartupOutcome::VersionRejected(e) => {
                        // The server rejected this daemon's version as unsupported. This is
                        // terminal — exit non-zero so the service manager surfaces a failure
                        // instead of the process parking 'active (running)' forever. With
                        // Restart=always this becomes a visible restart loop until updated.
                        tracing::error!(
                            "This daemon's version is no longer supported and was rejected by the \
                             server: {e}. Update the daemon binary to the latest version from the \
                             Scanopy UI under Discover > Daemons, then restart. The daemon will not \
                             run until it is updated."
                        );
                        std::process::exit(1);
                    }
                }
            } else if network_id.is_some() {
                tracing::error!(
                    "Daemon is missing an API key. Fix: re-run the install command from the Scanopy UI. Server: {}",
                    server_addr.as_deref().unwrap_or("<server url>")
                );
                Err(())
            } else {
                tracing::info!("Missing network ID — waiting for server to hit /api/initialize...");
                Ok(())
            }
        }
        DaemonMode::ServerPoll => {
            if api_key.is_none() {
                tracing::error!(
                    "ServerPoll daemon has no API key configured. \
                     Configure with the key from provision response."
                );
                Err(())
            } else {
                Ok(())
            }
        }
    };

    // Mode-specific ready message and runtime loop
    match mode {
        DaemonMode::ServerPoll => {
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            tracing::info!("Daemon ready [ServerPoll mode]");
            tracing::info!("  Waiting for the server to poll for status and discovery work");
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
        DaemonMode::DaemonPoll => {
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            if startup_result.is_ok() {
                tracing::info!("Daemon ready [DaemonPoll mode]");
                tracing::info!(
                    "  Polling the server for discovery work every {}s",
                    interval_secs
                );
            } else {
                tracing::error!(
                    "Daemon NOT ready — fix the issue above and restart the daemon (Ctrl+C to stop)"
                );
            }
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            // Only start polling once successfully connected
            if startup_result.is_ok() {
                tokio::spawn(async move {
                    loop {
                        if let Err(e) = runtime_service.request_work().await {
                            tracing::warn!(
                                "Polling failed: {}. Retrying in {}s...",
                                e,
                                interval_secs
                            );
                            tokio::time::sleep(interval).await;
                        }
                    }
                });
            }
        }
    }

    // Keep process alive until the shutdown signal (Ctrl+C in the foreground, or the SCM STOP
    // control when running as a Windows service).
    shutdown.await;

    tracing::info!("Shutdown signal received");
    tracing::info!("Daemon stopped");

    Ok(())
}

/// Windows Service Control Manager integration. Only compiled on Windows; the daemon runs here
/// when `daemon install` registered it as a service and the SCM launched the binary.
#[cfg(windows)]
mod win_service {
    use super::{DaemonArgs, build_runtime, run_daemon};
    use clap::Parser;
    use scanopy::daemon::shared::config::DaemonCli;
    use std::{
        ffi::OsString,
        sync::{Arc, Mutex},
        time::Duration,
    };
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };

    /// The service name is only meaningful for shared-process services; ours is OWN_PROCESS, so
    /// the SCM ignores it, but the API still requires a value.
    const SERVICE_NAME: &str = "scanopy-daemon";
    const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
    /// ERROR_FAILED_SERVICE_CONTROLLER_CONNECT — returned by StartServiceCtrlDispatcher when the
    /// process was not launched by the SCM (i.e. we're running from a normal console).
    const ERROR_FAILED_SERVICE_CONTROLLER_CONNECT: i32 = 1063;

    pub enum Dispatch {
        /// The SCM launched us and the service ran to completion (or failed).
        RanAsService(anyhow::Result<()>),
        /// Not launched by the SCM — the caller should run the daemon in the foreground.
        NotUnderScm,
    }

    /// Attempt to connect to the SCM and run as a service. Blocks until the service stops when it
    /// is a service; returns `NotUnderScm` immediately when running from a console.
    pub fn try_dispatch() -> Dispatch {
        match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
            Ok(()) => Dispatch::RanAsService(Ok(())),
            Err(windows_service::Error::Winapi(io))
                if io.raw_os_error() == Some(ERROR_FAILED_SERVICE_CONTROLLER_CONNECT) =>
            {
                Dispatch::NotUnderScm
            }
            Err(e) => Dispatch::RanAsService(Err(e.into())),
        }
    }

    define_windows_service!(ffi_service_main, service_main);

    fn service_main(_arguments: Vec<OsString>) {
        // Errors here can't reach the SCM meaningfully; the daemon's own file log (via --log-file
        // in the service binPath) captures startup failures.
        if let Err(e) = run_service() {
            eprintln!("scanopy-daemon service error: {e}");
        }
    }

    fn run_service() -> anyhow::Result<()> {
        // Bridge the SCM STOP control (delivered on an SCM thread) to the async shutdown future.
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));

        let event_handler = {
            let shutdown_tx = shutdown_tx.clone();
            move |control_event| -> ServiceControlHandlerResult {
                match control_event {
                    ServiceControl::Stop => {
                        if let Some(tx) = shutdown_tx.lock().unwrap().take() {
                            let _ = tx.send(());
                        }
                        ServiceControlHandlerResult::NoError
                    }
                    ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                    _ => ServiceControlHandlerResult::NotImplemented,
                }
            }
        };

        let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

        // Report RUNNING promptly so the SCM start handshake succeeds. The daemon's own
        // connect/retry loop happens afterwards inside `run_daemon` and must not block this.
        let running_status = ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        };
        status_handle.set_service_status(running_status)?;

        // The service binPath carries the daemon flags (--name, --config-dir, --log-file, ...),
        // so re-parse them from the process command line.
        let cli = DaemonCli::parse();
        let run_args: DaemonArgs = cli.run_args;

        let result = build_runtime()?.block_on(run_daemon(run_args, async move {
            let _ = shutdown_rx.await;
        }));

        // Always report STOPPED so the SCM doesn't leave the service stuck in STOP_PENDING.
        let stopped_status = ServiceStatus {
            service_type: SERVICE_TYPE,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(if result.is_ok() { 0 } else { 1 }),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        };
        status_handle.set_service_status(stopped_status)?;

        result
    }
}
