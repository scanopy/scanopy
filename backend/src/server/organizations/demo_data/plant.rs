//! The demo's "Plant" site: the default generated estate.

use super::estate_site::{ScanHistory, add_estate_site, network_snmp_credential};
use super::large_estate::{EstateParams, generate_large_estate, plant_site};
use super::*;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;

/// Adds the Plant site and everything on it to `data`.
pub(super) fn add_plant_site(
    data: &mut DemoData,
    organization_id: Uuid,
    user_id: Uuid,
    use_case: UseCase,
    now: DateTime<Utc>,
) {
    let site = plant_site(organization_id, now);
    let params = EstateParams::default();
    let estate = generate_large_estate(&params, &site, organization_id, now);

    // Plant's one Issue: the first access switch didn't answer SNMP on the last scan, so Home
    // lists it on this site too. The run collected from every other switch and firewall.
    let network_snmp = network_snmp_credential(data);
    let warnings: Vec<DiscoveryWarning> = estate
        .hosts_with_services
        .iter()
        .find(|h| h.host.base.name.value().as_str().starts_with("acc-sw-"))
        .and_then(|h| h.ip_addresses.first())
        .map(|ip| DiscoveryWarning::CredentialTargetNotResponding {
            address: ip.base.ip_address,
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            credential_id: Some(network_snmp),
        })
        .into_iter()
        .collect();
    let history = ScanHistory {
        collected_hosts: (params.switch_count() + params.firewalls - warnings.len()) as u32,
        warnings,
        duration: Duration::minutes(94),
    };
    add_estate_site(data, site, estate, history, user_id, use_case, now);
}
