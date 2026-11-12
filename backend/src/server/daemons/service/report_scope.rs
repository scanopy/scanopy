//! Binds what a daemon reports to the daemon that reported it.
//!
//! A report is input, on both daemon modes. A DaemonPoll daemon posts it with a key; a ServerPoll
//! daemon answers a poll from an address its owner chose, and processing then runs as `System`.
//! Every report passes through here first, so the identity and ids it carries are settled once
//! rather than at each reader.
use super::*;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::credentials::r#impl::base::Credential;

/// The subnet ids a report's warnings name.
fn warning_subnet_ids(warnings: &mut [DiscoveryWarning]) -> Vec<Uuid> {
    warnings
        .iter_mut()
        .filter_map(|warning| warning.entity_id_slots().subnet_id.and_then(|slot| *slot))
        .collect()
}

/// Clear every warning's subnet id that is not in `keep`.
fn clear_warning_subnet_ids_except(
    warnings: &mut [DiscoveryWarning],
    keep: &std::collections::HashSet<Uuid>,
) {
    for warning in warnings {
        if let Some(slot) = warning.entity_id_slots().subnet_id
            && slot.is_some_and(|id| !keep.contains(&id))
        {
            *slot = None;
        }
    }
}

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

        // A warning's subnet id is stored in a foreign-key column, and the whole run's warnings
        // are written in one insert. An id for another site's subnet, or one deleted during the
        // run, is cleared; the warning keeps its CIDR and still reads.
        let mut named = warning_subnet_ids(&mut update.warnings);
        if !named.is_empty() {
            self.subnet_service
                .retain_on_site(&mut named, site_id)
                .await?;
            clear_warning_subnet_ids_except(&mut update.warnings, &named.into_iter().collect());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn container_warning(subnet_id: Uuid) -> DiscoveryWarning {
        DiscoveryWarning::ContainerSubnetScanned {
            subnet_id: Some(subnet_id),
            cidr: "172.17.0.0/16".to_string(),
        }
    }

    /// A daemon names the subnet in the warning it sends. One on the daemon's site keeps its id;
    /// one that is not loses the id and keeps the rest.
    #[test]
    fn a_warning_keeps_only_a_subnet_id_on_the_daemons_site() {
        let (ours, theirs) = (Uuid::new_v4(), Uuid::new_v4());
        let mut warnings = vec![
            container_warning(ours),
            container_warning(theirs),
            DiscoveryWarning::ArpSweepUnavailable,
        ];

        assert_eq!(warning_subnet_ids(&mut warnings), vec![ours, theirs]);

        clear_warning_subnet_ids_except(&mut warnings, &[ours].into_iter().collect());

        assert_eq!(
            warnings,
            vec![
                container_warning(ours),
                DiscoveryWarning::ContainerSubnetScanned {
                    subnet_id: None,
                    cidr: "172.17.0.0/16".to_string(),
                },
                DiscoveryWarning::ArpSweepUnavailable,
            ]
        );
    }
}
