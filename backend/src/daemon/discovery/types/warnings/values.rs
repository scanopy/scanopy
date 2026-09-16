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
    /// Carried here so the interface count can be checked against `ifNumber`, **not** so it can be
    /// reported as a short walk: interface shortfalls have their own record and their own prose,
    /// because a truncated interface *set* means something different to an operator than a
    /// truncated table.
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
