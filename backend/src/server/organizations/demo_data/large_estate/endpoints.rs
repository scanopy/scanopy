//! Endpoints and servers: an address and a MAC each, placed on an access switch port.

use super::*;
use crate::server::hosts::r#impl::device_type::DeviceType;

const GIGABIT: i64 = 1_000_000_000;
const FAST_ETHERNET: i64 = 100_000_000;

/// Servers fill the first access switches this many to a switch, as a server room does.
const SERVERS_PER_SWITCH: usize = 40;

/// Every access switch has at least this many edge ports, as a 48-port switch does.
const MIN_EDGE_PORTS: i32 = 48;

/// Who made a device and what identified it.
struct Vendor {
    /// `None` leaves the maker unrecorded, as for a workstation no probe named.
    name: Option<&'static str>,
    /// The maker's OUI, so the MAC reads as theirs. `None` uses a locally administered MAC.
    oui: Option<[u8; 3]>,
    device_type: DeviceType,
    source: AttributeSource,
    services: &'static [ServiceSpec],
}

/// A kind of endpoint and how many of every hundred endpoints it accounts for.
struct Kind {
    role: Role,
    weight: usize,
    prefix: &'static str,
    description: &'static str,
    speed_bps: i64,
    /// Whether a reverse DNS lookup names it. Office machines are in DNS; field devices are not.
    in_dns: bool,
    vendors: &'static [Vendor],
}

const WORKSTATION: &[ServiceSpec] = &[("Workstation", "Workstation", Some(PortType::Rdp))];
const MODBUS: &[ServiceSpec] = &[("Modbus TCP", "Modbus TCP", Some(PortType::ModbusTcp))];
const ETHERNET_IP: &[ServiceSpec] = &[("EtherNet/IP", "EtherNet/IP", Some(PortType::EtherNetIp))];
const RTSP: &[ServiceSpec] = &[("RTSP Camera", "Camera", Some(PortType::Rtsp))];
const BACNET: &[ServiceSpec] = &[("BACnet", "BACnet", Some(PortType::BACnet))];

const UNNAMED_PC: Vendor = Vendor {
    name: None,
    oui: None,
    device_type: DeviceType::Desktop,
    source: AttributeSource::Unspecified,
    services: WORKSTATION,
};

const CONTROLLERS: &[Vendor] = &[
    Vendor {
        name: Some("Rockwell Automation"),
        oui: Some([0x00, 0x00, 0xbc]),
        device_type: DeviceType::Plc,
        source: AttributeSource::CipVendorId,
        services: ETHERNET_IP,
    },
    Vendor {
        name: Some("Siemens"),
        oui: Some([0x00, 0x1b, 0x1b]),
        device_type: DeviceType::Plc,
        source: AttributeSource::ProfinetDcp,
        services: MODBUS,
    },
    Vendor {
        name: Some("Schneider Electric"),
        oui: Some([0x00, 0x80, 0xf4]),
        device_type: DeviceType::Plc,
        source: AttributeSource::HttpServerMatch,
        services: MODBUS,
    },
];

const HMIS: &[Vendor] = &[
    Vendor {
        name: Some("Rockwell Automation"),
        oui: Some([0x00, 0x00, 0xbc]),
        device_type: DeviceType::HmiController,
        source: AttributeSource::CipVendorId,
        services: ETHERNET_IP,
    },
    Vendor {
        name: Some("Siemens"),
        oui: Some([0x00, 0x1b, 0x1b]),
        device_type: DeviceType::HmiController,
        source: AttributeSource::ProfinetDcp,
        services: &[],
    },
];

const KINDS: &[Kind] = &[
    Kind {
        role: Role::Office,
        weight: 22,
        prefix: "ws",
        description: "Office workstation",
        speed_bps: GIGABIT,
        in_dns: true,
        vendors: &[UNNAMED_PC],
    },
    Kind {
        role: Role::Voice,
        weight: 12,
        prefix: "phone",
        description: "Desk phone",
        speed_bps: GIGABIT,
        in_dns: false,
        vendors: &[
            Vendor {
                name: Some("Yealink"),
                oui: Some([0x80, 0x5e, 0xc0]),
                device_type: DeviceType::Phone,
                source: AttributeSource::HttpServerMatch,
                services: &[],
            },
            Vendor {
                name: Some("Poly"),
                oui: Some([0x00, 0x04, 0xf2]),
                device_type: DeviceType::Phone,
                source: AttributeSource::HttpServerMatch,
                services: &[],
            },
        ],
    },
    Kind {
        role: Role::Printers,
        weight: 3,
        prefix: "prn",
        description: "Printer",
        speed_bps: GIGABIT,
        in_dns: true,
        vendors: &[
            Vendor {
                name: Some("HP"),
                oui: Some([0x3c, 0xd9, 0x2b]),
                device_type: DeviceType::Printer,
                source: AttributeSource::HtmlTitleMatch,
                services: &[("IPP Printer", "Printer", Some(PortType::Ipp))],
            },
            Vendor {
                name: Some("Brother"),
                oui: Some([0x00, 0x80, 0x77]),
                device_type: DeviceType::Printer,
                source: AttributeSource::HtmlTitleMatch,
                services: &[("IPP Printer", "Printer", Some(PortType::Ipp))],
            },
        ],
    },
    Kind {
        role: Role::Wireless,
        weight: 5,
        prefix: "ap",
        description: "Wireless access point",
        speed_bps: GIGABIT,
        in_dns: false,
        vendors: &[
            Vendor {
                name: Some("Aruba"),
                oui: Some([0x00, 0x0b, 0x86]),
                device_type: DeviceType::AccessPoint,
                source: AttributeSource::HtmlTitleMatch,
                services: &[("Access Point", "Access Point", None)],
            },
            Vendor {
                name: Some("Ruckus"),
                oui: None,
                device_type: DeviceType::AccessPoint,
                source: AttributeSource::HtmlTitleMatch,
                services: &[("Access Point", "Access Point", None)],
            },
        ],
    },
    Kind {
        role: Role::Contractors,
        weight: 2,
        prefix: "ctr",
        description: "Contractor laptop",
        speed_bps: GIGABIT,
        in_dns: false,
        vendors: &[Vendor {
            name: None,
            oui: None,
            device_type: DeviceType::Laptop,
            source: AttributeSource::Unspecified,
            services: &[],
        }],
    },
    Kind {
        role: Role::ProcessControlA,
        weight: 7,
        prefix: "plc-a",
        description: "Process controller",
        speed_bps: FAST_ETHERNET,
        in_dns: false,
        vendors: CONTROLLERS,
    },
    Kind {
        role: Role::ProcessControlB,
        weight: 7,
        prefix: "plc-b",
        description: "Process controller",
        speed_bps: FAST_ETHERNET,
        in_dns: false,
        vendors: CONTROLLERS,
    },
    Kind {
        role: Role::ProcessControlA,
        weight: 2,
        prefix: "hmi-a",
        description: "Operator panel",
        speed_bps: FAST_ETHERNET,
        in_dns: false,
        vendors: HMIS,
    },
    Kind {
        role: Role::ProcessControlB,
        weight: 2,
        prefix: "hmi-b",
        description: "Operator panel",
        speed_bps: FAST_ETHERNET,
        in_dns: false,
        vendors: HMIS,
    },
    Kind {
        role: Role::Safety,
        weight: 2,
        prefix: "sis",
        description: "Safety controller",
        speed_bps: FAST_ETHERNET,
        in_dns: false,
        vendors: &[Vendor {
            name: Some("HIMA"),
            oui: None,
            device_type: DeviceType::Plc,
            source: AttributeSource::HttpServerMatch,
            services: MODBUS,
        }],
    },
    Kind {
        role: Role::Cctv,
        weight: 10,
        prefix: "cam",
        description: "IP camera",
        speed_bps: GIGABIT,
        in_dns: false,
        vendors: &[
            Vendor {
                name: Some("AXIS"),
                oui: Some([0x00, 0x40, 0x8c]),
                device_type: DeviceType::Camera,
                source: AttributeSource::HtmlTitleMatch,
                services: RTSP,
            },
            Vendor {
                name: Some("Hanwha Vision"),
                oui: None,
                device_type: DeviceType::Camera,
                source: AttributeSource::RtspServerMatch,
                services: RTSP,
            },
        ],
    },
    Kind {
        role: Role::Analyzers,
        weight: 6,
        prefix: "ana",
        description: "Process analyzer",
        speed_bps: FAST_ETHERNET,
        in_dns: false,
        vendors: &[
            Vendor {
                name: Some("ABB"),
                oui: None,
                device_type: DeviceType::Industrial,
                source: AttributeSource::HttpServerMatch,
                services: MODBUS,
            },
            Vendor {
                name: Some("Yokogawa"),
                oui: None,
                device_type: DeviceType::Industrial,
                source: AttributeSource::HttpServerMatch,
                services: MODBUS,
            },
            Vendor {
                name: Some("Emerson"),
                oui: None,
                device_type: DeviceType::Industrial,
                source: AttributeSource::HttpServerMatch,
                services: MODBUS,
            },
        ],
    },
    Kind {
        role: Role::FieldGateways,
        weight: 6,
        prefix: "gw",
        description: "Protocol converter",
        speed_bps: FAST_ETHERNET,
        in_dns: false,
        vendors: &[
            Vendor {
                name: Some("Moxa"),
                oui: Some([0x00, 0x90, 0xe8]),
                device_type: DeviceType::DeviceServer,
                source: AttributeSource::HtmlTitleMatch,
                services: MODBUS,
            },
            Vendor {
                name: Some("HMS Industrial Networks"),
                oui: None,
                device_type: DeviceType::DeviceServer,
                source: AttributeSource::HttpServerMatch,
                services: ETHERNET_IP,
            },
        ],
    },
    Kind {
        role: Role::Engineering,
        weight: 3,
        prefix: "ews",
        description: "Engineering workstation",
        speed_bps: GIGABIT,
        in_dns: true,
        vendors: &[UNNAMED_PC],
    },
    Kind {
        role: Role::BuildingAutomation,
        weight: 3,
        prefix: "bas",
        description: "Building controller",
        speed_bps: FAST_ETHERNET,
        in_dns: false,
        vendors: &[
            Vendor {
                name: Some("Honeywell"),
                oui: Some([0x00, 0x40, 0x84]),
                device_type: DeviceType::BuildingAutomation,
                source: AttributeSource::HttpServerMatch,
                services: BACNET,
            },
            Vendor {
                name: Some("Johnson Controls"),
                oui: None,
                device_type: DeviceType::BuildingAutomation,
                source: AttributeSource::HttpServerMatch,
                services: BACNET,
            },
        ],
    },
    Kind {
        role: Role::Lab,
        weight: 2,
        prefix: "lab",
        description: "Lab instrument",
        speed_bps: GIGABIT,
        in_dns: false,
        vendors: &[
            Vendor {
                name: Some("Agilent"),
                oui: None,
                device_type: DeviceType::TestInstrument,
                source: AttributeSource::HttpServerMatch,
                services: &[],
            },
            Vendor {
                name: Some("Thermo Fisher Scientific"),
                oui: None,
                device_type: DeviceType::TestInstrument,
                source: AttributeSource::HttpServerMatch,
                services: &[],
            },
        ],
    },
];

/// A server role, taken in turn as servers are generated.
struct ServerRole {
    role: Role,
    prefix: &'static str,
    description: &'static str,
    services: &'static [ServiceSpec],
}

/// The prefix of the server role the site's daemon runs on.
const DAEMON_SERVER_PREFIX: &str = "mon";

const SERVER_ROLES: &[ServerRole] = &[
    ServerRole {
        role: Role::Servers,
        prefix: DAEMON_SERVER_PREFIX,
        description: "Monitoring server",
        services: &[
            ("Grafana", "Grafana", Some(PortType::Http3000)),
            ("InfluxDB", "InfluxDB", Some(PortType::InfluxDb)),
            ("SSH", "SSH", Some(PortType::Ssh)),
        ],
    },
    ServerRole {
        role: Role::OtDmz,
        prefix: "hist",
        description: "Process historian",
        services: &[
            ("Microsoft SQL Server", "SQL Server", Some(PortType::MsSql)),
            ("Microsoft IIS", "Historian web", Some(PortType::Https)),
        ],
    },
    ServerRole {
        role: Role::Servers,
        prefix: "scada",
        description: "SCADA server",
        services: &[
            ("OPC UA", "OPC UA", Some(PortType::OpcUa)),
            ("Microsoft SQL Server", "SQL Server", Some(PortType::MsSql)),
        ],
    },
    ServerRole {
        role: Role::Servers,
        prefix: "ad",
        description: "Directory and DNS server",
        services: &[
            ("Active Directory", "Active Directory", Some(PortType::Ldap)),
            ("DNS Server", "DNS", Some(PortType::DnsUdp)),
        ],
    },
    ServerRole {
        role: Role::Servers,
        prefix: "ntp",
        description: "Time server",
        services: &[
            ("NTP Server", "NTP", Some(PortType::Ntp)),
            ("SSH", "SSH", Some(PortType::Ssh)),
        ],
    },
    ServerRole {
        role: Role::Servers,
        prefix: "log",
        description: "Syslog collector",
        services: &[
            ("Graylog", "Graylog", None),
            ("SSH", "SSH", Some(PortType::Ssh)),
        ],
    },
    ServerRole {
        role: Role::Servers,
        prefix: "db",
        description: "Database server",
        services: &[
            ("PostgreSQL", "PostgreSQL", Some(PortType::PostgreSQL)),
            ("SSH", "SSH", Some(PortType::Ssh)),
        ],
    },
    ServerRole {
        role: Role::OtDmz,
        prefix: "patch",
        description: "Patch and antivirus relay",
        services: &[("Microsoft IIS", "Patch relay", Some(PortType::Http))],
    },
    ServerRole {
        role: Role::Servers,
        prefix: "web",
        description: "Intranet web server",
        services: &[("Web Service", "Intranet", Some(PortType::Https))],
    },
];

/// What goes on an access port.
#[derive(Clone, Copy)]
enum Attached {
    Server { role: usize },
    Endpoint { kind: usize, vendor: usize },
}

impl Attached {
    fn role(self) -> Role {
        match self {
            Attached::Server { role } => SERVER_ROLES[role].role,
            Attached::Endpoint { kind, .. } => KINDS[kind].role,
        }
    }
}

struct Slot {
    attached: Attached,
    switch: usize,
    port: i32,
}

/// Every server and endpoint with the access switch and port it is cabled to.
pub(super) struct AttachmentPlan {
    slots: Vec<Slot>,
    edge_ports: Vec<i32>,
}

impl AttachmentPlan {
    /// How many hosts land on `role`'s VLAN.
    pub fn demand(&self, role: Role) -> usize {
        self.slots
            .iter()
            .filter(|s| s.attached.role() == role)
            .count()
    }

    /// Each access switch's edge port count: 48, or more when more hosts are cabled to it.
    pub fn edge_ports(&self) -> &[i32] {
        &self.edge_ports
    }
}

pub(super) fn plan_attachments(params: &EstateParams, rng: &mut fastrand::Rng) -> AttachmentPlan {
    assert!(params.access > 0, "an estate needs an access switch");
    let mut cabled = vec![0i32; params.access];
    let mut slots = Vec::with_capacity(params.servers + params.endpoints);
    let mut cable = |attached: Attached, switch: usize, slots: &mut Vec<Slot>| {
        cabled[switch] += 1;
        slots.push(Slot {
            attached,
            switch,
            port: cabled[switch],
        });
    };

    for i in 0..params.servers {
        let switch = (i / SERVERS_PER_SWITCH).min(params.access - 1);
        cable(
            Attached::Server {
                role: i % SERVER_ROLES.len(),
            },
            switch,
            &mut slots,
        );
    }

    // Each kind's share of the endpoints, with the rounding remainder going to workstations.
    let total_weight: usize = KINDS.iter().map(|k| k.weight).sum();
    let mut counts: Vec<usize> = KINDS
        .iter()
        .map(|k| params.endpoints * k.weight / total_weight)
        .collect();
    counts[0] += params.endpoints - counts.iter().sum::<usize>();
    let mut kinds: Vec<usize> = counts
        .iter()
        .enumerate()
        .flat_map(|(kind, &n)| std::iter::repeat_n(kind, n))
        .collect();
    rng.shuffle(&mut kinds);
    for (i, kind) in kinds.into_iter().enumerate() {
        let vendor = rng.usize(0..KINDS[kind].vendors.len());
        cable(
            Attached::Endpoint { kind, vendor },
            i % params.access,
            &mut slots,
        );
    }

    let edge_ports = cabled
        .iter()
        .map(|&n| ((n + 7) / 8 * 8).max(MIN_EDGE_PORTS))
        .collect();
    AttachmentPlan { slots, edge_ports }
}

/// Where an attached host is cabled, for the access port's FDB entry.
pub(super) struct Placement {
    pub switch_host_id: Uuid,
    pub port: i32,
    pub mac: MacAddress,
    pub role: Role,
    pub speed_bps: i64,
    pub name: String,
}

pub(super) struct AttachedHosts {
    pub hosts: Vec<HostWithServices>,
    pub placements: Vec<Placement>,
    pub daemon_host_id: Uuid,
}

/// Zero-padded per-prefix names, sized to the number of hosts sharing the prefix.
fn host_names(plan: &AttachmentPlan) -> Vec<String> {
    let prefix_of = |a: Attached| match a {
        Attached::Server { role } => SERVER_ROLES[role].prefix,
        Attached::Endpoint { kind, .. } => KINDS[kind].prefix,
    };
    let mut totals: HashMap<&str, usize> = HashMap::new();
    for slot in &plan.slots {
        *totals.entry(prefix_of(slot.attached)).or_default() += 1;
    }
    let mut seen: HashMap<&str, usize> = HashMap::new();
    plan.slots
        .iter()
        .map(|slot| {
            let prefix = prefix_of(slot.attached);
            let n = seen.entry(prefix).or_default();
            let name = switches::numbered(prefix, *n, totals[prefix]);
            *n += 1;
            name
        })
        .collect()
}

pub(super) fn build_attached_hosts(
    plan: &AttachmentPlan,
    site: &Site,
    addressing: &mut Addressing,
    services: &mut ServiceCache,
    access: &[switches::AccessSwitch],
    now: DateTime<Utc>,
) -> AttachedHosts {
    let names = host_names(plan);
    let mut hosts = Vec::with_capacity(plan.slots.len());
    let mut placements = Vec::with_capacity(plan.slots.len());
    let mut daemon_host_id = None;

    for (i, (slot, name)) in plan.slots.iter().zip(names).enumerate() {
        let role = slot.attached.role();
        let ip = addressing.segment_mut(role).allocate();
        let subnet = &addressing.segment(role).subnet;
        let serial = [(i >> 16) as u8, (i >> 8) as u8, i as u8];

        let (mac, hws, speed_bps) = match slot.attached {
            Attached::Server { role: r } => {
                let server = &SERVER_ROLES[r];
                let mac = [0x02, 0x5d, 0x00, serial[0], serial[1], serial[2]];
                let hws = services.host_with_services(
                    with_mac(
                        create_host(
                            &name,
                            Some(&format!("{name}.plant.local")),
                            Some(server.description),
                            site,
                            subnet,
                            ip,
                            vec![],
                            None,
                            None,
                            now,
                        ),
                        mac,
                    ),
                    server.services,
                    now,
                );
                if server.prefix == DAEMON_SERVER_PREFIX && daemon_host_id.is_none() {
                    daemon_host_id = Some(hws.host.id);
                }
                (mac, hws, 10 * GIGABIT)
            }
            Attached::Endpoint { kind: k, vendor: v } => {
                let kind = &KINDS[k];
                let vendor = &kind.vendors[v];
                let oui = vendor.oui.unwrap_or([0x02, 0x5e, 0x00]);
                let mac = [oui[0], oui[1], oui[2], serial[0], serial[1], serial[2]];
                let hostname = format!("{name}.plant.local");
                let mut host = with_mac(
                    create_host(
                        &name,
                        kind.in_dns.then_some(hostname.as_str()),
                        Some(kind.description),
                        site,
                        subnet,
                        ip,
                        vec![],
                        None,
                        None,
                        now,
                    ),
                    mac,
                );
                if let Some(maker) = vendor.name {
                    host = with_fingerprint(host, vendor.source, maker, vendor.device_type);
                }
                let hws = services.host_with_services(host, vendor.services, now);
                (mac, hws, kind.speed_bps)
            }
        };

        placements.push(Placement {
            switch_host_id: access[slot.switch].host_id,
            port: slot.port,
            mac: MacAddress::new(mac),
            role,
            speed_bps,
            name: hws.host.base.name.to_string(),
        });
        hosts.push(hws);
    }

    AttachedHosts {
        hosts,
        placements,
        daemon_host_id: daemon_host_id.expect("an estate has at least one monitoring server"),
    }
}
