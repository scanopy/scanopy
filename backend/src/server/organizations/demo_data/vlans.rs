//! VLANs, and subnet↔VLAN junction records derived from interface data.

use super::*;

pub(super) fn generate_vlans(
    networks: &[Network],
    organization_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<Vlan> {
    let mut vlans = Vec::new();

    let vlan_defs: Vec<(u16, &str)> = vec![
        (1, "Default"),
        (10, "Management"),
        (20, "Servers"),
        (30, "Users"),
        (100, "Guest"),
    ];

    for network in networks {
        for &(vlan_number, name) in &vlan_defs {
            vlans.push(Vlan {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                valid_from: now,
                valid_to: None,
                lineage_id: None,
                last_seen_at: now,
                last_discovery_id: None,
                first_discovery_id: None,
                base: VlanBase {
                    vlan_number,
                    name: name.to_string(),
                    description: None,
                    network_id: network.id,
                    organization_id,
                    source: EntitySource::Discovery,
                    subnet_ids: Vec::new(),
                },
            });
        }
    }

    vlans
}

/// Derive subnet↔VLAN junction rows the same way the server's
/// `reconcile_subnet_vlans_for_host` does at discovery time: a VLAN links to a
/// subnet when an interface whose `native_vlan_id` is that VLAN is attached
/// (via `ip_address_id`) to an IP address on that subnet. Only `native_vlan_id`
/// counts — tagged/trunk `vlan_ids` do not create subnet links. Deduped on
/// `(subnet_id, vlan_id)`.
pub(super) fn generate_subnet_vlan_records(
    interfaces: &[Interface],
    hosts_with_services: &[HostWithServices],
    now: DateTime<Utc>,
) -> Vec<SubnetVlanRecord> {
    use std::collections::{HashMap, HashSet};

    // ip_address_id → subnet_id, from every host's IP addresses.
    let ip_to_subnet: HashMap<Uuid, Uuid> = hosts_with_services
        .iter()
        .flat_map(|h| h.ip_addresses.iter())
        .map(|ip| (ip.id, ip.base.subnet_id))
        .collect();

    let mut seen: HashSet<(Uuid, Uuid)> = HashSet::new();
    let mut records = Vec::new();
    for iface in interfaces {
        if let (Some(vlan_id), Some(ip_id)) = (iface.base.native_vlan_id, iface.base.ip_address_id)
            && let Some(&subnet_id) = ip_to_subnet.get(&ip_id)
            && seen.insert((subnet_id, vlan_id))
        {
            records.push(SubnetVlanRecord {
                id: Uuid::new_v4(),
                created_at: now,
                valid_from: now,
                valid_to: None,
                lineage_id: None,
                base: SubnetVlanRecordBase::new(subnet_id, vlan_id),
            });
        }
    }
    records
}
