//! Daemon provisioning: mint a 1:1 api key and create (or re-bind) the daemon record.
use super::*;

impl DaemonService {
    /// Provision a daemon: mint an API key bound to it 1:1 and create the daemon record, or
    /// re-bind an existing record to a fresh key.
    ///
    /// Returns the record and the key's plaintext — the only moment it is available.
    ///
    /// Tenant authorization and the re-provision safety guard belong to the caller;
    /// `existing_daemon` is the already-loaded (and already access-checked) re-provision
    /// target, or `None` to create a new record.
    pub async fn provision(
        &self,
        request: &ProvisionDaemonRequest,
        existing_daemon: Option<Daemon>,
        auth: AuthenticatedEntity,
    ) -> Result<(Daemon, String), ApiError> {
        let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

        // Identity is server-owned. On the re-provision path it comes from the record — the
        // request's name/network/mode/url are ignored, since those are immutable post-provision
        // and the record already holds whatever the daemon last reported.
        let (network_id, name, mode, reachable_url) = match &existing_daemon {
            Some(daemon) => (
                daemon.base.network_id,
                daemon.base.name.clone(),
                daemon.base.mode,
                daemon.base.url.clone(),
            ),
            // A fresh provision must say what it is creating; only the re-provision path can
            // inherit these from a record.
            None => (
                request.network_id.ok_or_else(|| {
                    ApiError::bad_request("network_id is required when provisioning a new daemon")
                })?,
                request.name.clone().ok_or_else(|| {
                    ApiError::bad_request("name is required when provisioning a new daemon")
                })?,
                request.mode,
                request.url.clone().unwrap_or_default(),
            ),
        };
        let is_server_poll = mode == DaemonMode::ServerPoll;

        let org_id = self
            .network_service
            .get_by_id(&network_id)
            .await?
            .ok_or_else(|| ApiError::entity_not_found::<Network>(network_id))?
            .base
            .organization_id;

        if existing_daemon.is_none() {
            // A ServerPoll daemon never dials out, so the server can only reach it at a URL
            // supplied now. DaemonPoll dials the server, so its url is unused.
            if is_server_poll && reachable_url.trim().is_empty() {
                return Err(ApiError::bad_request(
                    "A reachable url is required to provision a ServerPoll daemon",
                ));
            }

            // Check daemon limit for unverified orgs (allows 1st daemon). Re-provisioning an
            // existing record adds no daemons, so it is exempt.
            self.check_unverified_daemon_limit(org_id).await?;
        }

        // ---- Mint the key and bind it 1:1 ------------------------------------------------

        // The partial-UNIQUE on api_keys.daemon_id makes the binding exclusive, so any key
        // already bound to this daemon has to go before the new one can take its place. Only a
        // *bound* key is removed: a legacy daemon's network-shared key has daemon_id = NULL, is
        // shared with every other legacy daemon on the network, and must survive untouched —
        // that is exactly what keeps the daemon running until it is reconfigured.
        if let Some(old_key_id) = existing_daemon.as_ref().and_then(|d| d.base.api_key_id) {
            let old_key = self.daemon_api_key_service.get_by_id(&old_key_id).await?;
            self.daemon_api_key_service
                .delete(&old_key_id, auth.clone())
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, api_key_id = %old_key_id, "Failed to remove superseded daemon api key");
                    ApiError::internal_error(&format!("Failed to remove superseded api key: {}", e))
                })?;
            // Evict the auth resolution cache so the deleted key stops resolving immediately
            // rather than waiting out the TTL.
            if let Some(old_key) = old_key {
                self.daemon_api_key_service
                    .invalidate_resolution(&old_key.base.key)
                    .await;
            }
        }

        // Generate API key (plaintext + hash)
        let (plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::Daemon);

        // Create API key record with plaintext stored (for ServerPoll mode)
        let api_key = DaemonApiKey::new(DaemonApiKeyBase {
            key: hashed,
            name: format!("{} API Key", name),
            last_used: None,
            expires_at: None,
            network_id,
            is_enabled: true,
            tags: Vec::new(),
            // When the daemon already exists the binding can be set outright. On the create path
            // it is completed below, once the daemon record exists (circular: the daemon needs
            // api_key_id, the key needs daemon_id).
            daemon_id: existing_daemon.as_ref().map(|d| d.id),
            // Only ServerPoll needs the server to hold the plaintext (to present it when
            // it dials the daemon). A DaemonPoll daemon carries its own key and dials out.
            plaintext: is_server_poll.then(|| SecretString::from(plaintext.clone())),
        });

        let created_api_key = self
            .daemon_api_key_service
            .create(api_key, auth.clone())
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create API key for provisioned daemon");
                ApiError::internal_error(&format!("Failed to create API key: {}", e))
            })?;

        // ---- Create or update the daemon record -----------------------------------------
        let created_daemon = match existing_daemon {
            // Re-provision: point the existing record at its new key. Everything else — host,
            // discovery jobs, history — is left in place. Setting api_key_id also promotes the
            // daemon to "provisioned", after which the server-side name and mode become
            // authoritative instead of the daemon's self-reported ones; those already match,
            // since the record holds what the daemon last reported.
            Some(mut daemon) => {
                daemon.base.api_key_id = Some(created_api_key.id);
                // Provisioning is what establishes ownership, so the member doing it becomes the
                // maintainer — the same rule the create path applies.
                daemon.base.user_id = user_id;
                self.update(&mut daemon, auth.clone()).await.map_err(|e| {
                    tracing::error!(error = %e, daemon_id = %daemon.id, "Failed to bind api key to existing daemon");
                    ApiError::internal_error(&format!("Failed to bind api key to daemon: {}", e))
                })?;
                daemon
            }
            None => {
                let created_daemon = self
                    .create_provisioned_daemon(
                        request,
                        network_id,
                        name,
                        mode,
                        reachable_url,
                        user_id,
                        created_api_key.id,
                        org_id,
                        auth.clone(),
                    )
                    .await?;

                // Complete the 1:1 binding now that the daemon exists: point the api key at its
                // daemon. The service update bypasses preserve_immutable_fields (that guard runs
                // only in the HTTP handler).
                let mut api_key_to_bind = created_api_key.clone();
                api_key_to_bind.base.daemon_id = Some(created_daemon.id);
                if let Err(e) = self
                    .daemon_api_key_service
                    .update(&mut api_key_to_bind, auth.clone())
                    .await
                {
                    tracing::error!(error = %e, daemon_id = %created_daemon.id, "Failed to bind api key to provisioned daemon");
                    return Err(ApiError::internal_error(&format!(
                        "Failed to bind api key to daemon: {}",
                        e
                    )));
                }
                created_daemon
            }
        };

        tracing::info!(
            daemon_id = %created_daemon.id,
            network_id = %network_id,
            user_id = %user_id,
            mode = ?created_daemon.base.mode,
            reprovisioned = request.daemon_id.is_some(),
            "Daemon provisioned"
        );

        Ok((created_daemon, plaintext))
    }

    /// Create the host + daemon records and seed the daemon's first discovery. Only used on the
    /// create path — re-provisioning reuses the existing records.
    #[allow(clippy::too_many_arguments)]
    async fn create_provisioned_daemon(
        &self,
        request: &ProvisionDaemonRequest,
        network_id: Uuid,
        name: String,
        mode: DaemonMode,
        reachable_url: String,
        user_id: Uuid,
        api_key_id: Uuid,
        org_id: Uuid,
        auth: AuthenticatedEntity,
    ) -> Result<Daemon, ApiError> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| ApiError::internal_error("HostService not initialized"))?;

        // Create host record for the daemon
        let mut host = Host::new(HostBase {
            // Placeholder identity: the name chosen at provision time, applied below as an
            // unattributed name. That ranks as a guess, so the hostname the daemon later reports
            // for itself titles the host instead.
            name: HostName::unnamed(),
            network_id,
            hostname: None,
            description: None,
            source: EntitySource::System,
            virtualization_metadata: None,
            virtualization_service_id: None,
            hidden: false,
            tags: Vec::new(),
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            firmware_revision: None,
            software_revision: None,
            credential_assignments: vec![],
        });
        host.base.apply_name(HostName::unattributed(name.clone()));

        let created_host = host_service.create(host, auth.clone()).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create host for provisioned daemon");
            ApiError::internal_error(&format!("Failed to create host: {}", e))
        })?;

        // Seed the daemon host's loopback so a daemon-host socket/proxy credential is probed on
        // the very first scan (the credential mapping is snapshotted before the daemon self-reports).
        if let Err(e) = host_service
            .seed_loopback(created_host.id, network_id, auth.clone())
            .await
        {
            tracing::warn!(host_id = %created_host.id, error = %e, "Failed to seed daemon host loopback");
        }

        // Create daemon record with the linked API key.
        // last_seen is None until first successful contact from poller.
        // version stays NULL until the daemon actually reports it — writing the
        // SERVER_VERSION optimistically here made a provisioned-but-never-installed
        // daemon read as "Current" forever and poisoned any installed-base view.
        let daemon = Daemon::new(DaemonBase {
            host_id: created_host.id,
            network_id,
            url: reachable_url,
            last_seen: None,
            mode,
            name,
            tags: Vec::new(),
            version: None,
            user_id,
            api_key_id: Some(api_key_id),
            is_unreachable: false,
            standby: false,
            standby_cleared_at: None,
        });

        let created_daemon = self.create(daemon, auth.clone()).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create provisioned daemon");
            ApiError::internal_error(&format!("Failed to create daemon: {}", e))
        })?;

        // Seed the daemon's first discovery: persist the credential refs on the daemon's
        // Discovery row so they are PROBED on that run and only assigned to a host once the
        // probe succeeds (including refs targeted at the daemon host / 127.0.0.1). We do NOT
        // write the host_credentials junction directly here — a seeded credential must earn
        // its assignment. create_default_discovery_jobs is idempotent, so the later
        // registration / first-contact paths won't duplicate this.
        let is_free_plan = self
            .organization_service
            .get_by_id(&org_id)
            .await
            .ok()
            .flatten()
            .and_then(|o| o.base.plan)
            .map(|p| p.is_free())
            .unwrap_or(true);

        if let Err(e) = self
            .create_default_discovery_jobs(
                created_daemon.id,
                network_id,
                created_host.id,
                is_free_plan,
                &request.seed_credential_refs,
            )
            .await
        {
            tracing::warn!(daemon_id = %created_daemon.id, error = ?e, "Failed to create default discovery jobs at provision");
        }

        Ok(created_daemon)
    }
}
