//! A guest's network identities: addresses and MACs it presents from interfaces of its own
//! beyond the NICs its config declares.
//!
//! A macvlan shim on a Docker host, a keepalived virtual IP, a service given its own LAN address,
//! an `snmpd` answering as a separate device: the guest agent (or an LXC container's interface
//! list) reports each with its name, MAC and addresses. That proves one fact, that the identity
//! lives inside the guest, and nothing about what it is. So each is recorded as its own host,
//! unnamed and unclassified, linked to a Network Identities service on the guest.
//!
//! Pure: no I/O, like [`super::mapping`].

use std::collections::BTreeMap;
use std::net::IpAddr;

use uuid::Uuid;

use crate::server::hosts::r#impl::{
    base::{Host, HostBase},
    virtualization::{HostVirtualization, NetworkIdentityVirtualization},
};
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, MacEvidence, MacEvidenceValue, is_unset_mac, mac_of,
};
use crate::server::services::definitions::network_identities::NetworkIdentities;
use crate::server::services::r#impl::base::{Service, ServiceBase};
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::MatchDetails;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::inference::placeable_subnet;

use super::mapping::{ConfigNic, GuestAddress, REPORTED};

/// One interface inside a guest that presents an address and MAC of its own on the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkIdentity {
    /// The guest's interface name (`mv-snmp4`).
    pub interface: Option<String>,
    /// Canonical lowercase-colon MAC.
    pub mac: String,
    /// Its addresses on subnets the site holds, in the order the guest reported them.
    pub addresses: Vec<IpAddr>,
}

/// The interfaces a running guest reports that are identities of their own.
///
/// An interface counts when it has a MAC, that MAC is not one the guest's config gives a `netN`
/// NIC (those are the guest's own addresses, see [`super::mapping::select_addresses`]), and at
/// least one of its addresses sits on a subnet the site holds that is not a container bridge
/// or loopback. That leaves out a Docker host's `docker0` and `br-*` (container bridge subnets, or
/// none at all), a `wg0` tunnel (no MAC), and loopback. Only the addresses on such subnets are
/// kept: the rest are the guest's internals.
pub fn network_identities(
    nics: &[ConfigNic],
    reported: &[GuestAddress],
    subnets: &[Subnet],
) -> Vec<NetworkIdentity> {
    let on_lan = |ip: IpAddr| {
        placeable_subnet(subnets, ip)
            .is_some_and(|s| !s.is_container_bridge_subnet() && !s.base.subnet_type.is_loopback())
    };
    let is_nic = |mac: &str| nics.iter().any(|n| n.mac.as_deref() == Some(mac));
    let is_unset = |mac: &str| {
        mac.parse::<mac_address::MacAddress>()
            .is_ok_and(|m| is_unset_mac(&m))
    };

    // Keyed by interface and MAC, in first-reported order.
    let mut identities: Vec<NetworkIdentity> = vec![];
    let mut index: BTreeMap<(Option<String>, String), usize> = BTreeMap::new();
    for address in reported {
        let Some(mac) = address.mac.as_deref() else {
            continue;
        };
        if is_nic(mac) || is_unset(mac) || !on_lan(address.ip) {
            continue;
        }
        let key = (address.interface.clone(), mac.to_string());
        let slot = *index.entry(key).or_insert_with(|| {
            identities.push(NetworkIdentity {
                interface: address.interface.clone(),
                mac: mac.to_string(),
                addresses: vec![],
            });
            identities.len() - 1
        });
        let identity = &mut identities[slot];
        if !identity.addresses.contains(&address.ip) {
            identity.addresses.push(address.ip);
        }
    }
    identities
}

/// The services a guest is submitted with: a Network Identities service when it has identities,
/// none otherwise. The service binds nothing: it stands for the identities, which link to it as
/// their owner.
pub fn guest_services(guest: &Host, identities: &[NetworkIdentity]) -> Vec<Service> {
    if identities.is_empty() {
        return vec![];
    }
    vec![Service::new(ServiceBase {
        host_id: guest.id,
        site_id: guest.base.site_id,
        name: ServiceDefinition::name(&NetworkIdentities).to_string(),
        service_definition: Box::new(NetworkIdentities),
        bindings: vec![],
        virtualization_metadata: None,
        virtualization_service_id: None,
        source: EntitySource::DiscoveryWithMatch {
            details: MatchDetails::new_certain(
                "the guest agent reports interfaces beyond the guest's configured NICs",
            ),
        },
        tags: vec![],
        position: 0,
    })]
}

/// The identity's MAC as the guest OS reports it.
///
/// Not the hypervisor's assignment, and a kernel macvlan MAC is random and locally administered.
/// So it is filed as [`REPORTED`], which leaves it weak: it never anchors a host on its own.
fn identity_mac(identity: &NetworkIdentity) -> Option<MacEvidence> {
    identity
        .mac
        .parse()
        .ok()
        .map(|m| MacEvidence::new(MacEvidenceValue(m), REPORTED))
}

/// The guest's own interface an identity sits on, carried in the guest's submission so the
/// identity can name it by its stored id ([`presenting_interface_id`]). No address: the
/// identity's addresses are its own host's.
pub fn identity_interface(identity: &NetworkIdentity, site_id: Uuid) -> Interface {
    Interface::new(InterfaceBase {
        host_id: Uuid::nil(), // Server assigns.
        site_id,
        if_name: identity.interface.clone(),
        mac_address: identity_mac(identity),
        ..Default::default()
    })
}

/// The stored id of the guest interface `identity` sits on, found among the interfaces the
/// guest's create response returned by its MAC and name.
pub fn presenting_interface_id(identity: &NetworkIdentity, stored: &[Interface]) -> Option<Uuid> {
    let mac = identity.mac.parse::<mac_address::MacAddress>().ok()?;
    stored
        .iter()
        .find(|i| mac_of(&i.base.mac_address) == Some(mac) && i.base.if_name == identity.interface)
        .map(|i| i.id)
}

/// An identity's host: its addresses, each with the identity's MAC, linked to the guest's
/// Network Identities service (`owner`) and to the guest interface that presents it
/// (`presenting_interface`). Unnamed, since the device's own data names it. Each address matches
/// the host the network sweep already holds for it.
pub fn identity_host(
    identity: &NetworkIdentity,
    owner: Uuid,
    presenting_interface: Option<Uuid>,
    subnets: &[Subnet],
    site_id: Uuid,
) -> (Host, Vec<IPAddress>) {
    let host = Host::new(HostBase {
        site_id,
        source: EntitySource::Discovery,
        virtualization_metadata: Some(HostVirtualization::NetworkIdentity(
            NetworkIdentityVirtualization {},
        )),
        virtualization_service_id: Some(owner),
        virtualization_interface_id: presenting_interface,
        ..Default::default()
    });
    let mac = identity_mac(identity);
    let ip_addresses = identity
        .addresses
        .iter()
        .enumerate()
        .filter_map(|(position, ip)| {
            IPAddress::discovered(
                site_id,
                subnets,
                *ip,
                mac.clone(),
                identity.interface.clone(),
                position as i32,
            )
        })
        .collect();
    (host, ip_addresses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::proxmox::mapping::{
        GuestReading, GuestSummary, agent_addresses, config_nics, guest_host, select_addresses,
    };
    use crate::daemon::discovery::integration::proxmox::types::{
        AgentInterfaces, GuestConfig, PveEnvelope,
    };
    use crate::server::credentials::r#impl::types::CredentialIntegration;
    use crate::server::hosts::r#impl::virtualization::ProxmoxGuestType;
    use crate::server::hosts::r#impl::virtualization::undeclared_virtualization;
    use crate::server::hosts::service::mac_identity::{MacQuality, grade};
    use crate::server::services::r#impl::definitions::{ServiceDefinitionExt, VirtualizationRole};
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::subnets::r#impl::base::{SubnetBase, SubnetCidr, SubnetCidrValue};
    use crate::server::subnets::r#impl::types::SubnetType;

    /// A Docker host VM: its agent reports `docker0` and three `br-*` bridges beside `ens18`.
    const QEMU_103_CONFIG: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_103_config.json");
    const QEMU_103_AGENT: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_103_agent_interfaces.json");

    fn data<T: serde::de::DeserializeOwned>(json: &str) -> T {
        serde_json::from_str::<PveEnvelope<T>>(json)
            .expect("fixture should parse")
            .data
    }

    fn subnet(cidr: &str, subnet_type: SubnetType) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr.parse().unwrap()),
                AttributeSource::Manual,
            ),
            subnet_type,
            ..Default::default()
        })
    }

    /// The lab's LAN, plus the two Docker bridges a Docker host's own scan records. The capture's
    /// third bridge (172.31.0.0/16) is on no subnet at all.
    fn subnets() -> Vec<Subnet> {
        vec![
            subnet("192.168.4.0/22", SubnetType::Lan),
            subnet("172.17.0.0/16", SubnetType::DockerBridge),
            subnet("172.19.0.0/16", SubnetType::DockerBridge),
            subnet("127.0.0.0/8", SubnetType::Loopback),
        ]
    }

    /// The Docker host's capture with two interfaces added: a macvlan shim holding a LAN address
    /// of its own, and a tunnel on the LAN with no MAC.
    fn docker_host_with_macvlan() -> (Vec<ConfigNic>, Vec<GuestAddress>) {
        let nics = config_nics(&data::<GuestConfig>(QEMU_103_CONFIG));
        let mut reported = agent_addresses(&data::<AgentInterfaces>(QEMU_103_AGENT));
        reported.push(GuestAddress {
            ip: "192.168.7.200".parse().unwrap(),
            mac: Some("3a:5e:01:aa:bb:cc".to_string()),
            interface: Some("mv-snmp4".to_string()),
        });
        reported.push(GuestAddress {
            ip: "192.168.7.201".parse().unwrap(),
            mac: None,
            interface: Some("wg0".to_string()),
        });
        (nics, reported)
    }

    /// The SNMP lab VM (107, `snmp-test`), recorded once its guest agent ran: `eth0` is its own
    /// NIC, and kernel macvlan links carry the simulated devices (`mv-snmp*`, `mv-ssh*`) plus two
    /// address-less links (`mv-dcp*`).
    const QEMU_107_CONFIG: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_107_config.json");
    const QEMU_107_AGENT: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_107_agent_interfaces.json");

    /// Every macvlan link holding a LAN address is an identity of the lab VM; its own NIC and the
    /// links without an address are not.
    #[test]
    fn the_lab_vms_macvlan_links_are_its_identities() {
        let nics = config_nics(&data::<GuestConfig>(QEMU_107_CONFIG));
        let reported = agent_addresses(&data::<AgentInterfaces>(QEMU_107_AGENT));
        let identities = network_identities(&nics, &reported, &subnets());

        let names: Vec<&str> = identities
            .iter()
            .filter_map(|i| i.interface.as_deref())
            .collect();
        assert!(
            names
                .iter()
                .all(|n| n.starts_with("mv-snmp") || n.starts_with("mv-ssh"))
        );
        assert_eq!(
            names.iter().filter(|n| n.starts_with("mv-snmp")).count(),
            34
        );
        assert!(!names.contains(&"eth0"));
        assert!(names.iter().all(|n| !n.starts_with("mv-dcp")));
        assert!(identities.iter().all(|i| !i.addresses.is_empty()));
    }

    /// `ens18` is the VM's own NIC, `docker0` and the `br-*` bridges sit on container bridge
    /// subnets or none, and loopback is loopback: the capture holds no identity.
    #[test]
    fn a_docker_hosts_nic_and_bridges_are_not_identities() {
        let nics = config_nics(&data::<GuestConfig>(QEMU_103_CONFIG));
        let reported = agent_addresses(&data::<AgentInterfaces>(QEMU_103_AGENT));
        assert!(
            reported
                .iter()
                .any(|a| a.interface.as_deref() == Some("docker0")),
            "the capture carries docker0"
        );
        assert!(network_identities(&nics, &reported, &subnets()).is_empty());
    }

    /// A macvlan with a LAN address is an identity, with that address; the tunnel without a MAC
    /// is not.
    #[test]
    fn a_macvlan_with_a_lan_address_is_an_identity() {
        let (nics, reported) = docker_host_with_macvlan();
        assert_eq!(
            network_identities(&nics, &reported, &subnets()),
            vec![NetworkIdentity {
                interface: Some("mv-snmp4".to_string()),
                mac: "3a:5e:01:aa:bb:cc".to_string(),
                addresses: vec!["192.168.7.200".parse().unwrap()],
            }]
        );
    }

    /// The guest carries the Network Identities service only when it has an identity, and the
    /// service binds nothing.
    #[test]
    fn the_guest_carries_the_service_only_with_identities() {
        let (nics, reported) = docker_host_with_macvlan();
        let guest = Host::new(HostBase::default());

        let identities = network_identities(&nics, &reported, &subnets());
        let services = guest_services(&guest, &identities);
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].base.host_id, guest.id);
        assert!(services[0].base.bindings.is_empty());
        assert!(matches!(
            ServiceDefinitionExt::virtualization_role(&services[0].base.service_definition),
            Some(VirtualizationRole::IdentityHost { .. })
        ));

        assert!(guest_services(&guest, &[]).is_empty());
    }

    /// The guest submits the interface an identity sits on; the identity finds it among the
    /// guest's stored interfaces by MAC and name, and its host links to both the guest's service
    /// and that interface, places its address on the LAN, and carries a MAC that cannot anchor it.
    #[test]
    fn an_identity_host_links_its_owner_and_presenting_interface_with_a_weak_mac() {
        let (nics, reported) = docker_host_with_macvlan();
        let subnets = subnets();
        let site_id = Uuid::new_v4();
        let identity = &network_identities(&nics, &reported, &subnets)[0];
        let owner = Uuid::new_v4();

        let submitted = identity_interface(identity, site_id);
        assert_eq!(submitted.base.if_name.as_deref(), Some("mv-snmp4"));
        // As stored: a server id, beside another of the guest's interfaces sharing the MAC.
        let stored = Interface::new(submitted.base.clone());
        let same_mac_other_name = Interface::new(InterfaceBase {
            if_name: Some("mv-snmp5".to_string()),
            ..submitted.base.clone()
        });
        let presenting = presenting_interface_id(identity, &[same_mac_other_name, stored.clone()]);
        assert_eq!(presenting, Some(stored.id));

        let (host, ip_addresses) = identity_host(identity, owner, presenting, &subnets, site_id);
        assert_eq!(
            undeclared_virtualization(CredentialIntegration::Proxmox, &host),
            None
        );
        assert_eq!(host.base.virtualization_service_id, Some(owner));
        assert_eq!(host.base.virtualization_interface_id, Some(stored.id));
        assert!(matches!(
            host.base.virtualization_metadata,
            Some(HostVirtualization::NetworkIdentity(_))
        ));
        assert_eq!(ip_addresses.len(), 1);
        assert_eq!(ip_addresses[0].base.subnet_id, subnets[0].id);
        let mac = ip_addresses[0].base.mac_address.as_ref().expect("MAC kept");
        assert_eq!(mac.source(), REPORTED);
        assert_eq!(grade(mac), MacQuality::Weak);
    }

    /// The guest's own submission keeps to its NIC: the identity's address is never on it, or the
    /// server would merge the guest into the identity's host by that address.
    #[test]
    fn no_identity_address_is_on_the_guest() {
        let (nics, reported) = docker_host_with_macvlan();
        let subnets = subnets();
        let identities = network_identities(&nics, &reported, &subnets);
        let guest = GuestSummary {
            node: "pve".to_string(),
            vmid: 103,
            name: Some("docker".to_string()),
            guest_type: ProxmoxGuestType::Qemu,
            running: true,
        };
        let reading = GuestReading {
            addresses: select_addresses(&nics, reported.clone()),
            nics,
            reported,
            ..Default::default()
        };
        let record =
            guest_host(&guest, &reading, None, &subnets, Uuid::new_v4()).expect("recorded");
        assert!(!record.ip_addresses.is_empty());
        assert!(record.ip_addresses.iter().all(|row| {
            identities
                .iter()
                .all(|i| !i.addresses.contains(&row.base.ip_address))
        }));
    }
}
