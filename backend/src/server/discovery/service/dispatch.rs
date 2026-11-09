//! Session lookups, daemon-request building, and session start and update.
use super::state::{self, PromotedSession, SessionMaps, TerminalEffects, UpdateOutcome};
use super::*;
use crate::server::credentials::service::DaemonPlacement;

impl DiscoveryService {
    /// Get the per-daemon integration targets for a session by reverse-looking up the
    /// discovery entity. These drive credential mappings for the scan (init-command targeting).
    pub async fn get_integration_targets_for_session(
        &self,
        session_id: &Uuid,
    ) -> Vec<IntegrationTarget> {
        let discovery_id = self
            .discovery_sessions
            .read()
            .await
            .iter()
            .find(|(_, sid)| *sid == session_id)
            .map(|(did, _)| *did);

        if let Some(discovery_id) = discovery_id
            && let Ok(Some(discovery)) = self.discovery_storage.get_by_id(&discovery_id).await
        {
            return discovery.integration_targets;
        }
        vec![]
    }

    /// Reverse-lookup the discovery_id for a given session_id from the discovery_sessions map.
    pub(crate) async fn lookup_discovery_id(&self, session_id: &Uuid) -> Option<Uuid> {
        self.discovery_sessions
            .read()
            .await
            .iter()
            .find(|(_, sid)| *sid == session_id)
            .map(|(did, _)| *did)
    }

    /// When the most recent historical discovery on `site_id` that finished
    /// strictly before `before` completed. `None` when there was none — a
    /// site's first-ever scan.
    ///
    /// For historical rows `updated_at` is the session's finished-at timestamp.
    /// Filtering on `before` rather than skipping the first row keeps this
    /// correct when the caller's own session row is already persisted (it is,
    /// by the time the digest runs) and when rows share a timestamp.
    ///
    /// The digest uses it to anchor "was this entity still inside its staleness
    /// window as of the previous scan?", which is what distinguishes an entity
    /// that just went stale from one that has been stale for weeks.
    pub async fn previous_historical_finished_at(
        &self,
        site_id: Uuid,
        before: chrono::DateTime<Utc>,
    ) -> Result<Option<chrono::DateTime<Utc>>, anyhow::Error> {
        let filter = StorableFilter::<Discovery>::new_from_site_ids(&[site_id])
            .historical_discovery()
            // A rescan touches one host, so letting it anchor "the previous scan"
            // would make the next scheduled digest under-report everything that
            // changed since the last real sweep.
            .exclude_rescans()
            .updated_before(before)
            .limit(1);
        let discoveries = self
            .discovery_storage
            .get_all_ordered(filter, "updated_at DESC")
            .await?;
        Ok(discoveries.first().map(|d| d.updated_at))
    }

    /// Build a DaemonDiscoveryRequest with all credential mappings resolved.
    /// Called by both DaemonPoll and ServerPoll dispatch points.
    /// `subnets` are the site's subnets as the server holds them, which a ServerPoll daemon
    /// cannot fetch for itself. See `DaemonDiscoveryRequest::subnets`.
    /// `subnets_scanned_elsewhere`: see [`DaemonPlacement::subnets_scanned_elsewhere`].
    pub async fn build_daemon_request(
        &self,
        session: &DiscoveryUpdatePayload,
        site_id: Uuid,
        integration_targets: &[IntegrationTarget],
        daemon_version: Option<&semver::Version>,
        subnets: Vec<Subnet>,
        subnets_scanned_elsewhere: HashSet<Uuid>,
    ) -> DaemonDiscoveryRequest {
        let (credential_mappings, withheld) = if session
            .discovery_type
            .carries_credential_mappings()
        {
            let placement = DaemonPlacement::new(
                session.discovery_type.daemon_host_id(),
                &subnets,
                subnets_scanned_elsewhere,
            );
            let (mut mappings, withheld) = self
                .credential_service
                .build_all_credential_mappings(
                    site_id,
                    integration_targets,
                    daemon_version,
                    &placement,
                )
                .await
                .unwrap_or_default();
            mappings.retain(|mapping| {
                mapping
                    .default_credential
                    .iter()
                    .chain(mapping.ip_overrides.iter().map(|o| &o.credential))
                    .any(|credential| session.discovery_type.keeps_credential(credential.into()))
            });
            // A refresh reports no run, so there is nowhere to say what was withheld from it.
            if session.discovery_type.records_history() {
                (mappings, withheld)
            } else {
                (mappings, Vec::new())
            }
        } else {
            (vec![], vec![])
        };
        // Reported on this scan's record at its terminal update; see `withheld_credentials`.
        self.note_withheld_credentials(session.daemon_id, withheld)
            .await;

        DaemonDiscoveryRequest {
            session_id: session.session_id,
            discovery_id: session.discovery_id.unwrap_or_default(),
            discovery_type: session.discovery_type.clone(),
            credential_mappings,
            subnets,
        }
    }

    /// Create a new discovery session
    pub async fn start_session(
        &self,
        mut discovery: Discovery,
        authentication: AuthenticatedEntity,
    ) -> Result<DiscoveryUpdatePayload, ApiError> {
        // Enforce one active session per discovery configuration
        if self
            .discovery_sessions
            .read()
            .await
            .contains_key(&discovery.id)
        {
            return Err(ApiError::conflict(
                "A session is already running for this discovery",
            ));
        }

        // Update last_run on the discovery (covers all code paths: handler, scheduler, registration)
        discovery.base.run_type.set_last_run(crate::server::shared::clock::now());
        discovery.updated_at = crate::server::shared::clock::now();
        if let Err(e) = self.discovery_storage.update(&mut discovery).await {
            tracing::error!(
                "Failed to update last_run for discovery {}: {}",
                discovery.id,
                e
            );
        }

        let session_id = Uuid::new_v4();
        // Read before `discovery_type` is moved out below.
        let is_full_scan = discovery.full_scan_is_due();

        // Hydrate SNMP credentials
        let discovery_type = if let DiscoveryType::Network {
            host_naming_fallback,
            subnet_ids,
            ..
        } = discovery.base.discovery_type
        {
            DiscoveryType::Network {
                subnet_ids,
                host_naming_fallback,
                snmp_credentials: self
                    .credential_service
                    .build_snmp_credentials_for_discovery(discovery.base.site_id)
                    .await
                    .map_err(|e| ApiError::internal_error(&e.to_string()))?,
            }
        } else {
            discovery.base.discovery_type
        };

        let mut session_payload = DiscoveryUpdatePayload::new(
            session_id,
            discovery.base.daemon_id,
            discovery.base.site_id,
            discovery_type,
            Some(discovery.id),
        );

        // Tell the daemon whether this run is a full port scan
        if let DiscoveryType::Unified {
            ref mut scan_settings,
            ..
        } = session_payload.discovery_type
        {
            scan_settings.is_full_scan = is_full_scan;
        }

        self.enqueue_session(session_payload, Some(discovery.id), authentication)
            .await
    }

    /// Start a daemon's interface refresh: a session with no discovery configuration behind it,
    /// queued like any other.
    pub async fn start_interface_refresh(
        &self,
        daemon_id: Uuid,
        site_id: Uuid,
        host_id: Uuid,
    ) -> Result<DiscoveryUpdatePayload, ApiError> {
        let session_payload = DiscoveryUpdatePayload::new(
            Uuid::new_v4(),
            daemon_id,
            site_id,
            DiscoveryType::InterfaceRefresh { host_id },
            None,
        );
        self.enqueue_session(session_payload, None, AuthenticatedEntity::System)
            .await
    }

    /// Put a new session on its daemon's queue, at the front when the daemon is idle.
    /// `discovery_id` is the configuration it runs, which may run one session at a time.
    async fn enqueue_session(
        &self,
        mut session_payload: DiscoveryUpdatePayload,
        discovery_id: Option<Uuid>,
        authentication: AuthenticatedEntity,
    ) -> Result<DiscoveryUpdatePayload, ApiError> {
        let session_id = session_payload.session_id;
        let site_id = session_payload.site_id;
        let daemon_id = session_payload.daemon_id;

        // Hold running_snapshots.read across the session insertion. This
        // serializes against try_acquire_site_for_snapshot (which takes
        // running_snapshots.write before reading sessions): the snapshot
        // either sees this session and returns false, or this session sees
        // the running_snapshots entry and starts in AwaitingSnapshot. Lock
        // order: running_snapshots → sessions → daemon_sessions.
        let snapshot_lock = self.running_snapshots.read().await;
        let snapshot_blocked = snapshot_lock.contains(&site_id);

        // Check if daemon has any sessions running
        let daemon_is_running_discovery =
            if let Some(daemon_sessions) = self.daemon_sessions.read().await.get(&daemon_id) {
                !daemon_sessions.is_empty()
            } else {
                false
            };

        // Phase decision:
        //   - snapshot in progress on this site → AwaitingSnapshot.
        //     Daemon's queue is irrelevant; release_site_for_snapshot
        //     will run the Queued/Pending decision when the snapshot finishes.
        //   - daemon idle → Pending (front of queue, dispatch event published below).
        //   - daemon busy → Queued (default; promoted later).
        if snapshot_blocked {
            session_payload.phase = DiscoveryPhase::AwaitingSnapshot;
        } else if !daemon_is_running_discovery {
            session_payload.phase = DiscoveryPhase::Pending;
            self.session_last_updated
                .write()
                .await
                .insert(session_id, crate::server::shared::clock::now());
        }

        // The session and its discovery mapping go in under one pair of guards. Inserted apart,
        // `sweep_old` could see the mapping with no session and no tombstone (a `Queued` or
        // `AwaitingSnapshot` session is never stamped) and drop it, losing the guard below for
        // this run. The re-check closes the gap since the check at the top.
        {
            let mut sessions = self.sessions.write().await;
            let mut discovery_sessions = self.discovery_sessions.write().await;
            if let Some(discovery_id) = discovery_id {
                if discovery_sessions.contains_key(&discovery_id) {
                    return Err(ApiError::conflict(
                        "A session is already running for this discovery",
                    ));
                }
                discovery_sessions.insert(discovery_id, session_id);
            }
            sessions.insert(session_id, session_payload.clone());
        }

        // Add session to queue
        self.daemon_sessions
            .write()
            .await
            .entry(daemon_id)
            .or_default()
            .push(session_id);

        // Drop the running_snapshots guard before any awaits that may take
        // running_snapshots.write (e.g. release_site_for_snapshot fired
        // by another task observing our session via the published event).
        drop(snapshot_lock);

        // Publish event only if the session is dispatchable now: not blocked
        // by a snapshot, and the daemon is otherwise idle. AwaitingSnapshot
        // and Queued sessions are published later by release_site_for_snapshot
        // or the existing terminal-completion promotion path.
        if !snapshot_blocked && !daemon_is_running_discovery {
            self.publish_session_event(
                session_payload.into_discovery_event_with_auth(authentication),
            )
            .await;
        }

        self.send_session_update(&session_payload);

        Ok(session_payload)
    }

    /// Update progress for a session
    /// If the session doesn't exist (e.g., server restarted during discovery),
    /// auto-creates it from the payload context to maintain resilience.
    /// Fold a successful scan into a discovery's config row: bump `scan_count`,
    /// clear `force_full_scan`, and consume the one-shot `integration_targets`.
    ///
    /// The row is read `FOR UPDATE` inside a transaction so concurrent session
    /// finalizations serialize instead of losing increments — and so the prune
    /// can't race a concurrent finalization that already dispatched the targets.
    ///
    /// `Site`-scope targets are migrated into the `site_credentials`
    /// junction first, because they are the one scope discovery never promotes
    /// on its own (see `Discovery::take_site_scope_credential_ids`). If that
    /// migration fails they are written back onto the row *after* the prune,
    /// rather than dropping a working broadcast credential — the next successful
    /// scan retries the migration.
    async fn finalize_successful_scan(&self, discovery_id: &Uuid) -> Result<(), Error> {
        let mut tx = self.discovery_storage.begin_transaction().await?;
        let Some(mut parent_discovery) = tx.get_by_id_for_update(discovery_id).await? else {
            return Ok(());
        };

        let site_id = parent_discovery.base.site_id;
        let site_scope_ids = parent_discovery.take_site_scope_credential_ids();
        let (promotable, unpromotable) = self
            .partition_site_promotable(&site_scope_ids, site_id)
            .await;

        for credential_id in unpromotable {
            tracing::warn!(
                discovery_id = %discovery_id,
                %credential_id,
                "Dropping broadcast integration target for a credential type that cannot be \
                 assigned to a site; it was never dispatched"
            );
        }

        let migration_failed = !promotable.is_empty()
            && match self
                .credential_service
                .merge_site_credentials(&site_id, &promotable)
                .await
            {
                Ok(()) => false,
                Err(e) => {
                    tracing::error!(
                        discovery_id = %discovery_id,
                        site_id = %site_id,
                        error = ?e,
                        "Failed to migrate broadcast integration targets to site credentials; \
                         keeping them on the discovery so the credentials stay in effect"
                    );
                    true
                }
            };

        parent_discovery.apply_successful_scan();
        // After the prune, not before — `apply_successful_scan` clears the field.
        if migration_failed {
            parent_discovery.integration_targets = promotable
                .into_iter()
                .map(|credential_id| IntegrationTarget::Site { credential_id })
                .collect();
        }
        parent_discovery.updated_at = crate::server::shared::clock::now();
        tx.update(&mut parent_discovery).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Split broadcast target credentials into those that may be assigned to a
    /// site and those that may not.
    ///
    /// The discovery update handler validates a target's credential type against
    /// the daemon version but not against the scope, so a `Site` target can
    /// exist for a type whose `targets()` excludes it (`apply_integration_target`
    /// warn-drops those at dispatch, so it was never sent to a daemon). Writing
    /// one into the junction would create exactly the assignment
    /// `POST /credentials` refuses. An unresolvable credential is treated as
    /// unpromotable — it no longer exists to assign.
    async fn partition_site_promotable(
        &self,
        credential_ids: &[Uuid],
        site_id: Uuid,
    ) -> (Vec<Uuid>, Vec<Uuid>) {
        let mut promotable = Vec::new();
        let mut unpromotable = Vec::new();
        for credential_id in credential_ids {
            match self.credential_service.get_by_id(credential_id).await {
                Ok(Some(credential))
                    if credential
                        .base
                        .credential_type
                        .targets()
                        .contains(&Target::Site) =>
                {
                    promotable.push(*credential_id)
                }
                Ok(_) => unpromotable.push(*credential_id),
                Err(e) => {
                    tracing::warn!(
                        %credential_id,
                        %site_id,
                        error = ?e,
                        "Failed to resolve broadcast integration target credential; not migrating it"
                    );
                    unpromotable.push(*credential_id);
                }
            }
        }
        (promotable, unpromotable)
    }

    pub async fn update_session(&self, mut update: DiscoveryUpdatePayload) {
        // Enrich discovery_id from authoritative server-side map.
        // Daemon-sent payloads won't have this; server always fills it in.
        if update.discovery_id.is_none() {
            update.discovery_id = self.lookup_discovery_id(&update.session_id).await;
        }

        self.record_update(update, state::apply_update).await;
    }

    /// End a running session on the server's own authority, for a failure the daemon cannot report:
    /// it restarted and lost the session, or it cannot be reached to cancel it. A session that has
    /// already finished, or is no longer tracked, is left alone; the daemon's own outcome stands.
    pub async fn fail_session(
        &self,
        session_id: Uuid,
        reason: DiscoveryTerminalReason,
        error: String,
    ) {
        let Some(mut update) = self.get_session(&session_id).await else {
            return;
        };
        if update.phase.is_terminal() {
            return;
        }

        update.phase = DiscoveryPhase::Failed;
        update.error = Some(error);
        update.finished_at = Some(crate::server::shared::clock::now());
        update.reason = Some(reason);
        self.record_update(update, state::apply_server_update).await;
    }

    /// Apply an update to the session maps with `apply`, then run whatever a finished session
    /// triggers once the locks are released.
    async fn record_update(
        &self,
        update: DiscoveryUpdatePayload,
        apply: impl FnOnce(
            &mut SessionMaps<'_>,
            &DiscoveryUpdatePayload,
            chrono::DateTime<Utc>,
        ) -> UpdateOutcome,
    ) {
        let (outcome, previous) = {
            // The lock order documented on `DiscoveryService`.
            let mut sessions = self.sessions.write().await;
            let mut last_updated = self.session_last_updated.write().await;
            let mut daemon_sessions = self.daemon_sessions.write().await;
            let mut pull_cancellations = self.daemon_pull_cancellations.write().await;
            let mut discovery_sessions = self.discovery_sessions.write().await;
            let previous = sessions.get(&update.session_id).map(|s| s.phase);

            let outcome = apply(
                &mut SessionMaps {
                    sessions: &mut sessions,
                    last_updated: &mut last_updated,
                    daemon_sessions: &mut daemon_sessions,
                    discovery_sessions: &mut discovery_sessions,
                    pull_cancellations: &mut pull_cancellations,
                },
                &update,
                crate::server::shared::clock::now(),
            );

            // A running session's update reaches the stream while the maps are still locked, so
            // two updates for one session arrive in the order they were applied. A finished
            // session's goes out from `finish_terminal`, after the RunFirstScan record it follows.
            // The session as recorded, which carries what the update left out (its last activity).
            if matches!(outcome, UpdateOutcome::Progress)
                && let Some(recorded) = sessions.get(&update.session_id)
            {
                self.send_session_update(recorded);
            }
            (outcome, previous)
        };

        let effects = match outcome {
            UpdateOutcome::Ignored => return,
            UpdateOutcome::Progress => {
                // Once per transition, so a session's events read as its sequence of phases. A
                // session the server had no record of (after a server restart) publishes its first
                // phase with no previous. A terminal phase is published by `finish_terminal`.
                if previous != Some(update.phase) {
                    let mut event = update.into_discovery_event();
                    event.scope.previous_phase = previous;
                    self.publish_session_event(event).await;
                }
                return;
            }
            UpdateOutcome::Terminal(effects) => *effects,
        };
        self.finish_terminal_detached(effects).await;
    }

    /// Run [`Self::finish_terminal`] with no session lock held, on its own task. The terminal
    /// work writes storage and publishes events whose subscribers can reach the network (digest
    /// emails), and a request that times out and drops this future must not cut it short: the
    /// session has already left the maps and its tombstone ignores the re-delivery, so an
    /// interrupted tail would lose the run's record. The caller still waits for the task.
    pub(super) async fn finish_terminal_detached(&self, effects: TerminalEffects) {
        match self.self_ref.upgrade() {
            Some(this) => {
                if let Err(e) =
                    tokio::spawn(async move { this.finish_terminal(effects).await }).await
                {
                    tracing::error!(error = %e, "Finishing a terminal discovery session panicked");
                }
            }
            None => self.finish_terminal(effects).await,
        }
    }

    /// Everything a finished session triggers outside the session maps, in the order its
    /// subscribers depend on. Runs with no session lock held.
    async fn finish_terminal(&self, effects: TerminalEffects) {
        let TerminalEffects {
            mut session,
            parent_discovery_id,
            promoted,
        } = effects;
        let session_id = session.session_id;
        let daemon_id = session.daemon_id;
        let site_id = session.site_id;
        record_session_duration(&session);

        // An interface refresh leaves no record: its only output is the interfaced subnets the
        // daemon reports on its next status. A failure is logged here, and the next change to
        // the daemon's interfaces starts another.
        if !session.discovery_type.records_history() {
            if session.phase != DiscoveryPhase::Complete {
                tracing::warn!(
                    session_id = %session_id,
                    daemon_id = %daemon_id,
                    phase = ?session.phase,
                    error = ?session.error,
                    "Interface refresh did not complete; the daemon's interfaced subnets stay as \
                     last reported"
                );
            }
            if let Some(promoted) = promoted {
                self.publish_promoted(daemon_id, site_id, promoted).await;
            }
            return;
        }

        {
            let is_rescan = session.discovery_type.rescan_target_host_ids().is_some();

            // Record RunFirstScan on the org row BEFORE the SSE update, so it's committed when the
            // SSE-triggered org refetch arrives. `record_setup` publishes only once.
            if session.phase == DiscoveryPhase::Complete
                && matches!(
                    session.discovery_type,
                    DiscoveryType::Network { .. } | DiscoveryType::Unified { .. }
                )
                && let Ok(Some(site)) = self.site_service.get_by_id(&site_id).await
                && let Err(e) = self
                    .organization_service
                    .record_setup(
                        site.base.organization_id,
                        SetupOperation::Completed {
                            item_type: SetupItemDiscriminants::RunFirstScan,
                            subject_id: Some(daemon_id),
                        },
                        AuthenticatedEntity::System,
                    )
                    .await
            {
                tracing::warn!(error = %e, %session_id, "Could not record the first scan");
            }

            session.daemon_version = self.daemon_version(daemon_id).await;
            let _ = self.update_tx.send(session.clone());

            // Here rather than anywhere earlier because this is the one place a terminal payload
            // is processed exactly once — a redundant terminal update from an old daemon has
            // already returned above, so the counters cannot be double-incremented by a ServerPoll
            // re-delivery.
            self.publish_warning_events(
                session.site_id,
                session.session_id,
                session.daemon_id,
                &session.warnings,
            )
            .await;

            // Create historical discovery record
            let name = self
                .historical_run_name(&session, parent_discovery_id)
                .await;

            let historical_discovery = Discovery {
                id: Uuid::new_v4(),
                created_at: session.started_at.unwrap_or(crate::server::shared::clock::now()),
                updated_at: crate::server::shared::clock::now(),
                base: DiscoveryBase {
                    daemon_id: session.daemon_id,
                    site_id: session.site_id,
                    name,
                    tags: Vec::new(),
                    discovery_type: session.discovery_type.clone(),
                    run_type: RunType::Historical {
                        results: Box::new(session.clone()),
                    },
                },
                scan_count: 0,
                force_full_scan: false,
                next_run_is_full_scan: false,
                integration_targets: vec![],
            };

            // Increment scan_count, clear ephemeral fields and consume the one-shot
            // integration targets only on successful completion. Failures/cancellations
            // preserve these so the next retry uses the same config. A rescan's parent is
            // deleted below, so there is nothing to carry forward and the write would be
            // wasted.
            if session.phase == DiscoveryPhase::Complete && !is_rescan {
                // Inside one transaction with a row lock: two finalizations for the
                // same discovery (possible across backend instances) must not both
                // read N and write N+1, nor prune targets the other just dispatched.
                if let Some(discovery_id) = parent_discovery_id
                    && let Err(e) = self.finalize_successful_scan(&discovery_id).await
                {
                    record_finish_error(DiscoveryFinishStep::FinalizeScan);
                    tracing::error!(
                        "Failed to finalize successful scan for discovery {}: {}",
                        discovery_id,
                        e
                    );
                }
            }

            // Save to database
            let history_written =
                if let Err(e) = self.discovery_storage.create(&historical_discovery).await {
                    record_finish_error(DiscoveryFinishStep::HistoryRecord);
                    tracing::error!(
                        "Failed to create historical discovery record for session {}: {}",
                        session.session_id,
                        e
                    );
                    false
                } else {
                    true
                };

            if history_written && let Err(e) = self.record_warnings(&historical_discovery).await {
                record_finish_error(DiscoveryFinishStep::WarningRecords);
                tracing::error!(
                    session_id = %session_id,
                    discovery_id = %historical_discovery.id,
                    error = %e,
                    "Failed to record warnings for historical discovery"
                );
            }

            // The FKs point at the historical row, so they are written once it exists and before
            // its `Created` event goes out.
            if history_written
                && let Some(scanned) = session.scanned.as_ref()
                && let Some(host_service) = self.host_service.get()
                && let Err(e) = host_service
                    .stamp_discovery(scanned, historical_discovery.id)
                    .await
            {
                record_finish_error(DiscoveryFinishStep::DiscoveryFks);
                tracing::error!(
                    session_id = %session_id,
                    discovery_id = %historical_discovery.id,
                    error = %e,
                    "Failed to stamp discovery FKs on scanned entities"
                );
            }

            // The digest covers successful sweeps. A rescan is user-initiated and watched live in
            // the UI, so a digest about one host is noise. Its email goes out from the
            // `DiscoveryDigestOperation` subscriber.
            if history_written
                && session.phase == DiscoveryPhase::Complete
                && !is_rescan
                && let Some(digest_service) = self.digest_service.get()
            {
                // `scanned` is how the digest tells a child removed from a scanned host from one
                // the scan never looked at. A daemon that sent none gets an empty set.
                let empty = ScannedEntityIds::default();
                let scanned = session.scanned.as_ref().unwrap_or(&empty);
                // Not retried: it may have published the digest, and so sent its emails, before
                // failing, and a second run would send them again.
                if let Err(e) = digest_service
                    .compute_and_publish(&session, scanned, historical_discovery.id)
                    .await
                {
                    record_finish_error(DiscoveryFinishStep::Digest);
                    tracing::error!(
                        session_id = %session_id,
                        error = %e,
                        "Failed to compute the discovery digest"
                    );
                }
            }

            // Guests a run no longer reported under the hosts whose guests it listed in full were
            // removed. After the digest, which reports them as removed from the stored rows.
            if let Some(scanned) = session.scanned.as_ref()
                && let Some(host_service) = self.host_service.get()
                && let Err(e) = host_service
                    .remove_unreported_guests(
                        site_id,
                        scanned,
                        &session.credential_results,
                        AuthenticatedEntity::System,
                    )
                    .await
            {
                tracing::error!(
                    session_id = %session_id,
                    error = %e,
                    "Failed to remove guests the run no longer reported"
                );
            }

            if history_written
                && let Some(scope) = EntityScope::from_ids(
                    historical_discovery.id(),
                    historical_discovery.clone().into(),
                    self.get_site_id(&historical_discovery),
                    self.get_organization_id(&historical_discovery),
                )
            {
                self.event_bus()
                    .publish(
                        Event::new(scope, EntityOperation::Created, AuthenticatedEntity::System)
                            .with_flags(EntityEventFlags::default()),
                    )
                    .await;
            }

            // The terminal phase event, for subscribers that want to know a session ended.
            //
            // Neighbour resolution used to be driven from here and annotated the row above, which
            // is why the row is created first — published the other way round, the debounced
            // subscriber raced a row that did not exist yet and quietly found nothing to annotate.
            // That work now runs in `DaemonService::process_discovery_progress` *before* this
            // function is called, so its findings are in the row as written and there is no race
            // left to order around.
            self.event_bus()
                .publish(session.into_discovery_event())
                .await;

            // A rescan's parent exists only to carry the targets to the daemon. Delete it now that
            // the historical row (which the user actually sees) is persisted and the scanned
            // entities' discovery FKs are stamped. Those FKs point at the historical row, not this
            // one, and are ON DELETE SET NULL regardless. The events published above carry their
            // own copy of what they describe, so their subscribers never read this row.
            if is_rescan
                && let Some(discovery_id) = parent_discovery_id
                && let Err(e) = self.discovery_storage.delete(&discovery_id).await
            {
                tracing::warn!(
                    discovery_id = %discovery_id,
                    error = ?e,
                    "Failed to delete transient rescan discovery; the startup sweeper will retry"
                );
            }

            // Publish event which will trigger notifying any daemons in ServerPoll to start session
            // If daemon is daemon_poll mode, it will request next session on its next poll
            if let Some(promoted) = promoted {
                self.publish_promoted(daemon_id, site_id, promoted).await;
            }
        }

        // Last, and whatever happened above. While the mapping exists, starting this discovery
        // again is refused as already running, which is what stops a second run beginning while
        // this one's record is still being written.
        self.discovery_sessions
            .write()
            .await
            .retain(|_, sid| *sid != session_id);
    }

    /// Announce a queued session that has moved to the front of its daemon's queue.
    pub(super) async fn publish_promoted(
        &self,
        daemon_id: Uuid,
        site_id: Uuid,
        promoted: PromotedSession,
    ) {
        let mut started_payload = DiscoveryUpdatePayload::new(
            promoted.session_id,
            daemon_id,
            site_id,
            promoted.discovery_type,
            promoted.discovery_id,
        );
        started_payload.phase = DiscoveryPhase::Pending;

        self.publish_session_event(started_payload.into_discovery_event())
            .await;
    }

    /// The daemon's version as the server last recorded it, for stamping a finished session.
    pub(super) async fn daemon_version(&self, daemon_id: Uuid) -> Option<String> {
        let daemon_service = self.daemon_service.get()?;
        let daemon = daemon_service.get_by_id(&daemon_id).await.ok().flatten()?;
        daemon.base.version.map(|v| v.to_string())
    }
}

/// A write that records a finished session, for `scanopy_discovery_finish_errors_total{step}`.
#[derive(Debug, Clone, Copy, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(super) enum DiscoveryFinishStep {
    /// Bumping `scan_count` and consuming the one-shot integration targets.
    FinalizeScan,
    /// The historical discovery row: the run's record.
    HistoryRecord,
    /// The run's warnings, as `discovery_warnings` rows.
    WarningRecords,
    /// `last_discovery_id` / `first_discovery_id` on the entities the run scanned.
    DiscoveryFks,
    /// The run's digest, computed and handed to the email subscriber.
    Digest,
}

/// A finished session the server failed to record. The session has already left the maps and its
/// tombstone ignores a re-sent update, so nothing retries it; the write's data is held only in
/// memory and is lost.
pub(super) fn record_finish_error(step: DiscoveryFinishStep) {
    let step: &'static str = step.into();
    metrics::counter!("scanopy_discovery_finish_errors_total", "step" => step).increment(1);
}

/// Wall-clock time from start to end of a session that ended, by terminal reason. A session that
/// never started (refused as busy, or dropped from the queue) has no duration to record.
pub(super) fn record_session_duration(session: &DiscoveryUpdatePayload) {
    use crate::server::shared::types::metadata::HasId;
    let (Some(started), Some(finished)) = (session.started_at, session.finished_at) else {
        return;
    };
    let seconds = finished
        .signed_duration_since(started)
        .num_milliseconds()
        .max(0) as f64
        / 1000.0;
    metrics::histogram!(
        "scanopy_discovery_session_duration_seconds",
        "reason" => session.reason.map(|r| r.id()).unwrap_or("none"),
    )
    .record(seconds);
}
