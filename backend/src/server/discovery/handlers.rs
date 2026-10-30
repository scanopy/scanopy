use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::openapi::tags as api_tags;
use crate::server::shared::sse::until_shutdown;
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::metadata::HasId;
use crate::server::{
    auth::middleware::permissions::{Authorized, IsDaemon, Member, Viewer},
    config::AppState,
    daemons::r#impl::{api::DiscoveryUpdatePayload, version::supports_unified_discovery},
    discovery::r#impl::{base::Discovery, types::DiscoveryType},
    discovery::service::start_refusal,
    shared::{
        extractors::Query,
        handlers::{
            list::list_page,
            ordering::{OrderField, in_sql},
            query::{
                FilterQueryExtractor, OrderDirection, PaginationParams, deserialize_bool_values,
            },
            traits::ListScope,
            traits::{create_handler, update_handler, validate_entity_references},
        },
        services::traits::CrudService,
        storage::{
            filter::StorableFilter,
            traits::{Entity, Storable},
        },
        types::{
            api::{
                ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
                PaginatedApiResponse,
            },
            error_codes::ErrorCode,
        },
        validation::validate_site_access,
    },
    sites::r#impl::Site,
};
use axum::http::StatusCode;
use axum::{
    extract::{Path, State},
    response::{
        Json, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use const_format::concatcp;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc};
use strum::IntoEnumIterator;
use tokio::sync::broadcast;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Discovery Ordering
// ============================================================================

/// Fields that discoveries can be ordered/grouped by.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryOrderField {
    /// Newest-first is the default reading of a run history, so this is the one
    /// people reach for. Kept as the enum default too.
    #[default]
    CreatedAt,
    Name,
    UpdatedAt,
    DaemonId,
    SiteId,
    /// `discovery_type` is JSONB, so this reads its tag the way `json_field_eq` does.
    DiscoveryType,
    /// A completed run's outcome: the terminal phase recorded in `run_type.results`. Rows that
    /// are not historical runs carry none and sort last.
    Phase,
    /// When the run started, from `run_type.results`. Runs with none sort last.
    StartedAt,
    /// When the run finished, from `run_type.results`. Runs with none sort last.
    FinishedAt,
    /// How long the run took: finished minus started. Runs missing either sort last.
    Duration,
    /// How many warnings the run recorded. Rows with no results count as zero.
    Warnings,
    /// How a scan configuration runs: `Scheduled` or `AdHoc`.
    RunType,
    /// When a scan configuration last ran. Configurations that never ran sort last.
    LastRun,
    /// How many scans a configuration has completed.
    ScanCount,
    /// Whether the configuration's next run is forced to a full scan.
    ForceFullScan,
    /// Whether the configuration's type is a frozen legacy one, from `DiscoveryType::is_legacy`.
    Legacy,
}

/// Whether a discovery's type is a legacy one, built from `DiscoveryType::is_legacy`.
static LEGACY_SQL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    in_sql(
        "discovery.discovery_type->>'type'",
        DiscoveryType::iter()
            .filter(DiscoveryType::is_legacy)
            .map(|t| t.id()),
    )
});

/// A historical run's start, read out of the JSONB payload as a timestamp so it orders by time
/// rather than by its text rendering.
const RUN_STARTED_AT_SQL: &str = "(discovery.run_type->'results'->>'started_at')::timestamptz";
/// A historical run's finish, read the same way as [`RUN_STARTED_AT_SQL`].
const RUN_FINISHED_AT_SQL: &str = "(discovery.run_type->'results'->>'finished_at')::timestamptz";

impl OrderField for DiscoveryOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "discovery.created_at",
            Self::Name => "discovery.name",
            Self::UpdatedAt => "discovery.updated_at",
            Self::DaemonId => "discovery.daemon_id",
            Self::SiteId => "discovery.site_id",
            Self::DiscoveryType => "discovery.discovery_type->>'type'",
            Self::Phase => "discovery.run_type->'results'->>'phase'",
            Self::StartedAt => RUN_STARTED_AT_SQL,
            Self::FinishedAt => RUN_FINISHED_AT_SQL,
            Self::Duration => concatcp!("(", RUN_FINISHED_AT_SQL, " - ", RUN_STARTED_AT_SQL, ")"),
            Self::Warnings => {
                "(SELECT COUNT(*) FROM discovery_warnings WHERE discovery_warnings.discovery_id = discovery.id)"
            }
            Self::RunType => "discovery.run_type->>'type'",
            Self::LastRun => "(discovery.run_type->>'last_run')::timestamptz",
            Self::ScanCount => "discovery.scan_count",
            Self::ForceFullScan => "discovery.force_full_scan",
            Self::Legacy => LEGACY_SQL.as_str(),
        }
    }

    fn sorts_by_site_name(&self) -> bool {
        matches!(self, Self::SiteId)
    }

    fn nulls_last(&self) -> bool {
        matches!(
            self,
            Self::Phase | Self::StartedAt | Self::FinishedAt | Self::Duration | Self::LastRun
        )
    }
}

// ============================================================================
// Discovery Filter Query
// ============================================================================

/// Query parameters for filtering and ordering discoveries.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct DiscoveryFilterQuery {
    /// Filter by site ID. Repeat the parameter to pass several.
    #[serde(alias = "site_id")]
    pub site_ids: Option<Vec<Uuid>>,
    /// Filter by daemon ID. Repeat the parameter to pass several.
    #[serde(alias = "daemon_id")]
    pub daemon_ids: Option<Vec<Uuid>>,
    /// Filter by specific entity IDs (for selective loading)
    pub ids: Option<Vec<Uuid>>,
    /// Only runs of one of these discovery types.
    pub discovery_types: Option<Vec<String>>,
    /// Only runs that ended in one of these phases (`run_type.results.phase`). Repeat for
    /// several. Configurations, which have no recorded outcome, never match.
    pub phases: Option<Vec<DiscoveryPhase>>,
    /// `true` returns only completed runs (the history view), `false` only the
    /// configurations that produce them. Omit for both.
    pub historical: Option<bool>,
    /// Free-text search across the discovery's name and the name of the daemon
    /// that runs it.
    pub search: Option<String>,
    /// Only scan configurations that run one of these ways (`Scheduled`, `AdHoc`). Repeat for
    /// several.
    pub run_types: Option<Vec<String>>,
    /// Only configurations of a legacy type (`true`), a current one (`false`), or both (repeat
    /// it). Sent empty, matches none.
    #[serde(default, deserialize_with = "deserialize_bool_values")]
    pub legacy: Option<Vec<bool>>,
    /// Only configurations whose next run is forced to a full scan (`true`), not (`false`), or
    /// both (repeat it). Sent empty, matches none.
    #[serde(default, deserialize_with = "deserialize_bool_values")]
    pub force_full_scan: Option<Vec<bool>>,
    /// Filter by tag IDs (returns discoveries that have ANY of the specified tags)
    pub tag_ids: Option<Vec<Uuid>>,
    /// Tag sets the discovery must hold one tag from each of. Each entry is a comma-separated
    /// list of tag ids. One entry per tag group the list is filtered by.
    pub tag_sets: Option<Vec<String>>,
    /// Group by the tag each row holds from this tag group. Takes the place of `group_by`.
    pub group_by_tag_group: Option<Uuid>,
    /// Sort by the position of the tag each row holds from this tag group. Takes the place of
    /// `order_by`.
    pub order_by_tag_group: Option<Uuid>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<DiscoveryOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<DiscoveryOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl FilterQueryExtractor for DiscoveryFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        // Intersect with what the caller can see — a requested site they
        // have no access to must narrow the result to nothing, never widen it.
        let mut filter = match &self.site_ids {
            Some(requested) => {
                let accessible: Vec<Uuid> = requested
                    .iter()
                    .copied()
                    .filter(|id| user_site_ids.contains(id))
                    .collect();
                filter.site_ids(&accessible)
            }
            None => filter.site_ids(user_site_ids),
        };
        filter = match &self.daemon_ids {
            Some(ids) => filter.daemon_ids(ids),
            None => filter,
        };
        filter = match &self.ids {
            Some(ids) if !ids.is_empty() => filter.entity_ids(ids),
            _ => filter,
        };
        filter = match &self.discovery_types {
            Some(types) if !types.is_empty() => filter.discovery_type_in(types),
            _ => filter,
        };
        filter = match &self.phases {
            Some(phases) if !phases.is_empty() => {
                let ids: Vec<&str> = phases.iter().map(|p| p.id()).collect();
                filter.json_text_in("run_type", &["results", "phase"], &ids)
            }
            _ => filter,
        };
        filter = match self.historical {
            Some(true) => filter.historical_discovery(),
            Some(false) => filter.exclude_historical(),
            None => filter,
        };

        filter
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

/// List discoveries
///
/// Returns discoveries the authenticated user has access to. The run history
/// grows without bound, so this is paginated and ordered server-side rather
/// than filtered in the browser.
#[utoipa::path(
    get,
    path = "",
    tag = Discovery::ENTITY_NAME_PLURAL,
    operation_id = "list_discoveries",
    summary = "List discoveries",
    params(DiscoveryFilterQuery),
    responses(
        (status = 200, description = "List of discoveries", body = PaginatedApiResponse<Discovery>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_discoveries(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<DiscoveryFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Discovery>>> {
    let scope = ListScope::of(&auth.entity)?;
    Ok(Json(list_page::<Discovery>(&state, &scope, &query).await?))
}

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Discovery);
    crate::crud_delete_handler!(Discovery);
    crate::crud_bulk_delete_handler!(Discovery);
    crate::crud_export_csv_handler!(Discovery);
    crate::crud_get_field_values_handler!(Discovery);
}

fn active_session_error() -> ApiError {
    ApiError::coded(
        StatusCode::CONFLICT,
        ErrorCode::EntityDeleteForbidden {
            entity: "discovery".to_string(),
            reason: Some("has an active discovery session — cancel the session first".to_string()),
        },
    )
}

/// A session the daemon is picking up right now can't be cancelled until it has started.
pub(super) fn session_starting_error() -> ApiError {
    ApiError::conflict("The session is starting on its daemon. Try again in a moment.")
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_discoveries, create_discovery))
        .routes(routes!(
            generated::get_by_id,
            update_discovery,
            delete_discovery
        ))
        .routes(routes!(bulk_delete_discoveries))
        .routes(routes!(generated::export_csv))
        .routes(routes!(generated::get_field_values))
        .routes(routes!(start_session))
        .routes(routes!(get_active_sessions))
        .routes(routes!(cancel_discovery))
        .merge(super::bulk::create_router())
        // Internal daemon endpoints
        .routes(routes!(receive_discovery_update))
        // SSE endpoint (internal - not well-supported by OpenAPI). Superseded by
        // `/api/v1/live-updates/stream`; kept one release for API-key clients that read it.
        .route("/stream", get(discovery_stream))
}

/// Delete discovery — blocks if discovery has an active session.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Discovery::ENTITY_NAME_PLURAL,
    operation_id = "delete_discovery",
    summary = "Delete discovery",
    params(("id" = Uuid, Path, description = "discovery ID")),
    responses(
        (status = 200, description = "discovery deleted", body = EmptyApiResponse),
        (status = 404, description = "discovery not found", body = ApiErrorResponse),
        (status = 409, description = "discovery has active session", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_discovery(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    if state
        .services
        .discovery_service
        .has_active_session_for_discovery(&id)
        .await
    {
        return Err(active_session_error());
    }
    generated::delete(state, auth, Path(id)).await
}

/// Bulk delete discoveries — blocks if any discovery has an active session.
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = Discovery::ENTITY_NAME_PLURAL,
    operation_id = "bulk_delete_discoveries",
    summary = "Bulk delete discoveries",
    request_body(content = Vec<Uuid>, description = "Array of Discovery IDs to delete"),
    responses(
        (status = 200, description = "discoveries deleted", body = ApiResponse<crate::server::shared::handlers::traits::BulkDeleteResponse>),
        (status = 409, description = "discovery has active session", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_delete_discoveries(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<crate::server::shared::handlers::traits::BulkDeleteResponse>>> {
    for id in &ids {
        if state
            .services
            .discovery_service
            .has_active_session_for_discovery(id)
            .await
        {
            return Err(active_session_error());
        }
    }
    generated::bulk_delete(state, auth, ApiJson(ids)).await
}

/// Create discovery
#[utoipa::path(
    post,
    path = "",
    tag = Discovery::ENTITY_NAME_PLURAL,
    request_body = Discovery,
    responses(
        (status = 200, description = "Discovery created successfully", body = ApiResponse<Discovery>),
        (status = 400, description = "Invalid subnet site", body = ApiErrorResponse),
        (status = 400, description = "Can't create historical discovery", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_discovery(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    ApiJson(discovery): ApiJson<Discovery>,
) -> ApiResult<Json<ApiResponse<Discovery>>> {
    if discovery.base.run_type.is_server_managed() {
        return Err(ApiError::discovery_historical_read_only());
    }
    if discovery
        .base
        .discovery_type
        .rescan_target_host_ids()
        .is_some()
    {
        return Err(ApiError::bad_request(
            "Rescans are created by the server and cannot be submitted \
             via the API.",
        ));
    }

    // Reject legacy discovery types — only Unified can be created
    if discovery.base.discovery_type.is_legacy() {
        return Err(ApiError::bad_request(&format!(
            "{} discoveries are frozen. Create a Unified discovery instead.",
            discovery.base.discovery_type,
        )));
    }

    // Ahead of the gates below, which read the daemon and credentials the body names.
    validate_entity_references::<Discovery>(
        &state,
        &discovery,
        None,
        &auth.site_ids(),
        auth.require_organization_id()?,
    )
    .await?;

    // For Unified: check daemon version supports it
    if discovery.base.discovery_type.runs_network_scan() {
        let daemon = state
            .services
            .daemon_service
            .get_by_id(&discovery.base.daemon_id)
            .await?
            .ok_or_else(|| ApiError::not_found("Daemon not found".to_string()))?;

        if !supports_unified_discovery(daemon.base.version.as_ref()) {
            // Distinguish "we don't know the version yet" from "the version is too
            // old". A provisioned daemon carries no version until it first contacts
            // the server (provisioning no longer writes one optimistically), so a
            // never-connected daemon lands here with `None` — telling that operator
            // to "upgrade" is misleading; the daemon simply hasn't checked
            // in yet.
            return Err(match daemon.base.version {
                None => ApiError::bad_request(
                    "Daemon has not connected to the server yet, so its version is unknown. \
                     Wait for it to contact the server, then trigger discovery again.",
                ),
                Some(_) => ApiError::bad_request(&format!(
                    "Daemon does not support unified discovery. Upgrade to a version newer \
                     than {}.",
                    crate::server::daemons::r#impl::version::last_without_unified_discovery()
                )),
            });
        }
    }

    // Validate subnet site membership for Network and Unified types
    let subnet_ids_to_check = match &discovery.base.discovery_type {
        DiscoveryType::Network { subnet_ids, .. } | DiscoveryType::Unified { subnet_ids, .. } => {
            subnet_ids.clone()
        }
        _ => None,
    };
    if let Some(ids) = subnet_ids_to_check {
        for subnet_id in &ids {
            if let Some(subnet) = state.services.subnet_service.get_by_id(subnet_id).await?
                && subnet.base.site_id != discovery.base.site_id
            {
                return Err(ApiError::discovery_subnet_site_mismatch(subnet.base.name()));
            }
        }
    }

    // Delegate to generic handler (handles validation, auth checks, creation)
    create_handler::<Discovery>(State(state), auth, ApiJson(discovery)).await
}

/// Update discovery
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Discovery::ENTITY_NAME_PLURAL,
    params(("id" = uuid::Uuid, Path, description = "Discovery ID")),
    request_body = Discovery,
    responses(
        (status = 200, description = "Discovery updated successfully", body = ApiResponse<Discovery>),
        (status = 400, description = "Invalid subnet site", body = ApiErrorResponse),
        (status = 400, description = "Can't update historical discovery", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn update_discovery(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    id: Path<Uuid>,
    ApiJson(discovery): ApiJson<Discovery>,
) -> ApiResult<Json<ApiResponse<Discovery>>> {
    if discovery.base.run_type.is_server_managed() {
        return Err(ApiError::discovery_historical_read_only());
    }
    if discovery
        .base
        .discovery_type
        .rescan_target_host_ids()
        .is_some()
    {
        return Err(ApiError::bad_request(
            "Rescans are created by the server and cannot be submitted \
             via the API.",
        ));
    }

    let existing = state.services.discovery_service.get_by_id(&id).await?;

    // Ahead of the gates below, which read the daemon and credentials the body names and quote
    // them in their refusals.
    if let Some(existing) = &existing {
        validate_site_access(Some(existing.base.site_id), &auth.site_ids(), "update")?;
    }
    validate_entity_references::<Discovery>(
        &state,
        &discovery,
        existing.as_ref(),
        &auth.site_ids(),
        auth.require_organization_id()?,
    )
    .await?;

    // Reject changing a legacy discovery's type
    if let Some(existing) = &existing
        && existing.base.discovery_type.is_legacy()
        && std::mem::discriminant(&existing.base.discovery_type)
            != std::mem::discriminant(&discovery.base.discovery_type)
    {
        return Err(ApiError::bad_request(
            "Cannot change the type of a legacy discovery. Create a new Unified discovery instead.",
        ));
    }

    // Single-endpoint guard: a daemon host can't run two credentials of the same
    // integration (e.g. a Docker socket + a Docker proxy, or the Podman pair). All
    // of this discovery's daemon-host targets resolve to its daemon's host, so check
    // them against each other before persisting. (Daemon-host credentials themselves
    // are managed via the host/credential modals — this discovery save never creates
    // or removes them.)
    if let Some(daemon) = state
        .services
        .daemon_service
        .get_by_id(&discovery.base.daemon_id)
        .await?
    {
        // Version gate: a Discovery binds to exactly one daemon, so reject up front
        // any target whose credential type is too new for that daemon to receive.
        // This is the authoritative "prevent adding" — the server-side dispatch filter
        // would otherwise silently drop the mapping at scan time.
        for target in &discovery.integration_targets {
            if let Some(cred) = state
                .services
                .credential_service
                .get_by_id(&target.credential_id())
                .await?
            {
                if let Some(refusal) = cred
                    .base
                    .daemon_os_refusal(&daemon.base.name, daemon.base.os)
                {
                    return Err(ApiError::bad_request(&refusal));
                }
                if let Some(refusal) = crate::server::credentials::service::refusal_for_daemon(
                    &cred,
                    &daemon.base.name,
                    daemon.base.version.as_ref(),
                ) {
                    return Err(ApiError::bad_request(&refusal));
                }
            }
        }

        if let Some((a, b)) = state
            .services
            .credential_service
            .find_daemon_host_target_conflict(daemon.base.host_id, &discovery.integration_targets)
            .await?
        {
            return Err(ApiError::bad_request(&format!(
                "\"{a}\" and \"{b}\" both target this daemon's host for the same integration, which allows only one credential per host. Remove one."
            )));
        }

        // Targets stay on the Discovery — including daemon-host ones. They're offered to the
        // daemon until a scan completes, and assigned to a host only once they probe
        // successfully, so saving a target here never assigns a credential on its own. A target
        // that matched is retried from the junction it earned; one that matched nothing is
        // dropped at completion, so re-adding it here is how a user retries it.
    }

    update_handler::<Discovery>(state, auth, id, ApiJson(discovery)).await
}

/// Receive discovery progress from daemon
///
/// Internal endpoint for daemons to report discovery progress.
#[utoipa::path(
    post,
    path = "/{session_id}/update",
    tags = [Discovery::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("session_id" = Uuid, Path, description = "Discovery session ID")),
    request_body = DiscoveryUpdatePayload,
    responses(
        (status = 200, description = "Update received", body = EmptyApiResponse),
    ),
    security(("daemon_api_key" = []))
)]
pub(crate) async fn receive_discovery_update(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(_session_id): Path<Uuid>,
    ApiJson(update): ApiJson<DiscoveryUpdatePayload>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // IsDaemon guarantees exactly one site_id and a daemon_id
    let daemon_site_id = auth.site_ids()[0];
    let daemon_id = auth
        .daemon_id()
        .ok_or_else(|| anyhow::anyhow!("Could not get daemon ID from authentication"))?;

    // Delegate to processor for shared progress update logic
    // This ensures both DaemonPoll and ServerPoll modes use the same logic, the site and identity
    // checks included
    state
        .services
        .daemon_service
        .process_discovery_progress(daemon_id, daemon_site_id, update)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

/// Start discovery session
#[utoipa::path(
    post,
    operation_id = "start_discovery_session",
    path = "/start-session",
    tag = Discovery::ENTITY_NAME_PLURAL,
    request_body = Uuid,
    responses(
        (status = 200, description = "Discovery session started", body = ApiResponse<DiscoveryUpdatePayload>),
        (status = 404, description = "Discovery not found", body = ApiErrorResponse),
        (status = 409, description = "A session is already running for this discovery", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn start_session(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    ApiJson(discovery_id): ApiJson<Uuid>,
) -> ApiResult<Json<ApiResponse<DiscoveryUpdatePayload>>> {
    let site_ids = auth.site_ids();
    let entity = auth.into_entity();

    let discovery = state
        .services
        .discovery_service
        .get_by_id(&discovery_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Discovery>(discovery_id))?;

    // Validate user has access to this discovery's site
    if !site_ids.contains(&discovery.base.site_id) {
        return Err(ApiError::entity_access_denied::<Site>(
            discovery.base.site_id,
        ));
    }
    let running = state
        .services
        .discovery_service
        .has_active_session_for_discovery(&discovery.id)
        .await;
    if let Some(refusal) = start_refusal(&discovery, running) {
        return Err(refusal.into_api_error(&discovery.base.name));
    }

    wake_daemon(&state, discovery.base.daemon_id).await?;

    let update = state
        .services
        .discovery_service
        .start_session(discovery, entity)
        .await?;

    Ok(Json(ApiResponse::success(update)))
}

/// Takes the discovery's daemon off standby, so it picks up the session about to be queued.
pub(super) async fn wake_daemon(state: &Arc<AppState>, daemon_id: Uuid) -> Result<(), ApiError> {
    if let Some(mut daemon) = state.services.daemon_service.get_by_id(&daemon_id).await? {
        state
            .services
            .daemon_service
            .wake_from_standby(&mut daemon)
            .await?;
    }
    Ok(())
}

/// Discovery progress only. Deprecated: the app reads `/api/v1/live-updates/stream`, which carries
/// the same updates as `LiveUpdate::Discovery`. Remove in the next release.
async fn discovery_stream(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.services.discovery_service.subscribe();
    let allowed_sites = auth.site_ids();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(update) => {
                    // Only emit if user has access to this discovery's site
                    if allowed_sites.contains(&update.site_id) {
                        let json = serde_json::to_string(&update).unwrap_or_default();
                        yield Ok(Event::default().data(json));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("SSE client lagged by {} messages", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(until_shutdown(stream, &state.shutdown)).keep_alive(KeepAlive::default())
}

/// List active discovery sessions
#[utoipa::path(
    get,
    operation_id = "list_active_discovery_sessions",
    path = "/active-sessions",
    tag = Discovery::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "List of active discovery sessions", body = ApiResponse<Vec<DiscoveryUpdatePayload>>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_active_sessions(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> ApiResult<Json<ApiResponse<Vec<DiscoveryUpdatePayload>>>> {
    let site_ids = auth.site_ids();
    let sessions = state
        .services
        .discovery_service
        .get_all_sessions(&site_ids)
        .await;

    Ok(Json(ApiResponse::success(sessions)))
}

/// Cancel discovery session
#[utoipa::path(
    post,
    operation_id = "cancel_discovery_session",
    path = "/{session_id}/cancel",
    tag = Discovery::ENTITY_NAME_PLURAL,
    params(("session_id" = Uuid, Path, description = "Session ID")),
    responses(
        (status = 200, description = "Discovery session cancelled", body = EmptyApiResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn cancel_discovery(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Get session and validate user has access to this session's site
    let session = state
        .services
        .discovery_service
        .get_session(&session_id)
        .await
        .ok_or_else(|| ApiError::discovery_session_not_found(session_id))?;

    if !auth.site_ids().contains(&session.site_id) {
        return Err(ApiError::entity_access_denied::<Site>(session.site_id));
    }
    if session.phase == DiscoveryPhase::Starting {
        return Err(session_starting_error());
    }

    state
        .services
        .discovery_service
        .cancel_session(session_id, auth.into_entity())
        .await?;

    Ok(Json(ApiResponse::success(())))
}
