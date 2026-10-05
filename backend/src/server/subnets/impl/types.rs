use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::shared::{
    concepts::Concept,
    entities::EntityDiscriminants,
    types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumDiscriminants,
    EnumIter,
    IntoStaticStr,
    Default,
    ToSchema,
)]
#[strum_discriminants(derive(Display, Hash, Serialize, Deserialize, EnumIter))]
pub enum SubnetType {
    Internet,
    Remote,

    Gateway,
    VpnTunnel,
    Dmz,

    Lan,
    WiFi,
    IoT,
    Guest,

    DockerBridge,
    PodmanBridge,
    MacVlan,
    IpVlan,
    Management,
    Storage,
    Loopback,

    // `other` makes any variant a newer server emits that this build doesn't
    // know (the production `unknown variant 'Loopback'` failure) degrade to
    // `Unknown` instead of hard-erroring. Subsumes the former `alias = "None"`.
    #[default]
    #[serde(other)]
    Unknown,
}

impl FromStr for SubnetType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Internet" => Ok(SubnetType::Internet),
            "Remote" => Ok(SubnetType::Remote),
            "Gateway" => Ok(SubnetType::Gateway),
            "VpnTunnel" => Ok(SubnetType::VpnTunnel),
            "Dmz" => Ok(SubnetType::Dmz),
            "Lan" => Ok(SubnetType::Lan),
            "WiFi" => Ok(SubnetType::WiFi),
            "IoT" => Ok(SubnetType::IoT),
            "Guest" => Ok(SubnetType::Guest),
            "DockerBridge" => Ok(SubnetType::DockerBridge),
            "PodmanBridge" => Ok(SubnetType::PodmanBridge),
            "MacVlan" => Ok(SubnetType::MacVlan),
            "IpVlan" => Ok(SubnetType::IpVlan),
            "Management" => Ok(SubnetType::Management),
            "Storage" => Ok(SubnetType::Storage),
            "Loopback" => Ok(SubnetType::Loopback),
            "Unknown" | "None" => Ok(SubnetType::Unknown),
            // Degrade rather than error, matching the `#[serde(other)]` behaviour on
            // the enum. This path is the DB read (`Subnet::from_row`), where an
            // `Err` fails the whole row load — so a binary that predates a newly
            // added variant would otherwise be unable to read those subnets at all.
            // `SubnetType` persists via `SqlValue::String`, so it is not covered by
            // the `db_enum_baseline` coexistence guard.
            other => {
                tracing::warn!(
                    subnet_type = %other,
                    "Unrecognized SubnetType; degrading to Unknown"
                );
                Ok(SubnetType::Unknown)
            }
        }
    }
}

impl SubnetType {
    /// Whether this subnet type represents a container-runtime network
    /// (Docker/Podman bridge, MacVLAN, IpVLAN).
    pub fn is_container_network(&self) -> bool {
        matches!(
            self,
            SubnetType::DockerBridge
                | SubnetType::PodmanBridge
                | SubnetType::MacVlan
                | SubnetType::IpVlan
        )
    }

    /// Classify a subnet from the interface name it was observed on.
    ///
    /// Interface names are a guess, so this deliberately **cannot** return a
    /// container-runtime type (`DockerBridge`, `PodmanBridge`, `MacVlan`,
    /// `IpVlan`). Those require evidence from the runtime's own API and are
    /// produced only by `ContainerRuntime::subnet_from_network`. A router's
    /// bridge (`br-guest`) and a Docker bridge (`br-1a2b3c4d5e6f`) are not
    /// distinguishable by name, and guessing labelled a reporter's AP guest
    /// network as Docker (#663).
    pub fn from_interface_name(interface_name: &str) -> Self {
        // Loopback ip_addresses (lo on Linux, lo0 on macOS)
        if Self::match_interface_names(&["lo"], interface_name) {
            return SubnetType::Loopback;
        }

        // VPN tunnels
        if Self::match_interface_names(&["tun", "utun", "wg", "tap", "ppp", "vpn"], interface_name)
        {
            return SubnetType::VpnTunnel;
        }

        // WiFi ip_addresses
        if Self::match_interface_names(&["wlan", "wifi", "wl"], interface_name) {
            return SubnetType::WiFi;
        }

        // Guest network (often labeled explicitly)
        if Self::match_interface_names(&["guest"], interface_name) {
            return SubnetType::Guest;
        }

        // IoT network (some routers use this naming)
        if Self::match_interface_names(&["iot"], interface_name) {
            return SubnetType::IoT;
        }

        // DMZ (often labeled explicitly)
        if Self::match_interface_names(&["dmz"], interface_name) {
            return SubnetType::Dmz;
        }

        // Management ip_addresses
        if Self::match_interface_names(&["mgmt", "ipmi", "bmc"], interface_name) {
            return SubnetType::Management;
        }

        // Storage networks
        if Self::match_interface_names(&["iscsi", "san", "storage"], interface_name) {
            return SubnetType::Storage;
        }

        // Standard LAN ip_addresses (catch-all for ethernet and Linux bridges)
        // Note: "br" (e.g., br0) is a Linux bridge, commonly used on Unraid/Proxmox for LAN
        if Self::match_interface_names(
            &["eth", "en", "eno", "enp", "ens", "br", "lan"],
            interface_name,
        ) {
            return SubnetType::Lan;
        }

        // A `br-<name>` bridge is named after the network it bridges, so classify
        // by the suffix: `br-guest` is a guest network, `br-iot` an IoT one.
        // Docker's `br-<12 hex>` bridges reach here too and fall through to
        // `Unknown` — container bridges are typed from the runtime API alone.
        if let Some(bridged) = interface_name.to_lowercase().strip_prefix("br-")
            && !bridged.is_empty()
        {
            return Self::from_interface_name(bridged);
        }

        SubnetType::Unknown
    }

    fn match_interface_names(patterns: &[&str], interface_name: &str) -> bool {
        let name_lower = interface_name.to_lowercase();
        patterns.iter().any(|pattern| {
            name_lower.starts_with(pattern)
                && name_lower
                    .get(pattern.len()..)
                    .map(|rest| {
                        rest.is_empty() || rest.chars().next().unwrap_or_default().is_ascii_digit()
                    })
                    .unwrap_or(false)
        })
    }

    /// Whether this subnet is a container-runtime bridge network
    /// (Docker or Podman). MacVLAN/IpVLAN are container networks but not bridges.
    pub fn is_container_bridge(&self) -> bool {
        matches!(self, SubnetType::DockerBridge | SubnetType::PodmanBridge)
    }

    /// Human-facing runtime label for a container bridge network
    /// (`"Docker"` / `"Podman"`), used to derive runtime-neutral container headers.
    /// `None` for non-bridge subnet types.
    pub fn container_runtime_label(&self) -> Option<&'static str> {
        match self {
            SubnetType::DockerBridge => Some("Docker"),
            SubnetType::PodmanBridge => Some("Podman"),
            _ => None,
        }
    }

    pub fn is_loopback(&self) -> bool {
        matches!(self, SubnetType::Loopback)
    }

    pub fn is_vlan_network(&self) -> bool {
        matches!(self, SubnetType::MacVlan | SubnetType::IpVlan)
    }

    pub fn exclude_from_topology(&self) -> bool {
        matches!(self, SubnetType::Loopback)
    }

    /// The categories Scanopy assigns to the subnets it fabricates for itself:
    /// the per-site `0.0.0.0/0` Internet and Remote supernets seeded by
    /// `seed_data`, and the `127.0.0.0/8` loopback row seeded per daemon host.
    ///
    /// One definition, shared by [`Self::is_synthetic_category`] and the SQL in
    /// `StorableFilter::<Subnet>::user_managed`, so the two cannot drift.
    pub fn synthetic_categories() -> &'static [SubnetType] {
        &[
            SubnetType::Internet,
            SubnetType::Remote,
            SubnetType::Loopback,
        ]
    }

    /// Whether this is one of the categories Scanopy fabricates rows in.
    ///
    /// **Half a verdict, never the whole one.** A user may legitimately assign
    /// any of these categories to a subnet they created, and that subnet is
    /// still theirs to manage — see [`Subnet::is_user_managed`], which is the
    /// predicate management lists must use. Keying visibility on the category
    /// alone is what stranded a reporter's `Remote` subnet in GH #677.
    ///
    /// [`Subnet::is_user_managed`]: crate::server::subnets::r#impl::base::Subnet::is_user_managed
    pub fn is_synthetic_category(&self) -> bool {
        Self::synthetic_categories().contains(self)
    }

    pub fn show_label(&self) -> bool {
        !matches!(self, SubnetType::Unknown | SubnetType::Loopback)
    }
}

impl HasId for SubnetType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SubnetType {
    fn color(&self) -> Color {
        match self {
            SubnetType::Internet => Color::Blue,
            SubnetType::Remote => EntityDiscriminants::Subnet.color(),

            SubnetType::Gateway => Concept::Gateway.color(),
            SubnetType::VpnTunnel => Concept::Vpn.color(),
            SubnetType::Dmz => Color::Rose,

            SubnetType::Lan => EntityDiscriminants::Subnet.color(),
            SubnetType::IoT => Concept::IoT.color(),
            SubnetType::Guest => Color::Green,
            SubnetType::WiFi => Color::Teal,

            SubnetType::Management => Color::Gray,
            SubnetType::DockerBridge => Concept::Containerization.color(),
            SubnetType::PodmanBridge => Concept::Containerization.color(),
            SubnetType::MacVlan => Concept::Containerization.color(),
            SubnetType::IpVlan => Concept::Containerization.color(),
            SubnetType::Storage => Concept::Storage.color(),
            SubnetType::Loopback => Color::Gray,

            SubnetType::Unknown => Color::Gray,
        }
    }
    fn icon(&self) -> Icon {
        match self {
            SubnetType::Internet => Icon::Globe,
            SubnetType::Remote => EntityDiscriminants::Subnet.icon(),

            SubnetType::Gateway => Concept::Gateway.icon(),
            SubnetType::VpnTunnel => Concept::Vpn.icon(),
            SubnetType::Dmz => EntityDiscriminants::Subnet.icon(),

            SubnetType::Lan => EntityDiscriminants::Subnet.icon(),
            SubnetType::IoT => Concept::IoT.icon(),
            SubnetType::Guest => Icon::User,
            SubnetType::WiFi => Icon::Wifi,

            SubnetType::Management => Icon::ServerCog,
            SubnetType::DockerBridge => Icon::Box,
            SubnetType::PodmanBridge => Icon::Box,
            SubnetType::MacVlan => Icon::Network,
            SubnetType::IpVlan => Icon::Network,
            SubnetType::Storage => Concept::Storage.icon(),
            SubnetType::Loopback => Icon::Network,

            SubnetType::Unknown => EntityDiscriminants::Subnet.icon(),
        }
    }
}

impl TypeMetadataProvider for SubnetType {
    fn name(&self) -> &'static str {
        match self {
            SubnetType::Internet => "Internet",
            SubnetType::Remote => "Remote",

            SubnetType::Gateway => "Gateway",
            SubnetType::VpnTunnel => "VPN",
            SubnetType::Dmz => "DMZ",

            SubnetType::Lan => "LAN",
            SubnetType::IoT => "IoT",
            SubnetType::Guest => "Guest",
            SubnetType::WiFi => "WiFi",

            SubnetType::Management => "Management",
            SubnetType::DockerBridge => "Docker Bridge",
            SubnetType::PodmanBridge => "Podman Bridge",
            SubnetType::MacVlan => "MacVLAN",
            SubnetType::IpVlan => "IpVLAN",
            SubnetType::Storage => "Storage",
            SubnetType::Loopback => "Loopback",

            SubnetType::Unknown => "Unknown",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            SubnetType::Internet => "Internet",
            SubnetType::Remote => "Remote network",

            SubnetType::Gateway => "Gateway subnet",
            SubnetType::VpnTunnel => "Virtual private network",
            SubnetType::Dmz => "Demilitarized zone",

            SubnetType::Lan => "Local area network",
            SubnetType::IoT => "Internet of things",
            SubnetType::Guest => "Guest network",
            SubnetType::WiFi => "WiFi network",

            SubnetType::Management => "Management network",
            SubnetType::DockerBridge => "Docker bridge network",
            SubnetType::PodmanBridge => "Podman bridge network",
            SubnetType::MacVlan => "MacVLAN network",
            SubnetType::IpVlan => "IpVLAN network",
            SubnetType::Storage => "Storage network",
            SubnetType::Loopback => "Host-local loopback, excluded from topology and scans",

            SubnetType::Unknown => "Unknown network type",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let network_scan_discovery_eligible = !matches!(
            &self,
            SubnetType::Remote
                | SubnetType::Internet
                | SubnetType::DockerBridge
                | SubnetType::PodmanBridge
                | SubnetType::Loopback
        );

        let is_for_containers = matches!(
            self,
            SubnetType::DockerBridge
                | SubnetType::PodmanBridge
                | SubnetType::MacVlan
                | SubnetType::IpVlan
        );

        serde_json::json!({
            "network_scan_discovery_eligible": network_scan_discovery_eligible,
            "is_for_containers": is_for_containers,
            "is_container_bridge": self.is_container_bridge(),
            "show_label": self.show_label(),
            "is_synthetic_category": self.is_synthetic_category()
        })
    }
}

#[cfg(test)]
mod interface_name_tests {
    use super::*;

    /// The invariant behind #663: no interface name, however Docker-shaped, may
    /// yield a container-runtime type. Those come only from
    /// `ContainerRuntime::subnet_from_network`, which has the runtime's own
    /// driver as evidence. A name is never enough — `br-guest` on an access
    /// point and `br-<12 hex>` on a Docker host are indistinguishable.
    #[test]
    fn no_interface_name_yields_a_container_type() {
        for name in [
            "docker0",
            "docker_gwbridge",
            "br-1a2b3c4d5e6f",
            "br-guest",
            "podman0",
            "cni-podman0",
            "macvlan0",
            "ipvlan0",
            "eth0",
            "lo",
            "",
        ] {
            let subnet_type = SubnetType::from_interface_name(name);
            assert!(
                !subnet_type.is_container_network(),
                "{name} classified as {subnet_type:?}; interface names must never imply a container runtime"
            );
        }
    }

    /// #663: an Araknis access point's NAT guest bridge, reported over SNMP as an
    /// `ifName`, was typed `DockerBridge` and rendered "Docker @ <AP>". A `br-`
    /// bridge is named for the network it bridges, so classify by that.
    #[test]
    fn router_bridge_classifies_by_what_it_bridges() {
        assert_eq!(
            SubnetType::from_interface_name("br-guest"),
            SubnetType::Guest
        );
        assert_eq!(SubnetType::from_interface_name("br-lan"), SubnetType::Lan);
        assert_eq!(SubnetType::from_interface_name("br-iot"), SubnetType::IoT);
        // Docker's own `br-<12 hex>` carries no such meaning and stays unknown
        // until the runtime API reports it.
        assert_eq!(
            SubnetType::from_interface_name("br-1a2b3c4d5e6f"),
            SubnetType::Unknown
        );
        // A plain Linux bridge is still a LAN.
        assert_eq!(SubnetType::from_interface_name("br0"), SubnetType::Lan);
    }
}

#[cfg(test)]
mod forward_compat_tests {
    use super::*;

    #[test]
    fn unknown_variant_degrades_to_unknown() {
        // Reproduces the production `unknown variant 'Loopback'` failure class:
        // a subnet type a newer server emits that this build doesn't know now
        // degrades to `Unknown` instead of failing the whole subnets response.
        let parsed: SubnetType = serde_json::from_str("\"SomeFutureType\"").unwrap();
        assert_eq!(parsed, SubnetType::Unknown);
    }

    #[test]
    fn none_still_parses_to_unknown() {
        // The former `#[serde(alias = "None")]` is subsumed by `#[serde(other)]`.
        let parsed: SubnetType = serde_json::from_str("\"None\"").unwrap();
        assert_eq!(parsed, SubnetType::Unknown);
    }

    #[test]
    fn known_variants_round_trip() {
        for variant in [
            SubnetType::Loopback,
            SubnetType::Lan,
            SubnetType::DockerBridge,
            SubnetType::Unknown,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: SubnetType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn container_runtime_label_present_iff_container_bridge() {
        // Invariant: a runtime label exists exactly for the container-bridge types.
        // Locks the two predicates together without restating each mapping, so it
        // fails only if a new bridge type forgets its label (or vice versa).
        use strum::IntoEnumIterator;
        for st in SubnetType::iter() {
            assert_eq!(
                st.container_runtime_label().is_some(),
                st.is_container_bridge(),
                "{st:?}: runtime label presence must match is_container_bridge()"
            );
        }
    }
}
