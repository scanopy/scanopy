use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::list::list_page;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, ListScope, bulk_delete_handler, create_handler,
    delete_handler, update_handler,
};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::PaginatedApiResponse;
use crate::server::{
    auth::middleware::{
        features::{CreateSiteFeature, RequireFeature},
        permissions::{Authorized, Can, Delete, Read, Write},
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
    sites::r#impl::{DEFAULT_STALE_AFTER_HOURS, Site},
};
use axum::extract::{Path, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Site Ordering
// ============================================================================

/// The staleness window a site judges by, its own or the server default, in hours.
static STALE_AFTER_HOURS_SQL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    format!("COALESCE(sites.stale_after_hours, {DEFAULT_STALE_AFTER_HOURS})")
});

/// Fields the sites list can be ordered and grouped by.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum SiteOrderField {
    #[default]
    CreatedAt,
    Name,
    UpdatedAt,
    /// The staleness window the site judges by, with the server default applied.
    EffectiveStaleAfterHours,
}

impl OrderField for SiteOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "sites.created_at",
            Self::Name => "sites.name",
            Self::UpdatedAt => "sites.updated_at",
            Self::EffectiveStaleAfterHours => STALE_AFTER_HOURS_SQL.as_str(),
        }
    }
}

// ============================================================================
// Site List Query
// ============================================================================

/// Query parameters for filtering and ordering the sites list.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct SiteListQuery {
    /// Only sites with one of these credentials assigned. Repeat for several.
    pub credential_ids: Option<Vec<Uuid>>,
    /// Only sites whose effective staleness window is one of these, in hours. Repeat for
    /// several.
    pub effective_stale_after_hours: Option<Vec<i64>>,
    /// Filter by tag IDs (returns sites that have ANY of the specified tags)
    pub tag_ids: Option<Vec<Uuid>>,
    /// Tag sets the site must hold one tag from each of. Each entry is a comma-separated list of
    /// tag ids. One entry per tag group the list is filtered by.
    pub tag_sets: Option<Vec<String>>,
    /// Free-text search. Case-insensitive substring match against the site's name and the names
    /// of its daemons, subnets and VLANs.
    pub search: Option<String>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<SiteOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<SiteOrderField>,
    /// Group by the tag each row holds from this tag group. Takes the place of `group_by`.
    pub group_by_tag_group: Option<Uuid>,
    /// Sort by the position of the tag each row holds from this tag group. Takes the place of
    /// `order_by`.
    pub order_by_tag_group: Option<Uuid>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl FilterQueryExtractor for SiteListQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        _user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        match &self.effective_stale_after_hours {
            Some(hours) if !hours.is_empty() => filter.expr_text_in(
                STALE_AFTER_HOURS_SQL.as_str(),
                &hours.iter().map(i64::to_string).collect::<Vec<_>>(),
            ),
            _ => filter,
        }
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    // get_all and get_by_id are now custom (below) to hydrate credential_ids
    crate::crud_export_csv_handler!(Site);
    crate::crud_get_field_values_handler!(Site);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_sites, create_site))
        .routes(routes!(get_by_id_site, update_site, delete_site))
        .routes(routes!(bulk_delete_sites))
        .routes(routes!(generated::export_csv))
        .routes(routes!(generated::get_field_values))
}

/// List sites
#[utoipa::path(
    get,
    operation_id = "list_sites",
    path = "",
    tag = Site::ENTITY_NAME_PLURAL,
    params(SiteListQuery),
    responses(
        (status = 200, description = "List of sites", body = inline(PaginatedApiResponse<Site>)),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_sites(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Site, Read>>,
    Query(query): Query<SiteListQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Site>>> {
    // Sites are scoped to the caller's own sites by `StorableFilter::new_for_access`.
    let scope = ListScope::of(&auth)?;
    Ok(Json(list_page::<Site>(&state, &scope, &query).await?))
}

/// Get site by ID
#[utoipa::path(
    get,
    operation_id = "get_site_by_id",
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
    auth: Authorized<Can<Site, Read>>,
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
        site.base.credential_ids = Some(
            state
                .services
                .credential_service
                .get_credential_ids_for_site(&id)
                .await
                .map_err(|e| ApiError::internal_error(&e.to_string()))?,
        );
    }

    Ok(response)
}

/// Create site
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
    auth: Authorized<Can<Site, Write>>,
    RequireFeature { .. }: RequireFeature<CreateSiteFeature>,
    ApiJson(site): ApiJson<Site>,
) -> ApiResult<Json<ApiResponse<Site>>> {
    let entity = auth.entity.clone();
    let organization_id = auth.organization_id();

    // Validate referenced credentials belong to the caller's org before any
    // writes — the credential_ids are caller-supplied and the site service syncs
    // them to the site→credential junction (which is later read to hand
    // credential secrets to daemons), so an unchecked foreign id would leak
    // another tenant's secret. Omitted, the service assigns the site-wide defaults.
    let org_id = organization_id.ok_or_else(ApiError::organization_required)?;
    if let Some(credential_ids) = &site.base.credential_ids {
        state
            .services
            .credential_service
            .validate_ids_in_org(credential_ids, org_id)
            .await?;
    }

    let response = create_handler::<Site>(State(state.clone()), auth, ApiJson(site)).await?;

    if let Some(site) = response.data() {
        let service = Site::get_service(&state);

        // A key reads sites only from its own list, so without this it could not reach the
        // site it just created.
        if let AuthenticatedEntity::ApiKey { api_key_id, .. } = &entity {
            state
                .services
                .user_api_key_service
                .add_site_access(api_key_id, &site.id)
                .await
                .map_err(|e| ApiError::internal_error(&e.to_string()))?;
        }

        // Emit SecondSiteCreated telemetry event
        if let Some(organization_id) = organization_id {
            // Count sites to confirm this is actually the second+
            let site_filter = StorableFilter::<Site>::new_from_org_id(&organization_id);
            let sites = service.get_all(site_filter).await.unwrap_or_default();
            let site_count = sites.len();

            if site_count >= 2
                && state
                    .services
                    .organization_service
                    .record_milestone(
                        organization_id,
                        OnboardingOperationDiscriminants::SecondSiteCreated,
                    )
                    .await?
            {
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
                    .await;
            }
        }
    }

    Ok(response)
}

/// Update site
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
    auth: Authorized<Can<Site, Write>>,
    Path(id): Path<Uuid>,
    ApiJson(site): ApiJson<Site>,
) -> ApiResult<Json<ApiResponse<Site>>> {
    let credential_ids = site.base.credential_ids.clone();

    // Validate referenced credentials belong to the caller's org before syncing
    // them to the site→credential junction (see create_site). Omitted, the
    // assignments stay as they are.
    let org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    if let Some(credential_ids) = &credential_ids {
        state
            .services
            .credential_service
            .validate_ids_in_org(credential_ids, org_id)
            .await?;
    }

    let mut response =
        update_handler::<Site>(State(state.clone()), auth, Path(id), ApiJson(site)).await?;

    let credential_ids = match credential_ids {
        Some(credential_ids) => {
            state
                .services
                .credential_service
                .set_site_credentials(&id, &credential_ids)
                .await
                .map_err(|e| ApiError::internal_error(&e.to_string()))?;
            credential_ids
        }
        None => state
            .services
            .credential_service
            .get_credential_ids_for_site(&id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?,
    };

    // Hydrate credential_ids on the response
    if let Some(site) = response.data_mut() {
        site.base.credential_ids = Some(credential_ids);
    }

    Ok(response)
}

/// Delete site
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
    auth: Authorized<Can<Site, Delete>>,
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

    delete_handler::<Site>(State(state), auth, path).await
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
    auth: Authorized<Can<Site, Delete>>,
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

    bulk_delete_handler::<Site>(State(state), auth, json).await
}
