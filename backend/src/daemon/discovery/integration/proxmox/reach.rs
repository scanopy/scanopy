//! A Proxmox VE node's bridges, read from `/nodes/{node}/network`, for
//! [`hypervisor::reach`](crate::daemon::discovery::integration::hypervisor::reach).

use crate::daemon::discovery::integration::hypervisor::reach::HypervisorBridge;

use super::types::NodeNetworkEntry;

/// The node's bridges, Linux and OVS. Other entries (NICs, bonds, VLANs) carry no guests.
pub fn node_bridges(entries: &[NodeNetworkEntry]) -> Vec<HypervisorBridge> {
    let has_ports = |ports: &Option<String>| {
        ports
            .as_deref()
            .is_some_and(|p| p.split_whitespace().any(|port| port != "none"))
    };
    entries
        .iter()
        .filter(|e| e.bridge_ports.is_some() || e.ovs_ports.is_some())
        .map(|e| HypervisorBridge {
            iface: e.iface.clone(),
            uplinked: has_ports(&e.bridge_ports) || has_ports(&e.ovs_ports),
            addressed: e.address.is_some() || e.address6.is_some(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::proxmox::types::PveEnvelope;

    // Recorded from the `pve` lab node (PVE 8.4.21) with the discovery token.
    const NODE_NETWORK: &str = include_str!("../../../../tests/proxmox/pve84_node_network.json");

    /// The lab node's `vmbr0`, plus the two internal shapes the lab doesn't have: a bridge with no
    /// ports the node routes and NATs for (`vmbr1`, 10.10.10.1/24), and one with no ports and no
    /// node address (`vmbr2`).
    #[test]
    fn the_lab_bridges_read_as_uplinked_routed_and_isolated() {
        let mut entries: Vec<NodeNetworkEntry> =
            serde_json::from_str::<PveEnvelope<Vec<NodeNetworkEntry>>>(NODE_NETWORK)
                .unwrap()
                .data;
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
        let bridges = node_bridges(&entries);
        let by_name = |name: &str| bridges.iter().find(|b| b.iface == name).unwrap().clone();
        assert!(by_name("vmbr0").uplinked && by_name("vmbr0").addressed);
        assert!(!by_name("vmbr1").uplinked && by_name("vmbr1").addressed);
        assert!(!by_name("vmbr2").uplinked && !by_name("vmbr2").addressed);
        // A NIC is not a bridge.
        assert!(bridges.iter().all(|b| b.iface != "enp1s0"));
    }
}
