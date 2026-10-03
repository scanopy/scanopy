use crate::server::auth::middleware::permissions::{Authorized, Member};
use crate::server::config::AppState;
use crate::server::hosts::r#impl::base::Host;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::ip_addresses::service::IPAddressService;
use crate::server::shared::handlers::query::IPAddressQuery;
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, create_handler, update_handler,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::lock::{self, DEFAULT_LOCK_TIMEOUT, LockKey};
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
};
use crate::server::shared::validation::{validate_bulk_delete_access, validate_delete_access};
use crate::server::subnets::r#impl::base::Subnet;
use axum::Json;
use axum::extract::{Path, State};
use std::collections::HashSet;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

impl CrudHandlers for IPAddress {
    type Service = IPAddressService;
    type FilterQuery = IPAddressQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.ip_address_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_all_handler!(IPAddress);
    crate::crud_export_csv_handler!(IPAddress);
    crate::crud_get_by_id_handler!(IPAddress);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_ip_address))
        .routes(routes!(
            generated::get_by_id,
            update_ip_address,
            delete_ip_address
        ))
        .routes(routes!(bulk_delete_ip_addresses))
        .routes(routes!(generated::export_csv))
}

/// Validate that the IP address's host and subnet are on the same network as the IP address,
/// and that the IP value falls within the subnet's CIDR range.
async fn validate_ip_address_consistency(
    state: &AppState,
    ip_address: &IPAddress,
) -> Result<(), ApiError> {
    // Validate host is on the same network
    if let Some(host) = state
        .services
        .host_service
        .get_by_id(&ip_address.base.host_id)
        .await?
        && host.base.network_id != ip_address.base.network_id
    {
        return Err(ApiError::entity_network_mismatch::<Host>());
    }

    // Validate subnet is on the same network AND IP is within CIDR
    if let Some(subnet) = state
        .services
        .subnet_service
        .get_by_id(&ip_address.base.subnet_id)
        .await?
    {
        if subnet.base.network_id != ip_address.base.network_id {
            return Err(ApiError::entity_network_mismatch::<Subnet>());
        }

        // Validate IP address is within subnet CIDR
        if !subnet.base.cidr.contains(&ip_address.base.ip_address) {
            return Err(ApiError::ip_address_out_of_range(
                &ip_address.base.ip_address.to_string(),
                &subnet.base.name,
            ));
        }
    }

    Ok(())
}

/// Create a new IP address
/// Position is automatically assigned to the end of the host's IP address list.
#[utoipa::path(
    post,
    path = "",
    tag = IPAddress::ENTITY_NAME_PLURAL,
    request_body = IPAddress,
    responses(
        (status = 200, description = "IP address created successfully", body = ApiResponse<IPAddress>),
        (status = 400, description = "Network mismatch or invalid request", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_ip_address(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    ApiJson(mut ip_address): ApiJson<IPAddress>,
) -> ApiResult<Json<ApiResponse<IPAddress>>> {
    validate_ip_address_consistency(&state, &ip_address).await?;

    // DB-level lock per host: the MAX+1 position assignment below plus the
    // insert are a read-modify-write with no row to lock (the new row
    // doesn't exist yet); without this, concurrent creates on one host get
    // duplicate positions. Error paths release via Drop.
    let lock_guard = lock::session_lock(
        &state.pool,
        LockKey::IpPositions {
            host_id: ip_address.base.host_id,
        },
        DEFAULT_LOCK_TIMEOUT,
    )
    .await
    .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Auto-assign position to end of list (ignore any position in the request)
    let next_position = state
        .services
        .ip_address_service
        .get_next_position_for_host(&ip_address.base.host_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    ip_address.base.position = next_position;

    let response = create_handler::<IPAddress>(State(state), auth, ApiJson(ip_address)).await;
    lock_guard
        .release()
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    response
}

/// Update an IP address
/// Position must be within valid range and not conflict with other IP addresses.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = IPAddress::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "IP address ID")),
    request_body = IPAddress,
    responses(
        (status = 200, description = "IP address updated successfully", body = ApiResponse<IPAddress>),
        (status = 400, description = "Network mismatch or invalid request", body = ApiErrorResponse),
        (status = 404, description = "IP address not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_ip_address(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
    ApiJson(ip_address): ApiJson<IPAddress>,
) -> ApiResult<Json<ApiResponse<IPAddress>>> {
    validate_ip_address_consistency(&state, &ip_address).await?;

    // Validate position is within range and doesn't conflict
    state
        .services
        .ip_address_service
        .validate_position_for_update(&path, &ip_address.base.host_id, ip_address.base.position)
        .await?;

    update_handler::<IPAddress>(State(state), auth, path, ApiJson(ip_address)).await
}

/// Delete an IP address
/// Remaining IP addresses for the host are renumbered to maintain sequential positions.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = IPAddress::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "IP address ID")),
    responses(
        (status = 200, description = "IP address deleted successfully", body = EmptyApiResponse),
        (status = 404, description = "IP address not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn delete_ip_address(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let entity_auth = auth.into_entity();

    let service = &state.services.ip_address_service;

    // Fetch entity first to verify ownership and get host_id
    let entity = service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<IPAddress>(id))?;

    validate_delete_access(
        Some(entity.base.network_id),
        None,
        &network_ids,
        organization_id,
    )?;

    let host_id = entity.base.host_id;

    // Delete the IP address
    service
        .delete(&id, entity_auth.clone())
        .await
        .map_err(ApiError::from)?;

    // Renumber remaining IP addresses for this host
    service
        .renumber_ip_addresses_for_host(&host_id, entity_auth)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

/// Bulk delete IP addresses
/// Remaining IP addresses for affected hosts are renumbered to maintain sequential positions.
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = IPAddress::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "Array of IP Address IDs to delete"),
    responses(
        (status = 200, description = "IP addresses deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 400, description = "No IDs provided", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_delete_ip_addresses(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    if ids.is_empty() {
        return Err(ApiError::bulk_empty());
    }

    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let entity_auth = auth.into_entity();

    let service = &state.services.ip_address_service;

    // Fetch all entities by the requested IDs
    let entity_filter = StorableFilter::<IPAddress>::new_from_entity_ids(&ids);
    let entities = service.get_all(entity_filter).await?;

    // Collect affected host IDs for renumbering
    let affected_host_ids: HashSet<Uuid> = entities.iter().map(|e| e.base.host_id).collect();

    // Verify ownership of ALL entities before deleting any
    for entity in &entities {
        validate_bulk_delete_access(
            Some(entity.base.network_id),
            None,
            &network_ids,
            organization_id,
        )?;
    }

    // Only delete entities that actually exist and user has access to
    let valid_ids: Vec<Uuid> = entities.iter().map(|e| e.id).collect();
    let deleted_count = service
        .delete_many(&valid_ids, entity_auth.clone())
        .await
        .map_err(ApiError::from)?;

    // Renumber remaining IP addresses for all affected hosts
    for host_id in affected_host_ids {
        service
            .renumber_ip_addresses_for_host(&host_id, entity_auth.clone())
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    Ok(Json(ApiResponse::success(BulkDeleteResponse {
        deleted_count,
        requested_count: ids.len(),
    })))
}
