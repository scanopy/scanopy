use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member, Viewer};
use crate::server::config::AppState;
use crate::server::dependencies::r#impl::base::{Dependency, DependencyMembers};
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::{OnboardingOperation, OnboardingOperationDiscriminants};
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::{create_handler, update_handler};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, PaginatedApiResponse,
};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};

// ============================================================================
// Dependency Ordering
// ============================================================================

/// Fields that dependencies can be ordered/grouped by.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DependencyOrderField {
    #[default]
    CreatedAt,
    Name,
    DependencyType,
    UpdatedAt,
    NetworkId,
}

impl OrderField for DependencyOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "dependencies.created_at",
            Self::Name => "dependencies.name",
            Self::DependencyType => "dependencies.dependency_type",
            Self::UpdatedAt => "dependencies.updated_at",
            Self::NetworkId => "dependencies.network_id",
        }
    }
}

// ============================================================================
// Dependency Filter Query
// ============================================================================

/// Query parameters for filtering and ordering dependencies.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct DependencyFilterQuery {
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<DependencyOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<DependencyOrderField>,
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
}

impl DependencyFilterQuery {
    /// Build the ORDER BY clause.
    pub fn apply_ordering(
        &self,
        filter: StorableFilter<Dependency>,
    ) -> (StorableFilter<Dependency>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "dependencies.created_at ASC",
        )
    }
}

impl FilterQueryExtractor for DependencyFilterQuery {
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

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Dependency);
    crate::crud_delete_handler!(Dependency);
    crate::crud_bulk_delete_handler!(Dependency);
    crate::crud_export_csv_handler!(Dependency);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_dependencies, create_dependency))
        .routes(routes!(
            generated::get_by_id,
            update_dependency,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
}

/// List all Dependencies
///
/// Returns all dependencies the authenticated user has access to.
/// Supports pagination via `limit` and `offset` query parameters,
/// and ordering via `group_by`, `order_by`, and `order_direction`.
#[utoipa::path(
    get,
    path = "",
    tag = Dependency::ENTITY_NAME_PLURAL,
    params(DependencyFilterQuery),
    responses(
        (status = 200, description = "List of dependencies", body = PaginatedApiResponse<Dependency>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_dependencies(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<DependencyFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Dependency>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let base_filter = StorableFilter::<Dependency>::new_from_network_ids(&network_ids);
    let filter = query
        .apply_to_filter(base_filter, &network_ids, organization_id)
        .live_or_as_of(query.at);

    // Apply pagination
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    // Apply ordering
    let (filter, order_by) = query.apply_ordering(filter);

    let result = state
        .services
        .dependency_service
        .get_paginated_ordered(filter, &order_by)
        .await?;

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

/// Create a new Dependency
#[utoipa::path(
    post,
    path = "",
    tag = Dependency::ENTITY_NAME_PLURAL,
    request_body = Dependency,
    responses(
        (status = 200, description = "Dependency created successfully", body = ApiResponse<Dependency>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_dependency(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(dependency): Json<Dependency>,
) -> ApiResult<Json<ApiResponse<Dependency>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;
    let entity = auth.entity.clone();

    // Validate members based on variant
    validate_dependency_members(&state, &dependency).await?;

    // Delegate to generic handler (handles validation, auth checks, creation)
    let response =
        create_handler::<Dependency>(State(state.clone()), auth, Json(dependency)).await?;

    // Emit FirstDependencyCreated telemetry event if this is the first dependency
    if response.data().is_some() {
        let organization = state
            .services
            .organization_service
            .get_by_id(&organization_id)
            .await?;

        if let Some(organization) = organization
            && organization.not_onboarded(&OnboardingOperationDiscriminants::FirstDependencyCreated)
        {
            state
                .services
                .dependency_service
                .event_bus()
                .publish(Event::new(
                    OrgScope { organization_id },
                    OnboardingOperation::FirstDependencyCreated,
                    entity,
                ))
                .await?;
        }
    }

    Ok(response)
}

/// Update a Dependency
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Dependency::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Dependency ID")),
    request_body = Dependency,
    responses(
        (status = 200, description = "Dependency updated successfully", body = ApiResponse<Dependency>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
        (status = 404, description = "Dependency not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_dependency(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
    Json(dependency): Json<Dependency>,
) -> ApiResult<Json<ApiResponse<Dependency>>> {
    validate_dependency_members(&state, &dependency).await?;
    update_handler::<Dependency>(State(state), auth, path, Json(dependency)).await
}

/// Validate dependency members based on the variant.
async fn validate_dependency_members(state: &AppState, dependency: &Dependency) -> ApiResult<()> {
    match &dependency.base.members {
        DependencyMembers::Services { service_ids } => {
            for service_id in service_ids {
                let service = state
                    .services
                    .service_service
                    .get_by_id(service_id)
                    .await?
                    .ok_or_else(|| {
                        ApiError::bad_request(&format!("Service {} not found", service_id))
                    })?;
                if service.base.network_id != dependency.base.network_id {
                    return Err(ApiError::bad_request(&format!(
                        "Dependency is on network {}, can't add service on network {}",
                        dependency.base.network_id, service.base.network_id
                    )));
                }
            }
        }
        DependencyMembers::Bindings { binding_ids } => {
            for binding_id in binding_ids {
                let binding_filter =
                    StorableFilter::<crate::server::bindings::r#impl::base::Binding>::new_from_entity_id(binding_id);
                let binding = state
                    .services
                    .binding_service
                    .get_unique(binding_filter)
                    .await?
                    .at_most_one()?
                    .ok_or_else(|| {
                        ApiError::bad_request(&format!("Binding {} not found", binding_id))
                    })?;
                if binding.base.network_id != dependency.base.network_id {
                    return Err(ApiError::bad_request(&format!(
                        "Dependency is on network {}, can't add binding on network {}",
                        dependency.base.network_id, binding.base.network_id
                    )));
                }
            }
        }
    }
    Ok(())
}
