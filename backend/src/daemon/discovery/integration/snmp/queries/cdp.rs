//! CDP: the Cisco neighbour cache.

use super::*;

/// Query CDP cache table for neighbor information (Cisco devices)
pub async fn query_cdp_neighbors<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<CdpNeighbor>>> {
    let mut neighbors: HashMap<(i32, i32), CdpNeighbor> = HashMap::new();
    let mut shortfall = Shortfall::default();

    let columns = [
        (oids::cdp::entry::CDP_CACHE_DEVICE_ID, "deviceId"),
        (oids::cdp::entry::CDP_CACHE_DEVICE_PORT, "devicePort"),
        (oids::cdp::entry::CDP_CACHE_PLATFORM, "platform"),
        (oids::cdp::entry::CDP_CACHE_ADDRESS, "address"),
    ];

    // See `query_lldp_neighbors`: the same distinction, for the same reason. CDP-MIB is a Cisco
    // enterprise MIB, so the overwhelming majority of devices answer `noSuchObject` for it.
    let mut all_columns_unsupported = true;

    for (base_oid_str, column_name) in columns {
        // CDP index: cdpCacheIfIndex.cdpCacheDeviceIndex
        let stop = walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                if suffix.len() < 2 {
                    return;
                }
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
            },
        )
        .await;
        if !stop.is_unsupported() {
            all_columns_unsupported = false;
        }
    }

    // cdpCacheDeviceId is what L2 resolution matches on, so a record without one is the CDP
    // analogue of a chassis-less LLDP neighbour: unusable, and destructive if it overwrites.
    let before = neighbors.len();
    let result: Vec<CdpNeighbor> = neighbors
        .into_values()
        .filter(|n| n.remote_device_id.is_some())
        .collect();
    // CDP has no separate index column to compare against, so the walk's own outcome is the only
    // evidence of why the id is absent: a column that stopped early can recover on a rescan, and a
    // walk that ran to the end and still produced idless rows never will.
    let discard_reason = (result.len() != before).then(|| {
        if shortfall.reason.is_some() {
            MalformedNeighbourReason::WalkCutShort
        } else {
            MalformedNeighbourReason::GhostRows
        }
    });
    if result.len() != before {
        shortfall.complete = false;
        warn!(
            ip = %ip,
            dropped = before - result.len(),
            reason = ?discard_reason,
            "CDP neighbours missing a device id; discarding them and marking the walk partial"
        );
    }
    let unsupported = all_columns_unsupported && result.is_empty();
    debug!(
        ip = %ip,
        neighbors = result.len(),
        complete = shortfall.complete,
        unsupported,
        "CDP query finished"
    );

    Ok(SnmpCollection {
        discarded: before - result.len(),
        discard_reason,
        records: result,
        complete: shortfall.complete,
        unsupported,
        reason: shortfall.reason,
        // The LLDP/CDP claim is the device's own local identity, which is read later in the
        // collection than this walk, so the caller attaches it.
        claim: None,
        local_port_is_if_index: false,
    })
}
