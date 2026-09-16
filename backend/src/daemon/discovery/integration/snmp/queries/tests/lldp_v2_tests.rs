use super::lldp::{split_lldp_rem_index, split_lldp_v2_man_addr_index, split_lldp_v2_rem_index};
use super::*;

/// A value an agent can return; `Value` borrows, so test data is `'static`.
#[derive(Clone)]
enum Canned {
    Int(i64),
    Str(&'static str),
    Bytes(&'static [u8]),
}

/// Same shape as `if_table_tests::FakeAgent`: a sorted OID table answered the way a real
/// agent pages a walk, with EndOfMibView past the last row so absent subtrees read as
/// unsupported rather than truncated.
///
/// A request under `stalls` gets no answer at all, which is how a walk of that subtree is
/// made to come up short without touching any other.
struct Agent {
    rows: Vec<(Vec<u64>, Canned)>,
    stalls: Option<Vec<u64>>,
}

impl Agent {
    fn new(rows: &[(&str, Canned)]) -> Self {
        let mut rows: Vec<(Vec<u64>, Canned)> = rows
            .iter()
            .map(|(oid, v)| (oids::oid_parts(oid), v.clone()))
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        Self { rows, stalls: None }
    }

    fn stalling_under(mut self, subtree: &str) -> Self {
        self.stalls = Some(oids::oid_parts(subtree));
        self
    }

    fn page(&self, from: &[u64]) -> Result<Varbinds<'_>> {
        if let Some(stalled) = &self.stalls
            && from.starts_with(stalled)
        {
            anyhow::bail!("timed out");
        }
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
        if page.is_empty() {
            return Ok(vec![(from.to_vec(), Value::EndOfMibView)]);
        }
        Ok(page)
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for Agent {
    async fn walk_getbulk<'a>(&'a mut self, from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
        Ok(WalkPage::Varbinds(self.page(from)?))
    }

    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
        self.page(from)
    }
}

fn ip() -> IpAddr {
    "192.0.2.1".parse().unwrap()
}

/// The rows an OcNOS 7.0.1 agent actually served (UfiSpace S9600-32X, `snmpwalk -On`,
/// 2026-08-24; identifiers rewritten): three neighbours indexed
/// `timeMark.ifIndex.destMacIndex.remIndex`, chassis ids as MAC octets, and a
/// management-address table with no address-length sub-id. Nothing under the classic root.
fn ocnos_rows() -> Vec<(&'static str, Canned)> {
    const CORE: &[u8] = &[0x00, 0x1a, 0x2b, 0x00, 0x10, 0x00];
    const SPINE1: &[u8] = &[0x00, 0x1a, 0x2b, 0x40, 0xe9, 0xca];
    const SPINE2: &[u8] = &[0x00, 0x1a, 0x2b, 0x40, 0xd4, 0xca];
    vec![
        // chassis subtype (4 = macAddress)
        ("1.3.111.2.802.1.1.13.1.4.1.1.5.0.3.1.4", Canned::Int(4)),
        ("1.3.111.2.802.1.1.13.1.4.1.1.5.0.10009.1.6", Canned::Int(4)),
        ("1.3.111.2.802.1.1.13.1.4.1.1.5.0.10073.1.2", Canned::Int(4)),
        // chassis id
        (
            "1.3.111.2.802.1.1.13.1.4.1.1.6.0.3.1.4",
            Canned::Bytes(CORE),
        ),
        (
            "1.3.111.2.802.1.1.13.1.4.1.1.6.0.10009.1.6",
            Canned::Bytes(SPINE1),
        ),
        (
            "1.3.111.2.802.1.1.13.1.4.1.1.6.0.10073.1.2",
            Canned::Bytes(SPINE2),
        ),
        // port id subtype / id
        ("1.3.111.2.802.1.1.13.1.4.1.1.7.0.3.1.4", Canned::Int(7)),
        ("1.3.111.2.802.1.1.13.1.4.1.1.7.0.10009.1.6", Canned::Int(5)),
        ("1.3.111.2.802.1.1.13.1.4.1.1.7.0.10073.1.2", Canned::Int(5)),
        (
            "1.3.111.2.802.1.1.13.1.4.1.1.8.0.3.1.4",
            Canned::Str("Ethernet5"),
        ),
        (
            "1.3.111.2.802.1.1.13.1.4.1.1.8.0.10009.1.6",
            Canned::Str("swp5"),
        ),
        (
            "1.3.111.2.802.1.1.13.1.4.1.1.8.0.10073.1.2",
            Canned::Str("swp5"),
        ),
        // sys name
        (
            "1.3.111.2.802.1.1.13.1.4.1.1.10.0.3.1.4",
            Canned::Str("switch-core-01"),
        ),
        (
            "1.3.111.2.802.1.1.13.1.4.1.1.10.0.10009.1.6",
            Canned::Str("switch-arcos-01"),
        ),
        (
            "1.3.111.2.802.1.1.13.1.4.1.1.10.0.10073.1.2",
            Canned::Str("switch-arcos-02"),
        ),
        // management addresses: OcNOS layout, address bytes to the end of the index —
        // and one row that carries a subtype but no address at all.
        ("1.3.111.2.802.1.1.13.1.4.2.1.3.0.3.1.4.2", Canned::Int(2)),
        (
            "1.3.111.2.802.1.1.13.1.4.2.1.3.0.10009.1.6.1.192.0.2.102",
            Canned::Int(2),
        ),
        (
            "1.3.111.2.802.1.1.13.1.4.2.1.3.0.10073.1.2.1.192.0.2.103",
            Canned::Int(2),
        ),
    ]
}

#[tokio::test]
async fn v2_only_agent_yields_neighbours_keyed_by_if_index() {
    let mut agent = Agent::new(&ocnos_rows());

    let got = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert!(!got.unsupported, "an agent serving V2 rows has LLDP");
    assert!(got.complete);
    assert_eq!(got.discarded, 0);
    assert!(
        got.local_port_is_if_index,
        "the caller must be told not to remap these"
    );
    let mut records = got.records;
    records.sort_by_key(|n| n.local_port_index);
    assert_eq!(
        records
            .iter()
            .map(|n| n.local_port_index)
            .collect::<Vec<_>>(),
        vec![3, 10009, 10073],
        "the V2 local identifier is the ifIndex, second from the front"
    );
    assert_eq!(
        records[0].remote_sys_name.as_deref(),
        Some("switch-core-01")
    );
    assert_eq!(
        records[1].remote_sys_name.as_deref(),
        Some("switch-arcos-01")
    );
    assert_eq!(
        records[1].remote_port_id_bytes.as_deref(),
        Some(b"swp5" as &[u8])
    );
    assert_eq!(
        records[1].remote_mgmt_addr,
        Some("192.0.2.102".parse().unwrap()),
        "V2 management address reconstructed from a length-less index"
    );
    assert!(
        records[0].remote_mgmt_addr.is_none(),
        "a management row with no address bytes resolves to nothing, not garbage"
    );
}

#[tokio::test]
async fn classic_agent_never_reaches_the_v2_walk() {
    // One classic neighbour (index timeMark.localPortNum.remIndex = 0.5.1) — and V2 rows
    // carrying different sys names. If the fallback ran anyway, the V2 rows would either
    // merge or collide; the classic result must come back alone and untouched.
    let mut rows = vec![
        ("1.0.8802.1.1.2.1.4.1.1.4.0.5.1", Canned::Int(4)),
        (
            "1.0.8802.1.1.2.1.4.1.1.5.0.5.1",
            Canned::Bytes(&[0x02, 0x00, 0x00, 0x00, 0x00, 0x01]),
        ),
        (
            "1.0.8802.1.1.2.1.4.1.1.9.0.5.1",
            Canned::Str("classic-peer"),
        ),
    ];
    rows.extend(ocnos_rows());
    let mut agent = Agent::new(&rows);

    let got = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert_eq!(got.records.len(), 1, "V2 rows must not be merged in");
    assert_eq!(got.records[0].local_port_index, 5);
    assert_eq!(
        got.records[0].remote_sys_name.as_deref(),
        Some("classic-peer")
    );
    assert!(
        !got.local_port_is_if_index,
        "a classic result still goes through the local-port remap"
    );
}

#[tokio::test]
async fn an_agent_with_neither_mib_is_still_unsupported() {
    // No rows at all: the fake then answers EndOfMibView at the first request of every
    // column, which is the shape `is_unsupported` keys on.
    let mut agent = Agent::new(&[]);

    let got = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert!(got.records.is_empty());
    assert!(
        got.unsupported,
        "falling back must not turn a no-LLDP agent into a supported-but-empty one"
    );
    assert!(!got.local_port_is_if_index);
}

/// An empty classic result from a walk that did not finish is a failed read, not a device
/// with nothing — and not a licence to go looking elsewhere. The V2 rows are there to be
/// found; the point is that they must not be.
#[tokio::test]
async fn an_incomplete_classic_walk_does_not_fall_back() {
    let mut agent = Agent::new(&ocnos_rows()).stalling_under(oids::lldp::LLDP_MIB);

    let got = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert!(
        got.records.is_empty(),
        "V2 rows read past a failed classic walk"
    );
    assert!(!got.complete);
    assert!(!got.local_port_is_if_index);
}

/// The mirror image: on a V2-only device the classic result is complete and *not*
/// unsupported, which is the shape the server takes as authority to clear what it holds. A
/// V2 walk that stalls must not hand that back — it returns its own incomplete, empty result.
#[tokio::test]
async fn an_incomplete_v2_walk_is_not_an_authoritative_empty_result() {
    let mut agent = Agent::new(&ocnos_rows()).stalling_under(oids::lldp_v2::LLDP_V2_MIB);

    let got = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

    assert!(got.records.is_empty());
    assert!(
        !got.complete,
        "a stalled fallback is a failed read, not an empty device"
    );
    assert!(!got.unsupported);
    assert!(got.local_port_is_if_index);
}

/// The V2 index is front-relative and whole: four sub-ids or nothing. Three is the classic
/// layout, which the end-relative classic splitter would happily accept and mis-key; five is
/// a row from some other table.
#[test]
fn the_v2_rem_index_needs_exactly_four_sub_ids() {
    assert_eq!(split_lldp_v2_rem_index(&[0, 10009, 1, 6]), Some((10009, 6)));
    assert_eq!(split_lldp_v2_rem_index(&[10009, 1, 6]), None);
    assert_eq!(split_lldp_v2_rem_index(&[0, 10009, 1, 6, 1]), None);
    assert_eq!(split_lldp_v2_rem_index(&[]), None);
}

/// The same suffix through the classic splitter is the failure the V2 one exists to avoid:
/// every neighbour keyed on the destination-address index.
#[test]
fn the_classic_splitter_mis_keys_a_v2_index() {
    assert_eq!(split_lldp_rem_index(&[0, 10009, 1, 6]), Some((1, 6)));
}

#[test]
fn the_v2_man_addr_index_is_read_with_or_without_a_length() {
    // OcNOS: no length sub-id, address to the end.
    assert_eq!(
        split_lldp_v2_man_addr_index(&[0, 10009, 1, 6, 1, 192, 0, 2, 102]),
        Some((10009, 6, vec![1, 192, 0, 2, 102]))
    );
    // Conformant: the length accounts for exactly what follows.
    assert_eq!(
        split_lldp_v2_man_addr_index(&[0, 10009, 1, 6, 1, 4, 192, 0, 2, 102]),
        Some((10009, 6, vec![1, 192, 0, 2, 102]))
    );
    // A subtype with nothing after it: the row exists and carries no address.
    assert_eq!(
        split_lldp_v2_man_addr_index(&[0, 3, 1, 4, 2]),
        Some((3, 4, vec![2]))
    );
    assert_eq!(split_lldp_v2_man_addr_index(&[0, 3, 1, 4]), None);
}
