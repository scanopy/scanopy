use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};
use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Resolved LLDP/CDP neighbor connection.
///
/// Represents the remote endpoint this port connects to, discovered via LLDP or CDP.
/// The two variants are mutually exclusive and represent different resolution states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "type", content = "id")]
pub enum Neighbor {
    /// Full resolution - the specific remote port was identified
    IfEntry(Uuid),
    /// Partial resolution - the remote device was identified but not the specific port
    Host(Uuid),
}

impl Neighbor {
    /// Get the IfEntry ID if this is a full resolution
    pub fn if_entry_id(&self) -> Option<Uuid> {
        match self {
            Neighbor::IfEntry(id) => Some(*id),
            Neighbor::Host(_) => None,
        }
    }

    /// Returns true if this is a full resolution (specific port known)
    pub fn is_full_resolution(&self) -> bool {
        matches!(self, Neighbor::IfEntry(_))
    }

    /// Returns true if this is a partial resolution (only host known)
    pub fn is_partial_resolution(&self) -> bool {
        matches!(self, Neighbor::Host(_))
    }
}

/// SNMP ifAdminStatus values per IF-MIB RFC 2863
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema)]
#[repr(i32)]
pub enum IfAdminStatus {
    #[default]
    Up = 1,
    Down = 2,
    Testing = 3,
}

impl From<i32> for IfAdminStatus {
    fn from(value: i32) -> Self {
        match value {
            1 => IfAdminStatus::Up,
            2 => IfAdminStatus::Down,
            3 => IfAdminStatus::Testing,
            _ => IfAdminStatus::Up,
        }
    }
}

impl From<IfAdminStatus> for i32 {
    fn from(value: IfAdminStatus) -> Self {
        value as i32
    }
}

/// SNMP ifOperStatus values per IF-MIB RFC 2863
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema)]
#[repr(i32)]
pub enum IfOperStatus {
    #[default]
    Up = 1,
    Down = 2,
    Testing = 3,
    Unknown = 4,
    Dormant = 5,
    NotPresent = 6,
    LowerLayerDown = 7,
}

impl From<i32> for IfOperStatus {
    fn from(value: i32) -> Self {
        match value {
            1 => IfOperStatus::Up,
            2 => IfOperStatus::Down,
            3 => IfOperStatus::Testing,
            4 => IfOperStatus::Unknown,
            5 => IfOperStatus::Dormant,
            6 => IfOperStatus::NotPresent,
            7 => IfOperStatus::LowerLayerDown,
            _ => IfOperStatus::Unknown,
        }
    }
}

impl From<IfOperStatus> for i32 {
    fn from(value: IfOperStatus) -> Self {
        value as i32
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct IfEntryBase {
    pub host_id: Uuid,
    pub network_id: Uuid,
    /// SNMP ifIndex - stable identifier within device
    pub if_index: i32,
    /// SNMP ifDescr - interface description (e.g., GigabitEthernet0/1)
    #[validate(length(min = 1, message = "Interface description is required"))]
    pub if_descr: String,
    /// SNMP ifName - short interface name (e.g., Gi1/0/1)
    pub if_name: Option<String>,
    /// SNMP ifAlias - user-configured description
    pub if_alias: Option<String>,
    /// SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)
    pub if_type: i32,
    /// Interface speed from ifSpeed/ifHighSpeed in bits per second
    pub speed_bps: Option<i64>,
    /// SNMP ifAdminStatus: 1=up, 2=down, 3=testing
    pub admin_status: IfAdminStatus,
    /// SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown
    pub oper_status: IfOperStatus,

    // Local links
    /// MAC address from SNMP ifPhysAddress - immutable once set
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub mac_address: Option<MacAddress>,
    /// FK to Interface entity - this port's IP assignment (must be on same host)
    pub interface_id: Option<Uuid>,

    // Neighbor resolution (LLDP/CDP) - remote endpoint this port connects to
    /// Resolved neighbor connection (mutually exclusive: either IfEntry or Host)
    pub neighbor: Option<Neighbor>,

    // Raw LLDP data (from SNMP lldpRemTable, used for resolution and display)
    /// Remote chassis identifier from LLDP neighbor (globally/locally unique)
    pub lldp_chassis_id: Option<LldpChassisId>,
    /// Remote port identifier from LLDP neighbor
    pub lldp_port_id: Option<LldpPortId>,
    /// Remote system name from LLDP neighbor (lldpRemSysName)
    pub lldp_sys_name: Option<String>,
    /// Remote port description from LLDP neighbor (lldpRemPortDesc)
    pub lldp_port_desc: Option<String>,
    /// Remote management IP from LLDP neighbor (lldpRemManAddr)
    #[schema(value_type = Option<String>)]
    pub lldp_mgmt_addr: Option<std::net::IpAddr>,
    /// Remote system description from LLDP neighbor (lldpRemSysDesc) - platform info
    pub lldp_sys_desc: Option<String>,

    // Raw CDP data (from SNMP cdpCacheTable, Cisco devices)
    /// Remote device ID from CDP (typically hostname, locally unique)
    pub cdp_device_id: Option<String>,
    /// Remote port ID from CDP
    pub cdp_port_id: Option<String>,
    /// Remote platform from CDP (e.g., "Cisco IOS")
    pub cdp_platform: Option<String>,
    /// Remote management IP from CDP (cdpCacheAddress)
    #[schema(value_type = Option<String>)]
    pub cdp_address: Option<std::net::IpAddr>,

    /// Bridge FDB: learned MAC addresses on this switch port.
    /// Single-MAC ports can be resolved to neighbor links server-side.
    /// Multi-MAC ports indicate uplinks where LLDP/CDP is the better source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fdb_macs: Option<Vec<String>>,
}

impl Default for IfEntryBase {
    fn default() -> Self {
        Self {
            host_id: Uuid::nil(),
            network_id: Uuid::nil(),
            if_index: 0,
            if_descr: String::new(),
            if_name: None,
            if_alias: None,
            if_type: 1, // other
            speed_bps: None,
            admin_status: IfAdminStatus::Up,
            oper_status: IfOperStatus::Up,
            mac_address: None,
            interface_id: None,
            neighbor: None,
            lldp_chassis_id: None,
            lldp_port_id: None,
            lldp_sys_name: None,
            lldp_port_desc: None,
            lldp_mgmt_addr: None,
            lldp_sys_desc: None,
            cdp_device_id: None,
            cdp_port_id: None,
            cdp_platform: None,
            cdp_address: None,
            fdb_macs: None,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema, Validate,
)]
pub struct IfEntry {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: IfEntryBase,
}

impl ChangeTriggersTopologyStaleness<IfEntry> for IfEntry {
    fn triggers_staleness(&self, other: Option<IfEntry>) -> bool {
        if let Some(other_entry) = other {
            // Topology changes if neighbor changes (link discovery)
            self.base.neighbor != other_entry.base.neighbor
                || self.base.interface_id != other_entry.base.interface_id
                || self.base.host_id != other_entry.base.host_id
        } else {
            true // New or deleted entry triggers staleness
        }
    }
}

impl Display for IfEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IfEntry {} (ifIndex {}): {}",
            self.id, self.base.if_index, self.base.if_descr
        )
    }
}

impl IfEntry {
    pub fn new(base: IfEntryBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }

    /// Returns true if interface is operationally up
    pub fn is_up(&self) -> bool {
        self.base.oper_status == IfOperStatus::Up
    }

    /// Returns true if interface is administratively up
    pub fn is_admin_up(&self) -> bool {
        self.base.admin_status == IfAdminStatus::Up
    }

    /// Get display name - prefer ifAlias if set, otherwise ifDescr
    pub fn display_name(&self) -> &str {
        self.base.if_alias.as_deref().unwrap_or(&self.base.if_descr)
    }

    /// Returns true if this port has a resolved neighbor connection
    pub fn has_neighbor(&self) -> bool {
        self.base.neighbor.is_some()
    }

    /// Returns true if neighbor is fully resolved (remote port known)
    pub fn has_full_neighbor_resolution(&self) -> bool {
        self.base
            .neighbor
            .as_ref()
            .map(|n| n.is_full_resolution())
            .unwrap_or(false)
    }

    /// Returns true if this port has raw LLDP data (may or may not be resolved)
    pub fn has_lldp_data(&self) -> bool {
        self.base.lldp_chassis_id.is_some() || self.base.lldp_port_id.is_some()
    }

    /// Returns true if this port has raw CDP data (may or may not be resolved)
    pub fn has_cdp_data(&self) -> bool {
        self.base.cdp_device_id.is_some() || self.base.cdp_port_id.is_some()
    }

    /// Returns true if this port has any neighbor discovery data (LLDP or CDP)
    pub fn has_neighbor_discovery_data(&self) -> bool {
        self.has_lldp_data() || self.has_cdp_data()
    }
}

/// Common IANAifType values for reference
/// Full list: https://www.iana.org/assignments/ianaiftype-mib/ianaiftype-mib
pub mod if_type {
    pub const OTHER: i32 = 1;
    pub const ETHERNET_CSMA_CD: i32 = 6;
    pub const ISO88023_CSMA_CD: i32 = 7;
    pub const FAST_ETHERNET: i32 = 62;
    pub const GIGABIT_ETHERNET: i32 = 117;
    pub const SOFTWARE_LOOPBACK: i32 = 24;
    pub const TUNNEL: i32 = 131;
    pub const PROP_VIRTUAL: i32 = 53;
    pub const IEEE8023AD_LAG: i32 = 161; // Link Aggregation Group
    pub const BRIDGE: i32 = 209;
    pub const VLAN: i32 = 135;
    pub const L2_VLAN: i32 = 136;
    pub const L3_IPVLAN: i32 = 137;
}
