//! Slot *values* carried by coded warnings.
//!
//! These are the nouns a warning's sentence is built around — which SNMP group came up short,
//! what the device said about itself, what a discarded neighbour record cost. They used to be
//! `&'static str` labels rendered on the daemon; they are wire values now, so the English lives
//! in [`TypeMetadataProvider`] and reaches the UI through the fixture and i18n pipeline like
//! every other metadata string.
//!
//! None of them carries a `#[serde(other)]` fallback, deliberately. The exhaustive matches below
//! are what force a new group to declare which consequence sentence describes it, and the
//! warnings field's own lenient deserializer already degrades a single unrecognised value to
//! `Unknown` rather than failing the payload.

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

/// An SNMP data group a walk may come up short on.
///
/// An enum rather than a free string so the code derivation below is exhaustive: every group has
/// to declare which consequence sentence describes it, and a new one cannot be added without
/// choosing.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum SnmpWalkGroup {
    Lldp,
    Cdp,
    /// `ifTable`/`ifXTable` — the device's own interfaces.
    ///
    /// Checked against `ifNumber`, and reported as a group only when its read failed having
    /// returned no interfaces. A walk that returned some and stopped has its own record and its
    /// own prose, because a truncated interface *set* means something different to an operator
    /// than a truncated table.
    Interfaces,
    /// `dot1dBasePortIfIndex` — the bridge-port numbering both groups below are keyed by.
    BridgePortNumbering,
    BridgeForwarding,
    VlanMembership,
    /// `ipNetToMediaTable` — the ARP cache, which is how a switch tells us about hosts that
    /// answer nothing themselves.
    ArpTable,
    /// `entPhysicalTable` — model, serial and hardware revision.
    DeviceInventory,
    /// `ipAddrTable` — the device's own addresses and their netmasks.
    IpAddresses,
    /// `lldpLocPortTable` — the device's own port numbering, needed to attach each LLDP
    /// neighbour to the right interface.
    LldpLocalPorts,
    /// `dot1qVlanStaticName` — VLAN names, as opposed to which ports are in them.
    VlanNames,
}

impl SnmpWalkGroup {
    /// Whether an empty result here means the device does not implement the table at all,
    /// rather than that a read of an implemented table fell short.
    ///
    /// Only true for the bridge-port numbering, because it is the *root* of the bridge MIB:
    /// a switch that serves none of it has no MAC-address table or VLAN membership to offer
    /// over SNMP at all, and telling its operator that "previously discovered values were
    /// kept" promises a refresh that will never come.
    pub fn absence_means_unsupported(self) -> bool {
        matches!(self, Self::BridgePortNumbering)
    }

    /// Whether a record in this group can be thrown away *after* a successful read.
    ///
    /// True only for the neighbour groups, whose records carry a mandatory identifier — the LLDP
    /// chassis ID, the CDP device id — that a device can omit while answering everything asked of
    /// it. Discarding one leaves the group incomplete without the walk having stopped early, which
    /// is the one case where "incomplete" must not be read as "cut short".
    pub fn discards_malformed_records(self) -> bool {
        matches!(self, Self::Lldp | Self::Cdp)
    }

    /// Whether the rows a short read *did* return are thrown away rather than recorded.
    ///
    /// Bridge forwarding and VLAN membership: for these, `preserve_uncollected_data` restores the
    /// stored value on every interface when the walk fell short, so a partial read contributes
    /// nothing. Everything else here is recorded as far as it got: a half-read ARP cache still
    /// creates the hosts it named.
    ///
    /// LLDP and CDP were on this list until GH #701 moved neighbour evidence into per-row
    /// candidates. `replace_candidates_from_discovery` now records every row a partial walk read
    /// and carries stored rows forward only for ports that got none, so a warning saying the rows
    /// were not recorded was false (GH #685).
    pub fn partial_read_is_discarded(self) -> bool {
        matches!(self, Self::BridgeForwarding | Self::VlanMembership)
    }

    /// The first column the daemon walks for this group, which is the request a failing group
    /// fails on. Published so the UI can hand an operator the `snmpwalk` that repeats it.
    pub fn first_column(self) -> &'static str {
        use crate::daemon::discovery::integration::snmp::oids;
        match self {
            Self::Lldp => oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
            Self::Cdp => oids::cdp::entry::CDP_CACHE_DEVICE_ID,
            Self::Interfaces => oids::if_mib::columns::IF_INDEX,
            Self::BridgePortNumbering => oids::bridge::DOT1D_BASE_PORT_IF_INDEX,
            Self::BridgeForwarding => oids::bridge::fdb_entry::DOT1D_TP_FDB_ADDRESS,
            Self::VlanMembership => oids::vlan::q_bridge::DOT1Q_PVID,
            Self::ArpTable => oids::arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            Self::DeviceInventory => oids::entity::entry::ENT_PHYSICAL_DESCR,
            Self::IpAddresses => oids::ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
            Self::LldpLocalPorts => oids::lldp::local::LLDP_LOC_PORT_ID_SUBTYPE,
            Self::VlanNames => oids::vlan::q_bridge::DOT1Q_VLAN_STATIC_NAME,
        }
    }

    /// The table this group's columns belong to, for a check that has to show whole rows.
    pub fn table(self) -> &'static str {
        use crate::daemon::discovery::integration::snmp::oids;
        match self {
            Self::Lldp => oids::lldp::remote::LLDP_REM_TABLE,
            Self::Cdp => oids::cdp::CDP_CACHE_TABLE,
            Self::Interfaces => oids::if_mib::IF_TABLE,
            Self::BridgePortNumbering => oids::bridge::DOT1D_BASE_PORT_TABLE,
            Self::BridgeForwarding => oids::bridge::DOT1D_TP_FDB_TABLE,
            Self::VlanMembership => oids::vlan::q_bridge::DOT1Q_PORT_VLAN_TABLE,
            Self::ArpTable => oids::arp::IP_NET_TO_MEDIA_TABLE,
            Self::DeviceInventory => oids::entity::ENT_PHYSICAL_TABLE,
            Self::IpAddresses => oids::ip_mib::IP_ADDR_TABLE,
            Self::LldpLocalPorts => oids::lldp::local::LLDP_LOC_PORT_TABLE,
            Self::VlanNames => oids::vlan::q_bridge::DOT1Q_VLAN_STATIC_TABLE,
        }
    }
}

/// The net-snmp commands that check a warning against the device, from the daemon's host.
///
/// Published on each warning code's metadata as `snmp_check`, so the UI builds the commands from
/// what the code declares instead of keeping its own list of codes. Each value is the comparison
/// that tells an operator whether the device returns what the warning says it returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SnmpCheck {
    /// The read returned nothing. The daemon's GETBULK page, a one-row GETBULK and GETNEXT on the
    /// group's first column: which of them answers separates a device that cannot serve GETBULK
    /// from one that serves nothing.
    Repeat,
    /// The read stopped partway, came up short or met no such object. The daemon's GETBULK page
    /// and GETNEXT on the first column, to compare where each ends.
    Recount,
    /// [`Self::Recount`], after a GET of the value the device published about itself.
    Claim,
    /// [`Self::Recount`] with net-snmp's increasing-OID check off (`-Cc`), so it prints the rows
    /// that came back out of order instead of stopping at the first.
    Unordered,
    /// The rows came back malformed or misplaced. The whole table with the daemon's page, so every
    /// column of those rows shows.
    Table,
}

impl HasId for SnmpWalkGroup {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SnmpWalkGroup {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        Icon::Table
    }
}

impl TypeMetadataProvider for SnmpWalkGroup {
    /// Noun phrase, used as the object of a sentence: "did not finish reporting {name}".
    fn name(&self) -> &'static str {
        match self {
            Self::Lldp => "LLDP neighbours",
            Self::Cdp => "CDP neighbours",
            Self::Interfaces => "interfaces",
            Self::BridgePortNumbering => "SNMP bridge-port numbering",
            Self::BridgeForwarding => "bridge forwarding",
            Self::VlanMembership => "VLAN membership",
            Self::ArpTable => "the ARP table",
            Self::DeviceInventory => "hardware inventory",
            Self::IpAddresses => "device IP addresses",
            Self::LldpLocalPorts => "LLDP local port numbering",
            Self::VlanNames => "VLAN names",
        }
    }

    /// What the UI needs to build the `snmpwalk` commands that repeat this group's read: the
    /// column, and the page size, timeout and retries the daemon read it with.
    fn metadata(&self) -> serde_json::Value {
        use crate::daemon::discovery::integration::snmp::{queries, session};
        serde_json::json!({
            "first_column": self.first_column(),
            "table": self.table(),
            "max_repetitions": queries::BULK_MAX_REPETITIONS,
            "timeout_seconds": session::SNMP_TIMEOUT.as_secs(),
            "retries": queries::MAX_TRANSPORT_RETRIES,
        })
    }
}

/// How a read that returned nothing ended, as the daemon observed it.
///
/// Each value is something the daemon saw on the wire, never an inference about the device: a
/// timeout is a request with no response, an error status is the status the device returned.
/// Collapsing these into one "stopped answering" told an operator whose device had answered every
/// request with `genErr` that it had gone silent.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum SnmpWalkCause {
    /// No response arrived within the timeout, on every retry.
    TimedOut,
    /// A response arrived with no varbinds on it.
    EmptyResponse,
    /// A response arrived and could not be decoded or authenticated.
    UnreadableResponse,
    /// `tooBig(1)`: the response would not fit in one message.
    TooBig,
    /// `genErr(5)`.
    GenErr,
    /// `noAccess(6)`.
    NoAccess,
    /// `authorizationError(16)`: the credential may not read this object.
    AuthorizationError,
    /// Any other non-zero error status.
    OtherErrorStatus,
}

impl SnmpWalkCause {
    /// The cause for a response carrying this non-zero `error-status`.
    pub fn from_error_status(status: u32) -> Self {
        match status {
            1 => Self::TooBig,
            5 => Self::GenErr,
            6 => Self::NoAccess,
            16 => Self::AuthorizationError,
            _ => Self::OtherErrorStatus,
        }
    }
}

impl HasId for SnmpWalkCause {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SnmpWalkCause {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        Icon::Info
    }
}

impl TypeMetadataProvider for SnmpWalkCause {
    /// Predicate for "every request for {groups} …".
    fn name(&self) -> &'static str {
        match self {
            Self::TimedOut => "timed out with no response",
            Self::EmptyResponse => "got a response with no data in it",
            Self::UnreadableResponse => "got a response the daemon could not decode",
            Self::TooBig => "got the error tooBig",
            Self::GenErr => "got the error genErr",
            Self::NoAccess => "got the error noAccess",
            Self::AuthorizationError => "got the error authorizationError",
            Self::OtherErrorStatus => "got an SNMP error status",
        }
    }
}

/// One group that returned nothing on a device, and how its read ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct SnmpWalkFailure {
    pub group: SnmpWalkGroup,
    pub cause: SnmpWalkCause,
}

/// Where a device's claim about itself came from.
///
/// Named rather than folded into a sentence because the operator's next step depends on it: a
/// wrong `ifNumber` is a firmware bug to report upstream, while a set bridge bit over an empty
/// bridge table is usually a missing SNMP view or VLAN context on their side.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum ClaimSource {
    /// `ifNumber.0` — how many interfaces the device says it has.
    IfNumber,
    /// `sysServices.0` bit 2 — the device says it operates at the datalink layer.
    SysServicesBridgeBit,
    /// The device answered `lldpLocChassisId`, so it runs an LLDP agent.
    LldpLocalIdentity,
    /// `dot1dBaseNumPorts.0` — how many bridge ports the device says it controls.
    Dot1dBaseNumPorts,
}

impl HasId for ClaimSource {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ClaimSource {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        Icon::Info
    }
}

impl TypeMetadataProvider for ClaimSource {
    /// How the device stated it, as the object of "the device reports …".
    fn name(&self) -> &'static str {
        match self {
            Self::IfNumber => "its ifNumber",
            Self::SysServicesBridgeBit => "its sysServices bridge bit",
            Self::LldpLocalIdentity => "an LLDP chassis ID of its own",
            Self::Dot1dBaseNumPorts => "its dot1dBaseNumPorts",
        }
    }

    /// The object the device published, for the `snmpget` that checks it.
    fn metadata(&self) -> serde_json::Value {
        use crate::daemon::discovery::integration::snmp::oids;
        let oid = match self {
            Self::IfNumber => oids::if_mib::IF_NUMBER,
            Self::SysServicesBridgeBit => oids::system::SYS_SERVICES,
            Self::LldpLocalIdentity => oids::lldp::local::LLDP_LOC_CHASSIS_ID,
            Self::Dot1dBaseNumPorts => oids::bridge::DOT1D_BASE_NUM_PORTS,
        };
        serde_json::json!({ "oid": oid })
    }
}

/// What discarding a device's malformed neighbour records cost it.
///
/// A slot value rather than two codes per reason: losing every link and losing some of them is a
/// difference in severity, not in failure mode, and the metric asks about mode. Splitting it into
/// codes would double the enum to say something the operator reads in one clause.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum MalformedNeighbourConsequence {
    /// Nothing survived, so the device contributes no physical links at all.
    AllLinksLost,
    /// Some records survived, so the device's link set is incomplete.
    SomeLinksLost,
}

impl MalformedNeighbourConsequence {
    /// Which clause describes a device that kept `kept` records.
    pub fn from_kept(kept: usize) -> Self {
        if kept == 0 {
            Self::AllLinksLost
        } else {
            Self::SomeLinksLost
        }
    }
}

impl HasId for MalformedNeighbourConsequence {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for MalformedNeighbourConsequence {
    fn color(&self) -> Color {
        match self {
            Self::AllLinksLost => Color::Red,
            Self::SomeLinksLost => Color::Amber,
        }
    }

    fn icon(&self) -> Icon {
        Icon::Unlink
    }
}

impl TypeMetadataProvider for MalformedNeighbourConsequence {
    /// Reads as the middle clause of "…, so {name}."
    fn name(&self) -> &'static str {
        match self {
            Self::AllLinksLost => "those devices contribute no physical links at all",
            Self::SomeLinksLost => "some of their physical links are missing",
        }
    }
}

/// The desktop container VM a daemon was found running inside.
///
/// Each one runs containers in a Linux VM whose network is NAT'd behind the machine it runs on, so
/// ARP, mDNS and LLDP from a daemon in it reach the VM's virtual network and not the LAN.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum DesktopVm {
    DockerDesktop,
    OrbStack,
    Colima,
}

impl HasId for DesktopVm {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for DesktopVm {
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        Icon::Box
    }
}

impl TypeMetadataProvider for DesktopVm {
    /// The product name, as the `{vm}` slot of "{daemon} runs inside {vm}".
    fn name(&self) -> &'static str {
        match self {
            Self::DockerDesktop => "Docker Desktop",
            Self::OrbStack => "OrbStack",
            Self::Colima => "Colima",
        }
    }
}
