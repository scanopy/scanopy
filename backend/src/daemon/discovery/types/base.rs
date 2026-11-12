use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use strum_macros::EnumDiscriminants;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::shared::types::metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider};
use crate::server::shared::types::{Color, Icon};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Copy,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    EnumDiscriminants,
    EnumIter,
)]
#[strum_discriminants(derive(
    Hash,
    EnumIter,
    strum::Display,
    strum::AsRefStr,
    Serialize,
    Deserialize
))]
pub enum DiscoveryPhase {
    /// Blocked: a site snapshot is in progress on this site. The session
    /// can't enter the normal Queued/Pending decision until
    /// `release_site_for_snapshot` clears the block. Daemons polling for
    /// work do NOT see AwaitingSnapshot sessions.
    AwaitingSnapshot,
    Queued,   // Waiting in daemon queue behind another session
    Pending,  // Front of queue, eligible for dispatch. Clock ticking.
    Starting, // get_pending_work() picked it up, dispatching to daemon
    Started,  // Daemon acknowledged, actively running
    Scanning,
    Complete,
    Failed,
    Cancelled,
}

impl DiscoveryPhase {
    pub fn log_level(&self) -> crate::server::shared::events::types::EventLogLevel {
        use crate::server::shared::events::types::EventLogLevel;
        match self {
            DiscoveryPhase::Failed => EventLogLevel::Warn,
            _ => EventLogLevel::Info,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            DiscoveryPhase::Complete | DiscoveryPhase::Cancelled | DiscoveryPhase::Failed
        )
    }

    /// Whether this phase is subject to stall cleanup.
    ///
    /// - `Queued`: No — waiting in queue, no dispatch attempted, no clock running
    /// - `Pending`: Yes — promoted to front of queue, dispatch expected within a poll cycle.
    ///   If still here after 5 min, daemon is unreachable.
    /// - `Starting`/`Started`/`Scanning`: Yes — dispatched, should be progressing
    /// - Terminal states: No — already done
    pub fn can_be_cleaned_up(&self) -> bool {
        matches!(
            self,
            DiscoveryPhase::Pending
                | DiscoveryPhase::Starting
                | DiscoveryPhase::Started
                | DiscoveryPhase::Scanning
        )
    }
}

impl HasId for DiscoveryPhase {
    fn id(&self) -> &'static str {
        match self {
            DiscoveryPhase::AwaitingSnapshot => "AwaitingSnapshot",
            DiscoveryPhase::Queued => "Queued",
            DiscoveryPhase::Pending => "Pending",
            DiscoveryPhase::Starting => "Starting",
            DiscoveryPhase::Started => "Started",
            DiscoveryPhase::Scanning => "Scanning",
            DiscoveryPhase::Complete => "Complete",
            DiscoveryPhase::Failed => "Failed",
            DiscoveryPhase::Cancelled => "Cancelled",
        }
    }
}

impl EntityMetadataProvider for DiscoveryPhase {
    fn color(&self) -> Color {
        match self {
            DiscoveryPhase::AwaitingSnapshot => Color::Yellow,
            DiscoveryPhase::Queued => Color::Gray,
            DiscoveryPhase::Pending => Color::Blue,
            DiscoveryPhase::Starting => Color::Blue,
            DiscoveryPhase::Started => Color::Blue,
            DiscoveryPhase::Scanning => Color::Blue,
            DiscoveryPhase::Complete => Color::Green,
            DiscoveryPhase::Failed => Color::Red,
            DiscoveryPhase::Cancelled => Color::Gray,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            DiscoveryPhase::AwaitingSnapshot => Icon::Camera,
            DiscoveryPhase::Queued => Icon::Clock,
            DiscoveryPhase::Pending => Icon::Clock,
            DiscoveryPhase::Starting => Icon::Play,
            DiscoveryPhase::Started => Icon::Play,
            DiscoveryPhase::Scanning => Icon::Radar,
            DiscoveryPhase::Complete => Icon::Check,
            DiscoveryPhase::Failed => Icon::X,
            DiscoveryPhase::Cancelled => Icon::CircleSlash,
        }
    }
}

impl TypeMetadataProvider for DiscoveryPhase {
    fn name(&self) -> &'static str {
        match self {
            DiscoveryPhase::AwaitingSnapshot => "Waiting for snapshot",
            DiscoveryPhase::Queued => "Queued",
            DiscoveryPhase::Pending => "Pending",
            DiscoveryPhase::Starting => "Starting",
            DiscoveryPhase::Started => "Started",
            DiscoveryPhase::Scanning => "Scanning",
            DiscoveryPhase::Complete => "Complete",
            DiscoveryPhase::Failed => "Failed",
            DiscoveryPhase::Cancelled => "Cancelled",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            DiscoveryPhase::AwaitingSnapshot => {
                "Waiting for snapshot — discovery will start once the snapshot completes"
            }
            DiscoveryPhase::Queued => "Waiting in queue — another scan is running on this daemon",
            DiscoveryPhase::Pending => "Ready to start — connecting to daemon",
            DiscoveryPhase::Starting => "Waiting for session to start on the daemon",
            DiscoveryPhase::Started => "Daemon acknowledged, actively running",
            DiscoveryPhase::Scanning => "Scanning for hosts",
            DiscoveryPhase::Complete => "Discovery complete",
            DiscoveryPhase::Failed => "Discovery failed",
            DiscoveryPhase::Cancelled => "Discovery cancelled",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({ "is_terminal": self.is_terminal() })
    }
}

/// Why a session reached its terminal phase.
///
/// `phase` says *that* a run ended; this says *why*. A stall reaped by the server, a daemon that
/// lost its state in a restart and a user pressing cancel all used to land as the same phase with
/// no way to tell them apart afterwards, which is what made stall reports undiagnosable.
///
/// The server decides every reason except the two the daemon alone can know, see
/// [`Self::daemon_assigned`]. Absent on runs recorded before this field existed.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Copy,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    EnumDiscriminants,
    EnumIter,
)]
#[strum_discriminants(derive(
    Hash,
    EnumIter,
    strum::Display,
    strum::AsRefStr,
    Serialize,
    Deserialize
))]
pub enum DiscoveryTerminalReason {
    Completed,
    UserCancelled,
    /// The daemon reported `Failed` without a more specific reason.
    DaemonReportedFailure,
    /// The server received nothing for the session within the stall threshold, and had no record
    /// of its daemon checking in to say why. Sessions stalled before the cause was recorded carry
    /// it too.
    StalledNoUpdates,
    /// The daemon restarted while the server had the session running: it announced its startup,
    /// or a ServerPoll daemon came back ready for work.
    DaemonRestarted,
    /// The server could not reach the daemon: to deliver a session, or a cancellation.
    DaemonUnreachable,
    /// The daemon refused the session because it was already running another.
    DaemonBusy,
    /// The daemon's watchdog ended a session that outlived its maximum duration.
    WatchdogTimeout,
    /// The session went quiet and so did its daemon: it stopped checking in with the server.
    DaemonOffline,
    /// The session went quiet while its daemon kept checking in with the server.
    SessionSilent,
    /// A DaemonPoll daemon came back idle while the server still had the session running, without
    /// a restart and without the session's final report.
    DaemonDroppedSession,
    /// The daemon's watchdog ended a session that kept running without advancing: no progress, no
    /// host created and no deep scan or integration finished for the activity limit.
    SessionStoppedAdvancing,
}

impl DiscoveryTerminalReason {
    /// Reasons only the daemon can observe. The server keeps these when a daemon reports them and
    /// overwrites any other value a daemon sends, so the server stays the authority for the rest.
    pub fn daemon_assigned(self) -> bool {
        matches!(
            self,
            DiscoveryTerminalReason::DaemonBusy
                | DiscoveryTerminalReason::WatchdogTimeout
                | DiscoveryTerminalReason::SessionStoppedAdvancing
        )
    }

    /// The run stopped making progress rather than failing outright.
    pub fn is_stall(self) -> bool {
        matches!(
            self,
            DiscoveryTerminalReason::StalledNoUpdates
                | DiscoveryTerminalReason::WatchdogTimeout
                | DiscoveryTerminalReason::DaemonOffline
                | DiscoveryTerminalReason::SessionSilent
                | DiscoveryTerminalReason::SessionStoppedAdvancing
        )
    }

    /// The daemon's own log lines for the run say what happened. Not for a run the server could
    /// not reach the daemon about, one the daemon refused, or one that ended normally: the daemon
    /// logged nothing for those that explains them.
    pub fn explained_by_daemon_logs(self) -> bool {
        matches!(
            self,
            DiscoveryTerminalReason::DaemonReportedFailure
                | DiscoveryTerminalReason::DaemonRestarted
                | DiscoveryTerminalReason::WatchdogTimeout
                | DiscoveryTerminalReason::DaemonOffline
                | DiscoveryTerminalReason::SessionSilent
                | DiscoveryTerminalReason::DaemonDroppedSession
                | DiscoveryTerminalReason::StalledNoUpdates
                | DiscoveryTerminalReason::SessionStoppedAdvancing
        )
    }
}

impl HasId for DiscoveryTerminalReason {
    fn id(&self) -> &'static str {
        match self {
            DiscoveryTerminalReason::Completed => "Completed",
            DiscoveryTerminalReason::UserCancelled => "UserCancelled",
            DiscoveryTerminalReason::DaemonReportedFailure => "DaemonReportedFailure",
            DiscoveryTerminalReason::StalledNoUpdates => "StalledNoUpdates",
            DiscoveryTerminalReason::DaemonRestarted => "DaemonRestarted",
            DiscoveryTerminalReason::DaemonUnreachable => "DaemonUnreachable",
            DiscoveryTerminalReason::DaemonBusy => "DaemonBusy",
            DiscoveryTerminalReason::WatchdogTimeout => "WatchdogTimeout",
            DiscoveryTerminalReason::DaemonOffline => "DaemonOffline",
            DiscoveryTerminalReason::SessionSilent => "SessionSilent",
            DiscoveryTerminalReason::DaemonDroppedSession => "DaemonDroppedSession",
            DiscoveryTerminalReason::SessionStoppedAdvancing => "SessionStoppedAdvancing",
        }
    }
}

impl EntityMetadataProvider for DiscoveryTerminalReason {
    fn color(&self) -> Color {
        match self {
            DiscoveryTerminalReason::Completed => Color::Green,
            DiscoveryTerminalReason::UserCancelled => Color::Gray,
            DiscoveryTerminalReason::DaemonReportedFailure => Color::Red,
            DiscoveryTerminalReason::StalledNoUpdates => Color::Amber,
            DiscoveryTerminalReason::DaemonRestarted => Color::Orange,
            DiscoveryTerminalReason::DaemonUnreachable => Color::Red,
            DiscoveryTerminalReason::DaemonBusy => Color::Amber,
            DiscoveryTerminalReason::WatchdogTimeout => Color::Amber,
            DiscoveryTerminalReason::DaemonOffline => Color::Red,
            DiscoveryTerminalReason::SessionSilent => Color::Amber,
            DiscoveryTerminalReason::DaemonDroppedSession => Color::Orange,
            DiscoveryTerminalReason::SessionStoppedAdvancing => Color::Amber,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            DiscoveryTerminalReason::Completed => Icon::Check,
            DiscoveryTerminalReason::UserCancelled => Icon::CircleSlash,
            DiscoveryTerminalReason::DaemonReportedFailure => Icon::X,
            DiscoveryTerminalReason::StalledNoUpdates => Icon::Clock,
            DiscoveryTerminalReason::DaemonRestarted => Icon::RotateCcw,
            DiscoveryTerminalReason::DaemonUnreachable => Icon::Unplug,
            DiscoveryTerminalReason::DaemonBusy => Icon::Hourglass,
            DiscoveryTerminalReason::WatchdogTimeout => Icon::TimerOff,
            DiscoveryTerminalReason::DaemonOffline => Icon::WifiOff,
            DiscoveryTerminalReason::SessionSilent => Icon::MessageCircleOff,
            DiscoveryTerminalReason::DaemonDroppedSession => Icon::CircleQuestionMark,
            DiscoveryTerminalReason::SessionStoppedAdvancing => Icon::CirclePause,
        }
    }
}

impl TypeMetadataProvider for DiscoveryTerminalReason {
    fn name(&self) -> &'static str {
        match self {
            DiscoveryTerminalReason::Completed => "Completed",
            DiscoveryTerminalReason::UserCancelled => "Cancelled",
            DiscoveryTerminalReason::DaemonReportedFailure => "Failed",
            DiscoveryTerminalReason::StalledNoUpdates => "Stalled",
            DiscoveryTerminalReason::DaemonRestarted => "Daemon restarted",
            DiscoveryTerminalReason::DaemonUnreachable => "Daemon unreachable",
            DiscoveryTerminalReason::DaemonBusy => "Daemon busy",
            DiscoveryTerminalReason::WatchdogTimeout => "Timed out",
            DiscoveryTerminalReason::DaemonOffline => "Daemon offline",
            DiscoveryTerminalReason::SessionSilent => "Scan went silent",
            DiscoveryTerminalReason::DaemonDroppedSession => "Daemon dropped scan",
            DiscoveryTerminalReason::SessionStoppedAdvancing => "Scan stopped advancing",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            DiscoveryTerminalReason::Completed => "The scan finished normally.",
            DiscoveryTerminalReason::UserCancelled => {
                "Someone stopped this scan before it finished."
            }
            DiscoveryTerminalReason::DaemonReportedFailure => {
                "The daemon reported an error and stopped the scan. The error message says what failed."
            }
            DiscoveryTerminalReason::StalledNoUpdates => {
                "The server heard nothing from the daemon for 5 minutes and stopped waiting. Check that the daemon is running and can reach the server."
            }
            DiscoveryTerminalReason::DaemonRestarted => {
                "The daemon restarted during the scan, so the scan's progress was lost. The daemon's logs show why it restarted."
            }
            DiscoveryTerminalReason::DaemonUnreachable => {
                "The server could not reach the daemon to stop this scan. Check that the daemon is running and reachable from the server."
            }
            DiscoveryTerminalReason::DaemonBusy => {
                "The daemon was already running another scan and refused this one."
            }
            DiscoveryTerminalReason::WatchdogTimeout => {
                "The scan ran past its maximum duration without finishing, so the daemon stopped it. Part of the scan stopped responding; the daemon's logs name the phase."
            }
            DiscoveryTerminalReason::DaemonOffline => {
                "The daemon stopped checking in with the server during the scan, so no progress or cancellation could get through. Check that the daemon is running and can reach the server."
            }
            DiscoveryTerminalReason::SessionSilent => {
                "The daemon kept checking in with the server but sent nothing about this scan for 5 minutes. The daemon's logs show where the scan stopped."
            }
            DiscoveryTerminalReason::DaemonDroppedSession => {
                "The daemon is running and idle but no longer had this scan, and its final report never reached the server. The daemon's logs show how the scan ended."
            }
            DiscoveryTerminalReason::SessionStoppedAdvancing => {
                "The scan kept running for 20 minutes without finding a host or finishing a step, so the daemon stopped it. The error names the hosts the scan was still working on."
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "is_stall": self.is_stall(),
            "explained_by_daemon_logs": self.explained_by_daemon_logs(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct DiscoverySessionInfo {
    pub session_id: Uuid,
    pub site_id: Uuid,
    pub daemon_id: Uuid,
    pub started_at: Option<DateTime<Utc>>,
    pub discovery_type: DiscoveryType,
    pub discovery_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct DiscoverySessionUpdate {
    pub phase: DiscoveryPhase,
    /// Percentage complete (0-100)
    pub progress: u8,
    pub error: Option<String>,
    /// Non-fatal warnings for a completed run (e.g. hit the time limit with hosts
    /// left un-scanned). Distinct from `error`, which marks the run as failed.
    pub warnings: Vec<DiscoveryWarning>,
    /// Per-credential results, carried on the terminal update.
    pub credential_results:
        Vec<crate::server::credentials::r#impl::run_results::CredentialRunResult>,
    pub finished_at: Option<DateTime<Utc>>,
    /// Set only for the reasons the daemon alone can know; see
    /// [`DiscoveryTerminalReason::daemon_assigned`].
    pub reason: Option<DiscoveryTerminalReason>,
}

impl DiscoverySessionUpdate {
    pub fn scanning(progress: u8) -> Self {
        Self {
            phase: DiscoveryPhase::Scanning,
            progress: progress.min(100),
            error: None,
            warnings: Vec::new(),
            credential_results: Vec::new(),
            finished_at: None,
            reason: None,
        }
    }
}

impl std::fmt::Display for DiscoveryPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryPhase::AwaitingSnapshot => {
                write!(f, "Waiting for site snapshot to complete")
            }
            DiscoveryPhase::Queued => write!(f, "Waiting in queue behind another session"),
            DiscoveryPhase::Pending => {
                write!(f, "Session created, waiting for daemon availability")
            }
            DiscoveryPhase::Starting => write!(f, "Sending session to daemon"),
            DiscoveryPhase::Started => write!(f, "Session started in daemon"),
            DiscoveryPhase::Scanning => write!(f, "Scanning for active hosts"),
            DiscoveryPhase::Complete => write!(f, "Discovery complete"),
            DiscoveryPhase::Cancelled => write!(f, "Discovery cancelled"),
            DiscoveryPhase::Failed => write!(f, "Discovery failed"),
        }
    }
}

pub enum DiscoveryCriticalError {
    ResourceExhaustion,
}

impl DiscoveryCriticalError {
    pub fn is_critical_error(error_str: String) -> bool {
        Self::from_error_string(error_str).is_some()
    }

    pub fn from_error_string(error_str: String) -> Option<Self> {
        let lower_error = error_str.to_lowercase();

        if lower_error.contains("too many open files")
            || lower_error.contains("file descriptor limit")
            || lower_error.contains("cannot allocate memory")
            || lower_error.contains("out of memory")
            || lower_error.contains("os error 24")
            || lower_error.contains("emfile")
        {
            return Some(DiscoveryCriticalError::ResourceExhaustion);
        }

        None
    }
}

impl Display for DiscoveryCriticalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryCriticalError::ResourceExhaustion => {
                write!(
                    f,
                    "The scan ran out of file descriptors (too many open files). Raise the daemon's hard file-descriptor limit and restart it: https://scanopy.net/docs/setting-up-daemons/troubleshooting-setup/#too-many-open-files-error"
                )
            }
        }
    }
}
