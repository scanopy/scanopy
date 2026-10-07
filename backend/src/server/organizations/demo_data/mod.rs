//! Demo data for populating demo organizations with realistic network infrastructure.
//!
//! This module provides a complete dataset representing "Acme Technologies", a mid-size
//! company with MSP operations. The data includes multiple sites, subnets, hosts,
//! services, daemons, API keys, tags, and dependencies.

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::{
    bindings::r#impl::base::Binding,
    credentials::r#impl::{
        base::{Credential, CredentialBase},
        mapping::IntegrationTarget,
        types::{CredentialAssignment, CredentialType, SecretValue},
    },
    daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase},
    daemons::r#impl::{
        api::DiscoveryUpdatePayload,
        base::{Daemon, DaemonBase, DaemonMode, DaemonOs},
    },
    dependencies::r#impl::{
        base::{Dependency, DependencyBase, DependencyMembers},
        types::DependencyType,
    },
    discovery::r#impl::{
        base::{Discovery, DiscoveryBase},
        scan_settings::ScanSettings,
        types::{DiscoveryType, HostNamingFallback, RunType},
    },
    hosts::r#impl::{
        attributes::{
            HostAssetTagValue, HostChassisIdValue, HostFirmwareRevisionValue,
            HostManufacturerValue, HostModelValue, HostOsValue, HostSerialNumberValue,
            HostSoftwareRevisionValue, HostSysContactValue, HostSysDescrValue,
            HostSysLocationValue, HostSysNameValue, HostSysObjectIdValue,
        },
        base::{Host, HostBase},
        name::{HostName, HostNameSources},
        os::{HostOs, HostOsFamily},
        virtualization::{
            ContainerHostVirtualization, ContainerNetworkType, HostVirtualization,
            ProxmoxGuestType, ProxmoxVirtualization,
        },
    },
    interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, Interface, InterfaceBase},
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    ports::r#impl::base::{Port, PortType},
    services::r#impl::patterns::{ClientProbe, MatchConfidence, MatchDetails, MatchReason},
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::{
            base::{Service, ServiceBase},
            virtualization::{DockerVirtualization, ServiceVirtualization},
        },
    },
    shared::attribution::{AttributeSource, Attributed},
    shared::{
        api_key_common::{ApiKeyType, generate_api_key_for_storage},
        types::{Color, entities::EntitySource},
    },
    shares::r#impl::base::{Share, ShareBase, ShareOptions},
    sites::r#impl::{Site, SiteBase},
    subnets::r#impl::base::{SubnetCidr, SubnetCidrValue},
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
    tags::r#impl::base::{Tag, TagBase},
    topology::types::{
        base::{Topology, TopologyBase, TopologyOptions, TopologyRequestOptions},
        edges::EdgeStyle,
        grouping::ElementRule,
    },
    user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase},
    users::r#impl::permissions::UserOrgPermissions,
    vlans::r#impl::base::{Vlan, VlanBase},
    vlans::r#impl::subnet_vlans::{SubnetVlanRecord, SubnetVlanRecordBase},
};
use chrono::{DateTime, Duration, Utc};
use cidr::{IpCidr, Ipv4Cidr};
use mac_address::MacAddress;
use secrecy::SecretString;
use semver::Version;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

// ============================================================================
// Demo Data Container
// ============================================================================

/// A host bundled with its ip_addresses, ports, and services for creation via discover_host
pub struct HostWithServices {
    pub host: Host,
    pub ip_addresses: Vec<IPAddress>,
    pub ports: Vec<Port>,
    pub services: Vec<Service>,
}

/// Deferred neighbor update to apply after all interfaces exist.
/// Uses host_name + if_index to identify entries (stable across creation)
/// instead of pre-generated UUIDs (which may not be preserved by storage).
pub struct NeighborUpdate {
    /// Source interface identifier
    pub source_host_name: String,
    pub source_if_index: i32,
    /// Target interface identifier (for Interface neighbors)
    pub target_host_name: String,
    pub target_if_index: i32,
}

/// Site-to-credential association for junction table seeding
pub struct SiteCredentialAssignment {
    pub site_id: Uuid,
    pub credential_ids: Vec<Uuid>,
}

/// Pre-generated UUIDs for dependency wiring.
/// Service IDs for HubAndSpoke deps (service-level members),
/// binding IDs for RequestPath deps (port-level members).
struct DependencyServiceIds {
    // HubAndSpoke: service-level members
    prometheus_hq: Uuid,
    grafana_hq: Uuid,
    uptime_kuma: Uuid,
    // RequestPath: binding-level members
    traefik_hq_binding: Uuid,
    gitea_hq_binding: Uuid,
    haproxy_dc_binding: Uuid,
    app01_dc_binding: Uuid,
    mariadb_dc_binding: Uuid,
    // Backup Flow (RequestPath): binding-level members
    pve_hq1_binding: Uuid,
    truenas_binding: Uuid,
    // Observability Stack (HubAndSpoke): service-level members
    prometheus_dc: Uuid,
    grafana_dc: Uuid,
    jaeger_dc: Uuid,
    // Storage Tier (HubAndSpoke): service-level members
    minio_dc: Uuid,
    ceph_dc: Uuid,
    elasticsearch_dc: Uuid,
    // Docker daemon service IDs: shared with the DockerBridge subnets' virtualization
    docker_hq: Uuid,
    docker_dc: Uuid,
}

/// Container for all demo data entities
pub struct DemoData {
    pub tags: Vec<Tag>,
    pub credentials: Vec<Credential>,
    pub site_credential_assignments: Vec<SiteCredentialAssignment>,
    pub sites: Vec<Site>,
    pub subnets: Vec<Subnet>,
    pub hosts_with_services: Vec<HostWithServices>,
    /// "Recently discovered" hosts created AFTER the per-site snapshot, so
    /// the snapshot captures an earlier state than the live view. See
    /// [`generate_recent_hosts`].
    pub recent_hosts_with_services: Vec<HostWithServices>,
    pub vlans: Vec<Vlan>,
    /// Subnet↔VLAN junction rows, derived from the interface `native_vlan_id`
    /// and IP↔subnet relationships (the same rule the server's discovery
    /// reconciler uses). See [`generate_subnet_vlan_records`].
    pub subnet_vlan_records: Vec<SubnetVlanRecord>,
    pub interfaces: Vec<Interface>,
    pub neighbor_updates: Vec<NeighborUpdate>,
    pub daemons: Vec<Daemon>,
    /// (daemon id, subnet ids) for the `daemon_interfaced_subnets` junction, derived from the
    /// daemon host's IP addresses. See [`generate_daemon_interfaced_subnets`].
    pub daemon_interfaced_subnets: Vec<(Uuid, Vec<Uuid>)>,
    pub api_keys: Vec<DaemonApiKey>,
    pub dependencies: Vec<Dependency>,
    pub topologies: Vec<Topology>,
    pub discoveries: Vec<Discovery>,
    pub shares: Vec<Share>,
    pub user_api_keys: Vec<(UserApiKey, Vec<Uuid>)>,
}

impl DemoData {
    /// Generate all demo data for the given organization
    pub fn generate(organization_id: Uuid, user_id: Uuid) -> Self {
        let now = Utc::now();

        // Pre-generate service UUIDs used by both host/service and dependency generators
        let dep_svc_ids = DependencyServiceIds {
            prometheus_hq: Uuid::new_v4(),
            grafana_hq: Uuid::new_v4(),
            uptime_kuma: Uuid::new_v4(),
            traefik_hq_binding: Uuid::new_v4(),
            gitea_hq_binding: Uuid::new_v4(),
            haproxy_dc_binding: Uuid::new_v4(),
            app01_dc_binding: Uuid::new_v4(),
            mariadb_dc_binding: Uuid::new_v4(),
            pve_hq1_binding: Uuid::new_v4(),
            truenas_binding: Uuid::new_v4(),
            prometheus_dc: Uuid::new_v4(),
            grafana_dc: Uuid::new_v4(),
            jaeger_dc: Uuid::new_v4(),
            minio_dc: Uuid::new_v4(),
            ceph_dc: Uuid::new_v4(),
            elasticsearch_dc: Uuid::new_v4(),
            docker_hq: Uuid::new_v4(),
            docker_dc: Uuid::new_v4(),
        };

        // Generate all entities in dependency order
        let tags = generate_tags(organization_id, now);
        let credentials = generate_credentials(organization_id, now);
        let sites = generate_sites(organization_id, &tags, &credentials, now);
        let subnets = generate_subnets(
            &sites,
            &tags,
            dep_svc_ids.docker_hq,
            dep_svc_ids.docker_dc,
            now,
        );
        let site_credential_assignments =
            generate_site_credential_assignments(&sites, &credentials);
        let mut hosts_with_services =
            generate_hosts_and_services(&sites, &subnets, &tags, &credentials, &dep_svc_ids, now);
        // The server matches an OS in what SNMP reported when a host is ingested, after the daemon
        // has offered its own readings (an SSH banner, say), so the demo matches last too.
        for host in &mut hosts_with_services {
            host.host.base.match_os_from_system_strings();
        }
        let recent_hosts_with_services = generate_recent_hosts(&sites, &subnets, now);

        // Collect hosts for daemon generation and interface generation
        let hosts: Vec<&Host> = hosts_with_services.iter().map(|h| &h.host).collect();
        let ip_addresses: Vec<&IPAddress> = hosts_with_services
            .iter()
            .flat_map(|h| h.ip_addresses.iter())
            .collect();

        let vlans = generate_vlans(&sites, &tags, organization_id, now);
        let (interfaces, neighbor_updates) =
            generate_interfaces(&sites, &hosts, &ip_addresses, &vlans, now);
        let subnet_vlan_records =
            generate_subnet_vlan_records(&interfaces, &hosts_with_services, now);
        let mut daemons = generate_daemons(&sites, &hosts, now, user_id);
        let daemon_interfaced_subnets = generate_daemon_interfaced_subnets(&daemons, &ip_addresses);
        let api_keys = generate_api_keys(&daemons, now);
        bind_daemon_api_keys(&mut daemons, &api_keys);
        let topologies = generate_topologies(&sites, &tags, now);
        let discoveries =
            generate_discoveries(&sites, &subnets, &daemons, &hosts, &credentials, now);
        let shares = generate_shares(&topologies, &sites, user_id, now);
        let user_api_keys = generate_user_api_keys(&sites, organization_id, now);

        let dependencies = generate_dependencies(&sites, &tags, &dep_svc_ids);

        Self {
            tags,
            credentials,
            site_credential_assignments,
            sites,
            subnets,
            vlans,
            subnet_vlan_records,
            hosts_with_services,
            recent_hosts_with_services,
            interfaces,
            neighbor_updates,
            daemons,
            daemon_interfaced_subnets,
            api_keys,
            dependencies,
            topologies,
            discoveries,
            shares,
            user_api_keys,
        }
    }
}

// ============================================================================
// Generation submodules
// ============================================================================

mod api_keys;
mod credentials;
mod daemons;
mod dependencies;
mod discoveries;
mod hosts;
mod interfaces;
mod shares;
mod sites;
mod subnets;
mod tags;
mod topologies;
mod vlans;

use api_keys::{bind_daemon_api_keys, generate_api_keys, generate_user_api_keys};
use credentials::{generate_credentials, generate_site_credential_assignments};
use daemons::{generate_daemon_interfaced_subnets, generate_daemons};
use dependencies::generate_dependencies;
use discoveries::generate_discoveries;
use hosts::{generate_hosts_and_services, generate_recent_hosts};
use interfaces::generate_interfaces;
use shares::generate_shares;
use sites::generate_sites;
use subnets::generate_subnets;
use tags::generate_tags;
use topologies::generate_topologies;
use vlans::{generate_subnet_vlan_records, generate_vlans};

// ============================================================================
// Host/Service builder helpers, shared by hosts.rs and interfaces.rs
// ============================================================================

// ============================================================================
// Hosts and Services
// ============================================================================

/// Helper to create a host with a single interface.
/// Returns (Host,IPAddress) - host has ip_address_ids: vec![] initially,
/// the server will populate it after creating the interface.
#[allow(clippy::too_many_arguments)]
fn create_host(
    name: &str,
    hostname: Option<&str>,
    description: Option<&str>,
    site: &Site,
    subnet: &Subnet,
    ip: Ipv4Addr,
    tags: Vec<Uuid>,
    _snmp_credential_id: Option<Uuid>,
    virtualization: Option<(HostVirtualization, Uuid)>,
    now: DateTime<Utc>,
) -> (Host, IPAddress) {
    let (virtualization_metadata, virtualization_service_id) = match virtualization {
        Some((metadata, service_id)) => (Some(metadata), Some(service_id)),
        None => (None, None),
    };
    let host_id = Uuid::new_v4();
    let ip_address = IPAddress {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        last_seen_at: now,
        last_discovery_id: None,
        first_discovery_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: IPAddressBase {
            site_id: site.id,
            host_id,
            subnet_id: subnet.id,
            ip_address: IpAddr::V4(ip),
            mac_address: None,
            name: Some("eth0".to_string()),
            position: 0,
        },
    };
    let host = Host {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        last_seen_at: now,
        last_discovery_id: None,
        first_discovery_id: None,
        id: host_id,
        created_at: now,
        updated_at: now,
        base: HostBase {
            name: HostName::manual(name.to_string()),
            site_id: site.id,
            // The demo stands in for scans, as `with_snmp` does, so its hostnames carry what a
            // scan's PTR lookup would record.
            hostname: hostname.map(|h| {
                Attributed::new(
                    crate::server::hosts::r#impl::attributes::HostHostnameValue(h.to_string()),
                    AttributeSource::ReverseDns,
                )
            }),
            description: description.map(String::from),
            source: EntitySource::Manual,
            virtualization_metadata,
            virtualization_service_id,
            virtualization_interface_id: None,
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
            asset_tag: None,
            firmware_revision: None,
            software_revision: None,
            os: None,
            credential_assignments: vec![],
        },
    };
    (host, ip_address)
}

/// Wraps a `create_host()` result to add SNMP system information fields.
#[allow(clippy::too_many_arguments)]
fn with_snmp(
    (mut host, ip_address): (Host, IPAddress),
    sys_descr: Option<&str>,
    sys_object_id: Option<&str>,
    sys_location: Option<&str>,
    sys_contact: Option<&str>,
    chassis_id: Option<&str>,
    manufacturer: Option<&str>,
    model: Option<&str>,
    serial_number: Option<&str>,
) -> (Host, IPAddress) {
    // Demo data stands in for a credentialed SNMP scan, so it claims what such a scan would: the
    // device's own answers, with `sysLocation` and `sysContact` as the two an operator typed into
    // it. Anything else would give the demo org a provenance no real scan produces.
    let probe = AttributeSource::Probe(ClientProbe::Snmp);
    let authored = AttributeSource::Authored(ClientProbe::Snmp);
    host.base.sys_descr = sys_descr.map(|v| Attributed::new(HostSysDescrValue(v.into()), probe));
    host.base.sys_object_id =
        sys_object_id.map(|v| Attributed::new(HostSysObjectIdValue(v.into()), probe));
    host.base.sys_location =
        sys_location.map(|v| Attributed::new(HostSysLocationValue(v.into()), authored));
    host.base.sys_contact =
        sys_contact.map(|v| Attributed::new(HostSysContactValue(v.into()), authored));
    host.base.chassis_id = chassis_id.map(|v| Attributed::new(HostChassisIdValue(v.into()), probe));
    // SNMP sysName conventionally mirrors the device hostname.
    host.base.sys_name = Some(Attributed::new(
        HostSysNameValue(host.base.name.to_string()),
        probe,
    ));
    host.base.manufacturer =
        manufacturer.map(|v| Attributed::new(HostManufacturerValue(v.into()), probe));
    host.base.model = model.map(|v| Attributed::new(HostModelValue(v.into()), probe));
    host.base.serial_number =
        serial_number.map(|v| Attributed::new(HostSerialNumberValue(v.into()), probe));
    (host, ip_address)
}

/// Wraps a `create_host()` result to add what the "Linux Inventory" SSH credential's script
/// reports: the OS and hardware identity, attributed to the script as a real scan would.
fn with_ssh_inventory(
    (mut host, ip_address): (Host, IPAddress),
    sys_descr: &str,
    manufacturer: &str,
    model: &str,
    serial_number: &str,
    firmware_revision: &str,
    os: HostOs,
) -> (Host, IPAddress) {
    let source = AttributeSource::SshScript;
    host.base.sys_descr = Some(Attributed::new(HostSysDescrValue(sys_descr.into()), source));
    host.base.manufacturer = Some(Attributed::new(
        HostManufacturerValue(manufacturer.into()),
        source,
    ));
    host.base.model = Some(Attributed::new(HostModelValue(model.into()), source));
    host.base.serial_number = Some(Attributed::new(
        HostSerialNumberValue(serial_number.into()),
        source,
    ));
    host.base.firmware_revision = Some(Attributed::new(
        HostFirmwareRevisionValue(firmware_revision.into()),
        source,
    ));
    host.base.os = Some(Attributed::new(HostOsValue(os), source));
    (host, ip_address)
}

/// Wraps a `create_host()` result to set the MAC address on the IP address.
fn with_mac((host, mut ip_address): (Host, IPAddress), mac: [u8; 6]) -> (Host, IPAddress) {
    ip_address.base.mac_address = Some(MacEvidence::new(
        MacEvidenceValue(MacAddress::new(mac)),
        AttributeSource::ArpReply,
    ));
    (host, ip_address)
}

/// Wraps a `create_host()` result to leave the host without a name, as a device nobody has named
/// arrives. Its title then comes from the display-name ladder: the sysName `with_snmp` copied from
/// the name it was built with, when there is one, and otherwise its address.
fn unnamed((mut host, ip_address): (Host, IPAddress)) -> (Host, IPAddress) {
    host.base.name = HostName::unnamed();
    (host, ip_address)
}

/// Wraps a `create_host()` result for a Proxmox guest to read as the Proxmox VE integration records
/// it: discovered, and titled by the name a person gave the guest in Proxmox.
fn reported_by_proxmox((mut host, ip_address): (Host, IPAddress)) -> (Host, IPAddress) {
    host.base.source = EntitySource::Discovery;
    if let Some(HostVirtualization::Proxmox(ProxmoxVirtualization {
        vm_name: Some(vm_name),
        ..
    })) = &host.base.virtualization_metadata
    {
        host.base.name = HostName::from_controller(vm_name.clone(), ClientProbe::Proxmox);
    }
    (host, ip_address)
}

/// Pins credentials to a host, covering all of its addresses. A `None` id (a credential the demo
/// set no longer defines) is skipped.
fn with_credentials(
    mut hws: HostWithServices,
    credential_ids: &[Option<Uuid>],
) -> HostWithServices {
    hws.host
        .base
        .credential_assignments
        .extend(
            credential_ids
                .iter()
                .flatten()
                .map(|&credential_id| CredentialAssignment {
                    credential_id,
                    ip_address_ids: None,
                }),
        );
    hws
}

/// Turns a host into a discovered device that has since dropped off the network: first seen at
/// `first_seen`, last answered a scan at `last_seen`. Its addresses, ports, services and bindings
/// carry the same dates, so the whole host reads as stale. `last_seen` must fall outside the demo
/// sites' staleness window.
fn gone_quiet(
    mut hws: HostWithServices,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
) -> HostWithServices {
    let host = &mut hws.host;
    host.base.source = EntitySource::Discovery;
    (host.created_at, host.valid_from) = (first_seen, first_seen);
    (host.updated_at, host.last_seen_at) = (last_seen, last_seen);
    for ip in &mut hws.ip_addresses {
        (ip.created_at, ip.valid_from) = (first_seen, first_seen);
        (ip.updated_at, ip.last_seen_at) = (last_seen, last_seen);
    }
    for port in &mut hws.ports {
        (port.created_at, port.valid_from) = (first_seen, first_seen);
        (port.updated_at, port.last_seen_at) = (last_seen, last_seen);
    }
    for svc in &mut hws.services {
        svc.base.source = EntitySource::Discovery;
        (svc.created_at, svc.valid_from) = (first_seen, first_seen);
        (svc.updated_at, svc.last_seen_at) = (last_seen, last_seen);
        for binding in &mut svc.base.bindings {
            (binding.created_at, binding.valid_from) = (first_seen, first_seen);
            (binding.updated_at, binding.last_seen_at) = (last_seen, last_seen);
        }
    }
    hws
}

/// Marks a host as a scan found it, and each of its services as the service matcher records one:
/// matched to its definition, with the reason and confidence that definition's pattern produces.
/// A service whose match [`scan_match`] does not reproduce stays plain `Discovery`.
fn found_by_scan(mut hws: HostWithServices) -> HostWithServices {
    hws.host.base.source = EntitySource::Discovery;
    for svc in &mut hws.services {
        svc.base.source = match scan_match(svc.base.service_definition.id()) {
            Some(details) => EntitySource::DiscoveryWithMatch { details },
            None => EntitySource::Discovery,
        };
    }
    hws
}

/// The match details the service matcher records for a definition, built the way it builds them:
/// an open well-known port alone is `Low`, a completed client probe is `Certain`, and an `AllOf`
/// takes the strongest of its parts.
fn scan_match(service_definition_id: &str) -> Option<MatchDetails> {
    use MatchConfidence::{Certain, Low};
    let port_open = |port: PortType| MatchReason::Reason(format!("Port {port} is open"));
    let probe =
        |probe: ClientProbe| MatchReason::Reason(format!("Client probe {probe:?} succeeded"));
    let all_of = |reasons: Vec<MatchReason>| MatchReason::Container("All of".to_string(), reasons);
    // `probe_pattern`: the app probe's port, then the probe itself.
    let probed = |port: PortType, client_probe: ClientProbe| {
        (all_of(vec![port_open(port), probe(client_probe)]), Certain)
    };
    let (reason, confidence) = match service_definition_id {
        "SSH" => probed(PortType::Ssh, ClientProbe::Ssh),
        "OpenVPN" => probed(PortType::OpenVPN, ClientProbe::OpenVpn),
        "Modbus TCP" => probed(PortType::ModbusTcp, ClientProbe::ModbusTcp),
        "Workstation" => (
            all_of(vec![probe(ClientProbe::Rdp), probe(ClientProbe::Smb)]),
            Certain,
        ),
        "BACnet" => (port_open(PortType::BACnet), Low),
        _ => return None,
    };
    Some(MatchDetails { reason, confidence })
}

/// Helper to create a service for a host.
/// Returns (Service, Option<Port>) - the port must be added to the host's ports list.
fn create_service(
    service_def_id: &str,
    name: &str,
    host: &Host,
    ip_address: &IPAddress,
    port_type: Option<PortType>,
    tags: Vec<Uuid>,
    now: DateTime<Utc>,
) -> Option<(Service, Option<Port>)> {
    let service_definition = ServiceDefinitionRegistry::find_by_id(service_def_id)?;

    let (bindings, port) = if let Some(pt) = port_type {
        let port = Port::new_hostless(pt);
        let binding = Binding::new_port_serviceless(port.id, Some(ip_address.id));
        (vec![binding], Some(port))
    } else {
        let binding = Binding::new_ip_address_serviceless(ip_address.id);
        (vec![binding], None)
    };

    Some((
        Service {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id: host.id,
                site_id: host.base.site_id,
                service_definition,
                name: name.to_string(),
                bindings,
                virtualization_metadata: None,
                virtualization_service_id: None,
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
    ip_address: &IPAddress,
    port_type: Option<PortType>,
    tags: Vec<Uuid>,
    now: DateTime<Utc>,
) -> Option<(Service, Option<Port>)> {
    let service_definition = ServiceDefinitionRegistry::find_by_id(service_def_id)?;

    let (bindings, port) = if let Some(pt) = port_type {
        let port = Port::new_hostless(pt);
        let binding = Binding::new_port_serviceless(port.id, Some(ip_address.id));
        (vec![binding], Some(port))
    } else {
        let binding = Binding::new_ip_address_serviceless(ip_address.id);
        (vec![binding], None)
    };

    Some((
        Service {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: service_id,
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id: host.id,
                site_id: host.base.site_id,
                service_definition,
                name: name.to_string(),
                bindings,
                virtualization_metadata: None,
                virtualization_service_id: None,
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
    ip_address: &IPAddress,
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
        let binding = Binding::new_port_serviceless(port.id, Some(ip_address.id));
        (vec![binding], Some(port))
    } else {
        let binding = Binding::new_ip_address_serviceless(ip_address.id);
        (vec![binding], None)
    };

    Some((
        Service {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id: host.id,
                site_id: host.base.site_id,
                service_definition,
                name: name.to_string(),
                bindings,
                virtualization_metadata: Some(ServiceVirtualization::Docker(
                    DockerVirtualization {
                        container_name: Some(container_name.to_string()),
                        container_id: Some(container_id.to_string()),
                        compose_project: Some("media-stack".to_string()),
                    },
                )),
                virtualization_service_id: Some(docker_daemon_svc_id),
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
/// Takes a tuple of (Host,IPAddress) from create_host().
macro_rules! host_with_services {
    ($host_tuple:expr, $now:expr, $( ($svc_def:expr, $svc_name:expr, $port:expr, $tags:expr) ),* $(,)?) => {{
        let (host, ip_address) = $host_tuple;
        let ip_addresses = vec![ip_address];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        $(
            if let Some((svc, port)) = create_service($svc_def, $svc_name, &host, &ip_addresses[0], $port, $tags, $now) {
                // Collect port separately if present
                if let Some(p) = port {
                    ports.push(p);
                }
                services.push(svc);
            }
        )*
        HostWithServices { host, ip_addresses, ports, services }
    }};
}
use host_with_services;

#[cfg(test)]
mod tests;

/// What the Docker engine on a demo Docker host reports about the machine it runs on. The daemon
/// there runs in the published image and self-reports that image's OS (Debian 12); the Docker
/// integration's engine reading replaces it by rank, so this is what a scan leaves on the host.
fn docker_engine_host_os() -> Option<crate::server::hosts::r#impl::attributes::HostOsAttributed> {
    Some(Attributed::new(
        HostOsValue(HostOs {
            family: HostOsFamily::Linux,
            name: Some("Ubuntu 24.04.1 LTS".to_string()),
            version: None,
            edition: None,
            codename: None,
            kernel_version: Some("6.8.0-45-generic".to_string()),
        }),
        AttributeSource::ContainerRuntimeInfo,
    ))
}

/// The identification string a Proxmox VE 8 node's SSH server sends, after the `SSH-2.0-` prefix.
const PROXMOX_SSH_BANNER: &str = "OpenSSH_9.2p1 Debian-2+deb12u3";

/// The identification string Windows' bundled OpenSSH Server sends.
const WINDOWS_SSH_BANNER: &str = "OpenSSH_for_Windows_9.5";

/// Wraps a `create_host()` result with the OS its SSH banner names, matched the way the daemon's
/// SSH probe matches it, so the demo carries an inferred OS beside the ones read off a host.
fn with_ssh_banner((mut host, ip_address): (Host, IPAddress), banner: &str) -> (Host, IPAddress) {
    if let Some(os) = crate::server::hosts::r#impl::os::recog::RecogDatabase::SshBanner.os(banner) {
        Attributed::apply(
            &mut host.base.os,
            Attributed::new(HostOsValue(os), AttributeSource::SshBannerMatch),
        );
    }
    (host, ip_address)
}
