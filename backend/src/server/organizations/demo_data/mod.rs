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
            LibvirtVirtualization, ProxmoxGuestType, ProxmoxVirtualization,
        },
    },
    interfaces::r#impl::base::{
        FdbMacs, FdbMacsValue, IfAdminStatus, IfOperStatus, Interface, InterfaceBase,
    },
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    organizations::r#impl::base::UseCase,
    ports::r#impl::base::{Port, PortType},
    services::r#impl::patterns::{ClientProbe, MatchConfidence, MatchDetails, MatchReason},
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::{
            base::{Service, ServiceBase},
            definitions::ServiceDefinition,
            virtualization::{DockerVirtualization, ServiceVirtualization},
        },
    },
    shared::attribution::{AttributeSource, Attributed},
    shared::{
        api_key_common::{ApiKeyType, generate_api_key_for_storage},
        types::{Color, entities::EntitySource, guest_state::GuestState},
    },
    shares::r#impl::base::{Share, ShareBase, ShareOptions},
    sites::r#impl::{Site, SiteBase},
    subnets::r#impl::base::{SubnetCidr, SubnetCidrValue},
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
    tags::{
        groups::base::TagGroup,
        r#impl::base::{Tag, TagBase},
    },
    topology::types::{
        base::{Topology, TopologyBase, TopologyOptions, TopologyRequestOptions},
        edges::EdgeStyle,
        grouping::ElementRule,
    },
    user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase},
    users::r#impl::permissions::UserOrgPermissions,
    vlans::r#impl::base::{Vlan, VlanBase, VlanName, VlanNameValue},
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
    /// Target interface identifier (for Interface neighbors). `None` names an interface with no
    /// ifIndex: a far end known only from what it advertised, never walked itself.
    pub target_host_name: String,
    pub target_if_index: Option<i32>,
}

/// Site-to-credential association for junction table seeding
pub struct SiteCredentialAssignment {
    pub site_id: Uuid,
    pub credential_ids: Vec<Uuid>,
}

/// Container for all demo data entities
pub struct DemoData {
    pub tag_groups: Vec<TagGroup>,
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
    /// Links known only as far as the far device, written as `Neighbor::Host`.
    pub host_neighbor_updates: Vec<HostNeighborUpdate>,
    pub daemons: Vec<Daemon>,
    /// (daemon id, subnet ids) for the `daemon_interfaced_subnets` junction, derived from the
    /// daemon host's IP addresses. See [`generate_daemon_interfaced_subnets`].
    pub daemon_interfaced_subnets: Vec<(Uuid, Vec<Uuid>)>,
    pub api_keys: Vec<DaemonApiKey>,
    pub dependencies: Vec<Dependency>,
    pub topologies: Vec<Topology>,
    pub discoveries: Vec<Discovery>,
    pub shares: Vec<Share>,
    pub user_api_keys: Vec<(UserApiKey, Option<Vec<Uuid>>)>,
    /// The sites the seeder snapshots, and when the snapshot is dated. Every row of those sites
    /// is valid from this time or earlier, so the snapshot's closed copies cover a real interval.
    pub snapshot_site_ids: Vec<Uuid>,
    pub snapshot_taken_at: DateTime<Utc>,
}

/// How long before seeding the hand-built sites' snapshot is dated.
const SNAPSHOT_AGE: Duration = Duration::days(30);

/// Moves each row's `valid_from` back to `at` when it is later. See
/// [`DemoData::snapshot_taken_at`].
pub(crate) fn backdate_rows<T: Snapshotable>(rows: &mut [T], at: DateTime<Utc>) {
    for row in rows {
        if row.valid_from() > at {
            row.set_valid_from(at);
        }
    }
}

impl DemoData {
    /// Generate all demo data for the given organization. `use_case` picks the topology's
    /// default grouping rules, as it does for any new topology in that organization.
    pub fn generate(organization_id: Uuid, user_id: Uuid, use_case: UseCase) -> Self {
        let now = crate::server::shared::clock::now();

        // The Docker daemons' service IDs, shared by the hosts that run them and the Docker
        // Bridge subnets they virtualize.
        let docker_hq = Uuid::new_v4();
        let docker_dc = Uuid::new_v4();

        // Generate all entities in dependency order
        let tag_groups = generate_tag_groups(organization_id, now);
        let tags = generate_tags(organization_id, &tag_groups, now);
        let credentials = generate_credentials(organization_id, now);
        let sites = generate_sites(organization_id, &tags, &credentials, now);
        let subnets = generate_subnets(&sites, docker_hq, docker_dc, now);
        let site_credential_assignments =
            generate_site_credential_assignments(&sites, &credentials);
        let mut hosts_with_services = generate_hosts_and_services(
            &sites,
            &subnets,
            &tags,
            &credentials,
            docker_hq,
            docker_dc,
            now,
        );
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
        let topologies = generate_topologies(&sites, &tags, use_case, now);
        let discoveries = generate_discoveries(
            &sites,
            &subnets,
            &daemons,
            &hosts,
            &interfaces,
            &credentials,
            now,
        );
        let shares = generate_shares(&topologies, &sites, user_id, now);
        let user_api_keys = generate_user_api_keys(&sites, organization_id, now);

        let dependencies = generate_dependencies(&sites, &hosts_with_services);
        let snapshot_site_ids = sites.iter().map(|s| s.id).collect();

        let mut data = Self {
            tag_groups,
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
            host_neighbor_updates: Vec::new(),
            snapshot_site_ids,
            snapshot_taken_at: now - SNAPSHOT_AGE,
        };
        // Before Plant is added: only the hand-built sites are snapshotted.
        data.backdate_for_snapshot();
        plant::add_plant_site(&mut data, organization_id, user_id, use_case, now);
        data
    }

    /// Makes every row present so far valid from the snapshot time or earlier. The snapshot's
    /// closed copies keep each row's `valid_from` and end at the snapshot time, so a row valid
    /// only from seeding time would close before it opened. The "recently discovered" hosts are
    /// left alone: they arrive after the snapshot.
    fn backdate_for_snapshot(&mut self) {
        let at = self.snapshot_taken_at;
        backdate_rows(&mut self.tags, at);
        backdate_rows(&mut self.subnets, at);
        backdate_rows(&mut self.vlans, at);
        backdate_rows(&mut self.subnet_vlan_records, at);
        backdate_rows(&mut self.interfaces, at);
        backdate_rows(&mut self.dependencies, at);
        for hws in &mut self.hosts_with_services {
            backdate_rows(std::slice::from_mut(&mut hws.host), at);
            backdate_rows(&mut hws.ip_addresses, at);
            backdate_rows(&mut hws.ports, at);
            backdate_rows(&mut hws.services, at);
            for service in &mut hws.services {
                backdate_rows(&mut service.base.bindings, at);
            }
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

pub mod large_estate;
mod plant;

use crate::server::shared::storage::snapshot::Snapshotable;
pub use large_estate::HostNeighborUpdate;

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
use tags::{generate_tag_groups, generate_tags};
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
            ip_address_source: AttributeSource::ArpReply,
        },
    };
    let host = Host {
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        last_seen_at: now,
        last_discovery_id: None,
        first_discovery_id: None,
        dismissed_at: None,
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
            matched_device_type: None,
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

/// Wraps a `create_host()` result to add what a Recog fingerprint of something the device emitted
/// names, attributed to that match as a real scan would: the maker and the device class.
fn with_fingerprint(
    (mut host, ip_address): (Host, IPAddress),
    source: AttributeSource,
    manufacturer: &str,
    device_type: crate::server::hosts::r#impl::device_type::DeviceType,
) -> (Host, IPAddress) {
    host.base.manufacturer = Some(Attributed::new(
        HostManufacturerValue(manufacturer.into()),
        source,
    ));
    host.base.matched_device_type = Some(Attributed::new(
        crate::server::hosts::r#impl::attributes::HostMatchedDeviceTypeValue(device_type),
        source,
    ));
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
    let probed = |client_probe: ClientProbe| (probe(client_probe), Certain);
    let (reason, confidence) = match service_definition_id {
        "SSH" => probed(ClientProbe::Ssh),
        "OpenVPN" => probed(ClientProbe::OpenVpn),
        "Modbus TCP" => probed(ClientProbe::ModbusTcp),
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
    Some(build_service(
        service_definition,
        name,
        host,
        ip_address,
        port_type,
        tags,
        now,
    ))
}

/// [`create_service`] for a definition the caller already resolved.
fn build_service(
    service_definition: Box<dyn ServiceDefinition>,
    name: &str,
    host: &Host,
    ip_address: &IPAddress,
    port_type: Option<PortType>,
    tags: Vec<Uuid>,
    now: DateTime<Utc>,
) -> (Service, Option<Port>) {
    let (bindings, port) = if let Some(pt) = port_type {
        let port = Port::new_hostless(pt).with_source(AttributeSource::PortScan);
        let binding = Binding::new_port_serviceless(port.id, Some(ip_address.id));
        (vec![binding], Some(port))
    } else {
        let binding = Binding::new_ip_address_serviceless(ip_address.id);
        (vec![binding], None)
    };

    (
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
    )
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
        let port = Port::new_hostless(pt).with_source(AttributeSource::PortScan);
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

    // The Docker integration reads a container's published ports from the engine.
    let (bindings, port) = if let Some(pt) = port_type {
        let port = Port::new_hostless(pt).with_source(AttributeSource::Probe(ClientProbe::Docker));
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
                        state: Default::default(),
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
