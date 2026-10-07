use crate::server::shared::extractors::Query;
use crate::server::{
    auth::middleware::permissions::{Authorized, Member, Viewer},
    config::AppState,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, Entity as EntityEnum},
        handlers::{ordering::OrderField, query::FilterQueryExtractor},
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, traits::Entity},
        types::api::{ApiError, ApiJson, ApiResponse, ApiResult, GroupCount, PaginatedApiResponse},
        types::entities::EntitySource,
        validation::{
            validate_bulk_delete_access, validate_create_access, validate_delete_access,
            validate_entity, validate_read_access, validate_update_access,
        },
    },
};
use async_trait::async_trait;
use axum::{
    Router,
    extract::{Path, State},
    response::Json,
    routing::{delete, get, post, put},
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// Trait for creating standard CRUD handlers for an entity
#[async_trait]
pub trait CrudHandlers:
    Entity + Serialize + for<'de> Deserialize<'de> + validator::Validate
where
    Self: Display + ChangeTriggersTopologyStaleness<Self> + Default,
    EntityEnum: From<Self>,
{
    /// Get the service from AppState (must implement CrudService)
    type Service: CrudService<Self> + Send + Sync;
    fn get_service(state: &AppState) -> &Self::Service;

    /// Query type for filtering in get_all requests.
    /// Use `SiteFilterQuery` for site-keyed entities,
    /// `OrganizationFilterQuery` for organization-keyed entities.
    type FilterQuery: FilterQueryExtractor;

    /// The fields `get_all` can sort and group by, and `field-values` can count.
    /// Use `NoOrderField` for entities without server-side ordering.
    type OrderField: OrderField + DeserializeOwned + ToSchema;

    /// The order global search lists this entity's matches in. `None` lists them in creation
    /// order.
    fn search_order() -> Option<Self::OrderField> {
        None
    }

    /// Get entity name for error messages (e.g., "Group", "Site")
    fn entity_name() -> &'static str {
        Self::table_name()
    }

    /// Validate entity before create/update (uses validator crate by default)
    fn validate(&self) -> Result<(), String> {
        validator::Validate::validate(self).map_err(|e| e.to_string())
    }
}

/// Create a standard CRUD router
pub fn create_crud_router<T>() -> Router<Arc<AppState>>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    Router::new()
        .route("/", post(create_handler::<T>))
        .route("/", get(get_all_handler::<T>))
        .route("/{id}", put(update_handler::<T>))
        .route("/{id}", delete(delete_handler::<T>))
        .route("/{id}", get(get_by_id_handler::<T>))
        .route("/bulk-delete", post(bulk_delete_handler::<T>))
}

pub async fn create_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    ApiJson(mut entity): ApiJson<T>,
) -> ApiResult<Json<ApiResponse<T>>>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    // Set source to Manual for user-created entities
    entity.set_source(EntitySource::Manual);

    validate_entity(|| CrudHandlers::validate(&entity), T::entity_name())?;

    let service = T::get_service(&state);
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id();

    validate_create_access(
        service.get_site_id(&entity),
        service.get_organization_id(&entity),
        &site_ids,
        organization_id,
    )?;

    let created = service
        .create(entity, auth.into_entity())
        .await
        .map_err(|e| {
            // Use From<anyhow::Error> to properly handle ValidationError (400) vs internal errors (500)
            let api_error = ApiError::from(e);
            if api_error.status.is_server_error() {
                tracing::error!(
                    entity_type = T::table_name(),
                    user_id = ?user_id,
                    error = %api_error.message,
                    "Failed to create entity"
                );
            }
            api_error
        })?;

    Ok(Json(ApiResponse::success(created)))
}

/// The filter every list-shaped read of `T` starts from: the caller's site or organization
/// scope, the SCD2 live/as-of narrowing, and the entity's own query filters.
///
/// Shared by `get_all_handler` and `get_field_values_handler`, so the values offered for a field
/// can never come from rows the list itself would not show.
pub fn base_list_filter<T>(
    site_ids: &[Uuid],
    organization_id: Uuid,
    query: &T::FilterQuery,
) -> StorableFilter<T>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let mut base_filter = StorableFilter::<T>::new_for_access(site_ids, &organization_id);

    // SCD2 entities: hide closed historical copies from frontend-facing GETs.
    // When the query carries an `at` timestamp (snapshot view), read as-of that
    // instant instead of live.
    if T::HAS_SCD2 {
        base_filter = base_filter.live_or_as_of(query.at());
    }

    // Apply entity-specific filters
    query.apply_to_filter(base_filter, site_ids, organization_id)
}

/// Every distinct value of one order field across the rows the caller can list, with how many
/// rows hold each. Feeds a filter's options for a free-valued column, which a paginated list
/// cannot collect from the page it has loaded.
pub async fn get_field_values_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(field): Path<T::OrderField>,
    Query(query): Query<T::FilterQuery>,
) -> ApiResult<Json<ApiResponse<Vec<GroupCount>>>>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let filter = field_values_filter::<T>(&site_ids, organization_id, &query, field);

    let counts = T::get_service(&state)
        .count_by_group(filter, field.to_sql())
        .await?;

    Ok(Json(ApiResponse::success(counts)))
}

/// The filter `get_field_values_handler` counts under: the list's base filter plus the JOIN the
/// field's expression reads from.
pub fn field_values_filter<T>(
    site_ids: &[Uuid],
    organization_id: Uuid,
    query: &T::FilterQuery,
    field: T::OrderField,
) -> StorableFilter<T>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let filter = base_list_filter::<T>(site_ids, organization_id, query);
    match field.join_sql() {
        Some(join) => filter.join(join),
        None => filter,
    }
}

pub async fn get_all_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<T::FilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<T>>>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id();

    let filter = base_list_filter::<T>(&site_ids, organization_id, &query);

    // Apply pagination
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    let service = T::get_service(&state);

    // Use paginated query to get items and total count
    let result = service.get_paginated(filter).await.map_err(|e| {
        tracing::error!(
            entity_type = T::table_name(),
            user_id = ?user_id,
            error = %e,
            "Failed to fetch entities"
        );
        ApiError::internal_error(&e.to_string())
    })?;

    // Get effective pagination values for response metadata
    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    // Return paginated response with metadata
    Ok(Json(PaginatedApiResponse::success(
        result.items,
        result.total_count,
        limit,
        offset,
    )))
}

pub async fn get_by_id_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<T>>>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id();

    let service = T::get_service(&state);
    let entity = service
        .get_by_id(&id)
        .await
        .map_err(|e| {
            tracing::error!(
                entity_type = T::table_name(),
                entity_id = %id,
                user_id = ?user_id,
                error = %e,
                "Failed to fetch entity by ID"
            );
            ApiError::internal_error(&e.to_string())
        })?
        .ok_or_else(|| {
            tracing::warn!(
                entity_type = T::table_name(),
                entity_id = %id,
                user_id = ?user_id,
                "Entity not found"
            );
            ApiError::entity_not_found::<T>(id)
        })?;

    validate_read_access(
        service.get_site_id(&entity),
        service.get_organization_id(&entity),
        &site_ids,
        organization_id,
    )?;

    // SCD2: closed historical copies are not addressable from frontend-facing
    // endpoints. Return 404 to match the live-only model.
    if T::HAS_SCD2 && !entity.is_live_row() {
        return Err(ApiError::entity_not_found::<T>(&id));
    }

    Ok(Json(ApiResponse::success(entity)))
}

pub async fn update_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    ApiJson(mut entity): ApiJson<T>,
) -> ApiResult<Json<ApiResponse<T>>>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id();

    let service = T::get_service(&state);

    // Fetch existing entity and verify ownership BEFORE any updates
    // The path ID is canonical - we use it to find the existing entity
    let existing = service
        .get_by_id(&id)
        .await
        .map_err(|e| {
            tracing::error!(
                entity_type = T::table_name(),
                entity_id = %id,
                user_id = ?user_id,
                error = %e,
                "Failed to fetch entity for update"
            );
            ApiError::internal_error(&e.to_string())
        })?
        .ok_or_else(|| {
            tracing::warn!(
                entity_type = T::table_name(),
                entity_id = %id,
                user_id = ?user_id,
                "Entity not found for update"
            );
            ApiError::entity_not_found::<T>(id)
        })?;

    // Preserve immutable fields from existing entity.
    // These fields cannot be changed via the API - the existing values are authoritative.
    // This includes: id, created_at (common to all entities), plus any entity-specific
    // immutable fields handled by preserve_immutable_fields (e.g., ApiKey.key, Daemon.url).
    entity.set_id(existing.id());
    entity.set_created_at(existing.created_at());
    entity.preserve_immutable_fields(&existing);

    // Validate entity (e.g., name length limits)
    validate_entity(|| CrudHandlers::validate(&entity), T::entity_name())?;

    validate_update_access(
        service.get_site_id(&existing),
        service.get_organization_id(&existing),
        service.get_site_id(&entity),
        service.get_organization_id(&entity),
        &site_ids,
        organization_id,
    )?;

    let updated = service
        .update(&mut entity, auth.into_entity())
        .await
        .map_err(|e| {
            // Use From<anyhow::Error> to properly handle ValidationError (400) vs internal errors (500)
            let api_error = ApiError::from(e);
            if api_error.status.is_server_error() {
                tracing::error!(
                    entity_type = T::table_name(),
                    entity_id = %id,
                    user_id = ?user_id,
                    error = %api_error.message,
                    "Failed to update entity"
                );
            }
            api_error
        })?;

    Ok(Json(ApiResponse::success(updated)))
}

pub async fn delete_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let service = T::get_service(&state);

    // Fetch entity first to verify ownership
    let entity = service
        .get_by_id(&id)
        .await
        .map_err(|e| {
            tracing::error!(
                entity_type = T::table_name(),
                entity_id = %id,
                error = %e,
                "Failed to fetch entity for deletion"
            );
            ApiError::internal_error(&e.to_string())
        })?
        .ok_or_else(|| {
            tracing::warn!(
                entity_type = T::table_name(),
                entity_id = %id,
                "Entity not found for deletion"
            );
            ApiError::entity_not_found::<T>(id)
        })?;

    validate_delete_access(
        service.get_site_id(&entity),
        service.get_organization_id(&entity),
        &site_ids,
        organization_id,
    )?;

    service.delete(&id, auth.into_entity()).await.map_err(|e| {
        // Use From<anyhow::Error> to properly handle ValidationError (400) vs internal errors (500)
        let api_error = ApiError::from(e);
        if api_error.status.is_server_error() {
            tracing::error!(
                entity_type = T::table_name(),
                entity_id = %id,
                error = %api_error.message,
                "Failed to delete entity"
            );
        }
        api_error
    })?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn bulk_delete_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>>
where
    T: CrudHandlers + 'static,
    EntityEnum: From<T>,
{
    if ids.is_empty() {
        return Err(ApiError::bulk_empty());
    }

    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id();

    let service = T::get_service(&state);

    // Fetch all entities by the requested IDs
    let entity_filter = StorableFilter::<T>::new_from_entity_ids(&ids);
    let entities = service.get_all(entity_filter).await?;

    // Verify we found all requested entities
    if entities.len() != ids.len() {
        let found_ids: Vec<Uuid> = entities.iter().map(|e| e.id()).collect();
        let missing: Vec<&Uuid> = ids.iter().filter(|id| !found_ids.contains(id)).collect();
        tracing::warn!(
            entity_type = T::table_name(),
            user_id = ?user_id,
            missing_ids = ?missing,
            "Bulk delete requested non-existent entities"
        );
    }

    // Verify ownership of ALL entities before deleting any
    for entity in &entities {
        validate_bulk_delete_access(
            service.get_site_id(entity),
            service.get_organization_id(entity),
            &site_ids,
            organization_id,
        )?;
    }

    // Only delete entities that actually exist and user has access to
    let valid_ids: Vec<Uuid> = entities.iter().map(|e| e.id()).collect();

    let deleted_count = service
        .delete_many(&valid_ids, auth.into_entity())
        .await
        .map_err(|e| {
            // Use From<anyhow::Error> to properly handle ValidationError (400) vs internal errors (500)
            let api_error = ApiError::from(e);
            if api_error.status.is_server_error() {
                tracing::error!(
                    entity_type = T::table_name(),
                    user_id = ?user_id,
                    error = %api_error.message,
                    "Failed to bulk delete entities"
                );
            }
            api_error
        })?;

    Ok(Json(ApiResponse::success(BulkDeleteResponse {
        deleted_count,
        requested_count: ids.len(),
    })))
}

#[derive(Serialize, ToSchema)]
pub struct BulkDeleteResponse {
    /// How many records were actually deleted.
    pub deleted_count: usize,
    /// How many IDs the request asked to delete.
    pub requested_count: usize,
}
