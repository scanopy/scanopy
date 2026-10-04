use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::Hash;
use strum_macros::{EnumDiscriminants, EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;
use validator::Validate;

use crate::server::{
    credentials::r#impl::types::CredentialIntegration,
    hosts::r#impl::base::Host,
    shared::{
        concepts::Concept,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
    topology::types::views::{FilterValueContext, HasFilterValues, MetadataFilterType},
};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumDiscriminants,
    VariantNames,
    ToSchema,
)]
#[strum_discriminants(derive(IntoStaticStr, EnumIter))]
#[schema(title = "HostVirtualization")]
#[serde(tag = "type", content = "details")]
pub enum HostVirtualization {
    #[schema(title = "Proxmox")]
    Proxmox(ProxmoxVirtualization),
    #[schema(title = "VCenter")]
    VCenter(VCenterVirtualization),
    #[schema(title = "ESXi")]
    ESXi(EsxiVirtualization),
    #[schema(title = "Docker")]
    Docker(ContainerHostVirtualization),
    #[schema(title = "Podman")]
    Podman(ContainerHostVirtualization),
    #[schema(title = "NetworkIdentity")]
    NetworkIdentity(NetworkIdentityVirtualization),
}

/// A container with its own identity on the LAN: a macvlan or ipvlan endpoint gives it a MAC and
/// an IP of its own, so it is a host under its runtime rather than a service on the runtime's
/// host. Containers on bridge networks stay services (`ServiceVirtualization`).
#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct ContainerHostVirtualization {
    /// Container name as reported by the runtime.
    pub container_name: Option<String>,
    /// Container ID as reported by the runtime.
    pub container_id: Option<String>,
    /// Compose project the container belongs to, when it was started by Compose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compose_project: Option<String>,
    /// The network driver that gives the container its own LAN address.
    pub network_type: ContainerNetworkType,
}

/// The container network drivers that put a container directly on the LAN.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumIter,
    VariantNames,
    ToSchema,
)]
pub enum ContainerNetworkType {
    /// A macvlan endpoint: its own MAC on the parent interface.
    MacVlan,
    /// An ipvlan endpoint: its own IP, sharing the parent interface's MAC.
    IpVlan,
}

impl ContainerNetworkType {
    /// Whether the endpoint puts a MAC of its own on the wire. An ipvlan endpoint answers ARP
    /// with its parent interface's MAC, so that MAC names the runtime's NIC, not the container.
    pub fn has_own_mac(&self) -> bool {
        match self {
            ContainerNetworkType::MacVlan => true,
            ContainerNetworkType::IpVlan => false,
        }
    }
}

impl HostVirtualization {
    /// Whether this host is a container under a runtime, rather than a machine.
    pub fn is_container(&self) -> bool {
        match self {
            HostVirtualization::Docker(_) | HostVirtualization::Podman(_) => true,
            HostVirtualization::Proxmox(_)
            | HostVirtualization::VCenter(_)
            | HostVirtualization::ESXi(_)
            | HostVirtualization::NetworkIdentity(_) => false,
        }
    }

    /// Whether a MAC seen at this host's addresses identifies this host. False for a container
    /// whose endpoint shares its parent's MAC: every other host answering with that MAC is the
    /// runtime's, and matching on it would merge them into the container.
    pub fn macs_identify_host(&self) -> bool {
        match self {
            HostVirtualization::Docker(c) | HostVirtualization::Podman(c) => {
                c.network_type.has_own_mac()
            }
            HostVirtualization::Proxmox(_)
            | HostVirtualization::VCenter(_)
            | HostVirtualization::ESXi(_)
            | HostVirtualization::NetworkIdentity(_) => true,
        }
    }
}

/// The kind of virtualization `host` carries that `integration` does not declare reporting
/// ([`CredentialIntegration::host_virtualizations`]), or `None` when it declares it or the host
/// carries none.
///
/// The declaration is what the docs list from, so a host an integration builds outside it means
/// the docs are wrong. Checked on every host an integration submits and in each integration's
/// mapping tests.
pub fn undeclared_virtualization(
    integration: CredentialIntegration,
    host: &Host,
) -> Option<HostVirtualizationDiscriminants> {
    let kind = HostVirtualizationDiscriminants::from(host.base.virtualization_metadata.as_ref()?);
    (!integration.host_virtualizations().contains(&kind)).then_some(kind)
}

/// An address and MAC that a host presents from an interface of its own beyond its configured
/// NICs: a macvlan shim, a virtual IP, a service given its own LAN address, an emulated device.
/// The reporting source proves only that the interface lives inside that host, not what it is,
/// so nothing here classifies it. The owner is the host's Network Identities service
/// (`HostBase::virtualization_service_id`), and the interface it sits on is
/// `HostBase::virtualization_interface_id`, both real columns with foreign keys. A stored row from
/// before the interface became a column still carries an `interface` name here, which is ignored.
#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct NetworkIdentityVirtualization {}

#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct ProxmoxVirtualization {
    /// Guest name as configured in Proxmox.
    pub vm_name: Option<String>,
    /// Proxmox VMID of the guest.
    pub vm_id: Option<String>,
    /// Whether the guest is a QEMU virtual machine or an LXC container. `None` when the guest was
    /// assigned by hand or recorded before the Proxmox integration reported it.
    #[serde(default)]
    pub guest_type: Option<ProxmoxGuestType>,
}

/// The two kinds of guest a Proxmox VE node runs. Both are hosts with their own addresses, so
/// both hang off the node's Proxmox VE service the same way.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumIter,
    VariantNames,
    ToSchema,
)]
pub enum ProxmoxGuestType {
    /// A QEMU/KVM virtual machine.
    Qemu,
    /// An LXC system container.
    Lxc,
}

#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct VCenterVirtualization {
    /// Guest name as configured in vCenter.
    pub vm_name: Option<String>,
    /// vCenter managed object ID of the guest.
    pub vm_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct EsxiVirtualization {
    /// Guest name as configured on the ESXi host.
    pub vm_name: Option<String>,
    /// ESXi identifier of the guest.
    pub vm_id: Option<String>,
}

// The virtualizing service is `Host::virtualization_service_id`, a real foreign key, rather than
// a field inside each of these payloads. Readers used to `match` a single variant to reach it —
// `get_host_is_virtualized_by` handled only Proxmox — so vCenter and ESXi guests silently had no
// hypervisor. A column has no variants to miss.

impl HasId for HostVirtualization {
    fn id(&self) -> &'static str {
        self.into()
    }
}

/// Metadata lives on the discriminant so it can be iterated into `host-virtualizations.json`
/// without inventing a payload per variant. The id is the serde `type` tag, the same string
/// `HostVirtualization::id()` returns.
impl HasId for HostVirtualizationDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for HostVirtualizationDiscriminants {
    fn color(&self) -> Color {
        Concept::Virtualization.color()
    }
    fn icon(&self) -> Icon {
        Concept::Virtualization.icon()
    }
}

impl TypeMetadataProvider for HostVirtualizationDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            Self::Proxmox => "Proxmox",
            Self::VCenter => "vCenter",
            Self::ESXi => "ESXi",
            Self::Docker => "Docker",
            Self::Podman => "Podman",
            Self::NetworkIdentity => "Network identity",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Proxmox => "A host running as a Proxmox VM or LXC container",
            Self::VCenter => "A host running as a vCenter-managed VM",
            Self::ESXi => "A host running as an ESXi VM",
            Self::Docker => "A host running as a Docker container with its own LAN address",
            Self::Podman => "A host running as a Podman container with its own LAN address",
            Self::NetworkIdentity => {
                "An address and MAC that another host presents on the network from one of its own interfaces"
            }
        }
    }
}

impl HasId for ContainerNetworkType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ContainerNetworkType {
    fn color(&self) -> Color {
        Concept::Containerization.color()
    }
    fn icon(&self) -> Icon {
        Concept::Containerization.icon()
    }
}

impl TypeMetadataProvider for ContainerNetworkType {
    fn name(&self) -> &'static str {
        match self {
            Self::MacVlan => "macvlan",
            Self::IpVlan => "ipvlan",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::MacVlan => "Container on a macvlan network, with its own MAC and IP on the LAN",
            Self::IpVlan => "Container on an ipvlan network, with its own IP on the LAN",
        }
    }
}

impl HasId for ProxmoxGuestType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ProxmoxGuestType {
    fn color(&self) -> Color {
        Concept::Virtualization.color()
    }
    fn icon(&self) -> Icon {
        Concept::Virtualization.icon()
    }
}

impl TypeMetadataProvider for ProxmoxGuestType {
    fn name(&self) -> &'static str {
        match self {
            Self::Qemu => "VM",
            Self::Lxc => "LXC",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Qemu => "QEMU virtual machine",
            Self::Lxc => "LXC container",
        }
    }
}

/// Coarse virtualization state used by the `Virtualization` metadata filter
/// on Host. Each host resolves to exactly one variant via `HasFilterValues`.
/// Today derived from `host.virtualization.is_some()`; future finer states
/// (e.g. per-hypervisor) can add variants here without breaking persistence
/// of the existing ids.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumIter,
    ToSchema,
)]
pub enum HostVirtualizationState {
    Virtualized,
    BareMetal,
}

impl HostVirtualizationState {
    pub fn from_host_virtualization(v: Option<&HostVirtualization>) -> Self {
        match v {
            Some(_) => Self::Virtualized,
            None => Self::BareMetal,
        }
    }
}

impl HasId for HostVirtualizationState {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for HostVirtualizationState {
    fn color(&self) -> Color {
        match self {
            Self::Virtualized => Concept::Virtualization.color(),
            Self::BareMetal => Color::Gray,
        }
    }
    fn icon(&self) -> Icon {
        match self {
            Self::Virtualized => Concept::Virtualization.icon(),
            Self::BareMetal => Icon::Server,
        }
    }
}

impl TypeMetadataProvider for HostVirtualizationState {
    fn name(&self) -> &'static str {
        match self {
            Self::Virtualized => "Virtualized",
            Self::BareMetal => "Bare metal",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Virtualized => "Hosts running as virtual machines or containers",
            Self::BareMetal => "Hosts running on physical hardware",
        }
    }
}

impl HasFilterValues for Host {
    fn filter_values(&self, _ctx: &FilterValueContext) -> BTreeMap<MetadataFilterType, String> {
        let mut values = BTreeMap::new();
        let state = HostVirtualizationState::from_host_virtualization(
            self.base.virtualization_metadata.as_ref(),
        );
        values.insert(MetadataFilterType::Virtualization, state.id().to_string());
        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::HostBase;
    use strum::IntoEnumIterator;

    fn identity_host() -> Host {
        Host::new(HostBase {
            virtualization_metadata: Some(HostVirtualization::NetworkIdentity(
                NetworkIdentityVirtualization {},
            )),
            ..Default::default()
        })
    }

    /// A host carrying a kind its integration declares passes; one it does not declare is named;
    /// a host with no virtualization is never a mismatch.
    #[test]
    fn a_host_outside_its_integrations_declaration_is_named() {
        let declaring = CredentialIntegration::iter()
            .find(|i| {
                i.host_virtualizations()
                    .contains(&HostVirtualizationDiscriminants::NetworkIdentity)
            })
            .expect("an integration reports network identities");
        let silent = CredentialIntegration::iter()
            .find(|i| i.host_virtualizations().is_empty())
            .expect("an integration reports no virtualization");

        assert_eq!(undeclared_virtualization(declaring, &identity_host()), None);
        assert_eq!(
            undeclared_virtualization(silent, &identity_host()),
            Some(HostVirtualizationDiscriminants::NetworkIdentity)
        );
        assert_eq!(
            undeclared_virtualization(silent, &Host::new(HostBase::default())),
            None
        );
    }

    #[test]
    fn host_virtualization_variants_round_trip_by_tag() {
        // The serde "type" tag is what the manual-assignment UI sends (sourced
        // from ServiceDefinition::virtualization_variant). Confirm each tag
        // deserializes and round-trips, and that the tag == HasId::id().
        for tag in ["Proxmox", "VCenter", "ESXi"] {
            let json = format!(r#"{{"type":"{tag}","details":{{"vm_name":null,"vm_id":null}}}}"#);
            let v: HostVirtualization =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("{tag}: {e}"));
            assert_eq!(v.id(), tag, "id() must equal serde tag for {tag}");
            let reserialized = serde_json::to_value(&v).unwrap();
            assert_eq!(reserialized["type"], tag);
        }
    }

    #[test]
    fn host_virtualization_discriminant_static_str_matches_serde_tag() {
        // The discriminant's IntoStaticStr is what VirtualizationRole::variant_tag
        // derives from; it must equal the serde "type" tag above.
        assert_eq!(
            <&'static str>::from(HostVirtualizationDiscriminants::Proxmox),
            "Proxmox"
        );
        assert_eq!(
            <&'static str>::from(HostVirtualizationDiscriminants::VCenter),
            "VCenter"
        );
        assert_eq!(
            <&'static str>::from(HostVirtualizationDiscriminants::ESXi),
            "ESXi"
        );
    }
}
