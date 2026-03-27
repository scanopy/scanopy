use crate::server::credentials::r#impl::mapping::SnmpCredentialMapping;
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::discovery::r#impl::types::HostNamingFallback;
use crate::server::services::r#impl::patterns::MatchDetails;
use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use strum_macros::EnumDiscriminants;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(
    Debug, Clone, Serialize, Deserialize, Default, Eq, PartialEq, Hash, EnumDiscriminants, ToSchema,
)]
#[strum_discriminants(derive(Hash))]
#[serde(tag = "type")]
pub enum EntitySource {
    #[schema(title = "Manual")]
    Manual,
    #[default]
    #[schema(title = "System")]
    System,
    #[schema(title = "Discovery")]
    Discovery { metadata: Vec<DiscoveryMetadata> },
    #[schema(title = "DiscoveryWithMatch")]
    DiscoveryWithMatch {
        metadata: Vec<DiscoveryMetadata>,
        details: MatchDetails,
    },
    #[schema(title = "Unknown")]
    Unknown,
}

/// Maximum number of discovery metadata entries to retain per entity.
/// Newest entries are kept; older entries are pruned on upsert.
pub const MAX_DISCOVERY_METADATA_ENTRIES: usize = 10;

impl EntitySource {
    /// Returns true if this entity was created via discovery (network, Docker, etc.)
    pub fn is_from_discovery(&self) -> bool {
        matches!(
            self,
            EntitySource::Discovery { .. } | EntitySource::DiscoveryWithMatch { .. }
        )
    }

    /// Truncate discovery metadata to the most recent entries.
    /// Since new entries are prepended, this keeps the newest.
    pub fn cap_metadata(mut self) -> Self {
        match &mut self {
            EntitySource::Discovery { metadata }
            | EntitySource::DiscoveryWithMatch { metadata, .. } => {
                metadata.truncate(MAX_DISCOVERY_METADATA_ENTRIES);
            }
            _ => {}
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct DiscoveryMetadata {
    #[serde(flatten)]
    pub discovery_type: DiscoveryType,
    pub daemon_id: Uuid,
    pub date: DateTime<Utc>,
}

impl DiscoveryMetadata {
    pub fn new(discovery_type: DiscoveryType, daemon_id: Uuid) -> Self {
        Self {
            // SNMP credentials are wrapped in Secret<String> and automatically
            // redacted during serialization — no manual sanitization needed
            discovery_type,
            daemon_id,
            date: Utc::now(),
        }
    }
}

impl Default for DiscoveryMetadata {
    fn default() -> Self {
        Self {
            discovery_type: DiscoveryType::Network {
                subnet_ids: None,
                host_naming_fallback: HostNamingFallback::BestService,
                snmp_credentials: SnmpCredentialMapping::default(),
            },
            daemon_id: Uuid::new_v4(),
            date: Utc::now(),
        }
    }
}
