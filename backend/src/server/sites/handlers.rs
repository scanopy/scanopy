use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::query::NoFilterQuery;
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, bulk_delete_handler, create_handler, delete_handler,
    get_all_handler, update_handler,
};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::PaginatedApiResponse;
use crate::server::{
    auth::middleware::{
        features::{CreateSiteFeature, RequireFeature},
        permissions::{Admin, Authorized, Member, Viewer},
    },
    shared::{
        events::{
            traits::{Event, OrgScope},
            types::{OnboardingOperation, OnboardingOperationDiscriminants},
        },
        types::api::{ApiError, ApiErrorResponse, EmptyApiResponse},
    },
};
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult},
    sites::r#impl::Site,
};
use axum::extract::{Path, State};
use axum::response::Json;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    // get_all and get_by_id are now custom (below) to hydrate credential_ids
    crate::crud_export_csv_handler!(Site);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_sites, create_site))
        .routes(routes!(get_by_id_site, update_site, delete_site))
        .routes(routes!(bulk_delete_sites))
        .routes(routes!(generated::export_csv))
}

/// List all sites
#[utoipa::path(
    get,
    path = "",
    tag = Site::ENTITY_NAME_PLURAL,
    params(NoFilterQuery),
    responses(
        (status = 200, description = "List of sites", body = inline(PaginatedApiResponse<Site>)),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_sites(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    query: Query<NoFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Site>>> {
    let mut response = get_all_handler::<Site>(State(state.clone()), auth, query).await?;

    // Hydrate credential_ids from junction table
    let site_ids: Vec<Uuid> = response.data.iter().map(|n| n.id).collect();
    let cred_map = state
        .services
        .credential_service
        .get_credential_ids_for_sites(&site_ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    for site in &mut response.data {
        if let Some(creds) = cred_map.get(&site.id) {
            site.base.credential_ids = creds.clone();
        }
    }

    Ok(response)
}

/// Get a site by ID
#[utoipa::path(
    get,
    path = "/{id}",
    tag = Site::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Site ID")),
    responses(
        (status = 200, description = "Site found", body = ApiResponse<Site>),
        (status = 404, description = "Site not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_by_id_site(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<Site>>> {
    let mut response = crate::server::shared::handlers::traits::get_by_id_handler::<Site>(
        State(state.clone()),
        auth,
        Path(id),
    )
    .await?;

    // Hydrate credential_ids from junction table
    if let Some(site) = response.data_mut() {
        site.base.credential_ids = state
            .services
            .credential_service
            .get_credential_ids_for_site(&id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    Ok(response)
}

/// Create a new site
#[utoipa::path(
    post,
    path = "",
    tag = Site::ENTITY_NAME_PLURAL,
    request_body = Site,
    responses(
        (status = 200, description = "Site created", body = ApiResponse<Site>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_site(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    RequireFeature { .. }: RequireFeature<CreateSiteFeature>,
    ApiJson(site): ApiJson<Site>,
) -> ApiResult<Json<ApiResponse<Site>>> {
    let entity = auth.entity.clone();
    let organization_id = auth.organization_id();

    // Validate referenced credentials belong to the caller's org before any
    // writes — the credential_ids are caller-supplied and are synced to the
    // site→credential junction below (which is later read to hand credential
    // secrets to daemons), so an unchecked foreign id would leak another
    // tenant's secret.
    let org_id = organization_id.ok_or_else(ApiError::organization_required)?;
    state
        .services
        .credential_service
        .validate_ids_in_org(&site.base.credential_ids, org_id)
        .await?;

    let response = create_handler::<Site>(
        State(state.clone()),
        auth.into_permission::<Member>(),
        ApiJson(site),
    )
    .await?;

    if let Some(site) = response.data() {
        // Sync credentials to junction table
        state
            .services
            .credential_service
            .set_site_credentials(&site.id, &site.base.credential_ids)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        let service = Site::get_service(&state);

        // System subnets + live-view topology row for this site.
        state
            .services
            .topology_service
            .ensure_site_setup(site.id, entity.clone())
            .await?;

        // Emit SecondSiteCreated telemetry event
        if let Some(organization_id) = organization_id {
            let organization = state
                .services
                .organization_service
                .get_by_id(&organization_id)
                .await?;

            if let Some(organization) = organization {
                // Check for SecondSiteCreated (if first is already onboarded but second is not)
                if organization.not_onboarded(&OnboardingOperationDiscriminants::SecondSiteCreated)
                {
                    // Count sites to confirm this is actually the second+
                    let site_filter = StorableFilter::<Site>::new_from_org_id(&organization_id);
                    let sites = service.get_all(site_filter).await.unwrap_or_default();
                    let site_count = sites.len();

                    if site_count >= 2 {
                        service
                            .event_bus()
                            .publish(Event::new(
                                OrgScope { organization_id },
                                OnboardingOperation::SecondSiteCreated {
                                    site_id: site.id,
                                    site_name: site.base.name.clone(),
                                    total_sites: site_count as u32,
                                },
                                entity,
                            ))
                            .await?;
                    }
                }
            }
        }
    }

    Ok(response)
}

/// Update a site
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Site::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Site ID")),
    request_body = Site,
    responses(
        (status = 200, description = "Site updated", body = ApiResponse<Site>),
        (status = 404, description = "Site not found", body = ApiErrorResponse),
        (status = 403, description = "User not admin", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_site(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    Path(id): Path<Uuid>,
    ApiJson(site): ApiJson<Site>,
) -> ApiResult<Json<ApiResponse<Site>>> {
    let credential_ids = site.base.credential_ids.clone();

    // Validate referenced credentials belong to the caller's org before syncing
    // them to the site→credential junction (see create_site).
    let org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    state
        .services
        .credential_service
        .validate_ids_in_org(&credential_ids, org_id)
        .await?;

    let mut response = update_handler::<Site>(
        State(state.clone()),
        auth.into_permission::<Member>(),
        Path(id),
        ApiJson(site),
    )
    .await?;

    // Sync credentials to junction table
    state
        .services
        .credential_service
        .set_site_credentials(&id, &credential_ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Hydrate credential_ids on the response
    if let Some(site) = response.data_mut() {
        site.base.credential_ids = credential_ids;
    }

    Ok(response)
}

/// Delete a site
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Site::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Site ID")),
    responses(
        (status = 200, description = "Site deleted", body = EmptyApiResponse),
        (status = 404, description = "Site not found", body = ApiErrorResponse),
        (status = 403, description = "User not admin", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn delete_site(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    path: Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let service = Site::get_service(&state);
    let site_filter = StorableFilter::<Site>::new_from_org_id(&organization_id);
    let site_count = service.get_all(site_filter).await.unwrap_or_default().len();

    if site_count <= 1 {
        return Err(ApiError::entity_delete_forbidden::<Site>(Some(
            "Organizations must have at least one site.",
        )));
    }

    delete_handler::<Site>(State(state), auth.into_permission::<Member>(), path).await
}

/// Bulk delete sites
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = Site::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "Array of Site IDs to delete"),
    responses(
        (status = 200, description = "Sites deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 403, description = "User not admin", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_delete_sites(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    json: ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let service = Site::get_service(&state);
    let site_filter = StorableFilter::<Site>::new_from_org_id(&organization_id);
    let site_count = service.get_all(site_filter).await.unwrap_or_default().len();

    if json.0.len() >= site_count {
        return Err(ApiError::entity_delete_forbidden::<Site>(Some(
            "Organizations must have at least one site.",
        )));
    }

    bulk_delete_handler::<Site>(State(state), auth.into_permission::<Member>(), json).await
}
