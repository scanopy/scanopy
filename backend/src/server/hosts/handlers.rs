use crate::daemon::runtime::state::BufferedEntities;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::features::{ensure_tagging_allowed, tags_changed};
use crate::server::auth::middleware::permissions::{
    Authorized, Can, Delete, IsDaemon, Or, Read, Write,
};
use crate::server::billing::types::base::{LimitSource, LimitType};
use crate::server::hosts::r#impl::os::HostOsFamily;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::openapi::tags as api_tags;
use crate::server::ports::r#impl::base::Port;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::BillingOperation;
use crate::server::shared::events::types::DeletionReason;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::list::list_page;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams, narrow_by_tags_and_search,
};
use crate::server::shared::handlers::traits::ListScope;
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, bulk_delete_handler, delete_handler,
};
use crate::server::shared::services::{
    entity_fields::{ServiceSource, entities_csv, validate_references},
    traits::CrudService,
};
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::storage::{filter::StorableFilter, traits::Storable};
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::{ApiErrorResponse, EmptyApiResponse};
use crate::server::shared::types::entities::EntitySourceDiscriminants;
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::shared::types::metadata::HasId;
use crate::server::shared::validation::{validate_read_access, validate_site_access};
use crate::server::sites::r#impl::Site;
use crate::server::{
    config::AppState,
    daemons::r#impl::{base::Daemon, version::pre_interface_to_ip_address_rename},
    hosts::r#impl::{
        api::{
            CreateHostRequest, DiscoveryHostRequest, HostResponse, IPAddressInput, InterfaceInput,
            ServiceInput, UpdateHostRequest,
        },
        base::{HOST_MAC_JOIN, Host, PRIMARY_INTERFACE_JOIN},
        legacy::{HostCreateRequestBody, HostCreateResponse, LegacyHostWithServicesResponse},
    },
    shared::types::api::{ApiError, ApiResponse, ApiResult, PaginatedApiResponse},
};
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write as _};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::result::ZipResult;
use zip::write::SimpleFileOptions;

// ============================================================================
// Host Ordering
// ============================================================================

/// Fields that hosts can be ordered/grouped by.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum HostOrderField {
    #[default]
    CreatedAt,
    /// Sort by the host's *title* — the [`Host::display_name`] ladder, not the stored `name`
    /// column, so the order matches what the Name column draws. Requires the primary-address
    /// JOIN for the ladder's last rung.
    Name,
    Hostname,
    UpdatedAt,
    /// Sort by virtualizing service name. Requires JOIN to services table.
    VirtualizedBy,
    SiteId,
    /// Sort by primary interface IP address. Requires JOIN to ip_addresses table.
    InterfaceIp,
    /// Sort by when discovery last observed the host. Surfaces stale assets.
    LastSeenAt,
    /// Sort by the host's lowest MAC across its live IP addresses and interfaces. Hosts with no
    /// MAC sort last in both directions. Requires the [`HOST_MAC_JOIN`].
    MacAddress,
    /// Sort by how the host came to exist (`source.type`).
    Source,
    /// Sort by hardware manufacturer. Hosts with none sort last.
    Manufacturer,
    /// Sort by hardware model. Hosts with none sort last.
    Model,
    /// Sort by SNMP sysLocation. Hosts with none sort last.
    SysLocation,
    /// Sort by operating system family (`os.family`). Hosts with no OS sort last.
    OsFamily,
}

/// The host title in SQL, built once: `to_sql` hands out `&'static str`.
static HOST_TITLE_SQL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    crate::server::hosts::r#impl::name_ladder::display_name_sql("hosts", "primary_interface")
});

impl OrderField for HostOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "hosts.created_at",
            Self::Name => HOST_TITLE_SQL.as_str(),
            Self::Hostname => "hosts.hostname",
            Self::UpdatedAt => "hosts.updated_at",
            Self::SiteId => "hosts.site_id",
            Self::LastSeenAt => "hosts.last_seen_at",
            Self::VirtualizedBy => "COALESCE(virt_service.name, '')",
            Self::InterfaceIp => "primary_interface.ip_address",
            Self::MacAddress => "host_mac.mac_address",
            Self::Source => "hosts.source->>'type'",
            Self::Manufacturer => "hosts.manufacturer",
            Self::Model => "hosts.model",
            Self::SysLocation => "hosts.sys_location",
            Self::OsFamily => "hosts.os->>'family'",
        }
    }

    fn sorts_by_site_name(&self) -> bool {
        matches!(self, Self::SiteId)
    }

    fn nulls_last(&self) -> bool {
        matches!(
            self,
            Self::MacAddress
                | Self::Manufacturer
                | Self::Model
                | Self::SysLocation
                | Self::OsFamily
        )
    }

    fn join_sql(&self) -> Option<&'static str> {
        match self {
            Self::VirtualizedBy => Some(
                "LEFT JOIN services AS virt_service ON \
                 hosts.virtualization_service_id = virt_service.id",
            ),
            // One const shared by both: grouping by interface ip while ordering by name must
            // produce a single `primary_interface` join. `apply_ordering` drops the second when
            // the two are equal, so what matters is that they cannot stop being equal.
            Self::Name | Self::InterfaceIp => Some(PRIMARY_INTERFACE_JOIN),
            Self::MacAddress => Some(HOST_MAC_JOIN),
            _ => None,
        }
    }
}

// ============================================================================
// Host Filter Query
// ============================================================================

/// Query parameters for filtering and ordering hosts.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct HostFilterQuery {
    /// Filter by site ID. Repeat the parameter to pass several.
    #[serde(alias = "site_id")]
    pub site_ids: Option<Vec<Uuid>>,
    /// Filter by specific entity IDs (for selective loading)
    pub ids: Option<Vec<Uuid>>,
    /// Filter by the service virtualizing the host. Repeat for several.
    pub virtualization_service_ids: Option<Vec<Uuid>>,
    /// Filter by the name of the service virtualizing the host, the value "Virtualized By"
    /// groups on. Repeat for several.
    pub virtualization_service_names: Option<Vec<String>>,
    /// `true` also returns hosts nothing virtualizes. Set on its own it returns
    /// only those — the "Not Virtualized" choice in the UI's filter.
    pub include_unvirtualized: Option<bool>,
    /// Filter to hosts running a service with one of these names.
    pub service_names: Option<Vec<String>>,
    /// Filter by how the host came to exist (`source.type`). Repeat for several;
    /// `Inferred` alone lists the hosts only a neighbour advertised.
    pub sources: Option<Vec<EntitySourceDiscriminants>>,
    /// Filter by hardware manufacturer. Repeat for several.
    pub manufacturers: Option<Vec<String>>,
    /// Filter by hardware model. Repeat for several.
    pub models: Option<Vec<String>>,
    /// Filter by SNMP sysLocation. Repeat for several.
    pub sys_locations: Option<Vec<String>>,
    /// Filter by operating system family. Repeat for several.
    pub os_families: Option<Vec<HostOsFamily>>,
    /// Filter to hosts assigned one of these credentials. Repeat for several.
    pub credential_ids: Option<Vec<Uuid>>,
    /// Filter by tag IDs (returns hosts that have ANY of the specified tags)
    pub tag_ids: Option<Vec<Uuid>>,
    /// Tag sets the host must hold one tag from each of. Each entry is a comma-separated list
    /// of tag ids; a host matches when it holds at least one tag of every entry. One entry per
    /// tag group the list is filtered by.
    pub tag_sets: Option<Vec<String>>,
    /// Free-text search. Case-insensitive substring match against the host's
    /// name, hostname, sysName, chassis id, serial number, asset tag and
    /// description, and against its IP addresses and the names of services
    /// running on it.
    ///
    /// sysName and chassis id are in there because they are rungs of the title
    /// ladder: a host with no name of its own is *shown* under one of them, and
    /// a title you can read but cannot search for is a dead end.
    pub search: Option<String>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<HostOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<HostOrderField>,
    /// Group by the tag each row holds from this tag group, in the group's tag order. Takes
    /// the place of `group_by`.
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
    /// As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
    /// instant (snapshot view) instead of live state.
    pub at: Option<chrono::DateTime<chrono::Utc>>,
    /// `true` returns only hosts discovery hasn't observed within their
    /// site's staleness window; `false` returns only those it has. Omit for
    /// both. Evaluated per row against the host's own site's window.
    pub stale: Option<bool>,
    /// `false` returns hosts with empty `ports`/`services`/`interfaces`.
    /// `ip_addresses` is always populated: the host's title can come from its
    /// address. The children dominate the payload, so callers that only need
    /// host identity — name pickers, id→name lookups, counts — should pass
    /// `false`. Defaults to `true`, so existing callers are unaffected.
    pub include_children: Option<bool>,
}

impl HostFilterQuery {
    /// Everything the Hosts list narrows by except staleness, which needs each site's cutoff:
    /// access, the query's own fields, tags, search and the field filters.
    pub fn narrow(
        &self,
        site_ids: &[Uuid],
        organization_id: Uuid,
    ) -> ApiResult<StorableFilter<Host>> {
        let base_filter = StorableFilter::<Host>::new_from_site_ids(site_ids);
        let filter = self.apply_to_filter(base_filter, site_ids, organization_id);

        // SCD2 read path: live by default, or as-of the snapshot timestamp when set.
        // Without this, close-and-clone's closed historical copies leak into the
        // host list as empty-shell duplicates.
        let filter = filter.live_or_as_of(self.at).not_dismissed();

        let filter = narrow_by_tags_and_search(
            filter,
            self.tag_ids.as_deref(),
            self.tag_sets.as_deref(),
            self.search.as_deref(),
        )?;

        Ok(self.apply_field_filters(filter))
    }

    /// Apply the field filters the Hosts tab offers.
    ///
    /// Shared by the list and the exports, for the same reason the staleness
    /// filter is: an export should mirror exactly what the user was looking at.
    /// Every one of these is applied server-side because the list is paginated —
    /// filtering the loaded page instead would hide matches on every other page
    /// and leave the total count describing a different set of rows.
    pub fn apply_field_filters(&self, filter: StorableFilter<Host>) -> StorableFilter<Host> {
        // "Not Virtualized" is a choice about absence, so it can arrive without
        // any service ids or names beside it.
        let filter = filter.virtualization_parent(
            self.virtualization_service_ids.as_deref().unwrap_or(&[]),
            self.virtualization_service_names.as_deref().unwrap_or(&[]),
            self.include_unvirtualized.unwrap_or(false),
        );

        let filter = match &self.service_names {
            Some(names) if !names.is_empty() => filter.has_service_named(names),
            _ => filter,
        };

        let filter = match &self.sources {
            Some(sources) if !sources.is_empty() => filter.source_type_in(sources),
            _ => filter,
        };

        let filter = match &self.manufacturers {
            Some(values) if !values.is_empty() => filter.text_column_in("manufacturer", values),
            _ => filter,
        };

        let filter = match &self.models {
            Some(values) if !values.is_empty() => filter.text_column_in("model", values),
            _ => filter,
        };

        let filter = match &self.sys_locations {
            Some(values) if !values.is_empty() => filter.text_column_in("sys_location", values),
            _ => filter,
        };

        let filter = match &self.os_families {
            Some(families) if !families.is_empty() => {
                let ids: Vec<&str> = families.iter().map(|f| f.id()).collect();
                filter.json_text_in("os", &["family"], &ids)
            }
            _ => filter,
        };

        match &self.credential_ids {
            Some(ids) if !ids.is_empty() => filter.has_credential(ids),
            _ => filter,
        }
    }
}

impl FilterQueryExtractor for HostFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        // Apply IDs filter first if provided
        let filter = match &self.ids {
            Some(ids) if !ids.is_empty() => filter.entity_ids(ids),
            _ => filter,
        };
        // Then apply site filter. Intersect with what the caller can see —
        // a requested site they have no access to must narrow the result to
        // nothing, never widen it.
        match &self.site_ids {
            Some(requested) => {
                let accessible: Vec<Uuid> = requested
                    .iter()
                    .copied()
                    .filter(|id| user_site_ids.contains(id))
                    .collect();
                filter.site_ids(&accessible)
            }
            None => filter.site_ids(user_site_ids),
        }
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

// Generated handlers for CSV export
mod generated {
    use super::*;
    crate::crud_export_csv_handler!(Host);
    crate::crud_get_field_values_handler!(Host);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_hosts, create_host))
        .routes(routes!(get_host_by_id, update_host, delete_host))
        .routes(routes!(bulk_delete_hosts))
        .routes(routes!(generated::export_csv))
        .routes(routes!(generated::get_field_values))
        .routes(routes!(export_hosts_zip))
        .routes(routes!(merge_hosts))
        .routes(routes!(confirm_host))
        .routes(routes!(dismiss_host))
        .routes(routes!(create_host_discovery))
        .merge(super::rescan::create_router())
}

/// Confirm unconfirmed host
///
/// Records that a person vouches for a host Scanopy knows only from another device's report (an
/// LLDP/CDP neighbour, or a switch's forwarding table). The host's source becomes `Manual`.
#[utoipa::path(
    post,
    path = "/{id}/confirm",
    tag = Host::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Host ID")),
    responses(
        (status = 200, description = "Host confirmed", body = ApiResponse<HostResponse>),
        (status = 400, description = "The host is already confirmed", body = ApiErrorResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn confirm_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Host, Write>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    settle_host(&state, auth, id, true).await
}

/// Dismiss unconfirmed host
///
/// Sets aside a host Scanopy knows only from another device's report. It leaves lists, topology
/// and scan warnings, and is not created again by later scans. A scan that reaches the device
/// restores it.
#[utoipa::path(
    post,
    path = "/{id}/dismiss",
    tag = Host::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Host ID")),
    responses(
        (status = 200, description = "Host dismissed", body = ApiResponse<HostResponse>),
        (status = 400, description = "The host is already confirmed", body = ApiErrorResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn dismiss_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Host, Write>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    settle_host(&state, auth, id, false).await
}

async fn settle_host(
    state: &AppState,
    auth: Authorized<Can<Host, Write>>,
    id: Uuid,
    confirm: bool,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let host = state
        .services
        .host_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(id))?;
    validate_read_access(
        Some(host.base.site_id),
        None,
        &auth.site_ids(),
        organization_id,
    )?;
    let host_service = &state.services.host_service;
    let response = if confirm {
        host_service.confirm_host(id, auth.into_entity()).await?
    } else {
        host_service.dismiss_host(id, auth.into_entity()).await?
    };
    Ok(Json(ApiResponse::success(response)))
}

/// List hosts
///
/// Returns all hosts the authenticated user has access to, with their
/// ip_addresses, ports, services and interfaces included — pass
/// `include_children=false` to omit the ports, services and interfaces and get a
/// much smaller payload.
/// Supports pagination via `limit` and `offset` query parameters, and ordering
/// via `group_by`, `order_by`, and `order_direction`.
#[utoipa::path(
    get,
    operation_id = "list_hosts",
    path = "",
    tag = Host::ENTITY_NAME_PLURAL,
    params(HostFilterQuery),
    responses(
        (status = 200, description = "List of hosts with their children", body = PaginatedApiResponse<HostResponse>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Host, Read>>,
    Query(query): Query<HostFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<HostResponse>>> {
    let scope = ListScope::of(&auth)?;
    Ok(Json(list_page::<Host>(&state, &scope, &query).await?))
}

/// Get host by ID
///
/// Returns a single host with its ip_addresses, ports, and services.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = Host::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Host ID")),
    responses(
        (status = 200, description = "Host found", body = ApiResponse<HostResponse>),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_host_by_id(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Host, Read>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let mut host = state
        .services
        .host_service
        .get_host_response(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(id))?;

    validate_read_access(Some(host.site_id), None, &site_ids, organization_id)?;

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[host.id], EntityDiscriminants::Host, None)
        .await?;
    if let Some(tags) = tags_map.get(&host.id) {
        host.tags = tags.clone();
    }

    // Hydrate credential assignments from junction table
    host.credential_assignments = state
        .services
        .credential_service
        .get_credential_assignments_for_host(&host.id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(host)))
}

/// The rows a host request carries with it, and the site they land on.
struct HostChildren<'a> {
    site_id: Uuid,
    ip_addresses: &'a [IPAddressInput],
    interfaces: &'a [InterfaceInput],
    services: &'a [ServiceInput],
}

/// Refuses a host request naming rows the caller can't use. `request` is the serialized request,
/// which carries the keys the registry declares for a host. Children have no entity of their own
/// yet, so what they name is checked here against the site, and ids the request itself creates
/// pass.
async fn validate_host_request(
    state: &AppState,
    mut request: serde_json::Value,
    stored: Option<&Host>,
    children: HostChildren<'_>,
    caller: &AuthenticatedEntity,
    organization_id: Uuid,
) -> Result<(), ApiError> {
    let HostChildren {
        site_id,
        ip_addresses,
        interfaces,
        services,
    } = children;

    request["site_id"] = serde_json::json!(site_id);
    let stored = stored
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let own_addresses: Vec<Uuid> = ip_addresses.iter().map(|a| a.id).collect();
    validate_references(
        &state.services,
        EntityDiscriminants::Host,
        &request,
        stored.as_ref(),
        &own_addresses,
        caller,
        organization_id,
    )
    .await?;

    for ip_address in ip_addresses {
        if let Some(subnet) = state
            .services
            .subnet_service
            .get_by_id(&ip_address.subnet_id)
            .await?
            && subnet.base.site_id != site_id
        {
            return Err(ApiError::bad_request(&format!(
                "Host is on site {}, cannot have an ip_address with a subnet \"{}\" which is on site {}.",
                site_id, subnet.base.name, subnet.base.site_id
            )));
        }
    }

    let named_addresses: Vec<Uuid> = interfaces
        .iter()
        .filter_map(|interface| interface.ip_address_id)
        .filter(|id| !own_addresses.contains(id))
        .collect();
    state
        .services
        .ip_address_service
        .validate_ids_in_sites(&named_addresses, &[site_id])
        .await?;

    let own_services: Vec<Uuid> = services.iter().map(|s| s.id).collect();
    let named_services: Vec<Uuid> = services
        .iter()
        .filter_map(|service| service.virtualization_service_id)
        .filter(|id| !own_services.contains(id))
        .collect();
    state
        .services
        .service_service
        .validate_ids_in_sites(&named_services, &[site_id])
        .await?;

    let service_tags: Vec<Uuid> = services.iter().flat_map(|s| s.tags.clone()).collect();
    state
        .services
        .tag_service
        .validate_ids_in_org(&service_tags, organization_id)
        .await
}

/// Create host
///
/// Creates a host with optional ip_addresses, ports, and services.
/// The `source` field is automatically set to `Manual`.
///
/// ### Tag Validation
///
/// - Tags must exist and belong to your organization
/// - Duplicate tag UUIDs are automatically deduplicated
/// - Invalid or cross-organization tag UUIDs return a 400 error
///
#[utoipa::path(
    post,
    path = "",
    tag = Host::ENTITY_NAME_PLURAL,
    request_body = CreateHostRequest,
    responses(
        (status = 200, description = "Host created successfully", body = ApiResponse<HostResponse>),
        (status = 400, description = "Validation error: site not found, subnet mismatch, or invalid tags", body = ApiErrorResponse),
        (status = 401, description = "No access to the specified site", body = ApiErrorResponse),
    ),
    security( ("user_api_key" = []),("session" = []), ("daemon_api_key" = []))
)]
async fn create_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Or<Can<Host, Write>, IsDaemon>>,
    ApiJson(request): ApiJson<HostCreateRequestBody>,
) -> ApiResult<Json<ApiResponse<HostCreateResponse>>> {
    let site_ids = auth.site_ids();
    let organization_id = auth.organization_id();
    let entity = auth.into_entity();
    let host_service = &state.services.host_service;

    match (request, &entity) {
        // New format - standard flow
        (HostCreateRequestBody::New(request), _) => {
            // Validate request (name length, etc.)
            request
                .validate()
                .map_err(|e| ApiError::bad_request(&e.to_string()))?;

            // Validate user has access to the site
            validate_site_access(Some(request.site_id), &site_ids, "create")?;

            if let Some(org_id) = organization_id
                && (!request.tags.is_empty() || request.services.iter().any(|s| !s.tags.is_empty()))
            {
                ensure_tagging_allowed(&state, &org_id).await?;
            }

            // Validate site_id exists
            let site = state
                .services
                .site_service
                .get_by_id(&request.site_id)
                .await?
                .ok_or_else(|| {
                    ApiError::bad_request(&format!("Site {} not found", request.site_id))
                })?;

            // Check host limit on plan
            if let Some(org_id) = organization_id {
                let plan = state
                    .services
                    .organization_service
                    .get_by_id(&org_id)
                    .await?
                    .map(|o| o.base.plan)
                    .unwrap_or_else(crate::server::billing::plans::get_free_plan);
                if let Some(limit) = plan.host_limit() {
                    // Counted the way the dashboard counts and the way the discovery path gates:
                    // through `count_for_sites`, which narrows SCD2 entities to live rows, and
                    // across *every* site the organization owns. Hand-rolling it here meant
                    // closed snapshot copies were counted and the caller's own accessible sites
                    // were the scope — so a user could be shown 18/25 and refused at 25, and a
                    // member whose sites are a subset of the org's could create past the limit.
                    let org_site_ids: Vec<Uuid> = state
                        .services
                        .site_service
                        .get_all(StorableFilter::<Site>::new_from_org_id(&org_id))
                        .await
                        .unwrap_or_default()
                        .iter()
                        .map(|n| n.id)
                        .collect();
                    let current_hosts = state
                        .services
                        .host_service
                        .count_for_sites(&org_site_ids)
                        .await?;
                    if current_hosts >= limit {
                        state
                            .services
                            .event_bus
                            .publish(Event::new(
                                OrgScope {
                                    organization_id: org_id,
                                },
                                BillingOperation::FeatureLimitHit {
                                    limit_type: LimitType::Hosts,
                                    current_count: current_hosts,
                                    limit,
                                    plan,
                                    source: LimitSource::Api,
                                },
                                entity.clone(),
                            ))
                            .await;

                        return Err(ApiError::coded(
                            StatusCode::FORBIDDEN,
                            ErrorCode::BillingHostLimitReached { limit },
                        ));
                    }
                }
            }

            validate_host_request(
                &state,
                serde_json::to_value(&request)
                    .map_err(|e| ApiError::internal_error(&e.to_string()))?,
                None,
                HostChildren {
                    site_id: request.site_id,
                    ip_addresses: &request.ip_addresses,
                    interfaces: &request.interfaces,
                    services: &request.services,
                },
                &entity,
                site.base.organization_id,
            )
            .await?;

            let host_response = host_service.create_from_request(request, entity).await?;

            // Sync credential assignments to junction table
            state
                .services
                .credential_service
                .set_host_credentials(&host_response.id, &host_response.credential_assignments)
                .await
                .map_err(|e| ApiError::internal_error(&e.to_string()))?;

            Ok(Json(ApiResponse::success(HostCreateResponse::New(
                host_response,
            ))))
        }

        // Legacy format from daemon - transform and process
        (
            HostCreateRequestBody::Legacy(legacy_request),
            AuthenticatedEntity::Daemon { daemon_id, .. },
        ) => {
            tracing::warn!(
                daemon_id = %daemon_id,
                "Legacy daemon request to POST /api/hosts - daemon should be updated"
            );

            let discovery_request = legacy_request.into_discovery_request();

            // Validate daemon has access to the site
            validate_site_access(
                Some(discovery_request.host.base.site_id),
                &site_ids,
                "create",
            )?;

            let DiscoveryHostRequest {
                host,
                ip_addresses,
                ports,
                services,
                interfaces,
                subnets,
                interfaces_complete,
                interface_data_complete,
                port_coverage,
                superseded_wire_shape,
            } = discovery_request;

            // Always true on this branch — reaching it means the daemon sent the pre-0.16.0
            // body. Recorded against the daemon so the scan record carries it, which the
            // `tracing::warn!` above never could: a self-hosted operator does not read server
            // logs.
            if superseded_wire_shape {
                state
                    .services
                    .discovery_service
                    .note_superseded_wire_shape(*daemon_id)
                    .await;
            }

            let daemon_id = *daemon_id;
            // Capture one scan_time for the whole submission so all entities
            // share consistent SCD2 timestamps. See ScanContext for rationale.
            let scan_ctx =
                crate::server::shared::services::scan_context::ScanContext::new(daemon_id);
            let discovered = host_service
                .discover_host(
                    host,
                    ip_addresses,
                    ports,
                    services,
                    interfaces,
                    subnets,
                    interfaces_complete,
                    interface_data_complete,
                    &port_coverage,
                    Some(&scan_ctx),
                    entity,
                    None,
                )
                .await?;

            // Latched for the scan record, as `superseded_wire_shape` is above.
            state
                .services
                .discovery_service
                .note_touched_subnets(daemon_id, discovered.subnet_ids)
                .await;

            let legacy_response =
                LegacyHostWithServicesResponse::from_host_response(discovered.host);

            Ok(Json(ApiResponse::success(HostCreateResponse::Legacy(
                legacy_response,
            ))))
        }

        // Legacy format from non-daemon (user) - reject
        (HostCreateRequestBody::Legacy(_), _) => Err(ApiError::bad_request(
            "Invalid request format. Please use the CreateHostRequest format.",
        )),
    }
}

/// Whether a host update changes the host's tags or any of its services' tags.
/// A service the request adds counts as a change only when it carries tags.
async fn host_update_changes_tags(
    state: &AppState,
    existing_host: &Host,
    request: &UpdateHostRequest,
) -> Result<bool, ApiError> {
    if tags_changed(existing_host.get_tags(), Some(&request.tags)) {
        return Ok(true);
    }
    for input in request.services.iter().flatten() {
        let existing = state.services.service_service.get_by_id(&input.id).await?;
        if tags_changed(
            existing.as_ref().and_then(|s| s.get_tags()),
            Some(&input.tags),
        ) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Update host
///
/// Updates host properties. Children (ip_addresses, ports, services, interfaces) are synced
/// from the fields on this same request body when provided — omit a field to leave that child
/// set untouched.
///
/// ### Tag Validation
///
/// - Tags must exist and belong to your organization
/// - Duplicate tag UUIDs are automatically deduplicated
/// - Invalid or cross-organization tag UUIDs return a 400 error
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Host::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Host ID")),
    request_body = UpdateHostRequest,
    responses(
        (status = 200, description = "Host updated", body = ApiResponse<HostResponse>),
        (status = 400, description = "Validation error: invalid tags", body = ApiErrorResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Host, Write>>,
    Path(id): Path<Uuid>,
    ApiJson(mut request): ApiJson<UpdateHostRequest>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Validate request (name length, etc.)
    request
        .validate()
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    let host_service = &state.services.host_service;

    // Path ID is canonical - override any ID in the body
    request.id = id;

    // Fetch existing host to validate site access
    let existing_host = host_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(id))?;

    validate_read_access(
        Some(existing_host.base.site_id),
        None,
        &site_ids,
        organization_id,
    )?;

    validate_host_request(
        &state,
        serde_json::to_value(&request).map_err(|e| ApiError::internal_error(&e.to_string()))?,
        Some(&existing_host),
        HostChildren {
            site_id: existing_host.base.site_id,
            ip_addresses: request.ip_addresses.as_deref().unwrap_or_default(),
            interfaces: request.interfaces.as_deref().unwrap_or_default(),
            services: request.services.as_deref().unwrap_or_default(),
        },
        &auth.entity,
        organization_id,
    )
    .await?;

    if host_update_changes_tags(&state, &existing_host, &request).await? {
        ensure_tagging_allowed(&state, &organization_id).await?;
    }

    let credential_assignments = request.credential_assignments.clone();

    let mut host_response = host_service
        .update_from_request(request, auth.into_entity())
        .await?;

    // Sync credential assignments if provided
    if let Some(ref assignments) = credential_assignments {
        state
            .services
            .credential_service
            .set_host_credentials(&host_response.id, assignments)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
        host_response.credential_assignments = assignments.clone();
    } else {
        // Hydrate from junction table
        host_response.credential_assignments = state
            .services
            .credential_service
            .get_credential_assignments_for_host(&host_response.id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[host_response.id], EntityDiscriminants::Host, None)
        .await?;
    if let Some(tags) = tags_map.get(&host_response.id) {
        host_response.tags = tags.clone();
    }

    Ok(Json(ApiResponse::success(host_response)))
}

/// Upsert host from daemon discovery
///
/// Used by daemons to report discovered hosts. Accepts full entities with
/// pre-generated IDs. Uses upsert behavior to merge with existing hosts.
///
/// Tagged as "internal" - included in OpenAPI spec for client generation
/// but hidden from public documentation.
#[utoipa::path(
    post,
    path = "/discovery",
    tags = [Host::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    request_body = DiscoveryHostRequest,
    responses(
        (status = 200, description = "Host discovered/updated successfully", body = ApiResponse<HostResponse>),
        (status = 403, description = "Daemon cannot create hosts on other sites", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn create_host_discovery(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    ApiJson(request): ApiJson<DiscoveryHostRequest>,
) -> ApiResult<impl IntoResponse> {
    // Legacy cleanup: remove once the enforced ceiling passes `last_without_ip_address_bindings()`
    let is_legacy_daemon = pre_interface_to_ip_address_rename(auth.entity.daemon_version());

    // Get daemon site_id from entity
    let daemon_site_id = auth
        .site_ids()
        .first()
        .copied()
        .ok_or_else(|| ApiError::forbidden("Daemon has no site assignment"))?;

    if request.host.base.site_id != daemon_site_id {
        return Err(ApiError::forbidden(
            "Daemon cannot create hosts on sites it's not assigned to",
        ));
    }

    // Delegate to processor for shared discovery logic
    // This ensures both DaemonPoll and ServerPoll modes use the same logic
    let entities = BufferedEntities {
        hosts: vec![request],
        subnets: vec![],
        vlans: vec![],
    };

    let created = state
        .services
        .daemon_service
        .process_discovery_entities(entities, auth.into_entity())
        .await?;

    let (_, host_response) = created.hosts.into_iter().next().ok_or_else(|| {
        if let Some((limit, _org_id)) = created.billing_limit_hit {
            ApiError::coded(
                StatusCode::FORBIDDEN,
                ErrorCode::BillingHostLimitReached { limit },
            )
        } else if let Some(err) = created.first_host_error {
            // Single-host discovery: answer with the real per-host failure and its
            // status (a refusal stays a 4xx) instead of a generic message. The
            // processor swallows per-host errors to keep multi-host batches going;
            // here there is exactly one host.
            err
        } else {
            ApiError::internal_error("No host returned from processor")
        }
    })?;

    // Legacy cleanup: remove once the enforced ceiling passes `last_without_ip_address_bindings()`
    // Pre-0.16.0 daemons expect "Interface"/"interface_id" in binding responses
    if is_legacy_daemon {
        let mut json = serde_json::to_value(ApiResponse::success(host_response))
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
        rewrite_response_for_legacy_daemon(&mut json);
        return Ok(Json(json));
    }

    Ok(Json(
        serde_json::to_value(ApiResponse::success(host_response))
            .map_err(|e| ApiError::internal_error(&e.to_string()))?,
    ))
}

/// Merge hosts
///
/// Merges all IP addresses, ports, and services from the host `other_id` into the host `id`,
/// then deletes `other_id`. Both hosts must be on the same site.
///
/// ### Merge Behavior
///
/// - **Interfaces**: Transferred to destination. If an interface with matching subnet+IP or MAC
///   already exists on destination, bindings are remapped to use the existing interface.
/// - **Ports**: Transferred to destination. If a port with the same number and protocol already
///   exists, bindings are remapped to use the existing port.
/// - **Services**: Transferred to destination with deduplication.
///   See [upsert behavior](https://scanopy.net/docs/discovery/#upsert-behavior) for details.
///
/// ### Restrictions
///
/// - Cannot merge a host with itself.
/// - Cannot merge away a host that has a daemon - merge into it instead.
#[utoipa::path(
    post,
    path = "/{id}/merge/{other_id}",
    tag = Host::ENTITY_NAME_PLURAL,
    params(
        ("id" = Uuid, Path, description = "Host that receives all children"),
        ("other_id" = Uuid, Path, description = "Host merged into `id`, then deleted")
    ),
    responses(
        (status = 200, description = "Hosts merged successfully", body = ApiResponse<HostResponse>),
        (status = 404, description = "One or both hosts not found", body = ApiErrorResponse),
        (status = 400, description = "Validation error: same host, has daemon, or different sites", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn merge_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Host, Write>>,
    Path((destination_host_id, other_host_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let host_service = &state.services.host_service;

    let destination_host = host_service
        .get_by_id(&destination_host_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(destination_host_id))?;
    let other_host = host_service
        .get_by_id(&other_host_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Host>(other_host_id))?;
    // An unconfirmed host can be merged away (that settles it), never merged into.
    destination_host
        .require_confirmed()
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    // Validate user has access to both hosts
    validate_read_access(
        Some(destination_host.base.site_id),
        None,
        &site_ids,
        organization_id,
    )?;
    validate_read_access(
        Some(other_host.base.site_id),
        None,
        &site_ids,
        organization_id,
    )?;

    // Make sure hosts are on same site
    if destination_host.base.site_id != other_host.base.site_id {
        return Err(ApiError::bad_request(&format!(
            "Destination Host is on site {}, other host \"{}\" can't be on a different site ({}).",
            destination_host.base.site_id, other_host.base.name, other_host.base.site_id
        )));
    }

    let reason = DeletionReason::HostMergedInto {
        host_id: destination_host.id,
        proven_by: Vec::new(),
    };
    let mut host_response = host_service
        .merge_hosts(destination_host, other_host, auth.into_entity(), reason)
        .await?;

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[host_response.id], EntityDiscriminants::Host, None)
        .await?;
    if let Some(tags) = tags_map.get(&host_response.id) {
        host_response.tags = tags.clone();
    }

    // Hydrate credential assignments from junction table
    host_response.credential_assignments = state
        .services
        .credential_service
        .get_credential_assignments_for_host(&host_response.id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(host_response)))
}

/// Delete host
///
/// Prevents deletion if the host has a daemon associated with it
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Host::ENTITY_NAME_PLURAL,
    params(
        ("id" = Uuid, Path, description = "Host ID")
    ),
    responses(
        (status = 200, description = "Host deleted", body = EmptyApiResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
        (status = 409, description = "Host has associated daemon", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn delete_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Host, Delete>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // An unconfirmed host is dismissed, not deleted: a deleted one comes back on the next scan.
    if let Some(host) = state.services.host_service.get_by_id(&id).await? {
        host.require_confirmed()
            .map_err(|e| ApiError::bad_request(&e.to_string()))?;
    }
    // Pre-validation: Can't delete a host with an associated daemon
    let daemon_filter = StorableFilter::<Daemon>::new_from_host_ids(&[id]);
    if state.services.daemon_service.exists(daemon_filter).await? {
        return Err(ApiError::coded(
            StatusCode::CONFLICT,
            ErrorCode::EntityDeleteForbidden {
                entity: "host".to_string(),
                reason: Some("has an associated daemon — delete the daemon first".to_string()),
            },
        ));
    }

    // Delegate to generic handler (handles auth checks, deletion)
    delete_handler::<Host>(State(state), auth, Path(id)).await
}

/// Bulk delete hosts
///
/// Deletes multiple hosts in a single request. The request body should be
/// an array of host IDs to delete. Fails if any host has an associated daemon.
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = Host::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "Array of Host IDs to delete"),
    responses(
        (status = 200, description = "Hosts deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 409, description = "One or more hosts has an associated daemon - delete daemons first", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_delete_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Host, Delete>>,
    ApiJson(ids): ApiJson<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    for host in state
        .services
        .host_service
        .get_all(StorableFilter::<Host>::new_from_entity_ids(&ids))
        .await?
    {
        host.require_confirmed()
            .map_err(|e| ApiError::bad_request(&e.to_string()))?;
    }

    let daemon_service = &state.services.daemon_service;

    let daemon_filter = StorableFilter::<Daemon>::new_from_host_ids(&ids);

    if !daemon_service.get_all(daemon_filter).await?.is_empty() {
        return Err(ApiError::coded(
            StatusCode::CONFLICT,
            ErrorCode::EntityDeleteForbidden {
                entity: "host".to_string(),
                reason: Some(
                    "one or more hosts has an associated daemon — delete the daemon(s) first"
                        .to_string(),
                ),
            },
        ));
    }

    bulk_delete_handler::<Host>(axum::extract::State(state), auth, ApiJson(ids)).await
}

/// Export hosts with children to ZIP
///
/// Exports all hosts matching the filter criteria along with their children
/// (ip_addresses, ports, services, interfaces) as a ZIP archive containing
/// separate CSV files for each entity type.
#[utoipa::path(
    get,
    path = "/export/zip",
    tag = Host::ENTITY_NAME_PLURAL,
    operation_id = "export_hosts_zip",
    params(HostFilterQuery),
    responses(
        (status = 200, description = "ZIP file containing CSVs", content_type = "application/zip", body = String),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn export_hosts_zip(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Host, Read>>,
    Query(query): Query<HostFilterQuery>,
) -> ApiResult<impl IntoResponse> {
    let scope = ListScope::of(&auth)?;
    let (site_ids, organization_id) = (scope.site_ids.clone(), scope.organization_id);

    // The list's own filter, so the export holds the rows the list showed.
    let filter = Host::list_filter(&state, &query, &scope).await?;

    // Get all hosts (no pagination for export)
    let hosts = state.services.host_service.get_all(filter).await?;
    let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();

    // Fetch children for these hosts
    let ip_addresses = state
        .services
        .ip_address_service
        .get_all(
            StorableFilter::<IPAddress>::new_from_host_ids(&host_ids)
                .site_ids(&site_ids)
                .live_or_as_of(query.at),
        )
        .await?;

    let ports = state
        .services
        .port_service
        .get_all(
            StorableFilter::<Port>::new_from_host_ids(&host_ids)
                .site_ids(&site_ids)
                .live_or_as_of(query.at),
        )
        .await?;

    let services = state
        .services
        .service_service
        .get_all(
            StorableFilter::<Service>::new_from_host_ids(&host_ids)
                .site_ids(&site_ids)
                .live_or_as_of(query.at),
        )
        .await?;

    let interfaces = state
        .services
        .interface_service
        .get_all(
            StorableFilter::<Interface>::new_from_host_ids(&host_ids)
                .site_ids(&site_ids)
                .live_or_as_of(query.at),
        )
        .await?;

    // Build CSVs from the field registry, related entities read through their services.
    let source = ServiceSource {
        services: &state.services,
        reader: &auth.entity,
        organization_id,
    };
    let csv_error =
        |e: anyhow::Error| ApiError::internal_error(&format!("Failed to build CSV: {e}"));
    let hosts_csv = entities_csv(&source, EntityDiscriminants::Host, &hosts)
        .await
        .map_err(csv_error)?;
    let ip_addresses_csv = entities_csv(&source, EntityDiscriminants::IPAddress, &ip_addresses)
        .await
        .map_err(csv_error)?;
    let ports_csv = entities_csv(&source, EntityDiscriminants::Port, &ports)
        .await
        .map_err(csv_error)?;
    let services_csv = entities_csv(&source, EntityDiscriminants::Service, &services)
        .await
        .map_err(csv_error)?;
    let interfaces_csv = entities_csv(&source, EntityDiscriminants::Interface, &interfaces)
        .await
        .map_err(csv_error)?;

    let zip_data = zip_archive(&[
        ("hosts.csv", &hosts_csv),
        ("ip_addresses.csv", &ip_addresses_csv),
        ("ports.csv", &ports_csv),
        ("services.csv", &services_csv),
        ("interfaces.csv", &interfaces_csv),
    ])
    .map_err(|e| ApiError::internal_error(&format!("Failed to build zip: {}", e)))?;

    // Build response with appropriate headers
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"hosts-export.zip\""),
    );

    Ok((headers, Body::from(zip_data)))
}

use crate::server::shared::legacy::rewrite_response_for_legacy_daemon;

/// Writes `files` (name, contents) into a Deflate-compressed zip archive, in the order given.
fn zip_archive(files: &[(&str, &[u8])]) -> ZipResult<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for (name, contents) in files {
        zip.start_file(*name, options)?;
        zip.write_all(contents)?;
    }

    Ok(zip.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use zip::ZipArchive;

    use super::*;

    #[test]
    fn zip_archive_reads_back_every_file() {
        let files: [(&str, &[u8]); 3] = [
            ("hosts.csv", b"id,name\n1,router\n"),
            ("ports.csv", b"id,number\n2,443\n"),
            ("empty.csv", b""),
        ];

        let bytes = zip_archive(&files).unwrap();

        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert_eq!(archive.len(), files.len());
        for (index, (name, contents)) in files.iter().enumerate() {
            let mut entry = archive.by_index(index).unwrap();
            assert_eq!(entry.name().unwrap(), *name);
            let mut read = Vec::new();
            entry.read_to_end(&mut read).unwrap();
            assert_eq!(&read, contents);
        }
    }
}
