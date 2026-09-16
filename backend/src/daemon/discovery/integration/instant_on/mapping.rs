//! Translate Instant On wire shapes ([`super::types`]) into Scanopy entities.
//!
//! Pure and synchronous — no I/O — so every rule here is unit-testable against fixtures.
//!
//! The guiding constraint is the same one the UniFi mapping works under: **produce exactly the
//! `InterfaceBase` columns an SNMP poll would**. Instant On is an alternative *source* of
//! neighbor and FDB data, not a parallel topology system, so the server's existing neighbor
//! resolution (`hosts/service/topology.rs`) consumes it unchanged. MACs therefore go through
//! [`canonical_mac`], which produces the byte-identical lowercase-colon form `from_snmp` writes.
//!
//! Two rules here are specific to Instant On and load-bearing:
//!
//! 1. **Only inventory devices become hosts.** A site's inventory is exactly its adopted Instant
//!    On hardware, so labelling those as Instant On devices is true of every one. Clients are
//!    laptops and printers that happen to be plugged in; they become FDB entries on a port, never
//!    hosts. That also sidesteps their frequently-absent IP addresses, and host identity is
//!    IP-based.
//! 2. **Stacked and standalone switches take one code path.** A stack is one inventory entry with
//!    one management IP, so it is one host. What differs is that its ports are member-qualified
//!    (`"1/1/1"`), which is handled by deriving interface identity from the port's own reported id
//!    rather than assuming a flat index — see [`port_if_index`].

use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::AttributeSource;
use crate::server::subnets::r#impl::inference::placeable_subnet;
use std::collections::HashMap;
use std::net::IpAddr;

use uuid::Uuid;

use crate::daemon::discovery::integration::controller::{ControllerIdentity, MappedClient};
use crate::server::interface_neighbors::r#impl::base::InterfaceNeighborEvidence;
use crate::server::interfaces::r#impl::base::{
    IfAdminStatus, IfOperStatus, Interface, InterfaceBase,
};
use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
use crate::server::lldp::{LldpChassisId, LldpPortId, canonical_mac};
use crate::server::subnets::r#impl::base::Subnet;

use super::types::{InstantOnClient, InstantOnDevice, InstantOnPort};

/// IANA ifType 6 — ethernetCsmacd. Every Instant On port we model is a physical Ethernet port.
const IF_TYPE_ETHERNET: i32 = 6;

/// An Instant On device translated into the entities Scanopy stores.
pub struct MappedDevice {
    /// What the portal knows this device by. The caller turns it into a host, or folds it into
    /// the host being scanned — either way the name reaches the ladder the same way.
    pub identity: ControllerIdentity,
    pub ip_address: IPAddress,
    pub interfaces: Vec<Interface>,
    /// The portal's device-class string (`"SWITCH"`, `"STACK"`, …), fed to the service matcher as
    /// `ManagedDevice` evidence.
    pub device_type: Option<String>,
    /// Management IP, kept separately because the caller matches it against the scanned IP to
    /// decide which device is the anchor host.
    pub ip: IpAddr,
}

/// Translate an `inventory` payload, attaching wired clients to the ports they sit on.
///
/// Devices whose IP is missing, unparseable, or outside every known subnet are **skipped**: host
/// deduplication is exclusively IP-based (`hosts/service/mod.rs::select_matching_host`), so a host
/// created without a resolvable IP would mint a fresh duplicate on every scan rather than updating
/// the existing one. The caller reports the count so this never reads as "the site is empty".
pub fn map_devices(
    devices: &[InstantOnDevice],
    clients: &[InstantOnClient],
    network_id: Uuid,
    subnets: &[Subnet],
) -> Vec<MappedDevice> {
    // Indexed by the portal's device id, because that is what an uplink names. Resolving it gives
    // the parent's MAC and the parent-side port, without which an uplink yields only a host-level
    // adjacency instead of a `PhysicalLink`.
    let by_id: HashMap<&str, &InstantOnDevice> = devices
        .iter()
        .filter_map(|d| Some((d.id.as_deref()?, d)))
        .collect();

    // Wired clients grouped by the switch they are attached to, so each device's mapping does a
    // single lookup rather than a scan of every client in the site.
    let mut clients_by_device: HashMap<&str, Vec<&InstantOnClient>> = HashMap::new();
    for client in clients.iter().filter(|c| c.is_wired()) {
        if let Some(device_id) = client.connected_to.as_deref() {
            clients_by_device.entry(device_id).or_default().push(client);
        }
    }

    devices
        .iter()
        .filter_map(|device| {
            let attached = device
                .id
                .as_deref()
                .and_then(|id| clients_by_device.get(id))
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            map_device(device, attached, network_id, subnets, &by_id)
        })
        .collect()
}

fn map_device(
    device: &InstantOnDevice,
    clients: &[&InstantOnClient],
    network_id: Uuid,
    subnets: &[Subnet],
    by_id: &HashMap<&str, &InstantOnDevice>,
) -> Option<MappedDevice> {
    let ip: IpAddr = device.ip_address.as_deref()?.trim().parse().ok()?;
    // Not first-match: the list carries the `0.0.0.0/0` organizational rows, which contain every
    // IPv4 address, so `find` returned `Internet` for everything and nothing was ever skipped.
    let subnet = placeable_subnet(subnets, ip)?;
    let device_mac = device.mac_address.as_deref().and_then(canonical_mac);

    let identity = ControllerIdentity {
        probe: ClientProbe::InstantOn,
        name: device.name.clone(),
        // Adopted infrastructure has no separate reported hostname; the portal's name is the
        // only one there is.
        hostname: None,
        // Same canonical form the SNMP daemon writes, so `find_host_by_chassis_id` can reach this
        // device from a neighbor's advertised chassis ID.
        chassis_id: device_mac.clone(),
        manufacturer: Some("HPE".to_string()),
        model: device.model.clone(),
        serial_number: device.serial_number.clone(),
        firmware_revision: device.firmware_version.clone(),
    };

    let ip_address = IPAddress::new(IPAddressBase {
        network_id,
        host_id: Uuid::nil(), // server assigns
        subnet_id: subnet.id,
        ip_address: ip,
        // The controller reporting a device it manages, not the device answering us.
        mac_address: device_mac.as_deref().and_then(|m| m.parse().ok()).map(|m| {
            MacEvidence::new(
                MacEvidenceValue(m),
                AttributeSource::Probe(ClientProbe::InstantOn),
            )
        }),
        name: None,
        position: 0,
    });

    Some(MappedDevice {
        identity,
        ip_address,
        interfaces: map_interfaces(device, clients, network_id, by_id),
        device_type: device.device_type.clone().filter(|t| !t.trim().is_empty()),
        ip,
    })
}

/// Build the device's interfaces, then decorate them with neighbor and FDB data.
fn map_interfaces(
    device: &InstantOnDevice,
    clients: &[&InstantOnClient],
    network_id: Uuid,
    by_id: &HashMap<&str, &InstantOnDevice>,
) -> Vec<Interface> {
    let mut interfaces: Vec<Interface> = device
        .ports
        .iter()
        .enumerate()
        .map(|(position, port)| port_to_interface(port, position, network_id))
        .collect();

    // A portless device — an access point, in practice — still has one real interface: its
    // uplink. Model it so the *switch* side of the link can resolve to a specific interface
    // rather than degrading to a host-only neighbor.
    if interfaces.is_empty()
        && let Some(uplink) = &device.uplink
    {
        let name = uplink
            .port_id
            .clone()
            .filter(|p| !p.trim().is_empty())
            .unwrap_or_else(|| "uplink".to_string());
        interfaces.push(Interface::new(InterfaceBase {
            host_id: Uuid::nil(),
            network_id,
            if_index: Some(1),
            if_descr: Some(name.clone()),
            if_name: Some(name),
            if_type: Some(IF_TYPE_ETHERNET),
            admin_status: Some(IfAdminStatus::Up),
            oper_status: Some(IfOperStatus::Up),
            ..Default::default()
        }));
    }

    apply_uplink(&mut interfaces, device, by_id);
    apply_client_macs(&mut interfaces, device, clients);
    interfaces
}

/// Stable, unique `if_index` for a port, derived from the identifier the portal gave it.
///
/// This is the one place where a flat port index would be actively wrong. A stack numbers ports
/// per member — `"1/1/1"` and `"2/1/1"` are different physical ports on different member switches
/// — so keying on `port_number` alone would collapse every member's port 1 onto one interface,
/// silently losing most of a stacked switch's topology. Folding all numeric components of the id
/// keeps them distinct, and a standalone switch's plain `"24"` still maps to 24.
///
/// Falls back to the port number and finally to array position, so a port id in a shape we have
/// not seen costs correct numbering rather than the whole port.
fn port_if_index(port: &InstantOnPort, position: usize) -> i32 {
    let folded = port.id.as_deref().and_then(|id| {
        let components: Vec<i64> = id
            .split(['/', '-', ':'])
            .filter_map(|c| c.trim().parse::<i64>().ok())
            .collect();
        if components.is_empty() {
            return None;
        }
        // Saturating: three components of sane size stay far inside i32, and anything absurd
        // clamps rather than wrapping into a collision with a real port.
        Some(
            components
                .iter()
                .fold(0i64, |acc, n| acc.saturating_mul(1_000).saturating_add(*n))
                .clamp(0, i32::MAX as i64) as i32,
        )
    });

    folded
        .or_else(|| port.port_number.map(|n| n.as_i32()))
        .unwrap_or((position as i32).saturating_add(1))
}

fn port_to_interface(port: &InstantOnPort, position: usize, network_id: Uuid) -> Interface {
    let if_index = port_if_index(port, position);
    // Prefer the portal's own label, then its port id — which on a stack is the member-qualified
    // form and therefore the most informative name available. `if_descr` is validated non-empty,
    // so an unnamed port needs a synthesized label.
    let name = port
        .name
        .clone()
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
        .or_else(|| {
            port.id
                .clone()
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty())
        })
        .unwrap_or_else(|| format!("Port {if_index}"));

    Interface::new(InterfaceBase {
        host_id: Uuid::nil(),
        network_id,
        if_index: Some(if_index),
        if_descr: Some(name.clone()),
        if_name: Some(name),
        if_type: Some(IF_TYPE_ETHERNET),
        speed_bps: port
            .speed
            .map(|s| s.as_i64() * 1_000_000)
            .filter(|s| *s > 0),
        admin_status: Some(match port.enabled.map(|e| e.as_bool()) {
            Some(false) => IfAdminStatus::Down,
            _ => IfAdminStatus::Up,
        }),
        oper_status: Some(match port.up.map(|u| u.as_bool()) {
            Some(true) => IfOperStatus::Up,
            _ => IfOperStatus::Down,
        }),
        // The portal's per-port `vlanId` is deliberately not mapped. `InterfaceBase`'s VLAN
        // columns hold VLAN *entity* ids, not raw tags, so recording one would mean resolving it
        // to a VLAN entity — and an access VLAN is not the membership list SNMP walks anyway.
        // `interface_data_complete().vlan_membership` is `false` to match.
        ..Default::default()
    })
}

/// Record the parent device on the port that faces it.
///
/// Instant On derives these uplinks itself rather than from LLDP, so they are authoritative for
/// the site even where LLDP is not running — which is the whole reason this integration can build
/// L2 topology for switches that expose no LLDP-MIB.
fn apply_uplink(
    interfaces: &mut [Interface],
    device: &InstantOnDevice,
    by_id: &HashMap<&str, &InstantOnDevice>,
) {
    let Some(uplink) = &device.uplink else { return };
    let Some(local_port_id) = uplink.port_id.as_deref().map(str::trim) else {
        return;
    };
    let Some(local_index) = index_of_port(device, local_port_id) else {
        return;
    };
    let Some(interface) = interfaces
        .iter_mut()
        .find(|i| i.base.if_index == Some(local_index))
    else {
        return;
    };

    let parent = uplink.connected_to.as_deref().and_then(|id| by_id.get(id));

    // The parent's MAC comes from the uplink entry when present, otherwise from the parent's own
    // inventory row — the same device, reported twice, and either spelling is canonicalised the
    // same way.
    let Some(parent_mac) = uplink
        .connected_to_mac_address
        .as_deref()
        .and_then(canonical_mac)
        .or_else(|| {
            parent
                .and_then(|p| p.mac_address.as_deref())
                .and_then(canonical_mac)
        })
    else {
        return;
    };

    // One candidate, replacing whatever this port already held — this function is authoritative
    // for the port it names (mirrors the pre-GH #701 scalar-overwrite semantics `InterfaceBase.
    // lldp_*` had).
    interface.base.neighbor_candidates = vec![InterfaceNeighborEvidence {
        lldp_chassis_id: Some(LldpChassisId::MacAddress(parent_mac)),
        // `LocallyAssigned` holding the parent's *if_index*, not its port id string: the resolver
        // tries a name lookup and then parses the value as an ifIndex, and the parent's
        // interfaces are numbered by the same `port_if_index`, so the index tier hits. Passing
        // the raw `"1/1/1"` would dead-end in both tiers.
        lldp_port_id: uplink
            .remote_port_id
            .as_deref()
            .map(str::trim)
            .zip(parent)
            .and_then(|(remote, parent)| index_of_port(parent, remote))
            .map(|idx| LldpPortId::LocallyAssigned(idx.to_string())),
        lldp_sys_name: parent
            .and_then(|p| p.name.clone())
            .filter(|n| !n.trim().is_empty()),
        ..Default::default()
    }];
}

/// The `if_index` this device's port with `port_id` was mapped to.
fn index_of_port(device: &InstantOnDevice, port_id: &str) -> Option<i32> {
    device
        .ports
        .iter()
        .enumerate()
        .find(|(_, p)| p.id.as_deref().map(str::trim) == Some(port_id))
        .map(|(position, port)| port_if_index(port, position))
}

/// Attach each wired client's MAC to the port it sits on, as a bridge-FDB entry.
///
/// This is the same shape UniFi's `mac_table` produces, and it is why clients do not become hosts:
/// the server's existing neighbor resolution already turns FDB entries into edges against hosts
/// that other discovery methods found, with their real identities intact.
fn apply_client_macs(
    interfaces: &mut [Interface],
    device: &InstantOnDevice,
    clients: &[&InstantOnClient],
) {
    let device_mac = device.mac_address.as_deref().and_then(canonical_mac);

    for client in clients {
        let Some(mac) = client.mac_address.as_deref().and_then(canonical_mac) else {
            continue;
        };
        // The switch's own MAC on its own port carries no adjacency information.
        if device_mac.as_deref() == Some(mac.as_str()) {
            continue;
        }
        let Some(port_id) = client.port_id.as_deref().map(str::trim) else {
            continue;
        };
        let Some(index) = index_of_port(device, port_id) else {
            continue;
        };
        let Some(interface) = interfaces
            .iter_mut()
            .find(|i| i.base.if_index == Some(index))
        else {
            continue;
        };

        // `None` rather than an empty vec: the server's unresolved-FDB filter keys on the column
        // being non-NULL.
        let fdb = interface.base.fdb_macs.get_or_insert_with(Vec::new);
        if !fdb.contains(&mac) {
            fdb.push(mac);
        }
    }
}

/// Translate a `clientSummary` payload into client hosts.
///
/// Wired clients keep contributing their MAC to the port's FDB entry as before — that is what
/// resolves them into topology edges. This is the other half: the portal is often the only place
/// a client's name exists at all, and without a host to hang it on it was parsed and discarded.
///
/// `device_ips` are the addresses of the site's adopted devices, which also appear in the client
/// list; excluding them keeps a client record's hostname from competing with the administrator's
/// name for the same device.
pub fn map_clients(
    clients: &[InstantOnClient],
    network_id: Uuid,
    device_ips: &[IpAddr],
) -> Vec<MappedClient> {
    clients
        .iter()
        .filter_map(|client| {
            let identity = ControllerIdentity {
                probe: ClientProbe::InstantOn,
                name: client.name.clone(),
                // The portal reports one name per client and does not distinguish an assigned
                // alias from a DHCP hostname.
                hostname: None,
                chassis_id: None,
                manufacturer: None,
                model: None,
                serial_number: None,
                firmware_revision: None,
            };
            MappedClient::new(
                identity,
                client.ip_address.as_deref(),
                client.mac_address.as_deref(),
                network_id,
            )
        })
        .filter(|client| !device_ips.contains(&client.ip))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::instant_on::types::InstantOnEnvelope;
    use crate::server::interfaces::r#impl::base::{IfAdminStatus, IfOperStatus};
    use crate::server::shared::types::entities::EntitySource;
    use crate::server::subnets::r#impl::base::SubnetBase;
    use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
    use crate::server::subnets::r#impl::types::SubnetType;

    // ------------------------------------------------------------------------------------
    // FIXTURE PROVENANCE
    //
    // HPE publishes no API for Instant On. These payloads are **hand-authored from the field
    // names in the portal's own web client — they were NOT captured from real hardware.** The
    // resource names and `device-type-enum` values are read from that client; the nesting is
    // our best reading of it.
    //
    // They are therefore adequate to pin our *mapping rules* and worthless as evidence that we
    // parse a real site correctly. When a real capture arrives, drop it in beside these and
    // re-run: shape errors surface as fields mapping to None.
    // ------------------------------------------------------------------------------------

    const INVENTORY: &str = include_str!("../../../../tests/instant_on/inventory.json");
    const CLIENTS: &str = include_str!("../../../../tests/instant_on/client_summary.json");

    fn parse<T: serde::de::DeserializeOwned>(json: &str) -> Vec<T> {
        let envelope: InstantOnEnvelope<T> =
            serde_json::from_str(json).expect("fixture should parse");
        envelope.elements
    }

    /// The 192.168.20.0/24 the fixtures live on, **plus the two `0.0.0.0/0` rows every network is
    /// seeded with** — because that is what the daemon actually receives, and a list without them
    /// is what let first-match placement file every device under `Internet` while looking correct
    /// in every test. `10.99.99.99` is deliberately outside the real subnet.
    fn test_subnets(network_id: Uuid) -> Vec<Subnet> {
        let subnet = |name: &str, cidr: &str, subnet_type| Subnet {
            base: SubnetBase {
                name: name.to_string(),
                network_id,
                cidr: SubnetCidr::new(
                    SubnetCidrValue(cidr.parse().expect("valid CIDR")),
                    AttributeSource::DaemonSelfReport,
                ),
                subnet_type,
                source: EntitySource::Discovery,
                ..Default::default()
            },
            ..Default::default()
        };

        // Seeded first, so they lead the `created_at ASC` order the daemon receives.
        vec![
            subnet("Internet", "0.0.0.0/0", SubnetType::Internet),
            subnet("Remote Network", "0.0.0.0/0", SubnetType::Remote),
            subnet("Test", "192.168.20.0/24", SubnetType::Lan),
        ]
    }

    /// A device on the real subnet is placed there, and one outside every range this network holds
    /// is skipped — not filed under a catch-all that technically contains it.
    #[test]
    fn devices_are_placed_on_real_subnets_and_never_on_a_catch_all() {
        let network_id = Uuid::new_v4();
        let subnets = test_subnets(network_id);
        let real = subnets
            .iter()
            .find(|s| s.base.cidr.to_string() == "192.168.20.0/24")
            .expect("the real subnet")
            .id;

        let mapped = map_devices(
            &parse::<InstantOnDevice>(INVENTORY),
            &parse::<InstantOnClient>(CLIENTS),
            network_id,
            &subnets,
        );

        assert!(
            !mapped.is_empty(),
            "the fixture must map at least one device"
        );
        for device in &mapped {
            assert_eq!(
                device.ip_address.base.subnet_id, real,
                "{} was filed somewhere other than the subnet that holds it",
                device.ip
            );
        }
        assert!(
            !mapped.iter().any(|d| d.ip.to_string() == "10.99.99.99"),
            "a device outside every held range must be skipped, not placed on a catch-all"
        );
    }

    fn map() -> Vec<MappedDevice> {
        let network_id = Uuid::new_v4();
        map_devices(
            &parse::<InstantOnDevice>(INVENTORY),
            &parse::<InstantOnClient>(CLIENTS),
            network_id,
            &test_subnets(network_id),
        )
    }

    fn find<'a>(devices: &'a [MappedDevice], name: &str) -> &'a MappedDevice {
        devices
            .iter()
            .find(|d| d.identity.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("expected a device named {name}"))
    }

    fn interface(device: &MappedDevice, if_index: i32) -> &Interface {
        device
            .interfaces
            .iter()
            .find(|i| i.base.if_index == Some(if_index))
            .unwrap_or_else(|| panic!("expected an interface at index {if_index}"))
    }

    /// The single candidate `apply_uplink` writes for a port — it is authoritative for the port
    /// it names, so a mapped port never carries more than one.
    fn candidate(port: &Interface) -> Option<&InterfaceNeighborEvidence> {
        port.base.neighbor_candidates.first()
    }

    /// The rule that stacking depends on. A stack numbers ports per member, so `"1/1/1"` and
    /// `"2/1/1"` are different physical ports on different member switches. Keying on
    /// `portNumber` would collapse them onto one interface and silently lose half the stack's
    /// ports — this asserts all four survive as distinct interfaces.
    #[test]
    fn stack_member_ports_stay_distinct() {
        let devices = map();
        let stack = find(&devices, "Core Stack");

        assert_eq!(stack.interfaces.len(), 4);
        let indexes: std::collections::HashSet<Option<i32>> =
            stack.interfaces.iter().map(|i| i.base.if_index).collect();
        assert_eq!(indexes.len(), 4, "every stack port needs its own if_index");

        // Both members' port 1 are present and are not the same interface.
        assert_ne!(
            interface(stack, 1_001_001).base.if_index,
            interface(stack, 2_001_001).base.if_index
        );
        // The member-qualified id survives as the interface name, so an operator can find the
        // port on the physical switch.
        assert_eq!(
            interface(stack, 2_001_001).base.if_descr.as_deref(),
            Some("2/1/1")
        );
    }

    /// A standalone switch numbers ports flatly, and must keep doing so — the stack handling
    /// must not renumber ordinary switches.
    #[test]
    fn standalone_switch_ports_keep_their_own_numbering() {
        let devices = map();
        let edge = find(&devices, "Edge Switch");
        assert_eq!(interface(edge, 24).base.if_descr.as_deref(), Some("Uplink"));
        // An unnamed port falls back to its port id rather than a synthesized label.
        assert_eq!(interface(edge, 2).base.if_descr.as_deref(), Some("2"));
    }

    /// An access point reports no ports at all. It must map cleanly to one synthesized uplink
    /// interface rather than erroring or vanishing — the same guarantee that keeps unseen models
    /// from failing the collection.
    #[test]
    fn port_less_access_point_maps_to_its_uplink() {
        let devices = map();
        let ap = find(&devices, "Office AP");
        assert_eq!(ap.interfaces.len(), 1);
        assert_eq!(ap.interfaces[0].base.if_descr.as_deref(), Some("eth0"));
        assert_eq!(ap.device_type.as_deref(), Some("ACCESS_POINT"));
    }

    /// The uplink is what makes L2 topology possible without LLDP. The parent's port must be
    /// recorded as the parent's *if_index*, not its raw id, or the server's resolver dead-ends
    /// and the link degrades to a host-level adjacency.
    #[test]
    fn uplink_points_at_the_parents_interface() {
        let devices = map();
        let edge = find(&devices, "Edge Switch");
        let uplink_port = interface(edge, 24);

        assert_eq!(
            candidate(uplink_port).and_then(|c| c.lldp_chassis_id.clone()),
            Some(LldpChassisId::MacAddress("aa:bb:cc:00:00:01".to_string()))
        );
        // "1/1/2" on the parent stack maps to if_index 1_001_002.
        assert_eq!(
            candidate(uplink_port).and_then(|c| c.lldp_port_id.clone()),
            Some(LldpPortId::LocallyAssigned("1001002".to_string()))
        );
        assert_eq!(
            candidate(uplink_port).and_then(|c| c.lldp_sys_name.clone()),
            Some("Core Stack".to_string())
        );
    }

    /// Wired clients become FDB entries on the port they sit on. Wireless clients have no port,
    /// the switch's own MAC carries no adjacency, and a port id we did not map is dropped rather
    /// than guessed at.
    #[test]
    fn wired_clients_become_port_fdb_entries() {
        let devices = map();
        let edge = find(&devices, "Edge Switch");
        let stack = find(&devices, "Core Stack");

        assert_eq!(
            interface(edge, 2).base.fdb_macs.as_deref(),
            Some(["de:ad:be:ef:00:01".to_string()].as_slice())
        );
        // A client on a stack member's port lands on that member's interface.
        assert_eq!(
            interface(stack, 2_001_001).base.fdb_macs.as_deref(),
            Some(["de:ad:be:ef:00:02".to_string()].as_slice())
        );
        // The wireless client is attached to the AP, which has no port to hold it.
        let ap = find(&devices, "Office AP");
        assert!(ap.interfaces[0].base.fdb_macs.is_none());
        // The switch's own MAC on its own port, and a client on an unmapped port id, are both
        // dropped — port 1 keeps a NULL fdb column rather than an empty list.
        assert!(interface(edge, 1).base.fdb_macs.is_none());
    }

    /// Host identity is IP-based, so a device with no address on a known subnet cannot be
    /// deduplicated across scans and is skipped. The caller reports the count.
    #[test]
    fn devices_without_a_known_subnet_ip_are_skipped() {
        let devices = map();
        assert_eq!(devices.len(), 4);
        assert!(!devices.iter().any(|d| {
            d.identity
                .name
                .as_deref()
                .is_some_and(|n| n == "Offsite Switch" || n == "Spare Switch")
        }));
    }

    /// Vendor JSON quotes scalars inconsistently. A single quoted number must not abort the
    /// parse, or the whole site's topology is lost — the symptom this integration exists to fix.
    #[test]
    fn tolerates_stringified_scalars_and_unknown_keys() {
        let devices = map();
        let stack = find(&devices, "Core Stack");

        // "speed": "1000" as a string still becomes 1 Gbit/s.
        assert_eq!(
            interface(stack, 1_001_002).base.speed_bps,
            Some(1_000_000_000)
        );
        // "up": "true" and "enabled": 1 both read as up.
        assert_eq!(
            interface(stack, 2_001_001).base.oper_status,
            Some(IfOperStatus::Up)
        );
        assert_eq!(
            interface(stack, 2_001_001).base.admin_status,
            Some(IfAdminStatus::Up)
        );
        // A down port is reported honestly.
        assert_eq!(
            interface(stack, 2_001_002).base.oper_status,
            Some(IfOperStatus::Down)
        );
        assert_eq!(
            interface(stack, 2_001_002).base.admin_status,
            Some(IfAdminStatus::Down)
        );
    }

    /// The device class is what lets the service matcher say "switch" or "access point" rather
    /// than only "Instant On", so it has to survive mapping verbatim.
    #[test]
    fn device_class_is_carried_through_for_service_matching() {
        let devices = map();
        assert_eq!(
            find(&devices, "Core Stack").device_type.as_deref(),
            Some("STACK")
        );
        assert_eq!(
            find(&devices, "Edge Switch").device_type.as_deref(),
            Some("SWITCH")
        );
        assert_eq!(
            find(&devices, "Site Gateway").device_type.as_deref(),
            Some("GATEWAY")
        );
    }

    /// Host fields the operator actually reads in the UI.
    #[test]
    fn host_carries_identity_from_the_inventory() {
        let devices = map();
        let stack = find(&devices, "Core Stack");
        assert_eq!(stack.identity.manufacturer.as_deref(), Some("HPE"));
        assert_eq!(stack.identity.model.as_deref(), Some("1960-48G-4SFP"));
        assert_eq!(stack.identity.serial_number.as_deref(), Some("STACK-SER-1"));
        assert_eq!(stack.identity.firmware_revision.as_deref(), Some("3.4.0"));
        // Canonicalised to the same lowercase-colon form SNMP writes, so a neighbor's advertised
        // chassis ID can reach this host by string equality.
        assert_eq!(
            stack.identity.chassis_id.as_deref(),
            Some("aa:bb:cc:00:00:01")
        );
    }
}
