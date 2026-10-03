use std::fmt::Display;

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use semver::Version;
use serde::{Deserialize, Serialize};
use strum::{Display, VariantNames};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::credentials::r#impl::types::OsFamily;
use crate::server::shared::entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants};
use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

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
    /// The operating system the daemon runs on. Picked when the daemon is created, then checked
    /// against what the daemon reports on every handshake once it is new enough to report (see
    /// `version_status.supports_os_reporting`); a daemon on another OS is refused. Credentials
    /// set up for another OS's file paths cannot be used by it. `None` for a daemon created
    /// before this was recorded that has not yet reported.
    #[serde(default)]
    #[schema(required)]
    pub os: Option<DaemonOs>,
}

/// What [`DaemonBase::accept_reported_os`] did with the OS a handshake reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportedOsOutcome {
    /// Nothing to record: the daemon reported no OS, or the one already recorded.
    Unchanged,
    /// No OS was recorded, so the reported one now is.
    Recorded,
    /// The recorded OS was a pick this daemon never confirmed, and it reports another. The
    /// reported one replaced `picked`.
    Adopted { picked: DaemonOs },
}

/// A daemon reported an OS other than the one it was created for, so its handshake is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonOsMismatch {
    pub expected: DaemonOs,
    pub actual: DaemonOs,
}

impl From<DaemonOsMismatch> for crate::server::shared::types::api::ApiError {
    fn from(m: DaemonOsMismatch) -> Self {
        Self::daemon_os_mismatch(m.expected.name(), m.actual.name())
    }
}

impl DaemonBase {
    /// Check the OS a handshake reported against the recorded one, recording it where there is
    /// nothing to check against.
    ///
    /// Reads `self.version` as the version stored *before* this handshake, so call it before the
    /// handshake's version is written. That version says whether the recorded OS has been checked
    /// before: a daemon that already reported it from an OS-reporting version is held to it, and a
    /// daemon reporting for the first time after an upgrade adopts what it reports, because the
    /// pick it was created with was never verified and refusing would lock a working daemon out.
    /// A daemon that has never connected is held to its pick: it is being installed now.
    pub fn accept_reported_os(
        &mut self,
        reported: Option<DaemonOs>,
    ) -> Result<ReportedOsOutcome, DaemonOsMismatch> {
        let Some(actual) = reported else {
            return Ok(ReportedOsOutcome::Unchanged);
        };
        let Some(expected) = self.os else {
            self.os = Some(actual);
            return Ok(ReportedOsOutcome::Recorded);
        };
        if expected == actual {
            return Ok(ReportedOsOutcome::Unchanged);
        }
        let verified_before = self.last_seen.is_none()
            || crate::server::daemons::r#impl::version::supports_os_reporting(
                self.version.as_ref(),
            );
        if verified_before {
            return Err(DaemonOsMismatch { expected, actual });
        }
        self.os = Some(actual);
        Ok(ReportedOsOutcome::Adopted { picked: expected })
    }
}

/// Deserialize a daemon-reported OS, reading a value this server does not know as `None`.
///
/// A newer daemon may report an OS added after this server was built. Failing on it would fail the
/// whole handshake payload it rides in; treating it as unreported only skips the check.
pub fn lenient_daemon_os<'de, D>(deserializer: D) -> Result<Option<DaemonOs>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(raw.and_then(|s| {
        use strum::IntoEnumIterator;
        DaemonOs::iter().find(|os| <&'static str>::from(*os) == s)
    }))
}

/// Operating system a daemon is installed on, as picked when it is created.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Deserialize,
    Serialize,
    strum_macros::IntoStaticStr,
    strum_macros::EnumIter,
    strum_macros::VariantNames,
    ToSchema,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum DaemonOs {
    Linux,
    MacOS,
    Windows,
    FreeBsd,
}

impl DaemonOs {
    /// The OS this binary was built for. A daemon reports it in warnings so they can name it.
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOS
        } else if cfg!(target_os = "freebsd") {
            Self::FreeBsd
        } else {
            Self::Linux
        }
    }
}

impl From<DaemonOs> for OsFamily {
    fn from(os: DaemonOs) -> Self {
        match os {
            DaemonOs::Windows => OsFamily::Windows,
            DaemonOs::Linux | DaemonOs::MacOS | DaemonOs::FreeBsd => OsFamily::Unix,
        }
    }
}

impl HasId for DaemonOs {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for DaemonOs {
    fn color(&self) -> Color {
        EntityDiscriminants::Daemon.color()
    }

    fn icon(&self) -> Icon {
        EntityDiscriminants::Daemon.icon()
    }
}

/// Emitted as `daemon-os.json`: the OS family each install OS maps to, so the create-daemon modal
/// stamps credentials and checks existing ones without its own copy of the mapping.
impl TypeMetadataProvider for DaemonOs {
    fn name(&self) -> &'static str {
        match self {
            DaemonOs::Linux => "Linux",
            DaemonOs::MacOS => "macOS",
            DaemonOs::Windows => "Windows",
            DaemonOs::FreeBsd => "FreeBSD",
        }
    }

    fn description(&self) -> &'static str {
        ""
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({ "os_family": OsFamily::from(*self) })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn daemon(os: Option<DaemonOs>, version: Option<&str>, connected: bool) -> DaemonBase {
        DaemonBase {
            os,
            version: version.map(|v| Version::parse(v).unwrap()),
            last_seen: connected.then(Utc::now),
            ..Default::default()
        }
    }

    const REPORTING: &str = "0.17.20";
    const PRE_REPORTING: &str = "0.17.19";

    #[test]
    fn a_daemon_that_reports_nothing_is_not_checked() {
        let mut base = daemon(Some(DaemonOs::Windows), Some(REPORTING), true);
        assert_eq!(
            base.accept_reported_os(None),
            Ok(ReportedOsOutcome::Unchanged)
        );
        assert_eq!(base.os, Some(DaemonOs::Windows));
    }

    #[test]
    fn a_daemon_with_no_recorded_os_records_the_reported_one() {
        let mut base = daemon(None, Some(PRE_REPORTING), true);
        assert_eq!(
            base.accept_reported_os(Some(DaemonOs::Linux)),
            Ok(ReportedOsOutcome::Recorded)
        );
        assert_eq!(base.os, Some(DaemonOs::Linux));
    }

    #[test]
    fn a_matching_report_is_accepted() {
        let mut base = daemon(Some(DaemonOs::Linux), Some(REPORTING), true);
        assert_eq!(
            base.accept_reported_os(Some(DaemonOs::Linux)),
            Ok(ReportedOsOutcome::Unchanged)
        );
    }

    #[test]
    fn a_daemon_being_installed_on_another_os_is_refused() {
        let mut base = daemon(Some(DaemonOs::Windows), None, false);
        assert_eq!(
            base.accept_reported_os(Some(DaemonOs::Linux)),
            Err(DaemonOsMismatch {
                expected: DaemonOs::Windows,
                actual: DaemonOs::Linux,
            })
        );
        assert_eq!(base.os, Some(DaemonOs::Windows));
    }

    #[test]
    fn the_first_report_after_an_upgrade_replaces_an_unverified_pick() {
        let mut base = daemon(Some(DaemonOs::MacOS), Some(PRE_REPORTING), true);
        assert_eq!(
            base.accept_reported_os(Some(DaemonOs::Linux)),
            Ok(ReportedOsOutcome::Adopted {
                picked: DaemonOs::MacOS
            })
        );
        assert_eq!(base.os, Some(DaemonOs::Linux));
    }

    #[test]
    fn a_daemon_that_reported_before_is_held_to_its_os() {
        let mut base = daemon(Some(DaemonOs::Linux), Some(REPORTING), true);
        assert!(base.accept_reported_os(Some(DaemonOs::Windows)).is_err());
        assert_eq!(base.os, Some(DaemonOs::Linux));
    }

    #[test]
    fn startup_from_a_daemon_that_predates_os_reporting_still_parses() {
        let request: crate::server::daemons::r#impl::api::DaemonStartupRequest =
            serde_json::from_str(r#"{"daemon_version":"0.17.18"}"#).unwrap();
        assert_eq!(request.os, None);
    }

    #[test]
    fn an_os_this_server_does_not_know_reads_as_unreported() {
        let request: crate::server::daemons::r#impl::api::DaemonStartupRequest =
            serde_json::from_str(r#"{"daemon_version":"0.18.0","os":"openbsd"}"#).unwrap();
        assert_eq!(request.os, None);
        assert_eq!(request.daemon_version, Version::new(0, 18, 0));
    }

    #[test]
    fn a_reported_os_round_trips() {
        let json = serde_json::to_string(&Some(DaemonOs::FreeBsd)).unwrap();
        let request: crate::server::daemons::r#impl::api::DaemonStartupRequest =
            serde_json::from_str(&format!(r#"{{"daemon_version":"0.18.0","os":{json}}}"#)).unwrap();
        assert_eq!(request.os, Some(DaemonOs::FreeBsd));
    }
}
