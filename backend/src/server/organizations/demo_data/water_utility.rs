//! The demo's "Water Utility" site: field stations on two fiber rings that meet at the core
//! switches of two treatment plants.

use super::estate_site::{ScanHistory, add_estate_site};
use super::large_estate::{RingParams, generate_ring_estate, water_utility_site};
use super::*;

/// Adds the Water Utility site and everything on it to `data`. Its last scan collected from every
/// switch.
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
    let history = ScanHistory {
        warnings: vec![],
        collected_hosts: params.switch_count() as u32,
        duration: Duration::minutes(31),
    };
    let site_id = site.id;
    add_estate_site(data, site, estate, history, user_id, use_case, now);

    let topology = data
        .topologies
        .iter()
        .find(|t| t.base.site_id == site_id)
        .expect("every demo site has a topology");
    data.shares.push(super::shares::water_utility_l2_share(
        topology, user_id, now,
    ));
}
