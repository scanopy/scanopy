use cidr::Ipv4Cidr;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::base::{
    SubnetCidr, SubnetCidrValue, SubnetName, SubnetTypeAttributed,
};
use crate::server::{
    bindings::r#impl::base::Binding,
    hosts::r#impl::{
        base::{Host, HostBase},
        name::{HostName, HostNameSources},
    },
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    ports::r#impl::base::{Port, PortType},
    services::{
        definitions::{client::Client, dns_server::DnsServer, web_service::WebService},
        r#impl::base::{Service, ServiceBase},
    },
    shared::{
        storage::traits::Storable,
        types::{Color, Icon, entities::EntitySource},
    },
    sites::r#impl::{Site, SiteBase},
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
    tags::{
        groups::base::{TagGroup, TagGroupType},
        r#impl::base::{Tag, TagBase, TagIcon},
    },
    users::r#impl::base::{User, UserBase},
};

pub fn create_user() -> User {
    User::new(UserBase::default())
}

pub fn create_site(organization_id: Uuid) -> Site {
    Site::new(SiteBase::new(organization_id))
}

pub fn create_wan_subnet(site_id: Uuid) -> Subnet {
    let base = SubnetBase {
        vlan_ids: Vec::new(),
        name: SubnetName::unattributed("Internet".to_string()),
        site_id,
        tags: Vec::new(),
        // Seeded by Scanopy on purpose, so it must never read as a guess to confirm.
        cidr: SubnetCidr::new(
            SubnetCidrValue(cidr::IpCidr::V4(
                Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 0).expect("Cidr for internet subnet"),
            )),
            AttributeSource::DaemonSelfReport,
        ),
        description: Some(
            "For representing services on the public internet, like DNS servers, \
cloud providers, or SaaS applications, that your site depends on but wouldn't \
be found by scanning. Create ip_addresses on this subnet to include them in your topology."
                .to_string(),
        ),
        subnet_type: SubnetTypeAttributed::unattributed(SubnetType::Internet),
        virtualization_service_id: None,
        source: EntitySource::System,
    };

    Subnet::new(base)
}

/// A tag every organization starts with: (name, description, color, icon).
type StarterTag = (&'static str, &'static str, Color, Icon);

/// The tag groups every organization starts with, by name, each with its tags in order. "Status"
/// is the system Status group; the others are custom groups, a starting point the organization can
/// rename, extend, empty or delete.
const STARTER_TAG_GROUPS: [(&str, &[StarterTag]); 4] = [
    (
        "Status",
        &[
            (
                "Planned",
                "Ordered or scheduled, not yet in service",
                Color::Blue,
                Icon::CalendarClock,
            ),
            ("Active", "In service", Color::Green, Icon::CircleCheck),
            (
                "Decommissioning",
                "Being retired; still on the network",
                Color::Orange,
                Icon::Hourglass,
            ),
            (
                "Decommissioned",
                "Retired from service; kept for the record",
                Color::Gray,
                Icon::Archive,
            ),
        ],
    ),
    (
        "Environment",
        &[
            (
                "Production",
                "Serves live users and business traffic",
                Color::Red,
                Icon::Server,
            ),
            (
                "Staging",
                "A pre-release copy of production for final testing",
                Color::Amber,
                Icon::FlaskConical,
            ),
            (
                "Development",
                "Used to build and test changes",
                Color::Blue,
                Icon::Code,
            ),
            (
                "Lab",
                "Test bench or experimental equipment",
                Color::Purple,
                Icon::TestTube,
            ),
        ],
    ),
    (
        "Criticality",
        &[
            (
                "Critical",
                "An outage stops the business",
                Color::Red,
                Icon::Siren,
            ),
            (
                "High",
                "An outage disrupts many people",
                Color::Orange,
                Icon::ArrowUp,
            ),
            (
                "Medium",
                "An outage disrupts some people",
                Color::Yellow,
                Icon::Minus,
            ),
            (
                "Low",
                "An outage has little impact",
                Color::Gray,
                Icon::ArrowDown,
            ),
        ],
    ),
    (
        "Management",
        &[
            (
                "Managed",
                "Administered by your IT team",
                Color::Green,
                Icon::ShieldCheck,
            ),
            (
                "Unmanaged",
                "Administered by no one, such as a personal device",
                Color::Orange,
                Icon::ShieldQuestionMark,
            ),
            (
                "Third-party",
                "Administered by a vendor or another organization",
                Color::Blue,
                Icon::Handshake,
            ),
        ],
    ),
];

/// The tag a new Visibility group starts with. The migration seeds the same tag.
pub fn hidden_tag(organization_id: Uuid, visibility_group_id: Uuid) -> Tag {
    Tag::new(TagBase {
        name: "Hidden".to_string(),
        description: Some("Left out of topology.".to_string()),
        color: Color::Gray,
        organization_id,
        tag_group_id: Some(visibility_group_id),
        position: None,
        icon: Some(TagIcon(Icon::EyeOff)),
        source: EntitySource::System,
    })
}

/// What a starter custom group is for.
pub fn starter_group_description(name: &str) -> Option<&'static str> {
    match name {
        "Environment" => Some("Which stage of delivery an asset serves."),
        "Criticality" => Some("How much an outage of the asset costs."),
        "Management" => Some("Who administers the asset."),
        _ => None,
    }
}

/// The custom groups every organization starts with, besides the system groups.
pub fn starter_custom_group_names() -> Vec<&'static str> {
    STARTER_TAG_GROUPS
        .iter()
        .map(|(group, _)| *group)
        .filter(|group| TagGroupType::system_named(group).is_none())
        .collect()
}

/// The tags every organization starts with, each in its group of `groups` and in the starter
/// order. A starter group missing from `groups` contributes no tags.
pub fn create_starter_tags(organization_id: Uuid, groups: &[TagGroup]) -> Vec<Tag> {
    STARTER_TAG_GROUPS
        .iter()
        .filter_map(|(group_name, tags)| {
            let group = groups
                .iter()
                .find(|g| match TagGroupType::system_named(group_name) {
                    Some(system) => g.base.group_type == system,
                    None => g.base.name.eq_ignore_ascii_case(group_name),
                })?;
            Some(
                tags.iter()
                    .zip(1..)
                    .map(move |((name, description, color, icon), position)| {
                        Tag::new(TagBase {
                            name: name.to_string(),
                            description: Some(description.to_string()),
                            color: *color,
                            organization_id,
                            tag_group_id: Some(group.id),
                            position: Some(position),
                            icon: Some(TagIcon(*icon)),
                            source: EntitySource::System,
                        })
                    }),
            )
        })
        .flatten()
        .collect()
}

pub fn create_remote_subnet(site_id: Uuid) -> Subnet {
    let base = SubnetBase {
        vlan_ids: Vec::new(),
        name: SubnetName::unattributed("Remote Network".to_string()),
        site_id,
        tags: Vec::new(),
        // Seeded by Scanopy on purpose, so it must never read as a guess to confirm.
        cidr: SubnetCidr::new(
            SubnetCidrValue(cidr::IpCidr::V4(
                Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 0).expect("Cidr for internet subnet"),
            )),
            AttributeSource::DaemonSelfReport,
        ),
        description: Some(
            "For representing services on networks outside your scan range, like \
branch offices, VPN endpoints, or remote infrastructure. Create ip_addresses on this \
subnet to include them in your topology."
                .to_string(),
        ),
        subnet_type: SubnetTypeAttributed::unattributed(SubnetType::Remote),
        virtualization_service_id: None,
        source: EntitySource::System,
    };

    Subnet::new(base)
}

/// Returns (Host, Vec<IPAddress>, Vec<Port>, Service) - children are passed separately to discover_host
pub fn create_remote_host(
    remote_subnet: &Subnet,
    site_id: Uuid,
) -> (Host, Vec<IPAddress>, Vec<Port>, Service) {
    // Create interface with placeholder host_id - server will set the correct one
    let ip_address = IPAddress::new(IPAddressBase::new_conceptual(Uuid::nil(), remote_subnet));

    let dynamic_port = Port::new_hostless(PortType::new_tcp(0)); // Ephemeral port
    let binding = Binding::new_port_serviceless(dynamic_port.id, Some(ip_address.id));

    let base = HostBase {
        // A deliberate label on a synthetic host, not something we derived — discovery
        // must never rename these.
        name: HostName::manual("Mobile Device".to_string()),
        hostname: None,
        site_id,
        tags: Vec::new(),
        description: Some("A mobile device connecting from a remote network".to_string()),
        source: EntitySource::System,
        virtualization_metadata: None,
        virtualization_service_id: None,
        ..Default::default()
    };

    let host = Host::new(base);

    let client_service = Service::new(ServiceBase {
        host_id: host.id,
        site_id,
        tags: Vec::new(),
        name: "Mobile Device".to_string(),
        service_definition: Box::new(Client),
        bindings: vec![binding],
        virtualization_metadata: None,
        virtualization_service_id: None,
        source: EntitySource::System,
        position: 0,
    });

    (host, vec![ip_address], vec![dynamic_port], client_service)
}

/// Returns (Host, Vec<IPAddress>, Vec<Port>, Service) - children are passed separately to discover_host
pub fn create_internet_connectivity_host(
    internet_subnet: &Subnet,
    site_id: Uuid,
) -> (Host, Vec<IPAddress>, Vec<Port>, Service) {
    // Create interface with placeholder host_id - server will set the correct one
    let ip_address = IPAddress::new(IPAddressBase::new_conceptual(Uuid::nil(), internet_subnet));

    let https_port = Port::new_hostless(PortType::Https);
    let binding = Binding::new_port_serviceless(https_port.id, Some(ip_address.id));

    let base = HostBase {
        // A deliberate label on a synthetic host, not something we derived — discovery
        // must never rename these.
        name: HostName::manual("Google.com".to_string()),
        site_id,
        tags: Vec::new(),
        hostname: None,
        description: None,
        source: EntitySource::System,
        virtualization_metadata: None,
        virtualization_service_id: None,
        ..Default::default()
    };

    let host = Host::new(base);

    let web_service = Service::new(ServiceBase {
        host_id: host.id,
        name: "Google.com".to_string(),
        site_id,
        tags: Vec::new(),
        service_definition: Box::new(WebService),
        bindings: vec![binding],
        virtualization_metadata: None,
        virtualization_service_id: None,
        source: EntitySource::System,
        position: 0,
    });

    (host, vec![ip_address], vec![https_port], web_service)
}

/// Returns (Host, Vec<IPAddress>, Vec<Port>, Service) - children are passed separately to discover_host
pub fn create_public_dns_host(
    internet_subnet: &Subnet,
    site_id: Uuid,
) -> (Host, Vec<IPAddress>, Vec<Port>, Service) {
    // Create interface with placeholder host_id - server will set the correct one
    let mut ip_address =
        IPAddress::new(IPAddressBase::new_conceptual(Uuid::nil(), internet_subnet));
    ip_address.base.ip_address = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
    let dns_udp_port = Port::new_hostless(PortType::DnsUdp);
    let binding = Binding::new_port_serviceless(dns_udp_port.id, Some(ip_address.id));

    let base = HostBase {
        // A deliberate label on a synthetic host, not something we derived — discovery
        // must never rename these.
        name: HostName::manual("Cloudflare DNS".to_string()),
        hostname: None,
        site_id,
        description: None,
        tags: Vec::new(),
        source: EntitySource::System,
        virtualization_metadata: None,
        virtualization_service_id: None,
        ..Default::default()
    };

    let host = Host::new(base);

    let dns_service = Service::new(ServiceBase {
        host_id: host.id,
        site_id,
        tags: Vec::new(),
        name: "Cloudflare DNS".to_string(),
        service_definition: Box::new(DnsServer),
        bindings: vec![binding],
        virtualization_metadata: None,
        virtualization_service_id: None,
        source: EntitySource::System,
        position: 0,
    });

    (host, vec![ip_address], vec![dns_udp_port], dns_service)
}
