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
    HostVirtualization, NetworkIdentityVirtualization,
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
