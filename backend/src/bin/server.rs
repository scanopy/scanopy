use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use axum::{
    Extension, Router,
    http::{HeaderValue, Method},
    middleware,
};
use axum_client_ip::ClientIpSource;
use clap::Parser;
use reqwest::header::{self, HeaderName};
use scanopy::server::{
    auth::middleware::{
        demo_mode::demo_mode_middleware, logging::request_logging_middleware,
        rate_limit::rate_limit_middleware,
    },
    billing::plans::get_purchasable_plans,
    config::{AppState, ServerCli, ServerConfig, get_deployment_type},
    shared::handlers::{
        cache::AppCache,
        factory::{create_public_share_routes, create_router},
    },
};
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Log target for consistent server logging output
pub const LOG_TARGET: &str = "server";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();

    let cli = ServerCli::parse();

    // Load configuration using figment
    let config = ServerConfig::load(cli)?;
    let listen_addr = format!("0.0.0.0:{}", &config.server_port);
    let web_external_path = config.web_external_path.clone();
    let client_ip_source = config.client_ip_source.clone();
    let public_url = config.public_url.clone();
    let log_level = config.log_level.clone();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(format!(
            "scanopy={},server={},request_log={}",
            config.log_level, config.log_level, config.log_level
        )))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Startup banner
    tracing::info!(target: LOG_TARGET, "");
    tracing::info!(target: LOG_TARGET, "   _____                                   ");
    tracing::info!(target: LOG_TARGET, "  / ___/_________ _____  ____  ____  __  __");
    tracing::info!(target: LOG_TARGET, "  \\__ \\/ ___/ __ `/ __ \\/ __ \\/ __ \\/ / / /");
    tracing::info!(target: LOG_TARGET, " ___/ / /__/ /_/ / / / / /_/ / /_/ / /_/ / ");
    tracing::info!(target: LOG_TARGET, "/____/\\___/\\__,_/_/ /_/\\____/ .___/\\__, /  ");
    tracing::info!(target: LOG_TARGET, "                           /_/    /____/   ");
    tracing::info!(target: LOG_TARGET, "");
    tracing::info!(target: LOG_TARGET, "Scanopy Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!(target: LOG_TARGET, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    tracing::info!(target: LOG_TARGET, "Initializing...");
    tracing::info!(target: LOG_TARGET, "  Connecting to database...");

    // Create app state (database + services)
    let state = AppState::new(config).await?;
    tracing::info!(target: LOG_TARGET, "  Database connected, migrations applied");
    tracing::info!(target: LOG_TARGET, "  Services initialized");

    let discovery_service = state.services.discovery_service.clone();
    let billing_service = state.services.billing_service.clone();
    let deployment_type = get_deployment_type(state.clone());

    // Create discovery cleanup task
    let discovery_cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;

            // Check for timeouts (fail sessions running > 10 minutes)
            // discovery_cleanup_state.discovery_manager.check_timeouts(10).await;

            // Clean up old sessions (remove completed sessions > 24 hours old)
            discovery_cleanup_state
                .services
                .discovery_service
                .cleanup_old_sessions(24)
                .await;
        }
    });

    // Create stalled discovery cleanup task
    let stalled_discovery_cleanup = discovery_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Every minute
        loop {
            interval.tick().await;
            stalled_discovery_cleanup.cleanup_stalled_sessions().await;
        }
    });

    // Create auth session cleanup task
    let auth_cleanup_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15 * 60)); // 15 minutes
        loop {
            interval.tick().await;
            auth_cleanup_state
                .services
                .auth_service
                .cleanup_old_login_attempts()
                .await;
        }
    });

    // Create invite link cleanup task
    let invite_service_cleanup = state.services.invite_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15 * 60)); // 15 minutes
        loop {
            interval.tick().await;
            invite_service_cleanup.cleanup_expired().await;
        }
    });

    // Start daemon polling loop for ServerPoll mode daemons
    let daemon_service = state.services.daemon_service.clone();
    tokio::spawn(async move {
        daemon_service.start_polling_loop().await;
    });

    tracing::info!(target: LOG_TARGET, "  Background tasks started");

    let (base_router, _openapi) = create_router(state.clone());
    let base_router = base_router.with_state(state.clone());
    tracing::info!(target: LOG_TARGET, "  Routes registered");

    let api_router = if let Some(static_path) = &web_external_path {
        tracing::debug!(target: LOG_TARGET, "  Serving web assets from {:?}", static_path);
        base_router.fallback_service(
            ServeDir::new(static_path)
                .append_index_html_on_directories(true)
                .fallback(ServeFile::new(format!(
                    "{}/index.html",
                    static_path.display()
                ))),
        )
    } else {
        tracing::debug!(target: LOG_TARGET, "  Web assets not configured (API-only mode)");
        base_router
    };

    let session_store = state.session_store.clone();

    let cors = if cfg!(debug_assertions) {
        // Development: Allow localhost with credentials
        CorsLayer::new()
            .allow_origin([
                "http://localhost:5173".parse::<HeaderValue>().unwrap(),
                "http://localhost:60072".parse::<HeaderValue>().unwrap(),
                "http://localhost:60073".parse::<HeaderValue>().unwrap(),
            ])
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
            .allow_credentials(true)
    } else {
        // Production: Restrict to public_url origin
        let parsed_url = url::Url::parse(&public_url).expect("Invalid public_url");
        let origin_str = format!("{}://{}", parsed_url.scheme(), parsed_url.authority());
        let public_origin: HeaderValue =
            origin_str.parse().expect("Invalid origin from public_url");

        CorsLayer::new()
            .allow_origin(public_origin)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
            .allow_credentials(true)
    };

    // Permissive CORS for public share routes (used by embeds on customer domains)
    // No allow_credentials — public share routes are unauthenticated
    let public_cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    let client_ip_source = client_ip_source
        .map(|s| ClientIpSource::from_str(&s))
        .unwrap_or(Ok(ClientIpSource::ConnectInfo))?;

    let cache_headers = SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate, private"),
    );

    // Security headers
    let content_type_options = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );

    let referrer_policy = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Clickjacking protection - prevents the app from being embedded in iframes
    // Share endpoints override this with their own frame-ancestors based on allowed_domains
    let frame_ancestors = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("frame-ancestors 'self'"),
    );

    // CSP Report-Only: non-blocking header that logs violations to browser console.
    // Used to identify what a full CSP would break before enforcing it.
    let csp_report_only = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("content-security-policy-report-only"),
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: blob:; \
             font-src 'self'; \
             connect-src 'self'; \
             frame-ancestors 'self'; \
             base-uri 'self'; \
             form-action 'self'",
        ),
    );

    let app_cache = Arc::new(AppCache::new());

    // Create main app with all middleware
    let protected_app = Router::new().merge(api_router).layer(
        ServiceBuilder::new()
            .layer(client_ip_source.into_extension())
            .layer(TraceLayer::new_for_http())
            .layer(cors.clone())
            .layer(session_store)
            .layer(middleware::from_fn_with_state(
                state.clone(),
                rate_limit_middleware,
            ))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                demo_mode_middleware,
            ))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                request_logging_middleware,
            ))
            .layer(Extension(app_cache))
            .layer(cache_headers)
            .layer(content_type_options)
            .layer(referrer_policy)
            .layer(frame_ancestors)
            .layer(csp_report_only),
    );

    // Add HSTS header when secure cookies are enabled (indicates HTTPS is in use)
    let protected_app = if state.config.use_secure_session_cookies {
        protected_app.layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
    } else {
        protected_app
    };

    // Public share routes with permissive CORS (embeds on customer domains)
    let (public_share_router, _) = create_public_share_routes().split_for_parts();
    let public_share_app = Router::new()
        .merge(public_share_router)
        .with_state(state.clone())
        .layer(public_cors);

    // Health check endpoint without middleware (for kamal-proxy health checks)
    // Metrics endpoint is now at /api/metrics with external service auth (see factory.rs)
    let app = Router::new()
        .route(
            "/api/health",
            axum::routing::get(|| async {
                axum::Json(serde_json::json!({
                    "success": true,
                    "data": format!("Scanopy Server {}", env!("CARGO_PKG_VERSION")),
                    "error": null
                }))
            }),
        )
        .with_state(state.clone())
        .layer(cors)
        .merge(public_share_app)
        .merge(protected_app);
    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;

    // Spawn server in background
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    // Start discovery scheduler
    discovery_service.start_scheduler().await?;

    // Initialize billing if configured
    if let Some(billing_service) = billing_service {
        billing_service
            .initialize_products(get_purchasable_plans())
            .await?;
        tracing::info!(target: LOG_TARGET, "  Billing service initialized");
    }

    // Sync existing organizations to Brevo if configured
    if let Some(brevo_service) = state.services.brevo_service.clone() {
        tracing::info!(target: LOG_TARGET, "  Spawning Brevo organization sync task");
        tokio::spawn(async move {
            if let Err(e) = brevo_service.sync_existing_organizations().await {
                tracing::error!(target: LOG_TARGET, error = %e, "Failed to sync existing organizations to Brevo");
            }
        });
    } else {
        tracing::info!(target: LOG_TARGET, "  Brevo service not configured, skipping org sync");
    }

    // Configuration summary
    tracing::info!(target: LOG_TARGET, "Configuration:");
    tracing::info!(target: LOG_TARGET, "  Listen:          {}", listen_addr);
    tracing::info!(target: LOG_TARGET, "  Public URL:      {}", public_url);
    tracing::info!(target: LOG_TARGET, "  Log level:       {}", log_level);
    tracing::info!(target: LOG_TARGET, "  Deployment:      {:?}", deployment_type);
    if web_external_path.is_some() {
        tracing::info!(target: LOG_TARGET, "  Web UI:          enabled");
    } else {
        tracing::info!(target: LOG_TARGET, "  Web UI:          disabled (API-only)");
    }
    if state.config.integrated_daemon_url.is_some() {
        tracing::info!(target: LOG_TARGET, "  Integrated daemon: {}", state.config.integrated_daemon_url.as_ref().unwrap());
    }
    if state.config.stripe_secret.is_some() {
        tracing::info!(target: LOG_TARGET, "  Billing:         enabled");
    }
    if state.services.oidc_service.is_some() {
        tracing::info!(target: LOG_TARGET, "  OIDC:            enabled");
    }
    if state.config.use_secure_session_cookies {
        tracing::info!(target: LOG_TARGET, "  Secure cookies:  enabled (HTTPS)");
    }
    if state.config.metrics_token.is_some() && state.config.external_service_allowed_ips.is_empty()
    {
        tracing::warn!(target: LOG_TARGET, "  WARNING: metrics_token is set but external_service_allowed_ips is empty — all IPs can access metrics endpoint");
    }

    // Ready message
    tracing::info!(target: LOG_TARGET, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!(target: LOG_TARGET, "Server ready");
    tracing::info!(target: LOG_TARGET, "  API:             {}/api", public_url);
    if web_external_path.is_some() {
        tracing::info!(target: LOG_TARGET, "  Web UI:          {}", public_url);
    }
    tracing::info!(target: LOG_TARGET, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    tracing::info!(target: LOG_TARGET, "Shutdown signal received");
    tracing::info!(target: LOG_TARGET, "Server stopped");

    Ok(())
}
