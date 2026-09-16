//! Turning an ifTable entry and the neighbour, FDB and VLAN data heard on it into an `Interface`.

use super::*;

/// Build one candidate per LLDP neighbour record heard on this port.
///
/// GH #701: previously `convert_snmp_if_entry` kept only the *first* matching record
/// (`.find()`), which is the confirmed root cause of "only 2 of 3 edges render on a shared L2
/// segment" — a shared segment's ports each hear more than one neighbour, and every one past the
/// first was silently discarded. `.filter()` instead of `.find()` is the entire fix at this layer;
/// resolution (server-side) is what turns multiple candidates into multiple links.
fn lldp_candidates_for_port(
    entry: &IfTableEntry,
    lldp_neighbors: &[LldpNeighbor],
) -> Vec<InterfaceNeighborEvidence> {
    lldp_neighbors
        .iter()
        .filter(|n| n.local_port_index == entry.if_index)
        .map(|n| InterfaceNeighborEvidence {
            lldp_chassis_id: n
                .remote_chassis_id_subtype
                .zip(n.remote_chassis_id_bytes.as_ref())
                .and_then(|(subtype, bytes)| LldpChassisId::from_snmp(subtype, bytes)),
            lldp_port_id: n
                .remote_port_id_subtype
                .zip(n.remote_port_id_bytes.as_ref())
                .and_then(|(subtype, bytes)| LldpPortId::from_snmp(subtype, bytes)),
            lldp_sys_name: n.remote_sys_name.clone(),
            lldp_port_desc: n.remote_port_desc.clone(),
            lldp_mgmt_addr: n.remote_mgmt_addr,
            lldp_sys_desc: n.remote_sys_desc.clone(),
            ..Default::default()
        })
        .collect()
}

/// Build one candidate per CDP neighbour record heard on this port. See
/// [`lldp_candidates_for_port`] — same fix, CDP side.
fn cdp_candidates_for_port(
    entry: &IfTableEntry,
    cdp_neighbors: &[CdpNeighbor],
) -> Vec<InterfaceNeighborEvidence> {
    cdp_neighbors
        .iter()
        .filter(|n| n.local_port_index == entry.if_index)
        .map(|n| InterfaceNeighborEvidence {
            cdp_device_id: n.remote_device_id.clone(),
            cdp_port_id: n.remote_port_id.clone(),
            cdp_platform: n.remote_platform.clone(),
            cdp_address: n.remote_address,
            ..Default::default()
        })
        .collect()
}

/// Convert SNMP ifTable entry to Interface entity with LLDP/CDP/FDB neighbor data.
/// Uses Uuid::nil() for host_id as placeholder - server will set correct host_id.
#[allow(clippy::too_many_arguments)]
pub(crate) fn convert_snmp_if_entry(
    entry: &IfTableEntry,
    network_id: Uuid,
    lldp_neighbors: &[LldpNeighbor],
    cdp_neighbors: &[CdpNeighbor],
    bridge_fdb: &[BridgeFdbEntry],
    port_vlan_membership: &[PortVlanMembership],
    vlan_number_to_uuid: &std::collections::HashMap<u16, Uuid>,
    ip_configured_if_indexes: &HashSet<i32>,
) -> Interface {
    // Every LLDP record and every CDP record heard on this port becomes its own candidate — an
    // LLDP entry and a CDP entry for the same physical neighbour stay two rows (see
    // `InterfaceNeighborEvidence`'s module docs), and a shared segment's port hearing several
    // distinct neighbours keeps every one of them instead of the first.
    let mut neighbor_candidates = lldp_candidates_for_port(entry, lldp_neighbors);
    neighbor_candidates.extend(cdp_candidates_for_port(entry, cdp_neighbors));

    // Find VLAN membership for this port
    let vlan_membership = port_vlan_membership
        .iter()
        .find(|m| m.if_index == entry.if_index);

    // Collect learned MACs from bridge FDB for this port.
    // Single-MAC ports are used for neighbor resolution server-side;
    // multi-MAC ports indicate uplinks where LLDP/CDP is the better source
    // for direct neighbor identification.
    let fdb_macs: Vec<String> = bridge_fdb
        .iter()
        .filter(|fdb| fdb.if_index == Some(entry.if_index) && fdb.status == 3)
        .map(|fdb| fdb.mac_address.to_string())
        .collect();

    Interface::new(InterfaceBase {
        host_id: Uuid::nil(), // Placeholder - server will set correct host_id
        network_id,
        if_index: Some(entry.if_index),
        if_descr: entry.if_descr.clone(),
        if_name: entry.if_name.clone(),
        if_alias: entry.if_alias.clone(),
        // Straight through: an ifTable that omitted ifType said nothing about it, and `1`
        // ("other") is a type the agent could have reported.
        if_type: entry.if_type,
        speed_bps: entry.if_speed.map(|s| s as i64),
        admin_status: Some(IfAdminStatus::from(entry.if_admin_status.unwrap_or(1))),
        oper_status: Some(IfOperStatus::from(entry.if_oper_status.unwrap_or(1))),
        // `ifPhysAddress` for one of the device's own interfaces.
        mac_address: entry.if_phys_address.map(|m| {
            MacEvidence::new(
                MacEvidenceValue(m),
                AttributeSource::Probe(ClientProbe::Snmp),
            )
        }),
        ip_address_id: None, // Linked server-side via MAC matching
        ip_configured: ip_configured_if_indexes.contains(&entry.if_index),
        neighbor_candidates,
        // Bridge FDB data
        fdb_macs: if fdb_macs.is_empty() {
            None
        } else {
            Some(fdb_macs)
        },
        // VLAN data: resolved to entity UUIDs by caller via vlan_number_to_uuid mapping
        native_vlan_id: vlan_membership
            .and_then(|m| m.native_vlan)
            .and_then(|vid| vlan_number_to_uuid.get(&vid).copied()),
        vlan_ids: vlan_membership
            .map(|m| {
                m.tagged_vlans
                    .iter()
                    .filter_map(|vid| vlan_number_to_uuid.get(vid).copied())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty()),
    })
}
