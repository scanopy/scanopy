//! Placing a device's LLDP neighbours onto its interfaces: translating `lldpLocPortNum` to
//! `ifIndex` through `lldpLocPortTable`, and counting the neighbours that reach no interface.

use super::*;

/// What placing a device's LLDP neighbours onto its interfaces produced.
///
/// Two different failures, because they call for different things. `unmatched` is a neighbour no
/// tier could place, which keeps its raw `lldpLocPortNum`; `dropped` is a neighbour that will
/// reach no interface at all and therefore contributes nothing — no chassis id is stored, no link
/// is drawn, and until this was counted the device simply looked as though it had no LLDP data.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LocalPortOutcome {
    /// Neighbours whose local port no tier could identify.
    pub unmatched: usize,
    /// Neighbours whose final index names no interface, or names one another neighbour already
    /// claimed. Every one of these is discarded whole by [`convert_snmp_if_entry`].
    pub dropped: usize,
}

/// Translate each LLDP neighbour's `local_port_index` from an `lldpLocPortNum` to the
/// device's real `ifIndex`, using `lldpLocPortTable` (`loc_ports`) resolved against the
/// interface table (`if_entries`). Neighbours whose port cannot be resolved keep their
/// original index. An empty `loc_ports` is identity — correct for devices where
/// `lldpLocPortNum == ifIndex` (e.g. Extreme VOSS) or that omit the table.
///
/// Both outcomes are counted because both are silent. An unmatched neighbour keeps its
/// `lldpLocPortNum`, which on a device where that is a separate namespace from `ifIndex` —
/// ExtremeXOS reports ports 1..N against ifIndexes 1001+ — attaches the link to whatever interface
/// happens to hold that index. Where it holds none, `convert_snmp_if_entry` attaches the neighbour
/// nowhere and the whole record is discarded: no `lldp_chassis_id` is ever written, so the device
/// contributes nothing and the server has nothing to resolve. The identity path is counted too —
/// returning zero there meant a device whose `lldpLocPortTable` was absent or unreadable dropped
/// every neighbour while raising no warning at all.
pub(crate) fn remap_lldp_local_ports(
    neighbors: &mut [LldpNeighbor],
    loc_ports: &HashMap<i32, LldpLocalPort>,
    if_entries: &[IfTableEntry],
) -> LocalPortOutcome {
    let mut outcome = LocalPortOutcome::default();

    // An empty table is the identity mapping, not a failure: devices where `lldpLocPortNum ==
    // ifIndex` (Extreme VOSS, most vendors) legitimately omit it. It still has to be checked —
    // identity is only correct where the number *is* an ifIndex.
    if !loc_ports.is_empty() {
        // Built once for the whole device rather than per neighbour, and deliberately only for
        // addresses belonging to exactly one interface. See [`unique_interface_macs`].
        let macs = unique_interface_macs(if_entries);
        for neighbor in neighbors.iter_mut() {
            let port = neighbor.local_port_index;
            match resolve_lldp_local_port(port, loc_ports, if_entries, &macs) {
                Some((if_index, evidence)) => {
                    tracing::debug!(
                        local_port = port,
                        if_index,
                        ?evidence,
                        "Matched an LLDP local port to an interface"
                    );
                    neighbor.local_port_index = if_index;
                }
                None => {
                    // The evidence, not just the failure. This is the line that decides whether the
                    // next unmatched switch needs another walk from its owner: it names what the
                    // device offered and therefore which tier would have to grow to place it.
                    let entry = loc_ports.get(&port);
                    tracing::debug!(
                        local_port = port,
                        subtype = ?entry.and_then(|e| e.port_id_subtype),
                        port_id = ?entry.and_then(|e| e.port_id.as_deref()),
                        port_id_mac = ?entry.and_then(|e| e.port_id_mac),
                        port_desc = ?entry.and_then(|e| e.port_desc.as_deref()),
                        "No interface matched an LLDP local port"
                    );
                    outcome.unmatched += 1;
                }
            }
        }
    }

    outcome.dropped = count_dropped_neighbours(neighbors, loc_ports, if_entries);
    outcome
}

/// Count the neighbours that will reach no interface, naming each one's evidence.
///
/// GH #701: `convert_snmp_if_entry` used to keep only the first neighbour per `local_port_index`
/// and this function counted every neighbour behind it as dropped — the exact mechanism behind
/// "only 2 of 3 edges render on a shared L2 segment." `convert_snmp_if_entry` now emits every
/// matching neighbour as its own candidate, so a second neighbour on an already-claimed port is no
/// longer discarded and must not be counted here either — only a neighbour naming an `ifIndex`
/// nothing on the device has is genuinely lost, and that is the one case left.
pub(crate) fn count_dropped_neighbours(
    neighbors: &[LldpNeighbor],
    loc_ports: &HashMap<i32, LldpLocalPort>,
    if_entries: &[IfTableEntry],
) -> usize {
    let if_indexes: HashSet<i32> = if_entries.iter().map(|e| e.if_index).collect();
    let mut dropped = 0;

    for neighbor in neighbors {
        let port = neighbor.local_port_index;
        if if_indexes.contains(&port) {
            continue;
        }

        let entry = loc_ports.get(&port);
        tracing::debug!(
            local_port = port,
            reason = "no interface on the device has this ifIndex",
            subtype = ?entry.and_then(|e| e.port_id_subtype),
            port_id = ?entry.and_then(|e| e.port_id.as_deref()),
            port_id_mac = ?entry.and_then(|e| e.port_id_mac),
            port_desc = ?entry.and_then(|e| e.port_desc.as_deref()),
            remote_chassis_subtype = ?neighbor.remote_chassis_id_subtype,
            remote_port_subtype = ?neighbor.remote_port_id_subtype,
            remote_port_desc = ?neighbor.remote_port_desc.as_deref(),
            remote_sys_name = ?neighbor.remote_sys_name.as_deref(),
            "Discarding an LLDP neighbour that reaches no interface"
        );
        dropped += 1;
    }

    dropped
}

/// Which column matched, for the log line that has to explain a device nothing matched on.
///
/// Ordered as the tiers are tried: an identifier that names the interface outright beats one that
/// has to be matched by shape, and both beat free text. Both name tiers come before both shape
/// tiers, whichever column they read — a whole name is more than the device had to tell us, and a
/// fragment that happens to match is less, so the column they arrive in does not outrank that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalPortEvidence {
    /// `lldpLocPortIdSubtype = 2` — the id is the ifIndex, and the device has one by that number.
    InterfaceIndex,
    /// `lldpLocPortIdSubtype = 3` — the id is a MAC held by exactly one interface.
    UniqueMac,
    /// `lldpLocPortId` equals an ifName, ifDescr or ifAlias.
    PortIdName,
    /// `lldpLocPortDesc` equals an ifName, ifDescr or ifAlias.
    PortDescName,
    /// `lldpLocPortId` is the tail of one interface's ifName or ifDescr, at a slot boundary.
    PortIdSuffix,
    /// One word of `lldpLocPortDesc` equals an ifName or ifDescr, and only one interface's.
    PortDescWord,
}

/// The interfaces whose `ifPhysAddress` identifies them on their own.
///
/// A MAC is only evidence of *which* port when the device gives each port a different one.
/// Westermo does; the D-Link DGS and TP-Link switches in GH #668 report the chassis address on
/// every interface, and matching on it there would collapse every neighbour onto one port —
/// worse than leaving them unresolved, because the resulting map looks complete. So an address
/// that appears more than once is dropped rather than arbitrated, and those devices fall through
/// to the description tiers.
///
/// The all-zero address is dropped for the same reason: it is what firmware reports for an
/// interface that has no hardware address, not an identity.
pub(crate) fn unique_interface_macs(if_entries: &[IfTableEntry]) -> HashMap<MacAddress, i32> {
    let unset = MacAddress::new([0; 6]);
    let mut by_mac: HashMap<MacAddress, Option<i32>> = HashMap::new();
    for e in if_entries {
        let Some(mac) = e.if_phys_address.filter(|m| *m != unset) else {
            continue;
        };
        by_mac
            .entry(mac)
            .and_modify(|slot| *slot = None)
            .or_insert(Some(e.if_index));
    }
    by_mac
        .into_iter()
        .filter_map(|(mac, if_index)| if_index.map(|i| (mac, i)))
        .collect()
}

/// Resolve a single `lldpLocPortNum` to an `ifIndex`, and say what matched it. Returns `None` to
/// keep the original value (no confident match).
///
/// Tiered most-specific-first, because the columns disagree in practice and the cost of a wrong
/// answer is a link drawn against the wrong port. Every tier is something a real device needed:
/// ExtremeXOS numbers its LLDP ports separately from its interfaces, Westermo identifies every
/// port by MAC and names it only in the description, and the description is free text on a device
/// that is under no obligation to make it parseable.
fn resolve_lldp_local_port(
    local_port_num: i32,
    loc_ports: &HashMap<i32, LldpLocalPort>,
    if_entries: &[IfTableEntry],
    unique_macs: &HashMap<MacAddress, i32>,
) -> Option<(i32, LocalPortEvidence)> {
    let entry = loc_ports.get(&local_port_num)?;

    // interfaceIndex(2): the port id is literally the ifIndex — but only if the device has an
    // interface by that number. Returning the advertised integer unchecked put neighbours on a
    // port that does not exist, where `count_dropped_neighbours` discards them whole and the
    // switch reads as having no LLDP at all; a Dell OS10 numbers its LLDP ports past 568 against
    // 23 interfaces, so an unchecked answer here is not a near miss (GH #685). Falling through
    // gives the name and description tiers, which know the OS10 port names, their turn.
    if entry.port_id_subtype == Some(2)
        && let Some(id) = entry.port_id.as_deref()
        && let Ok(idx) = id.trim().parse::<i32>()
        && if_entries.iter().any(|e| e.if_index == idx)
    {
        return Some((idx, LocalPortEvidence::InterfaceIndex));
    }

    // macAddress(3): the port id is the port's own hardware address, in raw octets. Only usable
    // where that address belongs to one interface — see `unique_interface_macs`.
    if entry.port_id_subtype == Some(3)
        && let Some(mac) = entry.port_id_mac
        && let Some(&if_index) = unique_macs.get(&mac)
    {
        return Some((if_index, LocalPortEvidence::UniqueMac));
    }

    let named = |text: &str, evidence| {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        // Exact match against ifName / ifDescr / ifAlias (VOSS: "1/1" == ifName "1/1"). ifAlias is
        // included to match the server's ladder in `server::lldp::resolver`, which added it
        // for Westermo WeOS — the daemon holding a narrower rule than the server meant the two
        // could place the same neighbour on different ports.
        if_entries
            .iter()
            .find(|e| {
                e.if_name.as_deref() == Some(text)
                    || e.if_descr.as_deref() == Some(text)
                    || e.if_alias.as_deref() == Some(text)
            })
            .map(|e| (e.if_index, evidence))
    };

    if let Some(id) = entry.port_id.as_deref()
        && let Some(hit) = named(id, LocalPortEvidence::PortIdName)
    {
        return Some(hit);
    }

    // An exact name in the description outranks a partial match on the id. Both are the same
    // question — which interface is this? — answered with different amounts of evidence, and the
    // suffix tier answers it from a fragment. A Dell OS10 breakout port advertising the bare id
    // "4" alongside the description "mgmt1/1/1" ends at the boundary in `ethernet1/1/4` and
    // nowhere else, so the fragment is unambiguous and wrong: it names a port on the front panel
    // while the device is telling us, in full, which port it means (GH #685).
    let desc = entry.port_desc.as_deref();
    if let Some(hit) = desc.and_then(|d| named(d, LocalPortEvidence::PortDescName)) {
        return Some(hit);
    }

    // Suffix match for vendors whose lldpLocPortId drops the slot prefix (EXOS: id
    // "4" vs ifName "1:4"). Anchor on a ':' or '/' boundary so "4" does not match
    // "14".
    //
    // Only when the boundary names one interface. The anchor characters mean different things to
    // different vendors — on EXOS ':' separates slot from port, on Dell OS10 it separates a port
    // from its breakout lane — so on a switch carrying both `ethernet1/1/1` and
    // `ethernet1/1/14:1` the id "1" ends at a boundary in three places at once, and taking the
    // first left a neighbour bound to a lane of an unrelated port, or to `mgmt1/1/1`, with
    // `PortIdSuffix` recorded as though it were evidence. Same rule as the description-word tier
    // below: an id matching two interfaces is evidence of neither.
    if let Some(id) = entry.port_id.as_deref() {
        let id = id.trim();
        if !id.is_empty() {
            let colon = format!(":{id}");
            let slash = format!("/{id}");
            let ends_at_boundary = |name: Option<&str>| {
                name.is_some_and(|n| n.ends_with(&colon) || n.ends_with(&slash))
            };
            let matched: Vec<i32> = if_entries
                .iter()
                .filter(|e| {
                    ends_at_boundary(e.if_name.as_deref())
                        || ends_at_boundary(e.if_descr.as_deref())
                })
                .map(|e| e.if_index)
                .collect();
            if let [only] = matched[..] {
                return Some((only, LocalPortEvidence::PortIdSuffix));
            }
        }
    }

    let desc = desc?;

    // The description is prose, and the interface name may be one word of it — Westermo sends
    // "100-T eth10" for the port whose ifName is "eth10". Take a word only when it identifies a
    // single interface: a description matching two of them is not evidence of either.
    let mut matched: Vec<i32> = Vec::new();
    for word in desc.split_whitespace() {
        for e in if_entries {
            if (e.if_name.as_deref() == Some(word) || e.if_descr.as_deref() == Some(word))
                && !matched.contains(&e.if_index)
            {
                matched.push(e.if_index);
            }
        }
    }
    if let [only] = matched[..] {
        return Some((only, LocalPortEvidence::PortDescWord));
    }

    None
}
