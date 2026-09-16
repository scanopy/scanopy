use super::lldp::split_lldp_rem_index;
use super::*;
use crate::server::lldp::LldpChassisId;

const IF_INDEX: &str = "1.3.6.1.2.1.2.2.1.1";
const IF_DESCR: &str = "1.3.6.1.2.1.2.2.1.2";

/// A value an agent can return. `Value` borrows, so the test data is `'static`.
#[derive(Clone)]
enum Canned {
    Int(i64),
    Str(&'static str),
    /// Raw octets, for columns whose value is not text — an `lldpLocPortId` carrying a MAC.
    Bytes(&'static [u8]),
}

/// An agent backed by a sorted OID table, answering GETNEXT/GETBULK the way a real one does:
/// every row strictly greater than the requested OID, in order. That is what makes the
/// multi-column walk behave as it does in production — each column walk asks from its own
/// base and stops when the responses leave that subtree.
struct FakeAgent {
    rows: Vec<(Vec<u64>, Canned)>,
}

impl FakeAgent {
    fn new(rows: &[(&str, Canned)]) -> Self {
        let mut rows: Vec<(Vec<u64>, Canned)> = rows
            .iter()
            .map(|(oid, v)| {
                (
                    oid.split('.').map(|p| p.parse().unwrap()).collect(),
                    v.clone(),
                )
            })
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        Self { rows }
    }

    /// The ifTable of a switch whose 16 ports live at high ifIndexes, like the Omada
    /// TL-SG3216 in the SNMP sim.
    fn omada() -> Vec<(&'static str, Canned)> {
        let mut rows = vec![
            ("1.3.6.1.2.1.2.2.1.1.1", Canned::Int(1)),
            ("1.3.6.1.2.1.2.2.1.2.1", Canned::Str("Vlan-interface1")),
        ];
        // A handful of ports is enough to prove the assembly; the real device has 16.
        for (idx, oid, descr) in [
            (
                49153u64,
                "1.3.6.1.2.1.2.2.1.1.49153",
                "gigabitEthernet 1/0/1",
            ),
            (49154, "1.3.6.1.2.1.2.2.1.1.49154", "gigabitEthernet 1/0/2"),
            (49155, "1.3.6.1.2.1.2.2.1.1.49155", "gigabitEthernet 1/0/3"),
        ] {
            rows.push((oid, Canned::Int(idx as i64)));
            rows.push((
                match idx {
                    49153 => "1.3.6.1.2.1.2.2.1.2.49153",
                    49154 => "1.3.6.1.2.1.2.2.1.2.49154",
                    _ => "1.3.6.1.2.1.2.2.1.2.49155",
                },
                Canned::Str(descr),
            ));
        }
        rows
    }

    fn page(&self, from: &[u64]) -> Varbinds<'_> {
        let page: Varbinds<'_> = self
            .rows
            .iter()
            .filter(|(oid, _)| oid.as_slice() > from)
            .take(BULK_MAX_REPETITIONS as usize)
            .map(|(oid, v)| {
                let value = match v {
                    Canned::Int(i) => Value::Integer(*i),
                    Canned::Str(s) => Value::OctetString(s.as_bytes()),
                    Canned::Bytes(b) => Value::OctetString(b),
                };
                (oid.clone(), value)
            })
            .collect();

        // Past the last row a real agent says so rather than answering with nothing — an
        // empty response is abnormal and the walk rightly treats it as truncation. Columns
        // this device doesn't implement have to end this way or every walk reads as partial.
        if page.is_empty() {
            return vec![(from.to_vec(), Value::EndOfMibView)];
        }
        page
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for FakeAgent {
    async fn walk_getbulk<'a>(&'a mut self, from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
        Ok(WalkPage::Varbinds(self.page(from)))
    }

    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
        Ok(self.page(from))
    }
}

fn ip() -> IpAddr {
    "192.0.2.1".parse().unwrap()
}

#[tokio::test]
async fn assembles_one_entry_per_if_index_across_columns() {
    let mut agent = FakeAgent::new(&FakeAgent::omada());

    let walk = walk_if_table(&mut agent, ip()).await.unwrap();
    let entries = walk.entries;

    assert!(walk.set_complete && walk.attributes_complete);
    assert_eq!(
        entries.iter().map(|e| e.if_index).collect::<Vec<_>>(),
        vec![1, 49153, 49154, 49155],
        "every ifIndex appears exactly once, in order"
    );
    assert_eq!(entries[0].if_descr.as_deref(), Some("Vlan-interface1"));
    assert_eq!(
        entries[1].if_descr.as_deref(),
        Some("gigabitEthernet 1/0/1"),
        "a high ifIndex must keep the description from its own column"
    );
}

/// The reported defect: a switch came back with an interface belonging to a different device.
///
/// Every column mints a row on sight, so a single varbind under `ifDescr` for an ifIndex the
/// device never listed in `ifIndex` was enough to invent an interface — and the walk still
/// reported itself complete, which lets the server prune real interfaces against a table it
/// should not trust (#649). The row must be discarded and the walk must admit it is partial.
#[tokio::test]
async fn a_row_for_an_unlisted_if_index_is_discarded_and_makes_the_walk_partial() {
    let mut rows = FakeAgent::omada();
    // ifIndex 2 exists only in the ifDescr column — the shape of the foreign row.
    rows.push(("1.3.6.1.2.1.2.2.1.2.2", Canned::Str("ge-0/0/1")));
    let mut agent = FakeAgent::new(&rows);

    let walk = walk_if_table(&mut agent, ip()).await.unwrap();
    let entries = walk.entries;

    assert!(
        !entries.iter().any(|e| e.if_index == 2),
        "an ifIndex the device never listed must not become an interface"
    );
    assert_eq!(
        entries.iter().map(|e| e.if_index).collect::<Vec<_>>(),
        vec![1, 49153, 49154, 49155]
    );
    assert!(
        !walk.set_complete,
        "a table carrying rows the device never listed is not authoritative, so the server \
         must not prune against it"
    );
}

/// The gap that let a foreign interface onto switch-exos-01: the guard used to engage only
/// when the index column *finished*, so on the one scan where that column was cut short — the
/// scan most likely to be carrying stray responses — it switched itself off and a row for an
/// ifIndex the device never listed became an interface.
///
/// A truncated column still names the indexes it did return, and those are still the only
/// interfaces the device claimed.
#[tokio::test]
async fn a_truncated_index_column_still_rejects_indexes_it_never_reported() {
    struct TruncatedIndexWithGhost {
        agent: FakeAgent,
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for TruncatedIndexWithGhost {
        async fn walk_getbulk<'a>(&'a mut self, from: &[u64], max: u32) -> Result<WalkPage<'a>> {
            // The index column answers once, then dies — so it reports 1 and 49153 only.
            if from == [1, 3, 6, 1, 2, 1, 2, 2, 1, 1] {
                return Ok(WalkPage::Varbinds(vec![
                    (vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 1, 1], Value::Integer(1)),
                    (
                        vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 1, 49153],
                        Value::Integer(49153),
                    ),
                ]));
            }
            if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1]) {
                return Err(anyhow::anyhow!("getbulk timed out"));
            }
            self.agent.walk_getbulk(from, max).await
        }

        async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
            if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1]) {
                return Err(anyhow::anyhow!("getnext timed out"));
            }
            self.agent.walk_getnext(from).await
        }
    }

    // The ifDescr column carries a row for ifIndex 2, which the index column never named.
    let mut rows = FakeAgent::omada();
    rows.push(("1.3.6.1.2.1.2.2.1.2.2", Canned::Str("ge-0/0/1")));
    let mut session = TruncatedIndexWithGhost {
        agent: FakeAgent::new(&rows),
    };

    let walk = walk_if_table(&mut session, ip()).await.unwrap();

    assert!(
        !walk.entries.iter().any(|e| e.if_index == 2),
        "a row the index column never named must be rejected even when that column was cut \
         short — that is precisely when stray responses are in play"
    );
    assert_eq!(
        walk.entries.iter().map(|e| e.if_index).collect::<Vec<_>>(),
        vec![1, 49153],
        "only the indexes the device actually reported survive"
    );
    assert!(
        !walk.set_complete,
        "a cut-short index column is never an authoritative set"
    );
}

/// The guard only applies once the device has actually told us its ifIndex set. An agent that
/// serves no ifIndex column at all still gets its other columns, as before.
#[tokio::test]
async fn a_device_serving_no_if_index_column_still_yields_interfaces() {
    let mut agent = FakeAgent::new(&[
        ("1.3.6.1.2.1.2.2.1.2.7", Canned::Str("eth7")),
        ("1.3.6.1.2.1.2.2.1.3.7", Canned::Int(6)),
    ]);

    let walk = walk_if_table(&mut agent, ip()).await.unwrap();

    assert_eq!(walk.entries.len(), 1);
    assert_eq!(walk.entries[0].if_index, 7);
    assert_eq!(walk.entries[0].if_descr.as_deref(), Some("eth7"));
}

/// A flaky attribute column costs descriptions, not interfaces.
///
/// The two used to be one flag, so a timed-out `ifDescr` read both blocked the server-side
/// prune — leaving stale interfaces on the host forever — and told the operator interfaces
/// might be missing when every one had been found.
#[tokio::test]
async fn a_truncated_attribute_column_keeps_the_interface_set_authoritative() {
    struct FlakyDescr {
        agent: FakeAgent,
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for FlakyDescr {
        async fn walk_getbulk<'a>(&'a mut self, from: &[u64], max: u32) -> Result<WalkPage<'a>> {
            // ifDescr is column 2; cut it short the way a timeout does.
            if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 2]) {
                return Err(anyhow::anyhow!("getbulk timed out"));
            }
            self.agent.walk_getbulk(from, max).await
        }

        async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
            if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 2]) {
                return Err(anyhow::anyhow!("getnext timed out"));
            }
            self.agent.walk_getnext(from).await
        }
    }

    let mut session = FlakyDescr {
        agent: FakeAgent::new(&FakeAgent::omada()),
    };

    let walk = walk_if_table(&mut session, ip()).await.unwrap();

    assert_eq!(
        walk.entries.iter().map(|e| e.if_index).collect::<Vec<_>>(),
        vec![1, 49153, 49154, 49155],
        "the interface set comes from the ifIndex column, which was unaffected"
    );
    assert!(
        walk.set_complete,
        "every interface the device listed is present, so the set is prunable"
    );
    assert!(
        !walk.attributes_complete,
        "descriptions are missing and the operator should be told so"
    );
    assert!(walk.entries.iter().all(|e| e.if_descr.is_none()));
}

/// The converse: losing the index column loses the set, whatever else succeeded.
#[tokio::test]
async fn a_truncated_index_column_makes_the_set_unauthoritative() {
    struct FlakyIndex {
        agent: FakeAgent,
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for FlakyIndex {
        async fn walk_getbulk<'a>(&'a mut self, from: &[u64], max: u32) -> Result<WalkPage<'a>> {
            if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1]) {
                return Err(anyhow::anyhow!("getbulk timed out"));
            }
            self.agent.walk_getbulk(from, max).await
        }

        async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
            if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1]) {
                return Err(anyhow::anyhow!("getnext timed out"));
            }
            self.agent.walk_getnext(from).await
        }
    }

    let mut session = FlakyIndex {
        agent: FakeAgent::new(&FakeAgent::omada()),
    };

    let walk = walk_if_table(&mut session, ip()).await.unwrap();

    assert!(
        !walk.set_complete,
        "without the index column we cannot know which interfaces exist, so pruning must \
         stay blocked"
    );
}

/// A neighbour record with no chassis ID is malformed — IEEE 802.1AB makes the chassis ID a
/// mandatory TLV — and in practice means the chassis column was cut short while the port-id
/// and sys-name columns completed. Emitting it overwrote a good chassis ID with NULL, and a
/// row without one is excluded from L2 resolution entirely, so the link could never recover.
#[tokio::test]
async fn a_neighbour_without_a_chassis_id_is_dropped_and_reported_partial() {
    // lldpRemTable index is timeMark.localPortNum.remIndex; port id and sys name are present
    // for remIndex 1, chassis id is not.
    let mut agent = FakeAgent::new(&[
        ("1.0.8802.1.1.2.1.4.1.1.7.0.1.1", Canned::Str("41")),
        (
            "1.0.8802.1.1.2.1.4.1.1.9.0.1.1",
            Canned::Str("switch-core-01"),
        ),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert!(
        walk.records.is_empty(),
        "a chassis-less neighbour must not reach the server"
    );
    assert!(
        !walk.complete,
        "dropping a malformed record means this walk is not authoritative, so the server \
         must keep what it already has"
    );
    assert_eq!(
        walk.discarded, 1,
        "the count has to survive to the caller — it is what tells an operator the device \
         answered and the record itself was unusable"
    );
}

/// A record whose chassis *value* arrived but whose subtype column answered with a
/// non-integer. Indistinguishable from a truncated walk in the old logging, and the two call
/// for opposite responses: one is worth a rescan, the other never will be. GH #668.
#[tokio::test]
async fn a_chassis_id_subtype_of_the_wrong_type_is_reported_as_a_discard() {
    let mut agent = FakeAgent::new(&[
        // .4 is lldpRemChassisIdSubtype and must be an integer; this agent sends a string.
        ("1.0.8802.1.1.2.1.4.1.1.4.0.1.1", Canned::Str("macAddress")),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.0.1.1",
            Canned::Str("00:1a:2b:00:10:00"),
        ),
        ("1.0.8802.1.1.2.1.4.1.1.7.0.1.1", Canned::Str("41")),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert!(walk.records.is_empty());
    assert_eq!(walk.discarded, 1);
}

/// The mirror: the subtype is fine and the chassis id itself is not an OCTET STRING.
#[tokio::test]
async fn a_chassis_id_value_of_the_wrong_type_is_reported_as_a_discard() {
    let mut agent = FakeAgent::new(&[
        ("1.0.8802.1.1.2.1.4.1.1.4.0.1.1", Canned::Int(4)),
        // .5 must be an OCTET STRING.
        ("1.0.8802.1.1.2.1.4.1.1.5.0.1.1", Canned::Int(0)),
        ("1.0.8802.1.1.2.1.4.1.1.7.0.1.1", Canned::Str("41")),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert!(walk.records.is_empty());
    assert_eq!(walk.discarded, 1);
}

/// The rows a truncated chassis column never reached are indistinguishable from rows it never
/// had — both are absent from it — so they land in the ghost-row bucket. Reporting them as
/// ghosts tells the operator their firmware is at fault and a rescan is pointless, when a
/// rescan is in fact the entire remedy. Truncation has to outrank the count.
#[tokio::test]
async fn a_cut_short_chassis_column_is_not_reported_as_a_firmware_defect() {
    struct TruncatedChassis {
        agent: FakeAgent,
    }

    // .1.0.8802.1.1.2.1.4.1.1.5 — lldpRemChassisId, the column that stops answering.
    const CHASSIS_ID: [u64; 11] = [1, 0, 8802, 1, 1, 2, 1, 4, 1, 1, 5];

    #[async_trait::async_trait]
    impl SnmpWalkTransport for TruncatedChassis {
        async fn walk_getbulk<'a>(&'a mut self, from: &[u64], max: u32) -> Result<WalkPage<'a>> {
            if from.starts_with(&CHASSIS_ID) {
                return Err(anyhow::anyhow!("getbulk timed out"));
            }
            self.agent.walk_getbulk(from, max).await
        }

        async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
            if from.starts_with(&CHASSIS_ID) {
                return Err(anyhow::anyhow!("getnext timed out"));
            }
            self.agent.walk_getnext(from).await
        }
    }

    let mut session = TruncatedChassis {
        agent: FakeAgent::new(&[
            ("1.0.8802.1.1.2.1.4.1.1.4.0.1.1", Canned::Int(4)),
            (
                "1.0.8802.1.1.2.1.4.1.1.5.0.1.1",
                Canned::Str("00:1a:2b:00:10:00"),
            ),
            ("1.0.8802.1.1.2.1.4.1.1.7.0.1.1", Canned::Str("41")),
        ]),
    };

    let walk = query_lldp_neighbors(&mut session, ip()).await.unwrap();

    assert!(walk.discarded > 0);
    assert_eq!(
        walk.discard_reason,
        Some(MalformedNeighbourReason::WalkCutShort),
        "a read that fell short is the one cause a rescan can recover from, and it must not \
         be outvoted by rows it is itself responsible for"
    );
}

/// A ghost row: a `(localPortNum, remIndex)` that only the later columns ever mention. There
/// was never a chassis ID to lose, so this is not evidence of a cut-short chassis column —
/// the distinction the walk previously could not draw at all.
#[tokio::test]
async fn a_row_only_the_later_columns_mention_is_still_discarded() {
    let mut agent = FakeAgent::new(&[
        // A well-formed neighbour at remIndex 1...
        ("1.0.8802.1.1.2.1.4.1.1.4.0.1.1", Canned::Int(4)),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.0.1.1",
            Canned::Str("00:1a:2b:00:10:00"),
        ),
        ("1.0.8802.1.1.2.1.4.1.1.7.0.1.1", Canned::Str("41")),
        // ...and a sys-name row at remIndex 2 that the chassis columns never listed.
        ("1.0.8802.1.1.2.1.4.1.1.9.0.1.2", Canned::Str("ghost")),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert_eq!(
        walk.records.len(),
        1,
        "the well-formed neighbour must still come through"
    );
    assert_eq!(walk.discarded, 1);
}

/// Nothing discarded means nothing to report — the count must not fire on a healthy walk.
#[tokio::test]
async fn a_clean_walk_discards_nothing() {
    let mut agent = FakeAgent::new(&[
        ("1.0.8802.1.1.2.1.4.1.1.4.0.1.1", Canned::Int(4)),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.0.1.1",
            Canned::Str("00:1a:2b:00:10:00"),
        ),
        ("1.0.8802.1.1.2.1.4.1.1.7.0.1.1", Canned::Str("41")),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert_eq!(walk.records.len(), 1);
    assert_eq!(walk.discarded, 0);
    assert!(walk.complete);
}

/// A complete neighbour record still comes through intact.
#[tokio::test]
async fn a_complete_neighbour_record_is_collected() {
    let mut agent = FakeAgent::new(&[
        ("1.0.8802.1.1.2.1.4.1.1.4.0.1.1", Canned::Int(4)),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.0.1.1",
            Canned::Str("00:1a:2b:00:10:00"),
        ),
        ("1.0.8802.1.1.2.1.4.1.1.6.0.1.1", Canned::Int(7)),
        ("1.0.8802.1.1.2.1.4.1.1.7.0.1.1", Canned::Str("41")),
        (
            "1.0.8802.1.1.2.1.4.1.1.9.0.1.1",
            Canned::Str("switch-core-01"),
        ),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert_eq!(walk.records.len(), 1);
    assert!(walk.complete);
    let n = &walk.records[0];
    assert_eq!(n.local_port_index, 1);
    assert_eq!(n.remote_sys_name.as_deref(), Some("switch-core-01"));
    assert!(n.remote_chassis_id_bytes.is_some());
}

/// GH #668, the reporter's TP-Link TL-SX3016F. The OIDs below are their `snmpwalk -Ox` output
/// verbatim: this firmware omits `lldpRemTimeMark` and indexes on `localPortNum.remIndex`
/// alone, so every row arrives one sub-id shorter than the MIB describes.
///
/// Requiring three sub-ids did not merely mis-parse them, it erased the switch without a
/// trace: no record was built, so nothing reached the discard counters, the walk still
/// reported itself complete, and an empty result from a sixteen-port switch was then treated
/// as authoritative and cleared the LLDP data the server held. It was the only failure in this
/// query that produced no warning at all, which is why the device appeared in none of the
/// reporter's scan warnings while every other problem device did.
/// The neighbour key is the last two sub-ids whatever precedes them, and a suffix too short to
/// hold both is rejected rather than half-read.
///
/// The rejection arm is the one that matters: it is what routes a malformed row to
/// `short_index` and into the operator warning, instead of building a neighbour keyed on a
/// port the device never named. The four-sub-id case stands in for `lldpV2RemEntry`, which
/// this splitter is not for — it parses without complaint and yields the destination-address
/// index in place of the local port, which is why a V2 walk needs its own front-relative
/// splitter rather than this one.
#[test]
fn the_neighbour_key_is_the_last_two_sub_ids() {
    assert_eq!(split_lldp_rem_index(&[0, 3, 7]), Some((3, 7)));
    assert_eq!(split_lldp_rem_index(&[3, 7]), Some((3, 7)));
    assert_eq!(split_lldp_rem_index(&[7]), None);
    assert_eq!(split_lldp_rem_index(&[]), None);
    assert_eq!(split_lldp_rem_index(&[0, 10009, 1, 6]), Some((1, 6)));
}

#[tokio::test]
async fn a_neighbour_table_indexed_without_a_time_mark_is_read() {
    let mut agent = FakeAgent::new(&[
        // lldpRemChassisIdSubtype — macAddress(4) on local ports 1, 3 and 5.
        ("1.0.8802.1.1.2.1.4.1.1.4.1.1", Canned::Int(4)),
        ("1.0.8802.1.1.2.1.4.1.1.4.3.1", Canned::Int(4)),
        ("1.0.8802.1.1.2.1.4.1.1.4.5.1", Canned::Int(4)),
        // lldpRemChassisId — MACs as ASCII text rather than six raw octets.
        (
            "1.0.8802.1.1.2.1.4.1.1.5.1.1",
            Canned::Str("00:AD:24:89:CC:F0"),
        ),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.3.1",
            Canned::Str("40:A6:B7:B9:D8:85"),
        ),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.5.1",
            Canned::Str("18:66:DA:5D:AA:8E"),
        ),
        ("1.0.8802.1.1.2.1.4.1.1.6.1.1", Canned::Int(5)),
        ("1.0.8802.1.1.2.1.4.1.1.7.1.1", Canned::Str("1/0/1")),
        (
            "1.0.8802.1.1.2.1.4.1.1.9.1.1",
            Canned::Str("switch-core-01"),
        ),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    let mut ports: Vec<i32> = walk.records.iter().map(|n| n.local_port_index).collect();
    ports.sort_unstable();
    assert_eq!(
        ports,
        vec![1, 3, 5],
        "the last two sub-ids are the local port and remote index under either index layout"
    );
    assert_eq!(walk.discarded, 0);
    assert!(
        walk.complete,
        "nothing was lost, so this walk may overwrite what the server holds"
    );

    // The far end has to survive as far as a chassis ID, or the rows resolve to nothing.
    let port_one = walk
        .records
        .iter()
        .find(|n| n.local_port_index == 1)
        .expect("local port 1");
    let chassis = LldpChassisId::from_snmp(
        port_one.remote_chassis_id_subtype.unwrap(),
        port_one.remote_chassis_id_bytes.as_ref().unwrap(),
    );
    assert_eq!(
        chassis,
        Some(LldpChassisId::MacAddress("00:ad:24:89:cc:f0".to_string()))
    );
}

/// The two chassis columns disagreeing about which rows exist is evidence that one read came
/// up short — evidence the walk's own completeness flag cannot supply.
///
/// The `lldpRemChassisId` column lists three neighbours; `lldpRemChassisIdSubtype` lists only
/// the first two. Both walks end cleanly, because a response that skips a successor is
/// byte-for-byte identical to the end of a column: same request id, well-formed, an OID that
/// simply moved further than it should have. Nothing at the transport can tell the two apart,
/// and no OID-position rule should try — that is exactly the assumption GH #674 removed so
/// unsorted firmware could be read at all.
///
/// What *is* available is the two columns naming different row sets. For a table whose
/// identifying columns are both mandatory, that means one of them stopped early, and a rescan
/// is the remedy. Reported as `IncompleteRecords` it told the operator the opposite — that the
/// device served the row without an identifier and retrying would change nothing.
#[tokio::test]
async fn chassis_columns_listing_different_rows_are_a_short_read_not_a_malformed_record() {
    let mut agent = FakeAgent::new(&[
        // Subtype column stops after two rows.
        ("1.0.8802.1.1.2.1.4.1.1.4.100.11.1", Canned::Int(7)),
        ("1.0.8802.1.1.2.1.4.1.1.4.500.19.2", Canned::Int(4)),
        // Value column lists all three.
        ("1.0.8802.1.1.2.1.4.1.1.5.100.11.1", Canned::Str("C230408")),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.500.19.2",
            Canned::Bytes(&[0xf0, 0x64, 0x26, 0xb3, 0x84, 0x00]),
        ),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.1400.16.3",
            Canned::Bytes(&[0x78, 0x8c, 0x77, 0xe5, 0x92, 0x7d]),
        ),
        ("1.0.8802.1.1.2.1.4.1.1.9.500.19.2", Canned::Str("VSAFC11")),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert_eq!(walk.records.len(), 2, "the two complete rows still resolve");
    assert_eq!(walk.discarded, 1);
    assert_eq!(
        walk.discard_reason,
        Some(MalformedNeighbourReason::WalkCutShort),
        "one column listed a row the other never did, so the read came up short"
    );
    assert!(
        !walk.complete,
        "a lost neighbour must not let the server prune what it already holds"
    );
}

/// The floor under the change above: a row *both* columns listed, whose subtype never arrived,
/// is still the device's doing and still not worth a rescan. Without this the new signal could
/// relabel every genuine firmware defect as a transient short read.
#[tokio::test]
async fn a_row_both_chassis_columns_listed_stays_a_malformed_record() {
    let mut agent = FakeAgent::new(&[
        ("1.0.8802.1.1.2.1.4.1.1.4.100.11.1", Canned::Int(4)),
        // Listed by the subtype column, but the value is a type that column cannot hold, so
        // the row is keyed by both and still unusable.
        (
            "1.0.8802.1.1.2.1.4.1.1.4.500.19.2",
            Canned::Str("not-an-int"),
        ),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.100.11.1",
            Canned::Bytes(&[0x00, 0x11, 0xb4, 0x8c, 0x02, 0xe0]),
        ),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.500.19.2",
            Canned::Bytes(&[0xf0, 0x64, 0x26, 0xb3, 0x84, 0x00]),
        ),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert_eq!(walk.discarded, 1);
    assert_ne!(
        walk.discard_reason,
        Some(MalformedNeighbourReason::WalkCutShort),
        "both columns listed the row, so nothing came up short — a rescan is not the remedy"
    );
}

/// The floor under the fix above. An index we still cannot key has to be counted and reported,
/// not skipped — silently dropping a row is precisely what hid the TL-SX3016F, and a firmware
/// serving some other shape must not be able to hide the same way.
#[tokio::test]
async fn an_index_too_short_to_key_is_reported_rather_than_skipped() {
    let mut agent = FakeAgent::new(&[
        ("1.0.8802.1.1.2.1.4.1.1.4.1", Canned::Int(4)),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.1",
            Canned::Str("00:1a:2b:00:10:00"),
        ),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert!(walk.records.is_empty());
    assert!(walk.discarded > 0, "the rows must be accounted for");
    assert!(
        !walk.complete,
        "an unreadable row means the result is not the whole truth, so it must not overwrite"
    );
    assert_eq!(
        walk.discard_reason,
        Some(MalformedNeighbourReason::UnreadableIndex),
        "the operator needs the cause, not just the count — this one no rescan will fix"
    );
}

/// The management-address table repeats the neighbour key before its own sub-ids, so the same
/// firmware shortens it the same way. The address is enrichment, but attaching it to a
/// neighbour that does not exist loses it silently.
#[tokio::test]
async fn a_management_address_survives_a_neighbour_index_without_a_time_mark() {
    let mut agent = FakeAgent::new(&[
        ("1.0.8802.1.1.2.1.4.1.1.4.3.1", Canned::Int(4)),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.3.1",
            Canned::Str("40:A6:B7:B9:D8:85"),
        ),
        // lldpRemManAddrIfSubtype, indexed localPortNum.remIndex.addrSubtype.addrLen.addr —
        // ipV4(1), four octets, 192.168.7.245.
        (
            "1.0.8802.1.1.2.1.4.2.1.3.3.1.1.4.192.168.7.245",
            Canned::Int(2),
        ),
    ]);

    let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert_eq!(walk.records.len(), 1);
    assert_eq!(
        walk.records[0].remote_mgmt_addr,
        Some("192.168.7.245".parse::<IpAddr>().unwrap())
    );
}

/// A whole-query timeout yields the `Default`, and that must not read as a device
/// authoritatively reporting no neighbours — otherwise one slow switch wipes every link on it.
#[test]
fn a_defaulted_collection_is_never_authoritative() {
    let timed_out: SnmpCollection<Vec<LldpNeighbor>> = Default::default();
    assert!(timed_out.records.is_empty());
    assert!(!timed_out.complete);
}

/// A response that leaves the subtree *without advancing* is not this walk's natural end — it
/// is an answer to some other question. Reporting it as a finished column is what let a
/// silently short ifTable claim to be complete.
#[tokio::test]
async fn a_non_advancing_out_of_subtree_response_reports_partial() {
    struct StaleAgent;

    #[async_trait::async_trait]
    impl SnmpWalkTransport for StaleAgent {
        async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
            // Below the requested base, so it neither belongs to the subtree nor advances.
            Ok(WalkPage::Varbinds(vec![(
                vec![1, 3, 6, 1, 2, 1, 1, 1, 0],
                Value::Integer(1),
            )]))
        }

        async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
            unreachable!("getbulk answers first")
        }
    }

    let complete = walk_subtree(&mut StaleAgent, ip(), IF_DESCR, |_, _| {})
        .await
        .unwrap()
        .is_complete();
    assert!(!complete);
}

/// The other side of that rule: a genuine end-of-column response *does* advance past the
/// subtree, and must still count as a complete walk.
#[tokio::test]
async fn a_natural_end_of_column_still_reports_complete() {
    let mut agent = FakeAgent::new(&[
        ("1.3.6.1.2.1.2.2.1.1.1", Canned::Int(1)),
        ("1.3.6.1.2.1.2.2.1.2.1", Canned::Str("eth0")),
    ]);

    let mut seen = 0usize;
    let complete = walk_subtree(&mut agent, ip(), IF_INDEX, |_, _| seen += 1)
        .await
        .unwrap()
        .is_complete();

    assert!(complete, "walking off the end of a column is a natural end");
    assert_eq!(seen, 1);
}

/// An agent with no LLDP-MIB at all: every request under it comes back `noSuchObject`,
/// which is what `snmpwalk 1.0.8802.1.1.2.1.4.1` reports on a Ubiquiti USW-Pro-Max
/// ("No Such Object available on this agent at this OID").
struct NoLldpMib;

#[async_trait::async_trait]
impl SnmpWalkTransport for NoLldpMib {
    async fn walk_getbulk<'a>(&'a mut self, from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
        Ok(WalkPage::Varbinds(vec![(
            from.to_vec(),
            Value::NoSuchObject,
        )]))
    }

    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
        Ok(vec![(from.to_vec(), Value::NoSuchObject)])
    }
}

/// That answer is not the device reporting it has no neighbours, and must not be handed to
/// the server as authority to clear what it holds — the UniFi controller integration writes
/// LLDP for these exact switches, in the same scan, in no fixed order, so treating it as
/// authoritative made the neighbours appear and disappear from one scan to the next.
///
/// Scoped to the `noSuchObject` form deliberately. An agent that instead answers by
/// advancing into the next subtree it does implement is indistinguishable from one with an
/// empty table, and guessing there would break neighbour removal on healthy switches.
#[tokio::test]
async fn an_absent_lldp_mib_is_not_authority_to_clear_neighbours() {
    let lldp = query_lldp_neighbors(&mut NoLldpMib, ip()).await.unwrap();

    assert!(lldp.records.is_empty());
    assert!(lldp.unsupported, "noSuchObject means the MIB is absent");
    // The walk itself did finish, so this is not a shortfall to warn about either.
    assert!(lldp.complete);
}

/// A response to a request the daemon already gave up on lands in the socket and is read by
/// the next one, where it fails request-id validation. That is a transient belonging to the
/// previous request, and ending the walk on it turned one slow answer into a truncated table
/// — the pair visible in a customer log as `GET timeout` immediately followed by
/// `RequestIdMismatch`.
///
/// Re-issuing is safe because the failed read consumed the stale datagram, so the retry
/// cannot be handed the same one again.
#[tokio::test]
async fn a_walk_survives_reading_one_stale_answer() {
    struct DesyncsOnce {
        answered: bool,
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for DesyncsOnce {
        async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
            if !self.answered {
                self.answered = true;
                return Err(anyhow::Error::new(snmp2::Error::RequestIdMismatch)
                    .context("SNMP session desynchronized"));
            }
            // In the subtree, then the walk ends naturally on the next round.
            Ok(WalkPage::Varbinds(vec![(
                vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 2, 1],
                Value::Integer(1),
            )]))
        }

        async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
            unreachable!("getbulk answers first")
        }
    }

    let mut seen = 0usize;
    let stop = walk_subtree(
        &mut DesyncsOnce { answered: false },
        ip(),
        IF_DESCR,
        |_, _| seen += 1,
    )
    .await
    .unwrap();

    assert!(
        !matches!(stop, WalkStop::Transport),
        "one stale answer should not end the walk as a transport failure, got {stop:?}"
    );
    // The exact count is an artefact of this agent repeating one OID until the
    // non-advancing guard stops it. What matters is that the retry ran and its data was
    // collected at all — before, the walk ended on the stale answer with nothing.
    assert!(seen > 0, "the retry's data should have been collected");
}

/// The simulator's documented failure mode, and the one a busy agent produces: a response
/// that is perfectly valid — right request id, right community — and carries an OID belonging
/// to an earlier request. No transport error, so the request-id retry never saw it, and the
/// column ended there. Measured against the sim, the request-id retry alone moved nothing,
/// which is what sent us looking at this path.
#[tokio::test]
async fn a_walk_survives_one_answer_meant_for_another_request() {
    /// Answers the first request with someone else's OID, then walks the column properly.
    struct AnswersLateOnce {
        page: usize,
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for AnswersLateOnce {
        async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
            self.page += 1;
            Ok(WalkPage::Varbinds(match self.page {
                // Below the requested base: belongs to some earlier question entirely.
                1 => vec![(vec![1, 3, 6, 1, 2, 1, 1, 1, 0], Value::Integer(1))],
                2 => vec![(vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 2, 1], Value::Integer(1))],
                3 => vec![(vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 2, 2], Value::Integer(2))],
                // Past the column, advancing — the natural end.
                _ => vec![(vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 3, 1], Value::Integer(6))],
            }))
        }

        async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
            unreachable!("getbulk answers first")
        }
    }

    let mut seen = 0usize;
    let stop = walk_subtree(&mut AnswersLateOnce { page: 0 }, ip(), IF_DESCR, |_, _| {
        seen += 1
    })
    .await
    .unwrap();

    assert!(
        !matches!(stop, WalkStop::StaleResponse | WalkStop::NonAdvancingOid),
        "one misdirected answer should not end the column, got {stop:?}"
    );
    assert!(stop.is_complete(), "expected a clean end, got {stop:?}");
    assert_eq!(
        seen, 2,
        "both rows after the misdirected answer should be collected"
    );
}

/// Bounded, so a device answering persistently out of step is reported as truncated rather
/// than spun on.
///
/// Desynchronisation belongs to the *session*, so both operations are answered the same way.
/// This fake used to declare getnext `unreachable!()` on the grounds that getbulk answered
/// first, which stopped being true once a walk that has run out of retries spends its last one
/// on getnext (GH #668) — and a walk reaching getnext here is correct: the session is what is
/// wrong, not the request type, and it has to be established rather than assumed.
#[tokio::test]
async fn a_persistently_desynced_session_still_reports_truncated() {
    #[derive(Default)]
    struct AlwaysDesyncs {
        requests: usize,
    }

    impl AlwaysDesyncs {
        fn desynced(&mut self) -> anyhow::Error {
            self.requests += 1;
            anyhow::Error::new(snmp2::Error::RequestIdMismatch)
                .context("SNMP session desynchronized")
        }
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for AlwaysDesyncs {
        async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
            Err(self.desynced())
        }

        async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
            Err(self.desynced())
        }
    }

    let mut session = AlwaysDesyncs::default();
    let stop = walk_subtree(&mut session, ip(), IF_DESCR, |_, _| {})
        .await
        .unwrap();

    assert!(stop.is_truncation(), "got {stop:?}");
    assert!(
        session.requests < 10,
        "a session that is wrong in every direction must be reported, not spun on (made {} \
         requests)",
        session.requests
    );
}

/// The other side of the rule, and the one that must keep working: a switch that *has*
/// LLDP-MIB and reports no neighbours is saying there are none, so the server should clear
/// the rows it holds. Its columns end by advancing past the table.
#[tokio::test]
async fn an_implemented_but_empty_lldp_table_stays_authoritative() {
    // FakeAgent answers every walk with the next row it holds, leaving the LLDP subtree by
    // advancing rather than by noSuchObject — the shape this rule must not catch.
    let mut agent = FakeAgent::new(&FakeAgent::omada());

    let lldp = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert!(lldp.records.is_empty());
    assert!(
        !lldp.unsupported,
        "a table walked past is implemented and empty, not absent"
    );
    assert!(lldp.complete);
}

/// `get_scalar`'s default body is a GETNEXT from the OID with its last sub-id removed, which
/// works because nothing sorts strictly between `P` and `P.0`. Proving it against a fake
/// agent is what makes the whole system group testable: `query_system_info` took a concrete
/// `Box<AsyncSession>` and could not be reached without a live socket.
#[tokio::test]
async fn a_scalar_is_read_through_the_walk_transport() {
    let mut agent = FakeAgent::new(&[
        ("1.3.6.1.2.1.1.5.0", Canned::Str("switch-core-01")),
        ("1.3.6.1.2.1.1.7.0", Canned::Int(6)),
        ("1.3.6.1.2.1.2.1.0", Canned::Int(23)),
    ]);

    let info = query_system_info(&mut agent, ip()).await.unwrap();

    assert_eq!(info.sys_name.as_deref(), Some("switch-core-01"));
    assert_eq!(info.if_number, Some(23));
    // sysServices 6 = bits 2 and 3: this device bridges and routes.
    assert_eq!(info.sys_services, Some(6));
}

/// The exact-match rule. Asking for a scalar the agent does not hold must read as absent —
/// never as whatever object happens to sort next, which on this agent is a different scalar
/// entirely. That mis-read is silent and would put one device's figure on another.
#[tokio::test]
async fn an_absent_scalar_does_not_return_the_next_object() {
    // No sysServices and no ifNumber; sysName sits above both and would be returned by a
    // GETNEXT that did not check which OID came back.
    let mut agent = FakeAgent::new(&[("1.3.6.1.2.1.1.5.0", Canned::Str("switch-mute-01"))]);

    let info = query_system_info(&mut agent, ip()).await.unwrap();

    assert_eq!(info.sys_name.as_deref(), Some("switch-mute-01"));
    assert_eq!(info.sys_services, None, "sysServices is not implemented");
    assert_eq!(info.if_number, None, "ifNumber is not implemented");
}

/// A device that publishes no `dot1dBaseNumPorts` makes no claim, and the collection must
/// carry `None` rather than a zero that would read as "it said it has no ports".
#[tokio::test]
async fn a_bridge_that_publishes_no_port_count_makes_no_claim() {
    let mut agent = FakeAgent::new(&[("1.3.6.1.2.1.17.1.4.1.2.1", Canned::Int(1))]);

    let bridge = query_bridge_port_mapping(&mut agent, ip()).await.unwrap();

    assert_eq!(bridge.records.len(), 1);
    assert!(bridge.claim.is_none());
}

/// The claim has to survive a walk that returns nothing, because that is the case worth
/// reporting: a switch declaring 48 bridge ports and then serving none of the table has
/// contradicted itself, and reading the scalar after the walk would lose exactly that.
#[tokio::test]
async fn a_declared_port_count_survives_an_empty_bridge_walk() {
    let mut agent = FakeAgent::new(&[("1.3.6.1.2.1.17.1.2.0", Canned::Int(48))]);

    let bridge = query_bridge_port_mapping(&mut agent, ip()).await.unwrap();

    assert!(bridge.records.is_empty());
    assert_eq!(
        bridge.claim,
        Some(DeviceClaim::Count {
            source: ClaimSource::Dot1dBaseNumPorts,
            expected: 48,
        })
    );
}

/// `lldpLocPortId` under subtype 3 is the port's MAC, sent either as six raw octets or —
/// on firmware that formats it itself — as text. Neither reached the resolver: the column
/// was read only as a string, so the octets decoded to nothing and the text to something
/// that matches no interface name. Both must arrive as the same address.
///
/// The description column is walked here too; it was not collected at all before, and on
/// this vendor it is the only column that names the interface.
#[tokio::test]
async fn a_mac_port_id_is_read_from_either_encoding_alongside_its_description() {
    const SUBTYPE: &str = "1.0.8802.1.1.2.1.3.7.1.2";
    const PORT_ID: &str = "1.0.8802.1.1.2.1.3.7.1.3";
    const PORT_DESC: &str = "1.0.8802.1.1.2.1.3.7.1.4";

    let mut agent = FakeAgent::new(&[
        (&format!("{SUBTYPE}.10"), Canned::Int(3)),
        (
            &format!("{PORT_ID}.10"),
            Canned::Bytes(&[2, 0, 0, 0, 0, 0xEA]),
        ),
        (&format!("{PORT_DESC}.10"), Canned::Str("100-T eth10")),
        (&format!("{SUBTYPE}.11"), Canned::Int(3)),
        (&format!("{PORT_ID}.11"), Canned::Str("02:00:00:00:00:e9")),
        (&format!("{PORT_DESC}.11"), Canned::Str("100-T eth9")),
    ]);

    let ports = query_lldp_local_ports(&mut agent, ip())
        .await
        .unwrap()
        .records;

    assert_eq!(
        ports[&10].port_id_mac,
        Some(mac_address::MacAddress::new([2, 0, 0, 0, 0, 0xEA])),
        "six raw octets are the port's address"
    );
    assert_eq!(
        ports[&11].port_id_mac,
        Some(mac_address::MacAddress::new([2, 0, 0, 0, 0, 0xE9])),
        "the same address written as text is the same address"
    );
    assert_eq!(ports[&10].port_desc.as_deref(), Some("100-T eth10"));
}
