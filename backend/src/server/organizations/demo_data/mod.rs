//! Demo data for populating demo organizations with realistic network infrastructure.
//!
//! This module provides a complete dataset representing "Acme Technologies", a mid-size
//! company with MSP operations. The data includes multiple networks, subnets, hosts,
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
        base::{Daemon, DaemonBase, DaemonMode},
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
            HostChassisIdValue, HostFirmwareRevisionValue, HostManufacturerValue, HostModelValue,
            HostSerialNumberValue, HostSoftwareRevisionValue, HostSysContactValue,
            HostSysDescrValue, HostSysLocationValue, HostSysNameValue, HostSysObjectIdValue,
        },
        base::{Host, HostBase},
        name::{HostName, HostNameSources},
        virtualization::{HostVirtualization, ProxmoxVirtualization},
    },
    interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, Interface, InterfaceBase},
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    networks::r#impl::{DEFAULT_STALE_AFTER_HOURS, Network, NetworkBase},
    ports::r#impl::base::{Port, PortType},
    services::r#impl::patterns::ClientProbe,
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

/// Network-to-credential association for junction table seeding
pub struct NetworkCredentialAssignment {
    pub network_id: Uuid,
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
    pub network_credential_assignments: Vec<NetworkCredentialAssignment>,
    pub networks: Vec<Network>,
    pub subnets: Vec<Subnet>,
    pub hosts_with_services: Vec<HostWithServices>,
    /// "Recently discovered" hosts created AFTER the per-network snapshot, so
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
        let networks = generate_networks(organization_id, &tags, &credentials, now);
        let subnets = generate_subnets(
            &networks,
            &tags,
            dep_svc_ids.docker_hq,
            dep_svc_ids.docker_dc,
            now,
        );
        let network_credential_assignments =
            generate_network_credential_assignments(&networks, &credentials);
        let hosts_with_services = generate_hosts_and_services(
            &networks,
            &subnets,
            &tags,
            &credentials,
            &dep_svc_ids,
            now,
        );
        let recent_hosts_with_services = generate_recent_hosts(&networks, &subnets, now);

        // Collect hosts for daemon generation and interface generation
        let hosts: Vec<&Host> = hosts_with_services.iter().map(|h| &h.host).collect();
        let ip_addresses: Vec<&IPAddress> = hosts_with_services
            .iter()
            .flat_map(|h| h.ip_addresses.iter())
            .collect();

        let vlans = generate_vlans(&networks, organization_id, now);
        let (interfaces, neighbor_updates) =
            generate_interfaces(&networks, &hosts, &ip_addresses, &vlans, now);
        let subnet_vlan_records =
            generate_subnet_vlan_records(&interfaces, &hosts_with_services, now);
        let daemons = generate_daemons(&networks, &hosts, &subnets, now, user_id);
        let api_keys = generate_api_keys(&networks, now);
        let topologies = generate_topologies(&networks, &tags, now);
        let discoveries =
            generate_discoveries(&networks, &subnets, &daemons, &hosts, &credentials, now);
        let shares = generate_shares(&topologies, &networks, user_id, now);
        let user_api_keys = generate_user_api_keys(&networks, organization_id, now);

        let dependencies = generate_dependencies(&networks, &tags, &dep_svc_ids);

        Self {
            tags,
            credentials,
            network_credential_assignments,
            networks,
            subnets,
            vlans,
            subnet_vlan_records,
            hosts_with_services,
            recent_hosts_with_services,
            interfaces,
            neighbor_updates,
            daemons,
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
mod networks;
mod shares;
mod subnets;
mod tags;
mod topologies;
mod vlans;

use api_keys::{generate_api_keys, generate_user_api_keys};
use credentials::{generate_credentials, generate_network_credential_assignments};
use daemons::generate_daemons;
use dependencies::generate_dependencies;
use discoveries::generate_discoveries;
use hosts::{generate_hosts_and_services, generate_recent_hosts};
use interfaces::generate_interfaces;
use networks::generate_networks;
use shares::generate_shares;
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
    network: &Network,
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
            network_id: network.id,
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
            firmware_revision: None,
            software_revision: None,
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
                network_id: host.base.network_id,
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
                network_id: host.base.network_id,
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
                network_id: host.base.network_id,
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
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn subnet_vlan_records_are_derived_and_reference_valid_entities() {
        let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4());

        // The derivation should link at least the subnets whose hosts carry a
        // native VLAN (otherwise the junction is silently empty and demo
        // subnets show no VLANs).
        assert!(
            !demo.subnet_vlan_records.is_empty(),
            "expected derived subnet↔vlan links"
        );

        let subnet_ids: HashSet<Uuid> = demo.subnets.iter().map(|s| s.id).collect();
        let vlan_ids: HashSet<Uuid> = demo.vlans.iter().map(|v| v.id).collect();
        let mut pairs: HashSet<(Uuid, Uuid)> = HashSet::new();
        for r in &demo.subnet_vlan_records {
            assert!(
                subnet_ids.contains(&r.base.subnet_id),
                "subnet_vlan references unknown subnet"
            );
            assert!(
                vlan_ids.contains(&r.base.vlan_id),
                "subnet_vlan references unknown vlan"
            );
            assert!(
                pairs.insert((r.base.subnet_id, r.base.vlan_id)),
                "duplicate subnet↔vlan link"
            );
        }
    }

    #[test]
    fn a_subnet_carries_a_provisional_cidr_source() {
        use crate::server::shared::attribution::AttributeMethod;

        let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4());
        assert!(
            demo.subnets
                .iter()
                .any(|s| s.base.cidr.source().method() == AttributeMethod::Inferred),
            "expected at least one subnet whose cidr_source is Inferred-tier (a provisional range)"
        );
    }

    #[test]
    fn a_host_is_known_only_by_inference() {
        let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4());
        assert!(
            demo.hosts_with_services
                .iter()
                .any(|hws| hws.host.base.source == EntitySource::Inferred),
            "expected at least one host with EntitySource::Inferred (never contacted directly)"
        );
    }

    #[test]
    fn a_discovery_carries_warnings_from_more_than_one_remedy_group() {
        use crate::daemon::discovery::types::warnings::DiscoveryWarning;
        use crate::server::discovery::r#impl::types::RunType;
        use crate::server::shared::types::metadata::TypeMetadataProvider;

        let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4());
        let warnings: Vec<&DiscoveryWarning> = demo
            .discoveries
            .iter()
            .filter_map(|d| match &d.base.run_type {
                RunType::Historical { results } => Some(&results.warnings),
                _ => None,
            })
            .flatten()
            .collect();
        assert!(
            !warnings.is_empty(),
            "expected at least one seeded discovery warning"
        );

        let remedy_groups: HashSet<&'static str> =
            warnings.iter().map(|w| w.code().category()).collect();
        assert!(
            remedy_groups.len() > 1,
            "expected seeded warnings to span more than one WarningRemedy group, got {remedy_groups:?}"
        );
    }

    #[test]
    fn a_service_is_in_the_industrial_category() {
        use crate::server::services::r#impl::categories::ServiceCategory;

        let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4());
        assert!(
            demo.hosts_with_services
                .iter()
                .flat_map(|hws| &hws.services)
                .any(|svc| svc.base.service_definition.category() == ServiceCategory::Industrial),
            "expected at least one seeded service in the Industrial category"
        );
    }

    /// The host editor explains each host's title by the rung that produced it, so the demo has a
    /// host titled from each: a named host, and nameless ones titled by their hostname, sysName,
    /// chassis ID and address.
    #[test]
    fn demo_hosts_are_titled_from_every_rung() {
        use crate::server::hosts::r#impl::name_ladder::HostNameRung;

        let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4());
        let rungs: HashSet<HostNameRung> = demo
            .hosts_with_services
            .iter()
            .filter_map(|hws| hws.host.resolved_name(&hws.ip_addresses))
            .map(|(_, rung)| rung)
            .collect();

        for rung in [
            HostNameRung::Name,
            HostNameRung::Hostname,
            HostNameRung::SysName,
            HostNameRung::ChassisId,
            HostNameRung::Address,
        ] {
            assert!(rungs.contains(&rung), "no demo host is titled by {rung:?}");
        }
    }

    #[test]
    fn a_host_shows_distinct_firmware_and_software_revisions() {
        let demo = DemoData::generate(Uuid::new_v4(), Uuid::new_v4());
        assert!(
            demo.hosts_with_services.iter().any(|hws| {
                match (
                    &hws.host.base.firmware_revision,
                    &hws.host.base.software_revision,
                ) {
                    (Some(firmware), Some(software)) => firmware.value().0 != software.value().0,
                    _ => false,
                }
            }),
            "expected at least one host with distinct, populated firmware and software revisions"
        );
    }
}
