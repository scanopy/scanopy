//! SNMP Query Functions
//!
//! Functions for querying SNMP data from devices.

use anyhow::{Result, anyhow};
use snmp2::{Oid, Value};
use std::collections::HashMap;
use std::net::IpAddr;
use tokio::time::timeout;
use tracing::{debug, trace, warn};

use crate::server::credentials::r#impl::mapping::SnmpQueryCredential;
use crate::server::credentials::r#impl::types::SnmpVersion;

use super::oids::{self, oid_to_vec, parse_oid};
use super::session::{MAX_WALK_ENTRIES, SNMP_TIMEOUT, create_session};
use super::types::{
    ArpEntry, BridgeFdbEntry, CdpNeighbor, DeviceInventory, IfTableEntry, IpAddrEntry,
    LldpLocalInfo, LldpNeighbor, PortVlanMembership, SystemInfo, VlanInfo,
};
use super::values::{
    parse_lldp_mgmt_addr, parse_portlist_bitmap, value_to_i32, value_to_ip, value_to_mac,
    value_to_string, value_to_u16, value_to_u64,
};

/// Max varbinds requested per GETBULK PDU. Kept modest so responses fit in a
/// single UDP datagram even for columns with long string values (ifAlias/ifDescr).
const BULK_MAX_REPETITIONS: u32 = 20;

/// Whether a credential's SNMP version supports GETBULK (v2c/v3; not v1).
fn supports_bulk(credential: &SnmpQueryCredential) -> bool {
    !matches!(credential.version, SnmpVersion::V1)
}

/// Walk a single OID subtree (one table column), invoking `visit(suffix, value)`
/// for every varbind strictly under `base_oid_str`, where `suffix` is the OID
/// components after the base (the table index). Returns the number visited.
///
/// Uses GETBULK when `use_bulk` is true (SNMPv2c/v3) to fetch up to
/// `BULK_MAX_REPETITIONS` rows per round-trip — far fewer round-trips than one
/// GETNEXT per row on large tables. Falls back to GETNEXT if the first GETBULK
/// errors. On a per-request timeout or error the walk stops and returns what it
/// has collected so far — partial data is kept, never discarded.
async fn walk_column(
    session: &mut snmp2::AsyncSession,
    ip: IpAddr,
    base_oid_str: &str,
    label: &str,
    use_bulk: bool,
    mut visit: impl FnMut(&[u64], &Value),
) -> Result<usize> {
    let base_oid = parse_oid(base_oid_str)?;
    let base_parts: Vec<u64> = base_oid_str
        .split('.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut current_oid = base_oid;
    let mut count = 0usize;
    let mut use_bulk = use_bulk;

    loop {
        if count >= MAX_WALK_ENTRIES {
            warn!("Walk limit reached for {} on {}", label, ip);
            break;
        }

        // Fetch the next chunk of varbinds after current_oid.
        let mut response = if use_bulk {
            match timeout(
                SNMP_TIMEOUT,
                session.getbulk(&[&current_oid], 0, BULK_MAX_REPETITIONS),
            )
            .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(e)) if count == 0 => {
                    // Device likely doesn't support GETBULK — fall back to GETNEXT.
                    debug!(
                        "GETBULK unsupported for {} on {} ({:?}); using GETNEXT",
                        label, ip, e
                    );
                    use_bulk = false;
                    continue;
                }
                Ok(Err(e)) => {
                    debug!("Bulk walk {} failed on {}: {:?}", label, ip, e);
                    break;
                }
                Err(_) => {
                    debug!("Bulk walk {} timeout on {}", label, ip);
                    break;
                }
            }
        } else {
            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    debug!("Walk {} failed on {}: {:?}", label, ip, e);
                    break;
                }
                Err(_) => {
                    debug!("Walk {} timeout on {}", label, ip);
                    break;
                }
            }
        };

        let mut last_parts: Option<Vec<u64>> = None;
        let mut stop = false;

        while let Some((resp_oid, value)) = response.varbinds.next() {
            // EndOfMibView/NoSuchObject/NoSuchInstance = no more data
            if matches!(
                value,
                Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
            ) {
                stop = true;
                break;
            }

            // Check if we're still in the same subtree
            let parts = oid_to_vec(&resp_oid);
            if parts.len() <= base_parts.len() || !parts.starts_with(&base_parts) {
                // We've walked past the column
                stop = true;
                break;
            }

            visit(&parts[base_parts.len()..], &value);
            count += 1;
            last_parts = Some(parts);

            if count >= MAX_WALK_ENTRIES {
                stop = true;
                break;
            }
        }

        if stop {
            break;
        }

        match last_parts {
            Some(parts) => {
                current_oid = Oid::from(parts.as_slice())
                    .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
            }
            None => break, // empty response — nothing more to walk
        }
    }

    Ok(count)
}

/// Query system MIB information from a device
pub async fn query_system_info(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<SystemInfo> {
    let mut session = create_session(ip, credential, port).await?;
    let mut info = SystemInfo::default();

    // Query each system OID
    let oids_to_query = [
        (oids::system::SYS_DESCR, "sysDescr"),
        (oids::system::SYS_OBJECT_ID, "sysObjectID"),
        (oids::system::SYS_NAME, "sysName"),
        (oids::system::SYS_LOCATION, "sysLocation"),
        (oids::system::SYS_CONTACT, "sysContact"),
        (oids::system::SYS_UPTIME, "sysUpTime"),
    ];

    for (oid_str, name) in oids_to_query {
        let oid = match parse_oid(oid_str) {
            Ok(o) => o,
            Err(e) => {
                warn!("Failed to parse OID {}: {}", oid_str, e);
                continue;
            }
        };

        match timeout(SNMP_TIMEOUT, session.get(&oid)).await {
            Ok(Ok(mut response)) => {
                if let Some((resp_oid, value)) = response.varbinds.next() {
                    trace!("SNMP {} from {}: {:?} = {:?}", name, ip, resp_oid, value);
                    match name {
                        "sysDescr" => info.sys_descr = value_to_string(&value),
                        "sysObjectID" => info.sys_object_id = value_to_string(&value),
                        "sysName" => info.sys_name = value_to_string(&value),
                        "sysLocation" => info.sys_location = value_to_string(&value),
                        "sysContact" => info.sys_contact = value_to_string(&value),
                        "sysUpTime" => info.sys_uptime = value_to_u64(&value),
                        _ => {}
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("SNMP GET {} failed from {}: {:?}", name, ip, e);
            }
            Err(_) => {
                debug!("SNMP GET {} timeout from {}", name, ip);
            }
        }
    }

    Ok(info)
}

/// Walk the ifTable and ifXTable to get interface information
pub async fn walk_if_table(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Vec<IfTableEntry>> {
    let mut session = create_session(ip, credential, port).await?;
    let mut entries: HashMap<i32, IfTableEntry> = HashMap::new();

    // Define the columns we want to walk
    let columns = [
        (oids::if_mib::columns::IF_INDEX, "ifIndex"),
        (oids::if_mib::columns::IF_DESCR, "ifDescr"),
        (oids::if_mib::columns::IF_TYPE, "ifType"),
        (oids::if_mib::columns::IF_MTU, "ifMtu"),
        (oids::if_mib::columns::IF_SPEED, "ifSpeed"),
        (oids::if_mib::columns::IF_PHYS_ADDRESS, "ifPhysAddress"),
        (oids::if_mib::columns::IF_ADMIN_STATUS, "ifAdminStatus"),
        (oids::if_mib::columns::IF_OPER_STATUS, "ifOperStatus"),
        (oids::if_mib::if_x_table::IF_NAME, "ifName"),
        (oids::if_mib::if_x_table::IF_HIGH_SPEED, "ifHighSpeed"),
        (oids::if_mib::if_x_table::IF_ALIAS, "ifAlias"),
    ];

    // Walk each column
    let use_bulk = supports_bulk(credential);
    for (base_oid_str, column_name) in columns {
        let count = walk_column(
            &mut session,
            ip,
            base_oid_str,
            column_name,
            use_bulk,
            |suffix, value| {
                // ifTable index is a single component: the ifIndex
                if let Some(&if_index_u64) = suffix.last() {
                    let if_index = if_index_u64 as i32;
                    let entry = entries.entry(if_index).or_insert_with(|| IfTableEntry {
                        if_index,
                        if_descr: None,
                        if_type: None,
                        if_mtu: None,
                        if_speed: None,
                        if_phys_address: None,
                        if_admin_status: None,
                        if_oper_status: None,
                        if_name: None,
                        if_alias: None,
                    });

                    match column_name {
                        "ifIndex" => {
                            // Already set above
                        }
                        "ifDescr" => entry.if_descr = value_to_string(value),
                        "ifType" => entry.if_type = value_to_i32(value),
                        "ifMtu" => entry.if_mtu = value_to_i32(value),
                        "ifSpeed" => {
                            // Only set if ifHighSpeed not already set
                            if entry.if_speed.is_none() {
                                entry.if_speed = value_to_u64(value);
                            }
                        }
                        "ifPhysAddress" => entry.if_phys_address = value_to_mac(value),
                        "ifAdminStatus" => entry.if_admin_status = value_to_i32(value),
                        "ifOperStatus" => entry.if_oper_status = value_to_i32(value),
                        "ifName" => entry.if_name = value_to_string(value),
                        "ifHighSpeed" => {
                            // ifHighSpeed is in Mbps, convert to bps for consistency
                            if let Some(mbps) = value_to_u64(value) {
                                entry.if_speed = Some(mbps * 1_000_000);
                            }
                        }
                        "ifAlias" => entry.if_alias = value_to_string(value),
                        _ => {}
                    }
                }
            },
        )
        .await?;

        trace!("Walked {} entries for {} from {}", count, column_name, ip);
    }

    let mut result: Vec<IfTableEntry> = entries.into_values().collect();
    result.sort_by_key(|e| e.if_index);

    debug!(
        "SNMP ifTable walk from {} returned {} ip_addresses",
        ip,
        result.len()
    );
    // Diagnostic for issue #614 (high-ifIndex interfaces missing): log the full set of
    // collected ifIndex values, not just the count, so we can tell whether a high-ifIndex
    // switch (e.g. ifIndex 49153-49168) is dropped at walk time or later during ingestion.
    debug!(
        ip = %ip,
        if_indexes = ?result.iter().map(|e| e.if_index).collect::<Vec<_>>(),
        "SNMP ifTable walk ifIndex set"
    );

    Ok(result)
}

/// Query LLDP remote table for neighbor information
pub async fn query_lldp_neighbors(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Vec<LldpNeighbor>> {
    let mut session = create_session(ip, credential, port).await?;
    let mut neighbors: HashMap<(i32, i32), LldpNeighbor> = HashMap::new();

    // LLDP remote table uses a complex index: lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex
    // We'll walk the columns and extract the local port from the OID

    let columns = [
        (
            oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
            "remChassisIdSubtype",
        ),
        (
            oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID,
            "remChassisId",
        ),
        (
            oids::lldp::remote::entry::LLDP_REM_PORT_ID_SUBTYPE,
            "remPortIdSubtype",
        ),
        (oids::lldp::remote::entry::LLDP_REM_PORT_ID, "remPortId"),
        (oids::lldp::remote::entry::LLDP_REM_PORT_DESC, "remPortDesc"),
        (oids::lldp::remote::entry::LLDP_REM_SYS_NAME, "remSysName"),
        (oids::lldp::remote::entry::LLDP_REM_SYS_DESC, "remSysDesc"),
        (oids::lldp::remote::entry::LLDP_REM_MAN_ADDR, "remManAddr"),
    ];

    let use_bulk = supports_bulk(credential);
    for (base_oid_str, column_name) in columns {
        walk_column(
            &mut session,
            ip,
            base_oid_str,
            column_name,
            use_bulk,
            |suffix, value| {
                // Extract index components from OID suffix
                // Format: base.timeMark.localPortNum.remIndex
                if suffix.len() >= 3 {
                    let local_port = suffix[1] as i32;
                    let rem_index = suffix[2] as i32;

                    let neighbor =
                        neighbors
                            .entry((local_port, rem_index))
                            .or_insert_with(|| LldpNeighbor {
                                local_port_index: local_port,
                                remote_chassis_id_subtype: None,
                                remote_chassis_id_bytes: None,
                                remote_port_id_subtype: None,
                                remote_port_id_bytes: None,
                                remote_port_desc: None,
                                remote_sys_name: None,
                                remote_sys_desc: None,
                                remote_mgmt_addr: None,
                            });

                    match column_name {
                        "remChassisIdSubtype" => {
                            neighbor.remote_chassis_id_subtype =
                                value_to_i32(value).map(|v| v as u8)
                        }
                        "remChassisId" => {
                            if let Value::OctetString(bytes) = value {
                                neighbor.remote_chassis_id_bytes = Some(bytes.to_vec());
                            }
                        }
                        "remPortIdSubtype" => {
                            neighbor.remote_port_id_subtype = value_to_i32(value).map(|v| v as u8)
                        }
                        "remPortId" => {
                            if let Value::OctetString(bytes) = value {
                                neighbor.remote_port_id_bytes = Some(bytes.to_vec());
                            }
                        }
                        "remPortDesc" => neighbor.remote_port_desc = value_to_string(value),
                        "remSysName" => neighbor.remote_sys_name = value_to_string(value),
                        "remSysDesc" => neighbor.remote_sys_desc = value_to_string(value),
                        "remManAddr" => {
                            // Management address is encoded as address family + address bytes
                            if let Value::OctetString(bytes) = value {
                                neighbor.remote_mgmt_addr = parse_lldp_mgmt_addr(bytes);
                            }
                        }
                        _ => {}
                    }
                }
            },
        )
        .await?;
    }

    let result: Vec<LldpNeighbor> = neighbors.into_values().collect();
    debug!("LLDP query from {} returned {} neighbors", ip, result.len());

    Ok(result)
}

/// Query ipAddrTable for IP address to ifIndex + subnet mask mappings.
/// Walks ipAdEntIfIndex and ipAdEntNetMask columns where the OID suffix
/// encodes the IP address as A.B.C.D.
pub async fn query_ip_addr_table(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<HashMap<IpAddr, IpAddrEntry>> {
    let mut session = create_session(ip, credential, port).await?;
    let mut if_index_map: HashMap<IpAddr, i32> = HashMap::new();
    let mut net_mask_map: HashMap<IpAddr, IpAddr> = HashMap::new();

    let use_bulk = supports_bulk(credential);

    // Walk ipAdEntIfIndex
    walk_column(
        &mut session,
        ip,
        oids::ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
        "ipAddrTable ifIndex",
        use_bulk,
        |suffix, value| {
            if suffix.len() == 4 {
                let addr = IpAddr::from([
                    suffix[0] as u8,
                    suffix[1] as u8,
                    suffix[2] as u8,
                    suffix[3] as u8,
                ]);
                if let Some(if_index) = value_to_i32(value) {
                    if_index_map.insert(addr, if_index);
                }
            }
        },
    )
    .await?;

    // Walk ipAdEntNetMask
    walk_column(
        &mut session,
        ip,
        oids::ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
        "ipAddrTable netMask",
        use_bulk,
        |suffix, value| {
            if suffix.len() == 4 {
                let addr = IpAddr::from([
                    suffix[0] as u8,
                    suffix[1] as u8,
                    suffix[2] as u8,
                    suffix[3] as u8,
                ]);
                if let Some(mask) = value_to_ip(value) {
                    net_mask_map.insert(addr, mask);
                }
            }
        },
    )
    .await?;

    // Combine ifIndex and netMask results
    let result: HashMap<IpAddr, IpAddrEntry> = if_index_map
        .into_iter()
        .map(|(addr, if_index)| {
            let net_mask = net_mask_map.get(&addr).copied();
            (addr, IpAddrEntry { if_index, net_mask })
        })
        .collect();

    debug!(
        "ipAddrTable walk from {} returned {} entries",
        ip,
        result.len()
    );

    Ok(result)
}

/// Query CDP cache table for neighbor information (Cisco devices)
pub async fn query_cdp_neighbors(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Vec<CdpNeighbor>> {
    let mut session = create_session(ip, credential, port).await?;
    let mut neighbors: HashMap<(i32, i32), CdpNeighbor> = HashMap::new();

    let columns = [
        (oids::cdp::entry::CDP_CACHE_DEVICE_ID, "deviceId"),
        (oids::cdp::entry::CDP_CACHE_DEVICE_PORT, "devicePort"),
        (oids::cdp::entry::CDP_CACHE_PLATFORM, "platform"),
        (oids::cdp::entry::CDP_CACHE_ADDRESS, "address"),
    ];

    let use_bulk = supports_bulk(credential);
    for (base_oid_str, column_name) in columns {
        walk_column(
            &mut session,
            ip,
            base_oid_str,
            column_name,
            use_bulk,
            |suffix, value| {
                // CDP index: base.cdpCacheIfIndex.cdpCacheDeviceIndex
                if suffix.len() >= 2 {
                    let if_index = suffix[0] as i32;
                    let device_index = suffix[1] as i32;

                    let neighbor = neighbors
                        .entry((if_index, device_index))
                        .or_insert_with(|| CdpNeighbor {
                            local_port_index: if_index,
                            remote_device_id: None,
                            remote_port_id: None,
                            remote_platform: None,
                            remote_address: None,
                        });

                    match column_name {
                        "deviceId" => neighbor.remote_device_id = value_to_string(value),
                        "devicePort" => neighbor.remote_port_id = value_to_string(value),
                        "platform" => neighbor.remote_platform = value_to_string(value),
                        "address" => {
                            // CDP address is encoded as 4 bytes for IPv4
                            if let Value::OctetString(bytes) = value
                                && bytes.len() == 4
                            {
                                neighbor.remote_address =
                                    Some(IpAddr::from([bytes[0], bytes[1], bytes[2], bytes[3]]));
                            }
                        }
                        _ => {}
                    }
                }
            },
        )
        .await?;
    }

    let result: Vec<CdpNeighbor> = neighbors.into_values().collect();
    debug!("CDP query from {} returned {} neighbors", ip, result.len());

    Ok(result)
}

/// Query ARP table (ipNetToMediaTable) for IP-to-MAC mappings.
/// Returns entries with ifIndex, MAC, and IP for each ARP cache entry.
pub async fn query_arp_table(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Vec<ArpEntry>> {
    let mut session = create_session(ip, credential, port).await?;

    // We need to walk 4 columns: ifIndex, physAddress, netAddress, type
    // OID suffix format: ifIndex.A.B.C.D
    struct ArpEntryBuilder {
        if_index: Option<i32>,
        mac_address: Option<mac_address::MacAddress>,
        ip_address: Option<IpAddr>,
        entry_type: Option<i32>,
    }

    let mut entries: HashMap<String, ArpEntryBuilder> = HashMap::new();

    let columns = [
        (oids::arp::entry::IP_NET_TO_MEDIA_IF_INDEX, "ifIndex"),
        (
            oids::arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
            "physAddress",
        ),
        (oids::arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS, "netAddress"),
        (oids::arp::entry::IP_NET_TO_MEDIA_TYPE, "type"),
    ];

    let use_bulk = supports_bulk(credential);
    for (base_oid_str, column_name) in columns {
        walk_column(
            &mut session,
            ip,
            base_oid_str,
            column_name,
            use_bulk,
            |suffix, value| {
                // OID suffix: ifIndex.A.B.C.D
                if suffix.len() >= 5 {
                    let key = suffix
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(".");

                    let entry = entries.entry(key).or_insert_with(|| ArpEntryBuilder {
                        if_index: None,
                        mac_address: None,
                        ip_address: None,
                        entry_type: None,
                    });

                    match column_name {
                        "ifIndex" => entry.if_index = value_to_i32(value),
                        "physAddress" => entry.mac_address = value_to_mac(value),
                        "netAddress" => entry.ip_address = value_to_ip(value),
                        "type" => entry.entry_type = value_to_i32(value),
                        _ => {}
                    }
                }
            },
        )
        .await?;
    }

    // Filter out invalid entries (type==2) and entries missing required fields
    let result: Vec<ArpEntry> = entries
        .into_values()
        .filter_map(|e| {
            let entry_type = e.entry_type.unwrap_or(0);
            // Skip invalid entries (type 2)
            if entry_type == 2 {
                return None;
            }
            Some(ArpEntry {
                if_index: e.if_index?,
                mac_address: e.mac_address?,
                ip_address: e.ip_address?,
            })
        })
        .collect();

    debug!(
        "ARP table walk from {} returned {} entries",
        ip,
        result.len()
    );

    Ok(result)
}

/// Query ENTITY-MIB entPhysicalTable for hardware inventory.
/// Returns the best-match physical entity (chassis > stack > module).
pub async fn query_entity_physical(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Option<DeviceInventory>> {
    let mut session = create_session(ip, credential, port).await?;

    struct PhysicalEntry {
        description: Option<String>,
        class: Option<i32>,
        name: Option<String>,
        serial_number: Option<String>,
        manufacturer: Option<String>,
        model: Option<String>,
    }

    let mut entries: HashMap<i32, PhysicalEntry> = HashMap::new();

    let columns = [
        (oids::entity::entry::ENT_PHYSICAL_DESCR, "descr"),
        (oids::entity::entry::ENT_PHYSICAL_CLASS, "class"),
        (oids::entity::entry::ENT_PHYSICAL_NAME, "name"),
        (oids::entity::entry::ENT_PHYSICAL_SERIAL_NUM, "serialNum"),
        (oids::entity::entry::ENT_PHYSICAL_MFG_NAME, "mfgName"),
        (oids::entity::entry::ENT_PHYSICAL_MODEL_NAME, "modelName"),
    ];

    let use_bulk = supports_bulk(credential);
    for (base_oid_str, column_name) in columns {
        walk_column(
            &mut session,
            ip,
            base_oid_str,
            column_name,
            use_bulk,
            |suffix, value| {
                // OID suffix is entPhysicalIndex (single integer)
                if let Some(&index_u64) = suffix.last() {
                    let index = index_u64 as i32;
                    let entry = entries.entry(index).or_insert_with(|| PhysicalEntry {
                        description: None,
                        class: None,
                        name: None,
                        serial_number: None,
                        manufacturer: None,
                        model: None,
                    });

                    match column_name {
                        "descr" => entry.description = value_to_string(value),
                        "class" => entry.class = value_to_i32(value),
                        "name" => entry.name = value_to_string(value),
                        "serialNum" => {
                            entry.serial_number = value_to_string(value).filter(|s| !s.is_empty())
                        }
                        "mfgName" => {
                            entry.manufacturer = value_to_string(value).filter(|s| !s.is_empty())
                        }
                        "modelName" => {
                            entry.model = value_to_string(value).filter(|s| !s.is_empty())
                        }
                        _ => {}
                    }
                }
            },
        )
        .await?;
    }

    // Select best match: prefer chassis (3), fallback to stack (11), then module (9)
    let best = entries
        .values()
        .find(|e| e.class == Some(3))
        .or_else(|| entries.values().find(|e| e.class == Some(11)))
        .or_else(|| entries.values().find(|e| e.class == Some(9)));

    let result = best.map(|e| DeviceInventory {
        description: e.description.clone().or_else(|| e.name.clone()),
        manufacturer: e.manufacturer.clone(),
        model: e.model.clone(),
        serial_number: e.serial_number.clone(),
    });

    debug!(
        "ENTITY-MIB query from {} returned {} physical entries, best match: {}",
        ip,
        entries.len(),
        result.is_some()
    );

    Ok(result)
}

/// Walk dot1dBasePortIfIndex to build bridge_port → ifIndex mapping.
/// Shared by query_bridge_fdb() and query_port_vlan_membership().
async fn walk_bridge_port_mapping(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
    use_bulk: bool,
) -> Result<HashMap<i32, i32>> {
    let mut port_to_if_index: HashMap<i32, i32> = HashMap::new();

    walk_column(
        &mut **session,
        ip,
        oids::bridge::DOT1D_BASE_PORT_IF_INDEX,
        "dot1dBasePortIfIndex",
        use_bulk,
        |suffix, value| {
            // OID suffix is bridge port number, value is ifIndex
            if let Some(&port_u64) = suffix.last() {
                let bridge_port = port_u64 as i32;
                if let Some(if_index) = value_to_i32(value) {
                    port_to_if_index.insert(bridge_port, if_index);
                }
            }
        },
    )
    .await?;

    Ok(port_to_if_index)
}

/// Query bridge FDB (dot1dTpFdbTable) for MAC-to-port mappings.
/// Also walks dot1dBasePortIfIndex to resolve bridge port numbers to ifIndex values.
pub async fn query_bridge_fdb(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Vec<BridgeFdbEntry>> {
    let mut session = create_session(ip, credential, port).await?;

    // Step 1: Walk dot1dBasePortIfIndex to build bridge_port → ifIndex map
    let port_to_if_index =
        walk_bridge_port_mapping(&mut session, ip, supports_bulk(credential)).await?;

    // Step 2: Walk dot1dTpFdbTable columns
    struct FdbBuilder {
        mac_address: Option<mac_address::MacAddress>,
        port: Option<i32>,
        status: Option<i32>,
    }

    let mut fdb_entries: HashMap<String, FdbBuilder> = HashMap::new();

    let columns = [
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_ADDRESS, "address"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_PORT, "port"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_STATUS, "status"),
    ];

    let use_bulk = supports_bulk(credential);
    for (base_oid_str, column_name) in columns {
        walk_column(
            &mut session,
            ip,
            base_oid_str,
            column_name,
            use_bulk,
            |suffix, value| {
                // OID suffix is 6-octet MAC encoded as 6 integers
                if suffix.len() == 6 {
                    let key = suffix
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(".");

                    let entry = fdb_entries.entry(key).or_insert_with(|| FdbBuilder {
                        mac_address: None,
                        port: None,
                        status: None,
                    });

                    match column_name {
                        "address" => entry.mac_address = value_to_mac(value),
                        "port" => entry.port = value_to_i32(value),
                        "status" => entry.status = value_to_i32(value),
                        _ => {}
                    }
                }
            },
        )
        .await?;
    }

    // Filter: keep learned (3) and self (5), resolve bridge port to ifIndex
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

    debug!(
        "Bridge FDB walk from {} returned {} entries ({} port mappings)",
        ip,
        result.len(),
        port_to_if_index.len()
    );

    Ok(result)
}

/// Query local LLDP chassis ID (scalar GETs, not walks).
/// Returns the device's own LLDP identity.
pub async fn query_lldp_local(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Option<LldpLocalInfo>> {
    let mut session = create_session(ip, credential, port).await?;

    // GET lldpLocChassisIdSubtype
    let subtype_oid = parse_oid(oids::lldp::local::LLDP_LOC_CHASSIS_ID_SUBTYPE)?;
    let subtype = match timeout(SNMP_TIMEOUT, session.get(&subtype_oid)).await {
        Ok(Ok(mut response)) => response
            .varbinds
            .next()
            .and_then(|(_, value)| value_to_i32(&value))
            .map(|v| v as u8),
        Ok(Err(e)) => {
            debug!(
                "LLDP local chassis ID subtype GET failed from {}: {:?}",
                ip, e
            );
            None
        }
        Err(_) => {
            debug!("LLDP local chassis ID subtype GET timeout from {}", ip);
            None
        }
    };

    // GET lldpLocChassisId
    let chassis_oid = parse_oid(oids::lldp::local::LLDP_LOC_CHASSIS_ID)?;
    let chassis_bytes = match timeout(SNMP_TIMEOUT, session.get(&chassis_oid)).await {
        Ok(Ok(mut response)) => response.varbinds.next().and_then(|(_, value)| {
            if let Value::OctetString(bytes) = &value {
                Some(bytes.to_vec())
            } else {
                None
            }
        }),
        Ok(Err(e)) => {
            debug!("LLDP local chassis ID GET failed from {}: {:?}", ip, e);
            None
        }
        Err(_) => {
            debug!("LLDP local chassis ID GET timeout from {}", ip);
            None
        }
    };

    match (subtype, chassis_bytes) {
        (Some(subtype), Some(bytes)) => {
            debug!(
                "LLDP local info from {}: subtype={}, bytes_len={}",
                ip,
                subtype,
                bytes.len()
            );
            Ok(Some(LldpLocalInfo {
                chassis_id_subtype: subtype,
                chassis_id_bytes: bytes,
            }))
        }
        _ => {
            debug!("LLDP local info incomplete from {}", ip);
            Ok(None)
        }
    }
}

/// Query VLAN table for VLAN IDs and names.
/// Tries Q-BRIDGE dot1qVlanStaticName first, falls back to Cisco VTP vtpVlanName.
pub async fn query_vlan_table(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Vec<VlanInfo>> {
    let mut session = create_session(ip, credential, port).await?;
    let mut vlans: Vec<VlanInfo> = Vec::new();

    let use_bulk = supports_bulk(credential);

    // Try Q-BRIDGE dot1qVlanStaticName first
    walk_column(
        &mut session,
        ip,
        oids::vlan::q_bridge::DOT1Q_VLAN_STATIC_NAME,
        "dot1qVlanStaticName",
        use_bulk,
        |suffix, value| {
            // OID suffix is VLAN ID
            if let Some(&vlan_u64) = suffix.last() {
                let vlan_id = vlan_u64 as u16;
                if let Some(name) = value_to_string(value) {
                    vlans.push(VlanInfo { vlan_id, name });
                }
            }
        },
    )
    .await?;

    // Fall back to Cisco VTP if Q-BRIDGE returned nothing
    if vlans.is_empty() {
        walk_column(
            &mut session,
            ip,
            oids::vlan::cisco_vtp::VTP_VLAN_NAME,
            "vtpVlanName",
            use_bulk,
            |suffix, value| {
                // VTP index is mgmtDomainIndex.vlanId — use last component as VLAN ID
                if let Some(&vlan_u64) = suffix.last() {
                    let vlan_id = vlan_u64 as u16;
                    if let Some(name) = value_to_string(value) {
                        vlans.push(VlanInfo { vlan_id, name });
                    }
                }
            },
        )
        .await?;
    }

    debug!(
        "VLAN table query from {} returned {} entries (Q-BRIDGE or VTP)",
        ip,
        vlans.len()
    );

    Ok(vlans)
}

/// Query per-port VLAN membership from Q-BRIDGE-MIB.
/// Uses dot1qPvid for native VLANs and dot1qVlanCurrentEgressPorts/UntaggedPorts
/// for tagged VLAN membership. Resolves bridge ports to ifIndex.
pub async fn query_port_vlan_membership(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Vec<PortVlanMembership>> {
    let mut session = create_session(ip, credential, port).await?;

    let use_bulk = supports_bulk(credential);

    // Step 1: Get bridge port → ifIndex mapping
    let port_to_if_index = walk_bridge_port_mapping(&mut session, ip, use_bulk).await?;

    if port_to_if_index.is_empty() {
        debug!(
            "No bridge port mappings from {} — skipping VLAN membership",
            ip
        );
        return Ok(Vec::new());
    }

    // Step 2: Walk dot1qPvid for native VLAN per bridge port
    let mut native_vlans: HashMap<i32, u16> = HashMap::new();
    walk_column(
        &mut session,
        ip,
        oids::vlan::q_bridge::DOT1Q_PVID,
        "dot1qPvid",
        use_bulk,
        |suffix, value| {
            // OID suffix is bridge port number, value is native VLAN ID
            if let Some(&port_u64) = suffix.last() {
                let bridge_port = port_u64 as i32;
                if let Some(vlan_id) = value_to_u16(value) {
                    native_vlans.insert(bridge_port, vlan_id);
                }
            }
        },
    )
    .await?;

    // Step 3: Walk dot1qVlanCurrentEgressPorts — PortList bitmap per VLAN
    // Indexed by timeFilter.vlanId (timeFilter is typically 0)
    let mut egress_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_column(
        &mut session,
        ip,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_EGRESS_PORTS,
        "dot1qVlanCurrentEgressPorts",
        use_bulk,
        |suffix, value| {
            // Suffix is timeFilter.vlanId — last component is VLAN ID
            if let Some(&vlan_u64) = suffix.last() {
                let vlan_id = vlan_u64 as u16;

                // Value is PortList bitmap (OCTET STRING)
                if let Value::OctetString(bytes) = value {
                    let bridge_ports = parse_portlist_bitmap(bytes);
                    for bp in bridge_ports {
                        egress_by_port.entry(bp).or_default().push(vlan_id);
                    }
                }
            }
        },
    )
    .await?;

    // Step 4: Walk dot1qVlanCurrentUntaggedPorts — same bitmap format
    let mut untagged_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_column(
        &mut session,
        ip,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_UNTAGGED_PORTS,
        "dot1qVlanCurrentUntaggedPorts",
        use_bulk,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last() {
                let vlan_id = vlan_u64 as u16;

                if let Value::OctetString(bytes) = value {
                    let bridge_ports = parse_portlist_bitmap(bytes);
                    for bp in bridge_ports {
                        untagged_by_port.entry(bp).or_default().push(vlan_id);
                    }
                }
            }
        },
    )
    .await?;

    // Step 5: Assemble per-port membership, resolving bridge port → ifIndex
    let mut result: Vec<PortVlanMembership> = Vec::new();

    for (&bridge_port, &if_index) in &port_to_if_index {
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

    Ok(result)
}
