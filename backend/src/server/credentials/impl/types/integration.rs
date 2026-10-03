use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::{
    services::{
        definitions::{
            docker_daemon::Docker, gnmi::Gnmi, instant_on::InstantOn, podman::Podman,
            proxmox::Proxmox, snmp::Snmp, ssh::Ssh, unifi_controller::UnifiController,
            wake_on_lan::WakeOnLan,
        },
        r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt},
    },
    shared::{
        concepts::Concept,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
};

use super::{CredentialCategory, CredentialTypeDiscriminants};

/// The integration a credential type connects to: one per associated service, shared by all of
/// that service's transports (SNMP v1/v2c/v3 are three transports of the `Snmp` integration).
///
/// The single source of the type → integration mapping
/// ([`CredentialTypeDiscriminants::integration`]); the associated service, category, fallback
/// icon, discovery text and docs guide all derive from it. The id is an identifier-safe code name
/// rather than the service's display name, so it can key `meta_credential_integrations_*`
/// messages.
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
pub enum CredentialIntegration {
    Snmp,
    Gnmi,
    Docker,
    Podman,
    UnifiController,
    InstantOn,
    Ssh,
    WakeOnLan,
    Proxmox,
}

impl CredentialTypeDiscriminants {
    /// The integration this credential type is a transport of. Exhaustive (no wildcard): a new
    /// credential variant cannot compile until it names its integration.
    pub fn integration(&self) -> CredentialIntegration {
        match self {
            Self::SnmpV1 | Self::SnmpV2c | Self::SnmpV3 => CredentialIntegration::Snmp,
            Self::Gnmi => CredentialIntegration::Gnmi,
            Self::DockerProxy | Self::DockerSocket => CredentialIntegration::Docker,
            Self::PodmanProxy | Self::PodmanSocket => CredentialIntegration::Podman,
            Self::UnifiApiKey | Self::UnifiLocalAdmin => CredentialIntegration::UnifiController,
            Self::InstantOnAccount => CredentialIntegration::InstantOn,
            Self::SshPassword | Self::SshKey => CredentialIntegration::Ssh,
            Self::WakeOnLan => CredentialIntegration::WakeOnLan,
            Self::ProxmoxApiToken => CredentialIntegration::Proxmox,
        }
    }
}

impl CredentialIntegration {
    /// The ServiceDefinition this integration discovers, used for its name, logo and color.
    pub fn service(&self) -> Box<dyn ServiceDefinition> {
        match self {
            Self::Snmp => Box::new(Snmp),
            Self::Gnmi => Box::new(Gnmi),
            Self::Docker => Box::new(Docker),
            Self::Podman => Box::new(Podman),
            Self::UnifiController => Box::new(UnifiController),
            Self::InstantOn => Box::new(InstantOn),
            Self::Ssh => Box::new(Ssh),
            Self::WakeOnLan => Box::new(WakeOnLan),
            Self::Proxmox => Box::new(Proxmox),
        }
    }

    pub fn credential_category(&self) -> CredentialCategory {
        match self {
            Self::Snmp | Self::Gnmi => CredentialCategory::NetworkMonitoring,
            Self::Docker | Self::Podman | Self::Proxmox => {
                CredentialCategory::ContainerVirtualization
            }
            Self::UnifiController | Self::InstantOn => CredentialCategory::NetworkController,
            Self::Ssh | Self::WakeOnLan => CredentialCategory::HostManagement,
        }
    }

    /// Canonical "what's discovered" for this integration, shared by all of its transports.
    /// The per-transport credential description
    /// ([`full_description`](CredentialTypeDiscriminants::full_description)), the picker's
    /// integration row, and the `integrations` fixture all derive from this.
    ///
    /// # Writing these
    ///
    /// A credential description answers exactly two questions, and nothing else:
    /// **what it discovers** (here) and **how it connects**
    /// ([`transport_note`](CredentialTypeDiscriminants::transport_note)). Every arm in both
    /// functions reads the same way, because they are rendered side by side in the credential
    /// picker and a longer one does not look more capable — it looks like the odd one out.
    ///
    /// Three things that do not belong:
    ///
    /// - **Setup instructions.** Which account to create, what role it needs, whether MFA has to
    ///   be off — that is field help text, next to the field it applies to
    ///   ([`field_definitions`](super::CredentialType::field_definitions)). Repeating it here
    ///   makes the picker a wall of prose the user has to read before they can even choose.
    /// - **What the integration does *not* do.** "Without enabling SNMP", "no agent required",
    ///   "does not modify anything" — an absence is not a capability, and it invites the reader
    ///   to wonder what else it might not do. State what it collects.
    /// - **Selling points.** The picker is for someone who has already decided to connect this
    ///   thing and now needs to know what they will get and what it will ask them for.
    ///
    /// A compatibility caveat *is* allowed in the transport note when it changes which option
    /// the user can pick — UniFi's "requires UniFi OS; the legacy Network Application does not
    /// support API keys" is the model, because it decides between two transports.
    pub fn discovers(&self) -> &'static str {
        match self {
            Self::Snmp => "Discover a host's interfaces, system details, and CDP/LLDP neighbors.",
            Self::Gnmi => "Discover a host's interfaces and LLDP neighbors over gNMI (OpenConfig).",
            Self::Docker => "Discover Docker containers and the services they expose.",
            Self::Podman => "Discover Podman containers and the services they expose.",
            Self::UnifiController => {
                "Discover UniFi-managed switches, access points and gateways, their ports, and the LLDP neighbors and uplinks the controller sees."
            }
            Self::InstantOn => {
                "Discover Instant On switches, access points and gateways, their ports, the uplinks between them, and the MACs attached to each port."
            }
            Self::Ssh => {
                "Run your own script on a host and record the system details and interfaces it reports."
            }
            Self::WakeOnLan => "Wake sleeping hosts before a scan so they are discovered.",
            Self::Proxmox => {
                "Discover Proxmox VE nodes and the VMs and LXC containers on each, with their addresses."
            }
        }
    }

    /// Path to this integration's documentation guide, relative to the site root and with a
    /// trailing slash.
    ///
    /// Exhaustive (no wildcard): a new integration cannot compile until it has a guide to point
    /// at. The website's `check-integration-guides.mjs` asserts each path resolves to the guide
    /// whose `integration` frontmatter names this service, so the two declarations cannot drift
    /// apart.
    ///
    /// A path rather than a URL: the website consumes it root-relative, and the app composes it
    /// onto the public docs base itself.
    pub fn docs_path(&self) -> &'static str {
        match self {
            Self::Snmp => "/docs/guides/integrations/snmp/",
            Self::Docker => "/docs/guides/integrations/docker/",
            Self::Podman => "/docs/guides/integrations/podman/",
            Self::UnifiController => "/docs/guides/integrations/unifi/",
            Self::InstantOn => "/docs/guides/integrations/instant-on/",
            Self::Gnmi => "/docs/guides/integrations/gnmi/",
            Self::Ssh => "/docs/guides/integrations/ssh/",
            Self::WakeOnLan => "/docs/guides/integrations/wake-on-lan/",
            Self::Proxmox => "/docs/guides/integrations/proxmox/",
        }
    }

    /// File extension of the service logo, matching the downloaded
    /// `logos/services/{slug}.{ext}`. Empty when the logo is missing or served locally.
    pub fn logo_ext(&self) -> &'static str {
        let url = self.service().logo_url();
        if url.is_empty() || url.starts_with('/') {
            return "";
        }
        url.rsplit('.')
            .next()
            .and_then(|e| e.split('?').next())
            .filter(|e| matches!(*e, "svg" | "png" | "webp"))
            .unwrap_or("svg")
    }
}

impl HasId for CredentialIntegration {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for CredentialIntegration {
    fn color(&self) -> Color {
        ServiceDefinition::category(&*self.service()).color()
    }

    fn icon(&self) -> Icon {
        // Fallback only — the service logo is what normally renders.
        match self {
            Self::Snmp => Concept::SNMP.icon(),
            Self::Gnmi | Self::UnifiController | Self::InstantOn => Concept::L2.icon(),
            Self::Docker | Self::Podman => Concept::Containerization.icon(),
            Self::Proxmox => Concept::Virtualization.icon(),
            Self::Ssh => Icon::SquareTerminal,
            Self::WakeOnLan => Icon::Power,
        }
    }
}

impl TypeMetadataProvider for CredentialIntegration {
    fn name(&self) -> &'static str {
        ServiceDefinition::name(&*self.service())
    }

    fn description(&self) -> &'static str {
        self.discovers()
    }

    fn category(&self) -> &'static str {
        self.credential_category().into()
    }

    fn metadata(&self) -> serde_json::Value {
        let service = self.service();
        serde_json::json!({
            "docs_path": self.docs_path(),
            // The logo is named after the service, as for credential types.
            "associated_service": ServiceDefinition::name(&*service),
            "has_logo": service.has_logo(),
            "logo_ext": self.logo_ext(),
            "logo_needs_white_background": service.logo_needs_white_background(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    /// Every integration is reached by at least one credential type, so the picker never shows an
    /// integration row with nothing to expand into.
    #[test]
    fn every_integration_has_a_credential_type() {
        for integration in CredentialIntegration::iter() {
            assert!(
                CredentialTypeDiscriminants::iter().any(|d| d.integration() == integration),
                "{integration:?} has no credential type"
            );
        }
    }

    /// Integration ids key `meta_credential_integrations_<id>_*` messages, so they must be
    /// identifier-safe. Service names ("Wake-on-LAN", "UniFi Controller") are not, which is why
    /// the id is the variant name rather than the service name.
    #[test]
    fn integration_ids_are_identifier_safe() {
        for integration in CredentialIntegration::iter() {
            let id = integration.id();
            assert!(
                id.chars().all(|c| c.is_ascii_alphanumeric()),
                "integration id {id:?} cannot key a message"
            );
        }
    }

    /// Transports of one integration share its service, so grouping by integration and grouping
    /// by associated service (what the website `integrations.json` does) agree.
    #[test]
    fn integration_and_service_grouping_agree() {
        for a in CredentialTypeDiscriminants::iter() {
            for b in CredentialTypeDiscriminants::iter() {
                let same_service = ServiceDefinition::name(&*a.integration().service())
                    == ServiceDefinition::name(&*b.integration().service());
                assert_eq!(
                    a.integration() == b.integration(),
                    same_service,
                    "{a:?} and {b:?} disagree"
                );
            }
        }
    }
}
