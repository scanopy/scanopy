use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member, Viewer};
use crate::server::bindings::r#impl::base::Binding;
use crate::server::config::AppState;
use crate::server::groups::r#impl::base::Group;
use crate::server::shared::events::types::{TelemetryEvent, TelemetryOperation};
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
use chrono::Utc;
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};

// ============================================================================
// Group Ordering
// ============================================================================

/// Fields that groups can be ordered/grouped by.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GroupOrderField {
    #[default]
    CreatedAt,
    Name,
    GroupType,
    UpdatedAt,
    NetworkId,
}

impl OrderField for GroupOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "groups.created_at",
            Self::Name => "groups.name",
            Self::GroupType => "groups.group_type",
            Self::UpdatedAt => "groups.updated_at",
            Self::NetworkId => "groups.network_id",
        }
    }
}

// ============================================================================
// Group Filter Query
// ============================================================================

/// Query parameters for filtering and ordering groups.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct GroupFilterQuery {
    /// Filter by network ID
    pub network_id: Option<Uuid>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<GroupOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<GroupOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl GroupFilterQuery {
    /// Build the ORDER BY clause.
    pub fn apply_ordering(&self, filter: StorableFilter<Group>) -> (StorableFilter<Group>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "groups.created_at ASC",
        )
    }
}

impl FilterQueryExtractor for GroupFilterQuery {
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
    crate::crud_get_by_id_handler!(Group);
    crate::crud_delete_handler!(Group);
    crate::crud_bulk_delete_handler!(Group);
    crate::crud_export_csv_handler!(Group);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_groups, create_group))
        .routes(routes!(
            generated::get_by_id,
            update_group,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
}

/// List all Groups
///
/// Returns all groups the authenticated user has access to.
/// Supports pagination via `limit` and `offset` query parameters,
/// and ordering via `group_by`, `order_by`, and `order_direction`.
#[utoipa::path(
    get,
    path = "",
    tag = Group::ENTITY_NAME_PLURAL,
    params(GroupFilterQuery),
    responses(
        (status = 200, description = "List of groups", body = PaginatedApiResponse<Group>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_groups(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    crate::server::shared::extractors::Query(query): crate::server::shared::extractors::Query<
        GroupFilterQuery,
    >,
) -> ApiResult<Json<PaginatedApiResponse<Group>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let base_filter = StorableFilter::<Group>::new_from_network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);

    // Apply pagination
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    // Apply ordering
    let (filter, order_by) = query.apply_ordering(filter);

    let result = state
        .services
        .group_service
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

/// Create a new Group
#[utoipa::path(
    post,
    path = "",
    tag = Group::ENTITY_NAME_PLURAL,
    request_body = Group,
    responses(
        (status = 200, description = "Group created successfully", body = ApiResponse<Group>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_group(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(group): Json<Group>,
) -> ApiResult<Json<ApiResponse<Group>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;
    let entity = auth.entity.clone();

    // Custom validation: Check for service bindings on different networks
    for binding_id in &group.base.binding_ids {
        let binding_id_filter = StorableFilter::<Binding>::new_from_entity_id(binding_id);

        if let Some(binding) = state
            .services
            .binding_service
            .get_one(binding_id_filter)
            .await?
            && binding.base.network_id != group.base.network_id
        {
            return Err(ApiError::bad_request(&format!(
                "Group is on network {}, can't add binding which is on network {}",
                group.base.network_id, binding.base.network_id
            )));
        }
    }

    // Delegate to generic handler (handles validation, auth checks, creation)
    let response = create_handler::<Group>(State(state.clone()), auth, Json(group)).await?;

    // Emit FirstGroupCreated telemetry event if this is the first group
    if response.data.is_some() {
        let organization = state
            .services
            .organization_service
            .get_by_id(&organization_id)
            .await?;

        if let Some(organization) = organization
            && organization.not_onboarded(&TelemetryOperation::FirstGroupCreated)
        {
            state
                .services
                .group_service
                .event_bus()
                .publish_telemetry(TelemetryEvent {
                    id: Uuid::new_v4(),
                    organization_id,
                    operation: TelemetryOperation::FirstGroupCreated,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({}),
                    authentication: entity,
                })
                .await?;
        }
    }

    Ok(response)
}

/// Update a Group
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Group::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Group ID")),
    request_body = Group,
    responses(
        (status = 200, description = "Group updated successfully", body = ApiResponse<Group>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
        (status = 404, description = "Group not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_group(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
    Json(group): Json<Group>,
) -> ApiResult<Json<ApiResponse<Group>>> {
    // Custom validation: Check for service bindings on different networks
    for binding_id in &group.base.binding_ids {
        let binding_id_filter = StorableFilter::<Binding>::new_from_entity_id(binding_id);

        if let Some(binding) = state
            .services
            .binding_service
            .get_one(binding_id_filter)
            .await?
            && binding.base.network_id != group.base.network_id
        {
            return Err(ApiError::bad_request(&format!(
                "Group is on network {}, can't add binding which is on network {}",
                group.base.network_id, binding.base.network_id
            )));
        }
    }

    // Delegate to generic handler (handles validation, auth checks, update)
    update_handler::<Group>(State(state), auth, path, Json(group)).await
}
