use crate::daemon::utils::app_probe::AppProbe;
use crate::server::hosts::r#impl::virtualization::HostVirtualizationDiscriminants;
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::definitions::docker_daemon::Docker;
use crate::server::services::definitions::esxi::Esxi;
use crate::server::services::definitions::network_identities::NetworkIdentities;
use crate::server::services::definitions::podman::Podman;
use crate::server::services::definitions::proxmox::Proxmox;
use crate::server::services::definitions::vcenter::VCenter;
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::patterns::Pattern;
use crate::server::services::r#impl::virtualization::ServiceVirtualizationDiscriminants;
use crate::server::shared::types::metadata::TypeMetadataProvider;
use crate::server::shared::types::metadata::{EntityMetadataProvider, HasId};
use crate::server::shared::types::{Color, Icon};
use dyn_clone::DynClone;
use dyn_eq::DynEq;
use dyn_hash::DynHash;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::hash::Hash;
use strum_macros::IntoStaticStr;
use utoipa::openapi::schema::{ObjectBuilder, SchemaType};
use utoipa::openapi::{RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

/// Why a service definition cannot validate what answered on its port.
///
/// Each variant names something that stops a probe existing, not how much work one would be.
/// "Nobody got round to it" is not among them, which is the point: the set is meant to shrink, and
/// the guard test in `services/impl/tests.rs` is what keeps it from growing quietly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectOnly {
    /// Writing to this port has side effects on the device.
    ///
    /// The raw-socket printer ports are the case: bytes sent to 9100 are printed. See
    /// [`crate::server::ports::r#impl::base::PortType::is_raw_socket`].
    ProbeUnsafe,
    /// No unauthenticated exchange distinguishes this service from any other listener.
    ///
    /// Encrypted or authenticated-first control channels: mutual TLS, CurveZMQ, a client
    /// certificate, CRAM-MD5 before anything identifying is sent.
    NoDistinguishingHandshake,
    /// There is no public implementation to check a match against.
    ///
    /// Commercial software with no pullable image and no published source. A match string written
    /// from vendor documentation alone is a guess, and a guess that happens to match any HTTP
    /// server is worse than the bare port it replaced — so the honest position is to say we could
    /// not verify one. Unlike the two above this is a statement about our access rather than about
    /// the protocol, which is why it is worth being able to tell apart: it becomes removable the
    /// day someone can point the probe at a real instance.
    NoVerifiableImplementation,
}

// Main trait used in service definition implementation
pub trait ServiceDefinition: HasId + DynClone + DynHash + DynEq + Send + Sync {
    /// Service name, will also be used as unique identifier. < 40 characters.
    fn name(&self) -> &'static str;

    /// Service description. < 100 characters.
    fn description(&self) -> &'static str;

    /// Category from ServiceCategory enum
    fn category(&self) -> ServiceCategory;

    /// How service should be identified during port scanning
    fn discovery_pattern(&self) -> Pattern<'_>;

    /// If service is not associated with a particular brand or vendor
    fn is_generic(&self) -> bool {
        false
    }

    /// The non-credentialed application probes that confirm this service.
    ///
    /// Empty by default: most definitions match on ports and HTTP endpoints alone. Hanging probes
    /// here rather than on a registry of their own is what makes "a probe cannot exist without a
    /// service definition" structural — see [`crate::daemon::utils::app_probe`].
    ///
    /// A list rather than one, because a service can speak more than one transport and each needs
    /// its own exchange. DNS is the case that forced it: UDP/53 resolves a name through a library
    /// client and TCP/53 sends a length-prefixed query, and a definition able to declare only one
    /// of them would leave the other port scanned but never validated.
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        Vec::new()
    }

    /// Why this definition is allowed to match on a completed TCP connection alone.
    ///
    /// `None` is the rule: a definition validates what answered, by reading a protocol response
    /// (`app_probe`) or an HTTP body or header (`Pattern::Endpoint`, `Pattern::Header`). A bare
    /// `Pattern::Port` names a service on evidence any middlebox in the path can manufacture, which
    /// is how a FortiGate SIP session helper became a "SIP Server" on every remote VLAN.
    ///
    /// Declaring a reason here is the deliberate exception, and it is checked: a test holds the set
    /// of definitions that declare one equal to the set
    /// [`Pattern::matches_on_connect_alone`] derives, so a new bare-port definition fails until its
    /// author either writes a probe or says here why they cannot.
    fn connect_only_rationale(&self) -> Option<ConnectOnly> {
        None
    }

    /// URL of icon, or static path if serving from /logos.
    /// Examples:
    /// Dashboard Icons: Home Assistant -> https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/home-assistant
    /// Simple Icons: Home Assistant -> https://simpleicons.org/icons/homeassistant.svg.
    /// Vector Logo Icons: Akamai -> https://www.vectorlogo.zone/logos/akamai/akamai-icon.svg
    /// Static file: Scanopy -> /logos/scanopy-logo.png
    fn logo_url(&self) -> &'static str {
        ""
    }

    /// Use this if available logo only has dark variant / if generally it would be more legible with a white background
    fn logo_needs_white_background(&self) -> bool {
        false
    }
}

impl<T: ServiceDefinition> HasId for T
where
    T: ServiceDefinition,
{
    fn id(&self) -> &'static str {
        self.name()
    }
}

impl ServiceDefinition for Box<dyn ServiceDefinition> {
    fn name(&self) -> &'static str {
        ServiceDefinition::name(&**self)
    }

    fn description(&self) -> &'static str {
        ServiceDefinition::description(&**self)
    }

    fn logo_url(&self) -> &'static str {
        ServiceDefinition::logo_url(&**self)
    }

    fn category(&self) -> ServiceCategory {
        ServiceDefinition::category(&**self)
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        ServiceDefinition::discovery_pattern(&**self)
    }

    fn is_generic(&self) -> bool {
        ServiceDefinition::is_generic(&**self)
    }

    fn logo_needs_white_background(&self) -> bool {
        ServiceDefinition::logo_needs_white_background(&**self)
    }

    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        ServiceDefinition::app_probes(&**self)
    }

    fn connect_only_rationale(&self) -> Option<ConnectOnly> {
        ServiceDefinition::connect_only_rationale(&**self)
    }
}

// Helper methods to be used in rest of codebase, not overridable by definition implementations
/// The virtualization role a manager service definition plays, paired with the
/// backing `HostVirtualization` / `ServiceVirtualization` enum variants it
/// produces. The variant strings (type + serde tag) are derived from the actual
/// enum discriminants, so they cannot drift from the persisted/deserialized
/// variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoStaticStr)]
pub enum VirtualizationRole {
    /// Manages VM hosts (Proxmox, vCenter, ESXi, …) -> "vms".
    #[strum(serialize = "vms")]
    Hypervisor {
        hosts: HostVirtualizationDiscriminants,
    },
    /// Manages containers (Docker, Podman, …) -> "containers". A container on a bridge network
    /// is a service (`services`); one with its own LAN address (macvlan, ipvlan) is a host
    /// (`hosts`).
    #[strum(serialize = "containers")]
    ContainerRuntime {
        services: ServiceVirtualizationDiscriminants,
        hosts: HostVirtualizationDiscriminants,
    },
    /// Holds the addresses and MACs a host presents beyond its own NICs (Network Identities) ->
    /// "identities". Each identity is a host (`hosts`).
    #[strum(serialize = "identities")]
    IdentityHost {
        hosts: HostVirtualizationDiscriminants,
    },
}

impl VirtualizationRole {
    /// Serde "type" discriminant of the variant manual assignment writes (e.g. "Proxmox",
    /// "VCenter", "Podman"): the host variant for a hypervisor, the service variant for a
    /// container runtime, the host variant for an identity host. Container hosts and network
    /// identities are never assigned by hand.
    pub fn variant_tag(&self) -> &'static str {
        match self {
            Self::Hypervisor { hosts } | Self::IdentityHost { hosts } => (*hosts).into(),
            Self::ContainerRuntime { services, .. } => (*services).into(),
        }
    }
}

pub trait ServiceDefinitionExt {
    fn can_be_manually_added(&self) -> bool;
    fn virtualization_role(&self) -> Option<VirtualizationRole>;
    fn is_scanopy(&self) -> bool;
    fn is_generic(&self) -> bool;
    fn is_gateway(&self) -> bool;
    fn is_open_ports(&self) -> bool;
    fn has_logo(&self) -> bool;
    fn gated_by_raw_socket_scanning(&self) -> bool;
}

impl ServiceDefinitionExt for Box<dyn ServiceDefinition> {
    fn can_be_manually_added(&self) -> bool {
        !matches!(
            ServiceDefinition::category(self),
            ServiceCategory::Scanopy
                | ServiceCategory::OpenPorts
                | ServiceCategory::NetworkIdentities
        )
    }

    fn is_generic(&self) -> bool {
        ServiceDefinition::is_generic(&**self)
    }

    fn is_scanopy(&self) -> bool {
        matches!(ServiceDefinition::category(self), ServiceCategory::Scanopy)
    }

    fn is_gateway(&self) -> bool {
        self.discovery_pattern().contains_gateway_ip_pattern()
    }

    fn is_open_ports(&self) -> bool {
        matches!(
            ServiceDefinition::category(self),
            ServiceCategory::OpenPorts
        )
    }

    fn has_logo(&self) -> bool {
        !self.logo_url().is_empty()
    }

    fn gated_by_raw_socket_scanning(&self) -> bool {
        self.discovery_pattern().gated_by_raw_socket_scanning()
    }

    /// Single source of truth mapping a manager service definition to the
    /// virtualization role + backing enum variant it produces. The "vms"/
    /// "containers"/"identities" type and the serde variant tag are both derived from this
    /// (see `VirtualizationRole`), so they cannot drift apart or from the enums.
    fn virtualization_role(&self) -> Option<VirtualizationRole> {
        let id = self.id();
        match id {
            _ if id == Proxmox.id() => Some(VirtualizationRole::Hypervisor {
                hosts: HostVirtualizationDiscriminants::Proxmox,
            }),
            _ if id == VCenter.id() => Some(VirtualizationRole::Hypervisor {
                hosts: HostVirtualizationDiscriminants::VCenter,
            }),
            _ if id == Esxi.id() => Some(VirtualizationRole::Hypervisor {
                hosts: HostVirtualizationDiscriminants::ESXi,
            }),
            _ if id == Docker.id() => Some(VirtualizationRole::ContainerRuntime {
                services: ServiceVirtualizationDiscriminants::Docker,
                hosts: HostVirtualizationDiscriminants::Docker,
            }),
            _ if id == Podman.id() => Some(VirtualizationRole::ContainerRuntime {
                services: ServiceVirtualizationDiscriminants::Podman,
                hosts: HostVirtualizationDiscriminants::Podman,
            }),
            _ if id == NetworkIdentities.id() => Some(VirtualizationRole::IdentityHost {
                hosts: HostVirtualizationDiscriminants::NetworkIdentity,
            }),
            _ => None,
        }
    }
}

impl EntityMetadataProvider for Box<dyn ServiceDefinition> {
    fn color(&self) -> Color {
        ServiceDefinition::category(self).color()
    }
    fn icon(&self) -> Icon {
        // Note: logo_url is available in metadata for services with custom logos
        ServiceDefinition::category(self).icon()
    }
}

impl TypeMetadataProvider for Box<dyn ServiceDefinition> {
    fn name(&self) -> &'static str {
        ServiceDefinition::name(self)
    }
    fn description(&self) -> &'static str {
        ServiceDefinition::description(self)
    }
    fn category(&self) -> &'static str {
        ServiceDefinition::category(self).id()
    }
    fn metadata(&self) -> serde_json::Value {
        let url = self.logo_url();
        let logo_ext = if url.is_empty() || url.starts_with('/') {
            ""
        } else {
            url.rsplit('.')
                .next()
                .and_then(|e| e.split('?').next())
                .filter(|e| matches!(*e, "svg" | "png" | "webp"))
                .unwrap_or("svg")
        };
        let role = self.virtualization_role();
        serde_json::json!({
            "can_be_added": self.can_be_manually_added(),
            "manages_virtualization": role.as_ref().map(<&'static str>::from),
            "virtualization_variant": role.as_ref().map(VirtualizationRole::variant_tag),
            "is_gateway": self.is_gateway(),
            "is_generic": ServiceDefinition::is_generic(&**self),
            "has_logo": self.has_logo(),
            "logo_ext": logo_ext,
            "logo_needs_white_background": self.logo_needs_white_background(),
            "gated_by_raw_socket_scanning": self.gated_by_raw_socket_scanning(),
            // Derived from the rationale rather than listed, so the set the `trust_port_only_
            // detections` setting governs shrinks by itself as exceptions are retired and cannot
            // drift from `connect_only_definitions_are_declared`.
            "connect_only": self.connect_only_rationale().is_some(),
        })
    }
}

dyn_eq::eq_trait_object!(ServiceDefinition);
dyn_hash::hash_trait_object!(ServiceDefinition);
dyn_clone::clone_trait_object!(ServiceDefinition);

impl Default for Box<dyn ServiceDefinition> {
    fn default() -> Self {
        Box::new(DefaultServiceDefinition)
    }
}

impl std::fmt::Debug for Box<dyn ServiceDefinition> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, category: {}, description: {}",
            ServiceDefinition::name(&**self),
            ServiceDefinition::category(&**self),
            ServiceDefinition::description(&**self)
        )
    }
}

impl Serialize for Box<dyn ServiceDefinition> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.id())
    }
}

impl<'de> Deserialize<'de> for Box<dyn ServiceDefinition> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = String::deserialize(deserializer)?;
        match ServiceDefinitionRegistry::find_by_id(&id) {
            Some(def) => Ok(def),
            None => {
                // Log a warning but don't fail deserialization
                tracing::warn!(
                    "Service definition not found: '{}'. Using UnknownServiceDefinition as fallback. \
                    This may indicate a missing module declaration in mod.rs or a renamed service.",
                    id
                );

                // Return Default instead of failing
                Ok(Box::new(DefaultServiceDefinition))
            }
        }
    }
}

/// OpenAPI schema for Box<dyn ServiceDefinition>
/// Serializes as a string containing the service definition ID
impl PartialSchema for Box<dyn ServiceDefinition> {
    fn schema() -> RefOr<Schema> {
        use utoipa::openapi::schema::Type;

        RefOr::T(Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::new(Type::String))
                .description(Some(
                    "Service definition ID - references metadata from static fixtures",
                ))
                .build(),
        ))
    }
}

impl ToSchema for Box<dyn ServiceDefinition> {
    fn name() -> Cow<'static, str> {
        Cow::Borrowed("ServiceDefinitionId")
    }
}

#[derive(Default, PartialEq, Eq, Hash, Clone)]
pub struct DefaultServiceDefinition;

impl ServiceDefinition for DefaultServiceDefinition {
    fn name(&self) -> &'static str {
        "Missing Service"
    }
    fn description(&self) -> &'static str {
        "If you are seeing this, a service definition was removed. Please create an issue."
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Unknown
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtualization_managers_declare_role_and_variant() {
        // (service id, role type == "vms"/"containers"/"identities", variant serde tag)
        let cases = [
            ("Proxmox VE", "vms", "Proxmox"),
            ("vCenter", "vms", "VCenter"),
            ("ESXi", "vms", "ESXi"),
            ("Docker", "containers", "Docker"),
            ("Podman", "containers", "Podman"),
            ("Network Identities", "identities", "NetworkIdentity"),
        ];
        for (id, role_type, variant) in cases {
            let def = ServiceDefinitionRegistry::find_by_id(id)
                .unwrap_or_else(|| panic!("{id} not registered"));
            let role = def
                .virtualization_role()
                .unwrap_or_else(|| panic!("{id} should declare a virtualization role"));
            assert_eq!(<&'static str>::from(&role), role_type, "{id} role type");
            assert_eq!(role.variant_tag(), variant, "{id} variant");
            // A runtime's container services and container hosts name the same runtime.
            if let VirtualizationRole::ContainerRuntime { services, hosts } = role {
                assert_eq!(
                    <&'static str>::from(services),
                    <&'static str>::from(hosts),
                    "{id} runtime variants"
                );
            }
        }
    }

    #[test]
    fn non_virtualizer_services_have_no_role() {
        let def = ServiceDefinitionRegistry::find_by_id("Termix").expect("Termix registered");
        assert!(def.virtualization_role().is_none());
    }
}
