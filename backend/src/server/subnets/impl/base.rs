use std::fmt::Display;
use std::net::Ipv4Addr;

use crate::server::shared::attribution::{
    self as attribution, AttributeMethod, AttributeSource, AttributeValue, Attributed,
};
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::api::deserialize_empty_string_as_none;
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::inference::overlaps;
use crate::server::subnets::r#impl::types::SubnetType;
use chrono::{DateTime, Utc};
use cidr::{IpCidr, Ipv4Cidr};
use pnet::ipnetwork::IpNetwork;
use serde::de::Error as DeError;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{ip_addresses::r#impl::base::IPAddress, services::r#impl::base::Service};

fn deserialize_cidr<'de, D>(deserializer: D) -> Result<IpCidr, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<IpCidr>().map_err(|e| {
        let msg = e.to_string();
        if msg.contains("host part of address was not zero") {
            DeError::custom(format!(
                "Invalid CIDR '{}': address doesn't align with the subnet mask. Use a network address (e.g., for /24, the last octet should be 0).",
                s
            ))
        } else {
            DeError::custom(format!("Invalid CIDR '{}': {}", s, msg))
        }
    })
}

/// A subnet's range, keeping the parse error that tells an operator their address does not align
/// with its mask.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct SubnetCidrValue(pub IpCidr);

impl<'de> Deserialize<'de> for SubnetCidrValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_cidr(deserializer).map(Self)
    }
}

impl std::fmt::Display for SubnetCidrValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl utoipa::PartialSchema for SubnetCidrValue {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
        use utoipa::openapi::schema::{ObjectBuilder, SchemaType, Type};
        utoipa::openapi::RefOr::T(utoipa::openapi::Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::new(Type::String))
                .description(Some("Subnet in CIDR notation, IPv4 or IPv6."))
                .pattern(Some(r"^[0-9A-Fa-f.:]+/\d{1,3}$"))
                .examples([serde_json::json!("192.168.1.0/24")])
                .build(),
        ))
    }
}

impl AttributeValue for SubnetCidrValue {
    const VALUE_KEY: &'static str = "cidr";
    const SOURCE_KEY: &'static str = "cidr_source";
    const SCHEMA_NAME: &'static str = "SubnetCidr";
    /// A subnet without a range is not a subnet.
    const VALUE_REQUIRED: bool = true;
    /// A range does move: a person correcting one is the whole point of the confirm action, and a
    /// device re-reporting its own netmask should refresh what it said last time.
    const REFRESHABLE: bool = true;
}

/// A subnet's range together with what produced it.
pub type SubnetCidr = Attributed<SubnetCidrValue>;

/// Read-only access to the range itself, so the geometry — `contains`, `iter`, `first_address` —
/// reads the same as it did when `cidr` was a bare `IpCidr`.
///
/// Only reads: `Attributed`'s fields are private and the applier is the only writer, so borrowing
/// the range cannot be turned into assigning one without its source. This is the one carrier that
/// earns it, because a CIDR is a value other code does arithmetic on rather than merely displays.
impl std::ops::Deref for SubnetCidr {
    type Target = IpCidr;

    fn deref(&self) -> &Self::Target {
        &self.value().0
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct SubnetBase {
    /// The subnet's range and how far it can be trusted.
    ///
    /// One field rather than two, which is what keeps the pair from drifting: a range and the
    /// confidence in it are one fact, and assigning `cidr` alone is how a guess ends up
    /// indistinguishable from a reading. It shipped as two, and three separate places had already
    /// grown their own rules for writing the source half behind the applier's back.
    ///
    /// Serialises as the two flat keys `cidr` and `cidr_source`, so the wire shape is unchanged.
    #[serde(flatten, deserialize_with = "attribution::required")]
    #[schema(value_type = SubnetCidr)]
    pub cidr: SubnetCidr,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// Human-facing name for this subnet.
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    /// Free-text notes about the subnet.
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    #[validate(length(min = 0, max = 500))]
    pub description: Option<String>,
    /// What kind of subnet this is — physical, virtual, container bridge, and so on.
    pub subnet_type: SubnetType,
    /// The container runtime service that owns this bridge network.
    ///
    /// Load-bearing for dedup: the same CIDR on two different Docker daemons is two distinct
    /// subnets, so bridge rows only merge when this matches as well as the CIDR and network.
    /// A foreign key rather than a field inside a JSONB blob because a stale value here is
    /// precisely what made a scan add a duplicate bridge row every time (GH #650) — now it
    /// cannot be written at all.
    #[serde(default)]
    #[schema(required)]
    pub virtualization_service_id: Option<Uuid>,
    #[serde(default)]
    #[schema(required)]
    /// Will be automatically set to Manual for creation through API
    pub source: EntitySource,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

impl Default for SubnetBase {
    fn default() -> Self {
        Self {
            cidr: SubnetCidr::new(
                SubnetCidrValue(IpCidr::V4(
                    Ipv4Cidr::new(Ipv4Addr::new(192, 168, 4, 0), 24).unwrap(),
                )),
                AttributeSource::Unspecified,
            ),
            name: "New Subnet".to_string(),
            network_id: Uuid::new_v4(),
            description: None,
            subnet_type: SubnetType::Unknown,
            virtualization_service_id: None,
            source: EntitySource::Manual,
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::subnet)]
pub struct Subnet {
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
    pub base: SubnetBase,
}

impl Subnet {
    /// Whether this subnet is a container-runtime bridge network (Docker or Podman).
    pub fn is_container_bridge_subnet(&self) -> bool {
        self.base.subnet_type.is_container_bridge()
    }

    pub fn is_vpn_subnet(&self) -> bool {
        self.base.subnet_type == SubnetType::VpnTunnel
    }

    /// Whether this fresh observation should correct `existing`'s container-bridge
    /// classification.
    ///
    /// Bridge types are produced only by `ContainerRuntime::subnet_from_network`,
    /// which always stamps `virtualization` to scope the bridge to its owning
    /// runtime service. A bridge row without one therefore predates that rule and
    /// can only have come from the interface-name heuristic that used to type an
    /// access point's `br-`-prefixed guest bridge as Docker (#663) — so a current
    /// non-bridge observation of the same CIDR supersedes it.
    ///
    /// Both rows must be `Discovery`. A `Manual` subnet type is a user's explicit
    /// assertion, not a guess to be repaired, and discovery must never silently
    /// overwrite it — the CIDR dedup happily matches a Manual row against an
    /// incoming discovered one, so this is the only thing keeping them apart.
    pub fn corrects_container_bridge_guess(&self, existing: &Subnet) -> bool {
        existing.base.source == EntitySource::Discovery
            && self.base.source == EntitySource::Discovery
            && existing.base.subnet_type.is_container_bridge()
            && existing.base.virtualization_service_id.is_none()
            && !self.base.subnet_type.is_container_bridge()
    }

    pub fn from_discovery(
        interface_name: String,
        ip_network: &IpNetwork,
        network_id: Uuid,
    ) -> Option<Self> {
        let mut subnet_type = SubnetType::from_interface_name(&interface_name);

        match ip_network {
            IpNetwork::V6(_) => None,
            IpNetwork::V4(ipv4_network) => {
                // Non-loopback CIDRs on loopback ip_addresses (e.g. 10.99.0.0/24 aliased
                // on lo0) are real networks, not loopback
                if subnet_type.is_loopback() && ipv4_network.ip().octets()[0] != 127 {
                    subnet_type = SubnetType::Unknown;
                }

                let (network_addr, prefix_len) = match (&subnet_type, ipv4_network.prefix()) {
                    // VPN tunnels with /32 -> expand to /24
                    (SubnetType::VpnTunnel, 32) => {
                        let ip_octets = ipv4_network.ip().octets();
                        let network_addr =
                            std::net::Ipv4Addr::new(ip_octets[0], ip_octets[1], ip_octets[2], 0);
                        (network_addr, 24)
                    }
                    // Skip other /32 single IPs
                    (_, 32) => return None,
                    // Normal case - use the network's actual network address and prefix
                    _ => (ipv4_network.network(), ipv4_network.prefix()),
                };

                let cidr = IpCidr::V4(Ipv4Cidr::new(network_addr, prefix_len).ok()?);

                Some(Subnet::new(SubnetBase {
                    // Straight off a daemon's own interface: it is describing the host it runs on,
                    // which is the strongest reading there is for a range.
                    cidr: SubnetCidr::new(SubnetCidrValue(cidr), AttributeSource::DaemonSelfReport),
                    network_id,
                    description: None,
                    tags: Vec::new(),
                    name: cidr.to_string(),
                    subnet_type,
                    virtualization_service_id: None,
                    source: EntitySource::Discovery,
                }))
            }
        }
    }

    pub fn has_interface_with_service(
        &self,
        host_interfaces: &[&IPAddress],
        service: &Service,
    ) -> bool {
        service.base.bindings.iter().any(|binding| {
            host_interfaces.iter().any(|ip_address| {
                let interface_match = match binding.ip_address_id() {
                    Some(id) => ip_address.id == id,
                    None => true, // Listens on all ip_addresses
                };

                interface_match && ip_address.base.subnet_id == self.id
            })
        })
    }

    pub fn is_organizational_subnet(&self) -> bool {
        let organizational_cidr = IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 0).unwrap());
        *self.base.cidr == organizational_cidr
    }

    /// Whether this subnet belongs to the inventory the user curates, and so
    /// appears in the management lists (Subnets, Networks and Daemon tabs) and
    /// in the dashboard's subnet count.
    ///
    /// Provenance, not category. The rows Scanopy fabricates for itself — the
    /// per-network `0.0.0.0/0` Internet and Remote supernets (`EntitySource::System`,
    /// `seed_data::create_wan_subnet` / `create_remote_subnet`) and the loopback
    /// row seeded per daemon host (`EntitySource::Discovery`) — are fixtures nobody
    /// curates, and omitting them is what
    /// [`SubnetType::is_synthetic_category`] was added for.
    ///
    /// But a `Manual` subnet is one the user deliberately created and must always
    /// be able to edit or delete. Keying purely on the category swallowed those
    /// too: in GH #677 assigning `Remote` to a new subnet made it vanish from every
    /// view while still blocking its own CIDR from being recreated, leaving no way
    /// to reach it from the UI at all.
    ///
    /// `StorableFilter::<Subnet>::user_managed` is the SQL form of this; the two
    /// must be changed together.
    /// Assign the range if `source` is at least as authoritative as what is stored.
    /// Returns whether anything changed.
    ///
    /// A thin wrapper on the shared applier now that the pair is one field: the ordering lives in
    /// [`AttributeSource::rank`], which every provenanced value uses, so a caller still only has to
    /// say where its range came from.
    ///
    /// Equal rank from the same source wins, which is what makes a re-scan idempotent in the useful
    /// direction: a device that re-reports its own `ipAdEntNetMask` refreshes that observation,
    /// while an inference never displaces a reading and nothing displaces a range a person
    /// confirmed. Equal rank from a *different* source does not win — the rungs the old `Observed`
    /// collapsed into are now distinct, so a daemon's own interface and a device's `ipAdEntNetMask`
    /// no longer flap the value depending on which finished first.
    pub fn apply_cidr(&mut self, cidr: IpCidr, source: AttributeSource) -> bool {
        self.base
            .cidr
            .apply_in_place(SubnetCidr::new(SubnetCidrValue(cidr), source))
    }

    /// Whether this reading should correct `existing` rather than be inserted beside it.
    ///
    /// The case it exists for: a range Scanopy inferred from LLDP neighbour addresses, and a real
    /// netmask later read for the same segment — from a daemon's own interface or a device's
    /// `ipAdEntNetMask`. Those two rarely agree exactly, and `Subnet::eq` is CIDR equality, so
    /// without this the reading inserts a second, overlapping subnet and the guess stays badged
    /// forever.
    ///
    /// Nesting is the containment test because for CIDRs it *is* containment — two ranges are
    /// equal, one holds the other, or they are disjoint; partial overlap cannot occur.
    ///
    /// The strict rank comparison is load-bearing twice over. It stops one inference correcting
    /// another, and it is what makes the caller's re-placement of displaced addresses terminate:
    /// re-placing creates inferred subnets, so the `create` that runs inside one can never correct
    /// anything and cannot recurse. That still holds with the shared ladder because every
    /// inferred-tier source ranks the same, so one inference never outranks another.
    ///
    /// The organizational rows are excluded because both are `0.0.0.0/0` and would otherwise
    /// "contain" every inferred range on the network.
    pub fn corrects_inferred_range(&self, existing: &Subnet) -> bool {
        existing.base.cidr.source().method() == AttributeMethod::Inferred
            && self.base.cidr.rank() > existing.base.cidr.rank()
            && self.base.network_id == existing.base.network_id
            && !self.is_organizational_subnet()
            && !existing.is_organizational_subnet()
            && overlaps(&self.base.cidr, &existing.base.cidr)
    }

    pub fn is_user_managed(&self) -> bool {
        self.base.source == EntitySource::Manual || !self.base.subnet_type.is_synthetic_category()
    }
}

impl PartialEq for Subnet {
    fn eq(&self, other: &Self) -> bool {
        // On the CIDR *value*, never on the carrier. `cidr` is `Attributed<SubnetCidrValue>` and
        // `Attributed` derives `PartialEq` over both halves, so a bare `==` on the field asks
        // whether the two rows learned the range the same way -- not whether it is the same range.
        // That is the distinction `ip_addresses_match` already draws for MAC: the question is the
        // hardware, not the paperwork.
        //
        // It matters on every upgrade of a populated database. The attribution backfill stamps
        // stored rows `Unspecified` while a daemon reports its interfaced subnets as
        // `DaemonSelfReport`, so the first registration afterwards matched nothing and inserted a
        // second live row for every interfaced CIDR. Duplicate subnet ids then break host
        // matching, which compares IP *and* `subnet_id`.
        let network_match = self.base.cidr.value() == other.base.cidr.value()
            && self.base.network_id == other.base.network_id;

        network_match || self.id == other.id
    }
}

impl Hash for Subnet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.cidr.hash(state);
    }
}

impl Display for Subnet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Subnet {}: {}", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<Subnet> for Subnet {
    fn triggers_staleness(&self, _other: Option<Subnet>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::services::r#impl::patterns::ClientProbe;
    use pnet::ipnetwork::IpNetwork;
    use std::str::FromStr;

    #[test]
    fn from_discovery_accepts_valid_prefix() {
        let ip = IpNetwork::from_str("192.168.1.0/24").unwrap();
        let result = Subnet::from_discovery("eth0".to_string(), &ip, Uuid::nil());
        assert!(result.is_some(), "/24 prefix should be accepted");
    }

    #[test]
    fn from_discovery_accepts_prefix_2() {
        let ip = IpNetwork::from_str("10.0.0.0/2").unwrap();
        let result = Subnet::from_discovery("eth0".to_string(), &ip, Uuid::nil());
        assert!(result.is_some(), "/2 prefix should be accepted");
    }

    fn subnet(subnet_type: SubnetType, virtualization_service_id: Option<Uuid>) -> Subnet {
        sourced_subnet(
            subnet_type,
            virtualization_service_id,
            EntitySource::Discovery,
        )
    }

    fn sourced_subnet(
        subnet_type: SubnetType,
        virtualization_service_id: Option<Uuid>,
        source: EntitySource,
    ) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr::IpCidr::from_str("172.30.10.0/24").unwrap()),
                AttributeSource::DaemonSelfReport,
            ),
            network_id: Uuid::nil(),
            name: "172.30.10.0/24".into(),
            description: None,
            subnet_type,
            virtualization_service_id,
            source,
            tags: Vec::new(),
        })
    }

    /// #663: the reporter's access-point guest subnet was already stored as
    /// `DockerBridge`, so the classification fix alone would never reach them —
    /// rediscovery dedups into the stale row. A bridge row with no virtualization
    /// can only be a name-derived guess, so a current non-bridge observation
    /// corrects it.
    #[test]
    fn stale_bridge_guess_is_corrected_by_a_fresh_observation() {
        let guess = subnet(SubnetType::DockerBridge, None);
        assert!(subnet(SubnetType::Guest, None).corrects_container_bridge_guess(&guess));
    }

    /// A bridge carrying virtualization came from the runtime API, so nothing
    /// derived from an interface name may downgrade it.
    #[test]
    fn authoritative_bridge_is_never_downgraded() {
        let authoritative = subnet(SubnetType::DockerBridge, Some(Uuid::new_v4()));
        assert!(!subnet(SubnetType::Guest, None).corrects_container_bridge_guess(&authoritative));
        // Nor may one bridge observation replace another.
        assert!(
            !subnet(SubnetType::DockerBridge, None)
                .corrects_container_bridge_guess(&subnet(SubnetType::DockerBridge, None))
        );
    }

    /// A manually-set subnet type is a user's assertion, not a guess to repair.
    /// The CIDR dedup matches a Manual row against an incoming discovered one,
    /// so without the source check discovery would silently overwrite it.
    #[test]
    fn manual_subnet_type_is_never_overwritten_by_discovery() {
        let manual = sourced_subnet(SubnetType::DockerBridge, None, EntitySource::Manual);
        assert!(!subnet(SubnetType::Guest, None).corrects_container_bridge_guess(&manual));

        // ...and a manual observation may not rewrite a discovered row either.
        let discovered = subnet(SubnetType::DockerBridge, None);
        assert!(
            !sourced_subnet(SubnetType::Guest, None, EntitySource::Manual)
                .corrects_container_bridge_guess(&discovered)
        );
    }

    /// GH #677: assigning `Remote` to a subnet they had created made it vanish from
    /// every management view — unlistable, so uneditable and undeletable, while its
    /// CIDR still blocked any attempt to recreate it. Whatever category a user picks,
    /// the subnet stays theirs to manage.
    #[test]
    fn a_user_created_subnet_is_managed_whatever_its_category() {
        use strum::IntoEnumIterator;
        for subnet_type in SubnetType::iter() {
            let manual = sourced_subnet(subnet_type, None, EntitySource::Manual);
            assert!(
                manual.is_user_managed(),
                "{subnet_type:?}: a Manual subnet must never drop out of the management lists"
            );
        }
    }

    /// The other half of the rule: the rows Scanopy fabricates for itself stay out
    /// of the lists a user curates. Built from the real constructors, so this fails
    /// if one of them stops stamping the source the rule reads.
    #[test]
    fn scanopy_fabricated_subnets_are_not_user_managed() {
        use crate::server::shared::storage::seed_data;

        let network_id = Uuid::new_v4();
        assert!(!seed_data::create_wan_subnet(network_id).is_user_managed());
        assert!(!seed_data::create_remote_subnet(network_id).is_user_managed());

        let loopback_ip = IpNetwork::V4(
            pnet::ipnetwork::Ipv4Network::new(std::net::Ipv4Addr::LOCALHOST, 8).unwrap(),
        );
        let loopback = Subnet::from_discovery("lo".to_string(), &loopback_ip, network_id)
            .expect("loopback /8 should produce a subnet");
        assert!(!loopback.is_user_managed());

        // A discovered subnet of any other category is real inventory and stays.
        let discovered = IpNetwork::from_str("192.168.1.0/24").unwrap();
        let lan = Subnet::from_discovery("eth0".to_string(), &discovered, network_id)
            .expect("/24 should produce a subnet");
        assert!(lan.is_user_managed());
    }

    /// Guards the invariants `HostService::seed_loopback` depends on: the daemon-host
    /// loopback seed must produce a `127.0.0.0/8`, `Discovery`-sourced, non-bridge subnet.
    /// Exact CIDR is required for subnet dedup (`Subnet::eq`), and the `Discovery` source is
    /// required for `SubnetService::create` to reuse it when self-report re-reports loopback
    /// (a `System` source would not dedup).
    #[test]
    fn from_discovery_loopback_seed_shape() {
        let ip = IpNetwork::V4(
            pnet::ipnetwork::Ipv4Network::new(std::net::Ipv4Addr::LOCALHOST, 8).unwrap(),
        );
        let subnet = Subnet::from_discovery("lo".to_string(), &ip, Uuid::nil())
            .expect("loopback /8 should produce a subnet");
        assert_eq!(subnet.base.cidr.to_string(), "127.0.0.0/8");
        assert_eq!(subnet.base.source, EntitySource::Discovery);
        assert!(subnet.base.subnet_type.is_loopback());
        assert!(!subnet.is_container_bridge_subnet());
    }

    /// The rungs these tests turn on, under the shared ladder. `Observed` split into the four
    /// vantage points it used to collapse; a daemon reading its own interface stands in for the
    /// "real reading" it meant, and `Probe(Snmp)` for a *different* source at the same rung.
    const INFERRED: AttributeSource = AttributeSource::LldpNeighbourAddress;
    const READING: AttributeSource = AttributeSource::DaemonSelfReport;
    const CONFIRMED: AttributeSource = AttributeSource::Manual;

    fn ranged(cidr: &str, cidr_source: AttributeSource) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue(cidr.parse().expect("valid test CIDR")),
                cidr_source,
            ),
            source: EntitySource::Discovery,
            network_id: Uuid::nil(),
            ..Default::default()
        })
    }

    /// The ladder in both directions: a reading settles a guess, and a guess never unsettles a
    /// reading.
    #[test]
    fn a_range_is_only_replaced_by_something_at_least_as_authoritative() {
        let mut subnet = ranged("10.20.30.0/24", INFERRED);

        assert!(subnet.apply_cidr("10.20.30.0/23".parse().unwrap(), READING));
        assert_eq!(subnet.base.cidr.to_string(), "10.20.30.0/23");
        assert_eq!(subnet.base.cidr.source(), READING);

        assert!(
            !subnet.apply_cidr("10.20.30.0/24".parse().unwrap(), INFERRED),
            "an inference must not displace a reading"
        );
        assert_eq!(subnet.base.cidr.to_string(), "10.20.30.0/23");
    }

    /// Equal rank still wins, which is what makes a re-scan idempotent in the useful direction: a
    /// device re-reporting its own netmask refreshes the observation rather than being ignored.
    #[test]
    fn an_equal_rung_refreshes_the_range_but_an_unchanged_one_reports_nothing() {
        let mut subnet = ranged("10.20.30.0/24", READING);

        assert!(subnet.apply_cidr("10.20.31.0/24".parse().unwrap(), READING));
        assert!(
            !subnet.apply_cidr("10.20.31.0/24".parse().unwrap(), READING),
            "the same range at the same rung is not a change"
        );
    }

    /// Two sources at one rung do not displace each other. `Observed` used to be a single rung
    /// covering a daemon's own interface, a device's `ipAdEntNetMask` and a container runtime's
    /// IPAM config, so this case could not arise; now that they are distinct, whichever integration
    /// finished first must not decide the range.
    #[test]
    fn an_equal_rung_from_a_different_source_does_not_displace() {
        let mut subnet = ranged("10.20.30.0/24", READING);
        let other_source_same_rung = AttributeSource::Probe(ClientProbe::Snmp);
        assert_eq!(
            other_source_same_rung.rank(),
            READING.rank(),
            "precondition: the same rung"
        );

        assert!(!subnet.apply_cidr("10.20.31.0/24".parse().unwrap(), other_source_same_rung));
        assert_eq!(subnet.base.cidr.to_string(), "10.20.30.0/24");
    }

    /// Nothing a scan reads displaces a range a person confirmed.
    #[test]
    fn a_confirmed_range_is_never_moved_by_a_reading() {
        let mut subnet = ranged("10.20.30.0/24", CONFIRMED);

        assert!(!subnet.apply_cidr("10.20.30.0/23".parse().unwrap(), READING));
        assert_eq!(subnet.base.cidr.to_string(), "10.20.30.0/24");
    }

    /// A reading corrects a guess it nests with, in either direction — the segment is the same one
    /// whether the netmask turned out wider or narrower than assumed.
    #[test]
    fn a_reading_corrects_an_inferred_range_it_nests_with() {
        let inferred = ranged("10.20.30.0/24", INFERRED);

        let wider = ranged("10.20.30.0/23", READING);
        let narrower = ranged("10.20.30.0/25", READING);
        let same = ranged("10.20.30.0/24", READING);

        assert!(wider.corrects_inferred_range(&inferred));
        assert!(narrower.corrects_inferred_range(&inferred));
        assert!(same.corrects_inferred_range(&inferred));
    }

    /// A range that shares no address with the guess says nothing about it.
    #[test]
    fn a_disjoint_reading_corrects_nothing() {
        let inferred = ranged("10.20.30.0/24", INFERRED);
        let elsewhere = ranged("192.168.4.0/22", READING);

        assert!(!elsewhere.corrects_inferred_range(&inferred));
    }

    /// One inference never corrects another. This is what stops the re-placement of displaced
    /// addresses recursing: re-placing mints `Inferred` subnets, and the `create` inside that call
    /// must not be able to correct anything.
    #[test]
    fn an_inference_never_corrects_another_inference() {
        let existing = ranged("10.20.30.0/24", INFERRED);
        let incoming = ranged("10.20.30.0/23", INFERRED);

        assert!(!incoming.corrects_inferred_range(&existing));
    }

    /// Nor does a reading correct a range that was already settled.
    #[test]
    fn a_settled_range_is_not_corrected() {
        let observed = ranged("10.20.30.0/24", READING);
        let incoming = ranged("10.20.30.0/23", CONFIRMED);

        assert!(!incoming.corrects_inferred_range(&observed));
    }

    /// The seeded catch-alls contain every private range on the network, so without excluding them
    /// the first `0.0.0.0/0` to arrive would swallow every inferred row.
    #[test]
    fn the_organizational_catch_alls_correct_nothing() {
        let inferred = ranged("10.20.30.0/24", INFERRED);

        let mut catch_all = ranged("0.0.0.0/0", READING);
        catch_all.base.source = EntitySource::System;
        catch_all.base.subnet_type = SubnetType::Internet;

        assert!(!catch_all.corrects_inferred_range(&inferred));
    }
}
