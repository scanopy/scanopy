use std::{collections::HashSet, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

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
    config::{AppState, DeploymentType, ServerCli, ServerConfig, get_deployment_type},
    license::middleware::license_guard_middleware,
    shared::handlers::{
        cache::AppCache,
        factory::{create_public_share_routes, create_router},
    },
    shared::web_assets::{self, WebUi},
    shares::handlers::{ShareIndexHtml, share_html_handler},
};
use tower::ServiceBuilder;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Log target for consistent server logging output
pub const LOG_TARGET: &str = "server";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let cli = ServerCli::parse();

    // Load configuration using figment
    let config = ServerConfig::load(cli)?;
    let oidc_domains: HashSet<String> = config
        .oidc_providers
        .clone()
        .unwrap_or_default()
        .iter()
        .filter_map(|o| {
            o.logo.as_ref().and_then(|u| {
                let url = url::Url::parse(u).ok()?;
                let host = url.host_str()?;

                Some(format!("{}://{}", url.scheme(), host))
            })
        })
        .collect();
    let listen_addr = format!("0.0.0.0:{}", &config.server_port);
    let web_external_path = config.web_external_path.clone();
    let client_ip_source = config.client_ip_source.clone();
    let public_url = config.public_url.clone();
    let log_level = config.log_level.clone();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(format!(
            "scanopy={lvl},server={lvl},request_log={lvl},events={lvl}",
            lvl = config.log_level
        )))
        .with(
            tracing_subscriber::fmt::layer()
                .fmt_fields(scanopy::server::logging::format::LabelFields)
                .with_ansi(scanopy::server::logging::format::supports_ansi()),
        )
        .init();

    let (web_ui, web_ui_warning) = web_assets::resolve(
        web_external_path.as_deref(),
        web_assets::is_embedded(),
        cfg!(debug_assertions),
    );

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
    let deployment_type = get_deployment_type(&state.config);

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

            // Sweep transient rescan rows whose session never reached a terminal
            // phase (server restart mid-scan). 24h is comfortably past the 6h
            // max_discovery_duration a queued rescan could wait on.
            discovery_cleanup_state
                .services
                .discovery_service
                .sweep_orphaned_rescans(24)
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
    let poll_email_service = state.services.email_service.clone();
    tokio::spawn(async move {
        daemon_service.start_polling_loop(poll_email_service).await;
    });

    // Check for inactive daemons and put them on standby (daily)
    let inactivity_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            if let Err(e) = inactivity_state
                .services
                .daemon_service
                .check_daemon_inactivity(inactivity_state.services.email_service.as_deref())
                .await
            {
                tracing::error!(error = %e, "Failed to check daemon inactivity");
            }
        }
    });

    // One-shot daemon-sunset announcement at boot. Emails orgs whose active
    // daemons run a version below the announced sunset floor; an org-level
    // ratchet suppresses re-sends across restarts. The eligible set only changes
    // when the server binary (and thus its release-line table) changes, so boot
    // is the natural trigger. No-op while the sunset machinery is dormant.
    if let Some(sunset_email) = state.services.email_service.clone() {
        tokio::spawn(async move {
            if let Err(e) = sunset_email.announce_daemon_sunsets().await {
                tracing::error!(error = %e, "Daemon sunset announcement sweep failed");
            }
        });
    }

    // Snapshot retention sweep (daily). Deletes snapshots past the per-plan
    // retention window; the FK cascade reaps closed entity rows + topology
    // rows tied to each deleted snapshot.
    let retention_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            if let Err(e) = retention_state
                .services
                .snapshot_service
                .run_retention(retention_state.config.snapshot_retention_days_override)
                .await
            {
                tracing::error!(error = %e, "Snapshot retention sweep failed");
            }
        }
    });

    // Air-gapped licence expiry warnings (daily). An air-gapped customer's
    // server never calls home and Stripe's upcoming-invoice event does not
    // fire for invoice-billed subscriptions, so this sweep is the only thing
    // that reaches them before a renewal bills. Ratcheted per licence period,
    // so a daily tick sends at most one email per organization per period.
    if let Some(airgap_email) = state.services.email_service.clone() {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
            loop {
                interval.tick().await;
                if let Err(e) = airgap_email.warn_expiring_airgap_keys().await {
                    tracing::error!(error = %e, "Air-gapped licence expiry sweep failed");
                }
            }
        });
    }

    // License key periodic re-validation (every 5 minutes). Only runs when a
    // license key is configured — keyless deployments have no license service.
    if let Some(license_revalidate) = state.license_service.clone() {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            loop {
                interval.tick().await;
                license_revalidate.revalidate().await;
            }
        });
    }

    // Online license keys fetch their entitlement from Scanopy Cloud at
    // startup and every 6 hours after.
    if let Some(license_check_in) = state.license_service.clone()
        && license_check_in.is_online()
    {
        tokio::spawn(license_check_in.run_check_ins());
    }

    tracing::info!(target: LOG_TARGET, "  Background tasks started");

    let (base_router, _openapi) = create_router(state.clone());
    let base_router = base_router.with_state(state.clone());
    tracing::info!(target: LOG_TARGET, "  Routes registered");

    let api_router = match &web_ui {
        WebUi::External { path, .. } => {
            tracing::debug!(target: LOG_TARGET, "  Serving web assets from {:?}", path);
            base_router.fallback_service(
                ServeDir::new(path)
                    .append_index_html_on_directories(true)
                    .fallback(ServeFile::new(path.join("index.html"))),
            )
        }
        WebUi::Embedded => {
            tracing::debug!(target: LOG_TARGET, "  Serving web assets compiled into the binary");
            base_router.fallback(web_assets::fallback_handler)
        }
        WebUi::Disabled => {
            tracing::debug!(target: LOG_TARGET, "  Web assets not configured (API-only mode)");
            base_router
        }
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
        // Production: Only allow app and demo origins. API keys are server-side only;
        // browser clients should proxy through their own backend.
        let parsed_url = url::Url::parse(&public_url).expect("Invalid public_url");
        let origin_str = format!("{}://{}", parsed_url.scheme(), parsed_url.authority());
        let public_origin: HeaderValue =
            origin_str.parse().expect("Invalid origin from public_url");
        let demo_origin: HeaderValue = "https://demo.scanopy.net".parse().unwrap();

        CorsLayer::new()
            .allow_origin(AllowOrigin::list([public_origin, demo_origin]))
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

    let oidc_logo_sources = oidc_domains.iter().cloned().collect::<Vec<_>>().join(" ");

    let csp_value = format!(
        "default-src 'self'; \
        script-src 'self' 'unsafe-inline' https://ph.scanopy.net https://js.stripe.com; \
        style-src 'self' 'unsafe-inline'; \
        img-src 'self' data: blob: {oidc_logo_sources}; \
        font-src 'self'; \
        connect-src 'self' https://ph.scanopy.net https://api.stripe.com; \
        frame-src 'self' https://demo.scanopy.net https://js.stripe.com https://hooks.stripe.com; \
        frame-ancestors 'self'; \
        base-uri 'self'; \
        form-action 'self'",
        oidc_logo_sources = oidc_logo_sources
    );

    let csp = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_str(&csp_value).expect("OIDC values will not change during runtime"),
    );

    let app_cache = Arc::new(AppCache::new());

    // Create main app with all middleware
    let protected_app = Router::new().merge(api_router).layer(
        ServiceBuilder::new()
            .layer(client_ip_source.into_extension())
            .layer(TraceLayer::new_for_http())
            .layer(cors)
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
                license_guard_middleware,
            ))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                request_logging_middleware,
            ))
            .layer(Extension(app_cache))
            .layer(cache_headers.clone())
            .layer(content_type_options)
            .layer(referrer_policy)
            .layer(csp),
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
    let mut public_share_app: Router<Arc<AppState>> = Router::new().merge(public_share_router);

    // Mount /share/{id} and /share/{id}/embed on the public share app so a
    // per-share, tight CSP applies to the HTML document. The SPA's
    // `PasswordGate` and error views still render client-side. Leaving
    // these to protected_app's fallback ServeDir would ship them with the
    // global (looser) CSP, so we intercept the HTML here.
    // The embedded branch serves the compiled-in copy, so a standalone binary
    // serves share links with the same per-share CSP a container deploy does.
    if let Some(index_html) = web_ui.share_index_html() {
        let share_index = ShareIndexHtml(Arc::new(index_html));

        public_share_app = public_share_app
            .route("/share/{id}", axum::routing::get(share_html_handler))
            .route("/share/{id}/embed", axum::routing::get(share_html_handler))
            .layer(Extension(share_index));
    }

    let public_share_app = public_share_app
        .with_state(state.clone())
        .layer(public_cors)
        .layer(cache_headers.clone());

    // Permissive CORS for health check (marketing site, uptime monitors, etc.)
    let health_cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    // Liveness (/api/health): static response, answers "is the process alive?"
    // Used by kamal-proxy and similar load-balancer health checks — must not depend on DB.
    // Readiness (/api/health/ready): runs a cheap DB probe, answers "can this process serve
    // traffic?" Used by the release backward-compat container harness to verify that the
    // currently-deployed binary can still execute SQL against a freshly-migrated schema.
    // Metrics endpoint is at /api/metrics with external service auth (see factory.rs).
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
        .route(
            "/api/health/ready",
            axum::routing::get(
                |axum::extract::State(state): axum::extract::State<Arc<AppState>>| async move {
                    // Fail fast: sqlx's default pool acquire_timeout is 30s, which makes a
                    // down-DB readiness probe look like a hang to callers (k8s, kamal,
                    // compat-check.sh). 5s leaves headroom for cold TCP+TLS handshake to
                    // managed Postgres (e.g., Neon) without flapping on transient blips.
                    let probe = tokio::time::timeout(
                        Duration::from_secs(5),
                        sqlx::query("SELECT 1").execute(&state.pool),
                    )
                    .await;
                    match probe {
                        Ok(Ok(_)) => (
                            axum::http::StatusCode::OK,
                            axum::Json(serde_json::json!({
                                "success": true,
                                "data": "ready",
                                "error": null
                            })),
                        ),
                        Ok(Err(e)) => (
                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                            axum::Json(serde_json::json!({
                                "success": false,
                                "data": null,
                                "error": format!("database unavailable: {}", e)
                            })),
                        ),
                        Err(_) => (
                            axum::http::StatusCode::SERVICE_UNAVAILABLE,
                            axum::Json(serde_json::json!({
                                "success": false,
                                "data": null,
                                "error": "database unavailable: probe timed out"
                            })),
                        ),
                    }
                },
            ),
        )
        .with_state(state.clone())
        .layer(health_cors)
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
        let org_sync_service = brevo_service.clone();
        tokio::spawn(async move {
            if let Err(e) = org_sync_service.sync_existing_organizations().await {
                tracing::error!(target: LOG_TARGET, error = %e, "Failed to sync existing organizations to Brevo");
            }
        });

        // Ephemeral release code — remove next release: one-shot backfill of
        // email-domain-classification contact attributes for existing users.
        tracing::info!(target: LOG_TARGET, "  Spawning Brevo domain-classification backfill task");
        let domain_backfill_service = brevo_service.clone();
        tokio::spawn(async move {
            if let Err(e) = domain_backfill_service
                .backfill_domain_classifications()
                .await
            {
                tracing::error!(target: LOG_TARGET, error = %e, "Failed to backfill Brevo domain classifications");
            }
        });

        // Ephemeral release code, remove next release: one-shot backfill of
        // SCANOPY_LICENSED_PLAN for contacts of orgs already on a licensed plan.
        tracing::info!(target: LOG_TARGET, "  Spawning Brevo licensed-plan contact backfill task");
        tokio::spawn(async move {
            if let Err(e) = brevo_service.backfill_licensed_plan_contacts().await {
                tracing::error!(target: LOG_TARGET, error = %e, "Failed to backfill Brevo licensed-plan contacts");
            }
        });
    }

    // Reconcile self-hosted org plan(s) to the license entitlement. An offline
    // key is env-only, so a key added/changed after orgs were provisioned only
    // takes effect on restart — this is that moment. An online key also
    // reconciles on every entitlement swap. Moves every org onto the
    // license-resolved tier (any direction), idempotently, and only while the
    // license is `Valid`. Cloud has no license service (the key is dropped via
    // `effective_license_key`), so reconciliation never runs there.
    if let Some(license_service) = state.license_service.clone() {
        tracing::info!(target: LOG_TARGET, "  Spawning self-hosted license plan reconciliation task");
        tokio::spawn(async move { license_service.reconcile_plans().await });
    }

    // Configuration summary
    tracing::info!(target: LOG_TARGET, "Configuration:");
    tracing::info!(target: LOG_TARGET, "  Listen:          {}", listen_addr);
    tracing::info!(target: LOG_TARGET, "  Public URL:      {}", public_url);
    tracing::info!(target: LOG_TARGET, "  Log level:       {}", log_level);
    tracing::info!(target: LOG_TARGET, "  Deployment:      {:?}", deployment_type);
    if let Some(contact) = &state.config.server_admin_contact_email {
        tracing::info!(target: LOG_TARGET, "  Admin contact:   {}", contact);
    }
    // Licensing is a self-hosted concept; on cloud the key is ignored entirely,
    // so don't log a (misleading) License line there.
    if deployment_type != DeploymentType::Cloud {
        match &state.license_service {
            None => {
                tracing::info!(target: LOG_TARGET, "  License:         not required (community)");
            }
            Some(license_service) => {
                let license_status = license_service.current_status().await;
                match &license_status {
                    scanopy::server::license::types::LicenseStatus::Valid(claims) => {
                        let intended_exp = chrono::DateTime::from_timestamp(claims.intended_exp, 0)
                            .map(|d| d.format("%Y-%m-%d").to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        if license_status.in_grace_period() {
                            let hard_exp = chrono::DateTime::from_timestamp(claims.exp, 0)
                                .map(|d| d.format("%Y-%m-%d").to_string())
                                .unwrap_or_else(|| "unknown".to_string());
                            tracing::warn!(
                                target: LOG_TARGET,
                                "  License:         GRACE PERIOD (expired {}, hard lockout {})",
                                intended_exp,
                                hard_exp,
                            );
                        } else {
                            tracing::info!(
                                target: LOG_TARGET,
                                "  License:         valid (expires {})",
                                intended_exp,
                            );
                        }
                    }
                    scanopy::server::license::types::LicenseStatus::Expired(_) => {
                        tracing::warn!(target: LOG_TARGET, "  License:         EXPIRED — server is in read-only mode");
                    }
                    scanopy::server::license::types::LicenseStatus::Invalid(reason) => {
                        tracing::error!(target: LOG_TARGET, "  License:         INVALID ({}) — server is in read-only mode", reason);
                    }
                    scanopy::server::license::types::LicenseStatus::Pending => {
                        tracing::warn!(
                            target: LOG_TARGET,
                            "  License:         PENDING — server is in read-only mode until it reaches {} for its first entitlement",
                            scanopy::server::license::service::CLOUD_BASE_URL,
                        );
                    }
                }
            }
        }
    }
    match &web_ui {
        WebUi::External { path, .. } => {
            tracing::info!(target: LOG_TARGET, "  Web UI:          enabled (from {})", path.display());
        }
        WebUi::Embedded => {
            tracing::info!(target: LOG_TARGET, "  Web UI:          enabled (compiled into binary)");
        }
        WebUi::Disabled => {
            tracing::info!(target: LOG_TARGET, "  Web UI:          disabled (API-only)");
        }
    }
    if let Some(warning) = &web_ui_warning {
        tracing::warn!(target: LOG_TARGET, "  WARNING: {}", warning);
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
    if state.config.disable_password_login {
        tracing::info!(target: LOG_TARGET, "  Password login:  disabled");
        if state.services.oidc_service.is_none() {
            tracing::warn!(target: LOG_TARGET, "  Password login is disabled but no OIDC providers are configured!");
        }
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
    if web_ui.is_enabled() {
        tracing::info!(target: LOG_TARGET, "  Web UI:          {}", public_url);
    }
    tracing::info!(target: LOG_TARGET, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    tracing::info!(target: LOG_TARGET, "Shutdown signal received");
    tracing::info!(target: LOG_TARGET, "Server stopped");

    Ok(())
}
