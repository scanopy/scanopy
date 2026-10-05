//! LLDP (Link Layer Discovery Protocol) types and resolution.
//!
//! This module provides enums for LLDP identifier types per IEEE 802.1AB,
//! along with resolution methods to convert LLDP neighbor data into
//! database entity references.

pub mod resolver;
pub mod snapshot;

pub use resolver::LldpResolver;
pub use snapshot::LldpInventorySnapshot;

use crate::server::shared::storage::pg_value::strip_nuls;
use crate::server::shared::storage::traits::Unique;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use strum_macros::VariantNames;
use utoipa::ToSchema;
use uuid::Uuid;

/// LLDP Chassis ID subtypes per IEEE 802.1AB.
///
/// The chassis ID identifies the remote device. Different network equipment
/// may use different subtypes depending on configuration and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames, ToSchema)]
#[serde(tag = "subtype", content = "value")]
pub enum LldpChassisId {
    /// Subtype 1: Chassis component (e.g., backplane serial number)
    #[schema(title = "ChassisComponent")]
    ChassisComponent(String),
    /// Subtype 2: Interface alias (ifAlias from IF-MIB)
    #[schema(title = "InterfaceAlias")]
    InterfaceAlias(String),
    /// Subtype 3: Port component (e.g., backplane port number)
    #[schema(title = "PortComponent")]
    PortComponent(String),
    /// Subtype 4: MAC address (most common)
    #[schema(title = "MacAddress")]
    MacAddress(String),
    /// Subtype 5: Network address (IP address stored as string)
    #[schema(value_type = String)]
    #[schema(title = "NetworkAddress")]
    NetworkAddress(#[serde(with = "ip_addr_serde")] IpAddr),
    /// Subtype 6: Interface name (ifName from IF-MIB)
    #[schema(title = "InterfaceName")]
    InterfaceName(String),
    /// Subtype 7: Locally assigned (device-specific identifier)
    #[schema(title = "LocallyAssigned")]
    LocallyAssigned(String),
}

/// LLDP Port ID subtypes per IEEE 802.1AB.
///
/// The port ID identifies the specific port on the remote device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames, ToSchema)]
#[serde(tag = "subtype", content = "value")]
pub enum LldpPortId {
    /// Subtype 1: Interface alias (ifAlias from IF-MIB)
    #[schema(title = "InterfaceAlias")]
    InterfaceAlias(String),
    /// Subtype 2: Port component (e.g., backplane port number)
    #[schema(title = "PortComponent")]
    PortComponent(String),
    /// Subtype 3: MAC address
    #[schema(title = "MacAddress")]
    MacAddress(String),
    /// Subtype 4: Network address (IP address stored as string)
    #[schema(value_type = String)]
    #[schema(title = "NetworkAddress")]
    NetworkAddress(#[serde(with = "ip_addr_serde")] IpAddr),
    /// Subtype 5: Interface name (ifName from IF-MIB)
    #[schema(title = "InterfaceName")]
    InterfaceName(String),
    /// Subtype 6: Agent circuit ID (used by some providers)
    #[schema(title = "AgentCircuitId")]
    AgentCircuitId(String),
    /// Subtype 7: Locally assigned (device-specific identifier)
    #[schema(title = "LocallyAssigned")]
    LocallyAssigned(String),
}

/// Outcome of resolving an advertised LLDP identifier to a database entity.
///
/// "Didn't resolve" has two causes that call for opposite responses, so they are not collapsed
/// into a bare `None`: [`Self::NoStrategy`] means the neighbor advertised an identity this system
/// has no way to look up (a code-side gap, or a subtype that genuinely carries no usable
/// identity), while [`Self::NotFound`] means the lookup ran correctly and the device simply isn't
/// in this site's inventory (an operator-side gap — scan it and the link appears).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityResolution {
    /// Matched exactly one entity.
    Resolved(Uuid),
    /// No lookup strategy applies to the advertised identifier.
    NoStrategy,
    /// A strategy ran and matched nothing.
    NotFound,
    /// A strategy ran and matched more than one entity, so the identifier does not identify.
    ///
    /// Split out from [`Self::NotFound`] because the two mean opposite things to whoever reads the
    /// stats: `NotFound` says the far end is absent from the inventory (scan it and the link
    /// appears), while `Ambiguous` says it is present several times over and the identifier cannot
    /// choose between them. The case this exists for is a switch that reports its chassis base MAC
    /// as `ifPhysAddress` on every port — legal SNMP, and common on D-Link/TP-Link/Omada/UniFi —
    /// where a MAC names the device but no single port on it (GH #668).
    Ambiguous,
}

impl IdentityResolution {
    /// `Resolved(id)` when `found` is `Some`, otherwise [`Self::NotFound`].
    ///
    /// Structurally cannot yield [`Self::Ambiguous`], so it is only for lookups whose filter is
    /// on a genuinely unique column. Anything on a non-unique one should come through
    /// [`Self::from_unique`] instead, or the ambiguity is silently reported as absence.
    pub fn found(found: Option<Uuid>) -> Self {
        match found {
            Some(id) => Self::Resolved(id),
            None => Self::NotFound,
        }
    }

    /// The storage layer's verdict, carried through unchanged.
    pub fn from_unique(found: Unique<Uuid>) -> Self {
        match found {
            Unique::One(id) => Self::Resolved(id),
            Unique::None => Self::NotFound,
            Unique::Multiple => Self::Ambiguous,
        }
    }

    /// Whether a tier produced no host, so the ladder should keep going.
    ///
    /// Ambiguity is not a stopping condition: two devices sharing a chassis id may still be told
    /// apart by their `sysName`, so a later tier is worth trying. What it must not do is vanish —
    /// see `resolve_host_id`, which remembers it for the terminal verdict.
    fn is_unresolved(self) -> bool {
        !matches!(self, Self::Resolved(_))
    }
}

/// What a neighbour advertised about itself *besides* the chassis id being resolved.
///
/// A parameter object rather than more positional arguments: the subtype-independent tiers of
/// [`LldpChassisId::resolve_host_id`] each consume one of these, and a bare `Option<&str>,
/// Option<IpAddr>` pair at a dozen call sites is the shape that eventually gets passed in the
/// wrong order.
#[derive(Debug, Clone, Copy, Default)]
pub struct AdvertisedIdentity<'a> {
    /// `lldpRemSysName`, or the CDP device id — an operator-assigned label, not an identifier.
    pub sys_name: Option<&'a str>,
    /// An address the far end published for itself: `lldpRemManAddr`, a `NetworkAddress` port id,
    /// or `cdpCacheAddress`. Already filtered by [`is_usable_identity_address`] where it comes
    /// from an entity — see [`crate::server::interfaces::r#impl::base::InterfaceBase::advertised_address`].
    pub address: Option<IpAddr>,
}

/// What a neighbour advertised about its *own* port, as opposed to about itself.
///
/// Separate from [`AdvertisedIdentity`] deliberately. A port name is not an identity — resolving a
/// device by one is how a far end ends up merged with whatever else happens to have a port called
/// `eth0` — and keeping the two apart means neither can be passed where the other is wanted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdvertisedFarEndPort<'a> {
    /// The far end's name for the port, where it published one that names rather than encodes.
    pub name: Option<&'a str>,
    /// The port's own MAC, where the port id carried one.
    pub mac: Option<&'a str>,
}

impl AdvertisedIdentity<'_> {
    /// Resolve a far-end *device* from what it published about itself, independent of any chassis
    /// id.
    ///
    /// These are tiers 2 and 3 of [`LldpChassisId::resolve_host_id`], lifted out because CDP has
    /// no chassis id and so cannot run that ladder — but a CDP device id is a `sysName` and
    /// `cdpCacheAddress` is a management address, and both deserve the same treatment. Sharing the
    /// code is what keeps the two protocols from drifting into different answers for the same
    /// evidence.
    ///
    /// Returns [`IdentityResolution::NoStrategy`] when neither tier had anything to run, which the
    /// caller must not fold into `NotFound`: "nothing to look up" and "looked and it is not here"
    /// send an operator to different places.
    pub async fn resolve_host_id<R: LldpResolver>(
        &self,
        resolver: &R,
        site_id: Uuid,
    ) -> IdentityResolution {
        let mut strategy_ran = false;
        let mut saw_ambiguous = false;

        // The address outranks the name: it is the far end's own statement of where it lives,
        // while `sysName` is a label an operator assigned and a vendor may have defaulted.
        if let Some(address) = self.address.filter(is_usable_identity_address) {
            strategy_ran = true;
            let by_address = resolver.find_host_by_ip(&address, site_id).await;
            if !by_address.is_unresolved() {
                return by_address;
            }
            saw_ambiguous |= by_address == IdentityResolution::Ambiguous;
        }

        if let Some(sys_name) = self.sys_name.map(str::trim).filter(|s| !s.is_empty()) {
            strategy_ran = true;
            let by_sys_name = resolver.find_host_by_sys_name(sys_name, site_id).await;
            if !by_sys_name.is_unresolved() {
                return by_sys_name;
            }
            saw_ambiguous |= by_sys_name == IdentityResolution::Ambiguous;
        }

        match (saw_ambiguous, strategy_ran) {
            (true, _) => IdentityResolution::Ambiguous,
            (false, true) => IdentityResolution::NotFound,
            (false, false) => IdentityResolution::NoStrategy,
        }
    }
}

/// Whether an advertised address can stand for a device at all.
///
/// LLDP's management-address parser accepts any 4 or 16 bytes (`parse_lldp_mgmt_addr`), and
/// misconfigured or half-initialised gear publishes all of these. None of them names a device on
/// this network, and looking any of them up would at best waste a query and at worst match the
/// wrong host: `0.0.0.0` and `127.0.0.1` are held by nothing, while a link-local address is
/// per-segment and can legitimately repeat across devices.
pub fn is_usable_identity_address(addr: &IpAddr) -> bool {
    if addr.is_unspecified() || addr.is_loopback() || addr.is_multicast() {
        return false;
    }
    match addr {
        IpAddr::V4(v4) => !v4.is_link_local() && !v4.is_broadcast(),
        IpAddr::V6(v6) => {
            // `is_unicast_link_local` is still unstable, so test the `fe80::/10` prefix directly.
            v6.segments()[0] & 0xffc0 != 0xfe80
        }
    }
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

/// Decode a textual LLDP TLV payload.
///
/// Lossy, so a single bad byte degrades one character rather than discarding the whole neighbour,
/// and NUL-stripped, because these strings land in the `lldp_chassis_id` / `lldp_port_id` JSONB
/// columns and PostgreSQL rejects the escape outright (SQLSTATE 22P05). D-Link's DGS series
/// NUL-terminates its port identifiers — `31 00` for port `"1"` — which used to fail the write of
/// the whole host (GH #668). Stripping here also makes the value comparable: `"1\0"` matches no
/// interface named `"1"`.
fn decode_tlv_text(value: &[u8]) -> String {
    strip_nuls(&String::from_utf8_lossy(value)).into_owned()
}

impl LldpChassisId {
    /// Parse from SNMP raw values (subtype byte + value bytes).
    ///
    /// LLDP chassis ID TLV format: subtype (1 byte) + value (variable)
    pub fn from_snmp(subtype: u8, value: &[u8]) -> Option<Self> {
        match subtype {
            1 => Some(Self::ChassisComponent(decode_tlv_text(value))),
            2 => Some(Self::InterfaceAlias(decode_tlv_text(value))),
            3 => Some(Self::PortComponent(decode_tlv_text(value))),
            4 => parse_mac_id(value).map(Self::MacAddress),
            5 => parse_network_address(value).map(Self::NetworkAddress),
            6 => Some(Self::InterfaceName(decode_tlv_text(value))),
            7 => Some(Self::LocallyAssigned(decode_tlv_text(value))),
            _ => None,
        }
    }

    /// Build a chassis ID from an identifier string, for sources that report LLDP neighbor data
    /// without the IEEE subtype byte.
    ///
    /// The UniFi controller API is the motivating case: `lldp_table[].chassis_id` is already a
    /// decoded string, so there is no subtype to dispatch on. MAC-shaped values canonicalize to
    /// the same lowercase colon form [`Self::from_snmp`] produces — which is what lets the
    /// raw-string `hosts.chassis_id` fallback in [`Self::resolve_host_id`] match a device
    /// regardless of whether SNMP or a controller recorded it. Anything else is
    /// [`Self::LocallyAssigned`], the honest label for "device-specific identifier, subtype
    /// unknown", and the one whose resolution assumes least about the value.
    pub fn from_identifier_str(value: &str) -> Self {
        match canonical_mac(value) {
            Some(mac) => Self::MacAddress(mac),
            None => Self::LocallyAssigned(value.to_string()),
        }
    }

    /// The identifier as the remote device advertised it, in the same canonical form the daemon
    /// stores on `hosts.chassis_id` for a device's own LLDP local identity. Having one definition
    /// is what lets a neighbor's chassis ID be matched against a scanned host's own chassis ID.
    pub fn identifier(&self) -> String {
        match self {
            Self::NetworkAddress(addr) => addr.to_string(),
            Self::ChassisComponent(s)
            | Self::InterfaceAlias(s)
            | Self::PortComponent(s)
            | Self::MacAddress(s)
            | Self::InterfaceName(s)
            | Self::LocallyAssigned(s) => s.clone(),
        }
    }

    /// Resolve this chassis ID to a host_id, trying each applicable strategy in turn.
    ///
    /// Subtype-specific strategy first — it matches the identifier against the type of column it
    /// actually is:
    /// - MacAddress: `ip_addresses.mac_address`, then `interfaces.mac_address`
    /// - NetworkAddress: `ip_addresses.ip_address`
    /// - InterfaceName: `interfaces.if_descr`
    /// - ChassisComponent/LocallyAssigned: `hosts.chassis_id`
    /// - InterfaceAlias/PortComponent: nothing reliable
    ///
    /// Then three subtype-independent fallbacks, all needed on real hardware:
    ///
    /// 1. `hosts.chassis_id`, for *every* subtype. A switch's chassis MAC is not required to be
    ///    any of its port MACs, and on several vendors it isn't: the Netgear GS724Tv3 advertises a
    ///    chassis MAC one octet below its port MACs and carries it on no interface and no IP, so
    ///    the MacAddress strategy above finds nothing (GH #664). The daemon already records each
    ///    scanned device's own LLDP chassis ID on `hosts.chassis_id` in this same canonical form,
    ///    which is the only place that MAC exists server-side.
    /// 2. An address the far end published for itself, against `ip_addresses.ip_address`. The
    ///    objection to using `lldpRemManAddr` has always been that it is "where you manage the
    ///    device, not necessarily the physical connection point" — true of a *port*, and beside
    ///    the point for a *device*, which is all this tier claims to identify. It is the only tier
    ///    that can place a device whose SNMP tables came back empty: such a host carries no
    ///    interface row to hold the chassis MAC and no `chassis_id` of its own, so tiers above are
    ///    structurally dead, yet its management address is exactly what its neighbours publish
    ///    (GH #668).
    /// 3. The neighbor's advertised `sysName` against `hosts.sys_name` — the same last resort CDP
    ///    has always used, for devices whose chassis identity is unrecoverable but whose name was
    ///    captured by an SNMP scan. Only accepted when exactly one host matches. Deliberately last:
    ///    `sysName` is operator-assigned and frequently a vendor default, so it is the weakest of
    ///    the three.
    pub async fn resolve_host_id<R: LldpResolver>(
        &self,
        resolver: &R,
        site_id: Uuid,
        advertised: AdvertisedIdentity<'_>,
    ) -> IdentityResolution {
        let mut strategy_ran = false;

        // An identifier that named more than one host anywhere on the ladder. Remembered rather
        // than returned on the spot, because a later tier may still tell the candidates apart —
        // but it must outrank `NotFound` at the end, since "this names two devices" and "this
        // names none" call for opposite things from an operator.
        let mut saw_ambiguous = false;

        let by_subtype = match self {
            Self::MacAddress(mac) => resolver.find_host_by_mac(mac, site_id).await,
            Self::NetworkAddress(ip) => resolver.find_host_by_ip(ip, site_id).await,
            Self::InterfaceName(name) => resolver.find_host_by_if_name(name, site_id).await,
            Self::ChassisComponent(id) | Self::LocallyAssigned(id) => {
                resolver.find_host_by_chassis_id(id, site_id).await
            }
            // These subtypes don't have reliable resolution strategies
            Self::InterfaceAlias(_) | Self::PortComponent(_) => IdentityResolution::NoStrategy,
        };
        strategy_ran |= !matches!(self, Self::InterfaceAlias(_) | Self::PortComponent(_));
        if !by_subtype.is_unresolved() {
            return by_subtype;
        }
        saw_ambiguous |= by_subtype == IdentityResolution::Ambiguous;

        let identifier = self.identifier();
        if !identifier.is_empty() {
            strategy_ran = true;
            let by_chassis = resolver.find_host_by_chassis_id(&identifier, site_id).await;
            if !by_chassis.is_unresolved() {
                return by_chassis;
            }
            saw_ambiguous |= by_chassis == IdentityResolution::Ambiguous;
        }

        // Tiers 2 and 3, which need nothing from the chassis id and are therefore shared with CDP.
        // `NoStrategy` here means neither ran, which must not promote a `NotFound` verdict.
        let by_advertised = advertised.resolve_host_id(resolver, site_id).await;
        if !by_advertised.is_unresolved() {
            return by_advertised;
        }
        strategy_ran |= by_advertised != IdentityResolution::NoStrategy;
        saw_ambiguous |= by_advertised == IdentityResolution::Ambiguous;

        match (saw_ambiguous, strategy_ran) {
            (true, _) => IdentityResolution::Ambiguous,
            (false, true) => IdentityResolution::NotFound,
            (false, false) => IdentityResolution::NoStrategy,
        }
    }
}

impl LldpPortId {
    /// Parse from SNMP raw values (subtype byte + value bytes).
    ///
    /// LLDP port ID TLV format: subtype (1 byte) + value (variable)
    pub fn from_snmp(subtype: u8, value: &[u8]) -> Option<Self> {
        match subtype {
            1 => Some(Self::InterfaceAlias(decode_tlv_text(value))),
            2 => Some(Self::PortComponent(decode_tlv_text(value))),
            3 => parse_mac_id(value).map(Self::MacAddress),
            4 => parse_network_address(value).map(Self::NetworkAddress),
            5 => Some(Self::InterfaceName(decode_tlv_text(value))),
            6 => Some(Self::AgentCircuitId(decode_tlv_text(value))),
            7 => Some(Self::LocallyAssigned(decode_tlv_text(value))),
            _ => None,
        }
    }

    /// The far end's own name for this port, where the subtype names a port at all.
    ///
    /// `None` for the two subtypes that encode something else entirely: a MAC identifies the port
    /// without naming it (see [`Self::port_mac`]), and a network address names the device. Both
    /// would look like names if stringified, and an interface row keyed on one would never match
    /// the ifTable row it stands for. `AgentCircuitId` is excluded for the same reason — a DHCP
    /// relay circuit id is an access-loop identifier, not the port's name.
    pub fn port_name(&self) -> Option<&str> {
        match self {
            Self::InterfaceName(s)
            | Self::InterfaceAlias(s)
            | Self::PortComponent(s)
            | Self::LocallyAssigned(s) => Some(s.as_str()),
            Self::MacAddress(_) | Self::NetworkAddress(_) | Self::AgentCircuitId(_) => None,
        }
    }

    /// The port's own MAC, where the subtype carried one. Evidence, though not a name.
    pub fn port_mac(&self) -> Option<&str> {
        match self {
            Self::MacAddress(mac) => Some(mac.as_str()),
            _ => None,
        }
    }

    /// Build a port ID from an identifier string, for sources that report LLDP neighbor data
    /// without the IEEE subtype byte. See [`LldpChassisId::from_identifier_str`] — same rationale.
    pub fn from_identifier_str(value: &str) -> Self {
        match canonical_mac(value) {
            Some(mac) => Self::MacAddress(mac),
            None => Self::LocallyAssigned(value.to_string()),
        }
    }

    /// Resolve this port ID to an interface_id using the appropriate lookup strategy.
    ///
    /// Requires the host_id to be already known (from chassis ID resolution), so every lookup is
    /// scoped to one device's own interfaces.
    ///
    /// The resolution strategy depends on the port ID subtype:
    /// - MacAddress: Look up via interfaces.mac_address, and only when that MAC belongs to exactly
    ///   one of the host's ports — see [`LldpResolver::find_interface_by_mac`]
    /// - NetworkAddress: Look up via ip_address_id FK on interfaces
    /// - InterfaceName/PortComponent/AgentCircuitId/LocallyAssigned/InterfaceAlias: device-local
    ///   port identifier — see [`Self::resolve_device_local_port`]
    pub async fn resolve_interface_id<R: LldpResolver>(
        &self,
        resolver: &R,
        host_id: Uuid,
    ) -> IdentityResolution {
        match self {
            Self::MacAddress(mac) => resolver.find_interface_by_mac(mac, host_id).await,
            // `ifAlias` is user-configurable and not required to be unique, so it is resolved the
            // same way every other name-shaped identifier is: against the far end's own ifDescr /
            // ifName / ifAlias columns, on a single match only. Declining outright cost the port
            // on every device that advertises subtype 1 — which on Westermo WeOS is the bare port
            // name its ifAlias column already holds.
            Self::InterfaceAlias(id) => {
                Self::resolve_device_local_port(resolver, id, host_id).await
            }
            Self::NetworkAddress(ip) => {
                IdentityResolution::found(resolver.find_interface_by_ip(ip, host_id).await)
            }
            Self::InterfaceName(id)
            | Self::PortComponent(id)
            | Self::AgentCircuitId(id)
            | Self::LocallyAssigned(id) => {
                Self::resolve_device_local_port(resolver, id, host_id).await
            }
        }
    }

    /// Resolve a device-local port identifier (subtypes 1, 2, 5, 6 and 7) against one host's
    /// interfaces.
    ///
    /// These subtypes are "whatever the device calls this port", which sounds unusable but in
    /// practice is one of two things, both of which the remote host's own ifTable already carries:
    /// the port's `ifDescr`/`ifName` (Aruba/HP ProCurve advertise bare port numbers such as `"41"`,
    /// which is exactly that switch's `ifDescr`), or its `ifIndex` as a decimal string.
    ///
    /// Treating the whole family as unresolvable is what produced the reported symptom: the
    /// neighbor resolved as far as the host and stopped, and a host-only neighbor renders no edge,
    /// so switches were absent from L2 Physical entirely (GH #649). Names are tried before indexes
    /// because a bare port number is ambiguous between the two and the name is the device's own
    /// label; both are scoped to the single already-resolved host, so a wrong match cannot reach
    /// another device.
    ///
    /// Subtype 5 (`interfaceName`) is routed here too, despite the name promising a real ifName.
    /// A D-Link DGS-1210-48 advertises subtype 5 with the bare port number `"2"` while its own
    /// interfaces are `Slot0/1..Slot0/48` — the value is an ifIndex wearing the wrong label, and
    /// name-only lookup could never match it (GH #668). This ladder is a strict superset of what
    /// subtype 5 did before: names are still tried first, so a device that means what it says is
    /// unaffected, and the index fallback is bounded to the one host already resolved.
    async fn resolve_device_local_port<R: LldpResolver>(
        resolver: &R,
        id: &str,
        host_id: Uuid,
    ) -> IdentityResolution {
        let id = id.trim();
        if id.is_empty() {
            return IdentityResolution::NoStrategy;
        }

        if let Some(interface_id) = resolver.find_interface_by_name(id, host_id).await {
            return IdentityResolution::Resolved(interface_id);
        }

        match id.parse::<i32>() {
            Ok(if_index) => IdentityResolution::found(
                resolver.find_interface_by_if_index(if_index, host_id).await,
            ),
            Err(_) => IdentityResolution::NotFound,
        }
    }
}

/// Parse an LLDP MAC-address identifier (chassis subtype 4 / port subtype 3).
///
/// Per IEEE 802.1AB a macAddress value is 6 raw octets, but some vendors
/// (MikroTik RouterOS, Extreme EXOS) instead send it as an ASCII string such as
/// `"48:A9:8A:BD:B4:7D"`. Accept both shapes and normalize to the same canonical
/// lowercase colon-separated form (`format_mac`) so downstream MAC matching is
/// independent of the wire encoding. Returns `None` for values that are neither.
///
/// The ASCII form is accepted with or without zero-padded octets: firmware that renders a MAC as
/// a string rather than emitting octets is formatting it itself, and `%x` ("0:1a:2b:0:10:0" —
/// also net-snmp's own display format) is as likely as `%02x`. Being strict here is not a
/// partial loss but a total one: an unparseable chassis ID makes `from_snmp` return `None`, the
/// neighbor record is never stored, and the device silently contributes nothing to L2 topology
/// at all — indistinguishable from a switch that advertises no neighbors.
fn parse_mac_id(value: &[u8]) -> Option<String> {
    if value.len() == 6 {
        return Some(format_mac(value));
    }

    // Vendor quirk: MAC encoded as an ASCII string instead of 6 raw octets.
    canonical_mac(std::str::from_utf8(value).ok()?)
}

/// Canonicalize a MAC rendered as text into the same lowercase colon-separated form
/// [`format_mac`] produces, or `None` if the value is not a MAC.
///
/// This is the text-only half of [`parse_mac_id`], split out because sources that report
/// identifiers as strings rather than raw TLV octets must not go through the 6-raw-octet
/// branch: a six-character name such as `"Switch"` is six bytes, and would otherwise be
/// silently reinterpreted as the MAC `53:77:69:74:63:68`.
///
/// Accepts the form with or without zero-padded octets: firmware that renders a MAC as a
/// string is formatting it itself, and `%x` ("0:1a:2b:0:10:0" — also net-snmp's own display
/// format) is as likely as `%02x`.
pub fn canonical_mac(value: &str) -> Option<String> {
    let s = value.trim();
    if let Ok(mac) = s.parse::<mac_address::MacAddress>() {
        return Some(format_mac(&mac.bytes()));
    }

    // Same string form, octets not zero-padded. Six colon-separated groups of one or two hex
    // digits is a MAC under any reading, and this is only reached once the padded parse has
    // failed, so there is nothing else it could be mistaken for.
    let octets: Vec<u8> = s
        .split(':')
        .map(|group| match group.len() {
            1 | 2 => u8::from_str_radix(group, 16).ok(),
            _ => None,
        })
        .collect::<Option<Vec<u8>>>()?;
    (octets.len() == 6).then(|| format_mac(&octets))
}

/// The forms a MAC-valued LLDP identifier takes on the wire.
///
/// This is the inverse of the tolerance [`parse_mac_id`] already implements, named rather than
/// implied. A `PhysAddress` is six raw octets, and [`Self::Octets`] is what real firmware sends —
/// but two vendors in the field send the identifier as *text* instead, so the parser accepts that
/// and something has to be able to produce it.
///
/// Naming the encoding is what stops the trap `SNMP-TEST-ENV.md` records as having "caught three
/// fixtures so far": a MAC written as a string where octets are meant arrives as 17 ASCII bytes,
/// [`crate::daemon::discovery::integration::snmp::values::value_to_mac`] correctly refuses to read
/// it as an address, and the value is silently dropped while everything downstream still looks
/// healthy. A caller that wants text now has to say so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MacEncoding {
    /// Six raw octets — what a conforming agent sends, and the default everywhere.
    #[default]
    Octets,
    /// `00:1a:2b:00:10:01` — zero-padded ASCII.
    AsciiLower,
    /// `00:1A:2B:00:10:00` — zero-padded ASCII, upper case. TP-Link's TL-SX3016F (GH #668).
    AsciiUpper,
    /// `0:1a:2b:0:10:0` — ASCII with unpadded octets, which is also net-snmp's own `%x` display
    /// form. ExtremeXOS sends its chassis id this way.
    AsciiAbbreviated,
}

impl MacEncoding {
    /// Render a MAC as this encoding puts it on the wire.
    pub fn encode(self, mac: &mac_address::MacAddress) -> Vec<u8> {
        let bytes = mac.bytes();
        match self {
            Self::Octets => bytes.to_vec(),
            Self::AsciiLower => format_mac(&bytes).into_bytes(),
            Self::AsciiUpper => format_mac(&bytes).to_uppercase().into_bytes(),
            Self::AsciiAbbreviated => bytes
                .iter()
                .map(|b| format!("{:x}", b))
                .collect::<Vec<_>>()
                .join(":")
                .into_bytes(),
        }
    }

    /// Render an identifier that [`parse_mac_id`] canonicalised back into wire bytes.
    ///
    /// Falls back to the value's own bytes when it is not a MAC at all. That case is reachable
    /// only for a `MacAddress` variant holding a non-MAC, which the constructors do not produce;
    /// emitting the text verbatim keeps a malformed value visible instead of turning it into six
    /// arbitrary octets.
    fn encode_identifier(self, value: &str) -> Vec<u8> {
        match value.parse::<mac_address::MacAddress>() {
            Ok(mac) => self.encode(&mac),
            Err(_) => value.as_bytes().to_vec(),
        }
    }
}

/// Render an LLDP network address the way [`parse_network_address`] reads one: IANA address
/// family byte, then the address octets.
fn encode_network_address(addr: &IpAddr) -> Vec<u8> {
    let mut bytes = Vec::new();
    match addr {
        IpAddr::V4(v4) => {
            bytes.push(1);
            bytes.extend_from_slice(&v4.octets());
        }
        IpAddr::V6(v6) => {
            bytes.push(2);
            bytes.extend_from_slice(&v6.octets());
        }
    }
    bytes
}

impl LldpChassisId {
    /// The subtype and value bytes an agent advertising this identifier would send — the inverse
    /// of [`Self::from_snmp`].
    ///
    /// The subtype comes from the variant, so a fixture cannot advertise subtype 4 carrying
    /// something that is not an address, or subtype 7 carrying raw octets. `mac_encoding` is
    /// consulted only by [`Self::MacAddress`]; every other variant is text by definition.
    pub fn to_snmp(&self, mac_encoding: MacEncoding) -> (u8, Vec<u8>) {
        match self {
            Self::ChassisComponent(s) => (1, s.as_bytes().to_vec()),
            Self::InterfaceAlias(s) => (2, s.as_bytes().to_vec()),
            Self::PortComponent(s) => (3, s.as_bytes().to_vec()),
            Self::MacAddress(s) => (4, mac_encoding.encode_identifier(s)),
            Self::NetworkAddress(addr) => (5, encode_network_address(addr)),
            Self::InterfaceName(s) => (6, s.as_bytes().to_vec()),
            Self::LocallyAssigned(s) => (7, s.as_bytes().to_vec()),
        }
    }
}

impl LldpPortId {
    /// The subtype and value bytes an agent advertising this identifier would send — the inverse
    /// of [`Self::from_snmp`]. See [`LldpChassisId::to_snmp`]; the subtype numbering differs.
    pub fn to_snmp(&self, mac_encoding: MacEncoding) -> (u8, Vec<u8>) {
        match self {
            Self::InterfaceAlias(s) => (1, s.as_bytes().to_vec()),
            Self::PortComponent(s) => (2, s.as_bytes().to_vec()),
            Self::MacAddress(s) => (3, mac_encoding.encode_identifier(s)),
            Self::NetworkAddress(addr) => (4, encode_network_address(addr)),
            Self::InterfaceName(s) => (5, s.as_bytes().to_vec()),
            Self::AgentCircuitId(s) => (6, s.as_bytes().to_vec()),
            Self::LocallyAssigned(s) => (7, s.as_bytes().to_vec()),
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

#[cfg(test)]
mod wire_round_trip_tests {
    use super::*;
    use strum::VariantNames;

    /// One value per variant. The length assertion below is what keeps this honest: a new
    /// variant that nobody adds here fails the test rather than escaping the property.
    fn chassis_variants() -> Vec<LldpChassisId> {
        vec![
            LldpChassisId::ChassisComponent("backplane-1".into()),
            LldpChassisId::InterfaceAlias("uplink to core".into()),
            LldpChassisId::PortComponent("port-7".into()),
            LldpChassisId::MacAddress("00:1a:2b:3c:4d:5e".into()),
            LldpChassisId::NetworkAddress("192.0.2.7".parse().unwrap()),
            LldpChassisId::InterfaceName("GigabitEthernet0/1".into()),
            LldpChassisId::LocallyAssigned("C230408".into()),
        ]
    }

    fn port_variants() -> Vec<LldpPortId> {
        vec![
            LldpPortId::InterfaceAlias("Ring port to peer".into()),
            LldpPortId::PortComponent("slot0-3".into()),
            LldpPortId::MacAddress("00:07:7c:20:01:e3".into()),
            LldpPortId::NetworkAddress("198.51.100.4".parse().unwrap()),
            LldpPortId::InterfaceName("ethernet1/1/14:1".into()),
            LldpPortId::AgentCircuitId("circuit-9".into()),
            LldpPortId::LocallyAssigned("41".into()),
        ]
    }

    /// The subtype is the variant, in both directions. Emitting through `to_snmp` and reading
    /// back through `from_snmp` must land on the same identity for every variant — which is what
    /// makes a fixture built from these enums unable to advertise a subtype that contradicts its
    /// value.
    #[test]
    fn every_identifier_variant_survives_the_wire() {
        assert_eq!(chassis_variants().len(), LldpChassisId::VARIANTS.len());
        assert_eq!(port_variants().len(), LldpPortId::VARIANTS.len());

        for id in chassis_variants() {
            let (subtype, value) = id.to_snmp(MacEncoding::Octets);
            assert_eq!(LldpChassisId::from_snmp(subtype, &value), Some(id));
        }
        for id in port_variants() {
            let (subtype, value) = id.to_snmp(MacEncoding::Octets);
            assert_eq!(LldpPortId::from_snmp(subtype, &value), Some(id));
        }
    }

    /// All four encodings of one address reach one identity.
    ///
    /// This is the property the lab depends on: `switch-macport-01` is named by six raw octets on
    /// its own device and by ASCII text from `switch-dlink-01`, and both have to resolve to the
    /// same host. It is also the guard on the trap itself — a MAC sent as text is not a different
    /// MAC, it is the same one, and anything that reads it as six arbitrary bytes fails here.
    #[test]
    fn every_mac_encoding_reads_back_as_the_same_address() {
        let mac: mac_address::MacAddress = "00:ad:24:af:4e:00".parse().unwrap();
        let expected = LldpChassisId::MacAddress("00:ad:24:af:4e:00".into());

        for encoding in [
            MacEncoding::Octets,
            MacEncoding::AsciiLower,
            MacEncoding::AsciiUpper,
            MacEncoding::AsciiAbbreviated,
        ] {
            let (subtype, value) = expected.to_snmp(encoding);
            assert_eq!(subtype, 4);
            assert_eq!(
                LldpChassisId::from_snmp(subtype, &value),
                Some(expected.clone()),
                "{encoding:?} did not read back as the same address"
            );
        }

        // And the encodings really are distinct on the wire, or the check above proves nothing.
        assert_eq!(MacEncoding::Octets.encode(&mac).len(), 6);
        assert_eq!(MacEncoding::AsciiLower.encode(&mac), b"00:ad:24:af:4e:00");
        assert_eq!(MacEncoding::AsciiUpper.encode(&mac), b"00:AD:24:AF:4E:00");
        assert_eq!(
            MacEncoding::AsciiAbbreviated.encode(&mac),
            b"0:ad:24:af:4e:0"
        );
    }
}

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
    fn test_chassis_id_from_snmp_mac_ascii_string() {
        // MikroTik/Extreme quirk: subtype 4 sent as 17-byte ASCII "48:A9:8A:BD:B4:7D"
        let ascii = b"48:A9:8A:BD:B4:7D";
        let chassis_id = LldpChassisId::from_snmp(4, ascii);
        assert_eq!(
            chassis_id,
            Some(LldpChassisId::MacAddress("48:a9:8a:bd:b4:7d".to_string()))
        );
    }

    /// Firmware that renders a MAC as a string rather than emitting octets formats it itself, and
    /// abbreviated octets are as likely as padded ones. Rejecting the abbreviated form does not
    /// degrade the record — it discards the whole neighbor, so the device contributes nothing to
    /// L2 topology and looks identical to one advertising no neighbors at all.
    #[test]
    fn test_chassis_id_from_snmp_mac_ascii_unpadded() {
        let ascii = b"0:1a:2b:0:10:0";
        assert_eq!(
            LldpChassisId::from_snmp(4, ascii),
            Some(LldpChassisId::MacAddress("00:1a:2b:00:10:00".to_string())),
            "an unpadded ASCII MAC must normalize to the same canonical form as a padded one"
        );
    }

    /// Both wire encodings of one address must land on the same string, or a neighbor advertising
    /// the abbreviated form would never match the host that reported the padded form.
    #[test]
    fn test_mac_encodings_agree_on_one_canonical_form() {
        let raw = LldpChassisId::from_snmp(4, &[0x00, 0x1a, 0x2b, 0x00, 0x10, 0x00]);
        let padded = LldpChassisId::from_snmp(4, b"00:1a:2b:00:10:00");
        let unpadded = LldpChassisId::from_snmp(4, b"0:1a:2b:0:10:0");
        assert_eq!(raw, padded);
        assert_eq!(raw, unpadded);
    }

    /// The whole point of `from_identifier_str`: a controller-sourced neighbor and an
    /// SNMP-sourced one must produce byte-identical chassis IDs, because `resolve_host_id`
    /// falls back to raw string equality against `hosts.chassis_id`.
    #[test]
    fn test_identifier_str_agrees_with_snmp_canonical_form() {
        let from_snmp = LldpChassisId::from_snmp(4, &[0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]);
        assert_eq!(
            LldpChassisId::from_identifier_str("0:1A:2b:3C:4d:5E"),
            from_snmp.unwrap()
        );
        assert_eq!(
            LldpPortId::from_identifier_str("00:1A:2B:3C:4D:5E"),
            LldpPortId::from_snmp(3, &[0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]).unwrap()
        );
    }

    /// A string source has no subtype byte, so it must never take the six-raw-octets branch:
    /// plenty of device names are exactly six characters, and reading one as a MAC would
    /// invent a neighbor that does not exist.
    #[test]
    fn test_identifier_str_does_not_read_short_names_as_macs() {
        for name in ["Switch", "ap-101", "core01"] {
            assert_eq!(
                LldpChassisId::from_identifier_str(name),
                LldpChassisId::LocallyAssigned(name.to_string()),
                "expected {name:?} to stay an opaque identifier"
            );
        }
    }

    #[test]
    fn test_identifier_str_non_mac_is_locally_assigned() {
        assert_eq!(
            LldpChassisId::from_identifier_str("edge-switch.lan"),
            LldpChassisId::LocallyAssigned("edge-switch.lan".to_string())
        );
        assert_eq!(
            LldpPortId::from_identifier_str("Port 7"),
            LldpPortId::LocallyAssigned("Port 7".to_string())
        );
    }

    #[test]
    fn test_port_id_from_snmp_mac_ascii_unpadded() {
        assert_eq!(
            LldpPortId::from_snmp(3, b"0:c:29:aa:bb:c0"),
            Some(LldpPortId::MacAddress("00:0c:29:aa:bb:c0".to_string()))
        );
    }

    /// The tolerance must not swallow things that merely look colon-separated. (A value of exactly
    /// six bytes is deliberately absent: the spec defines subtype 4 as six binary octets, so six
    /// bytes are octets by definition and no heuristic can second-guess that.)
    #[test]
    fn test_chassis_id_from_snmp_mac_rejects_non_macs() {
        for not_a_mac in [
            &b"0:1a:2b:0:10"[..],     // five groups
            &b"0:1a:2b:0:10:0:5"[..], // seven groups
            &b"0:1a:2b:0:10:zz"[..],  // non-hex group
            &b"0:1a:2b:0:10:000"[..], // over-long group
            &b"not-a-mac-at-all"[..],
        ] {
            assert_eq!(
                LldpChassisId::from_snmp(4, not_a_mac),
                None,
                "expected {:?} to be rejected",
                String::from_utf8_lossy(not_a_mac)
            );
        }
    }

    #[test]
    fn test_chassis_id_from_snmp_mac_invalid() {
        // A non-MAC, non-6-byte value for subtype 4 is rejected.
        let chassis_id = LldpChassisId::from_snmp(4, b"not-a-mac");
        assert_eq!(chassis_id, None);
    }

    #[test]
    fn test_port_id_from_snmp_mac_raw_octets() {
        let mac_bytes = [0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e];
        let port_id = LldpPortId::from_snmp(3, &mac_bytes);
        assert_eq!(
            port_id,
            Some(LldpPortId::MacAddress("00:1a:2b:3c:4d:5e".to_string()))
        );
    }

    #[test]
    fn test_port_id_from_snmp_mac_ascii_string() {
        let ascii = b"48:A9:8A:BD:B4:7D";
        let port_id = LldpPortId::from_snmp(3, ascii);
        assert_eq!(
            port_id,
            Some(LldpPortId::MacAddress("48:a9:8a:bd:b4:7d".to_string()))
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

/// Resolution-strategy tests against an in-memory inventory.
///
/// These exercise which identity a neighbor is joined on, which is where the reported "L2
/// Physical is empty" failures live — a neighbor that resolves only as far as the remote host
/// draws no edge at all, so a strategy gap and a genuinely unknown device look identical from
/// the outside.
#[cfg(test)]
mod resolution_tests {
    use super::*;
    use async_trait::async_trait;
    use std::net::IpAddr;

    #[derive(Default)]
    struct FakeHost {
        id: Uuid,
        chassis_id: Option<String>,
        sys_name: Option<String>,
        /// The host's `ip_addresses` rows, for the address tier. A host may hold several, and two
        /// hosts holding one is the duplicate the production `get_unique` reports as ambiguous.
        addresses: Vec<IpAddr>,
    }

    #[derive(Default)]
    struct FakeInterface {
        id: Uuid,
        host_id: Uuid,
        if_descr: Option<String>,
        if_name: Option<String>,
        if_alias: Option<String>,
        if_index: i32,
        mac: Option<String>,
        /// IANA ifType. Defaults to 0 rather than a physical value so a test that does not care
        /// is still treated as a port; the virtual families are named explicitly where they matter.
        if_type: i32,
    }

    impl FakeInterface {
        /// Mirrors the production resolver's SQL scope: virtual rows are not candidate far ends
        /// and do not contest a MAC lookup.
        fn is_physical(&self) -> bool {
            !crate::server::interfaces::r#impl::base::if_type::EXCLUDED_IF_TYPES
                .contains(&self.if_type)
        }
    }

    /// Stands in for the database: the same inventory the production resolver queries, matched
    /// with the same column semantics (exact match, and "exactly one" for the non-unique columns).
    #[derive(Default)]
    struct FakeInventory {
        hosts: Vec<FakeHost>,
        interfaces: Vec<FakeInterface>,
    }

    impl FakeInventory {
        /// The same 0 / 1 / many verdict `Storage::get_unique` returns, so these fakes cannot
        /// quietly disagree with the database about what "identifies a row" means.
        ///
        /// It is still a re-implementation, and that is the ceiling on what these tests can
        /// prove: they pin the *ladder*, not the queries. The query behaviour is covered against
        /// a real Postgres in `crate::tests::lldp_resolution`.
        fn only<T>(mut matches: Vec<T>) -> Unique<T> {
            match matches.len() {
                0 => Unique::None,
                1 => matches.pop().map(Unique::One).unwrap_or(Unique::None),
                _ => Unique::Multiple,
            }
        }
    }

    #[async_trait]
    impl LldpResolver for FakeInventory {
        async fn find_host_by_mac(&self, mac: &str, _site_id: Uuid) -> IdentityResolution {
            // Collapsed to distinct hosts before the single-match rule, exactly as the production
            // resolver does: many ports of one switch carrying the chassis MAC is one host.
            let hosts: Vec<Uuid> = self
                .interfaces
                .iter()
                .filter(|i| i.mac.as_deref() == Some(mac))
                .map(|i| i.host_id)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            IdentityResolution::from_unique(Self::only(hosts))
        }

        async fn find_host_by_ip(&self, ip: &IpAddr, _site_id: Uuid) -> IdentityResolution {
            IdentityResolution::from_unique(
                Self::only(
                    self.hosts
                        .iter()
                        .filter(|h| h.addresses.contains(ip))
                        .collect(),
                )
                .map(|h| h.id),
            )
        }

        async fn find_host_by_if_name(&self, name: &str, _site_id: Uuid) -> IdentityResolution {
            IdentityResolution::from_unique(Self::only(
                self.interfaces
                    .iter()
                    .filter(|i| i.if_descr.as_deref() == Some(name))
                    .map(|i| i.host_id)
                    .collect(),
            ))
        }

        async fn find_host_by_chassis_id(
            &self,
            chassis_id: &str,
            _site_id: Uuid,
        ) -> IdentityResolution {
            IdentityResolution::from_unique(
                Self::only(
                    self.hosts
                        .iter()
                        .filter(|h| h.chassis_id.as_deref() == Some(chassis_id))
                        .collect(),
                )
                .map(|h| h.id),
            )
        }

        async fn find_host_by_sys_name(
            &self,
            sys_name: &str,
            _site_id: Uuid,
        ) -> IdentityResolution {
            IdentityResolution::from_unique(
                Self::only(
                    self.hosts
                        .iter()
                        .filter(|h| h.sys_name.as_deref() == Some(sys_name))
                        .collect(),
                )
                .map(|h| h.id),
            )
        }

        async fn find_interface_by_mac(&self, mac: &str, host_id: Uuid) -> IdentityResolution {
            let matches: Vec<Uuid> = self
                .interfaces
                .iter()
                .filter(|i| i.host_id == host_id && i.mac.as_deref() == Some(mac))
                .filter(|i| i.is_physical())
                .map(|i| i.id)
                .collect();

            match matches.len() {
                0 => IdentityResolution::NotFound,
                1 => IdentityResolution::Resolved(matches[0]),
                _ => IdentityResolution::Ambiguous,
            }
        }

        async fn find_interface_by_name(&self, name: &str, host_id: Uuid) -> Option<Uuid> {
            self.interfaces
                .iter()
                .find(|i| {
                    i.host_id == host_id
                        && (i.if_descr.as_deref() == Some(name)
                            || i.if_name.as_deref() == Some(name)
                            || i.if_alias.as_deref() == Some(name))
                })
                .map(|i| i.id)
        }

        async fn find_interface_by_if_index(&self, if_index: i32, host_id: Uuid) -> Option<Uuid> {
            self.interfaces
                .iter()
                .find(|i| i.host_id == host_id && i.if_index == if_index)
                .map(|i| i.id)
        }

        async fn find_interface_by_ip(&self, _ip: &IpAddr, _host_id: Uuid) -> Option<Uuid> {
            None
        }
    }

    /// GH #664: the Netgear GS724Tv3 advertises a chassis MAC that is on none of its ports and
    /// on no IP — the only server-side record of it is the host's own `chassis_id`, captured
    /// from its LLDP local identity. Matching MACs against interfaces/IPs alone leaves every
    /// neighbour of such a switch unresolved and L2 Physical empty.
    #[tokio::test]
    async fn chassis_mac_absent_from_ports_resolves_via_the_hosts_own_chassis_id() {
        let switch = Uuid::new_v4();
        let inventory = FakeInventory {
            hosts: vec![FakeHost {
                id: switch,
                chassis_id: Some("00:1a:2b:3c:4d:63".to_string()),
                ..Default::default()
            }],
            // Ports carry a different MAC than the chassis — the whole point of the bug.
            interfaces: vec![FakeInterface {
                id: Uuid::new_v4(),
                host_id: switch,
                mac: Some("00:1a:2b:3c:4d:65".to_string()),
                ..Default::default()
            }],
        };

        let chassis = LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".to_string());
        assert_eq!(
            chassis
                .resolve_host_id(&inventory, Uuid::new_v4(), AdvertisedIdentity::default())
                .await,
            IdentityResolution::Resolved(switch)
        );
    }

    #[tokio::test]
    async fn neighbour_on_a_device_this_site_never_scanned_is_not_found() {
        let inventory = FakeInventory::default();
        let chassis = LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".to_string());

        assert_eq!(
            chassis
                .resolve_host_id(
                    &inventory,
                    Uuid::new_v4(),
                    AdvertisedIdentity {
                        sys_name: Some("some-ap"),
                        ..Default::default()
                    },
                )
                .await,
            IdentityResolution::NotFound
        );
    }

    /// sysName is operator-assigned and often left at a vendor default, so two devices sharing
    /// one is a real configuration. Attaching a physical link to an arbitrary one of them is
    /// worse than reporting the neighbour unresolved.
    #[tokio::test]
    async fn a_sys_name_shared_by_two_hosts_resolves_to_neither() {
        let inventory = FakeInventory {
            hosts: vec![
                FakeHost {
                    id: Uuid::new_v4(),
                    sys_name: Some("switch".to_string()),
                    ..Default::default()
                },
                FakeHost {
                    id: Uuid::new_v4(),
                    sys_name: Some("switch".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let chassis = LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".to_string());
        assert_eq!(
            chassis
                .resolve_host_id(
                    &inventory,
                    Uuid::new_v4(),
                    AdvertisedIdentity {
                        sys_name: Some("switch"),
                        ..Default::default()
                    },
                )
                .await,
            // The MAC tier found nothing and the sysName tier found two. The ladder keeps
            // descending past an ambiguous tier — a later one may still tell them apart — but
            // the verdict it carries out must be the ambiguity, not the miss before it.
            IdentityResolution::Ambiguous
        );
    }

    /// GH #649: Aruba/HP switches advertise the remote port as subtype 7 (locally assigned)
    /// carrying the port's own label — which is exactly that device's ifDescr. Treating the
    /// subtype as unresolvable stopped resolution at the host, and a host-only neighbour renders
    /// no edge, so whole switches were missing from L2 Physical.
    #[tokio::test]
    async fn locally_assigned_port_id_resolves_against_the_remote_ports_own_label() {
        let switch = Uuid::new_v4();
        let port_41 = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![
                FakeInterface {
                    id: port_41,
                    host_id: switch,
                    if_descr: Some("41".to_string()),
                    if_index: 41,
                    ..Default::default()
                },
                FakeInterface {
                    id: Uuid::new_v4(),
                    host_id: switch,
                    if_descr: Some("42".to_string()),
                    if_index: 42,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let port = LldpPortId::LocallyAssigned("41".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::Resolved(port_41)
        );
    }

    /// GH #668: a D-Link DGS-1210-48 sets port-id subtype 5 (`interfaceName`) but sends the bare
    /// port number, while its own interfaces are `Slot0/1..Slot0/48` with ifIndex == port number.
    /// Name-only lookup — which is all subtype 5 used to get — can never match that, so the
    /// neighbour stopped at the host and the switch drew no L2 edge.
    #[tokio::test]
    async fn interface_name_port_id_falls_back_to_the_remote_if_index() {
        let switch = Uuid::new_v4();
        let port_9 = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: port_9,
                host_id: switch,
                if_name: Some("Slot0/9".to_string()),
                if_descr: Some("D-Link DGS-1210-48 Rev.GX/7.20.003 Port 9".to_string()),
                if_index: 9,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::InterfaceName("9".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::Resolved(port_9)
        );
    }

    /// The fallback must not cost subtype 5 its literal reading: a device that means `interfaceName`
    /// still resolves by name, and the name is tried before the index.
    #[tokio::test]
    async fn interface_name_port_id_still_prefers_a_real_name() {
        let switch = Uuid::new_v4();
        let named = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![
                FakeInterface {
                    id: named,
                    host_id: switch,
                    if_name: Some("2".to_string()),
                    if_index: 40,
                    ..Default::default()
                },
                // Would win if the index were consulted first.
                FakeInterface {
                    id: Uuid::new_v4(),
                    host_id: switch,
                    if_name: Some("Gi0/2".to_string()),
                    if_index: 2,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let port = LldpPortId::InterfaceName("2".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::Resolved(named)
        );
    }

    /// GH #668: D-Link NUL-terminates its port identifiers, so `lldpRemPortId` arrives as
    /// `31 00`. The byte is valid UTF-8, so nothing rejected it — it reached `jsonb`, which
    /// cannot store the escape, and would never have matched an interface named "1" anyway.
    #[test]
    fn tlv_text_decoding_strips_nul_padding() {
        assert_eq!(
            LldpPortId::from_snmp(5, b"1\0"),
            Some(LldpPortId::InterfaceName("1".to_string()))
        );
        assert_eq!(
            LldpPortId::from_snmp(7, b"eth1/0/51\0\0"),
            Some(LldpPortId::LocallyAssigned("eth1/0/51".to_string()))
        );
        assert_eq!(
            LldpChassisId::from_snmp(7, b"switch4\0"),
            Some(LldpChassisId::LocallyAssigned("switch4".to_string()))
        );
    }

    /// The other shape of a locally-assigned port id: the remote device's ifIndex, on a switch
    /// whose port labels are something else entirely (HP A-series reports "A1".."A24" as ifDescr
    /// while advertising the index).
    #[tokio::test]
    async fn locally_assigned_port_id_falls_back_to_the_remote_if_index() {
        let switch = Uuid::new_v4();
        let uplink = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: uplink,
                host_id: switch,
                if_descr: Some("A5".to_string()),
                if_index: 197,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::LocallyAssigned("197".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::Resolved(uplink)
        );
    }

    #[tokio::test]
    async fn a_locally_assigned_port_id_matching_nothing_stays_unresolved() {
        let switch = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: Uuid::new_v4(),
                host_id: switch,
                if_descr: Some("1".to_string()),
                if_index: 1,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::LocallyAssigned("WAN PORT".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::NotFound
        );
    }

    /// A port id is only ever resolved against the host it was already attributed to, so an
    /// identifier that happens to collide with another device's port cannot cross the boundary.
    #[tokio::test]
    async fn port_resolution_cannot_reach_an_interface_on_another_host() {
        let switch = Uuid::new_v4();
        let other_switch = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: Uuid::new_v4(),
                host_id: other_switch,
                if_descr: Some("41".to_string()),
                if_index: 41,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::LocallyAssigned("41".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::NotFound
        );
    }

    /// An ifAlias is an operator-typed description, but it is a column the far end's own ifXTable
    /// carries, so it is looked up like every other name-shaped identifier. Declining outright
    /// cost the port on every device advertising subtype 1 — on Westermo WeOS that is the bare
    /// port name, which its ifAlias column holds verbatim while its ifDescr prefixes the media
    /// type ("100-T eth9").
    #[tokio::test]
    async fn an_alias_port_id_resolves_against_the_far_ends_alias_column() {
        let switch = Uuid::new_v4();
        let target = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: target,
                host_id: switch,
                if_descr: Some("100-T eth9".to_string()),
                if_name: Some("eth9".to_string()),
                if_alias: Some("eth9".to_string()),
                if_index: 11,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::InterfaceAlias("eth9".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::Resolved(target)
        );
    }

    /// The media-type prefix is the reason the alias tier exists: a neighbour naming the bare port
    /// matches neither the ifDescr it is embedded in nor an ifIndex, and before the alias column
    /// was consulted the link degraded to device level.
    #[tokio::test]
    async fn a_bare_port_name_resolves_when_only_the_alias_carries_it() {
        let switch = Uuid::new_v4();
        let target = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: target,
                host_id: switch,
                if_descr: Some("1000-LX eth1".to_string()),
                if_alias: Some("eth1".to_string()),
                if_index: 19,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::InterfaceName("eth1".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::Resolved(target)
        );
    }

    /// The Westermo port-11 neighbour, and the one path in this ladder with no second chance.
    ///
    /// Its chassis id is subtype 7 (`localOther`) `"C230408"` with no sysName and no port
    /// description — nothing that names a MAC, an address or an interface. The only server-side
    /// record of that identity is `hosts.chassis_id`, written from the far end's own
    /// `lldpLocChassisId` by way of `LldpChassisId::identifier()`. Both halves are exercised
    /// deliberately rather than assumed: if the two ever canonicalise differently, every neighbour
    /// of that device is unfindable and the counters cannot tell it apart from a device nobody
    /// scanned.
    #[tokio::test]
    async fn a_subtype_7_chassis_id_matches_the_same_value_recorded_from_the_far_ends_own_identity()
    {
        let westermo = Uuid::new_v4();
        // Exactly what the daemon stores: from_snmp on the far end's own lldpLocChassisId, then
        // `identifier()`. Anything else here would test a hand-written string, not the round trip.
        let recorded = LldpChassisId::from_snmp(7, b"C230408")
            .expect("subtype 7 is a chassis id this parses")
            .identifier();

        let inventory = FakeInventory {
            hosts: vec![FakeHost {
                id: westermo,
                chassis_id: Some(recorded),
                ..Default::default()
            }],
            ..Default::default()
        };

        // And exactly what the neighbour advertises, through the same parser.
        let advertised =
            LldpChassisId::from_snmp(7, b"C230408").expect("the neighbour sends the same bytes");
        assert_eq!(
            advertised
                .resolve_host_id(&inventory, Uuid::new_v4(), AdvertisedIdentity::default())
                .await,
            IdentityResolution::Resolved(westermo)
        );
    }

    /// The trailing-NUL form, because a device that pads its chassis id must still match the same
    /// device recorded from an unpadded one. `decode_tlv_text` strips them on both paths, and if
    /// it ever stopped doing so on one, this is the neighbour that would silently stop resolving.
    #[tokio::test]
    async fn a_nul_padded_subtype_7_chassis_id_reaches_the_same_device() {
        let westermo = Uuid::new_v4();
        let inventory = FakeInventory {
            hosts: vec![FakeHost {
                id: westermo,
                chassis_id: Some(
                    LldpChassisId::from_snmp(7, b"C230408")
                        .expect("parses")
                        .identifier(),
                ),
                ..Default::default()
            }],
            ..Default::default()
        };

        let advertised = LldpChassisId::from_snmp(7, b"C230408\0").expect("parses");
        assert_eq!(
            advertised
                .resolve_host_id(&inventory, Uuid::new_v4(), AdvertisedIdentity::default())
                .await,
            IdentityResolution::Resolved(westermo)
        );
    }

    /// The customer's Westermo shape: ten physical ports with unique addresses alongside six
    /// `propVirtual` VLAN interfaces sharing the chassis base MAC. A virtual row is not the far end
    /// of a cable, so it must not make a physical port's address look ambiguous — counting them
    /// turned every such lookup `Ambiguous` and cost the port on a device no port would have
    /// contested.
    #[tokio::test]
    async fn virtual_interfaces_sharing_a_mac_do_not_contest_a_physical_ports_lookup() {
        use crate::server::interfaces::r#impl::base::if_type;

        let switch = Uuid::new_v4();
        let target = Uuid::new_v4();
        let shared = "00:11:b4:8c:02:e0";
        let mut interfaces = vec![FakeInterface {
            id: target,
            host_id: switch,
            if_descr: Some("100-T eth10".to_string()),
            if_index: 10,
            mac: Some(shared.to_string()),
            if_type: if_type::ETHERNET_CSMA_CD,
            ..Default::default()
        }];
        interfaces.extend((22..=27).map(|n| FakeInterface {
            id: Uuid::new_v4(),
            host_id: switch,
            if_descr: Some(format!("vlan{n}")),
            if_index: n,
            mac: Some(shared.to_string()),
            if_type: if_type::PROP_VIRTUAL,
            ..Default::default()
        }));

        let inventory = FakeInventory {
            interfaces,
            ..Default::default()
        };

        let port = LldpPortId::MacAddress(shared.to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::Resolved(target)
        );
    }

    /// Three ports of one switch, all reporting the chassis base MAC as `ifPhysAddress` — the
    /// D-Link/TP-Link/Omada shape from GH #668.
    fn shared_chassis_mac_switch(host_id: Uuid, mac: &str) -> Vec<FakeInterface> {
        (1..=3)
            .map(|n| FakeInterface {
                id: Uuid::new_v4(),
                host_id,
                if_descr: Some(format!("Slot0/{n}")),
                if_index: n,
                mac: Some(mac.to_string()),
                ..Default::default()
            })
            .collect()
    }

    /// GH #668: a switch that repeats one MAC across every port makes that MAC name the device,
    /// not a port. Before the guard this resolved to whichever row the database returned first,
    /// drawing a port-precise link to an arbitrary port — a wrong answer that reads as an
    /// authoritative one, which is worse than the device-level link it now degrades to.
    #[tokio::test]
    async fn a_port_mac_the_device_repeats_on_every_port_identifies_no_port() {
        let switch = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: shared_chassis_mac_switch(switch, "00:ad:24:af:4e:00"),
            ..Default::default()
        };

        let port = LldpPortId::MacAddress("00:ad:24:af:4e:00".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::Ambiguous
        );
    }

    /// The guard must not cost the vendors that do give each port its own address — Westermo
    /// advertises exactly this, a distinct per-port MAC as the port identifier.
    #[tokio::test]
    async fn a_port_mac_unique_within_the_device_still_resolves() {
        let switch = Uuid::new_v4();
        let target = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![
                FakeInterface {
                    id: target,
                    host_id: switch,
                    if_index: 1,
                    mac: Some("00:11:b4:8c:02:ea".to_string()),
                    ..Default::default()
                },
                FakeInterface {
                    id: Uuid::new_v4(),
                    host_id: switch,
                    if_index: 2,
                    mac: Some("00:11:b4:8c:02:eb".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let port = LldpPortId::MacAddress("00:11:b4:8c:02:ea".to_string());
        assert_eq!(
            port.resolve_interface_id(&inventory, switch).await,
            IdentityResolution::Resolved(target)
        );
    }

    /// The same repetition that makes a MAC useless for naming a *port* leaves it perfectly good
    /// for naming the *device*: many ports, still one host. Guarding the port lookup must not take
    /// the host tier down with it, or every neighbour of an affected switch stops resolving at all.
    #[tokio::test]
    async fn a_chassis_mac_repeated_on_every_port_still_identifies_the_device() {
        let switch = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: shared_chassis_mac_switch(switch, "00:ad:24:af:4e:00"),
            ..Default::default()
        };

        let chassis = LldpChassisId::MacAddress("00:ad:24:af:4e:00".to_string());
        assert_eq!(
            chassis
                .resolve_host_id(&inventory, Uuid::new_v4(), AdvertisedIdentity::default())
                .await,
            IdentityResolution::Resolved(switch)
        );
    }

    /// Two devices answering to one MAC is a duplicate this cannot choose between — unlike the
    /// case above, where the repetition is within a single device.
    /// Ambiguity at one tier must not stop the ladder — a later tier may still tell the
    /// candidates apart, and here it does: two hosts share a chassis id, and the `sysName` the
    /// neighbour advertised belongs to only one of them.
    ///
    /// The failure this guards is returning `Ambiguous` the moment a tier sees two rows, which
    /// would cost a resolution that the very next lookup was about to make.
    #[tokio::test]
    async fn an_ambiguous_tier_does_not_stop_a_later_one_from_resolving() {
        let wanted = Uuid::new_v4();
        let inventory = FakeInventory {
            hosts: vec![
                FakeHost {
                    id: wanted,
                    chassis_id: Some("shared-chassis".to_string()),
                    sys_name: Some("switch-a".to_string()),
                    ..Default::default()
                },
                FakeHost {
                    id: Uuid::new_v4(),
                    chassis_id: Some("shared-chassis".to_string()),
                    sys_name: Some("switch-b".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let chassis = LldpChassisId::LocallyAssigned("shared-chassis".to_string());
        assert_eq!(
            chassis
                .resolve_host_id(
                    &inventory,
                    Uuid::new_v4(),
                    AdvertisedIdentity {
                        sys_name: Some("switch-a"),
                        ..Default::default()
                    },
                )
                .await,
            IdentityResolution::Resolved(wanted)
        );
    }

    #[tokio::test]
    async fn a_mac_carried_by_two_devices_identifies_neither() {
        let mac = "00:ad:24:af:4e:00";
        let inventory = FakeInventory {
            interfaces: vec![
                FakeInterface {
                    id: Uuid::new_v4(),
                    host_id: Uuid::new_v4(),
                    mac: Some(mac.to_string()),
                    ..Default::default()
                },
                FakeInterface {
                    id: Uuid::new_v4(),
                    host_id: Uuid::new_v4(),
                    mac: Some(mac.to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let chassis = LldpChassisId::MacAddress(mac.to_string());
        assert_eq!(
            chassis
                .resolve_host_id(&inventory, Uuid::new_v4(), AdvertisedIdentity::default())
                .await,
            // Ambiguous rather than NotFound: the MAC names two devices we have discovered, not
            // zero. An operator told "not found" goes looking for a device that is already
            // scanned — twice, which is the thing to fix.
            IdentityResolution::Ambiguous
        );
    }

    /// The subtype-independent fallbacks are *ordered*, and the order carries meaning: a chassis
    /// id is an identifier the far end asserted about itself, while `sysName` is an
    /// operator-assigned label two devices can share and a vendor can leave at a default. Where
    /// the two name different hosts the chassis id has to win.
    ///
    /// Pinned because tiers get inserted between them — the management-address tier is one — and
    /// an insertion that reordered these would silently start attaching links to whichever device
    /// happened to be named "switch".
    #[tokio::test]
    async fn the_chassis_id_fallback_outranks_the_sys_name_fallback() {
        let by_chassis = Uuid::new_v4();
        let inventory = FakeInventory {
            hosts: vec![
                FakeHost {
                    id: by_chassis,
                    chassis_id: Some("00:1a:2b:3c:4d:63".to_string()),
                    ..Default::default()
                },
                FakeHost {
                    id: Uuid::new_v4(),
                    sys_name: Some("core-switch".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let chassis = LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".to_string());
        assert_eq!(
            chassis
                .resolve_host_id(
                    &inventory,
                    Uuid::new_v4(),
                    AdvertisedIdentity {
                        sys_name: Some("core-switch"),
                        ..Default::default()
                    },
                )
                .await,
            IdentityResolution::Resolved(by_chassis)
        );
    }

    /// `NoStrategy` and `NotFound` are the two halves of "did not resolve" and mean opposite
    /// things to whoever reads the scan summary: nothing to look up, versus looked and the device
    /// is not here. A subtype with no strategy and nothing else to try must produce the former.
    ///
    /// Pinned because every new tier adds a site that sets `strategy_ran`, and one that sets it
    /// unconditionally would turn every one of these into a `NotFound` the operator is invited to
    /// go and scan.
    #[tokio::test]
    async fn a_subtype_with_no_strategy_and_nothing_else_to_try_reports_no_strategy() {
        let inventory = FakeInventory::default();

        // Empty, so even the raw-identifier `hosts.chassis_id` fallback declines to run.
        let chassis = LldpChassisId::PortComponent(String::new());
        assert_eq!(
            chassis
                .resolve_host_id(&inventory, Uuid::new_v4(), AdvertisedIdentity::default())
                .await,
            IdentityResolution::NoStrategy
        );
    }

    /// GH #668: a switch that answers SNMP but serves no ifTable, no addresses and no LLDP local
    /// identity holds no interface row to carry the chassis MAC its neighbours advertise and no
    /// `chassis_id` of its own — every tier above the address is structurally dead for it. The
    /// address it publishes in `lldpRemManAddr` is the one identifier that survives, and this
    /// site already holds it.
    ///
    /// The host deliberately has no `sys_name`, so only the address tier can produce this answer.
    #[tokio::test]
    async fn a_far_end_with_no_chassis_record_resolves_on_the_address_it_advertises() {
        let mute_switch = Uuid::new_v4();
        let advertised_address: IpAddr = "192.168.7.248".parse().unwrap();
        let inventory = FakeInventory {
            hosts: vec![FakeHost {
                id: mute_switch,
                addresses: vec![advertised_address],
                ..Default::default()
            }],
            ..Default::default()
        };

        let chassis = LldpChassisId::MacAddress("00:ad:24:89:cc:f0".to_string());
        assert_eq!(
            chassis
                .resolve_host_id(
                    &inventory,
                    Uuid::new_v4(),
                    AdvertisedIdentity {
                        sys_name: Some("Switch1"),
                        address: Some(advertised_address),
                    },
                )
                .await,
            IdentityResolution::Resolved(mute_switch)
        );
    }

    /// An address is the far end's own statement of where it lives; `sysName` is a label an
    /// operator assigned and a vendor may have defaulted. Where they name different hosts, the
    /// address has to win, or a fleet with two devices called `switch` attaches links by name in
    /// preference to by address.
    #[tokio::test]
    async fn the_address_tier_outranks_the_sys_name_tier() {
        let by_address = Uuid::new_v4();
        let advertised_address: IpAddr = "10.20.30.11".parse().unwrap();
        let inventory = FakeInventory {
            hosts: vec![
                FakeHost {
                    id: by_address,
                    addresses: vec![advertised_address],
                    ..Default::default()
                },
                FakeHost {
                    id: Uuid::new_v4(),
                    sys_name: Some("switch".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let chassis = LldpChassisId::MacAddress("00:ad:24:89:cc:f0".to_string());
        assert_eq!(
            chassis
                .resolve_host_id(
                    &inventory,
                    Uuid::new_v4(),
                    AdvertisedIdentity {
                        sys_name: Some("switch"),
                        address: Some(advertised_address),
                    },
                )
                .await,
            IdentityResolution::Resolved(by_address)
        );
    }

    /// `parse_lldp_mgmt_addr` accepts any four or sixteen bytes, so half-initialised firmware
    /// publishes `0.0.0.0` and a link-local address is per-segment and repeats across devices.
    /// Neither names a device here.
    ///
    /// The verdict matters as much as the miss: an unusable address must leave the row reading
    /// `NoStrategy` — nothing to look up — rather than `NotFound`, which tells an operator a
    /// device is missing from their inventory and invites them to go and scan for it.
    #[tokio::test]
    async fn an_address_that_can_name_no_device_is_not_looked_up() {
        let inventory = FakeInventory::default();
        let chassis = LldpChassisId::PortComponent(String::new());

        for unusable in ["0.0.0.0", "169.254.3.4", "127.0.0.1", "fe80::1"] {
            assert_eq!(
                chassis
                    .resolve_host_id(
                        &inventory,
                        Uuid::new_v4(),
                        AdvertisedIdentity {
                            sys_name: None,
                            address: Some(unusable.parse().unwrap()),
                        },
                    )
                    .await,
                IdentityResolution::NoStrategy,
                "{unusable} should not be looked up at all"
            );
        }
    }

    /// CDP carries no chassis id, so it cannot run the ladder above — but `cdpCacheAddress` is a
    /// management address and the device id is a `sysName`, which are exactly the tiers the ladder
    /// ends with. Sharing them is what stops a neighbour from being unresolvable purely for having
    /// arrived over the other protocol.
    #[tokio::test]
    async fn a_cdp_neighbour_resolves_from_the_address_it_cached() {
        let router = Uuid::new_v4();
        let cached_address: IpAddr = "10.20.30.1".parse().unwrap();
        let inventory = FakeInventory {
            hosts: vec![FakeHost {
                id: router,
                addresses: vec![cached_address],
                ..Default::default()
            }],
            ..Default::default()
        };

        assert_eq!(
            AdvertisedIdentity {
                sys_name: Some("core-rtr"),
                address: Some(cached_address),
            }
            .resolve_host_id(&inventory, Uuid::new_v4())
            .await,
            IdentityResolution::Resolved(router)
        );
    }
}
