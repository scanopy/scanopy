use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, IsDaemon, Member, Or, Viewer};
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::networks::r#impl::Network;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::{CrudHandlers, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiJson, ApiResponse, ApiResult, PaginatedApiResponse,
};
use crate::server::shared::types::entities::EntitySource;
use crate::server::{
    config::AppState,
    subnets::r#impl::base::{Subnet, SubnetCidr},
};
use axum::extract::{Path, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Subnet Ordering
// ============================================================================

/// Fields that subnets can be ordered/grouped by.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubnetOrderField {
    #[default]
    CreatedAt,
    Name,
    Cidr,
    SubnetType,
    UpdatedAt,
    NetworkId,
    /// Sort by when discovery last observed the subnet. Surfaces stale assets.
    LastSeenAt,
}

impl OrderField for SubnetOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "subnets.created_at",
            Self::Name => "subnets.name",
            Self::Cidr => "subnets.cidr",
            Self::SubnetType => "subnets.subnet_type",
            Self::UpdatedAt => "subnets.updated_at",
            Self::NetworkId => "subnets.network_id",
            Self::LastSeenAt => "subnets.last_seen_at",
        }
    }
}

// ============================================================================
// Subnet Filter Query
// ============================================================================

/// Query parameters for filtering and ordering subnets.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct SubnetFilterQuery {
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<SubnetOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<SubnetOrderField>,
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
    /// `true` returns only subnets discovery hasn't observed within their
    /// network's staleness window; `false` returns only those it has. Omit for
    /// both. Evaluated per row against the subnet's own network's window.
    pub stale: Option<bool>,
}

impl SubnetFilterQuery {
    /// Build the ORDER BY clause.
    pub fn apply_ordering(
        &self,
        filter: StorableFilter<Subnet>,
    ) -> (StorableFilter<Subnet>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "subnets.created_at ASC",
        )
    }
}

impl FilterQueryExtractor for SubnetFilterQuery {
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

// Generated handlers for most CRUD operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Subnet);
    crate::crud_delete_handler!(Subnet);
    crate::crud_bulk_delete_handler!(Subnet);
    crate::crud_export_csv_handler!(Subnet);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_subnets, create_subnet))
        .routes(routes!(
            generated::get_by_id,
            update_subnet,
            generated::delete
        ))
        .routes(routes!(merge_subnet))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
}

/// Get all subnets
///
/// Returns all subnets accessible to the authenticated user or daemon.
/// Daemons can only access subnets within their assigned network.
/// Supports pagination via `limit` and `offset` query parameters,
/// and ordering via `group_by`, `order_by`, and `order_direction`.
#[utoipa::path(
    get,
    path = "",
    tag = Subnet::ENTITY_NAME_PLURAL,
    operation_id = "list_subnets",
    summary = "List all subnets",
    params(SubnetFilterQuery),
    responses(
        (status = 200, description = "List of subnets", body = PaginatedApiResponse<Subnet>),
    ),
    security( ("user_api_key" = []),("session" = []), ("daemon_api_key" = []))
)]
async fn get_all_subnets(
    state: State<Arc<AppState>>,
    auth: Authorized<Or<Viewer, IsDaemon>>,
    query: Query<SubnetFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Subnet>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth.organization_id();
    let entity = auth.into_entity();

    match entity {
        AuthenticatedEntity::Daemon { network_id, .. } => {
            // Daemons can only access subnets in their network
            // Return all results (no pagination applied). SCD2: live rows only —
            // daemons operate on current state and must never see closed copies.
            let filter = StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live();
            let service = Subnet::get_service(&state);
            let result = service.get_all(filter).await.map_err(|e| {
                tracing::error!(
                    error = %e,
                    network_id = %network_id,
                    "Failed to fetch subnets for daemon"
                );
                ApiError::internal_error(&e.to_string())
            })?;
            let total_count = result.len() as u64;
            Ok(Json(PaginatedApiResponse::success(
                result,
                total_count,
                0,
                0,
            )))
        }
        _ => {
            // Users/API keys - use standard filter with query params
            let org_id = organization_id.ok_or_else(ApiError::organization_required)?;
            let base_filter = StorableFilter::<Subnet>::new_from_network_ids(&network_ids);
            let filter = query
                .apply_to_filter(base_filter, &network_ids, org_id)
                .live_or_as_of(query.at);

            // Staleness is per-network, so resolve each accessible network's
            // cutoff and let the filter compare every row against its own.
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

            // Apply ordering
            let (filter, order_by) = query.apply_ordering(filter);

            let result = state
                .services
                .subnet_service
                .get_paginated_ordered(filter, &order_by)
                .await?;

            let limit = pagination.effective_limit().unwrap_or(0);
            let offset = pagination.effective_offset();
            Ok(Json(PaginatedApiResponse::success(
                result.items,
                result.total_count,
                limit,
                offset,
            )))
        }
    }
}

/// Create a new subnet
#[utoipa::path(
    post,
    path = "",
    tag = Subnet::ENTITY_NAME_PLURAL,
    request_body = Subnet,
    responses(
        (status = 200, description = "Subnet created successfully", body = ApiResponse<Subnet>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    ),
    security( ("user_api_key" = []),("session" = []), ("daemon_api_key" = []))
)]
async fn create_subnet(
    state: State<Arc<AppState>>,
    auth: Authorized<Or<Member, IsDaemon>>,
    ApiJson(mut request): ApiJson<Subnet>,
) -> ApiResult<Json<ApiResponse<Subnet>>> {
    let network_ids = auth.network_ids();
    let entity = auth.into_entity();

    tracing::debug!(
        subnet_name = %request.base.name,
        subnet_cidr = %request.base.cidr,
        network_id = %request.base.network_id,
        entity_id = %entity.entity_id().unwrap_or_default(),
        "Subnet create request received"
    );

    if let Err(err) = request.validate() {
        tracing::warn!(
            subnet_name = %request.base.name,
            subnet_cidr = %request.base.cidr,
            entity_id = %entity.entity_id().unwrap_or_default(),
            error = %err,
            "Subnet validation failed"
        );
        return Err(ApiError::bad_request(&format!(
            "Subnet validation failed: {}",
            err
        )));
    }

    let created = match &entity {
        AuthenticatedEntity::Daemon { network_id, .. } => {
            if *network_id == request.base.network_id {
                let service = Subnet::get_service(&state);
                let created = service.create(request, entity).await.map_err(|e| {
                    tracing::error!(
                        error = %e,
                        "Failed to create subnet"
                    );
                    ApiError::internal_error(&e.to_string())
                })?;
                Json(ApiResponse::success(created))
            } else {
                return Err(ApiError::entity_network_mismatch::<Subnet>());
            }
        }
        _ => {
            // User/API key - validate network access and create
            if !network_ids.contains(&request.base.network_id) {
                return Err(ApiError::entity_access_denied::<Network>(
                    request.base.network_id,
                ));
            }
            // A range a person typed is an assertion, not a reading: `set_source` settles the
            // confidence ladder along with the source, so nothing a later scan reads displaces it.
            // Every other entity gets this from `create_handler`; subnets miss it because this
            // handler is written out longhand for the daemon case above, and never picked it up.
            // Stamped server-side rather than trusted from the body, so a client cannot claim
            // `Discovery` for a row it typed.
            request.set_source(EntitySource::Manual);
            let service = Subnet::get_service(&state);
            let created = service.create(request, entity).await.map_err(|e| {
                tracing::error!(error = %e, "Failed to create subnet");
                ApiError::internal_error(&e.to_string())
            })?;
            Json(ApiResponse::success(created))
        }
    };

    Ok(created)
}

/// Update a subnet
///
/// Updates subnet properties. If the CIDR is being changed, validates that
/// all existing ip_addresses on this subnet have IPs within the new CIDR range.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Subnet::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Subnet ID")),
    request_body = Subnet,
    responses(
        (status = 200, description = "Subnet updated", body = ApiResponse<Subnet>),
        (status = 400, description = "CIDR change would orphan existing ip_addresses", body = ApiErrorResponse),
        (status = 404, description = "Subnet not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_subnet(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    ApiJson(mut subnet): ApiJson<Subnet>,
) -> ApiResult<Json<ApiResponse<Subnet>>> {
    // Check if CIDR is being changed
    let current = state
        .services
        .subnet_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Subnet>(id))?;

    if current.base.cidr != subnet.base.cidr {
        // Moving the range through this endpoint is a person's assertion, so it settles the
        // confidence ladder: `Confirmed` outranks anything a scan reads, which is what stops a
        // corrected range being re-corrected on the next discovery. This lives here rather than in
        // `preserve_immutable_fields` because here it is *true* — the route is `Authorized<Member>`
        // and no daemon can reach it — whereas storage sees every writer alike.
        subnet.base.cidr =
            SubnetCidr::new(subnet.base.cidr.value().clone(), AttributeSource::Manual);

        // CIDR is changing - validate that all existing ip_addresses are within the new CIDR
        let filter = StorableFilter::<IPAddress>::new_from_subnet_id(&id);
        let ip_addresses = state
            .services
            .ip_address_service
            .get_all(filter)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        for ip_address in &ip_addresses {
            if !subnet.base.cidr.contains(&ip_address.base.ip_address) {
                return Err(ApiError::ip_address_out_of_range(
                    &ip_address.base.ip_address.to_string(),
                    &subnet.base.cidr.to_string(),
                ));
            }
        }
    }

    // Delegate to generic handler
    update_handler::<Subnet>(State(state), auth, Path(id), ApiJson(subnet)).await
}

/// Request body for merging a subnet into the range that contains it.
#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct MergeSubnetRequest {
    /// The subnet to merge into. Must contain the range being merged.
    pub into: Uuid,
}

/// Merge a subnet into the range that contains it
///
/// Moves every address to the covering subnet and removes this one. Offered for a range Scanopy
/// assumed that a later reading turned out to cover: discovery corrects such a range on its own
/// only where the answer is unambiguous, and folding several assumed ranges into one means deleting
/// rows, which is a person's call rather than a scan's.
#[utoipa::path(
    post,
    path = "/{id}/merge",
    tag = Subnet::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Subnet ID to merge away")),
    request_body = MergeSubnetRequest,
    responses(
        (status = 200, description = "Subnet merged", body = ApiResponse<Subnet>),
        (status = 400, description = "The target does not contain this subnet", body = ApiErrorResponse),
        (status = 404, description = "Subnet not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn merge_subnet(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    ApiJson(request): ApiJson<MergeSubnetRequest>,
) -> ApiResult<Json<ApiResponse<Subnet>>> {
    let merged = state
        .services
        .subnet_service
        .merge_into(id, request.into, auth.entity.clone())
        .await
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    Ok(Json(ApiResponse::success(merged)))
}
