use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::setup::types::SetupListEntry;

/// Per-site summary of entity counts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SiteSummary {
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// Name of the site.
    pub name: String,
    /// Hosts currently discovered on this site.
    pub host_count: u64,
    /// Services currently discovered on this site.
    pub service_count: u64,
    /// Subnets currently known on this site.
    pub subnet_count: u64,
    /// Daemons assigned to this site.
    pub daemon_count: u64,
}

/// Plan usage limits and current counts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlanUsage {
    /// Hosts included in the current plan. `null` when unlimited.
    pub host_limit: Option<u64>,
    /// Hosts currently counted against the plan.
    pub host_count: u64,
    /// Sites included in the current plan. `null` when unlimited.
    pub site_limit: Option<u64>,
    /// Sites currently counted against the plan.
    pub site_count: u64,
    /// Seats included in the current plan. `null` when unlimited.
    pub seat_limit: Option<u64>,
    /// Seats currently in use.
    pub seat_count: u64,
}

/// Dashboard summary response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardSummary {
    /// Per-site counts for every site the caller can see.
    pub sites: Vec<SiteSummary>,
    /// Current usage against the organization's plan allowances.
    pub plan_usage: PlanUsage,
    /// The organization's Setup steps as the caller sees them. Issues are paged from
    /// `/dashboard/issues`.
    pub setup_items: Vec<SetupListEntry>,
    /// The Issues the caller sees, on every site.
    pub issue_count: u64,
    /// The sites those Issues are on, which the Issues list can be narrowed to.
    pub issue_site_ids: Vec<Uuid>,
}
