//! VLANs, and subnet↔VLAN junction records derived from interface data.

use super::*;

pub(super) fn generate_vlans(
    sites: &[Site],
    tags: &[Tag],
    organization_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<Vlan> {
    let mut vlans = Vec::new();

    let tag_id = |name: &str| tags.iter().find(|t| t.base.name == name).map(|t| t.id);

    let vlan_defs: Vec<(u16, &str, &str, Option<&str>)> = vec![
        (
            1,
            "Default",
            "Untagged native VLAN on trunk ports. Nothing should live here.",
            None,
        ),
        (
            10,
            "Management",
            "Switch, firewall and hypervisor management interfaces.",
            Some("Critical"),
        ),
        (
            20,
            "Servers",
            "Production servers and the NAS.",
            Some("Production"),
        ),
        (30, "Users", "Staff workstations and printers.", None),
        (
            100,
            "Guest",
            "Visitor Wi-Fi. Internet access only, isolated from internal VLANs.",
            None,
        ),
    ];

    for site in sites {
        for &(vlan_number, name, description, tag) in &vlan_defs {
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
                    description: Some(description.to_string()),
                    site_id: site.id,
                    organization_id,
                    source: EntitySource::Discovery,
                    subnet_ids: Vec::new(),
                    tags: tag.and_then(tag_id).into_iter().collect(),
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
