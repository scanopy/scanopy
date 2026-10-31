use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::{
    discovery::r#impl::collection::CollectedData,
    hosts::r#impl::virtualization::HostVirtualizationDiscriminants,
    services::{
        definitions::{
            docker_daemon::Docker, gnmi::Gnmi, instant_on::InstantOn, podman::Podman,
            proxmox::Proxmox, snmp::Snmp, ssh::Ssh, unifi_controller::UnifiController,
            wake_on_lan::WakeOnLan,
        },
        r#impl::{definitions::ServiceDefinition, patterns::ClientProbe},
    },
    shared::{
        attribution::AttributeSource,
        concepts::Concept,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
};

use super::{CredentialCategory, CredentialTypeDiscriminants, Target};

/// The integration a credential type connects to: one per associated service, shared by all of
/// that service's transports (SNMP v1/v2c/v3 are three transports of the `Snmp` integration).
///
/// The single source of the type → integration mapping
/// ([`CredentialTypeDiscriminants::integration`]); the associated service, category, fallback
/// icon, discovery text and docs guide all derive from it. Its display name is the service's
/// unless [`TypeMetadataProvider::name`] says otherwise. The id is an identifier-safe code name
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

/// How much of a device's interface list an integration can see when it succeeds.
///
/// Deliberately not the same fact as `interfaces_complete`, which says whether *this* collection
/// finished. This says what a finished collection covers, and it is a property of the protocol,
/// not of the run: a UniFi `port_table` is a complete list of physical ports and still omits the
/// VLAN, loopback and CPU interfaces the same switch reports over SNMP, on every successful sync.
///
/// Several integrations can collect one host in a single scan, sharing one `HostData`. Before
/// this existed, the last one to call the old `replace_interfaces` simply overwrote the others,
/// and the only brake was a hand-written `if !host_data.interfaces.is_empty()` in two of them —
/// which made the answer depend on dispatch order and read "richer" as "non-empty".
///
/// Ordered: `NoInterfaces` < `PhysicalPortsOnly` < `FullIfTable`. The order breaks ties *per
/// interface*, never for the set as a whole — see `HostData::contribute_interfaces`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterfaceViewScope {
    /// Contributes no interfaces at all — the container integrations.
    NoInterfaces,
    /// Every physical port, and nothing else. A controller API reporting the ports it manages.
    PhysicalPortsOnly,
    /// The device's whole ifTable, virtual interfaces included. An SNMP walk.
    FullIfTable,
}

impl CredentialTypeDiscriminants {
    /// The integration this credential type is a transport of. Exhaustive (no wildcard): a new
    /// credential variant cannot compile until it names its integration.
    pub fn integration(&self) -> CredentialIntegration {
        match self {
            Self::SnmpCommunity | Self::SnmpV3 => CredentialIntegration::Snmp,
            Self::Gnmi => CredentialIntegration::Gnmi,
            Self::DockerProxy | Self::DockerSocket => CredentialIntegration::Docker,
            Self::PodmanProxy | Self::PodmanSocket => CredentialIntegration::Podman,
            Self::UnifiApiKey | Self::UnifiLocalAdmin => CredentialIntegration::UnifiController,
            Self::InstantOnAccount => CredentialIntegration::InstantOn,
            Self::SshPassword | Self::SshKey | Self::SshScript => CredentialIntegration::Ssh,
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

    /// The integration whose service is this definition. At most one: no two integrations share
    /// a service (`integration_and_service_grouping_agree`).
    pub fn for_service(definition: &dyn ServiceDefinition) -> Option<Self> {
        let name = ServiceDefinition::name(definition);
        Self::iter().find(|i| ServiceDefinition::name(&*i.service()) == name)
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

    /// How much of a host's interface list this integration sees when it succeeds. Shared by all
    /// of its transports.
    ///
    /// Exhaustive (no wildcard): several integrations can collect one host in one scan and this
    /// decides whose row survives where they disagree, so a new integration must say which it is
    /// rather than inherit an answer.
    pub fn interface_view_scope(&self) -> InterfaceViewScope {
        match self {
            // An ifTable walk is the device's own account of every interface it has, and gNMI's
            // `/interfaces/interface` has the same standing.
            Self::Snmp | Self::Gnmi => InterfaceViewScope::FullIfTable,
            // A UniFi `port_table` is every physical port and nothing else: the same switch under
            // SNMP also reports the VLAN, loopback and CPU interfaces UniFi never mentions. The
            // Instant On portal reports switch ports and AP uplinks, not a device's whole ifTable.
            Self::UnifiController | Self::InstantOn => InterfaceViewScope::PhysicalPortsOnly,
            // A script reports the interfaces its author chose to print. Narrower than an ifTable
            // walk, so SNMP's row wins where both describe one interface.
            Self::Ssh => InterfaceViewScope::PhysicalPortsOnly,
            // Container runtimes report networks and containers, never the host's ports.
            Self::Docker | Self::Podman => InterfaceViewScope::NoInterfaces,
            // The node's own NICs come from SNMP or SSH; the Proxmox API's view of them is the
            // Linux bridge config, not an interface table.
            Self::Proxmox => InterfaceViewScope::NoInterfaces,
            Self::WakeOnLan => InterfaceViewScope::NoInterfaces,
        }
    }

    /// Whether a successful collection contributes interfaces to the hosts it reaches. Gates the
    /// onboarding prompts that point at interface data, and names the integrations those prompts
    /// list.
    pub fn reports_interfaces(&self) -> bool {
        self.interface_view_scope() != InterfaceViewScope::NoInterfaces
    }

    /// Whether a scan that finds this integration's service on a host with no credential for it
    /// raises `IntegrationWithoutCredential`.
    ///
    /// Exhaustive (no wildcard), so a new integration has to say. SSH runs a script the user
    /// writes, so finding sshd (on nearly every Linux host) says nothing about wanting one.
    /// Wake-on-LAN has no service a scan can find.
    pub fn prompts_when_found(&self) -> bool {
        match self {
            Self::Snmp
            | Self::Gnmi
            | Self::Docker
            | Self::Podman
            | Self::UnifiController
            | Self::InstantOn
            | Self::Proxmox => true,
            Self::Ssh | Self::WakeOnLan => false,
        }
    }

    /// Whether a credential of this integration can apply to a whole site, so a prompt for it
    /// offers the site rather than the hosts. Derived from the transports' own `targets()`, which
    /// is what the create form validates against.
    ///
    /// A site-wide credential is tried on every host and pinned where it works, which is what a
    /// prompt wants for SNMP on a mixed estate: nothing tells which community or version a device
    /// accepts until one answers.
    pub fn applies_to_sites(&self) -> bool {
        CredentialTypeDiscriminants::iter()
            .filter(|t| t.integration() == *self)
            .any(|t| t.to_credential_type().targets().contains(&Target::Site))
    }

    /// The kinds of host virtualization this integration reports: the relationships it records
    /// between a host and what runs it.
    ///
    /// Exhaustive (no wildcard), so a new integration cannot compile without answering. Every
    /// host an integration submits is checked against this (`undeclared_virtualization`), which
    /// fails tests and debug builds when the two disagree. The docs list which integrations
    /// report which relationship from it, through the integrations fixture.
    pub fn host_virtualizations(&self) -> &'static [HostVirtualizationDiscriminants] {
        match self {
            Self::Proxmox => &[
                HostVirtualizationDiscriminants::Proxmox,
                HostVirtualizationDiscriminants::NetworkIdentity,
            ],
            Self::Docker => &[HostVirtualizationDiscriminants::Docker],
            Self::Podman => &[HostVirtualizationDiscriminants::Podman],
            Self::Snmp
            | Self::Gnmi
            | Self::UnifiController
            | Self::InstantOn
            | Self::Ssh
            | Self::WakeOnLan => &[],
        }
    }

    /// Whether any of this integration's credential types has a seeded default
    /// ([`CredentialTypeDiscriminants::seeded_default`]), so every org holds one from the start.
    pub fn seeded_by_default(&self) -> bool {
        CredentialTypeDiscriminants::iter()
            .any(|t| t.integration() == *self && t.seeded_default().is_some())
    }

    /// The sources this integration writes, each with the categories it writes from that source.
    /// Shared by all of its transports.
    ///
    /// One entry per source rather than one list per integration, because the same category can
    /// arrive from several of an integration's sources with different reach: a Proxmox guest's
    /// hostname comes from its config or from its guest agent. A derived source is listed where
    /// the integration's own data is its whole basis, such as the OS SNMP's system description
    /// implies.
    ///
    /// Exhaustive (no wildcard), so a new integration cannot compile without answering. What an
    /// integration submits is checked against this (`undeclared_collection`), which fails tests
    /// and debug builds when the two disagree. The security page lists what each integration
    /// collects from it, through the `collected-data` and `integrations` fixtures.
    /// Whether this integration runs the credential-free application probes against what it
    /// reaches, as the container scan does against each container's ports. Such an integration
    /// may submit whatever those probes collect, without listing each probe in [`Self::collection`].
    /// Exhaustive, for the reason `collection` is.
    pub fn runs_app_probes(&self) -> bool {
        match self {
            Self::Docker | Self::Podman => true,
            Self::Snmp
            | Self::Gnmi
            | Self::UnifiController
            | Self::InstantOn
            | Self::Ssh
            | Self::WakeOnLan
            | Self::Proxmox => false,
        }
    }

    pub fn collection(&self) -> &'static [(AttributeSource, &'static [CollectedData])] {
        use AttributeSource as S;
        use ClientProbe as P;
        use CollectedData::*;
        match self {
            // sys* and ENTITY-MIB details, the ifTable with LLDP/CDP neighbors, FDB and per-port
            // VLANs, ipAddrTable addresses, routes as subnets, the SNMP service itself, and ARP
            // entries as new hosts.
            Self::Snmp => &[
                (
                    S::Probe(P::Snmp),
                    &[
                        Addresses,
                        PortsAndServices,
                        DeviceDetails,
                        Interfaces,
                        Neighbors,
                        ForwardingTables,
                        Vlans,
                        Subnets,
                    ],
                ),
                (S::Authored(P::Snmp), &[DeviceDetails]),
                (S::ForwardingTable, &[Addresses, ConnectedClients]),
                (S::SysDescrMatch, &[OperatingSystem]),
                (S::SysObjectIdMatch, &[OperatingSystem]),
            ],
            // Interfaces and LLDP neighbors, and the gNMI port its probe answered on. The chassis
            // id it reads is an identifier.
            Self::Gnmi => &[(
                S::Probe(P::Gnmi),
                &[PortsAndServices, Interfaces, Neighbors],
            )],
            // Managed devices with their identity, ports, FDB, uplinks, per-port VLANs and the
            // service matched from their device type; clients as hosts, with UniFi's DHCP
            // hostnames; the controller's networks as subnets and VLANs, named as an administrator
            // named them.
            Self::UnifiController => &[
                (
                    S::Probe(P::UnifiController),
                    &[
                        Addresses,
                        Hostnames,
                        PortsAndServices,
                        DeviceDetails,
                        Interfaces,
                        Neighbors,
                        ForwardingTables,
                        Vlans,
                        Subnets,
                        ConnectedClients,
                    ],
                ),
                (S::Authored(P::UnifiController), &[Vlans]),
            ],
            // As UniFi, except the portal reports no client hostnames.
            Self::InstantOn => &[(
                S::Probe(P::InstantOn),
                &[
                    Addresses,
                    PortsAndServices,
                    DeviceDetails,
                    Interfaces,
                    Neighbors,
                    ForwardingTables,
                    ConnectedClients,
                ],
            )],
            Self::Ssh => &[(
                S::SshScript,
                &[Hostnames, OperatingSystem, DeviceDetails, Interfaces],
            )],
            // The engine host's OS, its bridge networks, container services and published ports,
            // endpoint addresses, and macvlan/ipvlan containers as hosts of their own, whose MAC
            // the runtime assigned.
            Self::Docker => &[
                (
                    S::Probe(P::Docker),
                    &[Addresses, PortsAndServices, Subnets, Containers],
                ),
                (S::ContainerRuntimeInfo, &[OperatingSystem]),
                (S::HypervisorConfig, &[Addresses]),
            ],
            Self::Podman => &[
                (
                    S::Probe(P::Podman),
                    &[Addresses, PortsAndServices, Subnets, Containers],
                ),
                (S::ContainerRuntimeInfo, &[OperatingSystem]),
                (S::HypervisorConfig, &[Addresses]),
            ],
            // Nodes with their hostname, OS, addresses and the Proxmox VE service, and guests with
            // addresses and their network identities. A guest's config sets an LXC container's
            // hostname and distribution and every guest NIC's MAC; the guest agent reads a VM's
            // hostname, OS and addresses.
            Self::Proxmox => &[
                (
                    S::Probe(P::Proxmox),
                    &[
                        Addresses,
                        Hostnames,
                        OperatingSystem,
                        PortsAndServices,
                        Interfaces,
                        VirtualMachines,
                    ],
                ),
                (
                    S::HypervisorConfig,
                    &[Addresses, Hostnames, OperatingSystem, Interfaces],
                ),
                (S::GuestAgent, &[Addresses, Hostnames, OperatingSystem]),
            ],
            Self::WakeOnLan => &[],
        }
    }

    /// The categories this integration collects, in display order. Derived from
    /// [`Self::collection`].
    pub fn collects(&self) -> Vec<CollectedData> {
        let categories: BTreeSet<CollectedData> = self
            .collection()
            .iter()
            .flat_map(|(_, categories)| categories.iter().copied())
            .collect();
        categories.into_iter().collect()
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
    /// The service's name, except where owners know the product by its vendor: the Instant On
    /// service stays "Instant On" on host badges, but the integration names Aruba so the picker
    /// and plan cards say whose portal it connects to.
    fn name(&self) -> &'static str {
        match self {
            Self::InstantOn => "Aruba Instant On",
            Self::Snmp
            | Self::Gnmi
            | Self::Docker
            | Self::Podman
            | Self::UnifiController
            | Self::Ssh
            | Self::WakeOnLan
            | Self::Proxmox => ServiceDefinition::name(&*self.service()),
        }
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
            "reports_interfaces": self.reports_interfaces(),
            "associated_service": ServiceDefinition::name(&*service),
            "logo": service.logo(),
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
