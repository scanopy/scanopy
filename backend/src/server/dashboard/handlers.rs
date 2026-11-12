use crate::server::openapi::tags as api_tags;
use crate::server::{
    auth::{
        access::{Action, Resource},
        middleware::permissions::{Authorized, InOrganization},
    },
    config::AppState,
    shared::{
        services::traits::CrudService,
        storage::filter::StorableFilter,
        types::api::{ApiError, ApiResponse, ApiResult},
    },
    sites::r#impl::Site,
};

use super::types::{DashboardSummary, PlanUsage, SiteSummary};
use crate::server::{
    setup::{
        service::IssueList,
        types::{IssueFilterQuery, SetupItemTier, SetupListEntry},
    },
    shared::{extractors::Query, handlers::list::computed_page, types::api::PaginatedApiResponse},
};

use axum::{extract::State, response::Json};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_dashboard_summary))
        .routes(routes!(get_dashboard_issues))
}

/// Get dashboard summary
///
/// Returns site metrics, plan usage and the organization's Setup list as the caller sees it.
#[utoipa::path(
    get,
    path = "/summary",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Dashboard summary", body = ApiResponse<DashboardSummary>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_dashboard_summary(
    State(state): State<Arc<AppState>>,
    auth: Authorized<InOrganization>,
) -> ApiResult<Json<ApiResponse<DashboardSummary>>> {
    // The summary spans resource types, so each count reads the sites its own type allows.
    let readable = |resource| auth.entity.site_ids_for(resource, Action::Read);
    let host_site_ids = readable(Resource::Host);
    let service_site_ids = readable(Resource::Service);
    let subnet_site_ids = readable(Resource::Subnet);
    let daemon_site_ids = readable(Resource::Daemon);
    let site_ids = readable(Resource::Site);
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    // Fetch sites the user has access to (filter by id, not site_id)
    let sites_filter = StorableFilter::<Site>::new_from_entity_ids(&site_ids);
    let sites_result = state
        .services
        .site_service
        .get_paginated(sites_filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Build per-site summaries with counts. `count_for_sites` narrows
    // SCD2 entities to live rows itself, so snapshot closed-copies aren't
    // counted (daemons are non-SCD2 → plain count).
    let mut site_summaries = Vec::new();
    for site in &sites_result.items {
        let map_err = |e: anyhow::Error| ApiError::internal_error(&e.to_string());
        let within = |allowed: &[Uuid]| {
            allowed
                .contains(&site.id)
                .then_some(site.id)
                .into_iter()
                .collect::<Vec<Uuid>>()
        };
        site_summaries.push(SiteSummary {
            id: site.id,
            name: site.base.name.clone(),
            host_count: state
                .services
                .host_service
                .confirmed_count_for_sites(&within(&host_site_ids))
                .await
                .map_err(map_err)?,
            service_count: state
                .services
                .service_service
                .count_for_sites(&within(&service_site_ids))
                .await
                .map_err(map_err)?,
            subnet_count: state
                .services
                .subnet_service
                .count_for_sites(&within(&subnet_site_ids))
                .await
                .map_err(map_err)?,
            daemon_count: state
                .services
                .daemon_service
                .count_for_sites(&within(&daemon_site_ids))
                .await
                .map_err(map_err)?,
        });
    }

    let organization = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Organization not found".to_string()))?;
    let plan = &organization.base.plan;
    let (host_limit, site_limit, seat_limit) =
        (plan.host_limit(), plan.site_limit(), plan.seat_limit());

    // Total host count across all sites (live only) and org seat count.
    let host_count = state
        .services
        .host_service
        .confirmed_count_for_sites(&host_site_ids)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let seat_count = state
        .services
        .user_service
        .count_for_org(&organization_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    let (issues, setup_items): (Vec<SetupListEntry>, Vec<SetupListEntry>) = state
        .services
        .setup_service
        .for_user(&organization, &auth.entity)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .into_iter()
        .partition(|entry| entry.tier == SetupItemTier::Issue);
    let mut issue_site_ids: Vec<Uuid> = Vec::new();
    for site_id in issues.iter().flat_map(|entry| &entry.site_ids) {
        if !issue_site_ids.contains(site_id) {
            issue_site_ids.push(*site_id);
        }
    }

    let plan_usage = PlanUsage {
        host_limit,
        host_count,
        site_limit,
        site_count: sites_result.total_count,
        seat_limit,
        seat_count,
    };

    Ok(Json(ApiResponse::success(DashboardSummary {
        sites: site_summaries,
        plan_usage,
        setup_items,
        issue_count: issues.len() as u64,
        issue_site_ids,
    })))
}

/// List dashboard Issues
///
/// One page of the Issues the caller sees, sorted, grouped and filtered as asked. The Setup steps come with the
/// summary.
#[utoipa::path(
    get,
    path = "/issues",
    tags = [api_tags::INTERNAL],
    params(IssueFilterQuery),
    responses(
        (status = 200, description = "Issues", body = PaginatedApiResponse<SetupListEntry>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_dashboard_issues(
    State(state): State<Arc<AppState>>,
    auth: Authorized<InOrganization>,
    Query(query): Query<IssueFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<SetupListEntry>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;
    let organization = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Organization not found".to_string()))?;
    // Already cut to what the caller may read, so `site_ids` only narrows it: a site the caller
    // can't read matches nothing.
    let issues: Vec<SetupListEntry> = state
        .services
        .setup_service
        .for_user(&organization, &auth.entity)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .into_iter()
        .filter(|entry| entry.tier == SetupItemTier::Issue)
        .collect();
    Ok(Json(computed_page::<IssueList>(
        issues,
        &query,
        query.pagination(),
    )))
}
