//! Binds what a daemon reports to the daemon that reported it.
//!
//! A report is input, on both daemon modes. A DaemonPoll daemon posts it with a key; a ServerPoll
//! daemon answers a poll from an address its owner chose, and processing then runs as `System`.
//! Every report passes through here first, so the identity and ids it carries are settled once
//! rather than at each reader.
use super::*;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::credentials::r#impl::base::Credential;

impl DaemonService {
    /// Refuse a report that isn't `daemon_id`'s on `site_id`, and drop the ids in it that name
    /// anything else.
    pub(super) async fn scope_discovery_report(
        &self,
        daemon_id: Uuid,
        site_id: Uuid,
        update: &mut DiscoveryUpdatePayload,
    ) -> Result<(), ApiError> {
        if update.site_id != site_id {
            return Err(ApiError::daemon_site_mismatch());
        }
        if update.daemon_id != daemon_id {
            return Err(ApiError::daemon_identity_mismatch());
        }
        // A shared key takes the daemon's identity from a header, so it proves only the site.
        let daemon = self.get_by_id(&daemon_id).await?;
        if daemon.is_none_or(|d| d.base.site_id != site_id) {
            return Err(ApiError::daemon_site_mismatch());
        }
        if let Some(session) = self.discovery_service.get_session(&update.session_id).await
            && session.site_id != site_id
        {
            return Err(ApiError::daemon_site_mismatch());
        }

        // The server maps a session to its discovery. A daemon has no say in it: the finished
        // session updates, and for a rescan deletes, the row this names.
        update.discovery_id = None;

        // Raised by the server later in processing, from stored assignments.
        update
            .warnings
            .retain(|w| !matches!(w, DiscoveryWarning::IntegrationWithoutCredential { .. }));

        if let DiscoveryType::Network { subnet_ids, .. } | DiscoveryType::Unified { subnet_ids, .. } =
            &mut update.discovery_type
            && let Some(subnet_ids) = subnet_ids
        {
            self.subnet_service
                .retain_on_site(subnet_ids, site_id)
                .await?;
        }

        self.drop_foreign_credential_results(site_id, update)
            .await?;

        // A failed lookup refuses the report: the list decides which rows are stamped.
        if let Some(scanned) = update.scanned.as_mut() {
            let Some(host_service) = self.host_service.get() else {
                return Err(ApiError::internal_error("HostService not initialized"));
            };
            host_service
                .retain_scanned_on_site(scanned, site_id)
                .await?;
        }

        Ok(())
    }

    /// Drop results for another organization's credentials. A credential deleted during the run
    /// keeps its result: that row is simply gone, and the run's record still says what it did.
    async fn drop_foreign_credential_results(
        &self,
        site_id: Uuid,
        update: &mut DiscoveryUpdatePayload,
    ) -> Result<(), ApiError> {
        if update.credential_results.is_empty() {
            return Ok(());
        }
        let Some(site) = self.site_service.get_by_id(&site_id).await? else {
            return Err(ApiError::daemon_site_mismatch());
        };
        let ids: Vec<Uuid> = update
            .credential_results
            .iter()
            .map(|result| result.credential_id)
            .collect();
        let foreign: std::collections::HashSet<Uuid> = self
            .credential_service
            .get_all(StorableFilter::<Credential>::new_from_entity_ids(&ids))
            .await?
            .into_iter()
            .filter(|credential| credential.base.organization_id != site.base.organization_id)
            .map(|credential| credential.id)
            .collect();
        update
            .credential_results
            .retain(|result| !foreign.contains(&result.credential_id));
        Ok(())
    }
}
