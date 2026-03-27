//! SNMP Collection Module
//!
//! Provides functions to query SNMP-enabled devices during network discovery.
//! Supports system MIB queries, ifTable walks, and LLDP/CDP neighbor discovery.

pub mod oids;
pub mod queries;
pub mod session;
pub mod types;
pub mod values;

// Re-export commonly used items
pub use queries::{
    query_arp_table, query_bridge_fdb, query_cdp_neighbors, query_entity_physical,
    query_ip_addr_table, query_lldp_local, query_lldp_neighbors, query_system_info, walk_if_table,
};
pub use session::SNMP_WALK_TIMEOUT;
pub use types::{
    ArpEntry, BridgeFdbEntry, CdpNeighbor, DeviceInventory, IfTableEntry, IpAddrEntry,
    LldpLocalInfo, LldpNeighbor, SystemInfo,
};

use anyhow::Result;
use std::net::IpAddr;
use tokio::time::timeout;
use tracing::debug;

use crate::server::credentials::r#impl::mapping::SnmpQueryCredential;

/// Perform a complete SNMP poll of a device
/// Returns system info, interface table, and neighbor information
#[allow(dead_code)] // Used during SNMP discovery integration
pub async fn poll_device(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<(
    SystemInfo,
    Vec<IfTableEntry>,
    Vec<LldpNeighbor>,
    Vec<CdpNeighbor>,
)> {
    debug!("Starting SNMP poll of {}", ip);

    // Query system info first to verify SNMP is working
    let system_info = timeout(SNMP_WALK_TIMEOUT, query_system_info(ip, credential, port))
        .await
        .map_err(|_| anyhow::anyhow!("System info query timeout"))??;

    // Walk interface table
    let if_entries = timeout(SNMP_WALK_TIMEOUT, walk_if_table(ip, credential, port))
        .await
        .map_err(|_| anyhow::anyhow!("ifTable walk timeout"))?
        .unwrap_or_default();

    // Query LLDP neighbors (may fail if not supported)
    let lldp_neighbors = timeout(
        SNMP_WALK_TIMEOUT,
        query_lldp_neighbors(ip, credential, port),
    )
    .await
    .unwrap_or(Ok(vec![]))
    .unwrap_or_default();

    // Query CDP neighbors (may fail if not Cisco or not supported)
    let cdp_neighbors = timeout(SNMP_WALK_TIMEOUT, query_cdp_neighbors(ip, credential, port))
        .await
        .unwrap_or(Ok(vec![]))
        .unwrap_or_default();

    debug!(
        "SNMP poll of {} complete: {} interfaces, {} LLDP neighbors, {} CDP neighbors",
        ip,
        if_entries.len(),
        lldp_neighbors.len(),
        cdp_neighbors.len()
    );

    Ok((system_info, if_entries, lldp_neighbors, cdp_neighbors))
}

#[cfg(test)]
mod tests {
    use super::values::{value_to_i32, value_to_mac, value_to_string};
    use snmp2::Value;

    #[test]
    fn test_value_to_string() {
        let value = Value::OctetString(b"test string");
        assert_eq!(value_to_string(&value), Some("test string".to_string()));
    }

    #[test]
    fn test_value_to_i32() {
        let value = Value::Integer(42);
        assert_eq!(value_to_i32(&value), Some(42));
    }

    #[test]
    fn test_value_to_mac() {
        let mac_bytes: [u8; 6] = [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34];
        let value = Value::OctetString(&mac_bytes);
        let mac = value_to_mac(&value).unwrap();
        assert_eq!(mac.bytes(), [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34]);
    }
}
