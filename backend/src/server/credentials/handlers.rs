use crate::server::auth::middleware::permissions::{Admin, Authorized, Viewer};
use crate::server::credentials::r#impl::base::Credential;
use crate::server::credentials::r#impl::mapping::Target;
use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;
use crate::server::credentials::service::CredentialService;
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, bulk_delete_handler, create_handler, delete_handler,
    get_by_id_handler, update_handler,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, EmptyApiResponse, PaginatedApiResponse,
};
use crate::server::shared::validation::validate_network_ids_access;
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult},
};
use axum::{
    extract::{Path, State},
    response::Json,
};
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
    /// Filter by credential type (e.g. `SnmpV2c`, `DockerProxy`).
    #[serde(rename = "type")]
    pub credential_type: Option<CredentialTypeDiscriminants>,
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
        if let Some(cred_type) = self.credential_type {
            // The JSONB tag is the variant name, which is what `IntoStaticStr` yields.
            filter = filter.json_field_eq("credential_type", "type", cred_type.into());
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
    // get_by_id is custom (below) to hydrate assignments
    crate::crud_export_csv_handler!(Credential);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_credentials, create_credential))
        .routes(routes!(generated::export_csv))
        .routes(routes!(
            get_by_id_credential,
            update_credential,
            delete_credential
        ))
        .routes(routes!(bulk_delete_credentials))
        .routes(routes!(bulk_create_credentials))
}

/// Hydrate `assigned_network_ids` + `host_assignments` onto credentials from the
/// junction tables (batch).
async fn hydrate_assignments(
    state: &AppState,
    credentials: &mut [Credential],
) -> Result<(), ApiError> {
    if credentials.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = credentials.iter().map(|c| c.id).collect();
    let network_map = state
        .services
        .credential_service
        .get_network_ids_for_credentials(&ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let host_map = state
        .services
        .credential_service
        .get_host_assignments_for_credentials(&ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    for credential in credentials.iter_mut() {
        if let Some(network_ids) = network_map.get(&credential.id) {
            credential.base.assigned_network_ids = network_ids.clone();
        }
        if let Some(assignments) = host_map.get(&credential.id) {
            credential.base.host_assignments = assignments.clone();
        }
    }
    Ok(())
}

/// Persist a credential's assignments to the junction tables and reflect them on
/// the response entity.
/// Reject a create/update that would give a single-endpoint integration (e.g.
/// Docker) two credentials targeting the same host/network/IP. Call before
/// persisting; `credential` must carry its intended assignments in `base`.
async fn enforce_single_endpoint(
    state: &AppState,
    credential: &Credential,
) -> Result<(), ApiError> {
    if let Some(existing) = state
        .services
        .credential_service
        .find_single_endpoint_conflict(credential)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
    {
        let integration =
            ServiceDefinition::name(&*credential.base.credential_type.associated_service());
        return Err(ApiError::bad_request(&format!(
            "{integration} allows only one credential per host, but \"{existing}\" already targets an overlapping host, network, or IP. Remove or retarget it first."
        )));
    }
    Ok(())
}

/// Reject a create/update whose assignments contradict the credential type's `targets()`.
///
/// `targets()` is the single source of truth for where a type may apply, and it is load-bearing:
/// a controller type deliberately excludes `Network` because a network assignment is dispatched
/// as the default credential for every IP in the subnet, spraying that controller's secret at
/// unrelated hosts. Only the per-daemon `integration_targets` path was checked (and there only by
/// skipping, at scan time) — the junction assignments written here reached dispatch unchecked, so
/// this is the authoritative "prevent writing it" for every client, UI or API.
/// A type targeting only `DaemonHost` is additionally checked against the hosts that actually run
/// a daemon: a local socket is reachable over the daemon's own loopback and nowhere else, so an
/// assignment to an ordinary host would be dispatched as a 127.0.0.1 override on a machine with no
/// daemon to serve it — a probe that can only ever fail.
async fn enforce_supported_targets(
    state: &AppState,
    credential: &Credential,
    user_network_ids: &[Uuid],
) -> Result<(), ApiError> {
    let permitted = credential.base.credential_type.targets();
    let integration =
        ServiceDefinition::name(&*credential.base.credential_type.associated_service());

    if !credential.base.assigned_network_ids.is_empty() && !permitted.contains(&Target::Network) {
        return Err(ApiError::bad_request(&format!(
            "{integration} credentials cannot be assigned to a network — a network assignment is offered to every host on the subnet. Assign it to specific hosts instead."
        )));
    }

    if credential.base.host_assignments.is_empty() {
        return Ok(());
    }

    if !permitted.contains(&Target::Hosts) {
        if !permitted.contains(&Target::DaemonHost) {
            return Err(ApiError::bad_request(&format!(
                "{integration} credentials cannot be assigned to hosts."
            )));
        }

        // Daemon-host-only: every assignment must name a host that runs a daemon. Read through
        // the daemon service (never its storage), scoped to the caller's networks so this can't
        // be used to probe for daemons in another tenant.
        let daemon_filter = StorableFilter::<Daemon>::new_from_network_ids(user_network_ids);
        let daemon_host_ids: Vec<Uuid> = state
            .services
            .daemon_service
            .get_all(daemon_filter)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?
            .into_iter()
            .map(|d| d.base.host_id)
            .collect();

        if credential
            .base
            .host_assignments
            .iter()
            .any(|a| !daemon_host_ids.contains(&a.host_id))
        {
            return Err(ApiError::bad_request(&format!(
                "{integration} credentials run on a daemon's own host, so they can only be assigned to a host that runs a daemon."
            )));
        }
    }

    Ok(())
}

async fn save_assignments(
    state: &AppState,
    credential: &mut Credential,
    assigned_network_ids: Vec<Uuid>,
    host_assignments: Vec<crate::server::credentials::r#impl::types::CredentialHostAssignment>,
    user_network_ids: &[Uuid],
) -> Result<(), ApiError> {
    // Validate caller-supplied references before writing the assignment
    // junction rows: a credential must not be assignable to a network or host
    // outside the caller's tenant (else a foreign daemon would receive this
    // credential's endpoint/secret during discovery).
    validate_network_ids_access(&assigned_network_ids, user_network_ids)?;
    let host_ids: Vec<Uuid> = host_assignments.iter().map(|a| a.host_id).collect();
    state
        .services
        .host_service
        .validate_ids_in_networks(&host_ids, user_network_ids)
        .await?;

    state
        .services
        .credential_service
        .set_credential_networks(&credential.id, &assigned_network_ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    state
        .services
        .credential_service
        .set_credential_host_assignments(&credential.id, &host_assignments)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    credential.base.assigned_network_ids = assigned_network_ids;
    credential.base.host_assignments = host_assignments;
    Ok(())
}

/// Get a Credential by ID
#[utoipa::path(
    get,
    path = "/{id}",
    tag = Credential::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Credential ID")),
    responses(
        (status = 200, description = "Credential found", body = ApiResponse<Credential>),
        (status = 404, description = "Credential not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_by_id_credential(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<Credential>>> {
    let mut response =
        get_by_id_handler::<Credential>(State(state.clone()), auth, Path(id)).await?;

    if let Some(credential) = response.data_mut() {
        hydrate_assignments(&state, std::slice::from_mut(credential)).await?;
    }

    Ok(response)
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
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    Path(id): Path<Uuid>,
    Json(entity): Json<Credential>,
) -> ApiResult<Json<ApiResponse<Credential>>> {
    entity
        .base
        .credential_type
        .validate()
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    let assigned_network_ids = entity.base.assigned_network_ids.clone();
    let host_assignments = entity.base.host_assignments.clone();
    let network_ids = auth.network_ids();

    enforce_supported_targets(&state, &entity, &network_ids).await?;
    enforce_single_endpoint(&state, &entity).await?;

    let mut response = update_handler::<Credential>(
        State(state.clone()),
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        Path(id),
        Json(entity),
    )
    .await?;

    if let Some(credential) = response.data_mut() {
        save_assignments(
            &state,
            credential,
            assigned_network_ids,
            host_assignments,
            &network_ids,
        )
        .await?;
    }

    Ok(response)
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
    request_body(content = Vec<Uuid>, description = "Array of Credential IDs to delete"),
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
/// Optionally filter by type (e.g. `?type=SnmpV2c`).
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

    let mut result = state
        .services
        .credential_service
        .get_paginated_ordered(filter, &order_by)
        .await?;

    hydrate_assignments(&state, &mut result.items).await?;

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
    let assigned_network_ids = credential.base.assigned_network_ids.clone();
    let host_assignments = credential.base.host_assignments.clone();
    let network_ids = auth.network_ids();

    enforce_supported_targets(&state, &credential, &network_ids).await?;
    enforce_single_endpoint(&state, &credential).await?;

    let mut response = create_handler::<Credential>(
        State(state.clone()),
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        Json(credential),
    )
    .await?;

    if let Some(created) = response.data_mut() {
        save_assignments(
            &state,
            created,
            assigned_network_ids,
            host_assignments,
            &network_ids,
        )
        .await?;
    }

    Ok(response)
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

    // This path calls credential_service.create directly (bypassing the generic
    // create_handler's validate_create_access), so enforce tenancy here: force
    // each credential's org to the caller's rather than trusting the body, and
    // validate assignment references (inside save_assignments).
    let org_id = auth.require_organization_id()?;
    let network_ids = auth.network_ids();

    // Validate all credential types and their assignments up front, so a batch containing one
    // contradicting credential creates none of them.
    for credential in &credentials {
        credential
            .base
            .credential_type
            .validate()
            .map_err(|e| ApiError::bad_request(&e.to_string()))?;
        enforce_supported_targets(&state, credential, &network_ids).await?;
    }

    let auth_entity = auth.into_entity();
    let mut created = Vec::with_capacity(credentials.len());
    for mut credential in credentials {
        credential.base.organization_id = org_id;
        let assigned_network_ids = credential.base.assigned_network_ids.clone();
        let host_assignments = credential.base.host_assignments.clone();
        // Checked sequentially, so each credential also sees earlier batch members
        // (already persisted) — catching intra-batch conflicts too.
        enforce_single_endpoint(&state, &credential).await?;
        let mut result = state
            .services
            .credential_service
            .create(credential, auth_entity.clone())
            .await?;
        save_assignments(
            &state,
            &mut result,
            assigned_network_ids,
            host_assignments,
            &network_ids,
        )
        .await?;
        created.push(result);
    }

    Ok(Json(ApiResponse::success(created)))
}
