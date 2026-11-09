//! A generated core/distribution/access estate.
//!
//! The demo seeds it as its "Plant" site, so prospects with hundreds of switches can see the views
//! at their scale, and `estate-replay` loads it into a self-hosted server at three sizes to measure
//! what the server needs (`docs/sizing-results.md`). Every count is an [`EstateParams`] field.
//!
//! Switches and firewalls carry SNMP interface rows and port-to-port links. Endpoints and servers
//! carry an address and a MAC, and sit behind the access port that learned the MAC, the way the
//! demo's HQ workstations do.

use super::*;
use std::collections::HashMap;

mod addressing;
mod endpoints;
mod switches;

#[cfg(test)]
mod tests;

use addressing::VLAN_PLAN;
use addressing::{Addressing, Role, build_segments};

/// The demo site the default estate is seeded as.
pub const PLANT_SITE_NAME: &str = "Plant";

/// The shape of a generated estate.
#[derive(Debug, Clone)]
pub struct EstateParams {
    /// Core switches, linked to each other.
    pub cores: usize,
    /// Distribution switches, each uplinked to every core (at most two uplinks per switch).
    pub distribution: usize,
    /// Access switches, each uplinked to one distribution switch, or two when dual-homed.
    pub access: usize,
    /// Endpoints spread over the access switches.
    pub endpoints: usize,
    /// Servers, on the first access switches.
    pub servers: usize,
    /// Firewalls between the OT DMZ and the office network, each linked to the cores.
    pub firewalls: usize,
    /// VLANs and subnets, from 4 to the length of [`VLAN_PLAN`].
    pub vlans: usize,
    /// Share of access switches uplinked to two distribution switches.
    pub dual_homed_access_pct: u8,
    /// Share of access uplinks where only the access switch identified the far device, not its
    /// port, so the link is drawn to the device.
    pub device_level_pct: u8,
    /// Seeds the vendor mix and the device-level links, so an estate generates the same each time.
    pub seed: u64,
}

impl Default for EstateParams {
    /// 250 switches and about 4,300 hosts.
    fn default() -> Self {
        Self {
            cores: 2,
            distribution: 24,
            access: 224,
            endpoints: 4000,
            servers: 60,
            firewalls: 2,
            vlans: VLAN_PLAN.len(),
            dual_homed_access_pct: 50,
            device_level_pct: 5,
            seed: 0x5ca7_0e57,
        }
    }
}

impl EstateParams {
    /// 25 switches and 500 hosts.
    pub fn small() -> Self {
        Self {
            distribution: 3,
            access: 20,
            servers: 8,
            endpoints: 500 - 25 - 8 - 2,
            ..Self::default()
        }
    }

    /// 250 switches and about 4,300 hosts: the default estate.
    pub fn medium() -> Self {
        Self::default()
    }

    /// 800 switches and 15,000 hosts.
    pub fn large() -> Self {
        Self {
            distribution: 48,
            access: 750,
            servers: 200,
            endpoints: 15_000 - 800 - 200 - 2,
            ..Self::default()
        }
    }

    pub fn switch_count(&self) -> usize {
        self.cores + self.distribution + self.access
    }

    pub fn host_count(&self) -> usize {
        self.switch_count() + self.firewalls + self.servers + self.endpoints
    }
}

/// A link authored at one end only, where the far end is known as a device but not a port: an
/// LLDP neighbour whose advertised port matched nothing the far switch reported.
pub struct HostNeighborUpdate {
    pub source_host_name: String,
    pub source_if_index: i32,
    pub target_host_name: String,
}

/// One generated estate on one site.
pub struct LargeEstate {
    pub subnets: Vec<Subnet>,
    pub vlans: Vec<Vlan>,
    pub subnet_vlan_records: Vec<SubnetVlanRecord>,
    pub hosts_with_services: Vec<HostWithServices>,
    pub interfaces: Vec<Interface>,
    pub neighbor_updates: Vec<NeighborUpdate>,
    pub host_neighbor_updates: Vec<HostNeighborUpdate>,
    /// The host the site's daemon runs on: the first monitoring server.
    pub daemon_host_id: Uuid,
}

/// The site the demo seeds the default estate as. Created after the hand-built sites so it sorts
/// after them.
pub fn plant_site(organization_id: Uuid, now: DateTime<Utc>) -> Site {
    Site {
        id: Uuid::new_v4(),
        created_at: now + Duration::seconds(2),
        updated_at: now + Duration::seconds(2),
        base: SiteBase {
            name: PLANT_SITE_NAME.to_string(),
            organization_id,
            tags: vec![],
            credential_ids: None,
            stale_after_hours: Some(24 * 365),
        },
        effective_stale_after_hours: 24 * 365,
    }
}

/// Generate an estate of `params`' shape on `site`.
pub fn generate_large_estate(
    params: &EstateParams,
    site: &Site,
    organization_id: Uuid,
    now: DateTime<Utc>,
) -> LargeEstate {
    let mut rng = fastrand::Rng::with_seed(params.seed);
    let plan = endpoints::plan_attachments(params, &mut rng);

    let mut addressing = build_segments(
        site,
        organization_id,
        params.vlans,
        |role| match role {
            Role::Management => params.switch_count() + params.firewalls,
            Role::Office => plan.demand(Role::Office) + params.firewalls,
            Role::OtDmz => plan.demand(Role::OtDmz) + params.firewalls,
            other => plan.demand(other),
        },
        now,
    );

    let mut services = ServiceCache::default();
    let fabric = switches::build_fabric(
        params,
        plan.edge_ports(),
        site,
        &mut addressing,
        &mut services,
        &mut rng,
        now,
    );
    let attached = endpoints::build_attached_hosts(
        &plan,
        site,
        &mut addressing,
        &mut services,
        fabric.access_ports(),
        now,
    );

    let switches::Fabric {
        mut hosts_with_services,
        mut interfaces,
        neighbor_updates,
        host_neighbor_updates,
        ..
    } = fabric;

    // Each access port that learned an attached host's MAC records it, as the bridge FDB does.
    let interface_index: HashMap<(Uuid, i32), usize> = interfaces
        .iter()
        .enumerate()
        .filter_map(|(i, iface)| Some(((iface.base.host_id, iface.base.if_index?), i)))
        .collect();
    for host in &attached.placements {
        let iface = &mut interfaces[interface_index[&(host.switch_host_id, host.port)]];
        iface.base.fdb_macs = Some(FdbMacs::new(
            FdbMacsValue(vec![host.mac.to_string()]),
            switches::SNMP,
        ));
        // The segment the host's address came from, which is the office's when the plan has no
        // VLAN of the host's own role.
        iface.base.native_vlan_id = Some(addressing.segment(host.role).vlan.id);
        iface.base.oper_status = Some(IfOperStatus::Up);
        iface.base.speed_bps = Some(host.speed_bps);
        iface.base.if_alias = Some(host.name.clone());
    }

    let daemon_host_id = attached.daemon_host_id;
    hosts_with_services.extend(attached.hosts);

    let subnet_vlan_records = generate_subnet_vlan_records(&interfaces, &hosts_with_services, now);
    let (subnets, vlans) = addressing.into_parts();

    LargeEstate {
        subnets,
        vlans,
        subnet_vlan_records,
        hosts_with_services,
        interfaces,
        neighbor_updates,
        host_neighbor_updates,
        daemon_host_id,
    }
}

/// A service a generated host runs: the definition id, the service's name, and the port it
/// answered on (`None` binds the service to the address).
type ServiceSpec = (&'static str, &'static str, Option<PortType>);

/// Service definitions by id. `ServiceDefinitionRegistry::find_by_id` builds every registered
/// definition per call, which at 15,000 hosts would be most of the generation time.
#[derive(Default)]
struct ServiceCache {
    definitions: HashMap<&'static str, Box<dyn ServiceDefinition>>,
}

impl ServiceCache {
    /// Bundles a host with its address and services. Panics on a definition id the registry
    /// doesn't know, so every test that generates the demo fails when a definition is renamed.
    fn host_with_services(
        &mut self,
        (host, ip_address): (Host, IPAddress),
        specs: &[ServiceSpec],
        now: DateTime<Utc>,
    ) -> HostWithServices {
        let mut ports = Vec::new();
        let mut services = Vec::new();
        for &(def_id, name, port_type) in specs {
            let definition = self
                .definitions
                .entry(def_id)
                .or_insert_with(|| {
                    ServiceDefinitionRegistry::find_by_id(def_id)
                        .unwrap_or_else(|| panic!("no service definition has the id {def_id:?}"))
                })
                .clone();
            let (service, port) =
                build_service(definition, name, &host, &ip_address, port_type, vec![], now);
            ports.extend(port);
            services.push(service);
        }
        found_by_scan(HostWithServices {
            host,
            ip_addresses: vec![ip_address],
            ports,
            services,
        })
    }
}
