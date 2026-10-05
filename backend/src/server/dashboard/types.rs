use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::daemons::r#impl::api::DaemonResponse;
use crate::server::discovery::r#impl::base::Discovery;

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
    /// Daemons the caller can see, with their current status.
    pub daemons: Vec<DaemonResponse>,
    /// The most recent discovery runs, newest first.
    pub recent_discoveries: Vec<Discovery>,
    /// Current usage against the organization's plan allowances.
    pub plan_usage: PlanUsage,
}
