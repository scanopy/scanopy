use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    credentials::r#impl::mapping::IntegrationTarget,
    discovery::r#impl::{
        scan_settings::defaults as scan_settings_defaults,
        types::{DiscoveryType, RunType},
    },
    shared::entities::ChangeTriggersTopologyStaleness,
    shared::events::{
        log::{EntityLog, EventLog, changes_log},
        types::{EventLogLevel, UpdateReason},
    },
};

#[derive(
    Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, Default, ToSchema, Validate,
)]
pub struct DiscoveryBase {
    /// What this run scans — a subnet, a single host, a container runtime, and so on.
    pub discovery_type: DiscoveryType,
    /// Whether this run was triggered by hand or on a schedule.
    pub run_type: RunType,
    /// Human-facing name for this discovery.
    pub name: String,
    /// The daemon this entity refers to.
    pub daemon_id: Uuid,
    /// The site this entity belongs to.
    pub site_id: Uuid,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct Discovery {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: DiscoveryBase,
    /// Number of completed scans (incremented by server on session completion)
    #[serde(
        default,
        deserialize_with = "crate::server::shared::types::api::ignored"
    )]
    #[schema(read_only)]
    pub scan_count: u32,
    /// When true, the next scan will be a full port scan regardless of interval
    #[serde(default)]
    pub force_full_scan: bool,
    /// Whether the next run of this discovery will be a full scan of all 65,535 ports rather than
    /// a light scan of common ports. Computed by the server on read and never stored.
    #[serde(default)]
    #[schema(read_only)]
    pub next_run_is_full_scan: bool,
    /// Per-daemon integration targeting: which integrations run on this daemon, and on which
    /// IPs. Delivered via the init command at registration and editable via the discovery
    /// modal. This is the single home for cred↔IP targeting; it replaces the global
    /// `credential.target_ips` (race-prone, consumed once).
    ///
    /// One-shot: a target is offered to the daemon until a scan completes successfully, then
    /// dropped by [`Discovery::apply_successful_scan`]. Credentials that earned a durable home
    /// during the scan keep being retried from there — `host_credentials` for one that probed
    /// successfully, `site_credentials` for a broadcast one (see
    /// [`Discovery::take_site_scope_credential_ids`]).
    #[serde(default)]
    #[schema(required)]
    pub integration_targets: Vec<IntegrationTarget>,
}

impl Discovery {
    /// Remove the `Site`-scope integration targets and return their credential ids.
    ///
    /// A broadcast target is pinned only to the hosts it worked on, and a site-wide credential has
    /// to keep reaching hosts found later. Pruning one outright would silently stop it, typically
    /// an SNMP community seeded by the install command, after a single scan. So the caller
    /// migrates these into the `site_credentials` junction, which is the durable site-wide
    /// channel, before dropping them.
    pub fn take_site_scope_credential_ids(&mut self) -> Vec<Uuid> {
        let mut credential_ids = Vec::new();
        self.integration_targets.retain(|target| match target {
            IntegrationTarget::Site { credential_id } => {
                credential_ids.push(*credential_id);
                false
            }
            IntegrationTarget::DaemonHost { .. } | IntegrationTarget::Hosts { .. } => true,
        });
        credential_ids
    }

    /// Fold the effects of a successfully completed scan into the config row.
    ///
    /// `integration_targets` are one-shot and are dropped here. A target that probed
    /// successfully has already been promoted to a `host_credentials` assignment on the host it
    /// worked on (`hosts/service/create.rs` → `merge_host_credentials`), so it keeps being
    /// dispatched from there — including a Docker/Podman socket credential, which earns its
    /// assignment on the daemon host's own loopback address. A target that matched nothing has
    /// no assignment to inherit and is not retried, which is the point: before this, an
    /// unmatched discovery-scoped credential re-probed on every scan forever.
    ///
    /// Only call this for a terminal `Complete`. A failed or cancelled session must keep its
    /// config so the retry runs with the same targeting.
    pub fn apply_successful_scan(&mut self) {
        self.scan_count += 1;
        self.force_full_scan = false;
        self.integration_targets.clear();
        self.next_run_is_full_scan = self.full_scan_is_due();
    }

    /// Whether the next run should scan all 65,535 ports. Only Unified discoveries port-scan.
    ///
    /// The first run is light so the first results arrive fast. The second run
    /// (`scan_count == 1`) is a full baseline, and after that every `full_scan_interval`-th run
    /// is full.
    /// An interval of 0 means light scans only, 1 means every run is full, and
    /// `force_full_scan` overrides the interval for one run.
    pub fn full_scan_is_due(&self) -> bool {
        // A historical run is a record of one finished scan and has no next run.
        if matches!(self.base.run_type, RunType::Historical { .. }) {
            return false;
        }
        let DiscoveryType::Unified { scan_settings, .. } = &self.base.discovery_type else {
            return false;
        };
        let interval = scan_settings
            .full_scan_interval
            .unwrap_or_else(scan_settings_defaults::full_scan_interval);
        self.force_full_scan
            || (interval != 0
                && (interval == 1
                    || self.scan_count == 1
                    || (self.scan_count > 1 && self.scan_count.is_multiple_of(interval))))
    }

    pub fn disable(&mut self) {
        match self.base.run_type {
            RunType::Scheduled {
                ref mut enabled, ..
            } => *enabled = false,
            // Nothing to disable — these are not scheduler-driven.
            RunType::Historical { .. } | RunType::AdHoc { .. } => {}
        }
    }
}

impl Display for Discovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Discovery {}: {}", self.base.name, self.id)
    }
}

/// A discovery that failed to schedule is disabled; that gets an ERROR line saying so, since
/// it won't run again until someone re-enables it.
impl EntityLog for Discovery {
    fn update_log(&self, before: &Self, id: Uuid, reason: &UpdateReason) -> EventLog {
        let log = changes_log(before, self, id, reason, Self::LOG_IGNORED_PATHS);
        match reason {
            UpdateReason::DiscoveryScheduleFailed { .. } => log.with_line(
                EventLogLevel::Error,
                format!("Discovery {} {}", self.base.name, reason.describe()),
            ),
            _ => log,
        }
    }
}

impl ChangeTriggersTopologyStaleness<Discovery> for Discovery {
    fn triggers_staleness(&self, _other: Option<Discovery>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
    use crate::server::discovery::r#impl::scan_settings::ScanSettings;
    use std::net::IpAddr;

    fn discovery(integration_targets: Vec<IntegrationTarget>) -> Discovery {
        Discovery {
            integration_targets,
            force_full_scan: true,
            ..Default::default()
        }
    }

    /// Broadcast targets are lifted out for the caller to migrate into the
    /// `site_credentials` junction; the scopes discovery can promote on its own are left for
    /// the prune to take.
    #[test]
    fn only_broadcast_targets_are_taken_for_migration() {
        let broadcast = Uuid::new_v4();
        let socket = Uuid::new_v4();
        let host_scoped = Uuid::new_v4();
        let mut discovery = discovery(vec![
            IntegrationTarget::DaemonHost {
                credential_id: socket,
            },
            IntegrationTarget::Site {
                credential_id: broadcast,
            },
            IntegrationTarget::Hosts {
                credential_id: host_scoped,
                ips: vec!["10.0.0.7".parse::<IpAddr>().unwrap()],
            },
        ]);

        assert_eq!(discovery.take_site_scope_credential_ids(), vec![broadcast]);
        assert_eq!(
            discovery
                .integration_targets
                .iter()
                .map(|t| t.credential_id())
                .collect::<Vec<_>>(),
            vec![socket, host_scoped],
            "the promotable scopes stay for the prune to take"
        );
    }

    /// The one-shot contract: a completed scan consumes every remaining target, so an unmatched
    /// discovery-scoped credential is not re-probed on the next scan. Anything that matched is
    /// retried from the junction it earned instead.
    #[test]
    fn a_successful_scan_consumes_every_remaining_target() {
        let mut discovery = discovery(vec![
            IntegrationTarget::DaemonHost {
                credential_id: Uuid::new_v4(),
            },
            IntegrationTarget::Hosts {
                credential_id: Uuid::new_v4(),
                ips: vec!["10.0.0.7".parse::<IpAddr>().unwrap()],
            },
        ]);

        discovery.apply_successful_scan();

        assert!(discovery.integration_targets.is_empty());
        assert_eq!(discovery.scan_count, 1);
        assert!(!discovery.force_full_scan);
    }

    fn unified(full_scan_interval: Option<u32>) -> Discovery {
        Discovery {
            base: DiscoveryBase {
                discovery_type: DiscoveryType::Unified {
                    host_id: Uuid::new_v4(),
                    subnet_ids: None,
                    host_naming_fallback: Default::default(),
                    scan_settings: ScanSettings {
                        full_scan_interval,
                        ..Default::default()
                    },
                },
                run_type: RunType::AdHoc { last_run: None },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Whether each of the first `runs` runs is full, driven through `apply_successful_scan` as the server
    /// does, reading the computed field the UI receives before each run.
    fn full_scan_by_run(mut discovery: Discovery, runs: usize) -> Vec<bool> {
        discovery.next_run_is_full_scan = discovery.full_scan_is_due();
        (0..runs)
            .map(|_| {
                let full = discovery.next_run_is_full_scan;
                assert_eq!(full, discovery.full_scan_is_due());
                discovery.apply_successful_scan();
                full
            })
            .collect()
    }

    #[test]
    fn first_run_is_light_second_is_full_then_every_interval() {
        assert_eq!(
            full_scan_by_run(unified(None), 8),
            vec![false, true, false, true, false, false, true, false]
        );
    }

    #[test]
    fn interval_zero_is_light_only_and_one_is_always_full() {
        assert!(
            full_scan_by_run(unified(Some(0)), 5)
                .iter()
                .all(|full| !full)
        );
        assert!(
            full_scan_by_run(unified(Some(1)), 5)
                .iter()
                .all(|full| *full)
        );
    }

    /// A forced full scan applies to one run, then the interval resumes.
    #[test]
    fn forced_full_scan_applies_once() {
        let mut discovery = unified(Some(0));
        discovery.force_full_scan = true;
        assert_eq!(full_scan_by_run(discovery, 3), vec![true, false, false]);
    }

    #[test]
    fn only_unified_schedulable_discoveries_have_a_full_next_run() {
        let mut historical = unified(Some(1));
        historical.base.run_type = RunType::Historical {
            results: Box::new(DiscoveryUpdatePayload::new(
                Uuid::nil(),
                Uuid::nil(),
                Uuid::nil(),
                DiscoveryType::default(),
                None,
            )),
        };
        assert!(!historical.full_scan_is_due());

        let self_report = Discovery {
            force_full_scan: true,
            ..Default::default()
        };
        assert!(!self_report.full_scan_is_due());
    }

    /// Rows and payloads written before these fields existed still parse, as light.
    #[test]
    fn records_without_full_scan_fields_parse_as_light() {
        let mut value = serde_json::to_value(unified(None)).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .remove("next_run_is_full_scan");
        value["discovery_type"]["scan_settings"]
            .as_object_mut()
            .unwrap()
            .remove("is_full_scan");

        let discovery: Discovery = serde_json::from_value(value).unwrap();
        assert!(!discovery.next_run_is_full_scan);
        let DiscoveryType::Unified { scan_settings, .. } = discovery.base.discovery_type else {
            panic!("expected Unified");
        };
        assert!(!scan_settings.is_full_scan);
    }
}
