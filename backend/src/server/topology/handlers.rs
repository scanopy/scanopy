use crate::server::shared::extractors::Query;
use crate::server::shared::storage::traits::Entity;
use crate::server::{
    auth::middleware::permissions::{Authorized, IsUser, Member, Viewer},
    config::AppState,
    shared::{
        events::types::{OnboardingEvent, OnboardingOperation},
        handlers::{
            query::{FilterQueryExtractor, NetworkFilterQuery},
            traits::{CrudHandlers, update_handler},
        },
        services::traits::CrudService,
        storage::{filter::StorableFilter, traits::Storable},
        types::api::{
            ApiError, ApiErrorResponse, ApiJson, ApiResponse, ApiResult, EmptyApiResponse,
            PaginatedApiResponse,
        },
    },
    topology::{
        service::main::BuildGraphParams,
        types::base::{
            SetEntitiesParams, Topology, TopologyEdgeHandleUpdate, TopologyMetadataUpdate,
            TopologyNodePositionUpdate, TopologyNodeResizeUpdate, TopologyRebuildRequest,
        },
    },
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, header},
    response::{
        IntoResponse, Json, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use chrono::Utc;
use futures::{Stream, stream};
use std::{convert::Infallible, sync::Arc};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for generic CRUD operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Topology);
    crate::crud_delete_handler!(Topology);
    crate::crud_export_csv_handler!(Topology);
}

/// Topology endpoints are internal-only (hidden from public docs)
pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_topologies, create_topology))
        .routes(routes!(
            generated::get_by_id,
            update_topology,
            generated::delete
        ))
        .routes(routes!(generated::export_csv))
        .routes(routes!(export_mermaid))
        .routes(routes!(export_confluence))
        .routes(routes!(refresh))
        .routes(routes!(rebuild))
        .routes(routes!(update_node_position))
        .routes(routes!(update_node_resize))
        .routes(routes!(update_edge_handles))
        .routes(routes!(update_metadata))
        .routes(routes!(lock))
        .routes(routes!(unlock))
        // SSE endpoint (not well-supported by OpenAPI)
        .route("/stream", get(staleness_stream))
}

#[utoipa::path(
    put,
    path = "/{id}",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    responses(
        (status = 200, description = "Topology updated", body = ApiResponse<Topology>),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_topology(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    id: Path<Uuid>,
    topology: Json<Topology>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    update_handler::<Topology>(state, auth, id, topology).await
}

/// Get all topologies
#[utoipa::path(
    get,
    path = "",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    params(NetworkFilterQuery),
    responses(
        (status = 200, description = "List of topologies", body = PaginatedApiResponse<Topology>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_topologies(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    query: Query<NetworkFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Topology>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    // Apply network filter and pagination
    let base_filter = StorableFilter::<Topology>::new_from_network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    let service = Topology::get_service(&state);
    let result = service.get_paginated(filter).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch topologies");
        ApiError::internal_error(&e.to_string())
    })?;

    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        result.items,
        result.total_count,
        limit,
        offset,
    )))
}

/// Create topology
#[utoipa::path(
    post,
    path = "",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    request_body = Topology,
    responses(
        (status = 200, description = "Topology created", body = ApiResponse<Topology>),
        (status = 400, description = "Validation failed", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_topology(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    ApiJson(mut topology): ApiJson<Topology>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    let user_id = auth.user_id();
    let network_ids = auth.network_ids();

    // Validate user has access to this network
    if !network_ids.contains(&topology.base.network_id) {
        return Err(ApiError::forbidden("You don't have access to this network"));
    }

    if let Err(err) = topology.validate() {
        tracing::warn!(
            entity_type = Topology::table_name(),
            user_id = ?user_id,
            error = %err,
            "Entity validation failed"
        );
        return Err(ApiError::bad_request(&format!(
            "{} validation failed: {}",
            Topology::entity_name(),
            err
        )));
    }

    tracing::debug!(
        entity_type = Topology::table_name(),
        user_id = ?user_id,
        "Create request received"
    );

    let service = Topology::get_service(&state);

    let (hosts, interfaces, subnets, groups, ports, bindings, if_entries) =
        service.get_entity_data(topology.base.network_id).await?;

    let services = service
        .get_service_data(topology.base.network_id, &topology.base.options)
        .await?;

    let entity_tags = service.get_entity_tags(&hosts, &services, &subnets).await?;

    let (nodes, edges) = service.build_graph(BuildGraphParams {
        options: &topology.base.options,
        hosts: &hosts,
        interfaces: &interfaces,
        subnets: &subnets,
        services: &services,
        groups: &groups,
        ports: &ports,
        bindings: &bindings,
        if_entries: &if_entries,
        old_edges: &[],
        old_nodes: &[],
    });

    topology.set_entities(SetEntitiesParams {
        hosts,
        interfaces,
        services,
        subnets,
        groups,
        ports,
        bindings,
        if_entries,
        entity_tags,
    });

    topology.set_graph(nodes, edges);

    topology.clear_stale();

    let entity = auth.into_entity();
    let created = service
        .create(topology, entity.clone())
        .await
        .map_err(|e| {
            tracing::error!(
                entity_type = Topology::table_name(),
                user_id = ?user_id,
                error = %e,
                "Failed to create entity"
            );
            ApiError::internal_error(&e.to_string())
        })?;

    tracing::info!(
        entity_type = Topology::table_name(),
        entity_id = %created.id(),
        user_id = ?user_id,
        "Entity created via API"
    );

    Ok(Json(ApiResponse::success(created)))
}

/// Refresh topology data
#[utoipa::path(
    post,
    path = "/{id}/refresh",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyRebuildRequest,
    responses(
        (status = 200, description = "Topology refreshed", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn refresh(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyRebuildRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    // Validate user has access to this topology's network
    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    // Fetch the existing topology
    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    // Update options from request
    topology.base.options = request.options;

    let (hosts, interfaces, subnets, groups, ports, bindings, if_entries) =
        service.get_entity_data(request.network_id).await?;

    let services = service
        .get_service_data(request.network_id, &topology.base.options)
        .await?;

    let entity_tags = service.get_entity_tags(&hosts, &services, &subnets).await?;

    topology.set_entities(SetEntitiesParams {
        hosts,
        services,
        interfaces,
        subnets,
        groups,
        ports,
        bindings,
        if_entries,
        entity_tags,
    });

    service.update(&mut topology, auth.into_entity()).await?;

    // Return will be handled through event subscriber which triggers SSE

    Ok(Json(ApiResponse::success(())))
}

/// Rebuild topology layout
#[utoipa::path(
    post,
    path = "/{id}/rebuild",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyRebuildRequest,
    responses(
        (status = 200, description = "Topology rebuilt", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn rebuild(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyRebuildRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    // Validate user has access to this topology's network
    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    // Fetch the existing topology
    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    // Update options from request
    topology.base.options = request.options.clone();

    let (hosts, interfaces, subnets, groups, ports, bindings, if_entries) =
        service.get_entity_data(request.network_id).await?;

    let services = service
        .get_service_data(request.network_id, &topology.base.options)
        .await?;

    let entity_tags = service.get_entity_tags(&hosts, &services, &subnets).await?;

    let (nodes, edges) = service.build_graph(BuildGraphParams {
        options: &topology.base.options,
        hosts: &hosts,
        interfaces: &interfaces,
        subnets: &subnets,
        services: &services,
        groups: &groups,
        ports: &ports,
        bindings: &bindings,
        if_entries: &if_entries,
        old_nodes: &request.nodes,
        old_edges: &request.edges,
    });

    topology.set_entities(SetEntitiesParams {
        hosts,
        services,
        interfaces,
        subnets,
        groups,
        ports,
        bindings,
        if_entries,
        entity_tags,
    });

    topology.set_graph(nodes, edges);

    topology.clear_stale();

    let organization_id = auth.organization_id();
    let entity = auth.into_entity();

    // Publish onboarding milestone BEFORE topology update so it's
    // in the DB when the SSE-triggered org refetch arrives
    if let Some(org_id) = organization_id {
        let organization = state
            .services
            .organization_service
            .get_by_id(&org_id)
            .await?;

        if let Some(organization) = organization
            && organization.not_onboarded(&OnboardingOperation::FirstTopologyRebuild)
            && !organization.not_onboarded(&OnboardingOperation::FirstDiscoveryCompleted)
        {
            state
                .services
                .event_bus
                .publish_onboarding(OnboardingEvent {
                    id: Uuid::new_v4(),
                    organization_id: entity.organization_id().expect("User should have org_id"),
                    operation: OnboardingOperation::FirstTopologyRebuild,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({}),
                    authentication: entity.clone(),
                })
                .await?;
        }
    }

    service.update(&mut topology, entity).await?;

    // Return will be handled through event subscriber which triggers SSE

    Ok(Json(ApiResponse::success(())))
}

/// Update a single node's position
///
/// Lightweight endpoint for drag operations. Instead of sending the entire topology
/// (which can be several megabytes), only sends the node ID and new position.
/// Fixes HTTP 413 errors on drag operations for large topologies.
#[utoipa::path(
    post,
    path = "/{id}/node-position",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
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
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyNodePositionUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    // Validate user has access to this topology's network
    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    // Fetch the existing topology
    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    // Find and update the node's position
    let node = topology
        .base
        .nodes
        .iter_mut()
        .find(|n| n.id == request.node_id)
        .ok_or_else(|| {
            ApiError::not_found(format!("Node {} not found in topology", request.node_id))
        })?;

    node.position = request.position;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Update an edge's handles
///
/// Lightweight endpoint for edge reconnect operations. Instead of sending the entire
/// topology, only sends the edge ID and new handle positions.
/// Fixes HTTP 413 errors on edge reconnect operations for large topologies.
#[utoipa::path(
    post,
    path = "/{id}/edge-handles",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
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
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyEdgeHandleUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    // Validate user has access to this topology's network
    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    // Fetch the existing topology
    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    // Find and update the edge's handles
    let edge = topology
        .base
        .edges
        .iter_mut()
        .find(|e| e.id == request.edge_id)
        .ok_or_else(|| {
            ApiError::not_found(format!("Edge {} not found in topology", request.edge_id))
        })?;

    edge.source_handle = request.source_handle;
    edge.target_handle = request.target_handle;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Update a node's size and position
///
/// Lightweight endpoint for subnet resize operations. Instead of sending the entire
/// topology, only sends the node ID, new size, and new position.
/// Fixes HTTP 413 errors on resize operations for large topologies.
#[utoipa::path(
    post,
    path = "/{id}/node-resize",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
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
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyNodeResizeUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    // Validate user has access to this topology's network
    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    // Fetch the existing topology
    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    // Find and update the node's size and position
    let node = topology
        .base
        .nodes
        .iter_mut()
        .find(|n| n.id == request.node_id)
        .ok_or_else(|| {
            ApiError::not_found(format!("Node {} not found in topology", request.node_id))
        })?;

    node.size = request.size;
    node.position = request.position;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Update topology metadata
///
/// Lightweight endpoint for editing topology name and parent. Instead of sending
/// the entire topology (which includes all hosts, interfaces, services, etc.),
/// only sends the metadata fields.
/// Fixes HTTP 413 errors on metadata edit operations for large topologies.
#[utoipa::path(
    post,
    path = "/{id}/metadata",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyMetadataUpdate,
    responses(
        (status = 200, description = "Metadata updated", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_metadata(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyMetadataUpdate>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    // Validate user has access to this topology's network
    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    // Fetch the existing topology
    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    // Update metadata fields
    topology.base.name = request.name;
    topology.base.parent_id = request.parent_id;

    service.update(&mut topology, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Lock a topology
#[utoipa::path(
    post,
    path = "/{id}/lock",
    tags = [Topology::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    responses(
        (status = 200, description = "Topology locked", body = ApiResponse<Topology>),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn lock(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    let service = Topology::get_service(&state);
    let network_ids = auth.network_ids();
    let user_id = auth
        .user_id()
        .ok_or_else(|| ApiError::forbidden("User context required"))?;

    if let Some(mut topology) = service.get_by_id(&id).await? {
        // Validate user has access to this topology's network
        if !network_ids.contains(&topology.base.network_id) {
            return Err(ApiError::forbidden(
                "You don't have access to this topology",
            ));
        }

        topology.lock(user_id);

        let updated = service.update(&mut topology, auth.into_entity()).await?;

        Ok(Json(ApiResponse::success(updated)))
    } else {
        Err(ApiError::not_found(format!(
            "Could not find topology {}",
            id
        )))
    }
}

/// Unlock a topology
#[utoipa::path(
    post,
    path = "/{id}/unlock",
    tags = [Topology::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "Topology ID")),
    responses(
        (status = 200, description = "Topology unlocked", body = ApiResponse<Topology>),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn unlock(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    let service = Topology::get_service(&state);
    let network_ids = auth.network_ids();

    if let Some(mut topology) = service.get_by_id(&id).await? {
        // Validate user has access to this topology's network
        if !network_ids.contains(&topology.base.network_id) {
            return Err(ApiError::forbidden(
                "You don't have access to this topology",
            ));
        }

        topology.unlock();

        let updated = service.update(&mut topology, auth.into_entity()).await?;

        Ok(Json(ApiResponse::success(updated)))
    } else {
        Err(ApiError::not_found(format!(
            "Could not find topology {}",
            id
        )))
    }
}

/// Export topology as Mermaid flowchart
#[utoipa::path(
    get,
    path = "/{id}/export/mermaid",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    responses(
        (status = 200, description = "Mermaid flowchart export", content_type = "text/plain"),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn export_mermaid(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let network_ids = auth.network_ids();

    let service = Topology::get_service(&state);
    let topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    if !network_ids.contains(&topology.base.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology",
        ));
    }

    let content = crate::server::topology::types::export::topology_to_mermaid(&topology);

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
    path = "/{id}/export/confluence",
    tags = [Topology::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    responses(
        (status = 200, description = "Confluence wiki markup export", content_type = "text/plain"),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn export_confluence(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let network_ids = auth.network_ids();

    let service = Topology::get_service(&state);
    let topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    if !network_ids.contains(&topology.base.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology",
        ));
    }

    let content = crate::server::topology::types::export::topology_to_confluence(&topology);

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

async fn staleness_stream(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state
        .services
        .topology_service
        .subscribe_staleness_changes();

    let allowed_networks = auth.network_ids();

    let stream = stream::unfold(rx, move |mut rx| {
        let allowed = allowed_networks.clone();
        async move {
            loop {
                match rx.recv().await {
                    Ok(update) => {
                        // Only emit if user has access to this topology's network
                        if allowed.contains(&update.base.network_id) {
                            let json = serde_json::to_string(&update).ok()?;
                            return Some((Ok(Event::default().data(json)), rx));
                        }
                        // Otherwise skip and wait for next message
                    }
                    Err(_) => return None,
                }
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
