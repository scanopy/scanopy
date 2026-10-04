//! A container's network endpoints, read from its inspect response.
//!
//! Pure: no I/O, so the tests below build containers from bollard struct literals.

use std::collections::HashMap;
use std::net::IpAddr;

use bollard::models::{ContainerInspectResponse, ContainerSummary, EndpointSettings};
use mac_address::MacAddress;
use uuid::Uuid;

use crate::server::hosts::r#impl::virtualization::ContainerNetworkType;
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue,
};
use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::inference::placeable_subnet;
use crate::server::subnets::r#impl::types::SubnetType;

use super::ContainerRuntime;

/// The container network type a runtime network's subnet type stands for. `None` for every type
/// that does not put a container on the LAN with an address of its own.
pub fn lan_network_type(subnet_type: SubnetType) -> Option<ContainerNetworkType> {
    match subnet_type {
        SubnetType::MacVlan => Some(ContainerNetworkType::MacVlan),
        SubnetType::IpVlan => Some(ContainerNetworkType::IpVlan),
        _ => None,
    }
}

/// A container's attachments, sorted by network name.
///
/// The runtime reports them in a map, so iteration order varies run to run. The first address a
/// caller builds becomes the container's primary endpoint, so the order has to be stable.
fn sorted_endpoints(container: &ContainerInspectResponse) -> Vec<(&String, &EndpointSettings)> {
    let mut endpoints: Vec<(&String, &EndpointSettings)> = container
        .network_settings
        .as_ref()
        .and_then(|s| s.networks.as_ref())
        .map(|networks| networks.iter().collect())
        .unwrap_or_default();
    endpoints.sort_by(|(a, _), (b, _)| a.cmp(b));
    endpoints
}

/// Whether `container` has an endpoint on one of the runtime's macvlan or ipvlan networks, and
/// which type the first such network (by name) is.
///
/// `lan_subnets` are the runtime's own macvlan/ipvlan networks, named after the network as
/// [`ContainerRuntime::subnet_from_network`] names them. The endpoint is bound by that name, as
/// [`subnet_for_container_network`] binds bridge endpoints, so the driver comes from the runtime's
/// network listing rather than from the address.
pub fn container_lan_network_type(
    container: &ContainerInspectResponse,
    lan_subnets: &[Subnet],
) -> Option<ContainerNetworkType> {
    sorted_endpoints(container)
        .into_iter()
        .find_map(|(network_name, _)| {
            lan_subnets
                .iter()
                .find(|s| s.base.name == *network_name)
                .and_then(|s| lan_network_type(s.base.subnet_type))
        })
}

/// The subnet a container endpoint on `network_name` belongs to.
///
/// Bound by the runtime's own network **identity**, not by which CIDR happens to contain the
/// address. The API already told us which network the endpoint is on and
/// `create_network_subnets` names each subnet after it, so re-deriving that by address would
/// trade a fact for a guess, and a guess that goes wrong in two ways: the network-wide list
/// carries `0.0.0.0/0` catch-alls that contain every IPv4 address, and a bridge is host-scoped,
/// so two daemons legitimately hold the same `172.17.0.0/16` and containment cannot tell them
/// apart.
///
/// A network with several IPAM pools yields several subnets sharing a name, so containment
/// chooses among *that network's* subnets, never outside them.
pub fn subnet_for_container_network<'s>(
    bridge_subnets: &'s [Subnet],
    network_name: &str,
    ip_address: IpAddr,
) -> Option<&'s Subnet> {
    let candidates: Vec<&Subnet> = bridge_subnets
        .iter()
        .filter(|s| s.base.name == network_name)
        .collect();
    candidates
        .iter()
        .find(|s| s.base.cidr.contains(&ip_address))
        .or_else(|| candidates.first())
        .copied()
}

/// An endpoint's MAC as the runtime recorded it, filed under `source`.
fn endpoint_mac(endpoint: &EndpointSettings, source: AttributeSource) -> Option<MacEvidence> {
    endpoint
        .mac_address
        .as_ref()
        .and_then(|mac_str| mac_str.parse::<MacAddress>().ok())
        .map(|m| MacEvidence::new(MacEvidenceValue(m), source))
}

/// An endpoint's address on a bridge network, placed in that network's subnet.
///
/// The MAC is filed as the runtime's report (`Probe`): a bridge is private to its host, so its
/// MAC identifies nothing on the LAN.
fn bridge_endpoint_address(
    runtime: ContainerRuntime,
    bridge_subnets: &[Subnet],
    network_name: &str,
    endpoint: &EndpointSettings,
) -> Option<(IPAddress, Subnet)> {
    let ip_address = endpoint.ip_address.as_ref()?.parse::<IpAddr>().ok()?;
    let subnet = subnet_for_container_network(bridge_subnets, network_name, ip_address)?;
    Some((
        IPAddress::new(IPAddressBase {
            network_id: subnet.base.network_id,
            host_id: Uuid::nil(), // Placeholder - server will set correct host_id
            subnet_id: subnet.id,
            ip_address,
            mac_address: endpoint_mac(endpoint, AttributeSource::Probe(runtime.client_probe())),
            name: Some(network_name.to_owned()),
            position: 0,
        }),
        subnet.clone(),
    ))
}

/// The addresses of every container on today's path (bridge, host networking, shared netns),
/// keyed by container id, each with the subnet it sits in.
///
/// A bridge container's own endpoints come first, then the runtime host's addresses: its
/// published ports are reached there. A host-networking container has only the host's
/// addresses. Endpoints on networks with no bridge subnet are left out, which is where a
/// macvlan/ipvlan endpoint goes when such a container is passed here.
pub fn container_interfaces(
    runtime: ContainerRuntime,
    containers: &[(ContainerInspectResponse, ContainerSummary)],
    bridge_subnets: &[Subnet],
    known_subnets: &[Subnet],
    host_interfaces: &mut [IPAddress],
) -> HashMap<String, Vec<(IPAddress, Subnet)>> {
    // The host's own addresses are a genuine address lookup (nothing names a network for them),
    // so they are placed by the shared rule: longest prefix, never a `0.0.0.0/0` catch-all.
    let host_interfaces_and_subnets = host_interfaces
        .iter_mut()
        .filter_map(|i| {
            let placed = placeable_subnet(known_subnets, i.base.ip_address)
                .or_else(|| placeable_subnet(bridge_subnets, i.base.ip_address));

            match placed {
                Some(subnet) => {
                    i.base.subnet_id = subnet.id;
                    Some((i.clone(), subnet.clone()))
                }
                None => {
                    tracing::warn!(
                        ip = %i.base.ip_address,
                        "No subnet holds this host address; it is left unplaced"
                    );
                    None
                }
            }
        })
        .collect::<Vec<(IPAddress, Subnet)>>();

    let mut interfaces_by_id: HashMap<String, Vec<(IPAddress, Subnet)>> = containers
        .iter()
        .filter_map(|(container, _)| {
            let host_networking_mode = container
                .host_config
                .as_ref()
                .and_then(|c| c.network_mode.clone())
                .unwrap_or_default()
                == "host";

            let mut ip_addresses_and_subnets: Vec<(IPAddress, Subnet)> = if host_networking_mode {
                host_interfaces_and_subnets.clone()
            } else {
                sorted_endpoints(container)
                    .into_iter()
                    .filter_map(|(network_name, endpoint)| {
                        let address = bridge_endpoint_address(
                            runtime,
                            bridge_subnets,
                            network_name,
                            endpoint,
                        );
                        if address.is_none() {
                            tracing::warn!(
                                "No matching subnet found for container {:?} on network '{}'",
                                container.name,
                                network_name
                            );
                        }
                        address
                    })
                    .collect()
            };

            // The first entry becomes the container's primary endpoint (the one service matching
            // is anchored to, and the one the container-runtime edge targets), so sort by network
            // name to keep both stable across scans.
            ip_addresses_and_subnets.sort_by(|(a, _), (b, _)| {
                a.base
                    .name
                    .cmp(&b.base.name)
                    .then_with(|| a.base.ip_address.cmp(&b.base.ip_address))
            });

            // A bridge container's published ports are reached at the host's addresses.
            if !host_networking_mode {
                ip_addresses_and_subnets.extend(host_interfaces_and_subnets.clone());
            }

            container
                .id
                .as_ref()
                .map(|id| (id.clone(), ip_addresses_and_subnets))
        })
        .collect();

    // Pod / shared-netns members run with NetworkMode "container:<id>": they share the
    // referenced container's network namespace and report no networks of their own (so the
    // pass above leaves them with empty interfaces and they'd be dropped). Inherit the
    // referenced container's interfaces so the member is still discovered (e.g. a pod's
    // nginx member sharing the infra container's IP). The reference may be a short or full id.
    let shared_netns_members: Vec<(String, String)> = containers
        .iter()
        .filter_map(|(container, _)| {
            let mode = container
                .host_config
                .as_ref()
                .and_then(|c| c.network_mode.clone())
                .unwrap_or_default();
            let reference = mode.strip_prefix("container:")?.to_string();
            Some((container.id.clone()?, reference))
        })
        .collect();

    for (member_id, reference) in shared_netns_members {
        if let Some(parent_id) = interfaces_by_id
            .keys()
            .find(|k| k.starts_with(&reference))
            .cloned()
            && let Some(parent_ifaces) = interfaces_by_id.get(&parent_id).cloned()
            && !parent_ifaces.is_empty()
        {
            interfaces_by_id.insert(member_id, parent_ifaces);
        }
    }

    interfaces_by_id
}

/// A container's addresses as a host of its own, with the subnet each was placed in.
///
/// Its macvlan/ipvlan addresses come first (IPv4 and the global IPv6 address), named by network
/// and filed by [`IPAddress::discovered`] over `placement_subnets`: the subnet is `None` when the
/// server is to infer one, and an address it could place nowhere is left out. Bridge addresses follow, on
/// their bridge subnet. The runtime host's addresses are not among them: they belong to the
/// runtime's host.
///
/// A macvlan endpoint's MAC is the runtime's own assignment, kept for the container's life and
/// put on the wire for it, so it is filed as `HypervisorConfig`, the same as a hypervisor's guest
/// NIC. An ipvlan endpoint has no MAC of its own (it shares the parent interface's), so none is
/// recorded: filing the parent's MAC would merge the container into the host it runs on.
pub fn container_host_addresses(
    runtime: ContainerRuntime,
    container: &ContainerInspectResponse,
    lan_subnets: &[Subnet],
    bridge_subnets: &[Subnet],
    placement_subnets: &[Subnet],
    network_id: Uuid,
) -> Vec<(IPAddress, Option<Subnet>)> {
    let endpoints = sorted_endpoints(container);
    let mut lan_addresses: Vec<(IPAddress, Option<Subnet>)> = Vec::new();
    let mut bridge_addresses: Vec<(IPAddress, Option<Subnet>)> = Vec::new();

    for (network_name, endpoint) in endpoints {
        let lan_type = lan_subnets
            .iter()
            .find(|s| s.base.name == *network_name)
            .and_then(|s| lan_network_type(s.base.subnet_type));

        let Some(lan_type) = lan_type else {
            if let Some((address, subnet)) =
                bridge_endpoint_address(runtime, bridge_subnets, network_name, endpoint)
            {
                bridge_addresses.push((address, Some(subnet)));
            }
            continue;
        };

        let mac_address = match lan_type {
            ContainerNetworkType::MacVlan => {
                endpoint_mac(endpoint, AttributeSource::HypervisorConfig)
            }
            ContainerNetworkType::IpVlan => None,
        };
        let ips = [&endpoint.ip_address, &endpoint.global_ipv6_address]
            .into_iter()
            .filter_map(|ip| ip.as_ref()?.parse::<IpAddr>().ok());
        for ip_address in ips {
            let Some(address) = IPAddress::discovered(
                network_id,
                placement_subnets,
                ip_address,
                mac_address.clone(),
                Some(network_name.to_owned()),
                0,
            ) else {
                continue;
            };
            let subnet = placeable_subnet(placement_subnets, ip_address).cloned();
            lan_addresses.push((address, subnet));
        }
    }

    let mut addresses = lan_addresses;
    addresses.extend(bridge_addresses);
    for (position, (address, _)) in addresses.iter_mut().enumerate() {
        address.base.position = position as i32;
    }
    addresses
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::server::subnets::r#impl::base::{SubnetBase, SubnetCidr, SubnetCidrValue};
    use bollard::models::{HostConfig, NetworkSettings};

    pub fn subnet(name: &str, cidr: &str, subnet_type: SubnetType) -> Subnet {
        Subnet {
            id: Uuid::new_v4(),
            base: SubnetBase {
                name: name.to_string(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(cidr.parse().expect("valid test CIDR")),
                    AttributeSource::DaemonSelfReport,
                ),
                subnet_type,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn bridge_subnet(name: &str, cidr: &str) -> Subnet {
        subnet(name, cidr, SubnetType::DockerBridge)
    }

    pub fn endpoint(ip: &str, ipv6: Option<&str>, mac: Option<&str>) -> EndpointSettings {
        EndpointSettings {
            ip_address: Some(ip.to_string()),
            global_ipv6_address: ipv6.map(str::to_string),
            mac_address: mac.map(str::to_string),
            ..Default::default()
        }
    }

    pub fn container(
        id: &str,
        name: &str,
        networks: Vec<(&str, EndpointSettings)>,
    ) -> ContainerInspectResponse {
        ContainerInspectResponse {
            id: Some(id.to_string()),
            name: Some(format!("/{name}")),
            host_config: Some(HostConfig {
                network_mode: Some(networks.first().map_or("default", |(n, _)| n).to_string()),
                ..Default::default()
            }),
            network_settings: Some(NetworkSettings {
                networks: Some(
                    networks
                        .into_iter()
                        .map(|(n, e)| (n.to_string(), e))
                        .collect(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// The hazard binding-by-identity removes. A container bridge is host-scoped, so two daemons
    /// legitimately hold the same `172.17.0.0/16`: containment cannot tell them apart, and the
    /// address is identical on both. Only the network the runtime named can.
    #[test]
    fn an_endpoint_binds_to_its_own_daemons_bridge_when_two_share_a_cidr() {
        let ours = bridge_subnet("scanopy_default", "172.17.0.0/16");
        let theirs = bridge_subnet("other_default", "172.17.0.0/16");
        let subnets = vec![theirs.clone(), ours.clone()];

        let bound = subnet_for_container_network(
            &subnets,
            "scanopy_default",
            "172.17.0.2".parse().unwrap(),
        )
        .expect("the named network");

        assert_eq!(bound.id, ours.id);
    }

    /// A Docker network with several IPAM pools yields several subnets sharing a name, so
    /// containment chooses among *that network's* subnets, never outside them.
    #[test]
    fn containment_only_chooses_among_the_named_networks_own_pools() {
        let v4 = bridge_subnet("dual", "172.20.0.0/16");
        let v6 = bridge_subnet("dual", "fd00:dead:beef::/64");
        let unrelated = bridge_subnet("other", "10.0.0.0/8");
        let subnets = vec![unrelated, v4.clone(), v6.clone()];

        let bound =
            subnet_for_container_network(&subnets, "dual", "fd00:dead:beef::2".parse().unwrap())
                .expect("the named network");

        assert_eq!(bound.id, v6.id);
    }

    /// An endpoint on a network no bridge subnet was created for has nowhere to go, and saying so
    /// is better than filing it under whichever range happens to contain the address.
    #[test]
    fn an_endpoint_on_an_unknown_network_binds_to_nothing() {
        let subnets = vec![bridge_subnet("scanopy_default", "172.17.0.0/16")];

        assert!(
            subnet_for_container_network(
                &subnets,
                "a_network_we_never_listed",
                "172.17.0.2".parse().unwrap(),
            )
            .is_none()
        );
    }

    /// The runtime's network listing decides which endpoints put a container on the LAN; a
    /// bridge-only container has none.
    #[test]
    fn only_a_macvlan_or_ipvlan_endpoint_makes_a_container_a_lan_host() {
        let lan = vec![
            subnet("lan", "192.168.1.0/24", SubnetType::MacVlan),
            subnet("iot", "192.168.20.0/24", SubnetType::IpVlan),
        ];

        let bridge_only = container(
            "a",
            "web",
            vec![("app_default", endpoint("172.18.0.2", None, None))],
        );
        assert_eq!(container_lan_network_type(&bridge_only, &lan), None);

        let macvlan = container(
            "b",
            "pihole",
            vec![
                ("app_default", endpoint("172.18.0.3", None, None)),
                ("lan", endpoint("192.168.1.53", None, None)),
            ],
        );
        assert_eq!(
            container_lan_network_type(&macvlan, &lan),
            Some(ContainerNetworkType::MacVlan)
        );

        let ipvlan = container(
            "c",
            "sensor",
            vec![("iot", endpoint("192.168.20.9", None, None))],
        );
        assert_eq!(
            container_lan_network_type(&ipvlan, &lan),
            Some(ContainerNetworkType::IpVlan)
        );
    }

    /// Bridge containers keep today's shape: their own bridge endpoint first, then the runtime
    /// host's addresses, where their published ports are reached.
    #[test]
    fn a_bridge_container_keeps_its_endpoint_and_the_host_addresses() {
        let bridge = bridge_subnet("app_default", "172.18.0.0/16");
        let lan = subnet("lan", "192.168.1.0/24", SubnetType::Lan);
        let containers = vec![(
            container(
                "a",
                "web",
                vec![("app_default", endpoint("172.18.0.2", None, None))],
            ),
            ContainerSummary::default(),
        )];
        let mut host_interfaces = vec![IPAddress::new(IPAddressBase {
            ip_address: "192.168.1.10".parse().unwrap(),
            ..Default::default()
        })];

        let interfaces = container_interfaces(
            ContainerRuntime::Docker,
            &containers,
            std::slice::from_ref(&bridge),
            std::slice::from_ref(&lan),
            &mut host_interfaces,
        );

        let placed: Vec<(IpAddr, Uuid)> = interfaces["a"]
            .iter()
            .map(|(i, _)| (i.base.ip_address, i.base.subnet_id))
            .collect();
        assert_eq!(
            placed,
            vec![
                ("172.18.0.2".parse().unwrap(), bridge.id),
                ("192.168.1.10".parse().unwrap(), lan.id),
            ]
        );
    }

    /// A container on a bridge and a macvlan network carries both addresses as a host: the LAN
    /// one first, placed in the network's own subnet, and the bridge one on its bridge.
    #[test]
    fn a_bridge_and_macvlan_container_carries_both_addresses() {
        let bridge = bridge_subnet("app_default", "172.18.0.0/16");
        let lan_network = subnet("lan", "192.168.1.0/24", SubnetType::MacVlan);
        let known_lan = subnet("Office LAN", "192.168.1.0/24", SubnetType::Lan);
        let known_v6 = subnet("Office LAN v6", "2001:db8:1::/64", SubnetType::Lan);
        let network_id = Uuid::new_v4();
        let pihole = container(
            "b",
            "pihole",
            vec![
                (
                    "app_default",
                    endpoint("172.18.0.3", None, Some("02:42:ac:12:00:03")),
                ),
                (
                    "lan",
                    endpoint(
                        "192.168.1.53",
                        Some("2001:db8:1::53"),
                        Some("02:42:c0:a8:01:35"),
                    ),
                ),
            ],
        );

        let addresses = container_host_addresses(
            ContainerRuntime::Docker,
            &pihole,
            std::slice::from_ref(&lan_network),
            std::slice::from_ref(&bridge),
            &[known_lan.clone(), known_v6.clone()],
            network_id,
        );

        let placed: Vec<(IpAddr, Uuid, Option<&str>, i32)> = addresses
            .iter()
            .map(|(i, _)| {
                (
                    i.base.ip_address,
                    i.base.subnet_id,
                    i.base.name.as_deref(),
                    i.base.position,
                )
            })
            .collect();
        assert_eq!(
            placed,
            vec![
                (
                    "192.168.1.53".parse().unwrap(),
                    known_lan.id,
                    Some("lan"),
                    0
                ),
                (
                    "2001:db8:1::53".parse().unwrap(),
                    known_v6.id,
                    Some("lan"),
                    1
                ),
                (
                    "172.18.0.3".parse().unwrap(),
                    bridge.id,
                    Some("app_default"),
                    2
                ),
            ]
        );

        // The macvlan MAC anchors identity like a hypervisor's guest NIC; the bridge MAC stays a
        // report, since a bridge is private to its host.
        let sources: Vec<Option<AttributeSource>> = addresses
            .iter()
            .map(|(i, _)| i.base.mac_address.as_ref().map(|m| m.source()))
            .collect();
        assert_eq!(
            sources,
            vec![
                Some(AttributeSource::HypervisorConfig),
                Some(AttributeSource::HypervisorConfig),
                Some(AttributeSource::Probe(
                    ContainerRuntime::Docker.client_probe()
                )),
            ]
        );
    }

    /// An ipvlan endpoint shares its parent interface's MAC, so recording it would merge the
    /// container into the host it runs on. An address nothing holds is left for the server.
    #[test]
    fn an_ipvlan_address_carries_no_mac_and_an_unheld_one_no_subnet() {
        let lan_network = subnet("iot", "192.168.20.0/24", SubnetType::IpVlan);
        let sensor = container(
            "c",
            "sensor",
            vec![(
                "iot",
                endpoint("192.168.20.9", None, Some("bc:24:11:00:00:01")),
            )],
        );

        let addresses = container_host_addresses(
            ContainerRuntime::Podman,
            &sensor,
            std::slice::from_ref(&lan_network),
            &[],
            &[],
            Uuid::new_v4(),
        );

        assert_eq!(addresses.len(), 1);
        let (address, subnet) = &addresses[0];
        assert!(address.base.mac_address.is_none());
        assert_eq!(address.base.subnet_id, Uuid::nil());
        assert!(subnet.is_none());
    }
}
