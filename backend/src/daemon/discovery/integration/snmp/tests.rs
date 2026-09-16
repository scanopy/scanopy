use super::values::{value_to_i32, value_to_mac, value_to_string};
use snmp2::Value;

/// The interface set is persisted as soon as the ifTable walk finishes, before the
/// neighbour/FDB/VLAN queries have run — so it is built with no enrichment available.
/// Those bare interfaces still have to be complete, usable entities (the host is created
/// from them if a later query hangs), carrying every ifTable field and simply no
/// LLDP/CDP/FDB/VLAN data.
#[test]
fn interfaces_built_without_enrichment_keep_their_iftable_identity() {
    use super::*;

    let entry = types::IfTableEntry {
        if_index: 7,
        if_descr: Some("Port 7".to_string()),
        if_name: Some("swp7".to_string()),
        if_type: Some(6),
        if_speed: Some(1_000_000_000),
        if_admin_status: Some(1),
        if_oper_status: Some(1),
        ..Default::default()
    };
    let network_id = Uuid::new_v4();

    let interface = convert_snmp_if_entry(
        &entry,
        network_id,
        &[],
        &[],
        &[],
        &[],
        &std::collections::HashMap::new(),
        &std::collections::HashSet::new(),
    );

    // ifTable data survives the enrichment-free conversion.
    assert_eq!(interface.base.if_index, Some(7));
    assert_eq!(interface.base.if_descr.as_deref(), Some("Port 7"));
    assert_eq!(interface.base.if_name.as_deref(), Some("swp7"));
    assert_eq!(interface.base.if_type, Some(6));
    assert_eq!(interface.base.speed_bps, Some(1_000_000_000));
    assert_eq!(interface.base.network_id, network_id);

    // Enrichment that hasn't been collected yet is absent, not fabricated.
    assert!(interface.base.neighbor_candidates.is_empty());
    assert!(interface.base.fdb_macs.is_none());
    assert!(interface.base.native_vlan_id.is_none());
    assert!(interface.base.vlan_ids.is_none());
}

#[test]
fn test_value_to_string() {
    let value = Value::OctetString(b"test string");
    assert_eq!(value_to_string(&value), Some("test string".to_string()));
}

#[test]
fn test_value_to_i32() {
    let value = Value::Integer(42);
    assert_eq!(value_to_i32(&value), Some(42));
}

#[test]
fn test_value_to_mac() {
    let mac_bytes: [u8; 6] = [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34];
    let value = Value::OctetString(&mac_bytes);
    let mac = value_to_mac(&value).unwrap();
    assert_eq!(mac.bytes(), [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34]);
}

#[test]
fn test_convert_snmp_if_entry_with_vlan_data() {
    use super::convert_snmp_if_entry;
    use super::types::{IfTableEntry, PortVlanMembership};
    use uuid::Uuid;

    let entry = IfTableEntry {
        if_index: 5,
        if_descr: Some("GigabitEthernet0/5".to_string()),
        ..Default::default()
    };

    let membership = vec![
        PortVlanMembership {
            if_index: 5,
            native_vlan: Some(10),
            tagged_vlans: vec![20, 30],
        },
        PortVlanMembership {
            if_index: 7,
            native_vlan: Some(20),
            tagged_vlans: vec![],
        },
    ];

    let result = convert_snmp_if_entry(
        &entry,
        Uuid::nil(),
        &[],
        &[],
        &[],
        &membership,
        &std::collections::HashMap::new(),
        &std::collections::HashSet::new(),
    );

    assert_eq!(result.base.native_vlan_id, None);
    assert_eq!(result.base.vlan_ids, None);
}

#[test]
fn test_convert_snmp_if_entry_no_vlan_data() {
    use super::convert_snmp_if_entry;
    use super::types::IfTableEntry;
    use uuid::Uuid;

    let entry = IfTableEntry {
        if_index: 3,
        if_descr: Some("Loopback0".to_string()),
        ..Default::default()
    };

    let result = convert_snmp_if_entry(
        &entry,
        Uuid::nil(),
        &[],
        &[],
        &[],
        &[],
        &std::collections::HashMap::new(),
        &std::collections::HashSet::new(),
    );

    assert_eq!(result.base.native_vlan_id, None);
    assert_eq!(result.base.vlan_ids, None);
}

#[test]
fn test_convert_snmp_if_entry_empty_tagged_vlans() {
    use super::convert_snmp_if_entry;
    use super::types::{IfTableEntry, PortVlanMembership};
    use uuid::Uuid;

    let entry = IfTableEntry {
        if_index: 1,
        if_descr: Some("FastEthernet0/1".to_string()),
        ..Default::default()
    };

    // Access port: native VLAN only, no tagged VLANs
    let membership = vec![PortVlanMembership {
        if_index: 1,
        native_vlan: Some(10),
        tagged_vlans: vec![],
    }];

    let result = convert_snmp_if_entry(
        &entry,
        Uuid::nil(),
        &[],
        &[],
        &[],
        &membership,
        &std::collections::HashMap::new(),
        &std::collections::HashSet::new(),
    );

    assert_eq!(result.base.native_vlan_id, None);
    // Empty tagged_vlans should be stored as None (filtered)
    assert_eq!(result.base.vlan_ids, None);
}

// --- LLDP local-port remap (Issue 2: ExtremeXOS vs VOSS) ---

use super::types::{IfTableEntry, LldpLocalPort, LldpNeighbor};

/// Minimal LldpNeighbor carrying only a local-port index + a marker sys name.
fn lldp_neighbor(local_port_index: i32, sys_name: &str) -> LldpNeighbor {
    LldpNeighbor {
        local_port_index,
        remote_chassis_id_subtype: None,
        remote_chassis_id_bytes: None,
        remote_port_id_subtype: None,
        remote_port_id_bytes: None,
        remote_port_desc: None,
        remote_sys_name: Some(sys_name.to_string()),
        remote_sys_desc: None,
        remote_mgmt_addr: None,
    }
}

fn if_entry(if_index: i32, if_name: &str) -> IfTableEntry {
    IfTableEntry {
        if_index,
        if_name: Some(if_name.to_string()),
        ..Default::default()
    }
}

fn loc_port(subtype: u8, id: &str) -> LldpLocalPort {
    LldpLocalPort {
        port_id_subtype: Some(subtype),
        port_id: Some(id.to_string()),
        ..Default::default()
    }
}

#[test]
fn test_remap_lldp_exos_suffix_match() {
    use super::remap_lldp_local_ports;
    // ExtremeXOS X435: lldpRemTable local-port is an lldpLocPortNum (4, 11) in a
    // 1..N space; real ifIndex is 1001+, ifName "1:N". lldpLocPortId is "N",
    // subtype interfaceName(5) — must suffix-match against ifName "1:N".
    let if_entries = [
        if_entry(1001, "1:1"),
        if_entry(1004, "1:4"),
        if_entry(1011, "1:11"),
    ];
    let mut loc_ports = std::collections::HashMap::new();
    loc_ports.insert(4, loc_port(5, "4"));
    loc_ports.insert(11, loc_port(5, "11"));

    let mut neighbors = vec![lldp_neighbor(4, "peer-a"), lldp_neighbor(11, "peer-b")];
    remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(neighbors[0].local_port_index, 1004);
    assert_eq!(neighbors[1].local_port_index, 1011);
}

#[test]
fn test_remap_lldp_voss_exact_match_identity() {
    use super::remap_lldp_local_ports;
    // Extreme VOSS: lldpLocPortNum == ifIndex and lldpLocPortId ("1/1") matches
    // ifName exactly, so the resolved ifIndex equals the original index.
    let if_entries = [if_entry(192, "1/1"), if_entry(193, "1/2")];
    let mut loc_ports = std::collections::HashMap::new();
    loc_ports.insert(192, loc_port(5, "1/1"));
    loc_ports.insert(193, loc_port(5, "1/2"));

    let mut neighbors = vec![lldp_neighbor(192, "peer")];
    remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(neighbors[0].local_port_index, 192);
}

#[test]
fn test_remap_lldp_no_loc_table_is_identity() {
    use super::remap_lldp_local_ports;
    // No lldpLocPortTable (e.g. devices that report lldpLocPortNum == ifIndex):
    // indices are left untouched so existing behaviour is preserved.
    let if_entries = [if_entry(5, "Gi0/5")];
    let empty = std::collections::HashMap::new();
    let mut neighbors = vec![lldp_neighbor(5, "peer")];
    let outcome = remap_lldp_local_ports(&mut neighbors, &empty, &if_entries);
    assert_eq!(neighbors[0].local_port_index, 5);
    assert_eq!(outcome, super::LocalPortOutcome::default());
}

/// The identity path is only correct where the local port number *is* an ifIndex. Where the
/// device numbers its LLDP ports separately and serves no `lldpLocPortTable` — or served one
/// the walk could not read — every neighbour lands on an index no interface holds and is
/// discarded by `convert_snmp_if_entry` without a word. Returning zero here is what let a
/// switch report neighbours all day and appear to have none.
#[test]
fn an_absent_local_port_table_over_a_separate_numbering_drops_every_neighbour() {
    use super::remap_lldp_local_ports;

    let if_entries = [if_entry(1001, "1:1"), if_entry(1002, "1:2")];
    let empty = std::collections::HashMap::new();
    let mut neighbors = vec![lldp_neighbor(1, "peer-a"), lldp_neighbor(2, "peer-b")];

    let outcome = remap_lldp_local_ports(&mut neighbors, &empty, &if_entries);

    assert_eq!(outcome.dropped, 2);
    assert_eq!(
        outcome.unmatched, 0,
        "no tier ran, so nothing failed to match — the loss is the drop"
    );
}

/// GH #701: `convert_snmp_if_entry` used to attach only the first neighbour whose index
/// matched, so a second one on the same port was lost as completely as one on no port — the
/// confirmed root cause of "only 2 of 3 edges render on a shared L2 segment." It now emits
/// every matching neighbour as its own candidate, so both survive: neither
/// `remap_lldp_local_ports`' drop count nor `convert_snmp_if_entry`'s candidate set loses the
/// second one.
#[test]
fn a_second_neighbour_on_one_port_is_kept() {
    use super::{convert_snmp_if_entry, remap_lldp_local_ports};
    use uuid::Uuid;

    let if_entries = [if_entry(3, "Gi0/3")];
    let empty = std::collections::HashMap::new();
    let mut neighbors = vec![lldp_neighbor(3, "phone"), lldp_neighbor(3, "laptop")];

    let outcome = remap_lldp_local_ports(&mut neighbors, &empty, &if_entries);
    assert_eq!(outcome.dropped, 0, "both neighbours reach a real ifIndex");

    let result = convert_snmp_if_entry(
        &if_entries[0],
        Uuid::nil(),
        &neighbors,
        &[],
        &[],
        &[],
        &std::collections::HashMap::new(),
        &std::collections::HashSet::new(),
    );

    let sys_names: Vec<Option<String>> = result
        .base
        .neighbor_candidates
        .iter()
        .map(|c| c.lldp_sys_name.clone())
        .collect();
    assert_eq!(
        sys_names,
        vec![Some("phone".to_string()), Some("laptop".to_string())],
        "both neighbours on the shared port must survive as distinct candidates"
    );
}

#[test]
fn test_remap_lldp_interface_index_subtype() {
    use super::remap_lldp_local_ports;
    // lldpLocPortId subtype interfaceIndex(2): the id is literally the ifIndex.
    let if_entries = [if_entry(1007, "1:7")];
    let mut loc_ports = std::collections::HashMap::new();
    loc_ports.insert(7, loc_port(2, "1007"));
    let mut neighbors = vec![lldp_neighbor(7, "peer")];
    remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);
    assert_eq!(neighbors[0].local_port_index, 1007);
}

#[test]
fn test_remap_then_convert_attaches_exos_neighbor() {
    use super::{convert_snmp_if_entry, remap_lldp_local_ports};
    use uuid::Uuid;
    // End-to-end at the convert layer: after remap, the EXOS neighbour attaches
    // to the correct interface (which it would NOT before the fix, since
    // local_port_index 4 != ifIndex 1004).
    let if_entries = [if_entry(1004, "1:4")];
    let mut loc_ports = std::collections::HashMap::new();
    loc_ports.insert(4, loc_port(5, "4"));

    let mut neighbors = vec![lldp_neighbor(4, "switch-peer")];
    remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    let result = convert_snmp_if_entry(
        &if_entries[0],
        Uuid::nil(),
        &neighbors,
        &[],
        &[],
        &[],
        &std::collections::HashMap::new(),
        &std::collections::HashSet::new(),
    );
    assert_eq!(
        result
            .base
            .neighbor_candidates
            .first()
            .and_then(|c| c.lldp_sys_name.clone()),
        Some("switch-peer".to_string())
    );
}

// --- macAddress(3) local ports (Westermo industrial switches) ---

use std::collections::HashMap;

fn mac(last: u8) -> mac_address::MacAddress {
    mac_address::MacAddress::new([0x02, 0x00, 0x00, 0x00, 0x00, last])
}

fn if_entry_with_mac(if_index: i32, if_name: &str, phys: mac_address::MacAddress) -> IfTableEntry {
    IfTableEntry {
        if_phys_address: Some(phys),
        ..if_entry(if_index, if_name)
    }
}

/// A switch reporting `lldpLocPortIdSubtype = 3` gives each port's own MAC as the id, in raw
/// octets. That is not text, so the id never survived being read as a string and the port had
/// nothing to match on — every neighbour on the device stayed unresolved.
#[test]
fn a_port_identified_by_its_own_mac_resolves_to_that_interface() {
    use super::remap_lldp_local_ports;

    let if_entries = [
        if_entry_with_mac(1, "eth1", mac(0xE1)),
        if_entry_with_mac(2, "eth2", mac(0xE2)),
    ];
    let mut loc_ports = HashMap::new();
    loc_ports.insert(
        19,
        LldpLocalPort {
            port_id_subtype: Some(3),
            port_id_mac: Some(mac(0xE1)),
            ..Default::default()
        },
    );

    let mut neighbors = vec![lldp_neighbor(19, "peer")];
    let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(outcome.unmatched, 0);
    assert_eq!(neighbors[0].local_port_index, 1);
}

/// The D-Link DGS and TP-Link switches in GH #668 report the chassis MAC on every interface.
/// Matching on it would put every neighbour on whichever port won the lookup — a map that
/// looks complete and is wrong. The tier must decline and let a later one answer.
#[test]
fn a_mac_shared_by_every_interface_resolves_through_the_description_instead() {
    use super::remap_lldp_local_ports;

    let shared = mac(0xAA);
    let if_entries = [
        if_entry_with_mac(1, "eth1", shared),
        if_entry_with_mac(2, "eth2", shared),
    ];
    let mut loc_ports = HashMap::new();
    loc_ports.insert(
        19,
        LldpLocalPort {
            port_id_subtype: Some(3),
            port_id_mac: Some(shared),
            port_desc: Some("1000-LX eth2".to_string()),
            ..Default::default()
        },
    );

    let mut neighbors = vec![lldp_neighbor(19, "peer")];
    let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(outcome.unmatched, 0);
    assert_eq!(
        neighbors[0].local_port_index, 2,
        "the description names the port; the shared MAC names nothing"
    );
}

/// The reference case. Local port numbers run 10..19 against interfaces eth10 down to eth1,
/// so there is no arithmetic to exploit and the description is the only authority — and it
/// carries the media type in front of the name.
#[test]
fn local_port_numbers_that_run_backwards_map_through_the_description() {
    use super::remap_lldp_local_ports;

    let if_entries: Vec<IfTableEntry> = (1..=10)
        .map(|n| if_entry_with_mac(n, &format!("eth{n}"), mac(0xE0 + n as u8)))
        .collect();
    // Port 10 is eth10 and each port after it counts the interfaces back down.
    let mut loc_ports = HashMap::new();
    for port in 10..=19i32 {
        let interface = 20 - port;
        loc_ports.insert(
            port,
            LldpLocalPort {
                port_id_subtype: Some(3),
                // A distinct MAC per port, as this vendor sends — but one that belongs to no
                // interface, so only the description can place it.
                port_id_mac: Some(mac(0x70 + port as u8)),
                port_desc: Some(format!("100-T eth{interface}")),
                ..Default::default()
            },
        );
    }

    let mut neighbors = vec![
        lldp_neighbor(11, "peer-a"),
        lldp_neighbor(19, "peer-b"),
        lldp_neighbor(16, "peer-c"),
    ];
    let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(outcome.unmatched, 0);
    assert_eq!(
        neighbors
            .iter()
            .map(|n| n.local_port_index)
            .collect::<Vec<_>>(),
        vec![9, 1, 4]
    );
}

/// A description word that names two interfaces is not evidence of either. Leaving the
/// neighbour unresolved is the honest outcome — it is counted and warned about, where a
/// wrong port would be neither.
#[test]
fn a_description_matching_two_interfaces_resolves_to_neither() {
    use super::remap_lldp_local_ports;

    // Two interfaces answering to the same name across ifName and ifDescr.
    let if_entries = [
        if_entry(1, "eth1"),
        IfTableEntry {
            if_index: 2,
            if_descr: Some("eth1".to_string()),
            ..Default::default()
        },
    ];
    let mut loc_ports = HashMap::new();
    loc_ports.insert(
        11,
        LldpLocalPort {
            port_id_subtype: Some(3),
            port_desc: Some("100-T eth1".to_string()),
            ..Default::default()
        },
    );

    let mut neighbors = vec![lldp_neighbor(11, "peer")];
    let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(outcome.unmatched, 1);
    assert_eq!(
        neighbors[0].local_port_index, 11,
        "an unresolved neighbour keeps its local port number rather than guessing"
    );
    assert_eq!(
        outcome.dropped, 1,
        "port 11 is no interface's ifIndex, so the neighbour reaches nothing at all"
    );
}

// --- Dell OS10 breakout ports (GH #685) ---

/// A Dell PowerSwitch S4112T-ON as the reporter's switch is configured: port 14 broken out
/// into three lanes, so the interface names carry both a `/` and a `:`, and `mgmt1/1/1`
/// repeats the `/1` the lanes end on. Breakout lanes come before the management port because
/// OS10 numbers its ethernet interfaces below it.
fn dell_os10_if_entries() -> Vec<IfTableEntry> {
    let mut entries = vec![
        if_entry(15, "ethernet1/1/14:1"),
        if_entry(16, "ethernet1/1/14:2"),
        if_entry(17, "ethernet1/1/14:3"),
    ];
    entries.extend((1..=13).map(|n| if_entry(n + 1, &format!("ethernet1/1/{n}"))));
    entries.push(if_entry(1, "mgmt1/1/1"));
    entries
}

fn loc_port_named(subtype: u8, id: &str, desc: &str) -> LldpLocalPort {
    LldpLocalPort {
        port_desc: Some(desc.to_string()),
        ..loc_port(subtype, id)
    }
}

/// The mapping the reporter published, end to end: local ports 4, 568, 569 and 570 reach
/// `mgmt1/1/1` and the three lanes of port 14, and nothing else. `lldpLocPortNum` is a
/// separate namespace here — it runs past 568 against 23 interfaces — so every one of these
/// has to come from the port table rather than from the number itself.
#[test]
fn dell_os10_breakout_neighbours_reach_the_ports_the_switch_names() {
    use super::remap_lldp_local_ports;
    let if_entries = dell_os10_if_entries();
    let mut loc_ports = HashMap::new();
    loc_ports.insert(4, loc_port(5, "mgmt1/1/1"));
    loc_ports.insert(568, loc_port(5, "ethernet1/1/14:1"));
    loc_ports.insert(569, loc_port(5, "ethernet1/1/14:2"));
    loc_ports.insert(570, loc_port(5, "ethernet1/1/14:3"));

    let mut neighbors = vec![
        lldp_neighbor(570, "TAMMIERENEW"),
        lldp_neighbor(4, "unnamed-host"),
        lldp_neighbor(568, "EVILCORP"),
        lldp_neighbor(569, "VIRTUALPC"),
    ];
    let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(outcome, super::LocalPortOutcome::default());
    let placed: Vec<(&str, i32)> = neighbors
        .iter()
        .map(|n| (n.remote_sys_name.as_deref().unwrap(), n.local_port_index))
        .collect();
    assert_eq!(
        placed,
        vec![
            ("TAMMIERENEW", 17),
            ("unnamed-host", 1),
            ("EVILCORP", 15),
            ("VIRTUALPC", 16),
        ]
    );
}

/// The suffix tier anchors on `:` and `/`, and on this switch a bare port id ends at one in
/// three places at once — `mgmt1/1/1`, `ethernet1/1/1` and the first lane of port 14 all
/// qualify for the id "1". Taking the first match placed the neighbour on whichever interface
/// the ifTable happened to list first and recorded `PortIdSuffix` as though that were
/// evidence. An id matching three interfaces is evidence of none of them, so the walk falls
/// through to the description, which names one port and only one.
#[test]
fn a_port_id_ending_at_three_boundaries_defers_to_the_description() {
    use super::remap_lldp_local_ports;
    let if_entries = dell_os10_if_entries();
    let mut loc_ports = HashMap::new();
    loc_ports.insert(4, loc_port_named(5, "1", "mgmt1/1/1"));

    let mut neighbors = vec![lldp_neighbor(4, "peer")];
    let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(outcome, super::LocalPortOutcome::default());
    assert_eq!(
        neighbors[0].local_port_index, 1,
        "the description names mgmt1/1/1; the ambiguous suffix must not outrank it"
    );
}

/// An unambiguous suffix match can still be the wrong port. The management port advertising
/// the bare id "4" ends at a boundary in `ethernet1/1/4` and nowhere else, so uniqueness does
/// not save it — but the same row's description says `mgmt1/1/1` in full. A fragment that
/// matches one interface is still weaker evidence than a name that matches one interface, and
/// the tiers have to be ordered by how much the device actually told us.
#[test]
fn an_exact_name_in_the_description_outranks_a_matching_id_fragment() {
    use super::remap_lldp_local_ports;
    let if_entries = dell_os10_if_entries();
    let mut loc_ports = HashMap::new();
    loc_ports.insert(4, loc_port_named(7, "4", "mgmt1/1/1"));

    let mut neighbors = vec![lldp_neighbor(4, "peer")];
    let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(outcome, super::LocalPortOutcome::default());
    assert_eq!(
        neighbors[0].local_port_index, 1,
        "\"4\" ends at a boundary in ethernet1/1/4, but the device named mgmt1/1/1 outright"
    );
}

/// `interfaceIndex(2)` says the port id *is* an ifIndex, and the tier used to return whatever
/// integer arrived without asking whether the device has an interface by that number. On a
/// switch numbering its LLDP ports past 568 against 23 interfaces that is not a near miss:
/// the neighbour reaches no interface, `convert_snmp_if_entry` discards it whole, and the
/// switch reads as having no LLDP at all. An index naming nothing is not an answer, so the
/// later tiers get their turn.
#[test]
fn an_advertised_index_naming_no_interface_falls_through() {
    use super::remap_lldp_local_ports;
    let if_entries = dell_os10_if_entries();
    let mut loc_ports = HashMap::new();
    loc_ports.insert(568, loc_port_named(2, "568", "ethernet1/1/14:1"));

    let mut neighbors = vec![lldp_neighbor(568, "EVILCORP")];
    let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

    assert_eq!(outcome, super::LocalPortOutcome::default());
    assert_eq!(
        neighbors[0].local_port_index, 15,
        "no interface has ifIndex 568, so the description has to place the neighbour"
    );
}
