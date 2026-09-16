use crate::daemon::runtime::state::BufferedEntities;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, IsDaemon, Member, Or, Viewer};
use crate::server::billing::types::base::{LimitSource, LimitType};
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::daemons::r#impl::version::{minimum_targeted_rescan, supports_targeted_rescan};
use crate::server::discovery::r#impl::base::{Discovery, DiscoveryBase};
use crate::server::discovery::r#impl::scan_settings::{RescanSettings, ScanSettings};
use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::networks::r#impl::Network;
use crate::server::openapi::tags as api_tags;
use crate::server::ports::r#impl::base::{Port, PortType};
use crate::server::services::r#impl::base::Service;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::BillingOperation;
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
use crate::server::shared::types::entities::EntitySourceDiscriminants;
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::shared::validation::{validate_network_access, validate_read_access};
use crate::server::{
    config::AppState,
    daemons::r#impl::{base::Daemon, version::pre_interface_to_ip_address_rename},
    hosts::r#impl::{
        api::{CreateHostRequest, DiscoveryHostRequest, HostResponse, UpdateHostRequest},
        base::{Host, PRIMARY_INTERFACE_JOIN},
        legacy::{HostCreateRequestBody, HostCreateResponse, LegacyHostWithServicesResponse},
    },
    shared::types::api::{ApiError, ApiResponse, ApiResult, PaginatedApiResponse},
};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Json};
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
    /// Sort by the host's *title* — the [`Host::display_name`] ladder, not the stored `name`
    /// column, so the order matches what the Name column draws. Requires the primary-address
    /// JOIN for the ladder's last rung.
    Name,
    Hostname,
    UpdatedAt,
    /// Sort by virtualizing service name. Requires JOIN to services table.
    VirtualizedBy,
    NetworkId,
    /// Sort by primary interface IP address. Requires JOIN to ip_addresses table.
    InterfaceIp,
    /// Sort by when discovery last observed the host. Surfaces stale assets.
    LastSeenAt,
}

/// The host title in SQL, built once: `to_sql` hands out `&'static str`.
static HOST_TITLE_SQL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    crate::server::hosts::r#impl::name_ladder::display_name_sql("hosts", "primary_interface")
});

impl OrderField for HostOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "hosts.created_at",
            Self::Name => HOST_TITLE_SQL.as_str(),
            Self::Hostname => "hosts.hostname",
            Self::UpdatedAt => "hosts.updated_at",
            Self::NetworkId => "hosts.network_id",
            Self::LastSeenAt => "hosts.last_seen_at",
            Self::VirtualizedBy => "COALESCE(virt_service.name, '')",
            Self::InterfaceIp => "primary_interface.ip_address",
        }
    }

    fn join_sql(&self) -> Option<&'static str> {
        match self {
            Self::VirtualizedBy => Some(
                "LEFT JOIN services AS virt_service ON \
                 hosts.virtualization_service_id = virt_service.id",
            ),
            // One const shared by both: grouping by interface ip while ordering by name must
            // produce a single `primary_interface` join. `apply_ordering` drops the second when
            // the two are equal, so what matters is that they cannot stop being equal.
            Self::Name | Self::InterfaceIp => Some(PRIMARY_INTERFACE_JOIN),
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
    /// Filter by network ID. Repeat the parameter to pass several.
    #[serde(alias = "network_id")]
    pub network_ids: Option<Vec<Uuid>>,
    /// Filter by specific entity IDs (for selective loading)
    pub ids: Option<Vec<Uuid>>,
    /// Filter by the `hidden` flag. Repeat the parameter to accept both values;
    /// omit it for no constraint.
    pub hidden: Option<Vec<bool>>,
    /// Filter by the service virtualizing the host. Repeat for several.
    pub virtualization_service_ids: Option<Vec<Uuid>>,
    /// `true` also returns hosts nothing virtualizes. Set on its own it returns
    /// only those — the "Not Virtualized" choice in the UI's filter.
    pub include_unvirtualized: Option<bool>,
    /// Filter to hosts running a service with one of these names.
    pub service_names: Option<Vec<String>>,
    /// Filter by how the host came to exist (`source.type`). Repeat for several;
    /// `Inferred` alone lists the hosts only a neighbour advertised.
    pub sources: Option<Vec<EntitySourceDiscriminants>>,
    /// Filter by tag IDs (returns hosts that have ANY of the specified tags)
    pub tag_ids: Option<Vec<Uuid>>,
    /// Free-text search. Case-insensitive substring match against the host's
    /// name, hostname, sysName, chassis id and description, and against its IP
    /// addresses and the names of services running on it.
    ///
    /// sysName and chassis id are in there because they are rungs of the title
    /// ladder: a host with no name of its own is *shown* under one of them, and
    /// a title you can read but cannot search for is a dead end.
    pub search: Option<String>,
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
    /// As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
    /// instant (snapshot view) instead of live state.
    pub at: Option<chrono::DateTime<chrono::Utc>>,
    /// `true` returns only hosts discovery hasn't observed within their
    /// network's staleness window; `false` returns only those it has. Omit for
    /// both. Evaluated per row against the host's own network's window.
    pub stale: Option<bool>,
    /// `false` returns hosts with empty `ip_addresses`/`ports`/`services`/
    /// `interfaces`. The children dominate the payload, so callers that only need
    /// host identity — name pickers, id→name lookups, counts — should pass
    /// `false`. Defaults to `true`, so existing callers are unaffected.
    pub include_children: Option<bool>,
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

    /// Apply the field filters the Hosts tab offers.
    ///
    /// Shared by the list and the exports, for the same reason the staleness
    /// filter is: an export should mirror exactly what the user was looking at.
    /// Every one of these is applied server-side because the list is paginated —
    /// filtering the loaded page instead would hide matches on every other page
    /// and leave the total count describing a different set of rows.
    pub fn apply_field_filters(&self, filter: StorableFilter<Host>) -> StorableFilter<Host> {
        let filter = match &self.hidden {
            Some(values) if !values.is_empty() => filter.hidden_in(values),
            _ => filter,
        };

        // "Not Virtualized" is a choice about absence, so it can arrive without
        // any service ids beside it.
        let include_unvirtualized = self.include_unvirtualized.unwrap_or(false);
        let virtualization_ids = self.virtualization_service_ids.as_deref().unwrap_or(&[]);
        let filter = if include_unvirtualized || !virtualization_ids.is_empty() {
            filter.virtualization_service_in(virtualization_ids, include_unvirtualized)
        } else {
            filter
        };

        let filter = match &self.service_names {
            Some(names) if !names.is_empty() => filter.has_service_named(names),
            _ => filter,
        };

        match &self.sources {
            Some(sources) if !sources.is_empty() => filter.source_type_in(sources),
            _ => filter,
        }
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
        // Then apply network filter. Intersect with what the caller can see —
        // a requested network they have no access to must narrow the result to
        // nothing, never widen it.
        match &self.network_ids {
            Some(requested) => {
                let accessible: Vec<Uuid> = requested
                    .iter()
                    .copied()
                    .filter(|id| user_network_ids.contains(id))
                    .collect();
                filter.network_ids(&accessible)
            }
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
        .routes(routes!(rescan_host))
}

/// List all hosts
///
/// Returns all hosts the authenticated user has access to, with their
/// ip_addresses, ports, services and interfaces included — pass
/// `include_children=false` to omit those and get a much smaller payload.
/// Supports pagination via `limit` and `offset` query parameters, and ordering
/// via `group_by`, `order_by`, and `order_direction`.
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

    // SCD2 read path: live by default, or as-of the snapshot timestamp when set.
    // Without this, close-and-clone's closed historical copies leak into the
    // host list as empty-shell duplicates.
    let filter = filter.live_or_as_of(query.at);

    // Apply tag filter if specified
    let filter = match &query.tag_ids {
        Some(tag_ids) if !tag_ids.is_empty() => {
            filter.has_any_tags(tag_ids, EntityDiscriminants::Host)
        }
        _ => filter,
    };

    // Server-side because the list is paginated: a client-side search would
    // only ever match the page already loaded.
    let filter = match query.search.as_deref() {
        Some(search) if !search.trim().is_empty() => filter.text_search(search),
        _ => filter,
    };

    let filter = query.apply_field_filters(filter);

    // Staleness is per-network, so resolve each accessible network's cutoff and
    // let the filter compare every row against its own.
    let filter = match query.stale {
        Some(stale) => {
            let cutoffs = state
                .services
                .network_service
                .stale_cutoffs(&network_ids)
                .await?;
            filter.stale_by_network(&cutoffs, stale)
        }
        None => filter,
    };

    // Apply pagination
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    // Apply ordering and JOINs
    let (filter, order_by) = query.apply_ordering(filter);

    // Grouped lists report each group's full size, not the slice of it that
    // landed on this page. Runs against the same filter — including the JOIN
    // `apply_ordering` just added, which the group expression may reference.
    let group_counts = match query.group_by {
        Some(group_field) => Some(
            state
                .services
                .host_service
                .count_by_group(filter.clone(), group_field.to_sql())
                .await?,
        ),
        None => None,
    };

    let mut result = state
        .services
        .host_service
        .get_all_host_responses_paginated(
            filter,
            &order_by,
            query.at,
            query.include_children.unwrap_or(true),
        )
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

    let response = PaginatedApiResponse::success(result.items, result.total_count, limit, offset);

    Ok(Json(match group_counts {
        Some(counts) => response.with_group_counts(counts),
        None => response,
    }))
}

/// Get a host by ID
///
/// Returns a single host with its ip_addresses, ports, and services.
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
        .get_tags_map(&[host.id], EntityDiscriminants::Host, None)
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
/// Creates a host with optional ip_addresses, ports, and services.
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
            if let Some(org_id) = organization_id {
                let plan = state
                    .services
                    .organization_service
                    .get_by_id(&org_id)
                    .await?
                    .and_then(|o| o.base.plan)
                    .unwrap_or_else(crate::server::billing::plans::get_free_plan);
                if let Some(limit) = plan.host_limit() {
                    // Counted the way the dashboard counts and the way the discovery path gates:
                    // through `count_for_networks`, which narrows SCD2 entities to live rows, and
                    // across *every* network the organization owns. Hand-rolling it here meant
                    // closed snapshot copies were counted and the caller's own accessible networks
                    // were the scope — so a user could be shown 18/25 and refused at 25, and a
                    // member whose networks are a subset of the org's could create past the limit.
                    let org_network_ids: Vec<Uuid> = state
                        .services
                        .network_service
                        .get_all(StorableFilter::<Network>::new_from_org_id(&org_id))
                        .await
                        .unwrap_or_default()
                        .iter()
                        .map(|n| n.id)
                        .collect();
                    let current_hosts = state
                        .services
                        .host_service
                        .count_for_networks(&org_network_ids)
                        .await?;
                    if current_hosts >= limit {
                        let _ = state
                            .services
                            .event_bus
                            .publish(Event::new(
                                OrgScope {
                                    organization_id: org_id,
                                },
                                BillingOperation::FeatureLimitHit {
                                    limit_type: LimitType::Hosts,
                                    current_count: current_hosts,
                                    limit,
                                    plan,
                                    source: LimitSource::Api,
                                },
                                entity.clone(),
                            ))
                            .await;

                        return Err(ApiError::coded(
                            StatusCode::FORBIDDEN,
                            ErrorCode::BillingHostLimitReached { limit },
                        ));
                    }
                }
            }

            // Check interface subnets are on the same network
            for ip_address in &request.ip_addresses {
                if let Some(subnet) = state
                    .services
                    .subnet_service
                    .get_by_id(&ip_address.subnet_id)
                    .await?
                    && subnet.base.network_id != request.network_id
                {
                    return Err(ApiError::bad_request(&format!(
                        "Host is on network {}, cannot have an ip_address with a subnet \"{}\" which is on network {}.",
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
                ip_addresses,
                ports,
                services,
                interfaces,
                subnets,
                interfaces_complete,
                interface_data_complete,
                superseded_wire_shape,
            } = discovery_request;

            // Always true on this branch — reaching it means the daemon sent the pre-0.16.0
            // body. Recorded against the daemon so the scan record carries it, which the
            // `tracing::warn!` above never could: a self-hosted operator does not read server
            // logs.
            if superseded_wire_shape {
                state
                    .services
                    .discovery_service
                    .note_superseded_wire_shape(*daemon_id)
                    .await;
            }

            // Capture one scan_time for the whole submission so all entities
            // share consistent SCD2 timestamps. See ScanContext for rationale.
            let scan_ctx =
                crate::server::shared::services::scan_context::ScanContext::new(*daemon_id);
            let host_response = host_service
                .discover_host(
                    host,
                    ip_addresses,
                    ports,
                    services,
                    interfaces,
                    subnets,
                    interfaces_complete,
                    interface_data_complete,
                    Some(&scan_ctx),
                    entity,
                    None,
                )
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
/// Updates host properties. Children (ip_addresses, ports, services, interfaces) are synced
/// from the fields on this same request body when provided — omit a field to leave that child
/// set untouched.
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
        .get_tags_map(&[host_response.id], EntityDiscriminants::Host, None)
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
    tags = [Host::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
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
) -> ApiResult<impl IntoResponse> {
    // Legacy cleanup: remove once minimum_supported >= 0.16.0
    let is_legacy_daemon = pre_interface_to_ip_address_rename(auth.entity.daemon_version());

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
        } else if let Some(err) = created.first_host_error {
            // Single-host discovery: surface the real per-host failure instead of
            // a generic message. The processor swallows per-host errors to keep
            // multi-host batches going; here there is exactly one host.
            ApiError::internal_error(&format!("Failed to process discovered host: {err}"))
        } else {
            ApiError::internal_error("No host returned from processor")
        }
    })?;

    // Legacy cleanup: remove once minimum_supported >= 0.16.0
    // Pre-0.16.0 daemons expect "Interface"/"interface_id" in binding responses
    if is_legacy_daemon {
        let mut json = serde_json::to_value(ApiResponse::success(host_response))
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
        rewrite_response_for_legacy_daemon(&mut json);
        return Ok(Json(json));
    }

    Ok(Json(
        serde_json::to_value(ApiResponse::success(host_response))
            .map_err(|e| ApiError::internal_error(&e.to_string()))?,
    ))
}

/// Rescan a host
///
/// Starts a one-shot scan of this host's addresses and nothing else, answering
/// "is this host still there, and is its data current?" without sweeping the
/// whole subnet.
///
/// The scan runs on the daemon that last discovered this host — evidence it can
/// reach the address — and only if that daemon still has an interface on a
/// subnet containing one of the host's scannable IPs. Where that interface has a
/// MAC the daemon ARPs the target, which sees a live host even when every port
/// is firewalled; on a MAC-less interface (a point-to-point tunnel) it falls
/// back to a TCP probe. When no interface covers any of the host's addresses the
/// request is refused with the specific reason. A loopback address is not a
/// scannable IP — it is reached locally and is excluded from the target set.
///
/// Returns the session, which streams progress over `/api/v1/discovery/stream`
/// like any other scan. A `Queued` phase means the daemon is busy; it will start
/// when the running scan finishes.
#[utoipa::path(
    post,
    path = "/{id}/rescan",
    tag = Host::ENTITY_NAME_PLURAL,
    params(("id" = uuid::Uuid, Path, description = "Host ID")),
    responses(
        (status = 200, description = "Rescan session started", body = ApiResponse<DiscoveryUpdatePayload>),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
        (status = 400, description = "Host cannot be rescanned (never scanned, daemon gone, daemon unreachable, or daemon too old)", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn rescan_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(host_id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<DiscoveryUpdatePayload>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let host = state
        .services
        .host_service
        .get_by_id(&host_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(host_id))?;

    validate_read_access(
        Some(host.base.network_id),
        None,
        &network_ids,
        organization_id,
    )?;

    let ip_addresses = state
        .services
        .ip_address_service
        .get_for_host(&host_id)
        .await?;
    if ip_addresses.is_empty() {
        return Err(ApiError::bad_request(
            "This host has no IP addresses to scan.",
        ));
    }

    let daemon = resolve_rescan_daemon(&state, &host).await?;

    // Only the addresses this daemon has a route to. The daemon ARPs the ones
    // on an interface with a MAC and falls back to a TCP probe on the ones
    // without (a point-to-point tunnel has no MAC but is perfectly routable) —
    // refusing the latter outright made rescan impossible on VPN-only daemons
    // rather than merely lower fidelity.
    //
    // A loopback address passes the junction check — the daemon reports its own
    // 127.0.0.0/8 as an interfaced subnet — but is not scannable by either path:
    // loopback subnets are dropped from every scan. Excluding it here keeps this
    // endpoint and the daemon's own target resolution agreeing, and keeps
    // 127.0.0.1 out of the frozen rescan name.
    let interfaced = state
        .services
        .daemon_service
        .get_interfaced_subnet_ids(&daemon.id)
        .await;
    let reachable: Vec<&IPAddress> = ip_addresses
        .iter()
        .filter(|ip| interfaced.contains(&ip.base.subnet_id) && !ip.base.ip_address.is_loopback())
        .collect();

    if reachable.is_empty() {
        return Err(ApiError::bad_request(&format!(
            "Daemon \"{}\" last scanned this host but has no interface on a subnet holding any \
             of its scannable addresses, so it can't scan them directly.",
            daemon.base.name
        )));
    }

    // The ports already recorded on this host. `get_for_hosts` applies the live
    // filter; `get_for_host` does not, and would resurrect historically-closed
    // ports into the scan list.
    let ports: Vec<PortType> = state
        .services
        .port_service
        .get_for_hosts(&[host_id], None)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .remove(&host_id)
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.base.port_type)
        .collect();

    // Inherit the daemon's configured scan settings so a deliberately lowered
    // scan rate — or raw-socket probing the operator turned on — carries over.
    // The conversion drops the full-scan fields, which a rescan must not have.
    let settings = RescanSettings::from(
        &daemon_scan_settings(&state, daemon.id)
            .await
            .unwrap_or_default(),
    );

    let discovery = Discovery::new(DiscoveryBase {
        run_type: RunType::AdHoc { last_run: None },
        discovery_type: DiscoveryType::Rescan {
            host_id: daemon.base.host_id,
            target_host_id: host.id,
            ips: reachable.iter().map(|ip| ip.base.ip_address).collect(),
            ports,
            settings,
        },
        // Both the host name and the addresses are only known here, and the
        // historical record inherits this name — so it carries the address the
        // scan actually aimed at, not just the host's current name.
        name: format!(
            "Rescan of {} ({})",
            host.base.name,
            reachable
                .iter()
                .map(|ip| ip.base.ip_address.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        daemon_id: daemon.id,
        network_id: host.base.network_id,
        tags: Vec::new(),
    });

    let discovery = state
        .services
        .discovery_service
        .create_discovery(discovery, AuthenticatedEntity::System)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Auto-wake the daemon, matching POST /discovery/start-session — start_session
    // itself doesn't clear standby, and a standby daemon never picks up the work.
    let mut daemon = daemon;
    if daemon.base.standby {
        daemon.base.standby = false;
        daemon.base.standby_cleared_at = Some(chrono::Utc::now());
        state
            .services
            .daemon_service
            .update(&mut daemon, AuthenticatedEntity::System)
            .await?;
        tracing::info!(daemon_id = %daemon.id, "Cleared daemon standby (rescan started)");
    }

    let update = state
        .services
        .discovery_service
        .start_session(discovery, auth.into_entity())
        .await?;

    Ok(Json(ApiResponse::success(update)))
}

/// The scan settings on this daemon's own discovery configuration, if it has one.
async fn daemon_scan_settings(state: &Arc<AppState>, daemon_id: Uuid) -> Option<ScanSettings> {
    let filter = StorableFilter::new_from_uuid_column("daemon_id", &daemon_id).live_configs();
    let discoveries = state
        .services
        .discovery_service
        .get_all(filter)
        .await
        .ok()?;
    discoveries
        .into_iter()
        .find_map(|d| match d.base.discovery_type {
            DiscoveryType::Unified { scan_settings, .. } => Some(scan_settings),
            _ => None,
        })
}

/// The daemon that last discovered this host, validated as usable for a rescan.
///
/// Provenance beats inference here: the daemon that produced this host's data
/// demonstrably reached the address, which is a stronger signal than any
/// subnet-membership calculation over the interfaced-subnet junction.
async fn resolve_rescan_daemon(state: &Arc<AppState>, host: &Host) -> Result<Daemon, ApiError> {
    let Some(discovery_id) = host.last_discovery_id else {
        return Err(ApiError::bad_request(
            "This host hasn't been scanned yet, so there is no daemon known to reach it. \
             Run a discovery first.",
        ));
    };

    // The FK is ON DELETE SET NULL, so a pruned historical row lands above rather
    // than here; this covers a row that vanished between reads.
    let last_scan = state
        .services
        .discovery_service
        .get_by_id(&discovery_id)
        .await?
        .ok_or_else(|| {
            ApiError::bad_request(
                "The scan that last found this host is no longer on record, so its daemon \
                 can't be identified. Run a discovery first.",
            )
        })?;

    let daemon = state
        .services
        .daemon_service
        .get_by_id(&last_scan.base.daemon_id)
        .await?
        .ok_or_else(|| {
            ApiError::bad_request(
                "The daemon that last scanned this host no longer exists. Run a discovery from \
                 another daemon to refresh it.",
            )
        })?;

    if daemon.base.network_id != host.base.network_id {
        return Err(ApiError::bad_request(&format!(
            "Daemon \"{}\" has moved to a different network and can no longer reach this host.",
            daemon.base.name
        )));
    }

    if !supports_targeted_rescan(daemon.base.version.as_ref()) {
        return Err(match daemon.base.version {
            None => ApiError::bad_request(&format!(
                "Daemon \"{}\" has not connected to the server yet, so its version is unknown. \
                 Wait for it to check in, then try again.",
                daemon.base.name
            )),
            Some(_) => ApiError::bad_request(&format!(
                "Daemon \"{}\" does not support rescanning a single host. Upgrade it to version \
                 {} or later.",
                daemon.base.name,
                minimum_targeted_rescan()
            )),
        });
    }

    Ok(daemon)
}

/// Consolidate hosts
///
/// Merges all ip_addresses, ports, and services from `other_host` into
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
        .get_tags_map(&[host_response.id], EntityDiscriminants::Host, None)
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
    if state.services.daemon_service.exists(daemon_filter).await? {
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
    request_body(content = Vec<Uuid>, description = "Array of Host IDs to delete"),
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
/// (ip_addresses, ports, services, interfaces) as a ZIP archive containing
/// separate CSV files for each entity type.
#[utoipa::path(
    get,
    path = "/export/zip",
    tag = Host::ENTITY_NAME_PLURAL,
    operation_id = "export_hosts_zip",
    params(HostFilterQuery),
    responses(
        (status = 200, description = "ZIP file containing CSVs", content_type = "application/zip", body = String),
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

    // SCD2 read path: live by default, or as-of the snapshot timestamp when set.
    // Without this, close-and-clone's closed historical copies leak into the
    // host list as empty-shell duplicates.
    let filter = filter.live_or_as_of(query.at);

    // Apply tag filter if specified
    let filter = match &query.tag_ids {
        Some(tag_ids) if !tag_ids.is_empty() => {
            filter.has_any_tags(tag_ids, EntityDiscriminants::Host)
        }
        _ => filter,
    };

    // Honour the same staleness filter as the list view so an export mirrors
    // exactly what the user was looking at when they triggered it.
    let filter = match query.stale {
        Some(stale) => {
            let cutoffs = state
                .services
                .network_service
                .stale_cutoffs(&network_ids)
                .await?;
            filter.stale_by_network(&cutoffs, stale)
        }
        None => filter,
    };

    let filter = query.apply_field_filters(filter);

    // Get all hosts (no pagination for export)
    let hosts = state.services.host_service.get_all(filter).await?;
    let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();

    // Fetch children for these hosts
    let ip_addresses = state
        .services
        .ip_address_service
        .get_all(
            StorableFilter::<IPAddress>::new_from_host_ids(&host_ids).network_ids(&network_ids),
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

    let interfaces = state
        .services
        .interface_service
        .get_all(
            StorableFilter::<Interface>::new_from_host_ids(&host_ids).network_ids(&network_ids),
        )
        .await?;

    // Build CSVs
    let hosts_csv = build_csv(&hosts)
        .map_err(|e| ApiError::internal_error(&format!("Failed to build hosts CSV: {}", e)))?;
    let ip_addresses_csv = build_csv(&ip_addresses).map_err(|e| {
        ApiError::internal_error(&format!("Failed to build ip_addresses CSV: {}", e))
    })?;
    let ports_csv = build_csv(&ports)
        .map_err(|e| ApiError::internal_error(&format!("Failed to build ports CSV: {}", e)))?;
    let services_csv = build_csv(&services)
        .map_err(|e| ApiError::internal_error(&format!("Failed to build services CSV: {}", e)))?;
    let if_entries_csv = build_csv(&interfaces)
        .map_err(|e| ApiError::internal_error(&format!("Failed to build interfaces CSV: {}", e)))?;

    // Build zip archive
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file("hosts.csv", options)
            .map_err(|e| ApiError::internal_error(&format!("Failed to create zip: {}", e)))?;
        zip.write_all(&hosts_csv)
            .map_err(|e| ApiError::internal_error(&format!("Failed to write zip: {}", e)))?;

        zip.start_file("ip_addresses.csv", options)
            .map_err(|e| ApiError::internal_error(&format!("Failed to create zip: {}", e)))?;
        zip.write_all(&ip_addresses_csv)
            .map_err(|e| ApiError::internal_error(&format!("Failed to write zip: {}", e)))?;

        zip.start_file("ports.csv", options)
            .map_err(|e| ApiError::internal_error(&format!("Failed to create zip: {}", e)))?;
        zip.write_all(&ports_csv)
            .map_err(|e| ApiError::internal_error(&format!("Failed to write zip: {}", e)))?;

        zip.start_file("services.csv", options)
            .map_err(|e| ApiError::internal_error(&format!("Failed to create zip: {}", e)))?;
        zip.write_all(&services_csv)
            .map_err(|e| ApiError::internal_error(&format!("Failed to write zip: {}", e)))?;

        zip.start_file("interfaces.csv", options)
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

use crate::server::shared::legacy::rewrite_response_for_legacy_daemon;
