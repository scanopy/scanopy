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
                    // As an SNMP scan records it: a name set on the switch, read back over SNMP.
                    name: VlanName::new(
                        VlanNameValue(name.to_string()),
                        AttributeSource::Authored(ClientProbe::Snmp),
                    ),
                    description: Some(description.to_string()),
                    site_id: site.id,
                    organization_id,
                    source: EntitySource::Discovery,
                    subnet_ids: Vec::new(),
                    interface_ids: Vec::new(),
                    tags: tag.and_then(tag_id).into_iter().collect(),
                },
            });
        }
    }

    vlans
}

/// Derive subnet↔VLAN junction rows the same way the server's
/// `reconcile_subnet_vlans_for_host` does at discovery time, from its two kinds of evidence:
///
/// - an interface whose `native_vlan_id` is that VLAN, attached (via `ip_address_id`) to an IP
///   address on that subnet. Only `native_vlan_id` counts — tagged/trunk `vlan_ids` do not;
/// - an access port in that VLAN that learned the MAC of an address on that subnet, by
///   `access_port_vlans`, the function the reconciler itself calls.
///
/// Deduped on `(subnet_id, vlan_id)`.
pub(super) fn generate_subnet_vlan_records(
    interfaces: &[Interface],
    hosts_with_services: &[HostWithServices],
    now: DateTime<Utc>,
) -> Vec<SubnetVlanRecord> {
    use crate::server::hosts::service::subnet_vlans::access_port_vlans;
    use crate::server::ip_addresses::r#impl::base::mac_of;
    use mac_address::MacAddress;
    use std::collections::{HashMap, HashSet};

    // ip_address_id → subnet_id, from every host's IP addresses.
    let ip_to_subnet: HashMap<Uuid, Uuid> = hosts_with_services
        .iter()
        .flat_map(|h| h.ip_addresses.iter())
        .map(|ip| (ip.id, ip.base.subnet_id))
        .collect();

    let ip_addresses: Vec<&IPAddress> = hosts_with_services
        .iter()
        .flat_map(|h| h.ip_addresses.iter())
        .collect();
    let mut subnets_by_mac: HashMap<MacAddress, HashSet<Uuid>> = HashMap::new();
    let mut ips_by_subnet: HashMap<Uuid, Vec<IPAddress>> = HashMap::new();
    for ip in &ip_addresses {
        if let Some(mac) = mac_of(&ip.base.mac_address) {
            subnets_by_mac
                .entry(mac)
                .or_default()
                .insert(ip.base.subnet_id);
        }
        ips_by_subnet
            .entry(ip.base.subnet_id)
            .or_default()
            .push((*ip).clone());
    }

    let direct = interfaces.iter().filter_map(|iface| {
        let vlan_id = iface.base.native_vlan_id?;
        let subnet_id = *ip_to_subnet.get(&iface.base.ip_address_id?)?;
        Some((subnet_id, vlan_id))
    });
    let learned = ips_by_subnet.iter().flat_map(|(&subnet_id, subnet_ips)| {
        access_port_vlans(subnet_id, subnet_ips, &subnets_by_mac, interfaces)
            .into_iter()
            .map(move |vlan_id| (subnet_id, vlan_id))
    });

    let mut seen: HashSet<(Uuid, Uuid)> = HashSet::new();
    direct
        .chain(learned)
        .filter(|pair| seen.insert(*pair))
        .map(|(subnet_id, vlan_id)| SubnetVlanRecord {
            id: Uuid::new_v4(),
            created_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            base: SubnetVlanRecordBase::new(subnet_id, vlan_id),
        })
        .collect()
}

/// The links a person made by hand. HQ Annex has had no scan, so discovery has no interface on it
/// to read a VLAN from, but whoever wired the annex knows its staff ports carry the Users VLAN.
pub(super) fn generate_manual_subnet_vlan_records(
    subnets: &[Subnet],
    vlans: &[Vlan],
    now: DateTime<Utc>,
) -> Vec<SubnetVlanRecord> {
    let annex = subnets.iter().find(|s| s.base.name() == "HQ Annex");
    let users = annex.and_then(|annex| {
        vlans
            .iter()
            .find(|v| v.base.site_id == annex.base.site_id && v.base.vlan_number == 30)
    });
    match (annex, users) {
        (Some(annex), Some(users)) => vec![SubnetVlanRecord {
            id: Uuid::new_v4(),
            created_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            base: SubnetVlanRecordBase::manual(annex.id, users.id),
        }],
        _ => Vec::new(),
    }
}
