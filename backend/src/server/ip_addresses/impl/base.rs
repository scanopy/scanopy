use crate::server::shared::attribution::{self as attribution, AttributeValue, Attributed};
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::position::Positioned;
use crate::server::subnets::r#impl::base::Subnet;
use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

pub const ALL_IP_ADDRESSES_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

/// Whether `mac` is the all-zero placeholder: what firmware and OS APIs report for an interface
/// with no hardware address (Net-SNMP's `lo`, several platforms' NIC enumeration).
///
/// It is not an address. Every loopback in an estate wears it, and its OUI `000000` happens to be
/// registered, so a check that only asks "is this a real vendor block" grades it as strong
/// identity. One definition so the daemon's readers and the server's identity and storage rules
/// cannot disagree about it.
pub fn is_unset_mac(mac: &MacAddress) -> bool {
    mac.bytes() == [0; 6]
}

/// A MAC address, and the evidence for it.
///
/// Four sites produce one and they are not equally good: our own ARP request answered by the
/// address itself, a device reporting its own `ifPhysAddress`, a *router's* ARP cache talking about
/// somebody else, and a controller reporting a device it manages. §6's minting rule turns on that
/// distinction — conjuring a host from a MAC nothing has ever contacted, on a third party's say-so,
/// is how ghost hosts get created.
///
/// Shared by `IPAddressBase` and `InterfaceBase`: identical keys, identical schema, so the two
/// cannot drift on pattern or example the way the copy-pasted declarations had begun to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MacEvidenceValue(pub MacAddress);

impl std::fmt::Display for MacEvidenceValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl AttributeValue for MacEvidenceValue {
    const VALUE_KEY: &'static str = "mac_address";
    const SOURCE_KEY: &'static str = "mac_address_source";
    const SCHEMA_NAME: &'static str = "MacEvidence";
    /// A NIC does not change its address, so a re-read from the same source never moves it. A
    /// stronger source may still correct one a weaker source got wrong, which is the point: an ARP
    /// reply we solicited should be able to fix a MAC a forwarding table reported.
    const REFRESHABLE: bool = false;

    /// The all-zero placeholder is an absent MAC. Reads back as `None`, is never written, and
    /// yields to any real MAC whatever its rung. That last rule clears one an older daemon sent.
    fn is_blank(&self) -> bool {
        is_unset_mac(&self.0)
    }
}

impl utoipa::PartialSchema for MacEvidenceValue {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
        use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
        utoipa::openapi::RefOr::T(utoipa::openapi::Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::new(Type::String))
                .description(Some("MAC address, as discovered."))
                .pattern(Some(r"^(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$"))
                .examples([serde_json::json!("a4:bb:6d:12:34:56")])
                .build(),
        ))
    }
}

/// A MAC address together with what produced it.
pub type MacEvidence = Attributed<MacEvidenceValue>;

/// The address itself, for the matching and counting code that cares which MAC it is rather than
/// how we came to know it.
pub fn mac_of(evidence: &Option<MacEvidence>) -> Option<MacAddress> {
    evidence.as_ref().map(|e| e.value().0)
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema, Validate)]
pub struct IPAddressBase {
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// The host this entity belongs to.
    pub host_id: Uuid,
    /// The subnet this entity belongs to.
    pub subnet_id: Uuid,
    /// IPv4 or IPv6 address.
    #[schema(value_type = String, example = "192.168.1.10")]
    pub ip_address: IpAddr,
    /// MAC address discovered from ARP, SNMP or a container runtime, with the evidence for it.
    #[serde(flatten, deserialize_with = "attribution::optional")]
    #[schema(value_type = MacEvidence)]
    pub mac_address: Option<MacEvidence>,
    /// Human-facing name for this IP address.
    #[schema(required)]
    pub name: Option<String>,
    /// Position of this IP address in the host's IP address list (for ordering)
    #[serde(default)]
    pub position: i32,
}

impl Default for IPAddressBase {
    fn default() -> Self {
        Self {
            network_id: Uuid::nil(),
            host_id: Uuid::nil(),
            subnet_id: Uuid::nil(),
            ip_address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            mac_address: None,
            name: None,
            position: 0,
        }
    }
}

impl IPAddressBase {
    /// Merge a newer reading of this address's MAC into the one on file, by rung. Returns whether
    /// anything changed.
    ///
    /// A reading with no MAC says nothing about the address and leaves the stored one alone.
    pub fn apply_mac_address(&mut self, incoming: Option<MacEvidence>) -> bool {
        incoming.is_some_and(|mac| Attributed::apply(&mut self.mac_address, mac))
    }

    /// Create a conceptual IP address for a subnet.
    /// `host_id` can be `Uuid::nil()` as a placeholder - server will set the correct one.
    pub fn new_conceptual(host_id: Uuid, subnet: &Subnet) -> Self {
        let ip_address = IpAddr::V4(Ipv4Addr::new(203, 0, 113, rand::rng().random_range(1..255)));

        Self {
            network_id: subnet.base.network_id,
            host_id,
            subnet_id: subnet.id,
            ip_address,
            mac_address: None,
            name: Some(subnet.base.name.clone()),
            position: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::ip_address)]
pub struct IPAddress {
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
    #[serde(flatten)]
    #[validate(nested)]
    pub base: IPAddressBase,
}

impl Hash for IPAddress {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.ip_address.hash(state);
        self.base.subnet_id.hash(state);
    }
}

/// Two IP-address records are equal when they represent the same logical address:
/// - Same IP value on the same subnet (primary network identity), OR
/// - Same database ID (both non-nil)
///
/// MAC address is intentionally excluded from equality. VLAN sub-interfaces, bridge
/// members, and bond interfaces legitimately share a parent's MAC while exposing distinct
/// IP addresses on different subnets. MAC-based matching was previously here (added
/// in 3f69301b for Docker DHCP dedup) but caused IP addresses on VLAN sub-interfaces to collapse into one.
///
/// MAC-based matching now lives in explicit call-site logic where the context allows
/// distinguishing shared MACs (VLANs) from unique MACs (Docker/DHCP):
/// - IP address upsert: `create_with_children()` in `hosts/service.rs`
/// - Host dedup: `find_matching_host_by_ip_addresses()` in `hosts/service.rs`
/// - Host merge: `merge_hosts()` in `hosts/service.rs`
impl PartialEq for IPAddress {
    fn eq(&self, other: &Self) -> bool {
        (self.base.ip_address == other.base.ip_address
            && self.base.subnet_id == other.base.subnet_id)
            || (self.id == other.id && self.id != Uuid::nil() && other.id != Uuid::nil())
    }
}

impl Display for IPAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IPAddress {}: {} on subnet {}",
            self.id, self.base.ip_address, self.base.subnet_id
        )
    }
}

impl IPAddress {
    pub fn new(base: IPAddressBase) -> Self {
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
            base,
        }
    }
}

impl ChangeTriggersTopologyStaleness<IPAddress> for IPAddress {
    fn triggers_staleness(&self, other: Option<IPAddress>) -> bool {
        if let Some(other_ip_address) = other {
            self.base.ip_address != other_ip_address.base.ip_address
                || self.base.subnet_id != other_ip_address.base.subnet_id
                || self.base.host_id != other_ip_address.base.host_id
        } else {
            true
        }
    }
}

impl Positioned for IPAddress {
    fn position(&self) -> i32 {
        self.base.position
    }

    fn set_position(&mut self, position: i32) {
        self.base.position = position;
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn entity_name() -> &'static str {
        "ip_address"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::attribution::AttributeSource;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn make_ip_address(
        ip: IpAddr,
        subnet_id: Uuid,
        mac: Option<MacAddress>,
        id: Uuid,
    ) -> IPAddress {
        IPAddress {
            id,
            base: IPAddressBase {
                ip_address: ip,
                subnet_id,
                mac_address: mac
                    .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::ArpReply)),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn hash_of(iface: &IPAddress) -> u64 {
        let mut hasher = DefaultHasher::new();
        iface.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn same_mac_different_ip_subnet_not_equal() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let a = make_ip_address("10.0.0.1".parse().unwrap(), s1, Some(mac), Uuid::nil());
        let b = make_ip_address("20.0.0.1".parse().unwrap(), s2, Some(mac), Uuid::nil());
        assert_ne!(a, b, "VLAN sub-interfaces with same MAC must not be equal");
    }

    #[test]
    fn same_ip_subnet_equal_regardless_of_mac() {
        let subnet = Uuid::new_v4();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let mac_a = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let mac_b = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02]);
        let a = make_ip_address(ip, subnet, Some(mac_a), Uuid::nil());
        let b = make_ip_address(ip, subnet, Some(mac_b), Uuid::nil());
        assert_eq!(a, b, "Same IP+subnet should be equal regardless of MAC");
    }

    #[test]
    fn hash_consistent_with_eq() {
        let subnet = Uuid::new_v4();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let mac_a = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let mac_b = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02]);
        let a = make_ip_address(ip, subnet, Some(mac_a), Uuid::nil());
        let b = make_ip_address(ip, subnet, Some(mac_b), Uuid::nil());
        assert_eq!(a, b);
        assert_eq!(
            hash_of(&a),
            hash_of(&b),
            "Equal IP addresses must have equal hashes"
        );
    }

    #[test]
    fn nil_ids_not_equal_when_different_ip_subnet() {
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let a = make_ip_address("10.0.0.1".parse().unwrap(), s1, None, Uuid::nil());
        let b = make_ip_address("20.0.0.1".parse().unwrap(), s2, None, Uuid::nil());
        assert_ne!(a, b, "Nil IDs with different IP/subnet must not be equal");
    }

    #[test]
    fn same_non_nil_id_equal() {
        let id = Uuid::new_v4();
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let a = make_ip_address("10.0.0.1".parse().unwrap(), s1, None, id);
        let b = make_ip_address("20.0.0.1".parse().unwrap(), s2, None, id);
        assert_eq!(a, b, "Same non-nil ID should be equal");
    }
}
