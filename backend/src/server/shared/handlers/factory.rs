use crate::server::auth::middleware::billing::require_billing_for_users;
use crate::server::auth::middleware::fixture_capture::capture_fixtures_middleware;
use crate::server::config::{__path_get_public_config, get_public_config};
use crate::server::github::handlers::{__path_get_stars, get_stars};
use crate::server::openapi::create_docs_router;
use crate::server::shared::types::api::ApiResponse;
use crate::server::shared::types::metadata::{__path_get_metadata_registry, get_metadata_registry};
use crate::server::{
    auth::handlers as auth_handlers, billing::handlers as billing_handlers,
    bindings::handlers as binding_handlers, config::AppState,
    daemon_api_keys::handlers as daemon_api_key_handlers, daemons::handlers as daemon_handlers,
    dashboard::handlers as dashboard_handlers, discovery::handlers as discovery_handlers,
    groups::handlers as group_handlers, hosts::handlers as host_handlers,
    if_entries::handlers as if_entry_handlers, interfaces::handlers as interface_handlers,
    invites::handlers as invite_handlers, metrics::handlers as metrics_handlers,
    networks::handlers as network_handlers, organizations::handlers as organization_handlers,
    ports::handlers as port_handlers, services::handlers as service_handlers,
    shares::handlers as share_handlers, snmp_credentials::handlers as snmp_credential_handlers,
    subnets::handlers as subnet_handlers, tags::handlers as tag_handlers,
    topology::handlers as topology_handlers, user_api_keys::handlers as user_api_key_handlers,
    users::handlers as user_handlers,
};
use axum::Json;
use axum::Router;
use axum::http::HeaderValue;
use axum::middleware;
use reqwest::header;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::set_header::SetResponseHeaderLayer;
use utoipa::ToSchema;
use utoipa::openapi::OpenApi;
use utoipa_axum::router::OpenApiRouter;

/// Version information for API compatibility checking
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VersionInfo {
    /// Current API version (integer, increments on breaking changes)
    pub api_version: u32,
    /// Server version (semver)
    #[schema(value_type = String, example = "0.12.10")]
    pub server_version: Version,
    /// Minimum client version that can use this API (optional, for future use)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub min_compatible_client: Option<Version>,
}

/// Get API version information
#[utoipa::path(
    get,
    path = "/api/version",
    tags = ["system", "internal"],
    responses(
        (status = 200, description = "Version information", body = ApiResponse<VersionInfo>)
    )
)]
pub async fn get_version() -> Json<ApiResponse<VersionInfo>> {
    Json(ApiResponse::success(VersionInfo {
        api_version: 1,
        server_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
        min_compatible_client: None,
    }))
}

/// Creates the OpenApiRouter with billed entity routes.
/// All entity routes are versioned under /api/v1/.
fn create_billed_openapi_routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .nest("/api/v1/hosts", host_handlers::create_router())
        .nest("/api/v1/interfaces", interface_handlers::create_router())
        .nest("/api/v1/subnets", subnet_handlers::create_router())
        .nest("/api/v1/networks", network_handlers::create_router())
        .nest("/api/v1/groups", group_handlers::create_router())
        .nest("/api/v1/daemons", daemon_handlers::create_router())
        .nest("/api/v1/dashboard", dashboard_handlers::create_router())
        .nest("/api/v1/discovery", discovery_handlers::create_router())
        .nest("/api/v1/services", service_handlers::create_router())
        .nest("/api/v1/users", user_handlers::create_router())
        .nest(
            "/api/v1/organizations",
            organization_handlers::create_router(),
        )
        .nest("/api/v1/invites", invite_handlers::create_router())
        .nest("/api/v1/tags", tag_handlers::create_router())
        .nest("/api/v1/ports", port_handlers::create_router())
        .nest("/api/v1/bindings", binding_handlers::create_router())
        // API key routes (versioned)
        .nest("/api/v1/auth/keys", user_api_key_handlers::create_router())
        .nest(
            "/api/v1/auth/daemon",
            daemon_api_key_handlers::create_router(),
        )
        // SNMP entity routes
        .nest(
            "/api/v1/snmp-credentials",
            snmp_credential_handlers::create_router(),
        )
        .nest("/api/v1/if-entries", if_entry_handlers::create_router())
        // Topology endpoints (tagged as internal - hidden from public docs)
        .nest("/api/v1/topology", topology_handlers::create_router())
}

/// Creates the OpenApiRouter with exempt routes (not subject to billing middleware).
/// Shares are versioned (user-facing), auth and billing are unversioned.
fn create_exempt_openapi_routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .nest("/api/billing", billing_handlers::create_router())
        .nest("/api/v1/shares", share_handlers::create_router())
        .nest("/api/auth", auth_handlers::create_router())
        .nest("/api/daemons", daemon_handlers::create_internal_router())
        .routes(utoipa_axum::routes!(get_version))
        // Metrics endpoint for Prometheus scraping (external service auth)
        .route(
            "/api/metrics",
            axum::routing::get(metrics_handlers::get_metrics),
        )
}

/// Creates the public share router (unauthenticated, permissive CORS).
/// Separated from the main router so these routes can use different CORS policy.
pub fn create_public_share_routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().nest("/api/v1/shares", share_handlers::create_public_router())
}

/// Creates the OpenApiRouter with cacheable routes.
fn create_cacheable_openapi_routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(utoipa_axum::routes!(get_metadata_registry))
        .routes(utoipa_axum::routes!(get_public_config))
        .routes(utoipa_axum::routes!(get_stars))
}

/// Collects all OpenAPI route definitions without building the actual router.
/// This is the single source of truth for OpenAPI spec generation.
/// Used by both the server and standalone spec generation.
pub fn collect_all_openapi_routes() -> OpenApi {
    let (_, mut openapi) = create_billed_openapi_routes().split_for_parts();
    let (_, exempt_openapi) = create_exempt_openapi_routes().split_for_parts();
    let (_, cacheable_openapi) = create_cacheable_openapi_routes().split_for_parts();
    let (_, public_share_openapi) = create_public_share_routes().split_for_parts();

    openapi.merge(exempt_openapi);
    openapi.merge(cacheable_openapi);
    openapi.merge(public_share_openapi);
    openapi
}

/// Creates the application router and returns both the router and OpenAPI spec.
/// The OpenAPI spec is built from annotated handlers using utoipa-axum.
pub fn create_router(state: Arc<AppState>) -> (Router<Arc<AppState>>, OpenApi) {
    // Get the complete OpenAPI spec from single source of truth
    let openapi = collect_all_openapi_routes();

    // Build routers from the same helper functions (discarding their OpenAPI output)
    // Billed routes require billing middleware
    let (billed_router, _) = create_billed_openapi_routes().split_for_parts();
    let billed_router = billed_router.layer(middleware::from_fn_with_state(
        state,
        require_billing_for_users,
    ));

    // Exempt routes (no billing middleware)
    let (exempt_router, _) = create_exempt_openapi_routes().split_for_parts();

    // Legacy routes for backwards compatibility with older daemons (v0.12.x)
    // These are not documented in OpenAPI but must remain functional
    let legacy_entity_router: Router<Arc<AppState>> = Router::new()
        .nest("/api/hosts", host_handlers::create_router().into())
        .nest("/api/subnets", subnet_handlers::create_router().into())
        .nest("/api/services", service_handlers::create_router().into())
        .nest("/api/groups", group_handlers::create_router().into())
        .nest("/api/discovery", discovery_handlers::create_router().into());

    // Cacheable routes with cache headers
    let (cacheable_router, _) = create_cacheable_openapi_routes().split_for_parts();
    let cacheable_routes = cacheable_router.layer(SetResponseHeaderLayer::if_not_present(
        header::CACHE_CONTROL,
        HeaderValue::from_static("max-age=3600, must-revalidate"),
    ));

    let router = Router::new()
        .merge(billed_router)
        .merge(exempt_router)
        .merge(legacy_entity_router)
        .merge(cacheable_routes)
        .merge(create_docs_router(openapi.clone()))
        // Fixture capture middleware (no-op unless capture-fixtures feature is enabled)
        .layer(middleware::from_fn(capture_fixtures_middleware));

    (router, openapi)
}
