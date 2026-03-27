//! Example data for OpenAPI documentation.
//!
//! These examples are used by `#[schema(example = ...)]` attributes to provide
//! realistic sample data in the API documentation. Based on test fixtures but
//! with static placeholder IDs.

use chrono::{TimeZone, Utc};
use cidr::{IpCidr, Ipv4Cidr};
use email_address::EmailAddress;
use mac_address::MacAddress;
use semver::Version;
use std::net::{IpAddr, Ipv4Addr};

use crate::server::{
    bindings::r#impl::base::Binding,
    credentials::r#impl::mapping::SnmpCredentialMapping,
    daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase},
    daemons::r#impl::{
        api::DaemonCapabilities,
        base::{Daemon, DaemonBase, DaemonMode},
    },
    discovery::r#impl::{
        base::{Discovery, DiscoveryBase},
        types::{DiscoveryType, RunType},
    },
    groups::r#impl::{
        base::{Group, GroupBase},
        types::GroupType,
    },
    hosts::r#impl::{
        api::{
            BindingInput, CreateHostRequest, HostResponse, InterfaceInput, PortInput, ServiceInput,
        },
        base::{Host, HostBase},
    },
    if_entries::r#impl::base::{IfAdminStatus, IfEntry, IfEntryBase, IfOperStatus},
    interfaces::r#impl::base::{Interface, InterfaceBase},
    networks::r#impl::{Network, NetworkBase},
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
        },
    }
}

/// Example Host entity.
pub fn host() -> Host {
    Host {
        id: ids::HOST,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: HostBase {
            name: "web-server-01".to_string(),
            hostname: Some("web-server-01.local".to_string()),
            network_id: ids::NETWORK,
            description: Some("Primary web server".to_string()),
            source: EntitySource::Manual,
            virtualization: None,
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
        base: SubnetBase {
            name: "LAN".to_string(),
            description: Some("Local area network".to_string()),
            network_id: ids::NETWORK,
            cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(192, 168, 1, 0), 24).unwrap()),
            subnet_type: SubnetType::Lan,
            source: EntitySource::Manual,
            tags: vec![],
        },
    }
}

/// Example Interface entity.
pub fn interface() -> Interface {
    Interface {
        id: ids::INTERFACE,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: InterfaceBase {
            network_id: ids::NETWORK,
            host_id: ids::HOST,
            subnet_id: ids::SUBNET,
            ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            mac_address: Some(MacAddress::new([0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE])),
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
        base: PortBase {
            host_id: ids::HOST,
            network_id: ids::NETWORK,
            port_type: PortType::Http,
        },
    }
}

/// Example Group entity.
pub fn group() -> Group {
    Group {
        id: ids::GROUP,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: GroupBase {
            name: "Web Services".to_string(),
            description: Some("HTTP/HTTPS services group".to_string()),
            network_id: ids::NETWORK,
            color: Color::Blue,
            group_type: GroupType::RequestPath,
            binding_ids: vec![],
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
        base: ServiceBase {
            name: "nginx".to_string(),
            host_id: ids::HOST,
            network_id: ids::NETWORK,
            service_definition: service_def,
            bindings: vec![binding()],
            virtualization: None,
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
        base: TagBase {
            name: "production".to_string(),
            description: Some("Production environment resources".to_string()),
            color: Color::Green,
            organization_id: ids::ORGANIZATION,
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
            capabilities: DaemonCapabilities {
                has_docker_socket: true,
                interfaced_subnet_ids: vec![ids::SUBNET],
            },
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
            brevo_company_id: None,
            plan_limit_notifications: Default::default(),
            use_case: None,
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
        pending_credential_ids: vec![],
    }
}

/// Example IfEntry entity.
pub fn if_entry() -> IfEntry {
    IfEntry {
        id: ids::IF_ENTRY,
        created_at: example_timestamp(),
        updated_at: example_timestamp(),
        base: IfEntryBase {
            host_id: ids::HOST,
            network_id: ids::NETWORK,
            if_index: 1,
            if_descr: "GigabitEthernet0/1".to_string(),
            if_name: Some("Gi0/1".to_string()),
            if_alias: Some("Uplink to Core Switch".to_string()),
            if_type: 6,                     // ethernet
            speed_bps: Some(1_000_000_000), // 1 Gbps
            admin_status: IfAdminStatus::Up,
            oper_status: IfOperStatus::Up,
            mac_address: Some(MacAddress::new([0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE])),
            interface_id: Some(ids::INTERFACE),
            neighbor: None,
            lldp_chassis_id: None,
            lldp_port_id: None,
            lldp_sys_name: None,
            lldp_port_desc: None,
            lldp_mgmt_addr: None,
            lldp_sys_desc: None,
            cdp_device_id: None,
            cdp_port_id: None,
            cdp_platform: None,
            cdp_address: None,
            fdb_macs: None,
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
        virtualization: None,
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
        interfaces: vec![InterfaceInput {
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
                interface_id: Some(ids::INTERFACE),
            }],
            virtualization: None,
            tags: vec![],
            position: Some(0),
        }],
        if_entries: vec![],
    }
}

// =============================================================================
// RESPONSE EXAMPLES
// =============================================================================

/// Example HostResponse.
pub fn host_response() -> HostResponse {
    HostResponse::from_host_with_children(
        host(),
        vec![interface()],
        vec![port()],
        vec![service()],
        vec![if_entry()],
    )
}
