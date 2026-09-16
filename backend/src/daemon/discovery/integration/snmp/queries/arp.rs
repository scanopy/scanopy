//! ARP: the ipNetToMediaTable.

use super::*;

/// Query ARP table (ipNetToMediaTable) for IP-to-MAC mappings.
/// Returns entries with ifIndex, MAC, and IP for each ARP cache entry.
pub async fn query_arp_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<ArpEntry>>> {
    // We need to walk 4 columns: ifIndex, physAddress, netAddress, type
    // OID suffix format: ifIndex.A.B.C.D
    struct ArpEntryBuilder {
        if_index: Option<i32>,
        mac_address: Option<mac_address::MacAddress>,
        ip_address: Option<IpAddr>,
        entry_type: Option<i32>,
    }

    let mut entries: HashMap<String, ArpEntryBuilder> = HashMap::new();
    let mut shortfall = Shortfall::default();

    let columns = [
        (oids::arp::entry::IP_NET_TO_MEDIA_IF_INDEX, "ifIndex"),
        (
            oids::arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
            "physAddress",
        ),
        (oids::arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS, "netAddress"),
        (oids::arp::entry::IP_NET_TO_MEDIA_TYPE, "type"),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix: ifIndex.A.B.C.D
        walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                if suffix.len() < 5 {
                    return;
                }
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
            },
        )
        .await;
    }

    let rows_read = entries.len();

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

    // `rows_read` alongside the result is what makes an empty ARP table diagnosable. The entry is
    // a join across four columns and needs all of them, so a column that comes back empty drops
    // every row the others read — reported as "no ARP entries" from a device that answered
    // hundreds of them (GH #674). The two numbers together say which happened.
    debug!(
        ip = %ip,
        entries = result.len(),
        rows_read,
        complete = shortfall.complete,
        "ARP table walk finished"
    );

    Ok(SnmpCollection::from_walk(result, shortfall))
}
