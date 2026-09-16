//! The same identity lookups [`LldpResolverImpl`] runs as queries, served from one preloaded
//! network.
//!
//! Resolution asks the same handful of questions once per neighbour-bearing interface — which host
//! carries this chassis id, which port on it carries this name — and a query apiece made the pass
//! scale with round-trips rather than with work. On a 145-interface network that is ~330 ms; the
//! same shape at a few thousand interfaces approaches the daemon's 30 s request budget, and the
//! completion request is what waits for it.
//!
//! Every lookup keys on an *identity* column — MAC, address, `if_descr` / `if_name` / `if_alias`,
//! `if_index`, chassis id, sys name. None reads the `neighbor_*` columns the resolution loop writes
//! as it goes, which is what makes a snapshot taken once at the start of the pass safe to use
//! throughout it. A lookup that ever needs to see resolution's own writes does not belong here.
//!
//! The indexes keep the **0 / 1 / many** verdict rather than a first match. `Unique::Multiple` is
//! how a non-unique column reports that it identifies nothing, and flattening it to "found one"
//! would start attaching links to an arbitrary one of several identically named devices — the
//! failure `find_host_by_sys_name` and `find_if_entry_by_mac` are documented as preventing.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::net::IpAddr;

use async_trait::async_trait;
use mac_address::MacAddress;
use uuid::Uuid;

use super::{IdentityResolution, LldpResolver};
use crate::server::hosts::r#impl::base::Host;
use crate::server::interfaces::r#impl::base::{Interface, if_type::EXCLUDED_IF_TYPES};
use crate::server::ip_addresses::r#impl::base::{IPAddress, mac_of};
use crate::server::shared::attribution::text_of;
use crate::server::shared::storage::traits::Unique;

/// One entry of an index over a column that does not promise uniqueness.
///
/// Mirrors [`Unique`] without owning a row: the id is only meaningful while exactly one thing
/// claimed the key, and a second claimant erases it rather than queueing behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Claim {
    One(Uuid),
    Many,
}

impl Claim {
    fn verdict(found: Option<&Self>) -> Unique<Uuid> {
        match found {
            Some(Self::One(id)) => Unique::One(*id),
            Some(Self::Many) => Unique::Multiple,
            None => Unique::None,
        }
    }
}

/// Record that `id` claims `key`, collapsing to [`Claim::Many`] on the second distinct claimant.
///
/// Distinct is the operative word: the same row indexed twice — which happens wherever one
/// interface is reachable under two spellings of the same name — must not make the key ambiguous
/// with itself.
fn claim<K: Eq + Hash>(index: &mut HashMap<K, Claim>, key: K, id: Uuid) {
    index
        .entry(key)
        .and_modify(|existing| {
            if *existing != Claim::One(id) {
                *existing = Claim::Many;
            }
        })
        .or_insert(Claim::One(id));
}

/// Every identity lookup resolution needs, for one network, read once.
///
/// Built from live rows only — each index is populated from an already-filtered load, so there is
/// no `valid_to` test here and none of the queries this replaces had one beyond `.live()`.
pub struct LldpInventorySnapshot {
    /// `hosts.chassis_id` → host.
    host_by_chassis_id: HashMap<String, Claim>,
    /// `hosts.sys_name` → host.
    host_by_sys_name: HashMap<String, Claim>,
    /// `ip_addresses.ip_address` → host.
    host_by_address: HashMap<IpAddr, Claim>,
    /// `ip_addresses.mac_address` → host, on the *row*, matching the `get_unique` this replaces:
    /// two rows carrying one MAC is not a resolution even when both sit on one host.
    host_by_address_mac: HashMap<MacAddress, Claim>,
    /// `interfaces.mac_address` → the distinct hosts carrying it. A switch repeating its chassis
    /// MAC across 48 ports is one host and one answer, so this collapses before counting.
    hosts_by_interface_mac: HashMap<MacAddress, HashSet<Uuid>>,
    /// `interfaces.if_descr` → host, network-wide.
    host_by_interface_descr: HashMap<String, Claim>,

    /// `(host, MAC)` → interface, physical rows only.
    interface_by_host_mac: HashMap<(Uuid, MacAddress), Claim>,
    /// `(host, MAC)` → interface, physical rows with `ip_configured` set only. Consulted as a tie-break
    /// when `interface_by_host_mac` comes back `Claim::Many` — see `find_if_entry_by_mac`.
    interface_by_host_mac_ip_configured: HashMap<(Uuid, MacAddress), Claim>,
    /// `(host, if_descr)` → interface.
    interface_by_host_descr: HashMap<(Uuid, String), Claim>,
    /// `(host, if_name)` → interface.
    interface_by_host_name: HashMap<(Uuid, String), Claim>,
    /// `(host, if_alias)` → interface.
    interface_by_host_alias: HashMap<(Uuid, String), Claim>,
    /// `(host, if_index)` → interface.
    interface_by_host_index: HashMap<(Uuid, i32), Claim>,
    /// `(host, address)` → the `ip_addresses` row, for the FK hop below.
    address_row_by_host_address: HashMap<(Uuid, IpAddr), Claim>,
    /// `interfaces.ip_address_id` → interface, the second half of that hop.
    interface_by_address_row: HashMap<Uuid, Claim>,
}

impl LldpInventorySnapshot {
    /// Index one network's live hosts, interfaces and addresses.
    ///
    /// The caller loads them; this only decides how they are looked up. Keeping the loads outside
    /// means the snapshot has no opinion about which service or filter produced the rows, and can
    /// be built in tests from literals.
    pub fn new(hosts: &[Host], interfaces: &[Interface], addresses: &[IPAddress]) -> Self {
        let mut snapshot = Self {
            host_by_chassis_id: HashMap::new(),
            host_by_sys_name: HashMap::new(),
            host_by_address: HashMap::new(),
            host_by_address_mac: HashMap::new(),
            hosts_by_interface_mac: HashMap::new(),
            host_by_interface_descr: HashMap::new(),
            interface_by_host_mac: HashMap::new(),
            interface_by_host_mac_ip_configured: HashMap::new(),
            interface_by_host_descr: HashMap::new(),
            interface_by_host_name: HashMap::new(),
            interface_by_host_alias: HashMap::new(),
            interface_by_host_index: HashMap::new(),
            address_row_by_host_address: HashMap::new(),
            interface_by_address_row: HashMap::new(),
        };

        for host in hosts {
            if let Some(chassis_id) = text_of(&host.base.chassis_id) {
                claim(&mut snapshot.host_by_chassis_id, chassis_id, host.id);
            }
            if let Some(sys_name) = text_of(&host.base.sys_name) {
                claim(&mut snapshot.host_by_sys_name, sys_name, host.id);
            }
        }

        for address in addresses {
            let host_id = address.base.host_id;
            claim(
                &mut snapshot.host_by_address,
                address.base.ip_address,
                host_id,
            );
            if let Some(mac) = mac_of(&address.base.mac_address) {
                claim(&mut snapshot.host_by_address_mac, mac, host_id);
            }
            claim(
                &mut snapshot.address_row_by_host_address,
                (host_id, address.base.ip_address),
                address.id,
            );
        }

        for interface in interfaces {
            let host_id = interface.base.host_id;

            if let Some(mac) = mac_of(&interface.base.mac_address) {
                snapshot
                    .hosts_by_interface_mac
                    .entry(mac)
                    .or_default()
                    .insert(host_id);
                // Virtual rows are excluded from the per-host lookup only. A VLAN interface is not
                // the far end of a cable, and letting it contest the MAC turned lookups ambiguous
                // that no physical port would have contested. An unread `if_type` counts as
                // physical, matching `physical_if_types()`.
                if interface
                    .base
                    .if_type
                    .is_none_or(|if_type| !EXCLUDED_IF_TYPES.contains(&if_type))
                {
                    claim(
                        &mut snapshot.interface_by_host_mac,
                        (host_id, mac),
                        interface.id,
                    );
                    // Same population, narrowed to rows the device's own ipAddrTable binds an
                    // address to — see `InterfaceBase::ip_configured`.
                    if interface.base.ip_configured {
                        claim(
                            &mut snapshot.interface_by_host_mac_ip_configured,
                            (host_id, mac),
                            interface.id,
                        );
                    }
                }
            }

            if let Some(ref if_descr) = interface.base.if_descr {
                claim(
                    &mut snapshot.host_by_interface_descr,
                    if_descr.clone(),
                    host_id,
                );
                claim(
                    &mut snapshot.interface_by_host_descr,
                    (host_id, if_descr.clone()),
                    interface.id,
                );
            }
            if let Some(ref if_name) = interface.base.if_name {
                claim(
                    &mut snapshot.interface_by_host_name,
                    (host_id, if_name.clone()),
                    interface.id,
                );
            }
            if let Some(ref if_alias) = interface.base.if_alias {
                claim(
                    &mut snapshot.interface_by_host_alias,
                    (host_id, if_alias.clone()),
                    interface.id,
                );
            }
            if let Some(if_index) = interface.base.if_index {
                claim(
                    &mut snapshot.interface_by_host_index,
                    (host_id, if_index),
                    interface.id,
                );
            }
            if let Some(address_row) = interface.base.ip_address_id {
                claim(
                    &mut snapshot.interface_by_address_row,
                    address_row,
                    interface.id,
                );
            }
        }

        snapshot
    }

    /// One rung of `find_if_entry_by_name`: the three name columns, in the order the queries try
    /// them, resolving on a single match only.
    fn interface_named(&self, host_id: Uuid, name: &str) -> Option<Uuid> {
        let key = (host_id, name.to_string());
        [
            &self.interface_by_host_descr,
            &self.interface_by_host_name,
            &self.interface_by_host_alias,
        ]
        .into_iter()
        .find_map(|index| Claim::verdict(index.get(&key)).found())
    }
}

#[async_trait]
impl LldpResolver for LldpInventorySnapshot {
    async fn find_host_by_mac(&self, mac: &str, _network_id: Uuid) -> IdentityResolution {
        let Ok(mac_addr) = mac.parse::<MacAddress>() else {
            return IdentityResolution::NotFound;
        };

        // Addresses first, and only a single row short-circuits: several rows carrying the MAC
        // fall through to the interface tier rather than resolving, exactly as the `Unique::One`
        // test in the query version does.
        if let Unique::One(host_id) = Claim::verdict(self.host_by_address_mac.get(&mac_addr)) {
            return IdentityResolution::Resolved(host_id);
        }

        match self.hosts_by_interface_mac.get(&mac_addr) {
            None => IdentityResolution::NotFound,
            Some(hosts) => match hosts.len() {
                0 => IdentityResolution::NotFound,
                1 => hosts
                    .iter()
                    .next()
                    .copied()
                    .map(IdentityResolution::Resolved)
                    .unwrap_or(IdentityResolution::NotFound),
                _ => IdentityResolution::Ambiguous,
            },
        }
    }

    async fn find_host_by_ip(&self, ip: &IpAddr, _network_id: Uuid) -> IdentityResolution {
        IdentityResolution::from_unique(Claim::verdict(self.host_by_address.get(ip)))
    }

    async fn find_host_by_if_name(&self, name: &str, _network_id: Uuid) -> IdentityResolution {
        IdentityResolution::from_unique(Claim::verdict(self.host_by_interface_descr.get(name)))
    }

    async fn find_host_by_chassis_id(
        &self,
        chassis_id: &str,
        _network_id: Uuid,
    ) -> IdentityResolution {
        IdentityResolution::from_unique(Claim::verdict(self.host_by_chassis_id.get(chassis_id)))
    }

    async fn find_host_by_sys_name(&self, sys_name: &str, _network_id: Uuid) -> IdentityResolution {
        IdentityResolution::from_unique(Claim::verdict(self.host_by_sys_name.get(sys_name)))
    }

    async fn find_if_entry_by_mac(&self, mac: &str, host_id: Uuid) -> IdentityResolution {
        let Ok(mac_addr) = mac.parse::<MacAddress>() else {
            return IdentityResolution::NotFound;
        };
        let key = (host_id, mac_addr);
        match Claim::verdict(self.interface_by_host_mac.get(&key)) {
            // Several physical rows share this MAC — see if exactly one of them is also the one
            // the device's own ipAddrTable binds an address to (mirrors `LldpResolverImpl`'s
            // `get_all`-then-narrow; this index already holds only that subset). Zero matches
            // here is still a tie among the physical candidates, not a fresh "not found" — the
            // base lookup already proved at least two rows exist.
            Unique::Multiple => {
                match Claim::verdict(self.interface_by_host_mac_ip_configured.get(&key)) {
                    Unique::One(id) => IdentityResolution::Resolved(id),
                    Unique::None | Unique::Multiple => IdentityResolution::Ambiguous,
                }
            }
            verdict => IdentityResolution::from_unique(verdict),
        }
    }

    async fn find_if_entry_by_name(&self, name: &str, host_id: Uuid) -> Option<Uuid> {
        if let Some(id) = self.interface_named(host_id, name) {
            return Some(id);
        }
        // MikroTik RouterOS advertises bridged ports as "<bridge>/<port>", which matches no stored
        // name; retry on the segment after the last '/'.
        let (_, suffix) = name.rsplit_once('/')?;
        if suffix.is_empty() {
            return None;
        }
        self.interface_named(host_id, suffix)
    }

    async fn find_if_entry_by_if_index(&self, if_index: i32, host_id: Uuid) -> Option<Uuid> {
        Claim::verdict(self.interface_by_host_index.get(&(host_id, if_index))).found()
    }

    async fn find_if_entry_by_ip(&self, ip: &IpAddr, host_id: Uuid) -> Option<Uuid> {
        let address_row =
            Claim::verdict(self.address_row_by_host_address.get(&(host_id, *ip))).found()?;
        Claim::verdict(self.interface_by_address_row.get(&address_row)).found()
    }
}
