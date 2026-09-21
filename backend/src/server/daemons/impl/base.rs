use std::fmt::Display;

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use semver::Version;
use serde::{Deserialize, Serialize};
use strum::{Display, VariantNames};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::entities::ChangeTriggersTopologyStaleness;

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct DaemonBase {
    /// The host this entity belongs to.
    pub host_id: Uuid,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// Address the *server* dials for a ServerPoll daemon. Editable (a daemon can move);
    /// unused and not editable for DaemonPoll, which dials out instead.
    #[serde(default)]
    #[schema(required)]
    /// Base URL the server reaches this daemon on.
    #[schema(format = "uri", example = "https://daemon.example.com:60073")]
    pub url: String,
    /// Timestamp of last successful contact with daemon.
    /// NULL for provisioned ServerPoll daemons that haven't been contacted yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub last_seen: Option<DateTime<Utc>>,
    /// How the daemon connects: it polls the server, or the server polls it.
    pub mode: DaemonMode,
    /// Human-facing name for this daemon.
    pub name: String,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Daemon software version (semver format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>, pattern = r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$", example = "0.17.7")]
    pub version: Option<Version>,
    /// User responsible for maintaining this daemon
    pub user_id: Uuid,
    /// Foreign key to API key used for ServerPoll authentication.
    /// NULL for DaemonPoll daemons or those not yet linked to a key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<Uuid>,
    /// Whether the daemon is unreachable (for ServerPoll circuit breaker).
    /// Set to true after repeated polling failures, reset via retry-connection endpoint.
    #[serde(default)]
    pub is_unreachable: bool,
    /// Whether the daemon is on standby due to inactivity (no discovery in 30 days).
    #[serde(default)]
    #[schema(read_only)]
    pub standby: bool,
    /// Timestamp of the most recent standby → active transition. Set by
    /// `process_startup` when a restarted daemon is un-standby'd, and by
    /// the discovery auto-wake path. The nightly inactivity check skips
    /// daemons within the grace window (see `STANDBY_GRACE_PERIOD_DAYS`)
    /// to prevent the "restart → cleared → re-standby'd before discovery
    /// runs" race.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub standby_cleared_at: Option<DateTime<Utc>>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct Daemon {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: DaemonBase,
}

impl Daemon {
    pub fn suppress_logs(&self, other: &Self) -> bool {
        self.base.mode == other.base.mode
            && self.base.url == other.base.url
            && self.base.network_id == other.base.network_id
            && self.base.host_id == other.base.host_id
    }

    /// Check if daemon supports full ServerPoll mode (v0.14.0+).
    ///
    /// Legacy daemons (< v0.14.0) only support `/api/discovery/initiate` and
    /// `/api/discovery/cancel` endpoints without authentication.
    /// They don't support the newer endpoints: `/api/status`, `/api/poll`,
    /// `/api/first-contact`, `/api/discovery/entities-created`.
    ///
    /// Returns `false` for daemons without a version (assume legacy).
    pub fn supports_full_server_poll(&self) -> bool {
        // Floor owned by the version registry (single source of truth).
        crate::server::daemons::r#impl::version::supports_full_server_poll(
            self.base.version.as_ref(),
        )
    }

    /// Whether this daemon's `ready_for_work` means it is idle; see
    /// [`crate::server::daemons::r#impl::version::reports_ready_for_work`].
    pub fn reports_ready_for_work(&self) -> bool {
        crate::server::daemons::r#impl::version::reports_ready_for_work(self.base.version.as_ref())
    }
}

impl Display for Daemon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.base.url, self.id)
    }
}

/// Daemon operating mode that determines the communication pattern.
///
/// - **DaemonPoll** (formerly "Pull"): Daemon makes outbound connections to the server.
///   The daemon registers itself and polls for work. Best for daemons behind NAT/firewall.
///
/// - **ServerPoll** (formerly "Push"): Server makes connections to the daemon.
///   Server polls daemon for status and discovery results. Best for DMZ deployments
///   where daemon cannot make outbound connections.
#[derive(
    Debug,
    Display,
    Copy,
    Clone,
    Serialize,
    Deserialize,
    Default,
    PartialEq,
    Eq,
    ValueEnum,
    Hash,
    VariantNames,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum DaemonMode {
    /// Server polls daemon (daemon cannot make outbound connections)
    #[serde(alias = "push", alias = "Push")]
    #[value(alias = "push")]
    ServerPoll,
    /// Daemon polls server (default, firewall-friendly)
    #[default]
    #[serde(alias = "pull", alias = "Pull")]
    #[value(alias = "pull")]
    DaemonPoll,
}

impl ChangeTriggersTopologyStaleness<Daemon> for Daemon {
    fn triggers_staleness(&self, _other: Option<Daemon>) -> bool {
        false
    }
}
