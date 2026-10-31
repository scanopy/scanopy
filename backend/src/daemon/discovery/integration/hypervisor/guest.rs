//! A hypervisor's guests → Scanopy hosts, whichever hypervisor reports them.
//!
//! Every hypervisor integration reads the same three things about a guest: the NICs its config
//! declares, the addresses something reports for it (a guest agent, a running container's
//! interfaces, a DHCP lease, the config's static addresses), and, where a guest agent says, its OS
//! and hostname. The rules for turning those into one host per guest live here, so a guest is
//! treated alike under any hypervisor.
//!
//! Pure: no I/O.

use std::collections::BTreeSet;
use std::net::IpAddr;

use uuid::Uuid;

use crate::server::hosts::r#impl::{
    attributes::{HostHostnameAttributed, HostOsAttributed},
    base::{Host, HostBase},
    name::{HostName, HostNameSources},
    virtualization::HostVirtualization,
};
use crate::server::hosts::service::mac_identity::identity_permits_minting;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
use crate::server::ip_addresses::r#impl::base::{IPAddress, MacEvidence, MacEvidenceValue};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::position::renumber_positions;
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::base::Subnet;

/// The source of what a guest's config sets and the hypervisor applies to the guest: its NICs'
/// MACs and static addresses, an LXC container's hostname and distribution.
pub const GUEST_CONFIG: AttributeSource = AttributeSource::HypervisorConfig;

/// The source of what the QEMU guest agent reads inside a VM: its addresses, hostname and OS.
pub const GUEST_AGENT: AttributeSource = AttributeSource::GuestAgent;

/// The source of a guest NIC's MAC. Every MAC an integration submits for a guest is one the
/// guest's config declares (see [`select_addresses`]), so it is the hypervisor's own assignment
/// rather than a report about the guest — which is what lets a guest with no address still be one
/// host.
const NIC_MAC: AttributeSource = AttributeSource::HypervisorConfig;

/// One address a guest holds, with the NIC it sits on where the source says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestAddress {
    pub ip: IpAddr,
    /// Canonical lowercase-colon MAC.
    pub mac: Option<String>,
    pub interface: Option<String>,
    /// What reported the address: the guest agent, the hypervisor about the running guest, or the
    /// guest's config.
    pub source: AttributeSource,
}

/// A NIC a guest's config declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigNic {
    /// The hypervisor's own name for the NIC: Proxmox's `net0`, libvirt's target device.
    pub key: String,
    /// The interface name inside the guest, where the config names one (an LXC container's
    /// `eth0`). A VM's config carries none.
    pub name: Option<String>,
    pub mac: Option<String>,
    /// The hypervisor bridge it is attached to (`vmbr0`, `br0`).
    pub bridge: Option<String>,
    pub static_ips: Vec<IpAddr>,
}

/// The addresses that identify a guest: those on its own virtual NICs, else its config's static
/// ones.
///
/// What runs inside a guest reports every interface it has, and most of those are the guest's own
/// internals: a Docker host's `docker0` (172.17.0.1 on every Docker host there is) and its
/// `br-*` networks, Home Assistant's `hassio`, a WireGuard `wg0`. Submitted as the guest's
/// addresses they would merge unrelated guests on the server, which matches hosts by address. An
/// interface counts only when its MAC is one the guest's config gives a NIC, which is the one
/// thing that ties it to the hypervisor's view of the guest.
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
                source: GUEST_CONFIG,
            })
        })
        .filter(|a| reachable(a.ip))
        .collect()
}

/// An address something else on the network could use to reach the guest. Loopback and
/// link-local are on every guest and identify none of them.
pub fn reachable(ip: IpAddr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
        return false;
    }
    match ip {
        IpAddr::V4(v4) => !v4.is_link_local(),
        IpAddr::V6(v6) => !v6.is_unicast_link_local(),
    }
}

/// What a hypervisor reports about one guest: the NICs its config declares, the addresses it
/// holds, what its runtime reported on every interface, and, where a guest agent or the guest's
/// config says, its OS and hostname.
#[derive(Debug, Clone, Default)]
pub struct GuestReading {
    pub nics: Vec<ConfigNic>,
    pub addresses: Vec<GuestAddress>,
    /// Every reachable address the running guest reported, on any interface. The source of its
    /// network identities ([`super::identities::network_identities`]); never its own addresses.
    pub reported: Vec<GuestAddress>,
    pub os: Option<HostOsAttributed>,
    pub hostname: Option<HostHostnameAttributed>,
    /// Whether the running guest's own interfaces were read (a guest agent, an LXC container's
    /// interfaces), so `reported` is every address it presents rather than nothing known.
    pub interfaces_read: bool,
}

/// Who a guest is to its hypervisor: what its host carries as virtualization metadata, and the
/// name a person gave it there, credited to the hypervisor that reported it.
#[derive(Debug, Clone)]
pub struct GuestIdentity {
    pub virtualization: HostVirtualization,
    pub name: Option<String>,
    pub reported_by: ClientProbe,
}

/// What a guest is submitted as: its host, its addresses, and, for a guest no address is reported
/// for, one interface per configured NIC.
pub struct GuestRecord {
    pub host: Host,
    pub ip_addresses: Vec<IPAddress>,
    pub interfaces: Vec<Interface>,
}

/// A guest's host, linked to its hypervisor's service when `owner` is known.
///
/// A guest is a workload whether or not anything reports an address for it (a stopped VM, one
/// without a guest agent). Without an address its NICs' MACs identify it, carried as bare
/// interfaces the way a PROFINET station's is, so every scan lands on the same host and a later
/// sweep that finds the guest's address joins it by MAC. `None` only for a guest with neither an
/// address nor a NIC whose MAC can anchor a host: nothing would keep it one host from scan to
/// scan.
pub fn guest_host(
    identity: GuestIdentity,
    reading: &GuestReading,
    owner: Option<Uuid>,
    subnets: &[Subnet],
    site_id: Uuid,
) -> Option<GuestRecord> {
    let GuestReading {
        nics,
        addresses,
        os,
        hostname,
        ..
    } = reading;
    let nic_macs: Vec<&ConfigNic> = nics.iter().filter(|n| n.mac.is_some()).collect();
    if addresses.is_empty() && nic_macs.is_empty() {
        return None;
    }

    let mut host = Host::new(HostBase {
        site_id,
        source: EntitySource::Discovery,
        virtualization_metadata: Some(identity.virtualization),
        virtualization_service_id: owner,
        os: os.clone(),
        hostname: hostname.clone(),
        ..Default::default()
    });
    if let Some(name) = identity.name {
        // A person named the guest in its hypervisor.
        host.base
            .apply_name(HostName::from_controller(name, identity.reported_by));
    }

    let mut seen = BTreeSet::new();
    let mut ip_addresses: Vec<IPAddress> = addresses
        .iter()
        .filter(|a| seen.insert(a.ip))
        .enumerate()
        .filter_map(|(position, a)| {
            IPAddress::discovered(
                site_id,
                subnets,
                a.ip,
                a.source,
                mac_evidence(a.mac.as_deref()),
                a.interface.clone(),
                position as i32,
            )
        })
        .collect();
    renumber_positions(&mut ip_addresses);

    let interfaces: Vec<Interface> = if ip_addresses.is_empty() {
        nic_macs
            .into_iter()
            .map(|nic| {
                Interface::new(InterfaceBase {
                    host_id: Uuid::nil(), // Server assigns.
                    site_id,
                    // An LXC config names the NIC (`eth0`); a VM's does not, and no name is
                    // invented.
                    if_name: nic.name.clone(),
                    mac_address: mac_evidence(nic.mac.as_deref()),
                    // The NIC the guest's config declares.
                    interface_source: NIC_MAC,
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
