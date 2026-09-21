//! Session lookups, daemon-request building, and session start/update/cancel.
use super::state::{self, PromotedSession, SessionMaps, TerminalEffects, UpdateOutcome};
use super::*;

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

    /// When the most recent historical discovery on `network_id` that finished
    /// strictly before `before` completed. `None` when there was none — a
    /// network's first-ever scan.
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
        network_id: Uuid,
        before: chrono::DateTime<Utc>,
    ) -> Result<Option<chrono::DateTime<Utc>>, anyhow::Error> {
        let filter = StorableFilter::<Discovery>::new_from_network_ids(&[network_id])
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
    pub async fn build_daemon_request(
        &self,
        session: &DiscoveryUpdatePayload,
        network_id: Uuid,
        integration_targets: &[IntegrationTarget],
        daemon_version: Option<&semver::Version>,
    ) -> Result<DaemonDiscoveryRequest, anyhow::Error> {
        let credential_mappings = if session.discovery_type.runs_network_scan() {
            self.credential_service
                .build_all_credential_mappings(network_id, integration_targets, daemon_version)
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };

        Ok(DaemonDiscoveryRequest {
            session_id: session.session_id,
            discovery_id: session.discovery_id.unwrap_or_default(),
            discovery_type: session.discovery_type.clone(),
            credential_mappings,
        })
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
        discovery.base.run_type.set_last_run(Utc::now());
        discovery.updated_at = Utc::now();
        if let Err(e) = self.discovery_storage.update(&mut discovery).await {
            tracing::error!(
                "Failed to update last_run for discovery {}: {}",
                discovery.id,
                e
            );
        }

        let session_id = Uuid::new_v4();

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
                    .build_snmp_credentials_for_discovery(discovery.base.network_id)
                    .await
                    .map_err(|e| ApiError::internal_error(&e.to_string()))?,
            }
        } else {
            discovery.base.discovery_type
        };

        let mut session_payload = DiscoveryUpdatePayload::new(
            session_id,
            discovery.base.daemon_id,
            discovery.base.network_id,
            discovery_type,
            Some(discovery.id),
        );

        // Compute whether this scan should be a full port scan and set on scan_settings
        if let DiscoveryType::Unified {
            ref mut scan_settings,
            ..
        } = session_payload.discovery_type
        {
            let full_scan_interval = scan_settings.full_scan_interval.unwrap_or(
                crate::server::discovery::r#impl::scan_settings::defaults::full_scan_interval(),
            );
            // 0 = never full scan (all light), 1 = every scan is full
            scan_settings.is_full_scan = discovery.force_full_scan
                || (full_scan_interval != 0
                    && (full_scan_interval == 1
                        || discovery.scan_count == 1
                        || (discovery.scan_count > 1
                            && discovery.scan_count.is_multiple_of(full_scan_interval))));
        }

        // Track discovery -> session mapping
        self.discovery_sessions
            .write()
            .await
            .insert(discovery.id, session_id);

        // Hold running_snapshots.read across the session insertion. This
        // serializes against try_acquire_network_for_snapshot (which takes
        // running_snapshots.write before reading sessions): the snapshot
        // either sees this session and returns false, or this session sees
        // the running_snapshots entry and starts in AwaitingSnapshot. Lock
        // order: running_snapshots → sessions → daemon_sessions.
        let snapshot_lock = self.running_snapshots.read().await;
        let snapshot_blocked = snapshot_lock.contains(&discovery.base.network_id);

        // Check if daemon has any sessions running
        let daemon_is_running_discovery = if let Some(daemon_sessions) = self
            .daemon_sessions
            .read()
            .await
            .get(&discovery.base.daemon_id)
        {
            !daemon_sessions.is_empty()
        } else {
            false
        };

        // Phase decision:
        //   - snapshot in progress on this network → AwaitingSnapshot.
        //     Daemon's queue is irrelevant; release_network_for_snapshot
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
                .insert(session_id, Utc::now());
        }

        // Add to session map
        self.sessions
            .write()
            .await
            .insert(session_id, session_payload.clone());

        // Add session to queue
        self.daemon_sessions
            .write()
            .await
            .entry(discovery.base.daemon_id)
            .or_default()
            .push(session_id);

        // Drop the running_snapshots guard before any awaits that may take
        // running_snapshots.write (e.g. release_network_for_snapshot fired
        // by another task observing our session via the published event).
        drop(snapshot_lock);

        // Publish event only if the session is dispatchable now: not blocked
        // by a snapshot, and the daemon is otherwise idle. AwaitingSnapshot
        // and Queued sessions are published later by release_network_for_snapshot
        // or the existing terminal-completion promotion path.
        if !snapshot_blocked && !daemon_is_running_discovery {
            self.event_bus()
                .publish(session_payload.into_discovery_event_with_auth(authentication))
                .await
                .map_err(|e| ApiError::internal_error(&e.to_string()))?;
        }

        let _ = self.update_tx.send(session_payload.clone());

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
    /// `Network`-scope targets are migrated into the `network_credentials`
    /// junction first, because they are the one scope discovery never promotes
    /// on its own (see `Discovery::take_network_scope_credential_ids`). If that
    /// migration fails they are written back onto the row *after* the prune,
    /// rather than dropping a working broadcast credential — the next successful
    /// scan retries the migration.
    async fn finalize_successful_scan(&self, discovery_id: &Uuid) -> Result<(), Error> {
        let mut tx = self.discovery_storage.begin_transaction().await?;
        let Some(mut parent_discovery) = tx.get_by_id_for_update(discovery_id).await? else {
            return Ok(());
        };

        let network_id = parent_discovery.base.network_id;
        let network_scope_ids = parent_discovery.take_network_scope_credential_ids();
        let (promotable, unpromotable) = self
            .partition_network_promotable(&network_scope_ids, network_id)
            .await;

        for credential_id in unpromotable {
            tracing::warn!(
                discovery_id = %discovery_id,
                %credential_id,
                "Dropping broadcast integration target for a credential type that cannot be \
                 assigned to a network; it was never dispatched"
            );
        }

        let migration_failed = !promotable.is_empty()
            && match self
                .credential_service
                .merge_network_credentials(&network_id, &promotable)
                .await
            {
                Ok(()) => false,
                Err(e) => {
                    tracing::error!(
                        discovery_id = %discovery_id,
                        network_id = %network_id,
                        error = ?e,
                        "Failed to migrate broadcast integration targets to network credentials; \
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
                .map(|credential_id| IntegrationTarget::Network { credential_id })
                .collect();
        }
        parent_discovery.updated_at = Utc::now();
        tx.update(&mut parent_discovery).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Split broadcast target credentials into those that may be assigned to a
    /// network and those that may not.
    ///
    /// The discovery update handler validates a target's credential type against
    /// the daemon version but not against the scope, so a `Network` target can
    /// exist for a type whose `targets()` excludes it (`apply_integration_target`
    /// warn-drops those at dispatch, so it was never sent to a daemon). Writing
    /// one into the junction would create exactly the assignment
    /// `POST /credentials` refuses. An unresolvable credential is treated as
    /// unpromotable — it no longer exists to assign.
    async fn partition_network_promotable(
        &self,
        credential_ids: &[Uuid],
        network_id: Uuid,
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
                        .contains(&Target::Network) =>
                {
                    promotable.push(*credential_id)
                }
                Ok(_) => unpromotable.push(*credential_id),
                Err(e) => {
                    tracing::warn!(
                        %credential_id,
                        %network_id,
                        error = ?e,
                        "Failed to resolve broadcast integration target credential; not migrating it"
                    );
                    unpromotable.push(*credential_id);
                }
            }
        }
        (promotable, unpromotable)
    }

    pub async fn update_session(&self, mut update: DiscoveryUpdatePayload) -> Result<(), Error> {
        // Enrich discovery_id from authoritative server-side map.
        // Daemon-sent payloads won't have this; server always fills it in.
        if update.discovery_id.is_none() {
            update.discovery_id = self.lookup_discovery_id(&update.session_id).await;
        }

        tracing::debug!("Updated session {:?}", update);

        self.record_update(update, state::apply_update).await;
        Ok(())
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

        tracing::warn!(
            session_id = %session_id,
            daemon_id = %update.daemon_id,
            network_id = %update.network_id,
            phase = ?update.phase,
            reason = ?reason,
            error = %error,
            "Failing discovery session"
        );

        update.phase = DiscoveryPhase::Failed;
        update.error = Some(error);
        update.finished_at = Some(Utc::now());
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
        let outcome = {
            // The lock order documented on `DiscoveryService`.
            let mut sessions = self.sessions.write().await;
            let mut last_updated = self.session_last_updated.write().await;
            let mut daemon_sessions = self.daemon_sessions.write().await;
            let mut pull_cancellations = self.daemon_pull_cancellations.write().await;
            let mut discovery_sessions = self.discovery_sessions.write().await;

            let outcome = apply(
                &mut SessionMaps {
                    sessions: &mut sessions,
                    last_updated: &mut last_updated,
                    daemon_sessions: &mut daemon_sessions,
                    discovery_sessions: &mut discovery_sessions,
                    pull_cancellations: &mut pull_cancellations,
                },
                &update,
                Utc::now(),
            );

            // A running session's update reaches the stream while the maps are still locked, so
            // two updates for one session arrive in the order they were applied. A finished
            // session's goes out from `finish_terminal`, after the onboarding milestone it follows.
            if matches!(outcome, UpdateOutcome::Progress) {
                let _ = self.update_tx.send(update.clone());
            }
            outcome
        };

        let effects = match outcome {
            UpdateOutcome::Ignored => return,
            UpdateOutcome::Progress => {
                tracing::debug!(
                    session_id = %update.session_id,
                    phase = %update.phase,
                    progress = %update.progress,
                    "Updated session",
                );
                return;
            }
            UpdateOutcome::Terminal(effects) => *effects,
        };

        // Nothing below holds a session lock. The terminal work writes storage and publishes
        // events whose subscribers can reach the network (digest emails), and it runs on its own
        // task so a daemon request that times out and drops this future cannot cut it short: the
        // session has already left the maps and its tombstone ignores the re-delivery, so an
        // interrupted tail would lose the run's record. The request still waits for the task, so
        // callers see the same ordering as before.
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
        let network_id = session.network_id;

        {
            let is_rescan = session.discovery_type.rescan_target_host_id().is_some();

            // Publish onboarding milestone BEFORE SSE update so it's
            // in the DB when the SSE-triggered org refetch arrives
            if session.phase == DiscoveryPhase::Complete
                && matches!(
                    session.discovery_type,
                    DiscoveryType::Network { .. } | DiscoveryType::Unified { .. }
                )
                && let Ok(Some(network)) = self.network_service.get_by_id(&network_id).await
                && let Ok(Some(org)) = self
                    .organization_service
                    .get_by_id(&network.base.organization_id)
                    .await
                && org.not_onboarded(&OnboardingOperationDiscriminants::FirstDiscoveryCompleted)
            {
                let _ = self
                    .event_bus
                    .publish(Event::new(
                        OrgScope {
                            organization_id: org.id,
                        },
                        OnboardingOperation::FirstDiscoveryCompleted {
                            discovery_type: session.discovery_type.clone(),
                        },
                        AuthenticatedEntity::System,
                    ))
                    .await;
            }

            session.daemon_version = self.daemon_version(daemon_id).await;
            let _ = self.update_tx.send(session.clone());

            // Here rather than anywhere earlier because this is the one place a terminal payload
            // is processed exactly once — a redundant terminal update from an old daemon has
            // already returned above, so the counters cannot be double-incremented by a ServerPoll
            // re-delivery.
            self.publish_warning_events(
                session.network_id,
                session.session_id,
                session.daemon_id,
                &session.warnings,
            )
            .await;

            // Create historical discovery record
            let network_name = match self.network_service.get_by_id(&session.network_id).await {
                Ok(Some(network)) => network.base.name,
                _ => "Unknown Network".to_string(),
            };

            // Inherit the transient parent's name for a rescan — it was minted as
            // "Rescan of <host> (<ip>)", the only place host name and address are
            // both known, and the parent is deleted moments from now. Frozen at
            // scan time on purpose: a historical record should read as what was
            // true then, even if the host is later renamed or removed.
            let rescan_name = match (is_rescan, parent_discovery_id) {
                (true, Some(parent_id)) => self
                    .discovery_storage
                    .get_by_id(&parent_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|parent| parent.base.name),
                _ => None,
            };

            let historical_discovery = Discovery {
                id: Uuid::new_v4(),
                created_at: session.started_at.unwrap_or(Utc::now()),
                updated_at: Utc::now(),
                base: DiscoveryBase {
                    daemon_id: session.daemon_id,
                    network_id: session.network_id,
                    name: if let Some(name) = rescan_name {
                        name
                    } else if matches!(session.discovery_type, DiscoveryType::Unified { .. }) {
                        "Discovery".to_string()
                    } else {
                        format!("{} \u{2014} {}", session.discovery_type, network_name)
                    },
                    tags: Vec::new(),
                    discovery_type: session.discovery_type.clone(),
                    run_type: RunType::Historical {
                        results: Box::new(session.clone()),
                    },
                },
                scan_count: 0,
                force_full_scan: false,
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
                    tracing::error!(
                        "Failed to finalize successful scan for discovery {}: {}",
                        discovery_id,
                        e
                    );
                }
            }

            // Save to database
            if let Err(e) = self.discovery_storage.create(&historical_discovery).await {
                tracing::error!(
                    "Failed to create historical discovery record for session {}: {}",
                    session.session_id,
                    e
                );
            } else if let Some(scope) = EntityScope::from_ids(
                historical_discovery.id(),
                historical_discovery.clone().into(),
                self.get_network_id(&historical_discovery),
                self.get_organization_id(&historical_discovery),
            ) && let Err(e) = self
                .event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Created, AuthenticatedEntity::System)
                        .with_flags(EntityEventFlags::default()),
                )
                .await
            {
                tracing::error!(
                    session_id = %session_id,
                    error = %e,
                    "Failed to publish the historical discovery record's creation"
                );
            }

            // The terminal phase event, for subscribers that want to know a session ended.
            //
            // Neighbour resolution used to be driven from here and annotated the row above, which
            // is why the row is created first — published the other way round, the debounced
            // subscriber raced a row that did not exist yet and quietly found nothing to annotate.
            // That work now runs in `DaemonService::process_discovery_progress` *before* this
            // function is called, so its findings are in the row as written and there is no race
            // left to order around. The row still goes first, because the `Created` event above is
            // what carries the session's scanned set to the discovery-FK and digest subscribers.
            if let Err(e) = self
                .event_bus()
                .publish(session.into_discovery_event())
                .await
            {
                tracing::error!(
                    session_id = %session_id,
                    error = %e,
                    "Failed to publish the terminal discovery event"
                );
            }

            // A rescan's parent exists only to carry the targets to the daemon.
            // Delete it now that the historical row (which the user actually sees)
            // is persisted and its subscribers — including the scan-target subnet
            // reaper — have run. Discovery FKs on scanned entities point at the
            // historical row, not this one, and are ON DELETE SET NULL regardless.
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
                self.publish_promoted(daemon_id, network_id, promoted).await;
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
        network_id: Uuid,
        promoted: PromotedSession,
    ) {
        let mut started_payload = DiscoveryUpdatePayload::new(
            promoted.session_id,
            daemon_id,
            network_id,
            promoted.discovery_type,
            promoted.discovery_id,
        );
        started_payload.phase = DiscoveryPhase::Pending;

        if let Err(e) = self
            .event_bus()
            .publish(started_payload.into_discovery_event())
            .await
        {
            tracing::error!(
                session_id = %promoted.session_id,
                error = %e,
                "Failed to publish the promoted session"
            );
        }
    }

    /// The daemon's version as the server last recorded it, for stamping a finished session.
    pub(super) async fn daemon_version(&self, daemon_id: Uuid) -> Option<String> {
        let daemon_service = self.daemon_service.get()?;
        let daemon = daemon_service.get_by_id(&daemon_id).await.ok().flatten()?;
        daemon.base.version.map(|v| v.to_string())
    }

    pub async fn cancel_session(
        &self,
        session_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        // Get the session
        let session = match self.get_session(&session_id).await {
            Some(session) => session,
            None => {
                return Err(anyhow!("Session '{}' not found", session_id));
            }
        };

        let network_id = session.network_id;
        let daemon_id = session.daemon_id;
        let phase = session.phase;
        let discovery_id = self.lookup_discovery_id(&session_id).await;
        // Capture before `session.discovery_type` is moved into the update below.
        let is_rescan = session.discovery_type.rescan_target_host_id().is_some();

        let cancelled_update = DiscoveryUpdatePayload {
            session_id,
            network_id,
            daemon_id,
            phase: DiscoveryPhase::Cancelled,
            progress: 0,
            error: None,
            warnings: Vec::new(),
            started_at: session.started_at,
            finished_at: Some(Utc::now()),
            discovery_type: session.discovery_type,
            hosts_discovered: None,
            estimated_remaining_secs: None,
            discovery_id,
            // Preserve it: a cancelled rescan still needs its transient subnet
            // reaped and its digest suppressed.
            scanned: None,
            reason: Some(DiscoveryTerminalReason::UserCancelled),
            last_update_at: None,
            daemon_version: None,
        };

        // Handle based on current phase
        match phase {
            // Queued/Pending/AwaitingSnapshot sessions: just remove from queue.
            // AwaitingSnapshot is treated like Queued for cancellation — the
            // session has not yet been dispatched to the daemon, so cleanup
            // is just a queue removal.
            DiscoveryPhase::Queued | DiscoveryPhase::Pending | DiscoveryPhase::AwaitingSnapshot => {
                let mut sessions = self.sessions.write().await;
                let mut daemon_sessions = self.daemon_sessions.write().await;

                let was_pending = phase == DiscoveryPhase::Pending;

                // Remove from sessions map
                sessions.remove(&session_id);

                // Remove from daemon queue
                if let Some(queue) = daemon_sessions.get_mut(&daemon_id) {
                    queue.retain(|id| *id != session_id);

                    // If we removed the Pending session, promote next Queued → Pending
                    if was_pending
                        && let Some(next_session) =
                            queue.first().and_then(|next_id| sessions.get_mut(next_id))
                    {
                        next_session.phase = DiscoveryPhase::Pending;
                        self.session_last_updated
                            .write()
                            .await
                            .insert(next_session.session_id, Utc::now());
                    }
                }

                // Remove from discovery_sessions map
                self.discovery_sessions
                    .write()
                    .await
                    .retain(|_, sid| *sid != session_id);

                drop(sessions);
                drop(daemon_sessions);

                // Broadcast cancellation update so frontend knows
                let _ = self.update_tx.send(cancelled_update);

                tracing::info!("Cancelled {} session {} from queue", phase, session_id);

                // A rescan cancelled before dispatch never reaches the
                // daemon-reported terminal finalization that deletes its transient
                // parent, so the AdHoc Rescan row would linger in the UI with only a
                // delete action. Reap it here, matching what completion does.
                if is_rescan
                    && let Some(discovery_id) = discovery_id
                    && let Err(e) = self.discovery_storage.delete(&discovery_id).await
                {
                    tracing::warn!(
                        discovery_id = %discovery_id,
                        error = ?e,
                        "Failed to delete cancelled transient rescan; the startup sweeper will retry"
                    );
                }
                Ok(())
            }

            // Starting phase: wait briefly then retry
            DiscoveryPhase::Starting => Err(anyhow!(
                "Session is starting on daemon. Please try again in a moment."
            )),

            // Active phases: send cancellation to daemon
            // We do BOTH actions to support both daemon modes:
            // 1. Publish DiscoveryCancelled event - DaemonService subscriber handles ServerPoll mode
            // 2. Set cancellation flag - DaemonPoll mode checks on next poll via request_work
            DiscoveryPhase::Started | DiscoveryPhase::Scanning => {
                self.event_bus()
                    .publish(cancelled_update.into_discovery_event_with_auth(authentication))
                    .await?;

                // Set cancellation flag for DaemonPoll mode (checked on next poll)
                self.daemon_pull_cancellations
                    .write()
                    .await
                    .insert(daemon_id, (true, session_id));

                tracing::info!(
                    daemon_id = %daemon_id,
                    session_id = %session_id,
                    "Discovery cancellation requested",
                );

                Ok(())
            }

            // Terminal phases: already done
            DiscoveryPhase::Complete | DiscoveryPhase::Failed | DiscoveryPhase::Cancelled => {
                tracing::info!(
                    "Session {} is already in terminal state: {}, nothing to cancel",
                    session_id,
                    phase
                );
                Ok(())
            }
        }
    }
}
