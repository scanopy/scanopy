//! LLDP: the neighbour walk against either LLDP MIB revision, the local port table and the
//! device's own chassis identity.

use super::*;

/// Query LLDP remote table for neighbor information.
///
/// The classic LLDP-MIB is walked first, and the LLDP-V2-MIB only when that walk finished and
/// found nothing (GH #688). The two are never merged: a device that serves both serves the same
/// neighbours twice, under different keys, and the classic result is the one every existing
/// device is read through.
pub async fn query_lldp_neighbors<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<LldpNeighbor>>> {
    let classic = query_lldp_neighbors_for(session, ip, &CLASSIC_LLDP_MIB).await?;

    // A classic walk that read nothing can mean three different agents: one with no LLDP at all,
    // one whose table is implemented and empty, and one that implements only the 802.1AB-2009
    // revision of the MIB under 1.3.111. The third cannot be told from the first two by the stop
    // reason: `unsupported` covers an agent that answers `noSuchObject`, but IP Infusion OcNOS
    // serves the `lldpExtensions` subtree under the classic root, so its classic columns end
    // `EndOfSubtree` — "implemented and empty" — and `unsupported` stays false. Zero rows from a
    // finished walk is therefore the gate, deliberately wider than `unsupported`: every device
    // whose classic table is implemented and empty — a host, a printer, a switch with nothing
    // plugged in — pays for it with eight single-page walks (the seven remote columns and the
    // management-address table) that find nothing.
    //
    // A walk that did *not* finish is a different matter. Its empty result is a read that failed,
    // not a device that has nothing, and falling back on it would let a V2 walk — or, worse, an
    // equally empty one — stand in for neighbours the classic table holds.
    if !classic.complete || !classic.records.is_empty() {
        return Ok(classic);
    }

    let mut v2 = query_lldp_neighbors_for(session, ip, &V2_LLDP_MIB).await?;
    // Only a V2 walk that finished and read nothing at all yields to the classic verdict. That
    // verdict is kept on purpose: an equally empty fallback must not launder an agent with no
    // LLDP into a supported-but-empty one, which would give the server authority to clear the
    // neighbours another integration wrote for it.
    //
    // A V2 walk that came up short is the opposite case and must *not* yield. On a V2-only
    // device the classic result is complete and not unsupported — the empty table the server
    // treats as authoritative — so handing it back after a transient V2 stall would clear the
    // edges the previous scan wrote. The incomplete V2 result goes back instead, empty and
    // marked as such. Rows the walk read and had to discard are likewise the device's own,
    // and the discard is what the operator needs told.
    if v2.complete && v2.records.is_empty() && v2.discarded == 0 {
        return Ok(classic);
    }
    debug!(
        ip = %ip,
        neighbors = v2.records.len(),
        complete = v2.complete,
        "classic LLDP-MIB empty; neighbours read from LLDP-V2-MIB"
    );
    v2.local_port_is_if_index = true;
    Ok(v2)
}

/// The neighbour walk, against whichever LLDP MIB `mib` names.
///
/// Everything here except the three fields of [`LldpMibProfile`] is MIB-agnostic — the shortfall
/// accumulators, the chassis-column key sets and their disagreement test, the ghost-row
/// classification, the wrong-type reporting, the short-index counter and
/// [`dominant_discard_reason`] are all about *how a walk failed*, not which OIDs it walked. That
/// machinery is the accumulated answer to GH #668, #674, #649 and #685, and a second MIB must
/// reuse it rather than grow a second copy that drifts.
pub async fn query_lldp_neighbors_for<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
    mib: &LldpMibProfile,
) -> Result<SnmpCollection<Vec<LldpNeighbor>>> {
    let mut neighbors: HashMap<(i32, i32), LldpNeighbor> = HashMap::new();
    let mut shortfall = Shortfall::default();

    // The two chassis columns get their own accumulators as well as folding into `shortfall`.
    //
    // A record missing its chassis ID is discarded below, and three unrelated things cause that:
    // one of these two columns stopping early, the agent answering them with a type we reject, or
    // rows existing in the later columns that these two never listed. The shared accumulator
    // cannot tell a chassis-column stop from a `remSysDesc` stop, which left `dropped=N` as the
    // only evidence and sent us asking customers to run snmpwalk by hand (GH #668). Same shape as
    // the management-address walk further down, for the same reason.
    let mut chassis_subtype_shortfall = Shortfall::default();
    let mut chassis_value_shortfall = Shortfall::default();

    // Keys each chassis column listed, kept apart rather than merged.
    //
    // Their union answers the ghost-row question: a key present in `neighbors` but in neither of
    // these was conjured by a later column alone, not lost to a truncated read. (`walk_if_table`
    // has the same guard in `known_if_indexes`; this table had none.)
    //
    // Their *disagreement* answers a second question the walk cannot. Both columns are mandatory
    // per IEEE 802.1AB, so a row one lists and the other does not means one read came up short —
    // whether the agent skipped a successor or the walk stopped early. That distinction is
    // invisible at the transport: a response that skips a row carries the right request id and a
    // well-formed OID, and is byte-for-byte a legitimate end-of-column. Judging it by OID position
    // instead is the assumption GH #674 had to remove before unsorted firmware could be read at
    // all. Which rows each column enumerated is evidence of a different kind, and it is already
    // here for the asking.
    let mut subtype_keys: HashSet<(i32, i32)> = HashSet::new();
    let mut value_keys: HashSet<(i32, i32)> = HashSet::new();

    // Values rejected for being the wrong ASN.1 type, by the type the agent actually sent. A
    // count says something went wrong; the type says what, and the two point at different
    // remedies.
    let mut unexpected_subtype_type: Option<&'static str> = None;
    let mut unexpected_value_type: Option<&'static str> = None;

    // Rows whose OID index carried too few sub-identifiers to key at all. Counted rather than
    // skipped: this used to be a bare `return` and it is how an entire switch went missing in
    // silence (see `split_lldp_rem_index`).
    let mut short_index = 0usize;

    // Which columns to walk, and how to read a neighbour key out of a row's index, are the whole
    // of what varies between LLDP MIB revisions — see [`LldpMibProfile`].
    let columns = mib.remote_columns;

    // Every column answering "no such object" is how an agent says it has no LLDP-MIB, as
    // opposed to walking past the end of a table it implements but has no neighbours in.
    let mut all_columns_unsupported = true;

    for (base_oid_str, column_name) in columns {
        let stop = walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                let Some((local_port, rem_index)) = (mib.split_rem_index)(suffix) else {
                    short_index += 1;
                    return;
                };
                let neighbor =
                    neighbors
                        .entry((local_port, rem_index))
                        .or_insert_with(|| LldpNeighbor {
                            local_port_index: local_port,
                            remote_chassis_id_subtype: None,
                            remote_chassis_id_bytes: None,
                            remote_port_id_subtype: None,
                            remote_port_id_bytes: None,
                            remote_port_desc: None,
                            remote_sys_name: None,
                            remote_sys_desc: None,
                            remote_mgmt_addr: None,
                        });
                match column_name {
                    "remChassisIdSubtype" => {
                        subtype_keys.insert((local_port, rem_index));
                        match value_to_i32(value) {
                            Some(v) => neighbor.remote_chassis_id_subtype = Some(v as u8),
                            // Not a silent discard any more. An agent answering the subtype with
                            // a Null or an Opaque looks exactly like a walk that never reached
                            // this row, and only one of those is worth retrying.
                            None => {
                                unexpected_subtype_type.get_or_insert(value_type_name(value));
                            }
                        }
                    }
                    "remChassisId" => {
                        value_keys.insert((local_port, rem_index));
                        match value {
                            Value::OctetString(bytes) => {
                                neighbor.remote_chassis_id_bytes = Some(bytes.to_vec());
                            }
                            other => {
                                unexpected_value_type.get_or_insert(value_type_name(other));
                            }
                        }
                    }
                    "remPortIdSubtype" => {
                        neighbor.remote_port_id_subtype = value_to_i32(value).map(|v| v as u8)
                    }
                    "remPortId" => {
                        if let Value::OctetString(bytes) = value {
                            neighbor.remote_port_id_bytes = Some(bytes.to_vec());
                        }
                    }
                    "remPortDesc" => neighbor.remote_port_desc = value_to_string(value),
                    "remSysName" => neighbor.remote_sys_name = value_to_string(value),
                    "remSysDesc" => neighbor.remote_sys_desc = value_to_string(value),
                    _ => {}
                }
            },
        )
        .await;
        match column_name {
            "remChassisIdSubtype" => chassis_subtype_shortfall.record(stop),
            "remChassisId" => chassis_value_shortfall.record(stop),
            _ => {}
        }
        if !stop.is_unsupported() {
            all_columns_unsupported = false;
        }
    }

    // Resolve remote management addresses from the separate lldpRemManAddrTable.
    // Its index is timeMark.localPortNum.remIndex.addrSubtype.addrLen.addr, so the
    // address lives in the OID *index*, not the column value. We walk an accessible
    // column (lldpRemManAddrIfSubtype) and reconstruct the address from the index.
    let man_base_oid_str = mib.man_addr_column;
    // Management address is optional enrichment; ignore walk errors (keeps the
    // neighbours already collected above).
    // Management address is optional enrichment, so it gets its own accumulator and its outcome
    // is not folded into the neighbours'.
    let mut mgmt = Shortfall::default();
    walk_column(
        session,
        ip,
        man_base_oid_str,
        &mut mgmt,
        |suffix, _value| {
            if let Some((local_port, rem_index, buf)) = (mib.split_man_addr_index)(suffix)
                && let Some(addr) = parse_lldp_mgmt_addr(&buf)
                && let Some(neighbor) = neighbors.get_mut(&(local_port, rem_index))
            {
                neighbor.remote_mgmt_addr = Some(addr);
            }
        },
    )
    .await;
    // A missing management address never gates resolution (topology.rs matches on chassis/port
    // only), so a truncated walk here is not a reason to withhold the neighbours themselves.
    if !mgmt.complete {
        debug!(ip = %ip, "LLDP management-address walk was cut short");
    }

    // Per IEEE 802.1AB the chassis ID is a mandatory TLV, so a neighbour record without one is
    // malformed by construction — in practice, the tail of a cut-short chassis column while the
    // port-id and sys-name columns completed. Emitting it would overwrite a good chassis ID with
    // NULL, and a row with no chassis ID is excluded from L2 resolution entirely, so it could
    // never recover. Drop it and report the walk as partial instead.
    let before = neighbors.len();
    // Classified as we filter, because "14 were dropped" is not a diagnosis. Each counter below
    // has a different remedy — a truncated column is worth a rescan, an agent answering with the
    // wrong type never will be, and a ghost row is neither.
    let mut ghost_rows = 0usize;
    let mut missing_subtype = 0usize;
    let mut missing_value = 0usize;
    // Rows the chassis columns *did* list and that still arrived without a usable chassis ID.
    // Counted per row rather than as `missing_subtype + missing_value`, which double-counts a row
    // that lost both halves and would then outweigh the other causes for no reason.
    let mut missing_chassis = 0usize;
    let result: Vec<LldpNeighbor> = neighbors
        .into_iter()
        .filter(|(key, n)| {
            let has_subtype = n.remote_chassis_id_subtype.is_some();
            let has_value = n.remote_chassis_id_bytes.is_some();
            if has_subtype && has_value {
                return true;
            }
            if !subtype_keys.contains(key) && !value_keys.contains(key) {
                // Neither chassis column ever listed this (localPortNum, remIndex). A later
                // column invented it, so there was never a chassis ID to lose.
                ghost_rows += 1;
            } else {
                missing_chassis += 1;
                if !has_subtype {
                    missing_subtype += 1;
                }
                if !has_value {
                    missing_value += 1;
                }
            }
            false
        })
        .map(|(_, n)| n)
        .collect();
    // Rows lost before they could be keyed never reached `neighbors`, so they are not in
    // `before - result.len()`. They are still records the device served and we could not use.
    let discarded = (before - result.len()) + short_index;
    let discard_reason = dominant_discard_reason(
        ghost_rows,
        missing_chassis,
        short_index,
        unexpected_subtype_type.is_some() || unexpected_value_type.is_some(),
        !chassis_subtype_shortfall.complete || !chassis_value_shortfall.complete,
        // Only a *partial* disagreement is evidence of a stop. A column that listed nothing at all
        // while its sibling listed rows, on a walk that ended cleanly, is a column the device does
        // not implement — truncation means rows arrived and then stopped. Without that guard,
        // firmware that simply omits `lldpRemChassisIdSubtype` is reported as a read worth
        // retrying, which is the opposite of the advice its operator needs.
        subtype_keys != value_keys && !subtype_keys.is_empty() && !value_keys.is_empty(),
    );
    if discarded > 0 {
        shortfall.complete = false;
        warn!(
            ip = %ip,
            dropped = discarded,
            ghost_rows,
            missing_subtype,
            missing_value,
            short_index,
            unexpected_subtype_type,
            unexpected_value_type,
            subtype_walk = ?chassis_subtype_shortfall.reason,
            value_walk = ?chassis_value_shortfall.reason,
            // Non-zero with both walks reporting clean is the shape that has no other tell: one
            // column simply listed rows the other did not.
            chassis_column_gap = subtype_keys.symmetric_difference(&value_keys).count(),
            reason = ?discard_reason,
            "LLDP neighbours missing the mandatory chassis ID; discarding them and marking the \
             walk partial"
        );
    }
    // Only when nothing was read: a device that answered with rows plainly has the MIB, whatever
    // the last column's stop reason was.
    let unsupported = all_columns_unsupported && result.is_empty();
    debug!(
        ip = %ip,
        neighbors = result.len(),
        complete = shortfall.complete,
        unsupported,
        "LLDP query finished"
    );

    Ok(SnmpCollection {
        discarded,
        discard_reason,
        records: result,
        complete: shortfall.complete,
        unsupported,
        reason: shortfall.reason,
        // The LLDP/CDP claim is the device's own local identity, which is read later in the
        // collection than this walk, so the caller attaches it.
        claim: None,
        local_port_is_if_index: false,
    })
}

/// Reads `(local port, remote index, [family, addr…])` out of a management-address table index.
///
/// The address lives in the OID index rather than a column value, which is why this returns bytes
/// rather than a parsed address — `parse_lldp_mgmt_addr` does that part.
type ManAddrIndexSplitter = fn(&[u64]) -> Option<(i32, i32, Vec<u8>)>;

/// Which LLDP MIB a neighbour walk is reading.
///
/// The classic LLDP-MIB (`1.0.8802.1.1.2`) is not the only one in the field: some NOSes implement
/// only the 802.1AB-2009 LLDP-V2-MIB (`1.3.111.2.802.1.1.13`), and a device that serves one and not
/// the other contributes no L2 edges at all. The two differ in exactly three ways — which columns
/// to walk, how to read a neighbour key out of a row's index, and where the management address
/// lives — so they are named here rather than duplicating [`query_lldp_neighbors_for`], whose bulk
/// is failure diagnosis that neither MIB gets to have its own version of.
///
/// The subtype enumerations are *identical* between the two revisions, so nothing downstream of
/// the walk — `LldpChassisId`, `LldpPortId`, `from_snmp`, the stored JSONB — varies by profile.
pub struct LldpMibProfile {
    /// The seven remote-table columns, each with the short name used in warnings. Note the column
    /// numbers are not shared between revisions: `lldpV2RemEntry` inserts `lldpV2RemLocalIfIndex`
    /// as column 2, so every V2 remote column sits one above its classic counterpart.
    pub remote_columns: [(&'static str, &'static str); 7],
    /// Read `(local port, remote index)` out of the sub-ids following a remote column's OID.
    pub split_rem_index: fn(&[u64]) -> Option<(i32, i32)>,
    /// The accessible column of the separate management-address table, walked for its *index*.
    pub man_addr_column: &'static str,
    /// Read the neighbour key and address bytes out of that table's index.
    pub split_man_addr_index: ManAddrIndexSplitter,
}

/// The classic LLDP-MIB, `1.0.8802.1.1.2`.
pub static CLASSIC_LLDP_MIB: LldpMibProfile = LldpMibProfile {
    remote_columns: [
        (
            oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
            "remChassisIdSubtype",
        ),
        (
            oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID,
            "remChassisId",
        ),
        (
            oids::lldp::remote::entry::LLDP_REM_PORT_ID_SUBTYPE,
            "remPortIdSubtype",
        ),
        (oids::lldp::remote::entry::LLDP_REM_PORT_ID, "remPortId"),
        (oids::lldp::remote::entry::LLDP_REM_PORT_DESC, "remPortDesc"),
        (oids::lldp::remote::entry::LLDP_REM_SYS_NAME, "remSysName"),
        (oids::lldp::remote::entry::LLDP_REM_SYS_DESC, "remSysDesc"),
    ],
    split_rem_index: split_lldp_rem_index,
    // `lldpRemManAddr` is deliberately not among the columns above: it lives in the separate
    // `lldpRemManAddrTable`, whose index carries extra trailing sub-ids, so the neighbour-key
    // splitter does not apply to it.
    man_addr_column: oids::lldp::remote::entry::LLDP_REM_MAN_ADDR_IF_SUBTYPE,
    split_man_addr_index: split_lldp_man_addr_index,
};

/// Split an `lldpRemEntry` OID index into `(lldpRemLocalPortNum, lldpRemIndex)`.
///
/// The index is `timeMark.localPortNum.remIndex`, and not every firmware serves all three. A
/// TP-Link TL-SX3016F omits the time mark and indexes on the remaining two, confirmed by snmpwalk
/// against the reporter's switch (GH #668):
///
/// ```text
///   iso.0.8802.1.1.2.1.4.1.1.4.1.1 = INTEGER: 4
///   iso.0.8802.1.1.2.1.4.1.1.5.1.1 = STRING: "00:AD:24:89:CC:F0"
/// ```
///
/// — three well-formed neighbours on local ports 1, 3 and 5, each remIndex 1.
///
/// So the pair is read off the *end* rather than from a fixed offset: the local port and remote
/// index are the final two sub-ids under either layout, and a conformant three-element index
/// parses exactly as before.
///
/// The old `suffix.len() < 3` guard did not merely mis-parse these rows, it made the device
/// disappear without trace: no record was created, so nothing reached the discard counters,
/// `complete` stayed true, and an empty result from a switch with sixteen ports was then treated
/// as authoritative — overwriting the LLDP data the server held with NULL. It was the only
/// failure in this walk that produced no warning of any kind.
///
/// **Reading from the end is what makes this table-specific, and it does not generalise.**
/// `lldpV2RemEntry` (LLDP-V2-MIB, `1.3.111.2.802.1.1.13`) is indexed
/// `timeMark.localIfIndex.localDestMACAddress.remIndex` — four sub-ids, because
/// `LldpV2DestAddressTableIndex` is an `Unsigned32(1..4096)` row pointer into
/// `lldpV2DestAddressTable`, a single sub-id and not six octets of MAC. Passed a V2 suffix this
/// function returns `(destAddressIndex, remIndex)`, so every neighbour on the device collapses
/// onto the destination-address index — in practice 1, the nearest-bridge group address — and all
/// but one is discarded as a duplicate, silently, in exactly the way described above. A V2 walk
/// needs its own splitter that reads from the front and requires all four sub-ids; it must also
/// skip `remap_lldp_local_ports`, since `lldpV2RemLocalIfIndex` is already an ifIndex.
pub(super) fn split_lldp_rem_index(suffix: &[u64]) -> Option<(i32, i32)> {
    let (&rem_index, head) = suffix.split_last()?;
    let &local_port = head.last()?;
    Some((local_port as i32, rem_index as i32))
}

/// Split an `lldpRemManAddrTable` OID index into its neighbour key and management address.
///
/// The index is `timeMark.localPortNum.remIndex.addrSubtype.addrLen.addr`, and the firmware that
/// omits `lldpRemTimeMark` from `lldpRemTable` omits it here too, leaving a two-element neighbour
/// key instead of three. The two layouts are told apart by arithmetic rather than guessed: the
/// address length is declared inside the index, so only one prefix length makes it account for
/// exactly the sub-ids that follow. The conformant layout is tried first.
///
/// Returns `(localPortNum, remIndex, [ianaFamily, addr bytes...])` — the byte buffer
/// `parse_lldp_mgmt_addr` expects.
fn split_lldp_man_addr_index(suffix: &[u64]) -> Option<(i32, i32, Vec<u8>)> {
    for prefix in [3usize, 2] {
        if suffix.len() < prefix + 2 {
            continue;
        }
        let addr_len = suffix[prefix + 1] as usize;
        if addr_len == 0 || suffix.len() != prefix + 2 + addr_len {
            continue;
        }
        let mut buf = Vec::with_capacity(1 + addr_len);
        buf.push(suffix[prefix] as u8);
        buf.extend(suffix[prefix + 2..].iter().map(|&b| b as u8));
        return Some((suffix[prefix - 2] as i32, suffix[prefix - 1] as i32, buf));
    }
    None
}

/// The LLDP-V2-MIB, `1.3.111.2.802.1.1.13` (GH #688).
///
/// Walked only as a fallback — see [`query_lldp_neighbors`] — and never through the classic
/// splitters: its remote entry has one more index sub-id and one more column than the classic
/// one, and its local identifier is an ifIndex rather than an `lldpLocPortNum`.
pub static V2_LLDP_MIB: LldpMibProfile = LldpMibProfile {
    remote_columns: [
        (
            oids::lldp_v2::remote::entry::LLDP_V2_REM_CHASSIS_ID_SUBTYPE,
            "remChassisIdSubtype",
        ),
        (
            oids::lldp_v2::remote::entry::LLDP_V2_REM_CHASSIS_ID,
            "remChassisId",
        ),
        (
            oids::lldp_v2::remote::entry::LLDP_V2_REM_PORT_ID_SUBTYPE,
            "remPortIdSubtype",
        ),
        (
            oids::lldp_v2::remote::entry::LLDP_V2_REM_PORT_ID,
            "remPortId",
        ),
        (
            oids::lldp_v2::remote::entry::LLDP_V2_REM_PORT_DESC,
            "remPortDesc",
        ),
        (
            oids::lldp_v2::remote::entry::LLDP_V2_REM_SYS_NAME,
            "remSysName",
        ),
        (
            oids::lldp_v2::remote::entry::LLDP_V2_REM_SYS_DESC,
            "remSysDesc",
        ),
    ],
    split_rem_index: split_lldp_v2_rem_index,
    man_addr_column: oids::lldp_v2::remote::entry::LLDP_V2_REM_MAN_ADDR_IF_SUBTYPE,
    split_man_addr_index: split_lldp_v2_man_addr_index,
};

/// Split an `lldpV2RemEntry` OID index into `(lldpV2RemLocalIfIndex, lldpV2RemIndex)`.
///
/// The index is `timeMark.localIfIndex.localDestMACAddress.remIndex`, read from the front and
/// required whole. This is deliberately not [`split_lldp_rem_index`], which reads its pair off
/// the end to tolerate an omitted time mark: applied to a four-sub-id V2 index that returns
/// `(destAddressIndex, remIndex)`, collapsing every neighbour on the device onto the
/// destination-address index — 1, the nearest-bridge group address — and discarding all but one
/// as duplicates, silently. No V2 firmware has been seen omitting the time mark, and guessing at
/// a three-sub-id layout would re-open exactly that ambiguity, so a short or long index is
/// counted in `short_index` rather than parsed.
pub(super) fn split_lldp_v2_rem_index(suffix: &[u64]) -> Option<(i32, i32)> {
    match suffix {
        [_time_mark, local_if_index, _dest_mac_index, rem_index] => {
            Some((*local_if_index as i32, *rem_index as i32))
        }
        _ => None,
    }
}

/// Split an `lldpV2RemManAddrTable` OID index into its neighbour key and management address.
///
/// The index is `timeMark.ifIndex.destMacIndex.remIndex.addrSubtype.addrLen.addr…`, and the
/// conformant layout is tried first, accepted only when the declared length accounts for exactly
/// the sub-ids that follow. The firmware this was written against (IP Infusion OcNOS 7.0.1) serves
/// **no address-length sub-identifier** — the address bytes simply run to the end of the index —
/// so that layout is the fallback:
///
/// ```text
///   1.3.111...13.1.4.2.1.3.0.10009.1.6.1.192.0.2.102   (ifIndex 10009, remIndex 6, IPv4)
///   1.3.111...13.1.4.2.1.3.0.3.1.4.2                   (a row with subtype but no address)
/// ```
///
/// The second shape yields an empty address buffer, which `parse_lldp_mgmt_addr` rejects — the
/// right outcome for a row that carries nothing to resolve. The two layouts are ambiguous in one
/// case: a length-less address whose first octet happens to equal the count of octets after it
/// (an IPv4 address starting with 3) parses as conformant and loses that octet, and
/// `parse_lldp_mgmt_addr` then rejects the three-byte IPv4 — so the row resolves to no address,
/// never to a wrong one.
///
/// Returns `(ifIndex, remIndex, [ianaFamily, addr bytes...])` — the byte buffer
/// `parse_lldp_mgmt_addr` expects.
pub(super) fn split_lldp_v2_man_addr_index(suffix: &[u64]) -> Option<(i32, i32, Vec<u8>)> {
    let [
        _time_mark,
        if_index,
        _dest_mac_index,
        rem_index,
        addr_subtype,
        rest @ ..,
    ] = suffix
    else {
        return None;
    };
    let mut buf = Vec::with_capacity(1 + rest.len());
    buf.push(*addr_subtype as u8);
    match rest {
        // Conformant: the length sub-id is followed by exactly that many address sub-ids.
        [addr_len, addr @ ..] if *addr_len > 0 && addr.len() == *addr_len as usize => {
            buf.extend(addr.iter().map(|&b| b as u8));
        }
        // Length-less: whatever follows the subtype is the address.
        addr => buf.extend(addr.iter().map(|&b| b as u8)),
    }
    Some((*if_index as i32, *rem_index as i32, buf))
}

/// The cause that explains most of what a device's LLDP walk threw away.
///
/// One device can hit several at once and the operator-facing warning has room for one sentence
/// per device, so the largest count wins — except for truncation, which is not a cause among
/// several. What the sentence must get right is whether a rescan is worth their time: only a
/// cut-short read recovers on its own, and telling someone to retry a switch whose firmware serves
/// malformed records wastes the one action they have.
///
/// Returns `None` when nothing was discarded, so a healthy walk carries no reason to report.
fn dominant_discard_reason(
    ghost_rows: usize,
    missing_chassis: usize,
    short_index: usize,
    wrong_type: bool,
    chassis_walk_truncated: bool,
    chassis_columns_disagree: bool,
) -> Option<MalformedNeighbourReason> {
    if ghost_rows + missing_chassis + short_index == 0 {
        return None;
    }
    // Truncation overrides the counts rather than competing with them. A chassis column that
    // stopped early lists none of the rows past the stop, so its casualties are indistinguishable
    // from rows the column never had — they land in `ghost_rows`, or in `missing_chassis` when the
    // sibling column did list them, and would otherwise be reported as a firmware defect no rescan
    // can fix.
    //
    // The evidence is ranked, because it is not equally good. A shortfall the walk recognised is
    // decisive; a type we actually recorded is decisive about the firmware; two chassis columns
    // enumerating different rows is only circumstantial, and is the sole trace an agent leaves
    // when it skips a successor row.
    // A column that reported a shortfall stopped for a reason the walk itself recognised, and
    // that is the strongest evidence there is.
    if chassis_walk_truncated {
        return Some(MalformedNeighbourReason::WalkCutShort);
    }
    // A recorded type outranks the circumstantial signal below it. We saw what the agent put on
    // the wire for that column, which a truncated read never gets to see — so a device answering
    // `lldpRemChassisIdSubtype` with an OCTET STRING is telling us about its firmware, not about
    // our read, and a rescan will produce the same answer forever.
    if wrong_type {
        return Some(MalformedNeighbourReason::UnexpectedType);
    }
    // Circumstantial: an agent that skips a successor row ends the column on a clean
    // `EndOfSubtree` — right request id, well-formed OID, nothing to retry on — and the only trace
    // left is the two mandatory chassis columns having enumerated different rows.
    if chassis_columns_disagree {
        return Some(MalformedNeighbourReason::WalkCutShort);
    }
    // The read finished, so a row the chassis columns listed and left without a usable value is
    // something the device did.
    let missing_reason = MalformedNeighbourReason::IncompleteRecords;
    [
        (missing_chassis, missing_reason),
        (ghost_rows, MalformedNeighbourReason::GhostRows),
        (short_index, MalformedNeighbourReason::UnreadableIndex),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    // Strictly-greater keeps the first of a tie, so the order above is the tie-break and the
    // result does not depend on iteration order.
    .reduce(|best, next| if next.0 > best.0 { next } else { best })
    .map(|(_, reason)| reason)
}

/// Walk lldpLocPortTable, returning `lldpLocPortNum -> LldpLocalPort`.
///
/// The local-port index reported in `lldpRemTable` is an `lldpLocPortNum`, which on
/// some vendors (e.g. ExtremeXOS) is a separate namespace from `ifIndex`. This table
/// maps that number to a textual port id (`lldpLocPortId`), which the caller resolves
/// back to the real ifIndex. Returns an empty map if the device does not expose the
/// table (callers fall back to treating the local-port number as the ifIndex).
pub async fn query_lldp_local_ports<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<HashMap<i32, LldpLocalPort>>> {
    let mut ports: HashMap<i32, LldpLocalPort> = HashMap::new();
    let mut shortfall = Shortfall::default();

    let columns = [
        (oids::lldp::local::LLDP_LOC_PORT_ID_SUBTYPE, "subtype"),
        (oids::lldp::local::LLDP_LOC_PORT_ID, "id"),
        (oids::lldp::local::LLDP_LOC_PORT_DESC, "desc"),
    ];

    for (base_oid_str, column_name) in columns {
        // Index is a single sub-id: lldpLocPortNum.
        walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                let Some(&local_port_num) = suffix.first() else {
                    return;
                };
                let entry = ports.entry(local_port_num as i32).or_default();
                match column_name {
                    "subtype" => entry.port_id_subtype = value_to_i32(value).map(|v| v as u8),
                    "id" => {
                        // Both readings of the same column, because the subtype decides which one
                        // is meaningful and the columns arrive in separate walks. A macAddress(3)
                        // port id is six raw octets, which is not text — reading it only as a
                        // string dropped it silently and left the port unresolvable.
                        //
                        // Reading every id as a MAC would misread a six-character port *name* as
                        // one (`canonical_mac` documents the trap). That is safe here only because
                        // the resolver consults this field on subtype 3 alone.
                        entry.port_id = value_to_string(value);
                        entry.port_id_mac = value_to_mac(value).or_else(|| {
                            // Firmware that renders the address as text rather than octets, the
                            // same quirk `LldpPortId::from_snmp` already absorbs on the remote side.
                            entry
                                .port_id
                                .as_deref()
                                .and_then(canonical_mac)
                                .and_then(|m| m.parse().ok())
                        });
                    }
                    "desc" => entry.port_desc = value_to_string(value),
                    _ => {}
                }
            },
        )
        .await;
    }

    debug!(
        "lldpLocPortTable from {} returned {} local ports",
        ip,
        ports.len()
    );
    Ok(SnmpCollection::from_walk(ports, shortfall))
}

/// Query local LLDP chassis ID (scalar GETs, not walks).
/// Returns the device's own LLDP identity.
pub async fn query_lldp_local<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<Option<LldpLocalInfo>> {
    let (subtype, chassis_bytes) = get_lldp_local_chassis(
        session,
        ip,
        oids::lldp::local::LLDP_LOC_CHASSIS_ID_SUBTYPE,
        oids::lldp::local::LLDP_LOC_CHASSIS_ID,
    )
    .await;

    // The same fallback as `query_lldp_neighbors`, for the same agents (GH #688). A device whose
    // LLDP lives only under the V2 root has no classic `lldpLocChassisId` either, and without its
    // own identity it can sit in every other device's neighbour table yet never resolve as
    // itself. Two scalar GETs, attempted only when the classic pair returned nothing.
    let (subtype, chassis_bytes) = match (subtype, chassis_bytes) {
        (None, None) => {
            let v2 = get_lldp_local_chassis(
                session,
                ip,
                oids::lldp_v2::local::LLDP_V2_LOC_CHASSIS_ID_SUBTYPE,
                oids::lldp_v2::local::LLDP_V2_LOC_CHASSIS_ID,
            )
            .await;
            if v2.0.is_some() || v2.1.is_some() {
                debug!("LLDP local identity read from LLDP-V2-MIB for {}", ip);
            }
            v2
        }
        classic => classic,
    };

    match (subtype, chassis_bytes) {
        (Some(subtype), Some(bytes)) => {
            debug!(
                "LLDP local info from {}: subtype={}, bytes_len={}",
                ip,
                subtype,
                bytes.len()
            );
            Ok(Some(LldpLocalInfo {
                chassis_id_subtype: subtype,
                chassis_id_bytes: bytes,
            }))
        }
        _ => {
            debug!("LLDP local info incomplete from {}", ip);
            Ok(None)
        }
    }
}

/// GET the two scalars that make up a device's own LLDP chassis identity, from whichever MIB
/// revision `subtype_oid`/`chassis_oid` name. Either half is `None` when the agent does not
/// serve it, answers with the wrong type, or fails the request.
async fn get_lldp_local_chassis<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
    subtype_oid: &str,
    chassis_oid: &str,
) -> (Option<u8>, Option<Vec<u8>>) {
    let subtype = match session.get_scalar(&oids::oid_parts(subtype_oid)).await {
        Ok(value) => value
            .and_then(|value| value_to_i32(&value))
            .map(|v| v as u8),
        Err(e) => {
            debug!(
                "LLDP local chassis ID subtype GET failed from {}: {}",
                ip, e
            );
            None
        }
    };

    let chassis_bytes = match session.get_scalar(&oids::oid_parts(chassis_oid)).await {
        Ok(value) => value.and_then(|value| match value {
            Value::OctetString(bytes) => Some(bytes.to_vec()),
            _ => None,
        }),
        Err(e) => {
            debug!("LLDP local chassis ID GET failed from {}: {}", ip, e);
            None
        }
    };

    (subtype, chassis_bytes)
}
