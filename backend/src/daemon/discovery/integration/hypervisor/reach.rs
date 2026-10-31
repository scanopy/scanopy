//! Which guests no subnet this scan covers reaches, and what would reach them.
//!
//! A guest's ports and services come from the network scan, which merges its host with the one
//! its hypervisor's integration records. A guest on a bridge with no uplink, or on an address
//! outside every subnet the scan sweeps, is never swept: it keeps its inventory and gets no
//! services. The hypervisor's network config says where such a guest sits, which decides what
//! reaches it: the scan's own subnets, a daemon on the hypervisor itself, or only a daemon on that
//! bridge. Each integration reads its hypervisor's bridges its own way (Proxmox's `/network`,
//! libvirt's host interfaces) into [`HypervisorBridge`].

use std::net::IpAddr;

use crate::daemon::discovery::service::warnings::UnreachedGuestReason;
use crate::server::subnets::r#impl::base::Subnet;

use super::guest::{ConfigNic, GuestAddress};

/// One bridge of a hypervisor's network config, as far as reaching its guests goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypervisorBridge {
    pub iface: String,
    /// It has a port onto a physical network (a NIC, a bond, a VLAN), so its guests sit on a
    /// network other hosts share.
    pub uplinked: bool,
    /// The hypervisor holds an address on it, so a daemon on the hypervisor has an interface
    /// there.
    pub addressed: bool,
}

/// A running guest whose every address lies outside the subnets this scan covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreachedGuest {
    /// The address it is named by in the warning: its first.
    pub address: IpAddr,
    /// What would reach it.
    pub reason: UnreachedGuestReason,
}

/// Whether the guest on `hypervisor` holding `addresses` on `nics` is out of the scan's reach,
/// and if so what reaches it. `None` when one address lies in a scanned subnet, or the guest has
/// none: the scan sweeps the first, and nothing could address the second.
pub fn unreached(
    nics: &[ConfigNic],
    addresses: &[GuestAddress],
    bridges: &[HypervisorBridge],
    scanned: &[Subnet],
    hypervisor: &str,
) -> Option<UnreachedGuest> {
    let first = addresses.first()?;
    // A container bridge is a network inside the daemon's own host. A guest address in its range
    // overlaps it by coincidence, and the sweep of that bridge never reaches the guest.
    if addresses.iter().any(|a| {
        scanned
            .iter()
            .filter(|s| !s.is_container_bridge_subnet())
            .any(|s| s.base.cidr.contains(&a.ip))
    }) {
        return None;
    }
    // The bridge of the NIC its first address sits on, else of its only NIC.
    let nic = first
        .mac
        .as_ref()
        .and_then(|mac| nics.iter().find(|n| n.mac.as_ref() == Some(mac)))
        .or(match nics {
            [only] => Some(only),
            _ => None,
        });
    let bridge = nic
        .and_then(|n| n.bridge.as_deref())
        .and_then(|name| bridges.iter().find(|b| b.iface == name));
    // Uplinked, or not in the hypervisor's config (a Proxmox SDN vnet, or a token without
    // `Sys.Audit`, which reads no `/network`): nothing says the network is internal.
    let reason = match bridge {
        Some(b) if b.uplinked => UnreachedGuestReason::OutsideScan,
        Some(b) if b.addressed => UnreachedGuestReason::BehindHypervisor {
            hypervisor: hypervisor.to_string(),
            bridge: b.iface.clone(),
        },
        Some(b) => UnreachedGuestReason::Isolated {
            bridge: b.iface.clone(),
        },
        None => UnreachedGuestReason::OutsideScan,
    };
    Some(UnreachedGuest {
        address: first.ip,
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    // Recorded from the Proxmox lab, so the Proxmox adapters read them in.
    use crate::daemon::discovery::integration::proxmox::mapping::{
        agent_addresses, config_nics, select_addresses,
    };
    use crate::daemon::discovery::integration::proxmox::reach::node_bridges;
    use crate::daemon::discovery::integration::proxmox::types::{
        AgentInterfaces, GuestConfig, NodeNetworkEntry, PveEnvelope,
    };
    use crate::server::shared::attribution::AttributeSource;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::subnets::r#impl::base::{
        SubnetBase, SubnetCidr, SubnetCidrValue, SubnetTypeAttributed,
    };
    use crate::server::subnets::r#impl::types::SubnetType;

    // Recorded from the `pve` lab node (PVE 8.4.21) with the discovery token.
    const NODE_NETWORK: &str = include_str!("../../../../tests/proxmox/pve84_node_network.json");
    const QEMU_107_CONFIG: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_107_config.json");
    const QEMU_107_AGENT: &str =
        include_str!("../../../../tests/proxmox/pve84_qemu_107_agent_interfaces.json");

    fn data<T: serde::de::DeserializeOwned>(json: &str) -> T {
        serde_json::from_str::<PveEnvelope<T>>(json).unwrap().data
    }

    fn subnet(cidr: &str) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr.parse().unwrap()),
                AttributeSource::Manual,
            ),
            ..Default::default()
        })
    }

    /// The lab node's `vmbr0`, plus the two internal shapes the lab doesn't have: a bridge with no
    /// ports the node routes and NATs for (`vmbr1`, 10.10.10.1/24), and one with no ports and no
    /// node address (`vmbr2`). Written the way `/nodes/{node}/network` reports them.
    fn bridges() -> Vec<HypervisorBridge> {
        let mut entries: Vec<NodeNetworkEntry> = data(NODE_NETWORK);
        entries.push(NodeNetworkEntry {
            iface: "vmbr1".to_string(),
            address: Some("10.10.10.1".to_string()),
            bridge_ports: Some("none".to_string()),
            ..Default::default()
        });
        entries.push(NodeNetworkEntry {
            iface: "vmbr2".to_string(),
            bridge_ports: Some(String::new()),
            ..Default::default()
        });
        node_bridges(&entries)
    }

    /// `snmp-test` (107) as recorded, on `vmbr0` at 192.168.4.21, optionally moved to another
    /// bridge and address.
    fn snmp_test(moved_to: Option<(&str, &str)>) -> (Vec<ConfigNic>, Vec<GuestAddress>) {
        let mut nics = config_nics(&data::<GuestConfig>(QEMU_107_CONFIG));
        let mut addresses = select_addresses(
            &nics,
            agent_addresses(&data::<AgentInterfaces>(QEMU_107_AGENT)),
        );
        if let Some((bridge, ip)) = moved_to {
            nics[0].bridge = Some(bridge.to_string());
            addresses.truncate(1);
            addresses[0].ip = ip.parse().unwrap();
        }
        (nics, addresses)
    }

    #[test]
    fn a_guest_in_a_scanned_subnet_is_reached() {
        let (nics, addresses) = snmp_test(None);
        assert_eq!(
            unreached(
                &nics,
                &addresses,
                &bridges(),
                &[subnet("192.168.4.0/22")],
                "pve"
            ),
            None
        );
    }

    #[test]
    fn a_guest_with_one_address_in_scope_is_reached() {
        let (nics, mut addresses) = snmp_test(Some(("vmbr2", "10.20.0.5")));
        let mut lan = addresses[0].clone();
        lan.ip = "192.168.4.21".parse().unwrap();
        addresses.push(lan);
        assert_eq!(
            unreached(
                &nics,
                &addresses,
                &bridges(),
                &[subnet("192.168.4.0/22")],
                "pve"
            ),
            None
        );
    }

    /// The lab daemon's host runs a Docker network `cc-snmp-lab` on 10.77.0.0/28, and the scan
    /// sweeps it. A guest on an isolated bridge at 10.77.0.10 is still out of reach.
    #[test]
    fn a_daemon_side_container_bridge_overlapping_the_guest_does_not_reach_it() {
        let (nics, addresses) = snmp_test(Some(("vmbr2", "10.77.0.10")));
        let mut docker_bridge = subnet("10.77.0.0/28");
        docker_bridge.base.subnet_type =
            SubnetTypeAttributed::unattributed(SubnetType::DockerBridge);
        let found = unreached(
            &nics,
            &addresses,
            &bridges(),
            &[subnet("192.168.4.0/22"), docker_bridge],
            "pve",
        );
        assert_eq!(
            found.map(|f| f.reason),
            Some(UnreachedGuestReason::Isolated {
                bridge: "vmbr2".to_string()
            })
        );
    }

    #[test]
    fn a_guest_on_the_uplinked_bridge_outside_the_scan_needs_its_subnet_scanned() {
        let (nics, addresses) = snmp_test(None);
        let found = unreached(
            &nics,
            &addresses,
            &bridges(),
            &[subnet("10.0.0.0/24")],
            "pve",
        )
        .unwrap();
        assert_eq!(found.reason, UnreachedGuestReason::OutsideScan);
        assert_eq!(found.address, addresses[0].ip);
    }

    #[test]
    fn a_guest_on_a_bridge_only_the_hypervisor_holds_an_address_on_is_reached_through_it() {
        let (nics, addresses) = snmp_test(Some(("vmbr1", "10.10.10.20")));
        let found = unreached(
            &nics,
            &addresses,
            &bridges(),
            &[subnet("192.168.4.0/22")],
            "pve",
        );
        assert_eq!(
            found.map(|f| f.reason),
            Some(UnreachedGuestReason::BehindHypervisor {
                hypervisor: "pve".to_string(),
                bridge: "vmbr1".to_string()
            })
        );
    }

    #[test]
    fn a_guest_on_a_bridge_with_no_uplink_and_no_hypervisor_address_is_isolated() {
        let (nics, addresses) = snmp_test(Some(("vmbr2", "10.20.0.5")));
        let found = unreached(
            &nics,
            &addresses,
            &bridges(),
            &[subnet("192.168.4.0/22")],
            "pve",
        );
        assert_eq!(
            found.map(|f| f.reason),
            Some(UnreachedGuestReason::Isolated {
                bridge: "vmbr2".to_string()
            })
        );
    }

    /// An SDN vnet, or any bridge `/network` didn't list (a token without `Sys.Audit` reads none):
    /// nothing says it is internal, so the warning says only that the scan doesn't cover it.
    #[test]
    fn a_guest_on_a_bridge_the_hypervisor_config_does_not_list_is_outside_the_scan() {
        let (nics, addresses) = snmp_test(Some(("vnet7", "10.30.0.5")));
        let found = unreached(&nics, &addresses, &[], &[subnet("192.168.4.0/22")], "pve");
        assert_eq!(
            found.map(|f| f.reason),
            Some(UnreachedGuestReason::OutsideScan)
        );
    }

    #[test]
    fn a_guest_with_no_address_is_not_reported() {
        let (nics, _) = snmp_test(None);
        assert_eq!(
            unreached(&nics, &[], &bridges(), &[subnet("10.0.0.0/24")], "pve"),
            None
        );
    }
}
