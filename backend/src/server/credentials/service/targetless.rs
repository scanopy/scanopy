//! Credentials no scan will send to a daemon.
//!
//! A credential reaches a daemon through a site assignment, a host assignment, or a discovery's
//! integration targets (see [`CredentialService::build_all_credential_mappings`]). One with none of
//! the three is stored and never used, and since the daemon never receives it, nothing it does
//! reports it either. Raised as `CredentialWithoutTarget` at the end of a run, and re-checked for
//! the home nudge so it clears as soon as the credential gets a target.

use super::*;

/// The credentials holding none of the three channels: no site or host junction row (`assigned`)
/// and no discovery target (`targeted`), ordered by name.
///
/// Seeded defaults are left out. The Docker and Podman sockets are offered to every new daemon as
/// one-shot targets and keep nothing where no runtime answered, which is routine, not a mistake. A
/// login is left out too: it runs nothing, and reaches a daemon inside the credentials that use it.
pub fn find_targetless(
    credentials: Vec<Credential>,
    assigned: &HashSet<Uuid>,
    targeted: &HashSet<Uuid>,
) -> Vec<Credential> {
    let mut targetless: Vec<Credential> = credentials
        .into_iter()
        .filter(|c| {
            !c.is_default
                && c.base.credential_type.discriminant().role() == CredentialRole::Dispatched
                && !assigned.contains(&c.id)
                && !targeted.contains(&c.id)
        })
        .collect();
    targetless.sort_by(|a, b| a.base.name.cmp(&b.base.name));
    targetless
}

impl Credential {
    pub fn targetless_warning(&self) -> DiscoveryWarning {
        DiscoveryWarning::CredentialWithoutTarget {
            integration: self.base.credential_type.discriminant().wire_discriminant(),
            credential_id: Some(self.id),
        }
    }
}

impl CredentialService {
    /// The organization's credentials with no site or host assignment and no id in `targeted`,
    /// the credentials its discoveries still target.
    pub async fn targetless_credentials(
        &self,
        organization_id: &Uuid,
        targeted: &HashSet<Uuid>,
    ) -> Result<Vec<Credential>, Error> {
        let credentials = self
            .get_all(StorableFilter::<Credential>::new_from_org_id(
                organization_id,
            ))
            .await?;
        if credentials.is_empty() {
            return Ok(Vec::new());
        }

        let ids: Vec<Uuid> = credentials.iter().map(|c| c.id).collect();
        let site_rows = self
            .site_credential_storage
            .inner()
            .get_all_ordered(
                StorableFilter::<SiteCredential>::new_from_uuids_column("credential_id", &ids),
                "credential_id ASC",
            )
            .await?;
        let host_rows = self
            .host_credential_storage
            .inner()
            .get_all_ordered(
                StorableFilter::<HostCredential>::new_from_uuids_column("credential_id", &ids),
                "credential_id ASC",
            )
            .await?;
        let assigned: HashSet<Uuid> = site_rows
            .iter()
            .map(|r| r.credential_id)
            .chain(host_rows.iter().map(|r| r.credential_id))
            .collect();

        Ok(find_targetless(credentials, &assigned, targeted))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::base::CredentialBase;

    fn credential(name: &str) -> Credential {
        let mut c = Credential::new(CredentialBase {
            name: name.to_string(),
            ..Default::default()
        });
        c.is_default = false;
        c
    }

    #[test]
    fn only_a_credential_with_no_channel_is_targetless() {
        let site = credential("site");
        let host = credential("host");
        let targeted = credential("targeted");
        let unused = credential("unused");
        let assigned = HashSet::from([site.id, host.id]);
        let pending = HashSet::from([targeted.id]);

        let found = find_targetless(
            vec![site, host, targeted, unused.clone()],
            &assigned,
            &pending,
        );
        assert_eq!(found.iter().map(|c| c.id).collect::<Vec<_>>(), [unused.id]);
    }

    /// The seeded sockets keep no target on a daemon host without a runtime, and say nothing.
    #[test]
    fn seeded_defaults_never_count() {
        let mut socket = credential("Docker socket");
        socket.is_default = true;
        assert!(find_targetless(vec![socket], &HashSet::new(), &HashSet::new()).is_empty());
    }
}
