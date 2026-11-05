//! The inventory script's output → what each domain and the host report.
//!
//! Pure: no I/O, so every rule here runs against output recorded from the lab host in the tests
//! below. `virsh` prints tables meant for people; each parser reads only the columns it needs and
//! skips headers and rule lines, so it reads `virsh -q` and plain `virsh` alike.

use std::collections::BTreeMap;
use std::net::IpAddr;

use crate::daemon::discovery::integration::hypervisor::guest::{ConfigNic, GuestAddress};
use crate::daemon::discovery::integration::hypervisor::qemu_agent::OsInfo;
use crate::daemon::discovery::integration::hypervisor::reach::HypervisorBridge;
use crate::server::lldp::canonical_mac;
use crate::server::shared::attribution::AttributeSource;

/// Everything one inventory run printed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Inventory {
    pub hostname: Option<String>,
    /// `ip -j addr`, as printed.
    pub ip_addr: String,
    /// Each bridge's ports, from `/sys/class/net/<bridge>/brif`.
    pub bridge_ports: BTreeMap<String, Vec<String>>,
    /// Each libvirt network's bridge.
    pub network_bridges: BTreeMap<String, String>,
    pub domains: Vec<Domain>,
    /// `virsh list` answered, so `domains` is every domain the connection defines.
    pub listed: bool,
    /// The script reached its end, so every domain's section is complete.
    pub complete: bool,
}

/// One domain's section.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Domain {
    pub uuid: String,
    pub name: Option<String>,
    pub state: String,
    pub iflist: String,
    pub lease: String,
    pub arp: String,
    /// `None` when the agent was not asked (a read-only connection).
    pub agent: Option<String>,
    pub guestinfo: Option<String>,
}

/// Split the script's output on its `@@` markers.
pub fn inventory(output: &str) -> Inventory {
    let mut inventory = Inventory::default();
    // Where the lines under the current marker go.
    enum Into {
        Nothing,
        Hostname,
        IpAddr,
        Bridge(String),
        Network(String),
        Name,
        State,
        IfList,
        Lease,
        Arp,
        Agent,
        GuestInfo,
    }
    let mut into = Into::Nothing;
    for line in output.lines() {
        if let Some(marker) = line.strip_prefix("@@") {
            let (tag, arg) = marker.split_once(' ').unwrap_or((marker, ""));
            into = match (tag, arg) {
                ("hostname", _) => Into::Hostname,
                ("ip-addr", _) => Into::IpAddr,
                ("brif", bridge) => {
                    inventory
                        .bridge_ports
                        .entry(bridge.to_string())
                        .or_default();
                    Into::Bridge(bridge.to_string())
                }
                ("net", network) => Into::Network(network.to_string()),
                ("domains", _) => {
                    inventory.listed = true;
                    Into::Nothing
                }
                ("domain", uuid) => {
                    inventory.domains.push(Domain {
                        uuid: uuid.trim().to_string(),
                        ..Default::default()
                    });
                    Into::Nothing
                }
                ("name", _) => Into::Name,
                ("state", _) => Into::State,
                ("iflist", _) => Into::IfList,
                ("addr", "lease") => Into::Lease,
                ("addr", "arp") => Into::Arp,
                ("addr", "agent") => {
                    if let Some(d) = inventory.domains.last_mut() {
                        d.agent = Some(String::new());
                    }
                    Into::Agent
                }
                ("guestinfo", _) => {
                    if let Some(d) = inventory.domains.last_mut() {
                        d.guestinfo = Some(String::new());
                    }
                    Into::GuestInfo
                }
                ("end", _) => {
                    inventory.complete = true;
                    Into::Nothing
                }
                _ => Into::Nothing,
            };
            continue;
        }
        let push = |text: &mut String| {
            text.push_str(line);
            text.push('\n');
        };
        let domain = inventory.domains.last_mut();
        match (&into, domain) {
            (Into::Hostname, _) => {
                if inventory.hostname.is_none() && !line.trim().is_empty() {
                    inventory.hostname = Some(line.trim().to_string());
                }
            }
            (Into::IpAddr, _) => push(&mut inventory.ip_addr),
            (Into::Bridge(bridge), _) => {
                if !line.trim().is_empty()
                    && let Some(ports) = inventory.bridge_ports.get_mut(bridge)
                {
                    ports.push(line.trim().to_string());
                }
            }
            (Into::Network(network), _) => {
                if let Some(bridge) = info_value(line, "Bridge") {
                    inventory
                        .network_bridges
                        .insert(network.clone(), bridge.to_string());
                }
            }
            (Into::Name, Some(d)) => {
                if d.name.is_none() && !line.trim().is_empty() {
                    d.name = Some(line.trim().to_string());
                }
            }
            (Into::State, Some(d)) => {
                if d.state.is_empty() {
                    d.state = line.trim().to_string();
                }
            }
            (Into::IfList, Some(d)) => push(&mut d.iflist),
            (Into::Lease, Some(d)) => push(&mut d.lease),
            (Into::Arp, Some(d)) => push(&mut d.arp),
            (Into::Agent, Some(d)) => {
                if let Some(agent) = d.agent.as_mut() {
                    push(agent);
                }
            }
            (Into::GuestInfo, Some(d)) => {
                if let Some(info) = d.guestinfo.as_mut() {
                    push(info);
                }
            }
            _ => {}
        }
    }
    inventory
}

/// `Key:    value` from `virsh net-info` or `virsh guestinfo`.
fn info_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let (k, v) = line.split_once(':')?;
    (k.trim() == key)
        .then(|| v.trim())
        .filter(|v| !v.is_empty())
}

/// Whether a table line is a header or a rule rather than a row.
fn is_table_chrome(line: &str, first_header: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty()
        || trimmed.starts_with('-') && trimmed.chars().all(|c| c == '-')
        || trimmed.split_whitespace().next() == Some(first_header)
}

/// The NICs `virsh domiflist --inactive` lists, their bridge resolved through `network_bridges`
/// for one on a libvirt network.
///
/// Columns: Interface (the target device, `-` while stopped), Type, Source, Model, MAC.
pub fn nics(iflist: &str, network_bridges: &BTreeMap<String, String>) -> Vec<ConfigNic> {
    iflist
        .lines()
        .filter(|l| !is_table_chrome(l, "Interface"))
        .enumerate()
        .filter_map(|(index, line)| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            let [target, link_type, source, _model, mac] = columns.as_slice() else {
                return None;
            };
            let bridge = match *link_type {
                "bridge" => Some(source.to_string()),
                "network" => network_bridges.get(*source).cloned(),
                _ => None,
            };
            Some(ConfigNic {
                // The target device names a running domain's NIC; a stopped one has none.
                key: if *target == "-" {
                    format!("net{index}")
                } else {
                    target.to_string()
                },
                name: None,
                mac: canonical_mac(mac),
                bridge,
                static_ips: vec![],
            })
        })
        .collect()
}

/// The addresses `virsh domifaddr` lists, filed under `source`.
///
/// Columns: Name, MAC address, Protocol, Address (CIDR). A NIC's second address repeats neither
/// name nor MAC (`-`), so both carry over from the row above.
pub fn addresses(domifaddr: &str, source: AttributeSource) -> Vec<GuestAddress> {
    let mut addresses = vec![];
    let (mut name, mut mac) = (None::<String>, None::<String>);
    for line in domifaddr.lines().filter(|l| !is_table_chrome(l, "Name")) {
        let columns: Vec<&str> = line.split_whitespace().collect();
        let [row_name, row_mac, _protocol, address] = columns.as_slice() else {
            continue;
        };
        if *row_name != "-" {
            name = Some(row_name.to_string());
            mac = canonical_mac(row_mac);
        }
        let Some(ip) = address
            .split('/')
            .next()
            .and_then(|a| a.parse::<IpAddr>().ok())
        else {
            continue;
        };
        if crate::daemon::discovery::integration::hypervisor::guest::reachable(ip) {
            addresses.push(GuestAddress {
                ip,
                mac: mac.clone(),
                interface: name.clone(),
                source,
            });
        }
    }
    addresses
}

/// What `virsh guestinfo --os --hostname` prints: `os.<field> : value` lines and `hostname`.
pub fn guest_info(guestinfo: &str) -> (OsInfo, Option<String>) {
    let field = |key: &str| {
        guestinfo
            .lines()
            .find_map(|l| info_value(l, key))
            .map(str::to_string)
    };
    (
        OsInfo {
            id: field("os.id"),
            name: field("os.name"),
            version: field("os.version"),
            version_id: field("os.version-id"),
            kernel_release: field("os.kernel-release"),
        },
        field("hostname"),
    )
}

/// The host's bridges: uplinked when a port is a NIC rather than a guest's tap or a container's
/// veth, addressed when `ip -j addr` gives the bridge a reachable address.
pub fn bridges(inventory: &Inventory) -> Vec<HypervisorBridge> {
    let addressed = interface_addresses(&inventory.ip_addr);
    let is_guest_port = |port: &str| {
        ["vnet", "tap", "veth", "vif", "macvtap"]
            .iter()
            .any(|prefix| port.starts_with(prefix))
    };
    inventory
        .bridge_ports
        .iter()
        .map(|(bridge, ports)| HypervisorBridge {
            iface: bridge.clone(),
            uplinked: ports.iter().any(|p| !is_guest_port(p)),
            addressed: addressed.iter().any(|(iface, _)| iface == bridge),
        })
        .collect()
}

/// Every reachable address `ip -j addr` gives an interface, as `(interface, address)`.
pub fn interface_addresses(ip_addr: &str) -> Vec<(String, IpAddr)> {
    #[derive(serde::Deserialize)]
    struct Link {
        ifname: String,
        #[serde(default)]
        addr_info: Vec<Addr>,
    }
    #[derive(serde::Deserialize)]
    struct Addr {
        #[serde(default)]
        local: Option<String>,
    }
    let links: Vec<Link> = serde_json::from_str(ip_addr.trim()).unwrap_or_default();
    links
        .into_iter()
        .flat_map(|link| {
            link.addr_info
                .into_iter()
                .filter_map(|a| a.local?.parse::<IpAddr>().ok())
                .filter(|ip| {
                    crate::daemon::discovery::integration::hypervisor::guest::reachable(*ip)
                })
                .map(move |ip| (link.ifname.clone(), ip))
        })
        .collect()
}
