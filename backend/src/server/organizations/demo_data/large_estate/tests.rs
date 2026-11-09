use super::*;
use std::collections::{HashSet, VecDeque};

fn estate(params: &EstateParams) -> LargeEstate {
    let organization_id = Uuid::new_v4();
    let now = crate::server::shared::clock::now();
    generate_large_estate(
        params,
        &plant_site(organization_id, now),
        organization_id,
        now,
    )
}

fn names(estate: &LargeEstate) -> HashMap<Uuid, String> {
    estate
        .hosts_with_services
        .iter()
        .map(|h| (h.host.id, h.host.base.name.to_string()))
        .collect()
}

/// The seeder resolves links by host name, so a duplicate name would drop links silently.
#[test]
fn host_names_are_unique() {
    let estate = estate(&EstateParams::default());
    let names = names(&estate);
    let unique: HashSet<&String> = names.values().collect();
    assert_eq!(unique.len(), names.len());
}

/// Every link names a host and port that exist, as the seeder needs to write it.
#[test]
fn every_link_resolves_to_an_interface() {
    let estate = estate(&EstateParams::default());
    let names = names(&estate);
    let ports: HashSet<(String, Option<i32>)> = estate
        .interfaces
        .iter()
        .map(|i| (names[&i.base.host_id].clone(), i.base.if_index))
        .collect();
    let host_names: HashSet<&String> = names.values().collect();

    for link in &estate.neighbor_updates {
        assert!(ports.contains(&(link.source_host_name.clone(), Some(link.source_if_index))));
        assert!(ports.contains(&(link.target_host_name.clone(), link.target_if_index)));
    }
    for link in &estate.host_neighbor_updates {
        assert!(ports.contains(&(link.source_host_name.clone(), Some(link.source_if_index))));
        assert!(host_names.contains(&link.target_host_name));
    }
    assert!(
        !estate.host_neighbor_updates.is_empty(),
        "the default estate mixes in device-level links"
    );
}

/// Port-to-port links are recorded from both ends, so the L2 view draws each once from either
/// switch.
#[test]
fn port_to_port_links_are_reciprocal() {
    let estate = estate(&EstateParams::default());
    let links: HashSet<(&str, i32, &str, Option<i32>)> = estate
        .neighbor_updates
        .iter()
        .map(|n| {
            (
                n.source_host_name.as_str(),
                n.source_if_index,
                n.target_host_name.as_str(),
                n.target_if_index,
            )
        })
        .collect();
    for &(source, source_port, target, target_port) in &links {
        let target_port = target_port.expect("estate links are port to port");
        assert!(links.contains(&(target, target_port, source, Some(source_port))));
    }
}

/// Every switch and firewall reaches every other over links, so the fabric is one tree from the
/// cores down, not islands.
#[test]
fn the_fabric_is_connected() {
    for params in [EstateParams::small(), EstateParams::default()] {
        let estate = estate(&params);
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
        let edges = estate
            .neighbor_updates
            .iter()
            .map(|n| (n.source_host_name.as_str(), n.target_host_name.as_str()))
            .chain(
                estate
                    .host_neighbor_updates
                    .iter()
                    .map(|n| (n.source_host_name.as_str(), n.target_host_name.as_str())),
            );
        for (a, b) in edges {
            adjacency.entry(a).or_default().push(b);
            adjacency.entry(b).or_default().push(a);
        }

        let names = names(&estate);
        let linked_hosts: HashSet<&str> = estate
            .interfaces
            .iter()
            .map(|i| names[&i.base.host_id].as_str())
            .collect();
        let root = *linked_hosts.iter().next().unwrap();
        let mut seen = HashSet::from([root]);
        let mut queue = VecDeque::from([root]);
        while let Some(host) = queue.pop_front() {
            for &next in adjacency.get(host).into_iter().flatten() {
                if seen.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        assert_eq!(seen, linked_hosts);
        assert_eq!(linked_hosts.len(), params.switch_count() + params.firewalls);
    }
}

/// Every attached host sits behind exactly one access port, which learned its MAC.
#[test]
fn every_attached_host_is_learned_on_one_port() {
    let params = EstateParams::default();
    let estate = estate(&params);
    let mut learned: HashMap<String, usize> = HashMap::new();
    for iface in &estate.interfaces {
        for mac in iface.base.fdb_macs.iter().flat_map(|f| f.value().0.iter()) {
            *learned.entry(mac.clone()).or_default() += 1;
        }
    }
    assert_eq!(learned.len(), params.servers + params.endpoints);
    assert!(learned.values().all(|&n| n == 1));
}

#[test]
fn addresses_fall_inside_their_subnets_without_duplicates() {
    let estate = estate(&EstateParams::default());
    let cidrs: HashMap<Uuid, IpCidr> = estate
        .subnets
        .iter()
        .map(|s| (s.id, s.base.cidr.value().0))
        .collect();
    let mut seen = HashSet::new();
    for ip in estate
        .hosts_with_services
        .iter()
        .flat_map(|h| &h.ip_addresses)
    {
        assert!(cidrs[&ip.base.subnet_id].contains(&ip.base.ip_address));
        assert!(
            seen.insert(ip.base.ip_address),
            "{} assigned twice",
            ip.base.ip_address
        );
    }
}

#[test]
fn presets_hit_their_switch_and_host_counts() {
    for (params, switches, hosts) in [
        (EstateParams::small(), 25, 500),
        (EstateParams::large(), 800, 15_000),
    ] {
        let estate = estate(&params);
        let management = estate
            .subnets
            .iter()
            .find(|s| s.base.subnet_type.value().0 == SubnetType::Management)
            .unwrap()
            .id;
        let on_management = estate
            .hosts_with_services
            .iter()
            .filter(|h| {
                h.ip_addresses
                    .iter()
                    .any(|ip| ip.base.subnet_id == management)
            })
            .count();
        assert_eq!(on_management, switches);
        assert_eq!(estate.hosts_with_services.len(), hosts);
    }
}

/// A smaller VLAN plan still places every host, on the office segment when its own is absent.
#[test]
fn a_truncated_vlan_plan_still_places_every_host() {
    let params = EstateParams {
        vlans: 4,
        ..EstateParams::small()
    };
    let estate = estate(&params);
    assert_eq!(estate.vlans.len(), 4);
    assert_eq!(estate.hosts_with_services.len(), params.host_count());
}
