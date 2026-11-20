//! The demo's "Water Utility" site: field stations on two fiber rings that meet at the core
//! switches of two treatment plants.

use super::estate_site::{ScanHistory, add_estate_site, network_snmp_credential};
use super::large_estate::{RingParams, generate_ring_estate, water_utility_site};
use super::*;
use crate::daemon::discovery::types::warnings::DiscoveryWarning;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::topology::types::grouping::EdgeColoring;
use crate::server::topology::types::views::TopologyView;

/// Adds the Water Utility site and everything on it to `data`.
pub(super) fn add_water_utility_site(
    data: &mut DemoData,
    organization_id: Uuid,
    user_id: Uuid,
    use_case: UseCase,
    now: DateTime<Utc>,
) {
    let site = water_utility_site(organization_id, now);
    let params = RingParams::default();
    let estate = generate_ring_estate(&params, &site, organization_id, now);

    // Water Utility's one Issue: the first east-ring station switch didn't answer SNMP on the
    // last scan, so Home lists it on this site too. A station switch is named `east-<code><nn>`;
    // the devices behind it add a suffix (`east-ps01-plc`).
    let network_snmp = network_snmp_credential(data);
    let warnings: Vec<DiscoveryWarning> = estate
        .hosts_with_services
        .iter()
        .find(|h| {
            let name = h.host.base.name.value().as_str();
            name.starts_with("east-") && name.matches('-').count() == 1
        })
        .and_then(|h| h.ip_addresses.first())
        .map(|ip| DiscoveryWarning::CredentialTargetNotResponding {
            address: ip.base.ip_address,
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            credential_id: Some(network_snmp),
        })
        .into_iter()
        .collect();
    let history = ScanHistory {
        collected_hosts: (params.switch_count() - warnings.len()) as u32,
        warnings,
        duration: Duration::minutes(31),
    };
    let site_id = site.id;
    add_estate_site(data, site, estate, history, user_id, use_case, now);

    let topology = data
        .topologies
        .iter_mut()
        .find(|t| t.base.site_id == site_id)
        .expect("every demo site has a topology");
    // The rings run at 10 Gb/s and the stations' devices at 1 Gb/s: the site is where the demo
    // shows links coloured by speed.
    topology
        .base
        .options
        .local
        .edge_coloring
        .insert(TopologyView::L2Physical, EdgeColoring::Speed);
    let share = super::shares::water_utility_l2_share(topology, user_id, now);
    data.shares.push(share);
}
