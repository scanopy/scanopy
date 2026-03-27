use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::query::NoFilterQuery;
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, bulk_delete_handler, create_handler, delete_handler,
    get_all_handler, update_handler,
};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::PaginatedApiResponse;
use crate::server::topology::types::base::{Topology, TopologyBase};
use crate::server::{
    auth::middleware::{
        features::{CreateNetworkFeature, RequireFeature},
        permissions::{Admin, Authorized, Member, Viewer},
    },
    shared::{
        events::types::{OnboardingEvent, OnboardingOperation},
        types::api::{ApiError, ApiErrorResponse, EmptyApiResponse},
    },
};
use crate::server::{
    config::AppState,
    networks::r#impl::Network,
    shared::types::api::{ApiResponse, ApiResult},
};
use axum::extract::{Path, State};
use axum::response::Json;
use chrono::Utc;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    // get_all and get_by_id are now custom (below) to hydrate credential_ids
    crate::crud_export_csv_handler!(Network);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_networks, create_network))
        .routes(routes!(get_by_id_network, update_network, delete_network))
        .routes(routes!(bulk_delete_networks))
        .routes(routes!(generated::export_csv))
}

/// List all networks
#[utoipa::path(
    get,
    path = "",
    tag = Network::ENTITY_NAME_PLURAL,
    params(NoFilterQuery),
    responses(
        (status = 200, description = "List of networks", body = inline(PaginatedApiResponse<Network>)),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_networks(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    query: Query<NoFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Network>>> {
    let mut response = get_all_handler::<Network>(State(state.clone()), auth, query).await?;

    // Hydrate credential_ids from junction table
    let network_ids: Vec<Uuid> = response.data.iter().map(|n| n.id).collect();
    let cred_map = state
        .services
        .credential_service
        .get_credential_ids_for_networks(&network_ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    for network in &mut response.data {
        if let Some(creds) = cred_map.get(&network.id) {
            network.base.credential_ids = creds.clone();
        }
    }

    Ok(response)
}

/// Get a network by ID
#[utoipa::path(
    get,
    path = "/{id}",
    tag = Network::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Network ID")),
    responses(
        (status = 200, description = "Network found", body = ApiResponse<Network>),
        (status = 404, description = "Network not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_by_id_network(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<Network>>> {
    let mut response = crate::server::shared::handlers::traits::get_by_id_handler::<Network>(
        State(state.clone()),
        auth,
        Path(id),
    )
    .await?;

    // Hydrate credential_ids from junction table
    if let Some(ref mut network) = response.data {
        network.base.credential_ids = state
            .services
            .credential_service
            .get_credential_ids_for_network(&id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    Ok(response)
}

/// Create a new network
#[utoipa::path(
    post,
    path = "",
    tag = Network::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "Network created", body = ApiResponse<Network>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_network(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    RequireFeature { .. }: RequireFeature<CreateNetworkFeature>,
    Json(network): Json<Network>,
) -> ApiResult<Json<ApiResponse<Network>>> {
    let entity = auth.entity.clone();
    let organization_id = auth.organization_id();

    let response = create_handler::<Network>(
        State(state.clone()),
        auth.into_permission::<Member>(),
        Json(network),
    )
    .await?;

    if let Some(network) = &response.data {
        // Sync credentials to junction table
        state
            .services
            .credential_service
            .set_network_credentials(&network.id, &network.base.credential_ids)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        let service = Network::get_service(&state);
        service
            .create_organizational_subnets(network.id, entity.clone())
            .await?;

        let topology = Topology::new(TopologyBase::new(network.base.name.clone(), network.id));

        state
            .services
            .topology_service
            .create(topology, entity.clone())
            .await?;

        // Emit SecondNetworkCreated telemetry event
        if let Some(organization_id) = organization_id {
            let organization = state
                .services
                .organization_service
                .get_by_id(&organization_id)
                .await?;

            if let Some(organization) = organization {
                // Check for SecondNetworkCreated (if first is already onboarded but second is not)
                if organization.not_onboarded(&OnboardingOperation::SecondNetworkCreated) {
                    // Count networks to confirm this is actually the second+
                    let network_filter =
                        StorableFilter::<Network>::new_from_org_id(&organization_id);
                    let networks = service.get_all(network_filter).await.unwrap_or_default();
                    let network_count = networks.len();

                    if network_count >= 2 {
                        service
                            .event_bus()
                            .publish_onboarding(OnboardingEvent {
                                id: Uuid::new_v4(),
                                organization_id,
                                operation: OnboardingOperation::SecondNetworkCreated,
                                timestamp: Utc::now(),
                                metadata: serde_json::json!({
                                    "network_id": network.id,
                                    "network_name": network.base.name,
                                    "total_networks": network_count
                                }),
                                authentication: entity,
                            })
                            .await?;
                    }
                }
            }
        }
    }

    Ok(response)
}

/// Update a network
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Network::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Network ID")),
    request_body = Network,
    responses(
        (status = 200, description = "Network updated", body = ApiResponse<Network>),
        (status = 404, description = "Network not found", body = ApiErrorResponse),
        (status = 403, description = "User not admin", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_network(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    Path(id): Path<Uuid>,
    Json(network): Json<Network>,
) -> ApiResult<Json<ApiResponse<Network>>> {
    let credential_ids = network.base.credential_ids.clone();

    let mut response = update_handler::<Network>(
        State(state.clone()),
        auth.into_permission::<Member>(),
        Path(id),
        Json(network),
    )
    .await?;

    // Sync credentials to junction table
    state
        .services
        .credential_service
        .set_network_credentials(&id, &credential_ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Hydrate credential_ids on the response
    if let Some(ref mut network) = response.data {
        network.base.credential_ids = credential_ids;
    }

    Ok(response)
}

/// Delete a network
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Network::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Network ID")),
    responses(
        (status = 200, description = "Network deleted", body = EmptyApiResponse),
        (status = 404, description = "Network not found", body = ApiErrorResponse),
        (status = 403, description = "User not admin", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn delete_network(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    path: Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let service = Network::get_service(&state);
    let network_filter = StorableFilter::<Network>::new_from_org_id(&organization_id);
    let network_count = service
        .get_all(network_filter)
        .await
        .unwrap_or_default()
        .len();

    if network_count <= 1 {
        return Err(ApiError::entity_delete_forbidden::<Network>(Some(
            "Organizations must have at least one network.",
        )));
    }

    delete_handler::<Network>(State(state), auth.into_permission::<Member>(), path).await
}

/// Bulk delete networks
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = Network::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "Array of network IDs to delete"),
    responses(
        (status = 200, description = "Networks deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 403, description = "User not admin", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_delete_networks(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    json: Json<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let service = Network::get_service(&state);
    let network_filter = StorableFilter::<Network>::new_from_org_id(&organization_id);
    let network_count = service
        .get_all(network_filter)
        .await
        .unwrap_or_default()
        .len();

    if json.len() >= network_count {
        return Err(ApiError::entity_delete_forbidden::<Network>(Some(
            "Organizations must have at least one network.",
        )));
    }

    bulk_delete_handler::<Network>(State(state), auth.into_permission::<Member>(), json).await
}
