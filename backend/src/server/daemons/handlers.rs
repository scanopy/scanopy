use crate::daemon::runtime::state::DaemonStatus;
use crate::daemon::shared::config::{DaemonArgs, parse_integration_target_tokens};
use crate::server::auth::middleware::permissions::{
    And, Authorized, IsDaemon, IsUser, Member, Viewer,
};
use crate::server::credentials::r#impl::mapping::IntegrationTarget;
use crate::server::daemons::r#impl::api::{
    DaemonDiscoveryRequest, DaemonHeartbeatPayload, ProvisionDaemonRequest,
    ProvisionDaemonResponse, TestReachabilityRequest, TestReachabilityResponse,
};
use crate::server::openapi::tags as api_tags;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::{CrudHandlers, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::ApiErrorResponse;
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::shared::validation::validate_network_access;
use crate::server::{
    config::AppState,
    daemons::r#impl::{
        api::{
            DaemonRegistrationRequest, DaemonRegistrationResponse, DaemonResponse,
            DaemonStartupRequest, LegacyCapabilities, ServerCapabilities,
        },
        base::{Daemon, DaemonMode},
        install_artifacts::{InstallCommandKind, WINDOWS_MSI_URL},
        version::DaemonVersionPolicy,
    },
    shared::{
        handlers::cache::AppCache,
        types::api::{ApiError, ApiResponse, ApiResult, EmptyApiResponse, PaginatedApiResponse},
    },
};
use axum::http::StatusCode;
use axum::{
    Extension,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Daemon Ordering
// ============================================================================

/// Fields that daemons can be ordered/grouped by.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DaemonOrderField {
    #[default]
    CreatedAt,
    Name,
    LastSeen,
    UpdatedAt,
    NetworkId,
}

impl OrderField for DaemonOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "daemons.created_at",
            Self::Name => "daemons.name",
            Self::LastSeen => "daemons.last_seen",
            Self::UpdatedAt => "daemons.updated_at",
            Self::NetworkId => "daemons.network_id",
        }
    }
}

// ============================================================================
// Daemon Filter Query
// ============================================================================

/// Query parameters for filtering and ordering daemons.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct DaemonFilterQuery {
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<DaemonOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<DaemonOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl DaemonFilterQuery {
    /// Build the ORDER BY clause.
    pub fn apply_ordering(
        &self,
        filter: StorableFilter<Daemon>,
    ) -> (StorableFilter<Daemon>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "daemons.created_at ASC",
        )
    }
}

impl FilterQueryExtractor for DaemonFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        match self.network_id {
            Some(id) if user_network_ids.contains(&id) => filter.network_ids(&[id]),
            Some(_) => filter.network_ids(&[]), // User doesn't have access - return empty
            None => filter.network_ids(user_network_ids),
        }
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    crate::crud_delete_handler!(Daemon);
    crate::crud_bulk_delete_handler!(Daemon);
    crate::crud_export_csv_handler!(Daemon);
}

/// Returns a conflict error when trying to delete a daemon with active discovery sessions.
fn active_session_error() -> ApiError {
    ApiError::coded(
        StatusCode::CONFLICT,
        ErrorCode::EntityDeleteForbidden {
            entity: "daemon".to_string(),
            reason: Some("has active discovery sessions — cancel the discovery first".to_string()),
        },
    )
}

/// Update a Daemon
///
/// Edits the server-side daemon record: its name, maintainer, tags, and — for ServerPoll —
/// the url the server dials. Identity and server-managed fields (network, mode, host, key
/// binding, version, liveness) are restored from the existing record by
/// `preserve_immutable_fields`.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "update_daemon",
    summary = "Update daemon",
    params(("id" = Uuid, Path, description = "daemon ID")),
    request_body = Daemon,
    responses(
        (status = 200, description = "daemon updated", body = ApiResponse<Daemon>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
        (status = 404, description = "daemon not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_daemon(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(mut request): Json<Daemon>,
) -> ApiResult<Json<ApiResponse<Daemon>>> {
    let network_ids = auth.network_ids();

    let existing = Daemon::get_service(&state)
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    // Tenant isolation: the caller must have access to the daemon's *current* network.
    // network_id is restored below, so an update cannot move a daemon between networks.
    validate_network_access(Some(existing.base.network_id), &network_ids, "update")?;

    // A DaemonPoll daemon dials out and is never dialed, so its url is unused — silently
    // storing one would suggest a reachability that does not exist.
    if existing.base.mode == DaemonMode::DaemonPoll && request.base.url != existing.base.url {
        return Err(ApiError::bad_request(
            "url only applies to ServerPoll daemons; a DaemonPoll daemon dials the server",
        ));
    }

    request.preserve_immutable_fields(&existing);

    update_handler::<Daemon>(State(state), auth, Path(id), Json(request)).await
}

/// Query for [`get_install_command`]. `purpose` is required; the rest are the client-settable
/// advanced daemon settings, folded into an `install` command. Lists are comma-joined
/// (`interfaces`, `credential_refs`).
#[derive(Deserialize, Debug, Clone, IntoParams)]
pub struct InstallCommandQuery {
    /// `install` (with the api-key placeholder) or `reconfigure` (credential-free).
    pub purpose: InstallCommandKind,
    /// Log verbosity the daemon should run at (e.g. `info`, `debug`).
    pub log_level: Option<String>,
    /// Path the daemon should write its log file to.
    pub log_file: Option<String>,
    /// How often the daemon reports in, in seconds.
    pub heartbeat_interval: Option<u64>,
    /// Address and port the daemon should listen on, for server-polled mode.
    pub bind_address: Option<String>,
    /// Accept a self-signed certificate when connecting back to the server.
    pub allow_self_signed_certs: Option<bool>,
    /// Continue scanning targets that present an untrusted certificate.
    pub accept_invalid_scan_certs: Option<bool>,
    /// Comma-separated interface names.
    pub interfaces: Option<String>,
    /// Comma-separated credential/integration tokens (for the docker-compose env).
    pub credential_refs: Option<String>,
}

impl InstallCommandQuery {
    /// The advanced settings as a `DaemonArgs`. Only the client-settable fields are read; the
    /// server-controlled ones stay `None` and are filled from the record by the builder.
    fn install_config(&self) -> DaemonArgs {
        DaemonArgs {
            log_level: self.log_level.clone(),
            log_file: self.log_file.clone(),
            heartbeat_interval: self.heartbeat_interval,
            bind_address: self.bind_address.clone(),
            allow_self_signed_certs: self.allow_self_signed_certs,
            accept_invalid_scan_certs: self.accept_invalid_scan_certs,
            interfaces: self.interfaces.as_ref().map(|s| {
                s.split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect()
            }),
            ..Default::default()
        }
    }

    fn credential_refs(&self) -> Vec<IntegrationTarget> {
        self.credential_refs
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect::<Vec<_>>()
            })
            .and_then(|tokens| parse_integration_target_tokens(&tokens).ok())
            .unwrap_or_default()
    }
}

/// Generate a Daemon install command
///
/// A pure, idempotent builder — it never mints or persists anything. The api key in an `install`
/// command is a placeholder (`<API_KEY>`) the caller substitutes from the plaintext it holds; a
/// `reconfigure` command carries no key at all. Minting is a separate mutation
/// (`POST /provision`), so regenerating a command here (advanced-setting change, OS switch, the
/// Details reconfigure view) never rotates the daemon's key.
///
/// The server derives the exact command shape from the record: DaemonPoll vs ServerPoll for the
/// flags, and — for `install` — whether the daemon has checked in (`last_seen`) to decide between
/// a first-install and a minimal re-key command.
#[utoipa::path(
    get,
    path = "/{id}/install-command",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "get_daemon_install_command",
    summary = "Generate daemon install command",
    params(("id" = Uuid, Path, description = "daemon ID"), InstallCommandQuery),
    responses(
        (status = 200, description = "Install command", body = ApiResponse<crate::server::daemons::r#impl::install_artifacts::InstallArtifacts>),
        (status = 404, description = "daemon not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_install_command(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
    Query(query): Query<InstallCommandQuery>,
) -> ApiResult<Json<ApiResponse<crate::server::daemons::r#impl::install_artifacts::InstallArtifacts>>>
{
    let network_ids = auth.network_ids();

    let daemon = Daemon::get_service(&state)
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    validate_network_access(
        Some(daemon.base.network_id),
        &network_ids,
        "get install command",
    )?;

    let install_config = query.install_config();
    let artifacts = crate::server::daemons::r#impl::install_artifacts::build_install_artifacts(
        &state.config.public_url,
        &daemon,
        Some(&install_config),
        &query.credential_refs(),
        query.purpose,
    );

    Ok(Json(ApiResponse::success(artifacts)))
}

/// Cache key for the proxied Windows MSI bytes (see [`get_windows_msi`]).
const WINDOWS_MSI_CACHE_KEY: &str = "windows_msi_bytes";

/// Proxy the Windows daemon MSI through our own origin.
///
/// The MSI itself carries no tenant-specific data (it's already a public, unauthenticated GitHub
/// release asset) — only the *filename* a client saves it under is per-daemon, and that's encoded
/// entirely client-side by the caller from the value `get_install_command` already returned. This
/// endpoint exists solely so a same-origin download can set that filename: linking directly to
/// GitHub doesn't work because its signed asset URL bakes in a fixed `Content-Disposition`, which
/// browsers always prefer over an `<a download>` attribute.
#[utoipa::path(
    get,
    path = "/api/install/windows-msi",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "get_windows_msi",
    summary = "Download the Windows daemon MSI",
    description = "Proxies the Windows daemon MSI from GitHub so the browser can save it under a caller-chosen filename. Cached for an hour to avoid refetching from GitHub on every request.",
    responses(
        (status = 200, description = "Windows daemon MSI installer", content_type = "application/octet-stream", body = Vec<u8>),
        (status = 502, description = "Failed to fetch the MSI from GitHub", body = ApiErrorResponse),
    )
)]
pub async fn get_windows_msi(
    Extension(cache): Extension<Arc<AppCache>>,
) -> ApiResult<impl IntoResponse> {
    let bytes = match cache.get::<bytes::Bytes>(WINDOWS_MSI_CACHE_KEY).await {
        Some(cached) => cached,
        None => {
            let response = reqwest::get(WINDOWS_MSI_URL).await.map_err(|e| {
                ApiError::bad_gateway(format!("Failed to fetch MSI from GitHub: {e}"))
            })?;

            if !response.status().is_success() {
                return Err(ApiError::bad_gateway(format!(
                    "GitHub returned {} fetching the MSI",
                    response.status()
                )));
            }

            let fetched = response
                .bytes()
                .await
                .map_err(|e| ApiError::bad_gateway(format!("Failed to read MSI body: {e}")))?;

            cache.set(WINDOWS_MSI_CACHE_KEY, fetched.clone(), 1).await;
            fetched
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"scanopy-daemon-windows-amd64.msi\""),
    );

    Ok((headers, Body::from(bytes)))
}

/// Delete daemon — blocks if daemon has active discovery sessions.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "delete_daemon",
    summary = "Delete daemon",
    params(("id" = Uuid, Path, description = "daemon ID")),
    responses(
        (status = 200, description = "daemon deleted", body = EmptyApiResponse),
        (status = 404, description = "daemon not found", body = ApiErrorResponse),
        (status = 409, description = "daemon has active sessions", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_daemon(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    if state
        .services
        .discovery_service
        .has_active_session_for_daemon(&id)
        .await
    {
        return Err(active_session_error());
    }
    generated::delete(state, auth, Path(id)).await
}

/// Bulk delete daemons — blocks if any daemon has active discovery sessions.
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "bulk_delete_daemons",
    summary = "Bulk delete daemons",
    request_body(content = Vec<Uuid>, description = "Array of Daemon IDs to delete"),
    responses(
        (status = 200, description = "daemons deleted", body = ApiResponse<crate::server::shared::handlers::traits::BulkDeleteResponse>),
        (status = 409, description = "daemon has active sessions", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_delete_daemons(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(ids): Json<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<crate::server::shared::handlers::traits::BulkDeleteResponse>>> {
    for id in &ids {
        if state
            .services
            .discovery_service
            .has_active_session_for_daemon(id)
            .await
        {
            return Err(active_session_error());
        }
    }
    generated::bulk_delete(state, auth, Json(ids)).await
}

/// Operating system the install command was generated for.
#[derive(
    Debug, Clone, Copy, Deserialize, Serialize, strum_macros::IntoStaticStr, utoipa::ToSchema,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum DaemonOs {
    Linux,
    MacOS,
    Windows,
    FreeBsd,
}

/// Request body for emailing an install command to the authenticated user.
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct EmailInstallCommandRequest {
    /// The install command to send, exactly as shown in the UI.
    pub install_command: String,
    /// Operating system the command targets, used to pick the email wording.
    pub os: DaemonOs,
}

/// Email the install command to the authenticated user's email address.
///
/// Session-only, and `IsUser` says so at the extractor rather than in the body. "The current user"
/// has no answer for an automation identity: a user API key carries `user_id` but no address, so
/// this endpoint could never serve one. An API key that wants the command reads it directly from
/// `GET /api/v1/daemons/{id}/install-command`.
#[utoipa::path(
    post,
    path = "/email-install-command",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "email_install_command",
    summary = "Email install command to current user",
    request_body = EmailInstallCommandRequest,
    responses(
        (status = 200, description = "Email sent", body = EmptyApiResponse),
        (status = 400, description = "Email service not configured", body = ApiErrorResponse),
        (status = 403, description = "User session required", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
async fn email_install_command(
    State(state): State<Arc<AppState>>,
    auth: Authorized<And<Member, IsUser>>,
    Json(request): Json<EmailInstallCommandRequest>,
) -> ApiResult<Json<EmptyApiResponse>> {
    // Unreachable: `IsUser` has already rejected every variant that lacks an address, and the
    // `User` variant's email is a plain `EmailAddress`, not an `Option`. Kept as a fallback that
    // returns the same error the extractor would, so a future widening of the extractor fails
    // honestly instead of claiming the account has no email.
    let email = auth
        .entity
        .email()
        .cloned()
        .ok_or_else(ApiError::user_required)?;

    let email_service = state
        .services
        .email_service
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("Email service is not configured"))?;

    // `?` rather than a formatted `internal_error`: the latter put the mail server's raw
    // reply — relay hostname and provider diagnostics included — into the response body,
    // where the client's error middleware showed it to the user in a toast.
    email_service
        .send_install_command_email(email, &request.install_command, request.os.into())
        .await?;

    Ok(Json(ApiResponse::success(())))
}

/// User-facing daemon management endpoints (versioned at /api/v1/daemons)
pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all))
        .routes(routes!(get_by_id, update_daemon, delete_daemon))
        .routes(routes!(get_install_command))
        .routes(routes!(bulk_delete_daemons))
        .routes(routes!(generated::export_csv))
        .routes(routes!(provision_daemon))
        .routes(routes!(retry_connection))
        .routes(routes!(test_reachability))
        .routes(routes!(email_install_command))
}

/// Daemon-internal endpoints (unversioned at /api/daemon)
/// These are called by daemons themselves, not by users.
pub fn create_internal_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(register_daemon))
        .routes(routes!(daemon_startup))
        .routes(routes!(update_capabilities))
        .routes(routes!(receive_work_request))
        .routes(routes!(receive_heartbeat))
}

/// Get all Daemons
///
/// Returns all daemons accessible to the user.
/// Supports pagination via `limit` and `offset` query parameters,
/// and ordering via `group_by`, `order_by`, and `order_direction`.
#[utoipa::path(
    get,
    path = "",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "get_daemons",
    summary = "Get all daemons",
    params(DaemonFilterQuery),
    responses(
        (status = 200, description = "List of daemons", body = PaginatedApiResponse<DaemonResponse>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    query: Query<DaemonFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<DaemonResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Apply network filter and pagination
    let base_filter = StorableFilter::<Daemon>::new_from_network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    // Apply ordering
    let (filter, order_by) = query.apply_ordering(filter);

    let result = state
        .services
        .daemon_service
        .get_paginated_ordered(filter, &order_by)
        .await?;

    // Batch-load interfaced subnets from the junction to avoid N+1.
    let daemon_ids: Vec<Uuid> = result.items.iter().map(|d| d.id).collect();
    let subnet_ids_map = state
        .services
        .daemon_service
        .get_interfaced_subnet_ids_batch(&daemon_ids)
        .await;

    let policy = DaemonVersionPolicy::default();
    let responses: Vec<DaemonResponse> = result
        .items
        .into_iter()
        .map(|d| {
            let version_status = policy.evaluate(d.base.version.as_ref());
            let interfaced_subnet_ids = subnet_ids_map.get(&d.id).cloned().unwrap_or_default();
            DaemonResponse {
                id: d.id,
                created_at: d.created_at,
                updated_at: d.updated_at,
                base: d.base,
                version_status,
                interfaced_subnet_ids,
            }
        })
        .collect();

    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        responses,
        result.total_count,
        limit,
        offset,
    )))
}

/// Get Daemon by ID
///
/// Returns a specific daemon with computed version status.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "get_daemon_by_id",
    summary = "Get daemon by ID",
    params(("id" = Uuid, Path, description = "Daemon ID")),
    responses(
        (status = 200, description = "Daemon found", body = ApiResponse<DaemonResponse>),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_by_id(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<DaemonResponse>>> {
    let network_ids = auth.network_ids();

    let mut daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    // Validate user has access to this daemon's network
    if !network_ids.contains(&daemon.base.network_id) {
        return Err(ApiError::entity_access_denied::<Daemon>(id));
    }

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[daemon.id], EntityDiscriminants::Daemon, None)
        .await?;
    if let Some(tags) = tags_map.get(&daemon.id) {
        daemon.base.tags = tags.clone();
    }

    let policy = DaemonVersionPolicy::default();
    let version_status = policy.evaluate(daemon.base.version.as_ref());
    let interfaced_subnet_ids = state
        .services
        .daemon_service
        .get_interfaced_subnet_ids(&daemon.id)
        .await;

    Ok(Json(ApiResponse::success(DaemonResponse {
        id: daemon.id,
        created_at: daemon.created_at,
        updated_at: daemon.updated_at,
        base: daemon.base,
        version_status,
        interfaced_subnet_ids,
    })))
}

/// Register a new Daemon
///
/// Internal endpoint for daemon self-registration. Creates a host entry
/// and sets up default discovery jobs for the daemon.
#[utoipa::path(
    post,
    path = "/register",
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    request_body = DaemonRegistrationRequest,
    responses(
        (status = 200, description = "Daemon registered successfully", body = ApiResponse<DaemonRegistrationResponse>),
        (status = 403, description = "Daemon registration disabled in demo mode", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn register_daemon(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Json(request): Json<DaemonRegistrationRequest>,
) -> ApiResult<Json<ApiResponse<DaemonRegistrationResponse>>> {
    // Delegate to processor for shared registration logic
    // This ensures both DaemonPoll and ServerPoll modes use the same logic
    let response = state
        .services
        .daemon_service
        .process_registration(request, auth.into_entity())
        .await?;

    Ok(Json(ApiResponse::success(response)))
}

/// Daemon startup handshake
///
/// Internal endpoint for daemons to report their version on startup.
/// Updates the daemon's version and last_seen timestamp, returns server capabilities.
#[utoipa::path(
    post,
    path = "/{id}/startup",
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Daemon ID")),
    request_body = DaemonStartupRequest,
    responses(
        (status = 200, description = "Startup acknowledged", body = ApiResponse<ServerCapabilities>),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn daemon_startup(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(id): Path<Uuid>,
    Json(request): Json<DaemonStartupRequest>,
) -> ApiResult<Json<ApiResponse<ServerCapabilities>>> {
    let daemon_network_id = auth.network_ids()[0];

    // Validate daemon exists and belongs to the authenticated daemon's network
    let daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    if daemon.base.network_id != daemon_network_id {
        return Err(ApiError::entity_access_denied::<Daemon>(id));
    }

    // Use processor for shared startup logic
    let capabilities = state
        .services
        .daemon_service
        .process_startup(id, request.daemon_version, auth.into_entity())
        .await?;

    Ok(Json(ApiResponse::success(capabilities)))
}

/// Update Daemon capabilities
///
/// Legacy internal endpoint for pre-0.15 daemons to report their interfaced
/// subnets as bare ids. Modern daemons report them via the status heartbeat's
/// `interfaced_subnets` channel; this remains functional so older daemons in a
/// rolling deploy keep reporting (and don't 404).
#[utoipa::path(
    post,
    path = "/{id}/update-capabilities",
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Daemon ID")),
    request_body = LegacyCapabilities,
    responses(
        (status = 200, description = "Capabilities updated", body = EmptyApiResponse),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn update_capabilities(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(id): Path<Uuid>,
    Json(updated_capabilities): Json<LegacyCapabilities>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let daemon_network_id = auth.network_ids()[0];

    // Validate daemon exists and belongs to the authenticated daemon's network
    let daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    if daemon.base.network_id != daemon_network_id {
        return Err(ApiError::entity_access_denied::<Daemon>(id));
    }

    // Use processor for shared capabilities update logic
    state
        .services
        .daemon_service
        .process_capabilities(id, updated_capabilities, auth.into_entity())
        .await?;

    Ok(Json(ApiResponse::success(())))
}

/// Request work from server
///
/// Internal endpoint for daemons to poll for pending discovery sessions.
/// Also updates heartbeat and returns any pending cancellation requests.
/// Returns tuple of (next_session, should_cancel).
#[utoipa::path(
    post,
    path = "/{id}/request-work",
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Daemon ID")),
    request_body = DaemonStatus,
    responses(
        (status = 200, description = "Work request processed - returns (Option<Value>, bool)"),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn receive_work_request(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(daemon_id): Path<Uuid>,
    Json(status): Json<DaemonStatus>,
) -> ApiResult<Json<ApiResponse<(Option<serde_json::Value>, bool)>>> {
    let daemon_network_id = auth.network_ids()[0];

    // Validate daemon exists and belongs to the authenticated daemon's network
    let daemon = state
        .services
        .daemon_service
        .get_by_id(&daemon_id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?;

    let daemon = match daemon {
        Some(d) => d,
        None => {
            // Daemon was deleted or DB was reset. Version-split the response:
            // - Daemons >= 0.15.0 get DaemonNotRegistered (they handle it explicitly)
            // - Older daemons get DaemonStandby (which they already handle by entering standby)
            let supports_not_registered =
                crate::server::daemons::r#impl::version::supports_unified_discovery(
                    status.version.as_ref(),
                );
            if supports_not_registered {
                return Err(ApiError::coded(
                    StatusCode::NOT_FOUND,
                    ErrorCode::DaemonNotRegistered,
                ));
            } else {
                return Err(ApiError::coded(
                    StatusCode::FORBIDDEN,
                    ErrorCode::DaemonStandby,
                ));
            }
        }
    };

    if daemon.base.network_id != daemon_network_id {
        return Err(ApiError::entity_access_denied::<Daemon>(daemon_id));
    }

    // Reject work requests from daemons on standby (inactivity)
    if daemon.base.standby {
        return Err(ApiError::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::DaemonStandby,
        ));
    }

    // Use processor for shared heartbeat logic
    tracing::debug!(
        daemon_id = %daemon_id,
        ready_for_work = status.ready_for_work,
        interfaced_subnet_ids = ?status.capabilities.interfaced_subnet_ids,
        "DaemonPoll work request received"
    );
    state
        .services
        .daemon_service
        .process_status(daemon_id, status.clone(), auth.entity.clone())
        .await?;

    // Only dispatch work if daemon reports ready
    let next_session = if status.ready_for_work {
        state
            .services
            .daemon_service
            .get_pending_work(daemon_id)
            .await
    } else {
        None
    };
    let cancellation = state
        .services
        .daemon_service
        .get_pending_cancellation(daemon_id)
        .await;

    let has_cancellation = cancellation.is_some();

    // Log work request for tracing/debugging (previously done in service.receive_work_request)
    if next_session.is_some() || has_cancellation {
        tracing::debug!(
            daemon_id = %daemon_id,
            has_work = next_session.is_some(),
            has_cancellation = has_cancellation,
            "Daemon work request processed"
        );
    }

    // Serialize discovery payload for daemon transmission.
    // Unified: build credential_mappings via discovery_service and use with_exposed_credentials()
    // Legacy: use with_exposed_snmp() (SNMP inline in DiscoveryType::Network)
    let next_session_value = match next_session {
        Some(payload) if payload.discovery_type.runs_network_scan() => {
            let integration_targets = state
                .services
                .discovery_service
                .get_integration_targets_for_session(&payload.session_id)
                .await;
            let request = state
                .services
                .discovery_service
                .build_daemon_request(
                    &payload,
                    daemon_network_id,
                    &integration_targets,
                    daemon.base.version.as_ref(),
                    state
                        .services
                        .daemon_service
                        .network_subnets(daemon_network_id)
                        .await,
                )
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to build daemon request: {}", e);
                    DaemonDiscoveryRequest {
                        session_id: payload.session_id,
                        discovery_id: payload.discovery_id.unwrap_or_default(),
                        discovery_type: payload.discovery_type,
                        credential_mappings: vec![],
                        subnets: vec![],
                    }
                });
            Some(request.with_exposed_credentials())
        }
        Some(payload) => Some(payload.with_exposed_snmp()),
        None => None,
    };

    Ok(Json(ApiResponse::success((
        next_session_value,
        has_cancellation,
    ))))
}

/// Receive daemon heartbeat (DEPRECATED - for backwards compatibility with pre-v0.14.0 daemons)
///
/// Internal endpoint for legacy daemons to send periodic heartbeats.
/// New daemons (v0.14.0+) use the /request-work endpoint which includes heartbeat functionality.
/// This endpoint is kept for backwards compatibility and will be removed in a future version.
#[utoipa::path(
    post,
    path = "/{id}/heartbeat",
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL, api_tags::DEPRECATED],
    params(("id" = Uuid, Path, description = "Daemon ID")),
    request_body = DaemonHeartbeatPayload,
    responses(
        (status = 200, description = "Heartbeat received", body = EmptyApiResponse),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn receive_heartbeat(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(id): Path<Uuid>,
    Json(request): Json<DaemonHeartbeatPayload>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let daemon_network_id = auth.network_ids()[0];

    // Validate daemon exists and belongs to the authenticated daemon's network
    let daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    if daemon.base.network_id != daemon_network_id {
        return Err(ApiError::entity_access_denied::<Daemon>(id));
    }

    // Use processor for shared heartbeat logic (same as request-work)
    // Legacy daemons assumed ready (existing behavior)
    let status = crate::daemon::runtime::state::DaemonStatus {
        url: Some(request.url),
        name: request.name,
        mode: request.mode,
        version: None, // Old daemons don't send version in heartbeat
        capabilities: LegacyCapabilities::default(),
        interfaced_subnets: Vec::new(),
        ready_for_work: true,
    };
    state
        .services
        .daemon_service
        .process_status(id, status, auth.into_entity())
        .await?;

    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// Pre-provisioning (ServerPoll mode only)
// ============================================================================

/// Load the daemon a re-provision request targets, enforcing tenant access and the
/// mint-a-fresh-key safety guard.
///
/// Re-provisioning always mints a new key, which is safe in exactly two situations: a daemon
/// that has never checked in (the create flow re-running because advanced settings changed),
/// and a legacy daemon with no bound key (its separate network-shared key row is untouched, so
/// it keeps authenticating until it is reconfigured). For a live provisioned daemon the new key
/// would take effect with no way for the daemon to learn it — silently cutting it off — so that
/// case is refused. Rotating a live daemon's key has its own endpoint.
async fn load_reprovision_target(
    state: &AppState,
    daemon_id: Uuid,
    network_ids: &[Uuid],
) -> ApiResult<Daemon> {
    let daemon = state
        .services
        .daemon_service
        .get_by_id(&daemon_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(daemon_id))?;

    validate_network_access(
        Some(daemon.base.network_id),
        network_ids,
        "re-provision daemon",
    )?;

    if daemon.base.last_seen.is_some() && daemon.base.api_key_id.is_some() {
        return Err(ApiError::conflict(
            "This daemon is already connected with its own API key. Rotate its key instead of re-provisioning it.",
        ));
    }

    Ok(daemon)
}

/// Provision a Daemon, or re-provision an existing one
///
/// Creates a daemon record on the server before the daemon is installed and mints an API key
/// bound to it 1:1. Returns the daemon record and that key, which is shown only once and must
/// be configured on the daemon.
///
/// When `daemon_id` is supplied the existing record is reused instead of creating a new one,
/// giving a legacy daemon (one with no bound key) a pathway to a dedicated key without losing
/// its host, discovery jobs, or history. Re-provisioning always mints a fresh key.
///
/// Install commands are not built here — fetch them from the install-command endpoint, which
/// builds them idempotently and fills in the key this returns. That keeps a display-only
/// regenerate (an OS switch, an advanced-setting change) from re-minting the key.
#[utoipa::path(
    post,
    path = "/provision",
    tags = [Daemon::ENTITY_NAME_PLURAL],
    operation_id = "provision_daemon",
    summary = "Provision a daemon, or re-provision an existing one",
    request_body = ProvisionDaemonRequest,
    responses(
        (status = 201, description = "Daemon provisioned successfully", body = ApiResponse<ProvisionDaemonResponse>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
        (status = 403, description = "Forbidden", body = ApiErrorResponse),
        (status = 409, description = "Daemon is live and already has a bound key", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn provision_daemon(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(request): Json<ProvisionDaemonRequest>,
) -> ApiResult<Json<ApiResponse<ProvisionDaemonResponse>>> {
    let network_ids = auth.network_ids();

    // ---- Resolve the target daemon, enforcing tenant access ---------------------------
    // `load_reprovision_target` access-checks the existing record; the create path is checked
    // here against the requested network. Everything past this point is mechanics, and lives
    // in the service so the integrated-daemon bootstrap can share it.
    let existing_daemon = match request.daemon_id {
        Some(daemon_id) => Some(load_reprovision_target(&state, daemon_id, &network_ids).await?),
        None => {
            let network_id = request.network_id.ok_or_else(|| {
                ApiError::bad_request("network_id is required when provisioning a new daemon")
            })?;
            validate_network_access(Some(network_id), &network_ids, "provision daemon")?;
            None
        }
    };

    let (created_daemon, plaintext) = state
        .services
        .daemon_service
        .provision(&request, existing_daemon, auth.entity.clone())
        .await?;

    // Compute version status for response
    let policy = DaemonVersionPolicy::default();
    let version_status = policy.evaluate(created_daemon.base.version.as_ref());

    // Install commands are not built here — the caller fetches them from the install-command
    // endpoint (which fills in the key returned below). That keeps a display-only regenerate
    // from re-minting the key.
    Ok(Json(ApiResponse::success(ProvisionDaemonResponse {
        daemon: DaemonResponse {
            id: created_daemon.id,
            created_at: created_daemon.created_at,
            updated_at: created_daemon.updated_at,
            base: created_daemon.base,
            version_status,
            // Freshly provisioned; interfaced subnets populate on first heartbeat.
            interfaced_subnet_ids: Vec::new(),
        },
        daemon_api_key: plaintext,
    })))
}

/// Retry connection to an unreachable Daemon
///
/// Resets the is_unreachable flag for a daemon that was marked unreachable
/// due to repeated polling failures. The poller will attempt to contact
/// the daemon again on the next cycle.
#[utoipa::path(
    post,
    path = "/{id}/retry-connection",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "retry_daemon_connection",
    summary = "Retry connection to unreachable daemon",
    params(("id" = Uuid, Path, description = "Daemon ID")),
    responses(
        (status = 200, description = "Connection retry initiated", body = EmptyApiResponse),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn retry_connection(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    let mut daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    // Validate user has access to this daemon's network
    if !network_ids.contains(&daemon.base.network_id) {
        return Err(ApiError::entity_access_denied::<Daemon>(id));
    }

    // Only allow retry for ServerPoll daemons
    if daemon.base.mode != DaemonMode::ServerPoll {
        return Err(ApiError::bad_request(
            "Connection retry is only available for ServerPoll mode daemons",
        ));
    }

    // Reset unreachability flag
    if daemon.base.is_unreachable {
        daemon.base.is_unreachable = false;
        state
            .services
            .daemon_service
            .update(&mut daemon, auth.into_entity())
            .await?;

        tracing::info!(
            daemon_id = %id,
            daemon_name = %daemon.base.name,
            "Daemon connection retry initiated - marked as reachable"
        );
    }

    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// Reachability Testing
// ============================================================================

/// Test reachability of a daemon URL
///
/// Performs a TCP connection test and optionally an HTTP health check
/// to verify that a daemon URL is reachable from the server.
#[utoipa::path(
    post,
    path = "/test-reachability",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "test_daemon_reachability",
    summary = "Test reachability of a daemon URL",
    request_body = TestReachabilityRequest,
    responses(
        (status = 200, description = "Reachability test result", body = ApiResponse<TestReachabilityResponse>),
        (status = 400, description = "Invalid URL", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn test_reachability(
    State(state): State<Arc<AppState>>,
    _auth: Authorized<Member>,
    Json(request): Json<TestReachabilityRequest>,
) -> ApiResult<Json<ApiResponse<TestReachabilityResponse>>> {
    // Parse the URL
    let parsed =
        url::Url::parse(&request.url).map_err(|_| ApiError::bad_request("Invalid URL format"))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| ApiError::bad_request("URL must contain a host"))?;

    let port = parsed.port().unwrap_or(match parsed.scheme() {
        "https" => 443,
        _ => 80,
    });

    // Resolve hostname to IP for SSRF check
    let addr_str = format!("{}:{}", host, port);
    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host(&addr_str)
        .await
        .map_err(|_| ApiError::bad_request(&format!("Could not resolve host: {}", host)))?
        .collect();

    if addrs.is_empty() {
        return Err(ApiError::bad_request(&format!(
            "Could not resolve host: {}",
            host
        )));
    }

    // SSRF protection: reject private IPs in cloud mode only.
    // Self-hosted deployments (Commercial/Community) need to reach LAN daemons.
    let deployment_type = crate::server::config::get_deployment_type(&state.config);
    let is_cloud = deployment_type == crate::server::config::DeploymentType::Cloud;

    if is_cloud {
        for addr in &addrs {
            if crate::server::daemons::ssrf::is_private_ip(&addr.ip()) {
                return Err(ApiError::bad_request(
                    "Cannot test reachability to private/loopback addresses",
                ));
            }
        }
    }

    // TCP connection test with 5s timeout
    let tcp_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::net::TcpStream::connect(addrs.as_slice()),
    )
    .await;

    let reachable = match tcp_result {
        Ok(Ok(_stream)) => true,
        Ok(Err(e)) => {
            let message = match e.kind() {
                std::io::ErrorKind::ConnectionRefused => {
                    format!(
                        "Connection refused — no service is listening on port {} at {}",
                        port, host
                    )
                }
                std::io::ErrorKind::TimedOut => {
                    format!(
                        "Connection timed out — {}:{} may be unreachable or a firewall is blocking the port",
                        host, port
                    )
                }
                std::io::ErrorKind::AddrNotAvailable => {
                    format!("Address not available — {}", host)
                }
                _ => format!("Connection failed: {}", e),
            };
            return Ok(Json(ApiResponse::success(TestReachabilityResponse {
                reachable: false,
                error: Some(message),
                health: None,
            })));
        }
        Err(_) => {
            return Ok(Json(ApiResponse::success(TestReachabilityResponse {
                reachable: false,
                error: Some(format!(
                    "Connection timed out after 5 seconds — could not reach {}:{}",
                    host, port
                )),
                health: None,
            })));
        }
    };

    // Optional health check
    let health = if request.check_health {
        let health_url = format!("{}/api/health", request.url.trim_end_matches('/'));
        // Pin the request to the already-validated resolved address so reqwest
        // does not independently re-resolve the hostname — this closes the
        // DNS-rebinding TOCTOU where a name resolves to a public IP during the
        // SSRF check above and to an internal IP for this request.
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .resolve(host, addrs[0])
            .build()
            .unwrap_or_default();

        match client.get(&health_url).send().await {
            Ok(resp) => Some(resp.status().is_success()),
            Err(_) => Some(false),
        }
    } else {
        None
    };

    Ok(Json(ApiResponse::success(TestReachabilityResponse {
        reachable,
        error: None,
        health,
    })))
}
