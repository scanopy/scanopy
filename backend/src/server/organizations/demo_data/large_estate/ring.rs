//! A ring estate: field stations on fiber rings that meet at two core switches, each at a
//! treatment plant.
//!
//! The demo seeds it as its "Water Utility" site, so an operator whose network is a ring can see
//! the L2 view draw the rings and trace a path around them.

use super::addressing::{Addressing, Role, VlanSpec, build_segments};
use super::endpoints::{Cabled, build_attached_hosts, plan_by_switch};
use super::switches::{RingPlan, RingSwitch, build_ring_fabric};
use super::*;

/// The demo site the ring estate is seeded as.
pub const WATER_UTILITY_SITE_NAME: &str = "Water Utility";

pub const WATER_UTILITY_NAMING: SiteNaming = SiteNaming {
    label: WATER_UTILITY_SITE_NAME,
    domain: "water.local",
    base: Ipv4Addr::new(10, 60, 0, 0),
    // Above any index the default estate reaches, so no attached host shares a Plant host's MAC.
    serial_base: 0x80_0000,
};

/// The shape of a ring estate.
#[derive(Debug, Clone)]
pub struct RingParams {
    /// Each ring's name and how many station switches it has.
    pub rings: Vec<(&'static str, usize)>,
    /// Seeds the station mix and switch models.
    pub seed: u64,
}

impl Default for RingParams {
    /// Two rings of 49 stations, which with the two cores is 100 switches.
    fn default() -> Self {
        Self {
            rings: vec![("East", 49), ("West", 49)],
            seed: 0x7a7e_5ca7,
        }
    }
}

impl RingParams {
    pub fn switch_count(&self) -> usize {
        2 + self.rings.iter().map(|(_, n)| n).sum::<usize>()
    }
}

/// A type of field station: its host-name code, what it is called, and what is cabled to its
/// switch beyond the controller and the RTU every station has.
struct StationType {
    code: &'static str,
    name: &'static str,
    hmi: bool,
    cameras: usize,
}

/// Stations take these types in turn around each ring.
const STATION_TYPES: [StationType; 5] = [
    StationType {
        code: "ps",
        name: "Pump Station",
        hmi: true,
        cameras: 2,
    },
    StationType {
        code: "ls",
        name: "Lift Station",
        hmi: false,
        cameras: 1,
    },
    StationType {
        code: "tank",
        name: "Reservoir",
        hmi: false,
        cameras: 1,
    },
    StationType {
        code: "well",
        name: "Well",
        hmi: false,
        cameras: 1,
    },
    StationType {
        code: "bs",
        name: "Booster Station",
        hmi: true,
        cameras: 1,
    },
];

/// The two cores: host name, what they are called, and the servers in each plant's control room.
/// The daemon runs on the north plant's monitoring server.
const CORES: [(&str, &str, &[(&str, &str)]); 2] = [
    (
        "wtp-north-core",
        "North Treatment Plant core",
        &[
            ("mon", "wtp-north-mon"),
            ("scada", "wtp-north-scada"),
            ("hist", "wtp-north-hist"),
            ("ntp", "wtp-north-ntp"),
        ],
    ),
    (
        "wtp-south-core",
        "South Treatment Plant core",
        &[
            ("scada", "wtp-south-scada"),
            ("hist", "wtp-south-hist"),
            ("log", "wtp-south-log"),
        ],
    ),
];

/// The site's VLANs. Air-gapped, so there is no office or DMZ network: the Operations segment
/// holds the operators' machines and any role the plan has no VLAN for.
const RING_VLAN_PLAN: [VlanSpec; 6] = [
    VlanSpec {
        role: Role::Management,
        number: 10,
        name: "Management",
        description: "Switch management interfaces.",
        subnet_type: SubnetType::Management,
    },
    VlanSpec {
        role: Role::Servers,
        number: 20,
        name: "Control Room Servers",
        description: "SCADA, monitoring, time and log servers at the two treatment plants.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::Office,
        number: 30,
        name: "Operations",
        description: "Operator machines in the treatment plant control rooms.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::ProcessControlA,
        number: 100,
        name: "Station Control",
        description: "Station PLCs and operator panels.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::FieldGateways,
        number: 140,
        name: "Telemetry",
        description: "Remote terminal units reporting levels, flows and pump states.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::Cctv,
        number: 120,
        name: "Station Cameras",
        description: "Perimeter and wet well cameras at the stations.",
        subnet_type: SubnetType::IoT,
    },
];

/// Every switch's name, role and attached hosts, in [`RingPlan::switches`] order.
fn plan_switches(params: &RingParams) -> (RingPlan, Vec<Vec<Cabled>>) {
    let mut loads: Vec<Vec<Cabled>> = Vec::new();
    let mut core_switch = |(name, role, servers): (&str, &str, &[(&'static str, &str)])| {
        loads.push(
            servers
                .iter()
                .map(|&(prefix, host)| Cabled::Server(prefix, host.to_string()))
                .collect(),
        );
        RingSwitch {
            name: name.to_string(),
            role: role.to_string(),
            edge_ports: 8,
        }
    };
    let cores = [core_switch(CORES[0]), core_switch(CORES[1])];

    let mut rings = Vec::new();
    for (r, &(ring, stations)) in params.rings.iter().enumerate() {
        let mut numbers = [0usize; STATION_TYPES.len()];
        let mut switches = Vec::new();
        for i in 0..stations {
            // Offset per ring, so the two rings don't start with the same type.
            let t = (i + 2 * r) % STATION_TYPES.len();
            let station_type = &STATION_TYPES[t];
            numbers[t] += 1;
            let number = numbers[t];
            let name = format!("{}-{}{:02}", ring.to_lowercase(), station_type.code, number);

            let mut load = vec![
                Cabled::Endpoint("plc-a", format!("{name}-plc")),
                Cabled::Endpoint("rtu", format!("{name}-rtu")),
            ];
            if station_type.hmi {
                load.push(Cabled::Endpoint("hmi-a", format!("{name}-hmi")));
            }
            for c in 1..=station_type.cameras {
                load.push(Cabled::Endpoint("cam", format!("{name}-cam{c}")));
            }
            loads.push(load);
            switches.push(RingSwitch {
                name,
                role: format!("{ring} ring, {} {number}", station_type.name),
                edge_ports: 8,
            });
        }
        rings.push(switches);
    }
    (RingPlan { cores, rings }, loads)
}

/// Generate a ring estate of `params`' shape on `site`.
pub fn generate_ring_estate(
    params: &RingParams,
    site: &Site,
    organization_id: Uuid,
    now: DateTime<Utc>,
) -> LargeEstate {
    let mut rng = fastrand::Rng::with_seed(params.seed);
    let (ring_plan, loads) = plan_switches(params);
    let plan = plan_by_switch(&loads, 8, &mut rng);

    let mut addressing: Addressing = build_segments(
        site,
        organization_id,
        &WATER_UTILITY_NAMING,
        &RING_VLAN_PLAN,
        |role| match role {
            Role::Management => params.switch_count(),
            other => plan.demand(other),
        },
        now,
    );

    let mut services = ServiceCache::default();
    let fabric = build_ring_fabric(
        &ring_plan,
        site,
        &mut addressing,
        &mut services,
        &mut rng,
        now,
    );
    let attached = build_attached_hosts(
        &plan,
        site,
        &mut addressing,
        &mut services,
        fabric.access_ports(),
        now,
    );
    assemble_estate(fabric, attached, addressing, AccessLinks::Lldp, now)
}

/// The site the demo seeds the ring estate as. Created after Plant so it sorts after it.
pub fn water_utility_site(organization_id: Uuid, now: DateTime<Utc>) -> Site {
    Site {
        id: Uuid::new_v4(),
        created_at: now + Duration::seconds(3),
        updated_at: now + Duration::seconds(3),
        base: SiteBase {
            name: WATER_UTILITY_SITE_NAME.to_string(),
            organization_id,
            tags: vec![],
            credential_ids: None,
            stale_after_hours: Some(24 * 365),
        },
        effective_stale_after_hours: 24 * 365,
    }
}
