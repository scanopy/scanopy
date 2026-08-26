//! LLDP, local and remote.
//!
//! Identifiers are [`LldpChassisId`] / [`LldpPortId`], so the subtype is the variant and a fixture
//! cannot advertise subtype 4 carrying something that is not an address. What the enums do not
//! carry — because they are post-parse types — is how the bytes leave the agent, and that is
//! [`Advertised`].
//!
//! **The encoding here is not a detail to normalise.** Unlike `ifPhysAddress`, where six raw
//! octets are the only correct form and text is silently dropped, `parse_mac_id` accepts *both*
//! for an LLDP identifier — deliberately, because real firmware sends both. The lab covers both
//! branches on purpose: `switch-dlink-01` sends raw octets (the only end-to-end coverage of that
//! branch), `switch-tplink-01` sends uppercase ASCII, and `switch-exos-01`'s own chassis id is
//! left *abbreviated* (`0:4:96:1:e0:0`) as the standing guard on the unpadded form. Changing any
//! of those removes coverage rather than fixing anything.

use super::wire::{MacEncoding, PassValue, Row};
use crate::daemon::discovery::integration::snmp::oids::{lldp, lldp_v2};
use crate::server::lldp::{LldpChassisId, LldpPortId};

/// Which LLDP MIB a simulated table serves.
///
/// The classic LLDP-MIB is not the only one devices implement: some NOSes ship only the
/// 802.1AB-2009 LLDP-V2-MIB, rooted elsewhere, with the remote columns shifted by one and a
/// four-sub-id row index. A device that serves one and not the other is a real failure shape, so
/// the lab has to be able to describe it — which means the OIDs and the index layout are values
/// here rather than constants baked into `wire_rows`.
///
/// [`CLASSIC`] and [`V2`] are each a table of constants plus an index composer; nothing else in
/// the simulator knows which a device serves — `data_files` and `registrations` derive the
/// filename and the served subtree from `root`/`file_suffix`, and `SimAgent` serves whatever it
/// is registered for.
#[derive(Debug)]
pub struct SimLldpMib {
    /// Subtree the agent registers, and the whole of what a walk of this MIB can reach.
    pub root: &'static str,
    /// Distinguishes this MIB's data file from another's on the same device.
    pub file_suffix: &'static str,
    pub local: SimLldpLocalColumns,
    pub remote: SimLldpRemoteColumns,
    /// The index sub-ids a remote row is keyed by.
    pub rem_suffix: fn(&RemoteNeighbour) -> Vec<u64>,
}

/// `lldpLocalSystemData` columns. The scalars must carry their own trailing `.0`: `Row::scalar`
/// appends nothing, so a constant without it is served at the object OID and no walk finds it.
#[derive(Debug)]
pub struct SimLldpLocalColumns {
    pub chassis_id_subtype: &'static str,
    pub chassis_id: &'static str,
    pub sys_name: &'static str,
    pub sys_desc: &'static str,
    pub port_id_subtype: &'static str,
    pub port_id: &'static str,
    pub port_desc: &'static str,
}

/// `lldpRemEntry` columns. Column *numbers* are not shared between MIB revisions.
#[derive(Debug)]
pub struct SimLldpRemoteColumns {
    pub chassis_id_subtype: &'static str,
    pub chassis_id: &'static str,
    pub port_id_subtype: &'static str,
    pub port_id: &'static str,
    pub port_desc: &'static str,
    pub sys_name: &'static str,
    pub sys_desc: &'static str,
}

/// The classic LLDP-MIB, `1.0.8802.1.1.2`.
pub static CLASSIC: SimLldpMib = SimLldpMib {
    root: lldp::LLDP_MIB,
    file_suffix: "lldp",
    local: SimLldpLocalColumns {
        chassis_id_subtype: lldp::local::LLDP_LOC_CHASSIS_ID_SUBTYPE,
        chassis_id: lldp::local::LLDP_LOC_CHASSIS_ID,
        sys_name: lldp::local::LLDP_LOC_SYS_NAME,
        sys_desc: lldp::local::LLDP_LOC_SYS_DESC,
        port_id_subtype: lldp::local::LLDP_LOC_PORT_ID_SUBTYPE,
        port_id: lldp::local::LLDP_LOC_PORT_ID,
        port_desc: lldp::local::LLDP_LOC_PORT_DESC,
    },
    remote: SimLldpRemoteColumns {
        chassis_id_subtype: lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
        chassis_id: lldp::remote::entry::LLDP_REM_CHASSIS_ID,
        port_id_subtype: lldp::remote::entry::LLDP_REM_PORT_ID_SUBTYPE,
        port_id: lldp::remote::entry::LLDP_REM_PORT_ID,
        port_desc: lldp::remote::entry::LLDP_REM_PORT_DESC,
        sys_name: lldp::remote::entry::LLDP_REM_SYS_NAME,
        sys_desc: lldp::remote::entry::LLDP_REM_SYS_DESC,
    },
    rem_suffix: classic_rem_suffix,
};

/// The LLDP-V2-MIB, `1.3.111.2.802.1.1.13` (GH #688).
///
/// The local tables keep the classic column numbering; the remote entry inserts
/// `lldpV2RemLocalIfIndex` as column 2, so every remote column sits one above its classic
/// counterpart, and its index gains a fourth sub-id — see [`v2_rem_suffix`].
pub static V2: SimLldpMib = SimLldpMib {
    root: lldp_v2::LLDP_V2_MIB,
    file_suffix: "lldpv2",
    local: SimLldpLocalColumns {
        chassis_id_subtype: lldp_v2::local::LLDP_V2_LOC_CHASSIS_ID_SUBTYPE,
        chassis_id: lldp_v2::local::LLDP_V2_LOC_CHASSIS_ID,
        sys_name: lldp_v2::local::LLDP_V2_LOC_SYS_NAME,
        sys_desc: lldp_v2::local::LLDP_V2_LOC_SYS_DESC,
        port_id_subtype: lldp_v2::local::LLDP_V2_LOC_PORT_ID_SUBTYPE,
        port_id: lldp_v2::local::LLDP_V2_LOC_PORT_ID,
        port_desc: lldp_v2::local::LLDP_V2_LOC_PORT_DESC,
    },
    remote: SimLldpRemoteColumns {
        chassis_id_subtype: lldp_v2::remote::entry::LLDP_V2_REM_CHASSIS_ID_SUBTYPE,
        chassis_id: lldp_v2::remote::entry::LLDP_V2_REM_CHASSIS_ID,
        port_id_subtype: lldp_v2::remote::entry::LLDP_V2_REM_PORT_ID_SUBTYPE,
        port_id: lldp_v2::remote::entry::LLDP_V2_REM_PORT_ID,
        port_desc: lldp_v2::remote::entry::LLDP_V2_REM_PORT_DESC,
        sys_name: lldp_v2::remote::entry::LLDP_V2_REM_SYS_NAME,
        sys_desc: lldp_v2::remote::entry::LLDP_V2_REM_SYS_DESC,
    },
    rem_suffix: v2_rem_suffix,
};

/// The `lldpV2DestAddressTable` row every neighbour in the lab is learned through: 1, the
/// nearest-bridge group address, which is the only destination the firmware this models uses.
const NEAREST_BRIDGE_DEST_INDEX: u64 = 1;

/// `lldpV2RemTimeMark.lldpV2RemLocalIfIndex.lldpV2RemLocalDestMACAddress.lldpV2RemIndex` — four
/// sub-ids, the second a real ifIndex and the third a row pointer, not six octets of MAC. No V2
/// firmware has been seen omitting the time mark, so [`TimeMark::Omitted`] serves it as 0 rather
/// than inventing a three-sub-id layout the daemon deliberately rejects.
fn v2_rem_suffix(neighbour: &RemoteNeighbour) -> Vec<u64> {
    let mark = match neighbour.time_mark {
        TimeMark::At(mark) => mark as u64,
        TimeMark::Omitted => 0,
    };
    vec![
        mark,
        neighbour.local_port as u64,
        NEAREST_BRIDGE_DEST_INDEX,
        neighbour.index as u64,
    ]
}

/// `lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex` — three sub-ids, or two where the firmware
/// omits the time mark (GH #668).
fn classic_rem_suffix(neighbour: &RemoteNeighbour) -> Vec<u64> {
    match neighbour.time_mark {
        TimeMark::At(mark) => vec![
            mark as u64,
            neighbour.local_port as u64,
            neighbour.index as u64,
        ],
        TimeMark::Omitted => vec![neighbour.local_port as u64, neighbour.index as u64],
    }
}

/// An identifier together with how the agent puts it on the wire.
///
/// The encoding is only consulted for the MAC-valued variants; everything else is text by
/// definition. Constructing one forces the choice to be made explicitly at the call site, which is
/// the point.
#[derive(Debug, Clone)]
pub struct Advertised<T> {
    pub id: T,
    pub encoding: MacEncoding,
}

impl<T> Advertised<T> {
    /// Six raw octets — what a conforming agent sends.
    pub fn octets(id: T) -> Self {
        Self {
            id,
            encoding: MacEncoding::Octets,
        }
    }

    /// The identifier as text. Legitimate for LLDP — `parse_mac_id` accepts it — and named so the
    /// choice is visible in the device definition rather than implied by a quoted string.
    pub fn text(id: T, encoding: MacEncoding) -> Self {
        Self { id, encoding }
    }
}

/// How a firmware indexes `lldpRemEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeMark {
    /// `lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex`, as the MIB describes.
    At(u32),
    /// The time mark omitted, so every row arrives one sub-id short (GH #668, TP-Link
    /// TL-SX3016F). A parser requiring three sub-ids built no record at all, nothing reached the
    /// discard counters, and the walk still reported itself complete.
    Omitted,
}

/// A chassis column served wrongly, on purpose.
///
/// The one place a fixture may contradict itself, because these are the shapes real firmware
/// produces and each drives a different per-cause counter and a different piece of operator
/// advice (GH #668). Naming them is what keeps "deliberate defect" apart from "mistake".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChassisDefect {
    /// The row appears in no chassis column at all — a ghost row. Indistinguishable from a
    /// chassis column that never had those positions, which is why it reads as `GhostRows`.
    NoChassisColumns,
    /// `lldpRemChassisId` present, its subtype absent — an incomplete record.
    NoSubtype,
    /// The subtype served as an OCTET STRING where an INTEGER belongs, carrying the text the
    /// device actually sends — `macAddress` rather than `4`. Reads as a *complete* walk, with no
    /// truncation signal anywhere, so before the per-cause counters the only evidence was the
    /// record silently going missing.
    SubtypeWrongType(&'static str),
}

/// A neighbour as the agent advertises it.
#[derive(Debug, Clone)]
pub struct RemoteNeighbour {
    pub time_mark: TimeMark,
    pub local_port: u32,
    pub index: u32,
    pub chassis: Option<Advertised<LldpChassisId>>,
    pub port: Option<Advertised<LldpPortId>>,
    pub port_desc: Option<String>,
    pub sys_name: Option<String>,
    pub sys_desc: Option<String>,
    /// Set only where the malformed shape is the point.
    pub defect: Option<ChassisDefect>,
}

impl RemoteNeighbour {
    /// A well-formed neighbour with a time mark of 0, which is what most of the lab serves.
    pub fn new(
        local_port: u32,
        chassis: Advertised<LldpChassisId>,
        port: Advertised<LldpPortId>,
    ) -> Self {
        Self {
            time_mark: TimeMark::At(0),
            local_port,
            index: 1,
            chassis: Some(chassis),
            port: Some(port),
            port_desc: None,
            sys_name: None,
            sys_desc: None,
            defect: None,
        }
    }

    pub fn time_mark(mut self, time_mark: TimeMark) -> Self {
        self.time_mark = time_mark;
        self
    }

    pub fn index(mut self, index: u32) -> Self {
        self.index = index;
        self
    }

    pub fn port_desc(mut self, desc: &str) -> Self {
        self.port_desc = Some(desc.to_string());
        self
    }

    pub fn sys_name(mut self, name: &str) -> Self {
        self.sys_name = Some(name.to_string());
        self
    }

    pub fn sys_desc(mut self, desc: &str) -> Self {
        self.sys_desc = Some(desc.to_string());
        self
    }

    pub fn defect(mut self, defect: ChassisDefect) -> Self {
        self.defect = Some(defect);
        self
    }

    fn wire_rows(&self, mib: &SimLldpMib) -> Vec<Row> {
        let suffix = (mib.rem_suffix)(self);
        let mut rows = Vec::new();

        if let Some(chassis) = &self.chassis {
            let (subtype, value) = chassis.id.to_snmp(chassis.encoding);
            match self.defect {
                // Lists the row in no chassis column at all.
                Some(ChassisDefect::NoChassisColumns) => {}
                Some(ChassisDefect::NoSubtype) => {
                    rows.push(Row::at(
                        mib.remote.chassis_id,
                        &suffix,
                        chassis_value(&chassis.id, value),
                    ));
                }
                Some(ChassisDefect::SubtypeWrongType(text)) => {
                    rows.push(Row::at(
                        mib.remote.chassis_id_subtype,
                        &suffix,
                        PassValue::Str(text.to_string()),
                    ));
                    rows.push(Row::at(
                        mib.remote.chassis_id,
                        &suffix,
                        chassis_value(&chassis.id, value),
                    ));
                }
                None => {
                    rows.push(Row::at(
                        mib.remote.chassis_id_subtype,
                        &suffix,
                        PassValue::Integer(subtype as i64),
                    ));
                    rows.push(Row::at(
                        mib.remote.chassis_id,
                        &suffix,
                        chassis_value(&chassis.id, value),
                    ));
                }
            }
        }

        if let Some(port) = &self.port {
            let (subtype, value) = port.id.to_snmp(port.encoding);
            rows.push(Row::at(
                mib.remote.port_id_subtype,
                &suffix,
                PassValue::Integer(subtype as i64),
            ));
            rows.push(Row::at(
                mib.remote.port_id,
                &suffix,
                port_value(&port.id, value),
            ));
        }
        for (base, text) in [
            (mib.remote.port_desc, &self.port_desc),
            (mib.remote.sys_name, &self.sys_name),
            (mib.remote.sys_desc, &self.sys_desc),
        ] {
            if let Some(text) = text {
                rows.push(Row::at(base, &suffix, PassValue::Str(text.clone())));
            }
        }
        rows
    }
}

/// An identifier's wire bytes as the right `pass` value: raw octets stay `octet`, everything else
/// is text. A MAC asked for in an ASCII encoding has already become text in `to_snmp`.
fn chassis_value(id: &LldpChassisId, bytes: Vec<u8>) -> PassValue {
    match id {
        LldpChassisId::MacAddress(_) | LldpChassisId::NetworkAddress(_) if bytes.len() <= 17 => {
            octet_or_text(bytes)
        }
        _ => PassValue::Str(String::from_utf8_lossy(&bytes).into_owned()),
    }
}

fn port_value(id: &LldpPortId, bytes: Vec<u8>) -> PassValue {
    match id {
        LldpPortId::MacAddress(_) | LldpPortId::NetworkAddress(_) => octet_or_text(bytes),
        _ => PassValue::Str(String::from_utf8_lossy(&bytes).into_owned()),
    }
}

/// Six bytes is a raw address; anything longer is the ASCII rendering of one.
fn octet_or_text(bytes: Vec<u8>) -> PassValue {
    if bytes.len() == 6 {
        PassValue::Octets(bytes)
    } else {
        PassValue::Str(String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// One row of `lldpLocPortTable`, keyed by `lldpLocPortNum`.
///
/// That key is a separate namespace from `ifIndex` on some firmware — ExtremeXOS numbers 1..N
/// against ifIndex 1001+, and the Dell OS10 runs 4 and 555-570 against ifIndex values in the
/// millions — which is why the daemon walks this table at all.
#[derive(Debug, Clone)]
pub struct LocalPort {
    pub num: u32,
    pub id: Advertised<LldpPortId>,
    pub desc: Option<String>,
}

impl LocalPort {
    pub fn new(num: u32, id: Advertised<LldpPortId>) -> Self {
        Self {
            num,
            id,
            desc: None,
        }
    }

    pub fn desc(mut self, desc: &str) -> Self {
        self.desc = Some(desc.to_string());
        self
    }
}

/// A device's whole LLDP MIB: who it says it is, its local ports, and its neighbours.
#[derive(Debug, Clone)]
pub struct LldpTable {
    /// Which MIB this table is served under. `new` picks the classic one; `in_mib` names another.
    pub mib: &'static SimLldpMib,
    pub chassis: Advertised<LldpChassisId>,
    pub sys_name: String,
    pub sys_desc: Option<String>,
    pub local_ports: Vec<LocalPort>,
    pub neighbours: Vec<RemoteNeighbour>,
}

impl LldpTable {
    pub fn new(chassis: Advertised<LldpChassisId>, sys_name: &str) -> Self {
        Self {
            mib: &CLASSIC,
            chassis,
            sys_name: sys_name.to_string(),
            sys_desc: None,
            local_ports: Vec::new(),
            neighbours: Vec::new(),
        }
    }

    /// Serve this table under a MIB other than the classic one.
    pub fn in_mib(mut self, mib: &'static SimLldpMib) -> Self {
        self.mib = mib;
        self
    }

    pub fn sys_desc(mut self, desc: &str) -> Self {
        self.sys_desc = Some(desc.to_string());
        self
    }

    pub fn local_ports(mut self, ports: Vec<LocalPort>) -> Self {
        self.local_ports = ports;
        self
    }

    pub fn neighbours(mut self, neighbours: Vec<RemoteNeighbour>) -> Self {
        self.neighbours = neighbours;
        self
    }

    pub fn wire_rows(&self) -> Vec<Row> {
        let (subtype, value) = self.chassis.id.to_snmp(self.chassis.encoding);
        let mut rows = vec![
            Row::scalar(
                self.mib.local.chassis_id_subtype,
                PassValue::Integer(subtype as i64),
            ),
            Row::scalar(
                self.mib.local.chassis_id,
                chassis_value(&self.chassis.id, value),
            ),
            Row::scalar(
                self.mib.local.sys_name,
                PassValue::Str(self.sys_name.clone()),
            ),
        ];
        if let Some(desc) = &self.sys_desc {
            rows.push(Row::scalar(
                self.mib.local.sys_desc,
                PassValue::Str(desc.clone()),
            ));
        }

        for port in &self.local_ports {
            let suffix = [port.num as u64];
            let (subtype, value) = port.id.id.to_snmp(port.id.encoding);
            rows.push(Row::at(
                self.mib.local.port_id_subtype,
                &suffix,
                PassValue::Integer(subtype as i64),
            ));
            rows.push(Row::at(
                self.mib.local.port_id,
                &suffix,
                port_value(&port.id.id, value),
            ));
            if let Some(desc) = &port.desc {
                rows.push(Row::at(
                    self.mib.local.port_desc,
                    &suffix,
                    PassValue::Str(desc.clone()),
                ));
            }
        }

        rows.extend(self.neighbours.iter().flat_map(|n| n.wire_rows(self.mib)));
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chassis() -> Advertised<LldpChassisId> {
        Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()))
    }

    fn port() -> Advertised<LldpPortId> {
        Advertised::octets(LldpPortId::InterfaceName("Gi0/1".into()))
    }

    fn suffixes(rows: &[Row], column: &str) -> Vec<Vec<u64>> {
        let base_len = crate::daemon::discovery::integration::snmp::oids::oid_parts(column).len();
        rows.iter()
            .filter(|row| {
                row.oid.len() > base_len
                    && row.oid[..base_len]
                        == crate::daemon::discovery::integration::snmp::oids::oid_parts(column)[..]
            })
            .map(|row| row.oid[base_len..].to_vec())
            .collect()
    }

    /// A table serves the OIDs its own MIB names, so the classic descriptor is what makes the
    /// classic rows classic — not a constant reached for inside `wire_rows`.
    ///
    /// Asserted through the roots rather than by restating each column, so this fails when a
    /// table stops being served where a walk of that MIB would look, and not when a column is
    /// legitimately renamed.
    #[test]
    fn a_table_serves_the_oids_its_own_mib_names() {
        for mib in [&CLASSIC, &V2] {
            let rows = LldpTable::new(
                Advertised::octets(LldpChassisId::MacAddress("00:11:22:33:44:55".into())),
                "switch-somewhere",
            )
            .in_mib(mib)
            .local_ports(vec![LocalPort::new(
                1,
                Advertised::text(
                    LldpPortId::InterfaceName("Gi0/1".into()),
                    MacEncoding::AsciiLower,
                ),
            )])
            .neighbours(vec![
                RemoteNeighbour::new(
                    1,
                    Advertised::octets(LldpChassisId::MacAddress("00:aa:bb:cc:dd:ee".into())),
                    Advertised::text(
                        LldpPortId::InterfaceName("Gi0/2".into()),
                        MacEncoding::AsciiLower,
                    ),
                ),
                // The wrong-typed subtype is the one row that used to name a classic column
                // regardless of the MIB the table was in.
                RemoteNeighbour::new(
                    2,
                    Advertised::octets(LldpChassisId::MacAddress("00:aa:bb:cc:dd:ef".into())),
                    Advertised::text(
                        LldpPortId::InterfaceName("Gi0/3".into()),
                        MacEncoding::AsciiLower,
                    ),
                )
                .defect(ChassisDefect::SubtypeWrongType("macAddress")),
            ])
            .wire_rows();

            assert!(!rows.is_empty());
            let root = crate::daemon::discovery::integration::snmp::oids::oid_parts(mib.root);
            assert!(
                rows.iter().all(|row| row.oid.starts_with(&root)),
                "every row must fall under {}, the MIB the table names",
                mib.root
            );
        }
    }

    /// The V2 index has four sub-ids, the third of them the destination-address row rather than
    /// anything the neighbour carries. It is the shape the classic end-relative splitter mis-reads
    /// (GH #688), so a fixture that served three would not be testing the fallback at all.
    #[test]
    fn a_v2_neighbour_is_keyed_by_time_mark_if_index_dest_address_and_index() {
        let rows = RemoteNeighbour::new(10009, chassis(), port())
            .index(6)
            .wire_rows(&V2);
        assert_eq!(
            suffixes(&rows, lldp_v2::remote::entry::LLDP_V2_REM_CHASSIS_ID),
            vec![vec![0, 10009, 1, 6]]
        );
    }

    /// The MIB's three-part index. Every device but one.
    #[test]
    fn a_neighbour_is_keyed_by_time_mark_port_and_index() {
        let rows = RemoteNeighbour::new(2, chassis(), port())
            .time_mark(TimeMark::At(31577700))
            .index(3)
            .wire_rows(&CLASSIC);
        assert_eq!(
            suffixes(&rows, lldp::remote::entry::LLDP_REM_CHASSIS_ID),
            vec![vec![31577700, 2, 3]]
        );
    }

    /// GH #668: firmware that omits `lldpRemTimeMark` indexes on the remaining two sub-ids, so
    /// every row arrives one shorter. The shape that made a sixteen-port switch vanish without
    /// raising a warning of any kind.
    #[test]
    fn a_neighbour_indexed_without_a_time_mark_is_one_sub_id_shorter() {
        let rows = RemoteNeighbour::new(1, chassis(), port())
            .time_mark(TimeMark::Omitted)
            .wire_rows(&CLASSIC);
        assert_eq!(
            suffixes(&rows, lldp::remote::entry::LLDP_REM_CHASSIS_ID),
            vec![vec![1, 1]]
        );
    }

    /// A ghost row lists itself in no chassis column, which is what makes it indistinguishable
    /// from a column that never held those positions.
    #[test]
    fn a_ghost_row_appears_in_neither_chassis_column() {
        let rows = RemoteNeighbour::new(2, chassis(), port())
            .defect(ChassisDefect::NoChassisColumns)
            .sys_name("switch-core-01")
            .wire_rows(&CLASSIC);

        assert!(suffixes(&rows, lldp::remote::entry::LLDP_REM_CHASSIS_ID).is_empty());
        assert!(suffixes(&rows, lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE).is_empty());
        // ...while the rest of the row is served, which is what makes it a *row* rather than an
        // absence.
        assert_eq!(
            suffixes(&rows, lldp::remote::entry::LLDP_REM_SYS_NAME),
            vec![vec![0, 2, 1]]
        );
    }

    /// The subtype served as text where an integer belongs. It reads as a complete walk, which is
    /// why it needed its own counter to become visible at all.
    #[test]
    fn a_wrong_typed_subtype_is_served_as_a_string() {
        let rows = RemoteNeighbour::new(1, chassis(), port())
            .defect(ChassisDefect::SubtypeWrongType("macAddress"))
            .wire_rows(&CLASSIC);
        let subtype = rows
            .iter()
            .find(|row| {
                row.oid.starts_with(
                    &crate::daemon::discovery::integration::snmp::oids::oid_parts(
                        lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
                    ),
                )
            })
            .expect("the subtype column is served — that is what makes the walk look complete");
        assert_eq!(subtype.value.type_token(), "string");
    }

    /// Both encodings are legitimate here and both are in the lab on purpose. The model must be
    /// able to express either without one being the accident.
    #[test]
    fn a_chassis_id_can_be_advertised_as_octets_or_as_text() {
        let id = LldpChassisId::MacAddress("00:ad:24:af:4e:00".into());

        let raw = LldpTable::new(Advertised::octets(id.clone()), "switch-dlink-01").wire_rows();
        let raw_value = &raw[1].value;
        assert_eq!(raw_value.type_token(), "octet");
        assert_eq!(raw_value.render(), "00 ad 24 af 4e 00");

        let text = LldpTable::new(
            Advertised::text(id, MacEncoding::AsciiUpper),
            "switch-tplink-01",
        )
        .wire_rows();
        let text_value = &text[1].value;
        assert_eq!(text_value.type_token(), "string");
        assert_eq!(text_value.render(), "00:AD:24:AF:4E:00");
    }
}
