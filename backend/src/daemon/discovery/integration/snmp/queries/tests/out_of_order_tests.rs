use super::*;

const ARP_IF_INDEX: &str = "1.3.6.1.2.1.4.22.1.1";
const ARP_PHYS: &str = "1.3.6.1.2.1.4.22.1.2";
const ARP_NET: &str = "1.3.6.1.2.1.4.22.1.3";
const ARP_TYPE: &str = "1.3.6.1.2.1.4.22.1.4";

fn ip() -> IpAddr {
    "192.0.2.1".parse().unwrap()
}

fn oid(s: &str) -> Vec<u64> {
    s.split('.').map(|p| p.parse().unwrap()).collect()
}

enum Cell {
    Int(i64),
    Bytes(Vec<u8>),
    Ip([u8; 4]),
}

/// An agent that iterates its rows in the order it was given them, not in OID order.
struct ScrambledAgent {
    seq: Vec<(Vec<u64>, Cell)>,
}

impl ScrambledAgent {
    fn page(&self, from: &[u64], max: usize) -> Varbinds<'_> {
        let start = match self.seq.iter().position(|(o, _)| o.as_slice() == from) {
            Some(i) => i + 1,
            // A bare column base names no row of its own. A real agent answers it with the
            // first row it holds beyond that point — which, iterating its own order, need
            // not be the numerically smallest one.
            None => self
                .seq
                .iter()
                .position(|(o, _)| o.as_slice() > from)
                .unwrap_or(self.seq.len()),
        };
        let page: Varbinds<'_> = self.seq[start..]
            .iter()
            .take(max)
            .map(|(o, cell)| {
                let value = match cell {
                    Cell::Int(i) => Value::Integer(*i),
                    Cell::Bytes(b) => Value::OctetString(b),
                    Cell::Ip(a) => Value::IpAddress(*a),
                };
                (o.clone(), value)
            })
            .collect();
        if page.is_empty() {
            return vec![(from.to_vec(), Value::EndOfMibView)];
        }
        page
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for ScrambledAgent {
    async fn walk_getbulk<'a>(&'a mut self, from: &[u64], max: u32) -> Result<WalkPage<'a>> {
        Ok(WalkPage::Varbinds(self.page(from, max as usize)))
    }
    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
        Ok(self.page(from, 1))
    }
}

/// Host octets in the order the agent hands them out: evens first, then odds. The point is
/// only that a later page ends lower than an earlier one did, which is what a strictly
/// ascending walk cannot survive. `HOSTS` is sized so that happens on a full second page.
fn scrambled_hosts() -> Vec<u8> {
    let evens = (1..=45u8).filter(|n| n % 2 == 0);
    let odds = (1..=45u8).filter(|n| n % 2 == 1);
    evens.chain(odds).collect()
}

/// A complete four-column ARP table, every column served in the scrambled order.
fn scrambled_arp() -> ScrambledAgent {
    let mut seq = Vec::new();
    for (column, _) in [
        (ARP_IF_INDEX, 0),
        (ARP_PHYS, 1),
        (ARP_NET, 2),
        (ARP_TYPE, 3),
    ] {
        for host in scrambled_hosts() {
            let key = format!("{column}.3.192.0.2.{host}");
            let cell = match column {
                ARP_PHYS => Cell::Bytes(vec![0x00, 0x11, 0x22, 0x33, 0x44, host]),
                ARP_NET => Cell::Ip([192, 0, 2, host]),
                ARP_TYPE => Cell::Int(3),
                _ => Cell::Int(3),
            };
            seq.push((oid(&key), cell));
        }
    }
    ScrambledAgent { seq }
}

/// The defect itself: rows that go backwards are still rows. A walk that refuses them reads
/// part of the table and calls the rest absent, which is what emptied the reporter's scan.
#[tokio::test]
async fn a_table_served_out_of_order_is_read_in_full() {
    let mut agent = scrambled_arp();

    let entries = query_arp_table(&mut agent, ip()).await.unwrap().records;

    assert_eq!(
        entries.len(),
        45,
        "every ARP row the device holds must be collected, whatever order it serves them in"
    );
}

/// The reporter's symptom was `count=0`, not a short count — and an ordering fault alone does
/// not explain that, because rows already passed to the collector are kept. This is what does:
/// the ARP row is a join across four columns and needs all of them, so one column coming up
/// empty discards every row the other three read in full.
///
/// The join is right to insist — an ARP entry with no MAC is not usable — so the fix is not to
/// relax it but to stop the loss being silent (`SnmpWalkGroup::ArpTable`).
#[tokio::test]
async fn one_empty_column_discards_every_row_the_others_read() {
    let mut agent = scrambled_arp();
    // A device that answers the other three columns and holds nothing under physAddress.
    agent.seq.retain(|(o, _)| !o.starts_with(&oid(ARP_PHYS)));

    let entries = query_arp_table(&mut agent, ip()).await.unwrap().records;

    assert!(
        entries.is_empty(),
        "the join drops rows with no MAC, so the collection reports nothing at all"
    );
}

/// An agent that serves a fixed script regardless of what was asked, so a page can be
/// re-delivered exactly as a real one does when the walk re-asks.
struct ScriptedAgent {
    pages: std::collections::VecDeque<Varbinds<'static>>,
}

#[async_trait::async_trait]
impl SnmpWalkTransport for ScriptedAgent {
    async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
        Ok(WalkPage::Varbinds(
            self.pages.pop_front().unwrap_or_default(),
        ))
    }
    async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
        Ok(self.pages.pop_front().unwrap_or_default())
    }
}

/// A page that ends in a response belonging to another question is re-asked, and the rows
/// ahead of that response on the same page have already reached the collector. The cursor has
/// not moved, so the agent serves them again — which used to append a second copy of every
/// VLAN and every per-port membership, the two collectors that push rather than key by index.
#[tokio::test]
async fn re_asking_a_page_does_not_deliver_its_rows_twice() {
    const BASE: &str = "1.3.6.1.2.1.2.2.1.2";
    let row = |oid_str: &str| (oid(oid_str), Value::Integer(1));
    // The trailing OID is below the base and outside it: a leftover answer to an earlier
    // question, which is what triggers the re-ask. `Value` is not `Clone`, so the page the
    // agent re-delivers is built a second time rather than copied.
    let interrupted = || {
        vec![
            row("1.3.6.1.2.1.2.2.1.2.1"),
            row("1.3.6.1.2.1.2.2.1.2.2"),
            row("1.3.6.1.2.1.2.2.1.1.9"),
        ]
    };
    let mut agent = ScriptedAgent {
        pages: [
            interrupted(),
            interrupted(),
            vec![row("1.3.6.1.2.1.2.2.1.3.1")],
        ]
        .into_iter()
        .collect(),
    };

    let mut suffixes = Vec::new();
    walk_subtree(&mut agent, ip(), BASE, |suffix, _v| {
        suffixes.push(suffix.to_vec())
    })
    .await
    .unwrap();

    assert_eq!(
        suffixes,
        vec![vec![1], vec![2]],
        "each row must reach the collector once, however many times the page is served"
    );
}

/// Tolerating rows that go backwards removes the ordering guarantee that used to bound the
/// walk, so something else has to. An agent that never repeats itself and never leaves the
/// subtree cannot be told from a very large table, and only the entry cap ends it.
#[tokio::test]
async fn an_agent_that_never_repeats_still_stops_at_the_entry_cap() {
    /// Answers every request with rows it has never sent before, for ever.
    struct EndlessAgent {
        next: u64,
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for EndlessAgent {
        async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], max: u32) -> Result<WalkPage<'a>> {
            let page = (0..max as u64)
                .map(|i| {
                    (
                        oid(&format!("1.3.6.1.2.1.2.2.1.2.{}", self.next + i)),
                        Value::Integer(1),
                    )
                })
                .collect();
            self.next += max as u64;
            Ok(WalkPage::Varbinds(page))
        }
        async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
            unreachable!("the walk uses getbulk here")
        }
    }

    let mut agent = EndlessAgent { next: 1 };
    let mut count = 0usize;
    let stop = walk_subtree(&mut agent, ip(), "1.3.6.1.2.1.2.2.1.2", |_s, _v| count += 1)
        .await
        .unwrap();

    assert_eq!(count, MAX_WALK_ENTRIES);
    assert!(!stop.is_complete(), "a capped walk is not a finished one");
}
