//! Interface refresh: classifying a daemon's interfaced subnets again when its interfaces change.
//!
//! A daemon reports its interfaced subnets once, after a session classifies them, and every
//! status report carries a fingerprint of its interfaces. The server keeps, per daemon, the
//! fingerprint its interfaced subnets were last classified from. A report whose fingerprint
//! differs starts an `InterfaceRefresh` session, which classifies them again with the host's
//! container credentials and reports them on the next status.
//!
//! The map is in memory only. After a restart it is empty, so each daemon gets one refresh.
use super::*;

/// What a status report's interface fingerprint calls for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InterfaceRefreshDecision {
    /// The fingerprint matches the last classified one, or a refresh can't start yet.
    Nothing,
    /// The report carries freshly classified subnets: their fingerprint is the new baseline.
    Record,
    /// The interfaces changed since they were last classified: start a refresh.
    Start,
}

/// Decide what a report calls for. `reported` is its fingerprint, `last` the one the daemon's
/// interfaced subnets were last classified from. `daemon_busy` holds while the daemon has a
/// session queued or running (each one classifies the subnets and reports them) or its
/// organization may not run discovery. The refresh then waits for a later report.
pub(crate) fn interface_refresh_decision(
    reported: &str,
    carries_interfaced_subnets: bool,
    last: Option<&str>,
    daemon_busy: bool,
) -> InterfaceRefreshDecision {
    if carries_interfaced_subnets {
        InterfaceRefreshDecision::Record
    } else if last == Some(reported) || daemon_busy {
        InterfaceRefreshDecision::Nothing
    } else {
        InterfaceRefreshDecision::Start
    }
}

impl DaemonService {
    /// Start an interface refresh when a daemon's interfaces differ from the ones its interfaced
    /// subnets were last classified from. Only daemons that send a fingerprint get one, so a
    /// daemon too old to run the type never receives it.
    pub(crate) async fn refresh_interfaces_if_changed(
        &self,
        daemon_id: Uuid,
        site_id: Uuid,
        host_id: Uuid,
        reported: Option<String>,
        carries_interfaced_subnets: bool,
    ) {
        let Some(reported) = reported else {
            return;
        };
        let last = self
            .classified_interfaces
            .read()
            .await
            .get(&daemon_id)
            .cloned();
        // Read only when the fingerprints differ.
        let daemon_busy = !carries_interfaced_subnets
            && last.as_deref() != Some(reported.as_str())
            && (self
                .discovery_service
                .sessions_on_daemon(&daemon_id)
                .await
                .iter()
                .any(|s| !s.phase.is_terminal())
                || self.site_organization_discovery_paused(site_id).await);

        match interface_refresh_decision(
            &reported,
            carries_interfaced_subnets,
            last.as_deref(),
            daemon_busy,
        ) {
            InterfaceRefreshDecision::Nothing => {}
            InterfaceRefreshDecision::Record => {
                self.classified_interfaces
                    .write()
                    .await
                    .insert(daemon_id, reported);
            }
            InterfaceRefreshDecision::Start => {
                // Recorded before it runs, so a refresh that fails is not retried on every
                // report. The next change to the interfaces retries it.
                self.classified_interfaces
                    .write()
                    .await
                    .insert(daemon_id, reported);
                match self
                    .discovery_service
                    .start_interface_refresh(daemon_id, site_id, host_id)
                    .await
                {
                    Ok(session) => tracing::debug!(
                        daemon_id = %daemon_id,
                        session_id = %session.session_id,
                        "Daemon interfaces changed; refreshing its interfaced subnets"
                    ),
                    Err(e) => tracing::warn!(
                        daemon_id = %daemon_id,
                        error = ?e,
                        "Could not start an interface refresh"
                    ),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use InterfaceRefreshDecision::*;

    #[test]
    fn a_report_carrying_classified_subnets_sets_the_baseline() {
        assert_eq!(interface_refresh_decision("a", true, None, false), Record);
        assert_eq!(
            interface_refresh_decision("a", true, Some("b"), true),
            Record
        );
    }

    #[test]
    fn changed_interfaces_start_a_refresh_once_the_daemon_is_free() {
        // A daemon the server has no baseline for (first poll, or after a restart) gets one.
        assert_eq!(interface_refresh_decision("a", false, None, false), Start);
        assert_eq!(
            interface_refresh_decision("a", false, Some("b"), false),
            Start
        );
        // A queued or running session reports the subnets itself.
        assert_eq!(
            interface_refresh_decision("a", false, Some("b"), true),
            Nothing
        );
        assert_eq!(
            interface_refresh_decision("a", false, Some("a"), false),
            Nothing
        );
    }
}
