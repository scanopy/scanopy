//! The Issues built from a daemon's latest runs: the warnings they raised, and the scans whose
//! latest run didn't finish.

use super::*;

/// One item per daemon and code, from the newest run that raised it. A daemon deleted since has
/// nothing to fix. A credential withheld from a daemon too old for it clears once the daemon is
/// upgraded past the gap, before the next run says so.
pub(super) fn run_warning_items(ctx: &SetupContext) -> Vec<SetupItem> {
    let mut items: Vec<SetupItem> = Vec::new();
    for run in &ctx.run_warnings {
        let Some(daemon) = ctx.daemons.iter().find(|d| d.id == run.daemon_id) else {
            continue;
        };
        for warning in &run.warnings {
            if let DiscoveryWarning::CredentialNeedsNewerDaemon {
                last_unsupported_daemon_version,
                ..
            } = warning
                && daemon
                    .version
                    .as_ref()
                    .is_some_and(|v| v > last_unsupported_daemon_version)
            {
                continue;
            }
            let code = warning.code();
            let existing = items.iter_mut().find_map(|item| match item {
                SetupItem::RunWarnings {
                    daemon_id,
                    code: c,
                    warnings,
                    ..
                } if *daemon_id == daemon.id && *c == code => Some(warnings),
                _ => None,
            });
            match existing {
                Some(warnings) => warnings.push(warning.clone()),
                None => items.push(SetupItem::RunWarnings {
                    daemon_id: daemon.id,
                    daemon_name: daemon.name.clone(),
                    site_id: daemon.site_id,
                    discovery_id: run.discovery_id,
                    scan_id: run.scan_id,
                    code,
                    warnings: vec![warning.clone()],
                }),
            }
        }
    }
    items
}

/// One item per daemon and reason, naming each scan whose latest run ended that way. A deleted
/// daemon's scans have nothing to fix.
///
/// Left out:
/// - A lost connection while the daemon's health still reports it. The DaemonHealth item owns
///   that cause and its fix, and no rerun can start until the connection is back. Once the daemon
///   reconnects, the scan, still not finished, is listed here with its own fix.
/// - A one-off the next run usually gets past ([`DiscoveryTerminalReason::clears_on_next_run`]),
///   until it happens twice in a row or no schedule will run the scan again.
pub(super) fn failed_scan_items(ctx: &SetupContext) -> Vec<SetupItem> {
    let mut items: Vec<SetupItem> = Vec::new();
    for run in &ctx.failed_scans {
        let Some(daemon) = ctx.daemons.iter().find(|d| d.id == run.daemon_id) else {
            continue;
        };
        if run.reason.is_lost_connection() && daemon.health.is_disconnected() {
            continue;
        }
        if run.reason.clears_on_next_run() && run.scheduled && !run.previous_run_failed {
            continue;
        }
        let scan = FailedScan {
            scan_id: run.scan_id,
            name: run.scan_name.clone(),
            run_id: run.run_id,
        };
        let existing = items.iter_mut().find_map(|item| match item {
            SetupItem::FailedScans {
                daemon_id,
                reason,
                scans,
                ..
            } if *daemon_id == daemon.id && *reason == run.reason => Some(scans),
            _ => None,
        });
        match existing {
            Some(scans) => scans.push(scan),
            None => items.push(SetupItem::FailedScans {
                daemon_id: daemon.id,
                daemon_name: daemon.name.clone(),
                site_id: daemon.site_id,
                reason: run.reason,
                scans: vec![scan],
            }),
        }
    }
    items
}

/// Whether a stored run warning still belongs on the Setup list: its code is a setup issue, and a
/// credential warning still names its credential (one deleted since has nothing to fix).
pub fn is_setup_issue_warning(warning: &DiscoveryWarning) -> bool {
    let code: DiscoveryWarningCode = warning.code();
    code.is_setup_issue() && (!code.concerns_credential() || warning.credential_id().is_some())
}
