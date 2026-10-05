//! Coverage for merging the hosts one discovery payload proves are the same device, end to end
//! through `discover_host` and `consolidate_hosts` on a testcontainers Postgres.
//!
//! The rule itself (`hosts_proven_same_device`) is pure and unit-tested beside it. These cover
//! what only the database shows: that the merge happens on the real write path, that it carries
//! every row the merged-away host held instead of cascading them away, and that the cases the rule
//! refuses leave every host in place.

use std::net::IpAddr;

use chrono::Utc;
use mac_address::MacAddress;
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::dependencies::r#impl::base::DependencyMembers;
use crate::server::hosts::r#impl::api::HostResponse;
use crate::server::hosts::r#impl::base::{Host, HostBase};
use crate::server::hosts::r#impl::virtualization::{
    ContainerHostVirtualization, ContainerNetworkType, HostVirtualization,
    NetworkIdentityVirtualization,
};
use crate::server::interface_neighbors::r#impl::base::Neighbor;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase, InterfaceDataComplete};
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue,
};
use crate::server::networks::r#impl::{Network, NetworkBase};
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::base::{Service, ServiceBase};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::services::factory::ServiceFactory;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase, SubnetCidr, SubnetCidrValue};
use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::tags::r#impl::base::{Tag, TagBase};

use super::{daemon, dependency, organization, test_services, user};

/// VM 103's two NICs.
fn ens18() -> MacAddress {
    "02:1c:65:32:e5:dd".parse().unwrap()
}
fn ens19() -> MacAddress {
    "bc:24:11:9b:24:86".parse().unwrap()
}

/// Seed an organization and a network with a LAN and a Docker bridge subnet, and hand back the
/// wired-up services. The container is returned so the caller keeps Postgres alive.
macro_rules! harness {
    ($storage:ident, $services:ident, $lab:ident, $container:ident) => {
        let ($storage, $services, $container) = test_services().await;
        let org = organization();
        $storage.organizations.create(&org).await.unwrap();
        let network = $services
            .network_service
            .create(
                Network::new(NetworkBase::new(org.id)),
                AuthenticatedEntity::System,
            )
            .await
            .unwrap();
        let lan = create_subnet(&$services, network.id, "192.168.4.0/22", SubnetType::Lan).await;
        let bridge = create_subnet(
            &$services,
            network.id,
            "172.17.0.0/16",
            SubnetType::DockerBridge,
        )
        .await;
        let $lab = Lab {
            organization_id: org.id,
            network_id: network.id,
            lan,
            bridge,
        };
    };
}

struct Lab {
    organization_id: Uuid,
    network_id: Uuid,
    lan: Uuid,
    bridge: Uuid,
}

async fn create_subnet(
    services: &ServiceFactory,
    network_id: Uuid,
    cidr: &str,
    subnet_type: SubnetType,
) -> Uuid {
    services
        .subnet_service
        .create(
            Subnet::new(SubnetBase {
                name: cidr.to_string(),
                network_id,
                cidr: SubnetCidr::new(
                    SubnetCidrValue(cidr.parse().unwrap()),
                    AttributeSource::DaemonSelfReport,
                ),
                subnet_type,
                source: EntitySource::Discovery,
                ..Default::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap()
        .id
}

fn address(
    lab: &Lab,
    subnet_id: Uuid,
    ip: &str,
    mac: Option<(MacAddress, AttributeSource)>,
) -> IPAddress {
    IPAddress::new(IPAddressBase {
        network_id: lab.network_id,
        subnet_id,
        ip_address: ip.parse::<IpAddr>().unwrap(),
        mac_address: mac.map(|(m, s)| MacEvidence::new(MacEvidenceValue(m), s)),
        ..Default::default()
    })
}
fn arp(lab: &Lab, ip: &str, mac: MacAddress) -> IPAddress {
    address(lab, lab.lan, ip, Some((mac, AttributeSource::ArpReply)))
}
fn config(lab: &Lab, ip: &str, mac: MacAddress) -> IPAddress {
    address(
        lab,
        lab.lan,
        ip,
        Some((mac, AttributeSource::HypervisorConfig)),
    )
}
fn snmp(lab: &Lab, ip: &str, mac: MacAddress) -> IPAddress {
    address(
        lab,
        lab.lan,
        ip,
        Some((mac, AttributeSource::Probe(ClientProbe::Snmp))),
    )
}

fn service(lab: &Lab, definition: &str) -> Service {
    Service::new(ServiceBase {
        name: definition.to_string(),
        network_id: lab.network_id,
        service_definition: ServiceDefinitionRegistry::find_by_id(definition)
            .unwrap_or_else(|| panic!("{definition} is registered")),
        source: EntitySource::Discovery,
        ..Default::default()
    })
}

fn interface(lab: &Lab, name: &str, mac: MacAddress) -> Interface {
    Interface::new(InterfaceBase {
        network_id: lab.network_id,
        if_name: Some(name.to_string()),
        mac_address: Some(MacEvidence::new(
            MacEvidenceValue(mac),
            AttributeSource::Probe(ClientProbe::Snmp),
        )),
        ..Default::default()
    })
}

/// One discovery submission, as a daemon sends it.
async fn discover(
    services: &ServiceFactory,
    lab: &Lab,
    host: HostBase,
    ip_addresses: Vec<IPAddress>,
    host_services: Vec<Service>,
    interfaces: Vec<Interface>,
) -> HostResponse {
    services
        .host_service
        .discover_host(
            Host::new(HostBase {
                network_id: lab.network_id,
                source: EntitySource::Discovery,
                ..host
            }),
            ip_addresses,
            vec![],
            host_services,
            interfaces,
            vec![],
            false,
            InterfaceDataComplete::none(),
            None,
            AuthenticatedEntity::System,
            None,
        )
        .await
        .expect("the submission persists")
        .host
}

async fn live_hosts(services: &ServiceFactory, lab: &Lab) -> Vec<Host> {
    services
        .host_service
        .get_all(StorableFilter::<Host>::new_from_network_ids(&[lab.network_id]).live())
        .await
        .unwrap()
}

async fn host_ips(services: &ServiceFactory, host_id: Uuid) -> Vec<IpAddr> {
    let mut ips: Vec<IpAddr> = services
        .ip_address_service
        .get_for_host(&host_id)
        .await
        .unwrap()
        .into_iter()
        .map(|a| a.base.ip_address)
        .collect();
    ips.sort();
    ips
}

/// The VM 103 split, replayed in the lab's order, with everything a merge could lose hung off the
/// host that gets merged away.
///
/// The sweep found .63 first (ARP flux showed it behind ens18). The Docker runtime then reported
/// .126 (behind ens19) with its Docker and Portainer services, a container guest, an interface
/// with an adjacency, a switch naming it as a neighbour, a tag, and a Portainer that also runs on
/// the first host and sits in a dependency. VM 103's Proxmox payload then names both addresses
/// with its config MACs.
#[tokio::test]
async fn a_payload_naming_both_halves_of_a_split_vm_merges_them_and_keeps_everything() {
    harness!(storage, services, lab, _container);

    let h1 = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.63", ens18())],
        vec![service(&lab, "Portainer")],
        vec![],
    )
    .await;

    let h2 = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            arp(&lab, "192.168.4.126", ens19()),
            address(
                &lab,
                lab.bridge,
                "172.17.0.2",
                Some((
                    "f6:af:f2:ff:a5:ff".parse().unwrap(),
                    AttributeSource::Probe(ClientProbe::Docker),
                )),
            ),
        ],
        vec![service(&lab, "Docker"), service(&lab, "Portainer")],
        vec![interface(
            &lab,
            "eth9",
            "aa:00:00:00:00:99".parse().unwrap(),
        )],
    )
    .await;
    assert_ne!(h1.id, h2.id, "the lab's split reproduces");
    let docker_id = h2
        .services
        .iter()
        .find(|s| s.base.name == "Docker")
        .unwrap()
        .id;
    let h1_portainer = h1
        .services
        .iter()
        .find(|s| s.base.name == "Portainer")
        .unwrap()
        .id;
    let h2_portainer = h2
        .services
        .iter()
        .find(|s| s.base.name == "Portainer")
        .unwrap()
        .id;
    let h2_interface = h2.interfaces[0].id;

    // A container guest of H2's Docker.
    let guest = discover(
        &services,
        &lab,
        HostBase {
            virtualization_service_id: Some(docker_id),
            ..Default::default()
        },
        vec![arp(
            &lab,
            "192.168.7.230",
            "26:b6:d3:de:10:b5".parse().unwrap(),
        )],
        vec![],
        vec![],
    )
    .await;
    assert_eq!(guest.virtualization_service_id, Some(docker_id));

    // A switch whose port names H2 as its neighbour, and H2's interface naming the switch.
    let switch = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![snmp(
            &lab,
            "192.168.4.2",
            "00:1b:54:00:00:01".parse().unwrap(),
        )],
        vec![],
        vec![interface(
            &lab,
            "ge-0/0/1",
            "00:1b:54:00:00:02".parse().unwrap(),
        )],
    )
    .await;
    let switch_port = switch.interfaces[0].id;
    services
        .interface_neighbor_service
        .reconcile_interface_neighbors(
            lab.network_id,
            switch_port,
            &[(Neighbor::Host(h2.id), None)],
            Utc::now(),
            None,
        )
        .await
        .unwrap();
    services
        .interface_neighbor_service
        .reconcile_interface_neighbors(
            lab.network_id,
            h2_interface,
            &[(Neighbor::Host(switch.id), None)],
            Utc::now(),
            None,
        )
        .await
        .unwrap();

    // A tag on H2, and a dependency through H2's duplicate Portainer.
    let tag = Tag {
        id: Uuid::new_v4(),
        base: TagBase {
            name: "lab".to_string(),
            organization_id: lab.organization_id,
            ..Default::default()
        },
        ..Default::default()
    };
    storage.tags.create(&tag).await.unwrap();
    services
        .entity_tag_service
        .add_tag(
            h2.id,
            EntityDiscriminants::Host,
            tag.id,
            lab.organization_id,
        )
        .await
        .unwrap();
    let mut chain = dependency(&lab.network_id);
    chain.base.members = DependencyMembers::Services {
        service_ids: vec![h2_portainer, docker_id],
    };
    let chain = services
        .dependency_service
        .create(chain, AuthenticatedEntity::System)
        .await
        .unwrap();

    let h2_last_seen = h2.last_seen_at;
    let survivor = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            config(&lab, "192.168.4.126", ens18()),
            config(&lab, "192.168.4.63", ens19()),
        ],
        vec![],
        vec![],
    )
    .await;

    assert_eq!(survivor.id, h1.id, "the older record survives");
    assert!(
        services
            .host_service
            .get_by_id(&h2.id)
            .await
            .unwrap()
            .is_none(),
        "the merged-away record is gone"
    );
    let addresses = host_ips(&services, h1.id).await;
    assert_eq!(
        addresses,
        vec![
            "172.17.0.2".parse::<IpAddr>().unwrap(),
            "192.168.4.63".parse().unwrap(),
            "192.168.4.126".parse().unwrap(),
        ],
        "each address once, on the survivor"
    );

    let survivor_services = services
        .service_service
        .get_all(StorableFilter::<Service>::new_from_host_ids(&[h1.id]).live())
        .await
        .unwrap();
    assert!(
        survivor_services.iter().any(|s| s.id == docker_id),
        "the Docker service moved with its id"
    );
    let guest_now = services
        .host_service
        .get_by_id(&guest.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(guest_now.base.virtualization_service_id, Some(docker_id));

    let survivor_interfaces = services
        .interface_service
        .get_for_host(&h1.id)
        .await
        .unwrap();
    assert!(
        survivor_interfaces.iter().any(|i| i.id == h2_interface),
        "the interface moved with its id"
    );
    let own_adjacency = services
        .interface_neighbor_service
        .resolved_for_interface(&h2_interface)
        .await
        .unwrap();
    assert_eq!(own_adjacency.len(), 1, "its adjacency moved with it");
    let switch_adjacency = services
        .interface_neighbor_service
        .resolved_for_interface(&switch_port)
        .await
        .unwrap();
    assert_eq!(
        switch_adjacency
            .iter()
            .map(|r| r.neighbor)
            .collect::<Vec<_>>(),
        vec![Neighbor::Host(h1.id)],
        "the switch now names the survivor"
    );

    let tags = services
        .entity_tag_service
        .get_tags(&h1.id, &EntityDiscriminants::Host)
        .await
        .unwrap();
    assert_eq!(tags, vec![tag.id], "the tag carried across");

    let chain = services
        .dependency_service
        .get_by_id(&chain.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        chain.base.members,
        DependencyMembers::Services {
            service_ids: vec![h1_portainer, docker_id]
        },
        "the dependency names the surviving Portainer"
    );

    let survivor_host = services
        .host_service
        .get_by_id(&h1.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        survivor_host.last_seen_at > h2_last_seen,
        "the merge never rewinds the survivor's last-seen time"
    );

    // Sent again, nothing more changes.
    let again = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            config(&lab, "192.168.4.126", ens18()),
            config(&lab, "192.168.4.63", ens19()),
        ],
        vec![],
        vec![],
    )
    .await;
    assert_eq!(again.id, h1.id);
    assert_eq!(host_ips(&services, h1.id).await, addresses);
    assert_eq!(
        live_hosts(&services, &lab).await.len(),
        3,
        "survivor, guest and switch"
    );
}

/// When both halves run a Docker runtime, the merged-away one is a duplicate; the guests it ran
/// move to the survivor's runtime rather than losing their link.
#[tokio::test]
async fn a_duplicate_runtimes_guests_move_to_the_surviving_runtime() {
    harness!(_storage, services, lab, _container);

    let h1 = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.63", ens18())],
        vec![service(&lab, "Docker")],
        vec![],
    )
    .await;
    let h2 = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.126", ens19())],
        vec![service(&lab, "Docker")],
        vec![],
    )
    .await;
    let kept = h1.services[0].id;
    let duplicate = h2.services[0].id;
    let guest = discover(
        &services,
        &lab,
        HostBase {
            virtualization_service_id: Some(duplicate),
            ..Default::default()
        },
        vec![arp(
            &lab,
            "192.168.7.230",
            "26:b6:d3:de:10:b5".parse().unwrap(),
        )],
        vec![],
        vec![],
    )
    .await;

    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            config(&lab, "192.168.4.126", ens18()),
            config(&lab, "192.168.4.63", ens19()),
        ],
        vec![],
        vec![],
    )
    .await;

    let guest_now = services
        .host_service
        .get_by_id(&guest.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(guest_now.base.virtualization_service_id, Some(kept));
}

/// A switch whose SVIs each answer with their own MAC: the sweep finds three unrelated hosts, and
/// the switch's own SNMP payload, listing all three, merges them.
#[tokio::test]
async fn a_switchs_svis_found_apart_merge_on_its_own_payload() {
    harness!(_storage, services, lab, _container);
    let svis: [(&str, MacAddress); 3] = [
        ("192.168.4.10", "00:1b:54:00:00:01".parse().unwrap()),
        ("192.168.5.10", "00:1b:54:00:00:02".parse().unwrap()),
        ("192.168.6.10", "00:1b:54:00:00:03".parse().unwrap()),
    ];

    let mut swept = Vec::new();
    for (ip, mac) in svis {
        swept.push(
            discover(
                &services,
                &lab,
                HostBase::default(),
                vec![arp(&lab, ip, mac)],
                vec![],
                vec![],
            )
            .await
            .id,
        );
    }
    assert_eq!(
        live_hosts(&services, &lab).await.len(),
        3,
        "the sweep finds three hosts"
    );

    let merged = discover(
        &services,
        &lab,
        HostBase::default(),
        svis.iter().map(|(ip, mac)| snmp(&lab, ip, *mac)).collect(),
        vec![],
        vec![],
    )
    .await;
    assert_eq!(merged.id, swept[0]);
    assert_eq!(live_hosts(&services, &lab).await.len(), 1);
    assert_eq!(host_ips(&services, swept[0]).await.len(), 3);
}

/// keepalived with no virtual MAC: node B's payload names its own address and the VIP, which
/// node A also holds. A holds an address B's payload does not claim, so A is not B and both
/// records stay.
#[tokio::test]
async fn an_ha_pair_sharing_a_floating_vip_is_never_merged() {
    harness!(_storage, services, lab, _container);
    let (mac_a, mac_b): (MacAddress, MacAddress) = (
        "aa:00:00:00:00:01".parse().unwrap(),
        "aa:00:00:00:00:02".parse().unwrap(),
    );

    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            snmp(&lab, "192.168.4.11", mac_a),
            snmp(&lab, "192.168.4.10", mac_a),
        ],
        vec![],
        vec![],
    )
    .await;
    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![snmp(&lab, "192.168.4.12", mac_b)],
        vec![],
        vec![],
    )
    .await;
    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            snmp(&lab, "192.168.4.12", mac_b),
            snmp(&lab, "192.168.4.10", mac_b),
        ],
        vec![],
        vec![],
    )
    .await;

    assert_eq!(live_hosts(&services, &lab).await.len(), 2);
}

/// A stale record of an address's previous DHCP holder carries that device's MAC, which the new
/// holder's payload does not, so it is left alone.
#[tokio::test]
async fn a_stale_record_of_a_reused_address_is_not_merged() {
    harness!(_storage, services, lab, _container);
    let (old, new_a, new_b): (MacAddress, MacAddress, MacAddress) = (
        "aa:00:00:00:00:09".parse().unwrap(),
        "bc:24:11:00:00:01".parse().unwrap(),
        "bc:24:11:00:00:02".parse().unwrap(),
    );

    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.50", old)],
        vec![],
        vec![],
    )
    .await;
    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.51", new_b)],
        vec![],
        vec![],
    )
    .await;
    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            config(&lab, "192.168.4.50", new_a),
            config(&lab, "192.168.4.51", new_b),
        ],
        vec![],
        vec![],
    )
    .await;

    assert_eq!(live_hosts(&services, &lab).await.len(), 2);
}

/// A controller reporting a client's MAC is a third party's say-so; its payload proves no merge.
#[tokio::test]
async fn a_controller_reported_payload_merges_nothing() {
    harness!(_storage, services, lab, _container);
    let controller = |ip: &str, mac: MacAddress| {
        address(
            &lab,
            lab.lan,
            ip,
            Some((mac, AttributeSource::Probe(ClientProbe::UnifiController))),
        )
    };

    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.63", ens18())],
        vec![],
        vec![],
    )
    .await;
    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.126", ens19())],
        vec![],
        vec![],
    )
    .await;
    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            controller("192.168.4.126", ens18()),
            controller("192.168.4.63", ens19()),
        ],
        vec![],
        vec![],
    )
    .await;

    assert_eq!(live_hosts(&services, &lab).await.len(), 2);
}

/// A host carrying a daemon is the record that daemon reports as; discovery never merges it away,
/// and the submission that proves it the same device still succeeds.
#[tokio::test]
async fn a_daemon_host_is_never_merged_away() {
    harness!(storage, services, lab, _container);

    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.63", ens18())],
        vec![],
        vec![],
    )
    .await;
    let h2 = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.126", ens19())],
        vec![],
        vec![],
    )
    .await;
    let owner = user(&lab.organization_id);
    storage.users.create(&owner).await.unwrap();
    let mut d = daemon(&lab.network_id, &h2.id);
    d.base.user_id = owner.id;
    storage.daemons.create(&d).await.unwrap();

    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            config(&lab, "192.168.4.126", ens18()),
            config(&lab, "192.168.4.63", ens19()),
        ],
        vec![],
        vec![],
    )
    .await;

    assert!(
        services
            .host_service
            .get_by_id(&h2.id)
            .await
            .unwrap()
            .is_some()
    );
}

/// The manual merge endpoint's function keeps what it used to cascade away: interfaces, their
/// adjacencies, and tags.
#[tokio::test]
async fn a_manual_merge_keeps_interfaces_adjacencies_and_tags() {
    harness!(storage, services, lab, _container);

    let a = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(
            &lab,
            "192.168.4.20",
            "aa:00:00:00:00:20".parse().unwrap(),
        )],
        vec![],
        vec![],
    )
    .await;
    let b = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(
            &lab,
            "192.168.4.21",
            "aa:00:00:00:00:21".parse().unwrap(),
        )],
        vec![],
        vec![interface(
            &lab,
            "eth1",
            "aa:00:00:00:00:21".parse().unwrap(),
        )],
    )
    .await;
    let b_interface = b.interfaces[0].id;
    services
        .interface_neighbor_service
        .reconcile_interface_neighbors(
            lab.network_id,
            b_interface,
            &[(Neighbor::Host(a.id), None)],
            Utc::now(),
            None,
        )
        .await
        .unwrap();
    let tag = Tag {
        id: Uuid::new_v4(),
        base: TagBase {
            name: "kept".to_string(),
            organization_id: lab.organization_id,
            ..Default::default()
        },
        ..Default::default()
    };
    storage.tags.create(&tag).await.unwrap();
    services
        .entity_tag_service
        .add_tag(b.id, EntityDiscriminants::Host, tag.id, lab.organization_id)
        .await
        .unwrap();

    let a_host = services
        .host_service
        .get_by_id(&a.id)
        .await
        .unwrap()
        .unwrap();
    let b_host = services
        .host_service
        .get_by_id(&b.id)
        .await
        .unwrap()
        .unwrap();
    services
        .host_service
        .consolidate_hosts(a_host, b_host, AuthenticatedEntity::System)
        .await
        .unwrap();

    let interfaces = services
        .interface_service
        .get_for_host(&a.id)
        .await
        .unwrap();
    assert!(interfaces.iter().any(|i| i.id == b_interface));
    // B's interface named A; on A that would be A naming itself, so it is dropped.
    let adjacency = services
        .interface_neighbor_service
        .resolved_for_interface(&b_interface)
        .await
        .unwrap();
    assert!(adjacency.is_empty());
    let tags = services
        .entity_tag_service
        .get_tags(&a.id, &EntityDiscriminants::Host)
        .await
        .unwrap();
    assert_eq!(tags, vec![tag.id]);
}

/// A network identity names the guest interface that presents it, by stored id; a rescan keeps
/// the same id, and deleting the interface clears the reference without touching the host.
#[tokio::test]
async fn a_network_identity_references_its_presenting_interface() {
    harness!(_storage, services, lab, _container);
    let mv: MacAddress = "8e:12:00:00:00:04".parse().unwrap();

    let guest_submission = || {
        (
            vec![config(&lab, "192.168.4.21", ens18())],
            vec![service(&lab, "Network Identities")],
            vec![interface(&lab, "mv-snmp4", mv)],
        )
    };
    let (ips, svcs, ifs) = guest_submission();
    let guest = discover(&services, &lab, HostBase::default(), ips, svcs, ifs).await;
    let owner = guest.services[0].id;
    let presenting = guest.interfaces[0].id;

    let identity = discover(
        &services,
        &lab,
        HostBase {
            virtualization_metadata: Some(HostVirtualization::NetworkIdentity(
                NetworkIdentityVirtualization {},
            )),
            virtualization_service_id: Some(owner),
            virtualization_interface_id: Some(presenting),
            ..Default::default()
        },
        vec![address(
            &lab,
            lab.lan,
            "192.168.7.196",
            Some((mv, AttributeSource::Probe(ClientProbe::Proxmox))),
        )],
        vec![],
        vec![],
    )
    .await;
    assert_eq!(identity.virtualization_interface_id, Some(presenting));

    let (ips, svcs, ifs) = guest_submission();
    let rescan = discover(&services, &lab, HostBase::default(), ips, svcs, ifs).await;
    assert_eq!(rescan.id, guest.id);
    assert!(
        rescan.interfaces.iter().any(|i| i.id == presenting),
        "the interface keeps its id"
    );

    services
        .interface_service
        .delete(&presenting, AuthenticatedEntity::System)
        .await
        .unwrap();
    let identity_now = services
        .host_service
        .get_by_id(&identity.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(identity_now.base.virtualization_interface_id, None);
    assert_eq!(identity_now.base.virtualization_service_id, Some(owner));
}

/// A presenting interface that is not the owner's: discovery drops it, the API rejects it.
#[tokio::test]
async fn a_presenting_interface_on_another_host_is_refused() {
    harness!(_storage, services, lab, _container);

    let guest = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![config(&lab, "192.168.4.21", ens18())],
        vec![service(&lab, "Network Identities")],
        vec![],
    )
    .await;
    let other = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.30", ens19())],
        vec![],
        vec![interface(&lab, "eth0", ens19())],
    )
    .await;
    let owner = guest.services[0].id;
    let foreign_interface = other.interfaces[0].id;

    let identity = discover(
        &services,
        &lab,
        HostBase {
            virtualization_service_id: Some(owner),
            virtualization_interface_id: Some(foreign_interface),
            ..Default::default()
        },
        vec![arp(
            &lab,
            "192.168.7.196",
            "8e:12:00:00:00:04".parse().unwrap(),
        )],
        vec![],
        vec![],
    )
    .await;
    assert_eq!(identity.virtualization_service_id, Some(owner));
    assert_eq!(identity.virtualization_interface_id, None);

    let mut request = crate::server::shared::types::examples::create_host_request();
    request.network_id = lab.network_id;
    request.ip_addresses = vec![];
    request.ports = vec![];
    request.services = vec![];
    request.interfaces = vec![];
    request.virtualization_service_id = Some(owner);
    request.virtualization_interface_id = Some(foreign_interface);
    let rejected = services
        .host_service
        .create_from_request(request, AuthenticatedEntity::System)
        .await;
    assert!(rejected.is_err());
}

/// The same MAC at a second address in the same range is recorded beside the first, not folded
/// into it: a two-NIC host on one segment answers for both addresses with either NIC's MAC (ARP
/// flux), and folding lost the second address on every scan.
#[tokio::test]
async fn a_second_address_behind_the_same_mac_is_kept() {
    harness!(_storage, services, lab, _container);

    let first = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.63", ens19())],
        vec![],
        vec![],
    )
    .await;
    let second = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.126", ens19())],
        vec![],
        vec![],
    )
    .await;

    assert_eq!(second.id, first.id, "the MAC still identifies the host");
    assert_eq!(
        host_ips(&services, first.id).await,
        vec![
            "192.168.4.63".parse::<IpAddr>().unwrap(),
            "192.168.4.126".parse().unwrap()
        ]
    );
}

/// A NIC whose address moved to a range its old subnet does not hold is still the same row,
/// re-homed rather than duplicated.
#[tokio::test]
async fn a_nic_that_moved_to_another_range_is_rehomed() {
    harness!(_storage, services, lab, _container);
    let other = create_subnet(&services, lab.network_id, "10.20.0.0/24", SubnetType::Lan).await;
    let nic: MacAddress = "bc:24:11:00:00:70".parse().unwrap();

    let first = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.70", nic)],
        vec![],
        vec![],
    )
    .await;
    let moved = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![address(
            &lab,
            other,
            "10.20.0.5",
            Some((nic, AttributeSource::ArpReply)),
        )],
        vec![],
        vec![],
    )
    .await;

    assert_eq!(moved.id, first.id);
    assert_eq!(
        host_ips(&services, first.id).await,
        vec!["10.20.0.5".parse::<IpAddr>().unwrap()]
    );
}

/// The runtime host as its scan submits it: its LAN address, the API port, the runtime service
/// bound to that port, and any `extra` services (already bound by the caller).
fn runtime_payload(
    lab: &Lab,
    extra: impl FnOnce(&IPAddress, &crate::server::ports::r#impl::base::Port) -> Vec<Service>,
) -> (
    Vec<IPAddress>,
    Vec<crate::server::ports::r#impl::base::Port>,
    Vec<Service>,
) {
    use crate::server::bindings::r#impl::base::Binding;
    use crate::server::ports::r#impl::base::{Port, PortBase, PortType};

    let ip = arp(lab, "192.168.4.126", ens18());
    let port = Port::new(PortBase {
        port_type: PortType::new_tcp(2375),
        host_id: Uuid::nil(),
        network_id: lab.network_id,
    });
    let mut runtime = service(lab, "Docker");
    runtime.base.bindings = vec![Binding::new_port_serviceless(port.id, Some(ip.id))];
    let mut services = vec![runtime];
    services.extend(extra(&ip, &port));
    (vec![ip], vec![port], services)
}

async fn discover_with_ports(
    services: &ServiceFactory,
    lab: &Lab,
    (ip_addresses, ports, host_services): (
        Vec<IPAddress>,
        Vec<crate::server::ports::r#impl::base::Port>,
        Vec<Service>,
    ),
) -> HostResponse {
    services
        .host_service
        .discover_host(
            Host::new(HostBase {
                network_id: lab.network_id,
                source: EntitySource::Discovery,
                ..Default::default()
            }),
            ip_addresses,
            ports,
            host_services,
            vec![],
            vec![],
            false,
            InterfaceDataComplete::none(),
            None,
            AuthenticatedEntity::System,
            None,
        )
        .await
        .expect("the submission persists")
        .host
}

/// A container service owned by the runtime, bound to the runtime's API port.
fn container_on_api_port(
    lab: &Lab,
    definition: &str,
    runtime_id: Uuid,
    ip: &IPAddress,
    port: &crate::server::ports::r#impl::base::Port,
) -> Service {
    use crate::server::bindings::r#impl::base::Binding;
    use crate::server::services::r#impl::virtualization::{
        DockerVirtualization, ServiceVirtualization,
    };
    let mut proxy = service(lab, definition);
    proxy.base.name = "docker-api-proxy".to_string();
    proxy.base.virtualization_service_id = Some(runtime_id);
    proxy.base.virtualization_metadata =
        Some(ServiceVirtualization::Docker(DockerVirtualization {
            container_name: Some("docker-api-proxy".to_string()),
            container_id: Some("a27773f8c424".to_string()),
            compose_project: Some("proxy".to_string()),
        }));
    proxy.base.bindings = vec![Binding::new_port_serviceless(port.id, Some(ip.id))];
    proxy
}

async fn docker_services(services: &ServiceFactory, lab: &Lab) -> Vec<Service> {
    services
        .service_service
        .get_all(StorableFilter::<Service>::new_from_network_ids(&[lab.network_id]).live())
        .await
        .unwrap()
}

/// The lab's path: the runtime holds its API port from the host scan, and a daemon that cannot
/// identify the proxy sends it as a generic container on the same port. The container safety net
/// must not copy the proxy's identity onto the runtime.
#[tokio::test]
async fn an_unidentified_api_proxy_never_becomes_the_runtimes_identity() {
    harness!(_storage, services, lab, _container);
    discover_with_ports(&services, &lab, runtime_payload(&lab, |_, _| vec![])).await;

    let payload = runtime_payload(&lab, |_, _| vec![]);
    let runtime_id = payload.2[0].id;
    let (ips, ports, mut svcs) = payload;
    svcs.push(container_on_api_port(
        &lab,
        "Docker Container",
        runtime_id,
        &ips[0],
        &ports[0],
    ));
    discover_with_ports(&services, &lab, (ips, ports, svcs)).await;

    let all = docker_services(&services, &lab).await;
    let runtime = all
        .iter()
        .find(|s| s.base.service_definition.name() == "Docker")
        .expect("runtime stored");
    assert_eq!(runtime.base.virtualization_service_id, None);
    assert_eq!(runtime.base.virtualization_metadata, None);
}

/// The new daemon's path: the proxy arrives as the runtime's API proxy, owned by the runtime and
/// bound to the port. It takes the port from the runtime, which keeps it from earlier scans.
#[tokio::test]
async fn an_api_proxy_takes_the_port_its_runtime_is_reached_through() {
    harness!(_storage, services, lab, _container);
    discover_with_ports(&services, &lab, runtime_payload(&lab, |_, _| vec![])).await;

    let payload = runtime_payload(&lab, |_, _| vec![]);
    let runtime_id = payload.2[0].id;
    let (ips, ports, mut svcs) = payload;
    svcs.push(container_on_api_port(
        &lab,
        "Docker API Proxy",
        runtime_id,
        &ips[0],
        &ports[0],
    ));
    discover_with_ports(&services, &lab, (ips, ports, svcs)).await;

    let all = docker_services(&services, &lab).await;
    let runtime = all
        .iter()
        .find(|s| s.base.service_definition.name() == "Docker")
        .expect("runtime stored");
    let proxy = all
        .iter()
        .find(|s| s.base.service_definition.name() == "Docker API Proxy")
        .expect("proxy stored");
    assert_eq!(runtime.base.virtualization_service_id, None);
    assert_eq!(proxy.base.virtualization_service_id, Some(runtime.id));
    assert_eq!(proxy.base.name, "docker-api-proxy");
    assert!(proxy.base.bindings.iter().any(|b| b.port_id().is_some()));
    assert!(
        runtime.base.bindings.iter().all(|b| b.port_id().is_none()),
        "the runtime no longer binds the port it is reached through"
    );
}

/// A runtime already stored as its own container (the lab's state) is cleared by its next scan.
#[tokio::test]
async fn a_stored_self_owned_runtime_is_cleared_on_rescan() {
    use crate::server::services::r#impl::virtualization::{
        DockerVirtualization, ServiceVirtualization,
    };
    harness!(storage, services, lab, _container);
    discover_with_ports(&services, &lab, runtime_payload(&lab, |_, _| vec![])).await;
    let mut runtime = docker_services(&services, &lab)
        .await
        .into_iter()
        .find(|s| s.base.service_definition.name() == "Docker")
        .expect("runtime stored");
    runtime.base.virtualization_service_id = Some(runtime.id);
    runtime.base.virtualization_metadata =
        Some(ServiceVirtualization::Docker(DockerVirtualization {
            container_name: Some("docker-api-proxy".to_string()),
            container_id: Some("a27773f8c424".to_string()),
            compose_project: None,
        }));
    storage.services.update(&mut runtime).await.unwrap();

    discover_with_ports(&services, &lab, runtime_payload(&lab, |_, _| vec![])).await;

    let runtime = services
        .service_service
        .get_by_id(&runtime.id)
        .await
        .unwrap()
        .expect("runtime still stored");
    assert_eq!(runtime.base.virtualization_service_id, None);
    assert_eq!(runtime.base.virtualization_metadata, None);
}

/// The lab's ipvlan endpoint, replayed in the lab's order. VM 103's Proxmox payload lists .63 for
/// ens19. The sweep then finds .231 behind ens19's MAC, which is the ipvlan container sharing the
/// NIC, and the container scan submits .231 as a Docker ipvlan guest. The VM keeps its own two
/// addresses, and .231 is recorded as the container.
#[tokio::test]
async fn an_address_the_hypervisor_did_not_list_for_a_nic_stays_off_the_guest() {
    harness!(_storage, services, lab, _container);

    let vm = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            config(&lab, "192.168.4.126", ens18()),
            config(&lab, "192.168.4.63", ens19()),
        ],
        vec![service(&lab, "Docker")],
        vec![],
    )
    .await;
    let docker_id = vm
        .services
        .iter()
        .find(|s| s.base.name == "Docker")
        .unwrap()
        .id;

    let swept = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.7.231", ens19())],
        vec![],
        vec![],
    )
    .await;
    assert_ne!(swept.id, vm.id, "the sweep's .231 is not the VM's");

    let container = discover(
        &services,
        &lab,
        HostBase {
            virtualization_service_id: Some(docker_id),
            virtualization_metadata: Some(HostVirtualization::Docker(
                ContainerHostVirtualization {
                    container_name: Some("ipvlan-test".to_string()),
                    container_id: Some("aec2a3e9".to_string()),
                    compose_project: None,
                    network_type: ContainerNetworkType::IpVlan,
                },
            )),
            ..Default::default()
        },
        vec![address(&lab, lab.lan, "192.168.7.231", None)],
        vec![],
        vec![],
    )
    .await;

    assert_eq!(container.id, swept.id, "the container is the swept host");
    assert_eq!(container.virtualization_service_id, Some(docker_id));
    assert!(matches!(
        container.virtualization_metadata,
        Some(HostVirtualization::Docker(ContainerHostVirtualization {
            network_type: ContainerNetworkType::IpVlan,
            ..
        }))
    ));
    assert_eq!(
        host_ips(&services, vm.id).await,
        vec![
            "192.168.4.63".parse::<IpAddr>().unwrap(),
            "192.168.4.126".parse().unwrap()
        ]
    );
    assert_eq!(
        host_ips(&services, container.id).await,
        vec!["192.168.7.231".parse::<IpAddr>().unwrap()]
    );
}

/// One sweep result: an address behind `mac`, with a service bound to it on `port`.
fn swept_with_service(
    lab: &Lab,
    ip: &str,
    mac: MacAddress,
    definition: &str,
    port: u16,
) -> (
    Vec<IPAddress>,
    Vec<crate::server::ports::r#impl::base::Port>,
    Vec<Service>,
) {
    use crate::server::bindings::r#impl::base::Binding;
    use crate::server::ports::r#impl::base::{Port, PortBase, PortType};

    let address = arp(lab, ip, mac);
    let port = Port::new(PortBase {
        port_type: PortType::new_tcp(port),
        host_id: Uuid::nil(),
        network_id: lab.network_id,
    });
    let mut found = service(lab, definition);
    found.base.bindings = vec![Binding::new_port_serviceless(port.id, Some(address.id))];
    (vec![address], vec![port], vec![found])
}

async fn hosts_holding(services: &ServiceFactory, lab: &Lab, ip: &str) -> Vec<Uuid> {
    let ip: IpAddr = ip.parse().unwrap();
    let mut holders = Vec::new();
    for host in live_hosts(services, lab).await {
        if host_ips(services, host.id).await.contains(&ip) {
            holders.push(host.id);
        }
    }
    holders
}

async fn service_names_on(services: &ServiceFactory, host_id: Uuid) -> Vec<String> {
    let mut names: Vec<String> = services
        .service_service
        .get_all(StorableFilter::<Service>::new_from_host_ids(&[host_id]).live())
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.base.name)
        .collect();
    names.sort();
    names
}

/// The lab's 19:54 run, in its order: the sweep reaches .63 and .231 (both behind ens19) before
/// Proxmox reports VM 103, so .231 is kept as a second address on the .63 host, with a service the
/// container runs. Docker's report for ipvlan-test then takes .231 and that service to a host of
/// its own and leaves .63, so the VM's services found at .63 land on the .63 host, and the Proxmox
/// payload merges that host into the VM. .63 ends up on one host.
#[tokio::test]
async fn a_container_report_takes_its_address_off_a_host_holding_another_devices() {
    harness!(_storage, services, lab, _container);

    let vm = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.126", ens18())],
        vec![service(&lab, "Docker")],
        vec![],
    )
    .await;
    let docker_id = vm
        .services
        .iter()
        .find(|s| s.base.name == "Docker")
        .unwrap()
        .id;
    let split = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.63", ens19())],
        vec![],
        vec![],
    )
    .await;
    let folded = discover_with_ports(
        &services,
        &lab,
        swept_with_service(&lab, "192.168.7.231", ens19(), "Grafana", 3000),
    )
    .await;
    assert_eq!(folded.id, split.id, "the sweep keeps .231 beside .63");

    let container = discover(
        &services,
        &lab,
        HostBase {
            virtualization_service_id: Some(docker_id),
            virtualization_metadata: Some(HostVirtualization::Docker(
                ContainerHostVirtualization {
                    container_name: Some("ipvlan-test".to_string()),
                    container_id: Some("aec2a3e9".to_string()),
                    compose_project: None,
                    network_type: ContainerNetworkType::IpVlan,
                },
            )),
            ..Default::default()
        },
        vec![address(&lab, lab.lan, "192.168.7.231", None)],
        vec![],
        vec![],
    )
    .await;
    assert_ne!(
        container.id, split.id,
        "the container gets a host of its own"
    );

    discover_with_ports(
        &services,
        &lab,
        swept_with_service(&lab, "192.168.4.63", ens19(), "Portainer", 9443),
    )
    .await;
    discover(
        &services,
        &lab,
        HostBase::default(),
        vec![
            config(&lab, "192.168.4.126", ens18()),
            config(&lab, "192.168.4.63", ens19()),
        ],
        vec![],
        vec![],
    )
    .await;

    assert_eq!(
        hosts_holding(&services, &lab, "192.168.4.63").await,
        vec![vm.id]
    );
    assert_eq!(
        host_ips(&services, container.id).await,
        vec!["192.168.7.231".parse::<IpAddr>().unwrap()]
    );
    assert_eq!(
        service_names_on(&services, container.id).await,
        vec!["Grafana"]
    );
    assert!(
        service_names_on(&services, vm.id)
            .await
            .contains(&"Portainer".to_string())
    );
    assert_eq!(live_hosts(&services, &lab).await.len(), 2);
}

/// A runtime on bare metal: the sweep files the ipvlan endpoint's address on the runtime's own
/// host (its NIC's MAC, and no hypervisor list to rule it out). Docker's report takes the address
/// to a container host under the runtime, and the runtime keeps its own address and service.
#[tokio::test]
async fn a_container_report_takes_its_address_off_its_runtimes_host() {
    harness!(_storage, services, lab, _container);

    let runtime = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.4.126", ens18())],
        vec![service(&lab, "Docker")],
        vec![],
    )
    .await;
    let docker_id = runtime
        .services
        .iter()
        .find(|s| s.base.name == "Docker")
        .unwrap()
        .id;
    let swept = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![arp(&lab, "192.168.7.231", ens18())],
        vec![],
        vec![],
    )
    .await;
    assert_eq!(swept.id, runtime.id, "the sweep files .231 on the runtime");

    let container = discover(
        &services,
        &lab,
        HostBase {
            virtualization_service_id: Some(docker_id),
            virtualization_metadata: Some(HostVirtualization::Docker(
                ContainerHostVirtualization {
                    container_name: Some("ipvlan-test".to_string()),
                    container_id: Some("aec2a3e9".to_string()),
                    compose_project: None,
                    network_type: ContainerNetworkType::IpVlan,
                },
            )),
            ..Default::default()
        },
        vec![address(&lab, lab.lan, "192.168.7.231", None)],
        vec![],
        vec![],
    )
    .await;

    assert_ne!(container.id, runtime.id);
    assert_eq!(container.virtualization_service_id, Some(docker_id));
    assert_eq!(
        host_ips(&services, runtime.id).await,
        vec!["192.168.4.126".parse::<IpAddr>().unwrap()]
    );
    assert_eq!(
        service_names_on(&services, runtime.id).await,
        vec!["Docker"]
    );
    assert_eq!(
        host_ips(&services, container.id).await,
        vec!["192.168.7.231".parse::<IpAddr>().unwrap()]
    );
}

/// The lab's PROFINET DCP simulator: a macvlan interface `mv-dcp0` in the snmp-test VM with a
/// locally administered MAC. Proxmox reports the interface on the VM and the identity it presents,
/// and the DCP sweep then reports the device by that MAC alone. The device's own answer
/// corroborates Proxmox's copy, and of the two records holding the MAC the identity is the device,
/// so the DCP report lands on it instead of being refused.
#[tokio::test]
async fn a_dcp_report_joins_the_network_identity_its_mac_presents() {
    harness!(_storage, services, lab, _container);
    let mv: MacAddress = "8a:2e:53:fe:33:50".parse().unwrap();
    let proxmox = AttributeSource::Probe(ClientProbe::Proxmox);
    let nic = |source: AttributeSource| {
        Interface::new(InterfaceBase {
            network_id: lab.network_id,
            if_name: Some("mv-dcp0".to_string()),
            mac_address: Some(MacEvidence::new(MacEvidenceValue(mv), source)),
            ..Default::default()
        })
    };

    let guest = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![config(&lab, "192.168.4.21", ens18())],
        vec![service(&lab, "Network Identities")],
        vec![nic(proxmox)],
    )
    .await;
    let identity = discover(
        &services,
        &lab,
        HostBase {
            virtualization_metadata: Some(HostVirtualization::NetworkIdentity(
                NetworkIdentityVirtualization {},
            )),
            virtualization_service_id: Some(guest.services[0].id),
            virtualization_interface_id: Some(guest.interfaces[0].id),
            ..Default::default()
        },
        vec![address(&lab, lab.lan, "192.168.7.199", Some((mv, proxmox)))],
        vec![],
        vec![],
    )
    .await;

    let dcp = discover(
        &services,
        &lab,
        HostBase::default(),
        vec![],
        vec![],
        vec![nic(AttributeSource::ProfinetDcp)],
    )
    .await;

    assert_eq!(dcp.id, identity.id);
    assert_eq!(live_hosts(&services, &lab).await.len(), 2);
}

/// What a guest's hypervisor config and the guest itself say outrank DNS names, and an LXC's
/// config distribution and its SSH banner build one OS whichever lands first.
#[tokio::test]
async fn hypervisor_readings_outrank_dns_names_and_refine_the_banner_os() {
    use crate::server::hosts::r#impl::attributes::{HostHostnameValue, HostOsValue};
    use crate::server::hosts::r#impl::os::{HostOs, HostOsFamily};
    use crate::server::shared::attribution::Attributed;

    harness!(_storage, services, lab, _container);
    let debian = |version: Option<&str>| HostOs {
        family: HostOsFamily::Linux,
        name: Some("Debian".to_string()),
        version: version.map(str::to_string),
        edition: None,
        codename: None,
        kernel_version: None,
    };
    let reading = |hostname: Option<(&str, AttributeSource)>,
                   os: Option<(HostOs, AttributeSource)>| HostBase {
        hostname: hostname.map(|(h, s)| Attributed::new(HostHostnameValue(h.to_string()), s)),
        os: os.map(|(os, s)| Attributed::new(HostOsValue(os), s)),
        ..Default::default()
    };
    let lxc_mac: MacAddress = "bc:24:11:72:2e:bc".parse().unwrap();
    let bare_mac: MacAddress = "bc:24:11:77:70:53".parse().unwrap();

    // pihole: the sweep's reverse DNS name and SSH banner first, then Proxmox's config.
    let pihole = discover(
        &services,
        &lab,
        reading(
            Some(("pi.hole", AttributeSource::ReverseDns)),
            Some((debian(Some("12.0")), AttributeSource::SshBannerMatch)),
        ),
        vec![arp(&lab, "192.168.4.188", lxc_mac)],
        vec![],
        vec![],
    )
    .await;
    discover(
        &services,
        &lab,
        reading(
            Some(("pihole", AttributeSource::HypervisorConfig)),
            Some((debian(None), AttributeSource::HypervisorConfig)),
        ),
        vec![config(&lab, "192.168.4.188", lxc_mac)],
        vec![],
        vec![],
    )
    .await;

    // debian: Proxmox's config first, with no SSH; the banner's release arrives later.
    let bare = discover(
        &services,
        &lab,
        reading(
            None,
            Some((debian(None), AttributeSource::HypervisorConfig)),
        ),
        vec![config(&lab, "192.168.4.67", bare_mac)],
        vec![],
        vec![],
    )
    .await;
    discover(
        &services,
        &lab,
        reading(
            None,
            Some((debian(Some("12.0")), AttributeSource::SshBannerMatch)),
        ),
        vec![arp(&lab, "192.168.4.67", bare_mac)],
        vec![],
        vec![],
    )
    .await;

    let stored = |id: Uuid| {
        let services = &services;
        async move {
            services
                .host_service
                .get_by_id(&id)
                .await
                .unwrap()
                .expect("live")
        }
    };
    let pihole = stored(pihole.id).await;
    assert_eq!(
        pihole.base.hostname.as_ref().map(|h| h.value().0.as_str()),
        Some("pihole")
    );
    assert_eq!(
        pihole.base.os.as_ref().map(|os| os.value().0.clone()),
        Some(debian(Some("12.0")))
    );
    let bare = stored(bare.id).await;
    assert_eq!(
        bare.base.os.as_ref().map(|os| os.value().0.clone()),
        Some(debian(Some("12.0")))
    );
}
