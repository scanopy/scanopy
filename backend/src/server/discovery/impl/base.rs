use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    credentials::r#impl::mapping::IntegrationTarget,
    discovery::r#impl::types::{DiscoveryType, RunType},
    shared::entities::ChangeTriggersTopologyStaleness,
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
    #[serde(default)]
    #[schema(read_only)]
    pub scan_count: u32,
    /// When true, the next scan will be a full port scan regardless of interval
    #[serde(default)]
    pub force_full_scan: bool,
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
    /// Broadcast targets are the one scope discovery never promotes: the daemon only reports a
    /// credential assignment for a probe it can attribute to a stored credential, and a
    /// broadcast default reaches the probe with no id at all (`daemon/discovery/credentials.rs`
    /// → `dispatch.rs`, which skips `None`). Pruning one outright would silently stop a
    /// site-wide credential — typically an SNMP community seeded by the install command —
    /// after a single scan. So the caller migrates these into the `site_credentials`
    /// junction, which is the durable site-wide channel, before dropping them.
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

impl ChangeTriggersTopologyStaleness<Discovery> for Discovery {
    fn triggers_staleness(&self, _other: Option<Discovery>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
