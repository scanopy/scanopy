use axum::Json;
use axum::extract::{Path, State};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member};
use crate::server::config::AppState;
use crate::server::hosts::r#impl::base::Host;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::interfaces::service::InterfaceService;
use crate::server::shared::handlers::query::HostChildQuery;
use crate::server::shared::handlers::traits::{CrudHandlers, create_handler, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult};

impl CrudHandlers for Interface {
    type Service = InterfaceService;
    type FilterQuery = HostChildQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.interface_service
    }
}

// Generated handlers
mod generated {
    use super::*;
    crate::crud_get_all_handler!(Interface);
    crate::crud_get_by_id_handler!(Interface);
    crate::crud_delete_handler!(Interface);
    crate::crud_bulk_delete_handler!(Interface);
    crate::crud_export_csv_handler!(Interface);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_if_entry))
        .routes(routes!(generated::export_csv))
        .routes(routes!(
            generated::get_by_id,
            update_if_entry,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
}

/// Validate that if entry's host is on the same network as the entry
async fn validate_if_entry_network_consistency(
    state: &AppState,
    interface: &Interface,
) -> Result<(), ApiError> {
    if let Some(host) = state
        .services
        .host_service
        .get_by_id(&interface.base.host_id)
        .await?
        && host.base.network_id != interface.base.network_id
    {
        return Err(ApiError::entity_network_mismatch::<Host>());
    }

    Ok(())
}

/// Create a new Interface
///
/// Creates an SNMP ifTable entry for a host. These are typically created by
/// SNMP discovery, but can also be created manually.
#[utoipa::path(
    post,
    path = "",
    tag = Interface::ENTITY_NAME_PLURAL,
    request_body = Interface,
    responses(
        (status = 200, description = "If entry created successfully", body = ApiResponse<Interface>),
        (status = 400, description = "Network mismatch or duplicate if_index", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn create_if_entry(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(interface): Json<Interface>,
) -> ApiResult<Json<ApiResponse<Interface>>> {
    validate_if_entry_network_consistency(&state, &interface).await?;
    state
        .services
        .interface_service
        .validate_relationships(&interface)
        .await
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;
    create_handler::<Interface>(State(state), auth, Json(interface)).await
}

/// Update an Interface
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Interface::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "If entry ID")),
    request_body = Interface,
    responses(
        (status = 200, description = "If entry updated successfully", body = ApiResponse<Interface>),
        (status = 400, description = "Network mismatch or invalid request", body = ApiErrorResponse),
        (status = 404, description = "If entry not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_if_entry(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
    Json(interface): Json<Interface>,
) -> ApiResult<Json<ApiResponse<Interface>>> {
    validate_if_entry_network_consistency(&state, &interface).await?;
    state
        .services
        .interface_service
        .validate_relationships(&interface)
        .await
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;
    update_handler::<Interface>(State(state), auth, path, Json(interface)).await
}
