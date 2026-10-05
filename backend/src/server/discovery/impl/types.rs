use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use strum::{Display, EnumDiscriminants, EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;
use uuid::Uuid;

use std::net::IpAddr;

use crate::server::credentials::r#impl::mapping::SnmpCredentialMapping;
use crate::server::discovery::r#impl::scan_settings::{RescanSettings, ScanSettings};
use crate::server::ports::r#impl::base::PortType;
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
    VariantNames,
    ToSchema,
)]
#[serde(tag = "type")]
pub enum DiscoveryType {
    #[schema(title = "SelfReport")]
    SelfReport {
        /// The host the daemon is running on.
        host_id: Uuid,
    },
    #[schema(title = "Network")]
    Network {
        /// Subnets to sweep. `null` sweeps every subnet on the site.
        #[schema(required)]
        subnet_ids: Option<Vec<Uuid>>,
        /// What to name a host by when reverse DNS gives nothing.
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
        /// The host the daemon is running on.
        host_id: Uuid,
        /// What to name a host by when reverse DNS gives nothing.
        #[serde(default)]
        #[schema(required)]
        host_naming_fallback: HostNamingFallback,
    },
    /// A one-shot verification of a single host: re-check the addresses and
    /// ports already recorded for it, rather than sweeping a subnet.
    ///
    /// Created by the server only (never via the API) and deleted once its
    /// session reaches a terminal phase, so it is not a discovery configuration
    /// anyone owns or sees in their scan list.
    #[schema(title = "Rescan")]
    Rescan {
        /// ID of the host that the daemon is running on — same meaning as every
        /// other variant. The host being rescanned is `target_host_id`.
        host_id: Uuid,
        /// The host being rescanned.
        target_host_id: Uuid,
        /// Addresses to scan on that host.
        #[schema(value_type = Vec<String>)]
        ips: Vec<IpAddr>,
        /// Ports already known on that host, re-checked to confirm they are
        /// still open. Scanned in addition to the standard discovery set, so a
        /// rescan also surfaces newly-opened services.
        #[serde(default)]
        ports: Vec<PortType>,
        #[serde(default)]
        settings: RescanSettings,
    },
    #[schema(title = "Unified")]
    Unified {
        /// ID of the host that the daemon is running on
        host_id: Uuid,
        /// Subnets to scan. None = scan all interfaced subnets.
        #[schema(required)]
        subnet_ids: Option<Vec<Uuid>>,
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

    /// The host a one-shot rescan is verifying. `None` for every other type,
    /// so `Some` also answers "is this a rescan".
    pub fn rescan_target_host_id(&self) -> Option<Uuid> {
        match self {
            DiscoveryType::Rescan { target_host_id, .. } => Some(*target_host_id),
            DiscoveryType::SelfReport { .. }
            | DiscoveryType::Network { .. }
            | DiscoveryType::Docker { .. }
            | DiscoveryType::Unified { .. } => None,
        }
    }

    /// Whether the daemon runs the full scan pipeline for this type — i.e. it
    /// needs credential mappings and the settings the server computes.
    pub fn runs_network_scan(&self) -> bool {
        matches!(
            self,
            DiscoveryType::Unified { .. } | DiscoveryType::Rescan { .. }
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
            DiscoveryType::Rescan { .. } => write!(f, "Rescan"),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames, ToSchema)]
#[serde(tag = "type")]
pub enum RunType {
    #[schema(title = "Scheduled")]
    Scheduled {
        /// Cron expression deciding when the scan runs.
        cron_schedule: String,
        /// When the scan last ran.
        #[serde(default)]
        #[schema(read_only)]
        last_run: Option<DateTime<Utc>>,
        /// Whether the schedule is active.
        enabled: bool,
        /// IANA timezone for cron evaluation, e.g. "America/New_York". None = UTC.
        #[serde(default)]
        timezone: Option<String>,
    },
    #[schema(title = "Historical")]
    /// Historical discovery runs are created by the server and cannot be submitted via API
    Historical {
        /// The recorded outcome of the run.
        results: Box<DiscoveryUpdatePayload>,
    },
    #[schema(title = "AdHoc")]
    AdHoc {
        /// When the scan last ran.
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

/// The parts of a `Scheduled` run needed to drive the cron scheduler.
pub struct ScheduleParts<'a> {
    pub cron_schedule: &'a str,
    pub enabled: bool,
    pub timezone: Option<&'a str>,
}

impl RunType {
    /// When this discovery last started a session, if it tracks that.
    pub fn last_run(&self) -> Option<DateTime<Utc>> {
        match self {
            RunType::Scheduled { last_run, .. } | RunType::AdHoc { last_run } => *last_run,
            // Historical runs *are* a completed run; `results.finished_at` is the
            // timestamp that means anything for them.
            RunType::Historical { .. } => None,
        }
    }

    /// Stamp the time a session started. No-op for variants that don't track it.
    pub fn set_last_run(&mut self, at: DateTime<Utc>) {
        match self {
            RunType::Scheduled { last_run, .. } | RunType::AdHoc { last_run } => {
                *last_run = Some(at)
            }
            RunType::Historical { .. } => {}
        }
    }

    /// Cron parameters, for the variants the scheduler drives.
    pub fn schedule(&self) -> Option<ScheduleParts<'_>> {
        match self {
            RunType::Scheduled {
                cron_schedule,
                enabled,
                timezone,
                ..
            } => Some(ScheduleParts {
                cron_schedule,
                enabled: *enabled,
                timezone: timezone.as_deref(),
            }),
            RunType::Historical { .. } | RunType::AdHoc { .. } => None,
        }
    }

    /// Whether the cron scheduler should hold a job for this discovery.
    pub fn is_scheduled_enabled(&self) -> bool {
        self.schedule().is_some_and(|s| s.enabled)
    }

    /// Created by the server only — rejected on the public create/update routes.
    pub fn is_server_managed(&self) -> bool {
        match self {
            RunType::Historical { .. } => true,
            RunType::Scheduled { .. } | RunType::AdHoc { .. } => false,
        }
    }

    /// Whether this row is a discovery configuration a user owns and sees —
    /// i.e. not a historical record and not a transient rescan. Mirrors the
    /// `exclude_ephemeral` storage filter.
    pub fn is_live_config(&self) -> bool {
        !self.is_server_managed()
    }

    /// The completed session this row records, for historical runs.
    pub fn historical_results(&self) -> Option<&DiscoveryUpdatePayload> {
        match self {
            RunType::Historical { results } => Some(results),
            RunType::Scheduled { .. } | RunType::AdHoc { .. } => None,
        }
    }

    /// Whether this discovery has never started a session.
    pub fn never_ran(&self) -> bool {
        match self {
            RunType::Scheduled { last_run, .. } | RunType::AdHoc { last_run } => last_run.is_none(),
            RunType::Historical { .. } => false,
        }
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
            DiscoveryType::Rescan { .. } => "Re-check a single host's known addresses and ports",
            DiscoveryType::Unified { .. } => {
                "Unified discovery combining self-report, network scanning, and Docker container detection"
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            // Frozen types the daemon no longer runs. Surfaced so the UI can
            // flag them without keeping its own copy of the list — `Rescan` is
            // new, not legacy, and a hardcoded `!= Unified` check said otherwise.
            "is_legacy": self.is_legacy(),
        })
    }
}
