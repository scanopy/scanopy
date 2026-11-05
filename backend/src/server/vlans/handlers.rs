use crate::server::auth::middleware::permissions::{Authorized, Can, IsDaemon, Read, Write};
use crate::server::openapi::tags as api_tags;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::handlers::list::list_page;
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams, on_requested_sites,
};
use crate::server::shared::handlers::traits::ListScope;
use crate::server::shared::handlers::traits::create_handler;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, PaginatedApiResponse};
use crate::server::shared::types::entities::EntitySourceDiscriminants;
use crate::server::vlans::r#impl::base::Vlan;
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult},
};
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Vlan Ordering
// ============================================================================

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum VlanOrderField {
    #[default]
    CreatedAt,
    Name,
    VlanNumber,
    UpdatedAt,
    SiteId,
    /// Sort by how the VLAN came to exist (`source.type`).
    Source,
    /// Sort by when discovery last observed the VLAN.
    LastSeenAt,
}

impl OrderField for VlanOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "vlans.created_at",
            Self::Name => "vlans.name",
            Self::VlanNumber => "vlans.vlan_number",
            Self::UpdatedAt => "vlans.updated_at",
            Self::SiteId => "vlans.site_id",
            Self::Source => "vlans.source->>'type'",
            Self::LastSeenAt => "vlans.last_seen_at",
        }
    }

    fn sorts_by_site_name(&self) -> bool {
        matches!(self, Self::SiteId)
    }
}

// ============================================================================
// Vlan Filter Query
// ============================================================================

#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct VlanFilterQuery {
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<VlanOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<VlanOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
    /// Filter by site ID. Repeat the parameter to pass several.
    #[serde(alias = "site_id")]
    pub site_ids: Option<Vec<Uuid>>,
    /// Filter by how the VLAN came to exist (`source.type`). Repeat for several.
    pub sources: Option<Vec<EntitySourceDiscriminants>>,
    /// Filter to VLANs linked to one of these subnets. Repeat for several.
    pub subnet_ids: Option<Vec<Uuid>>,
    /// Filter by tag IDs (returns VLANs that have ANY of the specified tags)
    pub tag_ids: Option<Vec<Uuid>>,
    /// Tag sets the VLAN must hold one tag from each of. Each entry is a comma-separated list
    /// of tag ids. One entry per tag group the list is filtered by.
    pub tag_sets: Option<Vec<String>>,
    /// Group by the tag each row holds from this tag group. Takes the place of `group_by`.
    pub group_by_tag_group: Option<Uuid>,
    /// Sort by the position of the tag each row holds from this tag group. Takes the place of
    /// `order_by`.
    pub order_by_tag_group: Option<Uuid>,
    /// As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
    /// instant (snapshot view) instead of live state.
    pub at: Option<chrono::DateTime<chrono::Utc>>,
    /// Free-text search. Case-insensitive substring match against the VLAN ID,
    /// name and description.
    pub search: Option<String>,
}

impl FilterQueryExtractor for VlanFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        let filter = on_requested_sites(filter, self.site_ids.as_deref(), user_site_ids);
        match &self.sources {
            Some(sources) if !sources.is_empty() => filter.source_type_in(sources),
            _ => filter,
        }
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }

    fn at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.at
    }
}

// Generated handlers for most CRUD operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Vlan);
    crate::crud_update_handler!(Vlan);
    crate::crud_delete_handler!(Vlan);
    crate::crud_bulk_delete_handler!(Vlan);
    crate::crud_export_csv_handler!(Vlan);
    crate::crud_get_field_values_handler!(Vlan);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_vlans, create_vlan))
        .routes(routes!(
            generated::get_by_id,
            generated::update,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
        .routes(routes!(generated::get_field_values))
        .routes(routes!(discovery_upsert_vlans))
}

/// List VLANs
///
/// Returns VLANs accessible to the authenticated user, optionally filtered by site.
#[utoipa::path(
    get,
    operation_id = "list_vlans",
    path = "",
    tag = Vlan::ENTITY_NAME_PLURAL,
    params(VlanFilterQuery),
    responses(
        (status = 200, description = "List of VLANs", body = PaginatedApiResponse<Vlan>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_vlans(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Vlan, Read>>,
    crate::server::shared::extractors::Query(query): crate::server::shared::extractors::Query<
        VlanFilterQuery,
    >,
) -> ApiResult<Json<PaginatedApiResponse<Vlan>>> {
    let scope = ListScope::of(&auth)?;
    // Rows come through the service, which hydrates each VLAN's `subnet_ids` and
    // `interface_ids`.
    Ok(Json(list_page::<Vlan>(&state, &scope, &query).await?))
}

/// Create VLAN
///
/// Creates a VLAN scoped to a site. VLAN numbers must be unique within a site.
#[utoipa::path(
    post,
    path = "",
    tag = Vlan::ENTITY_NAME_PLURAL,
    request_body = Vlan,
    responses(
        (status = 200, description = "VLAN created successfully", body = ApiResponse<Vlan>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
        (status = 409, description = "VLAN number already exists in this site", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_vlan(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Vlan, Write>>,
    ApiJson(vlan): ApiJson<Vlan>,
) -> ApiResult<Json<ApiResponse<Vlan>>> {
    let site_ids = auth.site_ids();

    // Verify user has access to the target site
    if !site_ids.contains(&vlan.base.site_id) {
        return Err(ApiError::forbidden("Access denied to this site"));
    }

    // Check uniqueness: (site_id, vlan_number)
    let existing_filter =
        StorableFilter::<Vlan>::new_from_uuid_column("site_id", &vlan.base.site_id)
            .u16_column("vlan_number", vlan.base.vlan_number);

    if state.services.vlan_service.exists(existing_filter).await? {
        return Err(ApiError::conflict(&format!(
            "VLAN {} already exists in this site",
            vlan.base.vlan_number
        )));
    }

    create_handler::<Vlan>(State(state), auth, ApiJson(vlan)).await
}

// ============================================================================
// Discovery Upsert
// ============================================================================

/// Request body for daemon VLAN discovery upsert
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VlanDiscoveryRequest {
    /// The site this entity belongs to.
    pub site_id: Uuid,
    /// VLANs observed by the daemon.
    pub vlans: Vec<VlanDiscoveryItem>,
}

/// Upper bound on VLANs in one discovery submission: the 802.1Q ID space (1-4094).
const MAX_VLANS_PER_DISCOVERY: usize = 4094;

impl VlanDiscoveryRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if self.vlans.len() > MAX_VLANS_PER_DISCOVERY {
            return Err(ApiError::bad_request(&format!(
                "A discovery submission can contain at most {} VLANs, got {}",
                MAX_VLANS_PER_DISCOVERY,
                self.vlans.len()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct VlanDiscoveryItem {
    /// 802.1Q VLAN ID.
    pub vlan_number: u16,
    /// VLAN name as configured on the device.
    pub name: String,
    /// Where the daemon read the name. Defaults to an SNMP read: daemons that predate this field
    /// sent VLANs from SNMP alone.
    #[serde(default = "snmp_vlan_name_source")]
    pub name_source: AttributeSource,
}

fn snmp_vlan_name_source() -> AttributeSource {
    AttributeSource::Authored(ClientProbe::Snmp)
}

/// Response for discovery upsert
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VlanDiscoveryResponse {
    /// Mapping of vlan_number → VLAN entity UUID
    pub vlans: Vec<VlanDiscoveryResponseItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VlanDiscoveryResponseItem {
    /// 802.1Q VLAN ID.
    pub vlan_number: u16,
    /// Server-assigned unique identifier.
    pub id: Uuid,
}

/// Bulk upsert VLANs from discovery
///
/// Used by daemons to report discovered VLANs. Creates new VLANs or updates names.
/// Returns the mapping of VLAN numbers to entity UUIDs for Interface construction.
#[utoipa::path(
    post,
    path = "/discovery",
    tags = [Vlan::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    request_body = VlanDiscoveryRequest,
    responses(
        (status = 200, description = "VLANs upserted", body = ApiResponse<VlanDiscoveryResponse>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
pub async fn discovery_upsert_vlans(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    ApiJson(request): ApiJson<VlanDiscoveryRequest>,
) -> ApiResult<Json<ApiResponse<VlanDiscoveryResponse>>> {
    request.validate()?;

    // Daemons don't carry org_id directly — resolve from the site
    let site = state
        .services
        .site_service
        .get_by_id(&request.site_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Site not found".to_string()))?;
    let organization_id = site.base.organization_id;

    // Verify daemon has access to this site
    let daemon_site_ids = auth.site_ids();
    if !daemon_site_ids.contains(&request.site_id) {
        return Err(ApiError::forbidden(
            "Daemon cannot create VLANs on sites it's not assigned to",
        ));
    }

    // Capture one scan_time for the whole submission so all VLANs share
    // consistent SCD2 origination timestamps. See ScanContext for rationale.
    let scan_ctx = auth.daemon_id().map(|daemon_id| {
        crate::server::shared::services::scan_context::ScanContext::new(daemon_id)
    });

    let response = state
        .services
        .vlan_service
        .upsert_discovery_batch(
            request.site_id,
            organization_id,
            request.vlans,
            scan_ctx.as_ref(),
        )
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to upsert VLAN: {}", e)))?;

    Ok(Json(ApiResponse::success(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    fn request_with(count: usize) -> VlanDiscoveryRequest {
        VlanDiscoveryRequest {
            site_id: Uuid::new_v4(),
            vlans: (0..count)
                .map(|i| VlanDiscoveryItem {
                    vlan_number: (i % MAX_VLANS_PER_DISCOVERY) as u16 + 1,
                    name: String::new(),
                    name_source: snmp_vlan_name_source(),
                })
                .collect(),
        }
    }

    #[test]
    fn discovery_request_at_bound_is_accepted() {
        assert!(request_with(MAX_VLANS_PER_DISCOVERY).validate().is_ok());
    }

    #[test]
    fn discovery_request_over_bound_is_rejected() {
        let err = request_with(MAX_VLANS_PER_DISCOVERY + 1)
            .validate()
            .unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    /// Daemons older than `name_source` POST `{vlan_number, name}`. Their names came from SNMP,
    /// and must keep the rung that lets the same switch rename them later.
    #[test]
    fn a_request_without_a_name_source_reads_as_an_snmp_name() {
        let item: VlanDiscoveryItem =
            serde_json::from_str(r#"{"vlan_number": 20, "name": "Servers"}"#).unwrap();
        assert_eq!(
            item.name_source,
            AttributeSource::Authored(ClientProbe::Snmp)
        );
    }
}
