use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use strum::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::SnmpCredentialMapping;
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::{
    daemons::r#impl::api::DiscoveryUpdatePayload,
    shared::types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    IntoStaticStr,
    EnumDiscriminants,
    EnumIter,
    ToSchema,
)]
#[serde(tag = "type")]
pub enum DiscoveryType {
    #[schema(title = "SelfReport")]
    SelfReport {
        // ID of the host that the daemon is running on
        host_id: Uuid,
    },
    #[schema(title = "Network")]
    Network {
        #[schema(required)]
        subnet_ids: Option<Vec<Uuid>>,
        #[serde(default)]
        #[schema(required)]
        host_naming_fallback: HostNamingFallback,
        /// SNMP credentials for querying devices during discovery
        /// Server builds this mapping before initiating discovery
        #[serde(default)]
        #[schema(value_type = Object)]
        snmp_credentials: SnmpCredentialMapping,
    },
    #[schema(title = "Docker")]
    Docker {
        // ID of the host that the daemon is running on
        host_id: Uuid,
        #[serde(default)]
        #[schema(required)]
        host_naming_fallback: HostNamingFallback,
    },
    #[schema(title = "Unified")]
    Unified {
        /// ID of the host that the daemon is running on
        host_id: Uuid,
        /// Subnets to scan. None = scan all interfaced subnets.
        #[schema(required)]
        subnet_ids: Option<Vec<Uuid>>,
        /// Whether to scan the local Docker socket for containers
        #[serde(default)]
        scan_local_docker_socket: bool,
        /// Fallback strategy for naming discovered hosts
        #[serde(default)]
        #[schema(required)]
        host_naming_fallback: HostNamingFallback,
        /// Per-discovery scan performance settings
        #[serde(default)]
        scan_settings: ScanSettings,
    },
}

impl Default for DiscoveryType {
    fn default() -> Self {
        Self::SelfReport {
            host_id: Uuid::nil(),
        }
    }
}

impl DiscoveryType {
    /// Returns true for legacy discovery types (SelfReport, Network, Docker).
    /// Legacy types are frozen and cannot be created — only Unified is allowed.
    pub fn is_legacy(&self) -> bool {
        matches!(
            self,
            DiscoveryType::SelfReport { .. }
                | DiscoveryType::Network { .. }
                | DiscoveryType::Docker { .. }
        )
    }

    /// Serialize with SNMP credentials exposed as plaintext.
    /// Used only for daemon transmission where the daemon needs actual credentials.
    ///
    /// We patch the serde_json::Value rather than duplicating the entire internally-tagged
    /// enum, since DiscoveryType has multiple variants and #[serde(tag = "type")] flattening.
    /// ResolvableSecret redacts community by default; this replaces each redacted
    /// value with the actual plaintext for daemon consumption.
    pub fn with_exposed_snmp(&self) -> serde_json::Value {
        use crate::server::credentials::r#impl::mapping::SnmpCredentialMappingExposed;

        let mut value = serde_json::to_value(self).unwrap_or_default();
        if let DiscoveryType::Network {
            snmp_credentials, ..
        } = self
            && let serde_json::Value::Object(ref mut map) = value
        {
            let exposed: SnmpCredentialMappingExposed = snmp_credentials.into();
            if let Ok(exposed_value) = serde_json::to_value(&exposed) {
                map.insert("snmp_credentials".to_string(), exposed_value);
            }
        }
        value
    }
}

impl Display for DiscoveryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryType::SelfReport { .. } => write!(f, "Self Report"),
            DiscoveryType::Network { .. } => write!(f, "Network Discovery"),
            DiscoveryType::Docker { .. } => write!(f, "Docker Discovery"),
            DiscoveryType::Unified { .. } => write!(f, "Unified Discovery"),
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Copy, Deserialize, Eq, PartialEq, Hash, Display, Default, ToSchema,
)]
pub enum HostNamingFallback {
    Ip,
    #[default]
    BestService,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "type")]
pub enum RunType {
    #[schema(title = "Scheduled")]
    Scheduled {
        cron_schedule: String,
        #[serde(default)]
        #[schema(read_only)]
        last_run: Option<DateTime<Utc>>,
        enabled: bool,
        /// IANA timezone for cron evaluation, e.g. "America/New_York". None = UTC.
        #[serde(default)]
        timezone: Option<String>,
    },
    #[schema(title = "Historical")]
    /// Historical discovery runs are created by the server and cannot be submitted via API
    Historical {
        results: Box<DiscoveryUpdatePayload>,
    },
    #[schema(title = "AdHoc")]
    AdHoc {
        #[serde(default)]
        #[schema(read_only)]
        last_run: Option<DateTime<Utc>>,
    },
}

impl Default for RunType {
    fn default() -> Self {
        Self::AdHoc { last_run: None }
    }
}

impl HasId for DiscoveryType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for DiscoveryType {
    fn color(&self) -> Color {
        EntityDiscriminants::Discovery.color()
    }

    fn icon(&self) -> Icon {
        EntityDiscriminants::Discovery.icon()
    }
}

impl TypeMetadataProvider for DiscoveryType {
    fn name(&self) -> &'static str {
        self.id()
    }
    fn description(&self) -> &'static str {
        match self {
            DiscoveryType::Docker { .. } => {
                "Discover Docker containers and their configurations on the daemon's host"
            }
            DiscoveryType::Network { .. } => {
                "Scan network subnets to discover hosts, open ports, and running services"
            }
            DiscoveryType::SelfReport { .. } => {
                "The daemon reports its own host configuration and network details"
            }
            DiscoveryType::Unified { .. } => {
                "Unified discovery combining self-report, network scanning, and Docker container detection"
            }
        }
    }
}
