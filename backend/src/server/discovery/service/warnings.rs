//! A historical run's warnings: written as `discovery_warnings` rows when the run is recorded, and
//! read back onto `run_type.results.warnings` wherever the run is read through this service.
use super::*;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::credentials::r#impl::base::Credential;
use crate::server::credentials::service::{FoundIntegration, SiteTriedAt};
use crate::server::setup::items::{DaemonRunWarnings, FailedScanRun, is_setup_issue_warning};
use crate::server::sites::r#impl::Site;

impl DiscoveryService {
    /// Store the warnings a historical run carries in memory. Called once, right after the run's
    /// row is created, since the row's storage never holds them.
    pub async fn record_warnings(&self, discovery: &Discovery) -> Result<()> {
        let RunType::Historical { results } = &discovery.base.run_type else {
            return Ok(());
        };
        self.warning_service
            .record_for_run(discovery.id, discovery.base.site_id, &results.warnings)
            .await
    }

    /// What each site's latest completed full scan found on hosts with no credential, as
    /// recorded then, and when each of those scans started. A host deleted since is left out. The
    /// caller re-checks coverage, since a credential saved after the scan covers the host now, and
    /// the start times say which site credentials that scan already tried.
    ///
    /// A rescan is skipped: it touches one host, so it would hide every other host the last full
    /// scan reported.
    pub async fn latest_integrations_without_credentials(
        &self,
        site_ids: &[Uuid],
    ) -> Result<(Vec<FoundIntegration>, SiteTriedAt)> {
        let mut found = Vec::new();
        let mut tried_at = SiteTriedAt::new();
        for site_id in site_ids {
            let filter = StorableFilter::<Discovery>::new_from_site_ids(&[*site_id])
                .historical_discovery()
                .exclude_rescans()
                .completed_runs()
                .limit(1);
            let mut latest = self
                .discovery_storage
                .get_all_ordered(filter, "updated_at DESC")
                .await?;
            self.attach_warnings(&mut latest).await?;
            for discovery in &latest {
                let RunType::Historical { results } = &discovery.base.run_type else {
                    continue;
                };
                if let Some(started) = results.started_at.or(results.finished_at) {
                    tried_at.insert(discovery.base.site_id, started);
                }
                found.extend(results.warnings.iter().filter_map(|w| match w {
                    DiscoveryWarning::IntegrationWithoutCredential {
                        host_id: Some(host_id),
                        integration,
                    } => Some(FoundIntegration {
                        host_id: *host_id,
                        site_id: discovery.base.site_id,
                        integration: *integration,
                    }),
                    _ => None,
                }));
            }
        }
        Ok((found, tried_at))
    }

    /// A daemon's recorded runs the Setup list reads, newest first: the 20 latest, rescans left
    /// out, since a rescan touches one host and would hide what the last full scan found. Run
    /// warnings and failed scans both read this window, so they agree on which runs count.
    fn latest_runs_filter(daemon_id: &Uuid) -> StorableFilter<Discovery> {
        StorableFilter::<Discovery>::new_from_uuid_column("daemon_id", daemon_id)
            .historical_discovery()
            .exclude_rescans()
            .limit(20)
    }

    /// Each daemon's scans whose latest run in [`Self::latest_runs_filter`]'s window didn't
    /// finish, newest first. A later completed run of the scan replaces the failed one, which
    /// clears it, and so does a run someone cancelled. A run of a scan deleted since, or recorded
    /// before runs named their scan, has nothing to rerun and is left out.
    pub async fn latest_failed_scans_per_daemon(
        &self,
        daemon_ids: &[Uuid],
    ) -> Result<Vec<FailedScanRun>> {
        let mut failed = Vec::new();
        for daemon_id in daemon_ids {
            let scans: HashMap<Uuid, Discovery> = self
                .discovery_storage
                .get_all(
                    StorableFilter::<Discovery>::new_from_uuid_column("daemon_id", daemon_id)
                        .live_configs(),
                )
                .await?
                .into_iter()
                .map(|d| (d.id, d))
                .collect();
            let runs = self
                .discovery_storage
                .get_all_ordered(Self::latest_runs_filter(daemon_id), "updated_at DESC")
                .await?;
            failed.extend(failed_scan_runs(*daemon_id, &runs, &scans));
        }
        Ok(failed)
    }

    /// Each daemon's latest completed run of each discovery type, newest first, holding only the
    /// warnings that are Setup issues. A newer run of a type replaces the older one, so a warning
    /// clears once the next run of its type comes back clean, and a run of another type leaves it.
    pub async fn latest_runs_per_daemon(
        &self,
        daemon_ids: &[Uuid],
    ) -> Result<Vec<DaemonRunWarnings>> {
        let mut latest = Vec::new();
        for daemon_id in daemon_ids {
            let filter = Self::latest_runs_filter(daemon_id).completed_runs();
            let runs = self
                .discovery_storage
                .get_all_ordered(filter, "updated_at DESC")
                .await?;
            let mut seen_types: HashSet<&'static str> = HashSet::new();
            let mut per_type: Vec<Discovery> = runs
                .into_iter()
                .filter(|run| seen_types.insert((&run.base.discovery_type).into()))
                .collect();
            self.attach_warnings(&mut per_type).await?;
            latest.extend(per_type.into_iter().filter_map(|run| {
                let RunType::Historical { results } = run.base.run_type else {
                    return None;
                };
                let warnings: Vec<DiscoveryWarning> = results
                    .warnings
                    .into_iter()
                    .filter(is_setup_issue_warning)
                    .collect();
                (!warnings.is_empty()).then_some(DaemonRunWarnings {
                    daemon_id: *daemon_id,
                    discovery_id: run.id,
                    scan_id: results.discovery_id,
                    warnings,
                })
            }));
        }
        Ok(latest)
    }

    /// The organization's credentials no future scan will send to a daemon: no site or host
    /// assignment, and no discovery of the organization targets them.
    ///
    /// `finishing_session` is a run completing now. Its `Hosts` and `DaemonHost` targets don't
    /// count, because the successful scan clears them: a working one becomes a host assignment, a
    /// failed one leaves nothing. Its `Site` targets still count, since the scan moves them into
    /// the site's credentials.
    pub async fn targetless_credentials(
        &self,
        organization_id: &Uuid,
        finishing_session: Option<&Uuid>,
    ) -> Result<Vec<Credential>> {
        let site_ids: Vec<Uuid> = self
            .site_service
            .get_all(StorableFilter::<Site>::new_from_org_id(organization_id))
            .await?
            .into_iter()
            .map(|s| s.id)
            .collect();
        let finishing = match finishing_session {
            Some(session_id) => self.lookup_discovery_id(session_id).await,
            None => None,
        };

        let mut targeted = HashSet::new();
        if !site_ids.is_empty() {
            let filter = StorableFilter::<Discovery>::new_from_site_ids(&site_ids).live_configs();
            for discovery in self.discovery_storage.get_all(filter).await? {
                let clearing = Some(discovery.id) == finishing;
                targeted.extend(
                    discovery
                        .integration_targets
                        .iter()
                        .filter(|t| !clearing || matches!(t, IntegrationTarget::Site { .. }))
                        .map(IntegrationTarget::credential_id),
                );
            }
        }

        self.credential_service
            .targetless_credentials(organization_id, &targeted)
            .await
    }

    /// Fill each historical run's warnings from the table.
    ///
    /// Rows are read only from the runs' own sites, so a warning is reachable only through a run
    /// the caller was already allowed to read, the same way its tags are.
    pub(crate) async fn attach_warnings(&self, discoveries: &mut [Discovery]) -> Result<()> {
        let historical: Vec<&Discovery> = discoveries
            .iter()
            .filter(|d| matches!(d.base.run_type, RunType::Historical { .. }))
            .collect();
        if historical.is_empty() {
            return Ok(());
        }
        let ids: Vec<Uuid> = historical.iter().map(|d| d.id).collect();
        let site_ids: Vec<Uuid> = historical
            .iter()
            .map(|d| d.base.site_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let mut by_run = self.warning_service.for_runs(&ids, &site_ids).await?;
        for discovery in discoveries {
            if let RunType::Historical { results } = &mut discovery.base.run_type {
                // A row on another site than its run is never attached, even when it shares the
                // run's id.
                results.warnings = by_run
                    .remove(&discovery.id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|(site_id, _)| *site_id == discovery.base.site_id)
                    .map(|(_, warning)| warning)
                    .collect();
            }
        }
        Ok(())
    }
}

/// The scans among `scans` (a daemon's live configs) whose newest run in `runs` (newest first)
/// didn't finish, with whether the run before it failed too.
fn failed_scan_runs(
    daemon_id: Uuid,
    runs: &[Discovery],
    scans: &HashMap<Uuid, Discovery>,
) -> Vec<FailedScanRun> {
    // Each scan's runs' end reasons and errors, newest first, in the order its newest run appears.
    type Ended<'a> = (Uuid, Option<DiscoveryTerminalReason>, Option<&'a String>);
    let mut by_scan: Vec<(Uuid, Vec<Ended>)> = Vec::new();
    for run in runs {
        let RunType::Historical { results } = &run.base.run_type else {
            continue;
        };
        let Some(scan_id) = results.discovery_id else {
            continue;
        };
        let ended = (
            run.id,
            DiscoveryTerminalReason::of_run(results.phase, results.reason),
            results.error.as_ref(),
        );
        match by_scan.iter_mut().find(|(id, _)| *id == scan_id) {
            Some((_, runs)) => runs.push(ended),
            None => by_scan.push((scan_id, vec![ended])),
        }
    }
    let failed =
        |reason: Option<DiscoveryTerminalReason>| reason.is_some_and(|r| r.issue_fix().is_some());
    by_scan
        .into_iter()
        .filter_map(|(scan_id, ended)| {
            let scan = scans.get(&scan_id)?;
            let &(run_id, Some(reason), error) = ended.first()? else {
                return None;
            };
            failed(Some(reason)).then(|| FailedScanRun {
                daemon_id,
                scan_id,
                scan_name: scan.base.name.clone(),
                run_id,
                reason,
                error: error.cloned(),
                previous_run_failed: ended.get(1).is_some_and(|&(_, r, _)| failed(r)),
                scheduled: scan.base.run_type.is_scheduled_enabled(),
                next_run_full: scan.next_run_is_full_scan,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(scheduled: bool) -> Discovery {
        let mut scan = Discovery {
            id: Uuid::new_v4(),
            ..Default::default()
        };
        scan.base.name = "Office LAN".to_string();
        if scheduled {
            scan.base.run_type = RunType::Scheduled {
                cron_schedule: "0 0 * * * *".to_string(),
                last_run: None,
                enabled: true,
                timezone: None,
            };
        }
        scan
    }

    /// A recorded run of `scan` that ended in `phase`, with `reason` as the server stored it.
    fn run(
        scan: &Discovery,
        phase: DiscoveryPhase,
        reason: Option<DiscoveryTerminalReason>,
    ) -> Discovery {
        let mut results = DiscoveryUpdatePayload::new(
            Uuid::new_v4(),
            Uuid::nil(),
            Uuid::nil(),
            DiscoveryType::default(),
            None,
        );
        results.phase = phase;
        results.reason = reason;
        results.discovery_id = Some(scan.id);
        let mut run = Discovery {
            id: Uuid::new_v4(),
            ..Default::default()
        };
        run.base.run_type = RunType::Historical {
            results: Box::new(results),
        };
        run
    }

    fn failed(scan: &Discovery, reason: DiscoveryTerminalReason) -> Discovery {
        run(scan, DiscoveryPhase::Failed, Some(reason))
    }

    fn scans(list: &[&Discovery]) -> HashMap<Uuid, Discovery> {
        list.iter().map(|s| (s.id, (*s).clone())).collect()
    }

    /// Each reason a run can fail with makes its scan a failed scan, carrying the run's error. A run
    /// that completed, or that someone cancelled, doesn't.
    #[test]
    fn a_scan_whose_latest_run_failed_is_listed_with_its_reason() {
        use strum::IntoEnumIterator;
        let s = scan(false);
        for reason in DiscoveryTerminalReason::iter() {
            let mut run = failed(&s, reason);
            if let RunType::Historical { results } = &mut run.base.run_type {
                results.error = Some("raw socket denied".to_string());
            }
            let found = failed_scan_runs(Uuid::nil(), &[run], &scans(&[&s]));
            let listed = !matches!(
                reason,
                DiscoveryTerminalReason::Completed | DiscoveryTerminalReason::UserCancelled
            );
            assert_eq!(found.len(), usize::from(listed), "{reason:?}");
            if listed {
                assert_eq!(found[0].reason, reason);
                assert_eq!(found[0].error.as_deref(), Some("raw socket denied"));
            }
        }
    }

    /// A later completed run of the same scan clears it; another scan's failure stays.
    #[test]
    fn a_completed_rerun_clears_only_its_own_scan() {
        let (a, b) = (scan(false), scan(false));
        let runs = [
            run(
                &a,
                DiscoveryPhase::Complete,
                Some(DiscoveryTerminalReason::Completed),
            ),
            failed(&b, DiscoveryTerminalReason::SessionSilent),
            failed(&a, DiscoveryTerminalReason::DaemonReportedFailure),
        ];
        let found = failed_scan_runs(Uuid::nil(), &runs, &scans(&[&a, &b]));
        assert_eq!(found.iter().map(|f| f.scan_id).collect::<Vec<_>>(), [b.id]);
    }

    /// A run recorded before runs carried a reason reads its phase. A deleted scan has nothing to
    /// rerun.
    #[test]
    fn a_failed_run_without_a_reason_counts_and_a_deleted_scans_does_not() {
        let (kept, deleted) = (scan(false), scan(false));
        let runs = [
            run(&kept, DiscoveryPhase::Failed, None),
            failed(&deleted, DiscoveryTerminalReason::DaemonReportedFailure),
        ];
        let found = failed_scan_runs(Uuid::nil(), &runs, &scans(&[&kept]));
        assert!(matches!(
            found.as_slice(),
            [FailedScanRun { scan_id, reason: DiscoveryTerminalReason::DaemonReportedFailure, .. }]
                if *scan_id == kept.id
        ));
    }

    /// The run before the latest says whether a one-off repeated; the scan's own schedule says
    /// whether anything will run it again.
    #[test]
    fn a_failed_scan_knows_whether_it_failed_before_and_runs_again() {
        let s = scan(true);
        let twice = [
            failed(&s, DiscoveryTerminalReason::DaemonBusy),
            failed(&s, DiscoveryTerminalReason::DaemonRestarted),
        ];
        let found = failed_scan_runs(Uuid::nil(), &twice, &scans(&[&s]));
        assert!(found[0].previous_run_failed && found[0].scheduled);

        let after_success = [
            failed(&s, DiscoveryTerminalReason::DaemonBusy),
            run(
                &s,
                DiscoveryPhase::Complete,
                Some(DiscoveryTerminalReason::Completed),
            ),
        ];
        let found = failed_scan_runs(Uuid::nil(), &after_success, &scans(&[&s]));
        assert!(!found[0].previous_run_failed);
    }
}
