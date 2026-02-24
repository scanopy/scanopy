use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::daemons::r#impl::version::DaemonVersionStatus;
use crate::server::discovery::r#impl::base::Discovery;

/// Per-network summary of entity counts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkSummary {
    pub id: Uuid,
    pub name: String,
    pub host_count: u64,
    pub service_count: u64,
    pub subnet_count: u64,
    pub daemon_count: u64,
}

/// Daemon health summary
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonSummary {
    pub id: Uuid,
    pub name: String,
    pub network_id: Uuid,
    pub last_seen: Option<DateTime<Utc>>,
    pub is_unreachable: bool,
    pub version_status: DaemonVersionStatus,
}

/// Plan usage limits and current counts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlanUsage {
    pub host_limit: Option<u64>,
    pub host_count: u64,
    pub network_limit: Option<u64>,
    pub network_count: u64,
    pub seat_limit: Option<u64>,
    pub seat_count: u64,
}

/// Dashboard summary response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardSummary {
    pub networks: Vec<NetworkSummary>,
    pub daemons: Vec<DaemonSummary>,
    pub recent_discoveries: Vec<Discovery>,
    pub plan_usage: PlanUsage,
}
