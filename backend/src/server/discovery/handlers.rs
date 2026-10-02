use crate::server::openapi::tags as api_tags;
use crate::server::{
    auth::middleware::{
        auth::AuthenticatedEntity,
        permissions::{Authorized, IsDaemon, Member, Viewer},
    },
    config::AppState,
    credentials::r#impl::types::{CredentialTypeDiscriminants, OsFamily},
    daemons::r#impl::{api::DiscoveryUpdatePayload, version::supports_unified_discovery},
    discovery::r#impl::{base::Discovery, types::DiscoveryType},
    networks::r#impl::Network,
    shared::{
        extractors::Query,
        handlers::{
            ordering::OrderField,
            query::{FilterQueryExtractor, OrderDirection, PaginationParams},
            traits::{create_handler, update_handler},
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
    },
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
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::broadcast;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Discovery Ordering
// ============================================================================

/// Fields that discoveries can be ordered/grouped by.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryOrderField {
    /// Newest-first is the default reading of a run history, so this is the one
    /// people reach for. Kept as the enum default too.
    #[default]
    CreatedAt,
    Name,
    UpdatedAt,
    DaemonId,
    NetworkId,
    /// `discovery_type` is JSONB, so this reads its tag the way `json_field_eq` does.
    DiscoveryType,
}

impl OrderField for DiscoveryOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "discovery.created_at",
            Self::Name => "discovery.name",
            Self::UpdatedAt => "discovery.updated_at",
            Self::DaemonId => "discovery.daemon_id",
            Self::NetworkId => "discovery.network_id",
            Self::DiscoveryType => "discovery.discovery_type->>'type'",
        }
    }
}

// ============================================================================
// Discovery Filter Query
// ============================================================================

/// Query parameters for filtering and ordering discoveries.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct DiscoveryFilterQuery {
    /// Filter by network ID. Repeat the parameter to pass several.
    #[serde(alias = "network_id")]
    pub network_ids: Option<Vec<Uuid>>,
    /// Filter by daemon ID. Repeat the parameter to pass several.
    #[serde(alias = "daemon_id")]
    pub daemon_ids: Option<Vec<Uuid>>,
    /// Only runs of one of these discovery types.
    pub discovery_types: Option<Vec<String>>,
    /// `true` returns only completed runs (the history view), `false` only the
    /// configurations that produce them. Omit for both.
    pub historical: Option<bool>,
    /// Free-text search across the discovery's name and the name of the daemon
    /// that runs it.
    pub search: Option<String>,
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

impl DiscoveryFilterQuery {
    /// Build the ORDER BY clause.
    pub fn apply_ordering(
        &self,
        filter: StorableFilter<Discovery>,
    ) -> (StorableFilter<Discovery>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "discovery.created_at DESC",
        )
    }
}

impl FilterQueryExtractor for DiscoveryFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        // Intersect with what the caller can see — a requested network they
        // have no access to must narrow the result to nothing, never widen it.
        let mut filter = match &self.network_ids {
            Some(requested) => {
                let accessible: Vec<Uuid> = requested
                    .iter()
                    .copied()
                    .filter(|id| user_network_ids.contains(id))
                    .collect();
                filter.network_ids(&accessible)
            }
            None => filter.network_ids(user_network_ids),
        };
        filter = match &self.daemon_ids {
            Some(ids) => filter.daemon_ids(ids),
            None => filter,
        };
        filter = match &self.discovery_types {
            Some(types) if !types.is_empty() => filter.discovery_type_in(types),
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

/// List all discoveries
///
/// Returns discoveries the authenticated user has access to. The run history
/// grows without bound, so this is paginated and ordered server-side rather
/// than filtered in the browser.
#[utoipa::path(
    get,
    path = "",
    tag = Discovery::ENTITY_NAME_PLURAL,
    operation_id = "get_all_discoveries",
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
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let base_filter = StorableFilter::<Discovery>::new_from_network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);

    // Server-side because the list is paginated: a client-side search would
    // only ever match the page already loaded.
    let filter = match query.search.as_deref() {
        Some(search) if !search.trim().is_empty() => filter.text_search(search),
        _ => filter,
    };

    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);
    let (filter, order_by) = query.apply_ordering(filter);

    // Grouped lists report each group's full size, not the slice on this page.
    let group_counts = match query.group_by {
        Some(group_field) => Some(
            state
                .services
                .discovery_service
                .count_by_group(filter.clone(), group_field.to_sql())
                .await?,
        ),
        None => None,
    };

    let result = state
        .services
        .discovery_service
        .get_paginated_ordered(filter, &order_by)
        .await?;

    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    let response = PaginatedApiResponse::success(result.items, result.total_count, limit, offset);

    Ok(Json(match group_counts {
        Some(counts) => response.with_group_counts(counts),
        None => response,
    }))
}

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Discovery);
    crate::crud_delete_handler!(Discovery);
    crate::crud_bulk_delete_handler!(Discovery);
    crate::crud_export_csv_handler!(Discovery);
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
        .routes(routes!(start_session))
        .routes(routes!(get_active_sessions))
        .routes(routes!(cancel_discovery))
        // Internal daemon endpoints
        .routes(routes!(receive_discovery_update))
        // SSE endpoint (internal - not well-supported by OpenAPI)
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
    Json(ids): Json<Vec<Uuid>>,
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
    generated::bulk_delete(state, auth, Json(ids)).await
}

/// Create new Discovery
#[utoipa::path(
    post,
    path = "",
    tag = Discovery::ENTITY_NAME_PLURAL,
    request_body = Discovery,
    responses(
        (status = 200, description = "Discovery created successfully", body = ApiResponse<Discovery>),
        (status = 400, description = "Invalid subnet network", body = ApiErrorResponse),
        (status = 400, description = "Can't create historical discovery", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_discovery(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(discovery): Json<Discovery>,
) -> ApiResult<Json<ApiResponse<Discovery>>> {
    if discovery.base.run_type.is_server_managed() {
        return Err(ApiError::discovery_historical_read_only());
    }
    if discovery
        .base
        .discovery_type
        .rescan_target_host_id()
        .is_some()
    {
        return Err(ApiError::bad_request(
            "Rescans are created by the server for a single host and cannot be submitted \
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
            // to "upgrade to 0.15.0" is misleading; the daemon simply hasn't checked
            // in yet.
            return Err(match daemon.base.version {
                None => ApiError::bad_request(
                    "Daemon has not connected to the server yet, so its version is unknown. \
                     Wait for it to contact the server, then trigger discovery again.",
                ),
                Some(_) => ApiError::bad_request(
                    "Daemon does not support unified discovery. Upgrade to version 0.15.0 or later.",
                ),
            });
        }
    }

    // Validate subnet network membership for Network and Unified types
    let subnet_ids_to_check = match &discovery.base.discovery_type {
        DiscoveryType::Network { subnet_ids, .. } | DiscoveryType::Unified { subnet_ids, .. } => {
            subnet_ids.clone()
        }
        _ => None,
    };
    if let Some(ids) = subnet_ids_to_check {
        for subnet_id in &ids {
            if let Some(subnet) = state.services.subnet_service.get_by_id(subnet_id).await?
                && subnet.base.network_id != discovery.base.network_id
            {
                return Err(ApiError::discovery_subnet_network_mismatch(
                    &subnet.base.name,
                ));
            }
        }
    }

    // Delegate to generic handler (handles validation, auth checks, creation)
    create_handler::<Discovery>(State(state), auth, Json(discovery)).await
}

/// Update Discovery
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Discovery::ENTITY_NAME_PLURAL,
    params(("id" = uuid::Uuid, Path, description = "Discovery ID")),
    request_body = Discovery,
    responses(
        (status = 200, description = "Discovery updated successfully", body = ApiResponse<Discovery>),
        (status = 400, description = "Invalid subnet network", body = ApiErrorResponse),
        (status = 400, description = "Can't update historical discovery", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn update_discovery(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    id: Path<Uuid>,
    discovery: Json<Discovery>,
) -> ApiResult<Json<ApiResponse<Discovery>>> {
    if discovery.base.run_type.is_server_managed() {
        return Err(ApiError::discovery_historical_read_only());
    }
    if discovery
        .base
        .discovery_type
        .rescan_target_host_id()
        .is_some()
    {
        return Err(ApiError::bad_request(
            "Rescans are created by the server for a single host and cannot be submitted \
             via the API.",
        ));
    }

    // Reject changing a legacy discovery's type
    if let Some(existing) = state.services.discovery_service.get_by_id(&id).await?
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
                    .daemon_os_refusal(&daemon.base.name, daemon.base.os.map(OsFamily::from))
                {
                    return Err(ApiError::bad_request(&refusal));
                }
                let disc = CredentialTypeDiscriminants::from(&cred.base.credential_type);
                if !disc.compatible_with_daemon(daemon.base.version.as_ref()) {
                    return Err(ApiError::bad_request(&format!(
                        "Credential type \"{}\" requires daemon version {} or newer, but this daemon is on {}. Upgrade the daemon or choose a compatible credential type.",
                        disc.display_name(),
                        disc.minimum_daemon_version(),
                        daemon
                            .base
                            .version
                            .as_ref()
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "an unknown version".to_string()),
                    )));
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

    update_handler::<Discovery>(state, auth, id, discovery).await
}

/// Receive discovery progress update from daemon
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
async fn receive_discovery_update(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(_session_id): Path<Uuid>,
    Json(update): Json<DiscoveryUpdatePayload>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // IsDaemon guarantees exactly one network_id and a daemon_id
    let daemon_network_id = auth.network_ids()[0];
    let daemon_id = auth
        .daemon_id()
        .ok_or_else(|| anyhow::anyhow!("Could not get daemon ID from authentication"))?;

    // Validate daemon can only send updates for their own network
    if update.network_id != daemon_network_id {
        return Err(ApiError::daemon_network_mismatch());
    }

    // Validate daemon can only send updates as themselves
    if update.daemon_id != daemon_id {
        return Err(ApiError::daemon_identity_mismatch());
    }

    // Delegate to processor for shared progress update logic
    // This ensures both DaemonPoll and ServerPoll modes use the same logic
    state
        .services
        .daemon_service
        .process_discovery_progress(update)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

/// Start a Discovery Session
#[utoipa::path(
    post,
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
    Json(discovery_id): Json<Uuid>,
) -> ApiResult<Json<ApiResponse<DiscoveryUpdatePayload>>> {
    let network_ids = auth.network_ids();
    let entity = auth.into_entity();

    let discovery = state
        .services
        .discovery_service
        .get_by_id(&discovery_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Discovery>(discovery_id))?;

    // Validate user has access to this discovery's network
    if !network_ids.contains(&discovery.base.network_id) {
        return Err(ApiError::entity_access_denied::<Network>(
            discovery.base.network_id,
        ));
    }

    // Auto-wake daemon if on standby. Grant the same grace window as the
    // restart reactivation path so the nightly inactivity check doesn't
    // re-standby the daemon before the just-queued discovery completes.
    if let Some(mut daemon) = state
        .services
        .daemon_service
        .get_by_id(&discovery.base.daemon_id)
        .await?
        && daemon.base.standby
    {
        daemon.base.standby = false;
        daemon.base.standby_cleared_at = Some(chrono::Utc::now());
        state
            .services
            .daemon_service
            .update(&mut daemon, AuthenticatedEntity::System)
            .await?;
        tracing::info!(
            daemon_id = %daemon.id,
            "Cleared daemon standby (discovery session started)"
        );
    }

    let update = state
        .services
        .discovery_service
        .start_session(discovery, entity)
        .await?;

    Ok(Json(ApiResponse::success(update)))
}

async fn discovery_stream(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.services.discovery_service.subscribe();
    let allowed_networks = auth.network_ids();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(update) => {
                    // Only emit if user has access to this discovery's network
                    if allowed_networks.contains(&update.network_id) {
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

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Get active Discovery Sessions
#[utoipa::path(
    get,
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
    let network_ids = auth.network_ids();
    let sessions = state
        .services
        .discovery_service
        .get_all_sessions(&network_ids)
        .await;

    Ok(Json(ApiResponse::success(sessions)))
}

/// Cancel a Discovery Session
#[utoipa::path(
    post,
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
    // Get session and validate user has access to this session's network
    let session = state
        .services
        .discovery_service
        .get_session(&session_id)
        .await
        .ok_or_else(|| ApiError::discovery_session_not_found(session_id))?;

    if !auth.network_ids().contains(&session.network_id) {
        return Err(ApiError::entity_access_denied::<Network>(
            session.network_id,
        ));
    }

    state
        .services
        .discovery_service
        .cancel_session(session_id, auth.into_entity())
        .await?;

    tracing::info!("Discovery session was {} cancelled", session_id);
    Ok(Json(ApiResponse::success(())))
}
