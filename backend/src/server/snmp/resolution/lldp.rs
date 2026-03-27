//! LLDP (Link Layer Discovery Protocol) types and resolution.
//!
//! This module provides enums for LLDP identifier types per IEEE 802.1AB,
//! along with resolution methods to convert LLDP neighbor data into
//! database entity references.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use utoipa::ToSchema;
use uuid::Uuid;

/// LLDP Chassis ID subtypes per IEEE 802.1AB.
///
/// The chassis ID identifies the remote device. Different network equipment
/// may use different subtypes depending on configuration and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "subtype", content = "value")]
pub enum LldpChassisId {
    /// Subtype 1: Chassis component (e.g., backplane serial number)
    ChassisComponent(String),
    /// Subtype 2: Interface alias (ifAlias from IF-MIB)
    InterfaceAlias(String),
    /// Subtype 3: Port component (e.g., backplane port number)
    PortComponent(String),
    /// Subtype 4: MAC address (most common)
    MacAddress(String),
    /// Subtype 5: Network address (IP address stored as string)
    #[schema(value_type = String)]
    NetworkAddress(#[serde(with = "ip_addr_serde")] IpAddr),
    /// Subtype 6: Interface name (ifName from IF-MIB)
    InterfaceName(String),
    /// Subtype 7: Locally assigned (device-specific identifier)
    LocallyAssigned(String),
}

/// LLDP Port ID subtypes per IEEE 802.1AB.
///
/// The port ID identifies the specific port on the remote device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "subtype", content = "value")]
pub enum LldpPortId {
    /// Subtype 1: Interface alias (ifAlias from IF-MIB)
    InterfaceAlias(String),
    /// Subtype 2: Port component (e.g., backplane port number)
    PortComponent(String),
    /// Subtype 3: MAC address
    MacAddress(String),
    /// Subtype 4: Network address (IP address stored as string)
    #[schema(value_type = String)]
    NetworkAddress(#[serde(with = "ip_addr_serde")] IpAddr),
    /// Subtype 5: Interface name (ifName from IF-MIB)
    InterfaceName(String),
    /// Subtype 6: Agent circuit ID (used by some providers)
    AgentCircuitId(String),
    /// Subtype 7: Locally assigned (device-specific identifier)
    LocallyAssigned(String),
}

/// Serde helper for IpAddr as string
mod ip_addr_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::net::IpAddr;

    pub fn serialize<S>(ip: &IpAddr, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&ip.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<IpAddr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl LldpChassisId {
    /// Parse from SNMP raw values (subtype byte + value bytes).
    ///
    /// LLDP chassis ID TLV format: subtype (1 byte) + value (variable)
    pub fn from_snmp(subtype: u8, value: &[u8]) -> Option<Self> {
        match subtype {
            1 => Some(Self::ChassisComponent(
                String::from_utf8_lossy(value).to_string(),
            )),
            2 => Some(Self::InterfaceAlias(
                String::from_utf8_lossy(value).to_string(),
            )),
            3 => Some(Self::PortComponent(
                String::from_utf8_lossy(value).to_string(),
            )),
            4 if value.len() == 6 => Some(Self::MacAddress(format_mac(value))),
            5 => parse_network_address(value).map(Self::NetworkAddress),
            6 => Some(Self::InterfaceName(
                String::from_utf8_lossy(value).to_string(),
            )),
            7 => Some(Self::LocallyAssigned(
                String::from_utf8_lossy(value).to_string(),
            )),
            _ => None,
        }
    }

    /// Resolve this chassis ID to a host_id using the appropriate lookup strategy.
    ///
    /// The resolution strategy depends on the chassis ID subtype:
    /// - MacAddress: Look up via interfaces.mac_address → host
    /// - NetworkAddress: Look up via interfaces table (IP address)
    /// - InterfaceName: Look up via if_entries.if_descr
    /// - ChassisComponent/LocallyAssigned: Look up via hosts.chassis_id
    /// - InterfaceAlias/PortComponent: No reliable resolution strategy
    pub async fn resolve_host_id<R: LldpResolver>(
        &self,
        resolver: &R,
        network_id: Uuid,
    ) -> Option<Uuid> {
        match self {
            Self::MacAddress(mac) => resolver.find_host_by_mac(mac, network_id).await,
            Self::NetworkAddress(ip) => resolver.find_host_by_ip(ip, network_id).await,
            Self::InterfaceName(name) => resolver.find_host_by_if_name(name, network_id).await,
            Self::ChassisComponent(id) | Self::LocallyAssigned(id) => {
                resolver.find_host_by_chassis_id(id, network_id).await
            }
            // These subtypes don't have reliable resolution strategies
            Self::InterfaceAlias(_) | Self::PortComponent(_) => None,
        }
    }
}

impl LldpPortId {
    /// Parse from SNMP raw values (subtype byte + value bytes).
    ///
    /// LLDP port ID TLV format: subtype (1 byte) + value (variable)
    pub fn from_snmp(subtype: u8, value: &[u8]) -> Option<Self> {
        match subtype {
            1 => Some(Self::InterfaceAlias(
                String::from_utf8_lossy(value).to_string(),
            )),
            2 => Some(Self::PortComponent(
                String::from_utf8_lossy(value).to_string(),
            )),
            3 if value.len() == 6 => Some(Self::MacAddress(format_mac(value))),
            4 => parse_network_address(value).map(Self::NetworkAddress),
            5 => Some(Self::InterfaceName(
                String::from_utf8_lossy(value).to_string(),
            )),
            6 => Some(Self::AgentCircuitId(
                String::from_utf8_lossy(value).to_string(),
            )),
            7 => Some(Self::LocallyAssigned(
                String::from_utf8_lossy(value).to_string(),
            )),
            _ => None,
        }
    }

    /// Resolve this port ID to an if_entry_id using the appropriate lookup strategy.
    ///
    /// Requires the host_id to be already known (from chassis ID resolution).
    ///
    /// The resolution strategy depends on the port ID subtype:
    /// - MacAddress: Look up via if_entries.mac_address
    /// - InterfaceName/InterfaceAlias: Look up via if_entries.if_descr
    /// - NetworkAddress: Look up via interface_id FK on if_entries
    /// - PortComponent/AgentCircuitId/LocallyAssigned: No reliable resolution
    pub async fn resolve_if_entry_id<R: LldpResolver>(
        &self,
        resolver: &R,
        host_id: Uuid,
    ) -> Option<Uuid> {
        match self {
            Self::MacAddress(mac) => resolver.find_if_entry_by_mac(mac, host_id).await,
            Self::InterfaceName(name) => resolver.find_if_entry_by_name(name, host_id).await,
            Self::InterfaceAlias(_) => None, // user-configurable, non-unique
            Self::NetworkAddress(ip) => resolver.find_if_entry_by_ip(ip, host_id).await,
            // These subtypes don't have reliable resolution strategies
            Self::PortComponent(_) | Self::AgentCircuitId(_) | Self::LocallyAssigned(_) => None,
        }
    }
}

/// Format MAC address bytes as colon-separated hex string.
fn format_mac(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(":")
}

/// Parse LLDP network address format.
///
/// LLDP network address format: address family (1 byte) + address bytes
/// - Family 1: IPv4 (4 bytes)
/// - Family 2: IPv6 (16 bytes)
fn parse_network_address(value: &[u8]) -> Option<IpAddr> {
    if value.is_empty() {
        return None;
    }
    let addr_family = value[0];
    let addr_bytes = &value[1..];
    match addr_family {
        1 if addr_bytes.len() == 4 => Some(IpAddr::V4(std::net::Ipv4Addr::new(
            addr_bytes[0],
            addr_bytes[1],
            addr_bytes[2],
            addr_bytes[3],
        ))),
        2 if addr_bytes.len() == 16 => {
            let arr: [u8; 16] = addr_bytes.try_into().ok()?;
            Some(IpAddr::V6(std::net::Ipv6Addr::from(arr)))
        }
        _ => None,
    }
}

// Re-export LldpResolver trait from resolver module for backward compatibility
pub use super::resolver::LldpResolver;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chassis_id_from_snmp_mac() {
        let mac_bytes = [0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e];
        let chassis_id = LldpChassisId::from_snmp(4, &mac_bytes);
        assert_eq!(
            chassis_id,
            Some(LldpChassisId::MacAddress("00:1a:2b:3c:4d:5e".to_string()))
        );
    }

    #[test]
    fn test_chassis_id_from_snmp_locally_assigned() {
        let id_bytes = b"switch-1";
        let chassis_id = LldpChassisId::from_snmp(7, id_bytes);
        assert_eq!(
            chassis_id,
            Some(LldpChassisId::LocallyAssigned("switch-1".to_string()))
        );
    }

    #[test]
    fn test_chassis_id_from_snmp_ipv4() {
        // Family 1 (IPv4) + 192.168.1.1
        let addr_bytes = [1, 192, 168, 1, 1];
        let chassis_id = LldpChassisId::from_snmp(5, &addr_bytes);
        assert_eq!(
            chassis_id,
            Some(LldpChassisId::NetworkAddress(IpAddr::V4(
                std::net::Ipv4Addr::new(192, 168, 1, 1)
            )))
        );
    }

    #[test]
    fn test_port_id_from_snmp_interface_name() {
        let name_bytes = b"GigabitEthernet0/1";
        let port_id = LldpPortId::from_snmp(5, name_bytes);
        assert_eq!(
            port_id,
            Some(LldpPortId::InterfaceName("GigabitEthernet0/1".to_string()))
        );
    }

    #[test]
    fn test_chassis_id_serialization() {
        let chassis_id = LldpChassisId::MacAddress("00:1a:2b:3c:4d:5e".to_string());
        let json = serde_json::to_string(&chassis_id).unwrap();
        assert_eq!(
            json,
            r#"{"subtype":"MacAddress","value":"00:1a:2b:3c:4d:5e"}"#
        );

        let deserialized: LldpChassisId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, chassis_id);
    }
}
