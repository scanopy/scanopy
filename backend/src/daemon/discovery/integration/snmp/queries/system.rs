//! The device's own view of itself: the system group, the ifTable/ifXTable and the ipAddrTable.

use super::*;

/// Query system MIB information from a device.
///
/// `sysServices` and `ifNumber` are read here alongside the descriptive scalars because they are
/// what the device claims about itself: the bridge bit in the first says it switches, and the
/// second says how many interfaces to expect. Both are compared against what the walks actually
/// return, so a device that short-changes a collection can be reported rather than believed.
pub async fn query_system_info<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SystemInfo> {
    let mut info = SystemInfo::default();

    // Query each system OID
    let oids_to_query = [
        (oids::system::SYS_DESCR, "sysDescr"),
        (oids::system::SYS_OBJECT_ID, "sysObjectID"),
        (oids::system::SYS_NAME, "sysName"),
        (oids::system::SYS_LOCATION, "sysLocation"),
        (oids::system::SYS_CONTACT, "sysContact"),
        (oids::system::SYS_UPTIME, "sysUpTime"),
        (oids::system::SYS_SERVICES, "sysServices"),
        (oids::if_mib::IF_NUMBER, "ifNumber"),
    ];

    for (oid_str, name) in oids_to_query {
        match session.get_scalar(&oids::oid_parts(oid_str)).await {
            Ok(Some(value)) => {
                trace!("SNMP {} from {}: {:?}", name, ip, value);
                match name {
                    "sysDescr" => info.sys_descr = value_to_string(&value),
                    "sysObjectID" => info.sys_object_id = value_to_string(&value),
                    "sysName" => info.sys_name = value_to_string(&value),
                    "sysLocation" => info.sys_location = value_to_string(&value),
                    "sysContact" => info.sys_contact = value_to_string(&value),
                    "sysUpTime" => info.sys_uptime = value_to_u64(&value),
                    "sysServices" => info.sys_services = value_to_i32(&value),
                    "ifNumber" => info.if_number = value_to_i32(&value),
                    _ => {}
                }
            }
            Ok(None) => {
                debug!("SNMP GET {} returned nothing from {}", name, ip);
            }
            Err(e) => {
                debug!("SNMP GET {} failed from {}: {}", name, ip, e);
            }
        }
    }

    Ok(info)
}

/// Walk the ifTable/ifXTable columns.
///
/// See [`IfTableWalk`] for what the two completeness flags mean and why they are separate.
pub async fn walk_if_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<IfTableWalk> {
    let mut entries: HashMap<i32, IfTableEntry> = HashMap::new();
    // Cleared to false the moment any column walk is cut short (error/timeout/limit).
    let mut shortfall = Shortfall::default();
    // Whether the index column specifically survived. `None` until it has been walked.
    let mut index_column_complete: Option<bool> = None;

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

    // ifIndex is walked first and is the table's index column, so once it has returned a
    // non-empty set every later column must land inside it. A row appearing only in a later
    // column is not an interface this device reported — it is a response that doesn't belong to
    // this walk — and minting an interface from it is how a foreign port ended up on a switch.
    // Only trusted when the ifIndex column itself completed; a device that doesn't serve it at
    // all still gets the old permissive behaviour.
    let mut known_if_indexes: Option<HashSet<i32>> = None;
    let mut foreign_rows = 0usize;

    // Walk each column. ifTable/ifXTable are indexed by a single sub-id (ifIndex).
    for (base_oid_str, column_name) in columns {
        let known = known_if_indexes.clone();
        let mut column_indexes: HashSet<i32> = HashSet::new();
        let mut column_foreign = 0usize;
        let walked = walk_subtree(session, ip, base_oid_str, |suffix, value| {
            let Some(&if_index_u64) = suffix.last() else {
                return;
            };
            let if_index = if_index_u64 as i32;
            column_indexes.insert(if_index);
            if let Some(known) = &known
                && !known.contains(&if_index)
            {
                column_foreign += 1;
                return;
            }
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
                "ifIndex" => {} // already set above
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
        })
        .await
        .map(WalkStop::is_complete)
        .unwrap_or(false);

        // A column cut short (timeout/error/limit) means this is NOT an authoritative
        // full ifTable — the server must not prune stale interfaces against it (#649).
        if !walked {
            shortfall.complete = false;
        }

        if column_name == "ifIndex" {
            index_column_complete = Some(walked);
            // A column cut short still names the indexes it *did* return, and a row outside that
            // set is not an interface this device listed — truncated or not. Only a column that
            // returned nothing leaves no basis to judge, and that is the sole case that falls back
            // to accepting whatever the other columns mint. Requiring the column to have finished
            // let a foreign ifIndex through on exactly the scan where the guard was needed most.
            if !column_indexes.is_empty() {
                known_if_indexes = Some(column_indexes);
            }
        }

        if column_foreign > 0 {
            // Something answered for an interface this device never listed. Whatever the cause,
            // what we hold is not a faithful copy of its ifTable.
            foreign_rows += column_foreign;
            shortfall.complete = false;
            tracing::warn!(
                ip = %ip,
                column = column_name,
                rows = column_foreign,
                "SNMP ifTable column returned rows for unknown ifIndexes; discarding them and \
                 marking the walk partial"
            );
        }
    }

    let mut result: Vec<IfTableEntry> = entries.into_values().collect();
    result.sort_by_key(|e| e.if_index);

    // The ifTable keeps its own two-flag model (`set_complete` / `attributes_complete`) rather
    // than reporting a `ShortfallReason`: a truncated interface *set* and a truncated attribute
    // *column* mean different things to the server, and only the first may gate pruning. The
    // accumulator is used here purely for the attribute-column flag.
    let complete = shortfall.complete;

    // A foreign row means something answered for an interface this device never listed, so the
    // set itself is suspect — not just its attributes.
    let set_complete = match index_column_complete {
        // The index column decides membership, so its own completeness is the set's.
        Some(true) => foreign_rows == 0,
        Some(false) => false,
        // A device that serves no index column at all gives us no independent read on membership;
        // fall back to requiring every column, which is what this did before the split.
        None => complete,
    };

    // `complete` distinguishes an authoritative full ifTable from a partial walk cut short by
    // timeout/error. The server prunes stale interfaces only on a complete walk (GH #649), so
    // surface it at debug level for self-hosted daemon-log triage (enable SCANOPY_LOG_LEVEL=debug).
    tracing::debug!(
        ip = %ip,
        if_count = result.len(),
        set_complete = set_complete,
        attributes_complete = complete,
        foreign_rows = foreign_rows,
        "SNMP ifTable walk finished"
    );
    // Diagnostic for issue #614 (high-ifIndex interfaces missing): log the full set of
    // collected ifIndex values, not just the count, so we can tell whether a high-ifIndex
    // switch (e.g. ifIndex 49153-49168) is dropped at walk time or later during ingestion.
    debug!(
        ip = %ip,
        if_indexes = ?result.iter().map(|e| e.if_index).collect::<Vec<_>>(),
        "SNMP ifTable walk ifIndex set"
    );

    Ok(IfTableWalk {
        entries: result,
        set_complete,
        attributes_complete: complete,
    })
}

/// Query ipAddrTable for IP address to ifIndex + subnet mask mappings.
/// Walks ipAdEntIfIndex and ipAdEntNetMask columns where the OID suffix
/// encodes the IP address as A.B.C.D.
pub async fn query_ip_addr_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<HashMap<IpAddr, IpAddrEntry>>> {
    let mut if_index_map: HashMap<IpAddr, i32> = HashMap::new();
    let mut net_mask_map: HashMap<IpAddr, IpAddr> = HashMap::new();
    let mut shortfall = Shortfall::default();

    // Walk ipAdEntIfIndex — OID suffix encodes the IP address as A.B.C.D.
    walk_column(
        session,
        ip,
        oids::ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
        &mut shortfall,
        |suffix, value| {
            if suffix.len() == 4
                && let Some(if_index) = value_to_i32(value)
            {
                let addr = IpAddr::from([
                    suffix[0] as u8,
                    suffix[1] as u8,
                    suffix[2] as u8,
                    suffix[3] as u8,
                ]);
                if_index_map.insert(addr, if_index);
            }
        },
    )
    .await;

    // Walk ipAdEntNetMask
    walk_column(
        session,
        ip,
        oids::ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
        &mut shortfall,
        |suffix, value| {
            if suffix.len() == 4
                && let Some(mask) = value_to_ip(value)
            {
                let addr = IpAddr::from([
                    suffix[0] as u8,
                    suffix[1] as u8,
                    suffix[2] as u8,
                    suffix[3] as u8,
                ]);
                net_mask_map.insert(addr, mask);
            }
        },
    )
    .await;

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

    Ok(SnmpCollection::from_walk(result, shortfall))
}
