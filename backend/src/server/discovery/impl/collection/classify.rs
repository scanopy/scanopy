//! How each provenance source is collected, and which producer may write it.

use strum::IntoEnumIterator;

use crate::server::{
    credentials::r#impl::types::CredentialIntegration,
    services::r#impl::patterns::ClientProbe,
    shared::{attribution::AttributeSource, types::metadata::HasId},
};

use super::CollectedData;

/// How a source of collected data runs, and what it collects when it runs with no credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCollection {
    /// Tried on every host the daemon reaches, with no credential. Lists the categories this
    /// source collects.
    CredentialFree(&'static [CollectedData]),
    /// Runs only through an integration credential. Which categories it collects is the
    /// integration's declaration ([`CredentialIntegration::collection`]).
    Credentialed,
    /// Inferred from data already collected, so it adds no collection of its own and is never
    /// charged to the producer that wrote it.
    Derived,
    /// Not a collection method: a value nothing attributed, or one a person entered.
    NotCollected,
}

impl AttributeSource {
    /// How this source collects. Exhaustive (no wildcard), so a new source cannot compile until it
    /// says whether it needs a credential and, if not, what it collects.
    pub fn collection(&self) -> SourceCollection {
        use CollectedData::*;
        use SourceCollection as C;
        match self {
            Self::Unspecified | Self::Manual => C::NotCollected,

            // Readings Scanopy derives from something it already collected. A CIP vendor ID is the
            // EtherNet/IP reply's own field, turned into a manufacturer string, so the EtherNet/IP
            // probe is the method.
            Self::OwnAddress
            | Self::ServiceMatch
            | Self::LldpNeighbourAddress
            | Self::InterfaceNameMatch
            | Self::CipVendorId
            | Self::SysDescrMatch
            | Self::SysObjectIdMatch
            | Self::LldpSysDescMatch
            | Self::LldpChassisId => C::Derived,

            // An operating system, and the device's maker, model and class, read from what a
            // service or an mDNS record says about the host.
            Self::SshBannerMatch
            | Self::HttpServerMatch
            | Self::X509SubjectMatch
            | Self::X509IssuerMatch
            | Self::HtmlTitleMatch
            | Self::HttpWwwAuthMatch
            | Self::HttpCookieMatch
            | Self::FaviconMatch
            | Self::FtpBannerMatch
            | Self::TelnetBannerMatch
            | Self::RtspServerMatch
            | Self::DnsSdDeviceInfoMatch
            | Self::DnsSdAirPlayMatch => C::CredentialFree(&[OperatingSystem, DeviceDetails]),
            // An operating system family read from the host's network stack.
            Self::TcpStackMatch => C::CredentialFree(&[OperatingSystem]),

            Self::ReverseDns | Self::DnsSdInstanceName => C::CredentialFree(&[Hostnames]),
            // An mDNS answer names the host and also proves the address is live.
            Self::DnsSdHostname => C::CredentialFree(&[Addresses, Hostnames]),
            // A port a device advertised, which names the service listening on it.
            Self::DnsSdServicePort => C::CredentialFree(&[PortsAndServices]),
            Self::ArpReply | Self::IcmpEcho => C::CredentialFree(&[Addresses]),
            // A port that answered, and the address of a host found only by its open ports.
            Self::PortScan => C::CredentialFree(&[Addresses, PortsAndServices]),
            // A PROFINET station reports its station name, which names the device as an SNMP
            // sysName does, and the one interface it answered on.
            Self::ProfinetDcp => C::CredentialFree(&[DeviceDetails, Interfaces]),
            // The neighbours lldpd on the daemon's own host has heard.
            Self::Lldpd => C::CredentialFree(&[Neighbors]),
            // The daemon reading its own host: hostname, OS, addresses, NICs, the daemon's own
            // service and the subnets it sits on.
            Self::DaemonSelfReport => C::CredentialFree(&[
                Addresses,
                Hostnames,
                OperatingSystem,
                PortsAndServices,
                Interfaces,
                Subnets,
            ]),

            Self::ForwardingTable
            | Self::ContainerRuntimeInfo
            | Self::HypervisorConfig
            | Self::GuestAgent
            | Self::SshScript => C::Credentialed,

            Self::Probe(probe) => probe.collection(),
            // `AttributeSource::all` enumerates `Authored` for every probe, but a person can only
            // set a value the daemon reads back through an integration. The application probes
            // read nothing a person typed.
            Self::Authored(probe) => match probe.collection() {
                C::Credentialed => C::Credentialed,
                C::CredentialFree(_) | C::Derived | C::NotCollected => C::NotCollected,
            },
        }
    }
}

impl ClientProbe {
    /// How this probe collects. Exhaustive (no wildcard), for the reason
    /// [`AttributeSource::collection`] is.
    pub fn collection(&self) -> SourceCollection {
        use CollectedData::*;
        use SourceCollection as C;
        match self {
            // An integration's own API or protocol session, opened with its credential.
            Self::Docker
            | Self::Podman
            | Self::Snmp
            | Self::Gnmi
            | Self::UnifiController
            | Self::InstantOn
            | Self::Proxmox
            | Self::Libvirt => C::Credentialed,

            // Industrial protocols report the device's identity as well as confirming the port.
            Self::ModbusTcp | Self::EtherNetIp => {
                C::CredentialFree(&[PortsAndServices, DeviceDetails])
            }
            // Spotify Connect's `getInfo` names the brand and model.
            Self::SpotifyConnect => C::CredentialFree(&[PortsAndServices, DeviceDetails]),
            // Google Wifi's status API names the manufacturer, board and firmware.
            Self::GoogleWifi => C::CredentialFree(&[PortsAndServices, DeviceDetails]),
            // The SMB negotiation states the Windows release.
            Self::Smb => C::CredentialFree(&[PortsAndServices, OperatingSystem]),
            // Lockdown's `GetValue` names the product type and OS version.
            Self::AppleLockdown => {
                C::CredentialFree(&[PortsAndServices, DeviceDetails, OperatingSystem])
            }
            // Presence probes: the protocol answered on the port, which identifies the service.
            Self::OpcUa
            | Self::Sip
            | Self::Ssh
            | Self::Ftp
            | Self::Telnet
            | Self::Vnc
            | Self::Rtsp
            | Self::Nut
            | Self::EpicsCa
            | Self::ZabbixAgent
            | Self::CheckMkAgent
            | Self::Ldap
            | Self::Kerberos
            | Self::MySql
            | Self::PostgreSql
            | Self::MsSql
            | Self::MongoDb
            | Self::Redis
            | Self::Cassandra
            | Self::Kafka
            | Self::Amqp
            | Self::Mqtt
            | Self::OracleTns
            | Self::Rdp
            | Self::Nfs
            | Self::DnsTcp
            | Self::DockerSwarm
            | Self::Tls
            | Self::Ike
            | Self::OpenVpn
            | Self::Zmtp
            | Self::Bacula
            | Self::BeszelAgent
            | Self::H323
            | Self::SnmpEngine
            | Self::DockerPing
            | Self::PodmanPing
            | Self::Ipp
            | Self::Lpd
            | Self::Smux
            | Self::Rpcbind
            | Self::Llmnr
            | Self::DnsUdp
            | Self::Ntp
            | Self::Dhcp
            | Self::Bacnet
            | Self::Tftp
            | Self::Mgcp => C::CredentialFree(&[PortsAndServices]),
        }
    }
}

/// What produced a submission: the credential-free discovery the daemon runs on every host (the
/// network scan and the daemon's report on its own host), or one integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Producer {
    CredentialFree,
    Integration(CredentialIntegration),
}

impl From<CredentialIntegration> for Producer {
    fn from(integration: CredentialIntegration) -> Self {
        Self::Integration(integration)
    }
}

impl Producer {
    /// Whether this producer declares writing `source` into `category`.
    ///
    /// A derived source is always allowed: the server or daemon inferred it from data the producer
    /// collected, and that data is checked on its own. An unattributed or person-entered source is
    /// never allowed, because a current producer must say where every value it writes came from.
    pub fn declares(&self, category: CollectedData, source: AttributeSource) -> bool {
        match source.collection() {
            SourceCollection::Derived => true,
            SourceCollection::NotCollected => false,
            SourceCollection::CredentialFree(categories) => match self {
                Self::CredentialFree => categories.contains(&category),
                // A container scan runs the same app probes as the network scan, so it may submit
                // what they collect.
                Self::Integration(integration) => {
                    integration.declares(category, source)
                        || (integration.runs_app_probes()
                            && matches!(source, AttributeSource::Probe(_))
                            && categories.contains(&category))
                }
            },
            SourceCollection::Credentialed => match self {
                // The network scan runs each integration's probe with its credential, and a probe
                // that answers confirms the port it answered on. The scan submits that port, so
                // it is accepted from the scan wherever an integration declares it.
                Self::CredentialFree => {
                    category == CollectedData::PortsAndServices
                        && matches!(source, AttributeSource::Probe(_))
                        && CredentialIntegration::iter()
                            .any(|integration| integration.declares(category, source))
                }
                Self::Integration(integration) => integration.declares(category, source),
            },
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::CredentialFree => "CredentialFree",
            Self::Integration(integration) => integration.id(),
        }
    }
}

impl CredentialIntegration {
    /// Whether this integration declares writing `source` into `category`.
    pub fn declares(&self, category: CollectedData, source: AttributeSource) -> bool {
        self.collection()
            .iter()
            .any(|(declared, categories)| *declared == source && categories.contains(&category))
    }
}

/// The credential-free sources that collect `category`, in [`AttributeSource::all`] order.
pub(super) fn credential_free_methods(category: CollectedData) -> Vec<AttributeSource> {
    AttributeSource::all()
        .into_iter()
        .filter(|source| match source.collection() {
            SourceCollection::CredentialFree(categories) => categories.contains(&category),
            SourceCollection::Credentialed
            | SourceCollection::Derived
            | SourceCollection::NotCollected => false,
        })
        .collect()
}

/// The integrations that collect `category`, in declaration order.
pub(super) fn collecting_integrations(category: CollectedData) -> Vec<CredentialIntegration> {
    CredentialIntegration::iter()
        .filter(|integration| integration.collects().contains(&category))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A container scan runs every credential-free app probe against a container's ports, so the
    /// container integrations accept what each probe collects, and no other integration does.
    /// Before this, a PostgreSQL container panicked a debug daemon on the collection check.
    #[test]
    fn container_scans_may_submit_what_any_app_probe_collects() {
        for probe in ClientProbe::iter() {
            let SourceCollection::CredentialFree(categories) = probe.collection() else {
                continue;
            };
            let source = AttributeSource::Probe(probe);
            for &category in categories {
                for integration in CredentialIntegration::iter() {
                    let declared = Producer::Integration(integration).declares(category, source);
                    if integration.runs_app_probes() {
                        assert!(declared, "{integration:?} runs {probe:?} but rejects it");
                    } else {
                        assert_eq!(
                            declared,
                            integration.declares(category, source),
                            "{integration:?} runs no app probes but accepts {probe:?}"
                        );
                    }
                }
            }
        }
    }

    /// A category with neither a method nor an integration would be listed on the security page
    /// with nothing behind it.
    #[test]
    fn every_category_has_a_method_or_an_integration() {
        for category in CollectedData::iter() {
            assert!(
                !credential_free_methods(category).is_empty()
                    || !collecting_integrations(category).is_empty(),
                "{category:?} has no method and no integration"
            );
        }
    }

    /// A source that needs a credential but that no integration declares would be accepted from
    /// no producer, and would appear nowhere on the security page.
    ///
    /// `Authored` is left out: most integrations write a person-set value only into a host's
    /// name, which is not a collected category. One that writes it into a category is held to a
    /// declaration by the check on what it submits.
    #[test]
    fn every_credentialed_source_is_declared_by_an_integration() {
        for source in AttributeSource::all() {
            if source.collection() == SourceCollection::Credentialed
                && !matches!(source, AttributeSource::Authored(_))
            {
                assert!(
                    CredentialIntegration::iter()
                        .any(|i| i.collection().iter().any(|(s, _)| *s == source)),
                    "{source} needs a credential but no integration declares it"
                );
            }
        }
    }

    /// An integration listing an unattributed or person-entered source would make its declaration
    /// accept exactly the values the check exists to catch.
    #[test]
    fn no_integration_declares_a_source_that_is_not_collection() {
        for integration in CredentialIntegration::iter() {
            for (source, _) in integration.collection() {
                assert_ne!(
                    source.collection(),
                    SourceCollection::NotCollected,
                    "{integration:?} declares {source}"
                );
            }
        }
    }
}
