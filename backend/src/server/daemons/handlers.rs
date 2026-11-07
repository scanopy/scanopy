use crate::daemon::runtime::state::DaemonStatus;
use crate::daemon::shared::config::{DaemonArgs, parse_integration_target_tokens};
use crate::server::auth::middleware::permissions::{
    And, Authorized, Can, Delete, IsDaemon, IsUser, Read, RequireVerified, Write,
};
use crate::server::credentials::r#impl::mapping::IntegrationTarget;
use crate::server::daemon_api_keys::r#impl::base::DaemonApiKey;
use crate::server::daemons::r#impl::api::{
    DaemonHealth, DaemonHeartbeatPayload, ProvisionDaemonRequest, ProvisionDaemonResponse,
    ProvisionEligibility, TestReachabilityRequest, TestReachabilityResponse,
};
use crate::server::daemons::r#impl::base::DaemonOs;
use crate::server::daemons::service::DaemonLimit;
use crate::server::metrics::dependency::{Dependency, observe_http};
use crate::server::openapi::tags as api_tags;
use crate::server::shared::api_key_common::hash_api_key;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::list::list_page;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::query::{
    narrow_by_tags_and_search, on_requested_sites, serde_names,
};
use crate::server::shared::handlers::traits::{
    CrudHandlers, ListScope, base_list_filter, update_handler,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::{Junction, StorableFilter};
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::trusted_ca::TrustedCaBundle;
use crate::server::shared::types::api::ApiErrorResponse;
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::shared::validation::validate_site_access;
use crate::server::{
    config::AppState,
    daemons::r#impl::{
        api::{
            DaemonRegistrationRequest, DaemonRegistrationResponse, DaemonResponse,
            DaemonStartupRequest, LegacyCapabilities, ServerCapabilities,
        },
        base::{Daemon, DaemonMode},
        install_artifacts::{
            API_KEY_PLACEHOLDER, DownloadSource, InstallCommandType, UpgradeArtifacts,
            WINDOWS_MSI_SCARF_URL, WINDOWS_MSI_URL, build_upgrade_artifacts,
        },
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
use axum_client_ip::ClientIp;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Daemon Ordering
// ============================================================================

/// Fields that daemons can be ordered/grouped by.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum DaemonOrderField {
    #[default]
    CreatedAt,
    Name,
    LastSeen,
    UpdatedAt,
    SiteId,
    /// The OS the daemon is installed on. Daemons that never reported one sort last.
    Os,
    /// How the daemon connects.
    Mode,
    /// The user who maintains the daemon.
    Maintainer,
    /// The daemon's version, in version order rather than text order: 0.17.10 after 0.17.9.
    Version,
    /// The status the daemon's chip shows, most severe first. Derived in code
    /// ([`DaemonHealth::of`]), so it orders and groups in memory.
    Status,
}

impl OrderField for DaemonOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "daemons.created_at",
            Self::Name => "daemons.name",
            Self::LastSeen => "daemons.last_seen",
            Self::UpdatedAt => "daemons.updated_at",
            Self::SiteId => "daemons.site_id",
            // Both are stored as JSON strings; unwrapped so they group and filter as the bare value.
            Self::Os => "(daemons.os::jsonb #>> '{}')",
            Self::Mode => "(daemons.mode::jsonb #>> '{}')",
            Self::Maintainer => "daemons.user_id",
            // What the version filter's options count. The list orders it in memory.
            Self::Version => "daemons.version",
            // Never read: the status is computed.
            Self::Status => "daemons.id",
        }
    }

    fn sorts_by_site_name(&self) -> bool {
        matches!(self, Self::SiteId)
    }

    fn nulls_last(&self) -> bool {
        matches!(self, Self::Os | Self::Version)
    }
}

// ============================================================================
// Daemon Filter Query
// ============================================================================

/// Query parameters for filtering and ordering daemons.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct DaemonFilterQuery {
    /// Filter by site ID. Repeat the parameter to pass several.
    #[serde(alias = "site_id")]
    pub site_ids: Option<Vec<Uuid>>,
    /// Only daemons in one of these statuses. Repeat for several.
    pub statuses: Option<Vec<DaemonHealth>>,
    /// Only daemons installed on one of these operating systems. Repeat for several.
    pub os: Option<Vec<DaemonOs>>,
    /// Only daemons that connect one of these ways. Repeat for several.
    pub modes: Option<Vec<DaemonMode>>,
    /// Only daemons maintained by one of these users. Repeat for several.
    pub maintainer_ids: Option<Vec<Uuid>>,
    /// Only daemons running one of these versions. Repeat for several.
    pub versions: Option<Vec<String>>,
    /// Only daemons with an interface on one of these subnets. Repeat for several.
    pub interfaced_subnet_ids: Option<Vec<Uuid>>,
    /// Filter by tag IDs (returns daemons that have ANY of the specified tags)
    pub tag_ids: Option<Vec<Uuid>>,
    /// Tag sets the daemon must hold one tag from each of. Each entry is a comma-separated list
    /// of tag ids. One entry per tag group the list is filtered by.
    pub tag_sets: Option<Vec<String>>,
    /// Free-text search. Case-insensitive substring match against the name and url.
    pub search: Option<String>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<DaemonOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<DaemonOrderField>,
    /// Group by the tag each row holds from this tag group. Takes the place of `group_by`.
    pub group_by_tag_group: Option<Uuid>,
    /// Sort by the position of the tag each row holds from this tag group. Takes the place of
    /// `order_by`.
    pub order_by_tag_group: Option<Uuid>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

/// The subnets each daemon has an interface on.
const DAEMON_SUBNETS: Junction = Junction {
    table: "daemon_interfaced_subnets",
    own_column: "daemon_id",
    other_column: "subnet_id",
    has_history: false,
};

impl DaemonFilterQuery {
    /// Everything the daemons list narrows by in SQL: access, sites, OS, mode, maintainer,
    /// version, interfaced subnets, tags and search. Status, derived in code, is applied by the
    /// list itself.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<Daemon>> {
        let filter = narrow_by_tags_and_search(
            base_list_filter::<Daemon>(scope, self),
            self.tag_ids.as_deref(),
            self.tag_sets.as_deref(),
            self.search.as_deref(),
        )?;
        let filter = match &self.os {
            Some(os) if !os.is_empty() => {
                filter.expr_text_in(DaemonOrderField::Os.to_sql(), &serde_names(os))
            }
            _ => filter,
        };
        let filter = match &self.modes {
            Some(modes) if !modes.is_empty() => {
                filter.expr_text_in(DaemonOrderField::Mode.to_sql(), &serde_names(modes))
            }
            _ => filter,
        };
        let filter = match &self.maintainer_ids {
            Some(ids) if !ids.is_empty() => {
                let ids: Vec<String> = ids.iter().map(Uuid::to_string).collect();
                filter.expr_text_in(DaemonOrderField::Maintainer.to_sql(), &ids)
            }
            _ => filter,
        };
        let filter = match &self.versions {
            Some(versions) if !versions.is_empty() => filter.text_column_in("version", versions),
            _ => filter,
        };
        Ok(match &self.interfaced_subnet_ids {
            Some(ids) if !ids.is_empty() => filter.linked_to(DAEMON_SUBNETS, ids),
            _ => filter,
        })
    }
}

impl FilterQueryExtractor for DaemonFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        on_requested_sites(filter, self.site_ids.as_deref(), user_site_ids)
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
    crate::crud_get_field_values_handler!(Daemon);
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
/// the url the server dials. Identity and server-managed fields (site, mode, host, key
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
    auth: Authorized<Can<Daemon, Write>>,
    Path(id): Path<Uuid>,
    ApiJson(mut request): ApiJson<Daemon>,
) -> ApiResult<Json<ApiResponse<Daemon>>> {
    let site_ids = auth.site_ids();

    let existing = Daemon::get_service(&state)
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    // Tenant isolation: the caller must have access to the daemon's *current* site.
    // site_id is restored below, so an update cannot move a daemon between sites.
    validate_site_access(Some(existing.base.site_id), &site_ids, "update")?;

    // A DaemonPoll daemon dials out and is never dialed, so its url is unused — silently
    // storing one would suggest a reachability that does not exist.
    if existing.base.mode == DaemonMode::DaemonPoll && request.base.url != existing.base.url {
        return Err(ApiError::bad_request(
            "url only applies to ServerPoll daemons; a DaemonPoll daemon dials the server",
        ));
    }

    request.preserve_immutable_fields(&existing);

    update_handler::<Daemon>(State(state), auth, Path(id), ApiJson(request)).await
}

/// Query for [`get_install_command`]. `purpose` is required; the rest are the client-settable
/// advanced daemon settings, folded into an `install` command. Lists are comma-joined
/// (`interfaces`, `credential_refs`).
#[derive(Deserialize, Serialize, Debug, Clone, IntoParams, utoipa::ToSchema)]
pub struct InstallCommandQuery {
    /// `install` or `rekey` (with the api-key placeholder), or `reconfigure` (credential-free).
    pub purpose: InstallCommandType,
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
/// or `rekey` command is a placeholder (`<API_KEY>`) the caller substitutes from the plaintext it holds; a
/// `reconfigure` command carries no key at all. Minting is a separate mutation
/// (`POST /provision`), so regenerating a command here (advanced-setting change, OS switch, the
/// Details reconfigure view) never rotates the daemon's key.
///
/// The server derives the exact command shape from the record: DaemonPoll vs ServerPoll for the
/// flags, and — for `rekey` — whether the daemon has checked in (`last_seen`) to decide whether
/// the command names the existing install it re-keys. An `install` command reads the same before
/// and after the daemon's first handshake.
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
    auth: Authorized<Can<DaemonApiKey, Read>>,
    Path(id): Path<Uuid>,
    Query(query): Query<InstallCommandQuery>,
) -> ApiResult<Json<ApiResponse<crate::server::daemons::r#impl::install_artifacts::InstallArtifacts>>>
{
    let site_ids = auth.site_ids();

    let daemon = Daemon::get_service(&state)
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    validate_site_access(Some(daemon.base.site_id), &site_ids, "get install command")?;

    let install_config = query.install_config();
    let artifacts = crate::server::daemons::r#impl::install_artifacts::build_install_artifacts(
        &state.config.public_url,
        &daemon,
        Some(&install_config),
        &query.credential_refs(),
        query.purpose,
        DownloadSource::for_server(&state).await,
    );

    Ok(Json(ApiResponse::success(artifacts)))
}

/// Generate daemon upgrade commands
///
/// The steps to replace a daemon's binary and restart it, per OS, plus the docker method. Like the
/// install command, a pure builder over the daemon record.
#[utoipa::path(
    get,
    path = "/{id}/upgrade-command",
    tag = Daemon::ENTITY_NAME_PLURAL,
    operation_id = "get_daemon_upgrade_command",
    summary = "Generate daemon upgrade commands",
    params(("id" = Uuid, Path, description = "daemon ID")),
    responses(
        (status = 200, description = "Upgrade commands", body = ApiResponse<UpgradeArtifacts>),
        (status = 404, description = "daemon not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_upgrade_command(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<DaemonApiKey, Read>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<UpgradeArtifacts>>> {
    let daemon = Daemon::get_service(&state)
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    validate_site_access(
        Some(daemon.base.site_id),
        &auth.site_ids(),
        "get upgrade command",
    )?;

    Ok(Json(ApiResponse::success(build_upgrade_artifacts(
        &daemon,
        DownloadSource::for_server(&state).await,
    ))))
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
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    operation_id = "get_windows_msi",
    summary = "Download Windows daemon MSI",
    description = "Proxies the Windows daemon MSI from GitHub so the browser can save it under a caller-chosen filename. Cached for an hour to avoid refetching from GitHub on every request.",
    responses(
        (status = 200, description = "Windows daemon MSI installer", content_type = "application/octet-stream", body = Vec<u8>),
        (status = 502, description = "Failed to fetch the MSI from GitHub", body = ApiErrorResponse),
    )
)]
pub async fn get_windows_msi(
    State(state): State<Arc<AppState>>,
    Extension(cache): Extension<Arc<AppCache>>,
    ClientIp(client_ip): ClientIp,
) -> ApiResult<impl IntoResponse> {
    if DownloadSource::for_server(&state).await == DownloadSource::Scarf {
        count_msi_download(&state, client_ip);
    }

    let bytes = match cache.get::<bytes::Bytes>(WINDOWS_MSI_CACHE_KEY).await {
        Some(cached) => cached,
        None => {
            let response = observe_http(Dependency::Github, reqwest::get(WINDOWS_MSI_URL))
                .await
                .map_err(|e| {
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

/// Count one MSI download in Scarf without serving through it. The bytes above come from the
/// cache or GitHub, so this request only records the download: Scarf answers with a redirect,
/// which is not followed. `X-Scarf-IP` attributes the download to the browser's IP rather than
/// the server's; a private client IP (a self-host on its LAN) is left out, so Scarf falls back
/// to the server's own public IP. Runs in the background and ignores failures, so Scarf being
/// down or blocked never delays or fails the download. Called only when the server's
/// [`DownloadSource`] is Scarf.
fn count_msi_download(state: &AppState, client_ip: std::net::IpAddr) {
    // No default-client fallback: it follows redirects and would fetch the whole MSI.
    let Ok(client) = TrustedCaBundle::apply(
        state.services.trusted_ca.as_deref(),
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none()),
    )
    .build() else {
        return;
    };

    let mut request = client.get(WINDOWS_MSI_SCARF_URL);
    if !crate::server::daemons::ssrf::is_private_ip(&client_ip) {
        request = request.header("X-Scarf-IP", client_ip.to_string());
    }

    tokio::spawn(async move {
        let _ = observe_http(Dependency::Scarf, request.send()).await;
    });
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
    auth: Authorized<Can<Daemon, Delete>>,
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
    auth: Authorized<Can<Daemon, Delete>>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
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
    generated::bulk_delete(state, auth, ApiJson(ids)).await
}

/// Request body for emailing an install command to the authenticated user. The server builds
/// the command from the daemon record, exactly as [`get_install_command`] does; the client only
/// picks which method's command to send.
#[derive(Deserialize, Serialize, utoipa::ToSchema)]
pub struct EmailInstallCommandRequest {
    /// The daemon whose command to send.
    pub daemon_id: Uuid,
    /// The same purpose and advanced settings `GET /{id}/install-command` takes.
    #[serde(flatten)]
    pub query: InstallCommandQuery,
    /// Operating system the command targets.
    pub os: DaemonOs,
    /// Send the docker compose instead of the binary command. Linux only.
    #[serde(default)]
    pub docker: bool,
    /// The daemon's api key, from the provision response. Required when `purpose` embeds the
    /// key (`install`, `rekey`) and rejected otherwise. The server stores only its hash, so it
    /// can't fill the key in itself; it checks this one against the daemon's bound key.
    pub api_key: Option<String>,
}

/// Email the install command to the authenticated user's email address.
///
/// Requires a verified email: the endpoint mails on demand, so an unproven address would let
/// anyone send Scanopy mail to an inbox they don't own.
///
/// Session-only, and `IsUser` says so at the extractor rather than in the body. "The current user"
/// has no answer for an automation identity: a user API key carries `user_id` but no address, so
/// this endpoint could never serve one. An API key that wants the command reads it directly from
/// `GET /api/v1/daemons/{id}/install-command`.
#[utoipa::path(
    post,
    path = "/email-install-command",
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    operation_id = "email_install_command",
    summary = "Email install command to current user",
    request_body = EmailInstallCommandRequest,
    responses(
        (status = 200, description = "Email sent", body = EmptyApiResponse),
        (status = 400, description = "Email service not configured, no command for this method, or api key missing or not this daemon's", body = ApiErrorResponse),
        (status = 403, description = "User session or verified email required", body = ApiErrorResponse),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
async fn email_install_command(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<And<IsUser, Can<DaemonApiKey, Read>>>>,
    ApiJson(request): ApiJson<EmailInstallCommandRequest>,
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

    let daemon = Daemon::get_service(&state)
        .get_by_id(&request.daemon_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(request.daemon_id))?;
    validate_site_access(
        Some(daemon.base.site_id),
        &auth.site_ids(),
        "email install command",
    )?;

    let artifacts = crate::server::daemons::r#impl::install_artifacts::build_install_artifacts(
        &state.config.public_url,
        &daemon,
        Some(&request.query.install_config()),
        &request.query.credential_refs(),
        request.query.purpose,
        DownloadSource::for_server(&state).await,
    );
    let command = artifacts
        .command(request.os, request.docker)
        .ok_or_else(|| ApiError::bad_request("No install command for this method"))?;

    let command = match (request.query.purpose.embeds_key(), &request.api_key) {
        (true, Some(api_key)) => {
            let bound_key = match daemon.base.api_key_id {
                Some(key_id) => {
                    state
                        .services
                        .daemon_api_key_service
                        .get_by_id(&key_id)
                        .await?
                }
                None => None,
            };
            if bound_key.is_none_or(|key| key.base.key != hash_api_key(api_key)) {
                return Err(ApiError::bad_request(
                    "Api key does not belong to this daemon",
                ));
            }
            command.replace(API_KEY_PLACEHOLDER, api_key)
        }
        (true, None) => {
            return Err(ApiError::bad_request(
                "This command needs the daemon's api key",
            ));
        }
        (false, Some(_)) => {
            return Err(ApiError::bad_request("This command carries no api key"));
        }
        (false, None) => command.to_string(),
    };

    // `?` rather than a formatted `internal_error`: the latter put the mail server's raw
    // reply — relay hostname and provider diagnostics included — into the response body,
    // where the client's error middleware showed it to the user in a toast.
    email_service
        .send_install_command_email(email, &command, request.os.into())
        .await?;

    Ok(Json(ApiResponse::success(())))
}

/// User-facing daemon management endpoints (versioned at /api/v1/daemons)
pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all))
        .routes(routes!(get_by_id, update_daemon, delete_daemon))
        .routes(routes!(get_install_command))
        .routes(routes!(get_upgrade_command))
        .routes(routes!(bulk_delete_daemons))
        .routes(routes!(generated::export_csv))
        .routes(routes!(generated::get_field_values))
        .routes(routes!(provision_daemon))
        .routes(routes!(get_provision_eligibility))
        .merge(super::retry::create_router())
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
    operation_id = "list_daemons",
    summary = "List daemons",
    params(DaemonFilterQuery),
    responses(
        (status = 200, description = "List of daemons", body = PaginatedApiResponse<DaemonResponse>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Daemon, Read>>,
    Query(query): Query<DaemonFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<DaemonResponse>>> {
    let scope = ListScope::of(&auth)?;
    Ok(Json(list_page::<Daemon>(&state, &scope, &query).await?))
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
    auth: Authorized<Can<Daemon, Read>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<DaemonResponse>>> {
    let site_ids = auth.site_ids();

    let mut daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    // Validate user has access to this daemon's site
    if !site_ids.contains(&daemon.base.site_id) {
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

    Ok(Json(ApiResponse::success(DaemonResponse::new(
        daemon,
        version_status,
        interfaced_subnet_ids,
    ))))
}

/// Register daemon
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
    ApiJson(request): ApiJson<DaemonRegistrationRequest>,
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
    ApiJson(request): ApiJson<DaemonStartupRequest>,
) -> ApiResult<Json<ApiResponse<ServerCapabilities>>> {
    let daemon_site_id = auth.site_ids()[0];

    // Validate daemon exists and belongs to the authenticated daemon's site
    let daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    if daemon.base.site_id != daemon_site_id {
        return Err(ApiError::entity_access_denied::<Daemon>(id));
    }

    // Use processor for shared startup logic
    let capabilities = state
        .services
        .daemon_service
        .process_startup(id, request.daemon_version, request.os, auth.into_entity())
        .await?;

    // Only DaemonPoll daemons announce their startup, once per boot.
    state
        .services
        .daemon_service
        .fail_sessions_lost_to_restart(id)
        .await;

    Ok(Json(ApiResponse::success(capabilities)))
}

/// Update daemon capabilities
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
    ApiJson(updated_capabilities): ApiJson<LegacyCapabilities>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let daemon_site_id = auth.site_ids()[0];

    // Validate daemon exists and belongs to the authenticated daemon's site
    let daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    if daemon.base.site_id != daemon_site_id {
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
    ApiJson(status): ApiJson<DaemonStatus>,
) -> ApiResult<Json<ApiResponse<(Option<serde_json::Value>, bool)>>> {
    let daemon_site_id = auth.site_ids()[0];

    // Validate daemon exists and belongs to the authenticated daemon's site
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

    if daemon.base.site_id != daemon_site_id {
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
    state
        .services
        .daemon_service
        .process_status(daemon_id, status.clone(), auth.entity.clone())
        .await?;

    // Before dispatch and the cancellation read: a session the daemon dropped ends here, and the
    // one promoted in its place goes out in this response.
    state
        .services
        .daemon_service
        .fail_sessions_dropped_by_daemon(&daemon, status.ready_for_work)
        .await;

    // Only dispatch work if daemon reports ready
    let next_session = if status.ready_for_work {
        state
            .services
            .daemon_service
            .get_pending_work(&daemon)
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
                    daemon_site_id,
                    &integration_targets,
                    daemon.base.version.as_ref(),
                    state
                        .services
                        .daemon_service
                        .site_subnets(daemon_site_id)
                        .await,
                    state
                        .services
                        .daemon_service
                        .subnets_scanned_elsewhere(&payload)
                        .await,
                )
                .await;
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

/// Receive daemon heartbeat (deprecated)
///
/// Internal endpoint for legacy daemons to send periodic heartbeats.
/// New daemons (v0.14.0+) use the /request-work endpoint which includes heartbeat functionality.
/// This endpoint is kept for backwards compatibility and will be removed in a future version.
#[utoipa::path(
    post,
    path = "/{id}/heartbeat",
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
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
    ApiJson(request): ApiJson<DaemonHeartbeatPayload>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let daemon_site_id = auth.site_ids()[0];

    // Validate daemon exists and belongs to the authenticated daemon's site
    let daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(id))?;

    if daemon.base.site_id != daemon_site_id {
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
        os: None, // Old daemons don't report their OS
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
/// and a legacy daemon with no bound key (its separate site-shared key row is untouched, so
/// it keeps authenticating until it is reconfigured). For a live provisioned daemon the new key
/// would take effect with no way for the daemon to learn it — silently cutting it off — so that
/// case is refused. Rotating a live daemon's key has its own endpoint.
async fn load_reprovision_target(
    state: &AppState,
    daemon_id: Uuid,
    site_ids: &[Uuid],
) -> ApiResult<Daemon> {
    let daemon = state
        .services
        .daemon_service
        .get_by_id(&daemon_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Daemon>(daemon_id))?;

    validate_site_access(Some(daemon.base.site_id), site_ids, "re-provision daemon")?;

    if daemon.base.last_seen.is_some() && daemon.base.api_key_id.is_some() {
        return Err(ApiError::conflict(
            "This daemon is already connected with its own API key. Rotate its key instead of re-provisioning it.",
        ));
    }

    Ok(daemon)
}

/// Whether a new daemon can be provisioned
///
/// The create wizard reads this before its first step, so a refusal from `POST /provision`
/// (no verified owner and a daemon already exists) stops the user up front rather than after
/// they have configured everything. Reads only the caller's own organization.
#[utoipa::path(
    get,
    path = "/provision-eligibility",
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    operation_id = "get_provision_eligibility",
    summary = "Check whether a new daemon can be provisioned",
    responses(
        (status = 200, description = "Provisioning eligibility", body = ApiResponse<ProvisionEligibility>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_provision_eligibility(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Daemon, Write>>,
) -> ApiResult<Json<ApiResponse<ProvisionEligibility>>> {
    let organization_id = auth.require_organization_id()?;
    let limit = state
        .services
        .daemon_service
        .daemon_limit_reached(organization_id)
        .await?;

    Ok(Json(ApiResponse::success(ProvisionEligibility {
        email_verification_required: limit == Some(DaemonLimit::EmailVerification),
        plan_required: limit == Some(DaemonLimit::PlanRequired),
    })))
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
    summary = "Provision or re-provision daemon",
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
    auth: Authorized<Can<Daemon, Write>>,
    ApiJson(request): ApiJson<ProvisionDaemonRequest>,
) -> ApiResult<Json<ApiResponse<ProvisionDaemonResponse>>> {
    let site_ids = auth.site_ids();

    let seeded_credential_ids: Vec<Uuid> = request
        .seed_credential_refs
        .iter()
        .map(|target| target.credential_id())
        .collect();
    state
        .services
        .credential_service
        .validate_ids_in_org(&seeded_credential_ids, auth.require_organization_id()?)
        .await?;

    // ---- Resolve the target daemon, enforcing tenant access ---------------------------
    // `load_reprovision_target` access-checks the existing record; the create path is checked
    // here against the requested site. Everything past this point is mechanics, and lives
    // in the service so the integrated-daemon bootstrap can share it.
    let existing_daemon = match request.daemon_id {
        Some(daemon_id) => Some(load_reprovision_target(&state, daemon_id, &site_ids).await?),
        None => {
            let site_id = request.site_id.ok_or_else(|| {
                ApiError::bad_request("site_id is required when provisioning a new daemon")
            })?;
            validate_site_access(Some(site_id), &site_ids, "provision daemon")?;
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
        // Freshly provisioned; interfaced subnets populate on first heartbeat.
        daemon: DaemonResponse::new(created_daemon, version_status, Vec::new()),
        daemon_api_key: plaintext,
    })))
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
    tags = [Daemon::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
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
    _auth: Authorized<Can<Daemon, Write>>,
    ApiJson(request): ApiJson<TestReachabilityRequest>,
) -> axum::response::Response {
    // Resolving, connecting to and health-checking a user-chosen address takes up to its
    // timeouts by design, so it stays out of the server's latency histogram.
    let mut response = check_reachability(&state, request).await.into_response();
    response
        .extensions_mut()
        .insert(crate::server::shared::types::api::LatencyExempt);
    response
}

async fn check_reachability(
    state: &AppState,
    request: TestReachabilityRequest,
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

    // TCP connection test with 5s timeout. The server testing a daemon URL, not a probe.
    #[allow(clippy::disallowed_methods)]
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
        let client = TrustedCaBundle::apply(
            state.services.trusted_ca.as_deref(),
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .resolve(host, addrs[0]),
        )
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
