use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    discovery::r#impl::types::{DiscoveryType, RunType},
    shared::entities::ChangeTriggersTopologyStaleness,
};

#[derive(
    Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, Default, ToSchema, Validate,
)]
pub struct DiscoveryBase {
    pub discovery_type: DiscoveryType,
    pub run_type: RunType,
    pub name: String,
    pub daemon_id: Uuid,
    pub network_id: Uuid,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct Discovery {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: DiscoveryBase,
    /// Number of completed scans (incremented by server on session completion)
    #[serde(default)]
    #[schema(read_only)]
    pub scan_count: u32,
    /// When true, the next scan will be a full port scan regardless of interval
    #[serde(default)]
    pub force_full_scan: bool,
    /// Credential IDs to include in the next scan's credential_mappings.
    /// Set by the discovery edit modal, cleared after each scan completes.
    #[serde(default)]
    pub pending_credential_ids: Vec<Uuid>,
}

impl Discovery {
    pub fn disable(&mut self) {
        if let RunType::Scheduled {
            ref mut enabled, ..
        } = self.base.run_type
        {
            *enabled = false;
        }
    }
}

impl Display for Discovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Discovery {}: {}", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<Discovery> for Discovery {
    fn triggers_staleness(&self, _other: Option<Discovery>) -> bool {
        false
    }
}
