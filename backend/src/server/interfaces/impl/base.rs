use crate::server::interface_neighbors::r#impl::base::InterfaceNeighborEvidence;
pub use crate::server::interface_neighbors::r#impl::base::Neighbor;
use crate::server::ip_addresses::r#impl::base::MacEvidence;
use crate::server::shared::attribution;
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};
use crate::server::topology::types::views::{
    FilterValueContext, HasFilterValues, MetadataFilterType,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;
use strum_macros::{EnumIter, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Which groups of per-interface data the daemon read in full during a scan.
///
/// Each group comes from its own SNMP walk, and a walk cut short by a timeout yields exactly the
/// same empty result as a device that genuinely has nothing to report. Without knowing which
/// happened, the server overwrote good data with NULL on every truncation — and for the neighbour
/// fields that also dropped the row out of L2 resolution permanently, since the resolution filter
/// requires a chassis id or CDP device id to be present.
///
/// Every field defaults to `true`, so a daemon predating this behaves exactly as before: it
/// reports everything as authoritative and the server overwrites.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct InterfaceDataComplete {
    /// `lldp_chassis_id`, `lldp_port_id`, `lldp_sys_name`, `lldp_port_desc`, `lldp_mgmt_addr`,
    /// `lldp_sys_desc`
    #[serde(default = "crate::server::interfaces::r#impl::base::complete_default")]
    pub lldp: bool,
    /// `cdp_device_id`, `cdp_port_id`, `cdp_platform`, `cdp_address`
    #[serde(default = "crate::server::interfaces::r#impl::base::complete_default")]
    pub cdp: bool,
    /// `fdb_macs`
    #[serde(default = "crate::server::interfaces::r#impl::base::complete_default")]
    pub fdb: bool,
    /// `native_vlan_id`, `vlan_ids`
    #[serde(default = "crate::server::interfaces::r#impl::base::complete_default")]
    pub vlan_membership: bool,
}

pub(crate) fn complete_default() -> bool {
    true
}

impl Default for InterfaceDataComplete {
    fn default() -> Self {
        Self {
            lldp: true,
            cdp: true,
            fdb: true,
            vlan_membership: true,
        }
    }
}

impl InterfaceDataComplete {
    /// Whether every group was read in full.
    pub fn all(&self) -> bool {
        self.lldp && self.cdp && self.fdb && self.vlan_membership
    }

    /// No group has been read yet, so the server keeps everything it already holds.
    ///
    /// The counterpart to [`Default`], which claims every group is authoritative. That default is
    /// right for the wire (an old daemon that never sends the field behaved that way), and wrong
    /// for a checkpoint written partway through a collection: SNMP persists its interface set as
    /// soon as the ifTable walk finishes, long before the neighbour and VLAN walks run, and
    /// shipping the all-`true` default alongside it told the server those columns were
    /// authoritatively empty. It cleared them — and an interface with no chassis id drops out of
    /// L2 resolution for good.
    pub fn none() -> Self {
        Self {
            lldp: false,
            cdp: false,
            fdb: false,
            vlan_membership: false,
        }
    }
}

/// Whether a port resolved to a neighbour, for the `LinkState` metadata filter on Interface.
///
/// The L2 view draws one element per row of a device's SNMP ifTable, so its node count scales with
/// total port count rather than device count: a network of ~700 devices produced 17,236 nodes
/// against 857 links, because most of those ports are unused access ports and virtual adapters.
/// Nearly all of them carry no adjacency and so contribute nothing to the fabric the view exists
/// to show — but they are not noise either. A down access port is exactly what an operator looks
/// at when asking why something is unreachable, so this classifies rather than discards: the view
/// hides unlinked ports by default and the filter panel shows them again on one click.
///
/// `Linked` covers partial resolution as well as full. A neighbour known only at device level
/// still draws an edge (`NeighborLink` rather than `PhysicalLink`), so the port is visibly
/// connected and hiding it would break the diagram.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    IntoStaticStr,
    EnumIter,
    ToSchema,
)]
pub enum InterfaceLinkState {
    Linked,
    Unlinked,
}

impl InterfaceLinkState {
    /// Classify a port from evidence in **both** directions.
    ///
    /// A link is recorded on one side only, so an interface reporting no neighbour of its own is
    /// still linked when something points at it. There is no outbound-only constructor on purpose:
    /// the previous one existed, was never called, and encoded exactly the mistake the frontend
    /// shipped — judging the local `neighbor` alone drew 11 edges where it should have drawn 720.
    ///
    /// A partial resolution (`Neighbor::Host` — remote device known but not the port) counts as
    /// linked: it still draws an edge, so hiding it would break the diagram.
    ///
    /// `has_neighbor` is "this interface has at least one live row of its own in either resolved
    /// table" — the successor to reading `Interface.neighbor.is_some()` now that a port's
    /// adjacencies are a `Vec` rather than a single field.
    pub fn classify(has_neighbor: bool, referenced_as_neighbour: bool) -> Self {
        if has_neighbor || referenced_as_neighbour {
            Self::Linked
        } else {
            Self::Unlinked
        }
    }
}

impl HasFilterValues for Interface {
    fn filter_values(&self, ctx: &FilterValueContext) -> BTreeMap<MetadataFilterType, String> {
        let state = InterfaceLinkState::classify(
            ctx.interfaces_with_neighbours.contains(&self.id),
            ctx.interfaces_referenced_as_neighbours.contains(&self.id),
        );
        BTreeMap::from([(MetadataFilterType::LinkState, state.id().to_string())])
    }
}

impl HasId for InterfaceLinkState {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for InterfaceLinkState {
    fn color(&self) -> Color {
        match self {
            Self::Linked => Color::Green,
            Self::Unlinked => Color::Gray,
        }
    }
    fn icon(&self) -> Icon {
        match self {
            Self::Linked => Icon::Cable,
            Self::Unlinked => Icon::Circle,
        }
    }
}

impl TypeMetadataProvider for InterfaceLinkState {
    fn name(&self) -> &'static str {
        match self {
            Self::Linked => "Linked",
            Self::Unlinked => "Unlinked",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Linked => "Ports with a discovered neighbour",
            Self::Unlinked => "Ports with no discovered neighbour",
        }
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
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    Default,
    ToSchema,
    strum_macros::Display,
)]
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
pub struct InterfaceBase {
    /// The host this entity belongs to.
    pub host_id: Uuid,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// SNMP ifIndex — stable identifier within device, where one was read.
    ///
    /// `None` for a port learned from a neighbour's LLDP/CDP advertisement rather than from the
    /// device's own ifTable: the far end tells us the port exists and what it is called, never its
    /// index. `0` used to stand in for that, which is a real ifIndex on some agents and made
    /// "never read" indistinguishable from "read as zero".
    ///
    /// Not the identity of the row. `match_existing_interface` tries `(host_id, if_name)` first
    /// and the live unique index is on that pair; the index tier only runs for a row that has one.
    pub if_index: Option<i32>,
    /// SNMP ifDescr - interface description (e.g., GigabitEthernet0/1), where one was read.
    ///
    /// `None` for a source with nothing to put here — PROFINET DCP Identify carries no per-port
    /// description at all. Same principle as `if_index`/`if_type`: an absent value is `None`, never
    /// a fabricated string standing in for it.
    pub if_descr: Option<String>,
    /// SNMP ifName - short interface name (e.g., Gi1/0/1)
    pub if_name: Option<String>,
    /// SNMP ifAlias - user-configured description
    pub if_alias: Option<String>,
    /// SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.), where one was read.
    ///
    /// `None` is *unknown*, never a type. Everything that filters on this must treat unknown as
    /// included — a port at the far end of a cable is physical by construction, and excluding it
    /// for lack of a number would drop the row from resolution and from the map.
    pub if_type: Option<i32>,
    /// Interface speed from ifSpeed/ifHighSpeed in bits per second
    pub speed_bps: Option<i64>,
    /// SNMP ifAdminStatus: 1=up, 2=down, 3=testing, or `None` where nothing read it.
    ///
    /// Optional rather than defaulting to `Up`: an advertisement carries no status, and recording
    /// one as up would be a claim nothing made.
    pub admin_status: Option<IfAdminStatus>,
    /// SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent,
    /// 7=lowerLayerDown — or `None` where nothing read it.
    ///
    /// Deliberately not `IfOperStatus::Unknown`, which is the MIB's value 4 and means *the device
    /// said it does not know*. "We never asked" is a different statement, and folding the two
    /// would make an inferred port indistinguishable from one that reported unknown.
    pub oper_status: Option<IfOperStatus>,

    // Local links
    /// MAC address, usually from SNMP `ifPhysAddress`, with the evidence for it.
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = MacEvidence)]
    pub mac_address: Option<MacEvidence>,
    /// FK to IPAddress entity - this port's IP assignment (must be on same host).
    /// Old daemons send this as "interface_id".
    #[serde(alias = "interface_id")]
    pub ip_address_id: Option<Uuid>,
    /// Whether the device's own SNMP `ipAddrTable` lists this ifIndex as carrying one of its
    /// configured IP addresses.
    ///
    /// Independent of `ip_address_id`: that FK is set server-side and requires the interface's MAC
    /// to be unique on the host before it links anything (`plan_interface_ip_links`), so it stays
    /// `NULL` on exactly the hosts this field exists to help — a Windows NIC and its NDIS
    /// filter/LWF pseudo-interfaces sharing one MAC (GH #668). `ipAddrTable` only ever binds an
    /// address to a real IP-stack adapter; a filter driver is never a separate one, so this
    /// distinguishes the physical interface among MAC-sharing candidates. `#[serde(default)]` so a
    /// daemon predating this field is read as `false` on every row — never worse than today's
    /// behavior.
    #[serde(default)]
    pub ip_configured: bool,

    // Neighbor resolution (LLDP/CDP) moved off `interfaces` in GH #701: raw evidence lives in
    // `interface_neighbor_candidates`, resolved adjacencies in `interface_neighbor_interfaces` /
    // `interface_neighbor_hosts` — a port can now carry several of each, where this struct only
    // ever had room for one. See `interface_neighbors` for the read/write paths that replaced
    // `neighbor`, `neighbor_seen_at`, and the 6 `lldp_*` + 4 `cdp_*` fields that used to live here.
    /// Raw LLDP/CDP evidence this scan heard on the port — the wire shape a **current** daemon
    /// submits, one entry per distinct LLDP or CDP record. Drained into `interface_neighbor_
    /// candidates` by the discovery ingest path (`InterfaceService::create_or_update_from_
    /// discovery`); never read anywhere else. Not `skip_serializing`: this field is the daemon's
    /// *outgoing* discovery payload as much as it is the server's read model, and that attribute
    /// has no notion of direction — it silently dropped every daemon's submitted evidence before
    /// the request ever left the process (found investigating the GH #701 candidate-persistence
    /// regression; nothing downstream of the wire was ever at fault). Harmless for API responses:
    /// `create_or_update_from_discovery` takes this field via `mem::take` before returning or
    /// persisting `Interface`, and it is not a stored column, so nothing read back from the
    /// database or echoed in a response ever has it populated.
    #[serde(default)]
    pub neighbor_candidates: Vec<InterfaceNeighborEvidence>,
    /// Bridge FDB: learned MAC addresses on this switch port.
    /// Single-MAC ports can be resolved to neighbor links server-side.
    /// Multi-MAC ports indicate uplinks where LLDP/CDP is the better source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fdb_macs: Option<Vec<String>>,

    /// Native/untagged VLAN entity ID on this port (resolved from Q-BRIDGE dot1qPvid)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_vlan_id: Option<Uuid>,

    /// Tagged VLAN entity IDs on this port (resolved from Q-BRIDGE dot1qVlanCurrentEgressPorts)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlan_ids: Option<Vec<Uuid>>,
}

impl Default for InterfaceBase {
    fn default() -> Self {
        Self {
            host_id: Uuid::nil(),
            network_id: Uuid::nil(),
            if_index: None,
            if_descr: None,
            if_name: None,
            if_alias: None,
            if_type: None,
            speed_bps: None,
            admin_status: None,
            oper_status: None,
            mac_address: None,
            ip_address_id: None,
            ip_configured: false,
            neighbor_candidates: Vec::new(),
            fdb_macs: None,
            native_vlan_id: None,
            vlan_ids: None,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema, Validate,
)]
pub struct Interface {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// Start of the interval this revision was current for (SCD2 history).
    #[serde(default)]
    #[schema(read_only)]
    pub valid_from: DateTime<Utc>,
    /// End of the interval this revision was current for. `null` while it is the live revision.
    #[serde(default)]
    #[schema(read_only)]
    pub valid_to: Option<DateTime<Utc>>,
    /// Stable identifier shared by every revision of the same entity across its history.
    #[serde(default)]
    #[schema(read_only)]
    pub lineage_id: Option<Uuid>,
    /// When a discovery last observed this entity.
    #[serde(default)]
    #[schema(read_only)]
    pub last_seen_at: DateTime<Utc>,
    /// The most recent discovery that observed this entity.
    #[serde(default)]
    #[schema(read_only)]
    pub last_discovery_id: Option<Uuid>,
    /// The discovery that first observed this entity.
    #[serde(default)]
    #[schema(read_only)]
    pub first_discovery_id: Option<Uuid>,
    /// What to call this interface when it has no `if_alias`/`if_descr`: the MAC it was
    /// identified by, or "Interface" when it has neither.
    ///
    /// Read-only and computed from [`Interface::display_name`] — the same ladder topology port
    /// labels an interface with, so it cannot be called one thing in a list and another on the
    /// map. Only set on outbound responses nested under a host (`HostResponse::interfaces`);
    /// absent on a daemon's own submission and on the standalone `/interfaces` CRUD endpoints,
    /// which return `Interface` directly without this computation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub display_name: Option<String>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: InterfaceBase,
}

impl ChangeTriggersTopologyStaleness<Interface> for Interface {
    fn triggers_staleness(&self, other: Option<Interface>) -> bool {
        if let Some(other_entry) = other {
            // Neighbour changes no longer flow through this hook: resolution writes go through
            // `InterfaceNeighborService`, not `CrudService::update`, so they mark topology stale
            // directly rather than via this per-entity comparison. See `hosts/service/topology/mod.rs`.
            self.base.ip_address_id != other_entry.base.ip_address_id
                || self.base.host_id != other_entry.base.host_id
        } else {
            true // New or deleted entry triggers staleness
        }
    }
}

impl Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let descr = self.base.if_descr.as_deref().unwrap_or("(no description)");
        match self.base.if_index {
            Some(if_index) => {
                write!(f, "Interface {} (ifIndex {}): {}", self.id, if_index, descr)
            }
            // A port learned from a neighbour's advertisement has no index to name it by.
            None => write!(f, "Interface {}: {}", self.id, descr),
        }
    }
}

impl Interface {
    pub fn new(base: InterfaceBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            display_name: None,
            base,
        }
    }

    /// Returns true if interface is operationally up.
    ///
    /// An unread status is not up. "We never asked" is not evidence of health, and reporting it
    /// as up is the failure mode the nullable column exists to prevent.
    pub fn is_up(&self) -> bool {
        self.base.oper_status == Some(IfOperStatus::Up)
    }

    /// Returns true if interface is administratively up. Unread is not up — see [`Self::is_up`].
    pub fn is_admin_up(&self) -> bool {
        self.base.admin_status == Some(IfAdminStatus::Up)
    }

    /// Display name for UI/topology labels: ifAlias, then ifDescr, then — for a source with
    /// neither, such as a PROFINET DCP identify — the MAC it was identified by, which is more
    /// useful than a bare placeholder since it is the one thing every producer of this row has.
    /// "Interface" only when none of those exist, mirroring the `"Unnamed host"`/`"Unknown Host"`
    /// precedent elsewhere in this module for backend-owned fallback text (not user-facing app
    /// chrome, so not paraglide — device data and its absence, like a hostname or an IP).
    pub fn display_name(&self) -> String {
        if let Some(alias) = self.base.if_alias.as_deref().filter(|s| !s.is_empty()) {
            return alias.to_string();
        }
        if let Some(descr) = self.base.if_descr.as_deref().filter(|s| !s.is_empty()) {
            return descr.to_string();
        }
        if let Some(mac) = self.base.mac_address.as_ref() {
            return mac.value().0.to_string();
        }
        "Interface".to_string()
    }

    /// Keep the stored values for any group of data this scan did not finish reading.
    ///
    /// LLDP/CDP evidence moved off this struct in GH #701 — see
    /// `InterfaceNeighborService::replace_candidates_from_discovery`, which applies the same
    /// per-group completeness guard to the candidate rows that replaced `lldp_*`/`cdp_*` here.
    /// This method now only covers the two groups that stayed on `interfaces`: FDB and VLAN
    /// membership. A walk cut short by a timeout returns exactly what a device with nothing to
    /// report returns: nothing. Overwriting on that is destructive rather than merely stale, so a
    /// group the daemon did *not* finish reading keeps its stored value; a group it *did* read in
    /// full is authoritative in both directions, including clearing a value that is genuinely gone.
    pub fn preserve_uncollected_data(&mut self, existing: &Self, collected: InterfaceDataComplete) {
        if !collected.fdb {
            self.base.fdb_macs = existing.base.fdb_macs.clone();
        }
        if !collected.vlan_membership {
            self.base.native_vlan_id = existing.base.native_vlan_id;
            self.base.vlan_ids = existing.base.vlan_ids.clone();
        }
    }

    /// Drop identity fields the device reported blank, so absence is recorded as absence.
    ///
    /// A zero-length ifXTable `ifName` is a legitimate SNMP answer meaning "this device has no
    /// name for this port", but it reaches the server as `Some("")` and from there is treated as
    /// a real name: the tiered discovery match keys on `if_name.is_some()`, and the partial unique
    /// index `(host_id, if_name) WHERE if_name IS NOT NULL` counts `""` as a value. A switch that
    /// answers `""` for every port then hits a duplicate-key violation on its second port —
    /// truncating that host's whole ingest. `if_index` remains as the identity for such ports,
    /// which is the correct one anyway.
    ///
    /// Called on the discovery ingest path so devices behind older daemons are covered too.
    pub fn normalize_blank_identity(&mut self) {
        if self
            .base
            .if_name
            .as_deref()
            .is_some_and(|name| name.trim().is_empty())
        {
            self.base.if_name = None;
        }
        if self
            .base
            .if_alias
            .as_deref()
            .is_some_and(|alias| alias.trim().is_empty())
        {
            self.base.if_alias = None;
        }
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
    pub const IEEE80211: i32 = 71; // Wi-Fi

    /// The virtual/software interface families, excluded wherever a question is about a physical
    /// port: which rows the L2 view draws, and which rows count towards a MAC's uniqueness within
    /// a device.
    ///
    /// Kept here rather than beside either consumer because the two must agree. A VLAN interface
    /// carrying the chassis base MAC is not a candidate far end for a cable, so it must neither
    /// be drawn as a port nor make a physical port's address look ambiguous — the customer's
    /// Westermo has six `propVirtual` VLAN rows sharing `…02:E0` while every physical port has a
    /// unique address.
    pub const EXCLUDED_IF_TYPES: &[i32] = &[
        SOFTWARE_LOOPBACK,
        PROP_VIRTUAL,
        IEEE80211,
        TUNNEL,
        VLAN,
        L2_VLAN,
        BRIDGE,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::lldp::LldpChassisId;

    /// The regression this whole thing is about: `neighbor_candidates` is not just the server's
    /// read model, it is also the shape a daemon serializes into its outgoing discovery request.
    /// `skip_serializing` has no notion of direction — it silently dropped every submitted
    /// candidate before the request left the daemon process, and nothing downstream (the
    /// completeness veto, host-identity matching, `replace_candidates_from_discovery`) was ever
    /// wrong, because none of it ever saw real evidence to begin with.
    #[test]
    fn neighbor_candidates_survives_a_json_round_trip() {
        let mut interface = Interface::default();
        interface.base.neighbor_candidates = vec![InterfaceNeighborEvidence {
            lldp_chassis_id: Some(LldpChassisId::MacAddress("00:1a:2b:00:11:00".into())),
            ..Default::default()
        }];

        let json = serde_json::to_value(&interface).unwrap();
        let round_tripped: Interface = serde_json::from_value(json).unwrap();

        assert_eq!(
            round_tripped.base.neighbor_candidates, interface.base.neighbor_candidates,
            "a daemon's submitted LLDP/CDP evidence must survive serialization, not just \
             deserialization"
        );
    }
}
