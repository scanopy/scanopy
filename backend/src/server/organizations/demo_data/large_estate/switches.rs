//! The switch fabric and firewalls: core, distribution and access tiers, linked port to port.

use super::*;

/// Every estate interface, and its VLAN membership, reads as an SNMP walk of its device.
pub(super) const SNMP: AttributeSource = AttributeSource::Probe(ClientProbe::Snmp);

const GIGABIT: i64 = 1_000_000_000;

/// Ports a switch uses toward the tier above. Access switches have four and use one or two.
const ACCESS_UPLINK_PORTS: i32 = 4;

/// What a switch tier is built from.
struct Model {
    manufacturer: &'static str,
    model: &'static str,
    sys_descr: &'static str,
    sys_object_id: &'static str,
    port_name: &'static str,
    uplink_port_name: &'static str,
}

const CORE_MODEL: Model = Model {
    manufacturer: "Cisco",
    model: "C9606R",
    sys_descr: "Cisco IOS Software [Cupertino], Catalyst L3 Switch Software (CAT9K_IOSXE), Version 17.9.4a, RELEASE SOFTWARE (fc3)",
    sys_object_id: "1.3.6.1.4.1.9.1.2530",
    port_name: "HundredGigE1/0/",
    uplink_port_name: "HundredGigE1/0/",
};

const DISTRIBUTION_MODEL: Model = Model {
    manufacturer: "Cisco",
    model: "C9500-48Y4C",
    sys_descr: "Cisco IOS Software [Cupertino], Catalyst L3 Switch Software (CAT9K_IOSXE), Version 17.9.4a, RELEASE SOFTWARE (fc3)",
    sys_object_id: "1.3.6.1.4.1.9.1.2494",
    port_name: "TwentyFiveGigE1/0/",
    uplink_port_name: "HundredGigE1/0/",
};

const OFFICE_ACCESS_MODEL: Model = Model {
    manufacturer: "Cisco",
    model: "C9200L-48P-4X",
    sys_descr: "Cisco IOS Software [Cupertino], Catalyst L3 Switch Software (CAT9K_LITE_IOSXE), Version 17.9.4a, RELEASE SOFTWARE (fc3)",
    sys_object_id: "1.3.6.1.4.1.9.1.2655",
    port_name: "GigabitEthernet1/0/",
    uplink_port_name: "TenGigabitEthernet1/1/",
};

const PLANT_ACCESS_MODEL: Model = Model {
    manufacturer: "Hirschmann",
    model: "GRS1042",
    sys_descr: "Hirschmann GREYHOUND Switch GRS1042 HiOS-3S-09.1.03",
    sys_object_id: "1.3.6.1.4.1.248.11.2.1.3",
    port_name: "1/",
    uplink_port_name: "2/",
};

const FIREWALL_MODEL: Model = Model {
    manufacturer: "Fortinet",
    model: "FortiGate-600F",
    sys_descr: "FortiGate-600F v7.4.4,build2662,240514 (GA.F)",
    sys_object_id: "1.3.6.1.4.1.12356.101.1.6006",
    port_name: "port",
    uplink_port_name: "port",
};

/// An access switch, which attached hosts are cabled to.
pub(super) struct AccessSwitch {
    pub host_id: Uuid,
}

pub(super) struct Fabric {
    pub hosts_with_services: Vec<HostWithServices>,
    pub interfaces: Vec<Interface>,
    pub neighbor_updates: Vec<NeighborUpdate>,
    pub host_neighbor_updates: Vec<HostNeighborUpdate>,
    access: Vec<AccessSwitch>,
}

impl Fabric {
    pub fn access_ports(&self) -> &[AccessSwitch] {
        &self.access
    }

    /// Authors a port-to-port link from both ends, as each switch's LLDP table records it.
    fn link(&mut self, a: (&str, i32), b: (&str, i32)) {
        for ((source, source_port), (target, target_port)) in [(a, b), (b, a)] {
            self.neighbor_updates.push(NeighborUpdate {
                source_host_name: source.to_string(),
                source_if_index: source_port,
                target_host_name: target.to_string(),
                target_if_index: Some(target_port),
            });
        }
    }
}

/// Zero-padded host names, so they sort in tier order.
pub(super) fn numbered(prefix: &str, index: usize, count: usize) -> String {
    let width = count.to_string().len().max(2);
    format!("{prefix}-{:0width$}", index + 1)
}

/// A MAC for an estate device. Locally administered, so no OUI lookup claims a vendor for it;
/// the SNMP reading names the vendor.
fn device_mac(tier: u8, index: usize, port: u8) -> [u8; 6] {
    [0x02, 0x5c, tier, (index >> 8) as u8, index as u8, port]
}

pub(super) fn build_fabric(
    params: &EstateParams,
    edge_ports: &[i32],
    site: &Site,
    addressing: &mut Addressing,
    services: &mut ServiceCache,
    rng: &mut fastrand::Rng,
    now: DateTime<Utc>,
) -> Fabric {
    let mut fabric = Fabric {
        hosts_with_services: Vec::new(),
        interfaces: Vec::new(),
        neighbor_updates: Vec::new(),
        host_neighbor_updates: Vec::new(),
        access: Vec::new(),
    };
    let cores = params.cores.max(1);
    let distribution = params.distribution.max(1);
    let mgmt_vlan = addressing.segment(Role::Management).vlan.id;
    let all_vlans: Vec<Uuid> = addressing.segments().iter().map(|s| s.vlan.id).collect();

    let core_names: Vec<String> = (0..cores).map(|i| numbered("core-sw", i, cores)).collect();
    let dist_names: Vec<String> = (0..distribution)
        .map(|i| numbered("dist-sw", i, distribution))
        .collect();

    // Each access switch's distribution uplinks, decided first so every distribution switch
    // knows how many downlink ports it needs.
    let access_uplinks: Vec<Vec<usize>> = (0..params.access)
        .map(|a| {
            let primary = a % distribution;
            let dual = distribution > 1 && rng.u8(0..100) < params.dual_homed_access_pct;
            if dual {
                vec![
                    primary,
                    (primary + (distribution / 2).max(1)) % distribution,
                ]
            } else {
                vec![primary]
            }
        })
        .collect();
    let mut downlinks_per_dist = vec![0i32; distribution];
    for uplinks in &access_uplinks {
        for &d in uplinks {
            downlinks_per_dist[d] += 1;
        }
    }

    // Core ports: one per distribution switch, then four for the links between cores (two to
    // the next core, two from the previous), then two per firewall.
    let core_dist_port = |d: usize| d as i32 + 1;
    let core_next_port = |lane: i32| distribution as i32 + 1 + lane;
    let core_prev_port = |lane: i32| distribution as i32 + 3 + lane;
    let core_fw_port = |f: usize, side: i32| distribution as i32 + 5 + 2 * f as i32 + side;
    let core_ports = round_up(core_fw_port(params.firewalls, 0) - 1, 8).max(32);

    for (i, name) in core_names.iter().enumerate() {
        let host = switch_host(
            name,
            &CORE_MODEL,
            "Core",
            device_mac(1, i, 0),
            site,
            addressing,
            services,
            now,
        );
        let ports: Vec<(i32, i64)> = (1..=core_ports).map(|p| (p, 100 * GIGABIT)).collect();
        add_switch_interfaces(
            &mut fabric,
            &host,
            &CORE_MODEL,
            &ports,
            &[],
            device_mac(1, i, 0),
            mgmt_vlan,
            &all_vlans,
            now,
        );
        fabric.hosts_with_services.push(host);
    }
    for i in 1..cores {
        for lane in 0..2 {
            fabric.link(
                (&core_names[i - 1], core_next_port(lane)),
                (&core_names[i], core_prev_port(lane)),
            );
        }
    }

    // Distribution: uplinks to up to two cores on ports 1 and 2, downlinks from port 3.
    let mut next_downlink: Vec<i32> = vec![3; distribution];
    // Ports cabled to something no link row records from their own side.
    let mut cabled: Vec<(String, i32)> = Vec::new();
    for (d, name) in dist_names.iter().enumerate() {
        let host = switch_host(
            name,
            &DISTRIBUTION_MODEL,
            "Distribution",
            device_mac(2, d, 0),
            site,
            addressing,
            services,
            now,
        );
        let used = 2 + downlinks_per_dist[d];
        let port_count = round_up(used, 8).max(24);
        let ports: Vec<(i32, i64)> = (1..=port_count)
            .map(|p| (p, if p <= 2 { 100 } else { 25 } * GIGABIT))
            .collect();
        add_switch_interfaces(
            &mut fabric,
            &host,
            &DISTRIBUTION_MODEL,
            &ports,
            &[1, 2],
            device_mac(2, d, 0),
            mgmt_vlan,
            &all_vlans,
            now,
        );
        fabric.hosts_with_services.push(host);
        for (lane, core) in [d % cores, (d + 1) % cores].into_iter().enumerate() {
            if lane == 1 && cores == 1 {
                break;
            }
            fabric.link(
                (name, lane as i32 + 1),
                (&core_names[core], core_dist_port(d)),
            );
        }
    }

    // Access: edge ports first, then four uplink ports.
    for (a, uplinks) in access_uplinks.iter().enumerate() {
        let name = numbered("acc-sw", a, params.access);
        // Plant areas run industrial switches; the office and server rooms run Cisco.
        let model = if rng.u8(0..100) < 40 {
            &PLANT_ACCESS_MODEL
        } else {
            &OFFICE_ACCESS_MODEL
        };
        let host = switch_host(
            &name,
            model,
            "Access",
            device_mac(3, a, 0),
            site,
            addressing,
            services,
            now,
        );
        let edge = edge_ports[a];
        let ports: Vec<(i32, i64)> = (1..=edge)
            .map(|p| (p, GIGABIT))
            .chain((edge + 1..=edge + ACCESS_UPLINK_PORTS).map(|p| (p, 10 * GIGABIT)))
            .collect();
        let uplink_ports: Vec<i32> = (edge + 1..=edge + ACCESS_UPLINK_PORTS).collect();
        add_switch_interfaces(
            &mut fabric,
            &host,
            model,
            &ports,
            &uplink_ports,
            device_mac(3, a, 0),
            mgmt_vlan,
            &all_vlans,
            now,
        );
        fabric.access.push(AccessSwitch {
            host_id: host.host.id,
        });
        fabric.hosts_with_services.push(host);

        for (lane, &d) in uplinks.iter().enumerate() {
            let source_port = edge + 1 + lane as i32;
            let dist_port = next_downlink[d];
            next_downlink[d] += 1;
            if rng.u8(0..100) < params.device_level_pct {
                // The distribution switch advertised a port this access switch's LLDP could not
                // match, and the distribution switch's own table never listed the access switch.
                fabric.host_neighbor_updates.push(HostNeighborUpdate {
                    source_host_name: name.clone(),
                    source_if_index: source_port,
                    target_host_name: dist_names[d].clone(),
                });
                cabled.push((dist_names[d].clone(), dist_port));
            } else {
                fabric.link((&name, source_port), (&dist_names[d], dist_port));
            }
        }
    }

    // Firewalls: an OT DMZ leg and an office leg, each cabled to a core.
    for f in 0..params.firewalls {
        let name = numbered("fw", f, params.firewalls);
        let host = firewall_host(&name, f, site, addressing, services, now);
        let legs = [
            (Role::OtDmz, host.ip_addresses[0].id),
            (Role::Office, host.ip_addresses[1].id),
        ];
        for (side, (role, ip_address_id)) in legs.into_iter().enumerate() {
            let side = side as i32;
            let mut iface = port_interface(
                &host.host,
                side + 1,
                format!("{}{}", FIREWALL_MODEL.port_name, side + 1),
                10 * GIGABIT,
                device_mac(4, f, side as u8 + 1),
                true,
                now,
            );
            iface.base.ip_address_id = Some(ip_address_id);
            iface.base.native_vlan_id = Some(addressing.segment(role).vlan.id);
            fabric.interfaces.push(iface);
            let core = (f + side as usize) % cores;
            fabric.link(
                (&name, side + 1),
                (&core_names[core], core_fw_port(f, side)),
            );
        }
        fabric.hosts_with_services.push(host);
    }

    // Every cabled port is up.
    let linked: std::collections::HashSet<(String, i32)> = fabric
        .neighbor_updates
        .iter()
        .map(|n| (n.source_host_name.clone(), n.source_if_index))
        .chain(
            fabric
                .host_neighbor_updates
                .iter()
                .map(|n| (n.source_host_name.clone(), n.source_if_index)),
        )
        .chain(cabled)
        .collect();
    let names: HashMap<Uuid, String> = fabric
        .hosts_with_services
        .iter()
        .map(|h| (h.host.id, h.host.base.name.to_string()))
        .collect();
    for iface in &mut fabric.interfaces {
        if let Some(if_index) = iface.base.if_index
            && linked.contains(&(names[&iface.base.host_id].clone(), if_index))
        {
            iface.base.oper_status = Some(IfOperStatus::Up);
        }
    }

    fabric
}

fn round_up(n: i32, step: i32) -> i32 {
    (n + step - 1) / step * step
}

#[allow(clippy::too_many_arguments)]
fn switch_host(
    name: &str,
    model: &Model,
    tier: &str,
    chassis_mac: [u8; 6],
    site: &Site,
    addressing: &mut Addressing,
    services: &mut ServiceCache,
    now: DateTime<Utc>,
) -> HostWithServices {
    let ip = addressing.segment_mut(Role::Management).allocate();
    let mgmt = &addressing.segment(Role::Management).subnet;
    let chassis = MacAddress::new(chassis_mac).to_string();
    let serial = format!(
        "{}{:06X}",
        &model.model[..3].to_uppercase(),
        u32::from(ip) & 0xff_ffff
    );
    services.host_with_services(
        with_snmp(
            with_mac(
                create_host(
                    name,
                    Some(&format!("{name}.plant.local")),
                    Some(&format!("{tier} switch")),
                    site,
                    mgmt,
                    ip,
                    vec![],
                    None,
                    None,
                    now,
                ),
                chassis_mac,
            ),
            Some(model.sys_descr),
            Some(model.sys_object_id),
            Some(&format!("Plant / {tier}")),
            Some("Plant network team"),
            Some(&chassis),
            Some(model.manufacturer),
            Some(model.model),
            Some(&serial),
        ),
        &[
            ("SNMP", "SNMP", Some(PortType::Snmp)),
            ("SSH", "SSH", Some(PortType::Ssh)),
            ("Switch", "Switch", None),
        ],
        now,
    )
}

fn firewall_host(
    name: &str,
    index: usize,
    site: &Site,
    addressing: &mut Addressing,
    services: &mut ServiceCache,
    now: DateTime<Utc>,
) -> HostWithServices {
    let dmz_ip = addressing.segment_mut(Role::OtDmz).allocate();
    let office_ip = addressing.segment_mut(Role::Office).allocate();
    let chassis_mac = device_mac(4, index, 0);
    let mut host = services.host_with_services(
        with_snmp(
            with_mac(
                create_host(
                    name,
                    Some(&format!("{name}.plant.local")),
                    Some("Firewall between the plant and business networks"),
                    site,
                    &addressing.segment(Role::OtDmz).subnet,
                    dmz_ip,
                    vec![],
                    None,
                    None,
                    now,
                ),
                device_mac(4, index, 1),
            ),
            Some(FIREWALL_MODEL.sys_descr),
            Some(FIREWALL_MODEL.sys_object_id),
            Some("Plant / Server room"),
            Some("Plant network team"),
            Some(&MacAddress::new(chassis_mac).to_string()),
            Some(FIREWALL_MODEL.manufacturer),
            Some(FIREWALL_MODEL.model),
            Some(&format!("FG6H0F{:08}", index + 1)),
        ),
        &[
            ("SNMP", "SNMP", Some(PortType::Snmp)),
            ("Fortinet", "FortiGate", Some(PortType::Https)),
        ],
        now,
    );
    let mut office = host.ip_addresses[0].clone();
    office.id = Uuid::new_v4();
    office.base.subnet_id = addressing.segment(Role::Office).subnet.id;
    office.base.ip_address = IpAddr::V4(office_ip);
    office.base.name = Some("port2".to_string());
    office.base.position = 1;
    office.base.mac_address = Some(MacEvidence::new(
        MacEvidenceValue(MacAddress::new(device_mac(4, index, 2))),
        AttributeSource::ArpReply,
    ));
    host.ip_addresses[0].base.name = Some("port1".to_string());
    host.ip_addresses.push(office);
    host
}

/// A switch's ports, plus the management SVI that holds its address. `trunks` are the uplink
/// ports, tagged with every VLAN.
#[allow(clippy::too_many_arguments)]
fn add_switch_interfaces(
    fabric: &mut Fabric,
    host: &HostWithServices,
    model: &Model,
    ports: &[(i32, i64)],
    trunks: &[i32],
    chassis_mac: [u8; 6],
    mgmt_vlan: Uuid,
    all_vlans: &[Uuid],
    now: DateTime<Utc>,
) {
    for &(if_index, speed) in ports {
        let trunk = trunks.contains(&if_index);
        let mut mac = chassis_mac;
        mac[5] = if_index as u8;
        let name = if trunk {
            model.uplink_port_name
        } else {
            model.port_name
        };
        let mut iface = port_interface(
            &host.host,
            if_index,
            format!("{name}{if_index}"),
            speed,
            mac,
            false,
            now,
        );
        if trunk {
            iface.base.vlan_ids = Some(all_vlans.to_vec());
        }
        fabric.interfaces.push(iface);
    }

    let mut svi = port_interface(
        &host.host,
        // Above any physical port, as switches number their VLAN interfaces.
        10_000 + i32::from(VLAN_PLAN[0].number),
        format!("Vlan{}", VLAN_PLAN[0].number),
        GIGABIT,
        chassis_mac,
        true,
        now,
    );
    svi.base.if_type = Some(53);
    svi.base.ip_address_id = Some(host.ip_addresses[0].id);
    svi.base.native_vlan_id = Some(mgmt_vlan);
    fabric.interfaces.push(svi);
}

pub(super) fn port_interface(
    host: &Host,
    if_index: i32,
    descr: String,
    speed_bps: i64,
    mac: [u8; 6],
    up: bool,
    now: DateTime<Utc>,
) -> Interface {
    Interface {
        display_name: None,
        valid_from: now,
        valid_to: None,
        lineage_id: None,
        last_seen_at: now,
        last_discovery_id: None,
        first_discovery_id: None,
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: InterfaceBase {
            neighbor_candidates: Default::default(),
            host_id: host.id,
            site_id: host.base.site_id,
            if_index: Some(if_index),
            if_descr: Some(descr),
            if_name: None,
            if_alias: None,
            if_type: Some(6),
            speed_bps: Some(speed_bps),
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(if up {
                IfOperStatus::Up
            } else {
                IfOperStatus::Down
            }),
            mac_address: Some(MacEvidence::new(
                MacEvidenceValue(MacAddress::new(mac)),
                SNMP,
            )),
            ip_address_id: None,
            ip_configured: false,
            fdb_macs: None,
            interface_source: SNMP,
            vlan_membership_source: SNMP,
            native_vlan_id: None,
            vlan_ids: None,
        },
    }
}
