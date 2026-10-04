use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::{
    organizations::r#impl::base::UseCase,
    services::r#impl::base::Service,
    shared::{
        concepts::Concept,
        entities::EntityDiscriminants,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
    topology::types::views::{FilterValueContext, HasFilterValues, MetadataFilterType},
};

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Display,
    Deserialize,
    EnumDiscriminants,
    EnumIter,
    IntoStaticStr,
    ToSchema,
)]
pub enum ServiceCategory {
    // Infrastructure (always-on, core network services)
    NetworkCore,   // DHCP, Gateway, NTP, Switch, SNMP
    NetworkAccess, // WiFi APs, switches for end devices

    // Network appliances (router/firewall/VPN platforms)
    #[serde(alias = "NetworkSecurity")]
    NetworkAppliance, // MikroTik, pfSense, OPNsense, FortiGate, Firewall

    // Remote access protocols
    RemoteAccess, // SSH, Telnet, RDP

    // Server Services
    Storage, // NAS, file servers
    Backup,
    Media,          // Plex, Jellyfin
    HomeAutomation, // Home Assistant
    #[serde(alias = "Virtualization")]
    Hypervisor, // Proxmox, ESXi, Hyper-V
    ContainerRuntime, // Docker, Podman
    Container,      // Docker containers, LXC instances
    Orchestrator,   // Kubernetes, Portainer, Rancher, Nomad

    // Network Services
    DNS, // All DNS services
    VPN, // All VPN services
    #[serde(alias = "SNMP")]
    Monitoring, // Monitoring tools, SNMP
    AdBlock,
    ReverseProxy,

    // End Devices
    Workstation, // Desktops, laptops
    Mobile,      // Phones, tablets
    IoT,         // Smart devices, sensors
    /// Operational-technology devices: PLCs, RTUs, drives, building controllers, and the
    /// gateways fronting them. Separate from `IoT` because the protocols that identify them
    /// (Modbus, EtherNet/IP, OPC UA, BACnet) and the estate they belong to are different — a
    /// steel mill's PLCs are not smart speakers.
    Industrial,
    Printer, // All printing devices

    // Applications
    Database,    // DB servers
    Development, // Dev tools, CI/CD, config management
    Dashboard,
    MessageQueue,
    IdentityAndAccess,
    Integration, // AS2/EDI, ESB, iPaaS middleware

    // Office & Productivity
    Office,            // Document editing, notes, file management
    ProjectManagement, // Task tracking, wikis, kanban boards

    // Communication
    Messaging,    // Team chat (text-based)
    Conferencing, // Video/audio meetings
    Telephony,    // VoIP/PBX infrastructure
    Email,        // Email servers

    // Content
    Publishing, // CMS, blogs, forums

    // Special
    Unknown,
    Custom,
    Scanopy,
    OpenPorts,
    /// Addresses and MACs a host presents beyond its own NICs (the Network Identities service).
    NetworkIdentities,
}

/// Deserialize a list of service categories, dropping any this build does not know.
///
/// `ServiceCategory` has no `#[serde(other)]` fallback, and its text reaches the database inside
/// `topologies.options` JSONB via `ElementRule::ByServiceCategory`. Adding a variant is safe going
/// forward; **rolling back is not** — an older binary reading a rule that names a category it was
/// built without would fail the whole topology read, not just that entry.
///
/// So an unknown entry is dropped and the rest of the rule survives, which mirrors what
/// `SubnetStorage` already does with an unrecognised `SubnetCidrSource`. Deliberately narrow: a
/// lenient `Deserialize` on the enum itself would also make the API silently accept a misspelled
/// category on write, and a rule that quietly does nothing is worse than a rejected request.
pub fn deserialize_known_categories<'de, D>(
    deserializer: D,
) -> Result<Vec<ServiceCategory>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Vec::<serde_json::Value>::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .filter_map(
            |value| match serde_json::from_value::<ServiceCategory>(value.clone()) {
                Ok(category) => Some(category),
                Err(_) => {
                    tracing::warn!(
                        %value,
                        "Unrecognized service category in a grouping rule; dropping it"
                    );
                    None
                }
            },
        )
        .collect())
}

impl HasId for ServiceCategory {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for ServiceCategory {
    fn icon(&self) -> Icon {
        match self {
            // Infrastructure (always-on, core network services)
            ServiceCategory::NetworkCore => Icon::Network,
            ServiceCategory::NetworkAccess => Icon::Router,
            ServiceCategory::NetworkAppliance => Icon::BrickWall,
            ServiceCategory::RemoteAccess => Icon::Terminal,

            // Server Services
            ServiceCategory::Storage => Icon::HardDrive,
            ServiceCategory::Media => Icon::CirclePlay,
            ServiceCategory::HomeAutomation => Icon::House,
            ServiceCategory::Hypervisor => Concept::Virtualization.icon(),
            ServiceCategory::ContainerRuntime => Concept::Containerization.icon(),
            ServiceCategory::Container => Concept::Containerization.icon(),
            ServiceCategory::Orchestrator => Icon::Layers,
            ServiceCategory::Backup => Icon::DatabaseBackup,

            // Network Services
            ServiceCategory::DNS => Concept::Dns.icon(),
            ServiceCategory::VPN => Concept::Vpn.icon(),
            ServiceCategory::Monitoring => Icon::Activity,
            ServiceCategory::AdBlock => Icon::ShieldCheck,
            ServiceCategory::ReverseProxy => Concept::ReverseProxy.icon(),

            // End devices
            ServiceCategory::Workstation => Icon::Monitor,
            ServiceCategory::Mobile => Icon::Smartphone,
            ServiceCategory::IoT => Concept::IoT.icon(),
            ServiceCategory::Industrial => Icon::Factory,
            ServiceCategory::Printer => Icon::Printer,

            // Applications
            ServiceCategory::Database => Icon::Database,
            ServiceCategory::Development => Icon::Code,
            ServiceCategory::MessageQueue => Icon::MessageSquareCode,
            ServiceCategory::Dashboard => Icon::LayoutDashboard,
            ServiceCategory::IdentityAndAccess => Icon::KeyRound,
            ServiceCategory::Integration => Icon::Workflow,

            // Office & Productivity
            ServiceCategory::Office => Icon::FileText,
            ServiceCategory::ProjectManagement => Icon::SquareKanban,

            // Communication
            ServiceCategory::Messaging => Icon::MessageCircle,
            ServiceCategory::Conferencing => Icon::Video,
            ServiceCategory::Telephony => Icon::Phone,
            ServiceCategory::Email => Icon::Mail,

            // Content
            ServiceCategory::Publishing => Icon::PenLine,

            // Special
            ServiceCategory::Scanopy => Icon::Zap,
            ServiceCategory::Custom => Icon::Sparkle,
            ServiceCategory::OpenPorts => EntityDiscriminants::Port.icon(),
            ServiceCategory::NetworkIdentities => Icon::FingerprintPattern,
            ServiceCategory::Unknown => Icon::CircleQuestionMark,
        }
    }

    fn color(&self) -> Color {
        match self {
            // Infrastructure (always-on, core network services)
            ServiceCategory::NetworkCore => Color::Yellow,
            ServiceCategory::NetworkAccess => Color::Green,
            ServiceCategory::NetworkAppliance => Color::Red,
            ServiceCategory::RemoteAccess => Color::Amber,

            // Server Services
            ServiceCategory::Storage => Color::Green,
            ServiceCategory::Media => Color::Blue,
            ServiceCategory::HomeAutomation => Color::Blue,
            ServiceCategory::Hypervisor => Concept::Virtualization.color(),
            ServiceCategory::ContainerRuntime => Concept::Containerization.color(),
            ServiceCategory::Container => Concept::Containerization.color(),
            ServiceCategory::Orchestrator => Color::Purple,
            ServiceCategory::Backup => Color::Gray,

            // Network Services
            ServiceCategory::DNS => Concept::Dns.color(),
            ServiceCategory::VPN => Concept::Vpn.color(),
            ServiceCategory::Monitoring => Color::Orange,
            ServiceCategory::AdBlock => Concept::Dns.color(),
            ServiceCategory::ReverseProxy => Concept::ReverseProxy.color(),

            // End devices
            ServiceCategory::Workstation => Color::Green,
            ServiceCategory::Mobile => Color::Blue,
            ServiceCategory::IoT => Concept::IoT.color(),
            ServiceCategory::Industrial => Color::Orange,
            ServiceCategory::Printer => Color::Gray,

            // Applications
            ServiceCategory::Database => Color::Gray,
            ServiceCategory::Development => Color::Red,
            ServiceCategory::Dashboard => Color::Purple,
            ServiceCategory::MessageQueue => Color::Green,
            ServiceCategory::IdentityAndAccess => Color::Yellow,
            ServiceCategory::Integration => Color::Cyan,

            // Office & Productivity
            ServiceCategory::Office => Color::Blue,
            ServiceCategory::ProjectManagement => Color::Indigo,

            // Communication
            ServiceCategory::Messaging => Color::Green,
            ServiceCategory::Conferencing => Color::Teal,
            ServiceCategory::Telephony => Color::Orange,
            ServiceCategory::Email => Color::Rose,

            // Content
            ServiceCategory::Publishing => Color::Purple, // was "violet", mapped to purple

            // Special
            ServiceCategory::Scanopy => Color::Purple,
            ServiceCategory::Custom => Color::Rose,
            ServiceCategory::OpenPorts => EntityDiscriminants::Port.color(),
            ServiceCategory::NetworkIdentities => Concept::Virtualization.color(),
            ServiceCategory::Unknown => Color::Gray,
        }
    }
}

impl TypeMetadataProvider for ServiceCategory {
    fn name(&self) -> &'static str {
        use ServiceCategory::*;
        match self {
            NetworkCore => "Network Core",
            NetworkAccess => "Network Access",
            NetworkAppliance => "Network Appliance",
            RemoteAccess => "Remote Access",
            Storage => "Storage",
            Backup => "Backup",
            Media => "Media",
            HomeAutomation => "Home Automation",
            Hypervisor => "Hypervisor",
            ContainerRuntime => "Container Runtime",
            Container => "Container",
            Orchestrator => "Orchestrator",
            DNS => "DNS",
            VPN => "VPN",
            Monitoring => "Monitoring",
            AdBlock => "Ad Blocking",
            ReverseProxy => "Reverse Proxy",
            Workstation => "Workstation",
            Mobile => "Mobile",
            IoT => "IoT",
            Industrial => "Industrial Control",
            Printer => "Printer",
            Database => "Database",
            Development => "Development",
            Dashboard => "Dashboard",
            MessageQueue => "Message Queue",
            IdentityAndAccess => "Identity & Access",
            Integration => "Integration",
            Office => "Office",
            ProjectManagement => "Project Management",
            Messaging => "Messaging",
            Conferencing => "Conferencing",
            Telephony => "Telephony",
            Email => "Email",
            Publishing => "Publishing",
            Unknown => "Unknown",
            Custom => "Custom",
            Scanopy => "Scanopy",
            OpenPorts => "Open Ports",
            NetworkIdentities => "Network Identities",
        }
    }

    fn description(&self) -> &'static str {
        use ServiceCategory::*;
        match self {
            NetworkCore => "Core network services like DHCP, NTP, gateways, and switches",
            NetworkAccess => "WiFi access points, mesh routers, and network access devices",
            NetworkAppliance => "Router, firewall, and VPN platforms like pfSense and MikroTik",
            RemoteAccess => "Remote access protocols like SSH, Telnet, and RDP",
            Storage => "Network-attached storage, file servers, and object storage",
            Backup => "Backup and data protection services",
            Media => "Media servers and streaming services like Plex and Jellyfin",
            HomeAutomation => "Smart home platforms like Home Assistant and openHAB",
            Hypervisor => "Virtual machine managers and hypervisors",
            ContainerRuntime => "Container engines that directly run containers",
            Container => "Application and system container instances",
            Orchestrator => "Container orchestration and management platforms",
            DNS => "DNS servers and resolvers",
            VPN => "VPN servers like OpenVPN and WireGuard",
            Monitoring => "Monitoring, observability, and SNMP agents",
            AdBlock => "Network-level ad and tracker blocking",
            ReverseProxy => "Reverse proxies and API gateways like Nginx and Traefik",
            Workstation => "Desktop computers and laptops",
            Mobile => "Phones and tablets",
            IoT => "Smart devices, sensors, and connected appliances",
            Industrial => {
                "Industrial control devices: PLCs, RTUs, drives, and building automation controllers"
            }
            Printer => "Printers and print servers",
            Database => "Database servers like PostgreSQL, MySQL, and Redis",
            Development => "Dev tools, CI/CD pipelines, and configuration management",
            Dashboard => "Dashboard and homepage applications",
            MessageQueue => "Message brokers and streaming platforms like Kafka and RabbitMQ",
            IdentityAndAccess => "Identity providers, SSO, and secret management",
            Integration => "B2B/EDI and application integration servers (AS2, ESB, iPaaS)",
            Office => "Document editing, notes, and file management",
            ProjectManagement => "Task tracking, wikis, and project boards",
            Messaging => "Team chat and messaging platforms",
            Conferencing => "Video and audio conferencing",
            Telephony => "VoIP and PBX infrastructure",
            Email => "Email servers and relay services",
            Publishing => "CMS, blogs, and forums",
            Unknown => "Services that could not be identified",
            Custom => "User-defined custom services",
            Scanopy => "Scanopy platform services",
            OpenPorts => "Unclaimed open ports without a matched service",
            NetworkIdentities => "Addresses and MACs a host presents beyond its own NICs",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "application_relevant_use_cases": self.application_relevant_use_cases()
        })
    }
}

impl ServiceCategory {
    /// Returns the use cases for which this category is considered application-relevant.
    /// Categories not relevant for a given use case are hidden by default in the
    /// Application perspective.
    pub fn application_relevant_use_cases(&self) -> Vec<UseCase> {
        use ServiceCategory::*;
        match self {
            // Infrastructure plumbing — never application-relevant
            NetworkCore | NetworkAccess | RemoteAccess | Workstation | Mobile | Printer
            | OpenPorts | NetworkIdentities => vec![],

            // Network appliances: infra for most, but MSPs manage these
            NetworkAppliance => vec![UseCase::Msp],

            // IoT: infra for non-homelab
            IoT => vec![UseCase::Homelab],

            // Everything else: relevant for all use cases
            _ => UseCase::iter().collect(),
        }
    }
}

impl HasFilterValues for Service {
    fn filter_values(&self, _ctx: &FilterValueContext) -> BTreeMap<MetadataFilterType, String> {
        use crate::server::services::r#impl::definitions::ServiceDefinition;
        let mut values = BTreeMap::new();
        // Disambiguate: Box<dyn ServiceDefinition> also impls TypeMetadataProvider
        // which carries a separate `category() -> &str`.
        let category = ServiceDefinition::category(&*self.base.service_definition);
        values.insert(MetadataFilterType::Category, category.id().to_string());
        values
    }
}
