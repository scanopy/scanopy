use crate::server::{
    auth::middleware::permissions::{Authorized, Viewer},
    config::AppState,
    daemons::r#impl::{base::Daemon, version::DaemonVersionPolicy},
    discovery::r#impl::base::Discovery,
    hosts::r#impl::base::Host,
    networks::r#impl::Network,
    services::r#impl::base::Service,
    shared::{
        services::traits::CrudService,
        storage::filter::StorableFilter,
        types::api::{ApiError, ApiResponse, ApiResult},
    },
    subnets::r#impl::base::Subnet,
    users::r#impl::base::User,
};

use super::types::{DaemonSummary, DashboardSummary, NetworkSummary, PlanUsage};

use axum::{extract::State, response::Json};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(get_dashboard_summary))
}

/// Get dashboard summary
///
/// Returns aggregated dashboard data including network metrics, daemon health,
/// recent discoveries, and plan usage.
#[utoipa::path(
    get,
    path = "/summary",
    tags = ["dashboard", "internal"],
    responses(
        (status = 200, description = "Dashboard summary", body = ApiResponse<DashboardSummary>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_dashboard_summary(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
) -> ApiResult<Json<ApiResponse<DashboardSummary>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    // Fetch networks
    let networks_filter = StorableFilter::<Network>::new_from_network_ids(&network_ids);
    let networks_result = state
        .services
        .network_service
        .get_paginated(networks_filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Build per-network summaries with counts
    let mut network_summaries = Vec::new();
    for network in &networks_result.items {
        let host_filter = StorableFilter::<Host>::new_from_network_ids(&[network.id]).limit(0);
        let host_result = state
            .services
            .host_service
            .get_paginated(host_filter)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        let service_filter =
            StorableFilter::<Service>::new_from_network_ids(&[network.id]).limit(0);
        let service_result = state
            .services
            .service_service
            .get_paginated(service_filter)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        let subnet_filter = StorableFilter::<Subnet>::new_from_network_ids(&[network.id]).limit(0);
        let subnet_result = state
            .services
            .subnet_service
            .get_paginated(subnet_filter)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        let daemon_filter = StorableFilter::<Daemon>::new_from_network_ids(&[network.id]).limit(0);
        let daemon_result = state
            .services
            .daemon_service
            .get_paginated(daemon_filter)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        network_summaries.push(NetworkSummary {
            id: network.id,
            name: network.base.name.clone(),
            host_count: host_result.total_count,
            service_count: service_result.total_count,
            subnet_count: subnet_result.total_count,
            daemon_count: daemon_result.total_count,
        });
    }

    // Fetch all daemons with version status
    let daemons_filter = StorableFilter::<Daemon>::new_from_network_ids(&network_ids);
    let daemons_result = state
        .services
        .daemon_service
        .get_paginated(daemons_filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    let policy = DaemonVersionPolicy::default();
    let daemon_summaries: Vec<DaemonSummary> = daemons_result
        .items
        .into_iter()
        .map(|d| {
            let version_status = policy.evaluate(d.base.version.as_ref());
            DaemonSummary {
                id: d.id,
                name: d.base.name,
                network_id: d.base.network_id,
                last_seen: d.base.last_seen,
                is_unreachable: d.base.is_unreachable,
                version_status,
            }
        })
        .collect();

    // Fetch recent historical discoveries (last 5)
    let discovery_filter = StorableFilter::<Discovery>::new_from_network_ids(&network_ids)
        .historical_discovery()
        .limit(5);
    let discovery_result = state
        .services
        .discovery_service
        .get_paginated_ordered(discovery_filter, "created_at DESC")
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // Plan usage
    let org = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Organization not found".to_string()))?;

    let (host_limit, network_limit, seat_limit) = match &org.base.plan {
        Some(plan) => (plan.host_limit(), plan.network_limit(), plan.seat_limit()),
        None => (None, None, None),
    };

    // Total host count across all networks
    let total_host_filter = StorableFilter::<Host>::new_from_network_ids(&network_ids).limit(0);
    let total_host_result = state
        .services
        .host_service
        .get_paginated(total_host_filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // User (seat) count for the organization
    let user_filter = StorableFilter::<User>::new_from_org_id(&organization_id).limit(0);
    let user_result = state
        .services
        .user_service
        .get_paginated(user_filter)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    let plan_usage = PlanUsage {
        host_limit,
        host_count: total_host_result.total_count,
        network_limit,
        network_count: networks_result.total_count,
        seat_limit,
        seat_count: user_result.total_count,
    };

    Ok(Json(ApiResponse::success(DashboardSummary {
        networks: network_summaries,
        daemons: daemon_summaries,
        recent_discoveries: discovery_result.items,
        plan_usage,
    })))
}
