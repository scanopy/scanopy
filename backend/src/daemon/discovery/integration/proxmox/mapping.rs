//! Proxmox VE wire shapes → Scanopy hosts.
//!
//! Pure: no I/O, so every rule here runs against recorded API responses in the tests below.

use std::collections::BTreeSet;
use std::net::IpAddr;

use uuid::Uuid;

use crate::server::hosts::r#impl::{
    base::{Host, HostBase},
    name::{HostName, HostNameSources},
    virtualization::{HostVirtualization, ProxmoxGuestType, ProxmoxVirtualization},
};
use crate::server::hosts::service::mac_identity::identity_permits_minting;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
use crate::server::ip_addresses::r#impl::base::{
    IPAddress, IPAddressBase, MacEvidence, MacEvidenceValue,
};
use crate::server::lldp::canonical_mac;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::types::entities::EntitySource;

use super::types::{
    AgentInterfaces, ClusterResource, ClusterStatusEntry, GuestConfig, LxcInterface,
};

/// The source of a guest NIC's MAC. Every MAC this integration submits is one a guest's config
/// declares (see [`select_addresses`]), so it is the hypervisor's own assignment rather than a
/// report about the guest — which is what lets a guest with no address still be one host.
const NIC_MAC: AttributeSource = AttributeSource::HypervisorConfig;

/// A node of the cluster, as far as the API places it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSpec {
    pub name: String,
    /// Where the node is reached. The scanned address for the node that answered; the cluster
    /// address `/cluster/status` reports for the others. `None` when neither is known.
    pub ip: Option<IpAddr>,
    /// The node whose API answered, which is the host being scanned.
    pub local: bool,
}

/// The nodes of the cluster, with an address for each where one is known.
///
/// `/cluster/status` names every node with its cluster address and flags the one that answered.
/// It needs `Sys.Audit`; without it only `/cluster/resources` is left, which names the nodes but
/// not their addresses. A single node is then still placeable, because it can only be the one
/// that answered; with several, which one answered is unknown and none get an address.
pub fn nodes(
    resources: &[ClusterResource],
    status: Option<&[ClusterStatusEntry]>,
    scanned_ip: IpAddr,
) -> Vec<NodeSpec> {
    if let Some(status) = status {
        let mut nodes: Vec<NodeSpec> = status
            .iter()
            .filter(|e| e.entry_type == "node")
            .map(|e| {
                let local = e.local.is_some_and(|l| l.as_bool());
                NodeSpec {
                    name: e.name.clone(),
                    ip: if local {
                        Some(scanned_ip)
                    } else {
                        e.ip.as_deref().and_then(|ip| ip.trim().parse().ok())
                    },
                    local,
                }
            })
            .collect();
        if !nodes.is_empty() {
            nodes.sort_by(|a, b| a.name.cmp(&b.name));
            return nodes;
        }
    }

    let names: BTreeSet<&str> = resources
        .iter()
        .filter(|r| r.resource_type == "node")
        .filter_map(|r| r.node.as_deref())
        .collect();
    let only_one = names.len() == 1;
    names
        .into_iter()
        .map(|name| NodeSpec {
            name: name.to_string(),
            ip: only_one.then_some(scanned_ip),
            local: only_one,
        })
        .collect()
}

/// A guest from the cluster-wide resource list, before its addresses are read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestSummary {
    pub node: String,
    pub vmid: i64,
    pub name: Option<String>,
    pub guest_type: ProxmoxGuestType,
    pub running: bool,
}

impl GuestSummary {
    /// The API path for this guest, e.g. `/nodes/pve/qemu/110`.
    pub fn path(&self) -> String {
        let kind = match self.guest_type {
            ProxmoxGuestType::Qemu => "qemu",
            ProxmoxGuestType::Lxc => "lxc",
        };
        format!("/nodes/{}/{}/{}", self.node, kind, self.vmid)
    }
}

/// Every VM and container in the cluster, templates excluded: a template never runs and has
/// nothing on the network to find.
pub fn guests(resources: &[ClusterResource]) -> Vec<GuestSummary> {
    let mut guests: Vec<GuestSummary> = resources
        .iter()
        .filter_map(|r| {
            let guest_type = match r.resource_type.as_str() {
                "qemu" => ProxmoxGuestType::Qemu,
                "lxc" => ProxmoxGuestType::Lxc,
                _ => return None,
            };
            if r.template.is_some_and(|t| t.as_bool()) {
                return None;
            }
            Some(GuestSummary {
                node: r.node.clone()?,
                vmid: r.vmid?.as_i64(),
                name: r
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(str::to_string),
                guest_type,
                running: r.status.as_deref() == Some("running"),
            })
        })
        .collect();
    guests.sort_by_key(|g| g.vmid);
    guests
}

/// One address a guest holds, with the NIC it sits on where the source says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestAddress {
    pub ip: IpAddr,
    /// Canonical lowercase-colon MAC.
    pub mac: Option<String>,
    pub interface: Option<String>,
}

/// The NICs a guest's config declares, with any static addresses it gives them.
///
/// A QEMU NIC is `net0: virtio=BC:24:11:..,bridge=vmbr0`: the MAC is the value of the model key,
/// or of `macaddr`. Its static address, when cloud-init sets one, is in the matching
/// `ipconfig0: ip=192.168.7.170/22,gw=...`. An LXC NIC carries both in one value:
/// `name=eth0,hwaddr=BC:24:11:..,ip=10.0.0.5/24`. `dhcp`, `manual` and `auto` are not addresses.
pub fn config_nics(config: &GuestConfig) -> Vec<ConfigNic> {
    let indexed = |prefix: &'static str| {
        config.iter().filter_map(move |(key, value)| {
            let index = key.strip_prefix(prefix)?;
            (!index.is_empty() && index.chars().all(|c| c.is_ascii_digit()))
                .then(|| (index.to_string(), value.as_str()))
        })
    };
    let pairs = |value: &str| -> Vec<(String, String)> {
        value
            .split(',')
            .filter_map(|part| part.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect()
    };
    let static_ip = |key: &str, value: &str| -> Option<IpAddr> {
        matches!(key, "ip" | "ip6")
            .then(|| value.split('/').next()?.parse().ok())
            .flatten()
    };

    let mut nics: Vec<ConfigNic> = indexed("net")
        .filter_map(|(index, value)| {
            let mut nic = ConfigNic {
                key: format!("net{index}"),
                name: None,
                mac: None,
                static_ips: vec![],
            };
            for (k, v) in pairs(value?) {
                match k.as_str() {
                    "name" => nic.name = Some(v),
                    "ip" | "ip6" => nic.static_ips.extend(static_ip(&k, &v)),
                    "bridge" | "tag" | "firewall" | "rate" | "mtu" | "queues" | "trunks" | "gw"
                    | "gw6" | "type" | "link_down" => {}
                    // `hwaddr` (LXC), `macaddr`, or the QEMU model key carrying the MAC.
                    _ => {
                        if nic.mac.is_none() {
                            nic.mac = canonical_mac(&v);
                        }
                    }
                }
            }
            Some(nic)
        })
        .collect();

    for (index, value) in indexed("ipconfig") {
        let Some(nic) = nics.iter_mut().find(|n| n.key == format!("net{index}")) else {
            continue;
        };
        for (k, v) in pairs(value.unwrap_or_default()) {
            nic.static_ips.extend(static_ip(&k, &v));
        }
    }

    nics.sort_by(|a, b| a.key.cmp(&b.key));
    nics
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigNic {
    pub key: String,
    /// The interface name inside an LXC container (`eth0`). QEMU configs carry none.
    pub name: Option<String>,
    pub mac: Option<String>,
    pub static_ips: Vec<IpAddr>,
}

/// Addresses the QEMU guest agent reports from inside a running VM.
pub fn agent_addresses(agent: &AgentInterfaces) -> Vec<GuestAddress> {
    agent
        .result
        .iter()
        .flat_map(|iface| {
            let mac = iface.hardware_address.as_deref().and_then(canonical_mac);
            iface.ip_addresses.iter().filter_map(move |a| {
                let ip: IpAddr = a.ip_address.trim().parse().ok()?;
                Some(GuestAddress {
                    ip,
                    mac: mac.clone(),
                    interface: Some(iface.name.clone()),
                })
            })
        })
        .filter(|a| reachable(a.ip))
        .collect()
}

/// Addresses a running LXC container reports for its interfaces.
pub fn lxc_addresses(interfaces: &[LxcInterface]) -> Vec<GuestAddress> {
    interfaces
        .iter()
        .flat_map(|iface| {
            let mac = iface.hwaddr.as_deref().and_then(canonical_mac);
            let ips: Vec<IpAddr> = if iface.ip_addresses.is_empty() {
                [iface.inet.as_deref(), iface.inet6.as_deref()]
                    .into_iter()
                    .flatten()
                    .flat_map(|list| list.split([' ', ',']))
                    .filter_map(|cidr| cidr.split('/').next()?.trim().parse().ok())
                    .collect()
            } else {
                iface
                    .ip_addresses
                    .iter()
                    .filter_map(|a| a.ip_address.trim().parse().ok())
                    .collect()
            };
            ips.into_iter().map(move |ip| GuestAddress {
                ip,
                mac: mac.clone(),
                interface: Some(iface.name.clone()),
            })
        })
        .filter(|a| reachable(a.ip))
        .collect()
}

/// The addresses that identify a guest: those on its own virtual NICs, else its config's static
/// ones.
///
/// What runs inside a guest reports every interface it has, and most of those are the guest's own
/// internals: a Docker host's `docker0` (172.17.0.1 on every Docker host there is) and its
/// `br-*` networks, Home Assistant's `hassio`, a WireGuard `wg0`. Submitted as the guest's
/// addresses they would merge unrelated guests on the server, which matches hosts by address. An
/// interface counts only when its MAC is one the guest's config gives a `netN` NIC, which is the
/// one thing that ties it to the hypervisor's view of the guest.
pub fn select_addresses(nics: &[ConfigNic], runtime: Vec<GuestAddress>) -> Vec<GuestAddress> {
    let on_a_nic: Vec<GuestAddress> = runtime
        .into_iter()
        .filter(|a| {
            a.mac
                .as_ref()
                .is_some_and(|mac| nics.iter().any(|n| n.mac.as_ref() == Some(mac)))
        })
        .collect();
    if on_a_nic.is_empty() {
        static_addresses(nics)
    } else {
        on_a_nic
    }
}

/// Static addresses from a guest's config (an LXC `ip=`, or a VM's cloud-init `ipconfigN`), for
/// a guest that is stopped or whose runtime read gave nothing. Each carries its NIC's MAC.
pub fn static_addresses(nics: &[ConfigNic]) -> Vec<GuestAddress> {
    nics.iter()
        .flat_map(|nic| {
            nic.static_ips.iter().map(|ip| GuestAddress {
                ip: *ip,
                mac: nic.mac.clone(),
                interface: nic.name.clone(),
            })
        })
        .filter(|a| reachable(a.ip))
        .collect()
}

/// An address something else on the network could use to reach the guest. Loopback and
/// link-local are on every guest and identify none of them.
fn reachable(ip: IpAddr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
        return false;
    }
    match ip {
        IpAddr::V4(v4) => !v4.is_link_local(),
        IpAddr::V6(v6) => !v6.is_unicast_link_local(),
    }
}

/// A node's host: named by the node name, which is its hostname. The Proxmox VE service is added
/// by the caller's matcher, not here.
pub fn node_host(node: &NodeSpec, ip: IpAddr, network_id: Uuid) -> (Host, IPAddress) {
    let identity = crate::daemon::discovery::integration::controller::ControllerIdentity {
        probe: ClientProbe::Proxmox,
        name: None,
        hostname: Some(node.name.clone()),
        chassis_id: None,
        manufacturer: None,
        model: None,
        serial_number: None,
        firmware_revision: None,
    };
    (
        identity.into_host(network_id),
        reported_address(network_id, ip, None, None, 0),
    )
}

/// What a guest is submitted as: its host, its addresses, and, for a guest the API reports no
/// address for, one interface per configured NIC.
pub struct GuestRecord {
    pub host: Host,
    pub ip_addresses: Vec<IPAddress>,
    pub interfaces: Vec<Interface>,
}

/// A guest's host, linked to its node's Proxmox VE service when `owner` is known.
///
/// A guest is a workload whether or not anything reports an address for it (a stopped VM, one
/// without the guest agent). Without an address its NICs' MACs identify it, carried as bare
/// interfaces the way a PROFINET station's is, so every scan lands on the same host and a later
/// sweep that finds the guest's address joins it by MAC. `None` only for a guest with neither an
/// address nor a NIC whose MAC can anchor a host: nothing would keep it one host from scan to
/// scan.
pub fn guest_host(
    guest: &GuestSummary,
    addresses: &[GuestAddress],
    nics: &[ConfigNic],
    owner: Option<Uuid>,
    network_id: Uuid,
) -> Option<GuestRecord> {
    let nic_macs: Vec<&ConfigNic> = nics.iter().filter(|n| n.mac.is_some()).collect();
    if addresses.is_empty() && nic_macs.is_empty() {
        return None;
    }

    let mut host = Host::new(HostBase {
        network_id,
        source: EntitySource::Discovery,
        virtualization_metadata: Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
            vm_name: guest.name.clone(),
            vm_id: Some(guest.vmid.to_string()),
            guest_type: Some(guest.guest_type),
        })),
        virtualization_service_id: owner,
        ..Default::default()
    });
    if let Some(name) = &guest.name {
        // A person named the guest in Proxmox.
        host.base.apply_name(HostName::from_controller(
            name.clone(),
            ClientProbe::Proxmox,
        ));
    }

    let mut seen = BTreeSet::new();
    let ip_addresses: Vec<IPAddress> = addresses
        .iter()
        .filter(|a| seen.insert(a.ip))
        .enumerate()
        .map(|(position, a)| {
            reported_address(
                network_id,
                a.ip,
                a.mac.as_deref(),
                a.interface.clone(),
                position as i32,
            )
        })
        .collect();

    let interfaces: Vec<Interface> = if ip_addresses.is_empty() {
        nic_macs
            .into_iter()
            .map(|nic| {
                Interface::new(InterfaceBase {
                    host_id: Uuid::nil(), // Server assigns.
                    network_id,
                    // LXC names the NIC (`eth0`); a QEMU config does not, and no name is invented.
                    if_name: nic.name.clone(),
                    mac_address: mac_evidence(nic.mac.as_deref()),
                    ..Default::default()
                })
            })
            .collect()
    } else {
        vec![]
    };

    // The server's own rule for a payload with no address: its MAC must be able to anchor a host.
    // A locally administered MAC (one set by hand, like `02:…`) cannot, because the server never
    // matches on one either, so sending it would be refused or duplicate every scan.
    if !identity_permits_minting(&host, &ip_addresses, &interfaces) {
        return None;
    }

    Some(GuestRecord {
        host,
        ip_addresses,
        interfaces,
    })
}

fn mac_evidence(mac: Option<&str>) -> Option<MacEvidence> {
    mac.and_then(|m| m.parse().ok())
        .map(|m| MacEvidence::new(MacEvidenceValue(m), NIC_MAC))
}

/// An address the hypervisor reports, left for the server to attach and place.
fn reported_address(
    network_id: Uuid,
    ip: IpAddr,
    mac: Option<&str>,
    name: Option<String>,
    position: i32,
) -> IPAddress {
    IPAddress::new(IPAddressBase {
        network_id,
        host_id: Uuid::nil(),
        subnet_id: Uuid::nil(),
        ip_address: ip,
        mac_address: mac_evidence(mac),
        name,
        position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::proxmox::types::{PveEnvelope, PveVersion};

    // ------------------------------------------------------------------------------------
    // Recorded from a real Proxmox VE 8.4.21 node (the `pve` lab, `tools/wol/`) with
    // `curl -sk`, the way the daemon reads it. Only the cloud-init `sshkeys` value is scrubbed.
    // The token that recorded `pool_scoped` holds PVEVMAdmin on one pool and no `Sys.Audit`, so
    // `/cluster/status` answered 403: these are exactly what a narrowly granted token sees.
    // ------------------------------------------------------------------------------------

    const VERSION: &str = include_str!("../../../../tests/proxmox/pve84_version.json");
    const RESOURCES_POOL_SCOPED: &str =
        include_str!("../../../../tests/proxmox/pve84_cluster_resources_pool_scoped.json");
    const QEMU_110_CONFIG: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_110_config.json");

    // Recorded later from the same node with the read-only discovery token (PVEAuditor on `/`
    // plus VM.Monitor), so `/cluster/status` answers and the whole node is visible. The public
    // IPv6 prefix is rewritten to 2001:db8::/32 and cloud-init `sshkeys` scrubbed.
    const STATUS: &str = include_str!("../../../../tests/proxmox/pve84_cluster_status.json");
    const RESOURCES: &str = include_str!("../../../../tests/proxmox/pve84_cluster_resources.json");
    /// A Docker host VM: the agent reports `docker0` and three `br-*` bridges beside `ens18`.
    const QEMU_103_CONFIG: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_103_config.json");
    const QEMU_103_AGENT: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_103_agent_interfaces.json");
    /// A plain VM with the guest agent.
    const QEMU_111_CONFIG: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_111_config.json");
    const QEMU_111_AGENT: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_111_agent_interfaces.json");
    /// A WireGuard container: `eth0` plus a `wg0` tunnel with no MAC.
    const LXC_104_CONFIG: &str =
        include_str!("../../../../tests/proxmox/pve84_lxc_104_config.json");
    const LXC_104_INTERFACES: &str =
        include_str!("../../../../tests/proxmox/pve84_lxc_104_interfaces.json");

    fn ips(addresses: &[GuestAddress]) -> Vec<String> {
        addresses.iter().map(|a| a.ip.to_string()).collect()
    }

    /// With `/cluster/status` readable, the node that answered is flagged `local` and takes the
    /// address it was scanned at, which is the one the daemon can reach.
    #[test]
    fn cluster_status_places_the_answering_node_at_the_scanned_address() {
        let resources: Vec<ClusterResource> = data(RESOURCES);
        let status: Vec<ClusterStatusEntry> = data(STATUS);
        let reached: IpAddr = "10.9.9.9".parse().unwrap();
        assert_eq!(
            nodes(&resources, Some(&status), reached),
            vec![NodeSpec {
                name: "pve".to_string(),
                ip: Some(reached),
                local: true,
            }]
        );
    }

    /// The node runs both kinds of guest; every one is reported with its type, and the template
    /// is the only thing left out.
    #[test]
    fn a_whole_node_yields_vms_and_containers_but_no_template() {
        let resources: Vec<ClusterResource> = data(RESOURCES);
        let found = guests(&resources);
        assert!(found.iter().any(|g| g.guest_type == ProxmoxGuestType::Qemu));
        assert!(found.iter().any(|g| g.guest_type == ProxmoxGuestType::Lxc));
        assert!(!found.iter().any(|g| g.vmid == 9000), "9000 is a template");
        let listed = resources
            .iter()
            .filter(|r| matches!(r.resource_type.as_str(), "qemu" | "lxc"))
            .count();
        assert_eq!(found.len(), listed - 1);
    }

    /// The Docker host's agent reports `docker0` (172.17.0.1, on every Docker host) and its
    /// bridges. Only `ens18`, the NIC its config declares, identifies the VM.
    #[test]
    fn a_docker_hosts_bridges_are_not_its_addresses() {
        let nics = config_nics(&data(QEMU_103_CONFIG));
        let reported = agent_addresses(&data(QEMU_103_AGENT));
        assert!(
            ips(&reported).contains(&"172.17.0.1".to_string()),
            "the capture carries docker0"
        );

        let kept = select_addresses(&nics, reported);
        assert!(!kept.is_empty());
        assert!(kept.iter().all(|a| a.interface.as_deref() == Some("ens18")));
        assert!(ips(&kept).contains(&"192.168.4.126".to_string()));
    }

    /// The `wg0` tunnel has an address and no MAC; it is not the container's. Its global IPv6
    /// addresses come from `ip-addresses`, which `inet6` (link-local only) does not carry.
    #[test]
    fn a_containers_tunnel_is_dropped_and_its_global_ipv6_kept() {
        let nics = config_nics(&data(LXC_104_CONFIG));
        let interfaces: Vec<LxcInterface> = data(LXC_104_INTERFACES);
        let kept = select_addresses(&nics, lxc_addresses(&interfaces));
        assert_eq!(
            ips(&kept),
            vec![
                "192.168.4.191",
                "2001:db8:58d0:3a00:be24:11ff:fe71:ef5c",
                "fd0b:d38d:98f6:1:be24:11ff:fe71:ef5c",
            ]
        );
        assert!(
            kept.iter()
                .all(|a| a.mac.as_deref() == Some("bc:24:11:71:ef:5c"))
        );
    }

    /// With the agent running, its reading wins over the cloud-init static address, and both
    /// agree on the NIC.
    #[test]
    fn an_agent_reading_is_preferred_over_cloud_init() {
        let nics = config_nics(&data(QEMU_111_CONFIG));
        let kept = select_addresses(&nics, agent_addresses(&data(QEMU_111_AGENT)));
        assert!(ips(&kept).contains(&"192.168.7.171".to_string()));
        assert!(kept.iter().all(|a| a.interface.as_deref() == Some("eth0")));
    }

    fn data<T: serde::de::DeserializeOwned>(json: &str) -> T {
        serde_json::from_str::<PveEnvelope<T>>(json)
            .expect("fixture should parse")
            .data
    }

    fn scanned() -> IpAddr {
        "192.168.4.10".parse().unwrap()
    }

    #[test]
    fn version_parses() {
        let version: PveVersion = data(VERSION);
        assert_eq!(version.version, "8.4.21");
        assert_eq!(version.release.as_deref(), Some("8.4"));
    }

    /// Without `/cluster/status` a lone node can only be the one that answered, so it takes the
    /// scanned address and its guests can be linked to it.
    #[test]
    fn a_single_node_is_placed_at_the_scanned_address_without_cluster_status() {
        let resources: Vec<ClusterResource> = data(RESOURCES_POOL_SCOPED);
        assert_eq!(
            nodes(&resources, None, scanned()),
            vec![NodeSpec {
                name: "pve".to_string(),
                ip: Some(scanned()),
                local: true,
            }]
        );
    }

    /// The template (vmid 9000) never runs; the running VM and nothing else is a guest.
    #[test]
    fn templates_and_non_guest_resources_are_not_guests() {
        let resources: Vec<ClusterResource> = data(RESOURCES_POOL_SCOPED);
        assert_eq!(
            guests(&resources),
            vec![GuestSummary {
                node: "pve".to_string(),
                vmid: 110,
                name: Some("scanopy-wol-target".to_string()),
                guest_type: ProxmoxGuestType::Qemu,
                running: true,
            }]
        );
    }

    /// The VM has no guest agent, so its address comes from cloud-init's `ipconfig0`, paired with
    /// the MAC its `net0` declares.
    #[test]
    fn a_vm_without_the_agent_takes_its_cloud_init_address_and_nic_mac() {
        let config: GuestConfig = data(QEMU_110_CONFIG);
        assert_eq!(
            static_addresses(&config_nics(&config)),
            vec![GuestAddress {
                ip: "192.168.7.170".parse().unwrap(),
                mac: Some("bc:24:11:69:a3:e1".to_string()),
                interface: None,
            }]
        );
    }

    /// The guest host names its node's Proxmox VE service and says it is a QEMU VM, and the
    /// name a person gave it in Proxmox titles it.
    #[test]
    fn a_guest_host_links_to_its_node_and_carries_its_proxmox_identity() {
        let resources: Vec<ClusterResource> = data(RESOURCES_POOL_SCOPED);
        let config: GuestConfig = data(QEMU_110_CONFIG);
        let guest = &guests(&resources)[0];
        let owner = Uuid::new_v4();
        let network_id = Uuid::new_v4();

        let nics = config_nics(&config);
        let GuestRecord {
            host,
            ip_addresses: ips,
            interfaces,
        } = guest_host(
            guest,
            &static_addresses(&nics),
            &nics,
            Some(owner),
            network_id,
        )
        .expect("a guest with an address is recorded");

        assert!(interfaces.is_empty(), "the address row carries the MAC");
        assert_eq!(host.base.virtualization_service_id, Some(owner));
        assert_eq!(
            host.base.virtualization_metadata,
            Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                vm_name: Some("scanopy-wol-target".to_string()),
                vm_id: Some("110".to_string()),
                guest_type: Some(ProxmoxGuestType::Qemu),
            }))
        );
        assert_eq!(host.base.name.value().as_str(), "scanopy-wol-target");
        assert_eq!(
            host.base.name.source(),
            AttributeSource::Authored(ClientProbe::Proxmox)
        );
        assert_eq!(ips.len(), 1);
        assert_eq!(
            ips[0].base.mac_address.as_ref().map(|m| m.source()),
            Some(NIC_MAC)
        );
    }

    fn guest(resources: &[ClusterResource], vmid: i64) -> GuestSummary {
        GuestSummary {
            running: false,
            ..guests(resources)
                .into_iter()
                .find(|g| g.vmid == vmid)
                .expect("guest is listed")
        }
    }

    /// The WireGuard container stopped: DHCP in its config, so no address anywhere. It is still a
    /// workload, recorded by its NIC, whose Proxmox-assigned MAC the server accepts a host from.
    #[test]
    fn a_guest_without_an_address_is_recorded_by_its_nic() {
        let resources: Vec<ClusterResource> = data(RESOURCES);
        let nics = config_nics(&data(LXC_104_CONFIG));
        let addresses = select_addresses(&nics, vec![]);
        assert!(addresses.is_empty(), "ip=dhcp is not an address");

        let record = guest_host(
            &guest(&resources, 104),
            &addresses,
            &nics,
            None,
            Uuid::new_v4(),
        )
        .expect("a guest with a NIC is recorded");
        assert!(record.ip_addresses.is_empty());
        let interface = &record.interfaces[..];
        assert_eq!(interface.len(), 1);
        assert_eq!(interface[0].base.if_name.as_deref(), Some("eth0"));
        assert_eq!(
            interface[0].base.mac_address.as_ref().map(|m| m.source()),
            Some(NIC_MAC)
        );
    }

    /// The Docker-host VM's NIC carries a hand-set, locally administered MAC (`02:…`), which the
    /// server never matches on. Stopped, with no address, nothing would keep it one host, so it
    /// is not sent.
    #[test]
    fn a_guest_known_only_by_a_locally_administered_mac_is_not_recorded() {
        let resources: Vec<ClusterResource> = data(RESOURCES);
        let nics = config_nics(&data(QEMU_103_CONFIG));
        assert!(guest_host(&guest(&resources, 103), &[], &nics, None, Uuid::new_v4()).is_none());
    }

    /// No address and no NIC: nothing would keep it one host across scans, so it is not sent.
    #[test]
    fn a_guest_with_no_nic_and_no_address_is_not_recorded() {
        let resources: Vec<ClusterResource> = data(RESOURCES);
        let guest = &guests(&resources)[0];
        assert!(guest_host(guest, &[], &[], None, Uuid::new_v4()).is_none());
    }
}
