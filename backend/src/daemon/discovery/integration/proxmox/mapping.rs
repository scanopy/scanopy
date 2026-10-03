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

/// Everything Proxmox reports carries this as its source: the hypervisor describing what it runs.
const REPORTED: AttributeSource = AttributeSource::Probe(ClientProbe::Proxmox);

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

/// The NICs a guest's config declares: `(netN, mac, static addresses)`.
///
/// A QEMU NIC is `virtio=BC:24:11:..,bridge=vmbr0` — the MAC is the value of the model key, or
/// of `macaddr`. An LXC NIC is `name=eth0,hwaddr=BC:24:11:..,ip=10.0.0.5/24` and may carry
/// static addresses (`dhcp`, `manual` and `auto` are not addresses).
pub fn config_nics(config: &GuestConfig) -> Vec<ConfigNic> {
    let mut nics: Vec<ConfigNic> = config
        .iter()
        .filter(|(key, _)| {
            key.strip_prefix("net")
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
        .filter_map(|(key, value)| {
            let value = value.as_str()?;
            let mut nic = ConfigNic {
                key: key.clone(),
                name: None,
                mac: None,
                static_ips: vec![],
            };
            for part in value.split(',') {
                let Some((k, v)) = part.split_once('=') else {
                    continue;
                };
                match k.trim() {
                    "name" => nic.name = Some(v.trim().to_string()),
                    "ip" | "ip6" => {
                        if let Some(ip) = v.split('/').next().and_then(|ip| ip.parse().ok()) {
                            nic.static_ips.push(ip);
                        }
                    }
                    "bridge" | "tag" | "firewall" | "rate" | "mtu" | "queues" | "trunks"
                    | "gw" | "gw6" | "type" | "link_down" => {}
                    // `hwaddr` (LXC), `macaddr`, or the QEMU model key carrying the MAC.
                    _ => {
                        if nic.mac.is_none() {
                            nic.mac = canonical_mac(v);
                        }
                    }
                }
            }
            Some(nic)
        })
        .collect();
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
            [iface.inet.as_deref(), iface.inet6.as_deref()]
                .into_iter()
                .flatten()
                .flat_map(|list| list.split([' ', ',']))
                .filter_map(move |cidr| {
                    let ip: IpAddr = cidr.split('/').next()?.trim().parse().ok()?;
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

/// Static addresses from an LXC config, for a container that is stopped or whose runtime read
/// failed. Each carries its NIC's MAC.
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

/// A guest's host, linked to its node's Proxmox VE service when `owner` is known.
pub fn guest_host(
    guest: &GuestSummary,
    addresses: &[GuestAddress],
    owner: Option<Uuid>,
    network_id: Uuid,
) -> (Host, Vec<IPAddress>) {
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
        host.base
            .apply_name(HostName::from_controller(name.clone(), ClientProbe::Proxmox));
    }

    let mut seen = BTreeSet::new();
    let ips = addresses
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
    (host, ips)
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
        mac_address: mac
            .and_then(|m| m.parse().ok())
            .map(|m| MacEvidence::new(MacEvidenceValue(m), REPORTED)),
        name,
        position,
    })
}
