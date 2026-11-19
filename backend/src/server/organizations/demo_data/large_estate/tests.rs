use super::*;
use std::collections::{HashSet, VecDeque};

const GIGABIT: i64 = 1_000_000_000;

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

fn ring_estate(params: &RingParams) -> LargeEstate {
    let organization_id = Uuid::new_v4();
    let now = crate::server::shared::clock::now();
    generate_ring_estate(
        params,
        &water_utility_site(organization_id, now),
        organization_id,
        now,
    )
}

/// Switch-to-switch adjacency by host name, one entry per neighbour however many links join them.
fn switch_adjacency(estate: &LargeEstate) -> HashMap<&str, HashSet<&str>> {
    let mut adjacency: HashMap<&str, HashSet<&str>> = HashMap::new();
    for n in &estate.neighbor_updates {
        adjacency
            .entry(n.source_host_name.as_str())
            .or_default()
            .insert(n.target_host_name.as_str());
    }
    adjacency
}

/// Each ring leaves one core, passes through every one of its station switches exactly once, and
/// reaches the other core, which links back to the first: a closed loop through both cores.
#[test]
fn each_ring_is_a_closed_loop_through_both_cores() {
    let params = RingParams::default();
    let estate = ring_estate(&params);
    let adjacency = switch_adjacency(&estate);
    let cores: Vec<&str> = adjacency
        .keys()
        .copied()
        .filter(|name| name.starts_with("wtp-"))
        .collect();
    assert_eq!(cores.len(), 2);
    let (first, second) = if cores[0] < cores[1] {
        (cores[0], cores[1])
    } else {
        (cores[1], cores[0])
    };
    assert!(
        adjacency[first].contains(second),
        "the cores link to each other"
    );

    for &(ring, stations) in &params.rings {
        let prefix = format!("{}-", ring.to_lowercase());
        let entries: Vec<&str> = adjacency[first]
            .iter()
            .copied()
            .filter(|n| n.starts_with(&prefix))
            .collect();
        assert_eq!(entries.len(), 1, "{ring} ring leaves {first} once");

        let mut visited = vec![first];
        let mut current = entries[0];
        while current != second {
            assert!(
                current.starts_with(&prefix),
                "{current} is off the {ring} ring"
            );
            assert!(!visited.contains(&current), "{current} is visited twice");
            let next: Vec<&str> = adjacency[current]
                .iter()
                .copied()
                .filter(|n| *n != *visited.last().unwrap())
                .collect();
            assert_eq!(next.len(), 1, "{current} has two ring neighbours");
            visited.push(current);
            current = next[0];
        }
        assert_eq!(visited.len() - 1, stations);
    }
}

/// Every switch-to-switch link is recorded from both ends, port to port, between two 10 Gb/s
/// ports, so the L2 view resolves it to the port at each end.
#[test]
fn ring_links_resolve_port_to_port_at_ten_gigabit() {
    let estate = ring_estate(&RingParams::default());
    let names = names(&estate);
    let speeds: HashMap<(&str, i32), Option<i64>> = estate
        .interfaces
        .iter()
        .filter_map(|i| {
            Some((
                (names[&i.base.host_id].as_str(), i.base.if_index?),
                i.base.speed_bps,
            ))
        })
        .collect();
    let links: HashSet<(&str, i32, &str, i32)> = estate
        .neighbor_updates
        .iter()
        .map(|n| {
            (
                n.source_host_name.as_str(),
                n.source_if_index,
                n.target_host_name.as_str(),
                n.target_if_index.expect("ring links are port to port"),
            )
        })
        .collect();
    assert!(estate.host_neighbor_updates.is_empty());
    for &(source, source_port, target, target_port) in &links {
        assert!(links.contains(&(target, target_port, source, source_port)));
        for end in [(source, source_port), (target, target_port)] {
            assert_eq!(speeds.get(&end), Some(&Some(10 * GIGABIT)), "{end:?}");
        }
    }
}

/// Every station switch has a handful of devices cabled to it, each on a 1 Gb/s access port that
/// learned its MAC.
#[test]
fn every_station_carries_a_handful_of_devices_on_gigabit_ports() {
    let estate = ring_estate(&RingParams::default());
    let names = names(&estate);
    let mut per_switch: HashMap<&str, usize> = HashMap::new();
    for iface in &estate.interfaces {
        let Some(fdb) = &iface.base.fdb_macs else {
            continue;
        };
        assert_eq!(iface.base.speed_bps, Some(GIGABIT));
        *per_switch
            .entry(names[&iface.base.host_id].as_str())
            .or_default() += fdb.value().0.len();
    }
    let stations: Vec<&str> = switch_adjacency(&estate)
        .into_keys()
        .filter(|n| !n.starts_with("wtp-"))
        .collect();
    for station in stations {
        let devices = per_switch.get(station).copied().unwrap_or(0);
        assert!(
            (3..=6).contains(&devices),
            "{station} has {devices} devices"
        );
    }
}

/// The seeder resolves links by host name, and MACs identify hosts across the organization, so
/// neither repeats within the ring estate or between it and the default estate.
#[test]
fn ring_estate_names_and_macs_are_unique_and_distinct_from_plant() {
    let ring = ring_estate(&RingParams::default());
    let ring_names = names(&ring);
    assert_eq!(
        ring_names.values().collect::<HashSet<_>>().len(),
        ring_names.len()
    );

    let macs = |estate: &LargeEstate| -> HashSet<String> {
        estate
            .hosts_with_services
            .iter()
            .flat_map(|h| &h.ip_addresses)
            .filter_map(|ip| ip.base.mac_address.as_ref())
            .map(|m| m.value().0.to_string())
            .collect()
    };
    let ring_macs = macs(&ring);
    let plant_macs = macs(&estate(&EstateParams::default()));
    assert!(ring_macs.is_disjoint(&plant_macs));
}
