//! Which hosts run a service an integration reads, with no credential of that integration
//! working on them. Raised as `IntegrationWithoutCredential` at the end of a run, and re-checked
//! for the home nudge so it clears as soon as a covering credential is saved.
//!
//! A host is covered by a credential pinned to it: a host assignment (whole host or
//! address-scoped), or one of the run's `Hosts` / `DaemonHost` targets. A pinned credential that
//! failed still covers: the daemon reports that failure as a credential warning, and one fact gets
//! one warning.
//!
//! A site-wide credential is different. It is a guess the scan tries on every host of the site,
//! and its failures are silent, so it covers a host only until a scan has tried it. Where it
//! worked, the scan pinned it to the host as a host assignment, which covers from then on. Where
//! every one failed, the host stays uncovered and the prompt keeps naming it. That is what lets a
//! mixed SNMP estate converge: add the community most devices use, scan, and the prompt lists the
//! devices it did not open.

use super::*;
use crate::server::credentials::r#impl::types::CredentialIntegration;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::types::metadata::HasId;

/// When each site's credentials were last tried: the start of the scan that tried them. A site
/// with no entry has not been scanned since, so its site-wide credentials are untried.
pub type SiteTriedAt = HashMap<Uuid, DateTime<Utc>>;

/// An integration's service, found on one host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FoundIntegration {
    pub host_id: Uuid,
    pub site_id: Uuid,
    pub integration: CredentialIntegration,
}

impl FoundIntegration {
    /// The integration this service is the service of, when that integration prompts for a
    /// credential where it is found.
    pub fn from_service(service: &Service) -> Option<Self> {
        CredentialIntegration::for_service(&*service.base.service_definition)
            .filter(CredentialIntegration::prompts_when_found)
            .map(|integration| Self {
                host_id: service.base.host_id,
                site_id: service.base.site_id,
                integration,
            })
    }

    pub fn into_warning(self) -> DiscoveryWarning {
        DiscoveryWarning::IntegrationWithoutCredential {
            host_id: Some(self.host_id),
            integration: self.integration,
        }
    }
}

/// The integrations each host and site has a covering credential for.
#[derive(Debug, Default)]
pub struct IntegrationCoverage {
    hosts: HashMap<Uuid, HashSet<CredentialIntegration>>,
    sites: HashMap<Uuid, HashSet<CredentialIntegration>>,
}

impl IntegrationCoverage {
    pub fn add_host(&mut self, host_id: Uuid, integration: CredentialIntegration) {
        self.hosts.entry(host_id).or_default().insert(integration);
    }

    /// A credential assigned to a whole site, which covers its hosts only while no scan has tried
    /// it there: one created after `tried_at`, or on a site never scanned.
    pub fn add_site(
        &mut self,
        site_id: Uuid,
        integration: CredentialIntegration,
        created_at: DateTime<Utc>,
        tried_at: Option<DateTime<Utc>>,
    ) {
        if tried_at.is_none_or(|tried| created_at > tried) {
            self.sites.entry(site_id).or_default().insert(integration);
        }
    }

    fn covers(&self, found: &FoundIntegration) -> bool {
        let has = |map: &HashMap<Uuid, HashSet<CredentialIntegration>>, id: &Uuid| {
            map.get(id).is_some_and(|s| s.contains(&found.integration))
        };
        has(&self.hosts, &found.host_id) || has(&self.sites, &found.site_id)
    }
}

/// The found integrations no credential covers, once per host and integration, ordered by
/// integration and then host so a run's warnings come out in a stable order.
pub fn find_uncovered(
    found: impl IntoIterator<Item = FoundIntegration>,
    coverage: &IntegrationCoverage,
) -> Vec<FoundIntegration> {
    let mut uncovered: Vec<FoundIntegration> = found
        .into_iter()
        .filter(|f| !coverage.covers(f))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    uncovered.sort_by_key(|f| (f.integration.id(), f.host_id));
    uncovered
}

impl CredentialService {
    /// The found integrations no stored credential covers.
    ///
    /// `integration_targets` are the run's own one-shot targets: `DaemonHost` covers
    /// `daemon_host_id`, and `Hosts` covers each found host holding one of its addresses, until the
    /// scan pins a working one to the host. A `Site` target was tried on every host of the run, and
    /// is pinned where it worked, so it covers nothing more. Pass none outside a run.
    ///
    /// `site_tried_at` says when each site's site-wide credentials were last tried.
    pub async fn uncovered_integrations(
        &self,
        found: Vec<FoundIntegration>,
        integration_targets: &[IntegrationTarget],
        daemon_host_id: Option<Uuid>,
        site_tried_at: &SiteTriedAt,
    ) -> Result<Vec<FoundIntegration>, Error> {
        if found.is_empty() {
            return Ok(Vec::new());
        }

        let host_ids: Vec<Uuid> = found
            .iter()
            .map(|f| f.host_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let site_ids: Vec<Uuid> = found
            .iter()
            .map(|f| f.site_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let host_assignments = self.get_credential_assignments_for_hosts(&host_ids).await?;
        let site_assignments = self.get_credential_ids_for_sites(&site_ids).await?;

        let credential_ids: Vec<Uuid> = host_assignments
            .values()
            .flatten()
            .map(|a| a.credential_id)
            .chain(site_assignments.values().flatten().copied())
            .chain(
                integration_targets
                    .iter()
                    .map(IntegrationTarget::credential_id),
            )
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let credentials: HashMap<Uuid, (CredentialIntegration, DateTime<Utc>)> = self
            .get_all(StorableFilter::<Credential>::new_from_entity_ids(
                &credential_ids,
            ))
            .await?
            .into_iter()
            .map(|c| {
                let integration = c.base.credential_type.discriminant().integration();
                (c.id, (integration, c.created_at))
            })
            .collect();

        let mut coverage = IntegrationCoverage::default();
        for (host_id, assignments) in &host_assignments {
            for a in assignments {
                if let Some((integration, _)) = credentials.get(&a.credential_id) {
                    coverage.add_host(*host_id, *integration);
                }
            }
        }
        for (site_id, ids) in &site_assignments {
            for id in ids {
                if let Some((integration, created_at)) = credentials.get(id) {
                    let tried_at = site_tried_at.get(site_id).copied();
                    coverage.add_site(*site_id, *integration, *created_at, tried_at);
                }
            }
        }

        let has_host_targets = integration_targets
            .iter()
            .any(|t| matches!(t, IntegrationTarget::Hosts { .. }));
        let addresses: Vec<IPAddress> = if has_host_targets {
            self.ip_address_service
                .get_all(StorableFilter::<IPAddress>::new_from_host_ids(&host_ids))
                .await?
        } else {
            Vec::new()
        };
        for target in integration_targets {
            let Some((integration, _)) = credentials.get(&target.credential_id()).copied() else {
                continue;
            };
            match target {
                IntegrationTarget::Site { .. } => {}
                IntegrationTarget::DaemonHost { .. } => {
                    if let Some(host_id) = daemon_host_id {
                        coverage.add_host(host_id, integration);
                    }
                }
                IntegrationTarget::Hosts { ips, .. } => {
                    for address in addresses
                        .iter()
                        .filter(|a| ips.contains(&a.base.ip_address))
                    {
                        coverage.add_host(address.base.host_id, integration);
                    }
                }
            }
        }

        Ok(find_uncovered(found, &coverage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use strum::IntoEnumIterator;

    fn found(host_id: Uuid, site_id: Uuid, integration: CredentialIntegration) -> FoundIntegration {
        FoundIntegration {
            host_id,
            site_id,
            integration,
        }
    }

    #[test]
    fn a_host_with_no_credential_is_uncovered() {
        let (host, site) = (Uuid::new_v4(), Uuid::new_v4());
        let f = found(host, site, CredentialIntegration::Proxmox);
        assert_eq!(
            find_uncovered([f], &IntegrationCoverage::default()),
            vec![f]
        );
    }

    #[test]
    fn a_host_assignment_covers_its_host_only() {
        let (covered, other, site) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let mut coverage = IntegrationCoverage::default();
        coverage.add_host(covered, CredentialIntegration::Proxmox);

        let uncovered = find_uncovered(
            [
                found(covered, site, CredentialIntegration::Proxmox),
                found(other, site, CredentialIntegration::Proxmox),
            ],
            &coverage,
        );
        assert_eq!(
            uncovered,
            vec![found(other, site, CredentialIntegration::Proxmox)]
        );
    }

    #[test]
    fn a_credential_of_another_integration_does_not_cover() {
        let (host, site) = (Uuid::new_v4(), Uuid::new_v4());
        let mut coverage = IntegrationCoverage::default();
        coverage.add_host(host, CredentialIntegration::Snmp);

        let f = found(host, site, CredentialIntegration::Docker);
        assert_eq!(find_uncovered([f], &coverage), vec![f]);
    }

    /// An untried site credential covers every host on that site and none elsewhere, which is
    /// what clears the home nudge the moment one is saved.
    #[test]
    fn an_untried_site_credential_covers_every_host_on_its_site() {
        let (site, other_site) = (Uuid::new_v4(), Uuid::new_v4());
        let scanned = crate::server::shared::clock::now();
        let mut coverage = IntegrationCoverage::default();
        coverage.add_site(
            site,
            CredentialIntegration::Snmp,
            scanned + Duration::minutes(5),
            Some(scanned),
        );

        let on_site = found(Uuid::new_v4(), site, CredentialIntegration::Snmp);
        let elsewhere = found(Uuid::new_v4(), other_site, CredentialIntegration::Snmp);
        assert_eq!(
            find_uncovered([on_site, elsewhere], &coverage),
            vec![elsewhere]
        );
    }

    /// Once a scan has tried a site credential, it covers only the hosts it worked on, which the
    /// scan pinned as host assignments. On a mixed SNMP estate the v3-only device stays named.
    #[test]
    fn a_tried_site_credential_covers_only_where_it_was_pinned() {
        let site = Uuid::new_v4();
        let (answered, silent) = (Uuid::new_v4(), Uuid::new_v4());
        let scanned = crate::server::shared::clock::now();
        let mut coverage = IntegrationCoverage::default();
        coverage.add_site(
            site,
            CredentialIntegration::Snmp,
            scanned - Duration::days(1),
            Some(scanned),
        );
        coverage.add_host(answered, CredentialIntegration::Snmp);

        assert_eq!(
            find_uncovered(
                [
                    found(answered, site, CredentialIntegration::Snmp),
                    found(silent, site, CredentialIntegration::Snmp),
                ],
                &coverage
            ),
            vec![found(silent, site, CredentialIntegration::Snmp)]
        );
    }

    /// A pinned credential that failed on the host is still assigned to it. The daemon reports the
    /// failure as a credential warning, so this raises nothing on top.
    #[test]
    fn a_failed_pinned_credential_still_covers() {
        let (host, site) = (Uuid::new_v4(), Uuid::new_v4());
        let mut coverage = IntegrationCoverage::default();
        coverage.add_host(host, CredentialIntegration::UnifiController);

        assert!(
            find_uncovered(
                [found(host, site, CredentialIntegration::UnifiController)],
                &coverage
            )
            .is_empty()
        );
    }

    /// Several services of one integration on a host (a Docker daemon on two ports) make one
    /// occurrence, and forty hosts make forty, which the surfaces group into one row.
    #[test]
    fn one_occurrence_per_host_and_integration() {
        let site = Uuid::new_v4();
        let hosts: Vec<Uuid> = (0..40).map(|_| Uuid::new_v4()).collect();
        let mut all: Vec<FoundIntegration> = hosts
            .iter()
            .map(|h| found(*h, site, CredentialIntegration::Snmp))
            .collect();
        all.push(found(hosts[0], site, CredentialIntegration::Snmp));

        let uncovered = find_uncovered(all, &IntegrationCoverage::default());
        assert_eq!(uncovered.len(), 40);
        assert!(
            uncovered
                .iter()
                .all(|f| f.integration == CredentialIntegration::Snmp)
        );
    }

    #[test]
    fn only_prompting_integrations_are_found() {
        for integration in CredentialIntegration::iter() {
            let service = Service {
                base: crate::server::services::r#impl::base::ServiceBase {
                    host_id: Uuid::new_v4(),
                    site_id: Uuid::new_v4(),
                    service_definition: integration.service(),
                    ..Default::default()
                },
                ..Default::default()
            };
            assert_eq!(
                FoundIntegration::from_service(&service).is_some(),
                integration.prompts_when_found(),
                "{integration:?}"
            );
        }
    }
}
