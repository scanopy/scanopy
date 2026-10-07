//! SNMP Result Types
//!
//! Data structures for SNMP query results.

use mac_address::MacAddress;
use std::net::IpAddr;

/// System MIB information retrieved from a device
#[derive(Debug, Clone, Default)]
pub struct SystemInfo {
    /// sysDescr - Full textual description of the entity
    pub sys_descr: Option<String>,
    /// sysObjectID - Vendor's authoritative identification OID
    pub sys_object_id: Option<String>,
    /// sysName - Administratively-assigned name (usually FQDN)
    pub sys_name: Option<String>,
    /// sysLocation - Physical location of this node
    pub sys_location: Option<String>,
    /// sysContact - Contact person for this managed node
    pub sys_contact: Option<String>,
    /// sysUpTime - Time since last re-initialization (hundredths of seconds)
    pub sys_uptime: Option<u64>,
    /// sysServices - the layers this device claims to implement (RFC 1213 bitfield).
    ///
    /// Bit 2 (value 2) is the datalink layer: a device that sets it says it bridges, which is why
    /// an empty bridge table from one is a contradiction rather than a device that simply does
    /// not switch.
    pub sys_services: Option<i32>,
    /// ifNumber - how many interfaces the device says it has, for checking the ifTable walk
    /// against the device's own count rather than only against itself.
    pub if_number: Option<i32>,
}

/// Interface entry from ifTable/ifXTable
#[derive(Debug, Clone, Default)]
pub struct IfTableEntry {
    /// ifIndex - Unique value for each interface
    pub if_index: i32,
    /// ifDescr - Interface description string
    pub if_descr: Option<String>,
    /// ifType - Interface type (IANAifType)
    pub if_type: Option<i32>,
    /// ifMtu - Maximum transmission unit
    pub if_mtu: Option<i32>,
    /// ifSpeed - Interface speed in bits/sec (from ifSpeed or ifHighSpeed)
    pub if_speed: Option<u64>,
    /// ifPhysAddress - MAC address
    pub if_phys_address: Option<MacAddress>,
    /// ifAdminStatus - Desired state: 1=up, 2=down, 3=testing
    pub if_admin_status: Option<i32>,
    /// ifOperStatus - Current state: 1=up, 2=down, etc.
    pub if_oper_status: Option<i32>,
    /// ifName - Textual name of interface (from ifXTable)
    pub if_name: Option<String>,
    /// ifAlias - User-configured description (from ifXTable)
    pub if_alias: Option<String>,
}

/// LLDP neighbor information
#[derive(Debug, Clone)]
pub struct LldpNeighbor {
    /// Local port ifIndex where neighbor was seen
    pub local_port_index: i32,
    /// Remote chassis ID subtype (lldpRemChassisIdSubtype)
    pub remote_chassis_id_subtype: Option<u8>,
    /// Remote chassis ID raw bytes (lldpRemChassisId)
    pub remote_chassis_id_bytes: Option<Vec<u8>>,
    /// Remote port ID subtype (lldpRemPortIdSubtype)
    pub remote_port_id_subtype: Option<u8>,
    /// Remote port ID raw bytes (lldpRemPortId)
    pub remote_port_id_bytes: Option<Vec<u8>>,
    /// Remote port description
    pub remote_port_desc: Option<String>,
    /// Remote system name
    pub remote_sys_name: Option<String>,
    /// Remote system description
    pub remote_sys_desc: Option<String>,
    /// Remote management address (if available)
    pub remote_mgmt_addr: Option<IpAddr>,
}

/// Local port entry from lldpLocPortTable, keyed by lldpLocPortNum.
/// Used to translate an LLDP-local port number (the local-port index reported in
/// lldpRemTable) back to the device's real ifIndex.
#[derive(Debug, Clone, Default)]
pub struct LldpLocalPort {
    /// lldpLocPortIdSubtype (5 = interfaceName, 2 = interfaceIndex, 3 = macAddress, ...)
    pub port_id_subtype: Option<u8>,
    /// lldpLocPortId rendered as text (e.g. "1:4", "1/1", "mgmt")
    pub port_id: Option<String>,
    /// The same column read as six raw octets, which is what subtype 3 (macAddress) sends.
    /// Held separately because that encoding is not text and does not survive being read as
    /// one — the port then had no usable identifier at all.
    pub port_id_mac: Option<mac_address::MacAddress>,
    /// lldpLocPortDesc — free text, and on some vendors the only column naming the interface.
    pub port_desc: Option<String>,
}

/// IP address table entry from ipAddrTable
#[derive(Debug, Clone)]
pub struct IpAddrEntry {
    /// ifIndex for the interface with this IP
    pub if_index: i32,
    /// Subnet mask for this IP address
    pub net_mask: Option<IpAddr>,
}

/// ARP table entry from ipNetToMediaTable
#[derive(Debug, Clone)]
pub struct ArpEntry {
    /// Interface index where this ARP entry was learned
    pub if_index: i32,
    /// MAC address of the remote host
    pub mac_address: MacAddress,
    /// IP address of the remote host
    pub ip_address: IpAddr,
}

/// Hardware inventory from ENTITY-MIB entPhysicalTable
#[derive(Debug, Clone, Default)]
pub struct DeviceInventory {
    /// entPhysicalDescr - description of the physical entity
    pub description: Option<String>,
    /// entPhysicalMfgName - manufacturer name
    pub manufacturer: Option<String>,
    /// entPhysicalModelName - model name
    pub model: Option<String>,
    /// entPhysicalSerialNum - serial number
    pub serial_number: Option<String>,
    /// entPhysicalAssetID - asset tag an administrator set on the device. Empty on most devices,
    /// since nothing sets it but a person.
    pub asset_id: Option<String>,
    /// entPhysicalFirmwareRev - firmware revision
    pub firmware_revision: Option<String>,
    /// entPhysicalSoftwareRev - software revision
    ///
    /// Kept apart from the firmware revision rather than folded into it: RFC 4133 defines them as
    /// distinct objects, and a chassis reporting both is reporting two different versions.
    pub software_revision: Option<String>,
}

/// Bridge FDB entry from dot1dTpFdbTable
#[derive(Debug, Clone)]
pub struct BridgeFdbEntry {
    /// MAC address learned on this port
    pub mac_address: MacAddress,
    /// Bridge port number (not ifIndex)
    pub bridge_port: i32,
    /// Resolved ifIndex from dot1dBasePortIfIndex (None if unresolvable)
    pub if_index: Option<i32>,
    /// FDB entry status: 1=other, 2=invalid, 3=learned, 4=self, 5=mgmt
    pub status: i32,
}

/// Local LLDP information from lldpLocalSystemData
#[derive(Debug, Clone)]
pub struct LldpLocalInfo {
    /// lldpLocChassisIdSubtype - type of chassis ID
    pub chassis_id_subtype: u8,
    /// lldpLocChassisId - raw chassis ID bytes
    pub chassis_id_bytes: Vec<u8>,
}

/// CDP neighbor information (Cisco proprietary)
#[derive(Debug, Clone)]
pub struct CdpNeighbor {
    /// Local port ifIndex where neighbor was seen
    pub local_port_index: i32,
    /// Remote device ID (typically hostname)
    pub remote_device_id: Option<String>,
    /// Remote port ID string
    pub remote_port_id: Option<String>,
    /// Remote device platform
    pub remote_platform: Option<String>,
    /// Remote device IP address
    pub remote_address: Option<IpAddr>,
}

/// VLAN information from dot1qVlanStaticTable or Cisco VTP
#[derive(Debug, Clone)]
pub struct VlanInfo {
    /// 802.1Q VLAN ID (1-4094)
    pub vlan_id: u16,
    /// VLAN name (e.g., "default", "management")
    pub name: String,
}

/// Per-port VLAN membership from Q-BRIDGE-MIB
#[derive(Debug, Clone)]
pub struct PortVlanMembership {
    /// ifIndex of the port (resolved from bridge port number)
    pub if_index: i32,
    /// Native/untagged VLAN (dot1qPvid)
    pub native_vlan: Option<u16>,
    /// Tagged VLAN IDs (egress VLANs minus untagged VLANs)
    pub tagged_vlans: Vec<u16>,
}
