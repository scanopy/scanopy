//! Example data for OpenAPI documentation.
//!
//! These examples are used by `#[schema(example = ...)]` attributes to provide
//! realistic sample data in the API documentation. Based on test fixtures but
//! with static placeholder IDs.

use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use chrono::{TimeZone, Utc};
use cidr::{IpCidr, Ipv4Cidr};
use email_address::EmailAddress;
use mac_address::MacAddress;
use semver::Version;
use std::net::{IpAddr, Ipv4Addr};

use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
use crate::server::{
    bindings::r#impl::base::Binding,
    credentials::r#impl::mapping::SnmpCredentialMapping,
    daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase},
    daemons::r#impl::base::{Daemon, DaemonBase, DaemonMode},
    dependencies::r#impl::{
        base::{Dependency, DependencyBase, DependencyMembers},
        types::DependencyType,
    },
    discovery::r#impl::{
        base::{Discovery, DiscoveryBase},
        types::{DiscoveryType, RunType},
    },
    hosts::r#impl::{
        api::{
            BindingInput, CreateHostRequest, HostResponse, IPAddressInput, PortInput, ServiceInput,
        },
        base::{Host, HostBase},
        name::{HostName, HostNameSources},
    },
    interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, Interface, InterfaceBase},
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    networks::r#impl::{DEFAULT_STALE_AFTER_HOURS, Network, NetworkBase},
    organizations::r#impl::base::{Organization, OrganizationBase},
    ports::r#impl::base::{Port, PortBase, PortType, TransportProtocol},
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::base::{Service, ServiceBase},
    },
    shared::types::{Color, entities::EntitySource},
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
    tags::r#impl::base::{Tag, TagBase},
    topology::types::edges::EdgeStyle,
    users::r#impl::{
        base::{User, UserBase},
        permissions::UserOrgPermissions,
    },
};

// =============================================================================
// PLACEHOLDER IDS
// =============================================================================

/// Stable placeholder UUIDs for examples.
/// Using deterministic UUIDs so examples are consistent across regenerations.
pub mod ids {
    use uuid::Uuid;

    pub const ORGANIZATION: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440001);
    pub const NETWORK: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440002);
    pub const HOST: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440003);
    pub const SUBNET: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440004);
    pub const INTERFACE: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440005);
    pub const PORT: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440006);
    pub const SERVICE: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440007);
    pub const GROUP: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440008);
    pub const BINDING: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440009);
    pub const TAG: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_44665544000a);
    pub const API_KEY: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_44665544000b);
    pub const DAEMON: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_44665544000c);
    pub const USER: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_44665544000d);
    pub const DISCOVERY: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_44665544000e);
    pub const IF_ENTRY: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_44665544000f);
}

/// Example timestamp for created_at/updated_at fields.
fn example_timestamp() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 15, 10, 30, 0).unwrap()
}

// =============================================================================
// ENTITY EXAMPLES
// =============================================================================

/// Example Network entity.
pub fn network() -> Network {
    Network {
        id: ids::NETWORK,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: NetworkBase {
            name: "Home Network".to_string(),
            organization_id: ids::ORGANIZATION,
            tags: vec![],
            credential_ids: vec![],
            stale_after_hours: None,
        },
        effective_stale_after_hours: DEFAULT_STALE_AFTER_HOURS,
    }
}

/// Example Host entity.
pub fn host() -> Host {
    Host {
        id: ids::HOST,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        valid_from: example_timestamp(),
        valid_to: None,
        lineage_id: None,
        last_seen_at: example_timestamp(),
        last_discovery_id: None,
        first_discovery_id: None,
        base: HostBase {
            name: HostName::manual("web-server-01".to_string()),
            hostname: Some(crate::server::shared::attribution::Attributed::new(
                crate::server::hosts::r#impl::attributes::HostHostnameValue(
                    "web-server-01.local".to_string(),
                ),
                crate::server::shared::attribution::AttributeSource::Manual,
            )),
            network_id: ids::NETWORK,
            description: Some("Primary web server".to_string()),
            source: EntitySource::Manual,
            virtualization_metadata: None,
            virtualization_service_id: None,
            hidden: false,
            tags: vec![],
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            firmware_revision: None,
            software_revision: None,
            credential_assignments: vec![],
        },
    }
}

/// Example Subnet entity.
pub fn subnet() -> Subnet {
    Subnet {
        id: ids::SUBNET,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        valid_from: example_timestamp(),
        valid_to: None,
        lineage_id: None,
        last_seen_at: example_timestamp(),
        last_discovery_id: None,
        first_discovery_id: None,
        base: SubnetBase {
            name: "LAN".to_string(),
            description: Some("Local area network".to_string()),
            network_id: ids::NETWORK,
            cidr: SubnetCidr::new(
                SubnetCidrValue(IpCidr::V4(
                    Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap(),
                )),
                AttributeSource::DaemonSelfReport,
            ),
            subnet_type: SubnetType::Lan,
            virtualization_service_id: None,
            source: EntitySource::Manual,
            tags: vec![],
        },
    }
}

/// Example Interface entity.
pub fn ip_address() -> IPAddress {
    IPAddress {
        id: ids::INTERFACE,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        valid_from: example_timestamp(),
        valid_to: None,
        lineage_id: None,
        last_seen_at: example_timestamp(),
        last_discovery_id: None,
        first_discovery_id: None,
        base: IPAddressBase {
            network_id: ids::NETWORK,
            host_id: ids::HOST,
            subnet_id: ids::SUBNET,
            ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(MacAddress::new([0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE])),
                AttributeSource::ArpReply,
            )),
            name: Some("eth0".to_string()),
            position: 0,
        },
    }
}

/// Example Port entity.
pub fn port() -> Port {
    Port {
        id: ids::PORT,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        valid_from: example_timestamp(),
        valid_to: None,
        lineage_id: None,
        last_seen_at: example_timestamp(),
        last_discovery_id: None,
        first_discovery_id: None,
        base: PortBase {
            host_id: ids::HOST,
            network_id: ids::NETWORK,
            port_type: PortType::Http,
        },
    }
}

/// Example Dependency entity.
pub fn dependency() -> Dependency {
    Dependency {
        id: ids::GROUP,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        valid_from: example_timestamp(),
        valid_to: None,
        lineage_id: None,
        base: DependencyBase {
            name: "Web Services".to_string(),
            description: Some("HTTP/HTTPS services dependency".to_string()),
            network_id: ids::NETWORK,
            color: Color::Blue,
            dependency_type: DependencyType::RequestPath,
            members: DependencyMembers::default(),
            source: EntitySource::Manual,
            edge_style: EdgeStyle::Bezier,
            tags: vec![],
        },
    }
}

/// Example Service entity.
pub fn service() -> Service {
    let service_def = ServiceDefinitionRegistry::find_by_id("Nginx")
        .unwrap_or_else(|| ServiceDefinitionRegistry::all_service_definitions()[0].clone());

    Service {
        id: ids::SERVICE,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        valid_from: example_timestamp(),
        valid_to: None,
        lineage_id: None,
        last_seen_at: example_timestamp(),
        last_discovery_id: None,
        first_discovery_id: None,
        base: ServiceBase {
            name: "nginx".to_string(),
            host_id: ids::HOST,
            network_id: ids::NETWORK,
            service_definition: service_def,
            bindings: vec![binding()],
            virtualization_metadata: None,
            virtualization_service_id: None,
            source: EntitySource::Manual,
            tags: vec![],
            position: 0,
        },
    }
}

/// Example Binding entity.
pub fn binding() -> Binding {
    Binding::new_port(ids::SERVICE, ids::NETWORK, ids::PORT, Some(ids::INTERFACE))
}

/// Example Tag entity.
pub fn tag() -> Tag {
    Tag {
        id: ids::TAG,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        valid_from: example_timestamp(),
        valid_to: None,
        lineage_id: None,
        base: TagBase {
            name: "production".to_string(),
            description: Some("Production environment resources".to_string()),
            color: Color::Green,
            organization_id: ids::ORGANIZATION,
            is_application: false,
        },
    }
}

/// Example DaemonApiKey entity.
pub fn daemon_api_key() -> DaemonApiKey {
    DaemonApiKey {
        id: ids::API_KEY,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: DaemonApiKeyBase {
            name: "daemon-key-01".to_string(),
            key: "scp_d_••••••••••••••••••••••••••••••••".to_string(), // Masked in responses
            network_id: ids::NETWORK,
            last_used: Some(example_timestamp()),
            expires_at: None,
            is_enabled: true,
            tags: vec![],
            daemon_id: None,
            plaintext: None,
        },
    }
}

/// Example Daemon entity.
pub fn daemon() -> Daemon {
    Daemon {
        id: ids::DAEMON,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: DaemonBase {
            network_id: ids::NETWORK,
            host_id: ids::HOST,
            url: "http://192.168.1.100:8080".to_string(),
            mode: DaemonMode::DaemonPoll,
            last_seen: Some(example_timestamp()),
            name: "home-daemon".to_string(),
            tags: vec![],
            version: Version::parse(env!("CARGO_PKG_VERSION"))
                .map(Some)
                .unwrap_or_default(),
            user_id: ids::USER,
            api_key_id: None,
            is_unreachable: false,
            standby: false,
            standby_cleared_at: None,
        },
    }
}

/// Example User entity.
pub fn user() -> User {
    User {
        id: ids::USER,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: UserBase {
            email: EmailAddress::new_unchecked("alice@example.com"),
            organization_id: ids::ORGANIZATION,
            permissions: UserOrgPermissions::Admin,
            password_hash: None,
            has_password: false,
            oidc_provider: None,
            oidc_subject: None,
            oidc_linked_at: None,
            network_ids: vec![ids::NETWORK],
            terms_accepted_at: Some(example_timestamp()),
            email_verified: true,
            email_verification_token: None,
            email_verification_expires: None,
            password_reset_token: None,
            password_reset_expires: None,
            pending_email: None,
            email_settings: Default::default(),
            session_epoch: 0,
        },
    }
}

/// Example Organization entity.
pub fn organization() -> Organization {
    Organization {
        id: ids::ORGANIZATION,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: OrganizationBase {
            name: "Acme Corp".to_string(),
            stripe_customer_id: None,
            plan: None,
            plan_status: None,
            onboarding: vec![],
            has_payment_method: false,
            trial_end_date: None,
            last_paused_at: None,
            trial_extended_used: false,
            last_downgrade_at: None,
            last_downgrade_from_plan: None,
            last_discount_at: None,
            discount_save_offer_percent_off: None,
            discount_save_offer_active_until: None,
            next_renewal_at: None,
            brevo_company_id: None,
            license_entitlement: None,
            license_entitlement_at: None,
            notifications: Default::default(),
            use_case: Default::default(),
            license_paid_through: None,
            license_checkin_at: None,
            license_key_version: 0,
        },
    }
}

/// Example Discovery entity.
pub fn discovery() -> Discovery {
    Discovery {
        id: ids::DISCOVERY,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: DiscoveryBase {
            name: "Network Scan".to_string(),
            network_id: ids::NETWORK,
            daemon_id: ids::DAEMON,
            discovery_type: DiscoveryType::Network {
                subnet_ids: Some(vec![ids::SUBNET]),
                host_naming_fallback: Default::default(),
                snmp_credentials: SnmpCredentialMapping::default(),
            },
            run_type: RunType::AdHoc {
                last_run: Some(example_timestamp()),
            },
            tags: vec![],
        },
        scan_count: 0,
        force_full_scan: false,
        integration_targets: vec![],
    }
}

/// Example Interface entity.
pub fn interface() -> Interface {
    Interface {
        id: ids::IF_ENTRY,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        valid_from: example_timestamp(),
        valid_to: None,
        lineage_id: None,
        last_seen_at: example_timestamp(),
        last_discovery_id: None,
        first_discovery_id: None,
        // Matches the ladder `Interface::display_name` would compute for the `if_alias` below —
        // an example should show what a real response actually looks like.
        display_name: Some("Uplink to Core Switch".to_string()),
        base: InterfaceBase {
            host_id: ids::HOST,
            network_id: ids::NETWORK,
            if_index: Some(1),
            if_descr: Some("GigabitEthernet0/1".to_string()),
            if_name: Some("Gi0/1".to_string()),
            if_alias: Some("Uplink to Core Switch".to_string()),
            if_type: Some(6),               // ethernet
            speed_bps: Some(1_000_000_000), // 1 Gbps
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(IfOperStatus::Up),
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(MacAddress::new([0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE])),
                AttributeSource::ArpReply,
            )),
            ip_address_id: Some(ids::INTERFACE),
            ip_configured: true,
            neighbor_candidates: Vec::new(),
            fdb_macs: None,
            native_vlan_id: None,
            vlan_ids: None,
        },
    }
}

// =============================================================================
// REQUEST EXAMPLES
// =============================================================================

/// Example CreateHostRequest.
pub fn create_host_request() -> CreateHostRequest {
    let service_def = ServiceDefinitionRegistry::find_by_id("Nginx")
        .unwrap_or_else(|| ServiceDefinitionRegistry::all_service_definitions()[0].clone());

    CreateHostRequest {
        name: "web-server-01".to_string(),
        network_id: ids::NETWORK,
        hostname: Some("web-server-01.local".to_string()),
        description: Some("Primary web server".to_string()),
        virtualization_metadata: None,
        virtualization_service_id: None,
        hidden: false,
        tags: vec![],
        // SNMP fields (optional)
        sys_descr: None,
        sys_object_id: None,
        sys_location: None,
        sys_contact: None,
        management_url: None,
        chassis_id: None,
        credential_assignments: vec![],
        ip_addresses: vec![IPAddressInput {
            id: ids::INTERFACE,
            subnet_id: ids::SUBNET,
            ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            mac_address: Some(MacAddress::new([0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34])),
            name: Some("eth0".to_string()),
            position: Some(0),
        }],
        ports: vec![PortInput {
            id: ids::PORT,
            number: 80,
            protocol: TransportProtocol::Tcp,
        }],
        services: vec![ServiceInput {
            id: ids::SERVICE,
            name: "nginx".to_string(),
            service_definition: service_def,
            bindings: vec![BindingInput::Port {
                id: ids::BINDING,
                port_id: ids::PORT,
                ip_address_id: Some(ids::INTERFACE),
            }],
            virtualization_metadata: None,
            virtualization_service_id: None,
            tags: vec![],
            position: Some(0),
        }],
        interfaces: vec![],
    }
}

// =============================================================================
// RESPONSE EXAMPLES
// =============================================================================

/// Example HostResponse.
pub fn host_response() -> HostResponse {
    HostResponse::from_host_with_children(
        host(),
        vec![ip_address()],
        vec![port()],
        vec![service()],
        vec![interface()],
    )
}
