//! The `site_credentials` and `host_credentials` junctions: a site's or host's credentials, and a
//! credential's sites or hosts. Each end replaces only its own rows.
use crate::server::shared::relations as R;

use super::*;

impl CredentialService {
    /// Get credential IDs for a site from the junction table.
    pub async fn get_credential_ids_for_site(&self, site_id: &Uuid) -> Result<Vec<Uuid>, Error> {
        let filter = StorableFilter::<SiteCredential>::new_from_uuid_column("site_id", site_id);
        let records = self
            .site_credential_storage
            .get_all_ordered(filter, "credential_id ASC")
            .await?;
        Ok(records.into_iter().map(|r| r.credential_id).collect())
    }

    /// Get credential IDs for multiple sites (batch).
    pub async fn get_credential_ids_for_sites(
        &self,
        site_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<Uuid>>, Error> {
        if site_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let filter = StorableFilter::<SiteCredential>::new_from_uuids_column("site_id", site_ids);
        let records = self
            .site_credential_storage
            .get_all_ordered(filter, "site_id ASC")
            .await?;
        let mut map: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
        for record in records {
            map.entry(record.site_id)
                .or_default()
                .push(record.credential_id);
        }
        Ok(map)
    }

    /// Get credential assignments for a host from the junction table.
    pub async fn get_credential_assignments_for_host(
        &self,
        host_id: &Uuid,
    ) -> Result<Vec<CredentialAssignment>, Error> {
        let filter = StorableFilter::<HostCredential>::new_from_uuid_column("host_id", host_id);
        let records = self
            .host_credential_storage
            .get_all_ordered(filter, "credential_id ASC")
            .await?;
        Ok(records
            .into_iter()
            .map(|r| CredentialAssignment {
                credential_id: r.credential_id,
                ip_address_ids: r.ip_address_ids,
            })
            .collect())
    }

    /// Get credential assignments for multiple hosts (batch).
    pub async fn get_credential_assignments_for_hosts(
        &self,
        host_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<CredentialAssignment>>, Error> {
        if host_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let filter = StorableFilter::<HostCredential>::new_from_uuids_column("host_id", host_ids);
        let records = self
            .host_credential_storage
            .get_all_ordered(filter, "host_id ASC")
            .await?;
        let mut map: std::collections::HashMap<Uuid, Vec<CredentialAssignment>> =
            std::collections::HashMap::new();
        for record in records {
            map.entry(record.host_id)
                .or_default()
                .push(CredentialAssignment {
                    credential_id: record.credential_id,
                    ip_address_ids: record.ip_address_ids,
                });
        }
        Ok(map)
    }

    /// Replace all credentials for a site (atomic).
    pub async fn set_site_credentials(
        &self,
        site_id: &Uuid,
        credential_ids: &[Uuid],
    ) -> Result<(), Error> {
        self.refuse_login_assignment(credential_ids).await?;
        let rows: Vec<SiteCredential> = credential_ids
            .iter()
            .map(|credential_id| SiteCredential {
                site_id: *site_id,
                credential_id: *credential_id,
            })
            .collect();
        self.site_credential_storage
            .replace_for(&R::SITE_CREDENTIAL_SITE.at(*site_id), &rows)
            .await?;
        Ok(())
    }

    /// Bulk-insert many site↔credential assignments at once, skipping the
    /// per-site lock/delete. For seed paths on a freshly reset org. Each
    /// pair is `(site_id, credential_id)`.
    pub async fn create_site_credentials(&self, pairs: &[(Uuid, Uuid)]) -> Result<(), Error> {
        let records: Vec<SiteCredential> = pairs
            .iter()
            .map(|&(site_id, credential_id)| SiteCredential {
                site_id,
                credential_id,
            })
            .collect();
        if records.is_empty() {
            return Ok(());
        }
        self.site_credential_storage.create_many(&records).await?;
        Ok(())
    }

    /// Merge `incoming` credentials into a site's junction (additive — never prunes).
    ///
    /// The site-side counterpart of [`Self::merge_host_credentials`], for promoting a
    /// discovery's one-shot `IntegrationTarget::Site` targets into the durable site-wide
    /// channel when the scan completes. Replacing would delete credentials assigned from the
    /// sites page or the credential modal, which this path knows nothing about.
    pub async fn merge_site_credentials(
        &self,
        site_id: &Uuid,
        incoming: &[Uuid],
    ) -> Result<(), Error> {
        let existing = self.get_credential_ids_for_site(site_id).await?;
        let mut merged = existing;
        for credential_id in incoming {
            if !merged.contains(credential_id) {
                merged.push(*credential_id);
            }
        }
        self.set_site_credentials(site_id, &merged).await
    }

    /// Replace all credential assignments for a host (atomic).
    pub async fn set_host_credentials(
        &self,
        host_id: &Uuid,
        assignments: &[CredentialAssignment],
    ) -> Result<(), Error> {
        let ids: Vec<Uuid> = assignments.iter().map(|a| a.credential_id).collect();
        self.refuse_login_assignment(&ids).await?;
        let rows: Vec<HostCredential> = assignments
            .iter()
            .map(|assignment| HostCredential {
                host_id: *host_id,
                credential_id: assignment.credential_id,
                ip_address_ids: assignment.ip_address_ids.clone(),
            })
            .collect();
        self.host_credential_storage
            .replace_for(&R::HOST_CREDENTIAL_HOST.at(*host_id), &rows)
            .await?;
        Ok(())
    }

    /// Merge `incoming` credential assignments into a host's junction (additive — never prunes).
    ///
    /// Discovery self-reports only the credentials that probed successfully on a host. Using the
    /// replace path (`set_host_credentials`) there would delete user/init-assigned daemon-host
    /// credentials that didn't probe this scan (e.g. a Docker socket on a Podman-only host),
    /// regressing the junction-as-source-of-truth model. Merge keeps existing assignments and
    /// widens each one by what `incoming` reports for the same credential (see
    /// [`merge_assignment_scope`]), so discovery only adds. Explicit user edits still go through
    /// the replace path.
    pub async fn merge_host_credentials(
        &self,
        host_id: &Uuid,
        incoming: &[CredentialAssignment],
    ) -> Result<(), Error> {
        let mut by_host = self
            .get_credential_assignments_for_hosts(&[*host_id])
            .await?;
        let mut merged = by_host.remove(host_id).unwrap_or_default();

        for assignment in incoming {
            match merged
                .iter_mut()
                .find(|a| a.credential_id == assignment.credential_id)
            {
                Some(existing) => merge_assignment_scope(existing, assignment),
                None => merged.push(assignment.clone()),
            }
        }

        self.set_host_credentials(host_id, &merged).await
    }

    /// Get the site IDs a credential is assigned to (reverse lookup).
    pub async fn get_site_ids_for_credential(
        &self,
        credential_id: &Uuid,
    ) -> Result<Vec<Uuid>, Error> {
        let filter =
            StorableFilter::<SiteCredential>::new_from_uuid_column("credential_id", credential_id);
        let records = self
            .site_credential_storage
            .get_all_ordered(filter, "site_id ASC")
            .await?;
        Ok(records.into_iter().map(|r| r.site_id).collect())
    }

    /// Get the site IDs for multiple credentials (batch, reverse lookup).
    pub async fn get_site_ids_for_credentials(
        &self,
        credential_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<Uuid>>, Error> {
        if credential_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let filter = StorableFilter::<SiteCredential>::new_from_uuids_column(
            "credential_id",
            credential_ids,
        );
        let records = self
            .site_credential_storage
            .get_all_ordered(filter, "credential_id ASC")
            .await?;
        let mut map: std::collections::HashMap<Uuid, Vec<Uuid>> = std::collections::HashMap::new();
        for record in records {
            map.entry(record.credential_id)
                .or_default()
                .push(record.site_id);
        }
        Ok(map)
    }

    /// Get the host assignments for a credential (reverse lookup).
    pub async fn get_host_assignments_for_credential(
        &self,
        credential_id: &Uuid,
    ) -> Result<Vec<CredentialHostAssignment>, Error> {
        let filter =
            StorableFilter::<HostCredential>::new_from_uuid_column("credential_id", credential_id);
        let records = self
            .host_credential_storage
            .get_all_ordered(filter, "host_id ASC")
            .await?;
        Ok(records
            .into_iter()
            .map(|r| CredentialHostAssignment {
                host_id: r.host_id,
                ip_address_ids: r.ip_address_ids,
            })
            .collect())
    }

    /// Get the host assignments for multiple credentials (batch, reverse lookup).
    pub async fn get_host_assignments_for_credentials(
        &self,
        credential_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<CredentialHostAssignment>>, Error> {
        if credential_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let filter = StorableFilter::<HostCredential>::new_from_uuids_column(
            "credential_id",
            credential_ids,
        );
        let records = self
            .host_credential_storage
            .get_all_ordered(filter, "credential_id ASC")
            .await?;
        let mut map: std::collections::HashMap<Uuid, Vec<CredentialHostAssignment>> =
            std::collections::HashMap::new();
        for record in records {
            map.entry(record.credential_id)
                .or_default()
                .push(CredentialHostAssignment {
                    host_id: record.host_id,
                    ip_address_ids: record.ip_address_ids,
                });
        }
        Ok(map)
    }

    /// Replace the full set of sites a credential is assigned to (atomic).
    pub async fn set_credential_sites(
        &self,
        credential_id: &Uuid,
        site_ids: &[Uuid],
    ) -> Result<(), Error> {
        let rows: Vec<SiteCredential> = site_ids
            .iter()
            .map(|site_id| SiteCredential {
                site_id: *site_id,
                credential_id: *credential_id,
            })
            .collect();
        // Only this credential's rows, so other credentials on the same sites stay.
        self.site_credential_storage
            .replace_for(&R::SITE_CREDENTIAL_CREDENTIAL.at(*credential_id), &rows)
            .await?;
        Ok(())
    }

    /// Replace the full set of host assignments for a credential (atomic).
    pub async fn set_credential_host_assignments(
        &self,
        credential_id: &Uuid,
        assignments: &[CredentialHostAssignment],
    ) -> Result<(), Error> {
        let rows: Vec<HostCredential> = assignments
            .iter()
            .map(|assignment| HostCredential {
                host_id: assignment.host_id,
                credential_id: *credential_id,
                ip_address_ids: assignment.ip_address_ids.clone(),
            })
            .collect();
        // Only this credential's rows, so other credentials on the same hosts stay.
        self.host_credential_storage
            .replace_for(&R::HOST_CREDENTIAL_CREDENTIAL.at(*credential_id), &rows)
            .await?;
        Ok(())
    }
}

/// Widen `existing` by what discovery reports for the same credential.
///
/// A scan submits one host several times (the daemon-host phase, then each address the network
/// phase reaches), and each submission reports the credential at the one address it worked at.
/// Replacing the scope with each report kept only the last address, and turned a whole-host
/// assignment into a single address. Either side covering the whole host keeps the whole host;
/// two address lists are unioned.
pub(crate) fn merge_assignment_scope(
    existing: &mut CredentialAssignment,
    incoming: &CredentialAssignment,
) {
    match (&mut existing.ip_address_ids, &incoming.ip_address_ids) {
        (None, _) => {}
        (Some(_), None) => existing.ip_address_ids = None,
        (Some(ids), Some(more)) => {
            for id in more {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assignment(credential_id: Uuid, ids: Option<Vec<Uuid>>) -> CredentialAssignment {
        CredentialAssignment {
            credential_id,
            ip_address_ids: ids,
        }
    }

    #[test]
    fn a_reported_address_does_not_narrow_a_whole_host_assignment() {
        let cred = Uuid::new_v4();
        let mut existing = assignment(cred, None);
        merge_assignment_scope(&mut existing, &assignment(cred, Some(vec![Uuid::new_v4()])));
        assert_eq!(existing.ip_address_ids, None);
    }

    #[test]
    fn addresses_reported_by_separate_submissions_accumulate() {
        let cred = Uuid::new_v4();
        let (lan, bridge) = (Uuid::new_v4(), Uuid::new_v4());
        let mut existing = assignment(cred, Some(vec![lan]));
        merge_assignment_scope(&mut existing, &assignment(cred, Some(vec![bridge, lan])));
        assert_eq!(existing.ip_address_ids, Some(vec![lan, bridge]));
    }

    #[test]
    fn a_whole_host_report_widens_a_scoped_assignment() {
        let cred = Uuid::new_v4();
        let mut existing = assignment(cred, Some(vec![Uuid::new_v4()]));
        merge_assignment_scope(&mut existing, &assignment(cred, None));
        assert_eq!(existing.ip_address_ids, None);
    }
}
