//! Address allocation for the lab: derived from position in the one existing device list.
//!
//! No device chooses its own address — [`assign_addresses`] does, by walking `devices::all()`'s
//! Vec in order. Two branches that each append a new device to that Vec's end land at different
//! positions after a merge and get different addresses automatically, so nobody ever types the
//! number `.226` again and nobody can type it twice.
//!
//! `192.168.7.192`–`192.168.7.254` is reserved for the lab: the top of `192.168.7.0/24`, itself
//! the top /24 of the `/22` the daemon already scans. Checked against the live dev DB and clear
//! of every real host on that network; `.255` stays the /22's real broadcast address.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};

use super::SimDevice;

/// First address the lab allocates. `devices::all()`'s position 0 gets this address, position 1
/// gets the next, and so on.
pub const FIRST_OCTET: u8 = 192;

/// How many devices the reserved block holds, `192.168.7.192`–`.254`. `.255` is the /22's real
/// broadcast address, not the lab's to take.
pub const CAPACITY: usize = 63;

/// The real netmask of the LAN the lab's macvlans sit on — a /22, not the smaller block the lab
/// reserves within it. A device's own address row should report what a real host on that segment
/// would, not the lab's internal bookkeeping.
pub const SHARED_NETMASK: Ipv4Addr = Ipv4Addr::new(255, 255, 252, 0);

/// Give every device its address, by position in the slice.
///
/// Panics if a device already carries a real address — a device module must never hand-pick
/// one, only [`assign_addresses`] does — or if the lab has grown past [`CAPACITY`], which is a
/// signal to ask for a dedicated VLAN rather than to keep shrinking the floor.
pub fn assign_addresses(devices: &mut [SimDevice]) {
    assert!(
        devices.len() <= CAPACITY,
        "the lab has grown to {} devices, past the {CAPACITY} the reserved block \
         (192.168.7.{FIRST_OCTET}-254) can hold — this needs a dedicated VLAN, not a lower floor",
        devices.len()
    );
    for (i, device) in devices.iter_mut().enumerate() {
        assert_eq!(
            device.ip,
            Ipv4Addr::UNSPECIFIED,
            "{} already has an address set — addresses are allocated by position in \
             devices::all(), never chosen per device",
            device.name
        );
        device.ip = Ipv4Addr::new(192, 168, 7, FIRST_OCTET + i as u8);
    }
}

/// Resolve every peer reference to the address its target ended up at.
///
/// Runs after [`assign_addresses`], because no device knows its own final address until then —
/// and a device cannot look a peer up through [`super::device`] during its own construction
/// without recursing into the very `devices::all()` call still building it.
pub fn resolve_peer_addresses(devices: &mut [SimDevice]) {
    let by_name: HashMap<&'static str, Ipv4Addr> = devices.iter().map(|d| (d.name, d.ip)).collect();

    for device in devices.iter_mut() {
        for neighbour in &mut device.tables.cdp.neighbours {
            if neighbour.remote_address.is_none()
                && let Some(peer) = neighbour.remote_device_id.as_deref()
                && let Some(&ip) = by_name.get(peer)
            {
                neighbour.remote_address = Some(IpAddr::V4(ip));
            }
        }
        if let Some(lldp) = &mut device.tables.lldp {
            for neighbour in &mut lldp.neighbours {
                if let Some(peer) = neighbour.mgmt_addr_of {
                    let ip = *by_name
                        .get(peer)
                        .unwrap_or_else(|| panic!("{peer} names no lab device"));
                    neighbour.mgmt_addr = Some(IpAddr::V4(ip));
                }
            }
        }
    }
}
