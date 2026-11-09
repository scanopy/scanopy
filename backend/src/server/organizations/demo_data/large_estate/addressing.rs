//! The estate's VLAN and subnet plan: one subnet per VLAN, each sized for the hosts placed on it.

use super::*;
use crate::server::subnets::r#impl::base::{SubnetName, SubnetTypeAttributed};

/// What a VLAN carries. Endpoints, servers and switches are placed by role, so a smaller plan
/// (`EstateParams::vlans` below the table's length) still has somewhere to put every host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Role {
    Management,
    Servers,
    Office,
    ProcessControlA,
    Voice,
    Printers,
    Wireless,
    Guest,
    Contractors,
    ProcessControlB,
    Safety,
    Cctv,
    Analyzers,
    FieldGateways,
    OtDmz,
    Engineering,
    BuildingAutomation,
    Lab,
}

pub(super) struct VlanSpec {
    pub role: Role,
    pub number: u16,
    pub name: &'static str,
    pub description: &'static str,
    pub subnet_type: SubnetType,
}

/// The full plan, in the order `EstateParams::vlans` truncates it. The first four are the floor:
/// management, servers, an office network and a control network.
pub(super) const VLAN_PLAN: [VlanSpec; 18] = [
    VlanSpec {
        role: Role::Management,
        number: 10,
        name: "Management",
        description: "Switch and firewall management interfaces.",
        subnet_type: SubnetType::Management,
    },
    VlanSpec {
        role: Role::Servers,
        number: 20,
        name: "Servers",
        description: "Site servers: historian, SCADA, directory, DNS, NTP and monitoring.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::Office,
        number: 30,
        name: "Office",
        description: "Office workstations in the admin building.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::ProcessControlA,
        number: 100,
        name: "Process Control A",
        description: "PLCs and HMIs for process units 1 to 4.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::Voice,
        number: 40,
        name: "Voice",
        description: "Desk and control room phones.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::Printers,
        number: 50,
        name: "Printers",
        description: "Office and warehouse printers.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::Wireless,
        number: 60,
        name: "Wireless Infrastructure",
        description: "Access point management addresses.",
        subnet_type: SubnetType::WiFi,
    },
    VlanSpec {
        role: Role::Guest,
        number: 70,
        name: "Guest",
        description: "Visitor Wi-Fi. Internet access only.",
        subnet_type: SubnetType::Guest,
    },
    VlanSpec {
        role: Role::Contractors,
        number: 80,
        name: "Contractors",
        description: "Contractor laptops during turnarounds.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::ProcessControlB,
        number: 101,
        name: "Process Control B",
        description: "PLCs and HMIs for process units 5 to 8 and the tank farm.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::Safety,
        number: 110,
        name: "Safety Systems",
        description: "Safety instrumented system controllers.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::Cctv,
        number: 120,
        name: "CCTV",
        description: "Perimeter and process area cameras.",
        subnet_type: SubnetType::IoT,
    },
    VlanSpec {
        role: Role::Analyzers,
        number: 130,
        name: "Analyzers",
        description: "Gas chromatographs and online process analyzers.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::FieldGateways,
        number: 140,
        name: "Field Gateways",
        description: "Serial-to-Ethernet converters and fieldbus gateways.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::OtDmz,
        number: 150,
        name: "OT DMZ",
        description: "Between the plant and business networks. Firewalled both ways.",
        subnet_type: SubnetType::Dmz,
    },
    VlanSpec {
        role: Role::Engineering,
        number: 160,
        name: "Engineering",
        description: "Engineering workstations that program the controllers.",
        subnet_type: SubnetType::Lan,
    },
    VlanSpec {
        role: Role::BuildingAutomation,
        number: 170,
        name: "Building Automation",
        description: "HVAC and access control panels.",
        subnet_type: SubnetType::IoT,
    },
    VlanSpec {
        role: Role::Lab,
        number: 180,
        name: "Lab",
        description: "Quality lab instruments.",
        subnet_type: SubnetType::Lan,
    },
];

/// The first address of the estate's range. Subnets are carved from here upwards.
const ESTATE_BASE: Ipv4Addr = Ipv4Addr::new(10, 40, 0, 0);

/// One VLAN of the plan with its subnet and the addresses handed out on it so far.
pub(super) struct Segment {
    pub vlan: Vlan,
    pub subnet: Subnet,
    network: u32,
    capacity: u32,
    next_host: u32,
}

impl Segment {
    /// The next free host address. Panics when the subnet is full, which means the sizing in
    /// [`build_segments`] undercounted.
    pub fn allocate(&mut self) -> Ipv4Addr {
        self.next_host += 1;
        assert!(
            self.next_host <= self.capacity,
            "subnet {} is full",
            self.subnet.base.name
        );
        Ipv4Addr::from(self.network + self.next_host)
    }
}

/// The estate's segments, keyed by role. A role the truncated plan leaves out resolves to the
/// office segment, so every host still lands on a subnet.
pub(super) struct Addressing {
    segments: Vec<Segment>,
    index: HashMap<Role, usize>,
}

impl Addressing {
    pub fn index_of(&self, role: Role) -> usize {
        self.index
            .get(&role)
            .or_else(|| self.index.get(&Role::Office))
            .copied()
            .expect("the plan always holds the office segment")
    }

    pub fn segment(&self, role: Role) -> &Segment {
        &self.segments[self.index_of(role)]
    }

    pub fn segment_mut(&mut self, role: Role) -> &mut Segment {
        let i = self.index_of(role);
        &mut self.segments[i]
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn into_parts(self) -> (Vec<Subnet>, Vec<Vlan>) {
        self.segments
            .into_iter()
            .map(|s| (s.subnet, s.vlan))
            .unzip()
    }
}

/// Builds the first `vlan_count` segments of [`VLAN_PLAN`] (at least four), each subnet sized to
/// hold `demand(role)` hosts with a quarter spare, never smaller than a /24.
pub(super) fn build_segments(
    site: &Site,
    organization_id: Uuid,
    vlan_count: usize,
    demand: impl Fn(Role) -> usize,
    now: DateTime<Utc>,
) -> Addressing {
    let mut segments = Vec::new();
    let mut index = HashMap::new();
    let mut cursor = u32::from(ESTATE_BASE);

    let (included, folded) = VLAN_PLAN.split_at(vlan_count.clamp(4, VLAN_PLAN.len()));
    // The office segment also holds the hosts of every role the plan leaves out.
    let folded_demand: usize = folded.iter().map(|s| demand(s.role)).sum();
    for spec in included {
        let hosts = demand(spec.role)
            + if spec.role == Role::Office {
                folded_demand
            } else {
                0
            };
        let wanted = (hosts as u32 * 5 / 4 + 2).max(254);
        let host_bits = 32 - (wanted + 2).next_power_of_two().leading_zeros() - 1;
        let prefix = (32 - host_bits).min(24) as u8;
        let size = 1u32 << (32 - prefix);
        // Align the network to its own size.
        let network = cursor.div_ceil(size) * size;
        cursor = network + size;

        let cidr = Ipv4Cidr::new(Ipv4Addr::from(network), prefix).expect("aligned network");
        let subnet = Subnet {
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                vlan_ids: Vec::new(),
                cidr: SubnetCidr::new(
                    SubnetCidrValue(IpCidr::V4(cidr)),
                    AttributeSource::DaemonSelfReport,
                ),
                site_id: site.id,
                name: SubnetName::unattributed(format!("Plant {}", spec.name)),
                description: Some(spec.description.to_string()),
                subnet_type: SubnetTypeAttributed::unattributed(spec.subnet_type),
                virtualization_service_id: None,
                source: EntitySource::Discovery,
                tags: vec![],
            },
        };
        let vlan = Vlan {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base: VlanBase {
                vlan_number: spec.number,
                name: VlanName::new(
                    VlanNameValue(spec.name.to_string()),
                    AttributeSource::Authored(ClientProbe::Snmp),
                ),
                description: Some(spec.description.to_string()),
                site_id: site.id,
                organization_id,
                source: EntitySource::Discovery,
                subnet_ids: Vec::new(),
                interface_ids: Vec::new(),
                tags: vec![],
            },
        };

        index.insert(spec.role, segments.len());
        segments.push(Segment {
            vlan,
            subnet,
            network,
            // The first address is the gateway, held by the core switches, and the last is the
            // broadcast address.
            capacity: size - 2,
            next_host: 1,
        });
    }

    Addressing { segments, index }
}
