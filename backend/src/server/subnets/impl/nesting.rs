//! Which subnet sits inside which, and how full each one is.
//!
//! Two CIDR blocks either nest or are disjoint, so "overlap" between subnets is always one range
//! containing another. A site can hold a range a person recorded for the whole site above the
//! ranges discovery read for each segment, and those can nest again, to any depth.
//!
//! Addresses are filed in the most specific subnet that holds them
//! ([`placeable_subnet`](super::inference::placeable_subnet)), so a covering range holds none of
//! its nested ranges' addresses itself. Its utilization has to add theirs back in, or a `/16` over
//! four busy `/24`s would read as empty.
//!
//! Free of the database: it takes the subnets and the per-subnet address counts and returns the
//! derived view, so every rule here is testable without a Postgres.
use std::collections::HashMap;

use cidr::IpCidr;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::subnets::r#impl::base::{Subnet, SubnetCidrValue};

impl SubnetCidrValue {
    /// Addresses a host can hold in this range.
    ///
    /// IPv4 excludes the network and broadcast addresses, except on a `/31` (RFC 3021
    /// point-to-point, both usable) and a `/32` (the one address). IPv6 has no broadcast, so every
    /// address counts; ranges wider than a `/64` exceed `u64` and saturate.
    pub fn usable_addresses(&self) -> u64 {
        match self.0 {
            IpCidr::V4(c) => match c.network_length() {
                32 => 1,
                31 => 2,
                len => (1u64 << (32 - len)) - 2,
            },
            IpCidr::V6(c) => {
                let host_bits = 128 - u32::from(c.network_length());
                1u64.checked_shl(host_bits).unwrap_or(u64::MAX)
            }
        }
    }
}

/// A subnet together with what is derived from the rest of its site.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubnetResponse {
    #[serde(flatten)]
    pub subnet: Subnet,
    /// Distinct addresses held in this range, including those filed in subnets nested inside it.
    #[schema(read_only)]
    pub used_addresses: u64,
    /// Addresses a host can hold in this range. Saturates for IPv6 ranges wider than a `/64`.
    #[schema(read_only)]
    pub usable_addresses: u64,
    /// The most specific other subnet on the same site whose range contains this one. `null` when
    /// none does. Children are the subnets whose parent is this one.
    #[schema(read_only)]
    pub parent_subnet_id: Option<Uuid>,
}

/// Whether a subnet takes part in nesting at all.
///
/// The `0.0.0.0/0` organizational rows hold every address by design, so they would parent every
/// range on the site. Host-scoped subnets (a container runtime's bridge, keyed to its service) are
/// local to one host: every Docker host reuses 172.17.0.0/16, and none of them sits inside a range
/// on the LAN.
fn nests(subnet: &Subnet) -> bool {
    !subnet.is_organizational_subnet() && subnet.base.virtualization_service_id.is_none()
}

/// Whether `outer` strictly contains `inner`.
fn strictly_contains(outer: &IpCidr, inner: &IpCidr) -> bool {
    outer.family() == inner.family()
        && outer.network_length() < inner.network_length()
        && outer.contains(&inner.first_address())
}

/// Each subnet's parent: the most specific other subnet on its site that strictly contains it.
pub fn parents(subnets: &[Subnet]) -> HashMap<Uuid, Option<Uuid>> {
    subnets
        .iter()
        .map(|subnet| {
            let parent = nests(subnet)
                .then(|| {
                    subnets
                        .iter()
                        .filter(|candidate| {
                            candidate.id != subnet.id
                                && candidate.base.site_id == subnet.base.site_id
                                && nests(candidate)
                                && strictly_contains(&candidate.base.cidr, &subnet.base.cidr)
                        })
                        .max_by_key(|candidate| candidate.base.cidr.network_length())
                        .map(|candidate| candidate.id)
                })
                .flatten();
            (subnet.id, parent)
        })
        .collect()
}

/// The response for each of `targets`, derived against every subnet on their sites.
///
/// `site_subnets` must hold every live subnet on the sites `targets` belong to (it may include
/// `targets` themselves); `own_used` is each subnet's own distinct address count. A target's
/// `used_addresses` adds every descendant's own count to its own.
pub fn responses(
    targets: Vec<Subnet>,
    site_subnets: &[Subnet],
    own_used: &HashMap<Uuid, u64>,
) -> Vec<SubnetResponse> {
    let parent_of = parents(site_subnets);

    let mut used: HashMap<Uuid, u64> = HashMap::new();
    for subnet in site_subnets {
        let own = own_used.get(&subnet.id).copied().unwrap_or(0);
        if own == 0 {
            continue;
        }
        *used.entry(subnet.id).or_default() += own;
        // Strictly widening prefixes up the chain, so this always terminates.
        let mut ancestor = parent_of.get(&subnet.id).copied().flatten();
        while let Some(id) = ancestor {
            *used.entry(id).or_default() += own;
            ancestor = parent_of.get(&id).copied().flatten();
        }
    }

    targets
        .into_iter()
        .map(|subnet| {
            let used_addresses = used
                .get(&subnet.id)
                .copied()
                .unwrap_or_else(|| own_used.get(&subnet.id).copied().unwrap_or(0));
            let usable_addresses = subnet.base.cidr.value().usable_addresses();
            let parent_subnet_id = parent_of.get(&subnet.id).copied().flatten();
            SubnetResponse {
                subnet,
                used_addresses,
                usable_addresses,
                parent_subnet_id,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::subnets::r#impl::base::{SubnetBase, SubnetCidr};

    fn cidr(s: &str) -> SubnetCidrValue {
        SubnetCidrValue(s.parse().unwrap())
    }

    fn subnet(range: &str, site_id: Uuid) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(cidr(range), AttributeSource::Manual),
            site_id,
            ..Default::default()
        })
    }

    #[test]
    fn usable_addresses_follow_the_mask() {
        assert_eq!(cidr("10.0.0.0/24").usable_addresses(), 254);
        assert_eq!(cidr("10.0.0.0/30").usable_addresses(), 2);
        assert_eq!(cidr("10.0.0.0/31").usable_addresses(), 2);
        assert_eq!(cidr("10.0.0.1/32").usable_addresses(), 1);
        assert_eq!(cidr("fd00::/120").usable_addresses(), 256);
        assert_eq!(cidr("fd00::/64").usable_addresses(), u64::MAX);
        assert_eq!(cidr("fd00::/48").usable_addresses(), u64::MAX);
    }

    #[test]
    fn the_most_specific_container_is_the_parent() {
        let site = Uuid::new_v4();
        let wide = subnet("10.10.0.0/16", site);
        let mid = subnet("10.10.16.0/20", site);
        let leaf = subnet("10.10.16.0/24", site);
        let sibling = subnet("10.10.32.0/24", site);
        let parent_of = parents(&[wide.clone(), mid.clone(), leaf.clone(), sibling.clone()]);

        assert_eq!(parent_of[&wide.id], None);
        assert_eq!(parent_of[&mid.id], Some(wide.id));
        assert_eq!(parent_of[&leaf.id], Some(mid.id));
        assert_eq!(parent_of[&sibling.id], Some(wide.id));
    }

    #[test]
    fn other_sites_organizational_and_host_scoped_rows_never_nest() {
        let site = Uuid::new_v4();
        let wide = subnet("172.16.0.0/12", site);
        let elsewhere = subnet("172.17.0.0/24", Uuid::new_v4());
        let mut bridge = subnet("172.17.0.0/16", site);
        bridge.base.virtualization_service_id = Some(Uuid::new_v4());
        let internet = subnet("0.0.0.0/0", site);
        let lone = subnet("192.168.1.0/24", site);
        let parent_of = parents(&[
            wide.clone(),
            elsewhere.clone(),
            bridge.clone(),
            internet.clone(),
            lone.clone(),
        ]);

        assert_eq!(parent_of[&elsewhere.id], None);
        assert_eq!(parent_of[&bridge.id], None);
        assert_eq!(parent_of[&wide.id], None);
        assert_eq!(parent_of[&lone.id], None);
    }

    #[test]
    fn a_covering_range_counts_its_nested_ranges_addresses() {
        let site = Uuid::new_v4();
        let wide = subnet("10.10.0.0/16", site);
        let mid = subnet("10.10.16.0/20", site);
        let leaf = subnet("10.10.16.0/24", site);
        let lone = subnet("192.168.1.0/24", site);
        let all = vec![wide.clone(), mid.clone(), leaf.clone(), lone.clone()];
        let own_used = HashMap::from([(wide.id, 1), (mid.id, 2), (leaf.id, 10), (lone.id, 7)]);

        let used: HashMap<Uuid, u64> = responses(all.clone(), &all, &own_used)
            .into_iter()
            .map(|r| (r.subnet.id, r.used_addresses))
            .collect();

        assert_eq!(used[&wide.id], 13);
        assert_eq!(used[&mid.id], 12);
        assert_eq!(used[&leaf.id], 10);
        assert_eq!(used[&lone.id], 7);
    }
}
