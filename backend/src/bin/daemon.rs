use axum::{
    Router,
    http::{HeaderValue, Method},
    middleware,
};
use clap::Parser;
use scanopy::{
    daemon::{
        runtime::types::DaemonAppState,
        shared::{
            config::{AppConfig, ConfigStore, DaemonCli},
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
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_stack_size(4 * 1024 * 1024) // 4MB stack for deep async scanning
        .enable_all()
        .build()?;

    runtime.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    // Parse CLI and load config
    let cli = DaemonCli::parse();
    let config = AppConfig::load(cli)?;

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(format!(
            "scanopy={},daemon={}",
            config.log_level, config.log_level
        )))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get config path using daemon name for namespaced configs
    let (_, path) = AppConfig::get_config_path_for_name(Some(&config.name))?;
    let path_str = path.to_str().unwrap_or("<invalid path>");

    // Initialize unified storage with full config
    let config_store = Arc::new(ConfigStore::new(path.clone(), config.clone()));
    let utils = PlatformDaemonUtils::new();

    let daemon_id = config_store.get_id().await?;
    let daemon_name = config_store.get_name().await?;
    let server_addr = config_store.get_server_url().await?;
    let network_id = config_store.get_network_id().await?;
    let api_key = config_store.get_api_key().await?;
    let mode = config_store.get_mode().await?;
    let interval_secs = config_store.get_heartbeat_interval().await?;
    let interval = Duration::from_secs(interval_secs);
    let concurrent_scans = config.concurrent_scans;

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
    tracing::info!("  Daemon ID:       {}", daemon_id);
    tracing::info!("  Name:            {}", daemon_name);
    tracing::info!("  Config file:     {}", path_str);
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let state = DaemonAppState::new(config_store.clone(), utils).await?;
    let runtime_service = state.services.runtime_service.clone();

    // Create HTTP server with config values
    let api_router = create_router(state.clone()).with_state(state);

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

    // Get daemon URL for display
    let daemon_url = runtime_service.get_daemon_url().await?;
    let url_source = if config_store.get_daemon_url().await?.is_some() {
        "configured"
    } else {
        "auto-detected"
    };

    // Configuration summary
    tracing::info!("Configuration:");
    tracing::info!("  Server:          {}", server_addr);
    if let Some(nid) = &network_id {
        tracing::info!("  Network ID:      {}", nid);
    }
    tracing::info!("  Mode:            {:?}", mode);
    tracing::info!("  Bind address:    {}", bind_addr);
    tracing::info!("  Daemon URL:      {} ({})", daemon_url, url_source);
    tracing::info!("  Heartbeat:       every {}s", interval_secs);
    if concurrent_scans == 15 {
        tracing::info!("  Concurrent:      auto (determined at scan time)");
    } else {
        tracing::info!("  Concurrent:      {} parallel scans", concurrent_scans);
    }

    // Initialize services based on mode
    match mode {
        DaemonMode::DaemonPoll => {
            // DaemonPoll mode: Register with server and poll for work
            if let Some(network_id) = network_id {
                if let Some(api_key) = api_key {
                    if let Err(e) = runtime_service
                        .initialize_services(network_id, api_key.clone())
                        .await
                    {
                        tracing::warn!(
                            "Could not connect to server during startup: {e}. Will retry during polling."
                        );
                    }
                } else {
                    tracing::warn!(
                        "Daemon is missing an API key. Go to discovery tab in UI to generate an API key."
                    );
                }
            } else {
                tracing::info!("Missing network ID - waiting for server to hit /api/initialize...");
            }
        }
        DaemonMode::ServerPoll => {
            // ServerPoll mode: Don't register - daemon was provisioned via server UI
            // Just serve HTTP endpoints and wait for server to poll
            if api_key.is_none() {
                tracing::warn!(
                    "ServerPoll daemon has no API key configured. \
                     Configure with the key from provision response."
                );
            }
        }
    }

    // Mode-specific ready message and runtime loop
    match mode {
        DaemonMode::ServerPoll => {
            // ServerPoll mode: Server polls daemon, no outbound connections
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            tracing::info!("Daemon ready [ServerPoll mode]");
            tracing::info!(
                "  Server will poll this daemon at {} for status and discovery",
                daemon_url
            );
            tracing::info!("  No outbound connections");
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            // No outbound loop needed - daemon just serves HTTP endpoints
        }
        DaemonMode::DaemonPoll => {
            // DaemonPoll mode: Daemon polls server for work
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            tracing::info!("Daemon ready [DaemonPoll mode]");
            tracing::info!(
                "  Polling server every {}s for discovery work",
                interval_secs
            );
            tracing::info!("  No inbound connections");
            tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            tokio::spawn(async move {
                loop {
                    if let Err(e) = runtime_service.request_work().await {
                        tracing::warn!("Work request task failed: {}, retrying...", e);
                        tokio::time::sleep(interval).await;
                    }
                }
            });
        }
    }

    // Keep process alive until shutdown signal
    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutdown signal received");
    tracing::info!("Daemon stopped");

    Ok(())
}
