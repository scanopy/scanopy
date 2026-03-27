use crate::daemon::runtime::state::BufferedEntities;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, IsDaemon, Member, Or, Viewer};
use crate::server::if_entries::r#impl::base::IfEntry;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::ports::r#impl::base::Port;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::events::types::{BillingEvent, BillingOperation};
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, bulk_delete_handler, delete_handler,
};
use crate::server::shared::services::{csv::build_csv, traits::CrudService};
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::storage::{filter::StorableFilter, traits::Storable};
use crate::server::shared::types::api::{ApiErrorResponse, EmptyApiResponse};
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::shared::validation::{validate_network_access, validate_read_access};
use crate::server::{
    config::AppState,
    daemons::r#impl::base::Daemon,
    hosts::r#impl::{
        api::{CreateHostRequest, DiscoveryHostRequest, HostResponse, UpdateHostRequest},
        base::Host,
        legacy::{HostCreateRequestBody, HostCreateResponse, LegacyHostWithServicesResponse},
    },
    shared::types::api::{ApiError, ApiResponse, ApiResult, PaginatedApiResponse},
};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

// ============================================================================
// Host Ordering
// ============================================================================

/// Fields that hosts can be ordered/grouped by.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostOrderField {
    #[default]
    CreatedAt,
    Name,
    Hostname,
    UpdatedAt,
    /// Sort by virtualizing service name. Requires JOIN to services table.
    VirtualizedBy,
    NetworkId,
    /// Sort by primary interface IP address. Requires JOIN to interfaces table.
    InterfaceIp,
}

impl OrderField for HostOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "hosts.created_at",
            Self::Name => "hosts.name",
            Self::Hostname => "hosts.hostname",
            Self::UpdatedAt => "hosts.updated_at",
            Self::NetworkId => "hosts.network_id",
            Self::VirtualizedBy => "COALESCE(virt_service.name, '')",
            Self::InterfaceIp => "primary_interface.ip_address",
        }
    }

    fn join_sql(&self) -> Option<&'static str> {
        match self {
            Self::VirtualizedBy => Some(
                "LEFT JOIN services AS virt_service ON \
                 (hosts.virtualization->'details'->>'service_id')::uuid = virt_service.id",
            ),
            Self::InterfaceIp => Some(
                "LEFT JOIN (\
                    SELECT DISTINCT ON (host_id) host_id, ip_address \
                    FROM interfaces \
                    ORDER BY host_id, position ASC\
                ) AS primary_interface ON hosts.id = primary_interface.host_id",
            ),
            _ => None,
        }
    }
}

// ============================================================================
// Host Filter Query
// ============================================================================

/// Query parameters for filtering and ordering hosts.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct HostFilterQuery {
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Filter by specific entity IDs (for selective loading)
    pub ids: Option<Vec<Uuid>>,
    /// Filter by tag IDs (returns hosts that have ANY of the specified tags)
    pub tag_ids: Option<Vec<Uuid>>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<HostOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<HostOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl HostFilterQuery {
    /// Build the ORDER BY clause and apply any required JOINs to the filter.
    /// Returns: (modified_filter, order_by_sql)
    pub fn apply_ordering(&self, filter: StorableFilter<Host>) -> (StorableFilter<Host>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "hosts.created_at ASC",
        )
    }
}

impl FilterQueryExtractor for HostFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        // Apply IDs filter first if provided
        let filter = match &self.ids {
            Some(ids) if !ids.is_empty() => filter.entity_ids(ids),
            _ => filter,
        };
        // Then apply network filter
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

// Generated handlers for CSV export
mod generated {
    use super::*;
    crate::crud_export_csv_handler!(Host);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_hosts, create_host))
        .routes(routes!(get_host_by_id, update_host, delete_host))
        .routes(routes!(bulk_delete_hosts))
        .routes(routes!(generated::export_csv))
        .routes(routes!(export_hosts_zip))
        .routes(routes!(consolidate_hosts))
        .routes(routes!(create_host_discovery))
}

/// List all hosts
///
/// Returns all hosts the authenticated user has access to, with their
/// interfaces, ports, and services included. Supports pagination via
/// `limit` and `offset` query parameters, and ordering via `group_by`,
/// `order_by`, and `order_direction`.
#[utoipa::path(
    get,
    path = "",
    tag = Host::ENTITY_NAME_PLURAL,
    params(HostFilterQuery),
    responses(
        (status = 200, description = "List of hosts with their children", body = PaginatedApiResponse<HostResponse>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<HostFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<HostResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let base_filter = StorableFilter::<Host>::new_from_network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);

    // Apply tag filter if specified
    let filter = match &query.tag_ids {
        Some(tag_ids) if !tag_ids.is_empty() => {
            filter.has_any_tags(tag_ids, EntityDiscriminants::Host)
        }
        _ => filter,
    };

    // Apply pagination
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    // Apply ordering and JOINs
    let (filter, order_by) = query.apply_ordering(filter);

    let mut result = state
        .services
        .host_service
        .get_all_host_responses_paginated(filter, &order_by)
        .await?;

    // Hydrate credential assignments from junction table
    let host_ids: Vec<Uuid> = result.items.iter().map(|h| h.id).collect();
    let cred_map = state
        .services
        .credential_service
        .get_credential_assignments_for_hosts(&host_ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    for host in &mut result.items {
        if let Some(assignments) = cred_map.get(&host.id) {
            host.credential_assignments = assignments.clone();
        }
    }

    // Get effective pagination values for response metadata
    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        result.items,
        result.total_count,
        limit,
        offset,
    )))
}

/// Get a host by ID
///
/// Returns a single host with its interfaces, ports, and services.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = Host::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Host ID")),
    responses(
        (status = 200, description = "Host found", body = ApiResponse<HostResponse>),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_host_by_id(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let mut host = state
        .services
        .host_service
        .get_host_response(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(id))?;

    validate_read_access(Some(host.network_id), None, &network_ids, organization_id)?;

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[host.id], EntityDiscriminants::Host)
        .await?;
    if let Some(tags) = tags_map.get(&host.id) {
        host.tags = tags.clone();
    }

    // Hydrate credential assignments from junction table
    host.credential_assignments = state
        .services
        .credential_service
        .get_credential_assignments_for_host(&host.id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(host)))
}

/// Create a new host
///
/// Creates a host with optional interfaces, ports, and services.
/// The `source` field is automatically set to `Manual`.
///
/// ### Tag Validation
///
/// - Tags must exist and belong to your organization
/// - Duplicate tag UUIDs are automatically deduplicated
/// - Invalid or cross-organization tag UUIDs return a 400 error
///
#[utoipa::path(
    post,
    path = "",
    tag = Host::ENTITY_NAME_PLURAL,
    request_body = CreateHostRequest,
    responses(
        (status = 200, description = "Host created successfully", body = ApiResponse<HostResponse>),
        (status = 400, description = "Validation error: network not found, subnet mismatch, or invalid tags", body = ApiErrorResponse),
        (status = 401, description = "No access to the specified network", body = ApiErrorResponse),
    ),
    security( ("user_api_key" = []),("session" = []), ("daemon_api_key" = []))
)]
async fn create_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Or<Member, IsDaemon>>,
    Json(request): Json<HostCreateRequestBody>,
) -> ApiResult<Json<ApiResponse<HostCreateResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth.organization_id();
    let entity = auth.into_entity();
    let host_service = &state.services.host_service;

    match (request, &entity) {
        // New format - standard flow
        (HostCreateRequestBody::New(request), _) => {
            // Validate request (name length, etc.)
            request
                .validate()
                .map_err(|e| ApiError::bad_request(&e.to_string()))?;

            // Validate user has access to the network
            validate_network_access(Some(request.network_id), &network_ids, "create")?;

            // Validate network_id exists
            let _network = state
                .services
                .network_service
                .get_by_id(&request.network_id)
                .await?
                .ok_or_else(|| {
                    ApiError::bad_request(&format!("Network {} not found", request.network_id))
                })?;

            // Check host limit on plan
            if let Some(org_id) = organization_id
                && let Some(org) = state
                    .services
                    .organization_service
                    .get_by_id(&org_id)
                    .await?
                && let Some(plan) = &org.base.plan
                && let Some(limit) = plan.host_limit()
            {
                let org_filter = StorableFilter::<Host>::new_from_network_ids(&network_ids);
                let current_hosts =
                    state.services.host_service.get_all(org_filter).await?.len() as u64;
                if current_hosts >= limit {
                    let _ = state
                        .services
                        .event_bus
                        .publish_billing(BillingEvent::new(
                            Uuid::new_v4(),
                            org_id,
                            BillingOperation::FeatureLimitHit,
                            Utc::now(),
                            entity.clone(),
                            serde_json::json!({
                                "limit_type": "hosts",
                                "current_count": current_hosts,
                                "limit": limit,
                                "plan_type": plan.name(),
                            }),
                        ))
                        .await;

                    return Err(ApiError::coded(
                        StatusCode::FORBIDDEN,
                        ErrorCode::BillingHostLimitReached { limit },
                    ));
                }
            }

            // Check interface subnets are on the same network
            for interface in &request.interfaces {
                if let Some(subnet) = state
                    .services
                    .subnet_service
                    .get_by_id(&interface.subnet_id)
                    .await?
                    && subnet.base.network_id != request.network_id
                {
                    return Err(ApiError::bad_request(&format!(
                        "Host is on network {}, cannot have an interface with a subnet \"{}\" which is on network {}.",
                        request.network_id, subnet.base.name, subnet.base.network_id
                    )));
                }
            }

            let host_response = host_service.create_from_request(request, entity).await?;

            // Sync credential assignments to junction table
            state
                .services
                .credential_service
                .set_host_credentials(&host_response.id, &host_response.credential_assignments)
                .await
                .map_err(|e| ApiError::internal_error(&e.to_string()))?;

            Ok(Json(ApiResponse::success(HostCreateResponse::New(
                host_response,
            ))))
        }

        // Legacy format from daemon - transform and process
        (
            HostCreateRequestBody::Legacy(legacy_request),
            AuthenticatedEntity::Daemon { daemon_id, .. },
        ) => {
            tracing::warn!(
                daemon_id = %daemon_id,
                "Legacy daemon request to POST /api/hosts - daemon should be updated"
            );

            let discovery_request = legacy_request.into_discovery_request();

            // Validate daemon has access to the network
            validate_network_access(
                Some(discovery_request.host.base.network_id),
                &network_ids,
                "create",
            )?;

            let DiscoveryHostRequest {
                host,
                interfaces,
                ports,
                services,
                if_entries,
            } = discovery_request;

            let host_response = host_service
                .discover_host(host, interfaces, ports, services, if_entries, entity, None)
                .await?;

            let legacy_response = LegacyHostWithServicesResponse::from_host_response(host_response);

            Ok(Json(ApiResponse::success(HostCreateResponse::Legacy(
                legacy_response,
            ))))
        }

        // Legacy format from non-daemon (user) - reject
        (HostCreateRequestBody::Legacy(_), _) => Err(ApiError::bad_request(
            "Invalid request format. Please use the CreateHostRequest format.",
        )),
    }
}

/// Update a host
///
/// Updates host properties. Children (interfaces, ports, services)
/// are managed via their own endpoints.
///
/// ### Tag Validation
///
/// - Tags must exist and belong to your organization
/// - Duplicate tag UUIDs are automatically deduplicated
/// - Invalid or cross-organization tag UUIDs return a 400 error
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Host::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Host ID")),
    request_body = UpdateHostRequest,
    responses(
        (status = 200, description = "Host updated", body = ApiResponse<HostResponse>),
        (status = 400, description = "Validation error: invalid tags", body = ApiErrorResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(mut request): Json<UpdateHostRequest>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Validate request (name length, etc.)
    request
        .validate()
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    let host_service = &state.services.host_service;

    // Path ID is canonical - override any ID in the body
    request.id = id;

    // Fetch existing host to validate network access
    let existing_host = host_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(id))?;

    validate_read_access(
        Some(existing_host.base.network_id),
        None,
        &network_ids,
        organization_id,
    )?;

    let credential_assignments = request.credential_assignments.clone();

    let mut host_response = host_service
        .update_from_request(request, auth.into_entity())
        .await?;

    // Sync credential assignments if provided
    if let Some(ref assignments) = credential_assignments {
        state
            .services
            .credential_service
            .set_host_credentials(&host_response.id, assignments)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
        host_response.credential_assignments = assignments.clone();
    } else {
        // Hydrate from junction table
        host_response.credential_assignments = state
            .services
            .credential_service
            .get_credential_assignments_for_host(&host_response.id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[host_response.id], EntityDiscriminants::Host)
        .await?;
    if let Some(tags) = tags_map.get(&host_response.id) {
        host_response.tags = tags.clone();
    }

    Ok(Json(ApiResponse::success(host_response)))
}

/// Internal endpoint for daemon discovery
///
/// Used by daemons to report discovered hosts. Accepts full entities with
/// pre-generated IDs. Uses upsert behavior to merge with existing hosts.
///
/// Tagged as "internal" - included in OpenAPI spec for client generation
/// but hidden from public documentation.
#[utoipa::path(
    post,
    path = "/discovery",
    tags = ["hosts", "internal"],
    request_body = DiscoveryHostRequest,
    responses(
        (status = 200, description = "Host discovered/updated successfully", body = ApiResponse<HostResponse>),
        (status = 403, description = "Daemon cannot create hosts on other networks", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn create_host_discovery(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Json(request): Json<DiscoveryHostRequest>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    // Get daemon network_id from entity
    let daemon_network_id = auth
        .network_ids()
        .first()
        .copied()
        .ok_or_else(|| ApiError::forbidden("Daemon has no network assignment"))?;

    if request.host.base.network_id != daemon_network_id {
        return Err(ApiError::forbidden(
            "Daemon cannot create hosts on networks it's not assigned to",
        ));
    }

    // Delegate to processor for shared discovery logic
    // This ensures both DaemonPoll and ServerPoll modes use the same logic
    let entities = BufferedEntities {
        hosts: vec![request],
        subnets: vec![],
    };

    let created = state
        .services
        .daemon_service
        .process_discovery_entities(entities, auth.into_entity())
        .await?;

    let (_, host_response) = created.hosts.into_iter().next().ok_or_else(|| {
        if let Some((limit, _org_id)) = created.billing_limit_hit {
            ApiError::coded(
                StatusCode::FORBIDDEN,
                ErrorCode::BillingHostLimitReached { limit },
            )
        } else {
            ApiError::internal_error("No host returned from processor")
        }
    })?;

    Ok(Json(ApiResponse::success(host_response)))
}

/// Consolidate hosts
///
/// Merges all interfaces, ports, and services from `other_host` into
/// `destination_host`, then deletes `other_host`. Both hosts must be
/// on the same network.
///
/// ### Merge Behavior
///
/// - **Interfaces**: Transferred to destination. If an interface with matching subnet+IP or MAC
///   already exists on destination, bindings are remapped to use the existing interface.
/// - **Ports**: Transferred to destination. If a port with the same number and protocol already
///   exists, bindings are remapped to use the existing port.
/// - **Services**: Transferred to destination with deduplication.
///   See [upsert behavior](https://scanopy.net/docs/discovery/#upsert-behavior) for details.
///
/// ### Restrictions
///
/// - Cannot consolidate a host with itself.
/// - Cannot consolidate a host that has a daemon - consolidate into it instead.
#[utoipa::path(
    put,
    path = "/{destination_host}/consolidate/{other_host}",
    tag = Host::ENTITY_NAME_PLURAL,
    params(
        ("destination_host" = Uuid, Path, description = "Destination host ID - will receive all children"),
        ("other_host" = Uuid, Path, description = "Host to merge into destination - will be deleted")
    ),
    responses(
        (status = 200, description = "Hosts consolidated successfully", body = ApiResponse<HostResponse>),
        (status = 404, description = "One or both hosts not found", body = ApiErrorResponse),
        (status = 400, description = "Validation error: same host, has daemon, or different networks", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn consolidate_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path((destination_host_id, other_host_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let host_service = &state.services.host_service;

    let destination_host = host_service
        .get_by_id(&destination_host_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(destination_host_id))?;
    let other_host = host_service
        .get_by_id(&other_host_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(other_host_id))?;

    // Validate user has access to both hosts
    validate_read_access(
        Some(destination_host.base.network_id),
        None,
        &network_ids,
        organization_id,
    )?;
    validate_read_access(
        Some(other_host.base.network_id),
        None,
        &network_ids,
        organization_id,
    )?;

    // Make sure hosts are on same network
    if destination_host.base.network_id != other_host.base.network_id {
        return Err(ApiError::bad_request(&format!(
            "Destination Host is on network {}, other host \"{}\" can't be on a different network ({}).",
            destination_host.base.network_id, other_host.base.name, other_host.base.network_id
        )));
    }

    let mut host_response = host_service
        .consolidate_hosts(destination_host, other_host, auth.into_entity())
        .await?;

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[host_response.id], EntityDiscriminants::Host)
        .await?;
    if let Some(tags) = tags_map.get(&host_response.id) {
        host_response.tags = tags.clone();
    }

    // Hydrate credential assignments from junction table
    host_response.credential_assignments = state
        .services
        .credential_service
        .get_credential_assignments_for_host(&host_response.id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(host_response)))
}

/// Delete a host
///
/// Prevents deletion if the host has a daemon associated with it
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Host::ENTITY_NAME_PLURAL,
    params(
        ("id" = Uuid, Path, description = "Host ID")
    ),
    responses(
        (status = 200, description = "Host deleted", body = EmptyApiResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
        (status = 409, description = "Host has associated daemon", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn delete_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Pre-validation: Can't delete a host with an associated daemon
    let daemon_filter = StorableFilter::<Daemon>::new_from_host_ids(&[id]);
    if state
        .services
        .daemon_service
        .get_one(daemon_filter)
        .await?
        .is_some()
    {
        return Err(ApiError::coded(
            StatusCode::CONFLICT,
            ErrorCode::EntityDeleteForbidden {
                entity: "host".to_string(),
                reason: Some("has an associated daemon — delete the daemon first".to_string()),
            },
        ));
    }

    // Delegate to generic handler (handles auth checks, deletion)
    delete_handler::<Host>(State(state), auth, Path(id)).await
}

/// Bulk delete hosts
///
/// Deletes multiple hosts in a single request. The request body should be
/// an array of host IDs to delete. Fails if any host has an associated daemon.
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = Host::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "Array of host IDs to delete"),
    responses(
        (status = 200, description = "Hosts deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 409, description = "One or more hosts has an associated daemon - delete daemons first", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_delete_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(ids): Json<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    let daemon_service = &state.services.daemon_service;

    let daemon_filter = StorableFilter::<Daemon>::new_from_host_ids(&ids);

    if !daemon_service.get_all(daemon_filter).await?.is_empty() {
        return Err(ApiError::coded(
            StatusCode::CONFLICT,
            ErrorCode::EntityDeleteForbidden {
                entity: "host".to_string(),
                reason: Some(
                    "one or more hosts has an associated daemon — delete the daemon(s) first"
                        .to_string(),
                ),
            },
        ));
    }

    bulk_delete_handler::<Host>(axum::extract::State(state), auth, axum::extract::Json(ids)).await
}

/// Export hosts with children to ZIP
///
/// Exports all hosts matching the filter criteria along with their children
/// (interfaces, ports, services, if_entries) as a ZIP archive containing
/// separate CSV files for each entity type.
#[utoipa::path(
    get,
    path = "/export/zip",
    tag = Host::ENTITY_NAME_PLURAL,
    operation_id = "export_hosts_zip",
    params(HostFilterQuery),
    responses(
        (status = 200, description = "ZIP file containing CSVs", content_type = "application/zip"),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn export_hosts_zip(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<HostFilterQuery>,
) -> ApiResult<impl IntoResponse> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Build host filter (same as CSV export)
    let base_filter = StorableFilter::<Host>::new_from_network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);

    // Apply tag filter if specified
    let filter = match &query.tag_ids {
        Some(tag_ids) if !tag_ids.is_empty() => {
            filter.has_any_tags(tag_ids, EntityDiscriminants::Host)
        }
        _ => filter,
    };

    // Get all hosts (no pagination for export)
    let hosts = state.services.host_service.get_all(filter).await?;
    let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();

    // Fetch children for these hosts
    let interfaces = state
        .services
        .interface_service
        .get_all(
            StorableFilter::<Interface>::new_from_host_ids(&host_ids).network_ids(&network_ids),
        )
        .await?;

    let ports = state
        .services
        .port_service
        .get_all(StorableFilter::<Port>::new_from_host_ids(&host_ids).network_ids(&network_ids))
        .await?;

    let services = state
        .services
        .service_service
        .get_all(StorableFilter::<Service>::new_from_host_ids(&host_ids).network_ids(&network_ids))
        .await?;

    let if_entries = state
        .services
        .if_entry_service
        .get_all(StorableFilter::<IfEntry>::new_from_host_ids(&host_ids).network_ids(&network_ids))
        .await?;

    // Build CSVs
    let hosts_csv = build_csv(&hosts)
        .map_err(|e| ApiError::internal_error(&format!("Failed to build hosts CSV: {}", e)))?;
    let interfaces_csv = build_csv(&interfaces)
        .map_err(|e| ApiError::internal_error(&format!("Failed to build interfaces CSV: {}", e)))?;
    let ports_csv = build_csv(&ports)
        .map_err(|e| ApiError::internal_error(&format!("Failed to build ports CSV: {}", e)))?;
    let services_csv = build_csv(&services)
        .map_err(|e| ApiError::internal_error(&format!("Failed to build services CSV: {}", e)))?;
    let if_entries_csv = build_csv(&if_entries)
        .map_err(|e| ApiError::internal_error(&format!("Failed to build if_entries CSV: {}", e)))?;

    // Build zip archive
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("hosts.csv", options)
            .map_err(|e| ApiError::internal_error(&format!("Failed to create zip: {}", e)))?;
        zip.write_all(&hosts_csv)
            .map_err(|e| ApiError::internal_error(&format!("Failed to write zip: {}", e)))?;

        zip.start_file("interfaces.csv", options)
            .map_err(|e| ApiError::internal_error(&format!("Failed to create zip: {}", e)))?;
        zip.write_all(&interfaces_csv)
            .map_err(|e| ApiError::internal_error(&format!("Failed to write zip: {}", e)))?;

        zip.start_file("ports.csv", options)
            .map_err(|e| ApiError::internal_error(&format!("Failed to create zip: {}", e)))?;
        zip.write_all(&ports_csv)
            .map_err(|e| ApiError::internal_error(&format!("Failed to write zip: {}", e)))?;

        zip.start_file("services.csv", options)
            .map_err(|e| ApiError::internal_error(&format!("Failed to create zip: {}", e)))?;
        zip.write_all(&services_csv)
            .map_err(|e| ApiError::internal_error(&format!("Failed to write zip: {}", e)))?;

        zip.start_file("if_entries.csv", options)
            .map_err(|e| ApiError::internal_error(&format!("Failed to create zip: {}", e)))?;
        zip.write_all(&if_entries_csv)
            .map_err(|e| ApiError::internal_error(&format!("Failed to write zip: {}", e)))?;

        zip.finish()
            .map_err(|e| ApiError::internal_error(&format!("Failed to finalize zip: {}", e)))?;
    }

    let zip_data = buffer.into_inner();

    // Build response with appropriate headers
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"hosts-export.zip\""),
    );

    Ok((headers, Body::from(zip_data)))
}
