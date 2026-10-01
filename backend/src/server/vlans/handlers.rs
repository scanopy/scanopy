use crate::server::auth::middleware::permissions::{Authorized, IsDaemon, Member, Viewer};
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::create_handler;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, PaginatedApiResponse};
use crate::server::vlans::r#impl::base::Vlan;
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult},
};
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Vlan Ordering
// ============================================================================

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum VlanOrderField {
    #[default]
    CreatedAt,
    Name,
    VlanNumber,
    UpdatedAt,
}

impl OrderField for VlanOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "vlans.created_at",
            Self::Name => "vlans.name",
            Self::VlanNumber => "vlans.vlan_number",
            Self::UpdatedAt => "vlans.updated_at",
        }
    }
}

// ============================================================================
// Vlan Filter Query
// ============================================================================

#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct VlanFilterQuery {
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<VlanOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<VlanOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
    /// instant (snapshot view) instead of live state.
    pub at: Option<chrono::DateTime<chrono::Utc>>,
}

impl VlanFilterQuery {
    pub fn apply_ordering(&self, filter: StorableFilter<Vlan>) -> (StorableFilter<Vlan>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "vlans.vlan_number ASC",
        )
    }
}

impl FilterQueryExtractor for VlanFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        mut filter: StorableFilter<T>,
        user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        // If a specific network is requested, filter to it (must be in user's accessible networks)
        if let Some(network_id) = self.network_id {
            if user_network_ids.contains(&network_id) {
                filter = filter.uuid_column("network_id", &network_id);
            } else {
                // User doesn't have access to this network — return empty
                filter = filter.uuid_column("network_id", &Uuid::nil());
            }
        }
        filter
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
    crate::crud_get_by_id_handler!(Vlan);
    crate::crud_update_handler!(Vlan);
    crate::crud_delete_handler!(Vlan);
    crate::crud_bulk_delete_handler!(Vlan);
    crate::crud_export_csv_handler!(Vlan);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_vlans, create_vlan))
        .routes(routes!(
            generated::get_by_id,
            generated::update,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
        .routes(routes!(discovery_upsert_vlans))
}

/// List all VLANs
///
/// Returns VLANs accessible to the authenticated user, optionally filtered by network.
#[utoipa::path(
    get,
    path = "",
    tag = Vlan::ENTITY_NAME_PLURAL,
    params(VlanFilterQuery),
    responses(
        (status = 200, description = "List of VLANs", body = PaginatedApiResponse<Vlan>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_vlans(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    crate::server::shared::extractors::Query(query): crate::server::shared::extractors::Query<
        VlanFilterQuery,
    >,
) -> ApiResult<Json<PaginatedApiResponse<Vlan>>> {
    let network_ids = auth.network_ids();

    let base_filter = StorableFilter::<Vlan>::new_from_network_ids(&network_ids);
    let filter = query
        .apply_to_filter(base_filter, &network_ids, Uuid::nil())
        .live_or_as_of(query.at);

    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    let (filter, order_by) = query.apply_ordering(filter);

    // Through the service, not `storage()`, so each VLAN's `subnet_ids` are
    // hydrated from the `subnet_vlans` junction.
    let result = state
        .services
        .vlan_service
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

/// Create a new VLAN
///
/// Creates a VLAN scoped to a network. VLAN numbers must be unique within a network.
#[utoipa::path(
    post,
    path = "",
    tag = Vlan::ENTITY_NAME_PLURAL,
    request_body = Vlan,
    responses(
        (status = 200, description = "VLAN created successfully", body = ApiResponse<Vlan>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
        (status = 409, description = "VLAN number already exists in this network", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_vlan(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(vlan): Json<Vlan>,
) -> ApiResult<Json<ApiResponse<Vlan>>> {
    let network_ids = auth.network_ids();

    // Verify user has access to the target network
    if !network_ids.contains(&vlan.base.network_id) {
        return Err(ApiError::forbidden("Access denied to this network"));
    }

    // Check uniqueness: (network_id, vlan_number)
    let existing_filter =
        StorableFilter::<Vlan>::new_from_uuid_column("network_id", &vlan.base.network_id)
            .u16_column("vlan_number", vlan.base.vlan_number);

    if state.services.vlan_service.exists(existing_filter).await? {
        return Err(ApiError::conflict(&format!(
            "VLAN {} already exists in this network",
            vlan.base.vlan_number
        )));
    }

    create_handler::<Vlan>(State(state), auth, Json(vlan)).await
}

// ============================================================================
// Discovery Upsert
// ============================================================================

/// Request body for daemon VLAN discovery upsert
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VlanDiscoveryRequest {
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// VLANs observed by the daemon.
    pub vlans: Vec<VlanDiscoveryItem>,
}

/// Upper bound on VLANs in one discovery submission: the 802.1Q ID space (1-4094).
const MAX_VLANS_PER_DISCOVERY: usize = 4094;

impl VlanDiscoveryRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if self.vlans.len() > MAX_VLANS_PER_DISCOVERY {
            return Err(ApiError::bad_request(&format!(
                "A discovery submission can contain at most {} VLANs, got {}",
                MAX_VLANS_PER_DISCOVERY,
                self.vlans.len()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct VlanDiscoveryItem {
    /// 802.1Q VLAN ID.
    pub vlan_number: u16,
    /// VLAN name as configured on the device.
    pub name: String,
}

/// Response for discovery upsert
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VlanDiscoveryResponse {
    /// Mapping of vlan_number → VLAN entity UUID
    pub vlans: Vec<VlanDiscoveryResponseItem>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VlanDiscoveryResponseItem {
    /// 802.1Q VLAN ID.
    pub vlan_number: u16,
    /// Server-assigned unique identifier.
    pub id: Uuid,
}

/// Bulk upsert VLANs from discovery
///
/// Used by daemons to report discovered VLANs. Creates new VLANs or updates names.
/// Returns the mapping of VLAN numbers to entity UUIDs for Interface construction.
#[utoipa::path(
    post,
    path = "/discovery",
    tag = Vlan::ENTITY_NAME_PLURAL,
    request_body = VlanDiscoveryRequest,
    responses(
        (status = 200, description = "VLANs upserted", body = ApiResponse<VlanDiscoveryResponse>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
pub async fn discovery_upsert_vlans(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Json(request): Json<VlanDiscoveryRequest>,
) -> ApiResult<Json<ApiResponse<VlanDiscoveryResponse>>> {
    request.validate()?;

    // Daemons don't carry org_id directly — resolve from the network
    let network = state
        .services
        .network_service
        .get_by_id(&request.network_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Network not found".to_string()))?;
    let organization_id = network.base.organization_id;

    // Verify daemon has access to this network
    let daemon_network_ids = auth.network_ids();
    if !daemon_network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "Daemon cannot create VLANs on networks it's not assigned to",
        ));
    }

    // Capture one scan_time for the whole submission so all VLANs share
    // consistent SCD2 origination timestamps. See ScanContext for rationale.
    let scan_ctx = auth.daemon_id().map(|daemon_id| {
        crate::server::shared::services::scan_context::ScanContext::new(daemon_id)
    });

    let mut response_items = Vec::with_capacity(request.vlans.len());

    for item in request.vlans {
        let vlan = state
            .services
            .vlan_service
            .upsert_from_discovery(
                request.network_id,
                organization_id,
                item.vlan_number,
                item.name,
                scan_ctx.as_ref(),
            )
            .await
            .map_err(|e| ApiError::internal_error(&format!("Failed to upsert VLAN: {}", e)))?;

        response_items.push(VlanDiscoveryResponseItem {
            vlan_number: vlan.base.vlan_number,
            id: vlan.id,
        });
    }

    Ok(Json(ApiResponse::success(VlanDiscoveryResponse {
        vlans: response_items,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    fn request_with(count: usize) -> VlanDiscoveryRequest {
        VlanDiscoveryRequest {
            network_id: Uuid::new_v4(),
            vlans: (0..count)
                .map(|i| VlanDiscoveryItem {
                    vlan_number: (i % MAX_VLANS_PER_DISCOVERY) as u16 + 1,
                    name: String::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn discovery_request_at_bound_is_accepted() {
        assert!(request_with(MAX_VLANS_PER_DISCOVERY).validate().is_ok());
    }

    #[test]
    fn discovery_request_over_bound_is_rejected() {
        let err = request_with(MAX_VLANS_PER_DISCOVERY + 1)
            .validate()
            .unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }
}
