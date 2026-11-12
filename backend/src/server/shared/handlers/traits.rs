use crate::server::shared::events::types::{DeletionReason, UpdateReason};
use crate::server::shared::extractors::Query;
use crate::server::{
    auth::access::Grants,
    auth::middleware::{
        auth::AuthenticatedEntity,
        features::{ensure_tagging_allowed, tags_changed},
        permissions::{Authorized, Delete, Proves, Read, Scoped, Write},
    },
    config::AppState,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, Entity as EntityEnum},
        handlers::{ordering::OrderField, query::FilterQueryExtractor},
        services::{
            entity_fields::validate_references,
            traits::{CrudService, EventBusService},
        },
        storage::{filter::StorableFilter, traits::Entity},
        types::api::{
            ApiError, ApiJson, ApiResponse, ApiResult, FieldValues, PaginatedApiResponse,
        },
        types::entities::EntitySource,
        validation::{
            validate_bulk_delete_access, validate_create_access, validate_delete_access,
            validate_entity, validate_read_access, validate_update_access,
        },
    },
};
use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    response::Json,
};
use futures::stream::{self, StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Display;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// Who is reading a list, which decides the rows it may hold: the caller's sites and organization,
/// and for lists of the caller's own things (their API keys) or ruled by their role (the users
/// list), the user and their organization grants.
#[derive(Debug, Clone)]
pub struct ListScope {
    pub site_ids: Vec<Uuid>,
    pub organization_id: Uuid,
    /// The signed-in user, or the user an API key belongs to.
    pub user_id: Option<Uuid>,
    pub organization_grants: Option<Grants>,
}

impl ListScope {
    /// The scope of an authorized caller: the sites where it may read what it asked for. Lists are
    /// organization data, so a caller with no organization is refused.
    pub fn of<P: Scoped>(auth: &Authorized<P>) -> ApiResult<Self> {
        Ok(Self {
            site_ids: auth.site_ids(),
            organization_id: auth
                .organization_id()
                .ok_or_else(ApiError::organization_required)?,
            user_id: auth.user_id(),
            organization_grants: auth.entity.organization_grants(),
        })
    }
}

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
    type OrderField: OrderField + Serialize + DeserializeOwned + ToSchema;

    /// The order global search lists this entity's matches in. `None` lists them in creation
    /// order.
    fn search_order() -> Option<Self::OrderField> {
        None
    }

    /// Every row of this entity the caller may list, before any query narrows it: site-keyed
    /// entities on the caller's sites, the sites themselves narrowed to those, everything else
    /// across the caller's organization. An entity whose rows belong to a user, or are shown by
    /// role, narrows further here, so the list, its CSV export and its filter options all hold
    /// only those rows.
    fn access_filter(scope: &ListScope) -> StorableFilter<Self> {
        StorableFilter::new_for_access(&scope.site_ids, &scope.organization_id)
    }

    /// The rows a list request narrows to, before pagination and ordering.
    ///
    /// Every reader of a list's query builds its filter here, so the CSV export returns the rows
    /// the list does for the same query. Defaults to the access filter plus the query's own
    /// fields. An entity whose list also applies tags, search, staleness or field filters
    /// overrides it, and its list handler calls it too.
    async fn list_filter(
        state: &AppState,
        query: &Self::FilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Self>>
    where
        Self: 'static,
    {
        let _ = state;
        Ok(base_list_filter::<Self>(scope, query))
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

/// [`validate_references`] for a typed entity.
pub(crate) async fn validate_entity_references<T>(
    state: &AppState,
    entity: &T,
    stored: Option<&T>,
    caller: &AuthenticatedEntity,
    organization_id: Uuid,
) -> Result<(), ApiError>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let to_json = |entity: &T| {
        serde_json::to_value(entity).map_err(|e| ApiError::internal_error(&e.to_string()))
    };
    let stored = stored.map(to_json).transpose()?;
    validate_references(
        &state.services,
        T::entity_type(),
        &to_json(entity)?,
        stored.as_ref(),
        &[],
        caller,
        organization_id,
    )
    .await
}

pub async fn create_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<impl Proves<T, Write>>,
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

    // A row that is its own site (a `Site`) isn't on any site until it exists.
    let entity_id = entity.id();
    validate_create_access(
        service.get_site_id(&entity).filter(|id| *id != entity_id),
        service.get_organization_id(&entity),
        &site_ids,
        organization_id,
    )?;

    validate_entity_references::<T>(&state, &entity, None, &auth.entity, organization_id).await?;

    if tags_changed(None, entity.get_tags()) {
        ensure_tagging_allowed(&state, &organization_id).await?;
    }

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
/// Shared by `get_all_handler` and `field_values`, so the values offered for a field can never
/// come from rows the list itself would not show.
pub fn base_list_filter<T>(scope: &ListScope, query: &T::FilterQuery) -> StorableFilter<T>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let mut base_filter = T::access_filter(scope);

    // SCD2 entities: hide closed historical copies from frontend-facing GETs.
    // When the query carries an `at` timestamp (snapshot view), read as-of that
    // instant instead of live.
    if T::HAS_SCD2 {
        base_filter = base_filter.live_or_as_of(query.at());
    }

    // Apply entity-specific filters
    query.apply_to_filter(base_filter, &scope.site_ids, scope.organization_id)
}

/// The fields a field-values request counts. Repeat `fields` to name several.
#[derive(Deserialize, Debug)]
#[serde(bound(deserialize = "F: DeserializeOwned"))]
pub struct FieldValuesQuery<F> {
    #[serde(default)]
    pub fields: Vec<F>,
}

/// How many of one request's fields are counted at once. Each count holds a pool connection.
const FIELD_VALUES_CONCURRENCY: usize = 4;

/// Every distinct value of each named order field across the rows the caller can list, with how
/// many rows hold each. Feeds the filter options of a tab's free-valued columns in one request,
/// which a paginated list cannot collect from the page it has loaded.
pub async fn get_field_values_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<impl Proves<T, Read>>,
    Query(FieldValuesQuery { fields }): Query<FieldValuesQuery<T::OrderField>>,
    Query(query): Query<T::FilterQuery>,
) -> ApiResult<Json<ApiResponse<Vec<FieldValues<T::OrderField>>>>>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let scope = ListScope::of(&auth)?;
    let counts = field_values::<T>(T::get_service(&state), &scope, &query, fields).await?;
    Ok(Json(ApiResponse::success(counts)))
}

/// Each field's distinct values and counts, in the order the fields were named. Every field
/// counts under its own [`field_values_filter`], so one field's JOIN never changes another's
/// counts. A field named twice is counted once.
pub async fn field_values<T>(
    service: &T::Service,
    scope: &ListScope,
    query: &T::FilterQuery,
    mut fields: Vec<T::OrderField>,
) -> Result<Vec<FieldValues<T::OrderField>>, anyhow::Error>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let mut seen = HashSet::new();
    fields.retain(|field| seen.insert(serde_json::to_string(field).ok()));

    stream::iter(fields)
        .map(|field| async move {
            let values = service
                .count_by_group(
                    field_values_filter::<T>(scope, query, field),
                    field.to_sql(),
                )
                .await?;
            Ok(FieldValues { field, values })
        })
        .buffered(FIELD_VALUES_CONCURRENCY)
        .try_collect()
        .await
}

/// The filter a field's values are counted under: the list's base filter plus the JOIN the
/// field's expression reads from.
pub fn field_values_filter<T>(
    scope: &ListScope,
    query: &T::FilterQuery,
    field: T::OrderField,
) -> StorableFilter<T>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let filter = base_list_filter::<T>(scope, query);
    match field.join_sql() {
        Some(join) => filter.join(join),
        None => filter,
    }
}

pub async fn get_all_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<impl Proves<T, Read>>,
    Query(query): Query<T::FilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<T>>>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let scope = ListScope::of(&auth)?;
    let user_id = scope.user_id;

    let filter = T::list_filter(&state, &query, &scope).await?;

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
    auth: Authorized<impl Proves<T, Read>>,
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
    auth: Authorized<impl Proves<T, Write>>,
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

    validate_entity_references::<T>(
        &state,
        &entity,
        Some(&existing),
        &auth.entity,
        organization_id,
    )
    .await?;

    if tags_changed(existing.get_tags(), entity.get_tags()) {
        ensure_tagging_allowed(&state, &organization_id).await?;
    }

    let updated = service
        .update(
            &mut entity,
            auth.into_entity(),
            UpdateReason::EntityRequested,
        )
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
    auth: Authorized<impl Proves<T, Delete>>,
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

    service
        .delete(&id, auth.into_entity(), DeletionReason::EntityRequested)
        .await
        .map_err(|e| {
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
    auth: Authorized<impl Proves<T, Delete>>,
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
        .delete_many(
            &valid_ids,
            auth.into_entity(),
            DeletionReason::EntityRequested,
        )
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
