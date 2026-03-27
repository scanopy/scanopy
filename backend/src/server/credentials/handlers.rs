use crate::server::auth::middleware::permissions::{Admin, Authorized, Viewer};
use crate::server::credentials::r#impl::base::Credential;
use crate::server::credentials::service::CredentialService;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, bulk_delete_handler, create_handler, delete_handler,
    update_handler,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, EmptyApiResponse, PaginatedApiResponse,
};
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult},
};
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

impl CrudHandlers for Credential {
    type Service = CredentialService;
    type FilterQuery = CredentialFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.credential_service
    }
}

// ============================================================================
// Credential Ordering
// ============================================================================

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CredentialOrderField {
    #[default]
    CreatedAt,
    Name,
    UpdatedAt,
}

impl OrderField for CredentialOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "credentials.created_at",
            Self::Name => "credentials.name",
            Self::UpdatedAt => "credentials.updated_at",
        }
    }
}

// ============================================================================
// Credential Filter Query
// ============================================================================

#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct CredentialFilterQuery {
    /// Filter by credential type (e.g. "Snmp", "DockerProxy")
    #[serde(rename = "type")]
    pub credential_type: Option<String>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<CredentialOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<CredentialOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl CredentialFilterQuery {
    pub fn apply_ordering(
        &self,
        filter: StorableFilter<Credential>,
    ) -> (StorableFilter<Credential>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "credentials.created_at ASC",
        )
    }
}

impl FilterQueryExtractor for CredentialFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        mut filter: StorableFilter<T>,
        _user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        if let Some(ref cred_type) = self.credential_type {
            filter = filter.json_field_eq("credential_type", "type", cred_type);
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

// Generated handler for read-only operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Credential);
    crate::crud_export_csv_handler!(Credential);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_credentials, create_credential))
        .routes(routes!(generated::export_csv))
        .routes(routes!(
            generated::get_by_id,
            update_credential,
            delete_credential
        ))
        .routes(routes!(bulk_delete_credentials))
        .routes(routes!(bulk_create_credentials))
}

/// Update Credential
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Credential::ENTITY_NAME_PLURAL,
    params(
        ("id" = Uuid, Path, description = "Credential ID")
    ),
    request_body = Credential,
    responses(
        (status = 200, description = "Credential updated successfully", body = ApiResponse<Credential>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
        (status = 404, description = "Credential not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_credential(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    id: axum::extract::Path<Uuid>,
    entity: Json<Credential>,
) -> ApiResult<Json<ApiResponse<Credential>>> {
    entity
        .base
        .credential_type
        .validate()
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    update_handler::<Credential>(
        state,
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        id,
        entity,
    )
    .await
}

/// Delete Credential
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Credential::ENTITY_NAME_PLURAL,
    params(
        ("id" = Uuid, Path, description = "Credential ID")
    ),
    responses(
        (status = 200, description = "Credential deleted successfully", body = EmptyApiResponse),
        (status = 404, description = "Credential not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_credential(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    id: axum::extract::Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    delete_handler::<Credential>(
        state,
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        id,
    )
    .await
}

/// Bulk delete Credentials
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = Credential::ENTITY_NAME_PLURAL,
    request_body = Vec<Uuid>,
    responses(
        (status = 200, description = "Credentials deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_delete_credentials(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    ids: Json<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    bulk_delete_handler::<Credential>(
        state,
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        ids,
    )
    .await
}

/// List all Credentials
///
/// Returns all credentials in the authenticated user's organization.
/// Optionally filter by type (e.g. ?type=Snmp).
#[utoipa::path(
    get,
    path = "",
    tag = Credential::ENTITY_NAME_PLURAL,
    params(CredentialFilterQuery),
    responses(
        (status = 200, description = "List of credentials", body = PaginatedApiResponse<Credential>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_credentials(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    crate::server::shared::extractors::Query(query): crate::server::shared::extractors::Query<
        CredentialFilterQuery,
    >,
) -> ApiResult<Json<PaginatedApiResponse<Credential>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let base_filter = StorableFilter::<Credential>::new_from_org_id(&organization_id);

    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(base_filter);
    let filter = query.apply_to_filter(filter, &auth.network_ids(), organization_id);
    let (filter, order_by) = query.apply_ordering(filter);

    let result = state
        .services
        .credential_service
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

/// Create a new Credential
///
/// Creates a credential scoped to your organization.
#[utoipa::path(
    post,
    path = "",
    tag = Credential::ENTITY_NAME_PLURAL,
    request_body = Credential,
    responses(
        (status = 200, description = "Credential created successfully", body = ApiResponse<Credential>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_credential(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    Json(credential): Json<Credential>,
) -> ApiResult<Json<ApiResponse<Credential>>> {
    create_handler::<Credential>(
        State(state),
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        Json(credential),
    )
    .await
}

/// Bulk create Credentials
///
/// Creates multiple credentials in one request. Validation is atomic — if any
/// credential has an invalid type, none are created. Individual creates are
/// sequential, so a mid-batch DB error leaves earlier credentials committed.
#[utoipa::path(
    post,
    path = "/bulk",
    tag = Credential::ENTITY_NAME_PLURAL,
    request_body = Vec<Credential>,
    responses(
        (status = 200, description = "Credentials created successfully", body = ApiResponse<Vec<Credential>>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_create_credentials(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    Json(credentials): Json<Vec<Credential>>,
) -> ApiResult<Json<ApiResponse<Vec<Credential>>>> {
    if credentials.is_empty() {
        return Ok(Json(ApiResponse::success(vec![])));
    }

    // Validate all credential types
    for credential in &credentials {
        credential
            .base
            .credential_type
            .validate()
            .map_err(|e| ApiError::bad_request(&e.to_string()))?;
    }

    let auth_entity = auth.into_entity();
    let mut created = Vec::with_capacity(credentials.len());
    for credential in credentials {
        let result = state
            .services
            .credential_service
            .create(credential, auth_entity.clone())
            .await?;
        created.push(result);
    }

    Ok(Json(ApiResponse::success(created)))
}
