use crate::server::openapi::tags as api_tags;
use crate::server::shared::extractors::Query;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::ApiJson;
use crate::server::{
    auth::middleware::{
        features::{ConfluenceExportFeature, MermaidExportFeature, RequireFeature},
        permissions::{Authorized, Can, Read, Write},
    },
    config::AppState,
    setup::types::{SetupItemDiscriminants, SetupOperation},
    shared::{
        handlers::{
            query::{FilterQueryExtractor, SiteFilterQuery},
            traits::{CrudHandlers, update_handler},
        },
        services::traits::CrudService,
        storage::filter::StorableFilter,
        types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, PaginatedApiResponse},
    },
    // NOTE: `EmptyApiResponse` and the layout-override request DTOs
    // (`TopologyNodePositionUpdate` / `TopologyNodeResizeUpdate` /
    // `TopologyEdgeHandleUpdate`, defined in `topology::types::base`) are only
    // needed by the disabled node-position / node-resize / edge-handle
    // endpoints below. Re-add them here when those endpoints are revived.
    topology::types::{api::TopologyData, base::Topology, views::TopologyView},
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::sync::Arc;
use strum::IntoEnumIterator;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for generic CRUD operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Topology);
    crate::crud_export_csv_handler!(Topology);
}

/// Topology endpoints are internal-only (hidden from public docs).
///
/// A topology row now holds only the user's grouping `options` (one live row
/// per site). The per-view graph is built on request and returned on the
/// `/data` bundle; `get_all` / `get_by_id` list the slim rows, `update_topology`
/// persists `options`, plus exports. Live changes reach the app on the live-update stream
/// (`live_updates::handlers`).
///
/// Creation, deletion, lock/unlock, refresh, rebuild, metadata updates, and the
/// layout-override mutators are gone (overrides aren't persisted — see the
/// disabled handlers below). The live row is auto-created at site creation;
/// snapshots build their graph on request from closed copies (no snapshot rows).
pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_topologies))
        .routes(routes!(get_topology_data))
        .routes(routes!(generated::get_by_id, update_topology))
        .routes(routes!(generated::export_csv))
        .routes(routes!(export_mermaid))
        .routes(routes!(export_confluence))
    // Layout-override endpoints are DISABLED — overrides are no longer
    // persisted (the graph builds on request and ELK re-lays out every
    // render, so there's no mechanism to save position/size/handle
    // changes). Kept (commented) for revival; see the handlers below.
    // .routes(routes!(update_node_position))
    // .routes(routes!(update_node_resize))
    // .routes(routes!(update_edge_handles))
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct TopologyDataQuery {
    /// Site to read entities for. Required.
    pub site_id: Uuid,
    /// When set, returns the entity set as it was when this snapshot was taken.
    /// When omitted, returns live entities.
    #[serde(default)]
    pub snapshot_id: Option<Uuid>,
    /// When `true`, records the `OpenTopology` setup item (the user has viewed their topology),
    /// and `ExploreViews` when `view` is not the default view.
    /// Only the frontend's explicit on-tab view sets this — the background topology-data query
    /// never does — so it is never recorded from other tabs. Once per org.
    #[serde(default)]
    pub mark_viewed: Option<bool>,
    /// The view to build the graph for. When omitted, the graph of every view is built and
    /// returned. The topology tab always sends the view it draws.
    #[serde(default)]
    pub view: Option<TopologyView>,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct TopologyExportQuery {
    /// View to export. Defaults to the default view when omitted.
    #[serde(default)]
    pub view: TopologyView,
}

/// Get topology view data
///
/// The entity set the topology view draws.
///
/// `?snapshot_id=<id>` resolves to the snapshot's `taken_at` and returns the
/// as-of-T entity set; otherwise returns live entities. The frontend
/// `TopologyTab` is the sole intended consumer.
#[utoipa::path(
    get,
    path = "/data",
    tags = [Topology::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(TopologyDataQuery),
    responses(
        (status = 200, description = "Topology entity bundle", body = ApiResponse<TopologyData>),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Snapshot not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_topology_data(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Topology, Read>>,
    Query(params): Query<TopologyDataQuery>,
) -> ApiResult<Json<ApiResponse<TopologyData>>> {
    let site_ids = auth.site_ids();
    if !site_ids.contains(&params.site_id) {
        return Err(ApiError::forbidden("You don't have access to this site"));
    }

    // Snapshot path: verify the snapshot belongs to the requested site
    // before returning its closed copies.
    if let Some(snapshot_id) = params.snapshot_id {
        let snapshot = state
            .services
            .snapshot_service
            .get_by_id(&snapshot_id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?
            .ok_or_else(|| ApiError::not_found("Snapshot not found".to_string()))?;
        if snapshot.base.site_id != params.site_id {
            return Err(ApiError::forbidden("Snapshot belongs to a different site"));
        }
    }

    // Build the requested view's graph on request from entities + the site's
    // grouping options (live or snapshot entity set). The frontend runs ELK
    // each render.
    let views: Vec<TopologyView> = match params.view {
        Some(view) => vec![view],
        None => TopologyView::iter().collect(),
    };
    let mut data = state
        .services
        .topology_service
        .get_topology_render_data(params.site_id, params.snapshot_id, &views)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    let organization_id = auth.require_organization_id()?;
    data.has_interface_credential = state
        .services
        .credential_service
        .has_manual_interface_credential(organization_id)
        .await?;

    // Record OpenTopology, and ExploreViews for a view other than the default, only when the
    // frontend explicitly requests it (`mark_viewed=true`), which it does solely while the
    // topology tab is focused. The background topology-data query never sets the flag, so neither
    // is recorded from other tabs. Still gated to a live view with hosts once RunFirstScan is
    // done: the unlocked read skips the org lock once recorded, and `record_setup` decides which
    // request publishes.
    if params.mark_viewed.unwrap_or(false)
        && params.snapshot_id.is_none()
        && !data.hosts.is_empty()
        && let Ok(Some(org)) = state
            .services
            .organization_service
            .get_by_id(&organization_id)
            .await
        && org.base.plan.features().includes_topology()
        && org.has_setup(SetupItemDiscriminants::RunFirstScan)
    {
        let entity = auth.into_entity();
        for item_type in setup_items_viewed(params.view) {
            if org.has_setup(item_type) {
                continue;
            }
            if let Err(e) = state
                .services
                .organization_service
                .record_setup(
                    org.id,
                    SetupOperation::Completed {
                        item_type,
                        subject_id: None,
                    },
                    entity.clone(),
                )
                .await
            {
                tracing::warn!(error = %e, ?item_type, "Could not record a topology view");
            }
        }
    }

    Ok(Json(ApiResponse::success(data)))
}

/// The Setup items viewing `view` on the topology tab completes: OpenTopology, and ExploreViews
/// for a view other than the default. A fetch of every view (`None`) is no view switch.
fn setup_items_viewed(view: Option<TopologyView>) -> Vec<SetupItemDiscriminants> {
    let mut viewed = vec![SetupItemDiscriminants::OpenTopology];
    if view.is_some_and(|v| v != TopologyView::default()) {
        viewed.push(SetupItemDiscriminants::ExploreViews);
    }
    viewed
}

/// Update topology
///
/// Saves the topology's view options.
#[utoipa::path(
    put,
    path = "/{id}",
    tags = [Topology::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = Topology,
    responses(
        (status = 200, description = "Topology updated", body = ApiResponse<Topology>),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_topology(
    state: State<Arc<AppState>>,
    auth: Authorized<Can<Topology, Write>>,
    id: Path<Uuid>,
    topology: ApiJson<Topology>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    // `snapshot_id` is immutable post-insert; `Storable::preserve_immutable_fields`
    // restores it from the existing row before the UPDATE statement runs, so
    // any client-supplied value here is silently ignored. Layout changes
    // (nodes/edges/options) are the only mutations this handler accepts.
    update_handler::<Topology>(state, auth, id, topology).await
}

/// List topologies
///
/// Topologies for the caller's sites.
///
/// Returns both live-view rows (`snapshot_id IS NULL`) and snapshot-pinned
/// rows. The frontend renders the live one by default and renders snapshot
/// rows when the user picks one from the snapshots dropdown.
#[utoipa::path(
    get,
    operation_id = "list_topologies",
    path = "",
    tags = [Topology::ENTITY_NAME_PLURAL],
    params(SiteFilterQuery),
    responses(
        (status = 200, description = "List of topologies", body = PaginatedApiResponse<Topology>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_topologies(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Topology, Read>>,
    query: Query<SiteFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Topology>>> {
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let base_filter = StorableFilter::<Topology>::new_from_site_ids(&site_ids);
    let filter = query.apply_to_filter(base_filter, &site_ids, organization_id);
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    let service = Topology::get_service(&state);
    let mut result = service.get_paginated(filter).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch topologies");
        ApiError::internal_error(&e.to_string())
    })?;

    // Fill in any product default the stored options predate.
    //
    // The hide-set is applied client-side, so this response is what decides which values are
    // hidden. `#[serde(default)]` is a whole-field fallback and cannot help here: a row written
    // before a filter existed carries a complete `hide_metadata_values` map that simply has no
    // entry for it, so without this every existing topology would keep rendering as though the
    // filter had been switched off — which for L2's link-state filter means every ifTable row.
    // Only absent keys are filled; see `merge_missing_hide_defaults`.
    //
    // The infrastructure rule the grouping-rules migration wrote has no categories; it takes the
    // organization's set here, so the rule editor shows what the rule is meant to hold.
    let use_case = Topology::get_service(&state)
        .organization_use_case(organization_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    for topology in &mut result.items {
        topology.base.options.request.merge_missing_hide_defaults();
        topology
            .base
            .options
            .request
            .populate_empty_infra_rule(use_case);
    }

    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        result.items,
        result.total_count,
        limit,
        offset,
    )))
}

// === DISABLED: layout-override endpoints ===
// Overrides (node drag/resize, edge handle reconnect) are no longer persisted —
// there is no mechanism to save position/size/handle changes (the graph builds
// on request and ELK re-lays out every render). Kept commented for revival;
// when reviving, re-add the routes in `create_router` and the imports
// (`EmptyApiResponse` + the `Topology*Update` DTOs).
/*
/// Update a single node's position
#[utoipa::path(
    post,
    path = "/{id}/node-position",
    tags = [Topology::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyNodePositionUpdate,
    responses(
        (status = 200, description = "Node position updated", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology or node not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_node_position(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Topology, Write>>,
    Path(id): Path<Uuid>,
    ApiJson(request): ApiJson<TopologyNodePositionUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let site_ids = auth.site_ids();

    if !site_ids.contains(&request.site_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's site",
        ));
    }

    let service = Topology::get_service(&state);

    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    let node = topology
        .base
        .nodes
        .get_mut(&request.view)
        .and_then(|nodes| nodes.iter_mut().find(|n| n.id == request.node_id))
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Node {} not found in {:?} view",
                request.node_id, request.view
            ))
        })?;

    node.position = request.position;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Update an edge's handles
#[utoipa::path(
    post,
    path = "/{id}/edge-handles",
    tags = [Topology::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyEdgeHandleUpdate,
    responses(
        (status = 200, description = "Edge handles updated", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology or edge not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_edge_handles(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Topology, Write>>,
    Path(id): Path<Uuid>,
    ApiJson(request): ApiJson<TopologyEdgeHandleUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let site_ids = auth.site_ids();

    if !site_ids.contains(&request.site_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's site",
        ));
    }

    let service = Topology::get_service(&state);

    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    let edge = topology
        .base
        .edges
        .get_mut(&request.view)
        .and_then(|edges| edges.iter_mut().find(|e| e.id == request.edge_id))
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Edge {} not found in {:?} view",
                request.edge_id, request.view
            ))
        })?;

    edge.source_handle = request.source_handle;
    edge.target_handle = request.target_handle;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Update a node's size and position
#[utoipa::path(
    post,
    path = "/{id}/node-resize",
    tags = [Topology::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyNodeResizeUpdate,
    responses(
        (status = 200, description = "Node resized", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology or node not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_node_resize(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Topology, Write>>,
    Path(id): Path<Uuid>,
    ApiJson(request): ApiJson<TopologyNodeResizeUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let site_ids = auth.site_ids();

    if !site_ids.contains(&request.site_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's site",
        ));
    }

    let service = Topology::get_service(&state);

    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    let node = topology
        .base
        .nodes
        .get_mut(&request.view)
        .and_then(|nodes| nodes.iter_mut().find(|n| n.id == request.node_id))
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Node {} not found in {:?} view",
                request.node_id, request.view
            ))
        })?;

    node.size = request.size;
    node.position = request.position;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}
*/

/// Export topology as Mermaid flowchart
#[utoipa::path(
    get,
    operation_id = "export_topology_mermaid",
    path = "/{id}/export/mermaid",
    tags = [Topology::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "Topology ID"), TopologyExportQuery),
    responses(
        (status = 200, description = "Mermaid flowchart export", content_type = "text/plain", body = String),
        (status = 402, description = "Mermaid export not available on plan", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn export_mermaid(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Topology, Read>>,
    _feature: RequireFeature<MermaidExportFeature>,
    Path(id): Path<Uuid>,
    query: Query<TopologyExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let site_ids = auth.site_ids();

    let service = Topology::get_service(&state);
    let topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    if !site_ids.contains(&topology.base.site_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology",
        ));
    }

    // Build the graph on request and export the requested view's slice.
    let data = service
        .get_topology_render_data(topology.base.site_id, None, &[query.view])
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let nodes = data
        .nodes
        .get(&query.view)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let edges = data
        .edges
        .get(&query.view)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let content = crate::server::topology::types::export::topology_to_mermaid(nodes, edges, &data);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"topology.mmd\""),
    );

    Ok((headers, Body::from(content)))
}

/// Export topology as Confluence wiki markup
#[utoipa::path(
    get,
    operation_id = "export_topology_confluence",
    path = "/{id}/export/confluence",
    tags = [Topology::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "Topology ID"), TopologyExportQuery),
    responses(
        (status = 200, description = "Confluence wiki markup export", content_type = "text/plain", body = String),
        (status = 402, description = "Confluence export not available on plan", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn export_confluence(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Topology, Read>>,
    _feature: RequireFeature<ConfluenceExportFeature>,
    Path(id): Path<Uuid>,
    query: Query<TopologyExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let site_ids = auth.site_ids();

    let service = Topology::get_service(&state);
    let topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    if !site_ids.contains(&topology.base.site_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology",
        ));
    }

    // Build the graph on request and export the requested view's slice.
    let data = service
        .get_topology_render_data(topology.base.site_id, None, &[query.view])
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let nodes = data
        .nodes
        .get(&query.view)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let edges = data
        .edges
        .get(&query.view)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let content =
        crate::server::topology::types::export::topology_to_confluence(nodes, edges, &data);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"topology.txt\""),
    );

    Ok((headers, Body::from(content)))
}

#[cfg(test)]
mod setup_tests {
    use super::*;

    #[test]
    fn only_a_view_other_than_the_default_completes_explore_views() {
        let explores =
            |view| setup_items_viewed(view).contains(&SetupItemDiscriminants::ExploreViews);
        assert!(!explores(Some(TopologyView::default())));
        assert!(!explores(None));
        assert!(explores(Some(TopologyView::Workloads)));
        assert!(setup_items_viewed(None).contains(&SetupItemDiscriminants::OpenTopology));
    }
}
