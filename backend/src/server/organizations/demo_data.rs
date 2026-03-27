//! Demo data for populating demo organizations with realistic network infrastructure.
//!
//! This module provides a complete dataset representing "Acme Technologies", a mid-size
//! company with MSP operations. The data includes multiple networks, subnets, hosts,
//! services, daemons, API keys, tags, and groups.

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::{
    bindings::r#impl::base::Binding,
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        types::{CredentialAssignment, CredentialType, SecretValue},
    },
    daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase},
    daemons::r#impl::{
        api::{DaemonCapabilities, DiscoveryUpdatePayload},
        base::{Daemon, DaemonBase, DaemonMode},
    },
    discovery::r#impl::{
        base::{Discovery, DiscoveryBase},
        scan_settings::ScanSettings,
        types::{DiscoveryType, HostNamingFallback, RunType},
    },
    groups::r#impl::{
        base::{Group, GroupBase},
        types::GroupType,
    },
    hosts::r#impl::{
        base::{Host, HostBase},
        virtualization::{HostVirtualization, ProxmoxVirtualization},
    },
    if_entries::r#impl::base::{IfAdminStatus, IfEntry, IfEntryBase, IfOperStatus},
    interfaces::r#impl::base::{Interface, InterfaceBase},
    networks::r#impl::{Network, NetworkBase},
    ports::r#impl::base::{Port, PortType},
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::{
            base::{Service, ServiceBase},
            virtualization::{DockerVirtualization, ServiceVirtualization},
        },
    },
    shared::{
        api_key_common::{ApiKeyType, generate_api_key_for_storage},
        types::{Color, entities::EntitySource},
    },
    shares::r#impl::base::{Share, ShareBase, ShareOptions},
    snmp::resolution::lldp::{LldpChassisId, LldpPortId},
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
    tags::r#impl::base::{Tag, TagBase},
    topology::types::{
        base::{Topology, TopologyBase},
        edges::EdgeStyle,
    },
    user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase},
    users::r#impl::permissions::UserOrgPermissions,
};
use chrono::{DateTime, Duration, Utc};
use cidr::{IpCidr, Ipv4Cidr};
use secrecy::SecretString;
use semver::Version;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

// ============================================================================
// Demo Data Container
// ============================================================================

/// A host bundled with its interfaces, ports, and services for creation via discover_host
pub struct HostWithServices {
    pub host: Host,
    pub interfaces: Vec<Interface>,
    pub ports: Vec<Port>,
    pub services: Vec<Service>,
}

/// Deferred neighbor update to apply after all if_entries exist.
/// Uses host_name + if_index to identify entries (stable across creation)
/// instead of pre-generated UUIDs (which may not be preserved by storage).
pub struct NeighborUpdate {
    /// Source if_entry identifier
    pub source_host_name: String,
    pub source_if_index: i32,
    /// Target if_entry identifier (for IfEntry neighbors)
    pub target_host_name: String,
    pub target_if_index: i32,
}

/// Network-to-credential association for junction table seeding
pub struct NetworkCredentialAssignment {
    pub network_id: Uuid,
    pub credential_ids: Vec<Uuid>,
}

/// Container for all demo data entities
pub struct DemoData {
    pub tags: Vec<Tag>,
    pub credentials: Vec<Credential>,
    pub network_credential_assignments: Vec<NetworkCredentialAssignment>,
    pub networks: Vec<Network>,
    pub subnets: Vec<Subnet>,
    pub hosts_with_services: Vec<HostWithServices>,
    pub if_entries: Vec<IfEntry>,
    pub neighbor_updates: Vec<NeighborUpdate>,
    pub daemons: Vec<Daemon>,
    pub api_keys: Vec<DaemonApiKey>,
    pub groups: Vec<Group>,
    pub topologies: Vec<Topology>,
    pub discoveries: Vec<Discovery>,
    pub shares: Vec<Share>,
    pub user_api_keys: Vec<(UserApiKey, Vec<Uuid>)>,
}

impl DemoData {
    /// Generate all demo data for the given organization
    /// Note: Groups are intentionally empty - they must be generated after services are created
    /// because group bindings reference actual service binding IDs from the database.
    pub fn generate(organization_id: Uuid, user_id: Uuid) -> Self {
        let now = Utc::now();

        // Generate all entities in dependency order
        let tags = generate_tags(organization_id, now);
        let credentials = generate_credentials(organization_id, now);
        let networks = generate_networks(organization_id, &tags, &credentials, now);
        let subnets = generate_subnets(&networks, &tags, now);
        let network_credential_assignments =
            generate_network_credential_assignments(&networks, &credentials);
        let hosts_with_services =
            generate_hosts_and_services(&networks, &subnets, &tags, &credentials, now);

        // Collect hosts for daemon generation and if_entry generation
        let hosts: Vec<&Host> = hosts_with_services.iter().map(|h| &h.host).collect();
        let interfaces: Vec<&Interface> = hosts_with_services
            .iter()
            .flat_map(|h| h.interfaces.iter())
            .collect();

        let (if_entries, neighbor_updates) =
            generate_if_entries(&networks, &hosts, &interfaces, now);
        let daemons = generate_daemons(&networks, &hosts, &subnets, now, user_id);
        let api_keys = generate_api_keys(&networks, now);
        let topologies = generate_topologies(&networks, now);
        let discoveries =
            generate_discoveries(&networks, &subnets, &daemons, &hosts, &credentials, now);
        let shares = generate_shares(&topologies, &networks, user_id, now);
        let user_api_keys = generate_user_api_keys(&networks, organization_id, now);

        // Groups are empty - they'll be generated in the handler after services are created
        // This ensures group bindings reference actual service binding IDs
        let groups = vec![];

        Self {
            tags,
            credentials,
            network_credential_assignments,
            networks,
            subnets,
            hosts_with_services,
            if_entries,
            neighbor_updates,
            daemons,
            api_keys,
            groups,
            topologies,
            discoveries,
            shares,
            user_api_keys,
        }
    }
}

// ============================================================================
// Topologies
// ============================================================================

fn generate_topologies(networks: &[Network], now: DateTime<Utc>) -> Vec<Topology> {
    networks
        .iter()
        .map(|network| Topology {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: TopologyBase::new(format!("{} Topology", network.base.name), network.id),
        })
        .collect()
}

// ============================================================================
// Tags
// ============================================================================

fn generate_tags(organization_id: Uuid, now: DateTime<Utc>) -> Vec<Tag> {
    let tag_definitions: [(&str, &str, Color); 10] = [
        ("Production", "Systems running in production", Color::Red),
        ("Development", "Development and test systems", Color::Blue),
        ("Critical", "Business-critical services", Color::Orange),
        ("Backup Target", "Backup destinations", Color::Green),
        ("Monitoring", "Monitoring infrastructure", Color::Purple),
        ("Database", "Database servers", Color::Cyan),
        ("Web Tier", "Web and application servers", Color::Teal),
        ("IoT Device", "Smart devices", Color::Yellow),
        ("Needs Attention", "Requires admin review", Color::Rose),
        ("Managed Client", "Client-owned assets", Color::Indigo),
    ];

    tag_definitions
        .iter()
        .map(|(name, description, color)| Tag {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: TagBase {
                name: name.to_string(),
                description: Some(description.to_string()),
                color: *color,
                organization_id,
            },
        })
        .collect()
}

// ============================================================================
// Credentials
// ============================================================================

fn generate_credentials(organization_id: Uuid, now: DateTime<Utc>) -> Vec<Credential> {
    vec![
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                name: "Default SNMPv2c".to_string(),
                credential_type: CredentialType::SnmpV2c {
                    community: SecretValue::Inline {
                        value: SecretString::from("public".to_string()),
                    },
                },
                target_ips: None,
                tags: Vec::new(),
            },
        },
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                name: "Network Devices".to_string(),
                credential_type: CredentialType::SnmpV2c {
                    community: SecretValue::Inline {
                        value: SecretString::from("acme-network".to_string()),
                    },
                },
                target_ips: None,
                tags: Vec::new(),
            },
        },
        Credential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: CredentialBase {
                organization_id,
                name: "Docker TLS Proxy".to_string(),
                credential_type: CredentialType::DockerProxy {
                    port: 2376,
                    path: None,
                    ssl_cert: None,
                    ssl_key: None,
                    ssl_chain: None,
                },
                target_ips: None,
                tags: Vec::new(),
            },
        },
    ]
}

// ============================================================================
// Network-Credential Assignments
// ============================================================================

fn generate_network_credential_assignments(
    networks: &[Network],
    credentials: &[Credential],
) -> Vec<NetworkCredentialAssignment> {
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };
    let find_cred = |name: &str| {
        credentials
            .iter()
            .find(|c| c.base.name.contains(name))
            .unwrap()
    };

    let default_snmp = find_cred("Default SNMPv2c");
    let network_snmp = find_cred("Network Devices");
    let docker_proxy = find_cred("Docker TLS Proxy");
    let hq = find_network("Headquarters");
    let dc = find_network("Data Center");

    vec![
        // HQ: both SNMP credentials + Docker proxy
        NetworkCredentialAssignment {
            network_id: hq.id,
            credential_ids: vec![default_snmp.id, network_snmp.id, docker_proxy.id],
        },
        // DC: both SNMP credentials
        NetworkCredentialAssignment {
            network_id: dc.id,
            credential_ids: vec![default_snmp.id, network_snmp.id],
        },
    ]
}

// ============================================================================
// Networks
// ============================================================================

fn generate_networks(
    organization_id: Uuid,
    tags: &[Tag],
    _credentials: &[Credential],
    now: DateTime<Utc>,
) -> Vec<Network> {
    let production_tag = tags
        .iter()
        .find(|t| t.base.name == "Production")
        .map(|t| t.id);

    // Note: credential_ids are hydrated from junction tables, not stored on the network.
    // Network-credential associations would be created via credential_service.set_network_credentials().

    // Stagger timestamps so networks sort in predictable order (Headquarters first)
    vec![
        Network {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: NetworkBase {
                name: "Headquarters".to_string(),
                organization_id,
                tags: production_tag.into_iter().collect(),
                credential_ids: vec![],
            },
        },
        Network {
            id: Uuid::new_v4(),
            created_at: now + chrono::Duration::seconds(1),
            updated_at: now + chrono::Duration::seconds(1),
            base: NetworkBase {
                name: "Data Center".to_string(),
                organization_id,
                tags: production_tag.into_iter().collect(),
                credential_ids: vec![],
            },
        },
    ]
}

// ============================================================================
// Subnets
// ============================================================================

fn generate_subnets(networks: &[Network], tags: &[Tag], now: DateTime<Utc>) -> Vec<Subnet> {
    let hq = networks
        .iter()
        .find(|n| n.base.name == "Headquarters")
        .unwrap();
    let dc = networks
        .iter()
        .find(|n| n.base.name == "Data Center")
        .unwrap();

    let monitoring_tag = tags
        .iter()
        .find(|t| t.base.name == "Monitoring")
        .map(|t| t.id);

    vec![
        // ===== Headquarters subnets (7) =====
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 1, 0), 24).unwrap()),
                network_id: hq.id,
                name: "HQ Management".to_string(),
                description: Some("Network management and monitoring".to_string()),
                subnet_type: SubnetType::Management,
                source: EntitySource::Manual,
                tags: monitoring_tag.into_iter().collect(),
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 10, 0), 24).unwrap()),
                network_id: hq.id,
                name: "HQ Office LAN".to_string(),
                description: Some("Office workstations".to_string()),
                subnet_type: SubnetType::Lan,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 20, 0), 24).unwrap()),
                network_id: hq.id,
                name: "HQ Servers".to_string(),
                description: Some("On-premises servers and hypervisors".to_string()),
                subnet_type: SubnetType::Lan,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 40, 0), 24).unwrap()),
                network_id: hq.id,
                name: "HQ Storage".to_string(),
                description: Some("Storage area network".to_string()),
                subnet_type: SubnetType::Storage,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 30, 0), 24).unwrap()),
                network_id: hq.id,
                name: "HQ IoT".to_string(),
                description: Some("Smart office devices".to_string()),
                subnet_type: SubnetType::IoT,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 17, 0, 0), 16).unwrap()),
                network_id: hq.id,
                name: "HQ Docker Bridge".to_string(),
                description: Some("Docker container network".to_string()),
                subnet_type: SubnetType::DockerBridge,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 100, 0), 24).unwrap()),
                network_id: hq.id,
                name: "HQ Guest WiFi".to_string(),
                description: Some("Guest wireless network".to_string()),
                subnet_type: SubnetType::Guest,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        // ===== Data Center subnets (6) =====
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 16, 0, 0), 24).unwrap()),
                network_id: dc.id,
                name: "DC Management".to_string(),
                description: Some("Data center management network".to_string()),
                subnet_type: SubnetType::Management,
                source: EntitySource::Manual,
                tags: monitoring_tag.into_iter().collect(),
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 16, 10, 0), 24).unwrap()),
                network_id: dc.id,
                name: "DC Compute".to_string(),
                description: Some("Compute and hypervisor hosts".to_string()),
                subnet_type: SubnetType::Lan,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 16, 20, 0), 24).unwrap()),
                network_id: dc.id,
                name: "DC Storage".to_string(),
                description: Some("Storage network".to_string()),
                subnet_type: SubnetType::Storage,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 16, 30, 0), 24).unwrap()),
                network_id: dc.id,
                name: "DC DMZ".to_string(),
                description: Some("Demilitarized zone for public-facing services".to_string()),
                subnet_type: SubnetType::Dmz,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 18, 0, 0), 16).unwrap()),
                network_id: dc.id,
                name: "DC Docker Bridge".to_string(),
                description: Some("Docker container network".to_string()),
                subnet_type: SubnetType::DockerBridge,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 8, 0, 0), 24).unwrap()),
                network_id: dc.id,
                name: "DC VPN Tunnel".to_string(),
                description: Some("VPN tunnel to headquarters".to_string()),
                subnet_type: SubnetType::VpnTunnel,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
    ]
}

// ============================================================================
// Hosts and Services
// ============================================================================

/// Helper to create a host with a single interface.
/// Returns (Host, Interface) - host has interface_ids: vec![] initially,
/// the server will populate it after creating the interface.
#[allow(clippy::too_many_arguments)]
fn create_host(
    name: &str,
    hostname: Option<&str>,
    description: Option<&str>,
    network: &Network,
    subnet: &Subnet,
    ip: Ipv4Addr,
    tags: Vec<Uuid>,
    _snmp_credential_id: Option<Uuid>,
    virtualization: Option<HostVirtualization>,
    now: DateTime<Utc>,
) -> (Host, Interface) {
    let host_id = Uuid::new_v4();
    let interface = Interface {
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: InterfaceBase {
            network_id: network.id,
            host_id,
            subnet_id: subnet.id,
            ip_address: IpAddr::V4(ip),
            mac_address: None,
            name: Some("eth0".to_string()),
            position: 0,
        },
    };
    let host = Host {
        id: host_id,
        created_at: now,
        updated_at: now,
        base: HostBase {
            name: name.to_string(),
            network_id: network.id,
            hostname: hostname.map(String::from),
            description: description.map(String::from),
            source: EntitySource::Manual,
            virtualization,
            hidden: false,
            tags,
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
    };
    (host, interface)
}

/// Wraps a `create_host()` result to add SNMP system information fields.
fn with_snmp(
    (mut host, interface): (Host, Interface),
    sys_descr: Option<&str>,
    sys_object_id: Option<&str>,
    sys_location: Option<&str>,
    sys_contact: Option<&str>,
    chassis_id: Option<&str>,
) -> (Host, Interface) {
    host.base.sys_descr = sys_descr.map(String::from);
    host.base.sys_object_id = sys_object_id.map(String::from);
    host.base.sys_location = sys_location.map(String::from);
    host.base.sys_contact = sys_contact.map(String::from);
    host.base.chassis_id = chassis_id.map(String::from);
    (host, interface)
}

/// Helper to create a service for a host.
/// Returns (Service, Option<Port>) - the port must be added to the host's ports list.
fn create_service(
    service_def_id: &str,
    name: &str,
    host: &Host,
    interface: &Interface,
    port_type: Option<PortType>,
    tags: Vec<Uuid>,
    now: DateTime<Utc>,
) -> Option<(Service, Option<Port>)> {
    let service_definition = ServiceDefinitionRegistry::find_by_id(service_def_id)?;

    let (bindings, port) = if let Some(pt) = port_type {
        let port = Port::new_hostless(pt);
        let binding = Binding::new_port_serviceless(port.id, Some(interface.id));
        (vec![binding], Some(port))
    } else {
        let binding = Binding::new_interface_serviceless(interface.id);
        (vec![binding], None)
    };

    Some((
        Service {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id: host.id,
                network_id: host.base.network_id,
                service_definition,
                name: name.to_string(),
                bindings,
                virtualization: None,
                source: EntitySource::Manual,
                tags,
                position: 0,
            },
        },
        port,
    ))
}

/// Like `create_service` but accepts a pre-generated UUID for the service ID.
/// Used for Proxmox VE and Docker Daemon services that must have known IDs
/// before VM hosts/container services reference them.
#[allow(clippy::too_many_arguments)]
fn create_service_with_id(
    service_id: Uuid,
    service_def_id: &str,
    name: &str,
    host: &Host,
    interface: &Interface,
    port_type: Option<PortType>,
    tags: Vec<Uuid>,
    now: DateTime<Utc>,
) -> Option<(Service, Option<Port>)> {
    let service_definition = ServiceDefinitionRegistry::find_by_id(service_def_id)?;

    let (bindings, port) = if let Some(pt) = port_type {
        let port = Port::new_hostless(pt);
        let binding = Binding::new_port_serviceless(port.id, Some(interface.id));
        (vec![binding], Some(port))
    } else {
        let binding = Binding::new_interface_serviceless(interface.id);
        (vec![binding], None)
    };

    Some((
        Service {
            id: service_id,
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id: host.id,
                network_id: host.base.network_id,
                service_definition,
                name: name.to_string(),
                bindings,
                virtualization: None,
                source: EntitySource::Manual,
                tags,
                position: 0,
            },
        },
        port,
    ))
}

/// Create a Docker container service with ServiceVirtualization::Docker.
/// Binds service to the given interface (typically docker0).
#[allow(clippy::too_many_arguments)]
fn create_container_service(
    service_def_id: &str,
    name: &str,
    host: &Host,
    interface: &Interface,
    port_type: Option<PortType>,
    container_name: &str,
    container_id: &str,
    docker_daemon_svc_id: Uuid,
    tags: Vec<Uuid>,
    now: DateTime<Utc>,
) -> Option<(Service, Option<Port>)> {
    let service_definition = ServiceDefinitionRegistry::find_by_id(service_def_id)?;

    let (bindings, port) = if let Some(pt) = port_type {
        let port = Port::new_hostless(pt);
        let binding = Binding::new_port_serviceless(port.id, Some(interface.id));
        (vec![binding], Some(port))
    } else {
        let binding = Binding::new_interface_serviceless(interface.id);
        (vec![binding], None)
    };

    Some((
        Service {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id: host.id,
                network_id: host.base.network_id,
                service_definition,
                name: name.to_string(),
                bindings,
                virtualization: Some(ServiceVirtualization::Docker(DockerVirtualization {
                    container_name: Some(container_name.to_string()),
                    container_id: Some(container_id.to_string()),
                    service_id: docker_daemon_svc_id,
                })),
                source: EntitySource::Manual,
                tags,
                position: 0,
            },
        },
        port,
    ))
}

/// Helper macro to create a host with its services bundled together.
/// Ports are collected separately and bundled with the host.
/// Takes a tuple of (Host, Interface) from create_host().
macro_rules! host_with_services {
    ($host_tuple:expr, $now:expr, $( ($svc_def:expr, $svc_name:expr, $port:expr, $tags:expr) ),* $(,)?) => {{
        let (host, interface) = $host_tuple;
        let interfaces = vec![interface];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        $(
            if let Some((svc, port)) = create_service($svc_def, $svc_name, &host, &interfaces[0], $port, $tags, $now) {
                // Collect port separately if present
                if let Some(p) = port {
                    ports.push(p);
                }
                services.push(svc);
            }
        )*
        HostWithServices { host, interfaces, ports, services }
    }};
}

fn generate_hosts_and_services(
    networks: &[Network],
    subnets: &[Subnet],
    tags: &[Tag],
    credentials: &[Credential],
    now: DateTime<Utc>,
) -> Vec<HostWithServices> {
    let mut result = Vec::new();

    // Helper to find entities
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };
    let find_subnet = |name: &str| subnets.iter().find(|s| s.base.name.contains(name)).unwrap();
    let find_tag = |name: &str| tags.iter().find(|t| t.base.name == name).map(|t| t.id);

    let network_devices_cred = credentials
        .iter()
        .find(|c| c.base.name == "Network Devices")
        .map(|c| c.id);
    let docker_proxy_cred = credentials
        .iter()
        .find(|c| c.base.name == "Docker TLS Proxy")
        .map(|c| c.id);

    let critical_tag = find_tag("Critical");
    let production_tag = find_tag("Production");
    let database_tag = find_tag("Database");
    let monitoring_tag = find_tag("Monitoring");
    let iot_tag = find_tag("IoT Device");
    let web_tier_tag = find_tag("Web Tier");
    let backup_tag = find_tag("Backup Target");

    // Pre-generated service UUIDs for virtualization wiring
    let pve_hq1_svc_id = Uuid::new_v4(); // Proxmox VE on proxmox-hv01
    let pve_hq2_svc_id = Uuid::new_v4(); // Proxmox VE on proxmox-hv02
    let docker_hq_svc_id = Uuid::new_v4(); // Docker daemon on docker-prod01
    let pve_dc_svc_id = Uuid::new_v4(); // Proxmox VE on dc-proxmox-hv01
    let docker_dc_svc_id = Uuid::new_v4(); // Docker daemon on dc-docker01

    // ========================================================================
    // HEADQUARTERS NETWORK — 30 hosts
    // ========================================================================
    let hq = find_network("Headquarters");
    let hq_mgmt = find_subnet("HQ Management");
    let hq_servers = find_subnet("HQ Servers");
    let hq_storage = find_subnet("HQ Storage");
    let hq_lan = find_subnet("HQ Office LAN");
    let hq_iot = find_subnet("HQ IoT");
    let hq_docker = find_subnet("HQ Docker Bridge");
    let hq_guest = find_subnet("HQ Guest WiFi");

    // -- Management (10.0.1.x) --

    // 1. pfSense Firewall (Critical) — with host-level SNMP credential override
    let mut pfsense = host_with_services!(
        with_snmp(
            create_host(
                "pfsense-fw01",
                Some("pfsense-fw01.acme.local"),
                Some("Primary pfSense firewall"),
                hq,
                hq_mgmt,
                Ipv4Addr::new(10, 0, 1, 1),
                critical_tag.into_iter().collect(),
                network_devices_cred,
                None,
                now,
            ),
            Some("pfSense 2.7.0-RELEASE (amd64) built on FreeBSD 14.0-CURRENT"),
            Some("1.3.6.1.4.1.12325.1.1"),
            Some("HQ Server Room, Rack A1"),
            Some("netops@acme-corp.com"),
            None,
        ),
        now,
        (
            "pfSense",
            "pfSense",
            Some(PortType::Https),
            critical_tag.into_iter().collect()
        ),
    );
    pfsense.host.base.credential_assignments = network_devices_cred
        .into_iter()
        .map(|id| CredentialAssignment {
            credential_id: id,
            interface_ids: None,
        })
        .collect();
    result.push(pfsense);

    // 2. UniFi Controller
    result.push(host_with_services!(
        create_host(
            "unifi-controller",
            Some("unifi.acme.local"),
            Some("UniFi Network Controller"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 10),
            vec![],
            None,
            None,
            now
        ),
        now,
        (
            "UniFi Controller",
            "UniFi Controller",
            Some(PortType::Https8443),
            vec![]
        ),
    ));

    // 3. Core switch (48 ports, SNMP/LLDP)
    result.push(host_with_services!(
        with_snmp(
            create_host(
                "unifi-usw-48",
                Some("switch.acme.local"),
                Some("UniFi Switch 48 PoE"),
                hq,
                hq_mgmt,
                Ipv4Addr::new(10, 0, 1, 3),
                vec![],
                network_devices_cred,
                None,
                now,
            ),
            Some("UniFi USW-48-PoE, 6.6.65, Linux 5.4.0"),
            Some("1.3.6.1.4.1.41112.1.6"),
            Some("HQ Server Room, Rack A2"),
            Some("netops@acme-corp.com"),
            Some("78:45:c4:ab:cd:01"),
        ),
        now,
        ("SNMP", "SNMP", Some(PortType::Snmp), vec![]),
    ));

    // 4. Pi-hole DNS
    result.push(host_with_services!(
        create_host(
            "pihole-dns01",
            Some("pihole.acme.local"),
            Some("Pi-hole DNS ad blocker"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 5),
            vec![],
            None,
            None,
            now
        ),
        now,
        ("Pi-Hole", "Pi-hole", Some(PortType::Http), vec![]),
    ));

    // 5. Grafana
    result.push(host_with_services!(
        create_host(
            "grafana-mon",
            Some("grafana.acme.local"),
            Some("Grafana monitoring dashboard"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 50),
            monitoring_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Grafana",
            "Grafana",
            Some(PortType::Http3000),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // 6. Prometheus
    result.push(host_with_services!(
        create_host(
            "prometheus",
            Some("prometheus.acme.local"),
            Some("Prometheus metrics server"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 51),
            monitoring_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Prometheus",
            "Prometheus",
            Some(PortType::Http9000),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // 7. Uptime Kuma
    result.push(host_with_services!(
        create_host(
            "uptime-kuma",
            Some("status.acme.local"),
            Some("Uptime Kuma status page"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 52),
            monitoring_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "UptimeKuma",
            "Uptime Kuma",
            Some(PortType::Http3000),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // -- Servers (10.0.20.x) — hypervisors, VMs, Docker --

    // 8. Proxmox Hypervisor 1 (pre-generated Proxmox VE service ID)
    {
        let (host, interface) = with_snmp(
            create_host(
                "proxmox-hv01",
                Some("proxmox-hv01.acme.local"),
                Some("Proxmox hypervisor node 1"),
                hq,
                hq_servers,
                Ipv4Addr::new(10, 0, 20, 5),
                production_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            Some("Linux proxmox-hv01 6.8.12-1-pve #1 SMP PVE 6.8.12-1 x86_64"),
            Some("1.3.6.1.4.1.8072.3.2.10"),
            Some("HQ Server Room, Rack B1"),
            Some("sysadmin@acme-corp.com"),
            None,
        );
        let interfaces = vec![interface];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((svc, port)) = create_service_with_id(
            pve_hq1_svc_id,
            "Proxmox VE",
            "Proxmox VE",
            &host,
            &interfaces[0],
            Some(PortType::Https8443),
            production_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        if let Some((svc, port)) = create_service(
            "SSH",
            "SSH",
            &host,
            &interfaces[0],
            Some(PortType::Ssh),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            interfaces,
            ports,
            services,
        });
    }

    // 9. Proxmox Hypervisor 2 (pre-generated Proxmox VE service ID)
    {
        let (host, interface) = with_snmp(
            create_host(
                "proxmox-hv02",
                Some("proxmox-hv02.acme.local"),
                Some("Proxmox hypervisor node 2"),
                hq,
                hq_servers,
                Ipv4Addr::new(10, 0, 20, 6),
                production_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            Some("Linux proxmox-hv02 6.8.12-1-pve #1 SMP PVE 6.8.12-1 x86_64"),
            Some("1.3.6.1.4.1.8072.3.2.10"),
            Some("HQ Server Room, Rack B2"),
            Some("sysadmin@acme-corp.com"),
            None,
        );
        let interfaces = vec![interface];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((svc, port)) = create_service_with_id(
            pve_hq2_svc_id,
            "Proxmox VE",
            "Proxmox VE",
            &host,
            &interfaces[0],
            Some(PortType::Https8443),
            production_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        if let Some((svc, port)) = create_service(
            "SSH",
            "SSH",
            &host,
            &interfaces[0],
            Some(PortType::Ssh),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            interfaces,
            ports,
            services,
        });
    }

    // 10. gitlab-vm — VM on hv01 (vm_id=100)
    result.push(host_with_services!(
        create_host(
            "gitlab-vm",
            Some("gitlab.acme.local"),
            Some("GitLab instance (VM on proxmox-hv01)"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 10),
            production_tag.into_iter().collect(),
            None,
            Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                vm_name: Some("gitlab-vm".to_string()),
                vm_id: Some("100".to_string()),
                service_id: pve_hq1_svc_id,
            })),
            now
        ),
        now,
        (
            "GitLab",
            "GitLab",
            Some(PortType::Https),
            production_tag.into_iter().collect()
        ),
    ));

    // 11. nextcloud-vm — VM on hv01 (vm_id=101)
    result.push(host_with_services!(
        create_host(
            "nextcloud-vm",
            Some("cloud.acme.local"),
            Some("Nextcloud file sharing (VM on proxmox-hv01)"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 11),
            production_tag.into_iter().collect(),
            None,
            Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                vm_name: Some("nextcloud-vm".to_string()),
                vm_id: Some("101".to_string()),
                service_id: pve_hq1_svc_id,
            })),
            now
        ),
        now,
        (
            "NextCloud",
            "Nextcloud",
            Some(PortType::Https),
            production_tag.into_iter().collect()
        ),
    ));

    // 12. keycloak-vm — VM on hv02 (vm_id=200)
    result.push(host_with_services!(
        create_host(
            "keycloak-vm",
            Some("keycloak.acme.local"),
            Some("Keycloak SSO (VM on proxmox-hv02)"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 12),
            production_tag.into_iter().collect(),
            None,
            Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                vm_name: Some("keycloak-vm".to_string()),
                vm_id: Some("200".to_string()),
                service_id: pve_hq2_svc_id,
            })),
            now
        ),
        now,
        (
            "Keycloak",
            "Keycloak",
            Some(PortType::Https8443),
            production_tag.into_iter().collect()
        ),
    ));

    // 13. docker-prod01 — Docker host (2 interfaces: eth0 on Servers, docker0 on DockerBridge)
    {
        let host_id = Uuid::new_v4();
        let eth0 = Interface {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                network_id: hq.id,
                host_id,
                subnet_id: hq_servers.id,
                ip_address: IpAddr::V4(Ipv4Addr::new(10, 0, 20, 20)),
                mac_address: None,
                name: Some("eth0".to_string()),
                position: 0,
            },
        };
        let docker0 = Interface {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                network_id: hq.id,
                host_id,
                subnet_id: hq_docker.id,
                ip_address: IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1)),
                mac_address: None,
                name: Some("docker0".to_string()),
                position: 1,
            },
        };
        let host = Host {
            id: host_id,
            created_at: now,
            updated_at: now,
            base: HostBase {
                name: "docker-prod01".to_string(),
                network_id: hq.id,
                hostname: Some("docker-prod01.acme.local".to_string()),
                description: Some("Production Docker host".to_string()),
                source: EntitySource::Manual,
                virtualization: None,
                hidden: false,
                tags: production_tag.into_iter().collect(),
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
                credential_assignments: docker_proxy_cred
                    .into_iter()
                    .map(|id| CredentialAssignment {
                        credential_id: id,
                        interface_ids: None,
                    })
                    .collect(),
            },
        };

        let mut ports = Vec::new();
        let mut services = Vec::new();

        // Docker Daemon service with pre-generated ID on eth0
        if let Some((svc, port)) = create_service_with_id(
            docker_hq_svc_id,
            "Docker",
            "Docker Daemon",
            &host,
            &eth0,
            Some(PortType::Docker),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        // Portainer on eth0
        if let Some((svc, port)) = create_service(
            "Portainer",
            "Portainer",
            &host,
            &eth0,
            Some(PortType::Http9000),
            production_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }

        // Container services on docker0
        for (def_id, name, pt, cname, cid) in [
            (
                "Traefik",
                "Traefik",
                Some(PortType::Https),
                "traefik",
                "a1b2c3d4e5f6",
            ),
            (
                "Vaultwarden",
                "Vaultwarden",
                Some(PortType::Http8080),
                "vaultwarden",
                "d4e5f6a7b8c9",
            ),
            (
                "Gitea",
                "Gitea",
                Some(PortType::Http3000),
                "gitea",
                "g7h8i9j0k1l2",
            ),
            (
                "mailcow",
                "mailcow",
                Some(PortType::Https8443),
                "mailcow",
                "j0k1l2m3n4o5",
            ),
        ] {
            if let Some((svc, port)) = create_container_service(
                def_id,
                name,
                &host,
                &docker0,
                pt,
                cname,
                cid,
                docker_hq_svc_id,
                vec![],
                now,
            ) {
                if let Some(p) = port {
                    ports.push(p);
                }
                services.push(svc);
            }
        }

        result.push(HostWithServices {
            host,
            interfaces: vec![eth0, docker0],
            ports,
            services,
        });
    }

    // 14. Jenkins CI
    result.push(host_with_services!(
        create_host(
            "jenkins-ci",
            Some("jenkins.acme.local"),
            Some("Jenkins CI/CD server"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 30),
            production_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Jenkins",
            "Jenkins",
            Some(PortType::Http8080),
            production_tag.into_iter().collect()
        ),
    ));

    // 15. WireGuard VPN
    result.push(host_with_services!(
        create_host(
            "wireguard-vpn",
            Some("vpn.acme.local"),
            Some("WireGuard VPN server"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 35),
            vec![],
            None,
            None,
            now
        ),
        now,
        (
            "WireGuard",
            "WireGuard VPN",
            Some(PortType::Wireguard),
            vec![]
        ),
    ));

    // -- Storage (10.0.40.x) --

    // 16. db-vm — VM on hv02 (vm_id=201)
    result.push(host_with_services!(
        create_host(
            "db-vm",
            Some("db.acme.local"),
            Some("Database server (VM on proxmox-hv02)"),
            hq,
            hq_storage,
            Ipv4Addr::new(10, 0, 40, 10),
            database_tag.into_iter().chain(critical_tag).collect(),
            None,
            Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                vm_name: Some("db-vm".to_string()),
                vm_id: Some("201".to_string()),
                service_id: pve_hq2_svc_id,
            })),
            now
        ),
        now,
        (
            "PostgreSQL",
            "PostgreSQL",
            Some(PortType::PostgreSQL),
            database_tag.into_iter().collect()
        ),
        (
            "Redis",
            "Redis",
            Some(PortType::Redis),
            database_tag.into_iter().collect()
        ),
    ));

    // 17. TrueNAS Primary
    result.push(host_with_services!(
        with_snmp(
            create_host(
                "truenas-primary",
                Some("truenas.acme.local"),
                Some("Primary NAS storage"),
                hq,
                hq_storage,
                Ipv4Addr::new(10, 0, 40, 20),
                critical_tag.into_iter().chain(backup_tag).collect(),
                None,
                None,
                now,
            ),
            Some("TrueNAS SCALE 24.04 (Dragonfish) - Kernel 6.6.44-production+truenas"),
            Some("1.3.6.1.4.1.50536.3"),
            Some("HQ Server Room, Rack C1"),
            Some("sysadmin@acme-corp.com"),
            None,
        ),
        now,
        (
            "TrueNAS",
            "TrueNAS",
            Some(PortType::Https),
            backup_tag.into_iter().collect()
        ),
        ("NFS", "NFS", Some(PortType::Nfs), vec![]),
    ));

    // 18. Synology Backup
    result.push(host_with_services!(
        with_snmp(
            create_host(
                "synology-backup",
                Some("synology.acme.local"),
                Some("Synology backup NAS"),
                hq,
                hq_storage,
                Ipv4Addr::new(10, 0, 40, 21),
                backup_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            Some("Synology NAS DS1621+ DSM 7.2.1-69057 Update 5"),
            Some("1.3.6.1.4.1.6574.1"),
            Some("HQ Server Room, Rack C2"),
            Some("sysadmin@acme-corp.com"),
            None,
        ),
        now,
        (
            "Synology DSM",
            "Synology DSM",
            Some(PortType::Https),
            backup_tag.into_iter().collect()
        ),
    ));

    // -- Office LAN (10.0.10.x) --

    // 19-22. Workstations
    for (name, hostname, desc, ip_last) in [
        (
            "ws-engineering-01",
            "ws-eng-01.acme.local",
            "Engineering workstation 1",
            101,
        ),
        (
            "ws-engineering-02",
            "ws-eng-02.acme.local",
            "Engineering workstation 2",
            102,
        ),
        (
            "ws-accounting-01",
            "ws-acct-01.acme.local",
            "Accounting workstation",
            103,
        ),
        ("ws-hr-01", "ws-hr-01.acme.local", "HR workstation", 104),
    ] {
        result.push(host_with_services!(
            create_host(
                name,
                Some(hostname),
                Some(desc),
                hq,
                hq_lan,
                Ipv4Addr::new(10, 0, 10, ip_last),
                vec![],
                None,
                None,
                now
            ),
            now,
            ("Workstation", "Workstation", Some(PortType::Rdp), vec![]),
        ));
    }

    // -- IoT (10.0.30.x) --

    // 23. UniFi AP Lobby
    result.push(host_with_services!(
        with_snmp(
            create_host(
                "unifi-ap-lobby",
                Some("ap-lobby.acme.local"),
                Some("UniFi AP - Main Lobby"),
                hq,
                hq_iot,
                Ipv4Addr::new(10, 0, 30, 100),
                iot_tag.into_iter().collect(),
                network_devices_cred,
                None,
                now,
            ),
            Some("UniFi U6-Pro, 6.6.65, Linux 5.4.0"),
            Some("1.3.6.1.4.1.41112.1.6"),
            Some("HQ Main Lobby, Ceiling Mount"),
            Some("netops@acme-corp.com"),
            Some("fc:ec:da:aa:bb:01"),
        ),
        now,
        (
            "Unifi Access Point",
            "UniFi AP",
            None,
            iot_tag.into_iter().collect()
        ),
    ));

    // 24. UniFi AP Floor 2
    result.push(host_with_services!(
        with_snmp(
            create_host(
                "unifi-ap-floor2",
                Some("ap-floor2.acme.local"),
                Some("UniFi AP - Floor 2"),
                hq,
                hq_iot,
                Ipv4Addr::new(10, 0, 30, 101),
                iot_tag.into_iter().collect(),
                network_devices_cred,
                None,
                now,
            ),
            Some("UniFi U6-LR, 6.6.65, Linux 5.4.0"),
            Some("1.3.6.1.4.1.41112.1.6"),
            Some("HQ Floor 2, Hallway Ceiling"),
            Some("netops@acme-corp.com"),
            Some("fc:ec:da:aa:bb:02"),
        ),
        now,
        (
            "Unifi Access Point",
            "UniFi AP",
            None,
            iot_tag.into_iter().collect()
        ),
    ));

    // 25. Hue Bridge
    result.push(host_with_services!(
        create_host(
            "hue-bridge",
            None,
            Some("Philips Hue Bridge"),
            hq,
            hq_iot,
            Ipv4Addr::new(10, 0, 30, 10),
            iot_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Philips Hue Bridge",
            "Philips Hue",
            Some(PortType::Https),
            iot_tag.into_iter().collect()
        ),
    ));

    // 26. HP Printer
    result.push(host_with_services!(
        with_snmp(
            create_host(
                "printer-hp-main",
                None,
                Some("HP LaserJet Pro"),
                hq,
                hq_iot,
                Ipv4Addr::new(10, 0, 30, 50),
                iot_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            Some("HP LaserJet Pro MFP M428fdw, Firmware 20230809"),
            Some("1.3.6.1.4.1.11.2.3.9.1"),
            Some("HQ Floor 1, Copy Room"),
            Some("helpdesk@acme-corp.com"),
            None,
        ),
        now,
        (
            "HP Printer",
            "HP Printer",
            Some(PortType::Ipp),
            iot_tag.into_iter().collect()
        ),
    ));

    // 27. Camera Entrance
    result.push(host_with_services!(
        create_host(
            "cam-entrance",
            None,
            Some("Entrance security camera"),
            hq,
            hq_iot,
            Ipv4Addr::new(10, 0, 30, 60),
            iot_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "RTSP Camera",
            "Security Camera",
            Some(PortType::Rtsp),
            iot_tag.into_iter().collect()
        ),
    ));

    // 28. Camera Parking
    result.push(host_with_services!(
        create_host(
            "cam-parking",
            None,
            Some("Parking lot security camera"),
            hq,
            hq_iot,
            Ipv4Addr::new(10, 0, 30, 61),
            iot_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "RTSP Camera",
            "Security Camera",
            Some(PortType::Rtsp),
            iot_tag.into_iter().collect()
        ),
    ));

    // -- Guest WiFi (10.0.100.x) --

    // 29. Guest AP
    result.push(host_with_services!(
        with_snmp(
            create_host(
                "guest-ap",
                Some("guest-ap.acme.local"),
                Some("Guest WiFi access point"),
                hq,
                hq_guest,
                Ipv4Addr::new(10, 0, 100, 1),
                iot_tag.into_iter().collect(),
                network_devices_cred,
                None,
                now,
            ),
            Some("UniFi U6-Lite, 6.6.65, Linux 5.4.0"),
            Some("1.3.6.1.4.1.41112.1.6"),
            Some("HQ Guest Lobby, Ceiling Mount"),
            Some("netops@acme-corp.com"),
            Some("fc:ec:da:aa:bb:03"),
        ),
        now,
        (
            "Unifi Access Point",
            "UniFi AP",
            None,
            iot_tag.into_iter().collect()
        ),
    ));

    // 30. Bind9 secondary DNS (Management)
    result.push(host_with_services!(
        create_host(
            "bind9-dns",
            Some("bind9.acme.local"),
            Some("Bind9 secondary DNS server"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 6),
            vec![],
            None,
            None,
            now
        ),
        now,
        ("Bind9", "Bind9", Some(PortType::DnsUdp), vec![]),
    ));

    // ========================================================================
    // DATA CENTER NETWORK — 20 hosts
    // ========================================================================
    let dc = find_network("Data Center");
    let dc_mgmt = find_subnet("DC Management");
    let dc_compute = find_subnet("DC Compute");
    let dc_storage = find_subnet("DC Storage");
    let dc_dmz = find_subnet("DC DMZ");
    let dc_docker = find_subnet("DC Docker Bridge");
    let dc_vpn = find_subnet("DC VPN");

    // -- Management (172.16.0.x) --

    // 1. DC Firewall
    result.push(host_with_services!(
        with_snmp(
            create_host(
                "dc-fw01",
                Some("fw01.dc.acme.io"),
                Some("Data center firewall"),
                dc,
                dc_mgmt,
                Ipv4Addr::new(172, 16, 0, 1),
                critical_tag.into_iter().collect(),
                network_devices_cred,
                None,
                now,
            ),
            Some("FortiGate-60F v7.4.3, build 2573, 240514 (GA.F)"),
            Some("1.3.6.1.4.1.12356.101.1"),
            Some("DC-East, Cage 4, Rack 1"),
            Some("netops@acme-corp.com"),
            None,
        ),
        now,
        (
            "Fortinet",
            "FortiGate",
            Some(PortType::Https),
            critical_tag.into_iter().collect()
        ),
    ));

    // 2. DC Switch (24 ports, LLDP)
    result.push(host_with_services!(
        with_snmp(
            create_host(
                "dc-switch-01",
                Some("switch-01.dc.acme.io"),
                Some("Data center managed switch"),
                dc,
                dc_mgmt,
                Ipv4Addr::new(172, 16, 0, 2),
                vec![],
                network_devices_cred,
                None,
                now,
            ),
            Some("Arista DCS-7050SX3-48YC12, EOS-4.32.0F"),
            Some("1.3.6.1.4.1.30065.1.3011.7050.3735.48.3328.12"),
            Some("DC-East, Cage 4, Rack 2"),
            Some("netops@acme-corp.com"),
            Some("78:45:c4:ab:cd:02"),
        ),
        now,
        ("SNMP", "SNMP", Some(PortType::Snmp), vec![]),
        ("Switch", "Switch", None, vec![]),
    ));

    // 3. Zabbix Monitoring
    result.push(host_with_services!(
        create_host(
            "zabbix-mon",
            Some("zabbix.dc.acme.io"),
            Some("Zabbix monitoring server"),
            dc,
            dc_mgmt,
            Ipv4Addr::new(172, 16, 0, 10),
            monitoring_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Zabbix",
            "Zabbix",
            Some(PortType::Http8080),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // -- DMZ (172.16.30.x) --

    // 4. HAProxy Load Balancer
    result.push(host_with_services!(
        create_host(
            "haproxy-lb01",
            Some("lb01.dc.acme.io"),
            Some("HAProxy load balancer"),
            dc,
            dc_dmz,
            Ipv4Addr::new(172, 16, 30, 10),
            production_tag
                .into_iter()
                .chain(critical_tag)
                .chain(web_tier_tag)
                .collect(),
            None,
            None,
            now
        ),
        now,
        (
            "HAProxy",
            "HAProxy",
            Some(PortType::Https),
            web_tier_tag.into_iter().collect()
        ),
    ));

    // 5. App Server 01
    result.push(host_with_services!(
        create_host(
            "app-server-01",
            Some("app-01.dc.acme.io"),
            Some("Application server 1"),
            dc,
            dc_dmz,
            Ipv4Addr::new(172, 16, 30, 20),
            production_tag.into_iter().chain(web_tier_tag).collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Web Service",
            "Web Application",
            Some(PortType::Http8080),
            web_tier_tag.into_iter().collect()
        ),
        ("SSH", "SSH", Some(PortType::Ssh), vec![]),
    ));

    // 6. App Server 02
    result.push(host_with_services!(
        create_host(
            "app-server-02",
            Some("app-02.dc.acme.io"),
            Some("Application server 2"),
            dc,
            dc_dmz,
            Ipv4Addr::new(172, 16, 30, 21),
            production_tag.into_iter().chain(web_tier_tag).collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Web Service",
            "Web Application",
            Some(PortType::Http8080),
            web_tier_tag.into_iter().collect()
        ),
        ("SSH", "SSH", Some(PortType::Ssh), vec![]),
    ));

    // -- Compute (172.16.10.x) --

    // 7. DC Proxmox Hypervisor (pre-generated Proxmox VE service ID)
    {
        let (host, interface) = with_snmp(
            create_host(
                "dc-proxmox-hv01",
                Some("proxmox-hv01.dc.acme.io"),
                Some("Data center Proxmox hypervisor"),
                dc,
                dc_compute,
                Ipv4Addr::new(172, 16, 10, 5),
                production_tag.into_iter().collect(),
                None,
                None,
                now,
            ),
            Some("Linux dc-proxmox-hv01 6.8.12-1-pve #1 SMP PVE 6.8.12-1 x86_64"),
            Some("1.3.6.1.4.1.8072.3.2.10"),
            Some("DC-East, Cage 4, Rack 3"),
            Some("sysadmin@acme-corp.com"),
            None,
        );
        let interfaces = vec![interface];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        if let Some((svc, port)) = create_service_with_id(
            pve_dc_svc_id,
            "Proxmox VE",
            "Proxmox VE",
            &host,
            &interfaces[0],
            Some(PortType::Https8443),
            production_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        if let Some((svc, port)) = create_service(
            "SSH",
            "SSH",
            &host,
            &interfaces[0],
            Some(PortType::Ssh),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        result.push(HostWithServices {
            host,
            interfaces,
            ports,
            services,
        });
    }

    // 8. argocd-vm — VM on dc-proxmox-hv01 (vm_id=300)
    result.push(host_with_services!(
        create_host(
            "argocd-vm",
            Some("argocd.dc.acme.io"),
            Some("ArgoCD (VM on dc-proxmox-hv01)"),
            dc,
            dc_compute,
            Ipv4Addr::new(172, 16, 10, 10),
            production_tag.into_iter().collect(),
            None,
            Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                vm_name: Some("argocd-vm".to_string()),
                vm_id: Some("300".to_string()),
                service_id: pve_dc_svc_id,
            })),
            now
        ),
        now,
        (
            "ArgoCD",
            "ArgoCD",
            Some(PortType::Https8443),
            production_tag.into_iter().collect()
        ),
    ));

    // 9. graylog-vm — VM on dc-proxmox-hv01 (vm_id=301)
    result.push(host_with_services!(
        create_host(
            "graylog-vm",
            Some("graylog.dc.acme.io"),
            Some("Graylog log management (VM on dc-proxmox-hv01)"),
            dc,
            dc_compute,
            Ipv4Addr::new(172, 16, 10, 11),
            monitoring_tag.into_iter().collect(),
            None,
            Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                vm_name: Some("graylog-vm".to_string()),
                vm_id: Some("301".to_string()),
                service_id: pve_dc_svc_id,
            })),
            now
        ),
        now,
        (
            "Graylog",
            "Graylog",
            Some(PortType::Http9000),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // 10. mariadb-vm — VM on dc-proxmox-hv01 (vm_id=302, on Storage subnet)
    result.push(host_with_services!(
        create_host(
            "mariadb-vm",
            Some("mariadb.dc.acme.io"),
            Some("MariaDB database (VM on dc-proxmox-hv01)"),
            dc,
            dc_storage,
            Ipv4Addr::new(172, 16, 20, 10),
            database_tag.into_iter().collect(),
            None,
            Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
                vm_name: Some("mariadb-vm".to_string()),
                vm_id: Some("302".to_string()),
                service_id: pve_dc_svc_id,
            })),
            now
        ),
        now,
        (
            "MariaDB",
            "MariaDB",
            Some(PortType::MySql),
            database_tag.into_iter().collect()
        ),
    ));

    // 11. dc-docker01 — Docker host (2 interfaces: eth0 on Compute, docker0 on DC Docker Bridge)
    {
        let host_id = Uuid::new_v4();
        let eth0 = Interface {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                network_id: dc.id,
                host_id,
                subnet_id: dc_compute.id,
                ip_address: IpAddr::V4(Ipv4Addr::new(172, 16, 10, 20)),
                mac_address: None,
                name: Some("eth0".to_string()),
                position: 0,
            },
        };
        let docker0 = Interface {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: InterfaceBase {
                network_id: dc.id,
                host_id,
                subnet_id: dc_docker.id,
                ip_address: IpAddr::V4(Ipv4Addr::new(172, 18, 0, 1)),
                mac_address: None,
                name: Some("docker0".to_string()),
                position: 1,
            },
        };
        let host = Host {
            id: host_id,
            created_at: now,
            updated_at: now,
            base: HostBase {
                name: "dc-docker01".to_string(),
                network_id: dc.id,
                hostname: Some("docker01.dc.acme.io".to_string()),
                description: Some("Data center Docker host".to_string()),
                source: EntitySource::Manual,
                virtualization: None,
                hidden: false,
                tags: production_tag.into_iter().collect(),
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
                credential_assignments: docker_proxy_cred
                    .into_iter()
                    .map(|id| CredentialAssignment {
                        credential_id: id,
                        interface_ids: None,
                    })
                    .collect(),
            },
        };

        let mut ports = Vec::new();
        let mut services = Vec::new();

        // Docker Daemon service with pre-generated ID on eth0
        if let Some((svc, port)) = create_service_with_id(
            docker_dc_svc_id,
            "Docker",
            "Docker Daemon",
            &host,
            &eth0,
            Some(PortType::Docker),
            vec![],
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }
        // Portainer on eth0
        if let Some((svc, port)) = create_service(
            "Portainer",
            "Portainer",
            &host,
            &eth0,
            Some(PortType::Http9000),
            production_tag.into_iter().collect(),
            now,
        ) {
            if let Some(p) = port {
                ports.push(p);
            }
            services.push(svc);
        }

        // Container services on docker0
        for (def_id, name, pt, cname, cid) in [
            (
                "Prometheus",
                "Prometheus",
                Some(PortType::new_tcp(9090)),
                "prometheus",
                "p1r2o3m4e5t6",
            ),
            (
                "Grafana",
                "Grafana",
                Some(PortType::Http3000),
                "grafana",
                "g1r2a3f4a5n6",
            ),
            (
                "Jaeger",
                "Jaeger",
                Some(PortType::Https),
                "jaeger",
                "j1a2e3g4e5r6",
            ),
            (
                "Loki",
                "Loki",
                Some(PortType::new_tcp(3100)),
                "loki",
                "l1o2k3i4d5c6",
            ),
        ] {
            if let Some((svc, port)) = create_container_service(
                def_id,
                name,
                &host,
                &docker0,
                pt,
                cname,
                cid,
                docker_dc_svc_id,
                monitoring_tag.into_iter().collect(),
                now,
            ) {
                if let Some(p) = port {
                    ports.push(p);
                }
                services.push(svc);
            }
        }

        result.push(HostWithServices {
            host,
            interfaces: vec![eth0, docker0],
            ports,
            services,
        });
    }

    // 12. RabbitMQ
    result.push(host_with_services!(
        create_host(
            "rabbitmq-node01",
            Some("mq.dc.acme.io"),
            Some("RabbitMQ message broker"),
            dc,
            dc_compute,
            Ipv4Addr::new(172, 16, 10, 30),
            production_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "RabbitMQ",
            "RabbitMQ",
            Some(PortType::AMQP),
            production_tag.into_iter().collect()
        ),
    ));

    // 13. Redis Cluster
    result.push(host_with_services!(
        create_host(
            "redis-cluster01",
            Some("redis.dc.acme.io"),
            Some("Redis cluster node"),
            dc,
            dc_compute,
            Ipv4Addr::new(172, 16, 10, 31),
            database_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Redis",
            "Redis",
            Some(PortType::Redis),
            database_tag.into_iter().collect()
        ),
    ));

    // -- Storage (172.16.20.x) --

    // 14. MinIO
    result.push(host_with_services!(
        create_host(
            "minio-storage",
            Some("minio.dc.acme.io"),
            Some("MinIO object storage"),
            dc,
            dc_storage,
            Ipv4Addr::new(172, 16, 20, 20),
            backup_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "MinIO",
            "MinIO",
            Some(PortType::Https),
            backup_tag.into_iter().collect()
        ),
    ));

    // 15. Ceph
    result.push(host_with_services!(
        create_host(
            "ceph-node01",
            Some("ceph.dc.acme.io"),
            Some("Ceph storage node"),
            dc,
            dc_storage,
            Ipv4Addr::new(172, 16, 20, 21),
            backup_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        ("Ceph", "Ceph", None, backup_tag.into_iter().collect()),
    ));

    // 16. Elasticsearch
    result.push(host_with_services!(
        create_host(
            "elasticsearch-dc",
            Some("es.dc.acme.io"),
            Some("Elasticsearch cluster"),
            dc,
            dc_storage,
            Ipv4Addr::new(172, 16, 20, 30),
            database_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Elasticsearch",
            "Elasticsearch",
            Some(PortType::Elasticsearch),
            database_tag.into_iter().collect()
        ),
    ));

    // 17. InfluxDB
    result.push(host_with_services!(
        create_host(
            "influxdb-metrics",
            Some("influxdb.dc.acme.io"),
            Some("InfluxDB metrics store"),
            dc,
            dc_storage,
            Ipv4Addr::new(172, 16, 20, 31),
            database_tag.into_iter().chain(monitoring_tag).collect(),
            None,
            None,
            now
        ),
        now,
        (
            "InfluxDB",
            "InfluxDB",
            Some(PortType::InfluxDb),
            database_tag.into_iter().collect()
        ),
    ));

    // -- VPN Tunnel (10.8.0.x) --

    // 18. DC VPN
    result.push(host_with_services!(
        create_host(
            "dc-vpn",
            Some("vpn.dc.acme.io"),
            Some("Data center VPN endpoint"),
            dc,
            dc_vpn,
            Ipv4Addr::new(10, 8, 0, 1),
            vec![],
            None,
            None,
            now
        ),
        now,
        ("OpenVPN", "OpenVPN", Some(PortType::OpenVPN), vec![]),
    ));

    // -- Additional --

    // 19. Cloudflared Tunnel (DMZ)
    result.push(host_with_services!(
        create_host(
            "cloudflared-tunnel",
            Some("cloudflared.dc.acme.io"),
            Some("Cloudflare tunnel endpoint"),
            dc,
            dc_dmz,
            Ipv4Addr::new(172, 16, 30, 5),
            production_tag.into_iter().collect(),
            None,
            None,
            now
        ),
        now,
        (
            "Cloudflared",
            "Cloudflared",
            Some(PortType::Https),
            production_tag.into_iter().collect()
        ),
    ));

    // 20. DC Admin Workstation (Compute)
    result.push(host_with_services!(
        create_host(
            "dc-ws-admin",
            Some("ws-admin.dc.acme.io"),
            Some("DC admin workstation"),
            dc,
            dc_compute,
            Ipv4Addr::new(172, 16, 10, 100),
            vec![],
            None,
            None,
            now
        ),
        now,
        ("Workstation", "Workstation", Some(PortType::Rdp), vec![]),
        ("SSH", "SSH", Some(PortType::Ssh), vec![]),
    ));

    result
}

// ============================================================================
// IfEntries (SNMP Interface Data)
// ============================================================================

fn generate_if_entries(
    networks: &[Network],
    hosts: &[&Host],
    interfaces: &[&Interface],
    now: DateTime<Utc>,
) -> (Vec<IfEntry>, Vec<NeighborUpdate>) {
    let mut if_entries = Vec::new();
    let mut neighbor_updates = Vec::new();

    let find_host = |name: &str| hosts.iter().find(|h| h.base.name == name).copied();
    let find_interface = |host_id: Uuid| {
        interfaces
            .iter()
            .find(|i| i.base.host_id == host_id)
            .copied()
    };

    // HQ switch MAC (used as chassis ID)
    let hq_switch_mac = "78:45:c4:ab:cd:01";
    // DC switch MAC
    let dc_switch_mac = "78:45:c4:ab:cd:02";

    // ========================================================================
    // HQ: pfSense firewall — multiple interfaces
    // ========================================================================
    if let Some(host) = find_host("pfsense-fw01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        // WAN interface
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "igb0".to_string(),
                if_name: None,
                if_alias: Some("WAN".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
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
        });

        // LAN interface — connected to HQ switch port 1
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 2,
                if_descr: "igb1".to_string(),
                if_name: None,
                if_alias: Some("LAN".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(hq_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/1".to_string())),
                lldp_sys_name: Some("unifi-usw-48".to_string()),
                lldp_port_desc: Some("Port 1 - pfSense uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 3))),
                lldp_sys_desc: Some("UniFi USW-48-PoE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "pfsense-fw01".to_string(),
            source_if_index: 2,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 1,
        });

        // OPT1 interface (disabled)
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 3,
                if_descr: "igb2".to_string(),
                if_name: None,
                if_alias: Some("OPT1".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Down,
                oper_status: IfOperStatus::Down,
                mac_address: None,
                interface_id: None,
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
        });
    }

    // ========================================================================
    // HQ: TrueNAS — bonded interfaces, connected to switch port 2
    // ========================================================================
    if let Some(host) = find_host("truenas-primary") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "lagg0".to_string(),
                if_name: None,
                if_alias: Some("LACP Bond".to_string()),
                if_type: 161,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(hq_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/2".to_string())),
                lldp_sys_name: Some("unifi-usw-48".to_string()),
                lldp_port_desc: Some("Port 2 - TrueNAS uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 3))),
                lldp_sys_desc: Some("UniFi USW-48-PoE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "truenas-primary".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 2,
        });
    }

    // ========================================================================
    // HQ: Proxmox HV01 — with loopback, connected to switch port 3
    // ========================================================================
    if let Some(host) = find_host("proxmox-hv01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "eno1".to_string(),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(hq_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/3".to_string())),
                lldp_sys_name: Some("unifi-usw-48".to_string()),
                lldp_port_desc: Some("Port 3 - Proxmox HV01 uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 3))),
                lldp_sys_desc: Some("UniFi USW-48-PoE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "proxmox-hv01".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 3,
        });

        // Loopback
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 2,
                if_descr: "lo".to_string(),
                if_name: None,
                if_alias: None,
                if_type: 24,
                speed_bps: None,
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
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
        });
    }

    // ========================================================================
    // HQ: Proxmox HV02 — connected to switch port 4
    // ========================================================================
    if let Some(host) = find_host("proxmox-hv02") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "eno1".to_string(),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(hq_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/4".to_string())),
                lldp_sys_name: Some("unifi-usw-48".to_string()),
                lldp_port_desc: Some("Port 4 - Proxmox HV02 uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 3))),
                lldp_sys_desc: Some("UniFi USW-48-PoE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "proxmox-hv02".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 4,
        });
    }

    // ========================================================================
    // HQ: docker-prod01 — connected to switch port 5
    // ========================================================================
    if let Some(host) = find_host("docker-prod01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "eth0".to_string(),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(hq_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/5".to_string())),
                lldp_sys_name: Some("unifi-usw-48".to_string()),
                lldp_port_desc: Some("Port 5 - Docker host uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 3))),
                lldp_sys_desc: Some("UniFi USW-48-PoE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "docker-prod01".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 5,
        });
    }

    // ========================================================================
    // HQ Switch — unifi-usw-48 (48 ports)
    // ========================================================================
    if let Some(host) = find_host("unifi-usw-48") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        let pfsense_host = find_host("pfsense-fw01");
        let truenas_host = find_host("truenas-primary");
        let proxmox_hv01 = find_host("proxmox-hv01");
        let proxmox_hv02 = find_host("proxmox-hv02");
        let docker_host = find_host("docker-prod01");
        let ap_host = find_host("unifi-ap-lobby");

        // Port 1 ↔ pfsense-fw01
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "Port 1/0/1".to_string(),
                if_name: None,
                if_alias: Some("pfSense uplink".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("00:0d:b9:4a:f2:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("igb1".to_string())),
                lldp_sys_name: Some("pfsense-fw01".to_string()),
                lldp_port_desc: Some("LAN".to_string()),
                lldp_mgmt_addr: pfsense_host
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 1))),
                lldp_sys_desc: Some("pfSense 2.7.0-RELEASE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 1,
            target_host_name: "pfsense-fw01".to_string(),
            target_if_index: 2,
        });

        // Port 2 ↔ truenas-primary
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 2,
                if_descr: "Port 1/0/2".to_string(),
                if_name: None,
                if_alias: Some("TrueNAS uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("3c:ec:ef:12:34:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("lagg0".to_string())),
                lldp_sys_name: Some("truenas-primary".to_string()),
                lldp_port_desc: Some("LACP Bond".to_string()),
                lldp_mgmt_addr: truenas_host
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 40, 20))),
                lldp_sys_desc: Some("TrueNAS SCALE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 2,
            target_host_name: "truenas-primary".to_string(),
            target_if_index: 1,
        });

        // Port 3 ↔ proxmox-hv01
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 3,
                if_descr: "Port 1/0/3".to_string(),
                if_name: None,
                if_alias: Some("Proxmox HV01 uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("d4:be:d9:56:78:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("eno1".to_string())),
                lldp_sys_name: Some("proxmox-hv01".to_string()),
                lldp_port_desc: Some("Primary NIC".to_string()),
                lldp_mgmt_addr: proxmox_hv01
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 20, 5))),
                lldp_sys_desc: Some("Proxmox VE 8.1".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 3,
            target_host_name: "proxmox-hv01".to_string(),
            target_if_index: 1,
        });

        // Port 4 ↔ proxmox-hv02
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 4,
                if_descr: "Port 1/0/4".to_string(),
                if_name: None,
                if_alias: Some("Proxmox HV02 uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("d4:be:d9:56:78:02".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("eno1".to_string())),
                lldp_sys_name: Some("proxmox-hv02".to_string()),
                lldp_port_desc: Some("Primary NIC".to_string()),
                lldp_mgmt_addr: proxmox_hv02
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 20, 6))),
                lldp_sys_desc: Some("Proxmox VE 8.1".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 4,
            target_host_name: "proxmox-hv02".to_string(),
            target_if_index: 1,
        });

        // Port 5 ↔ docker-prod01
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 5,
                if_descr: "Port 1/0/5".to_string(),
                if_name: None,
                if_alias: Some("Docker host uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("aa:bb:cc:dd:ee:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("eth0".to_string())),
                lldp_sys_name: Some("docker-prod01".to_string()),
                lldp_port_desc: Some("Primary NIC".to_string()),
                lldp_mgmt_addr: docker_host
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 20, 20))),
                lldp_sys_desc: Some("Debian 12".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 5,
            target_host_name: "docker-prod01".to_string(),
            target_if_index: 1,
        });

        // Port 6 ↔ unifi-ap-lobby (deferred via NeighborUpdate)
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 6,
                if_descr: "Port 1/0/6".to_string(),
                if_name: None,
                if_alias: Some("UniFi AP".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("fc:ec:da:aa:bb:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("eth0".to_string())),
                lldp_sys_name: Some("unifi-ap-lobby".to_string()),
                lldp_port_desc: Some("Ethernet".to_string()),
                lldp_mgmt_addr: ap_host
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 30, 100))),
                lldp_sys_desc: Some("UniFi AP U6-Pro".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-48".to_string(),
            source_if_index: 6,
            target_host_name: "unifi-ap-lobby".to_string(),
            target_if_index: 1,
        });

        // Ports 7-48 — empty/down
        for port_num in 7..=48 {
            if_entries.push(IfEntry {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                base: IfEntryBase {
                    host_id: host.id,
                    network_id: network.id,
                    if_index: port_num,
                    if_descr: format!("Port 1/0/{}", port_num),
                    if_name: None,
                    if_alias: None,
                    if_type: 6,
                    speed_bps: Some(1_000_000_000),
                    admin_status: IfAdminStatus::Up,
                    oper_status: IfOperStatus::Down,
                    mac_address: None,
                    interface_id: None,
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
            });
        }
    }

    // ========================================================================
    // HQ: unifi-ap-lobby — single IfEntry for LLDP neighbor with switch port 6
    // ========================================================================
    if let Some(host) = find_host("unifi-ap-lobby") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "eth0".to_string(),
                if_name: None,
                if_alias: Some("Ethernet".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(hq_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/6".to_string())),
                lldp_sys_name: Some("unifi-usw-48".to_string()),
                lldp_port_desc: Some("Port 6 - UniFi AP".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 3))),
                lldp_sys_desc: Some("UniFi USW-48-PoE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-ap-lobby".to_string(),
            source_if_index: 1,
            target_host_name: "unifi-usw-48".to_string(),
            target_if_index: 6,
        });
    }

    // ========================================================================
    // DC: dc-fw01 — connected to DC switch port 1
    // ========================================================================
    if let Some(host) = find_host("dc-fw01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "port1".to_string(),
                if_name: None,
                if_alias: Some("LAN".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(dc_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/1".to_string())),
                lldp_sys_name: Some("dc-switch-01".to_string()),
                lldp_port_desc: Some("Port 1 - Firewall uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 0, 2))),
                lldp_sys_desc: Some("Managed Switch".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-fw01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-switch-01".to_string(),
            target_if_index: 1,
        });
    }

    // ========================================================================
    // DC: dc-proxmox-hv01 — connected to DC switch port 2
    // ========================================================================
    if let Some(host) = find_host("dc-proxmox-hv01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "eno1".to_string(),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(dc_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/2".to_string())),
                lldp_sys_name: Some("dc-switch-01".to_string()),
                lldp_port_desc: Some("Port 2 - Proxmox uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 0, 2))),
                lldp_sys_desc: Some("Managed Switch".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-proxmox-hv01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-switch-01".to_string(),
            target_if_index: 2,
        });
    }

    // ========================================================================
    // DC: dc-docker01 — connected to DC switch port 3
    // ========================================================================
    if let Some(host) = find_host("dc-docker01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "eth0".to_string(),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(dc_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/3".to_string())),
                lldp_sys_name: Some("dc-switch-01".to_string()),
                lldp_port_desc: Some("Port 3 - Docker host uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 0, 2))),
                lldp_sys_desc: Some("Managed Switch".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-docker01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-switch-01".to_string(),
            target_if_index: 3,
        });
    }

    // ========================================================================
    // DC: haproxy-lb01 — connected to DC switch port 4
    // ========================================================================
    if let Some(host) = find_host("haproxy-lb01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "eth0".to_string(),
                if_name: None,
                if_alias: Some("Primary NIC".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress(dc_switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/4".to_string())),
                lldp_sys_name: Some("dc-switch-01".to_string()),
                lldp_port_desc: Some("Port 4 - HAProxy uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 0, 2))),
                lldp_sys_desc: Some("Managed Switch".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "haproxy-lb01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-switch-01".to_string(),
            target_if_index: 4,
        });
    }

    // ========================================================================
    // DC Switch — dc-switch-01 (24 ports)
    // ========================================================================
    if let Some(host) = find_host("dc-switch-01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        let dc_fw = find_host("dc-fw01");
        let dc_proxmox = find_host("dc-proxmox-hv01");
        let dc_docker = find_host("dc-docker01");
        let dc_haproxy = find_host("haproxy-lb01");

        // Port 1 ↔ dc-fw01
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "Port 1/0/1".to_string(),
                if_name: None,
                if_alias: Some("Firewall uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("a0:36:9f:11:22:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("port1".to_string())),
                lldp_sys_name: Some("dc-fw01".to_string()),
                lldp_port_desc: Some("LAN".to_string()),
                lldp_mgmt_addr: dc_fw
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 0, 1))),
                lldp_sys_desc: Some("FortiGate-100F".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-switch-01".to_string(),
            source_if_index: 1,
            target_host_name: "dc-fw01".to_string(),
            target_if_index: 1,
        });

        // Port 2 ↔ dc-proxmox-hv01
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 2,
                if_descr: "Port 1/0/2".to_string(),
                if_name: None,
                if_alias: Some("Proxmox uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("d4:be:d9:aa:bb:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("eno1".to_string())),
                lldp_sys_name: Some("dc-proxmox-hv01".to_string()),
                lldp_port_desc: Some("Primary NIC".to_string()),
                lldp_mgmt_addr: dc_proxmox
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 10, 5))),
                lldp_sys_desc: Some("Proxmox VE 8.1".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-switch-01".to_string(),
            source_if_index: 2,
            target_host_name: "dc-proxmox-hv01".to_string(),
            target_if_index: 1,
        });

        // Port 3 ↔ dc-docker01
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 3,
                if_descr: "Port 1/0/3".to_string(),
                if_name: None,
                if_alias: Some("Docker host uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("aa:bb:cc:dd:ee:02".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("eth0".to_string())),
                lldp_sys_name: Some("dc-docker01".to_string()),
                lldp_port_desc: Some("Primary NIC".to_string()),
                lldp_mgmt_addr: dc_docker
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 10, 20))),
                lldp_sys_desc: Some("Debian 12".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-switch-01".to_string(),
            source_if_index: 3,
            target_host_name: "dc-docker01".to_string(),
            target_if_index: 1,
        });

        // Port 4 ↔ haproxy-lb01
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 4,
                if_descr: "Port 1/0/4".to_string(),
                if_name: None,
                if_alias: Some("HAProxy uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None,
                lldp_chassis_id: Some(LldpChassisId::MacAddress("11:22:33:44:55:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("eth0".to_string())),
                lldp_sys_name: Some("haproxy-lb01".to_string()),
                lldp_port_desc: Some("Primary NIC".to_string()),
                lldp_mgmt_addr: dc_haproxy
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(172, 16, 30, 10))),
                lldp_sys_desc: Some("HAProxy 2.8".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
                fdb_macs: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "dc-switch-01".to_string(),
            source_if_index: 4,
            target_host_name: "haproxy-lb01".to_string(),
            target_if_index: 1,
        });

        // Ports 5-24 — empty/down
        for port_num in 5..=24 {
            if_entries.push(IfEntry {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                base: IfEntryBase {
                    host_id: host.id,
                    network_id: network.id,
                    if_index: port_num,
                    if_descr: format!("Port 1/0/{}", port_num),
                    if_name: None,
                    if_alias: None,
                    if_type: 6,
                    speed_bps: Some(1_000_000_000),
                    admin_status: IfAdminStatus::Up,
                    oper_status: IfOperStatus::Down,
                    mac_address: None,
                    interface_id: None,
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
            });
        }
    }

    (if_entries, neighbor_updates)
}

// ============================================================================
// Daemons
// ============================================================================

fn generate_daemons(
    networks: &[Network],
    hosts: &[&Host],
    subnets: &[Subnet],
    now: DateTime<Utc>,
    user_id: Uuid,
) -> Vec<Daemon> {
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };
    let find_host = |name: &str| hosts.iter().find(|h| h.base.name == name).copied();
    let find_subnet = |name: &str| subnets.iter().find(|s| s.base.name.contains(name));

    let mut daemons = Vec::new();

    // HQ Daemon on docker-prod01
    if let (Some(host), Some(subnet)) = (find_host("docker-prod01"), find_subnet("HQ Servers")) {
        let network = find_network("Headquarters");
        daemons.push(Daemon {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonBase {
                host_id: host.id,
                network_id: network.id,
                url: "https://docker-prod01.acme.local:8443".to_string(),
                last_seen: Some(now),
                capabilities: DaemonCapabilities {
                    has_docker_socket: true,
                    interfaced_subnet_ids: vec![subnet.id],
                },
                mode: DaemonMode::DaemonPoll,
                name: "HQ Daemon".to_string(),
                tags: vec![],
                version: Version::parse(env!("CARGO_PKG_VERSION"))
                    .map(Some)
                    .unwrap_or_default(),
                user_id,
                api_key_id: None,
                is_unreachable: false,
                standby: false,
            },
        });
    }

    // DC Daemon on dc-docker01
    if let (Some(host), Some(subnet)) = (find_host("dc-docker01"), find_subnet("DC Compute")) {
        let network = find_network("Data Center");
        daemons.push(Daemon {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonBase {
                host_id: host.id,
                network_id: network.id,
                url: "https://docker01.dc.acme.io:8443".to_string(),
                last_seen: Some(now),
                capabilities: DaemonCapabilities {
                    has_docker_socket: true,
                    interfaced_subnet_ids: vec![subnet.id],
                },
                mode: DaemonMode::DaemonPoll,
                name: "DC Daemon".to_string(),
                tags: vec![],
                version: Version::parse(env!("CARGO_PKG_VERSION"))
                    .map(Some)
                    .unwrap_or_default(),
                user_id,
                api_key_id: None,
                is_unreachable: false,
                standby: false,
            },
        });
    }

    daemons
}

// ============================================================================
// API Keys
// ============================================================================

fn generate_api_keys(networks: &[Network], now: DateTime<Utc>) -> Vec<DaemonApiKey> {
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };

    vec![
        DaemonApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonApiKeyBase {
                key: format!("demo_hq_{}", Uuid::new_v4().simple()),
                name: "HQ Daemon Key".to_string(),
                last_used: Some(now),
                expires_at: None,
                network_id: find_network("Headquarters").id,
                is_enabled: true,
                tags: vec![],
                plaintext: None,
            },
        },
        DaemonApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonApiKeyBase {
                key: format!("demo_dc_{}", Uuid::new_v4().simple()),
                name: "DC Daemon Key".to_string(),
                last_used: Some(now),
                expires_at: None,
                network_id: find_network("Data Center").id,
                is_enabled: true,
                tags: vec![],
                plaintext: None,
            },
        },
    ]
}

// ============================================================================
// Discoveries
// ============================================================================

fn generate_discoveries(
    networks: &[Network],
    subnets: &[Subnet],
    daemons: &[Daemon],
    _hosts: &[&Host],
    _credentials: &[Credential],
    now: DateTime<Utc>,
) -> Vec<Discovery> {
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };
    let find_daemon = |name: &str| daemons.iter().find(|d| d.base.name.contains(name));
    let find_subnets_for_network = |network_id: Uuid| -> Vec<Uuid> {
        subnets
            .iter()
            .filter(|s| s.base.network_id == network_id)
            .map(|s| s.id)
            .collect()
    };

    let unified = |daemon: &Daemon, subnet_ids: Option<Vec<Uuid>>| DiscoveryType::Unified {
        host_id: daemon.base.host_id,
        subnet_ids,
        scan_local_docker_socket: daemon.base.capabilities.has_docker_socket,
        host_naming_fallback: HostNamingFallback::BestService,
        scan_settings: ScanSettings::default(),
    };

    let mut discoveries = Vec::new();

    // ===== HQ Unified discovery =====
    let hq = find_network("Headquarters");
    if let Some(daemon) = find_daemon("HQ") {
        let hq_subnet_ids = find_subnets_for_network(hq.id);
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DiscoveryBase {
                discovery_type: unified(daemon, Some(hq_subnet_ids.clone())),
                run_type: RunType::AdHoc {
                    last_run: Some(now - Duration::days(2)),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                network_id: hq.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            pending_credential_ids: vec![],
        });

        // Historical — completed 3 weeks ago
        let three_weeks_ago = now - Duration::weeks(3);
        let hq_unified = unified(daemon, Some(hq_subnet_ids.clone()));
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: three_weeks_ago,
            updated_at: three_weeks_ago,
            base: DiscoveryBase {
                discovery_type: hq_unified.clone(),
                run_type: RunType::Historical {
                    results: Box::new(DiscoveryUpdatePayload {
                        session_id: Uuid::new_v4(),
                        daemon_id: daemon.id,
                        network_id: hq.id,
                        phase: DiscoveryPhase::Complete,
                        discovery_type: hq_unified.clone(),
                        progress: 100,
                        error: None,
                        started_at: Some(three_weeks_ago),
                        finished_at: Some(three_weeks_ago + Duration::minutes(12)),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                    }),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                network_id: hq.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            pending_credential_ids: vec![],
        });

        // Historical — completed 1 week ago
        let one_week_ago = now - Duration::weeks(1);
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: one_week_ago,
            updated_at: one_week_ago,
            base: DiscoveryBase {
                discovery_type: hq_unified.clone(),
                run_type: RunType::Historical {
                    results: Box::new(DiscoveryUpdatePayload {
                        session_id: Uuid::new_v4(),
                        daemon_id: daemon.id,
                        network_id: hq.id,
                        phase: DiscoveryPhase::Complete,
                        discovery_type: hq_unified,
                        progress: 100,
                        error: None,
                        started_at: Some(one_week_ago),
                        finished_at: Some(one_week_ago + Duration::minutes(8)),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                    }),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                network_id: hq.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            pending_credential_ids: vec![],
        });
    }

    // ===== DC Unified discovery =====
    let dc = find_network("Data Center");
    if let Some(daemon) = find_daemon("DC") {
        let dc_subnet_ids = find_subnets_for_network(dc.id);
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DiscoveryBase {
                discovery_type: unified(daemon, Some(dc_subnet_ids.clone())),
                run_type: RunType::AdHoc {
                    last_run: Some(now - Duration::days(3)),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                network_id: dc.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            pending_credential_ids: vec![],
        });

        // Historical — failed 2 weeks ago
        let two_weeks_ago = now - Duration::weeks(2);
        let dc_unified = unified(daemon, Some(dc_subnet_ids));
        discoveries.push(Discovery {
            id: Uuid::new_v4(),
            created_at: two_weeks_ago,
            updated_at: two_weeks_ago,
            base: DiscoveryBase {
                discovery_type: dc_unified.clone(),
                run_type: RunType::Historical {
                    results: Box::new(DiscoveryUpdatePayload {
                        session_id: Uuid::new_v4(),
                        daemon_id: daemon.id,
                        network_id: dc.id,
                        phase: DiscoveryPhase::Failed,
                        discovery_type: dc_unified,
                        progress: 100,
                        error: Some("Connection timeout: daemon lost connectivity to subnet 172.16.20.0/24 during scan".to_string()),
                        started_at: Some(two_weeks_ago),
                        finished_at: Some(two_weeks_ago + Duration::minutes(3)),
                        hosts_discovered: None,
                        estimated_remaining_secs: None,
                    }),
                },
                name: "Discovery".to_string(),
                daemon_id: daemon.id,
                network_id: dc.id,
                tags: vec![],
            },
            scan_count: 0,
            force_full_scan: false,
            pending_credential_ids: vec![],
        });
    }

    discoveries
}

// ============================================================================
// Shares
// ============================================================================

fn generate_shares(
    topologies: &[Topology],
    networks: &[Network],
    user_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<Share> {
    let hq_network = networks.iter().find(|n| n.base.name == "Headquarters");
    let hq_topology =
        hq_network.and_then(|net| topologies.iter().find(|t| t.base.network_id == net.id));

    let mut shares = Vec::new();

    if let (Some(network), Some(topology)) = (hq_network, hq_topology) {
        if let Ok(id) = Uuid::parse_str("a1b2c3d4-e5f6-7890-abcd-ef1234567890") {
            shares.push(Share {
                // Fixed UUID for demo share — used in onboarding embed
                id,
                created_at: now,
                updated_at: now,
                base: ShareBase {
                    topology_id: topology.id,
                    network_id: network.id,
                    created_by: user_id,
                    name: "HQ Public View".to_string(),
                    is_enabled: true,
                    expires_at: None,
                    password_hash: None,
                    allowed_domains: None,
                    options: ShareOptions {
                        show_inspect_panel: false,
                        show_zoom_controls: false,
                        show_export_button: false,
                        show_minimap: false,
                    },
                },
            });
        }

        if let Ok(id) = Uuid::parse_str("b1b2c3d4-e5f6-7890-abcd-ef1234567890") {
            shares.push(Share {
                // Fixed UUID for demo share — used on website
                id,
                created_at: now,
                updated_at: now,
                base: ShareBase {
                    topology_id: topology.id,
                    network_id: network.id,
                    created_by: user_id,
                    name: "HQ Public View - With Inspect Panel".to_string(),
                    is_enabled: true,
                    expires_at: None,
                    password_hash: None,
                    allowed_domains: None,
                    options: ShareOptions {
                        show_inspect_panel: true,
                        show_zoom_controls: false,
                        show_export_button: false,
                        show_minimap: false,
                    },
                },
            });
        }
    }

    shares
}

// ============================================================================
// User API Keys
// ============================================================================

fn generate_user_api_keys(
    networks: &[Network],
    organization_id: Uuid,
    now: DateTime<Utc>,
) -> Vec<(UserApiKey, Vec<Uuid>)> {
    use super::handlers::DEMO_USER_ID;

    let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();
    let (_plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::User);

    vec![(
        UserApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: UserApiKeyBase {
                key: hashed,
                name: "Monitoring Integration Key".to_string(),
                user_id: DEMO_USER_ID,
                organization_id,
                permissions: UserOrgPermissions::Member,
                last_used: None,
                expires_at: None,
                is_enabled: true,
                tags: vec![],
                network_ids: vec![], // hydrated by create_with_networks
            },
        },
        network_ids,
    )]
}

// ============================================================================
// Groups
// ============================================================================

/// Generate demo groups using actual created services.
/// This must be called AFTER services are created to ensure binding IDs are correct.
pub fn generate_groups(networks: &[Network], services: &[Service], tags: &[Tag]) -> Vec<Group> {
    let now = Utc::now();
    let hq = networks
        .iter()
        .find(|n| n.base.name == "Headquarters")
        .unwrap();
    let dc = networks
        .iter()
        .find(|n| n.base.name == "Data Center")
        .unwrap();

    let monitoring_tag = tags
        .iter()
        .find(|t| t.base.name == "Monitoring")
        .map(|t| t.id);

    // Network-scoped binding lookup to avoid cross-network matches
    let find_binding = |name: &str, network_id: Uuid| -> Option<Uuid> {
        services
            .iter()
            .find(|s| s.base.name.contains(name) && s.base.network_id == network_id)
            .and_then(|s| s.base.bindings.first())
            .map(|b| b.id())
    };

    let mut groups = Vec::new();

    // ===== HQ Groups (3) =====

    // 1. Monitoring Stack: Prometheus → Grafana, Uptime Kuma
    let prometheus_binding = find_binding("Prometheus", hq.id);
    let grafana_binding = find_binding("Grafana", hq.id);
    let uptime_binding = find_binding("Uptime Kuma", hq.id);

    if let (Some(prometheus), Some(grafana)) = (prometheus_binding, grafana_binding) {
        let mut bindings = vec![prometheus, grafana];
        if let Some(uptime) = uptime_binding {
            bindings.push(uptime);
        }
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Monitoring Stack".to_string(),
                network_id: hq.id,
                description: Some(
                    "Prometheus metrics collection with Grafana visualization".to_string(),
                ),
                group_type: GroupType::HubAndSpoke,
                binding_ids: bindings,
                source: EntitySource::Manual,
                color: Color::Purple,
                edge_style: EdgeStyle::Straight,
                tags: monitoring_tag.into_iter().collect(),
            },
        });
    }

    // 2. Backup Flow: Proxmox VE (hv01) → TrueNAS
    let proxmox_binding = find_binding("Proxmox", hq.id);
    let truenas_binding = find_binding("TrueNAS", hq.id);

    if let (Some(proxmox), Some(truenas)) = (proxmox_binding, truenas_binding) {
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Backup Flow".to_string(),
                network_id: hq.id,
                description: Some("Server backup targets to TrueNAS storage".to_string()),
                group_type: GroupType::RequestPath,
                binding_ids: vec![proxmox, truenas],
                source: EntitySource::Manual,
                color: Color::Green,
                edge_style: EdgeStyle::SmoothStep,
                tags: vec![],
            },
        });
    }

    // 3. Network Access Path: pfSense → Portainer
    let pfsense_binding = find_binding("pfSense", hq.id);
    let portainer_binding = find_binding("Portainer", hq.id);

    if let (Some(pfsense), Some(portainer)) = (pfsense_binding, portainer_binding) {
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Network Access Path".to_string(),
                network_id: hq.id,
                description: Some("Traffic path from firewall to container management".to_string()),
                group_type: GroupType::RequestPath,
                binding_ids: vec![pfsense, portainer],
                source: EntitySource::Manual,
                color: Color::Cyan,
                edge_style: EdgeStyle::Bezier,
                tags: vec![],
            },
        });
    }

    // ===== DC Groups (3) =====

    // 4. Web Traffic Flow: HAProxy → Web Application → RabbitMQ
    let haproxy_binding = find_binding("HAProxy", dc.id);
    let app_binding = find_binding("Web Application", dc.id);
    let rabbitmq_binding = find_binding("RabbitMQ", dc.id);

    if let (Some(haproxy), Some(app), Some(rabbitmq)) =
        (haproxy_binding, app_binding, rabbitmq_binding)
    {
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Web Traffic Flow".to_string(),
                network_id: dc.id,
                description: Some(
                    "Production web request path from load balancer through app servers to message broker"
                        .to_string(),
                ),
                group_type: GroupType::RequestPath,
                binding_ids: vec![haproxy, app, rabbitmq],
                source: EntitySource::Manual,
                color: Color::Blue,
                edge_style: EdgeStyle::Bezier,
                tags: vec![],
            },
        });
    }

    // 5. Observability Stack: Prometheus (container) → Grafana (container), Jaeger (container)
    let dc_prometheus = find_binding("Prometheus", dc.id);
    let dc_grafana = find_binding("Grafana", dc.id);
    let dc_jaeger = find_binding("Jaeger", dc.id);

    if let (Some(prometheus), Some(grafana)) = (dc_prometheus, dc_grafana) {
        let mut bindings = vec![prometheus, grafana];
        if let Some(jaeger) = dc_jaeger {
            bindings.push(jaeger);
        }
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Observability Stack".to_string(),
                network_id: dc.id,
                description: Some(
                    "Containerized observability: Prometheus, Grafana, and Jaeger".to_string(),
                ),
                group_type: GroupType::HubAndSpoke,
                binding_ids: bindings,
                source: EntitySource::Manual,
                color: Color::Purple,
                edge_style: EdgeStyle::Straight,
                tags: monitoring_tag.into_iter().collect(),
            },
        });
    }

    // 6. Storage Tier: MinIO → Ceph, Elasticsearch
    let minio_binding = find_binding("MinIO", dc.id);
    let ceph_binding = find_binding("Ceph", dc.id);
    let es_binding = find_binding("Elasticsearch", dc.id);

    if let (Some(minio), Some(ceph)) = (minio_binding, ceph_binding) {
        let mut bindings = vec![minio, ceph];
        if let Some(es) = es_binding {
            bindings.push(es);
        }
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Storage Tier".to_string(),
                network_id: dc.id,
                description: Some("Object storage, distributed storage, and search".to_string()),
                group_type: GroupType::HubAndSpoke,
                binding_ids: bindings,
                source: EntitySource::Manual,
                color: Color::Orange,
                edge_style: EdgeStyle::Step,
                tags: vec![],
            },
        });
    }

    groups
}
