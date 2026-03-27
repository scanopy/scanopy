//! SNMP OID Constants and Utilities
//!
//! Centralized repository for SNMP MIB OIDs used in discovery and monitoring.
//! All OIDs are from standard MIBs unless otherwise noted.

use anyhow::{Result, anyhow};
use snmp2::Oid;

/// Parse an OID string into snmp2::Oid
pub fn parse_oid(oid_str: &str) -> Result<Oid<'static>> {
    let parts: Vec<u64> = oid_str
        .split('.')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<u64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow!("Invalid OID {}: {}", oid_str, e))?;

    Oid::from(parts.as_slice())
        .map_err(|e| anyhow!("Failed to create OID from {}: {:?}", oid_str, e))
}

/// Convert an OID to a Vec<u64> for manipulation
pub fn oid_to_vec(oid: &Oid) -> Vec<u64> {
    oid.iter().map(|iter| iter.collect()).unwrap_or_default()
}

/// System MIB OIDs (RFC 3418)
pub mod system {
    /// sysDescr.0 - Full textual description of the entity
    pub const SYS_DESCR: &str = "1.3.6.1.2.1.1.1.0";

    /// sysObjectID.0 - Vendor's authoritative identification OID
    pub const SYS_OBJECT_ID: &str = "1.3.6.1.2.1.1.2.0";

    /// sysUpTime.0 - Time since last re-initialization (hundredths of seconds)
    pub const SYS_UPTIME: &str = "1.3.6.1.2.1.1.3.0";

    /// sysContact.0 - Contact person for this managed node
    pub const SYS_CONTACT: &str = "1.3.6.1.2.1.1.4.0";

    /// sysName.0 - Administratively-assigned name (usually FQDN)
    pub const SYS_NAME: &str = "1.3.6.1.2.1.1.5.0";

    /// sysLocation.0 - Physical location of this node
    pub const SYS_LOCATION: &str = "1.3.6.1.2.1.1.6.0";

    /// sysServices.0 - Set of services available (bitfield)
    pub const SYS_SERVICES: &str = "1.3.6.1.2.1.1.7.0";
}

/// IF-MIB OIDs (RFC 2863)
pub mod if_mib {
    /// ifTable - Interface table
    pub const IF_TABLE: &str = "1.3.6.1.2.1.2.2";

    /// ifNumber.0 - Number of network interfaces
    pub const IF_NUMBER: &str = "1.3.6.1.2.1.2.1.0";

    /// ifEntry - Entry in interface table
    pub const IF_ENTRY: &str = "1.3.6.1.2.1.2.2.1";

    /// Column OIDs within ifEntry (append .ifIndex for specific interface)
    pub mod columns {
        /// ifIndex - Unique value for each interface
        pub const IF_INDEX: &str = "1.3.6.1.2.1.2.2.1.1";

        /// ifDescr - Interface description string
        pub const IF_DESCR: &str = "1.3.6.1.2.1.2.2.1.2";

        /// ifType - Interface type (IANAifType)
        pub const IF_TYPE: &str = "1.3.6.1.2.1.2.2.1.3";

        /// ifMtu - Maximum transmission unit
        pub const IF_MTU: &str = "1.3.6.1.2.1.2.2.1.4";

        /// ifSpeed - Interface speed in bits/sec (up to 4Gbps, use ifHighSpeed for faster)
        pub const IF_SPEED: &str = "1.3.6.1.2.1.2.2.1.5";

        /// ifPhysAddress - MAC address
        pub const IF_PHYS_ADDRESS: &str = "1.3.6.1.2.1.2.2.1.6";

        /// ifAdminStatus - Desired state: 1=up, 2=down, 3=testing
        pub const IF_ADMIN_STATUS: &str = "1.3.6.1.2.1.2.2.1.7";

        /// ifOperStatus - Current state: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown
        pub const IF_OPER_STATUS: &str = "1.3.6.1.2.1.2.2.1.8";

        /// ifLastChange - sysUpTime when interface entered current state
        pub const IF_LAST_CHANGE: &str = "1.3.6.1.2.1.2.2.1.9";
    }

    /// ifXTable - Extended interface table (IF-MIB)
    pub mod if_x_table {
        /// ifXEntry - Entry in extended interface table
        pub const IF_X_ENTRY: &str = "1.3.6.1.2.1.31.1.1.1";

        /// ifName - Textual name of interface
        pub const IF_NAME: &str = "1.3.6.1.2.1.31.1.1.1.1";

        /// ifHighSpeed - Interface speed in Mbps (for interfaces > 4Gbps)
        pub const IF_HIGH_SPEED: &str = "1.3.6.1.2.1.31.1.1.1.15";

        /// ifAlias - User-configured description (configurable by NMS)
        pub const IF_ALIAS: &str = "1.3.6.1.2.1.31.1.1.1.18";
    }
}

/// IP-MIB OIDs (RFC 4293)
pub mod ip_mib {
    /// ipAddrTable - IP address to interface mapping
    pub const IP_ADDR_TABLE: &str = "1.3.6.1.2.1.4.20";

    /// ipAddressTable - IPv4/IPv6 address table (newer)
    pub const IP_ADDRESS_TABLE: &str = "1.3.6.1.2.1.4.34";

    /// ipAddrEntry columns
    pub mod ip_addr_entry {
        /// ipAdEntAddr - IP address
        pub const IP_AD_ENT_ADDR: &str = "1.3.6.1.2.1.4.20.1.1";

        /// ipAdEntIfIndex - Interface ifIndex
        pub const IP_AD_ENT_IF_INDEX: &str = "1.3.6.1.2.1.4.20.1.2";

        /// ipAdEntNetMask - Subnet mask
        pub const IP_AD_ENT_NET_MASK: &str = "1.3.6.1.2.1.4.20.1.3";
    }
}

/// LLDP-MIB OIDs (IEEE 802.1AB)
pub mod lldp {
    /// lldpLocalSystemData - Local system information
    pub mod local {
        /// lldpLocChassisId - Local chassis ID (globally unique device identifier)
        pub const LLDP_LOC_CHASSIS_ID: &str = "1.0.8802.1.1.2.1.3.2.0";

        /// lldpLocChassisIdSubtype - Type of chassis ID
        pub const LLDP_LOC_CHASSIS_ID_SUBTYPE: &str = "1.0.8802.1.1.2.1.3.1.0";

        /// lldpLocSysName - Local system name
        pub const LLDP_LOC_SYS_NAME: &str = "1.0.8802.1.1.2.1.3.3.0";

        /// lldpLocSysDesc - Local system description
        pub const LLDP_LOC_SYS_DESC: &str = "1.0.8802.1.1.2.1.3.4.0";
    }

    /// lldpRemTable - Remote device information table
    pub mod remote {
        /// lldpRemTable - Table of information about remote devices
        pub const LLDP_REM_TABLE: &str = "1.0.8802.1.1.2.1.4.1";

        /// lldpRemEntry columns
        pub mod entry {
            /// lldpRemChassisId - Remote device chassis ID
            pub const LLDP_REM_CHASSIS_ID: &str = "1.0.8802.1.1.2.1.4.1.1.5";

            /// lldpRemChassisIdSubtype - Type of remote chassis ID
            pub const LLDP_REM_CHASSIS_ID_SUBTYPE: &str = "1.0.8802.1.1.2.1.4.1.1.4";

            /// lldpRemPortId - Remote port identifier
            pub const LLDP_REM_PORT_ID: &str = "1.0.8802.1.1.2.1.4.1.1.7";

            /// lldpRemPortIdSubtype - Type of remote port ID
            pub const LLDP_REM_PORT_ID_SUBTYPE: &str = "1.0.8802.1.1.2.1.4.1.1.6";

            /// lldpRemPortDesc - Remote port description
            pub const LLDP_REM_PORT_DESC: &str = "1.0.8802.1.1.2.1.4.1.1.8";

            /// lldpRemSysName - Remote system name
            pub const LLDP_REM_SYS_NAME: &str = "1.0.8802.1.1.2.1.4.1.1.9";

            /// lldpRemSysDesc - Remote system description
            pub const LLDP_REM_SYS_DESC: &str = "1.0.8802.1.1.2.1.4.1.1.10";

            /// lldpRemManAddrIfSubtype - Management address interface subtype
            pub const LLDP_REM_MAN_ADDR_IF_SUBTYPE: &str = "1.0.8802.1.1.2.1.4.2.1.3";

            /// lldpRemManAddr - Management address
            pub const LLDP_REM_MAN_ADDR: &str = "1.0.8802.1.1.2.1.4.2.1.2";
        }
    }
}

/// CDP-MIB OIDs (Cisco proprietary)
pub mod cdp {
    /// cdpCacheTable - CDP neighbor cache table
    pub const CDP_CACHE_TABLE: &str = "1.3.6.1.4.1.9.9.23.1.2.1";

    /// cdpCacheEntry columns
    pub mod entry {
        /// cdpCacheDeviceId - Remote device ID (typically hostname)
        pub const CDP_CACHE_DEVICE_ID: &str = "1.3.6.1.4.1.9.9.23.1.2.1.1.6";

        /// cdpCacheDevicePort - Remote port ID string
        pub const CDP_CACHE_DEVICE_PORT: &str = "1.3.6.1.4.1.9.9.23.1.2.1.1.7";

        /// cdpCachePlatform - Remote device platform
        pub const CDP_CACHE_PLATFORM: &str = "1.3.6.1.4.1.9.9.23.1.2.1.1.8";

        /// cdpCacheAddress - Remote device address
        pub const CDP_CACHE_ADDRESS: &str = "1.3.6.1.4.1.9.9.23.1.2.1.1.4";

        /// cdpCacheAddressType - Address type (1=IP, etc.)
        pub const CDP_CACHE_ADDRESS_TYPE: &str = "1.3.6.1.4.1.9.9.23.1.2.1.1.3";

        /// cdpCacheCapabilities - Device capabilities
        pub const CDP_CACHE_CAPABILITIES: &str = "1.3.6.1.4.1.9.9.23.1.2.1.1.9";
    }
}

/// ENTITY-MIB OIDs (RFC 4133) - Physical entity information
pub mod entity {
    /// entPhysicalTable - Physical entity table
    pub const ENT_PHYSICAL_TABLE: &str = "1.3.6.1.2.1.47.1.1.1";

    /// entPhysicalEntry columns
    pub mod entry {
        /// entPhysicalDescr - Physical entity description
        pub const ENT_PHYSICAL_DESCR: &str = "1.3.6.1.2.1.47.1.1.1.1.2";

        /// entPhysicalVendorType - Vendor OID
        pub const ENT_PHYSICAL_VENDOR_TYPE: &str = "1.3.6.1.2.1.47.1.1.1.1.3";

        /// entPhysicalClass - Entity class (chassis, module, port, etc.)
        pub const ENT_PHYSICAL_CLASS: &str = "1.3.6.1.2.1.47.1.1.1.1.5";

        /// entPhysicalName - Entity name
        pub const ENT_PHYSICAL_NAME: &str = "1.3.6.1.2.1.47.1.1.1.1.7";

        /// entPhysicalSerialNum - Serial number
        pub const ENT_PHYSICAL_SERIAL_NUM: &str = "1.3.6.1.2.1.47.1.1.1.1.11";

        /// entPhysicalMfgName - Manufacturer name
        pub const ENT_PHYSICAL_MFG_NAME: &str = "1.3.6.1.2.1.47.1.1.1.1.12";

        /// entPhysicalModelName - Model name
        pub const ENT_PHYSICAL_MODEL_NAME: &str = "1.3.6.1.2.1.47.1.1.1.1.13";
    }
}

/// ARP/MAC address table OIDs
pub mod arp {
    /// atTable - Address translation table (deprecated but widely supported)
    pub const AT_TABLE: &str = "1.3.6.1.2.1.3.1";

    /// ipNetToMediaTable - IP to MAC address mapping
    pub const IP_NET_TO_MEDIA_TABLE: &str = "1.3.6.1.2.1.4.22";

    /// ipNetToMediaEntry columns
    pub mod entry {
        /// ipNetToMediaIfIndex - Interface index
        pub const IP_NET_TO_MEDIA_IF_INDEX: &str = "1.3.6.1.2.1.4.22.1.1";

        /// ipNetToMediaPhysAddress - MAC address
        pub const IP_NET_TO_MEDIA_PHYS_ADDRESS: &str = "1.3.6.1.2.1.4.22.1.2";

        /// ipNetToMediaNetAddress - IP address
        pub const IP_NET_TO_MEDIA_NET_ADDRESS: &str = "1.3.6.1.2.1.4.22.1.3";

        /// ipNetToMediaType - Entry type (1=other, 2=invalid, 3=dynamic, 4=static)
        pub const IP_NET_TO_MEDIA_TYPE: &str = "1.3.6.1.2.1.4.22.1.4";
    }
}

/// Bridge MIB OIDs (RFC 4188) - MAC forwarding table
pub mod bridge {
    /// dot1dTpFdbTable - Transparent bridge forwarding database
    pub const DOT1D_TP_FDB_TABLE: &str = "1.3.6.1.2.1.17.4.3";

    /// dot1qTpFdbTable - VLAN-aware forwarding database (Q-BRIDGE-MIB)
    pub const DOT1Q_TP_FDB_TABLE: &str = "1.3.6.1.2.1.17.7.1.2.2";

    /// dot1dBasePortIfIndex - bridge port to ifIndex mapping
    pub const DOT1D_BASE_PORT_IF_INDEX: &str = "1.3.6.1.2.1.17.1.4.1.2";

    /// dot1dTpFdbEntry columns
    pub mod fdb_entry {
        /// dot1dTpFdbAddress - MAC address
        pub const DOT1D_TP_FDB_ADDRESS: &str = "1.3.6.1.2.1.17.4.3.1.1";

        /// dot1dTpFdbPort - Bridge port number
        pub const DOT1D_TP_FDB_PORT: &str = "1.3.6.1.2.1.17.4.3.1.2";

        /// dot1dTpFdbStatus - Entry status
        pub const DOT1D_TP_FDB_STATUS: &str = "1.3.6.1.2.1.17.4.3.1.3";
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_oid() {
        let oid = parse_oid("1.3.6.1.2.1.1.1.0").unwrap();
        let parts = oid_to_vec(&oid);
        assert_eq!(parts, vec![1, 3, 6, 1, 2, 1, 1, 1, 0]);
    }
}
