//! BRIDGE-MIB and Q-BRIDGE-MIB: bridge port numbering, the forwarding database, the VLAN table
//! and per-port VLAN membership.

use super::*;

/// Walk dot1dBasePortIfIndex to build bridge_port → ifIndex mapping.
///
/// This is the highest-leverage truncation in the file: both FDB and VLAN-membership collection
/// key everything off it, so a cut-short walk here silently empties both for the whole switch.
///
/// Walked **once per host** by the caller and handed to both consumers. It used to be walked
/// independently inside each of them, which on a device that answers this OID with silence
/// rather than `noSuchObject` — the Ubiquiti USW-Pro-Max does exactly this — paid the full
/// walk timeout twice per scan for a table that was never going to arrive.
pub async fn query_bridge_port_mapping<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<HashMap<i32, i32>>> {
    let mut port_to_if_index: HashMap<i32, i32> = HashMap::new();
    let mut shortfall = Shortfall::default();

    // Asked before the walk so the device's own figure survives a walk that returns nothing —
    // which is the case worth reporting. A switch declaring 48 bridge ports and then answering
    // this table with silence has contradicted itself, and that reads very differently to an
    // operator than the bare "does not implement SNMP bridge-port numbering" it produces today.
    let claim = session
        .get_scalar(&oids::oid_parts(oids::bridge::DOT1D_BASE_NUM_PORTS))
        .await
        .ok()
        .flatten()
        .and_then(|value| value_to_i32(&value))
        .filter(|ports| *ports > 0)
        .map(|ports| DeviceClaim::Count {
            source: ClaimSource::Dot1dBaseNumPorts,
            expected: ports as usize,
        });

    // OID suffix is the bridge port number; value is the ifIndex.
    walk_column(
        session,
        ip,
        oids::bridge::DOT1D_BASE_PORT_IF_INDEX,
        &mut shortfall,
        |suffix, value| {
            if let Some(&port_u64) = suffix.last()
                && let Some(if_index) = value_to_i32(value)
            {
                port_to_if_index.insert(port_u64 as i32, if_index);
            }
        },
    )
    .await;

    Ok(SnmpCollection {
        records: port_to_if_index,
        complete: shortfall.complete,
        unsupported: false,
        reason: shortfall.reason,
        discarded: 0,
        discard_reason: None,
        claim,
        local_port_is_if_index: false,
    })
}

/// In-progress FDB row assembled column-by-column across an SNMP walk, keyed by
/// its MAC. Shared by the legacy (dot1dTpFdbTable) and VLAN-aware (dot1qTpFdbTable)
/// collectors so their results can be merged by MAC.
#[derive(Default)]
struct FdbBuilder {
    mac_address: Option<mac_address::MacAddress>,
    port: Option<i32>,
    status: Option<i32>,
}

/// Query bridge FDB for MAC-to-port mappings, resolving bridge ports to ifIndex
/// values via dot1dBasePortIfIndex. Collects both the legacy `dot1dTpFdbTable`
/// (RFC 4188) and the VLAN-aware `dot1qTpFdbTable` (Q-BRIDGE, RFC 4363) — many
/// VLAN-aware switches (Aruba/HP ProCurve, etc.) populate only the latter and
/// leave the legacy table empty, so relying on dot1d alone silently produced no
/// L2 adjacency for them (GH #649).
pub async fn query_bridge_fdb<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
    // Step 1 (`query_bridge_port_mapping`) is done by the caller and shared with
    // `query_port_vlan_membership`. Both FDB tables reference this same dot1dBasePort space.
    bridge_ports: &SnmpCollection<HashMap<i32, i32>>,
) -> Result<SnmpCollection<Vec<BridgeFdbEntry>>> {
    // Seeded from the shared bridge-port walk: when *that* failed, everything keyed by it
    // inherits the reason rather than inventing one of its own.
    let mut shortfall = Shortfall {
        complete: bridge_ports.complete,
        reason: bridge_ports.reason,
    };
    let port_to_if_index = &bridge_ports.records;

    // Step 2: Walk legacy dot1dTpFdbTable columns.
    let mut fdb_entries: HashMap<String, FdbBuilder> = HashMap::new();

    // Every column answering "no such object" is a device with no bridge MIB *here* — which on a
    // switch that partitions its forwarding database by VLAN means "not in this context", not
    // "this switch forwards nothing". `unsupported` was hard-coded false, so a Catalyst read
    // without its VLAN context reported a complete, empty, authoritative read of a table it had
    // never been asked for, and the operator was told nothing at all (GH #686).
    let mut all_columns_unsupported = true;

    let columns = [
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_ADDRESS, "address"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_PORT, "port"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_STATUS, "status"),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix is a 6-octet MAC encoded as 6 sub-ids.
        let stop = walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                if suffix.len() != 6 {
                    return;
                }
                let key = suffix
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                let entry = fdb_entries.entry(key).or_default();
                match column_name {
                    "address" => entry.mac_address = value_to_mac(value),
                    "port" => entry.port = value_to_i32(value),
                    "status" => entry.status = value_to_i32(value),
                    _ => {}
                }
            },
        )
        .await;
        if !stop.is_unsupported() {
            all_columns_unsupported = false;
        }
    }

    // Step 3: Merge in VLAN-aware Q-BRIDGE dot1qTpFdbTable entries. Legacy rows
    // win; Q-BRIDGE fills in MACs the legacy table didn't report (or all of them,
    // on switches that populate only the Q-BRIDGE table).
    let qbridge = walk_qbridge_fdb(session, ip).await.unwrap_or_default();
    if !qbridge.complete {
        shortfall.complete = false;
    }
    if !qbridge.unsupported {
        all_columns_unsupported = false;
    }
    let qbridge = qbridge.records;
    for (key, builder) in qbridge {
        fdb_entries.entry(key).or_insert(builder);
    }

    // Filter: keep learned(3) and mgmt(5), resolve bridge port to ifIndex. Not self(4) — those
    // are the bridge's own port addresses, which name no neighbour. (The old comment here read
    // "self (5)"; 5 is mgmt, per the encoding documented on DOT1Q_TP_FDB_STATUS.)
    let result: Vec<BridgeFdbEntry> = fdb_entries
        .into_values()
        .filter_map(|e| {
            let status = e.status.unwrap_or(0);
            if status != 3 && status != 5 {
                return None;
            }
            let bridge_port = e.port?;
            Some(BridgeFdbEntry {
                mac_address: e.mac_address?,
                bridge_port,
                if_index: port_to_if_index.get(&bridge_port).copied(),
                status,
            })
        })
        .collect();

    tracing::debug!(
        ip = %ip,
        entries = result.len(),
        port_mappings = port_to_if_index.len(),
        complete = shortfall.complete,
        "Bridge FDB walk finished"
    );

    // Only when neither table was there to read. A device serving one row is reporting one row;
    // a device serving neither table is not reporting at all.
    let unsupported = all_columns_unsupported && result.is_empty();

    Ok(SnmpCollection {
        records: result,
        complete: shortfall.complete,
        unsupported,
        reason: shortfall.reason,
        discarded: 0,
        discard_reason: None,
        claim: None,
        local_port_is_if_index: false,
    })
}

/// Walk the VLAN-aware Q-BRIDGE FDB (`dot1qTpFdbTable`, RFC 4363) for MAC→port
/// mappings, keyed by MAC so results merge with the legacy `dot1dTpFdbTable`.
///
/// Unlike the legacy table, the MAC lives in the table INDEX
/// (`dot1qFdbId` + 6 MAC octets), not a column, so it's derived from the OID
/// suffix. Ports are `dot1dBasePort` numbers, resolved by the caller against the
/// same `dot1dBasePortIfIndex` map. VLAN-aware switches (Aruba/HP ProCurve, etc.)
/// often populate only this table (GH #649).
async fn walk_qbridge_fdb<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<HashMap<String, FdbBuilder>>> {
    let mut entries: HashMap<String, FdbBuilder> = HashMap::new();
    let mut shortfall = Shortfall::default();
    // Reported so the caller can tell a switch with no Q-BRIDGE MIB from one whose Q-BRIDGE
    // table is genuinely empty. Hard-coding this false made the caller's own check dead.
    let mut all_columns_unsupported = true;

    let columns = [
        (oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_PORT, "port"),
        (oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_STATUS, "status"),
    ];

    for (base_oid_str, column_name) in columns {
        // Q-BRIDGE index = dot1qFdbId (1 sub-id) + MAC (6 octets).
        let stop = walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                let Some(mac) = qbridge_fdb_index_to_mac(suffix) else {
                    return;
                };
                if suffix.len() < 7 {
                    return;
                }
                // Key by MAC alone (drop fdb_id) so the same MAC learned across VLANs
                // collapses to one entry and merges with the legacy table's MAC key.
                let key = suffix[1..7]
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                let entry = entries.entry(key).or_default();
                entry.mac_address = Some(mac);
                match column_name {
                    "port" => entry.port = value_to_i32(value),
                    "status" => entry.status = value_to_i32(value),
                    _ => {}
                }
            },
        )
        .await;
        if !stop.is_unsupported() {
            all_columns_unsupported = false;
        }
    }

    let entries_empty = entries.is_empty();

    Ok(SnmpCollection {
        records: entries,
        complete: shortfall.complete,
        unsupported: all_columns_unsupported && entries_empty,
        reason: shortfall.reason,
        discarded: 0,
        discard_reason: None,
        claim: None,
        local_port_is_if_index: false,
    })
}

/// Query VLAN table for VLAN IDs and names.
/// Tries Q-BRIDGE dot1qVlanStaticName first, falls back to Cisco VTP vtpVlanName.
pub async fn query_vlan_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<VlanInfo>>> {
    let mut vlans: Vec<VlanInfo> = Vec::new();
    let mut shortfall = Shortfall::default();

    // Try Q-BRIDGE dot1qVlanStaticName first. OID suffix is the VLAN ID.
    walk_column(
        session,
        ip,
        oids::vlan::q_bridge::DOT1Q_VLAN_STATIC_NAME,
        &mut shortfall,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last()
                && let Some(name) = value_to_string(value)
            {
                vlans.push(VlanInfo {
                    vlan_id: vlan_u64 as u16,
                    name,
                });
            }
        },
    )
    .await;

    // Fall back to Cisco VTP if Q-BRIDGE returned nothing. VTP index is
    // mgmtDomainIndex.vlanId — use the last sub-id as the VLAN ID.
    if vlans.is_empty() {
        // A device with no Q-BRIDGE VLAN names has not fallen short if VTP answers instead, so
        // the fallback starts the reckoning again rather than inheriting the first walk's stop.
        shortfall = Shortfall::default();
        walk_column(
            session,
            ip,
            oids::vlan::cisco_vtp::VTP_VLAN_NAME,
            &mut shortfall,
            |suffix, value| {
                if let Some(&vlan_u64) = suffix.last()
                    && let Some(name) = value_to_string(value)
                {
                    vlans.push(VlanInfo {
                        vlan_id: vlan_u64 as u16,
                        name,
                    });
                }
            },
        )
        .await;
    }

    debug!(
        "VLAN table query from {} returned {} entries (Q-BRIDGE or VTP)",
        ip,
        vlans.len()
    );

    Ok(SnmpCollection::from_walk(vlans, shortfall))
}

/// Query per-port VLAN membership from Q-BRIDGE-MIB.
/// Uses dot1qPvid for native VLANs and dot1qVlanCurrentEgressPorts/UntaggedPorts
/// for tagged VLAN membership. Resolves bridge ports to ifIndex.
pub async fn query_port_vlan_membership<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
    // Step 1 (`query_bridge_port_mapping`) is done by the caller and shared with
    // `query_bridge_fdb`; every result below is keyed by bridge port.
    bridge_ports: &SnmpCollection<HashMap<i32, i32>>,
) -> Result<SnmpCollection<Vec<PortVlanMembership>>> {
    // Seeded from the shared bridge-port walk: when *that* failed, everything keyed by it
    // inherits the reason rather than inventing one of its own.
    let mut shortfall = Shortfall {
        complete: bridge_ports.complete,
        reason: bridge_ports.reason,
    };
    let port_to_if_index = &bridge_ports.records;

    if port_to_if_index.is_empty() {
        debug!(
            "No bridge port mappings from {} — skipping VLAN membership",
            ip
        );
        return Ok(SnmpCollection {
            records: Vec::new(),
            complete: shortfall.complete,
            unsupported: false,
            reason: shortfall.reason,
            discarded: 0,
            discard_reason: None,
            claim: None,
            local_port_is_if_index: false,
        });
    }

    // Step 2: Walk dot1qPvid for native VLAN per bridge port. OID suffix is the
    // bridge port number; value is the native VLAN ID.
    let mut native_vlans: HashMap<i32, u16> = HashMap::new();
    walk_column(
        session,
        ip,
        oids::vlan::q_bridge::DOT1Q_PVID,
        &mut shortfall,
        |suffix, value| {
            if let Some(&port_u64) = suffix.last()
                && let Some(vlan_id) = value_to_u16(value)
            {
                native_vlans.insert(port_u64 as i32, vlan_id);
            }
        },
    )
    .await;

    // Step 3: Walk dot1qVlanCurrentEgressPorts — PortList bitmap per VLAN, indexed
    // by timeFilter.vlanId (last sub-id is the VLAN ID).
    let mut egress_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_column(
        session,
        ip,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_EGRESS_PORTS,
        &mut shortfall,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last()
                && let Value::OctetString(bytes) = value
            {
                let vlan_id = vlan_u64 as u16;
                for bp in parse_portlist_bitmap(bytes) {
                    egress_by_port.entry(bp).or_default().push(vlan_id);
                }
            }
        },
    )
    .await;

    // Step 4: Walk dot1qVlanCurrentUntaggedPorts — same bitmap format.
    let mut untagged_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_column(
        session,
        ip,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_UNTAGGED_PORTS,
        &mut shortfall,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last()
                && let Value::OctetString(bytes) = value
            {
                let vlan_id = vlan_u64 as u16;
                for bp in parse_portlist_bitmap(bytes) {
                    untagged_by_port.entry(bp).or_default().push(vlan_id);
                }
            }
        },
    )
    .await;

    // Step 5: Assemble per-port membership, resolving bridge port → ifIndex
    let mut result: Vec<PortVlanMembership> = Vec::new();

    for (&bridge_port, &if_index) in port_to_if_index {
        let native_vlan = native_vlans.get(&bridge_port).copied();
        let egress_vlans = egress_by_port.get(&bridge_port);
        let untagged_vlans = untagged_by_port.get(&bridge_port);

        // Tagged VLANs = egress VLANs minus untagged VLANs for this port
        let tagged_vlans: Vec<u16> = match egress_vlans {
            Some(egress) => {
                let untagged_set: std::collections::HashSet<u16> = untagged_vlans
                    .map(|v| v.iter().copied().collect())
                    .unwrap_or_default();
                egress
                    .iter()
                    .copied()
                    .filter(|v| !untagged_set.contains(v))
                    .collect()
            }
            None => Vec::new(),
        };

        // Only include ports that have some VLAN data
        if native_vlan.is_some() || !tagged_vlans.is_empty() {
            result.push(PortVlanMembership {
                if_index,
                native_vlan,
                tagged_vlans,
            });
        }
    }

    debug!(
        "VLAN membership query from {} returned {} port memberships ({} bridge port mappings)",
        ip,
        result.len(),
        port_to_if_index.len()
    );

    Ok(SnmpCollection {
        records: result,
        complete: shortfall.complete,
        unsupported: false,
        reason: shortfall.reason,
        discarded: 0,
        discard_reason: None,
        claim: None,
        local_port_is_if_index: false,
    })
}
