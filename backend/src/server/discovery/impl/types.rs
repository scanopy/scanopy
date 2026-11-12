use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use strum::{Display, EnumIter, IntoStaticStr, VariantNames};
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
    /// A one-shot verification of known hosts: re-check the addresses and ports
    /// already recorded for them, rather than sweeping a subnet. One rescan
    /// covers every requested host that the same daemon last found.
    ///
    /// Created by the server only (never via the API) and deleted once its
    /// session reaches a terminal phase, so it is not a discovery configuration
    /// anyone owns or sees in their scan list.
    #[schema(title = "Rescan")]
    Rescan {
        /// ID of the host that the daemon is running on — same meaning as every
        /// other variant. The hosts being rescanned are `target_host_ids`.
        host_id: Uuid,
        /// The hosts being rescanned. Reads the single `target_host_id` that
        /// rescans stored before multi-host rescans carry, and that older
        /// daemons echo back, as a list of one.
        #[serde(
            alias = "target_host_id",
            deserialize_with = "deserialize_target_host_ids"
        )]
        target_host_ids: Vec<Uuid>,
        /// Addresses to scan on those hosts.
        #[schema(value_type = Vec<String>)]
        ips: Vec<IpAddr>,
        /// Ports already known on those hosts, re-checked on every address to
        /// confirm they are still open. Scanned in addition to the standard
        /// discovery set, so a rescan also surfaces newly-opened services.
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
    /// Classify the daemon's interfaced subnets again and report them, scanning nothing. The
    /// server starts one when the daemon's interfaces change.
    ///
    /// Created by the server only, with no discovery configuration behind it, and kept out of
    /// the run history, the digest and the active sessions.
    #[schema(title = "InterfaceRefresh")]
    InterfaceRefresh {
        /// ID of the host that the daemon is running on
        host_id: Uuid,
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

    /// The hosts a one-shot rescan is verifying. `None` for every other type,
    /// so `Some` also answers "is this a rescan".
    pub fn rescan_target_host_ids(&self) -> Option<&[Uuid]> {
        match self {
            DiscoveryType::Rescan {
                target_host_ids, ..
            } => Some(target_host_ids),
            DiscoveryType::SelfReport { .. }
            | DiscoveryType::Network { .. }
            | DiscoveryType::Docker { .. }
            | DiscoveryType::Unified { .. }
            | DiscoveryType::InterfaceRefresh { .. } => None,
        }
    }

    /// The host the daemon runs on. `None` for the legacy `Network` type, which never carried it.
    pub fn daemon_host_id(&self) -> Option<Uuid> {
        match self {
            DiscoveryType::SelfReport { host_id }
            | DiscoveryType::Docker { host_id, .. }
            | DiscoveryType::Rescan { host_id, .. }
            | DiscoveryType::Unified { host_id, .. }
            | DiscoveryType::InterfaceRefresh { host_id } => Some(*host_id),
            DiscoveryType::Network { .. } => None,
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

    /// Whether the daemon is sent credential mappings for this type. An interface refresh reads
    /// the host's container runtimes, so it gets their credentials (see
    /// [`Self::keeps_credential`]) without running a scan.
    pub fn carries_credential_mappings(&self) -> bool {
        self.runs_network_scan() || matches!(self, DiscoveryType::InterfaceRefresh { .. })
    }

    /// Whether a credential of type `credential` goes to the daemon with this type. An interface
    /// refresh needs only the container runtimes' API credentials.
    pub fn keeps_credential(
        &self,
        credential: crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants,
    ) -> bool {
        use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants as D;
        match self {
            DiscoveryType::InterfaceRefresh { .. } => matches!(
                credential,
                D::DockerProxy | D::DockerSocket | D::PodmanProxy | D::PodmanSocket
            ),
            _ => true,
        }
    }

    /// Whether a session of this type is recorded and shown: a historical row, the digest,
    /// session events, and the live session list. An interface refresh is not.
    pub fn records_history(&self) -> bool {
        !matches!(self, DiscoveryType::InterfaceRefresh { .. })
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
            DiscoveryType::InterfaceRefresh { .. } => write!(f, "Interface Refresh"),
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

/// Reads `Rescan.target_host_ids` from either a list or the single id rescans
/// carried before they could cover several hosts. A rescan always targets at
/// least one host, so an empty list is refused.
fn deserialize_target_host_ids<'de, D>(deserializer: D) -> Result<Vec<Uuid>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(Uuid),
        Many(Vec<Uuid>),
    }

    let ids = match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(id) => vec![id],
        OneOrMany::Many(ids) => ids,
    };
    if ids.is_empty() {
        return Err(serde::de::Error::custom(
            "a rescan targets at least one host",
        ));
    }
    Ok(ids)
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
            DiscoveryType::Rescan { .. } => "Re-check known hosts' addresses and ports",
            DiscoveryType::Unified { .. } => {
                "Unified discovery combining self-report, network scanning, and integrations"
            }
            DiscoveryType::InterfaceRefresh { .. } => {
                "Re-read which subnets the daemon's host is attached to"
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rescan_json(targets: serde_json::Value, key: &str) -> serde_json::Value {
        let mut json = serde_json::json!({
            "type": "Rescan",
            "host_id": Uuid::nil(),
            "ips": ["10.0.0.1"],
        });
        json[key] = targets;
        json
    }

    #[test]
    fn a_rescan_reads_a_stored_single_target_as_a_list_of_one() {
        let id = Uuid::new_v4();
        let parsed: DiscoveryType =
            serde_json::from_value(rescan_json(serde_json::json!(id), "target_host_id")).unwrap();
        assert_eq!(parsed.rescan_target_host_ids(), Some(&[id][..]));
    }

    #[test]
    fn a_rescan_round_trips_several_targets() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let parsed: DiscoveryType =
            serde_json::from_value(rescan_json(serde_json::json!(ids), "target_host_ids")).unwrap();
        let reparsed: DiscoveryType =
            serde_json::from_value(serde_json::to_value(&parsed).unwrap()).unwrap();
        assert_eq!(reparsed.rescan_target_host_ids(), Some(&ids[..]));
    }

    #[test]
    fn a_rescan_with_no_targets_is_refused() {
        let empty = rescan_json(serde_json::json!([]), "target_host_ids");
        assert!(serde_json::from_value::<DiscoveryType>(empty).is_err());
    }
}
