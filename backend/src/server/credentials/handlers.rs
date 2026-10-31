use crate::server::auth::middleware::permissions::{Admin, Authorized, Viewer};
use crate::server::credentials::r#impl::base::Credential;
use crate::server::credentials::r#impl::mapping::Target;
use crate::server::credentials::r#impl::types::{
    CredentialIntegration, CredentialTypeDiscriminants, OsFamily, UpstreamSupport,
};
use crate::server::credentials::service::{CredentialService, check_type_in_plan};
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::shared::handlers::list::{ListOrdering, PaginatedList, list_page};
use crate::server::shared::handlers::ordering::{OrderField, TagGroupOrdering, case_sql, in_sql};
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams, deserialize_bool_values,
    narrow_by_tags_and_search,
};
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, ListScope, base_list_filter, bulk_delete_handler,
    create_handler, delete_handler, get_by_id_handler, update_handler,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::{Junction, StorableFilter};
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::Stability;
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, EmptyApiResponse, PaginatedApiResponse,
};
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::shared::validation::validate_site_ids_access;
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult},
};
use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};
use strum::{IntoDiscriminant, IntoEnumIterator};
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

/// The credential type's tag inside the `credential_type` JSONB.
const CREDENTIAL_TYPE_SQL: &str = "credentials.credential_type->>'type'";

/// The integration each credential type belongs to, from `CredentialTypeDiscriminants::integration`.
static INTEGRATION_SQL: LazyLock<String> = LazyLock::new(|| {
    let arms: Vec<(&str, &str)> = CredentialTypeDiscriminants::iter()
        .map(|t| (t.into(), t.integration().into()))
        .collect();
    case_sql(CREDENTIAL_TYPE_SQL, arms)
});

/// Whether the credential's type is in beta, from `CredentialTypeDiscriminants::stability`.
static BETA_SQL: LazyLock<String> = LazyLock::new(|| {
    in_sql(
        CREDENTIAL_TYPE_SQL,
        CredentialTypeDiscriminants::iter()
            .filter(|t| t.stability() == Stability::Beta)
            .map(Into::into),
    )
});

/// Whether the credential's type reads an API its vendor doesn't publish, from
/// `CredentialTypeDiscriminants::upstream_support`.
static UNOFFICIAL_API_SQL: LazyLock<String> = LazyLock::new(|| {
    in_sql(
        CREDENTIAL_TYPE_SQL,
        CredentialTypeDiscriminants::iter()
            .filter(|t| t.upstream_support() == UpstreamSupport::Undocumented)
            .map(Into::into),
    )
});

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum CredentialOrderField {
    #[default]
    CreatedAt,
    Name,
    UpdatedAt,
    /// The credential type, e.g. `SnmpCommunity`.
    CredentialType,
    /// The integration the type is a transport of, e.g. `Docker` for its socket and its proxy.
    Integration,
    /// Whether the type is in beta.
    Beta,
    /// Whether the type reads an API its vendor doesn't publish.
    UnofficialApi,
    /// The daemon OS a file or socket credential is for. Credentials with none sort last.
    DaemonOs,
}

impl OrderField for CredentialOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "credentials.created_at",
            Self::Name => "credentials.name",
            Self::UpdatedAt => "credentials.updated_at",
            Self::CredentialType => CREDENTIAL_TYPE_SQL,
            Self::Integration => INTEGRATION_SQL.as_str(),
            Self::Beta => BETA_SQL.as_str(),
            Self::UnofficialApi => UNOFFICIAL_API_SQL.as_str(),
            // Stored as a JSON string; unwrapped so it groups and filters as the bare family.
            Self::DaemonOs => "(credentials.daemon_os::jsonb #>> '{}')",
        }
    }

    fn nulls_last(&self) -> bool {
        matches!(self, Self::DaemonOs)
    }
}

// ============================================================================
// Credential Filter Query
// ============================================================================

#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct CredentialFilterQuery {
    /// Filter by credential type (e.g. `SnmpCommunity`, `DockerProxy`). Repeat for several.
    #[serde(rename = "type")]
    pub credential_type: Option<Vec<CredentialTypeDiscriminants>>,
    /// Only credentials of one of these integrations. Repeat for several.
    pub integrations: Option<Vec<CredentialIntegration>>,
    /// Only credentials whose type is in beta (`true`), stable (`false`), or both (repeat it).
    /// Sent empty, matches none.
    #[serde(default, deserialize_with = "deserialize_bool_values")]
    pub beta: Option<Vec<bool>>,
    /// Only credentials whose type reads an unpublished API (`true`), a published one (`false`),
    /// or both (repeat it). Sent empty, matches none.
    #[serde(default, deserialize_with = "deserialize_bool_values")]
    pub unofficial_api: Option<Vec<bool>>,
    /// Only credentials whose type can be applied to one of these targets. Repeat for several.
    pub targets: Option<Vec<Target>>,
    /// Only file and socket credentials for one of these daemon OS families. Repeat for several.
    pub daemon_os: Option<Vec<OsFamily>>,
    /// Only credentials assigned to one of these sites. Repeat for several.
    pub site_ids: Option<Vec<Uuid>>,
    /// Filter by tag IDs (returns credentials that have ANY of the specified tags)
    pub tag_ids: Option<Vec<Uuid>>,
    /// Tag sets the credential must hold one tag from each of. Each entry is a comma-separated
    /// list of tag ids. One entry per tag group the list is filtered by.
    pub tag_sets: Option<Vec<String>>,
    /// Free-text search. Case-insensitive substring match against the name and description.
    pub search: Option<String>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<CredentialOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<CredentialOrderField>,
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

/// The links between sites and the credentials assigned to them.
const SITE_CREDENTIALS: Junction = Junction {
    table: "site_credentials",
    own_column: "credential_id",
    other_column: "site_id",
    has_history: false,
};

/// `values` as the text a boolean expression reads as.
fn bool_texts(values: &[bool]) -> Vec<String> {
    values.iter().map(bool::to_string).collect()
}

impl CredentialFilterQuery {
    /// Everything the credentials list narrows by: the organization, the type and its metadata,
    /// the daemon OS, assigned sites, tags and search.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<Credential>> {
        let filter = narrow_by_tags_and_search(
            base_list_filter::<Credential>(scope, self),
            self.tag_ids.as_deref(),
            self.tag_sets.as_deref(),
            self.search.as_deref(),
        )?;
        let filter = match &self.integrations {
            Some(integrations) if !integrations.is_empty() => {
                let ids: Vec<String> = integrations
                    .iter()
                    .map(|i| <&str>::from(i).to_string())
                    .collect();
                filter.expr_text_in(INTEGRATION_SQL.as_str(), &ids)
            }
            _ => filter,
        };
        let filter = match &self.beta {
            Some(values) => filter.expr_text_in(BETA_SQL.as_str(), &bool_texts(values)),
            None => filter,
        };
        let filter = match &self.unofficial_api {
            Some(values) => filter.expr_text_in(UNOFFICIAL_API_SQL.as_str(), &bool_texts(values)),
            None => filter,
        };
        let filter = match &self.targets {
            Some(targets) if !targets.is_empty() => {
                // A target is a property of the type, so this is the types that offer one.
                let types: Vec<&str> = CredentialTypeDiscriminants::iter()
                    .filter(|t| {
                        t.to_credential_type()
                            .targets()
                            .iter()
                            .any(|target| targets.contains(target))
                    })
                    .map(Into::into)
                    .collect();
                filter.json_text_in("credential_type", &["type"], &types)
            }
            _ => filter,
        };
        let filter = match &self.daemon_os {
            Some(families) if !families.is_empty() => {
                let ids: Vec<String> = families
                    .iter()
                    .map(|f| <&str>::from(f).to_string())
                    .collect();
                filter.expr_text_in(CredentialOrderField::DaemonOs.to_sql(), &ids)
            }
            _ => filter,
        };
        Ok(match &self.site_ids {
            Some(ids) if !ids.is_empty() => filter.linked_to(SITE_CREDENTIALS, ids),
            _ => filter,
        })
    }
}

impl FilterQueryExtractor for CredentialFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        _user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        match &self.credential_type {
            // The JSONB tag is the variant name, which is what `IntoStaticStr` yields.
            Some(types) if !types.is_empty() => {
                let ids: Vec<&str> = types.iter().map(|t| (*t).into()).collect();
                filter.json_text_in("credential_type", &["type"], &ids)
            }
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

#[async_trait]
impl CrudHandlers for Credential {
    type Service = CredentialService;
    type FilterQuery = CredentialFilterQuery;
    type OrderField = CredentialOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.credential_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }

    async fn list_filter(
        _state: &AppState,
        query: &CredentialFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Credential>> {
        query.narrow(scope)
    }
}

#[async_trait]
impl PaginatedList for Credential {
    type Item = Credential;

    fn ordering(query: &CredentialFilterQuery) -> ListOrdering<CredentialOrderField> {
        ListOrdering {
            group_by: query.group_by,
            order_by: query.order_by,
            direction: query.order_direction,
            tag_group: TagGroupOrdering {
                group_by: query.group_by_tag_group,
                order_by: query.order_by_tag_group,
            },
        }
    }

    /// Each credential with its site and host assignments.
    async fn items(
        state: &AppState,
        mut rows: Vec<Credential>,
        _query: &CredentialFilterQuery,
    ) -> ApiResult<Vec<Credential>> {
        hydrate_assignments(state, &mut rows).await?;
        Ok(rows)
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

/// Hydrate `assigned_site_ids` + `host_assignments` onto credentials from the
/// junction tables (batch).
async fn hydrate_assignments(
    state: &AppState,
    credentials: &mut [Credential],
) -> Result<(), ApiError> {
    if credentials.is_empty() {
        return Ok(());
    }
    let ids: Vec<Uuid> = credentials.iter().map(|c| c.id).collect();
    let site_map = state
        .services
        .credential_service
        .get_site_ids_for_credentials(&ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let host_map = state
        .services
        .credential_service
        .get_host_assignments_for_credentials(&ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let users = state
        .services
        .credential_service
        .users_of_logins(credentials)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    for credential in credentials.iter_mut() {
        if let Some(used_by) = users.get(&credential.id) {
            credential.base.used_by = used_by.clone();
        }
        if let Some(site_ids) = site_map.get(&credential.id) {
            credential.base.assigned_site_ids = site_ids.clone();
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
/// Docker) two credentials targeting the same host/site/IP. Call before
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
            "{integration} allows only one credential per host, but \"{existing}\" already targets an overlapping host, site, or IP. Remove or retarget it first."
        )));
    }
    Ok(())
}

/// Reject a create/update whose assignments contradict the credential type's `targets()`.
///
/// `targets()` is the single source of truth for where a type may apply, and it is load-bearing:
/// a controller type deliberately excludes `Site` because a site assignment is dispatched
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
    user_site_ids: &[Uuid],
) -> Result<(), ApiError> {
    let permitted = credential.base.credential_type.targets();
    let integration =
        ServiceDefinition::name(&*credential.base.credential_type.associated_service());

    // A login runs nothing, so it has no target of its own: the credentials that use it do.
    let assigned = !credential.base.assigned_site_ids.is_empty()
        || !credential.base.host_assignments.is_empty();
    if assigned
        && credential.base.credential_type.discriminant().role()
            == crate::server::credentials::r#impl::types::CredentialRole::Login
    {
        return Err(ApiError::validation(
            ErrorCode::CredentialLoginNotAssignable {
                name: credential.base.name.clone(),
            },
        ));
    }

    if !credential.base.assigned_site_ids.is_empty() && !permitted.contains(&Target::Site) {
        return Err(ApiError::bad_request(&format!(
            "{integration} credentials cannot be assigned to a site — a site assignment is offered to every host on the subnet. Assign it to specific hosts instead."
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
        // the daemon service (never its storage), scoped to the caller's sites so this can't
        // be used to probe for daemons in another tenant.
        let daemon_filter = StorableFilter::<Daemon>::new_from_site_ids(user_site_ids);
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
    assigned_site_ids: Vec<Uuid>,
    host_assignments: Vec<crate::server::credentials::r#impl::types::CredentialHostAssignment>,
    user_site_ids: &[Uuid],
) -> Result<(), ApiError> {
    // Validate caller-supplied references before writing the assignment
    // junction rows: a credential must not be assignable to a site or host
    // outside the caller's tenant (else a foreign daemon would receive this
    // credential's endpoint/secret during discovery).
    validate_site_ids_access(&assigned_site_ids, user_site_ids)?;
    let host_ids: Vec<Uuid> = host_assignments.iter().map(|a| a.host_id).collect();
    state
        .services
        .host_service
        .validate_ids_in_sites(&host_ids, user_site_ids)
        .await?;

    state
        .services
        .credential_service
        .set_credential_sites(&credential.id, &assigned_site_ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    state
        .services
        .credential_service
        .set_credential_host_assignments(&credential.id, &host_assignments)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    credential.base.assigned_site_ids = assigned_site_ids;
    credential.base.host_assignments = host_assignments;
    Ok(())
}

/// Get credential by ID
#[utoipa::path(
    get,
    operation_id = "get_credential_by_id",
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

/// Update credential
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
    ApiJson(mut entity): ApiJson<Credential>,
) -> ApiResult<Json<ApiResponse<Credential>>> {
    entity
        .base
        .validate_settings()
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;
    entity.base.clear_unused_daemon_os();

    let assigned_site_ids = entity.base.assigned_site_ids.clone();
    let host_assignments = entity.base.host_assignments.clone();
    let site_ids = auth.site_ids();

    enforce_supported_targets(&state, &entity, &site_ids).await?;
    enforce_single_endpoint(&state, &entity).await?;

    let mut response = update_handler::<Credential>(
        State(state.clone()),
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        Path(id),
        ApiJson(entity),
    )
    .await?;

    if let Some(credential) = response.data_mut() {
        save_assignments(
            &state,
            credential,
            assigned_site_ids,
            host_assignments,
            &site_ids,
        )
        .await?;
    }

    Ok(response)
}

/// Delete credential
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

/// Bulk delete credentials
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
    ids: ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    bulk_delete_handler::<Credential>(
        state,
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        ids,
    )
    .await
}

/// List credentials
///
/// Returns all credentials in the authenticated user's organization.
/// Optionally filter by type (e.g. `?type=SnmpCommunity`).
#[utoipa::path(
    get,
    operation_id = "list_credentials",
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
    let scope = ListScope::of(&auth.entity)?;
    Ok(Json(list_page::<Credential>(&state, &scope, &query).await?))
}

/// Create credential
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
    ApiJson(credential): ApiJson<Credential>,
) -> ApiResult<Json<ApiResponse<Credential>>> {
    let assigned_site_ids = credential.base.assigned_site_ids.clone();
    let host_assignments = credential.base.host_assignments.clone();
    let site_ids = auth.site_ids();

    enforce_supported_targets(&state, &credential, &site_ids).await?;
    enforce_single_endpoint(&state, &credential).await?;

    let mut response = create_handler::<Credential>(
        State(state.clone()),
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        ApiJson(credential),
    )
    .await?;

    if let Some(created) = response.data_mut() {
        save_assignments(
            &state,
            created,
            assigned_site_ids,
            host_assignments,
            &site_ids,
        )
        .await?;
    }

    Ok(response)
}

/// Upper bound on credentials in one bulk create. Each one is a sequential DB write.
const MAX_BULK_CREATE_CREDENTIALS: usize = 100;

fn check_bulk_create_size(len: usize) -> Result<(), ApiError> {
    if len > MAX_BULK_CREATE_CREDENTIALS {
        return Err(ApiError::bad_request(&format!(
            "Bulk create accepts at most {} credentials, got {}",
            MAX_BULK_CREATE_CREDENTIALS, len
        )));
    }
    Ok(())
}

/// Bulk create credentials
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
    ApiJson(credentials): ApiJson<Vec<Credential>>,
) -> ApiResult<Json<ApiResponse<Vec<Credential>>>> {
    if credentials.is_empty() {
        return Ok(Json(ApiResponse::success(vec![])));
    }
    check_bulk_create_size(credentials.len())?;

    // This path calls credential_service.create directly (bypassing the generic
    // create_handler's validate_create_access), so enforce tenancy here: force
    // each credential's org to the caller's rather than trusting the body, and
    // validate assignment references (inside save_assignments).
    let org_id = auth.require_organization_id()?;
    let site_ids = auth.site_ids();

    // Validate all credential types and their assignments up front, so a batch containing one
    // contradicting credential creates none of them. The service checks the plan again per create.
    let plan = state
        .services
        .organization_service
        .get_by_id(&org_id)
        .await?
        .ok_or_else(ApiError::organization_required)?
        .base
        .plan;
    for credential in &credentials {
        check_type_in_plan(None, credential.base.credential_type.discriminant(), &plan)?;
        credential
            .base
            .validate_settings()
            .map_err(|e| ApiError::bad_request(&e.to_string()))?;
        enforce_supported_targets(&state, credential, &site_ids).await?;
    }

    let auth_entity = auth.into_entity();
    let mut created = Vec::new();
    for mut credential in credentials {
        credential.base.organization_id = org_id;
        let assigned_site_ids = credential.base.assigned_site_ids.clone();
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
            assigned_site_ids,
            host_assignments,
            &site_ids,
        )
        .await?;
        created.push(result);
    }

    Ok(Json(ApiResponse::success(created)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn bulk_create_at_bound_is_accepted() {
        assert!(check_bulk_create_size(MAX_BULK_CREATE_CREDENTIALS).is_ok());
    }

    #[test]
    fn bulk_create_over_bound_is_rejected() {
        let err = check_bulk_create_size(MAX_BULK_CREATE_CREDENTIALS + 1).unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }
}
